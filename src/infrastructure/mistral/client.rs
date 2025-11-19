//! Mistral AI HTTP client for chat completions with full tool calling support
//!
//! This module provides a complete Mistral API client implementation with OpenAI-compatible
//! endpoints, European compliance features, and comprehensive tool calling, streaming, 
//! and error handling capabilities.

use anyhow::{Context, Result};
use reqwest::Client;
use serde_json::Value;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio_stream::Stream;
use tracing::{debug, info, warn, error, trace};
use futures_util::StreamExt;

use crate::models::request::ChatCompletionRequest;
use crate::models::response::{ChatCompletionResponse, ChatCompletionChunk};
use crate::infrastructure::common::tools::{ToolCallManager, UnifiedToolCall, ToolCallResult, ToolCallConverter};

use super::auth::{MistralAuth, RateLimitInfo, ComplianceStats};
use super::types::{MistralConfig, MistralModel, MistralApiError};
use super::streaming::{MistralStreamParser, MistralStreamUtils, StreamingMetrics};
use super::converter::{MistralConverter, ToolCallContext};

/// Mistral HTTP client for chat completions with full tool calling support
/// Handles authentication, request/response formatting, streaming, error handling, and European compliance
pub struct MistralClient {
    /// HTTP client for making requests
    client: Client,

    /// Mistral configuration
    config: MistralConfig,

    /// Authentication handler
    auth: Arc<MistralAuth>,

    /// Tool call manager for unified tool calling
    tool_manager: Arc<ToolCallManager>,
    
    /// Mistral-specific converter for requests/responses
    converter: Arc<MistralConverter>,
    
    /// Performance metrics tracking
    request_count: std::sync::atomic::AtomicU64,
    total_response_time: std::sync::atomic::AtomicU64,
}

impl MistralClient {
    /// Create a new Mistral client
    pub async fn new(config: MistralConfig) -> Result<Self> {
        info!("Initializing Mistral client");

        // Validate configuration
        config.validate()
            .context("Invalid Mistral configuration")?;

        // Initialize authentication
        let auth = MistralAuth::new(&config)?;
        
        // Validate API key format
        auth.validate_api_key()
            .context("Invalid Mistral API key")?;

        let auth = Arc::new(auth);

        // Configure HTTP client with appropriate timeouts for European compliance
        let timeout = config.get_timeout();
        let client = Client::builder()
            .timeout(Duration::from_secs(timeout))
            .connect_timeout(Duration::from_secs(30)) // Longer for EU connections
            .pool_idle_timeout(Duration::from_secs(60))
            .pool_max_idle_per_host(8) // Conservative connection pooling
            .user_agent(config.user_agent.as_deref().unwrap_or("LLM-Supabase-RS/1.0 (Mistral)"))
            .build()
            .context("Failed to create HTTP client")?;

        // Initialize tool call manager with OpenAI-compatible converter (since Mistral uses same format)
        let tool_manager = Arc::new(ToolCallManager::openai());
        
        // Initialize Mistral-specific converter with configuration for compliance
        let converter = Arc::new(MistralConverter::with_config(config.clone()));

        info!("Successfully initialized Mistral client with API key: {} for model: {} (EU residency: {}, GDPR: {})", 
              auth.get_masked_api_key(), 
              config.model,
              config.prefers_eu_residency(),
              config.uses_gdpr_mode());

        Ok(Self {
            client,
            config,
            auth,
            tool_manager,
            converter,
            request_count: std::sync::atomic::AtomicU64::new(0),
            total_response_time: std::sync::atomic::AtomicU64::new(0),
        })
    }

    /// Create client from environment variables
    pub async fn from_env() -> Result<Self> {
        let config = MistralConfig::from_env()
            .context("Failed to load Mistral configuration from environment")?;
        Self::new(config).await
    }

    /// Make a chat completion request
    pub async fn chat_completion(&self, request: ChatCompletionRequest) -> Result<ChatCompletionResponse> {
        let start_time = Instant::now();
        self.request_count.fetch_add(1, std::sync::atomic::Ordering::SeqCst);

        debug!("Starting Mistral chat completion request");

        // Validate request
        self.validate_request(&request).await?;

        // Check rate limits
        let estimated_tokens = self.estimate_request_tokens(&request);
        if !self.auth.check_rate_limit(estimated_tokens).await? {
            return Err(anyhow::anyhow!("Rate limit exceeded"));
        }

        // Prepare the request
        let mut mistral_request = self.convert_request(&request)?;

        // Add European compliance parameters if configured
        self.add_compliance_parameters(&mut mistral_request)?;

        // Create headers
        let headers = self.auth.create_headers()?;

        // Make the API call with retry logic
        let response = self.make_request_with_retries(&mistral_request, &headers).await?;

        // Parse response
        let chat_response = self.parse_response(response).await?;

        // Record request for compliance and rate limiting
        let tokens_used = self.estimate_response_tokens(&chat_response);
        let request_id = self.auth.record_request(&self.config.get_chat_endpoint(), tokens_used).await?;

        // Update metrics
        let response_time = start_time.elapsed().as_millis() as u64;
        self.total_response_time.fetch_add(response_time, std::sync::atomic::Ordering::SeqCst);

        debug!("Completed Mistral chat completion request: {} in {}ms", request_id, response_time);

        Ok(chat_response)
    }

    /// Make a streaming chat completion request
    pub async fn chat_completion_stream(
        &self,
        request: ChatCompletionRequest,
    ) -> Result<impl Stream<Item = Result<ChatCompletionChunk>>> {
        let start_time = Instant::now();
        self.request_count.fetch_add(1, std::sync::atomic::Ordering::SeqCst);

        debug!("Starting Mistral streaming chat completion request");

        // Validate request and model streaming support
        self.validate_request(&request).await?;
        if !self.converter.supports_streaming(&request.model) {
            return Err(anyhow::anyhow!(
                "Model {} does not support streaming", request.model
            ));
        }

        // Check rate limits
        let estimated_tokens = self.estimate_request_tokens(&request);
        if !self.auth.check_rate_limit(estimated_tokens).await? {
            return Err(anyhow::anyhow!("Rate limit exceeded"));
        }

        // Prepare streaming request
        let mut mistral_request = self.convert_request(&request)?;
        mistral_request["stream"] = serde_json::Value::Bool(true);

        // Add European compliance parameters
        self.add_compliance_parameters(&mut mistral_request)?;

        // Create headers
        let headers = self.auth.create_headers()?;

        // Make streaming request
        let response = self.client
            .post(&self.config.get_chat_endpoint())
            .headers(headers)
            .json(&mistral_request)
            .send()
            .await
            .context("Failed to send streaming request to Mistral")?;

        let status = response.status();
        if !status.is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(anyhow::anyhow!(
                "Mistral API error {}: {}", status, error_text
            ));
        }

        // Create stream processor
        let request_id = format!("mistral_stream_{}", uuid::Uuid::new_v4().simple());
        let parser = MistralStreamParser::new(Some(self.config.clone()), request_id.clone());

        // Convert byte stream to chat completion chunks
        let byte_stream = response.bytes_stream();
        let chunk_stream = self.process_stream(byte_stream, parser).await?;

        // Record streaming request
        let _request_id = self.auth.record_request(&self.config.get_chat_endpoint(), estimated_tokens).await?;

        debug!("Started Mistral streaming request: {}", request_id);

        Ok(chunk_stream)
    }

    /// Test connection to Mistral API
    pub async fn test_connection(&self) -> Result<bool> {
        debug!("Testing Mistral API connection");

        let headers = self.auth.create_headers()?;

        let response = self.client
            .get(&self.config.get_models_endpoint())
            .headers(headers)
            .send()
            .await
            .context("Failed to connect to Mistral API")?;

        let success = response.status().is_success();
        
        if success {
            info!("Mistral API connection test successful");
        } else {
            warn!("Mistral API connection test failed: {}", response.status());
        }

        Ok(success)
    }

    /// Get available models from Mistral API
    pub async fn get_models(&self) -> Result<Vec<String>> {
        debug!("Fetching available models from Mistral");

        let headers = self.auth.create_headers()?;

        let response = self.client
            .get(&self.config.get_models_endpoint())
            .headers(headers)
            .send()
            .await
            .context("Failed to fetch models from Mistral")?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(anyhow::anyhow!(
                "Failed to fetch models: {}", error_text
            ));
        }

        let models_response: Value = response.json().await
            .context("Failed to parse models response")?;

        let models: Vec<String> = models_response.get("data")
            .and_then(|d| d.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|model| model.get("id").and_then(|id| id.as_str()))
                    .map(String::from)
                    .collect()
            })
            .unwrap_or_else(|| {
                // Fallback to known models
                warn!("Could not parse models response, using known models");
                MistralModel::get_all_models()
                    .into_iter()
                    .map(|m| m.name)
                    .collect()
            });

        debug!("Retrieved {} models from Mistral", models.len());
        Ok(models)
    }

    /// Check if a model supports tool calling
    pub fn supports_tool_calling(&self, model: &str) -> bool {
        self.converter.supports_tool_calling(model)
    }

    /// Get compliance statistics
    pub async fn get_compliance_stats(&self) -> Result<ComplianceStats> {
        self.auth.get_compliance_stats().await
    }

    /// Get current rate limit status
    pub async fn get_rate_limit_status(&self) -> Option<super::auth::RateLimitStatus> {
        self.auth.get_rate_limit_status().await
    }

    /// Get client performance metrics
    pub fn get_performance_metrics(&self) -> ClientPerformanceMetrics {
        let request_count = self.request_count.load(std::sync::atomic::Ordering::SeqCst);
        let total_response_time = self.total_response_time.load(std::sync::atomic::Ordering::SeqCst);

        ClientPerformanceMetrics {
            total_requests: request_count,
            average_response_time_ms: if request_count > 0 {
                total_response_time as f64 / request_count as f64
            } else {
                0.0
            },
            requests_per_minute: 0.0, // Could be calculated with timestamps
        }
    }

    /// Validate request before sending
    async fn validate_request(&self, request: &ChatCompletionRequest) -> Result<()> {
        if request.messages.is_empty() {
            return Err(anyhow::anyhow!("Request must contain at least one message"));
        }

        if let Some(tools) = &request.tools {
            self.converter.validate_tool_request(tools, &request.model)?;
        }

        // Check if model exists
        if !self.converter.supports_tool_calling(&request.model) && request.tools.is_some() {
            return Err(anyhow::anyhow!(
                "Model {} does not support tool calling", request.model
            ));
        }

        Ok(())
    }

    /// Convert ChatCompletionRequest to Mistral API format
    fn convert_request(&self, request: &ChatCompletionRequest) -> Result<Value> {
        // Mistral uses OpenAI-compatible format, so minimal conversion needed
        let mut mistral_request = serde_json::to_value(request)
            .context("Failed to serialize request")?;

        // Convert tools if present
        if let Some(tools) = &request.tools {
            let mistral_tools = ToolCallConverter::openai_tools_to_provider(&*self.converter, tools)?;
            mistral_request["tools"] = mistral_tools;
        }

        trace!("Converted request to Mistral format");
        Ok(mistral_request)
    }

    /// Add European compliance parameters to request
    fn add_compliance_parameters(&self, request: &mut Value) -> Result<()> {
        if self.config.prefers_eu_residency() {
            // Add EU residency preference (theoretical - Mistral would need to support this)
            if let Some(obj) = request.as_object_mut() {
                obj.insert("eu_residency".to_string(), serde_json::Value::Bool(true));
            }
        }

        if self.config.uses_gdpr_mode() {
            // Add GDPR compliance mode
            if let Some(obj) = request.as_object_mut() {
                obj.insert("gdpr_mode".to_string(), serde_json::Value::Bool(true));
            }
        }

        Ok(())
    }

    /// Make request with retry logic
    async fn make_request_with_retries(
        &self,
        request: &Value,
        headers: &reqwest::header::HeaderMap,
    ) -> Result<reqwest::Response> {
        let max_retries = self.config.get_max_retries();
        let mut attempt = 0;

        loop {
            let response = self.client
                .post(&self.config.get_chat_endpoint())
                .headers(headers.clone())
                .json(request)
                .send()
                .await
                .context("Failed to send request to Mistral")?;

            if response.status().is_success() {
                return Ok(response);
            }

            attempt += 1;
            if attempt >= max_retries {
                let status = response.status();
                let error_text = response.text().await.unwrap_or_default();
                return Err(anyhow::anyhow!(
                    "Mistral API error after {} attempts: {} - {}", 
                    max_retries, status, error_text
                ));
            }

            // Calculate retry delay with exponential backoff
            let delay_ms = if self.config.uses_exponential_backoff() {
                self.config.get_retry_delay_ms() * 2_u64.pow(attempt - 1)
            } else {
                self.config.get_retry_delay_ms()
            };

            warn!("Mistral request failed, retrying in {}ms (attempt {}/{})", 
                  delay_ms, attempt, max_retries);

            tokio::time::sleep(Duration::from_millis(delay_ms)).await;
        }
    }

    /// Parse API response into ChatCompletionResponse
    async fn parse_response(&self, response: reqwest::Response) -> Result<ChatCompletionResponse> {
        let response_text = response.text().await
            .context("Failed to read response body")?;

        let response_value: Value = serde_json::from_str(&response_text)
            .context("Failed to parse response JSON")?;

        // Check for API errors
        if let Some(error) = response_value.get("error") {
            let error_msg = error.get("message")
                .and_then(|m| m.as_str())
                .unwrap_or("Unknown Mistral API error");
            return Err(anyhow::anyhow!("Mistral API error: {}", error_msg));
        }

        let chat_response: ChatCompletionResponse = serde_json::from_value(response_value)
            .context("Failed to parse ChatCompletionResponse")?;

        Ok(chat_response)
    }

    /// Process streaming response
    async fn process_stream(
        &self,
        byte_stream: impl Stream<Item = Result<bytes::Bytes, reqwest::Error>>,
        mut parser: MistralStreamParser,
    ) -> Result<impl Stream<Item = Result<ChatCompletionChunk>>> {
        let processed_stream = byte_stream.map(move |bytes_result| {
            match bytes_result {
                Ok(bytes) => {
                    match parser.parse_chunk(&bytes) {
                        Ok(chunks) => {
                            // Convert Mistral chunks to standard ChatCompletionChunk
                            chunks.into_iter().map(|c| Ok(c.chunk)).collect::<Vec<_>>()
                        }
                        Err(e) => vec![Err(e)]
                    }
                }
                Err(e) => vec![Err(anyhow::anyhow!("Stream error: {}", e))]
            }
        })
        .flat_map(futures_util::stream::iter);

        Ok(processed_stream)
    }

    /// Estimate token usage for request
    fn estimate_request_tokens(&self, request: &ChatCompletionRequest) -> u32 {
        // Rough estimation: 4 characters per token
        let message_tokens: u32 = request.messages.iter().map(|msg| {
            (msg.content.len() / 4) as u32
        }).sum();

        let tool_tokens = request.tools.as_ref().map(|tools| {
            tools.iter().map(|tool| {
                let name_tokens = (tool.function.name.len() / 4) as u32;
                let desc_tokens = tool.function.description.as_ref()
                    .map_or(0, |d| (d.len() / 4) as u32);
                name_tokens + desc_tokens + 50 // Add for parameters
            }).sum::<u32>()
        }).unwrap_or(0);

        message_tokens + tool_tokens
    }

    /// Estimate token usage for response
    fn estimate_response_tokens(&self, response: &ChatCompletionResponse) -> u32 {
        response.choices.iter().map(|choice| {
            (choice.message.content.len() / 4) as u32
        }).sum()
    }
}

/// Client performance metrics
#[derive(Debug, Clone)]
pub struct ClientPerformanceMetrics {
    pub total_requests: u64,
    pub average_response_time_ms: f64,
    pub requests_per_minute: f64,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::common::{ChatMessage, MessageRole};

    #[tokio::test]
    async fn test_client_creation() {
        let config = MistralConfig {
            api_key: "test-api-key-123456".to_string(),
            ..Default::default()
        };

        let client = MistralClient::new(config).await;
        assert!(client.is_ok());
    }

    #[tokio::test]
    async fn test_request_validation() {
        let config = MistralConfig {
            api_key: "test-api-key-123456".to_string(),
            ..Default::default()
        };

        let client = MistralClient::new(config).await.unwrap();
        
        // Empty messages should fail
        let empty_request = ChatCompletionRequest {
            model: "mistral-large-latest".to_string(),
            messages: vec![],
            ..Default::default()
        };

        let result = client.validate_request(&empty_request).await;
        assert!(result.is_err());

        // Valid request should pass
        let valid_request = ChatCompletionRequest {
            model: "mistral-large-latest".to_string(),
            messages: vec![ChatMessage {
                role: MessageRole::User,
                content: "Hello".to_string(),
                ..Default::default()
            }],
            ..Default::default()
        };

        let result = client.validate_request(&valid_request).await;
        assert!(result.is_ok());
    }

    #[test]
    fn test_token_estimation() {
        let config = MistralConfig {
            api_key: "test-api-key-123456".to_string(),
            ..Default::default()
        };

        // Create client synchronously for test
        let client = tokio::runtime::Runtime::new().unwrap().block_on(async {
            MistralClient::new(config).await.unwrap()
        });

        let request = ChatCompletionRequest {
            model: "mistral-large-latest".to_string(),
            messages: vec![ChatMessage {
                role: MessageRole::User,
                content: "This is a test message".to_string(),
                ..Default::default()
            }],
            ..Default::default()
        };

        let tokens = client.estimate_request_tokens(&request);
        assert!(tokens > 0);
    }

    #[test]
    fn test_tool_calling_support() {
        let config = MistralConfig {
            api_key: "test-api-key-123456".to_string(),
            ..Default::default()
        };

        let client = tokio::runtime::Runtime::new().unwrap().block_on(async {
            MistralClient::new(config).await.unwrap()
        });

        assert!(client.supports_tool_calling("mistral-large-latest"));
        assert!(client.supports_tool_calling("codestral-latest"));
        assert!(!client.supports_tool_calling("mistral-tiny"));
    }

    #[test]
    fn test_performance_metrics() {
        let config = MistralConfig {
            api_key: "test-api-key-123456".to_string(),
            ..Default::default()
        };

        let client = tokio::runtime::Runtime::new().unwrap().block_on(async {
            MistralClient::new(config).await.unwrap()
        });

        let metrics = client.get_performance_metrics();
        assert_eq!(metrics.total_requests, 0);
        assert_eq!(metrics.average_response_time_ms, 0.0);
    }
}
