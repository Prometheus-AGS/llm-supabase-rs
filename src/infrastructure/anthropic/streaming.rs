//! Anthropic streaming response handling implementation
//! 
//! This module handles Server-Sent Events (SSE) streaming for Anthropic API responses,
//! including tool call detection and proper stream termination.

use anyhow::{anyhow, Context, Result};
use bytes::Bytes;
use futures_util::{Stream, StreamExt};
use reqwest::Response;
use serde_json::Value;
use std::collections::HashMap;
use std::pin::Pin;
use std::task::{Context as TaskContext, Poll};
use tokio_stream::wrappers::LinesStream;
use tracing::{debug, warn, error};

use crate::infrastructure::common::tools::{UnifiedToolCall, ToolCallResult};
use crate::models::response::ChatCompletionChunk;

use super::types::{
    AnthropicStreamChunk, AnthropicDelta, AnthropicContentBlock, 
    AnthropicMessageStart, AnthropicMessageDelta, AnthropicError
};

/// Anthropic streaming response parser
/// 
/// Parses Server-Sent Events from Anthropic API and converts them to OpenAI-compatible
/// streaming format while handling tool calls appropriately.
#[derive(Debug)]
pub struct AnthropicStreamParser {
    /// Current message ID being processed
    current_message_id: Option<String>,
    
    /// Current model being used
    current_model: Option<String>,
    
    /// Accumulated content for each content block index
    content_blocks: HashMap<u32, String>,
    
    /// Tool calls detected during streaming
    tool_calls: Vec<UnifiedToolCall>,
    
    /// Whether the stream has ended
    stream_ended: bool,
    
    /// Buffer for incomplete JSON parsing
    json_buffer: HashMap<u32, String>,
    
    /// Current tool call being built
    current_tool_calls: HashMap<u32, PartialToolCall>,
}

/// Partial tool call being built during streaming
#[derive(Debug, Clone)]
struct PartialToolCall {
    id: Option<String>,
    name: Option<String>,
    arguments: String,
}

impl Default for AnthropicStreamParser {
    fn default() -> Self {
        Self::new()
    }
}

impl AnthropicStreamParser {
    /// Create a new Anthropic stream parser
    pub fn new() -> Self {
        Self {
            current_message_id: None,
            current_model: None,
            content_blocks: HashMap::new(),
            tool_calls: Vec::new(),
            stream_ended: false,
            json_buffer: HashMap::new(),
            current_tool_calls: HashMap::new(),
        }
    }

    /// Parse a single SSE event from Anthropic
    pub fn parse_sse_event(&mut self, event_data: &str) -> Result<Option<ChatCompletionChunk>> {
        if event_data.trim().is_empty() {
            return Ok(None);
        }

        debug!("Parsing SSE event: {}", event_data);

        // Parse the JSON event
        let chunk: AnthropicStreamChunk = serde_json::from_str(event_data)
            .context("Failed to parse Anthropic stream chunk")?;

        match chunk {
            AnthropicStreamChunk::MessageStart { message } => {
                self.handle_message_start(message)
            }
            AnthropicStreamChunk::ContentBlockStart { index, content_block } => {
                self.handle_content_block_start(index, content_block)
            }
            AnthropicStreamChunk::ContentBlockDelta { index, delta } => {
                self.handle_content_block_delta(index, delta)
            }
            AnthropicStreamChunk::ContentBlockStop { index } => {
                self.handle_content_block_stop(index)
            }
            AnthropicStreamChunk::MessageDelta { delta, usage } => {
                self.handle_message_delta(delta, usage)
            }
            AnthropicStreamChunk::MessageStop => {
                self.handle_message_stop()
            }
            AnthropicStreamChunk::Ping => {
                // Ping events don't need special handling
                Ok(None)
            }
            AnthropicStreamChunk::Error { error } => {
                self.handle_error(error)
            }
        }
    }

    /// Handle message start event
    fn handle_message_start(&mut self, message: AnthropicMessageStart) -> Result<Option<ChatCompletionChunk>> {
        debug!("Handling message start: {}", message.id);

        self.current_message_id = Some(message.id.clone());
        self.current_model = Some(message.model.clone());

        // Create initial chunk
        let chunk = ChatCompletionChunk {
            id: message.id,
            object: "chat.completion.chunk".to_string(),
            created: chrono::Utc::now().timestamp() as u64,
            model: message.model,
            system_fingerprint: None,
            choices: vec![crate::models::response::ChatCompletionStreamChoice {
                index: 0,
                delta: crate::models::common::ChatMessage {
                    role: crate::models::common::MessageRole::Assistant,
                    content: String::new(),
                    name: None,
                    function_call: None,
                    tool_calls: None,
                    tool_call_id: None,
                },
                logprobs: None,
                finish_reason: None,
            }],
            usage: None,
        };

        Ok(Some(chunk))
    }

    /// Handle content block start event
    fn handle_content_block_start(
        &mut self, 
        index: u32, 
        content_block: AnthropicContentBlock
    ) -> Result<Option<ChatCompletionChunk>> {
        debug!("Handling content block start at index {}", index);

        match content_block {
            AnthropicContentBlock::Text { text } => {
                // Initialize text content block
                self.content_blocks.insert(index, text.clone());
                
                // Create chunk with initial text content
                if let (Some(id), Some(model)) = (&self.current_message_id, &self.current_model) {
                    let chunk = ChatCompletionChunk {
                        id: id.clone(),
                        object: "chat.completion.chunk".to_string(),
                        created: chrono::Utc::now().timestamp() as u64,
                        model: model.clone(),
                        system_fingerprint: None,
                        choices: vec![crate::models::response::ChatCompletionStreamChoice {
                            index: 0,
                            delta: crate::models::common::ChatMessage {
                                role: crate::models::common::MessageRole::Assistant,
                                content: if text.is_empty() { String::new() } else { text },
                                name: None,
                                function_call: None,
                                tool_calls: None,
                                tool_call_id: None,
                            },
                            logprobs: None,
                            finish_reason: None,
                        }],
                        usage: None,
                    };
                    return Ok(Some(chunk));
                }
            }
            AnthropicContentBlock::ToolUse { id, name, input } => {
                // Initialize tool call
                let partial_tool_call = PartialToolCall {
                    id: Some(id.clone()),
                    name: Some(name.clone()),
                    arguments: serde_json::to_string(&input).unwrap_or_else(|_| "{}".to_string()),
                };
                self.current_tool_calls.insert(index, partial_tool_call);

                // Create chunk with tool call start
                if let (Some(msg_id), Some(model)) = (&self.current_message_id, &self.current_model) {
                    let chunk = ChatCompletionChunk {
                        id: msg_id.clone(),
                        object: "chat.completion.chunk".to_string(),
                        created: chrono::Utc::now().timestamp() as u64,
                        model: model.clone(),
                        system_fingerprint: None,
                        choices: vec![crate::models::response::ChatCompletionStreamChoice {
                            index: 0,
                            delta: crate::models::common::ChatMessage {
                                role: crate::models::common::MessageRole::Assistant,
                                content: String::new(),
                                name: None,
                                function_call: None,
                                tool_calls: Some(vec![serde_json::to_value(crate::shared::types::ToolCall {
                                    id: id.clone(),
                                    tool_type: "function".to_string(),
                                    function: crate::shared::types::FunctionCall {
                                        name: name.clone(),
                                        arguments: "{}".to_string(),
                                    },
                                }).unwrap_or(serde_json::Value::Null)]),
                                tool_call_id: None,
                            },
                            logprobs: None,
                            finish_reason: None,
                        }],
                        usage: None,
                    };
                    return Ok(Some(chunk));
                }
            }
            _ => {
                // Other content block types
                warn!("Unhandled content block type in stream start");
            }
        }

        Ok(None)
    }

    /// Handle content block delta event
    fn handle_content_block_delta(
        &mut self, 
        index: u32, 
        delta: AnthropicDelta
    ) -> Result<Option<ChatCompletionChunk>> {
        debug!("Handling content block delta at index {}", index);

        match delta {
            AnthropicDelta::TextDelta { text } => {
                // Append to existing text content
                let current_content = self.content_blocks.entry(index).or_insert_with(String::new);
                current_content.push_str(&text);

                // Create chunk with text delta
                if let (Some(id), Some(model)) = (&self.current_message_id, &self.current_model) {
                    let chunk = ChatCompletionChunk {
                        id: id.clone(),
                        object: "chat.completion.chunk".to_string(),
                        created: chrono::Utc::now().timestamp() as u64,
                        model: model.clone(),
                        system_fingerprint: None,
                        choices: vec![crate::models::response::ChatCompletionStreamChoice {
                            index: 0,
                            delta: crate::models::common::ChatMessage {
                                role: crate::models::common::MessageRole::Assistant,
                                content: text,
                                name: None,
                                function_call: None,
                                tool_calls: None,
                                tool_call_id: None,
                            },
                            logprobs: None,
                            finish_reason: None,
                        }],
                        usage: None,
                    };
                    return Ok(Some(chunk));
                }
            }
            AnthropicDelta::InputJsonDelta { partial_json } => {
                // Accumulate JSON for tool call arguments
                let json_buffer = self.json_buffer.entry(index).or_insert_with(String::new);
                json_buffer.push_str(&partial_json);

                // Create chunk with tool call arguments delta
                if let (Some(id), Some(model)) = (&self.current_message_id, &self.current_model) {
                    let chunk = ChatCompletionChunk {
                        id: id.clone(),
                        object: "chat.completion.chunk".to_string(),
                        created: chrono::Utc::now().timestamp() as u64,
                        model: model.clone(),
                        system_fingerprint: None,
                        choices: vec![crate::models::response::ChatCompletionStreamChoice {
                            index: 0,
                            delta: crate::models::common::ChatMessage {
                                role: crate::models::common::MessageRole::Assistant,
                                content: String::new(),
                                name: None,
                                function_call: None,
                                tool_calls: Some(vec![serde_json::to_value(crate::shared::types::ToolCall {
                                    id: format!("call_{}", index),
                                    tool_type: "function".to_string(),
                                    function: crate::shared::types::FunctionCall {
                                        name: "unknown".to_string(),
                                        arguments: partial_json,
                                    },
                                }).unwrap_or(serde_json::Value::Null)]),
                                tool_call_id: None,
                            },
                            logprobs: None,
                            finish_reason: None,
                        }],
                        usage: None,
                    };
                    return Ok(Some(chunk));
                }
            }
        }

        Ok(None)
    }

    /// Handle content block stop event
    fn handle_content_block_stop(&mut self, index: u32) -> Result<Option<ChatCompletionChunk>> {
        debug!("Handling content block stop at index {}", index);

        // Finalize tool call if it exists
        if let Some(partial_tool_call) = self.current_tool_calls.remove(&index) {
            if let (Some(id), Some(name)) = (partial_tool_call.id, partial_tool_call.name) {
                // Parse final arguments
                let arguments = if let Some(json_str) = self.json_buffer.remove(&index) {
                    serde_json::from_str(&json_str).unwrap_or_else(|_| serde_json::Value::Object(Default::default()))
                } else {
                    serde_json::from_str(&partial_tool_call.arguments)
                        .unwrap_or_else(|_| serde_json::Value::Object(Default::default()))
                };

                // Create unified tool call
                let unified_tool_call = UnifiedToolCall {
                    id: id.clone(),
                    function_name: name,
                    arguments,
                    metadata: HashMap::new(),
                };

                self.tool_calls.push(unified_tool_call);
            }
        }

        Ok(None)
    }

    /// Handle message delta event
    fn handle_message_delta(
        &mut self, 
        delta: AnthropicMessageDelta, 
        usage: Option<super::types::AnthropicUsage>
    ) -> Result<Option<ChatCompletionChunk>> {
        debug!("Handling message delta");

        if let Some(stop_reason) = delta.stop_reason {
            // Create final chunk with finish reason
            if let (Some(id), Some(model)) = (&self.current_message_id, &self.current_model) {
                let finish_reason = match stop_reason.as_str() {
                    "end_turn" => "stop",
                    "max_tokens" => "length",
                    "tool_use" => "tool_calls",
                    "stop_sequence" => "stop",
                    _ => "stop",
                };

                let chunk = ChatCompletionChunk {
                    id: id.clone(),
                    object: "chat.completion.chunk".to_string(),
                    created: chrono::Utc::now().timestamp() as u64,
                    model: model.clone(),
                    system_fingerprint: None,
                    choices: vec![crate::models::response::ChatCompletionStreamChoice {
                        index: 0,
                        delta: crate::models::common::ChatMessage {
                            role: crate::models::common::MessageRole::Assistant,
                            content: String::new(),
                            name: None,
                            function_call: None,
                            tool_calls: None,
                            tool_call_id: None,
                        },
                        logprobs: None,
                        finish_reason: Some(match finish_reason {
                            "end_turn" => crate::models::common::FinishReason::Stop,
                            "max_tokens" => crate::models::common::FinishReason::Length,
                            "tool_use" => crate::models::common::FinishReason::ToolCalls,
                            "stop_sequence" => crate::models::common::FinishReason::Stop,
                            _ => crate::models::common::FinishReason::Stop,
                        }),
                    }],
                    usage: usage.map(|u| crate::models::common::Usage {
                        prompt_tokens: u.input_tokens,
                        completion_tokens: u.output_tokens,
                        total_tokens: u.input_tokens + u.output_tokens,
                        prompt_tokens_details: None,
                        completion_tokens_details: None,
                    }),
                };
                return Ok(Some(chunk));
            }
        }

        Ok(None)
    }

    /// Handle message stop event
    fn handle_message_stop(&mut self) -> Result<Option<ChatCompletionChunk>> {
        debug!("Handling message stop");
        self.stream_ended = true;

        // Create final [DONE] chunk
        if let (Some(id), Some(model)) = (&self.current_message_id, &self.current_model) {
            let chunk = ChatCompletionChunk {
                id: id.clone(),
                object: "chat.completion.chunk".to_string(),
                created: chrono::Utc::now().timestamp() as u64,
                model: model.clone(),
                system_fingerprint: None,
                choices: vec![crate::models::response::ChatCompletionStreamChoice {
                    index: 0,
                    delta: crate::models::common::ChatMessage {
                        role: crate::models::common::MessageRole::Assistant,
                        content: String::new(),
                        name: None,
                        function_call: None,
                        tool_calls: None,
                        tool_call_id: None,
                    },
                    logprobs: None,
                    finish_reason: Some(crate::models::common::FinishReason::Stop),
                }],
                usage: None,
            };
            return Ok(Some(chunk));
        }

        Ok(None)
    }

    /// Handle error event
    fn handle_error(&mut self, error: AnthropicError) -> Result<Option<ChatCompletionChunk>> {
        error!("Anthropic stream error: {} - {}", error.error_type, error.message);
        Err(anyhow!("Anthropic API error: {} - {}", error.error_type, error.message))
    }

    /// Get extracted tool calls
    pub fn get_tool_calls(&self) -> &[UnifiedToolCall] {
        &self.tool_calls
    }

    /// Check if stream has ended
    pub fn is_stream_ended(&self) -> bool {
        self.stream_ended
    }

    /// Reset parser state for new stream
    pub fn reset(&mut self) {
        self.current_message_id = None;
        self.current_model = None;
        self.content_blocks.clear();
        self.tool_calls.clear();
        self.stream_ended = false;
        self.json_buffer.clear();
        self.current_tool_calls.clear();
    }

    /// Get accumulated content
    pub fn get_accumulated_content(&self) -> String {
        self.content_blocks
            .values()
            .cloned()
            .collect::<Vec<String>>()
            .join("")
    }
}

/// Anthropic streaming utilities
pub struct AnthropicStreamUtils;

impl AnthropicStreamUtils {
    /// Create a stream from an HTTP response
    pub fn create_stream_from_response(
        response: Response,
    ) -> impl Stream<Item = Result<ChatCompletionChunk>> {
        AnthropicStreamFromResponse::new(response)
    }

    /// Parse SSE data line
    pub fn parse_sse_line(line: &str) -> Option<String> {
        if line.starts_with("data: ") {
            let data = &line[6..]; // Remove "data: " prefix
            if data == "[DONE]" {
                None // Signal end of stream
            } else {
                Some(data.to_string())
            }
        } else if line.starts_with("event: ") || line.starts_with("id: ") || line.starts_with(":") {
            // Ignore other SSE fields and comments
            None
        } else if line.trim().is_empty() {
            // Empty lines separate events
            None
        } else {
            // Malformed line - could be raw JSON
            Some(line.to_string())
        }
    }

    /// Validate stream chunk
    pub fn validate_chunk(chunk: &ChatCompletionChunk) -> Result<()> {
        if chunk.choices.is_empty() {
            return Err(anyhow!("Stream chunk must have at least one choice"));
        }

        // Additional validation can be added here
        Ok(())
    }
}

/// Stream implementation for Anthropic responses
struct AnthropicStreamFromResponse {
    parser: AnthropicStreamParser,
    lines_stream: Pin<Box<dyn Stream<Item = Result<String, std::io::Error>> + Send>>,
}

impl AnthropicStreamFromResponse {
    fn new(response: Response) -> Self {
        // Convert response to line stream
        let byte_stream = response.bytes_stream();
        let lines_stream = Box::pin(
            byte_stream
                .map(|result| {
                    result.map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))
                        .and_then(|bytes| {
                            String::from_utf8(bytes.to_vec())
                                .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))
                        })
                })
                .flat_map(|result| {
                    match result {
                        Ok(text) => {
                            // Split on newlines and create a stream of individual lines
                            futures_util::stream::iter(
                                text.lines()
                                    .map(|line| Ok(line.to_string()))
                                    .collect::<Vec<_>>()
                            ).left_stream()
                        }
                        Err(e) => futures_util::stream::once(async move { Err(e) }).right_stream()
                    }
                })
        );

        Self {
            parser: AnthropicStreamParser::new(),
            lines_stream,
        }
    }
}

impl Stream for AnthropicStreamFromResponse {
    type Item = Result<ChatCompletionChunk>;

    fn poll_next(
        mut self: Pin<&mut Self>, 
        cx: &mut TaskContext<'_>
    ) -> Poll<Option<Self::Item>> {
        loop {
            match self.lines_stream.as_mut().poll_next(cx) {
                Poll::Ready(Some(Ok(line))) => {
                    if let Some(data) = AnthropicStreamUtils::parse_sse_line(&line) {
                        match self.parser.parse_sse_event(&data) {
                            Ok(Some(chunk)) => {
                                if let Err(e) = AnthropicStreamUtils::validate_chunk(&chunk) {
                                    return Poll::Ready(Some(Err(e)));
                                }
                                return Poll::Ready(Some(Ok(chunk)));
                            }
                            Ok(None) => {
                                // Continue to next line
                                continue;
                            }
                            Err(e) => {
                                return Poll::Ready(Some(Err(e)));
                            }
                        }
                    }
                    // Continue to next line if no data extracted
                }
                Poll::Ready(Some(Err(e))) => {
                    return Poll::Ready(Some(Err(anyhow!("Stream error: {}", e))));
                }
                Poll::Ready(None) => {
                    // Stream ended
                    return Poll::Ready(None);
                }
                Poll::Pending => {
                    return Poll::Pending;
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn create_test_parser() -> AnthropicStreamParser {
        AnthropicStreamParser::new()
    }

    #[test]
    fn test_parser_creation() {
        let parser = create_test_parser();
        assert!(!parser.is_stream_ended());
        assert!(parser.get_tool_calls().is_empty());
        assert_eq!(parser.get_accumulated_content(), "");
    }

    #[test]
    fn test_message_start_parsing() {
        let mut parser = create_test_parser();
        
        let message_start = json!({
            "type": "message_start",
            "message": {
                "id": "msg_123",
                "type": "message",
                "role": "assistant",
                "content": [],
                "model": "claude-3-5-sonnet-20241022",
                "stop_reason": null,
                "stop_sequence": null,
                "usage": {
                    "input_tokens": 10,
                    "output_tokens": 0
                }
            }
        });

        let result = parser.parse_sse_event(&message_start.to_string());
        assert!(result.is_ok());
        
        let chunk = result.unwrap();
        assert!(chunk.is_some());
        
        let chunk = chunk.unwrap();
        assert_eq!(chunk.id, "msg_123");
        assert_eq!(chunk.model, "claude-3-5-sonnet-20241022");
    }

    #[test]
    fn test_text_delta_parsing() {
        let mut parser = create_test_parser();
        
        // First initialize with message start
        let message_start = json!({
            "type": "message_start",
            "message": {
                "id": "msg_123",
                "type": "message",
                "role": "assistant",
                "content": [],
                "model": "claude-3-5-sonnet-20241022",
                "stop_reason": null,
                "stop_sequence": null,
                "usage": {"input_tokens": 10, "output_tokens": 0}
            }
        });
        parser.parse_sse_event(&message_start.to_string()).unwrap();

        // Then test text delta
        let text_delta = json!({
            "type": "content_block_delta",
            "index": 0,
            "delta": {
                "type": "text_delta",
                "text": "Hello"
            }
        });

        let result = parser.parse_sse_event(&text_delta.to_string());
        assert!(result.is_ok());
        
        let chunk = result.unwrap();
        assert!(chunk.is_some());
        
        let chunk = chunk.unwrap();
        let choice = &chunk.choices[0];
        assert_eq!(choice.delta.content, Some("Hello".to_string()));
    }

    #[test]
    fn test_tool_use_parsing() {
        let mut parser = create_test_parser();
        
        // Initialize with message start
        let message_start = json!({
            "type": "message_start",
            "message": {
                "id": "msg_123",
                "type": "message", 
                "role": "assistant",
                "content": [],
                "model": "claude-3-5-sonnet-20241022",
                "stop_reason": null,
                "stop_sequence": null,
                "usage": {"input_tokens": 10, "output_tokens": 0}
            }
        });
        parser.parse_sse_event(&message_start.to_string()).unwrap();

        // Test tool use start
        let tool_start = json!({
            "type": "content_block_start",
            "index": 0,
            "content_block": {
                "type": "tool_use",
                "id": "tool_123",
                "name": "get_weather",
                "input": {"location": "San Francisco"}
            }
        });

        let result = parser.parse_sse_event(&tool_start.to_string());
        assert!(result.is_ok());
        
        let chunk = result.unwrap();
        assert!(chunk.is_some());
        
        let chunk = chunk.unwrap();
        let choice = &chunk.choices[0];
        assert!(choice.delta.tool_calls.is_some());
        
        let tool_calls = choice.delta.tool_calls.as_ref().unwrap();
        assert_eq!(tool_calls.len(), 1);
        assert_eq!(tool_calls[0].id, Some("tool_123".to_string()));
        
        // Test content block stop to finalize tool call
        let block_stop = json!({
            "type": "content_block_stop",
            "index": 0
        });

        parser.parse_sse_event(&block_stop.to_string()).unwrap();
        
        let extracted_calls = parser.get_tool_calls();
        assert_eq!(extracted_calls.len(), 1);
        assert_eq!(extracted_calls[0].id, "tool_123");
        assert_eq!(extracted_calls[0].function_name, "get_weather");
    }

    #[test]
    fn test_message_stop() {
        let mut parser = create_test_parser();
        
        let message_stop = json!({
            "type": "message_stop"
        });

        let result = parser.parse_sse_event(&message_stop.to_string());
        assert!(result.is_ok());
        assert!(parser.is_stream_ended());
    }

    #[test]
    fn test_error_handling() {
        let mut parser = create_test_parser();
        
        let error_event = json!({
            "type": "error",
            "error": {
                "type": "invalid_request_error",
                "message": "Invalid API key"
            }
        });

        let result = parser.parse_sse_event(&error_event.to_string());
        assert!(result.is_err());
    }

    #[test]
    fn test_sse_line_parsing() {
        assert_eq!(
            AnthropicStreamUtils::parse_sse_line("data: {\"type\":\"ping\"}"),
            Some("{\"type\":\"ping\"}".to_string())
        );
        
        assert_eq!(
            AnthropicStreamUtils::parse_sse_line("data: [DONE]"),
            None
        );
        
        assert_eq!(
            AnthropicStreamUtils::parse_sse_line("event: message_start"),
            None
        );
        
        assert_eq!(
            AnthropicStreamUtils::parse_sse_line(""),
            None
        );
    }

    #[test]
    fn test_parser_reset() {
        let mut parser = create_test_parser();
        
        // Set some state
        parser.current_message_id = Some("test".to_string());
        parser.stream_ended = true;
        parser.content_blocks.insert(0, "content".to_string());
        
        // Reset and verify
        parser.reset();
        assert!(parser.current_message_id.is_none());
        assert!(!parser.is_stream_ended());
        assert!(parser.content_blocks.is_empty());
        assert_eq!(parser.get_accumulated_content(), "");
    }
}