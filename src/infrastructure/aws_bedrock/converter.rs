//! AWS Bedrock multi-model tool call converter
//!
//! This module handles conversion between different Bedrock model formats and the unified
//! OpenAI-compatible format. It supports Anthropic Claude, Meta Llama, Mistral, and other
//! model families available through AWS Bedrock.

use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::HashMap;
use tracing::{debug, trace, warn};

use crate::infrastructure::common::tools::{ToolCallConverter, UnifiedToolCall, ToolCallResult};
use crate::models::common::ToolDefinition;
use crate::shared::types::{ToolCall, FunctionCall};
use super::types::{BedrockModelFamily, BedrockConfig};

/// Bedrock-specific tool call converter that handles multiple model families
#[derive(Debug, Clone)]
pub struct BedrockConverter {
    /// Model family configurations
    model_capabilities: HashMap<BedrockModelFamily, BedrockModelCapabilities>,
}

/// Model capabilities and format specifications for each Bedrock model family
#[derive(Debug, Clone)]
pub struct BedrockModelCapabilities {
    /// Whether this model family supports tool calling
    pub supports_tools: bool,
    
    /// Whether this model family supports streaming
    pub supports_streaming: bool,
    
    /// Tool call format used by this model family
    pub tool_format: BedrockToolFormat,
    
    /// Message format used by this model family
    pub message_format: BedrockMessageFormat,
}

/// Tool calling format variants for different model families
#[derive(Debug, Clone, PartialEq)]
pub enum BedrockToolFormat {
    /// Anthropic Claude format (similar to direct Claude API)
    Claude,
    
    /// Meta Llama format (function calling style)
    Llama,
    
    /// Mistral format (when supported)
    Mistral,
    
    /// Amazon Titan format (basic tool support)
    Titan,
    
    /// Unsupported tool calling
    Unsupported,
}

/// Message format variants for different model families
#[derive(Debug, Clone, PartialEq)]
pub enum BedrockMessageFormat {
    /// Anthropic messages format
    Anthropic,
    
    /// Meta Llama messages format
    Meta,
    
    /// Mistral messages format
    Mistral,
    
    /// Amazon Titan format
    Amazon,
}

/// Context for tool call conversion specific to Bedrock
#[derive(Debug, Clone)]
pub struct BedrockToolCallContext {
    /// Model family being used
    pub model_family: BedrockModelFamily,
    
    /// Model ID
    pub model_id: String,
    
    /// Whether streaming is enabled
    pub streaming: bool,
    
    /// Additional model-specific parameters
    pub model_params: HashMap<String, Value>,
}

impl BedrockConverter {
    /// Create a new Bedrock converter with default capabilities
    pub fn new() -> Self {
        let mut model_capabilities = HashMap::new();
        
        // Anthropic Claude models
        model_capabilities.insert(
            BedrockModelFamily::Anthropic,
            BedrockModelCapabilities {
                supports_tools: true,
                supports_streaming: true,
                tool_format: BedrockToolFormat::Claude,
                message_format: BedrockMessageFormat::Anthropic,
            },
        );
        
        // Meta Llama models
        model_capabilities.insert(
            BedrockModelFamily::Meta,
            BedrockModelCapabilities {
                supports_tools: true,
                supports_streaming: true,
                tool_format: BedrockToolFormat::Llama,
                message_format: BedrockMessageFormat::Meta,
            },
        );
        
        // Mistral models
        model_capabilities.insert(
            BedrockModelFamily::Mistral,
            BedrockModelCapabilities {
                supports_tools: false, // Most Mistral models don't support tools yet
                supports_streaming: true,
                tool_format: BedrockToolFormat::Unsupported,
                message_format: BedrockMessageFormat::Mistral,
            },
        );
        
        // Amazon Titan models
        model_capabilities.insert(
            BedrockModelFamily::Amazon,
            BedrockModelCapabilities {
                supports_tools: false,
                supports_streaming: true,
                tool_format: BedrockToolFormat::Unsupported,
                message_format: BedrockMessageFormat::Amazon,
            },
        );
        
        // Cohere models
        model_capabilities.insert(
            BedrockModelFamily::Cohere,
            BedrockModelCapabilities {
                supports_tools: false,
                supports_streaming: true,
                tool_format: BedrockToolFormat::Unsupported,
                message_format: BedrockMessageFormat::Amazon, // Similar to Amazon format
            },
        );
        
        Self {
            model_capabilities,
        }
    }
    
    /// Get capabilities for a model family
    pub fn get_capabilities(&self, family: &BedrockModelFamily) -> Option<&BedrockModelCapabilities> {
        self.model_capabilities.get(family)
    }
    
    /// Check if a model family supports tool calling
    pub fn supports_tools(&self, family: &BedrockModelFamily) -> bool {
        self.model_capabilities
            .get(family)
            .map(|caps| caps.supports_tools)
            .unwrap_or(false)
    }
    
    /// Convert OpenAI messages to Bedrock format for specific model family
    pub fn convert_messages_to_bedrock(
        &self,
        messages: &[crate::models::common::ChatMessage],
        family: BedrockModelFamily,
    ) -> Result<Value> {
        match family {
            BedrockModelFamily::Anthropic => self.convert_to_claude_messages(messages),
            BedrockModelFamily::Meta => self.convert_to_llama_messages(messages),
            BedrockModelFamily::Mistral => self.convert_to_mistral_messages(messages),
            BedrockModelFamily::Amazon => self.convert_to_titan_messages(messages),
            BedrockModelFamily::Cohere => self.convert_to_cohere_messages(messages),
            _ => Err(anyhow!("Unsupported model family: {:?}", family)),
        }
    }
    
    /// Convert to Claude/Anthropic message format
    fn convert_to_claude_messages(&self, messages: &[crate::models::common::ChatMessage]) -> Result<Value> {
        let mut claude_messages = Vec::new();
        let mut system_message = None;
        
        for message in messages {
            match message.role {
                crate::models::common::MessageRole::System => {
                    system_message = Some(message.content.clone());
                }
                crate::models::common::MessageRole::User => {
                    claude_messages.push(json!({
                        "role": "user",
                        "content": message.content
                    }));
                }
                crate::models::common::MessageRole::Assistant => {
                    let mut content = Vec::new();
                    
                    // Add text content if present
                    if !message.content.is_empty() {
                        content.push(json!({
                            "type": "text",
                            "text": message.content
                        }));
                    }
                    
                    // Add tool calls if present
                    if let Some(tool_calls) = &message.tool_calls {
                        for tool_call_value in tool_calls {
                            if let Ok(tool_call) = serde_json::from_value::<ToolCall>(tool_call_value.clone()) {
                                content.push(json!({
                                    "type": "tool_use",
                                    "id": tool_call.id,
                                    "name": tool_call.function.name,
                                    "input": serde_json::from_str::<Value>(&tool_call.function.arguments).unwrap_or_default()
                                }));
                            }
                        }
                    }
                    
                    claude_messages.push(json!({
                        "role": "assistant",
                        "content": content
                    }));
                }
                crate::models::common::MessageRole::Tool => {
                    // Find the corresponding tool call ID
                    if let Some(tool_call_id) = &message.tool_call_id {
                        claude_messages.push(json!({
                            "role": "user",
                            "content": [{
                                "type": "tool_result",
                                "tool_use_id": tool_call_id,
                                "content": message.content
                            }]
                        }));
                    }
                }
                crate::models::common::MessageRole::Function => {
                    // Function role is deprecated, treat as tool
                    if let Some(tool_call_id) = &message.tool_call_id {
                        claude_messages.push(json!({
                            "role": "user",
                            "content": [{
                                "type": "tool_result",
                                "tool_use_id": tool_call_id,
                                "content": message.content
                            }]
                        }));
                    }
                }
            }
        }
        
        let mut result = json!({
            "messages": claude_messages
        });
        
        if let Some(system) = system_message {
            result["system"] = json!(system);
        }
        
        Ok(result)
    }
    
    /// Convert to Llama message format
    fn convert_to_llama_messages(&self, messages: &[crate::models::common::ChatMessage]) -> Result<Value> {
        let mut llama_messages = Vec::new();
        
        for message in messages {
            match message.role {
                crate::models::common::MessageRole::System => {
                    llama_messages.push(json!({
                        "role": "system",
                        "content": message.content
                    }));
                }
                crate::models::common::MessageRole::User => {
                    llama_messages.push(json!({
                        "role": "user",
                        "content": message.content
                    }));
                }
                crate::models::common::MessageRole::Assistant => {
                    let mut assistant_message = json!({
                        "role": "assistant",
                        "content": message.content
                    });
                    
                    // Add function calls if present
                    if let Some(tool_calls) = &message.tool_calls {
                        let function_calls: Vec<Value> = tool_calls
                            .iter()
                            .filter_map(|tc_value| {
                                serde_json::from_value::<ToolCall>(tc_value.clone()).ok().map(|tc| {
                                    json!({
                                        "name": tc.function.name,
                                        "arguments": serde_json::from_str::<Value>(&tc.function.arguments).unwrap_or_default()
                                    })
                                })
                            })
                            .collect();
                            
                        assistant_message["function_calls"] = json!(function_calls);
                    }
                    
                    llama_messages.push(assistant_message);
                }
                crate::models::common::MessageRole::Tool => {
                    llama_messages.push(json!({
                        "role": "function",
                        "name": message.name.as_deref().unwrap_or("unknown"),
                        "content": message.content
                    }));
                }
                crate::models::common::MessageRole::Function => {
                    // Function role is deprecated, treat as tool
                    llama_messages.push(json!({
                        "role": "function",
                        "name": message.name.as_deref().unwrap_or("unknown"),
                        "content": message.content
                    }));
                }
            }
        }
        
        Ok(json!({
            "messages": llama_messages
        }))
    }
    
    /// Convert to Mistral message format
    fn convert_to_mistral_messages(&self, messages: &[crate::models::common::ChatMessage]) -> Result<Value> {
        // Mistral format is similar to OpenAI but without tool support currently
        let mistral_messages: Vec<Value> = messages
            .iter()
            .map(|msg| json!({
                "role": match msg.role {
                    crate::models::common::MessageRole::System => "system",
                    crate::models::common::MessageRole::User => "user",
                    crate::models::common::MessageRole::Assistant => "assistant",
                    crate::models::common::MessageRole::Tool => "assistant", // Map tool to assistant
                    crate::models::common::MessageRole::Function => "assistant", // Map function to assistant
                },
                "content": msg.content
            }))
            .collect();
            
        Ok(json!({
            "messages": mistral_messages
        }))
    }
    
    /// Convert to Amazon Titan message format
    fn convert_to_titan_messages(&self, messages: &[crate::models::common::ChatMessage]) -> Result<Value> {
        // Titan uses a simpler format
        let mut conversation = Vec::new();
        
        for message in messages {
            match message.role {
                crate::models::common::MessageRole::User => {
                    conversation.push(json!({
                        "role": "user",
                        "content": [{
                            "text": message.content
                        }]
                    }));
                }
                crate::models::common::MessageRole::Assistant => {
                    conversation.push(json!({
                        "role": "assistant", 
                        "content": [{
                            "text": message.content
                        }]
                    }));
                }
                _ => {
                    // Titan doesn't support system or tool messages well, convert to user
                    conversation.push(json!({
                        "role": "user",
                        "content": [{
                            "text": message.content
                        }]
                    }));
                }
            }
        }
        
        Ok(json!({
            "messages": conversation
        }))
    }
    
    /// Convert to Cohere message format
    fn convert_to_cohere_messages(&self, messages: &[crate::models::common::ChatMessage]) -> Result<Value> {
        // Similar to simple chat format
        let chat_history: Vec<Value> = messages
            .iter()
            .filter_map(|msg| {
                match msg.role {
                    crate::models::common::MessageRole::User => Some(json!({
                        "role": "USER",
                        "message": msg.content
                    })),
                    crate::models::common::MessageRole::Assistant => Some(json!({
                        "role": "CHATBOT",
                        "message": msg.content
                    })),
                    _ => None, // Skip system and tool messages
                }
            })
            .collect();
            
        Ok(json!({
            "chat_history": chat_history
        }))
    }
    
    /// Extract tool calls from Bedrock response based on model family
    pub fn extract_tool_calls_from_response(
        &self,
        response: &Value,
        family: BedrockModelFamily,
    ) -> Result<Vec<UnifiedToolCall>> {
        match family {
            BedrockModelFamily::Anthropic => self.extract_claude_tool_calls(response),
            BedrockModelFamily::Meta => self.extract_llama_tool_calls(response),
            _ => Ok(vec![]), // Other families don't support tools yet
        }
    }
    
    /// Extract tool calls from Claude response
    fn extract_claude_tool_calls(&self, response: &Value) -> Result<Vec<UnifiedToolCall>> {
        let mut tool_calls = Vec::new();
        
        if let Some(content) = response.get("content").and_then(|c| c.as_array()) {
            for item in content {
                if let Some(item_type) = item.get("type").and_then(|t| t.as_str()) {
                    if item_type == "tool_use" {
                        if let (Some(id), Some(name), Some(input)) = (
                            item.get("id").and_then(|i| i.as_str()),
                            item.get("name").and_then(|n| n.as_str()),
                            item.get("input")
                        ) {
                            tool_calls.push(UnifiedToolCall {
                                id: id.to_string(),
                                function_name: name.to_string(),
                                arguments: input.clone(),
                                metadata: HashMap::new(),
                            });
                        }
                    }
                }
            }
        }
        
        Ok(tool_calls)
    }
    
    /// Extract tool calls from Llama response
    fn extract_llama_tool_calls(&self, response: &Value) -> Result<Vec<UnifiedToolCall>> {
        let mut tool_calls = Vec::new();
        
        if let Some(function_calls) = response.get("function_calls").and_then(|fc| fc.as_array()) {
            for (index, call) in function_calls.iter().enumerate() {
                if let (Some(name), Some(args)) = (
                    call.get("name").and_then(|n| n.as_str()),
                    call.get("arguments")
                ) {
                    tool_calls.push(UnifiedToolCall {
                        id: format!("call_{}", index), // Generate ID for Llama
                        function_name: name.to_string(),
                        arguments: args.clone(),
                        metadata: HashMap::new(),
                    });
                }
            }
        }
        
        Ok(tool_calls)
    }
}

impl ToolCallConverter for BedrockConverter {
    /// Convert OpenAI tools to Bedrock format
    fn openai_tools_to_provider(&self, tools: &[ToolDefinition]) -> Result<Value> {
        // This will be used by individual model converters
        // Default to Claude format as it's most similar to OpenAI
        self.openai_tools_to_claude_format(tools)
    }
    
    /// Convert provider tool calls to unified format
    fn provider_tool_calls_to_unified(&self, provider_data: &Value) -> Result<Vec<UnifiedToolCall>> {
        // Try to detect the format from the data structure
        if provider_data.get("content").is_some() {
            // Looks like Claude format
            self.extract_claude_tool_calls(provider_data)
        } else if provider_data.get("function_calls").is_some() {
            // Looks like Llama format
            self.extract_llama_tool_calls(provider_data)
        } else {
            Ok(vec![])
        }
    }
    
    /// Convert unified tool calls to OpenAI format
    fn unified_to_openai_tool_calls(&self, unified_calls: &[UnifiedToolCall]) -> Vec<ToolCall> {
        unified_calls
            .iter()
            .map(|uc| ToolCall {
                id: uc.id.clone(),
                tool_type: "function".to_string(),
                function: FunctionCall {
                    name: uc.function_name.clone(),
                    arguments: serde_json::to_string(&uc.arguments).unwrap_or_default(),
                },
            })
            .collect()
    }
    
    /// Convert tool call results to provider format for continuation
    fn tool_results_to_provider_messages(&self, results: &[ToolCallResult]) -> Result<Value> {
        // Default to Claude format
        self.tool_results_to_claude_format(results)
    }
}

impl BedrockConverter {
    /// Convert OpenAI tools to Claude format
    fn openai_tools_to_claude_format(&self, tools: &[ToolDefinition]) -> Result<Value> {
        let claude_tools: Vec<Value> = tools
            .iter()
            .map(|tool| {
                json!({
                    "name": tool.function.name,
                    "description": tool.function.description,
                    "input_schema": tool.function.parameters
                })
            })
            .collect();
            
        Ok(json!(claude_tools))
    }
    
    /// Convert OpenAI tools to Llama format
    pub fn openai_tools_to_llama_format(&self, tools: &[ToolDefinition]) -> Result<Value> {
        let llama_tools: Vec<Value> = tools
            .iter()
            .map(|tool| {
                json!({
                    "type": "function",
                    "function": {
                        "name": tool.function.name,
                        "description": tool.function.description,
                        "parameters": tool.function.parameters
                    }
                })
            })
            .collect();
            
        Ok(json!(llama_tools))
    }
    
    /// Convert tool results to Claude format
    fn tool_results_to_claude_format(&self, results: &[ToolCallResult]) -> Result<Value> {
        let tool_results: Vec<Value> = results
            .iter()
            .map(|result| {
                json!({
                    "type": "tool_result",
                    "tool_use_id": result.tool_call_id,
                    "content": result.content,
                    "is_error": !result.success
                })
            })
            .collect();
            
        Ok(json!({
            "role": "user",
            "content": tool_results
        }))
    }
    
    /// Convert tool results to Llama format
    pub fn tool_results_to_llama_format(&self, results: &[ToolCallResult]) -> Result<Value> {
        let function_results: Vec<Value> = results
            .iter()
            .map(|result| {
                json!({
                    "role": "function",
                    "content": result.content
                })
            })
            .collect();
            
        Ok(json!(function_results))
    }
    
    /// Get the appropriate tool format for a model family
    pub fn get_tool_format_for_family(&self, family: &BedrockModelFamily, tools: &[ToolDefinition]) -> Result<Value> {
        if !self.supports_tools(family) {
            return Err(anyhow!("Model family {:?} does not support tool calling", family));
        }
        
        match family {
            BedrockModelFamily::Anthropic => self.openai_tools_to_claude_format(tools),
            BedrockModelFamily::Meta => self.openai_tools_to_llama_format(tools),
            _ => Err(anyhow!("Tool calling not implemented for family {:?}", family)),
        }
    }
    
    /// Create context for tool calling with specific model
    pub fn create_context(
        &self,
        model_family: BedrockModelFamily,
        model_id: String,
        streaming: bool,
    ) -> BedrockToolCallContext {
        BedrockToolCallContext {
            model_family,
            model_id,
            streaming,
            model_params: HashMap::new(),
        }
    }
}

impl Default for BedrockConverter {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::common::{ChatMessage, MessageRole, FunctionDefinition};

    #[test]
    fn test_converter_creation() {
        let converter = BedrockConverter::new();
        
        assert!(converter.supports_tools(&BedrockModelFamily::Anthropic));
        assert!(converter.supports_tools(&BedrockModelFamily::Meta));
        assert!(!converter.supports_tools(&BedrockModelFamily::Mistral));
    }

    #[test]
    fn test_claude_message_conversion() {
        let converter = BedrockConverter::new();
        
        let messages = vec![
            ChatMessage {
                role: MessageRole::System,
                content: "You are a helpful assistant".to_string(),
                name: None,
                tool_calls: None,
                tool_call_id: None,
            },
            ChatMessage {
                role: MessageRole::User,
                content: "Hello".to_string(),
                name: None,
                tool_calls: None,
                tool_call_id: None,
            },
        ];
        
        let result = converter.convert_to_claude_messages(&messages).unwrap();
        
        assert!(result.get("system").is_some());
        assert!(result.get("messages").is_some());
        
        let messages_array = result["messages"].as_array().unwrap();
        assert_eq!(messages_array.len(), 1);
        assert_eq!(messages_array[0]["role"], "user");
    }

    #[test]
    fn test_llama_message_conversion() {
        let converter = BedrockConverter::new();
        
        let messages = vec![
            ChatMessage {
                role: MessageRole::System,
                content: "You are a helpful assistant".to_string(),
                name: None,
                tool_calls: None,
                tool_call_id: None,
            },
            ChatMessage {
                role: MessageRole::User,
                content: "Hello".to_string(),
                name: None,
                tool_calls: None,
                tool_call_id: None,
            },
        ];
        
        let result = converter.convert_to_llama_messages(&messages).unwrap();
        
        let messages_array = result["messages"].as_array().unwrap();
        assert_eq!(messages_array.len(), 2);
        assert_eq!(messages_array[0]["role"], "system");
        assert_eq!(messages_array[1]["role"], "user");
    }

    #[test]
    fn test_tool_format_conversion() {
        let converter = BedrockConverter::new();
        
        let tools = vec![
            ToolDefinition {
                r#type: "function".to_string(),
                function: FunctionDefinition {
                    name: "test_function".to_string(),
                    description: Some("A test function".to_string()),
                    parameters: json!({
                        "type": "object",
                        "properties": {
                            "param": {"type": "string"}
                        }
                    }),
                },
            }
        ];
        
        // Test Claude format
        let claude_format = converter.get_tool_format_for_family(&BedrockModelFamily::Anthropic, &tools).unwrap();
        let claude_tools = claude_format.as_array().unwrap();
        assert_eq!(claude_tools[0]["name"], "test_function");
        assert!(claude_tools[0].get("input_schema").is_some());
        
        // Test Llama format
        let llama_format = converter.get_tool_format_for_family(&BedrockModelFamily::Meta, &tools).unwrap();
        let llama_tools = llama_format.as_array().unwrap();
        assert_eq!(llama_tools[0]["type"], "function");
        assert_eq!(llama_tools[0]["function"]["name"], "test_function");
    }

    #[test]
    fn test_claude_tool_call_extraction() {
        let converter = BedrockConverter::new();
        
        let response = json!({
            "content": [
                {
                    "type": "tool_use",
                    "id": "call_123",
                    "name": "test_function",
                    "input": {
                        "param": "value"
                    }
                }
            ]
        });
        
        let tool_calls = converter.extract_claude_tool_calls(&response).unwrap();
        assert_eq!(tool_calls.len(), 1);
        assert_eq!(tool_calls[0].id, "call_123");
        assert_eq!(tool_calls[0].function_name, "test_function");
    }

    #[test]
    fn test_llama_tool_call_extraction() {
        let converter = BedrockConverter::new();
        
        let response = json!({
            "function_calls": [
                {
                    "name": "test_function",
                    "arguments": {
                        "param": "value"
                    }
                }
            ]
        });
        
        let tool_calls = converter.extract_llama_tool_calls(&response).unwrap();
        assert_eq!(tool_calls.len(), 1);
        assert_eq!(tool_calls[0].function_name, "test_function");
        assert!(tool_calls[0].id.starts_with("call_"));
    }

    #[test]
    fn test_context_creation() {
        let converter = BedrockConverter::new();
        
        let context = converter.create_context(
            BedrockModelFamily::Anthropic,
            "anthropic.claude-3-5-sonnet-20241022-v2:0".to_string(),
            true,
        );
        
        assert_eq!(context.model_family, BedrockModelFamily::Anthropic);
        assert!(context.streaming);
    }

    #[test]
    fn test_unsupported_model_tool_calling() {
        let converter = BedrockConverter::new();
        
        let tools = vec![
            ToolDefinition {
                r#type: "function".to_string(),
                function: FunctionDefinition {
                    name: "test_function".to_string(),
                    description: Some("A test function".to_string()),
                    parameters: json!({}),
                },
            }
        ];
        
        let result = converter.get_tool_format_for_family(&BedrockModelFamily::Mistral, &tools);
        assert!(result.is_err());
    }
}