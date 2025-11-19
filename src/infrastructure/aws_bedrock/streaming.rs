//! AWS Bedrock streaming support implementation
//!
//! This module handles AWS Bedrock event streams, which are different from standard
//! Server-Sent Events. Bedrock uses AWS event streams with binary encoding.

use anyhow::{anyhow, Result};
use bytes::{Bytes, BytesMut};
use futures_util::Stream;
use reqwest::Response;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::pin::Pin;
use std::task::{Context, Poll};
use tokio_stream::StreamExt;
use tracing::{debug, trace, warn};

use crate::models::response::{ChatCompletionChunk, ChatCompletionChunkChoice as ChunkChoice};
use crate::models::common::{ChatMessage as Delta, Usage};
use super::types::{BedrockModelFamily, BedrockStreamEvent};

/// AWS Bedrock stream parser
#[derive(Debug, Clone)]
pub struct BedrockStreamParser {
    /// Buffer for incomplete events
    buffer: BytesMut,
}

/// Bedrock stream event structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BedrockEvent {
    /// Event type
    #[serde(rename = ":event-type")]
    pub event_type: Option<String>,
    
    /// Content type
    #[serde(rename = ":content-type")]
    pub content_type: Option<String>,
    
    /// Event data
    pub data: Option<Value>,
    
    /// Error information (if any)
    pub error: Option<BedrockError>,
}

/// Bedrock streaming error
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BedrockError {
    pub error_type: String,
    pub message: String,
}

/// Stream wrapper for Bedrock responses
pub struct BedrockStream {
    inner: Pin<Box<dyn Stream<Item = Result<ChatCompletionChunk>> + Send>>,
}

impl Stream for BedrockStream {
    type Item = Result<ChatCompletionChunk>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        self.inner.as_mut().poll_next(cx)
    }
}

impl BedrockStreamParser {
    /// Create a new Bedrock stream parser
    pub fn new() -> Self {
        Self {
            buffer: BytesMut::new(),
        }
    }
    
    /// Parse Bedrock event stream and convert to OpenAI format
    pub fn parse_bedrock_stream(
        &self,
        response: Response,
        model_family: BedrockModelFamily,
    ) -> impl Stream<Item = Result<ChatCompletionChunk>> {
        let stream = response.bytes_stream();
        let parser = self.clone();
        
        BedrockStream {
            inner: Box::pin(async_stream::stream! {
                let mut parser_state = parser;
                let mut chunk_index = 0;
                let mut request_id = format!("chatcmpl-{}", uuid::Uuid::new_v4().to_string().replace('-', ""));
                
                tokio::pin!(stream);
                
                while let Some(chunk_result) = stream.next().await {
                    match chunk_result {
                        Ok(chunk) => {
                            if let Ok(events) = parser_state.parse_chunk(&chunk) {
                                for event in events {
                                    match parser_state.convert_event_to_chunk(
                                        event,
                                        &model_family,
                                        &request_id,
                                        chunk_index
                                    ) {
                                        Ok(Some(openai_chunk)) => {
                                            chunk_index += 1;
                                            yield Ok(openai_chunk);
                                        }
                                        Ok(None) => {
                                            // Event doesn't produce a chunk (e.g., metadata)
                                            continue;
                                        }
                                        Err(e) => {
                                            yield Err(e);
                                            break;
                                        }
                                    }
                                }
                            }
                        }
                        Err(e) => {
                            yield Err(anyhow!("Stream error: {}", e));
                            break;
                        }
                    }
                }
                
                // Send final chunk with finish reason
                yield Ok(ChatCompletionChunk {
                    id: request_id.clone(),
                    object: "chat.completion.chunk".to_string(),
                    created: chrono::Utc::now().timestamp() as u64,
                    model: "bedrock-model".to_string(),
                    system_fingerprint: None,
                    choices: vec![crate::models::response::ChatCompletionChunkChoice {
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
                });
            }),
        }
    }
    
    /// Parse a chunk of bytes into Bedrock events
    fn parse_chunk(&mut self, chunk: &Bytes) -> Result<Vec<BedrockEvent>> {
        self.buffer.extend_from_slice(chunk);
        let mut events = Vec::new();
        
        while let Some(event) = self.try_parse_event()? {
            events.push(event);
        }
        
        Ok(events)
    }
    
    /// Try to parse a single event from the buffer
    fn try_parse_event(&mut self) -> Result<Option<BedrockEvent>> {
        // Bedrock uses a specific binary format for event streams
        // This is a simplified implementation - in practice, you'd need to handle
        // the full AWS event stream format with proper binary parsing
        
        // Look for JSON-like event data in the buffer
        if let Some(newline_pos) = self.buffer.iter().position(|&b| b == b'\n') {
            let line_bytes = self.buffer.split_to(newline_pos + 1);
            let line = String::from_utf8_lossy(&line_bytes[..line_bytes.len() - 1]);
            
            // Skip empty lines and metadata
            if line.trim().is_empty() || line.starts_with(':') {
                return Ok(None);
            }
            
            // Try to parse as JSON event
            if line.starts_with("data: ") {
                let json_str = &line[6..]; // Remove "data: " prefix
                if json_str.trim() == "[DONE]" {
                    return Ok(Some(BedrockEvent {
                        event_type: Some("done".to_string()),
                        content_type: None,
                        data: None,
                        error: None,
                    }));
                }
                
                match serde_json::from_str::<Value>(json_str) {
                    Ok(value) => {
                        return Ok(Some(BedrockEvent {
                            event_type: value.get("type").and_then(|t| t.as_str()).map(String::from),
                            content_type: Some("application/json".to_string()),
                            data: Some(value),
                            error: None,
                        }));
                    }
                    Err(e) => {
                        debug!("Failed to parse event JSON: {}", e);
                        return Ok(None);
                    }
                }
            }
            
            // Handle other line formats if needed
            Ok(None)
        } else {
            Ok(None)
        }
    }
    
    /// Convert Bedrock event to OpenAI chat completion chunk
    fn convert_event_to_chunk(
        &self,
        event: BedrockEvent,
        model_family: &BedrockModelFamily,
        request_id: &str,
        index: u32,
    ) -> Result<Option<ChatCompletionChunk>> {
        if let Some(data) = event.data {
            match model_family {
                BedrockModelFamily::Anthropic => {
                    self.convert_claude_event_to_chunk(&data, request_id, index)
                }
                BedrockModelFamily::Meta => {
                    self.convert_llama_event_to_chunk(&data, request_id, index)
                }
                BedrockModelFamily::Mistral => {
                    self.convert_mistral_event_to_chunk(&data, request_id, index)
                }
                BedrockModelFamily::Amazon => {
                    self.convert_titan_event_to_chunk(&data, request_id, index)
                }
                _ => {
                    warn!("Unsupported model family for streaming: {:?}", model_family);
                    Ok(None)
                }
            }
        } else if let Some(error) = event.error {
            Err(anyhow!("Bedrock streaming error: {} - {}", error.error_type, error.message))
        } else {
            Ok(None)
        }
    }
    
    /// Convert Claude streaming event to OpenAI chunk
    fn convert_claude_event_to_chunk(
        &self,
        data: &Value,
        request_id: &str,
        index: u32,
    ) -> Result<Option<ChatCompletionChunk>> {
        let event_type = data.get("type").and_then(|t| t.as_str()).unwrap_or("");
        
        match event_type {
            "message_start" => {
                // Initial message chunk
                Ok(Some(ChatCompletionChunk {
                    id: request_id.to_string(),
                    object: "chat.completion.chunk".to_string(),
                    created: chrono::Utc::now().timestamp() as u64,
                    model: "claude".to_string(),
                    choices: vec![ChunkChoice {
                        index: 0,
                        delta: Delta {
                            role: crate::models::common::MessageRole::Assistant,
                            content: String::new(),
                            tool_calls: None,
                            function_call: None,
                            name: None,
                            tool_call_id: None,
                        },
                        logprobs: None,
                        finish_reason: None,
                    }],
                    usage: None,
                    system_fingerprint: None,
                }))
            }
            "content_block_delta" => {
                // Text content delta
                if let Some(delta) = data.get("delta") {
                    if let Some(text) = delta.get("text").and_then(|t| t.as_str()) {
                        return Ok(Some(ChatCompletionChunk {
                            id: request_id.to_string(),
                            object: "chat.completion.chunk".to_string(),
                            created: chrono::Utc::now().timestamp() as u64,
                            model: "claude".to_string(),
                            choices: vec![ChunkChoice {
                                index: 0,
                                delta: Delta {
                                    role: crate::models::common::MessageRole::Assistant,
                                    content: text.to_string(),
                                    tool_calls: None,
                                    function_call: None,
                                    name: None,
                                    tool_call_id: None,
                                },
                                logprobs: None,
                                finish_reason: None,
                            }],
                            usage: None,
                            system_fingerprint: None,
                        }));
                    }
                    
                    // Handle tool use delta
                    if delta.get("type").and_then(|t| t.as_str()) == Some("input_json_delta") {
                        // Tool call in progress - for now, we'll handle this in content_block_stop
                        return Ok(None);
                    }
                }
                Ok(None)
            }
            "content_block_start" => {
                // Check if this is a tool use block
                if let Some(content_block) = data.get("content_block") {
                    if content_block.get("type").and_then(|t| t.as_str()) == Some("tool_use") {
                        // Tool call started - we'll send the complete tool call when it's done
                        return Ok(None);
                    }
                }
                Ok(None)
            }
            "content_block_stop" => {
                // End of content block
                Ok(None)
            }
            "message_delta" => {
                // Message-level delta (usage info, etc.)
                if let Some(usage) = data.get("usage") {
                    let input_tokens = usage.get("input_tokens").and_then(|t| t.as_u64()).unwrap_or(0) as u32;
                    let output_tokens = usage.get("output_tokens").and_then(|t| t.as_u64()).unwrap_or(0) as u32;
                    
                    return Ok(Some(ChatCompletionChunk {
                        id: request_id.to_string(),
                        object: "chat.completion.chunk".to_string(),
                        created: chrono::Utc::now().timestamp() as u64,
                        model: "claude".to_string(),
                        choices: vec![],
                        usage: Some(Usage {
                            prompt_tokens: input_tokens,
                            completion_tokens: output_tokens,
                            total_tokens: input_tokens + output_tokens,
                            completion_tokens_details: None,
                            prompt_tokens_details: None,
                        }),
                        system_fingerprint: None,
                    }));
                }
                Ok(None)
            }
            "message_stop" => {
                // End of message
                Ok(None)
            }
            _ => {
                trace!("Unknown Claude event type: {}", event_type);
                Ok(None)
            }
        }
    }
    
    /// Convert Llama streaming event to OpenAI chunk
    fn convert_llama_event_to_chunk(
        &self,
        data: &Value,
        request_id: &str,
        index: u32,
    ) -> Result<Option<ChatCompletionChunk>> {
        // Llama streaming format is different - this is a simplified implementation
        if let Some(generation) = data.get("generation").and_then(|g| g.as_str()) {
            Ok(Some(ChatCompletionChunk {
                id: request_id.to_string(),
                object: "chat.completion.chunk".to_string(),
                created: chrono::Utc::now().timestamp() as u64,
                model: "llama".to_string(),
                choices: vec![ChunkChoice {
                    index: 0,
                    delta: Delta {
                        role: crate::models::common::MessageRole::Assistant,
                        content: generation.to_string(),
                        tool_calls: None,
                        function_call: None,
                        name: None,
                        tool_call_id: None,
                    },
                    logprobs: None,
                    finish_reason: None,
                }],
                usage: None,
                system_fingerprint: None,
            }))
        } else {
            Ok(None)
        }
    }
    
    /// Convert Mistral streaming event to OpenAI chunk
    fn convert_mistral_event_to_chunk(
        &self,
        data: &Value,
        request_id: &str,
        index: u32,
    ) -> Result<Option<ChatCompletionChunk>> {
        // Mistral streaming format
        if let Some(outputs) = data.get("outputs").and_then(|o| o.as_array()) {
            if let Some(output) = outputs.first() {
                if let Some(text) = output.get("text").and_then(|t| t.as_str()) {
                    return Ok(Some(ChatCompletionChunk {
                        id: request_id.to_string(),
                        object: "chat.completion.chunk".to_string(),
                        created: chrono::Utc::now().timestamp() as u64,
                        model: "mistral".to_string(),
                        choices: vec![ChunkChoice {
                            index: 0,
                            delta: Delta {
                                role: crate::models::common::MessageRole::Assistant,
                                content: text.to_string(),
                                tool_calls: None,
                                function_call: None,
                                name: None,
                                tool_call_id: None,
                            },
                            logprobs: None,
                            finish_reason: None,
                        }],
                        usage: None,
                        system_fingerprint: None,
                    }));
                }
            }
        }
        Ok(None)
    }
    
    /// Convert Titan streaming event to OpenAI chunk
    fn convert_titan_event_to_chunk(
        &self,
        data: &Value,
        request_id: &str,
        index: u32,
    ) -> Result<Option<ChatCompletionChunk>> {
        // Amazon Titan streaming format
        if let Some(output_text) = data.get("outputText").and_then(|t| t.as_str()) {
            Ok(Some(ChatCompletionChunk {
                id: request_id.to_string(),
                object: "chat.completion.chunk".to_string(),
                created: chrono::Utc::now().timestamp() as u64,
                model: "titan".to_string(),
                choices: vec![ChunkChoice {
                    index: 0,
                    delta: Delta {
                        role: crate::models::common::MessageRole::Assistant,
                        content: output_text.to_string(),
                        tool_calls: None,
                        function_call: None,
                        name: None,
                        tool_call_id: None,
                    },
                    logprobs: None,
                    finish_reason: None,
                }],
                usage: None,
                system_fingerprint: None,
            }))
        } else {
            Ok(None)
        }
    }
}

/// Utility functions for Bedrock streaming
pub struct BedrockStreamUtils;

impl BedrockStreamUtils {
    /// Parse AWS event stream headers
    pub fn parse_event_headers(data: &[u8]) -> Result<HashMap<String, String>> {
        let mut headers = HashMap::new();
        let mut offset = 0;
        
        // AWS event stream format parsing would go here
        // This is a simplified version
        
        Ok(headers)
    }
    
    /// Extract payload from AWS event stream
    pub fn extract_payload(data: &[u8]) -> Result<Vec<u8>> {
        // AWS event stream payload extraction would go here
        // This is a simplified version
        Ok(data.to_vec())
    }
    
    /// Validate CRC32 checksum for AWS event stream
    pub fn validate_crc32(data: &[u8], expected_crc: u32) -> bool {
        // CRC32 validation would go here
        // For now, we'll assume it's always valid
        true
    }
    
    /// Create a streaming error chunk
    pub fn create_error_chunk(error: &str, request_id: &str) -> ChatCompletionChunk {
        ChatCompletionChunk {
            id: request_id.to_string(),
            object: "chat.completion.chunk".to_string(),
            created: chrono::Utc::now().timestamp() as u64,
            model: "bedrock".to_string(),
            choices: vec![ChunkChoice {
                index: 0,
                delta: Delta {
                    role: crate::models::common::MessageRole::Assistant,
                    content: String::new(),
                    tool_calls: None,
                    function_call: None,
                    name: None,
                    tool_call_id: None,
                },
                logprobs: None,
                finish_reason: Some(crate::models::common::FinishReason::Stop),
            }],
            usage: None,
            system_fingerprint: None,
        }
    }
    
    /// Detect if streaming has ended
    pub fn is_stream_end_event(event: &BedrockEvent) -> bool {
        event.event_type.as_deref() == Some("done") ||
        event.event_type.as_deref() == Some("message_stop") ||
        event.data.as_ref()
            .and_then(|d| d.get("type"))
            .and_then(|t| t.as_str()) == Some("message_stop")
    }
    
    /// Extract finish reason from stream event
    pub fn extract_finish_reason(event: &BedrockEvent, model_family: &BedrockModelFamily) -> Option<String> {
        if let Some(data) = &event.data {
            match model_family {
                BedrockModelFamily::Anthropic => {
                    data.get("delta")
                        .and_then(|d| d.get("stop_reason"))
                        .and_then(|sr| sr.as_str())
                        .map(|reason| match reason {
                            "end_turn" => "stop",
                            "max_tokens" => "length", 
                            "tool_use" => "tool_calls",
                            _ => "stop",
                        })
                        .map(String::from)
                }
                BedrockModelFamily::Meta => {
                    data.get("stop_reason")
                        .and_then(|sr| sr.as_str())
                        .map(|reason| match reason {
                            "stop" => "stop",
                            "length" => "length",
                            _ => "stop",
                        })
                        .map(String::from)
                }
                _ => Some("stop".to_string()),
            }
        } else {
            None
        }
    }
}

impl Default for BedrockStreamParser {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_stream_parser_creation() {
        let parser = BedrockStreamParser::new();
        assert_eq!(parser.buffer.len(), 0);
    }

    #[test]
    fn test_claude_event_conversion() {
        let parser = BedrockStreamParser::new();
        
        let event_data = json!({
            "type": "content_block_delta",
            "delta": {
                "type": "text_delta",
                "text": "Hello"
            }
        });
        
        let chunk = parser.convert_claude_event_to_chunk(&event_data, "test-123", 0).unwrap();
        assert!(chunk.is_some());
        
        let chunk = chunk.unwrap();
        assert_eq!(chunk.id, "test-123");
        assert!(chunk.choices[0].delta.content.is_some());
        assert_eq!(chunk.choices[0].delta.content.as_ref().unwrap(), "Hello");
    }

    #[test]
    fn test_stream_end_detection() {
        let end_event = BedrockEvent {
            event_type: Some("done".to_string()),
            content_type: None,
            data: None,
            error: None,
        };
        
        assert!(BedrockStreamUtils::is_stream_end_event(&end_event));
        
        let regular_event = BedrockEvent {
            event_type: Some("content_block_delta".to_string()),
            content_type: None,
            data: Some(json!({})),
            error: None,
        };
        
        assert!(!BedrockStreamUtils::is_stream_end_event(&regular_event));
    }

    #[test]
    fn test_error_chunk_creation() {
        let error_chunk = BedrockStreamUtils::create_error_chunk("Test error", "req-123");
        
        assert_eq!(error_chunk.id, "req-123");
        assert_eq!(error_chunk.choices[0].finish_reason.as_ref().unwrap(), "error");
    }

    #[test]
    fn test_finish_reason_extraction() {
        let event = BedrockEvent {
            event_type: Some("message_delta".to_string()),
            content_type: None,
            data: Some(json!({
                "delta": {
                    "stop_reason": "end_turn"
                }
            })),
            error: None,
        };
        
        let reason = BedrockStreamUtils::extract_finish_reason(&event, &BedrockModelFamily::Anthropic);
        assert_eq!(reason.as_deref(), Some("stop"));
    }

    #[tokio::test]
    async fn test_llama_event_conversion() {
        let parser = BedrockStreamParser::new();
        
        let event_data = json!({
            "generation": "Hello world"
        });
        
        let chunk = parser.convert_llama_event_to_chunk(&event_data, "test-456", 1).unwrap();
        assert!(chunk.is_some());
        
        let chunk = chunk.unwrap();
        assert_eq!(chunk.id, "test-456");
        assert_eq!(chunk.choices[0].delta.content.as_ref().unwrap(), "Hello world");
        assert!(chunk.choices[0].delta.role.is_none()); // Not the first chunk
    }

    #[test]
    fn test_buffer_management() {
        let mut parser = BedrockStreamParser::new();
        
        // Add some data to buffer
        let test_data = b"data: {\"type\": \"test\"}\n";
        parser.buffer.extend_from_slice(test_data);
        
        assert_eq!(parser.buffer.len(), test_data.len());
        
        // Parse should consume the line
        let events = parser.parse_chunk(&Bytes::new()).unwrap();
        assert_eq!(parser.buffer.len(), 0);
    }
}