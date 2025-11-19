//! Azure OpenAI streaming module
//!
//! This module handles Azure OpenAI streaming responses using Server-Sent Events (SSE).
//! Since Azure OpenAI uses the identical streaming format as OpenAI, this implementation
//! is essentially the same as the OpenAI streaming parser.

use anyhow::{Context, Result};
use futures_util::{Stream, StreamExt};
use reqwest::Response;
use serde_json::Value;
use std::pin::Pin;
use tokio_stream::wrappers::LinesStream;
use tokio_util::io::StreamReader;
use tracing::{debug, trace, warn, error};

use crate::models::response::{ChatCompletionChunk, ChatCompletionResponse};

/// Azure OpenAI Server-Sent Events (SSE) stream parser
/// Handles the Azure OpenAI streaming format: data: {json}\n\ndata: [DONE]\n\n
/// (Identical to OpenAI format)
pub struct AzureOpenAIStreamParser;

impl AzureOpenAIStreamParser {
    pub fn new() -> Self {
        Self
    }

    /// Parse an Azure OpenAI streaming response into ChatCompletionChunk items
    pub async fn parse_stream<S, E>(
        &self,
        byte_stream: S,
    ) -> Result<impl Stream<Item = Result<ChatCompletionChunk>>>
    where
        S: Stream<Item = std::result::Result<bytes::Bytes, E>> + Send + 'static,
        E: Into<Box<dyn std::error::Error + Send + Sync>> + Send + 'static,
    {
        debug!("Parsing Azure OpenAI Server-Sent Events stream");

        // Convert bytes stream to lines stream
        let reader = StreamReader::new(byte_stream.map(|result| {
            result.map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))
        }));

        let lines_stream = LinesStream::new(reader.lines());

        // Process SSE lines and convert to chunks
        let chunk_stream = lines_stream.filter_map(|line_result| async move {
            match line_result {
                Ok(line) => self.process_sse_line(&line).await,
                Err(e) => {
                    error!("Error reading line from Azure OpenAI stream: {}", e);
                    Some(Err(anyhow::anyhow!("Stream read error: {}", e)))
                }
            }
        });

        Ok(chunk_stream)
    }

    /// Process a single Server-Sent Events line
    async fn process_sse_line(&self, line: &str) -> Option<Result<ChatCompletionChunk>> {
        let line = line.trim();

        // Skip empty lines
        if line.is_empty() {
            return None;
        }

        // Handle SSE data lines
        if let Some(data) = line.strip_prefix("data: ") {
            return self.process_data_line(data).await;
        }

        // Skip other SSE fields (event:, id:, retry:, etc.)
        if line.contains(':') {
            trace!("Skipping Azure OpenAI SSE field line: {}", line);
            return None;
        }

        // Log unexpected content
        warn!("Unexpected line in Azure OpenAI SSE stream: {}", line);
        None
    }

    /// Process a data line from the SSE stream
    async fn process_data_line(&self, data: &str) -> Option<Result<ChatCompletionChunk>> {
        let data = data.trim();

        // Handle the [DONE] marker
        if data == "[DONE]" {
            debug!("Received [DONE] marker from Azure OpenAI, stream completed");
            return None; // End of stream
        }

        // Parse JSON data
        match serde_json::from_str::<Value>(data) {
            Ok(json_value) => {
                trace!("Parsed Azure OpenAI JSON chunk: {}", json_value);
                
                // Convert to ChatCompletionChunk
                match self.convert_to_chunk(json_value).await {
                    Ok(chunk) => Some(Ok(chunk)),
                    Err(e) => {
                        error!("Failed to convert Azure OpenAI JSON to chunk: {}", e);
                        Some(Err(e))
                    }
                }
            }
            Err(e) => {
                error!("Failed to parse Azure OpenAI JSON data: {} - Data: {}", e, data);
                Some(Err(anyhow::anyhow!("JSON parse error: {}", e)))
            }
        }
    }

    /// Convert JSON value to ChatCompletionChunk
    async fn convert_to_chunk(&self, json: Value) -> Result<ChatCompletionChunk> {
        // Azure OpenAI streaming response structure (identical to OpenAI)
        let chunk: ChatCompletionChunk = serde_json::from_value(json)
            .context("Failed to deserialize Azure OpenAI streaming chunk")?;

        trace!("Converted Azure OpenAI chunk with {} choices", chunk.choices.len());
        Ok(chunk)
    }

    /// Check if a chunk contains tool calls
    pub fn chunk_has_tool_calls(&self, chunk: &ChatCompletionChunk) -> bool {
        chunk.choices.iter().any(|choice| {
            choice.delta.tool_calls.as_ref()
                .map(|calls| !calls.is_empty())
                .unwrap_or(false)
        })
    }

    /// Check if a chunk indicates the stream is complete
    pub fn is_stream_complete(&self, chunk: &ChatCompletionChunk) -> bool {
        chunk.choices.iter().any(|choice| {
            choice.finish_reason.is_some()
        })
    }

    /// Extract finish reason from chunk
    pub fn get_finish_reason(&self, chunk: &ChatCompletionChunk) -> Option<String> {
        chunk.choices.first()
            .and_then(|choice| choice.finish_reason.as_ref())
            .map(|reason| {
                match reason {
                    crate::models::common::FinishReason::Stop => "stop".to_string(),
                    crate::models::common::FinishReason::Length => "length".to_string(),
                    crate::models::common::FinishReason::ToolCalls => "tool_calls".to_string(),
                    crate::models::common::FinishReason::ContentFilter => "content_filter".to_string(),
                    crate::models::common::FinishReason::FunctionCall => "function_call".to_string(),
                }
            })
    }

    /// Check if chunk indicates tool calls are needed
    pub fn chunk_needs_tool_calls(&self, chunk: &ChatCompletionChunk) -> bool {
        self.get_finish_reason(chunk)
            .map(|reason| reason == "tool_calls")
            .unwrap_or(false)
    }

    /// Create a test stream for development/testing
    pub fn create_test_stream() -> impl Stream<Item = Result<ChatCompletionChunk>> {
        use tokio_stream::iter;

        let test_chunks = vec![
            // First chunk with role
            Ok(ChatCompletionChunk {
                id: "chatcmpl-azure-test123".to_string(),
                object: "chat.completion.chunk".to_string(),
                created: 1699999999,
                model: "gpt-4o".to_string(),
                system_fingerprint: None,
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
                usage: None,
            }),
            // Content chunks
            Ok(ChatCompletionChunk {
                id: "chatcmpl-azure-test123".to_string(),
                object: "chat.completion.chunk".to_string(),
                created: 1699999999,
                model: "gpt-4o".to_string(),
                system_fingerprint: None,
                choices: vec![
                    crate::models::response::ChunkChoice {
                        index: 0,
                        delta: crate::models::common::ChatMessage {
                            role: crate::models::common::MessageRole::Assistant,
                            content: "Hello from Azure OpenAI!".to_string(),
                            name: None,
                            function_call: None,
                            tool_calls: None,
                            tool_call_id: None,
                        },
                        logprobs: None,
                        finish_reason: None,
                    }
                ],
                usage: None,
            }),
            // Final chunk
            Ok(ChatCompletionChunk {
                id: "chatcmpl-azure-test123".to_string(),
                object: "chat.completion.chunk".to_string(),
                created: 1699999999,
                model: "gpt-4o".to_string(),
                system_fingerprint: None,
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
                usage: None,
            }),
        ];

        iter(test_chunks)
    }

    /// Create a test stream with tool calls for testing
    pub fn create_test_tool_call_stream() -> impl Stream<Item = Result<ChatCompletionChunk>> {
        use tokio_stream::iter;

        let test_chunks = vec![
            // First chunk with role
            Ok(ChatCompletionChunk {
                id: "chatcmpl-azure-tool-test123".to_string(),
                object: "chat.completion.chunk".to_string(),
                created: 1699999999,
                model: "gpt-4o".to_string(),
                system_fingerprint: None,
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
                usage: None,
            }),
            // Tool call chunk
            Ok(ChatCompletionChunk {
                id: "chatcmpl-azure-tool-test123".to_string(),
                object: "chat.completion.chunk".to_string(),
                created: 1699999999,
                model: "gpt-4o".to_string(),
                system_fingerprint: None,
                choices: vec![
                    crate::models::response::ChunkChoice {
                        index: 0,
                        delta: crate::models::common::ChatMessage {
                            role: crate::models::common::MessageRole::Assistant,
                            content: String::new(),
                            name: None,
                            function_call: None,
                            tool_calls: Some(vec![
                                crate::shared::types::ToolCall {
                                    id: "call_test123".to_string(),
                                    tool_type: "function".to_string(),
                                    function: crate::shared::types::FunctionCall {
                                        name: "get_weather".to_string(),
                                        arguments: r#"{"location": "San Francisco"}"#.to_string(),
                                    },
                                }
                            ]),
                            tool_call_id: None,
                        },
                        logprobs: None,
                        finish_reason: Some(crate::models::common::FinishReason::ToolCalls),
                    }
                ],
                usage: None,
            }),
        ];

        iter(test_chunks)
    }
}

impl Default for AzureOpenAIStreamParser {
    fn default() -> Self {
        Self::new()
    }
}

/// Azure OpenAI streaming utilities
pub struct AzureOpenAIStreamUtils;

impl AzureOpenAIStreamUtils {
    /// Convert a reqwest Response to a streaming bytes stream
    pub fn response_to_byte_stream(
        response: Response,
    ) -> impl Stream<Item = Result<bytes::Bytes>> {
        response.bytes_stream().map(|result| {
            result.map_err(|e| anyhow::anyhow!("Stream error: {}", e))
        })
    }

    /// Collect streaming chunks into a complete response
    pub async fn collect_chunks<S>(
        mut stream: S,
    ) -> Result<ChatCompletionResponse>
    where
        S: Stream<Item = Result<ChatCompletionChunk>> + Unpin,
    {
        use crate::models::response::{ChatCompletionResponse, ChatCompletionChoice, ChatCompletionMessage};
        use crate::models::common::MessageRole;

        debug!("Collecting Azure OpenAI streaming chunks into complete response");

        let mut content_parts = Vec::new();
        let mut tool_calls = Vec::new();
        let mut finish_reason = None;
        let mut response_id = None;
        let mut model = None;
        let mut created = None;

        while let Some(chunk_result) = stream.next().await {
            match chunk_result {
                Ok(chunk) => {
                    // Store metadata from first chunk
                    if response_id.is_none() {
                        response_id = Some(chunk.id.clone());
                        model = Some(chunk.model.clone());
                        created = Some(chunk.created);
                    }

                    // Process choices
                    for choice in &chunk.choices {
                        // Collect content
                        if !choice.delta.content.is_empty() {
                            content_parts.push(choice.delta.content.clone());
                        }

                        // Collect tool calls
                        if let Some(ref chunk_tool_calls) = choice.delta.tool_calls {
                            for chunk_tool_call in chunk_tool_calls {
                                tool_calls.push(chunk_tool_call.clone());
                            }
                        }

                        // Update finish reason
                        if let Some(ref reason) = choice.finish_reason {
                            finish_reason = Some(reason.clone());
                        }
                    }
                }
                Err(e) => {
                    error!("Error in Azure OpenAI streaming chunk: {}", e);
                    return Err(e);
                }
            }
        }

        // Build complete response
        let message = ChatCompletionMessage {
            role: MessageRole::Assistant,
            content: if content_parts.is_empty() {
                None
            } else {
                Some(content_parts.join(""))
            },
            name: None,
            function_call: None,
            tool_calls: if tool_calls.is_empty() {
                None
            } else {
                Some(tool_calls)
            },
            tool_call_id: None,
        };

        let choice = ChatCompletionChoice {
            index: 0,
            message,
            logprobs: None,
            finish_reason: finish_reason.unwrap_or(crate::models::common::FinishReason::Stop),
        };

        let response = ChatCompletionResponse {
            id: response_id.unwrap_or_else(|| "chatcmpl-azure-unknown".to_string()),
            object: "chat.completion".to_string(),
            created: created.unwrap_or(0),
            model: model.unwrap_or_else(|| "unknown".to_string()),
            system_fingerprint: None,
            choices: vec![choice],
            usage: crate::models::common::Usage {
                prompt_tokens: 0,
                completion_tokens: 0,
                total_tokens: 0,
                prompt_tokens_details: None,
                completion_tokens_details: None,
            },
        };

        debug!("Collected Azure OpenAI streaming response with {} content characters", 
               response.choices[0].message.content.as_ref().map(|c| c.len()).unwrap_or(0));

        Ok(response)
    }

    /// Validate streaming response format
    pub fn validate_stream_format(chunk: &ChatCompletionChunk) -> Result<()> {
        if chunk.object != "chat.completion.chunk" {
            return Err(anyhow::anyhow!(
                "Invalid chunk object type: expected 'chat.completion.chunk', got '{}'",
                chunk.object
            ));
        }

        if chunk.choices.is_empty() {
            return Err(anyhow::anyhow!("Chunk must have at least one choice"));
        }

        Ok(())
    }

    /// Extract all content from streaming chunks
    pub async fn extract_content_stream<S>(
        mut stream: S,
    ) -> Result<String>
    where
        S: Stream<Item = Result<ChatCompletionChunk>> + Unpin,
    {
        let mut content_parts = Vec::new();

        while let Some(chunk_result) = stream.next().await {
            match chunk_result {
                Ok(chunk) => {
                    for choice in &chunk.choices {
                        if !choice.delta.content.is_empty() {
                            content_parts.push(choice.delta.content.clone());
                        }
                    }
                }
                Err(e) => return Err(e),
            }
        }

        Ok(content_parts.join(""))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use futures_util::stream;
    use tokio_stream::StreamExt;

    #[tokio::test]
    async fn test_stream_parser_creation() {
        let parser = AzureOpenAIStreamParser::new();
        assert_eq!(std::mem::size_of_val(&parser), 0); // Zero-sized struct
    }

    #[tokio::test]
    async fn test_process_sse_line() {
        let parser = AzureOpenAIStreamParser::new();

        // Test empty line
        let result = parser.process_sse_line("").await;
        assert!(result.is_none());

        // Test [DONE] marker
        let result = parser.process_data_line("[DONE]").await;
        assert!(result.is_none());

        // Test SSE field line
        let result = parser.process_sse_line("event: completion").await;
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_chunk_utility_methods() {
        let parser = AzureOpenAIStreamParser::new();

        let chunk = ChatCompletionChunk {
            id: "test".to_string(),
            object: "chat.completion.chunk".to_string(),
            created: 0,
            model: "gpt-4o".to_string(),
            system_fingerprint: None,
            choices: vec![
                crate::models::response::ChunkChoice {
                    index: 0,
                    delta: crate::models::common::ChatMessage {
                        role: crate::models::common::MessageRole::Assistant,
                        content: "test".to_string(),
                        name: None,
                        function_call: None,
                        tool_calls: None,
                        tool_call_id: None,
                    },
                    logprobs: None,
                    finish_reason: Some(crate::models::common::FinishReason::Stop),
                }
            ],
            usage: None,
        };

        assert!(parser.is_stream_complete(&chunk));
        assert_eq!(parser.get_finish_reason(&chunk), Some("stop".to_string()));
        assert!(!parser.chunk_has_tool_calls(&chunk));
        assert!(!parser.chunk_needs_tool_calls(&chunk));
    }

    #[tokio::test]
    async fn test_collect_chunks() {
        let test_stream = AzureOpenAIStreamParser::create_test_stream();
        let response = AzureOpenAIStreamUtils::collect_chunks(test_stream).await;

        assert!(response.is_ok());
        let response = response.unwrap();
        assert_eq!(response.choices.len(), 1);
        assert!(response.choices[0].message.content.is_some());
    }

    #[tokio::test]
    async fn test_extract_content_stream() {
        let test_stream = AzureOpenAIStreamParser::create_test_stream();
        let content = AzureOpenAIStreamUtils::extract_content_stream(test_stream).await;

        assert!(content.is_ok());
        let content = content.unwrap();
        assert!(!content.is_empty());
    }

    #[tokio::test]
    async fn test_validate_stream_format() {
        let valid_chunk = ChatCompletionChunk {
            id: "test".to_string(),
            object: "chat.completion.chunk".to_string(),
            created: 0,
            model: "gpt-4o".to_string(),
            system_fingerprint: None,
            choices: vec![
                crate::models::response::ChunkChoice {
                    index: 0,
                    delta: crate::models::common::ChatMessage {
                        role: crate::models::common::MessageRole::Assistant,
                        content: "test".to_string(),
                        name: None,
                        function_call: None,
                        tool_calls: None,
                        tool_call_id: None,
                    },
                    logprobs: None,
                    finish_reason: None,
                }
            ],
            usage: None,
        };

        assert!(AzureOpenAIStreamUtils::validate_stream_format(&valid_chunk).is_ok());

        let invalid_chunk = ChatCompletionChunk {
            id: "test".to_string(),
            object: "invalid".to_string(),
            created: 0,
            model: "gpt-4o".to_string(),
            system_fingerprint: None,
            choices: vec![],
            usage: None,
        };

        assert!(AzureOpenAIStreamUtils::validate_stream_format(&invalid_chunk).is_err());
    }
}