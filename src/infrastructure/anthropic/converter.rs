//! Anthropic tool calling format conversion implementation
//! 
//! This module handles conversion between OpenAI tool calling format and Anthropic's
//! native tool calling format, enabling seamless integration with existing infrastructure.

use anyhow::{anyhow, Context, Result};
use serde_json::{json, Value};
use std::collections::HashMap;
use tracing::{debug, warn};

use crate::infrastructure::common::tools::{ToolCallConverter, UnifiedToolCall, ToolCallResult};
use crate::models::common::{ToolDefinition, ChatMessage, MessageRole};
use crate::models::request::ChatCompletionRequest;
use crate::shared::types::{ToolCall, FunctionCall};

use super::types::{
    AnthropicMessagesRequest, AnthropicMessage, AnthropicContent, AnthropicContentBlock,
    AnthropicTool, AnthropicMessagesResponse
};

/// Anthropic-specific model capabilities
#[derive(Debug, Clone)]
pub struct AnthropicModelCapabilities {
    /// Whether the model supports tool calling
    pub supports_tools: bool,
    
    /// Whether the model supports streaming
    pub supports_streaming: bool,
    
    /// Maximum number of tokens the model can handle
    pub max_tokens: u32,
    
    /// Whether the model supports parallel tool calls
    pub supports_parallel_tools: bool,
    
    /// Whether the model supports image inputs
    pub supports_images: bool,
}

/// Context for tool call conversion and processing
#[derive(Debug, Clone)]
pub struct AnthropicToolCallContext {
    /// Current conversation ID
    pub conversation_id: Option<String>,
    
    /// Request ID for tracking
    pub request_id: String,
    
    /// Model being used
    pub model: String,
    
    /// Whether streaming is enabled
    pub streaming: bool,
    
    /// Additional metadata
    pub metadata: HashMap<String, Value>,
}

/// Anthropic tool call converter implementation
/// 
/// Converts between OpenAI and Anthropic tool calling formats using the
/// unified tool calling infrastructure.
#[derive(Debug, Clone)]
pub struct AnthropicConverter {
    /// Model capabilities cache
    capabilities_cache: HashMap<String, AnthropicModelCapabilities>,
}

impl Default for AnthropicConverter {
    fn default() -> Self {
        Self::new()
    }
}

impl AnthropicConverter {
    /// Create a new Anthropic converter
    pub fn new() -> Self {
        let mut converter = Self {
            capabilities_cache: HashMap::new(),
        };
        
        // Pre-populate known model capabilities
        converter.init_model_capabilities();
        converter
    }

    /// Initialize known model capabilities
    fn init_model_capabilities(&mut self) {
        let models = vec![
            ("claude-3-5-sonnet-20241022", AnthropicModelCapabilities {
                supports_tools: true,
                supports_streaming: true,
                max_tokens: 200000,
                supports_parallel_tools: true,
                supports_images: true,
            }),
            ("claude-3-5-haiku-20241022", AnthropicModelCapabilities {
                supports_tools: true,
                supports_streaming: true,
                max_tokens: 200000,
                supports_parallel_tools: true,
                supports_images: true,
            }),
            ("claude-3-opus-20240229", AnthropicModelCapabilities {
                supports_tools: true,
                supports_streaming: true,
                max_tokens: 200000,
                supports_parallel_tools: true,
                supports_images: true,
            }),
            ("claude-3-sonnet-20240229", AnthropicModelCapabilities {
                supports_tools: true,
                supports_streaming: true,
                max_tokens: 200000,
                supports_parallel_tools: false,
                supports_images: true,
            }),
            ("claude-3-haiku-20240307", AnthropicModelCapabilities {
                supports_tools: true,
                supports_streaming: true,
                max_tokens: 200000,
                supports_parallel_tools: false,
                supports_images: false,
            }),
        ];

        for (model, capabilities) in models {
            self.capabilities_cache.insert(model.to_string(), capabilities);
        }

        debug!("Initialized capabilities for {} models", self.capabilities_cache.len());
    }

    /// Get model capabilities
    pub fn get_model_capabilities(&self, model: &str) -> AnthropicModelCapabilities {
        self.capabilities_cache.get(model).cloned().unwrap_or_else(|| {
            warn!("Unknown model '{}', using default capabilities", model);
            AnthropicModelCapabilities {
                supports_tools: true,
                supports_streaming: true,
                max_tokens: 200000,
                supports_parallel_tools: true,
                supports_images: false,
            }
        })
    }

    /// Convert OpenAI ChatCompletionRequest to Anthropic MessagesRequest
    pub fn openai_request_to_anthropic(
        &self,
        request: &ChatCompletionRequest,
    ) -> Result<AnthropicMessagesRequest> {
        debug!("Converting OpenAI request to Anthropic format for model: {}", request.model);

        // Extract system message if present
        let mut system_message: Option<String> = None;
        let mut anthropic_messages = Vec::new();

        for message in &request.messages {
            match message.role.as_str() {
                "system" => {
                    if system_message.is_some() {
                        warn!("Multiple system messages found, concatenating");
                        system_message = Some(format!("{}\n\n{}", 
                            system_message.unwrap(), message.content));
                    } else {
                        system_message = Some(message.content.clone());
                    }
                }
                "user" | "assistant" => {
                    anthropic_messages.push(AnthropicMessage {
                        role: message.role.to_string(),
                        content: AnthropicContent::Text(message.content.clone()),
                    });
                }
                "tool" => {
                    // Convert tool results to Anthropic format
                    if let Some(tool_call_id) = &message.tool_call_id {
                        anthropic_messages.push(AnthropicMessage {
                            role: "user".to_string(),
                            content: AnthropicContent::Blocks(vec![
                                AnthropicContentBlock::ToolResult {
                                    tool_use_id: tool_call_id.clone(),
                                    content: message.content.clone(),
                                    is_error: None, // Could be enhanced to detect errors
                                }
                            ]),
                        });
                    } else {
                        warn!("Tool message without tool_call_id, treating as regular user message");
                        anthropic_messages.push(AnthropicMessage {
                            role: "user".to_string(),
                            content: AnthropicContent::Text(message.content.clone()),
                        });
                    }
                }
                _ => {
                    warn!("Unknown message role: {}, treating as user message", message.role);
                    anthropic_messages.push(AnthropicMessage {
                        role: "user".to_string(),
                        content: AnthropicContent::Text(message.content.clone()),
                    });
                }
            }
        }

        // Convert tools if present
        let anthropic_tools = if let Some(tools) = &request.tools {
            Some(self.openai_tools_to_anthropic_tools(tools)?)
        } else {
            None
        };

        // Determine max_tokens (Anthropic requires this field)
        let max_tokens = request.max_tokens.unwrap_or_else(|| {
            let capabilities = self.get_model_capabilities(&request.model);
            // Use a reasonable default (e.g., 4096 tokens)
            std::cmp::min(4096, capabilities.max_tokens)
        });

        let anthropic_request = AnthropicMessagesRequest {
            model: request.model.clone(),
            max_tokens,
            messages: anthropic_messages,
            system: system_message,
            tools: anthropic_tools,
            stream: request.stream,
            temperature: request.temperature.map(|t| t as f32),
            top_p: request.top_p.map(|t| t as f32),
            top_k: None, // OpenAI doesn't have top_k, could be derived from other params
            stop_sequences: request.stop.as_ref().map(|stop| {
                match stop {
                    crate::models::request::Stop::String(s) => vec![s.clone()],
                    crate::models::request::Stop::Array(arr) => arr.clone(),
                }
            }),
            metadata: None,
        };

        debug!("Successfully converted OpenAI request to Anthropic format");
        Ok(anthropic_request)
    }

    /// Convert OpenAI tools to Anthropic tools format
    fn openai_tools_to_anthropic_tools(&self, tools: &[ToolDefinition]) -> Result<Vec<AnthropicTool>> {
        let mut anthropic_tools = Vec::new();

        for tool in tools {
            let anthropic_tool = AnthropicTool {
                name: tool.function.name.clone(),
                description: tool.function.description.clone().unwrap_or_else(|| {
                    format!("Tool: {}", tool.function.name)
                }),
                input_schema: tool.function.parameters.clone().unwrap_or_else(|| {
                    serde_json::json!({
                        "type": "object",
                        "properties": {},
                        "required": []
                    })
                }),
            };
            anthropic_tools.push(anthropic_tool);
        }

        debug!("Converted {} OpenAI tools to Anthropic format", tools.len());
        Ok(anthropic_tools)
    }

    /// Convert Anthropic response to OpenAI format
    pub fn anthropic_response_to_openai(
        &self,
        response: &AnthropicMessagesResponse,
        context: &AnthropicToolCallContext,
    ) -> Result<crate::models::response::ChatCompletionResponse> {
        debug!("Converting Anthropic response to OpenAI format");

        // Extract content and tool calls
        let mut content_parts = Vec::new();
        let mut tool_calls = Vec::new();

        for content_block in &response.content {
            match content_block {
                AnthropicContentBlock::Text { text } => {
                    content_parts.push(text.clone());
                }
                AnthropicContentBlock::ToolUse { id, name, input } => {
                    // Convert to OpenAI tool call format
                    let arguments_str = serde_json::to_string(input)
                        .context("Failed to serialize tool call arguments")?;
                    
                    tool_calls.push(ToolCall {
                        id: id.clone(),
                        tool_type: "function".to_string(),
                        function: FunctionCall {
                            name: name.clone(),
                            arguments: arguments_str,
                        },
                    });
                }
                AnthropicContentBlock::ToolResult { .. } => {
                    // Tool results are typically not in assistant responses
                    warn!("Unexpected tool result in assistant response");
                }
                AnthropicContentBlock::Image { .. } => {
                    // Images in responses would need special handling
                    warn!("Image content in assistant response not yet supported");
                }
            }
        }

        // Combine text content
        let content = if content_parts.is_empty() {
            if !tool_calls.is_empty() {
                None // When there are tool calls but no text
            } else {
                Some("".to_string()) // Empty response
            }
        } else {
            Some(content_parts.join("\n"))
        };

        // Determine finish reason
        let finish_reason = match response.stop_reason.as_deref() {
            Some("end_turn") => "stop",
            Some("max_tokens") => "length",
            Some("tool_use") => "tool_calls",
            Some("stop_sequence") => "stop",
            _ => "stop",
        }.to_string();

        // Create the choice
        let choice = crate::models::response::ChatCompletionChoice {
            index: 0,
            message: crate::models::common::ChatMessage {
                role: crate::models::common::MessageRole::Assistant,
                content: content.unwrap_or_default(),
                name: None,
                function_call: None,
                tool_calls: if tool_calls.is_empty() { 
                    None 
                } else { 
                    Some(tool_calls.into_iter().map(|tc| {
                        serde_json::to_value(tc).unwrap_or_else(|_| serde_json::Value::Null)
                    }).collect())
                },
                tool_call_id: None,
            },
            logprobs: None,
            finish_reason: match finish_reason.as_str() {
                "stop" => crate::models::common::FinishReason::Stop,
                "length" => crate::models::common::FinishReason::Length,
                "tool_calls" => crate::models::common::FinishReason::ToolCalls,
                "function_call" => crate::models::common::FinishReason::FunctionCall,
                "content_filter" => crate::models::common::FinishReason::ContentFilter,
                _ => crate::models::common::FinishReason::Stop,
            },
        };

        // Create usage information
        let usage = crate::models::common::Usage {
            prompt_tokens: response.usage.input_tokens,
            completion_tokens: response.usage.output_tokens,
            total_tokens: response.usage.input_tokens + response.usage.output_tokens,
            completion_tokens_details: None,
            prompt_tokens_details: None,
        };

        let openai_response = crate::models::response::ChatCompletionResponse {
            id: response.id.clone(),
            object: "chat.completion".to_string(),
            created: chrono::Utc::now().timestamp() as u64,
            model: response.model.clone(),
            choices: vec![choice],
            usage,
            system_fingerprint: None,
        };

        debug!("Successfully converted Anthropic response to OpenAI format");
        Ok(openai_response)
    }

    /// Extract tool calls from Anthropic response content
    pub fn extract_tool_calls_from_response(
        &self,
        response: &AnthropicMessagesResponse,
    ) -> Result<Vec<UnifiedToolCall>> {
        let mut unified_calls = Vec::new();

        for content_block in &response.content {
            if let AnthropicContentBlock::ToolUse { id, name, input } = content_block {
                unified_calls.push(UnifiedToolCall {
                    id: id.clone(),
                    function_name: name.clone(),
                    arguments: input.clone(),
                    metadata: HashMap::new(),
                });
            }
        }

        debug!("Extracted {} tool calls from Anthropic response", unified_calls.len());
        Ok(unified_calls)
    }

    /// Create Anthropic message from tool call results
    pub fn create_tool_result_message(
        &self,
        results: &[ToolCallResult],
    ) -> Result<AnthropicMessage> {
        let mut content_blocks = Vec::new();

        for result in results {
            content_blocks.push(AnthropicContentBlock::ToolResult {
                tool_use_id: result.tool_call_id.clone(),
                content: result.content.clone(),
                is_error: Some(!result.success),
            });
        }

        Ok(AnthropicMessage {
            role: "user".to_string(),
            content: AnthropicContent::Blocks(content_blocks),
        })
    }

    /// Check if a model supports a specific capability
    pub fn model_supports_capability(&self, model: &str, capability: &str) -> bool {
        let capabilities = self.get_model_capabilities(model);
        
        match capability {
            "tools" => capabilities.supports_tools,
            "streaming" => capabilities.supports_streaming,
            "parallel_tools" => capabilities.supports_parallel_tools,
            "images" => capabilities.supports_images,
            _ => false,
        }
    }

    /// Get the maximum number of tokens for a model
    pub fn get_max_tokens(&self, model: &str) -> u32 {
        self.get_model_capabilities(model).max_tokens
    }
}

impl ToolCallConverter for AnthropicConverter {
    /// Convert OpenAI tools to Anthropic format
    fn openai_tools_to_provider(&self, tools: &[ToolDefinition]) -> Result<Value> {
        let anthropic_tools = self.openai_tools_to_anthropic_tools(tools)?;
        serde_json::to_value(anthropic_tools).context("Failed to serialize Anthropic tools")
    }

    /// Convert Anthropic tool calls to unified format
    fn provider_tool_calls_to_unified(&self, provider_data: &Value) -> Result<Vec<UnifiedToolCall>> {
        let mut unified_calls = Vec::new();

        // Handle both single response and array of content blocks
        let content_blocks = if let Some(array) = provider_data.as_array() {
            array.clone()
        } else if let Some(content) = provider_data.get("content").and_then(|c| c.as_array()) {
            content.clone()
        } else {
            return Ok(unified_calls);
        };

        for content_block in content_blocks {
            if let Some(tool_use) = content_block.get("tool_use") {
                if let (Some(id), Some(name), Some(input)) = (
                    tool_use.get("id").and_then(|v| v.as_str()),
                    tool_use.get("name").and_then(|v| v.as_str()),
                    tool_use.get("input")
                ) {
                    unified_calls.push(UnifiedToolCall {
                        id: id.to_string(),
                        function_name: name.to_string(),
                        arguments: input.clone(),
                        metadata: HashMap::new(),
                    });
                }
            } else if content_block.get("type").and_then(|t| t.as_str()) == Some("tool_use") {
                // Direct tool_use block format
                if let (Some(id), Some(name), Some(input)) = (
                    content_block.get("id").and_then(|v| v.as_str()),
                    content_block.get("name").and_then(|v| v.as_str()),
                    content_block.get("input")
                ) {
                    unified_calls.push(UnifiedToolCall {
                        id: id.to_string(),
                        function_name: name.to_string(),
                        arguments: input.clone(),
                        metadata: HashMap::new(),
                    });
                }
            }
        }

        debug!("Converted {} Anthropic tool calls to unified format", unified_calls.len());
        Ok(unified_calls)
    }

    /// Convert unified tool calls to OpenAI format
    fn unified_to_openai_tool_calls(&self, unified_calls: &[UnifiedToolCall]) -> Vec<ToolCall> {
        unified_calls.iter().map(|uc| {
            let arguments_str = if uc.arguments.is_string() {
                uc.arguments.as_str().unwrap_or("{}").to_string()
            } else {
                serde_json::to_string(&uc.arguments).unwrap_or_else(|_| "{}".to_string())
            };

            ToolCall {
                id: uc.id.clone(),
                tool_type: "function".to_string(),
                function: FunctionCall {
                    name: uc.function_name.clone(),
                    arguments: arguments_str,
                },
            }
        }).collect()
    }

    /// Convert tool call results to Anthropic message format
    fn tool_results_to_provider_messages(&self, results: &[ToolCallResult]) -> Result<Value> {
        let content_blocks: Vec<Value> = results.iter().map(|result| {
            json!({
                "type": "tool_result",
                "tool_use_id": result.tool_call_id,
                "content": result.content,
                "is_error": !result.success
            })
        }).collect();

        Ok(json!({
            "role": "user",
            "content": content_blocks
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::common::{FunctionDefinition, ToolDefinition};

    fn create_test_converter() -> AnthropicConverter {
        AnthropicConverter::new()
    }

    fn create_test_openai_request() -> ChatCompletionRequest {
        ChatCompletionRequest {
            model: "claude-3-5-sonnet-20241022".to_string(),
            messages: vec![
                ChatMessage {
                    role: MessageRole::System,
                    content: "You are a helpful assistant.".to_string(),
                    name: None,
                    function_call: None,
                    tool_call_id: None,
                    tool_calls: None,
                },
                ChatMessage {
                    role: MessageRole::User,
                    content: "Hello, can you help me?".to_string(),
                    name: None,
                    function_call: None,
                    tool_call_id: None,
                    tool_calls: None,
                },
            ],
            max_tokens: Some(1000),
            temperature: Some(0.7),
            top_p: Some(0.9),
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
            tools: Some(vec![
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
            ]),
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

    #[test]
    fn test_converter_creation() {
        let converter = create_test_converter();
        assert!(!converter.capabilities_cache.is_empty());
    }

    #[test]
    fn test_model_capabilities() {
        let converter = create_test_converter();
        
        let sonnet_caps = converter.get_model_capabilities("claude-3-5-sonnet-20241022");
        assert!(sonnet_caps.supports_tools);
        assert!(sonnet_caps.supports_streaming);
        assert!(sonnet_caps.supports_parallel_tools);
        
        let unknown_caps = converter.get_model_capabilities("unknown-model");
        assert!(unknown_caps.supports_tools); // Should use defaults
    }

    #[test]
    fn test_openai_request_conversion() {
        let converter = create_test_converter();
        let openai_request = create_test_openai_request();
        
        let result = converter.openai_request_to_anthropic(&openai_request);
        assert!(result.is_ok());
        
        let anthropic_request = result.unwrap();
        assert_eq!(anthropic_request.model, "claude-3-5-sonnet-20241022");
        assert_eq!(anthropic_request.max_tokens, 1000);
        assert_eq!(anthropic_request.system, Some("You are a helpful assistant.".to_string()));
        assert_eq!(anthropic_request.messages.len(), 1); // User message only
        assert!(anthropic_request.tools.is_some());
        assert_eq!(anthropic_request.tools.unwrap().len(), 1);
    }

    #[test]
    fn test_tools_conversion() {
        let converter = create_test_converter();
        
        let openai_tools = vec![
            ToolDefinition {
                tool_type: "function".to_string(),
                function: FunctionDefinition {
                    name: "test_function".to_string(),
                    description: Some("A test function".to_string()),
                    parameters: json!({"type": "object"}),
                },
            }
        ];
        
        let result = converter.openai_tools_to_provider(&openai_tools);
        assert!(result.is_ok());
        
        let anthropic_tools_value = result.unwrap();
        assert!(anthropic_tools_value.is_array());
        
        let tools_array = anthropic_tools_value.as_array().unwrap();
        assert_eq!(tools_array.len(), 1);
        
        let tool = &tools_array[0];
        assert_eq!(tool["name"], "test_function");
        assert_eq!(tool["description"], "A test function");
    }

    #[test]
    fn test_unified_tool_calls_conversion() {
        let converter = create_test_converter();
        
        let provider_data = json!([
            {
                "type": "tool_use",
                "id": "test-id-1",
                "name": "get_weather",
                "input": {
                    "location": "San Francisco"
                }
            }
        ]);
        
        let result = converter.provider_tool_calls_to_unified(&provider_data);
        assert!(result.is_ok());
        
        let unified_calls = result.unwrap();
        assert_eq!(unified_calls.len(), 1);
        
        let call = &unified_calls[0];
        assert_eq!(call.id, "test-id-1");
        assert_eq!(call.function_name, "get_weather");
        assert_eq!(call.arguments["location"], "San Francisco");
    }

    #[test]
    fn test_tool_results_conversion() {
        let converter = create_test_converter();
        
        let results = vec![
            ToolCallResult {
                tool_call_id: "test-id-1".to_string(),
                content: "Weather is sunny".to_string(),
                success: true,
                error: None,
            }
        ];
        
        let result = converter.tool_results_to_provider_messages(&results);
        assert!(result.is_ok());
        
        let message_value = result.unwrap();
        assert_eq!(message_value["role"], "user");
        
        let content = message_value["content"].as_array().unwrap();
        assert_eq!(content.len(), 1);
        
        let tool_result = &content[0];
        assert_eq!(tool_result["type"], "tool_result");
        assert_eq!(tool_result["tool_use_id"], "test-id-1");
        assert_eq!(tool_result["content"], "Weather is sunny");
        assert_eq!(tool_result["is_error"], false);
    }

    #[test]
    fn test_capability_checks() {
        let converter = create_test_converter();
        
        assert!(converter.model_supports_capability("claude-3-5-sonnet-20241022", "tools"));
        assert!(converter.model_supports_capability("claude-3-5-sonnet-20241022", "streaming"));
        assert!(converter.model_supports_capability("claude-3-5-sonnet-20241022", "parallel_tools"));
        assert!(!converter.model_supports_capability("claude-3-5-sonnet-20241022", "unknown"));
        
        assert_eq!(converter.get_max_tokens("claude-3-5-sonnet-20241022"), 200000);
    }
}