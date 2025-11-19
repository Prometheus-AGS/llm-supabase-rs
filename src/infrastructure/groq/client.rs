//! Groq HTTP client for chat completions with full tool calling support
//!
//! This module provides a complete Groq API client implementation with OpenAI-compatible
//! endpoints, optimized for Groq's ultra-fast inference speeds and including comprehensive
//! tool calling, streaming, and error handling capabilities.

use anyhow::{Context, Result};
use futures_util::StreamExt;
use reqwest::Client;
use serde_json::Value;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio_stream::Stream;
use tracing::{debug, error, info, trace, warn};

use crate::infrastructure::common::tools::{ToolCallManager, ToolCallResult, UnifiedToolCall};
use crate::models::request::ChatCompletionRequest;
use crate::models::response::{ChatCompletionChunk, ChatCompletionResponse};

use super::auth::{GroqAuth, RateLimitInfo};
use super::converter::GroqConverter;
use super::streaming::{GroqStreamParser, GroqStreamUtils, StreamingMetrics};
use super::types::{GroqConfig, GroqModel};

/// Groq HTTP client for chat completions with full tool calling support
/// Handles authentication, request/response formatting, streaming, error handling, and rate limiting
pub struct GroqClient {
    /// HTTP client for making requests
    client: Client,

    /// Groq configuration
    config: GroqConfig,

    /// Authentication handler
    auth: Arc<GroqAuth>,

    /// Tool call manager for unified tool calling
    tool_manager: Arc<ToolCallManager>,

    /// Groq-specific converter for requests/responses
    converter: Arc<GroqConverter>,

    /// Rate limiting tracker
    rate_limit_info: Option<RateLimitInfo>,

    /// Performance metrics tracking
    request_count: std::sync::atomic::AtomicU64,
    total_response_time: std::sync::atomic::AtomicU64,
}

impl GroqClient {
    /// Create a new Groq client
    pub async fn new(config: GroqConfig) -> Result<Self> {
        info!("Initializing Groq client");

        // Validate configuration
        config.validate().context("Invalid Groq configuration")?;

        // Initialize authentication
        let auth = GroqAuth::new(&config)?;

        // Validate API key format
        auth.validate_api_key().context("Invalid Groq API key")?;

        let auth = Arc::new(auth);

        // Configure HTTP client with optimized timeouts for Groq's speed
        let timeout = config.timeout.unwrap_or(60); // Groq is fast, shorter default timeout
        let client = Client::builder()
            .timeout(Duration::from_secs(timeout))
            .connect_timeout(Duration::from_secs(10)) // Fast connection for speed
            .pool_idle_timeout(Duration::from_secs(30))
            .pool_max_idle_per_host(10) // Higher connection pooling for performance
            .user_agent(
                config
                    .user_agent
                    .as_deref()
                    .unwrap_or("LLM-Supabase-RS/1.0 (Groq)"),
            )
            .build()
            .context("Failed to create HTTP client")?;

        // Initialize tool call manager with OpenAI-compatible converter (since Groq uses same format)
        let tool_manager = Arc::new(ToolCallManager::openai());

        // Initialize Groq-specific converter
        let converter = Arc::new(GroqConverter::new());

        info!(
            "Successfully initialized Groq client with API key: {} for model: {}",
            auth.get_masked_api_key(),
            config.model
        );

        Ok(Self {
            client,
            config,
            auth,
            tool_manager,
            converter,
            rate_limit_info: None,
            request_count: std::sync::atomic::AtomicU64::new(0),
            total_response_time: std::sync::atomic::AtomicU64::new(0),
        })
    }

    /// Create Groq client from environment variables
    pub async fn from_env() -> Result<Self> {
        let config =
            GroqConfig::from_env().context("Failed to load Groq configuration from environment")?;
        Self::new(config).await
    }

    /// Make a chat completion request (non-streaming)
    pub async fn chat_completion(
        &self,
        request: ChatCompletionRequest,
    ) -> Result<ChatCompletionResponse> {
        let start_time = Instant::now();
        debug!(
            "Making Groq chat completion request to model: {}",
            request.model
        );

        // Validate model supports tool calling if tools are provided
        if request.tools.is_some() {
            self.converter.validate_model_for_tools(&request.model)?;
        }

        // Validate and optimize request for Groq
        let mut request = request;
        self.converter.validate_tool_request(&request)?;
        let model = request.model.clone();
        self.converter.optimize_for_groq(&mut request, &model)?;

        // Check rate limits before making request
        self.check_rate_limits().await?;

        let headers = self.auth.create_headers()?;
        let url = self.config.get_chat_completions_url();

        // Ensure streaming is disabled for non-streaming requests
        request.stream = Some(false);

        trace!("Sending request to: {}", url);
        let response = self
            .client
            .post(&url)
            .headers(headers)
            .json(&request)
            .send()
            .await
            .context("Failed to send chat completion request to Groq")?;

        // Extract rate limit information from headers
        if let Some(rate_limit) =
            GroqStreamUtils::extract_rate_limit_from_headers(response.headers())
        {
            debug!(
                "Groq rate limit: {}/{} remaining, resets in {}s",
                rate_limit.remaining,
                rate_limit.limit,
                rate_limit.seconds_until_reset()
            );
        }

        let status = response.status();

        if !status.is_success() {
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());

            return Err(anyhow::anyhow!(
                "Groq API request failed with status {}: {}",
                status,
                error_text
            ));
        }

        let response_text = response
            .text()
            .await
            .context("Failed to read Groq response body")?;

        let chat_response: ChatCompletionResponse =
            serde_json::from_str(&response_text).context("Failed to parse Groq response")?;

        let elapsed = start_time.elapsed();
        self.update_performance_metrics(elapsed);

        debug!(
            "Successfully received Groq chat completion response with {} choices in {:?}",
            chat_response.choices.len(),
            elapsed
        );

        Ok(chat_response)
    }

    /// Make a streaming chat completion request
    pub async fn chat_completion_stream(
        &self,
        request: ChatCompletionRequest,
    ) -> Result<impl Stream<Item = Result<ChatCompletionChunk>>> {
        debug!(
            "Making Groq streaming chat completion request to model: {}",
            request.model
        );

        // Validate model supports streaming
        if !GroqModel::get_model(&request.model)
            .map(|m| m.supports_streaming)
            .unwrap_or(false)
        {
            return Err(anyhow::anyhow!(
                "Model '{}' does not support streaming",
                request.model
            ));
        }

        // Validate model supports tool calling if tools are provided
        if request.tools.is_some() {
            self.converter.validate_model_for_tools(&request.model)?;
        }

        // Validate and optimize request for Groq
        let mut request = request;
        let model = request.model.clone();
        self.converter.validate_tool_request(&request)?;
        self.converter
            .optimize_for_groq(&mut request, model.as_str())?;

        // Check rate limits before making request
        self.check_rate_limits().await?;

        let headers = self.auth.create_streaming_headers()?;
        let url = self.config.get_chat_completions_url();

        // Ensure streaming is enabled
        request.stream = Some(true);

        trace!("Sending streaming request to: {}", url);
        let response = self
            .client
            .post(&url)
            .headers(headers)
            .json(&request)
            .send()
            .await
            .context("Failed to send streaming chat completion request to Groq")?;

        let status = response.status();

        if !status.is_success() {
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());

            return Err(anyhow::anyhow!(
                "Groq streaming API request failed with status {}: {}",
                status,
                error_text
            ));
        }

        // Create Groq stream parser with model-specific optimizations
        let parsed_stream = GroqStreamUtils::parse_response_stream(response, request.model).await?;

        debug!("Successfully created Groq streaming response");
        Ok(parsed_stream)
    }

    /// Extract tool calls from a chat completion response
    pub fn extract_tool_calls(
        &self,
        response: &ChatCompletionResponse,
    ) -> Result<Vec<UnifiedToolCall>> {
        self.converter.extract_tool_calls(response)
    }

    /// Process tool call results and create continuation messages
    pub fn process_tool_results(
        &self,
        results: Vec<ToolCallResult>,
    ) -> Result<Vec<crate::models::common::ChatMessage>> {
        self.converter.convert_tool_results(&results)
    }

    /// Test the Groq API connection
    pub async fn test_connection(&self) -> Result<bool> {
        debug!("Testing Groq API connection");

        // Use a fast model for connection testing
        let test_model = "llama-3.1-8b-instant";

        // Make a simple completion request to test connectivity
        let test_request = ChatCompletionRequest {
            model: test_model.to_string(),
            messages: vec![crate::models::common::ChatMessage {
                role: crate::models::common::MessageRole::User,
                content: "Hi".to_string(),
                name: None,
                function_call: None,
                tool_calls: None,
                tool_call_id: None,
            }],
            max_tokens: Some(1),
            temperature: Some(0.0),
            ..Default::default()
        };

        match self.chat_completion(test_request).await {
            Ok(_) => {
                info!("Groq API connection test successful");
                Ok(true)
            }
            Err(e) => {
                warn!("Groq API connection test failed: {}", e);
                Ok(false)
            }
        }
    }

    /// Get available Groq models
    pub async fn get_models(&self) -> Result<Vec<String>> {
        debug!("Fetching available Groq models");

        let headers = self.auth.create_headers()?;
        let models_url = self.config.get_models_url();

        let response = self
            .client
            .get(&models_url)
            .headers(headers)
            .timeout(Duration::from_secs(10)) // Quick timeout for model listing
            .send()
            .await
            .context("Failed to fetch Groq models")?;

        if !response.status().is_success() {
            // Fallback to known models if API call fails
            warn!("Failed to fetch models from Groq API, using known models");
            return Ok(GroqModel::get_supported_models()
                .into_iter()
                .map(|model| model.name)
                .collect());
        }

        let models_response: Value = response
            .json()
            .await
            .context("Failed to parse Groq models response")?;

        let mut models = Vec::new();

        if let Some(data) = models_response.get("data").and_then(|d| d.as_array()) {
            for model in data {
                if let Some(model_id) = model.get("id").and_then(|id| id.as_str()) {
                    models.push(model_id.to_string());
                }
            }
        }

        // If no models found in API response, use known models
        if models.is_empty() {
            warn!("No models found in Groq API response, using known models");
            models = GroqModel::get_supported_models()
                .into_iter()
                .map(|model| model.name)
                .collect();
        }

        debug!("Found {} Groq models", models.len());
        Ok(models)
    }

    /// Check if a specific model supports tool calling
    pub fn supports_tool_calling(&self, model: &str) -> bool {
        GroqModel::supports_tool_calling(model)
    }

    /// Get model capabilities
    pub fn get_model_capabilities(&self, model: &str) -> super::converter::ModelCapabilities {
        self.converter.get_model_capabilities(model)
    }

    /// Check rate limits and handle throttling
    async fn check_rate_limits(&self) -> Result<()> {
        if let Some(ref rate_limit) = self.rate_limit_info {
            if rate_limit.remaining == 0 {
                let wait_time = rate_limit.seconds_until_reset();
                if wait_time > 0 {
                    warn!("Groq rate limit exceeded, waiting {}s", wait_time);
                    tokio::time::sleep(Duration::from_secs(wait_time.min(60))).await;
                }
            } else if rate_limit.is_approaching_limit(0.9) {
                // Slow down when approaching rate limit
                debug!("Approaching Groq rate limit, adding small delay");
                tokio::time::sleep(Duration::from_millis(100)).await;
            }
        }
        Ok(())
    }

    /// Update performance metrics
    fn update_performance_metrics(&self, elapsed: Duration) {
        use std::sync::atomic::Ordering;

        self.request_count.fetch_add(1, Ordering::Relaxed);
        self.total_response_time
            .fetch_add(elapsed.as_millis() as u64, Ordering::Relaxed);
    }

    /// Get performance statistics
    pub fn get_performance_stats(&self) -> PerformanceStats {
        use std::sync::atomic::Ordering;

        let request_count = self.request_count.load(Ordering::Relaxed);
        let total_time = self.total_response_time.load(Ordering::Relaxed);

        let average_response_time = if request_count > 0 {
            Duration::from_millis(total_time / request_count)
        } else {
            Duration::from_millis(0)
        };

        PerformanceStats {
            total_requests: request_count,
            average_response_time,
            total_response_time: Duration::from_millis(total_time),
        }
    }

    /// Create a continuation request with tool results
    pub fn create_continuation_request(
        &self,
        original_request: &ChatCompletionRequest,
        tool_results: Vec<ToolCallResult>,
    ) -> Result<ChatCompletionRequest> {
        let mut messages = original_request.messages.clone();

        // Add tool result messages
        let result_messages = self.converter.convert_tool_results(&tool_results)?;
        messages.extend(result_messages);

        let mut request = ChatCompletionRequest {
            model: original_request.model.clone(),
            messages,
            tools: original_request.tools.clone(),
            tool_choice: original_request.tool_choice.clone(),
            ..original_request.clone()
        };

        // Optimize for Groq
        let model = request.model.clone();
        self.converter.optimize_for_groq(&mut request, &model)?;

        Ok(request)
    }

    /// Get configuration
    pub fn config(&self) -> &GroqConfig {
        &self.config
    }

    /// Get auth handler
    pub fn auth(&self) -> &GroqAuth {
        &self.auth
    }

    /// Get converter
    pub fn converter(&self) -> &GroqConverter {
        &self.converter
    }

    /// Check if client is properly configured
    pub fn is_configured(&self) -> bool {
        self.auth.is_configured() && !self.config.model.is_empty()
    }

    /// Get recommended model for a specific use case
    pub fn get_recommended_model(&self, use_case: &str) -> Option<String> {
        GroqModel::get_recommended_for_use_case(use_case).map(|model| model.name)
    }

    /// Estimate request cost (Groq is typically very cost-effective)
    pub fn estimate_request_cost(&self, request: &ChatCompletionRequest) -> f64 {
        // Rough token estimation
        let estimated_tokens: usize = request
            .messages
            .iter()
            .map(|msg| msg.content.len() / 4) // Rough chars to tokens
            .sum();

        // Groq pricing is very competitive, rough estimates
        let cost_per_1k_tokens = match request.model.as_str() {
            "llama-3.1-8b-instant" => 0.05,
            "llama-3.1-70b-versatile" => 0.27,
            "mixtral-8x7b-32768" => 0.24,
            "gemma2-9b-it" => 0.10,
            _ => 0.20, // Default estimate
        };

        (estimated_tokens as f64 / 1000.0) * cost_per_1k_tokens
    }
}

/// Performance statistics for Groq client
#[derive(Debug, Clone)]
pub struct PerformanceStats {
    pub total_requests: u64,
    pub average_response_time: Duration,
    pub total_response_time: Duration,
}

impl PerformanceStats {
    /// Check if performance meets Groq's expected speeds
    pub fn is_performing_well(&self) -> bool {
        // Groq should be very fast - expect sub-second average response times
        self.average_response_time < Duration::from_millis(1000)
    }

    /// Get requests per second based on average response time
    pub fn get_requests_per_second(&self) -> f64 {
        if self.average_response_time.as_millis() > 0 {
            1000.0 / self.average_response_time.as_millis() as f64
        } else {
            0.0
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::common::{ChatMessage, MessageRole};

    #[tokio::test]
    async fn test_client_creation() {
        let config = GroqConfig {
            api_key: "gsk_test123456789012345678901234567890123456789012345".to_string(),
            ..Default::default()
        };

        let client = GroqClient::new(config).await;
        assert!(client.is_ok());

        let client = client.unwrap();
        assert!(client.is_configured());
        assert_eq!(client.config().model, "llama-3.1-70b-versatile");
    }

    #[test]
    fn test_model_support_checking() {
        let config = GroqConfig {
            api_key: "gsk_test123456789012345678901234567890123456789012345".to_string(),
            ..Default::default()
        };

        // We can test this without async since it doesn't require network
        let rt = tokio::runtime::Runtime::new().unwrap();
        let client = rt.block_on(GroqClient::new(config)).unwrap();

        assert!(client.supports_tool_calling("llama-3.1-70b-versatile"));
        assert!(client.supports_tool_calling("mixtral-8x7b-32768"));
        assert!(!client.supports_tool_calling("llama3-70b-8192"));
    }

    #[test]
    fn test_performance_stats() {
        let stats = PerformanceStats {
            total_requests: 100,
            average_response_time: Duration::from_millis(500),
            total_response_time: Duration::from_secs(50),
        };

        assert!(stats.is_performing_well());
        assert_eq!(stats.get_requests_per_second(), 2.0);
    }

    #[test]
    fn test_cost_estimation() {
        let config = GroqConfig {
            api_key: "gsk_test123456789012345678901234567890123456789012345".to_string(),
            ..Default::default()
        };

        let rt = tokio::runtime::Runtime::new().unwrap();
        let client = rt.block_on(GroqClient::new(config)).unwrap();

        let request = ChatCompletionRequest {
            model: "llama-3.1-8b-instant".to_string(),
            messages: vec![ChatMessage {
                role: MessageRole::User,
                content: "Hello world".to_string(),
                name: None,
                function_call: None,
                tool_calls: None,
                tool_call_id: None,
            }],
            ..Default::default()
        };

        let cost = client.estimate_request_cost(&request);
        assert!(cost > 0.0);
        assert!(cost < 1.0); // Should be very affordable
    }

    #[test]
    fn test_model_recommendations() {
        let config = GroqConfig {
            api_key: "gsk_test123456789012345678901234567890123456789012345".to_string(),
            ..Default::default()
        };

        let rt = tokio::runtime::Runtime::new().unwrap();
        let client = rt.block_on(GroqClient::new(config)).unwrap();

        let tool_model = client.get_recommended_model("tool_calling");
        assert!(tool_model.is_some());
        assert_eq!(tool_model.unwrap(), "llama-3.1-70b-versatile");

        let speed_model = client.get_recommended_model("speed");
        assert!(speed_model.is_some());
        assert_eq!(speed_model.unwrap(), "llama-3.1-8b-instant");
    }
}
