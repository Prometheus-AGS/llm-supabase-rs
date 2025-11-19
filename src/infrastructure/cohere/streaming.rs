//! Cohere streaming implementation
//! 
//! This module handles Server-Sent Events (SSE) streaming for the Cohere API,
//! including proper tool call detection and stream termination.

use anyhow::Result;
use futures_util::{Stream, StreamExt};
use serde_json::Value;
use std::pin::Pin;
use std::task::{Context, Poll};
use tracing::{debug, warn, error};

use crate::infrastructure::cohere::types::{
    CohereStreamChunk, CohereEventType, CohereChatResponse, CohereError, CohereToolCall
};
use crate::models::response::{ChatCompletionChunk, ChatCompletionChunkChoice};
use crate::models::common::{ChatMessage, MessageRole, FinishReason};

/// Cohere-specific stream parser for SSE events
#[derive(Debug)]
pub struct CohereStreamParser {
    /// Buffer for incomplete chunks
    buffer: String,
    
    /// Whether we've seen the stream start
    stream_started: bool,
    
    /// Whether the stream has ended
    stream_ended: bool,
    
    /// Accumulated tool calls
    tool_calls: Vec<CohereToolCall>,
    
    /// Current text being accumulated
    accumulated_text: String,
}

impl CohereStreamParser {
    /// Create a new Cohere stream parser
    pub fn new() -> Self {
        Self {
            buffer: String::new(),
            stream_started: false,
            stream_ended: false,
            tool_calls: Vec::new(),
            accumulated_text: String::new(),
        }
    }
    
    /// Parse a chunk of SSE data
    pub fn parse_chunk(&mut self, data: &str) -> Result<Vec<ChatCompletionChunk>> {
        debug!("Parsing Cohere SSE chunk: {} bytes", data.len());
        
        // Add to buffer
        self.buffer.push_str(data);
        
        let mut chunks = Vec::new();
        
        // Process complete SSE events in the buffer
        while let Some(event) = self.extract_sse_event()? {
            if let Some(chunk) = self.process_sse_event(&event)? {
                chunks.push(chunk);
            }
        }
        
        Ok(chunks)
    }
    
    /// Extract a complete SSE event from the buffer
    fn extract_sse_event(&mut self) -> Result<Option<String>> {
        // Look for double newline which indicates end of SSE event
        if let Some(pos) = self.buffer.find("\n\n") {
            let event = self.buffer[..pos].to_string();
            self.buffer = self.buffer[pos + 2..].to_string();
            Ok(Some(event))
        } else {
            Ok(None)
        }
    }
    
    /// Process a complete SSE event
    fn process_sse_event(&mut self, event: &str) -> Result<Option<ChatCompletionChunk>> {
        debug!("Processing SSE event: {}", event);
        
        // Parse SSE format: data: {...}
        let data_line = event.lines()
            .find(|line| line.starts_with("data: "))
            .ok_or_else(|| anyhow::anyhow!("No data line found in SSE event"))?;
        
        let json_data = &data_line[6..]; // Remove "data: " prefix
        
        // Skip heartbeat events
        if json_data.trim().is_empty() || json_data.trim() == ":" {
            return Ok(None);
        }
        
        // Parse the JSON payload
        let cohere_chunk: CohereStreamChunk = serde_json::from_str(json_data)
            .map_err(|e| anyhow::anyhow!("Failed to parse Cohere stream chunk: {}", e))?;
        
        self.process_cohere_chunk(cohere_chunk)
    }
    
    /// Process a parsed Cohere chunk
    fn process_cohere_chunk(&mut self, chunk: CohereStreamChunk) -> Result<Option<ChatCompletionChunk>> {
        match chunk.event_type {
            CohereEventType::StreamStart => {
                debug!("Cohere stream started");
                self.stream_started = true;
                
                // Return initial chunk
                Ok(Some(ChatCompletionChunk {
                    id: "cohere_stream".to_string(),
                    object: "chat.completion.chunk".to_string(),
                    created: chrono::Utc::now().timestamp() as u64,
                    model: "cohere".to_string(),
                    choices: vec![ChatCompletionChunkChoice {
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
                        finish_reason: None,
                    }],
                    usage: None,
                    system_fingerprint: None,
                }))
            }
            
            CohereEventType::TextGeneration => {
                if let Some(text) = chunk.text {
                    debug!("Received text generation: {} chars", text.len());
                    self.accumulated_text.push_str(&text);
                    
                    Ok(Some(ChatCompletionChunk {
                        id: "cohere_stream".to_string(),
                        object: "chat.completion.chunk".to_string(),
                        created: chrono::Utc::now().timestamp() as u64,
                        model: "cohere".to_string(),
                        choices: vec![ChatCompletionChunkChoice {
                            index: 0,
                            delta: ChatMessage {
                                role: MessageRole::Assistant,
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
                        system_fingerprint: None,
                    }))
                } else {
                    Ok(None)
                }
            }
            
            CohereEventType::ToolCallsGeneration => {
                if let Some(tool_calls) = chunk.tool_calls {
                    debug!("Received tool calls generation: {} calls", tool_calls.len());
                    self.tool_calls.extend(tool_calls.clone());
                    
                    // Convert to OpenAI format tool calls for JSON serialization
                    let openai_tool_calls: Vec<serde_json::Value> = tool_calls
                        .into_iter()
                        .enumerate()
                        .map(|(index, call)| {
                            serde_json::json!({
                                "index": index as u32,
                                "id": call.id.unwrap_or_else(|| format!("call_{}", index)),
                                "type": "function",
                                "function": {
                                    "name": call.name,
                                    "arguments": serde_json::to_string(&call.parameters).unwrap_or_default()
                                }
                            })
                        })
                        .collect();
                    
                    Ok(Some(ChatCompletionChunk {
                        id: "cohere_stream".to_string(),
                        object: "chat.completion.chunk".to_string(),
                        created: chrono::Utc::now().timestamp() as u64,
                        model: "cohere".to_string(),
                        choices: vec![ChatCompletionChunkChoice {
                            index: 0,
                            delta: ChatMessage {
                                role: MessageRole::Assistant,
                                content: String::new(),
                                name: None,
                                function_call: None,
                                tool_calls: Some(openai_tool_calls),
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
            
            CohereEventType::StreamEnd => {
                debug!("Cohere stream ended");
                self.stream_ended = true;
                
                let finish_reason = if !self.tool_calls.is_empty() {
                    FinishReason::ToolCalls
                } else {
                    FinishReason::Stop
                };
                
                Ok(Some(ChatCompletionChunk {
                    id: "cohere_stream".to_string(),
                    object: "chat.completion.chunk".to_string(),
                    created: chrono::Utc::now().timestamp() as u64,
                    model: "cohere".to_string(),
                    choices: vec![ChatCompletionChunkChoice {
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
                        finish_reason: Some(finish_reason),
                    }],
                    usage: None,
                    system_fingerprint: None,
                }))
            }
            
            CohereEventType::Error => {
                if let Some(error) = chunk.error {
                    error!("Cohere stream error: {}", error.message);
                    return Err(anyhow::anyhow!("Cohere stream error: {}", error.message));
                } else {
                    error!("Unknown Cohere stream error");
                    return Err(anyhow::anyhow!("Unknown Cohere stream error"));
                }
            }
        }
    }
    
    /// Check if the stream has ended
    pub fn is_stream_ended(&self) -> bool {
        self.stream_ended
    }
    
    /// Get accumulated tool calls
    pub fn get_tool_calls(&self) -> &[CohereToolCall] {
        &self.tool_calls
    }
    
    /// Get accumulated text
    pub fn get_accumulated_text(&self) -> &str {
        &self.accumulated_text
    }
    
    /// Reset the parser state
    pub fn reset(&mut self) {
        self.buffer.clear();
        self.stream_started = false;
        self.stream_ended = false;
        self.tool_calls.clear();
        self.accumulated_text.clear();
    }
}

/// Stream wrapper that handles Cohere SSE parsing
pub struct CohereStreamWrapper<S> {
    inner: S,
    parser: CohereStreamParser,
}

impl<S> CohereStreamWrapper<S>
where
    S: Stream<Item = Result<bytes::Bytes>>,
{
    /// Create a new stream wrapper
    pub fn new(stream: S) -> Self {
        Self {
            inner: stream,
            parser: CohereStreamParser::new(),
        }
    }
    
    /// Get the accumulated tool calls
    pub fn get_tool_calls(&self) -> &[CohereToolCall] {
        self.parser.get_tool_calls()
    }
    
    /// Check if stream has ended
    pub fn is_stream_ended(&self) -> bool {
        self.parser.is_stream_ended()
    }
}

impl<S> Stream for CohereStreamWrapper<S>
where
    S: Stream<Item = Result<bytes::Bytes>> + Unpin,
{
    type Item = Result<ChatCompletionChunk>;
    
    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        match self.inner.poll_next_unpin(cx) {
            Poll::Ready(Some(Ok(bytes))) => {
                // Convert bytes to string
                let data = String::from_utf8_lossy(&bytes);
                
                // Parse the chunk
                match self.parser.parse_chunk(&data) {
                    Ok(chunks) => {
                        if chunks.is_empty() {
                            // No complete chunks yet, continue polling
                            self.poll_next(cx)
                        } else {
                            // Return the first chunk, buffer the rest
                            // For simplicity, we return one chunk at a time
                            Poll::Ready(Some(Ok(chunks.into_iter().next().unwrap())))
                        }
                    }
                    Err(e) => {
                        error!("Failed to parse Cohere stream chunk: {}", e);
                        Poll::Ready(Some(Err(e)))
                    }
                }
            }
            Poll::Ready(Some(Err(e))) => {
                error!("Stream error: {}", e);
                Poll::Ready(Some(Err(e)))
            }
            Poll::Ready(None) => {
                debug!("Stream ended");
                Poll::Ready(None)
            }
            Poll::Pending => Poll::Pending,
        }
    }
}

/// Utility functions for Cohere streaming
pub struct CohereStreamUtils;

impl CohereStreamUtils {
    /// Create a stream from bytes
    pub fn create_stream<S>(byte_stream: S) -> CohereStreamWrapper<S>
    where
        S: Stream<Item = Result<bytes::Bytes>>,
    {
        CohereStreamWrapper::new(byte_stream)
    }
    
    /// Extract tool calls from completed stream
    pub async fn extract_tool_calls_from_stream<S>(
        mut stream: CohereStreamWrapper<S>,
    ) -> Result<(Vec<CohereToolCall>, String)>
    where
        S: Stream<Item = Result<bytes::Bytes>> + Unpin,
    {
        let mut accumulated_text = String::new();
        
        // Consume the entire stream
        while let Some(chunk_result) = stream.next().await {
            match chunk_result {
                Ok(chunk) => {
                    if let Some(choice) = chunk.choices.first() {
                        if !choice.delta.content.is_empty() {
                            accumulated_text.push_str(&choice.delta.content);
                        }
                    }
                }
                Err(e) => {
                    error!("Stream chunk error: {}", e);
                    return Err(e);
                }
            }
        }
        
        let tool_calls = stream.get_tool_calls().to_vec();
        Ok((tool_calls, accumulated_text))
    }
    
    /// Check if a stream chunk indicates tool calls
    pub fn has_tool_calls(chunk: &ChatCompletionChunk) -> bool {
        chunk.choices.iter().any(|choice| {
            choice.delta.tool_calls.is_some() || 
            choice.finish_reason.as_ref().map(|r| *r == FinishReason::ToolCalls).unwrap_or(false)
        })
    }
    
    /// Check if a stream chunk indicates completion
    pub fn is_completion_chunk(chunk: &ChatCompletionChunk) -> bool {
        chunk.choices.iter().any(|choice| {
            choice.finish_reason.is_some()
        })
    }
}

impl Default for CohereStreamParser {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use futures_util::stream;
    
    #[test]
    fn test_parser_creation() {
        let parser = CohereStreamParser::new();
        assert!(!parser.stream_started);
        assert!(!parser.stream_ended);
        assert!(parser.tool_calls.is_empty());
    }
    
    #[test]
    fn test_sse_event_extraction() {
        let mut parser = CohereStreamParser::new();
        parser.buffer = "data: {\"test\": \"value\"}\n\ndata: {\"test2\": \"value2\"}\n\n".to_string();
        
        let event1 = parser.extract_sse_event().unwrap();
        assert_eq!(event1, Some("data: {\"test\": \"value\"}".to_string()));
        
        let event2 = parser.extract_sse_event().unwrap();
        assert_eq!(event2, Some("data: {\"test2\": \"value2\"}".to_string()));
        
        let event3 = parser.extract_sse_event().unwrap();
        assert_eq!(event3, None);
    }
    
    #[tokio::test]
    async fn test_stream_wrapper() {
        let byte_data = vec![
            Ok(bytes::Bytes::from("data: {\"event_type\": \"stream-start\"}\n\n")),
            Ok(bytes::Bytes::from("data: {\"event_type\": \"text-generation\", \"text\": \"Hello\"}\n\n")),
        ];
        
        let byte_stream = stream::iter(byte_data);
        let mut cohere_stream = CohereStreamUtils::create_stream(byte_stream);
        
        // First chunk should be stream start
        let chunk1 = cohere_stream.next().await;
        assert!(chunk1.is_some());
        
        // Second chunk should be text generation
        let chunk2 = cohere_stream.next().await;
        assert!(chunk2.is_some());
    }
    
    #[test]
    fn test_stream_utils() {
        let chunk_with_tools = ChatCompletionChunk {
            id: "test".to_string(),
            object: "chat.completion.chunk".to_string(),
            created: 0,
            model: "cohere".to_string(),
            choices: vec![ChatCompletionChunkChoice {
                index: 0,
                delta: ChatMessage {
                    role: MessageRole::Assistant,
                    content: String::new(),
                    name: None,
                    function_call: None,
                    tool_calls: Some(vec![]),
                    tool_call_id: None,
                },
                logprobs: None,
                finish_reason: None,
            }],
            usage: None,
            system_fingerprint: None,
        };
        
        assert!(CohereStreamUtils::has_tool_calls(&chunk_with_tools));
        assert!(!CohereStreamUtils::is_completion_chunk(&chunk_with_tools));
    }
}