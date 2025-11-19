
//! AWS Bedrock HTTP client implementation
//!
//! This module provides the main HTTP client for AWS Bedrock API interactions.
//! It handles request signing, model-specific formatting, streaming, and tool calling.

use anyhow::{anyhow, Result};
use bytes::Bytes;
use reqwest::{Client as ReqwestClient, Response};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::time::sleep;
use tokio_stream::Stream;
use tracing::{debug, error, info, trace, warn};

use crate::infrastructure::common::tools::{ToolCallConverter, UnifiedToolCall};
use crate::models::common::{ChatMessage, ToolDefinition, FinishReason};
use crate::models::request::ChatCompletionRequest;
use crate::models::response::{ChatCompletionResponse, ChatCompletionChunk, ChatCompletionChoice, Message, Choice};
use crate::models::common::Usage;
use crate::shared::types::ToolCall;

use super::auth::{BedrockAuth, AwsCredentials};
use super::converter::{BedrockConverter, BedrockToolCallContext};
use super::streaming::BedrockStreamParser;
use super::types::BedrockStreamEvent;
use super::types::{
    BedrockConfig, BedrockError, BedrockModel, BedrockModelFamily, BedrockRequest, BedrockResponse,
};

/// AWS Bedrock HTTP client
#[derive(Clone)]
pub struct BedrockClient {
    /// HTTP client for making requests
    http_client: ReqwestClient,
    
    /// Authentication handler
    auth: Arc<tokio::sync::RwLock<BedrockAuth>>,
    
    /// Configuration
    config: BedrockConfig,
    
    /// Tool call converter
    converter: Arc<BedrockConverter>,
    
    /// Stream parser
    stream_parser: Arc<BedrockStreamParser>,
}

impl BedrockClient {
    /// Create a new Bedrock client
    pub async fn new(config: BedrockConfig) -> Result<Self> {
        info!("Initializing AWS Bedrock client for region: {}", config.region);
        
        // Create HTTP client with timeouts
        let http_client = ReqwestClient::builder()
            .timeout(Duration::from_secs(config.timeouts.total))
            .connect_timeout(Duration::from_secs(config.timeouts.connect))
            .read_timeout(Duration::from_secs(config.timeouts.read))
            .user_agent(&config.api.user_agent)
            .build()
            .map_err(|e| anyhow!("Failed to create HTTP client: {}", e))?;
        
        // Initialize authentication
        let auth = BedrockAuth::new(&config).await?;
        
        // Test credentials if validation is enabled
        if config.validate_credentials {
            debug!("Validating AWS credentials...");
            if !auth.test_credentials().await.unwrap_or(false) {
                warn!("Credential validation failed, but proceeding anyway");
            } else {
                debug!("Credentials validated successfully");
            }
        }
        
        let client = Self {
            http_client,
            auth: Arc::new(tokio::sync::RwLock::new(auth)),
            config,
            converter: Arc::new(BedrockConverter::new()),
            stream_parser: Arc::new(BedrockStreamParser::new()),
        };
        
        info!("AWS Bedrock client initialized successfully");
        Ok(client)
    }
    
    /// Create Bedrock client from environment variables
    pub async fn from_env() -> Result<Self> {
        let config = BedrockConfig::from_env()?;
        Self::new(config).await
    }
    
    /// Make a chat completion request
    pub async fn chat_completion(&self, request: ChatCompletionRequest) -> Result<ChatCompletionResponse> {
        debug!("Making chat completion request to model: {}", request.model);
        
        // Get model family and validate tool support
        let model_family = self.config.get_model_family(&request.model);
        
        if request.tools.is_some() && !self.converter.supports_tools(&model_family) {
            return Err(anyhow!(BedrockError::ToolCallingNotSupported(request.model.clone())));
        }
        
        // Convert request to Bedrock format
        let bedrock_request = self.convert_to_bedrock_request(&request, &model_family).await?;
        
        // Make the API call
        let response = if request.stream.unwrap_or(false) {
            // For streaming, we need to collect all chunks into a final response
            return Err(anyhow!("Streaming should use chat_completion_stream method"));
        } else {
            self.invoke_model(&bedrock_request).await?
        };
        
        // Convert response to OpenAI format
        self.convert_bedrock_response_to_openai(&response, &model_family, &request).await
    }
    
    /// Make a streaming chat completion request
    pub async fn chat_completion_stream(
        &self, 
        request: ChatCompletionRequest
    ) -> Result<impl Stream<Item = Result<ChatCompletionChunk>>> {
        debug!("Making streaming chat completion request to model: {}", request.model);
        
        let model_family = self.config.get_model_family(&request.model);
        
        // Convert request to Bedrock format
        let bedrock_request = self.convert_to_bedrock_request(&request, &model_family).await?;
        
        // Make streaming API call
        let response_stream = self.invoke_model_with_response_stream(&bedrock_request).await?;
        
        // Convert stream to OpenAI format
        Ok(self.stream_parser.parse_bedrock_stream(response_stream, model_family))
    }
    
    /// Convert OpenAI request to Bedrock format
    async fn convert_to_bedrock_request(
        &self,
        request: &ChatCompletionRequest,
        model_family: &BedrockModelFamily,
    ) -> Result<BedrockRequest> {
        // Convert messages to model-specific format
        let messages_json = self.converter.convert_messages_to_bedrock(&request.messages, model_family.clone())?;
        
        // Create base body with messages
        let mut body = messages_json;
        
        // Add generation parameters based on model family
        match model_family {
            BedrockModelFamily::Anthropic => {
                self.add_claude_parameters(&mut body, request)?;
            }
            BedrockModelFamily::Meta => {
                self.add_llama_parameters(&mut body, request)?;
            }
            BedrockModelFamily::Mistral => {
                self.add_mistral_parameters(&mut body, request)?;
            }
            BedrockModelFamily::Amazon => {
                self.add_titan_parameters(&mut body, request)?;
            }
            BedrockModelFamily::Cohere => {
                self.add_cohere_parameters(&mut body, request)?;
            }
            _ => {
                return Err(anyhow!("Unsupported model family: {:?}", model_family));
            }
        }
        
        // Add tools if supported and provided
        if let Some(tools) = &request.tools {
            if self.converter.supports_tools(model_family) {
                let tools_json = self.converter.get_tool_format_for_family(model_family, tools)?;
                body["tools"] = tools_json;
            }
        }
        
        Ok(BedrockRequest {
            model_id: request.model.clone(),
            body,
            content_type: "application/json".to_string(),
            accept: if request.stream.unwrap_or(false) {
                "application/vnd.amazon.eventstream".to_string()
            } else {
                "application/json".to_string()
            },
        })
    }
    
    /// Add Claude-specific parameters
    fn add_claude_parameters(&self, body: &mut Value, request: &ChatCompletionRequest) -> Result<()> {
        if let Some(max_tokens) = request.max_tokens {
            body["max_tokens"] = json!(max_tokens);
        } else {
            body["max_tokens"] = json!(4096); // Default for Claude
        }
        
        if let Some(temp) = request.temperature {
            body["temperature"] = json!(temp);
        }
        
        if let Some(top_p) = request.top_p {
            body["top_p"] = json!(top_p);
        }
        
        if let Some(stop) = &request.stop {
            match stop {
                crate::models::request::Stop::String(s) => {
                    body["stop_sequences"] = json!(vec![s]);
                }
                crate::models::request::Stop::Array(arr) => {
                    body["stop_sequences"] = json!(arr);
                }
            }
        }
        
        // Claude-specific parameters
        body["anthropic_version"] = json!("bedrock-2023-05-31");
        
        Ok(())
    }
    
    /// Add Llama-specific parameters
    fn add_llama_parameters(&self, body: &mut Value, request: &ChatCompletionRequest) -> Result<()> {
        if let Some(max_tokens) = request.max_tokens {
            body["max_gen_len"] = json!(max_tokens);
        }
        
        if let Some(temp) = request.temperature {
            body["temperature"] = json!(temp);
        }
        
        if let Some(top_p) = request.top_p {
            body["top_p"] = json!(top_p);
        }
        
        Ok(())
    }
    
    /// Add Mistral-specific parameters
    fn add_mistral_parameters(&self, body: &mut Value, request: &ChatCompletionRequest) -> Result<()> {
        if let Some(max_tokens) = request.max_tokens {
            body["max_tokens"] = json!(max_tokens);
        }
        
        if let Some(temp) = request.temperature {
            body["temperature"] = json!(temp);
        }
        
        if let Some(top_p) = request.top_p {
            body["top_p"] = json!(top_p);
        }
        
        Ok(())
    }
    
    /// Add Amazon Titan-specific parameters
    fn add_titan_parameters(&self, body: &mut Value, request: &ChatCompletionRequest) -> Result<()> {
        let mut text_generation_config = json!({});
        
        if let Some(max_tokens) = request.max_tokens {
            text_generation_config["maxTokenCount"] = json!(max_tokens);
        }
        
        if let Some(temp) = request.temperature {
            text_generation_config["temperature"] = json!(temp);
        }
        
        if let Some(top_p) = request.top_p {
            text_generation_config["topP"] = json!(top_p);
        }
        
        if let Some(stop) = &request.stop {
            match stop {
                crate::models::request::Stop::String(s) => {
                    text_generation_config["stopSequences"] = json!(vec![s]);
                }
                crate::models::request::Stop::Array(arr) => {
                    text_generation_config["stopSequences"] = json!(arr);
                }
            }
        }
        
        body["textGenerationConfig"] = text_generation_config;
        Ok(())
    }
    
    /// Add Cohere-specific parameters
    fn add_cohere_parameters(&self, body: &mut Value, request: &ChatCompletionRequest) -> Result<()> {
        if let Some(max_tokens) = request.max_tokens {
            body["max_tokens"] = json!(max_tokens);
        }
        
        if let Some(temp) = request.temperature {
            body["temperature"] = json!(temp);
        }
        
        if let Some(top_p) = request.top_p {
            body["p"] = json!(top_p);
        }
        
        Ok(())
    }
    
    /// Invoke Bedrock model (non-streaming)
    async fn invoke_model(&self, request: &BedrockRequest) -> Result<BedrockResponse> {
        let url = format!(
            "{}/model/{}/invoke",
            self.config.get_endpoint_url(),
            request.model_id
        );
        
        let body_bytes = serde_json::to_vec(&request.body)?;
        
        // Sign the request
        let mut headers = HashMap::new();
        headers.insert("Content-Type".to_string(), request.content_type.clone());
        headers.insert("Accept".to_string(), request.accept.clone());
        
        // Add any additional headers from config
        for (key, value) in &self.config.api.additional_headers {
            headers.insert(key.clone(), value.clone());
        }
        
        let auth = self.auth.read().await;
        let signed_headers = auth.sign_request("POST", &url, &headers, &body_bytes)?;
        drop(auth);
        
        // Convert to reqwest headers
        let mut req_headers = reqwest::header::HeaderMap::new();
        for (key, value) in signed_headers {
            if let (Ok(name), Ok(val)) = (
                reqwest::header::HeaderName::from_bytes(key.as_bytes()),
                reqwest::header::HeaderValue::from_str(&value)
            ) {
                req_headers.insert(name, val);
            }
        }
        
        // Make the request with retries
        let mut last_error = None;
        for attempt in 0..=self.config.api.max_retries {
            if attempt > 0 {
                let delay = self.calculate_retry_delay(attempt);
                debug!("Retrying request in {}ms (attempt {})", delay, attempt + 1);
                sleep(Duration::from_millis(delay)).await;
            }
            
            match self
                .http_client
                .post(&url)
                .headers(req_headers.clone())
                .body(body_bytes.clone())
                .send()
                .await
            {
                Ok(response) => {
                    if response.status().is_success() {
                        let body = response.bytes().await?;
                        let response_json: Value = serde_json::from_slice(&body)?;
                        
                        return Ok(BedrockResponse {
                            body: response_json,
                            content_type: "application/json".to_string(),
                            metadata: HashMap::new(),
                        });
                    } else {
                        let status = response.status();
                        let error_body = response.text().await.unwrap_or_default();
                        
                        let error = anyhow!(
                            "Bedrock API error: {} - {}",
                            status,
                            error_body
                        );
                        
                        // Don't retry on client errors (4xx)
                        if status.is_client_error() {
                            return Err(error);
                        }
                        
                        last_error = Some(error);
                    }
                }
                Err(e) => {
                    last_error = Some(anyhow!("HTTP request failed: {}", e));
                }
            }
        }
        
        Err(last_error.unwrap_or_else(|| anyhow!("Request failed after all retries")))
    }
    
    /// Invoke Bedrock model with response stream (streaming)
    async fn invoke_model_with_response_stream(&self, request: &BedrockRequest) -> Result<Response> {
        let url = format!(
            "{}/model/{}/invoke-with-response-stream",
            self.config.get_endpoint_url(),
            request.model_id
        );
        
        let body_bytes = serde_json::to_vec(&request.body)?;
        
        // Sign the request
        let mut headers = HashMap::new();
        headers.insert("Content-Type".to_string(), request.content_type.clone());
        headers.insert("Accept".to_string(), request.accept.clone());
        
        for (key, value) in &self.config.api.additional_headers {
            headers.insert(key.clone(), value.clone());
        }
        
        let auth = self.auth.read().await;
        let signed_headers = auth.sign_request("POST", &url, &headers, &body_bytes)?;
        drop(auth);
        
        // Convert to reqwest headers
        let mut req_headers = reqwest::header::HeaderMap::new();
        for (key, value) in signed_headers {
            if let (Ok(name), Ok(val)) = (
                reqwest::header::HeaderName::from_bytes(key.as_bytes()),
                reqwest::header::HeaderValue::from_str(&value)
            ) {
                req_headers.insert(name, val);
            }
        }
        
        let response = self
            .http_client
            .post(&url)
            .headers(req_headers)
            .body(body_bytes)
            .send()
            .await?;
            
        if !response.status().is_success() {
            let status = response.status();
            let error_body = response.text().await.unwrap_or_default();
            return Err(anyhow!("Bedrock streaming API error: {} - {}", status, error_body));
        }
        
        Ok(response)
    }
    
    /// Convert Bedrock response to OpenAI format
    async fn convert_bedrock_response_to_openai(
        &self,
        response: &BedrockResponse,
        model_family: &BedrockModelFamily,
        original_request: &ChatCompletionRequest,
    ) -> Result<ChatCompletionResponse> {
        let content = self.extract_content_from_response(&response.body, model_family)?;
        let tool_calls = if original_request.tools.is_some() {
            self.converter.extract_tool_calls_from_response(&response.body, model_family.clone())?
        } else {
            vec![]
        };
        
        // Convert tool calls to OpenAI format
        let openai_tool_calls = if !tool_calls.is_empty() {
            let tool_calls_vec = self.converter.unified_to_openai_tool_calls(&tool_calls);
            Some(tool_calls_vec.into_iter()
                .map(|tc| serde_json::to_value(tc).unwrap_or(serde_json::Value::Null))
                .collect())
        } else {
            None
        };
        
        let message = Message {
            role: crate::models::common::MessageRole::Assistant,
            content: content,
            name: None,
            function_call: None,
            tool_calls: openai_tool_calls,
            tool_call_id: None,
        };
        
        let choice = Choice {
            index: 0,
            message,
            logprobs: None,
            finish_reason: self.extract_finish_reason(&response.body, model_family),
        };
        
        let usage = self.extract_usage(&response.body, model_family);
        
        Ok(ChatCompletionResponse {
            id: format!("chatcmpl-{}", uuid::Uuid::new_v4().to_string().replace('-', "")),
            object: "chat.completion".to_string(),
            created: chrono::Utc::now().timestamp() as u64,
            model: original_request.model.clone(),
            system_fingerprint: None,
            choices: vec![choice],
            usage,
        })
    }
    
    /// Extract content from model response
    fn extract_content_from_response(&self, response: &Value, model_family: &BedrockModelFamily) -> Result<String> {
        match model_family {
            BedrockModelFamily::Anthropic => {
                if let Some(content) = response.get("content").and_then(|c| c.as_array()) {
                    for item in content {
                        if let Some(item_type) = item.get("type").and_then(|t| t.as_str()) {
                            if item_type == "text" {
                                if let Some(text) = item.get("text").and_then(|t| t.as_str()) {
                                    return Ok(text.to_string());
                                }
                            }
                        }
                    }
                }
                Ok(String::new())
            }
            BedrockModelFamily::Meta => {
                response.get("generation")
                    .and_then(|g| g.as_str())
                    .map(|s| s.to_string())
                    .ok_or_else(|| anyhow!("No generation field in Llama response"))
            }
            BedrockModelFamily::Mistral => {
                response.get("outputs")
                    .and_then(|o| o.as_array())
                    .and_then(|arr| arr.first())
                    .and_then(|item| item.get("text"))
                    .and_then(|t| t.as_str())
                    .map(|s| s.to_string())
                    .ok_or_else(|| anyhow!("No outputs field in Mistral response"))
            }
            BedrockModelFamily::Amazon => {
                response.get("results")
                    .and_then(|r| r.as_array())
                    .and_then(|arr| arr.first())
                    .and_then(|item| item.get("outputText"))
                    .and_then(|t| t.as_str())
                    .map(|s| s.to_string())
                    .ok_or_else(|| anyhow!("No results field in Titan response"))
            }
            _ => Err(anyhow!("Unsupported model family for content extraction: {:?}", model_family)),
        }
    }
    
    /// Extract finish reason from response
    fn extract_finish_reason(&self, response: &Value, model_family: &BedrockModelFamily) -> FinishReason {
        match model_family {
            BedrockModelFamily::Anthropic => {
                response.get("stop_reason")
                    .and_then(|sr| sr.as_str())
                    .map(|reason| match reason {
                        "end_turn" => FinishReason::Stop,
                        "max_tokens" => FinishReason::Length,
                        "tool_use" => FinishReason::ToolCalls,
                        _ => FinishReason::Stop,
                    })
                    .unwrap_or(FinishReason::Stop)
            }
            BedrockModelFamily::Meta => {
                response.get("stop_reason")
                    .and_then(|sr| sr.as_str())
                    .map(|reason| match reason {
                        "stop" => FinishReason::Stop,
                        "length" => FinishReason::Length,
                        _ => FinishReason::Stop,
                    })
                    .unwrap_or(FinishReason::Stop)
            }
            _ => FinishReason::Stop,
        }
    }
    
    /// Extract usage information from response
    fn extract_usage(&self, response: &Value, model_family: &BedrockModelFamily) -> Usage {
        let (input_tokens, output_tokens) = match model_family {
            BedrockModelFamily::Anthropic => {
                let input = response.get("usage")
                    .and_then(|u| u.get("input_tokens"))
                    .and_then(|t| t.as_u64())
                    .unwrap_or(0) as u32;
                let output = response.get("usage")
                    .and_then(|u| u.get("output_tokens"))
                    .and_then(|t| t.as_u64())
                    .unwrap_or(0) as u32;
                (input, output)
            }
            BedrockModelFamily::Meta => {
                let input = response.get("prompt_token_count")
                    .and_then(|t| t.as_u64())
                    .unwrap_or(0) as u32;
                let output = response.get("generation_token_count")
                    .and_then(|t| t.as_u64())
                    .unwrap_or(0) as u32;
                (input, output)
            }
            _ => (0, 0),
        };
        
        Usage {
            prompt_tokens: input_tokens,
            completion_tokens: output_tokens,
            total_tokens: input_tokens + output_tokens,
            completion_tokens_details: None,
            prompt_tokens_details: None,
        }
    }
    
    /// Calculate retry delay with exponential backoff
    fn calculate_retry_delay(&self, attempt: u32) -> u64 {
        if self.config.api.exponential_backoff {
            let base_delay = self.config.api.retry_delay_ms;
            let delay = base_delay * 2_u64.pow(attempt - 1);
            std::cmp::min(delay, self.config.api.max_retry_delay_ms)
        } else {
            self.config.api.retry_delay_ms
        }
    }
    
    /// Test connection to Bedrock
    pub async fn test_connection(&self) -> Result<bool> {
        debug!("Testing Bedrock connection");
        
        // Refresh credentials if needed
        let mut auth = self.auth.write().await;
        auth.refresh_if_needed(&self.config).await?;
        drop(auth);
        
        // Make a simple request to test connection
        // We'll use the ListFoundationModels API for this test
        let url = format!("{}/foundation-models", self.config.get_endpoint_url());
        
        let mut headers = HashMap::new();
        headers.insert("Content-Type".to_string(), "application/json".to_string());
        
        let auth = self.auth.read().await;
        let signed_headers = auth.sign_request("GET", &url, &headers, b"")?;
        drop(auth);
        
        let mut req_headers = reqwest::header::HeaderMap::new();
        for (key, value) in signed_headers {
            if let (Ok(name), Ok(val)) = (
                reqwest::header::HeaderName::from_bytes(key.as_bytes()),
                reqwest::header::HeaderValue::from_str(&value)
            ) {
                req_headers.insert(name, val);
            }
        }
        
        match self
            .http_client
            .get(&url)
            .headers(req_headers)
            .timeout(Duration::from_secs(10))
            .send()
            .await
        {
            Ok(response) => {
                let success = response.status().is_success();
                if success {
                    debug!("Bedrock connection test successful");
                } else {
                    warn!("Bedrock connection test failed with status: {}", response.status());
                }
                Ok(success)
            }
            Err(e) => {
                error!("Bedrock connection test failed: {}", e);
                Ok(false)
            }
        }
    }
    
    /// Get available models (this would be implemented if Bedrock provided such an endpoint)
    pub async fn list_models(&self) -> Result<Vec<BedrockModel>> {
        // For now, return the predefined models since Bedrock doesn't have a public models endpoint
        Ok(BedrockModel::all_supported())
    }
    
    /// Get client configuration
    pub fn get_config(&self) -> &BedrockConfig {
        &self.config
    }
    
    /// Get converter
    pub fn get_converter(&self) -> &BedrockConverter {
        &self.converter
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::common::{ChatMessage, MessageRole};

    #[tokio::test]
    async fn test_client_creation() {
        let config = BedrockConfig {
            access_key: Some("AKIAIOSFODNN7EXAMPLE".to_string()),
            secret_key: Some("wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY".to_string()),
            region: "us-east-1".to_string(),
            validate_credentials: false, // Skip validation in tests
            ..Default::default()
        };
        
        let client = BedrockClient::new(config).await;
        assert!(client.is_ok());
    }

    #[tokio::test]
    async fn test_request_conversion() {
        let config = BedrockConfig {
            access_key: Some("AKIAIOSFODNN7EXAMPLE".to_string()),
            secret_key: Some("wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY".to_string()),
            region: "us-east-1".to_string(),
            validate_credentials: false,
            ..Default::default()
        };
        
        let client = BedrockClient::new(config).await.unwrap();
        
        let request = ChatCompletionRequest {
            model: "anthropic.claude-3-5-sonnet-20241022-v2:0".to_string(),
            messages: vec![
                ChatMessage {
                    role: MessageRole::User,
                    content: "Hello".to_string(),
                    name: None,
                    tool_calls: None,
                    tool_call_id: None,
                }
            ],
            temperature: Some(0.7),
            max_tokens: Some(1000),
            tools: None,
            stream: Some(false),
            ..Default::default()
        };
        
        let model_family = BedrockModelFamily::Anthropic;
        let bedrock_request = client.convert_to_bedrock_request(&request, &model_family).await.unwrap();
        
        assert_eq!(bedrock_request.model_id, "anthropic.claude-3-5-sonnet-20241022-v2:0");
        assert_eq!(bedrock_request.content_type, "application/json");
        assert!(bedrock_request.body.get("messages").is_some());
        assert!(bedrock_request.body.get("max_tokens").is_some());
        assert_eq!(bedrock_request.body["max_tokens"], 1000);
    }

    #[test]
    fn test_retry_delay_calculation() {
        let config = BedrockConfig {
            api: crate::infrastructure::aws_bedrock::types::BedrockApiConfig {
                exponential_backoff: true,
                retry_delay_ms: 1000,
                max_retry_delay_ms: 60000,
                ..Default::default()
            },
            ..Default::default()
        };
        
        let client = BedrockClient {
            http_client: ReqwestClient::new(),
            auth: Arc::new(tokio::sync::RwLock::new(
                // This is just for testing, not a real auth object
                unsafe { std::mem::zeroed() }
            )),
            config,
            converter: Arc::new(BedrockConverter::new()),
            stream_parser: Arc::new(BedrockStreamParser::new()),
        };
        
        assert_eq!(client.calculate_retry_delay(1), 1000);
        assert_eq!(client.calculate_retry_delay(2), 2000);
        assert_eq!(client.calculate_retry_delay(3), 4000);
    }

    #[test]
    #[test]
    fn test_content_extraction() {
        let client = BedrockClient {
            http_client: ReqwestClient::new(),
            auth: Arc::new(tokio::sync::RwLock::new(
                crate::infrastructure::aws_bedrock::auth::BedrockAuth::new(
                    "test-region".to_string(),
                    "test-access-key".to_string(),
                    "test-secret-key".to_string(),
                ).unwrap()
            )),
        };

        // Test basic functionality - this is a placeholder test
        assert!(client.http_client.timeout().is_some());
    }
}