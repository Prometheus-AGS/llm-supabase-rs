// src/infrastructure/vertex/vertex_converter.rs
//
// Vertex AI specific implementation of the ProviderConverter trait

use anyhow::Result;
use async_trait::async_trait;
use tracing::{debug, warn};
use serde_json::Value;

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
    VertexMessage, VertexRole, VertexUsage, VertexContent, VertexTool, VertexToolChoice,
};

/// Vertex AI implementation of the ProviderConverter trait
pub struct VertexAIConverter {
    /// Default anthropic version for Vertex AI
    anthropic_version: String,
    
    /// Tool call manager for handling Claude-style tool calls
    #[allow(dead_code)] // Will be used for tool call processing in future
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
    fn convert_messages(&self, messages: &[ChatMessage]) -> Result<Vec<VertexMessage>> {
        let mut vertex_messages = Vec::new();
        let mut system_prompt = String::new();

        for message in messages {
            match message.role {
                MessageRole::System => {
                    // Collect system messages into system prompt
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
                    // For now, treat function/tool messages as user messages
                    // TODO: Implement proper tool support
                    warn!("Function/tool messages not fully supported, treating as user message");
                    vertex_messages.push(VertexMessage {
                        role: VertexRole::User,
                        content: message.content.clone(),
                    });
                }
            }
        }

        // If we have a system prompt, we need to handle it specially
        // Vertex AI doesn't have a separate system role, so we prepend it to the first user message
        if !system_prompt.is_empty() && !vertex_messages.is_empty() {
            if let Some(first_message) = vertex_messages.first_mut() {
                if first_message.role == VertexRole::User {
                    first_message.content = format!("{}\n\n{}", system_prompt, first_message.content);
                }
            }
        }

        Ok(vertex_messages)
    }

    /// Normalize model name for Vertex AI
    fn normalize_vertex_model(&self, model: &str) -> String {
        match model {
            "claude-4-sonnet" | "claude-4-sonnet-20250514" => "claude-sonnet-4-5@20250929".to_string(),
            "claude-3-5-haiku" => "claude-3-5-haiku@20241022".to_string(),
            "claude-3-5-sonnet" => "claude-3-5-sonnet@20241022".to_string(),
            _ => model.to_string(),
        }
    }

    /// Convert OpenAI tools to Vertex AI format
    fn convert_tools(&self, request: &ChatCompletionRequest) -> Result<Option<Vec<VertexTool>>> {
        let mut vertex_tools = Vec::new();

        // Convert modern tools
        if let Some(tools) = &request.tools {
            for tool in tools {
                if tool.tool_type == "function" {
                    let vertex_tool = VertexTool {
                        name: tool.function.name.clone(),
                        description: tool.function.description.clone().unwrap_or_default(),
                        input_schema: tool.function.parameters.clone().unwrap_or(serde_json::json!({})),
                    };
                    vertex_tools.push(vertex_tool);
                }
            }
        }

        // Convert legacy functions
        if let Some(functions) = &request.functions {
            for function in functions {
                let vertex_tool = VertexTool {
                    name: function.name.clone(),
                    description: function.description.clone().unwrap_or_default(),
                    input_schema: function.parameters.clone().unwrap_or(serde_json::json!({})),
                };
                vertex_tools.push(vertex_tool);
            }
        }

        if vertex_tools.is_empty() {
            Ok(None)
        } else {
            Ok(Some(vertex_tools))
        }
    }

    /// Convert OpenAI tool choice to Vertex AI format
    fn convert_tool_choice(&self, request: &ChatCompletionRequest) -> Result<Option<VertexToolChoice>> {
        // Handle modern tool_choice
        if let Some(tool_choice) = &request.tool_choice {
            match tool_choice {
                serde_json::Value::String(s) => {
                    match s.as_str() {
                        "auto" => Ok(Some(VertexToolChoice::Auto)),
                        "any" => Ok(Some(VertexToolChoice::Any)),
                        _ => Ok(None),
                    }
                }
                serde_json::Value::Object(obj) => {
                    if let Some(function_obj) = obj.get("function") {
                        if let Some(name) = function_obj.get("name").and_then(|n| n.as_str()) {
                            Ok(Some(VertexToolChoice::Tool { name: name.to_string() }))
                        } else {
                            Ok(None)
                        }
                    } else {
                        Ok(None)
                    }
                }
                _ => Ok(None),
            }
        }
        // Handle legacy function_call
        else if let Some(function_call) = &request.function_call {
            match function_call {
                serde_json::Value::String(s) => {
                    match s.as_str() {
                        "auto" => Ok(Some(VertexToolChoice::Auto)),
                        _ => Ok(None),
                    }
                }
                serde_json::Value::Object(obj) => {
                    if let Some(name) = obj.get("name").and_then(|n| n.as_str()) {
                        Ok(Some(VertexToolChoice::Tool { name: name.to_string() }))
                    } else {
                        Ok(None)
                    }
                }
                _ => Ok(None),
            }
        } else {
            Ok(None)
        }
    }

    /// Convert Vertex AI tool uses to OpenAI format
    fn convert_tool_uses_to_openai(
        &self,
        tool_uses: &[&VertexContent],
        original_request: &ChatCompletionRequest,
    ) -> Result<(Option<Vec<serde_json::Value>>, Option<serde_json::Value>)> {
        if tool_uses.is_empty() {
            return Ok((None, None));
        }

        let mut openai_tool_calls = Vec::new();
        let mut function_call = None;

        for (index, tool_use) in tool_uses.iter().enumerate() {
            if let VertexContent::ToolUse { id, name, input } = tool_use {
                // Create OpenAI tool call format
                let tool_call = serde_json::json!({
                    "id": id,
                    "type": "function",
                    "function": {
                        "name": name,
                        "arguments": serde_json::to_string(input).unwrap_or_default()
                    }
                });
                openai_tool_calls.push(tool_call);

                // For legacy compatibility, set function_call to the first tool
                if index == 0 && (original_request.functions.is_some() || original_request.function_call.is_some()) {
                    function_call = Some(serde_json::json!({
                        "name": name,
                        "arguments": serde_json::to_string(input).unwrap_or_default()
                    }));
                }
            }
        }

        let tool_calls = if openai_tool_calls.is_empty() {
            None
        } else {
            Some(openai_tool_calls)
        };

        Ok((tool_calls, function_call))
    }
}

#[async_trait]
impl ProviderConverter for VertexAIConverter {
    type ProviderRequest = VertexPredictRequest;
    type ProviderResponse = VertexPredictResponse;
    type ProviderStreamChunk = VertexStreamChunk;
    type ProviderError = VertexError;

    fn openai_to_provider_request(&self, request: &ChatCompletionRequest) -> Result<Self::ProviderRequest> {
        let vertex_messages = self.convert_messages(&request.messages)?;
        let vertex_tools = self.convert_tools(request)?;
        let vertex_tool_choice = self.convert_tool_choice(request)?;

        Ok(VertexPredictRequest {
            anthropic_version: self.anthropic_version.clone(),
            messages: vertex_messages,
            max_tokens: request.max_tokens.unwrap_or(16384),
            temperature: request.temperature,
            top_p: request.top_p,
            top_k: None,
            system: None,
            stream: false, // Non-streaming request
            stop_sequences: request.stop.as_ref().map(|stop| match stop {
                crate::models::request::Stop::String(s) => vec![s.clone()],
                crate::models::request::Stop::Array(arr) => arr.clone(),
            }),
            tools: vertex_tools,
            tool_choice: vertex_tool_choice,
        })
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
        let content = response.get_text();
        let tool_uses = response.get_tool_uses();
        
        // Convert tool uses to OpenAI format
        let (tool_calls, function_call) = self.convert_tool_uses_to_openai(&tool_uses, original_request)?;
        
        let message = ChatMessage {
            role: MessageRole::Assistant,
            content: if content.is_empty() && tool_calls.is_some() { 
                String::new() 
            } else { 
                content 
            },
            name: None,
            function_call,
            tool_calls,
            tool_call_id: None,
        };

        let choice = crate::models::response::ChatCompletionChoice {
            index: 0,
            message,
            logprobs: None,
            finish_reason: response.stop_reason.as_ref()
                .map(|r| normalize_finish_reason(r))
                .unwrap_or(FinishReason::Stop),
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
        // For now, just handle text content - tool streaming can be added later
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

            Ok(Some(ChatCompletionChunk::new(
                request_id.to_string(),
                model.to_string(),
                vec![choice],
            )))
        } else {
            Ok(None)
        }
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
        // Retry on rate limits and temporary server errors based on error type
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

        let vertex_messages = converter.convert_messages(&messages).unwrap();
        assert_eq!(vertex_messages.len(), 1);
        assert_eq!(vertex_messages[0].role, VertexRole::User);
        assert_eq!(vertex_messages[0].content, "Hello");
    }

    #[test]
    fn test_model_normalization() {
        let converter = VertexAIConverter::new();
        assert_eq!(
            converter.normalize_vertex_model("claude-4-sonnet"),
            "claude-sonnet-4-5@20250929"
        );
    }
}