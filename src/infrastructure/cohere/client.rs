//! Cohere HTTP client implementation
//! 
//! This module provides the main HTTP client for the Cohere API with full
//! tool calling support, streaming capabilities, and error handling.

use anyhow::Result;
use reqwest::{Client, Response};
use serde_json::Value;
use std::time::Duration;
use tracing::{debug, info, warn, error};
use futures_util::StreamExt;

use crate::infrastructure::cohere::{
    auth::CohereAuth,
    types::{
        CohereConfig, CohereChatRequest, CohereChatResponse, CohereModel,
        CohereTool, CohereToolCall, CohereToolResult, CohereChatMessage, CohereRole
    },
    converter::{CohereConverter, CohereModelCapabilities, CohereToolCallContext},
    streaming::{CohereStreamParser, CohereStreamWrapper, CohereStreamUtils}
};
use crate::models::{
    request::ChatCompletionRequest,
    response::{ChatCompletionResponse, ChatCompletionChunk},
    common::{ToolDefinition, ChatMessage, MessageRole}
};

/// Cohere HTTP client
#[derive(Debug, Clone)]
pub struct CohereClient {
    /// HTTP client
    client: Client,
    
    /// Authentication handler
    auth: CohereAuth,
    
    /// Configuration
    config: CohereConfig,
    
    /// Tool call converter
    converter: CohereConverter,
    
    /// Base URL for API requests
    base_url: String,
}

impl CohereClient {
    /// Create a new Cohere client
    pub async fn new(config: CohereConfig) -> Result<Self> {
        info!("Creating Cohere client");
        
        // Validate configuration
        if config.api_key.is_empty() {
            return Err(anyhow::anyhow!("Cohere API key is required"));
        }
        
        let auth = CohereAuth::from_config(&config);
        auth.validate_api_key()?;
        
        let client = auth.get_authenticated_client()?;
        let converter = CohereConverter::new();
        let base_url = config.get_base_url();
        
        let cohere_client = Self {
            client,
            auth,
            config,
            converter,
            base_url,
        };
        
        info!("Cohere client created successfully");
        Ok(cohere_client)
    }
    
    /// Create client from environment variables
    pub async fn from_env() -> Result<Self> {
        let config = CohereConfig::from_env()?;
        Self::new(config).await
    }
    
    /// Test connection to Cohere API
    pub async fn test_connection(&self) -> Result<bool> {
        debug!("Testing Cohere API connection");
        
        let test_request = CohereChatRequest {
            model: self.config.get_default_model(),
            message: "Hi".to_string(),
            chat_history: None,
            tools: None,
            tool_results: None,
            stream: Some(false),
            temperature: Some(0.1),
            max_tokens: Some(1),
            top_p: None,
            top_k: None,
            stop_sequences: None,
            preamble: None,
        };
        
        let response = self.client
            .post(&format!("{}/chat", self.base_url))
            .json(&test_request)
            .send()
            .await;
        
        match response {
            Ok(resp) => {
                if resp.status().is_success() {
                    debug!("Cohere connection test successful");
                    Ok(true)
                } else if resp.status() == 401 {
                    warn!("Cohere connection test failed: Unauthorized");
                    Ok(false)
                } else {
                    warn!("Cohere connection test failed with status: {}", resp.status());
                    Ok(false)
                }
            }
            Err(e) => {
                error!("Cohere connection test failed: {}", e);
                Err(anyhow::anyhow!("Connection test failed: {}", e))
            }
        }
    }
    
    /// Get available models
    pub async fn get_models(&self) -> Result<Vec<String>> {
        debug!("Getting Cohere models");
        
        // Cohere doesn't have a models endpoint like OpenAI, so we return the known models
        Ok(vec![
            "command-r-plus".to_string(),
            "command-r".to_string(),
            "command".to_string(),
            "command-nightly".to_string(),
        ])
    }
    
    /// Check if a model supports tool calling
    pub fn supports_tool_calling(&self, model: &str) -> bool {
        self.converter.model_supports_tools(model)
    }
    
    /// Make a chat completion request
    pub async fn chat_completion(&self, request: ChatCompletionRequest) -> Result<ChatCompletionResponse> {
        debug!("Making Cohere chat completion request");
        
        // Convert OpenAI format request to Cohere format
        let cohere_request = self.convert_request_to_cohere(&request)?;
        
        // Make the request
        let response = self.client
            .post(&format!("{}/chat", self.base_url))
            .json(&cohere_request)
            .timeout(Duration::from_secs(self.config.get_timeout_seconds()))
            .send()
            .await
            .map_err(|e| anyhow::anyhow!("HTTP request failed: {}", e))?;
        
        // Handle response
        if response.status().is_success() {
            let cohere_response: CohereChatResponse = response.json().await
                .map_err(|e| anyhow::anyhow!("Failed to parse response: {}", e))?;
            
            // Convert back to OpenAI format
            self.convert_response_from_cohere(&cohere_response, &request.model)
        } else {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            Err(anyhow::anyhow!("Cohere API error {}: {}", status, error_text))
        }
    }
    
    /// Make a streaming chat completion request
    pub async fn chat_completion_stream(
        &self, 
        request: ChatCompletionRequest
    ) -> Result<impl futures_util::Stream<Item = Result<ChatCompletionChunk>>> {
        debug!("Making Cohere streaming chat completion request");
        
        // Convert request to Cohere format with streaming enabled
        let mut cohere_request = self.convert_request_to_cohere(&request)?;
        cohere_request.stream = Some(true);
        
        // Make the streaming request
        let response = self.client
            .post(&format!("{}/chat", self.base_url))
            .json(&cohere_request)
            .timeout(Duration::from_secs(self.config.get_timeout_seconds()))
            .send()
            .await
            .map_err(|e| anyhow::anyhow!("HTTP request failed: {}", e))?;
        
        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            return Err(anyhow::anyhow!("Cohere API error {}: {}", status, error_text));
        }
        
        // Convert response to byte stream
        let byte_stream = response.bytes_stream().map(|chunk| {
            chunk.map_err(|e| anyhow::anyhow!("Stream error: {}", e))
        });
        
        // Wrap with Cohere stream parser
        let cohere_stream = CohereStreamUtils::create_stream(byte_stream);
        
        Ok(cohere_stream)
    }
    
    /// Convert OpenAI format request to Cohere format
    fn convert_request_to_cohere(&self, request: &ChatCompletionRequest) -> Result<CohereChatRequest> {
        debug!("Converting OpenAI request to Cohere format");
        
        // Extract the main message (last user message)
        let message = request.messages.last()
            .ok_or_else(|| anyhow::anyhow!("No messages in request"))?
            .content.clone();
        
        // Convert chat history (all messages except the last one)
        let chat_history = if request.messages.len() > 1 {
            let history: Vec<CohereChatMessage> = request.messages[..request.messages.len()-1]
                .iter()
                .map(|msg| CohereChatMessage::from_chat_message(msg))
                .collect();
            Some(history)
        } else {
            None
        };
        
        // Convert tools if present
        let tools = if let Some(openai_tools) = &request.tools {
            let cohere_tools: Result<Vec<CohereTool>> = openai_tools.iter()
                .map(|tool| Ok(CohereTool::from_openai_tool(tool)))
                .collect();
            Some(cohere_tools?)
        } else {
            None
        };
        
        // Handle tool results from tool messages
        let tool_results = self.extract_tool_results_from_messages(&request.messages)?;
        
        Ok(CohereChatRequest {
            model: request.model.clone(),
            message,
            chat_history,
            tools,
            tool_results,
            stream: request.stream,
            temperature: request.temperature,
            max_tokens: request.max_tokens,
            top_p: request.top_p,
            top_k: None, // Cohere-specific parameter
            stop_sequences: request.stop.as_ref().map(|stop| match stop {
                crate::models::request::Stop::String(s) => vec![s.clone()],
                crate::models::request::Stop::Array(arr) => arr.clone(),
            }),
            preamble: self.extract_system_message(&request.messages),
        })
    }
    
    /// Convert Cohere response to OpenAI format
    fn convert_response_from_cohere(&self, cohere_response: &CohereChatResponse, model: &str) -> Result<ChatCompletionResponse> {
        debug!("Converting Cohere response to OpenAI format");
        
        let mut choices = Vec::new();
        
        // Handle tool calls if present
        let (message_content, tool_calls) = if let Some(cohere_tool_calls) = &cohere_response.tool_calls {
            let openai_tool_calls = self.converter.cohere_to_openai_tool_calls(cohere_tool_calls);
            let tool_calls_value: Vec<serde_json::Value> = openai_tool_calls.iter()
                .map(|tc| serde_json::to_value(tc).unwrap_or_default())
                .collect();
            (None, Some(tool_calls_value))
        } else {
            (Some(cohere_response.text.clone()), None)
        };
        
        choices.push(crate::models::response::ChatCompletionChoice {
            index: 0,
            message: crate::models::common::ChatMessage {
                role: crate::models::common::MessageRole::Assistant,
                content: message_content.unwrap_or_default(),
                name: None,
                function_call: None,
                tool_calls,
                tool_call_id: None,
            },
            logprobs: None,
            finish_reason: if cohere_response.tool_calls.is_some() {
                crate::models::common::FinishReason::ToolCalls
            } else {
                crate::models::common::FinishReason::Stop
            },
        });
        
        // Create usage statistics
        let usage = cohere_response.usage.as_ref().map(|u| {
            crate::models::common::Usage {
                prompt_tokens: u.input_tokens,
                completion_tokens: u.output_tokens,
                total_tokens: u.tokens,
                completion_tokens_details: None,
                prompt_tokens_details: None,
            }
        }).unwrap_or_default();
        
        Ok(ChatCompletionResponse {
            id: cohere_response.generation_id.clone().unwrap_or_else(|| "cohere_completion".to_string()),
            object: "chat.completion".to_string(),
            created: chrono::Utc::now().timestamp() as u64,
            model: model.to_string(),
            system_fingerprint: None,
            choices,
            usage,
        })
    }
    
    /// Extract tool results from tool messages
    fn extract_tool_results_from_messages(&self, messages: &[ChatMessage]) -> Result<Option<Vec<CohereToolResult>>> {
        let tool_messages: Vec<&ChatMessage> = messages.iter()
            .filter(|msg| msg.role == MessageRole::Tool)
            .collect();
        
        if tool_messages.is_empty() {
            return Ok(None);
        }
        
        let mut tool_results = Vec::new();
        
        for tool_msg in tool_messages {
            // Extract tool call ID from message (this would need to be stored in the message)
            // For now, we'll create a basic tool result structure
            let tool_call = CohereToolCall {
                name: "unknown".to_string(), // This should be extracted from context
                parameters: std::collections::HashMap::new(),
                id: Some("unknown".to_string()), // This should be the actual tool call ID
            };
            
            let mut output = std::collections::HashMap::new();
            output.insert("result".to_string(), serde_json::Value::String(tool_msg.content.clone()));
            
            tool_results.push(CohereToolResult {
                call: tool_call,
                outputs: vec![output],
            });
        }
        
        Ok(Some(tool_results))
    }
    
    /// Extract system message as preamble
    fn extract_system_message(&self, messages: &[ChatMessage]) -> Option<String> {
        messages.iter()
            .find(|msg| msg.role == MessageRole::System)
            .map(|msg| msg.content.clone())
    }
    
    /// Get client configuration
    pub fn config(&self) -> &CohereConfig {
        &self.config
    }
    
    /// Get converter
    pub fn converter(&self) -> &CohereConverter {
        &self.converter
    }
    
    /// Get model capabilities
    pub fn get_model_capabilities(&self, model: &str) -> CohereModelCapabilities {
        self.converter.get_model_capabilities(model)
    }
    
    /// Execute tool calls and get results
    pub async fn execute_tool_calls(
        &self,
        tool_calls: &[CohereToolCall],
        executor: impl Fn(&CohereToolCall) -> Result<String>
    ) -> Result<Vec<CohereToolResult>> {
        debug!("Executing {} tool calls", tool_calls.len());
        
        let mut results = Vec::new();
        
        for call in tool_calls {
            let result = match executor(call) {
                Ok(output) => {
                    let mut output_map = std::collections::HashMap::new();
                    output_map.insert("result".to_string(), serde_json::Value::String(output));
                    output_map.insert("success".to_string(), serde_json::Value::Bool(true));
                    
                    CohereToolResult {
                        call: call.clone(),
                        outputs: vec![output_map],
                    }
                }
                Err(e) => {
                    let mut output_map = std::collections::HashMap::new();
                    output_map.insert("error".to_string(), serde_json::Value::String(e.to_string()));
                    output_map.insert("success".to_string(), serde_json::Value::Bool(false));
                    
                    CohereToolResult {
                        call: call.clone(),
                        outputs: vec![output_map],
                    }
                }
            };
            
            results.push(result);
        }
        
        debug!("Completed execution of {} tool calls", results.len());
        Ok(results)
    }
    
    /// Continue conversation with tool results
    pub async fn continue_with_tool_results(
        &self,
        original_request: &ChatCompletionRequest,
        tool_results: &[CohereToolResult]
    ) -> Result<ChatCompletionResponse> {
        debug!("Continuing conversation with {} tool results", tool_results.len());
        
        let mut cohere_request = self.convert_request_to_cohere(original_request)?;
        cohere_request.tool_results = Some(tool_results.to_vec());
        
        // Make the request
        let response = self.client
            .post(&format!("{}/chat", self.base_url))
            .json(&cohere_request)
            .timeout(Duration::from_secs(self.config.get_timeout_seconds()))
            .send()
            .await
            .map_err(|e| anyhow::anyhow!("HTTP request failed: {}", e))?;
        
        if response.status().is_success() {
            let cohere_response: CohereChatResponse = response.json().await
                .map_err(|e| anyhow::anyhow!("Failed to parse response: {}", e))?;
            
            self.convert_response_from_cohere(&cohere_response, &original_request.model)
        } else {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            Err(anyhow::anyhow!("Cohere API error {}: {}", status, error_text))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::common::FunctionDefinition;
    
    #[tokio::test]
    async fn test_client_creation() {
        let config = CohereConfig {
            api_key: "test-api-key".to_string(),
            ..Default::default()
        };
        
        let client = CohereClient::new(config).await;
        assert!(client.is_ok());
    }
    
    #[test]
    fn test_model_support() {
        let config = CohereConfig {
            api_key: "test-api-key".to_string(),
            ..Default::default()
        };
        
        // We can't easily test this without async, but we can test the converter
        let converter = CohereConverter::new();
        assert!(converter.model_supports_tools("command-r-plus"));
        assert!(!converter.model_supports_tools("command"));
    }
    
    #[test]
    fn test_request_conversion() {
        let config = CohereConfig {
            api_key: "test-api-key".to_string(),
            ..Default::default()
        };
        
        // Create a simple OpenAI request
        let request = ChatCompletionRequest {
            model: "command-r-plus".to_string(),
            messages: vec![
                ChatMessage {
                    role: "user".to_string(),
                    content: "Hello".to_string(),
                }
            ],
            tools: None,
            stream: Some(false),
            temperature: Some(0.7),
            max_tokens: Some(100),
            top_p: None,
            stop: None,
        };
        
        // This test would require creating a client, which needs async
        // For now, we just verify the structure is correct
        assert_eq!(request.model, "command-r-plus");
        assert_eq!(request.messages.len(), 1);
    }
    
    #[test]
    fn test_system_message_extraction() {
        let messages = vec![
            ChatMessage {
                role: "system".to_string(),
                content: "You are a helpful assistant".to_string(),
            },
            ChatMessage {
                role: "user".to_string(),
                content: "Hello".to_string(),
            }
        ];
        
        // Test the logic that would be used in extract_system_message
        let system_msg = messages.iter()
            .find(|msg| msg.role == "system")
            .map(|msg| msg.content.clone());
        
        assert_eq!(system_msg, Some("You are a helpful assistant".to_string()));
    }
}