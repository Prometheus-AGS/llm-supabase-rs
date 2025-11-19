use anyhow::{Context, Result};
use reqwest::Client;
use serde_json::Value;
use std::sync::Arc;
use tokio_stream::Stream;
use tracing::{debug, info, warn, error};
use futures_util::StreamExt;

use crate::models::request::ChatCompletionRequest;
use crate::models::response::{ChatCompletionResponse, ChatCompletionChunk};
use crate::infrastructure::common::tools::{ToolCallManager, UnifiedToolCall, ToolCallResult};

use super::auth::OpenAIAuth;
use crate::config::providers::OpenAIConfig;
use super::streaming::OpenAIStreamParser;

/// OpenAI HTTP client for chat completions with full tool calling support
/// Handles authentication, request/response formatting, streaming, and error handling
pub struct OpenAIClient {
    /// HTTP client for making requests
    client: Client,

    /// OpenAI configuration
    config: OpenAIConfig,

    /// Authentication handler
    auth: Arc<OpenAIAuth>,

    /// Tool call manager for unified tool calling
    tool_manager: Arc<ToolCallManager>,
}

impl OpenAIClient {
    /// Create a new OpenAI client
    pub async fn new(config: OpenAIConfig) -> Result<Self> {
        info!("Initializing OpenAI client");

        // Initialize authentication
        let auth = OpenAIAuth::from_config(&config)?;
        
        // Validate API key format
        auth.validate_api_key()
            .context("Invalid OpenAI API key")?;

        let auth = Arc::new(auth);

        // Configure HTTP client with timeouts
        let client = Client::builder()
            .timeout(std::time::Duration::from_secs(config.timeout.unwrap_or(120)))
            .connect_timeout(std::time::Duration::from_secs(30))
            .build()
            .context("Failed to create HTTP client")?;

        // Initialize tool call manager with OpenAI converter (pass-through)
        let tool_manager = Arc::new(ToolCallManager::openai());

        info!("Successfully initialized OpenAI client with API key: {}", 
              auth.get_masked_api_key());

        Ok(Self {
            client,
            config,
            auth,
            tool_manager,
        })
    }

    /// Create OpenAI client from environment variables
    pub async fn from_env() -> Result<Self> {
        let config = OpenAIConfig::from_env()
            .context("Failed to load OpenAI configuration from environment")?;
        Self::new(config).await
    }

    /// Make a chat completion request (non-streaming)
    pub async fn chat_completion(
        &self,
        request: ChatCompletionRequest,
    ) -> Result<ChatCompletionResponse> {
        debug!("Making OpenAI chat completion request to model: {}", request.model);

        let headers = self.auth.create_headers()?;
        let url = self.config.get_chat_completions_url();

        // Ensure streaming is disabled for non-streaming requests
        let mut request = request;
        request.stream = Some(false);

        let response = self
            .client
            .post(&url)
            .headers(headers)
            .json(&request)
            .send()
            .await
            .context("Failed to send chat completion request")?;

        let status = response.status();
        
        if !status.is_success() {
            let error_text = response.text().await
                .unwrap_or_else(|_| "Unknown error".to_string());
            
            return Err(anyhow::anyhow!(
                "OpenAI API request failed with status {}: {}", 
                status, error_text
            ));
        }

        let response_text = response.text().await
            .context("Failed to read response body")?;

        let chat_response: ChatCompletionResponse = serde_json::from_str(&response_text)
            .context("Failed to parse OpenAI response")?;

        debug!("Successfully received OpenAI chat completion response with {} choices", 
               chat_response.choices.len());

        Ok(chat_response)
    }

    /// Make a streaming chat completion request
    pub async fn chat_completion_stream(
        &self,
        request: ChatCompletionRequest,
    ) -> Result<impl Stream<Item = Result<ChatCompletionChunk>>> {
        debug!("Making OpenAI streaming chat completion request to model: {}", request.model);

        let headers = self.auth.create_headers()?;
        let url = self.config.get_chat_completions_url();

        // Ensure streaming is enabled
        let mut request = request;
        request.stream = Some(true);

        let response = self
            .client
            .post(&url)
            .headers(headers)
            .json(&request)
            .send()
            .await
            .context("Failed to send streaming chat completion request")?;

        let status = response.status();
        
        if !status.is_success() {
            let error_text = response.text().await
                .unwrap_or_else(|_| "Unknown error".to_string());
            
            return Err(anyhow::anyhow!(
                "OpenAI streaming API request failed with status {}: {}", 
                status, error_text
            ));
        }

        // Create stream parser for Server-Sent Events
        let stream_parser = OpenAIStreamParser::new();
        let byte_stream = response.bytes_stream();

        let parsed_stream = stream_parser.parse_stream(byte_stream).await?;

        debug!("Successfully created OpenAI streaming response");
        Ok(parsed_stream)
    }

    /// Extract tool calls from a chat completion response
    pub fn extract_tool_calls(&self, response: &ChatCompletionResponse) -> Result<Vec<UnifiedToolCall>> {
        let mut unified_calls = Vec::new();

        for choice in &response.choices {
            if let Some(ref tool_calls) = choice.message.tool_calls {
                for tool_call_value in tool_calls {
                    // Parse the tool call from the response
                    if let Ok(tool_call) = self.tool_manager.converter.provider_tool_calls_to_unified(tool_call_value) {
                        unified_calls.extend(tool_call);
                    }
                }
            }
        }

        Ok(unified_calls)
    }

    /// Process tool call results and create continuation messages
    pub fn process_tool_results(&self, results: Vec<ToolCallResult>) -> Result<Value> {
        self.tool_manager.converter.tool_results_to_provider_messages(&results)
    }

    /// Test the OpenAI API connection
    pub async fn test_connection(&self) -> Result<bool> {
        debug!("Testing OpenAI API connection");

        // Make a simple completion request to test connectivity
        let test_request = ChatCompletionRequest {
            model: self.config.default_model.clone().unwrap_or_else(|| "gpt-3.5-turbo".to_string()),
            messages: vec![
                crate::models::common::ChatMessage {
                    role: crate::models::common::MessageRole::User,
                    content: "Hello".to_string(),
                    name: None,
                    function_call: None,
                    tool_calls: None,
                    tool_call_id: None,
                }
            ],
            max_tokens: Some(5),
            temperature: Some(0.0),
            ..Default::default()
        };

        match self.chat_completion(test_request).await {
            Ok(_) => {
                info!("OpenAI API connection test successful");
                Ok(true)
            }
            Err(e) => {
                warn!("OpenAI API connection test failed: {}", e);
                Ok(false)
            }
        }
    }

    /// Get available OpenAI models
    pub async fn get_models(&self) -> Result<Vec<String>> {
        debug!("Fetching available OpenAI models");

        let headers = self.auth.create_headers()?;
        let models_url = format!("{}/models", self.config.get_base_url());

        let response = self
            .client
            .get(&models_url)
            .headers(headers)
            .send()
            .await
            .context("Failed to fetch OpenAI models")?;

        if !response.status().is_success() {
            return Err(anyhow::anyhow!(
                "Failed to fetch models: HTTP {}", 
                response.status()
            ));
        }

        let models_response: Value = response.json().await
            .context("Failed to parse models response")?;

        let mut models = Vec::new();
        
        if let Some(data) = models_response.get("data").and_then(|d| d.as_array()) {
            for model in data {
                if let Some(id) = model.get("id").and_then(|i| i.as_str()) {
                    models.push(id.to_string());
                }
            }
        }

        debug!("Found {} available models", models.len());
        Ok(models)
    }

    /// Check if a specific model supports tool calling
    pub fn supports_tool_calling(&self, model: &str) -> bool {
        // OpenAI models that support tool calling
        matches!(model, 
            "gpt-4" | "gpt-4o" | "gpt-4o-mini" | 
            "gpt-4-turbo" | "gpt-4-turbo-preview" |
            "gpt-3.5-turbo" | "gpt-3.5-turbo-0125"
        )
    }

    /// Check if a specific model supports streaming
    pub fn supports_streaming(&self, model: &str) -> bool {
        // Most OpenAI models support streaming
        !model.starts_with("davinci") && !model.starts_with("curie") && 
        !model.starts_with("babbage") && !model.starts_with("ada")
    }

    /// Get the recommended model for tool calling
    pub fn get_recommended_tool_model(&self) -> String {
        self.config.default_model.clone().unwrap_or_else(|| "gpt-4o".to_string())
    }

    /// Handle rate limiting with exponential backoff
    async fn handle_rate_limit(&self, attempt: u32) -> Result<()> {
        let max_retries = self.config.max_retries.unwrap_or(3);
        
        if attempt >= max_retries {
            return Err(anyhow::anyhow!("Maximum retry attempts exceeded"));
        }

        let delay = std::time::Duration::from_millis(1000 * (2_u64.pow(attempt)));
        warn!("Rate limit hit, retrying after {:?} (attempt {})", delay, attempt + 1);
        
        tokio::time::sleep(delay).await;
        Ok(())
    }

    /// Get client configuration
    pub fn get_config(&self) -> &OpenAIConfig {
        &self.config
    }

    /// Get tool call manager
    pub fn get_tool_manager(&self) -> &ToolCallManager {
        &self.tool_manager
    }
}

impl Default for ChatCompletionRequest {
    fn default() -> Self {
        Self {
            model: "gpt-4o".to_string(),
            messages: Vec::new(),
            max_tokens: None,
            temperature: None,
            top_p: None,
            n: None,
            stream: None,
            stream_options: None,
            stop: None,
            presence_penalty: None,
            frequency_penalty: None,
            logit_bias: None,
            logprobs: None,
            top_logprobs: None,
            user: None,
            functions: None,
            function_call: None,
            tools: None,
            tool_choice: None,
            parallel_tool_calls: None,
            response_format: None,
            seed: None,
            service_tier: None,
            metadata: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_openai_client_creation() {
        let config = OpenAIConfig {
            api_key: "sk-test123456789012345678901234567890123456789012345".to_string(),
            ..Default::default()
        };

        let client = OpenAIClient::new(config).await;
        assert!(client.is_ok());
    }

    #[test]
    fn test_tool_calling_support() {
        let config = OpenAIConfig::default();
        let client_result = std::thread::spawn(move || {
            tokio::runtime::Runtime::new().unwrap().block_on(async {
                OpenAIClient::new(config).await
            })
        }).join().unwrap();

        if let Ok(client) = client_result {
            assert!(client.supports_tool_calling("gpt-4o"));
            assert!(client.supports_tool_calling("gpt-3.5-turbo"));
            assert!(!client.supports_tool_calling("davinci-002"));
        }
    }

    #[test]
    fn test_streaming_support() {
        let config = OpenAIConfig::default();
        let client_result = std::thread::spawn(move || {
            tokio::runtime::Runtime::new().unwrap().block_on(async {
                OpenAIClient::new(config).await
            })
        }).join().unwrap();

        if let Ok(client) = client_result {
            assert!(client.supports_streaming("gpt-4o"));
            assert!(client.supports_streaming("gpt-3.5-turbo"));
            assert!(!client.supports_streaming("davinci-002"));
        }
    }
}