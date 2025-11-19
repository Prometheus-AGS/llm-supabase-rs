use anyhow::{Context, Result};
use futures_util::{Stream, StreamExt};
use reqwest::Response;
use serde_json::Value;
use std::pin::Pin;
use tokio::io::AsyncBufReadExt;
use tokio_stream::wrappers::LinesStream;
use tokio_util::io::StreamReader;
use tracing::{debug, trace, warn, error};

use crate::models::response::ChatCompletionChunk;

/// OpenAI Server-Sent Events (SSE) stream parser
/// Handles the OpenAI streaming format: data: {json}\n\ndata: [DONE]\n\n
pub struct OpenAIStreamParser;

impl OpenAIStreamParser {
    pub fn new() -> Self {
        Self
    }

    /// Parse an OpenAI streaming response into ChatCompletionChunk items
    pub async fn parse_stream<S, E>(
        &self,
        byte_stream: S,
    ) -> Result<impl Stream<Item = Result<ChatCompletionChunk>>>
    where
        S: Stream<Item = std::result::Result<bytes::Bytes, E>> + Send + 'static,
        E: Into<Box<dyn std::error::Error + Send + Sync>> + Send + 'static,
    {
        debug!("Parsing OpenAI Server-Sent Events stream");

        // Convert bytes stream to lines stream
        let reader = StreamReader::new(byte_stream.map(|result| {
            result.map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))
        }));
        let buf_reader = tokio::io::BufReader::new(reader);

        let lines_stream = LinesStream::new(buf_reader.lines());

        // Process SSE lines and convert to chunks
        // Note: We can't borrow self in the closure, so we use a static method approach
        let chunk_stream = lines_stream.filter_map(move |line_result| async move {
            match line_result {
                Ok(line) => Self::process_sse_line_static(&line).await,
                Err(e) => {
                    error!("Error reading line from stream: {}", e);
                    Some(Err(anyhow::anyhow!("Stream read error: {}", e)))
                }
            }
        });

        Ok(chunk_stream)
    }

    /// Process a single Server-Sent Events line (static version for use in closures)
    async fn process_sse_line_static(line: &str) -> Option<Result<ChatCompletionChunk>> {
        Self::process_sse_line_impl(line).await
    }

    /// Process a single Server-Sent Events line
    async fn process_sse_line(&self, line: &str) -> Option<Result<ChatCompletionChunk>> {
        Self::process_sse_line_impl(line).await
    }

    /// Internal implementation
    async fn process_sse_line_impl(line: &str) -> Option<Result<ChatCompletionChunk>> {
        let line = line.trim();

        // Skip empty lines
        if line.is_empty() {
            return None;
        }

        // Handle SSE data lines
        if let Some(data) = line.strip_prefix("data: ") {
            return Self::process_data_line_static(data).await;
        }

        // Skip other SSE fields (event:, id:, retry:, etc.)
        if line.contains(':') {
            trace!("Skipping SSE field line: {}", line);
            return None;
        }

        // Log unexpected content
        warn!("Unexpected line in SSE stream: {}", line);
        None
    }

    /// Process a data line from the SSE stream (static version)
    async fn process_data_line_static(data: &str) -> Option<Result<ChatCompletionChunk>> {
        let data = data.trim();

        // Handle the [DONE] marker
        if data == "[DONE]" {
            debug!("Received [DONE] marker, stream completed");
            return None; // End of stream
        }

        // Parse JSON data
        match serde_json::from_str::<Value>(data) {
            Ok(json_value) => {
                trace!("Parsed JSON chunk: {}", json_value);
                
                // Convert to ChatCompletionChunk
                match Self::convert_to_chunk_static(json_value).await {
                    Ok(chunk) => Some(Ok(chunk)),
                    Err(e) => {
                        error!("Failed to convert JSON to chunk: {}", e);
                        Some(Err(e))
                    }
                }
            }
            Err(e) => {
                error!("Failed to parse JSON data: {} - Data: {}", e, data);
                Some(Err(anyhow::anyhow!("JSON parse error: {}", e)))
            }
        }
    }

    /// Process a data line from the SSE stream
    async fn process_data_line(&self, data: &str) -> Option<Result<ChatCompletionChunk>> {
        Self::process_data_line_static(data).await
    }

    /// Convert JSON value to ChatCompletionChunk (static version)
    async fn convert_to_chunk_static(json: Value) -> Result<ChatCompletionChunk> {
        // OpenAI streaming response structure
        let chunk: ChatCompletionChunk = serde_json::from_value(json)
            .context("Failed to deserialize OpenAI streaming chunk")?;

        trace!("Converted chunk with {} choices", chunk.choices.len());
        Ok(chunk)
    }

    /// Create a test stream for development/testing
    pub fn create_test_stream() -> impl Stream<Item = Result<ChatCompletionChunk>> {
        use tokio_stream::iter;

        let test_chunks = vec![
            // First chunk with role
            Ok(ChatCompletionChunk {
                id: "chatcmpl-test123".to_string(),
                object: "chat.completion.chunk".to_string(),
                created: 1699999999,
                model: "gpt-4o".to_string(),
                system_fingerprint: None,
                usage: None,
                choices: vec![
                    crate::models::response::ChunkChoice {
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
                    }
                ],
            }),
            // Content chunks
            Ok(ChatCompletionChunk {
                id: "chatcmpl-test123".to_string(),
                object: "chat.completion.chunk".to_string(),
                created: 1699999999,
                model: "gpt-4o".to_string(),
                system_fingerprint: None,
                usage: None,
                choices: vec![
                    crate::models::response::ChunkChoice {
                        index: 0,
                        delta: crate::models::common::ChatMessage {
                            role: crate::models::common::MessageRole::Assistant,
                            content: "Hello".to_string(),
                            name: None,
                            function_call: None,
                            tool_calls: None,
                            tool_call_id: None,
                        },
                        logprobs: None,
                        finish_reason: None,
                    }
                ],
            }),
            Ok(ChatCompletionChunk {
                id: "chatcmpl-test123".to_string(),
                object: "chat.completion.chunk".to_string(),
                created: 1699999999,
                model: "gpt-4o".to_string(),
                system_fingerprint: None,
                usage: None,
                choices: vec![
                    crate::models::response::ChunkChoice {
                        index: 0,
                        delta: crate::models::common::ChatMessage {
                            role: crate::models::common::MessageRole::Assistant,
                            content: " world!".to_string(),
                            name: None,
                            function_call: None,
                            tool_calls: None,
                            tool_call_id: None,
                        },
                        logprobs: None,
                        finish_reason: None,
                    }
                ],
            }),
            // Final chunk with finish_reason
            Ok(ChatCompletionChunk {
                id: "chatcmpl-test123".to_string(),
                object: "chat.completion.chunk".to_string(),
                created: 1699999999,
                model: "gpt-4o".to_string(),
                system_fingerprint: None,
                usage: None,
                choices: vec![
                    crate::models::response::ChunkChoice {
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
                    }
                ],
            }),
        ];

        iter(test_chunks)
    }

    /// Handle tool calls in streaming responses
    /// OpenAI can send tool calls across multiple chunks
    pub fn accumulate_tool_calls(&self, chunks: Vec<ChatCompletionChunk>) -> Result<Vec<Value>> {
        debug!("Accumulating tool calls from {} chunks", chunks.len());

        let mut tool_calls_map: std::collections::HashMap<usize, Value> = std::collections::HashMap::new();

        for chunk in chunks {
            for choice in chunk.choices {
                if let Some(ref delta_tool_calls) = choice.delta.tool_calls {
                    for tool_call_value in delta_tool_calls {
                        // OpenAI tool calls have an index to identify which call they belong to
                        if let Some(index) = tool_call_value.get("index").and_then(|i| i.as_u64()) {
                            let index = index as usize;
                            
                            // Merge this chunk into the accumulated tool call
                            let accumulated = tool_calls_map.entry(index).or_insert_with(|| {
                                serde_json::json!({
                                    "id": "",
                                    "type": "function",
                                    "function": {
                                        "name": "",
                                        "arguments": ""
                                    }
                                })
                            });

                            // Merge ID
                            if let Some(id) = tool_call_value.get("id") {
                                accumulated["id"] = id.clone();
                            }

                            // Merge function name
                            if let Some(function) = tool_call_value.get("function") {
                                if let Some(name) = function.get("name") {
                                    accumulated["function"]["name"] = name.clone();
                                }
                                
                                // Accumulate arguments
                                if let Some(args) = function.get("arguments") {
                                    if let Some(args_str) = args.as_str() {
                                        let current_args = accumulated["function"]["arguments"]
                                            .as_str().unwrap_or("");
                                        accumulated["function"]["arguments"] = 
                                            serde_json::Value::String(format!("{}{}", current_args, args_str));
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        // Convert map to sorted vector
        let mut tool_calls: Vec<(usize, Value)> = tool_calls_map.into_iter().collect();
        tool_calls.sort_by_key(|(index, _)| *index);
        let result: Vec<Value> = tool_calls.into_iter().map(|(_, call)| call).collect();

        debug!("Accumulated {} tool calls", result.len());
        Ok(result)
    }
}

/// Utility functions for working with OpenAI streaming responses
pub struct OpenAIStreamUtils;

impl OpenAIStreamUtils {
    /// Collect all chunks from a stream into a complete response
    pub async fn collect_chunks<S>(
        mut stream: S,
    ) -> Result<Vec<ChatCompletionChunk>>
    where
        S: Stream<Item = Result<ChatCompletionChunk>> + Unpin,
    {
        let mut chunks = Vec::new();

        while let Some(chunk_result) = stream.next().await {
            match chunk_result {
                Ok(chunk) => chunks.push(chunk),
                Err(e) => return Err(e),
            }
        }

        debug!("Collected {} chunks from stream", chunks.len());
        Ok(chunks)
    }

    /// Extract the complete content from streaming chunks
    pub fn extract_content(chunks: &[ChatCompletionChunk]) -> String {
        let mut content = String::new();

        for chunk in chunks {
            for choice in &chunk.choices {
                if !choice.delta.content.is_empty() {
                    content.push_str(&choice.delta.content);
                }
            }
        }

        content
    }

    /// Check if the stream has finished (has a finish_reason)
    pub fn is_stream_finished(chunks: &[ChatCompletionChunk]) -> bool {
        chunks.iter().any(|chunk| {
            chunk.choices.iter().any(|choice| {
                choice.finish_reason.is_some()
            })
        })
    }

    /// Get the finish reason from the chunks
    pub fn get_finish_reason(chunks: &[ChatCompletionChunk]) -> Option<String> {
        for chunk in chunks.iter().rev() {
            for choice in &chunk.choices {
                if let Some(ref reason) = choice.finish_reason {
                    return Some(reason.to_string());
                }
            }
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use futures_util::stream;
    use tokio_test;

    #[tokio::test]
    async fn test_sse_parsing() {
        let parser = OpenAIStreamParser::new();
        
        // Test [DONE] marker
        let result = parser.process_sse_line("data: [DONE]").await;
        assert!(result.is_none());

        // Test empty line
        let result = parser.process_sse_line("").await;
        assert!(result.is_none());

        // Test non-data line
        let result = parser.process_sse_line("event: completion").await;
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_collect_chunks() {
        let test_stream = OpenAIStreamParser::create_test_stream();
        let chunks = OpenAIStreamUtils::collect_chunks(test_stream).await.unwrap();
        
        assert!(chunks.len() > 0);
        
        let content = OpenAIStreamUtils::extract_content(&chunks);
        assert_eq!(content, "Hello world!");
        
        assert!(OpenAIStreamUtils::is_stream_finished(&chunks));
        assert_eq!(OpenAIStreamUtils::get_finish_reason(&chunks), Some("stop".to_string()));
    }

    #[test]
    fn test_tool_call_accumulation() {
        let parser = OpenAIStreamParser::new();
        
        // Create test chunks with fragmented tool call
        let chunks = vec![
            ChatCompletionChunk {
                id: "test".to_string(),
                object: "chat.completion.chunk".to_string(),
                created: 0,
                model: "gpt-4o".to_string(),
                system_fingerprint: None,
                usage: None,
                choices: vec![
                    crate::models::response::ChunkChoice {
                        index: 0,
                        delta: crate::models::common::ChatMessage {
                            role: crate::models::common::MessageRole::Assistant,
                            content: String::new(),
                            name: None,
                            function_call: None,
                            tool_calls: Some(vec![
                                serde_json::json!({
                                    "index": 0,
                                    "id": "call_123",
                                    "function": {
                                        "name": "get_weather",
                                        "arguments": "{\"location\":"
                                    }
                                })
                            ]),
                            tool_call_id: None,
                        },
                        logprobs: None,
                        finish_reason: None,
                    }
                ],
            },
            ChatCompletionChunk {
                id: "test".to_string(),
                object: "chat.completion.chunk".to_string(),
                created: 0,
                model: "gpt-4o".to_string(),
                system_fingerprint: None,
                usage: None,
                choices: vec![
                    crate::models::response::ChunkChoice {
                        index: 0,
                        delta: crate::models::common::ChatMessage {
                            role: crate::models::common::MessageRole::Assistant,
                            content: String::new(),
                            name: None,
                            function_call: None,
                            tool_calls: Some(vec![
                                serde_json::json!({
                                    "index": 0,
                                    "function": {
                                        "arguments": "\"San Francisco\"}"
                                    }
                                })
                            ]),
                            tool_call_id: None,
                        },
                        logprobs: None,
                        finish_reason: None,
                    }
                ],
            },
        ];

        let result = parser.accumulate_tool_calls(chunks).unwrap();
        assert_eq!(result.len(), 1);
        
        let tool_call = &result[0];
        assert_eq!(tool_call["id"], "call_123");
        assert_eq!(tool_call["function"]["name"], "get_weather");
        assert_eq!(tool_call["function"]["arguments"], "{\"location\":\"San Francisco\"}");
    }
}