// src/infrastructure/vertex/vertex_converter.rs
//
// Production implementation for real tool calling with Vertex AI Claude models

use anyhow::Result;
use async_trait::async_trait;
use tracing::{warn, debug};

use crate::infrastructure::common::{
    ProviderConverter, ProviderErrorHandler,
    ToolCallManager,
    converter_utils::normalize_finish_reason,
};
use crate::infrastructure::common::AIProvider;
use crate::models::{
    request::ChatCompletionRequest,
    response::{ChatCompletionResponse, ChatCompletionChunk, ChatCompletionChunkChoice},
    common::{ChatMessage, MessageRole, FinishReason, Usage, ToolDefinition, FunctionDefinition},
};
use super::{
    VertexPredictRequest, VertexPredictResponse, VertexStreamChunk, VertexError,
    VertexMessage, VertexRole, VertexTool, VertexToolChoice, VertexContent,
};

/// Vertex AI implementation of the ProviderConverter trait
pub struct VertexAIConverter {
    /// Default anthropic version for Vertex AI
    anthropic_version: String,
    
    /// Tool call manager for handling Claude-style tool calls
    #[allow(dead_code)]
    tool_manager: ToolCallManager,
}

impl VertexAIConverter {
    pub fn new() -> Self {
        Self {
            anthropic_version: "vertex-2023-10-16".to_string(),
            tool_manager: ToolCallManager::claude(),
        }
    }

    /// Convert OpenAI messages to Vertex AI format
    fn convert_messages(&self, messages: &[ChatMessage]) -> Result<(Vec<VertexMessage>, Option<String>)> {
        let mut vertex_messages = Vec::new();
        let mut system_prompt = String::new();

        for message in messages {
            match message.role {
                MessageRole::System => {
                    if !system_prompt.is_empty() {
                        system_prompt.push('\n');
                    }
                    system_prompt.push_str(&message.content);
                }
                MessageRole::User => {
                    vertex_messages.push(VertexMessage {
                        role: VertexRole::User,
                        content: message.content.clone(),
                    });
                }
                MessageRole::Assistant => {
                    vertex_messages.push(VertexMessage {
                        role: VertexRole::Assistant,
                        content: message.content.clone(),
                    });
                }
                MessageRole::Function | MessageRole::Tool => {
                    // Convert tool/function results to user messages with proper formatting
                    let tool_result = if let Some(tool_call_id) = &message.tool_call_id {
                        format!("Tool result for {}: {}", tool_call_id, message.content)
                    } else {
                        format!("Function result: {}", message.content)
                    };
                    
                    vertex_messages.push(VertexMessage {
                        role: VertexRole::User,
                        content: tool_result,
                    });
                }
            }
        }

        let final_system = if system_prompt.is_empty() { None } else { Some(system_prompt) };
        Ok((vertex_messages, final_system))
    }

    /// Convert OpenAI tools to Vertex AI format
    fn convert_tools(&self, tools: &[ToolDefinition]) -> Result<Vec<VertexTool>> {
        let mut vertex_tools = Vec::new();
        
        for tool in tools {
            if tool.tool_type == "function" {
                let vertex_tool = VertexTool {
                    name: tool.function.name.clone(),
                    description: tool.function.description.clone().unwrap_or_default(),
                    input_schema: tool.function.parameters.clone().unwrap_or_else(|| {
                        serde_json::json!({
                            "type": "object",
                            "properties": {},
                            "required": []
                        })
                    }),
                };
                vertex_tools.push(vertex_tool);
            }
        }
        
        Ok(vertex_tools)
    }

    /// Convert legacy OpenAI functions to Vertex AI format
    fn convert_functions(&self, functions: &[FunctionDefinition]) -> Result<Vec<VertexTool>> {
        let mut vertex_tools = Vec::new();
        
        for function in functions {
            let vertex_tool = VertexTool {
                name: function.name.clone(),
                description: function.description.clone().unwrap_or_default(),
                input_schema: function.parameters.clone().unwrap_or_else(|| {
                    serde_json::json!({
                        "type": "object",
                        "properties": {},
                        "required": []
                    })
                }),
            };
            vertex_tools.push(vertex_tool);
        }
        
        Ok(vertex_tools)
    }

    /// Convert OpenAI tool_choice to Vertex AI format
    fn convert_tool_choice(&self, tool_choice: &serde_json::Value) -> Result<VertexToolChoice> {
        match tool_choice {
            serde_json::Value::String(s) => {
                match s.as_str() {
                    "auto" => Ok(VertexToolChoice::Auto {
                        tool_type: "auto".to_string()
                    }),
                    "required" => Ok(VertexToolChoice::Any {
                        tool_type: "any".to_string()
                    }),
                    "none" => {
                        // For "none", we'll return Auto but not include tools in the request
                        Ok(VertexToolChoice::Auto {
                            tool_type: "auto".to_string()
                        })
                    }
                    _ => Ok(VertexToolChoice::Auto {
                        tool_type: "auto".to_string()
                    }),
                }
            }
            serde_json::Value::Object(obj) => {
                if let Some(function_obj) = obj.get("function") {
                    if let Some(name) = function_obj.get("name").and_then(|n| n.as_str()) {
                        Ok(VertexToolChoice::Tool {
                            tool_type: "tool".to_string(),
                            name: name.to_string()
                        })
                    } else {
                        Ok(VertexToolChoice::Auto {
                            tool_type: "auto".to_string()
                        })
                    }
                } else {
                    Ok(VertexToolChoice::Auto {
                        tool_type: "auto".to_string()
                    })
                }
            }
            _ => Ok(VertexToolChoice::Auto {
                tool_type: "auto".to_string()
            }),
        }
    }

    /// Convert legacy function_call to Vertex AI format
    fn convert_function_call(&self, function_call: &serde_json::Value) -> Result<VertexToolChoice> {
        match function_call {
            serde_json::Value::String(s) => {
                match s.as_str() {
                    "auto" => Ok(VertexToolChoice::Auto {
                        tool_type: "auto".to_string()
                    }),
                    "none" => Ok(VertexToolChoice::Auto {
                        tool_type: "auto".to_string()
                    }), // Will not include tools
                    _ => Ok(VertexToolChoice::Auto {
                        tool_type: "auto".to_string()
                    }),
                }
            }
            serde_json::Value::Object(obj) => {
                if let Some(name) = obj.get("name").and_then(|n| n.as_str()) {
                    Ok(VertexToolChoice::Tool {
                        tool_type: "tool".to_string(),
                        name: name.to_string()
                    })
                } else {
                    Ok(VertexToolChoice::Auto {
                        tool_type: "auto".to_string()
                    })
                }
            }
            _ => Ok(VertexToolChoice::Auto {
                tool_type: "auto".to_string()
            }),
        }
    }

    /// Normalize model name for Vertex AI
    fn normalize_vertex_model(&self, model: &str) -> String {
        match model {
            "claude-4-sonnet" | "claude-4-sonnet-20250514" => "claude-sonnet-4@20250514".to_string(),
            "claude-3-5-haiku" => "claude-3-5-haiku@20241022".to_string(),
            "claude-3-5-sonnet" => "claude-3-5-sonnet@20241022".to_string(),
            _ => model.to_string(),
        }
    }

    /// Convert Vertex AI tool use to OpenAI tool calls format
    fn convert_tool_uses_to_openai(&self, tool_uses: &[&VertexContent]) -> Result<Vec<serde_json::Value>> {
        let mut tool_calls = Vec::new();
        
        for (index, tool_use) in tool_uses.iter().enumerate() {
            if let VertexContent::ToolUse { id, name, input } = tool_use {
                let tool_call = serde_json::json!({
                    "id": id,
                    "type": "function",
                    "function": {
                        "name": name,
                        "arguments": serde_json::to_string(input)?
                    }
                });
                tool_calls.push(tool_call);
            }
        }
        
        Ok(tool_calls)
    }

    /// Convert first tool use to legacy function_call format
    fn convert_first_tool_use_to_function_call(&self, tool_uses: &[&VertexContent]) -> Result<Option<serde_json::Value>> {
        if let Some(tool_use) = tool_uses.first() {
            if let VertexContent::ToolUse { name, input, .. } = tool_use {
                let function_call = serde_json::json!({
                    "name": name,
                    "arguments": serde_json::to_string(input)?
                });
                return Ok(Some(function_call));
            }
        }
        Ok(None)
    }

    /// Check if tools should be disabled based on tool_choice
    fn should_disable_tools(&self, request: &ChatCompletionRequest) -> bool {
        if let Some(tool_choice) = &request.tool_choice {
            if let serde_json::Value::String(s) = tool_choice {
                return s == "none";
            }
        }
        
        if let Some(function_call) = &request.function_call {
            if let serde_json::Value::String(s) = function_call {
                return s == "none";
            }
        }
        
        false
    }
}

#[async_trait]
impl ProviderConverter for VertexAIConverter {
    type ProviderRequest = VertexPredictRequest;
    type ProviderResponse = VertexPredictResponse;
    type ProviderStreamChunk = VertexStreamChunk;
    type ProviderError = VertexError;

    fn openai_to_provider_request(&self, request: &ChatCompletionRequest) -> Result<Self::ProviderRequest> {
        let (vertex_messages, system_prompt) = self.convert_messages(&request.messages)?;
        
        // Handle tools and tool_choice
        let (tools, tool_choice) = if self.should_disable_tools(request) {
            debug!("Tools disabled by tool_choice/function_call = 'none'");
            (None, None)
        } else if let Some(openai_tools) = &request.tools {
            debug!("Converting OpenAI tools: {:?}", openai_tools);
            let vertex_tools = self.convert_tools(openai_tools)?;
            let vertex_tool_choice = if let Some(tc) = &request.tool_choice {
                debug!("Converting tool_choice: {:?}", tc);
                Some(self.convert_tool_choice(tc)?)
            } else {
                debug!("No tool_choice specified, defaulting to Auto");
                Some(VertexToolChoice::Auto {
                    tool_type: "auto".to_string()
                })
            };
            (Some(vertex_tools), vertex_tool_choice)
        } else if let Some(openai_functions) = &request.functions {
            debug!("Converting OpenAI functions: {:?}", openai_functions);
            let vertex_tools = self.convert_functions(openai_functions)?;
            let vertex_tool_choice = if let Some(fc) = &request.function_call {
                debug!("Converting function_call: {:?}", fc);
                Some(self.convert_function_call(fc)?)
            } else {
                debug!("No function_call specified, defaulting to Auto");
                Some(VertexToolChoice::Auto {
                    tool_type: "auto".to_string()
                })
            };
            (Some(vertex_tools), vertex_tool_choice)
        } else {
            debug!("No tools or functions provided");
            (None, None)
        };

        debug!("Final converted tools: {:?}", tools);
        debug!("Final converted tool_choice: {:?}", tool_choice);

        let vertex_request = VertexPredictRequest {
            anthropic_version: self.anthropic_version.clone(),
            messages: vertex_messages,
            max_tokens: request.max_tokens.unwrap_or(16384),
            system: system_prompt,
            temperature: request.temperature,
            top_p: request.top_p,
            top_k: None,
            stream: false,
            stop_sequences: request.stop.as_ref().map(|stop| match stop {
                crate::models::request::Stop::String(s) => vec![s.clone()],
                crate::models::request::Stop::Array(arr) => arr.clone(),
            }),
            tools,
            tool_choice,
        };

        debug!("Complete Vertex AI request: {}", serde_json::to_string_pretty(&vertex_request)?);
        
        Ok(vertex_request)
    }

    fn openai_to_provider_streaming_request(&self, request: &ChatCompletionRequest) -> Result<Self::ProviderRequest> {
        let mut vertex_request = self.openai_to_provider_request(request)?;
        vertex_request.stream = true;
        Ok(vertex_request)
    }

    fn provider_to_openai_response(
        &self,
        response: &Self::ProviderResponse,
        request_id: &str,
        model: &str,
        original_request: &ChatCompletionRequest,
    ) -> Result<ChatCompletionResponse> {
        let text_content = response.get_text();
        let tool_uses = response.get_tool_uses();
        
        // Convert tool uses to OpenAI format
        let tool_calls = if !tool_uses.is_empty() {
            Some(self.convert_tool_uses_to_openai(&tool_uses)?)
        } else {
            None
        };
        
        // Convert first tool use to legacy function_call format if needed
        let function_call = if original_request.functions.is_some() && !tool_uses.is_empty() {
            self.convert_first_tool_use_to_function_call(&tool_uses)?
        } else {
            None
        };
        
        // Determine finish reason
        let finish_reason = if tool_calls.is_some() {
            if function_call.is_some() {
                FinishReason::FunctionCall
            } else {
                FinishReason::ToolCalls
            }
        } else {
            response.stop_reason.as_ref()
                .map(|r| normalize_finish_reason(r))
                .unwrap_or(FinishReason::Stop)
        };
        
        let message = ChatMessage {
            role: MessageRole::Assistant,
            content: text_content,
            name: None,
            function_call,
            tool_calls,
            tool_call_id: None,
        };

        let choice = crate::models::response::ChatCompletionChoice {
            index: 0,
            message,
            logprobs: None,
            finish_reason,
        };

        let usage = Usage {
            prompt_tokens: response.usage.input_tokens,
            completion_tokens: response.usage.output_tokens,
            total_tokens: response.usage.input_tokens + response.usage.output_tokens,
            prompt_tokens_details: None,
            completion_tokens_details: None,
        };

        Ok(ChatCompletionResponse::new(
            request_id.to_string(),
            model.to_string(),
            vec![choice],
            usage,
        ))
    }

    fn provider_chunk_to_openai_chunk(
        &self,
        chunk: &Self::ProviderStreamChunk,
        request_id: &str,
        model: &str,
    ) -> Result<Option<ChatCompletionChunk>> {
        // Handle text content
        if let Some(content) = chunk.get_content() {
            let delta = ChatMessage {
                role: MessageRole::Assistant,
                content,
                name: None,
                function_call: None,
                tool_calls: None,
                tool_call_id: None,
            };

            let choice = ChatCompletionChunkChoice {
                index: 0,
                delta,
                logprobs: None,
                finish_reason: None,
            };

            return Ok(Some(ChatCompletionChunk::new(
                request_id.to_string(),
                model.to_string(),
                vec![choice],
            )));
        }
        
        // Handle tool use chunks (for streaming tool calls)
        if let Some(content_array) = &chunk.content {
            for content in content_array {
                if let VertexContent::ToolUse { id, name, input } = content {
                    // Create streaming tool call delta
                    let tool_call_delta = serde_json::json!({
                        "id": id,
                        "type": "function",
                        "function": {
                            "name": name,
                            "arguments": serde_json::to_string(input)?
                        }
                    });
                    
                    // Also create legacy function_call format
                    let function_call_delta = serde_json::json!({
                        "name": name,
                        "arguments": serde_json::to_string(input)?
                    });
                    
                    let delta = ChatMessage {
                        role: MessageRole::Assistant,
                        content: String::new(),
                        name: None,
                        function_call: Some(function_call_delta),
                        tool_calls: Some(vec![tool_call_delta]),
                        tool_call_id: None,
                    };

                    let choice = ChatCompletionChunkChoice {
                        index: 0,
                        delta,
                        logprobs: None,
                        finish_reason: None,
                    };

                    return Ok(Some(ChatCompletionChunk::new(
                        request_id.to_string(),
                        model.to_string(),
                        vec![choice],
                    )));
                }
            }
        }
        
        // Handle finish reason
        if let Some(finish_reason) = chunk.get_finish_reason() {
            let openai_finish_reason = match finish_reason {
                super::types::VertexFinishReason::Stop => FinishReason::Stop,
                super::types::VertexFinishReason::MaxTokens => FinishReason::Length,
                _ => FinishReason::Stop,
            };
            
            let choice = ChatCompletionChunkChoice {
                index: 0,
                delta: ChatMessage {
                    role: MessageRole::Assistant,
                    content: String::new(),
                    name: None,
                    function_call: None,
                    tool_calls: None,
                    tool_call_id: None,
                },
                logprobs: None,
                finish_reason: Some(openai_finish_reason),
            };

            return Ok(Some(ChatCompletionChunk::new(
                request_id.to_string(),
                model.to_string(),
                vec![choice],
            )));
        }

        Ok(None)
    }

    fn provider_name(&self) -> &'static str {
        "vertex_ai"
    }

    fn supported_models(&self) -> Vec<String> {
        AIProvider::VertexAI.supported_models()
            .into_iter()
            .map(|s| s.to_string())
            .collect()
    }

    fn normalize_model_name(&self, model: &str) -> String {
        self.normalize_vertex_model(model)
    }
}

impl ProviderErrorHandler for VertexAIConverter {
    type ProviderError = VertexError;

    fn handle_provider_error(&self, error: Self::ProviderError) -> anyhow::Error {
        anyhow::anyhow!("Vertex AI error: {}", error.error.message)
    }

    fn is_retryable_error(&self, error: &Self::ProviderError) -> bool {
        error.error.error_type.contains("rate_limit") || 
        error.error.error_type.contains("server_error") ||
        error.error.message.contains("rate limit") ||
        error.error.message.contains("server error")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::common::MessageRole;

    #[test]
    fn test_vertex_converter_creation() {
        let converter = VertexAIConverter::new();
        assert_eq!(converter.provider_name(), "vertex_ai");
    }

    #[test]
    fn test_message_conversion() {
        let converter = VertexAIConverter::new();
        let messages = vec![
            ChatMessage {
                role: MessageRole::User,
                content: "Hello".to_string(),
                name: None,
                function_call: None,
                tool_calls: None,
                tool_call_id: None,
            }
        ];

        let (vertex_messages, system) = converter.convert_messages(&messages).unwrap();
        assert_eq!(vertex_messages.len(), 1);
        assert_eq!(vertex_messages[0].role, VertexRole::User);
        assert_eq!(vertex_messages[0].content, "Hello");
        assert!(system.is_none());
    }

    #[test]
    fn test_tool_conversion() {
        let converter = VertexAIConverter::new();
        let tools = vec![
            ToolDefinition {
                tool_type: "function".to_string(),
                function: FunctionDefinition {
                    name: "get_weather".to_string(),
                    description: Some("Get weather".to_string()),
                    parameters: Some(serde_json::json!({
                        "type": "object",
                        "properties": {
                            "location": {"type": "string"}
                        }
                    })),
                },
            }
        ];

        let vertex_tools = converter.convert_tools(&tools).unwrap();
        assert_eq!(vertex_tools.len(), 1);
        assert_eq!(vertex_tools[0].name, "get_weather");
        assert_eq!(vertex_tools[0].description, "Get weather");
    }

    #[test]
    fn test_model_normalization() {
        let converter = VertexAIConverter::new();
        assert_eq!(
            converter.normalize_vertex_model("claude-4-sonnet"),
            "claude-sonnet-4@20250514"
        );
    }
}