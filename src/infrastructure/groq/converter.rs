//! Groq tool call converter
//!
//! This module provides tool calling conversion for Groq provider. Since Groq uses
//! OpenAI-compatible API format, most conversion is pass-through, leveraging the
//! existing OpenAI tool converter from common infrastructure.

use anyhow::Result;
use serde_json::Value;
use std::collections::HashMap;
use tracing::{debug, trace};

use crate::infrastructure::common::tools::{ToolCallConverter, UnifiedToolCall, ToolCallResult};
use crate::models::request::ChatCompletionRequest;
use crate::models::response::ChatCompletionResponse;
use crate::models::common::{ToolDefinition, ChatMessage};

use super::types::GroqModel;

/// Groq tool call converter - leverages existing OpenAI infrastructure
/// Since Groq format is OpenAI-compatible, this provides minimal conversion
#[derive(Debug, Clone)]
pub struct GroqConverter {
    /// The underlying OpenAI tool converter from common infrastructure
    tool_converter: crate::infrastructure::common::tools::OpenAIToolConverter,
}

impl GroqConverter {
    pub fn new() -> Self {
        Self {
            tool_converter: crate::infrastructure::common::tools::OpenAIToolConverter,
        }
    }

    /// Convert a ChatCompletionRequest to Value (pass-through for Groq)
    pub fn convert_request(&self, request: &ChatCompletionRequest) -> Result<Value> {
        debug!("Converting ChatCompletionRequest to Groq format (pass-through)");
        
        // Since Groq uses OpenAI-compatible format, just serialize to Value
        let json = serde_json::to_value(request)?;
        
        trace!("Converted request: {}", serde_json::to_string_pretty(&json).unwrap_or_default());
        Ok(json)
    }

    /// Convert a Value response to ChatCompletionResponse
    pub fn convert_response(&self, response_value: Value) -> Result<ChatCompletionResponse> {
        debug!("Converting Groq response from Value");
        
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

    /// Convert tool definitions to Groq format (pass-through, OpenAI-compatible)
    pub fn convert_tools(&self, tools: &[ToolDefinition]) -> Result<Value> {
        debug!("Converting {} tools to Groq format", tools.len());
        
        // Use the common converter (OpenAI format works for Groq)
        self.tool_converter.openai_tools_to_provider(tools)
    }

    /// Convert tool results to Groq message format
    pub fn convert_tool_results(&self, results: &[ToolCallResult]) -> Result<Vec<ChatMessage>> {
        debug!("Converting {} tool results to Groq messages", results.len());
        
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
            .map(|choice| format!("{:?}", choice.finish_reason))
    }

    /// Check if the response indicates tool calls are needed
    pub fn needs_tool_calls(&self, response: &ChatCompletionResponse) -> bool {
        self.get_finish_reason(response)
            .map(|reason| reason == "tool_calls")
            .unwrap_or(false)
    }

    /// Convert unified tool calls back to Groq format for client response
    pub fn unified_to_groq_calls(&self, unified_calls: &[UnifiedToolCall]) -> Vec<Value> {
        debug!("Converting {} unified tool calls to Groq format", unified_calls.len());
        
        // Since Groq uses OpenAI format, use the OpenAI converter
        let groq_calls = self.tool_converter.unified_to_openai_tool_calls(unified_calls);
        
        // Convert to Value array for JSON serialization
        groq_calls.into_iter()
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
            // Groq supports "auto", "none", or specific function selection (OpenAI-compatible)
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

    /// Validate model supports tool calling
    pub fn validate_model_for_tools(&self, model: &str) -> Result<()> {
        if !GroqModel::supports_tool_calling(model) {
            return Err(anyhow::anyhow!(
                "Model '{}' does not support tool calling. Use one of: {}",
                model,
                GroqModel::get_tool_capable_models()
                    .iter()
                    .map(|m| m.name.as_str())
                    .collect::<Vec<_>>()
                    .join(", ")
            ));
        }
        Ok(())
    }

    /// Get model capabilities for tool calling (Groq-specific)
    pub fn get_model_capabilities(&self, model: &str) -> ModelCapabilities {
        match model {
            // Llama 3.1 models - most capable
            "llama-3.1-70b-versatile" => ModelCapabilities {
                supports_tools: true,
                supports_parallel_tools: true,
                supports_streaming: true,
                max_tool_calls: Some(16),
                speed_tier: "very_fast".to_string(),
            },
            "llama-3.1-8b-instant" => ModelCapabilities {
                supports_tools: true,
                supports_parallel_tools: true,
                supports_streaming: true,
                max_tool_calls: Some(8), // Smaller model, fewer parallel calls
                speed_tier: "ultra_fast".to_string(),
            },
            
            // Mixtral models
            "mixtral-8x7b-32768" => ModelCapabilities {
                supports_tools: true,
                supports_parallel_tools: true,
                supports_streaming: true,
                max_tool_calls: Some(12),
                speed_tier: "fast".to_string(),
            },
            
            // Gemma models
            "gemma2-9b-it" => ModelCapabilities {
                supports_tools: true,
                supports_parallel_tools: false, // More limited
                supports_streaming: true,
                max_tool_calls: Some(4),
                speed_tier: "very_fast".to_string(),
            },
            
            // Older models without tool support
            "llama3-70b-8192" | "llama3-8b-8192" => ModelCapabilities {
                supports_tools: false,
                supports_parallel_tools: false,
                supports_streaming: true,
                max_tool_calls: None,
                speed_tier: "very_fast".to_string(),
            },
            
            // Unknown models
            _ => ModelCapabilities {
                supports_tools: false,
                supports_parallel_tools: false,
                supports_streaming: true,
                max_tool_calls: None,
                speed_tier: "unknown".to_string(),
            },
        }
    }

    /// Check if model supports parallel tool calls
    pub fn supports_parallel_tools(&self, model: &str) -> bool {
        self.get_model_capabilities(model).supports_parallel_tools
    }

    /// Get maximum number of tool calls for a model
    pub fn get_max_tool_calls(&self, model: &str) -> Option<u32> {
        self.get_model_capabilities(model).max_tool_calls
    }

    /// Optimize tool request for Groq (considering speed and capabilities)
    pub fn optimize_for_groq(&self, request: &mut ChatCompletionRequest, model: &str) -> Result<()> {
        let capabilities = self.get_model_capabilities(model);
        
        // If model doesn't support tools, remove them
        if !capabilities.supports_tools {
            request.tools = None;
            request.tool_choice = None;
            debug!("Removed tools from request - model '{}' doesn't support them", model);
            return Ok(());
        }
        
        // Limit parallel tool calls if model doesn't support them
        if !capabilities.supports_parallel_tools {
            if let Some(ref tool_choice) = request.tool_choice {
                // Force sequential tool calling by setting tool_choice to "auto"
                if tool_choice.is_object() {
                    request.tool_choice = Some(serde_json::Value::String("auto".to_string()));
                    debug!("Forced sequential tool calling for model '{}'", model);
                }
            }
        }
        
        // Limit number of tools if exceeds model capabilities
        if let Some(ref mut tools) = request.tools {
            if let Some(max_tools) = capabilities.max_tool_calls {
                if tools.len() > max_tools as usize {
                    tools.truncate(max_tools as usize);
                    debug!("Limited tools to {} for model '{}'", max_tools, model);
                }
            }
        }
        
        // Optimize for speed if using ultra-fast models
        if capabilities.speed_tier == "ultra_fast" {
            // Use lower temperature for faster, more predictable responses
            if request.temperature.is_none() || request.temperature.unwrap_or(1.0) > 0.3 {
                request.temperature = Some(0.1);
                debug!("Optimized temperature for ultra-fast model");
            }
        }
        
        Ok(())
    }
}

/// Groq model capabilities for tool calling
#[derive(Debug, Clone)]
pub struct ModelCapabilities {
    pub supports_tools: bool,
    pub supports_parallel_tools: bool,
    pub supports_streaming: bool,
    pub max_tool_calls: Option<u32>,
    pub speed_tier: String,
}

impl Default for GroqConverter {
    fn default() -> Self {
        Self::new()
    }
}

/// Groq tool call execution context
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
        converter: &GroqConverter,
        tool_results: Vec<ToolCallResult>,
    ) -> Result<ChatCompletionRequest> {
        let mut messages = self.conversation_messages.clone();
        
        // Add tool result messages
        let result_messages = converter.convert_tool_results(&tool_results)?;
        messages.extend(result_messages);

        let mut request = ChatCompletionRequest {
            model: self.model.clone(),
            messages: messages.into_iter().map(|msg| msg).collect(),
            // Preserve other settings from original request as needed
            ..Default::default()
        };
        
        // Optimize for Groq
        converter.optimize_for_groq(&mut request, &self.model)?;
        
        Ok(request)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::common::{FunctionDefinition, MessageRole};
    use crate::models::response::{ChatCompletionChoice, ChatCompletionMessage};

    #[test]
    fn test_converter_creation() {
        let converter = GroqConverter::new();
        assert_eq!(std::mem::size_of_val(&converter), std::mem::size_of::<crate::infrastructure::common::tools::OpenAIToolConverter>());
    }

    #[test]
    fn test_convert_tools() {
        let converter = GroqConverter::new();
        
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
        let converter = GroqConverter::new();
        
        // Test Llama 3.1 models
        let llama_70b = converter.get_model_capabilities("llama-3.1-70b-versatile");
        assert!(llama_70b.supports_tools);
        assert!(llama_70b.supports_parallel_tools);
        assert_eq!(llama_70b.speed_tier, "very_fast");
        
        let llama_8b = converter.get_model_capabilities("llama-3.1-8b-instant");
        assert!(llama_8b.supports_tools);
        assert_eq!(llama_8b.speed_tier, "ultra_fast");
        
        // Test older models
        let old_model = converter.get_model_capabilities("llama3-70b-8192");
        assert!(!old_model.supports_tools);
        
        // Test unknown model
        let unknown = converter.get_model_capabilities("unknown-model");
        assert!(!unknown.supports_tools);
    }

    #[test]
    fn test_model_validation() {
        let converter = GroqConverter::new();
        
        // Valid models
        assert!(converter.validate_model_for_tools("llama-3.1-70b-versatile").is_ok());
        assert!(converter.validate_model_for_tools("mixtral-8x7b-32768").is_ok());
        
        // Invalid models
        assert!(converter.validate_model_for_tools("llama3-70b-8192").is_err());
        assert!(converter.validate_model_for_tools("unknown-model").is_err());
    }

    #[test]
    fn test_groq_optimization() {
        let converter = GroqConverter::new();
        
        let mut request = ChatCompletionRequest {
            model: "llama-3.1-8b-instant".to_string(),
            temperature: Some(1.0),
            tools: Some(vec![
                ToolDefinition {
                    tool_type: "function".to_string(),
                    function: FunctionDefinition {
                        name: "test_function".to_string(),
                        description: None,
                        parameters: None,
                    },
                }
            ]),
            ..Default::default()
        };
        
        let result = converter.optimize_for_groq(&mut request, "llama-3.1-8b-instant");
        assert!(result.is_ok());
        
        // Should optimize temperature for ultra-fast model
        assert!(request.temperature.unwrap() < 1.0);
    }

    #[test]
    fn test_tool_request_validation() {
        let converter = GroqConverter::new();
        
        // Valid request
        let valid_request = ChatCompletionRequest {
            tools: Some(vec![ToolDefinition {
                tool_type: "function".to_string(),
                function: FunctionDefinition {
                    name: "valid_function".to_string(),
                    description: None,
                    parameters: None,
                },
            }]),
            ..Default::default()
        };
        
        assert!(converter.validate_tool_request(&valid_request).is_ok());
        
        // Invalid tool type
        let invalid_request = ChatCompletionRequest {
            tools: Some(vec![ToolDefinition {
                tool_type: "invalid_type".to_string(),
                function: FunctionDefinition {
                    name: "test".to_string(),
                    description: None,
                    parameters: None,
                },
            }]),
            ..Default::default()
        };
        
        assert!(converter.validate_tool_request(&invalid_request).is_err());
    }

    #[test]
    fn test_parallel_tools_support() {
        let converter = GroqConverter::new();
        
        assert!(converter.supports_parallel_tools("llama-3.1-70b-versatile"));
        assert!(converter.supports_parallel_tools("mixtral-8x7b-32768"));
        assert!(!converter.supports_parallel_tools("gemma2-9b-it"));
        assert!(!converter.supports_parallel_tools("llama3-70b-8192"));
    }
}