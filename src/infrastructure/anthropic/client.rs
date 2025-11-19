//! Anthropic HTTP client implementation for chat completions with full tool calling support
//! 
//! This module provides the main client for interacting with Anthropic's Messages API,
//! handling authentication, request/response formatting, streaming, and error handling.

use anyhow::{anyhow, Context, Result};
use futures_util::StreamExt;
use reqwest::Client;
use std::sync::Arc;
use tokio_stream::Stream;
use tracing::{debug, info, warn};

use crate::infrastructure::common::tools::{ToolCallManager, UnifiedToolCall, ToolCallResult};
use crate::models::common::MessageRole;
use crate::models::request::ChatCompletionRequest;
use crate::models::response::{ChatCompletionResponse, ChatCompletionChunk};
use crate::shared::types::{ToolCall, FunctionCall};

use super::auth::AnthropicAuth;
use super::converter::{AnthropicConverter, AnthropicToolCallContext};
use super::streaming::AnthropicStreamUtils;
use super::types::{
    AnthropicConfig, AnthropicMessagesRequest, AnthropicMessagesResponse, 
    AnthropicErrorResponse
};

/// Anthropic HTTP client for chat completions with full tool calling support
/// Handles authentication, request/response formatting, streaming, and error handling
pub struct AnthropicClient {
    /// HTTP client for making requests
    client: Client,

    /// Anthropic configuration
    config: AnthropicConfig,

    /// Authentication handler
    auth: Arc<AnthropicAuth>,

    /// Tool call manager for unified tool calling
    tool_manager: Arc<ToolCallManager>,

    /// Format converter for request/response transformation
    converter: Arc<AnthropicConverter>,
}

impl AnthropicClient {
    /// Create a new Anthropic client
    pub async fn new(config: AnthropicConfig) -> Result<Self> {
        info!("Initializing Anthropic client");

        // Validate configuration
        config.validate()
            .context("Invalid Anthropic configuration")?;

        // Initialize authentication
        let auth = AnthropicAuth::new(&config)?;
        
        // Validate API key format
        auth.validate_api_key()
            .context("Invalid Anthropic API key")?;

        let auth = Arc::new(auth);

        // Configure HTTP client with timeouts
        let client = Client::builder()
            .timeout(std::time::Duration::from_secs(config.timeout.unwrap_or(120)))
            .connect_timeout(std::time::Duration::from_secs(30))
            .build()
            .context("Failed to create HTTP client")?;

        // Initialize tool call manager with Claude converter
        let tool_manager = Arc::new(ToolCallManager::claude());

        // Initialize converter
        let converter = Arc::new(AnthropicConverter::new());

        info!("Successfully initialized Anthropic client with API key: {}", 
              auth.get_masked_api_key());

        Ok(Self {
            client,
            config,
            auth,
            tool_manager,
            converter,
        })
    }

    /// Create Anthropic client from environment variables
    pub async fn from_env() -> Result<Self> {
        let config = AnthropicConfig::from_env()
            .context("Failed to load Anthropic configuration from environment")?;
        Self::new(config).await
    }

    /// Make a chat completion request (non-streaming)
    pub async fn chat_completion(
        &self,
        request: ChatCompletionRequest,
    ) -> Result<ChatCompletionResponse> {
        debug!("Making Anthropic chat completion request to model: {}", request.model);

        // Convert OpenAI request to Anthropic format
        let anthropic_request = self.converter.openai_request_to_anthropic(&request)
            .context("Failed to convert OpenAI request to Anthropic format")?;

        // Ensure streaming is disabled for non-streaming requests
        let mut anthropic_request = anthropic_request;
        anthropic_request.stream = Some(false);

        let headers = self.auth.create_headers()?;
        let url = self.config.get_messages_url();

        debug!("Sending request to: {}", url);
        debug!("Request payload: {}", serde_json::to_string_pretty(&anthropic_request)
            .unwrap_or_else(|_| "Failed to serialize".to_string()));

        let response = self
            .client
            .post(&url)
            .headers(headers)
            .json(&anthropic_request)
            .send()
            .await
            .context("Failed to send chat completion request")?;

        let status = response.status();
        
        if !status.is_success() {
            let error_text = response.text().await
                .unwrap_or_else(|_| "Unknown error".to_string());
            
            debug!("Error response: {}", error_text);

            // Try to parse as Anthropic error
            if let Ok(error_response) = serde_json::from_str::<AnthropicErrorResponse>(&error_text) {
                return Err(anyhow!(
                    "Anthropic API error ({}): {} - {}",
                    status,
                    error_response.error.error_type,
                    error_response.error.message
                ));
            } else {
                return Err(anyhow!("Anthropic API error ({}): {}", status, error_text));
            }
        }

        let response_text = response.text().await
            .context("Failed to read response body")?;

        debug!("Response: {}", response_text);

        let anthropic_response: AnthropicMessagesResponse = serde_json::from_str(&response_text)
            .context("Failed to parse Anthropic response")?;

        // Create conversion context
        let context = AnthropicToolCallContext {
            conversation_id: None, // Could be enhanced to track conversations
            request_id: anthropic_response.id.clone(),
            model: anthropic_response.model.clone(),
            streaming: false,
            metadata: std::collections::HashMap::new(),
        };

        // Convert Anthropic response to OpenAI format
        let openai_response = self.converter.anthropic_response_to_openai(&anthropic_response, &context)
            .context("Failed to convert Anthropic response to OpenAI format")?;

        info!("Successfully completed chat completion request. Tokens used: {} input, {} output",
              anthropic_response.usage.input_tokens, anthropic_response.usage.output_tokens);

        Ok(openai_response)
    }

    /// Make a streaming chat completion request
    pub async fn chat_completion_stream(
        &self,
        request: ChatCompletionRequest,
    ) -> Result<impl Stream<Item = Result<ChatCompletionChunk>>> {
        debug!("Making Anthropic streaming chat completion request to model: {}", request.model);

        // Convert OpenAI request to Anthropic format
        let anthropic_request = self.converter.openai_request_to_anthropic(&request)
            .context("Failed to convert OpenAI request to Anthropic format")?;

        // Ensure streaming is enabled
        let mut anthropic_request = anthropic_request;
        anthropic_request.stream = Some(true);

        let headers = self.auth.create_streaming_headers()?;
        let url = self.config.get_messages_url();

        debug!("Sending streaming request to: {}", url);

        let response = self
            .client
            .post(&url)
            .headers(headers)
            .json(&anthropic_request)
            .send()
            .await
            .context("Failed to send streaming chat completion request")?;

        let status = response.status();
        
        if !status.is_success() {
            let error_text = response.text().await
                .unwrap_or_else(|_| "Unknown error".to_string());
            
            // Try to parse as Anthropic error
            if let Ok(error_response) = serde_json::from_str::<AnthropicErrorResponse>(&error_text) {
                return Err(anyhow!(
                    "Anthropic API error ({}): {} - {}",
                    status,
                    error_response.error.error_type,
                    error_response.error.message
                ));
            } else {
                return Err(anyhow!("Anthropic API error ({}): {}", status, error_text));
            }
        }

        info!("Successfully started streaming chat completion");
        
        // Create stream from response
        let stream = AnthropicStreamUtils::create_stream_from_response(response);
        Ok(stream)
    }

    /// Test connection to Anthropic API
    pub async fn test_connection(&self) -> Result<bool> {
        debug!("Testing connection to Anthropic API");

        // Create a minimal test request
        let test_request = AnthropicMessagesRequest {
            model: self.config.default_model.clone()
                .unwrap_or_else(|| "claude-3-5-sonnet-20241022".to_string()),
            max_tokens: 1,
            messages: vec![super::types::AnthropicMessage {
                role: "user".to_string(),
                content: super::types::AnthropicContent::Text("test".to_string()),
            }],
            system: None,
            tools: None,
            stream: Some(false),
            temperature: None,
            top_p: None,
            top_k: None,
            stop_sequences: None,
            metadata: None,
        };

        let headers = self.auth.create_headers()?;
        let url = self.config.get_messages_url();

        match self
            .client
            .post(&url)
            .headers(headers)
            .json(&test_request)
            .send()
            .await
        {
            Ok(response) => {
                let success = response.status().is_success();
                debug!("Connection test result: {}", if success { "success" } else { "failed" });
                Ok(success)
            }
            Err(e) => {
                warn!("Connection test failed: {}", e);
                Ok(false)
            }
        }
    }

    /// Get available models (Anthropic doesn't provide a models endpoint, so we return known models)
    pub async fn get_models(&self) -> Result<Vec<String>> {
        debug!("Getting available Anthropic models");

        // Return known models since Anthropic doesn't have a models endpoint
        let models = vec![
            "claude-3-5-sonnet-20241022".to_string(),
            "claude-3-5-haiku-20241022".to_string(),
            "claude-3-opus-20240229".to_string(),
            "claude-3-sonnet-20240229".to_string(),
            "claude-3-haiku-20240307".to_string(),
        ];

        debug!("Available models: {:?}", models);
        Ok(models)
    }

    /// Check if a model supports tool calling
    pub fn supports_tool_calling(&self, model: &str) -> bool {
        self.converter.model_supports_capability(model, "tools")
    }

    /// Check if a model supports streaming
    pub fn supports_streaming(&self, model: &str) -> bool {
        self.converter.model_supports_capability(model, "streaming")
    }

    /// Get model capabilities
    pub fn get_model_capabilities(&self, model: &str) -> super::converter::AnthropicModelCapabilities {
        self.converter.get_model_capabilities(model)
    }

    /// Get the maximum number of tokens for a model
    pub fn get_max_tokens(&self, model: &str) -> u32 {
        self.converter.get_max_tokens(model)
    }

    /// Get the authentication handler (for configuration access)
    pub fn auth(&self) -> &AnthropicAuth {
        &self.auth
    }

    /// Get the converter (for format conversion operations)
    pub fn converter(&self) -> &AnthropicConverter {
        &self.converter
    }

    /// Get the configuration
    pub fn config(&self) -> &AnthropicConfig {
        &self.config
    }

    /// Execute a tool call workflow (for testing and integration)
    pub async fn execute_tool_call_workflow(
        &self,
        mut request: ChatCompletionRequest,
        tool_executor: impl Fn(&UnifiedToolCall) -> Result<ToolCallResult> + Send + Sync,
    ) -> Result<ChatCompletionResponse> {
        debug!("Executing tool call workflow for model: {}", request.model);

        // Make initial request
        let mut response = self.chat_completion(request.clone()).await?;

        // Check if there are tool calls in the response
        if let Some(ref choices) = response.choices.get(0) {
            if let Some(ref tool_calls) = choices.message.tool_calls {
                debug!("Found {} tool calls in response", tool_calls.len());

                // Convert tool calls to unified format
                let unified_calls: Vec<UnifiedToolCall> = tool_calls.iter().filter_map(|tc_json| {
                    // Deserialize JSON value to ToolCall struct
                    if let Ok(tc) = serde_json::from_value::<ToolCall>(tc_json.clone()) {
                        let arguments = serde_json::from_str(&tc.function.arguments)
                            .unwrap_or_else(|_| serde_json::Value::Object(Default::default()));
                        
                        Some(UnifiedToolCall {
                            id: tc.id,
                            function_name: tc.function.name,
                            arguments,
                            metadata: std::collections::HashMap::new(),
                        })
                    } else {
                        warn!("Failed to parse tool call: {:?}", tc_json);
                        None
                    }
                }).collect();

                // Execute tool calls
                let mut tool_results = Vec::new();
                for unified_call in &unified_calls {
                    debug!("Executing tool call: {} - {}", unified_call.id, unified_call.function_name);
                    let result = tool_executor(unified_call)?;
                    tool_results.push(result);
                }

                // Add assistant message with tool calls to conversation
                let assistant_message = crate::models::common::ChatMessage {
                    role: MessageRole::Assistant,
                    content: choices.message.content.clone(),
                    name: None,
                    function_call: None,
                    tool_call_id: None,
                    tool_calls: Some(tool_calls.clone()),
                };
                request.messages.push(assistant_message);

                // Add tool result messages
                for result in &tool_results {
                    let tool_message = crate::models::common::ChatMessage {
                        role: MessageRole::Tool,
                        content: result.content.clone(),
                        name: None,
                        function_call: None,
                        tool_call_id: Some(result.tool_call_id.clone()),
                        tool_calls: None,
                    };
                    request.messages.push(tool_message);
                }

                // Make follow-up request with tool results
                debug!("Making follow-up request with tool results");
                response = self.chat_completion(request).await?;
            }
        }

        Ok(response)
    }

    /// Retry a request with exponential backoff
    async fn retry_request<F, Fut, T>(&self, operation: F, max_retries: u32) -> Result<T>
    where
        F: Fn() -> Fut,
        Fut: std::future::Future<Output = Result<T>>,
    {
        let mut last_error = None;
        
        for attempt in 0..=max_retries {
            match operation().await {
                Ok(result) => return Ok(result),
                Err(e) => {
                    last_error = Some(e);
                    
                    if attempt < max_retries {
                        let delay = std::time::Duration::from_millis(1000 * (2_u64.pow(attempt)));
                        debug!("Request failed, retrying in {:?} (attempt {}/{})", delay, attempt + 1, max_retries);
                        tokio::time::sleep(delay).await;
                    }
                }
            }
        }
        
        Err(last_error.unwrap_or_else(|| anyhow!("All retry attempts failed")))
    }

    /// Make a request with retries
    pub async fn chat_completion_with_retries(
        &self,
        request: ChatCompletionRequest,
    ) -> Result<ChatCompletionResponse> {
        let max_retries = self.config.max_retries.unwrap_or(3);
        
        self.retry_request(|| {
            let req = request.clone();
            async move { self.chat_completion(req).await }
        }, max_retries).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::common::{ChatMessage, MessageRole, ToolDefinition, FunctionDefinition};
    use serde_json::json;

    fn create_test_config() -> AnthropicConfig {
        AnthropicConfig {
            api_key: "sk-ant-test123456789012345678901234567890123456789012345".to_string(),
            base_url: Some("https://api.anthropic.com/v1".to_string()),
            ..Default::default()
        }
    }

    fn create_test_request() -> ChatCompletionRequest {
        ChatCompletionRequest {
            model: "claude-3-5-sonnet-20241022".to_string(),
            messages: vec![
                ChatMessage {
                    role: MessageRole::User,
                    content: "Hello, how are you?".to_string(),
                    name: None,
                    function_call: None,
                    tool_call_id: None,
                    tool_calls: None,
                },
            ],
            max_tokens: Some(100),
            temperature: Some(0.7),
            top_p: None,
            n: None,
            stream: Some(false),
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
            metadata: None,
            service_tier: None,
            store: None,
            previous_response_id: None,
        }
    }

    #[tokio::test]
    async fn test_client_creation() {
        let config = create_test_config();
        let client = AnthropicClient::new(config).await;
        assert!(client.is_ok());
    }

    #[tokio::test]
    async fn test_client_creation_invalid_config() {
        let mut config = create_test_config();
        config.api_key = "".to_string();
        
        let client = AnthropicClient::new(config).await;
        assert!(client.is_err());
    }

    #[tokio::test]
    async fn test_model_capabilities() {
        let config = create_test_config();
        if let Ok(client) = AnthropicClient::new(config).await {
            assert!(client.supports_tool_calling("claude-3-5-sonnet-20241022"));
            assert!(client.supports_streaming("claude-3-5-sonnet-20241022"));
            
            let capabilities = client.get_model_capabilities("claude-3-5-sonnet-20241022");
            assert!(capabilities.supports_tools);
            assert!(capabilities.supports_streaming);
            assert_eq!(capabilities.max_tokens, 200000);
            
            assert_eq!(client.get_max_tokens("claude-3-5-sonnet-20241022"), 200000);
        }
    }

    #[tokio::test]
    async fn test_get_models() {
        let config = create_test_config();
        if let Ok(client) = AnthropicClient::new(config).await {
            let models = client.get_models().await;
            assert!(models.is_ok());
            
            let models = models.unwrap();
            assert!(!models.is_empty());
            assert!(models.contains(&"claude-3-5-sonnet-20241022".to_string()));
            assert!(models.contains(&"claude-3-5-haiku-20241022".to_string()));
        }
    }

    #[test]
    fn test_request_conversion() {
        let config = create_test_config();
        if let Ok(client) = tokio_test::block_on(AnthropicClient::new(config)) {
            let openai_request = create_test_request();
            
            let result = client.converter.openai_request_to_anthropic(&openai_request);
            assert!(result.is_ok());
            
            let anthropic_request = result.unwrap();
            assert_eq!(anthropic_request.model, "claude-3-5-sonnet-20241022");
            assert_eq!(anthropic_request.max_tokens, 100);
            assert_eq!(anthropic_request.messages.len(), 1);
        }
    }

    #[test]
    fn test_tool_calling_support() {
        let config = create_test_config();
        if let Ok(client) = tokio_test::block_on(AnthropicClient::new(config)) {
            let mut request = create_test_request();
            
            // Add tools to the request
            request.tools = Some(vec![
                ToolDefinition {
                    tool_type: "function".to_string(),
                    function: FunctionDefinition {
                        name: "get_weather".to_string(),
                        description: Some("Get current weather".to_string()),
                        parameters: Some(json!({
                            "type": "object",
                            "properties": {
                                "location": {
                                    "type": "string",
                                    "description": "The location to get weather for"
                                }
                            },
                            "required": ["location"]
                        })),
                    },
                }
            ]);
            
            let result = client.converter.openai_request_to_anthropic(&request);
            assert!(result.is_ok());
            
            let anthropic_request = result.unwrap();
            assert!(anthropic_request.tools.is_some());
            assert_eq!(anthropic_request.tools.unwrap().len(), 1);
        }
    }

    #[test]
    fn test_auth_access() {
        let config = create_test_config();
        if let Ok(client) = tokio_test::block_on(AnthropicClient::new(config)) {
            let auth = client.auth();
            assert!(auth.is_api_key_format_valid());
            assert!(!auth.get_masked_api_key().is_empty());
        }
    }

    #[test]
    fn test_config_access() {
        let config = create_test_config();
        if let Ok(client) = tokio_test::block_on(AnthropicClient::new(config.clone())) {
            let client_config = client.config();
            assert_eq!(client_config.api_key, config.api_key);
            assert_eq!(client_config.base_url, config.base_url);
        }
    }
}