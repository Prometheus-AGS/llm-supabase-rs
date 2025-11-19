//! Mistral AI streaming support with Server-Sent Events (SSE)
//!
//! This module handles real-time streaming responses from Mistral AI API using 
//! Server-Sent Events format. Since Mistral uses OpenAI-compatible streaming,
//! most of the parsing logic is similar but with Mistral-specific enhancements
//! for European compliance and performance optimization.

use anyhow::{Context, Result};
use futures_util::{Stream, StreamExt, TryStreamExt};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::pin::Pin;
use std::task::{Context as TaskContext, Poll};
use tokio::time::{Duration, Instant};
use tracing::{debug, trace, warn, error, info};

use crate::models::response::ChatCompletionChunk;
use crate::infrastructure::common::tools::UnifiedToolCall;

use super::types::MistralConfig;
use super::converter::{MistralConverter, MistralToolUtils};

/// Mistral streaming response parser for SSE format
pub struct MistralStreamParser {
    /// Buffer for incomplete chunks
    buffer: String,
    
    /// Converter for tool calling
    converter: MistralConverter,
    
    /// Configuration for compliance features
    config: Option<MistralConfig>,
    
    /// Streaming metrics for performance monitoring
    metrics: StreamingMetrics,
    
    /// Current tool calling state
    tool_call_state: ToolCallStreamState,
    
    /// Request ID for tracking
    request_id: String,
}

/// Tool calling state during streaming
#[derive(Debug, Clone)]
pub enum ToolCallStreamState {
    /// No tool calls detected
    None,
    
    /// Tool calls are being accumulated
    Accumulating {
        partial_calls: Vec<PartialToolCall>,
    },
    
    /// Tool calls complete, ready for execution
    Complete {
        tool_calls: Vec<UnifiedToolCall>,
    },
    
    /// Tool call results being processed
    ProcessingResults,
}

/// Partial tool call being accumulated during streaming
#[derive(Debug, Clone)]
pub struct PartialToolCall {
    /// Tool call ID
    pub id: Option<String>,
    
    /// Function name
    pub function_name: Option<String>,
    
    /// Accumulated arguments (JSON string)
    pub arguments: String,
    
    /// Whether this tool call is complete
    pub complete: bool,
}

/// Streaming performance metrics
#[derive(Debug, Clone)]
pub struct StreamingMetrics {
    /// Stream start time
    pub start_time: Instant,
    
    /// Number of chunks received
    pub chunks_received: u64,
    
    /// Total bytes received
    pub bytes_received: u64,
    
    /// Number of tool calls detected
    pub tool_calls_detected: u32,
    
    /// Average chunk processing time
    pub avg_chunk_time_ms: f64,
    
    /// Last chunk timestamp
    pub last_chunk_time: Option<Instant>,
    
    /// Stream completion time
    pub completion_time: Option<Instant>,
    
    /// European compliance flags
    pub eu_residency_used: bool,
    pub gdpr_mode_enabled: bool,
}

/// Parsed streaming chunk with Mistral-specific metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MistralStreamChunk {
    /// Standard chat completion chunk
    pub chunk: ChatCompletionChunk,
    
    /// Mistral-specific metadata
    pub metadata: MistralStreamMetadata,
    
    /// Tool calling information if present
    pub tool_calls: Option<Vec<PartialToolCall>>,
    
    /// Whether this chunk completes the response
    pub is_final: bool,
}

/// Mistral-specific streaming metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MistralStreamMetadata {
    /// Provider identifier
    pub provider: String,
    
    /// Request ID for tracking
    pub request_id: String,
    
    /// Chunk sequence number
    pub chunk_number: u64,
    
    /// Processing timestamp
    pub timestamp: Instant,
    
    /// European compliance information
    pub compliance: ComplianceInfo,
    
    /// Performance metrics
    pub performance: PerformanceInfo,
}

/// European compliance information for streaming
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComplianceInfo {
    /// Whether EU data residency was used
    pub eu_residency: bool,
    
    /// Whether GDPR mode is enabled
    pub gdpr_mode: bool,
    
    /// Data processing region
    pub processing_region: Option<String>,
}

/// Performance information for streaming
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceInfo {
    /// Chunk processing time in milliseconds
    pub processing_time_ms: f64,
    
    /// Chunk size in bytes
    pub chunk_size: u64,
    
    /// Cumulative tokens processed
    pub tokens_processed: Option<u32>,
}

impl MistralStreamParser {
    /// Create a new Mistral stream parser
    pub fn new(config: Option<MistralConfig>, request_id: String) -> Self {
        let converter = match &config {
            Some(cfg) => MistralConverter::with_config(cfg.clone()),
            None => MistralConverter::new(),
        };
        
        let metrics = StreamingMetrics {
            start_time: Instant::now(),
            chunks_received: 0,
            bytes_received: 0,
            tool_calls_detected: 0,
            avg_chunk_time_ms: 0.0,
            last_chunk_time: None,
            completion_time: None,
            eu_residency_used: config.as_ref()
                .map(|c| c.prefers_eu_residency())
                .unwrap_or(false),
            gdpr_mode_enabled: config.as_ref()
                .map(|c| c.uses_gdpr_mode())
                .unwrap_or(true),
        };
        
        debug!("Created Mistral stream parser for request: {}", request_id);
        
        Self {
            buffer: String::new(),
            converter,
            config,
            metrics,
            tool_call_state: ToolCallStreamState::None,
            request_id,
        }
    }

    /// Parse incoming SSE data and return processed chunks
    pub fn parse_chunk(&mut self, data: &[u8]) -> Result<Vec<MistralStreamChunk>> {
        let chunk_start = Instant::now();
        
        // Add new data to buffer
        self.buffer.push_str(
            &String::from_utf8_lossy(data)
        );
        self.metrics.bytes_received += data.len() as u64;

        let mut chunks = Vec::new();

        // Clone the buffer content to avoid borrowing issues
        let buffer_content = self.buffer.clone();
        let mut lines: Vec<String> = buffer_content.lines().map(|s| s.to_string()).collect();

        // Keep incomplete line in buffer
        if !self.buffer.ends_with('\n') && !self.buffer.ends_with("\r\n") {
            if let Some(last_line) = lines.pop() {
                self.buffer = last_line;
            } else {
                self.buffer.clear();
            }
        } else {
            self.buffer.clear();
        }

        // Process complete SSE events
        let mut current_event = SseEvent::default();

        for line in lines {
            if line.is_empty() {
                // Empty line indicates end of event
                if !current_event.data.is_empty() {
                    if let Some(chunk) = self.process_sse_event(&current_event, chunk_start)? {
                        chunks.push(chunk);
                    }
                }
                current_event = SseEvent::default();
            } else if let Some(colon_pos) = line.find(':') {
                let field = &line[..colon_pos];
                let value = line[colon_pos + 1..].trim_start();
                
                match field {
                    "data" => current_event.data = value.to_string(),
                    "event" => current_event.event_type = Some(value.to_string()),
                    "id" => current_event.id = Some(value.to_string()),
                    "retry" => {
                        if let Ok(retry_ms) = value.parse::<u64>() {
                            current_event.retry = Some(retry_ms);
                        }
                    }
                    _ => {} // Ignore unknown fields
                }
            }
        }

        // Update metrics
        self.metrics.chunks_received += chunks.len() as u64;
        self.metrics.last_chunk_time = Some(Instant::now());
        
        let processing_time = chunk_start.elapsed().as_secs_f64() * 1000.0;
        self.update_avg_processing_time(processing_time);

        trace!("Parsed {} chunks from {} bytes", chunks.len(), data.len());
        Ok(chunks)
    }

    /// Process a complete SSE event
    fn process_sse_event(&mut self, event: &SseEvent, chunk_start: Instant) -> Result<Option<MistralStreamChunk>> {
        // Handle [DONE] message
        if event.data == "[DONE]" {
            self.metrics.completion_time = Some(Instant::now());
            info!("Mistral stream completed for request: {}", self.request_id);
            return Ok(None);
        }

        // Skip empty data
        if event.data.is_empty() {
            return Ok(None);
        }

        // Parse JSON data
        let chunk_data: Value = serde_json::from_str(&event.data)
            .context("Failed to parse SSE event data as JSON")?;

        // Convert to ChatCompletionChunk
        let chunk: ChatCompletionChunk = serde_json::from_value(chunk_data.clone())
            .context("Failed to parse ChatCompletionChunk")?;

        // Process tool calls if present
        let tool_calls = self.process_tool_calls(&chunk_data)?;
        let is_final = self.is_final_chunk(&chunk);

        // Create metadata
        let processing_time = chunk_start.elapsed().as_secs_f64() * 1000.0;
        let metadata = self.create_chunk_metadata(processing_time, event.data.len() as u64);

        let mistral_chunk = MistralStreamChunk {
            chunk,
            metadata,
            tool_calls,
            is_final,
        };

        debug!("Processed Mistral stream chunk #{} for request: {}", 
               mistral_chunk.metadata.chunk_number, self.request_id);

        Ok(Some(mistral_chunk))
    }

    /// Process tool calls in the streaming chunk
    fn process_tool_calls(&mut self, chunk_data: &Value) -> Result<Option<Vec<PartialToolCall>>> {
        // Check if this chunk contains tool calls
        if !MistralToolUtils::has_tool_calls(chunk_data) {
            return Ok(None);
        }

        let choice = chunk_data.get("choices")
            .and_then(|c| c.as_array())
            .and_then(|arr| arr.first())
            .ok_or_else(|| anyhow::anyhow!("No choices in chunk"))?;

        let delta = choice.get("delta")
            .ok_or_else(|| anyhow::anyhow!("No delta in choice"))?;

        if let Some(tool_calls_value) = delta.get("tool_calls") {
            let mut partial_calls = Vec::new();
            
            if let Some(tool_calls_array) = tool_calls_value.as_array() {
                for (index, tc) in tool_calls_array.iter().enumerate() {
                    let mut partial_call = PartialToolCall {
                        id: tc.get("id").and_then(|v| v.as_str()).map(String::from),
                        function_name: tc.get("function")
                            .and_then(|f| f.get("name"))
                            .and_then(|n| n.as_str())
                            .map(String::from),
                        arguments: tc.get("function")
                            .and_then(|f| f.get("arguments"))
                            .and_then(|a| a.as_str())
                            .unwrap_or("")
                            .to_string(),
                        complete: false,
                    };

                    // Update existing tool call state
                    match &mut self.tool_call_state {
                        ToolCallStreamState::None => {
                            self.tool_call_state = ToolCallStreamState::Accumulating {
                                partial_calls: vec![partial_call.clone()],
                            };
                        }
                        ToolCallStreamState::Accumulating { partial_calls: existing_calls } => {
                            if index < existing_calls.len() {
                                // Accumulate arguments for existing tool call
                                existing_calls[index].arguments.push_str(&partial_call.arguments);
                                if let Some(id) = &partial_call.id {
                                    existing_calls[index].id = Some(id.clone());
                                }
                                if let Some(name) = &partial_call.function_name {
                                    existing_calls[index].function_name = Some(name.clone());
                                }
                                partial_call = existing_calls[index].clone();
                            } else {
                                // New tool call
                                existing_calls.push(partial_call.clone());
                            }
                        }
                        _ => {} // Other states don't need accumulation
                    }

                    partial_calls.push(partial_call);
                }

                self.metrics.tool_calls_detected += partial_calls.len() as u32;
                return Ok(Some(partial_calls));
            }
        }

        Ok(None)
    }

    /// Check if this is the final chunk in the stream
    fn is_final_chunk(&self, chunk: &ChatCompletionChunk) -> bool {
        chunk.choices.iter().any(|choice| {
            choice.finish_reason.is_some() || 
            choice.delta.content.as_ref().map_or(false, |c| c.is_empty())
        })
    }

    /// Create metadata for the chunk
    fn create_chunk_metadata(&mut self, processing_time_ms: f64, chunk_size: u64) -> MistralStreamMetadata {
        self.metrics.chunks_received += 1;
        
        MistralStreamMetadata {
            provider: "mistral".to_string(),
            request_id: self.request_id.clone(),
            chunk_number: self.metrics.chunks_received,
            timestamp: Instant::now(),
            compliance: ComplianceInfo {
                eu_residency: self.metrics.eu_residency_used,
                gdpr_mode: self.metrics.gdpr_mode_enabled,
                processing_region: if self.metrics.eu_residency_used {
                    Some("EU".to_string())
                } else {
                    None
                },
            },
            performance: PerformanceInfo {
                processing_time_ms,
                chunk_size,
                tokens_processed: None, // Could be enhanced with token counting
            },
        }
    }

    /// Update average processing time
    fn update_avg_processing_time(&mut self, processing_time_ms: f64) {
        let total_chunks = self.metrics.chunks_received as f64;
        self.metrics.avg_chunk_time_ms = 
            (self.metrics.avg_chunk_time_ms * (total_chunks - 1.0) + processing_time_ms) / total_chunks;
    }

    /// Get current streaming metrics
    pub fn get_metrics(&self) -> &StreamingMetrics {
        &self.metrics
    }

    /// Get current tool call state
    pub fn get_tool_call_state(&self) -> &ToolCallStreamState {
        &self.tool_call_state
    }

    /// Finalize tool calls and convert to unified format
    pub fn finalize_tool_calls(&mut self) -> Result<Option<Vec<UnifiedToolCall>>> {
        match std::mem::replace(&mut self.tool_call_state, ToolCallStreamState::ProcessingResults) {
            ToolCallStreamState::Accumulating { partial_calls } => {
                let mut unified_calls = Vec::new();
                
                for partial in partial_calls {
                    if let (Some(id), Some(name)) = (partial.id, partial.function_name) {
                        let arguments = if partial.arguments.is_empty() {
                            serde_json::Value::Object(serde_json::Map::new())
                        } else {
                            serde_json::from_str(&partial.arguments)
                                .unwrap_or_else(|_| serde_json::Value::String(partial.arguments))
                        };

                        let mut metadata = std::collections::HashMap::new();
                        metadata.insert("provider".to_string(), serde_json::Value::String("mistral".to_string()));
                        metadata.insert("stream_accumulated".to_string(), serde_json::Value::Bool(true));

                        unified_calls.push(UnifiedToolCall {
                            id,
                            function_name: name,
                            arguments,
                            metadata,
                        });
                    }
                }

                debug!("Finalized {} tool calls from stream", unified_calls.len());
                Ok(Some(unified_calls))
            }
            _ => Ok(None)
        }
    }
}

/// Server-Sent Event structure
#[derive(Debug, Default)]
struct SseEvent {
    data: String,
    event_type: Option<String>,
    id: Option<String>,
    retry: Option<u64>,
}

/// Mistral streaming utilities
pub struct MistralStreamUtils;

impl MistralStreamUtils {
    /// Create a stream processor for Mistral responses
    pub fn create_stream_processor(
        config: Option<MistralConfig>,
        request_id: String
    ) -> MistralStreamParser {
        MistralStreamParser::new(config, request_id)
    }

    /// Process a byte stream into Mistral chunks
    pub async fn process_byte_stream<S>(
        mut stream: S,
        config: Option<MistralConfig>,
        request_id: String,
    ) -> Result<Vec<MistralStreamChunk>>
    where
        S: Stream<Item = Result<bytes::Bytes>> + Unpin,
    {
        let mut parser = MistralStreamParser::new(config, request_id);
        let mut all_chunks = Vec::new();

        while let Some(bytes_result) = stream.next().await {
            let bytes = bytes_result.context("Failed to read stream bytes")?;
            let chunks = parser.parse_chunk(&bytes)
                .context("Failed to parse stream chunk")?;
            all_chunks.extend(chunks);
        }

        Ok(all_chunks)
    }

    /// Calculate streaming performance metrics
    pub fn calculate_performance_metrics(chunks: &[MistralStreamChunk]) -> StreamingPerformanceReport {
        if chunks.is_empty() {
            return StreamingPerformanceReport::default();
        }

        let total_chunks = chunks.len() as u64;
        let total_bytes: u64 = chunks.iter()
            .map(|c| c.metadata.performance.chunk_size)
            .sum();
        
        let avg_processing_time: f64 = chunks.iter()
            .map(|c| c.metadata.performance.processing_time_ms)
            .sum::<f64>() / chunks.len() as f64;

        let stream_duration = chunks.last().unwrap().metadata.timestamp
            .duration_since(chunks.first().unwrap().metadata.timestamp)
            .as_secs_f64();

        StreamingPerformanceReport {
            total_chunks,
            total_bytes,
            avg_processing_time_ms: avg_processing_time,
            stream_duration_seconds: stream_duration,
            throughput_chunks_per_second: if stream_duration > 0.0 { 
                total_chunks as f64 / stream_duration 
            } else { 
                0.0 
            },
            throughput_bytes_per_second: if stream_duration > 0.0 { 
                total_bytes as f64 / stream_duration 
            } else { 
                0.0 
            },
        }
    }

    /// Extract final response from stream chunks
    pub fn extract_final_response(chunks: &[MistralStreamChunk]) -> Result<String> {
        let mut content = String::new();
        
        for chunk in chunks {
            if let Some(choice) = chunk.chunk.choices.first() {
                if let Some(ref delta_content) = choice.delta.content {
                    content.push_str(delta_content);
                }
            }
        }

        Ok(content)
    }

    /// Check if stream contains tool calls
    pub fn has_tool_calls_in_stream(chunks: &[MistralStreamChunk]) -> bool {
        chunks.iter().any(|c| c.tool_calls.is_some())
    }
}

/// Streaming performance report
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct StreamingPerformanceReport {
    /// Total number of chunks processed
    pub total_chunks: u64,
    
    /// Total bytes processed
    pub total_bytes: u64,
    
    /// Average processing time per chunk (ms)
    pub avg_processing_time_ms: f64,
    
    /// Total stream duration (seconds)
    pub stream_duration_seconds: f64,
    
    /// Throughput in chunks per second
    pub throughput_chunks_per_second: f64,
    
    /// Throughput in bytes per second
    pub throughput_bytes_per_second: f64,
}

#[cfg(test)]
mod tests {
    use super::*;
    use bytes::Bytes;
    use futures_util::stream;

    #[tokio::test]
    async fn test_stream_parser_creation() {
        let config = Some(MistralConfig::default());
        let request_id = "test_request".to_string();
        let parser = MistralStreamParser::new(config, request_id);
        
        assert_eq!(parser.request_id, "test_request");
        assert_eq!(parser.metrics.chunks_received, 0);
    }

    #[tokio::test]
    async fn test_sse_parsing() {
        let mut parser = MistralStreamParser::new(None, "test".to_string());
        
        let sse_data = b"data: {\"choices\":[{\"delta\":{\"content\":\"Hello\"}}]}\n\n";
        let chunks = parser.parse_chunk(sse_data).unwrap();
        
        assert_eq!(chunks.len(), 1);
        assert_eq!(parser.metrics.chunks_received, 1);
    }

    #[tokio::test]
    async fn test_done_message() {
        let mut parser = MistralStreamParser::new(None, "test".to_string());
        
        let done_data = b"data: [DONE]\n\n";
        let chunks = parser.parse_chunk(done_data).unwrap();
        
        assert_eq!(chunks.len(), 0);
        assert!(parser.metrics.completion_time.is_some());
    }

    #[test]
    fn test_performance_metrics() {
        let chunks = vec![];
        let report = MistralStreamUtils::calculate_performance_metrics(&chunks);
        
        assert_eq!(report.total_chunks, 0);
        assert_eq!(report.total_bytes, 0);
    }

    #[test]
    fn test_final_response_extraction() {
        let chunks = vec![];
        let response = MistralStreamUtils::extract_final_response(&chunks).unwrap();
        
        assert!(response.is_empty());
    }
}