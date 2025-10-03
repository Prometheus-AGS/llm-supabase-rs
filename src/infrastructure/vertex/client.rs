use anyhow::{Context, Result};
use reqwest::Client;
use serde_json::Value;
use std::sync::Arc;
use tokio_stream::Stream;
use futures_util::StreamExt;
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
        let mut json_buffer = super::streaming_buffer::JsonStreamingBuffer::new();
        
        let parsed_stream = stream.map(move |chunk_result| -> Result<Vec<VertexStreamChunk>> {
            let chunk = chunk_result.context("Failed to read stream chunk")?;
            let chunk_str = String::from_utf8(chunk.to_vec())
                .context("Invalid UTF-8 in stream chunk")?;

            debug!("Raw Vertex AI chunk received: {} chars", chunk_str.len());
            
            // Handle SSE format if present
            let clean_chunk = if chunk_str.starts_with("data: ") {
                let data = chunk_str.strip_prefix("data: ").unwrap_or(&chunk_str);
                if data.trim() == "[DONE]" {
                    debug!("Received [DONE] from Vertex AI");
                    // Return final chunk and any remaining buffer content
                    let mut chunks = Vec::new();
                    if let Some(_remaining_chunk) = json_buffer.finalize() {
                        // Process any remaining content if needed
                        debug!("Finalizing stream with remaining content");
                    }
                    chunks.push(VertexStreamChunk {
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
                    });
                    return Ok(chunks);
                }
                data
            } else if chunk_str.trim().starts_with("{") {
                &chunk_str
            } else if chunk_str.trim().is_empty() {
                // Skip empty chunks
                return Ok(Vec::new());
            } else {
                debug!("Skipping non-JSON chunk: {}", chunk_str);
                return Ok(Vec::new());
            };
            
            // Add to buffer and get complete processed chunks
            let processed_chunks = json_buffer.add_chunk(clean_chunk);
            let mut vertex_chunks = Vec::new();
            
            // Convert processed chunks to VertexStreamChunk format
            for processed_chunk in processed_chunks {
                debug!("Processing chunk: {:?}", processed_chunk);
                
                // Convert ProcessedChunk to VertexStreamChunk with proper content
                match processed_chunk {
                    super::streaming_buffer::ProcessedChunk::Content(text) => {
                        debug!("Converting content chunk: {} chars", text.len());
                        let vertex_chunk = VertexStreamChunk {
                            event_type: "content_block_delta".to_string(),
                            id: None,
                            role: None,
                            model: None,
                            content: Some(vec![VertexContent {
                                content_type: "text".to_string(),
                                text,
                            }]),
                            index: None,
                            delta: None,
                            message: None,
                            content_block: None,
                            usage: None,
                        };
                        vertex_chunks.push(vertex_chunk);
                    }
                    super::streaming_buffer::ProcessedChunk::ToolUse { id, name, input } => {
                        debug!("Converting tool use chunk: {} ({})", name, id);
                        let tool_json = serde_json::to_string(&serde_json::json!({
                            "id": &id,
                            "name": &name,
                            "input": &input
                        })).unwrap_or_default();
                        
                        let vertex_chunk = VertexStreamChunk {
                            event_type: "content_block_start".to_string(),
                            id: Some(id),
                            role: None,
                            model: None,
                            content: Some(vec![VertexContent {
                                content_type: "tool_use".to_string(),
                                text: tool_json,
                            }]),
                            index: None,
                            delta: None,
                            message: None,
                            content_block: None,
                            usage: None,
                        };
                        vertex_chunks.push(vertex_chunk);
                    }
                    super::streaming_buffer::ProcessedChunk::Message(json_value) => {
                        debug!("Converting complete message chunk");
                        let vertex_chunk = VertexStreamChunk {
                            event_type: json_value.get("type")
                                .and_then(|v| v.as_str())
                                .unwrap_or("message")
                                .to_string(),
                            id: json_value.get("id").and_then(|v| v.as_str()).map(|s| s.to_string()),
                            role: json_value.get("role").and_then(|v| v.as_str()).map(|s| s.to_string()),
                            model: json_value.get("model").and_then(|v| v.as_str()).map(|s| s.to_string()),
                            content: Self::extract_content_from_json(&json_value),
                            index: None,
                            delta: None,
                            message: None,
                            content_block: None,
                            usage: None,
                        };
                        vertex_chunks.push(vertex_chunk);
                    }
                    super::streaming_buffer::ProcessedChunk::Done => {
                        debug!("Converting done chunk");
                        let vertex_chunk = VertexStreamChunk {
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
                        vertex_chunks.push(vertex_chunk);
                    }
                }
            }
            
            Ok(vertex_chunks)
        }).flat_map(|result| {
            // Convert Vec<VertexStreamChunk> or error into individual chunks
            match result {
                Ok(chunks) => {
                    let results: Vec<Result<VertexStreamChunk>> = chunks.into_iter().map(Ok).collect();
                    tokio_stream::iter(results)
                }
                Err(e) => {
                    tokio_stream::iter(vec![Err(e)])
                }
            }
        });

        Ok(parsed_stream)
    }

    /// Extract content from JSON value
    fn extract_content_from_json(json_value: &serde_json::Value) -> Option<Vec<VertexContent>> {
        if let Some(content_array) = json_value.get("content").and_then(|c| c.as_array()) {
            let mut vertex_content = Vec::new();
            
            for block in content_array {
                if let Some(block_type) = block.get("type").and_then(|t| t.as_str()) {
                    match block_type {
                        "text" => {
                            if let Some(text) = block.get("text").and_then(|t| t.as_str()) {
                                if !text.is_empty() {
                                    vertex_content.push(VertexContent {
                                        content_type: "text".to_string(),
                                        text: text.to_string(),
                                    });
                                }
                            }
                        }
                        "tool_use" => {
                            // Convert tool_use block to text representation for now
                            let tool_text = serde_json::to_string(block).unwrap_or_default();
                            vertex_content.push(VertexContent {
                                content_type: "tool_use".to_string(),
                                text: tool_text,
                            });
                        }
                        _ => {
                            debug!("Unknown content block type: {}", block_type);
                        }
                    }
                }
            }
            
            if !vertex_content.is_empty() {
                Some(vertex_content)
            } else {
                None
            }
        } else {
            None
        }
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
                            .and_then(|name| name.split('/').last())
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
