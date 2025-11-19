use anyhow::{Context, Result};
use serde_json::Value;
use std::sync::Arc;
use tokio_stream::Stream;
use tracing::{debug, info, warn};

use crate::models::request::ChatCompletionRequest;
use crate::models::response::{ChatCompletionResponse, ChatCompletionChunk};
use crate::infrastructure::common::tools::{ToolCallManager, UnifiedToolCall, ToolCallResult};
use crate::infrastructure::common::client::GenericLLMClient;
use crate::infrastructure::common::types::ProviderConfig;
use crate::config::providers::OpenAIConfig;

use super::auth::OpenAIAuth;
use super::converter::OpenAIConverter;

/// OpenAI HTTP client for chat completions with full tool calling support
/// Handles authentication, request/response formatting, streaming, and error handling
pub struct OpenAIClient {
    inner: GenericLLMClient<OpenAIConverter, OpenAIConfig>,
}

impl OpenAIClient {
    /// Create a new OpenAI client
    pub async fn new(config: OpenAIConfig) -> Result<Self> {
        info!("Initializing OpenAI client");

        // Initialize authentication validation (optional, as client handles it via config)
        let auth = OpenAIAuth::from_config(&config)?;
        auth.validate_api_key().context("Invalid OpenAI API key")?;

        let converter = OpenAIConverter::new();
        let tool_manager = ToolCallManager::openai();

        let inner = GenericLLMClient::new(config, converter, tool_manager)?;

        info!("Successfully initialized OpenAI client");

        Ok(Self { inner })
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
        self.inner.chat_completion(request).await
    }

    /// Make a streaming chat completion request
    pub async fn chat_completion_stream(
        &self,
        request: ChatCompletionRequest,
    ) -> Result<impl Stream<Item = Result<ChatCompletionChunk>>> {
        self.inner.chat_completion_stream(request).await
    }

    /// Extract tool calls from a chat completion response
    pub fn extract_tool_calls(&self, response: &ChatCompletionResponse) -> Result<Vec<UnifiedToolCall>> {
        let mut unified_calls = Vec::new();

        for choice in &response.choices {
            if let Some(ref tool_calls) = choice.message.tool_calls {
                for tool_call_value in tool_calls {
                    // Parse the tool call from the response
                    if let Ok(tool_call) = self.inner.tool_manager().converter().provider_tool_calls_to_unified(tool_call_value) {
                        unified_calls.extend(tool_call);
                    }
                }
            }
        }

        Ok(unified_calls)
    }

    /// Process tool call results and create continuation messages
    pub fn process_tool_results(&self, results: Vec<ToolCallResult>) -> Result<Value> {
        self.inner.tool_manager().converter().tool_results_to_provider_messages(&results)
    }

    /// Test the OpenAI API connection
    pub async fn test_connection(&self) -> Result<bool> {
        debug!("Testing OpenAI API connection");

        // Make a simple completion request to test connectivity
        let test_request = ChatCompletionRequest {
            model: self.inner.config().default_model.model_name.clone(),
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

        let url = format!("{}/models", self.inner.config().get_base_url());
        let auth_headers = self.inner.config().auth_headers();

        // Build header map
        let mut headers = reqwest::header::HeaderMap::new();
        for (k, v) in auth_headers {
            let header_name = reqwest::header::HeaderName::from_bytes(k.as_bytes())
                .map_err(|e| anyhow::anyhow!("Invalid header name '{}': {}", k, e))?;
            let header_value = reqwest::header::HeaderValue::from_str(&v)
                .map_err(|e| anyhow::anyhow!("Invalid header value for '{}': {}", k, e))?;
            headers.insert(header_name, header_value);
        }

        let response = self.inner.client()
            .get(&url)
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
        self.inner.config().default_model.model_name.clone()
    }

    /// Get client configuration
    pub fn get_config(&self) -> &OpenAIConfig {
        self.inner.config()
    }

    /// Get tool call manager
    pub fn get_tool_manager(&self) -> &ToolCallManager {
        self.inner.tool_manager()
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
            store: None,
            previous_response_id: None,
        }
    }
}
