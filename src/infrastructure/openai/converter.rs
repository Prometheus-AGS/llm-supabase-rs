use anyhow::Result;
use serde_json::Value;
use std::collections::HashMap;
use tracing::{debug, trace};

use crate::infrastructure::common::tools::{ToolCallConverter, UnifiedToolCall, ToolCallResult};
use crate::models::request::ChatCompletionRequest;
use crate::models::response::ChatCompletionResponse;
use crate::models::common::{ToolDefinition, ChatMessage};

/// OpenAI tool call converter - leverages existing infrastructure
/// Since OpenAI format is the native format, this provides minimal conversion
#[derive(Debug, Clone)]
pub struct OpenAIConverter {
    /// The underlying OpenAI tool converter from common infrastructure
    tool_converter: crate::infrastructure::common::tools::OpenAIToolConverter,
}

impl OpenAIConverter {
    pub fn new() -> Self {
        Self {
            tool_converter: crate::infrastructure::common::tools::OpenAIToolConverter,
        }
    }

    /// Convert a ChatCompletionRequest to Value (pass-through for OpenAI)
    pub fn convert_request(&self, request: &ChatCompletionRequest) -> Result<Value> {
        debug!("Converting ChatCompletionRequest to OpenAI format (pass-through)");
        
        // Since this is already in OpenAI format, just serialize to Value
        let json = serde_json::to_value(request)?;
        
        trace!("Converted request: {}", serde_json::to_string_pretty(&json).unwrap_or_default());
        Ok(json)
    }

    /// Convert a Value response to ChatCompletionResponse
    pub fn convert_response(&self, response_value: Value) -> Result<ChatCompletionResponse> {
        debug!("Converting OpenAI response from Value");
        
        let response: ChatCompletionResponse = serde_json::from_value(response_value)?;
        
        trace!("Converted response with {} choices", response.choices.len());
        Ok(response)
    }

    /// Extract tool calls from a response using the common infrastructure
    pub fn extract_tool_calls(&self, response: &ChatCompletionResponse) -> Result<Vec<UnifiedToolCall>> {
        debug!("Extracting tool calls from ChatCompletionResponse");
        
        let mut unified_calls = Vec::new();

        for choice in &response.choices {
            if let Some(ref tool_calls) = choice.message.tool_calls {
                // Convert to Value and use the common converter
                let tool_calls_value = serde_json::to_value(tool_calls)?;
                let calls = self.tool_converter.provider_tool_calls_to_unified(&tool_calls_value)?;
                unified_calls.extend(calls);
            }
        }

        debug!("Extracted {} tool calls", unified_calls.len());
        Ok(unified_calls)
    }

    /// Convert tool definitions to OpenAI format (pass-through)
    pub fn convert_tools(&self, tools: &[ToolDefinition]) -> Result<Value> {
        debug!("Converting {} tools to OpenAI format", tools.len());
        
        // Use the common converter
        self.tool_converter.openai_tools_to_provider(tools)
    }

    /// Convert tool results to OpenAI message format
    pub fn convert_tool_results(&self, results: &[ToolCallResult]) -> Result<Vec<ChatMessage>> {
        debug!("Converting {} tool results to OpenAI messages", results.len());
        
        let mut messages = Vec::new();

        for result in results {
            let message = ChatMessage {
                role: crate::models::common::MessageRole::Tool,
                content: result.content.clone(),
                name: None,
                function_call: None,
                tool_calls: None,
                tool_call_id: Some(result.tool_call_id.clone()),
            };
            messages.push(message);
        }

        debug!("Created {} tool result messages", messages.len());
        Ok(messages)
    }

    /// Check if tool calls are present in the response
    pub fn has_tool_calls(&self, response: &ChatCompletionResponse) -> bool {
        response.choices.iter().any(|choice| {
            choice.message.tool_calls.as_ref()
                .map(|calls| !calls.is_empty())
                .unwrap_or(false)
        })
    }

    /// Get the finish reason from the response
    pub fn get_finish_reason(&self, response: &ChatCompletionResponse) -> Option<String> {
        response.choices.first()
            .and_then(|choice| choice.finish_reason.clone())
    }

    /// Check if the response indicates tool calls are needed
    pub fn needs_tool_calls(&self, response: &ChatCompletionResponse) -> bool {
        self.get_finish_reason(response)
            .map(|reason| reason == "tool_calls")
            .unwrap_or(false)
    }

    /// Convert unified tool calls back to OpenAI format for client response
    pub fn unified_to_openai_calls(&self, unified_calls: &[UnifiedToolCall]) -> Vec<Value> {
        debug!("Converting {} unified tool calls to OpenAI format", unified_calls.len());
        
        let openai_calls = self.tool_converter.unified_to_openai_tool_calls(unified_calls);
        
        // Convert to Value array for JSON serialization
        openai_calls.into_iter()
            .map(|call| serde_json::to_value(call).unwrap_or_default())
            .collect()
    }

    /// Create a chat message from tool call results for conversation continuation
    pub fn create_tool_message(&self, tool_call_id: &str, result: &str) -> ChatMessage {
        ChatMessage {
            role: crate::models::common::MessageRole::Tool,
            content: result.to_string(),
            name: None,
            function_call: None,
            tool_calls: None,
            tool_call_id: Some(tool_call_id.to_string()),
        }
    }

    /// Validate that a request is properly formatted for tool calling
    pub fn validate_tool_request(&self, request: &ChatCompletionRequest) -> Result<()> {
        // Check that tools are properly defined
        if let Some(ref tools) = request.tools {
            for tool in tools {
                if tool.tool_type != "function" {
                    return Err(anyhow::anyhow!(
                        "Unsupported tool type: {}. Only 'function' is supported.",
                        tool.tool_type
                    ));
                }
                
                if tool.function.name.is_empty() {
                    return Err(anyhow::anyhow!(
                        "Function name cannot be empty"
                    ));
                }
            }
        }

        // Validate tool choice if specified
        if let Some(ref tool_choice) = request.tool_choice {
            // OpenAI supports "auto", "none", or specific function selection
            if tool_choice.is_string() {
                let choice_str = tool_choice.as_str().unwrap_or("");
                if !matches!(choice_str, "auto" | "none") {
                    return Err(anyhow::anyhow!(
                        "Invalid tool_choice string: {}. Must be 'auto' or 'none'.",
                        choice_str
                    ));
                }
            }
            // Object format for specific function is also valid
        }

        Ok(())
    }

    /// Get model capabilities for tool calling
    pub fn get_model_capabilities(&self, model: &str) -> ModelCapabilities {
        match model {
            "gpt-4" | "gpt-4o" | "gpt-4o-mini" => ModelCapabilities {
                supports_tools: true,
                supports_parallel_tools: true,
                supports_streaming: true,
                max_tool_calls: Some(16),
            },
            "gpt-4-turbo" | "gpt-4-turbo-preview" => ModelCapabilities {
                supports_tools: true,
                supports_parallel_tools: true,
                supports_streaming: true,
                max_tool_calls: Some(16),
            },
            "gpt-3.5-turbo" => ModelCapabilities {
                supports_tools: true,
                supports_parallel_tools: false, // Legacy model, sequential only
                supports_streaming: true,
                max_tool_calls: Some(1),
            },
            _ => ModelCapabilities {
                supports_tools: false,
                supports_parallel_tools: false,
                supports_streaming: true,
                max_tool_calls: None,
            },
        }
    }
}

/// Model capabilities for tool calling
#[derive(Debug, Clone)]
pub struct ModelCapabilities {
    pub supports_tools: bool,
    pub supports_parallel_tools: bool,
    pub supports_streaming: bool,
    pub max_tool_calls: Option<u32>,
}

impl Default for OpenAIConverter {
    fn default() -> Self {
        Self::new()
    }
}

/// OpenAI tool call execution context
#[derive(Debug, Clone)]
pub struct ToolCallContext {
    pub request_id: String,
    pub model: String,
    pub tool_calls: Vec<UnifiedToolCall>,
    pub conversation_messages: Vec<ChatMessage>,
}

impl ToolCallContext {
    pub fn new(
        request_id: String,
        model: String,
        tool_calls: Vec<UnifiedToolCall>,
        conversation_messages: Vec<ChatMessage>,
    ) -> Self {
        Self {
            request_id,
            model,
            tool_calls,
            conversation_messages,
        }
    }

    /// Create continuation request with tool results
    pub fn create_continuation_request(
        &self,
        converter: &OpenAIConverter,
        tool_results: Vec<ToolCallResult>,
    ) -> Result<ChatCompletionRequest> {
        let mut messages = self.conversation_messages.clone();
        
        // Add tool result messages
        let result_messages = converter.convert_tool_results(&tool_results)?;
        messages.extend(result_messages);

        Ok(ChatCompletionRequest {
            model: self.model.clone(),
            messages: messages.into_iter().map(|msg| msg).collect(),
            // Preserve other settings from original request as needed
            ..Default::default()
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::common::{FunctionDefinition, MessageRole};
    use crate::models::response::{ChatCompletionChoice, ChatCompletionMessage};

    #[test]
    fn test_converter_creation() {
        let converter = OpenAIConverter::new();
        assert_eq!(std::mem::size_of_val(&converter), std::mem::size_of::<crate::infrastructure::common::tools::OpenAIToolConverter>());
    }

    #[test]
    fn test_convert_tools() {
        let converter = OpenAIConverter::new();
        
        let tools = vec![ToolDefinition {
            tool_type: "function".to_string(),
            function: FunctionDefinition {
                name: "get_weather".to_string(),
                description: Some("Get weather information".to_string()),
                parameters: Some(serde_json::json!({
                    "type": "object",
                    "properties": {
                        "location": {"type": "string"}
                    }
                })),
            },
        }];

        let result = converter.convert_tools(&tools);
        assert!(result.is_ok());
    }

    #[test]
    fn test_model_capabilities() {
        let converter = OpenAIConverter::new();
        
        let gpt4_caps = converter.get_model_capabilities("gpt-4o");
        assert!(gpt4_caps.supports_tools);
        assert!(gpt4_caps.supports_parallel_tools);
        assert!(gpt4_caps.supports_streaming);

        let gpt35_caps = converter.get_model_capabilities("gpt-3.5-turbo");
        assert!(gpt35_caps.supports_tools);
        assert!(!gpt35_caps.supports_parallel_tools);
        
        let unknown_caps = converter.get_model_capabilities("unknown-model");
        assert!(!unknown_caps.supports_tools);
    }

    #[test]
    fn test_validate_tool_request() {
        let converter = OpenAIConverter::new();
        
        // Valid request
        let valid_request = ChatCompletionRequest {
            model: "gpt-4o".to_string(),
            messages: vec![],
            tools: Some(vec![ToolDefinition {
                tool_type: "function".to_string(),
                function: FunctionDefinition {
                    name: "test_function".to_string(),
                    description: Some("Test".to_string()),
                    parameters: None,
                },
            }]),
            tool_choice: Some(serde_json::json!("auto")),
            ..Default::default()
        };
        
        assert!(converter.validate_tool_request(&valid_request).is_ok());

        // Invalid tool type
        let invalid_request = ChatCompletionRequest {
            model: "gpt-4o".to_string(),
            messages: vec![],
            tools: Some(vec![ToolDefinition {
                tool_type: "invalid".to_string(),
                function: FunctionDefinition {
                    name: "test_function".to_string(),
                    description: Some("Test".to_string()),
                    parameters: None,
                },
            }]),
            ..Default::default()
        };
        
        assert!(converter.validate_tool_request(&invalid_request).is_err());
    }

    #[test]
    fn test_create_tool_message() {
        let converter = OpenAIConverter::new();
        
        let message = converter.create_tool_message("call_123", "Test result");
        
        assert_eq!(message.role, MessageRole::Tool);
        assert_eq!(message.content, "Test result");
        assert_eq!(message.tool_call_id, Some("call_123".to_string()));
    }
}