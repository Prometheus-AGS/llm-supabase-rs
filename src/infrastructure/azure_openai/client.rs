//! Azure OpenAI HTTP client implementation
//!
//! This module provides the main Azure OpenAI client with full tool calling support,
//! streaming capabilities, and comprehensive error handling.

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

use super::auth::AzureOpenAIAuth;
use super::types::{AzureOpenAIConfig, AzureOpenAIError};
use super::streaming::AzureOpenAIStreamParser;
use super::converter::AzureOpenAIConverter;

/// Azure OpenAI HTTP client for chat completions with full tool calling support
/// Handles Azure-specific authentication, request/response formatting, streaming, and error handling
pub struct AzureOpenAIClient {
    /// HTTP client for making requests
    client: Client,

    /// Azure OpenAI configuration
    config: AzureOpenAIConfig,

    /// Authentication handler
    auth: Arc<AzureOpenAIAuth>,

    /// Tool call manager for unified tool calling
    tool_manager: Arc<ToolCallManager>,

    /// Azure OpenAI converter for tool calls
    converter: Arc<AzureOpenAIConverter>,
}

impl AzureOpenAIClient {
    /// Create a new Azure OpenAI client
    pub async fn new(config: AzureOpenAIConfig) -> Result<Self> {
        info!("Initializing Azure OpenAI client");

        // Validate configuration
        config.validate()
            .context("Invalid Azure OpenAI configuration")?;

        // Initialize authentication
        let auth = AzureOpenAIAuth::new(&config)?;
        
        // Validate API key format
        auth.validate_api_key()
            .context("Invalid Azure OpenAI API key")?;

        let auth = Arc::new(auth);

        // Configure HTTP client with timeouts
        let client = Client::builder()
            .timeout(std::time::Duration::from_secs(config.timeout.unwrap_or(120)))
            .connect_timeout(std::time::Duration::from_secs(30))
            .build()
            .context("Failed to create HTTP client")?;

        // Initialize tool call manager with Azure OpenAI converter
        let converter = Arc::new(AzureOpenAIConverter::new());
        let tool_manager = Arc::new(ToolCallManager::new(Box::new(converter.as_ref().clone())));

        info!("Successfully initialized Azure OpenAI client for endpoint: {} with deployment: {}", 
              config.endpoint, config.deployment);

        Ok(Self {
            client,
            config,
            auth,
            tool_manager,
            converter,
        })
    }

    /// Create Azure OpenAI client from environment variables
    pub async fn from_env() -> Result<Self> {
        let config = AzureOpenAIConfig::from_env()
            .context("Failed to load Azure OpenAI configuration from environment")?;
        Self::new(config).await
    }

    /// Make a chat completion request (non-streaming)
    pub async fn chat_completion(
        &self,
        mut request: ChatCompletionRequest,
    ) -> Result<ChatCompletionResponse> {
        debug!("Making Azure OpenAI chat completion request to deployment: {}", self.config.deployment);

        // Map model to deployment name
        let deployment = self.config.get_deployment_for_model(&request.model);
        debug!("Using deployment '{}' for model '{}'", deployment, request.model);

        // Validate tool request if tools are present
        if request.tools.is_some() {
            self.converter.validate_tool_request(&request)
                .context("Invalid tool configuration")?;
        }

        let headers = self.auth.create_headers()?;
        let url = self.get_chat_completions_url(&deployment);

        // Ensure streaming is disabled for non-streaming requests
        request.stream = Some(false);

        let mut attempt = 0;
        let max_retries = self.config.max_retries.unwrap_or(3);

        loop {
            let response = self
                .client
                .post(&url)
                .headers(headers.clone())
                .json(&request)
                .send()
                .await
                .context("Failed to send Azure OpenAI chat completion request")?;

            let status = response.status();
            
            if status.is_success() {
                let response_text = response.text().await
                    .context("Failed to read response body")?;

                let chat_response: ChatCompletionResponse = serde_json::from_str(&response_text)
                    .context("Failed to parse Azure OpenAI response")?;

                debug!("Successfully received Azure OpenAI chat completion response with {} choices", 
                       chat_response.choices.len());

                return Ok(chat_response);
            }

            // Handle rate limiting
            if status.as_u16() == 429 && attempt < max_retries {
                self.handle_rate_limit(attempt).await?;
                attempt += 1;
                continue;
            }

            // Handle other errors
            let error_text = response.text().await
                .unwrap_or_else(|_| "Unknown error".to_string());
            
            // Try to parse Azure-specific error format
            if let Ok(azure_error) = serde_json::from_str::<AzureOpenAIError>(&error_text) {
                return Err(anyhow::anyhow!(
                    "Azure OpenAI API request failed with status {}: {} (Code: {:?})", 
                    status, 
                    azure_error.error.message,
                    azure_error.error.code
                ));
            }

            return Err(anyhow::anyhow!(
                "Azure OpenAI API request failed with status {}: {}", 
                status, error_text
            ));
        }
    }

    /// Make a streaming chat completion request
    pub async fn chat_completion_stream(
        &self,
        mut request: ChatCompletionRequest,
    ) -> Result<impl Stream<Item = Result<ChatCompletionChunk>>> {
        debug!("Making Azure OpenAI streaming chat completion request to deployment: {}", self.config.deployment);

        // Map model to deployment name
        let deployment = self.config.get_deployment_for_model(&request.model);
        debug!("Using deployment '{}' for streaming model '{}'", deployment, request.model);

        // Validate tool request if tools are present
        if request.tools.is_some() {
            self.converter.validate_tool_request(&request)
                .context("Invalid tool configuration")?;
        }

        let headers = self.auth.create_streaming_headers()?;
        let url = self.get_chat_completions_url(&deployment);

        // Ensure streaming is enabled
        request.stream = Some(true);

        let response = self
            .client
            .post(&url)
            .headers(headers)
            .json(&request)
            .send()
            .await
            .context("Failed to send Azure OpenAI streaming chat completion request")?;

        let status = response.status();
        
        if !status.is_success() {
            let error_text = response.text().await
                .unwrap_or_else(|_| "Unknown error".to_string());
            
            // Try to parse Azure-specific error format
            if let Ok(azure_error) = serde_json::from_str::<AzureOpenAIError>(&error_text) {
                return Err(anyhow::anyhow!(
                    "Azure OpenAI streaming API request failed with status {}: {} (Code: {:?})", 
                    status, 
                    azure_error.error.message,
                    azure_error.error.code
                ));
            }

            return Err(anyhow::anyhow!(
                "Azure OpenAI streaming API request failed with status {}: {}", 
                status, error_text
            ));
        }

        // Create stream parser for Server-Sent Events
        let stream_parser = AzureOpenAIStreamParser::new();
        let byte_stream = response.bytes_stream();

        let parsed_stream = stream_parser.parse_stream(byte_stream).await?;

        debug!("Successfully created Azure OpenAI streaming response");
        Ok(parsed_stream)
    }

    /// Extract tool calls from a chat completion response
    pub fn extract_tool_calls(&self, response: &ChatCompletionResponse) -> Result<Vec<UnifiedToolCall>> {
        self.converter.extract_tool_calls(response)
    }

    /// Process tool call results and create continuation messages
    pub fn process_tool_results(&self, results: Vec<ToolCallResult>) -> Result<Value> {
        self.converter.tool_results_to_provider_messages(&results)
    }

    /// Test the Azure OpenAI API connection
    pub async fn test_connection(&self) -> Result<bool> {
        debug!("Testing Azure OpenAI API connection");

        // Test authentication first
        match self.auth.test_authentication(&self.config.endpoint, &self.config.api_version).await {
            Ok(auth_success) => {
                if !auth_success {
                    warn!("Azure OpenAI authentication test failed");
                    return Ok(false);
                }
            }
            Err(e) => {
                warn!("Azure OpenAI authentication test error: {}", e);
                return Ok(false);
            }
        }

        // Make a simple completion request to test full functionality
        let test_request = ChatCompletionRequest {
            model: self.config.deployment.clone(),
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
                info!("Azure OpenAI API connection test successful");
                Ok(true)
            }
            Err(e) => {
                warn!("Azure OpenAI API connection test failed: {}", e);
                Ok(false)
            }
        }
    }

    /// Get available Azure OpenAI deployments (Azure OpenAI doesn't have a models endpoint)
    pub async fn get_deployments(&self) -> Result<Vec<String>> {
        debug!("Fetching available Azure OpenAI deployments");

        let headers = self.auth.create_headers()?;
        let deployments_url = self.config.get_models_url(); // This returns the deployments endpoint

        let response = self
            .client
            .get(&deployments_url)
            .headers(headers)
            .send()
            .await
            .context("Failed to fetch Azure OpenAI deployments")?;

        if !response.status().is_success() {
            // Azure OpenAI might not support this endpoint, return the configured deployment
            warn!("Failed to fetch deployments from API: HTTP {}, returning configured deployment", response.status());
            return Ok(vec![self.config.deployment.clone()]);
        }

        let deployments_response: Value = response.json().await
            .context("Failed to parse deployments response")?;

        let mut deployments = Vec::new();
        
        if let Some(data) = deployments_response.get("data").and_then(|d| d.as_array()) {
            for deployment in data {
                if let Some(id) = deployment.get("id").and_then(|i| i.as_str()) {
                    deployments.push(id.to_string());
                }
            }
        }

        // If no deployments found, include the configured one
        if deployments.is_empty() {
            deployments.push(self.config.deployment.clone());
        }

        debug!("Found {} available deployments", deployments.len());
        Ok(deployments)
    }

    /// Check if a specific deployment supports tool calling
    pub fn supports_tool_calling(&self, deployment: &str) -> bool {
        self.converter.deployment_supports_feature(deployment, "tools")
    }

    /// Check if a specific deployment supports streaming
    pub fn supports_streaming(&self, deployment: &str) -> bool {
        self.converter.deployment_supports_feature(deployment, "streaming")
    }

    /// Get the recommended deployment for tool calling
    pub fn get_recommended_tool_deployment(&self) -> String {
        self.config.deployment.clone()
    }

    /// Handle rate limiting with exponential backoff
    async fn handle_rate_limit(&self, attempt: u32) -> Result<()> {
        let max_retries = self.config.max_retries.unwrap_or(3);
        
        if attempt >= max_retries {
            return Err(anyhow::anyhow!("Maximum retry attempts exceeded"));
        }

        let base_delay = self.config.retry_delay_ms.unwrap_or(1000);
        let delay = if self.config.exponential_backoff.unwrap_or(true) {
            std::time::Duration::from_millis(base_delay * (2_u64.pow(attempt)))
        } else {
            std::time::Duration::from_millis(base_delay)
        };

        warn!("Azure OpenAI rate limit hit, retrying after {:?} (attempt {})", delay, attempt + 1);
        
        tokio::time::sleep(delay).await;
        Ok(())
    }

    /// Get the chat completions URL for a specific deployment
    fn get_chat_completions_url(&self, deployment: &str) -> String {
        format!(
            "{}/openai/deployments/{}/chat/completions?api-version={}",
            self.config.endpoint.trim_end_matches('/'),
            deployment,
            self.config.api_version
        )
    }

    /// Get client configuration
    pub fn get_config(&self) -> &AzureOpenAIConfig {
        &self.config
    }

    /// Get tool call manager
    pub fn get_tool_manager(&self) -> &ToolCallManager {
        &self.tool_manager
    }

    /// Get the converter
    pub fn get_converter(&self) -> &AzureOpenAIConverter {
        &self.converter
    }

    /// Get authentication handler
    pub fn get_auth(&self) -> &AzureOpenAIAuth {
        &self.auth
    }

    /// Map model names to deployment names
    pub fn map_model_to_deployment(&self, model: &str) -> String {
        self.config.get_deployment_for_model(model)
    }

    /// Check if the client is configured for Azure AD authentication
    pub fn uses_azure_ad(&self) -> bool {
        self.auth.has_azure_ad_config()
    }

    /// Get the resource name from the endpoint
    pub fn get_resource_name(&self) -> Option<String> {
        self.config.get_resource_name()
    }

    /// Validate a chat completion request for Azure OpenAI
    pub fn validate_request(&self, request: &ChatCompletionRequest) -> Result<()> {
        // Check deployment exists
        let deployment = self.config.get_deployment_for_model(&request.model);
        if deployment.is_empty() {
            return Err(anyhow::anyhow!("No deployment configured for model '{}'", request.model));
        }

        // Validate tool request if tools are present
        if request.tools.is_some() {
            self.converter.validate_tool_request(request)?;
        }

        // Check token limits
        if let Some(max_tokens) = request.max_tokens {
            if max_tokens > 128000 {
                return Err(anyhow::anyhow!("max_tokens cannot exceed 128000"));
            }
        }

        Ok(())
    }

    /// Create a continuation request with tool results
    pub fn create_tool_continuation_request(
        &self,
        original_request: &ChatCompletionRequest,
        tool_results: Vec<ToolCallResult>,
    ) -> Result<ChatCompletionRequest> {
        let mut messages = original_request.messages.clone();
        
        // Add tool result messages
        let result_messages = self.converter.convert_tool_results(&tool_results)?;
        messages.extend(result_messages);

        Ok(ChatCompletionRequest {
            model: original_request.model.clone(),
            messages,
            tools: original_request.tools.clone(),
            tool_choice: original_request.tool_choice.clone(),
            max_tokens: original_request.max_tokens,
            temperature: original_request.temperature,
            top_p: original_request.top_p,
            stream: original_request.stream,
            ..Default::default()
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_azure_openai_client_creation() {
        let config = AzureOpenAIConfig {
            api_key: "test-api-key-1234567890123456789012345678901234567890".to_string(),
            endpoint: "https://test-resource.openai.azure.com".to_string(),
            deployment: "gpt-4o".to_string(),
            api_version: "2024-02-15-preview".to_string(),
            ..Default::default()
        };

        let client = AzureOpenAIClient::new(config).await;
        assert!(client.is_ok());
    }

    #[tokio::test]
    async fn test_invalid_config() {
        let config = AzureOpenAIConfig {
            api_key: "short".to_string(), // Too short
            endpoint: "invalid-endpoint".to_string(), // Not HTTPS
            deployment: "".to_string(), // Empty
            ..Default::default()
        };

        let client = AzureOpenAIClient::new(config).await;
        assert!(client.is_err());
    }

    #[test]
    fn test_tool_calling_support() {
        let config = AzureOpenAIConfig {
            api_key: "test-api-key-1234567890123456789012345678901234567890".to_string(),
            endpoint: "https://test-resource.openai.azure.com".to_string(),
            deployment: "gpt-4o".to_string(),
            ..Default::default()
        };

        let client_result = tokio::runtime::Runtime::new()
            .unwrap()
            .block_on(AzureOpenAIClient::new(config));

        if let Ok(client) = client_result {
            assert!(client.supports_tool_calling("gpt-4o-deployment"));
            assert!(client.supports_tool_calling("gpt-35-turbo"));
            assert!(!client.supports_tool_calling("unknown-deployment"));
        }
    }

    #[test]
    fn test_streaming_support() {
        let config = AzureOpenAIConfig {
            api_key: "test-api-key-1234567890123456789012345678901234567890".to_string(),
            endpoint: "https://test-resource.openai.azure.com".to_string(),
            deployment: "gpt-4o".to_string(),
            ..Default::default()
        };

        let client_result = tokio::runtime::Runtime::new()
            .unwrap()
            .block_on(AzureOpenAIClient::new(config));

        if let Ok(client) = client_result {
            assert!(client.supports_streaming("gpt-4o-deployment"));
            assert!(client.supports_streaming("gpt-35-turbo"));
            // All Azure OpenAI deployments support streaming
        }
    }

    #[test]
    fn test_model_to_deployment_mapping() {
        let mut mappings = std::collections::HashMap::new();
        mappings.insert("gpt-4".to_string(), "my-gpt4-deployment".to_string());

        let config = AzureOpenAIConfig {
            api_key: "test-api-key-1234567890123456789012345678901234567890".to_string(),
            endpoint: "https://test-resource.openai.azure.com".to_string(),
            deployment: "default-deployment".to_string(),
            deployment_mappings: Some(mappings),
            ..Default::default()
        };

        let client_result = tokio::runtime::Runtime::new()
            .unwrap()
            .block_on(AzureOpenAIClient::new(config));

        if let Ok(client) = client_result {
            assert_eq!(client.map_model_to_deployment("gpt-4"), "my-gpt4-deployment");
            assert_eq!(client.map_model_to_deployment("unknown"), "default-deployment");
        }
    }

    #[test]
    fn test_chat_completions_url_generation() {
        let config = AzureOpenAIConfig {
            api_key: "test-api-key-1234567890123456789012345678901234567890".to_string(),
            endpoint: "https://test-resource.openai.azure.com".to_string(),
            deployment: "gpt-4o".to_string(),
            api_version: "2024-02-15-preview".to_string(),
            ..Default::default()
        };

        let client_result = tokio::runtime::Runtime::new()
            .unwrap()
            .block_on(AzureOpenAIClient::new(config));

        if let Ok(client) = client_result {
            let url = client.get_chat_completions_url("gpt-4o");
            assert_eq!(
                url, 
                "https://test-resource.openai.azure.com/openai/deployments/gpt-4o/chat/completions?api-version=2024-02-15-preview"
            );
        }
    }

    #[test]
    fn test_validate_request() {
        let config = AzureOpenAIConfig {
            api_key: "test-api-key-1234567890123456789012345678901234567890".to_string(),
            endpoint: "https://test-resource.openai.azure.com".to_string(),
            deployment: "gpt-4o".to_string(),
            ..Default::default()
        };

        let client_result = tokio::runtime::Runtime::new()
            .unwrap()
            .block_on(AzureOpenAIClient::new(config));

        if let Ok(client) = client_result {
            let valid_request = ChatCompletionRequest {
                model: "gpt-4o".to_string(),
                messages: vec![],
                max_tokens: Some(100),
                ..Default::default()
            };

            assert!(client.validate_request(&valid_request).is_ok());

            let invalid_request = ChatCompletionRequest {
                model: "gpt-4o".to_string(),
                messages: vec![],
                max_tokens: Some(200000), // Too high
                ..Default::default()
            };

            assert!(client.validate_request(&invalid_request).is_err());
        }
    }
}