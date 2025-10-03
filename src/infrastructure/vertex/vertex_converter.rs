// src/infrastructure/vertex/vertex_converter.rs
//
// Vertex AI specific implementation of the ProviderConverter trait

use anyhow::Result;
use async_trait::async_trait;
use tracing::{debug, warn};

use crate::infrastructure::common::{
    ProviderConverter, ProviderErrorHandler,
    ToolCallManager,
    converter_utils::normalize_finish_reason,
};
use crate::infrastructure::common::AIProvider;
use crate::models::{
    request::ChatCompletionRequest,
    response::{ChatCompletionResponse, ChatCompletionChunk, ChatCompletionChunkChoice},
    common::{ChatMessage, MessageRole, FinishReason, Usage},
};
use super::{
    VertexPredictRequest, VertexPredictResponse, VertexStreamChunk, VertexError,
    VertexMessage, VertexRole, VertexUsage,
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
}

impl Default for VertexAIConverter {
    fn default() -> Self {
        Self::new()
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
        _original_request: &ChatCompletionRequest,
    ) -> Result<ChatCompletionResponse> {
        let content = response.get_text();
        
        let message = ChatMessage {
            role: MessageRole::Assistant,
            content,
            name: None,
            function_call: None,
            tool_calls: None,
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
        debug!("Converting Vertex AI chunk - event_type: {}, has_delta: {}, has_content: {}, has_tool_calls: {}", 
            chunk.event_type, 
            chunk.delta.is_some(),
            chunk.get_content().is_some(),
            self.has_tool_calls(chunk)
        );

        // Check for tool calls first (they take precedence over content)
        if self.has_tool_calls(chunk) {
            debug!("Processing tool calls in chunk '{}'", chunk.event_type);
            return self.create_tool_calls_chunk(chunk, request_id, model);
        }

        // Then try to extract content
        if let Some(content) = chunk.get_content() {
            debug!("Found content in chunk '{}': {}", chunk.event_type, content);
            return self.create_content_chunk(content, request_id, model);
        }

        // Handle specific event types for non-content chunks
        match chunk.event_type.as_str() {
            // Standard Anthropic event types
            "message_start" => {
                debug!("Received message_start chunk");
                Ok(None) // Skip initial chunks
            }
            "content_block_start" => {
                debug!("Received content_block_start chunk");
                Ok(None) // Skip content block start
            }
            "content_block_delta" | "ping" => {
                // These should have been handled above, but just in case
                debug!("content_block_delta/ping chunk without content");
                Ok(None)
            }
            "content_block_stop" => {
                debug!("Received content_block_stop chunk");
                Ok(None) // Skip content block end
            }
            "message_delta" => {
                // Message delta with potential finish reason
                let finish_reason = chunk.get_finish_reason()
                    .map(|r| match r {
                        super::VertexFinishReason::Stop => FinishReason::Stop,
                        super::VertexFinishReason::MaxTokens => FinishReason::Length,
                        super::VertexFinishReason::Safety => FinishReason::ContentFilter,
                        super::VertexFinishReason::Recitation => FinishReason::ContentFilter,
                        super::VertexFinishReason::Other => FinishReason::Stop,
                    });

                if finish_reason.is_some() {
                    debug!("Found finish reason in message_delta: {:?}", finish_reason);
                    self.create_finish_chunk(finish_reason, request_id, model)
                } else {
                    Ok(None)
                }
            }
            "message_stop" => {
                // Final chunk with usage
                if let Some(usage) = &chunk.usage {
                    debug!("Found usage info in message_stop");
                    self.create_usage_chunk(usage, request_id, model)
                } else {
                    debug!("message_stop without usage, creating final chunk");
                    self.create_finish_chunk(Some(FinishReason::Stop), request_id, model)
                }
            }
            // Vertex AI and other specific event types
            "message" | "completion" | "text" | "delta" => {
                // These might contain usage information or be final chunks
                if let Some(usage) = &chunk.usage {
                    debug!("Found usage info in '{}' chunk", chunk.event_type);
                    self.create_usage_chunk(usage, request_id, model)
                } else {
                    debug!("'{}' chunk without content or usage, skipping", chunk.event_type);
                    Ok(None)
                }
            }
            unknown_type => {
                debug!("Unknown chunk type '{}', checking for usage or finish reason", unknown_type);
                
                // Check for usage information
                if let Some(usage) = &chunk.usage {
                    debug!("Found usage info in unknown chunk type '{}'", unknown_type);
                    self.create_usage_chunk(usage, request_id, model)
                } else if let Some(_finish_reason) = chunk.get_finish_reason() {
                    debug!("Found finish reason in unknown chunk type '{}'", unknown_type);
                    self.create_finish_chunk(Some(FinishReason::Stop), request_id, model)
                } else {
                    debug!("No useful data found in unknown chunk type '{}'", unknown_type);
                    Ok(None)
                }
            }
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

impl VertexAIConverter {
    /// Create a content chunk with text
    fn create_content_chunk(
        &self,
        content: String,
        request_id: &str,
        model: &str,
    ) -> Result<Option<ChatCompletionChunk>> {
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
    }

    /// Create a finish chunk with finish reason
    fn create_finish_chunk(
        &self,
        finish_reason: Option<FinishReason>,
        request_id: &str,
        model: &str,
    ) -> Result<Option<ChatCompletionChunk>> {
        let choice = ChatCompletionChunkChoice {
            index: 0,
            delta: ChatMessage::empty(),
            logprobs: None,
            finish_reason,
        };

        Ok(Some(ChatCompletionChunk::new(
            request_id.to_string(),
            model.to_string(),
            vec![choice],
        )))
    }

    /// Create a usage chunk with token counts
    fn create_usage_chunk(
        &self,
        usage: &VertexUsage,
        request_id: &str,
        model: &str,
    ) -> Result<Option<ChatCompletionChunk>> {
        let openai_usage = Usage {
            prompt_tokens: usage.input_tokens,
            completion_tokens: usage.output_tokens,
            total_tokens: usage.input_tokens + usage.output_tokens,
            prompt_tokens_details: None,
            completion_tokens_details: None,
        };

        let choice = ChatCompletionChunkChoice {
            index: 0,
            delta: ChatMessage::empty(),
            logprobs: None,
            finish_reason: Some(FinishReason::Stop),
        };

        Ok(Some(ChatCompletionChunk::new_with_usage(
            request_id.to_string(),
            model.to_string(),
            vec![choice],
            openai_usage,
        )))
    }

    /// Check if chunk contains tool calls
    fn has_tool_calls(&self, chunk: &VertexStreamChunk) -> bool {
        // Check if content contains tool_use blocks
        if let Some(content_blocks) = &chunk.content {
            for content in content_blocks {
                if content.content_type == "tool_use" {
                    return true;
                }
            }
        }
        false
    }

    /// Create a tool calls chunk
    fn create_tool_calls_chunk(
        &self,
        chunk: &VertexStreamChunk,
        request_id: &str,
        model: &str,
    ) -> Result<Option<ChatCompletionChunk>> {
        // Extract tool calls from chunk content
        if let Some(content_blocks) = &chunk.content {
            for content in content_blocks {
                if content.content_type == "tool_use" {
                    debug!("Processing tool_use content block with tool_manager");
                    
                    // Parse the tool call from the content text
                    if let Ok(tool_call_json) = serde_json::from_str::<serde_json::Value>(&content.text) {
                        // Create a temporary mutable tool manager for processing
                        let mut temp_tool_manager = ToolCallManager::claude();
                        
                        // Use the tool_manager to process the tool call
                        if let Ok(Some(tool_calls)) = temp_tool_manager.process_tool_calls(&tool_call_json, request_id.to_string()) {
                            debug!("Extracted {} tool calls from chunk", tool_calls.len());
                            
                            // Convert to OpenAI format
                            let openai_tool_calls = temp_tool_manager.to_openai_format(&tool_calls);
                            
                            // Convert ToolCall structs to JSON Values
                            let tool_calls_json: Vec<serde_json::Value> = openai_tool_calls
                                .into_iter()
                                .map(|tc| serde_json::to_value(tc).unwrap_or_default())
                                .collect();
                            
                            // Create OpenAI format tool calls chunk
                            let delta = ChatMessage {
                                role: MessageRole::Assistant,
                                content: String::new(),
                                name: None,
                                function_call: None,
                                tool_calls: Some(tool_calls_json),
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
            }
        }
        
        // No tool calls found or couldn't process them
        Ok(None)
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
        assert!(!converter.supported_models().is_empty());
    }

    #[test]
    fn test_model_normalization() {
        let converter = VertexAIConverter::new();
        
        assert_eq!(
            converter.normalize_model_name("claude-4-sonnet-20250514"),
            "claude-sonnet-4-5@20250929"
        );
        
        assert_eq!(
            converter.normalize_model_name("claude-3-5-haiku"),
            "claude-3-5-haiku@20241022"
        );
        
        // Unknown models should pass through
        assert_eq!(
            converter.normalize_model_name("unknown-model"),
            "unknown-model"
        );
    }

    #[test]
    fn test_message_conversion() {
        let converter = VertexAIConverter::new();
        
        let messages = vec![
            ChatMessage {
                role: MessageRole::System,
                content: "You are helpful".to_string(),
                name: None,
                function_call: None,
                tool_calls: None,
                tool_call_id: None,
            },
            ChatMessage {
                role: MessageRole::User,
                content: "Hello".to_string(),
                name: None,
                function_call: None,
                tool_calls: None,
                tool_call_id: None,
            },
        ];

        let vertex_messages = converter.convert_messages(&messages).unwrap();
        
        // Should have 1 message (system prompt prepended to user message)
        assert_eq!(vertex_messages.len(), 1);
        assert_eq!(vertex_messages[0].role, VertexRole::User);
        assert!(vertex_messages[0].content.contains("You are helpful"));
        assert!(vertex_messages[0].content.contains("Hello"));
    }
}
