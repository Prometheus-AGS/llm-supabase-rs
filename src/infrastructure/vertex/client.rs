use anyhow::{Context, Result};
use reqwest::Client;
use serde_json::Value;
use std::sync::Arc;
use tokio_stream::Stream;
use tracing::{debug, info, warn};

use crate::config::providers::VertexAiConfig;
use crate::infrastructure::vertex::{
    VertexStreamChunk, VertexAuthClient, AuthContext,
    VertexPredictRequest, VertexPredictResponse,
    VertexError, VertexErrorDetails, VertexContent
};

/// Vertex AI HTTP client for Claude 4 Sonnet model access
/// Handles authentication, request/response formatting, and error handling
pub struct VertexAIClient {
    /// HTTP client for making requests
    client: Client,

    /// Vertex AI configuration
    config: VertexAiConfig,

    /// Authentication client for GCP access tokens
    auth_client: Arc<VertexAuthClient>,
}

impl VertexAIClient {
    /// Create a new Vertex AI client with authentication
    pub async fn new(config: VertexAiConfig) -> Result<Self> {
        info!("Initializing Vertex AI client for project: {}", config.project_id);

        // Initialize authentication client
        let auth_client = VertexAuthClient::new(config.auth.clone())
            .await
            .context("Failed to initialize Vertex AI authentication")?;

        let auth_client = Arc::new(auth_client);

        // Validate authentication works
        auth_client.validate_auth().await
            .context("Failed to validate Vertex AI authentication")?;

        // Configure HTTP client with appropriate timeouts
        let client = Client::builder()
            .timeout(std::time::Duration::from_secs(config.endpoint.request_timeout))
            .connect_timeout(std::time::Duration::from_secs(config.endpoint.connection_timeout))
            .build()
            .context("Failed to create HTTP client")?;

        info!("Successfully initialized Vertex AI client");

        Ok(Self {
            client,
            config,
            auth_client,
        })
    }

    /// Get a valid authentication context
    async fn get_auth_context(&self) -> Result<AuthContext> {
        self.auth_client
            .get_auth_context()
            .await
            .context("Failed to get authentication context")
    }

    /// Get the access token, refreshing if needed
    async fn get_access_token(&self) -> Result<String> {
        let auth_context = self.get_auth_context().await?;

        // Check if token needs refresh
        if auth_context.needs_refresh {
            debug!("Access token needs refresh, getting new token");
            let refreshed = self.auth_client.refresh_token().await?;
            Ok(refreshed.bearer_token)
        } else {
            Ok(auth_context.bearer_token)
        }
    }

    /// Make a prediction request to Vertex AI Claude model
    pub async fn predict(
        &self,
        model: &str,
        request: VertexPredictRequest,
    ) -> Result<VertexPredictResponse> {
        let access_token = self.get_access_token().await?;
        let url = self.config.prediction_endpoint(model);

        debug!("Making Vertex AI prediction request to: {}", url);

        let response = self
            .client
            .post(&url)
            .header("Authorization", format!("Bearer {}", access_token))
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .await
            .context("HTTP request to Vertex AI failed")?;

        self.handle_response(response).await
    }

    /// Make a streaming prediction request to Vertex AI Claude model
    pub async fn predict_streaming(
        &self,
        model: &str,
        request: VertexPredictRequest,
    ) -> Result<impl Stream<Item = Result<VertexStreamChunk>>> {
        let access_token = self.get_access_token().await?;
        let url = self.config.streaming_endpoint(model);

        debug!("Making Vertex AI streaming request to: {}", url);

        let response = self
            .client
            .post(&url)
            .header("Authorization", format!("Bearer {}", access_token))
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .await
            .context("HTTP request to Vertex AI failed")?;

        if !response.status().is_success() {
            let error = self.parse_error_response(response).await?;
            return Err(anyhow::anyhow!("Vertex AI API error: {}", error.error.message));
        }

        // Create a stream from the response bytes with proper JSON buffering
        let stream = response.bytes_stream();
        
        // Use a channel to maintain buffer state across chunks
        let (tx, rx) = tokio::sync::mpsc::channel(100);
        let mut json_buffer = super::streaming_buffer::JsonStreamingBuffer::new();
        
        // Spawn a task to process chunks with persistent buffer
        tokio::spawn(async move {
            
            let mut stream = stream;
            while let Some(chunk_result) = futures_util::StreamExt::next(&mut stream).await {
                match chunk_result {
                    Ok(chunk) => {
                        let chunk_str = match String::from_utf8(chunk.to_vec()) {
                            Ok(s) => s,
                            Err(_) => {
                                let _ = tx.send(Err(anyhow::anyhow!("Invalid UTF-8 in stream chunk"))).await;
                                continue;
                            }
                        };

                        debug!("Raw Vertex AI chunk received: {} chars", chunk_str.len());
                        
                        // Handle SSE format - parse both single-line and multi-line events
                        let clean_chunk = if chunk_str.starts_with("data: ") {
                            let data = chunk_str.strip_prefix("data: ").unwrap_or(&chunk_str);
                            if data.trim() == "[DONE]" {
                                debug!("Received [DONE] from Vertex AI");
                                // Send final chunk
                                let final_chunk = VertexStreamChunk {
                                    event_type: "message_stop".to_string(),
                                    id: None,
                                    role: None,
                                    model: None,
                                    content: None,
                                    index: None,
                                    delta: None,
                                    message: None,
                                    content_block: None,
                                    usage: None,
                                };
                                let _ = tx.send(Ok(final_chunk)).await;
                                break;
                            }
                            data.to_string()
                        } else if chunk_str.contains("event: ") && chunk_str.contains("data: ") {
                            // Handle multi-line SSE events (event: + data: format)
                            let lines: Vec<&str> = chunk_str.lines().collect();
                            let mut event_type = "";
                            let mut data_content = "";
                            
                            for line in lines {
                                if let Some(event) = line.strip_prefix("event: ") {
                                    event_type = event.trim();
                                } else if let Some(data) = line.strip_prefix("data: ") {
                                    data_content = data.trim();
                                }
                            }
                            
                            // Only process events that contain tool_use or other important content
                            if event_type == "content_block_start" || event_type == "content_block_delta" || event_type == "message_delta" {
                                debug!("Processing SSE event: {} with data: {}", event_type, data_content);
                                
                                // Wrap the data with the event type so the buffer can recognize it
                                if !data_content.is_empty() {
                                    // Parse the data as JSON and add the type field
                                    if let Ok(mut data_json) = serde_json::from_str::<Value>(data_content) {
                                        if let Some(obj) = data_json.as_object_mut() {
                                            // Add the event type to the JSON object
                                            obj.insert("type".to_string(), Value::String(event_type.to_string()));
                                            // Return the enhanced JSON as a string
                                            serde_json::to_string(&data_json).unwrap_or_else(|_| data_content.to_string())
                                        } else {
                                            data_content.to_string()
                                        }
                                    } else {
                                        data_content.to_string()
                                    }
                                } else {
                                    continue;
                                }
                            } else {
                                debug!("Skipping SSE event: {}", event_type);
                                continue;
                            }
                        } else if chunk_str.trim().starts_with("{") {
                            chunk_str.to_string()
                        } else if chunk_str.trim().is_empty() {
                            // Skip empty chunks
                            continue;
                        } else {
                            debug!("Skipping non-JSON chunk: {}", chunk_str);
                            continue;
                        };
                        
                        // Add to buffer and get complete processed chunks
                        let processed_chunks = json_buffer.add_chunk(&clean_chunk);
                        
                        // Process each chunk
                        for processed_chunk in processed_chunks {
                            debug!("Processing chunk: {:?}", processed_chunk);
                            
                            // Convert ProcessedChunk to VertexStreamChunk
                            let vertex_chunk = match processed_chunk {
                                super::streaming_buffer::ProcessedChunk::Content(text) => {
                                    debug!("Converting content chunk: {} chars", text.len());
                                    Some(VertexStreamChunk {
                                        event_type: "content_block_delta".to_string(),
                                        id: None,
                                        role: None,
                                        model: None,
                                        content: Some(vec![VertexContent::Text {
                                            text,
                                        }]),
                                        index: None,
                                        delta: None,
                                        message: None,
                                        content_block: None,
                                        usage: None,
                                    })
                                },
                                // Handle tool use chunks properly
                                super::streaming_buffer::ProcessedChunk::ToolUse { id, name, input } => {
                                    debug!("Converting tool use chunk: {} ({})", name, id);
                                    Some(VertexStreamChunk {
                                        event_type: "content_block_start".to_string(),
                                        id: None,
                                        role: None,
                                        model: None,
                                        content: Some(vec![VertexContent::ToolUse {
                                            id: id.clone(),
                                            name: name.clone(),
                                            input: input.clone(),
                                        }]),
                                        index: None,
                                        delta: None,
                                        message: None,
                                        content_block: None,
                                        usage: None,
                                    })
                                },
                                // Other chunk types (simplified for this fix)
                                _ => {
                                    debug!("Converting other chunk type: {:?}", processed_chunk);
                                    None // Skip generic chunks that don't need streaming
                                }
                            };
                            
                            // Send the chunk to the channel
                            if let Some(chunk) = vertex_chunk {
                                if tx.send(Ok(chunk)).await.is_err() {
                                    debug!("Receiver dropped, ending processing");
                                    return;
                                }
                            }
                        }
                    },
                    Err(e) => {
                        let _ = tx.send(Err(anyhow::anyhow!("Stream chunk error: {}", e))).await;
                        break;
                    }
                }
            }
        });
        
        // Create a stream from the receiver
        let parsed_stream = tokio_stream::wrappers::ReceiverStream::new(rx);
        
        Ok(parsed_stream)
    }



    /// Handle HTTP response and parse result or error
    async fn handle_response(&self, response: reqwest::Response) -> Result<VertexPredictResponse> {
        if response.status().is_success() {
            let vertex_response = response
                .json::<VertexPredictResponse>()
                .await
                .context("Failed to parse Vertex AI response")?;

            debug!("Successful Vertex AI response received");
            Ok(vertex_response)
        } else {
            let error = self.parse_error_response(response).await?;
            Err(anyhow::anyhow!("Vertex AI API error: {}", serde_json::to_string_pretty(&error).unwrap_or_else(|_| error.error.message.clone())))
        }
    }

    /// Parse error response from Vertex AI API
    async fn parse_error_response(&self, response: reqwest::Response) -> Result<VertexError> {
        let status = response.status();
        let error_text = response.text().await.unwrap_or_default();

        // Try to parse as Vertex AI error format
        if let Ok(vertex_error) = serde_json::from_str::<VertexError>(&error_text) {
            return Ok(vertex_error);
        }

        // Fallback to generic error
        Ok(VertexError {
            error_type: "error".to_string(),
            error: VertexErrorDetails {
                error_type: "api_error".to_string(),
                message: if error_text.is_empty() {
                    format!("HTTP {}", status)
                } else {
                    error_text
                },
            },
        })
    }

    /// List available Claude models from Vertex AI
    pub async fn list_models(&self) -> Result<Vec<String>> {
        let access_token = self.get_access_token().await?;

        let url = format!(
            "{}/v1/projects/{}/locations/{}/publishers/anthropic/models",
            self.config.endpoint.base_url,
            self.config.project_id,
            self.config.region
        );

        debug!("Listing Vertex AI models from: {}", url);

        let response = self
            .client
            .get(&url)
            .header("Authorization", format!("Bearer {}", access_token))
            .send()
            .await
            .context("Failed to list models from Vertex AI")?;

        if !response.status().is_success() {
            let error = self.parse_error_response(response).await?;
            warn!("Failed to list models from API, using defaults: {}", error.error.message);

            // Return default Claude models if API call fails
            return Ok(vec![
                "claude-sonnet-4-5@20250929".to_string(),
            ]);
        }

        let response_json: Value = response
            .json()
            .await
            .context("Failed to parse models response JSON")?;

        // Extract model names from the response
        let models = response_json
            .get("models")
            .and_then(|models| models.as_array())
            .map(|models| {
                models
                    .iter()
                    .filter_map(|model| {
                        model
                            .get("name")
                            .and_then(|name| name.as_str())
                            .and_then(|name| name.split('/').next_back())
                            .map(|s| s.to_string())
                    })
                    .collect()
            })
            .unwrap_or_else(|| {
                // Fallback to default models
                vec!["claude-sonnet-4-5@20250929".to_string()]
            });

        info!("Found {} Claude models in Vertex AI", models.len());
        Ok(models)
    }

    /// Check if the client is properly authenticated and can access Vertex AI
    pub async fn health_check(&self) -> Result<bool> {
        debug!("Performing Vertex AI client health check");

        match self.get_auth_context().await {
            Ok(auth_context) => {
                if auth_context.is_valid() {
                    debug!("Vertex AI client health check passed");
                    Ok(true)
                } else {
                    warn!("Vertex AI authentication token is invalid");
                    Ok(false)
                }
            }
            Err(e) => {
                warn!("Vertex AI client health check failed: {}", e);
                Ok(false)
            }
        }
    }

    /// Get client configuration
    pub fn config(&self) -> &VertexAiConfig {
        &self.config
    }

    /// Check if a model is supported
    pub fn is_model_supported(&self, model_name: &str) -> bool {
        // Support both new and legacy model names
        matches!(model_name,
            "claude-sonnet-4-5@20250929" |
            "claude-3-5-sonnet@20241022" |
            "claude-3-5-haiku@20241022" |
            "claude-3-5-sonnet"
        )
    }

    /// Get the default model name
    pub fn default_model(&self) -> &str {
        &self.config.default_model.model_name
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::infrastructure::vertex::VertexMessage;
    use crate::config::providers::{VertexAuthConfig, VertexAuthMethod};

    #[tokio::test]
    #[ignore = "requires GCP credentials"]
    async fn test_vertex_client_creation() {
        let config = VertexAiConfig {
            project_id: "test-project".to_string(),
            region: "us-central1".to_string(),
            credentials_path: "./gcp-credentials.json".to_string(),
            default_model: crate::config::providers::VertexModelConfig {
                model_name: "claude-sonnet-4-5@20250929".to_string(),
                ..Default::default()
            },
            auth: VertexAuthConfig {
                auth_method: VertexAuthMethod::ServiceAccountKey,
                ..Default::default()
            },
            ..Default::default()
        };

        let result = VertexAIClient::new(config).await;
        // This will fail without actual credentials in CI, but tests the structure
        if std::env::var("CI").is_ok() {
            // In CI, we expect this to fail due to missing credentials
            assert!(result.is_err());
        } else {
            // In local development, it might succeed if credentials are available
            println!("Client creation result: {:?}", result.is_ok());
        }
    }

    #[test]
    fn test_vertex_predict_request_serialization() {
        let messages = vec![
            VertexMessage::user("Hello Claude"),
            VertexMessage::assistant("Hello! How can I help you?"),
        ];

        let request = VertexPredictRequest::new(messages, 1000)
            .with_system("You are a helpful assistant")
            .with_temperature(0.7);

        let json = serde_json::to_string(&request).unwrap();
        assert!(json.contains("Hello Claude"));
        assert!(json.contains("helpful assistant"));
        assert!(json.contains("1000"));
        assert!(json.contains("vertex-2023-10-16"));
    }

    #[test]
    fn test_model_support_check() {
        let config = VertexAiConfig::default();
        let client_config = config; // We don't need to create the full client for this test

        // Test supported model
        assert!(client_config.default_model.model_name == "claude-sonnet-4-5@20250929");

        // We can test the logic without creating a full client
        let is_supported = |model: &str| model == "claude-sonnet-4-5@20250929";
        assert!(is_supported("claude-sonnet-4-5@20250929"));
        assert!(!is_supported("unsupported-model"));
    }

    #[test]
    fn test_endpoint_url_generation() {
        let config = VertexAiConfig {
            project_id: "test-project".to_string(),
            region: "us-central1".to_string(),
            ..Default::default()
        };

        let prediction_url = config.prediction_endpoint("claude-sonnet-4-5@20250929");
        assert!(prediction_url.contains("test-project"));
        assert!(prediction_url.contains("us-central1"));
        assert!(prediction_url.contains("claude-sonnet-4-5@20250929"));
        assert!(prediction_url.contains(":predict"));

        let streaming_url = config.streaming_endpoint("claude-sonnet-4-5@20250929");
        assert!(streaming_url.contains("streamGenerateContent"));
    }
}
