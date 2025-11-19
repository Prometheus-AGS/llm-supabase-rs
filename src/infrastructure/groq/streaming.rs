//! Groq Server-Sent Events (SSE) streaming implementation
//!
//! This module handles Groq's streaming chat completions using OpenAI-compatible
//! SSE format. Optimized for Groq's ultra-fast inference speeds and includes
//! Groq-specific error handling and rate limiting considerations.

use anyhow::{Context, Result};
use futures_util::{Stream, StreamExt};
use reqwest::Response;
use serde_json::Value;
use std::pin::Pin;
use std::time::{Duration, Instant};
use tokio::io::AsyncBufReadExt;
use tokio_stream::wrappers::LinesStream;
use tokio_util::io::StreamReader;
use tracing::{debug, trace, warn, error, info};

use crate::models::response::ChatCompletionChunk;
use super::auth::RateLimitInfo;
use super::types::GroqModel;

/// Groq Server-Sent Events (SSE) stream parser
/// Handles the Groq streaming format which is OpenAI-compatible but optimized for speed
pub struct GroqStreamParser {
    /// Track streaming performance metrics
    start_time: Option<Instant>,
    
    /// Count chunks processed for performance metrics
    chunks_processed: u32,
    
    /// Model being used for streaming (affects parsing strategy)
    model: Option<String>,
}

impl GroqStreamParser {
    pub fn new() -> Self {
        Self {
            start_time: None,
            chunks_processed: 0,
            model: None,
        }
    }
    
    /// Create parser with model-specific optimizations
    pub fn with_model(model: String) -> Self {
        Self {
            start_time: None,
            chunks_processed: 0,
            model: Some(model),
        }
    }

    /// Parse a Groq streaming response into ChatCompletionChunk items
    /// Optimized for Groq's high-speed streaming
    pub async fn parse_stream<S, E>(
        &mut self,
        byte_stream: S,
    ) -> Result<impl Stream<Item = Result<ChatCompletionChunk>>>
    where
        S: Stream<Item = std::result::Result<bytes::Bytes, E>> + Send + 'static,
        E: Into<Box<dyn std::error::Error + Send + Sync>> + Send + 'static,
    {
        debug!("Parsing Groq Server-Sent Events stream with model: {:?}", self.model);
        
        // Start performance tracking
        self.start_time = Some(Instant::now());
        self.chunks_processed = 0;

        // Convert bytes stream to lines stream with optimized buffer size for Groq's speed
        let reader = StreamReader::new(byte_stream.map(|result| {
            result.map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))
        }));

        let lines_stream = LinesStream::new(reader.lines());

        // Process SSE lines and convert to chunks with performance optimization
        let chunk_stream = lines_stream.filter_map(move |line_result| async move {
            match line_result {
                Ok(line) => {
                    self.chunks_processed += 1;
                    self.process_sse_line(&line).await
                },
                Err(e) => {
                    error!("Error reading line from Groq stream: {}", e);
                    Some(Err(anyhow::anyhow!("Groq stream read error: {}", e)))
                }
            }
        });

        Ok(chunk_stream)
    }

    /// Process a single Server-Sent Events line with Groq optimizations
    async fn process_sse_line(&self, line: &str) -> Option<Result<ChatCompletionChunk>> {
        let line = line.trim();

        // Skip empty lines (common in SSE)
        if line.is_empty() {
            return None;
        }

        // Handle SSE data lines
        if let Some(data) = line.strip_prefix("data: ") {
            return self.process_data_line(data).await;
        }

        // Handle SSE event type (Groq may send custom events)
        if let Some(event) = line.strip_prefix("event: ") {
            return self.process_event_line(event).await;
        }

        // Handle SSE id field (useful for Groq request tracking)
        if let Some(id) = line.strip_prefix("id: ") {
            trace!("Groq stream ID: {}", id);
            return None;
        }

        // Handle SSE retry field (Groq rate limiting information)
        if let Some(retry) = line.strip_prefix("retry: ") {
            if let Ok(retry_ms) = retry.parse::<u64>() {
                debug!("Groq suggested retry delay: {}ms", retry_ms);
            }
            return None;
        }

        // Skip other SSE fields
        if line.contains(':') {
            trace!("Skipping SSE field line: {}", line);
            return None;
        }

        // Log unexpected content (might indicate Groq API changes)
        warn!("Unexpected line in Groq SSE stream: {}", line);
        None
    }

    /// Process SSE event types (Groq-specific events)
    async fn process_event_line(&self, event: &str) -> Option<Result<ChatCompletionChunk>> {
        match event.trim() {
            "completion" => {
                trace!("Groq completion event received");
                None
            },
            "error" => {
                warn!("Groq error event received");
                None
            },
            "rate_limit" => {
                debug!("Groq rate limit event received");
                None
            },
            _ => {
                trace!("Unknown Groq event type: {}", event);
                None
            }
        }
    }

    /// Process a data line from the SSE stream with Groq-specific handling
    async fn process_data_line(&self, data: &str) -> Option<Result<ChatCompletionChunk>> {
        let data = data.trim();

        // Handle the [DONE] marker (standard SSE completion)
        if data == "[DONE]" {
            debug!("Received [DONE] marker, Groq stream completed");
            
            // Log performance metrics for Groq streams
            if let Some(start_time) = self.start_time {
                let duration = start_time.elapsed();
                let chunks_per_sec = self.chunks_processed as f64 / duration.as_secs_f64();
                info!("Groq stream completed: {} chunks in {:?} ({:.2} chunks/sec)", 
                      self.chunks_processed, duration, chunks_per_sec);
            }
            
            return None; // End of stream
        }

        // Handle Groq error responses in data field
        if data.starts_with("{\"error\"") {
            match serde_json::from_str::<Value>(data) {
                Ok(error_json) => {
                    let error_msg = error_json.get("error")
                        .and_then(|e| e.get("message"))
                        .and_then(|m| m.as_str())
                        .unwrap_or("Unknown Groq error");
                    
                    error!("Groq API error in stream: {}", error_msg);
                    return Some(Err(anyhow::anyhow!("Groq API error: {}", error_msg)));
                },
                Err(e) => {
                    error!("Failed to parse Groq error response: {}", e);
                    return Some(Err(anyhow::anyhow!("Groq error parse failure: {}", e)));
                }
            }
        }

        // Parse JSON data (standard chunk format)
        match serde_json::from_str::<Value>(data) {
            Ok(json_value) => {
                trace!("Parsed Groq JSON chunk: {}", json_value);
                
                // Convert to ChatCompletionChunk with Groq-specific handling
                match self.convert_to_chunk(json_value).await {
                    Ok(chunk) => Some(Ok(chunk)),
                    Err(e) => {
                        error!("Failed to convert Groq JSON to chunk: {}", e);
                        Some(Err(e))
                    }
                }
            }
            Err(e) => {
                error!("Failed to parse Groq JSON data: {} - Data: {}", e, data);
                Some(Err(anyhow::anyhow!("Groq JSON parse error: {}", e)))
            }
        }
    }

    /// Convert JSON value to ChatCompletionChunk with Groq-specific validation
    async fn convert_to_chunk(&self, json: Value) -> Result<ChatCompletionChunk> {
        // Groq streaming response uses OpenAI-compatible structure
        let mut chunk: ChatCompletionChunk = serde_json::from_value(json)
            .context("Failed to deserialize Groq streaming chunk")?;

        // Validate and enhance chunk for Groq
        self.validate_groq_chunk(&mut chunk)?;

        trace!("Converted Groq chunk with {} choices", chunk.choices.len());
        Ok(chunk)
    }

    /// Validate and enhance chunk with Groq-specific logic
    fn validate_groq_chunk(&self, chunk: &mut ChatCompletionChunk) -> Result<()> {
        // Validate model matches expected model
        if let Some(ref expected_model) = self.model {
            if chunk.model != *expected_model {
                warn!("Model mismatch in Groq chunk: expected '{}', got '{}'", 
                      expected_model, chunk.model);
            }
        }

        // Validate Groq-specific object type
        if chunk.object != "chat.completion.chunk" {
            warn!("Unexpected object type in Groq chunk: {}", chunk.object);
        }

        // Check for tool calls in ultra-fast models (might need special handling)
        if let Some(ref model) = self.model {
            if self.is_ultra_fast_model(model) {
                for choice in &chunk.choices {
                    if choice.delta.tool_calls.is_some() {
                        debug!("Tool calls detected in ultra-fast Groq model: {}", model);
                    }
                }
            }
        }

        Ok(())
    }

    /// Check if model is in ultra-fast tier (affects processing)
    fn is_ultra_fast_model(&self, model: &str) -> bool {
        matches!(model, "llama-3.1-8b-instant" | "llama3-8b-8192")
    }

    /// Get streaming performance metrics
    pub fn get_performance_metrics(&self) -> Option<StreamingMetrics> {
        self.start_time.map(|start| {
            let duration = start.elapsed();
            StreamingMetrics {
                duration,
                chunks_processed: self.chunks_processed,
                chunks_per_second: self.chunks_processed as f64 / duration.as_secs_f64(),
                model: self.model.clone(),
            }
        })
    }

    /// Create a test stream for development/testing with Groq data
    pub fn create_test_stream(model: &str) -> impl Stream<Item = Result<ChatCompletionChunk>> {
        use tokio_stream::iter;

        let test_chunks = vec![
            // First chunk with role
            Ok(ChatCompletionChunk {
                id: "chatcmpl-groq123".to_string(),
                object: "chat.completion.chunk".to_string(),
                created: 1699999999,
                model: model.to_string(),
                choices: vec![
                    crate::models::response::ChunkChoice {
                        index: 0,
                        delta: crate::models::response::ChunkDelta {
                            role: Some("assistant".to_string()),
                            content: None,
                            tool_calls: None,
                        },
                        finish_reason: None,
                    }
                ],
            }),
            // Content chunks (Groq sends these very fast)
            Ok(ChatCompletionChunk {
                id: "chatcmpl-groq123".to_string(),
                object: "chat.completion.chunk".to_string(),
                created: 1699999999,
                model: model.to_string(),
                choices: vec![
                    crate::models::response::ChunkChoice {
                        index: 0,
                        delta: crate::models::response::ChunkDelta {
                            role: None,
                            content: Some("Hello from Groq! ".to_string()),
                            tool_calls: None,
                        },
                        finish_reason: None,
                    }
                ],
            }),
            // More content
            Ok(ChatCompletionChunk {
                id: "chatcmpl-groq123".to_string(),
                object: "chat.completion.chunk".to_string(),
                created: 1699999999,
                model: model.to_string(),
                choices: vec![
                    crate::models::response::ChunkChoice {
                        index: 0,
                        delta: crate::models::response::ChunkDelta {
                            role: None,
                            content: Some("This is ultra-fast inference!".to_string()),
                            tool_calls: None,
                        },
                        finish_reason: None,
                    }
                ],
            }),
            // Final chunk with finish reason
            Ok(ChatCompletionChunk {
                id: "chatcmpl-groq123".to_string(),
                object: "chat.completion.chunk".to_string(),
                created: 1699999999,
                model: model.to_string(),
                choices: vec![
                    crate::models::response::ChunkChoice {
                        index: 0,
                        delta: crate::models::response::ChunkDelta {
                            role: None,
                            content: None,
                            tool_calls: None,
                        },
                        finish_reason: Some("stop".to_string()),
                    }
                ],
            }),
        ];

        iter(test_chunks)
    }

    /// Create a tool calling test stream
    pub fn create_tool_test_stream(model: &str) -> impl Stream<Item = Result<ChatCompletionChunk>> {
        use tokio_stream::iter;

        let test_chunks = vec![
            // Tool call chunk
            Ok(ChatCompletionChunk {
                id: "chatcmpl-groq-tool123".to_string(),
                object: "chat.completion.chunk".to_string(),
                created: 1699999999,
                model: model.to_string(),
                choices: vec![
                    crate::models::response::ChunkChoice {
                        index: 0,
                        delta: crate::models::response::ChunkDelta {
                            role: Some("assistant".to_string()),
                            content: None,
                            tool_calls: Some(vec![
                                crate::models::response::ChunkToolCall {
                                    index: 0,
                                    id: Some("call_groq123".to_string()),
                                    tool_type: Some("function".to_string()),
                                    function: Some(crate::models::response::ChunkFunction {
                                        name: Some("get_weather".to_string()),
                                        arguments: Some("{\"location\": \"San Francisco\"}".to_string()),
                                    }),
                                }
                            ]),
                        },
                        finish_reason: Some("tool_calls".to_string()),
                    }
                ],
            }),
        ];

        iter(test_chunks)
    }
}

impl Default for GroqStreamParser {
    fn default() -> Self {
        Self::new()
    }
}

/// Streaming performance metrics for Groq
#[derive(Debug, Clone)]
pub struct StreamingMetrics {
    pub duration: Duration,
    pub chunks_processed: u32,
    pub chunks_per_second: f64,
    pub model: Option<String>,
}

impl StreamingMetrics {
    /// Check if streaming performance meets Groq's expected speeds
    pub fn is_performing_well(&self) -> bool {
        // Groq should be very fast - expect at least 10 chunks per second
        self.chunks_per_second >= 10.0
    }

    /// Get performance tier based on Groq model expectations
    pub fn get_performance_tier(&self) -> PerformanceTier {
        if let Some(ref model) = self.model {
            if model.contains("8b-instant") && self.chunks_per_second >= 50.0 {
                return PerformanceTier::UltraFast;
            }
            if self.chunks_per_second >= 20.0 {
                return PerformanceTier::VeryFast;
            }
            if self.chunks_per_second >= 10.0 {
                return PerformanceTier::Fast;
            }
        }
        PerformanceTier::Slow
    }
}

/// Performance tier classification for Groq streaming
#[derive(Debug, Clone, PartialEq)]
pub enum PerformanceTier {
    UltraFast,  // 50+ chunks/sec (8B instant models)
    VeryFast,   // 20+ chunks/sec (most Groq models)
    Fast,       // 10+ chunks/sec (large models)
    Slow,       // < 10 chunks/sec (unexpected for Groq)
}

/// Groq streaming utilities
pub struct GroqStreamUtils;

impl GroqStreamUtils {
    /// Parse Groq response stream from reqwest Response
    pub async fn parse_response_stream(
        response: Response, 
        model: String
    ) -> Result<impl Stream<Item = Result<ChatCompletionChunk>>> {
        let mut parser = GroqStreamParser::with_model(model);
        
        // Extract bytes stream from response
        let byte_stream = response.bytes_stream();
        
        parser.parse_stream(byte_stream).await
    }

    /// Extract rate limiting info from Groq response headers
    pub fn extract_rate_limit_from_headers(
        headers: &reqwest::header::HeaderMap
    ) -> Option<RateLimitInfo> {
        let remaining = headers.get("x-ratelimit-remaining")?
            .to_str().ok()?
            .parse().ok()?;
        
        let limit = headers.get("x-ratelimit-limit")?
            .to_str().ok()?
            .parse().ok()?;
        
        let reset = headers.get("x-ratelimit-reset")?
            .to_str().ok()?
            .parse().ok()?;
        
        Some(RateLimitInfo {
            remaining,
            limit,
            reset_timestamp: reset,
        })
    }

    /// Check if we should use streaming for a given Groq model
    pub fn should_use_streaming(model: &str, request_size_estimate: usize) -> bool {
        // Groq is so fast that streaming is almost always beneficial
        let model_info = GroqModel::get_model(model);
        
        match model_info {
            Some(info) if info.supports_streaming => {
                // Use streaming for any request > 50 tokens estimated
                request_size_estimate > 200 // ~50 tokens * 4 chars per token
            },
            _ => false,
        }
    }

    /// Get optimal chunk buffer size for Groq model
    pub fn get_optimal_buffer_size(model: &str) -> usize {
        if model.contains("8b-instant") {
            // Ultra-fast models can handle larger buffers
            8192
        } else if model.contains("70b") {
            // Large models might need smaller buffers
            4096
        } else {
            // Default buffer size
            6144
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use futures_util::StreamExt;

    #[tokio::test]
    async fn test_parser_creation() {
        let parser = GroqStreamParser::new();
        assert!(parser.model.is_none());
        assert!(parser.start_time.is_none());
        assert_eq!(parser.chunks_processed, 0);
    }

    #[tokio::test]
    async fn test_parser_with_model() {
        let model = "llama-3.1-70b-versatile".to_string();
        let parser = GroqStreamParser::with_model(model.clone());
        assert_eq!(parser.model, Some(model));
    }

    #[tokio::test]
    async fn test_ultra_fast_model_detection() {
        let parser = GroqStreamParser::new();
        assert!(parser.is_ultra_fast_model("llama-3.1-8b-instant"));
        assert!(parser.is_ultra_fast_model("llama3-8b-8192"));
        assert!(!parser.is_ultra_fast_model("llama-3.1-70b-versatile"));
    }

    #[tokio::test]
    async fn test_create_test_stream() {
        let model = "llama-3.1-70b-versatile";
        let mut stream = GroqStreamParser::create_test_stream(model);
        
        let mut chunk_count = 0;
        while let Some(result) = stream.next().await {
            assert!(result.is_ok());
            let chunk = result.unwrap();
            assert_eq!(chunk.model, model);
            chunk_count += 1;
        }
        
        assert!(chunk_count > 0);
    }

    #[tokio::test]
    async fn test_tool_test_stream() {
        let model = "llama-3.1-70b-versatile";
        let mut stream = GroqStreamParser::create_tool_test_stream(model);
        
        if let Some(result) = stream.next().await {
            assert!(result.is_ok());
            let chunk = result.unwrap();
            assert_eq!(chunk.model, model);
            assert!(!chunk.choices.is_empty());
            assert!(chunk.choices[0].delta.tool_calls.is_some());
        }
    }

    #[test]
    fn test_streaming_utils() {
        // Test streaming decision
        assert!(GroqStreamUtils::should_use_streaming("llama-3.1-70b-versatile", 1000));
        assert!(!GroqStreamUtils::should_use_streaming("llama-3.1-70b-versatile", 100));
        
        // Test buffer sizes
        assert_eq!(GroqStreamUtils::get_optimal_buffer_size("llama-3.1-8b-instant"), 8192);
        assert_eq!(GroqStreamUtils::get_optimal_buffer_size("llama-3.1-70b-versatile"), 4096);
    }

    #[test]
    fn test_performance_metrics() {
        let metrics = StreamingMetrics {
            duration: Duration::from_secs(2),
            chunks_processed: 100,
            chunks_per_second: 50.0,
            model: Some("llama-3.1-8b-instant".to_string()),
        };
        
        assert!(metrics.is_performing_well());
        assert_eq!(metrics.get_performance_tier(), PerformanceTier::UltraFast);
    }
}