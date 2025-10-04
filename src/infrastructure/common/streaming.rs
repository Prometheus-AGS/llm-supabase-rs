// src/infrastructure/common/streaming.rs
//
// Common streaming utilities and traits for all providers

use anyhow::Result;
use async_stream::stream;
use futures_util::Stream;
use serde_json::Value;
use tokio_stream::StreamExt;
use tracing::{debug, error, warn};

use crate::models::response::ChatCompletionChunk;
use super::converter::ProviderConverter;

/// Generic streaming processor that can handle any provider's streaming format
pub struct GenericStreamProcessor<T: ProviderConverter> {
    #[allow(dead_code)] // Will be used for provider-specific processing in future
    converter: T,
    request_id: String,
    model: String,
}

impl<T: ProviderConverter> GenericStreamProcessor<T> {
    pub fn new(converter: T, request_id: String, model: String) -> Self {
        Self {
            converter,
            request_id,
            model,
        }
    }

    /// Process a stream of raw bytes into OpenAI-compatible chunks
    pub fn process_byte_stream<S>(
        &self,
        byte_stream: S,
    ) -> impl Stream<Item = Result<ChatCompletionChunk>> + '_
    where
        S: Stream<Item = Result<bytes::Bytes, reqwest::Error>> + Send + 'static,
    {
        stream! {
            let mut buffer = String::new();
            
            tokio::pin!(byte_stream);
            
            while let Some(chunk_result) = byte_stream.next().await {
                match chunk_result {
                    Ok(bytes) => {
                        let chunk_str = String::from_utf8_lossy(&bytes);
                        buffer.push_str(&chunk_str);
                        
                        // Process complete lines
                        while let Some(line_end) = buffer.find('\n') {
                            let line = buffer[..line_end].trim().to_string();
                            buffer.drain(..line_end + 1);
                            
                            if let Some(openai_chunk) = self.process_line(&line).await {
                                match openai_chunk {
                                    Ok(chunk) => yield Ok(chunk),
                                    Err(e) => {
                                        error!("Failed to process streaming line: {}", e);
                                        yield Err(e);
                                        break;
                                    }
                                }
                            }
                        }
                    }
                    Err(e) => {
                        error!("Streaming error: {}", e);
                        yield Err(anyhow::anyhow!("Streaming error: {}", e));
                        break;
                    }
                }
            }
        }
    }

    /// Process a single line from the stream
    async fn process_line(&self, line: &str) -> Option<Result<ChatCompletionChunk>> {
        if line.is_empty() {
            return None;
        }

        // Handle Server-Sent Events format
        let json_str = if line.starts_with("data: ") {
            let data = line.strip_prefix("data: ").unwrap_or(line);
            if data.trim() == "[DONE]" {
                debug!("Received [DONE] marker");
                return None;
            }
            data
        } else if line.starts_with("{") {
            // Direct JSON
            line
        } else {
            // Skip non-JSON lines
            debug!("Skipping non-JSON line: {}", line);
            return None;
        };

        // Parse as generic JSON first
        let json_value = match serde_json::from_str::<Value>(json_str) {
            Ok(value) => value,
            Err(e) => {
                warn!("Failed to parse JSON: {} - Line: {}", e, json_str);
                return Some(Err(anyhow::anyhow!("Invalid JSON: {}", e)));
            }
        };

        // Try to convert to provider-specific chunk and then to OpenAI format
        match self.convert_json_to_openai_chunk(&json_value).await {
            Ok(Some(chunk)) => Some(Ok(chunk)),
            Ok(None) => None, // Skip this chunk
            Err(e) => Some(Err(e)),
        }
    }

    /// Convert generic JSON to OpenAI chunk using provider-specific logic
    async fn convert_json_to_openai_chunk(&self, json: &Value) -> Result<Option<ChatCompletionChunk>> {
        // This is where provider-specific logic would go
        // For now, we'll implement a generic approach that works with most providers
        
        let event_type = json.get("type")
            .or_else(|| json.get("event"))
            .and_then(|v| v.as_str())
            .unwrap_or("unknown");

        debug!("Processing chunk with event_type: {}", event_type);

        // Extract text content using common patterns
        let text_content = extract_text_from_json(json);

        if let Some(content) = text_content {
            debug!("Found content: {}", content);
            
            // Create OpenAI-compatible chunk
            use crate::models::{
                common::{ChatMessage, MessageRole},
                response::{ChatCompletionChunk, ChatCompletionChunkChoice},
            };

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
                self.request_id.clone(),
                self.model.clone(),
                vec![choice],
            )))
        } else {
            // Check if this is a final chunk with usage info
            if let Some(usage_obj) = json.get("usage") {
                debug!("Found usage info, creating final chunk");
                
                use crate::models::{
                    common::{ChatMessage, Usage, FinishReason},
                    response::{ChatCompletionChunk, ChatCompletionChunkChoice},
                };

                let usage = Usage {
                    prompt_tokens: usage_obj.get("input_tokens")
                        .or_else(|| usage_obj.get("prompt_tokens"))
                        .and_then(|v| v.as_u64())
                        .map(|v| v as u32)
                        .unwrap_or(0),
                    completion_tokens: usage_obj.get("output_tokens")
                        .or_else(|| usage_obj.get("completion_tokens"))
                        .and_then(|v| v.as_u64())
                        .map(|v| v as u32)
                        .unwrap_or(0),
                    total_tokens: usage_obj.get("total_tokens")
                        .and_then(|v| v.as_u64())
                        .map(|v| v as u32)
                        .unwrap_or(0),
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
                    self.request_id.clone(),
                    self.model.clone(),
                    vec![choice],
                    usage,
                )))
            } else {
                debug!("No content or usage found, skipping chunk");
                Ok(None)
            }
        }
    }
}

/// Extract text content from JSON using common patterns across providers
fn extract_text_from_json(json: &Value) -> Option<String> {
    // Try various common locations where text content might be stored
    json.get("text")
        .or_else(|| json.get("content"))
        .or_else(|| json.get("delta").and_then(|d| d.get("text")))
        .or_else(|| json.get("delta").and_then(|d| d.get("content")))
        .or_else(|| json.get("message").and_then(|m| m.get("content")))
        .or_else(|| json.get("choices")
            .and_then(|c| c.as_array())
            .and_then(|arr| arr.first())
            .and_then(|first| first.get("text")))
        .or_else(|| json.get("content_block").and_then(|cb| cb.get("text")))
        .and_then(|t| t.as_str())
        .map(|s| s.to_string())
        .filter(|s| !s.is_empty())
}

/// Utility for creating Server-Sent Events format
pub fn format_as_sse(data: &str) -> String {
    format!("data: {}\n\n", data)
}

/// Utility for creating the final [DONE] SSE message
pub fn create_done_message() -> String {
    "data: [DONE]\n\n".to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_extract_text_from_json() {
        // Test direct text field
        let json1 = json!({"text": "hello world"});
        assert_eq!(extract_text_from_json(&json1), Some("hello world".to_string()));

        // Test delta.text pattern (Anthropic)
        let json2 = json!({"delta": {"text": "streaming text"}});
        assert_eq!(extract_text_from_json(&json2), Some("streaming text".to_string()));

        // Test content field
        let json3 = json!({"content": "content text"});
        assert_eq!(extract_text_from_json(&json3), Some("content text".to_string()));

        // Test nested message.content
        let json4 = json!({"message": {"content": "nested content"}});
        assert_eq!(extract_text_from_json(&json4), Some("nested content".to_string()));

        // Test empty/missing content
        let json5 = json!({"type": "message_start"});
        assert_eq!(extract_text_from_json(&json5), None);

        // Test empty string (should be filtered out)
        let json6 = json!({"text": ""});
        assert_eq!(extract_text_from_json(&json6), None);
    }

    #[test]
    fn test_sse_formatting() {
        assert_eq!(format_as_sse("hello"), "data: hello\n\n");
        assert_eq!(create_done_message(), "data: [DONE]\n\n");
    }
}
