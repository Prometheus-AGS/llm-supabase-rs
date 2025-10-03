// src/infrastructure/vertex/streaming.rs
//
// Vertex AI streaming response handler
// Processes Server-Sent Events from Vertex AI and converts them to OpenAI format

use anyhow::{Context, Result};
use bytes::Bytes;
use futures_util::{Stream, StreamExt, TryStreamExt};
use std::pin::Pin;
use std::task::{Context as TaskContext, Poll};
use tokio_stream::wrappers::ReceiverStream;
use tokio::sync::mpsc;
use tracing::{debug, error, warn};

use crate::infrastructure::vertex::{
    VertexStreamChunk, FormatConverter
};
use crate::models::response::ChatCompletionChunk;

/// Streaming response handler for Vertex AI
///
/// This handles the processing of Server-Sent Events (SSE) from Vertex AI's
/// streaming API and converts them to OpenAI-compatible streaming format.
pub struct VertexStreamingHandler {
    /// Request ID for tracking
    request_id: String,

    /// Model name for response metadata
    model: String,

    /// Buffer for incomplete SSE chunks
    buffer: String,

    /// Whether the stream has ended
    ended: bool,
}

impl VertexStreamingHandler {
    /// Create a new streaming handler
    pub fn new(request_id: String, model: String) -> Self {
        Self {
            request_id,
            model,
            buffer: String::new(),
            ended: false,
        }
    }

    /// Process a raw bytes stream from Vertex AI into OpenAI chat completion chunks
    pub async fn process_stream<S>(
        &mut self,
        stream: S,
    ) -> Result<impl Stream<Item = Result<ChatCompletionChunk>>>
    where
        S: Stream<Item = Result<Bytes>> + Send + 'static,
    {
        let (tx, rx) = mpsc::channel(100);
        let request_id = self.request_id.clone();
        let model = self.model.clone();

        // Spawn a task to process the stream
        tokio::spawn(async move {
            let mut handler = VertexStreamingHandler::new(request_id.clone(), model.clone());
            let mut stream = Box::pin(stream);

            while let Some(chunk_result) = stream.next().await {
                match chunk_result {
                    Ok(bytes) => {
                        match handler.process_bytes(&bytes).await {
                            Ok(chunks) => {
                                for chunk in chunks {
                                    if tx.send(Ok(chunk)).await.is_err() {
                                        debug!("Stream receiver dropped");
                                        return;
                                    }
                                }
                            }
                            Err(e) => {
                                error!("Error processing stream chunk: {}", e);
                                if tx.send(Err(e)).await.is_err() {
                                    debug!("Stream receiver dropped");
                                }
                                return;
                            }
                        }
                    }
                    Err(e) => {
                        error!("Error receiving stream chunk: {}", e);
                        if tx.send(Err(e)).await.is_err() {
                            debug!("Stream receiver dropped");
                        }
                        return;
                    }
                }
            }

            // Send final chunk if needed
            if let Some(final_chunk) = handler.finalize().await {
                let _ = tx.send(Ok(final_chunk)).await;
            }
        });

        Ok(ReceiverStream::new(rx))
    }

    /// Process raw bytes and extract complete SSE events
    async fn process_bytes(&mut self, bytes: &[u8]) -> Result<Vec<ChatCompletionChunk>> {
        if self.ended {
            return Ok(vec![]);
        }

        // Convert bytes to string and add to buffer
        let chunk_str = String::from_utf8_lossy(bytes);
        self.buffer.push_str(&chunk_str);

        let mut chunks = Vec::new();

        // Process complete SSE events from buffer
        while let Some(event) = self.extract_next_sse_event()? {
            if let Some(chunk) = self.process_sse_event(&event).await? {
                // Check if this is the final chunk before moving
                let is_final = chunk.is_final();
                chunks.push(chunk);

                if is_final {
                    self.ended = true;
                    break;
                }
            }
        }

        Ok(chunks)
    }

    /// Extract the next complete SSE event from the buffer
    fn extract_next_sse_event(&mut self) -> Result<Option<String>> {
        // SSE events are separated by double newlines
        if let Some(end_pos) = self.buffer.find("\n\n") {
            let event = self.buffer[..end_pos].to_string();
            self.buffer = self.buffer[end_pos + 2..].to_string();

            // Skip empty events
            if event.trim().is_empty() {
                return self.extract_next_sse_event();
            }

            Ok(Some(event))
        } else {
            Ok(None)
        }
    }

    /// Process a single SSE event and convert to OpenAI format
    async fn process_sse_event(&self, event: &str) -> Result<Option<ChatCompletionChunk>> {
        debug!("Processing SSE event: {}", event);

        // Parse SSE format (data: {...})
        let mut data_lines = Vec::new();

        for line in event.lines() {
            let line = line.trim();

            // Skip comments and empty lines
            if line.is_empty() || line.starts_with(':') {
                continue;
            }

            // Handle data lines
            if let Some(data) = line.strip_prefix("data: ") {
                data_lines.push(data);
            }
        }

        if data_lines.is_empty() {
            return Ok(None);
        }

        // Join multiple data lines if present
        let data_str = data_lines.join("");

        // Handle [DONE] signal
        if data_str.trim() == "[DONE]" {
            debug!("Received [DONE] signal, ending stream");
            return Ok(Some(ChatCompletionChunk::final_chunk(
                self.request_id.clone(),
                self.model.clone(),
                Default::default(),
            )));
        }

        // Parse JSON data
        let vertex_chunk: VertexStreamChunk = serde_json::from_str(&data_str)
            .with_context(|| format!("Failed to parse Vertex AI streaming chunk: {}", data_str))?;

        // Convert to OpenAI format using the converter
        FormatConverter::vertex_chunk_to_openai_v2(
            &vertex_chunk,
            &self.request_id,
            &self.model,
        )
    }

    /// Finalize the stream and return a final chunk if needed
    async fn finalize(&self) -> Option<ChatCompletionChunk> {
        if !self.ended {
            // If we haven't received a proper end signal, send a final chunk
            Some(ChatCompletionChunk::final_chunk(
                self.request_id.clone(),
                self.model.clone(),
                Default::default(),
            ))
        } else {
            None
        }
    }
}

/// Streaming processor for converting Vertex AI streaming responses to OpenAI format
pub struct VertexStreamProcessor;

impl VertexStreamProcessor {
    /// Process a Vertex AI streaming response and convert to OpenAI format
    pub async fn process_response<S>(
        stream: S,
        request_id: &str,
        model: &str,
    ) -> Result<impl Stream<Item = Result<ChatCompletionChunk>>>
    where
        S: Stream<Item = Result<Bytes>> + Send + 'static,
    {
        let mut handler = VertexStreamingHandler::new(
            request_id.to_string(),
            model.to_string(),
        );

        handler.process_stream(stream).await
    }

    /// Create a streaming processor for Server-Sent Events format
    /// This handles the HTTP streaming response from Vertex AI
    pub fn create_sse_processor<S>(
        stream: S,
        _request_id: &str,
        _model: &str,
    ) -> impl Stream<Item = Result<String>>
    where
        S: Stream<Item = Result<ChatCompletionChunk>> + Send,
    {
        // let _request_id = request_id.to_string(); // Unused for now

        stream.map(move |chunk_result| {
            match chunk_result {
                Ok(chunk) => {
                    // Convert to SSE format
                    let json = serde_json::to_string(&chunk)
                        .map_err(|e| anyhow::anyhow!("Failed to serialize chunk: {}", e))?;

                    Ok(format!("data: {}\n\n", json))
                }
                Err(e) => Err(e),
            }
        })
    }

    /// Handle streaming errors and convert to appropriate error chunks
    pub fn handle_streaming_error(
        error: anyhow::Error,
        request_id: &str,
        model: &str,
    ) -> ChatCompletionChunk {
        warn!("Streaming error: {}", error);

        // Create an error chunk in OpenAI format
        ChatCompletionChunk::error_chunk(
            request_id.to_string(),
            model.to_string(),
            format!("Streaming error: {}", error),
        )
    }
}

/// Advanced streaming utilities for handling complex scenarios
pub struct StreamingUtils;

impl StreamingUtils {
    /// Buffer and batch streaming chunks for better performance
    pub fn batch_chunks<S>(
        stream: S,
        batch_size: usize,
    ) -> impl Stream<Item = Result<Vec<ChatCompletionChunk>>>
    where
        S: Stream<Item = Result<ChatCompletionChunk>>,
    {
        stream
            .try_chunks(batch_size)
            .map_err(|e| anyhow::anyhow!("Chunking error: {}", e.1))
    }

    /// Add timing information to streaming chunks
    pub fn with_timing<S>(
        stream: S,
    ) -> impl Stream<Item = Result<(ChatCompletionChunk, std::time::Duration)>>
    where
        S: Stream<Item = Result<ChatCompletionChunk>>,
    {
        let start_time = std::time::Instant::now();

        stream.map(move |chunk_result| {
            let elapsed = start_time.elapsed();
            chunk_result.map(|chunk| (chunk, elapsed))
        })
    }

    /// Filter out empty content chunks to reduce bandwidth
    pub fn filter_empty_content<S>(
        stream: S,
    ) -> impl Stream<Item = Result<ChatCompletionChunk>>
    where
        S: Stream<Item = Result<ChatCompletionChunk>>,
    {
        stream.try_filter(|chunk| {
            // Keep final chunks and chunks with content
            std::future::ready(
                chunk.is_final() ||
                chunk.choices.iter().any(|choice|
                    !choice.delta.content.is_empty()
                )
            )
        })
    }

    /// Add heartbeat chunks to prevent connection timeouts
    pub fn with_heartbeat<S>(
        stream: S,
        interval: std::time::Duration,
        request_id: String,
        model: String,
    ) -> impl Stream<Item = Result<ChatCompletionChunk>>
    where
        S: Stream<Item = Result<ChatCompletionChunk>> + Send + 'static,
    {
        use tokio_stream::StreamExt;

        let heartbeat_stream = futures_util::StreamExt::map(
            tokio_stream::wrappers::IntervalStream::new(
                tokio::time::interval(interval)
            ),
            move |_| {
                Ok(ChatCompletionChunk::heartbeat_chunk(
                    request_id.clone(),
                    model.clone(),
                ))
            }
        );

        // Merge the main stream with heartbeat
        stream.merge(heartbeat_stream)
    }
}

/// Pin-based stream adapter for complex streaming scenarios
pub struct PinnedVertexStream<S> {
    inner: Pin<Box<S>>,
}

impl<S> PinnedVertexStream<S> {
    pub fn new(stream: S) -> Self {
        Self {
            inner: Box::pin(stream),
        }
    }
}

impl<S> Stream for PinnedVertexStream<S>
where
    S: Stream<Item = Result<ChatCompletionChunk>>,
{
    type Item = Result<ChatCompletionChunk>;

    fn poll_next(
        mut self: Pin<&mut Self>,
        cx: &mut TaskContext<'_>,
    ) -> Poll<Option<Self::Item>> {
        self.inner.as_mut().poll_next(cx)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio_stream;
    use bytes::Bytes;

    #[tokio::test]
    async fn test_sse_event_parsing() {
        let mut handler = VertexStreamingHandler::new(
            "test-123".to_string(),
            "claude-4-sonnet-20250514".to_string(),
        );

        let sse_event = "data: {\"content\":\"Hello\",\"usage\":null,\"finish_reason\":null,\"error\":null}";
        let chunk = handler.process_sse_event(sse_event).await.unwrap();

        assert!(chunk.is_some());
        let chunk = chunk.unwrap();
        assert_eq!(chunk.id, "test-123");
        assert_eq!(chunk.model, "claude-4-sonnet-20250514");
        assert_eq!(chunk.choices[0].delta.content, "Hello");
    }

    #[tokio::test]
    async fn test_done_signal() {
        let mut handler = VertexStreamingHandler::new(
            "test-123".to_string(),
            "claude-4-sonnet-20250514".to_string(),
        );

        let sse_event = "data: [DONE]";
        let chunk = handler.process_sse_event(sse_event).await.unwrap();

        assert!(chunk.is_some());
        let chunk = chunk.unwrap();
        assert!(chunk.is_final());
    }

    #[tokio::test]
    async fn test_multiline_sse() {
        let mut handler = VertexStreamingHandler::new(
            "test-123".to_string(),
            "claude-4-sonnet-20250514".to_string(),
        );

        let sse_event = "event: completion\ndata: {\"content\":\"Test\"}\nid: 1";
        let chunk = handler.process_sse_event(sse_event).await.unwrap();

        assert!(chunk.is_some());
    }

    #[tokio::test]
    async fn test_buffer_processing() {
        let mut handler = VertexStreamingHandler::new(
            "test-123".to_string(),
            "claude-4-sonnet-20250514".to_string(),
        );

        // Simulate incomplete data
        let bytes1 = b"data: {\"content\":\"Hel";
        let bytes2 = b"lo\"}\n\n";

        // First chunk should not produce any results (incomplete)
        let chunks1 = handler.process_bytes(bytes1).await.unwrap();
        assert!(chunks1.is_empty());

        // Second chunk completes the event
        let chunks2 = handler.process_bytes(bytes2).await.unwrap();
        assert_eq!(chunks2.len(), 1);
        assert_eq!(chunks2[0].choices[0].delta.content, "Hello");
    }

    #[tokio::test]
    async fn test_stream_processor() {
        use tokio_stream::iter;

        let data = vec![
            Ok(Bytes::from("data: {\"content\":\"Hello\"}\n\n")),
            Ok(Bytes::from("data: [DONE]\n\n")),
        ];

        let stream = iter(data);
        let mut result_stream = VertexStreamProcessor::process_response(
            stream,
            "test-123",
            "claude-4-sonnet-20250514",
        ).await.unwrap();

        let first_chunk = result_stream.next().await.unwrap().unwrap();
        assert_eq!(first_chunk.choices[0].delta.content, "Hello");

        let final_chunk = result_stream.next().await.unwrap().unwrap();
        assert!(final_chunk.is_final());
    }

    #[tokio::test]
    async fn test_error_handling() {
        use tokio_stream::iter;

        let data = vec![
            Err(anyhow::anyhow!("Network error")),
        ];

        let stream = iter(data);
        let mut result_stream = VertexStreamProcessor::process_response(
            stream,
            "test-123",
            "claude-4-sonnet-20250514",
        ).await.unwrap();

        let error_result = result_stream.next().await.unwrap();
        assert!(error_result.is_err());
    }

    #[test]
    fn test_sse_extraction() {
        let mut handler = VertexStreamingHandler::new(
            "test".to_string(),
            "model".to_string(),
        );

        handler.buffer = "data: first\n\ndata: second\n\ndata: partial".to_string();

        let first = handler.extract_next_sse_event().unwrap().unwrap();
        assert_eq!(first, "data: first");

        let second = handler.extract_next_sse_event().unwrap().unwrap();
        assert_eq!(second, "data: second");

        let third = handler.extract_next_sse_event().unwrap();
        assert!(third.is_none());

        assert_eq!(handler.buffer, "data: partial");
    }

    #[tokio::test]
    async fn test_streaming_utils_batching() {
        use tokio_stream::iter;
        use crate::models::response::{ChatCompletionChunk};

        let chunks = vec![
            Ok(ChatCompletionChunk::new("1".to_string(), "model".to_string(), vec![])),
            Ok(ChatCompletionChunk::new("2".to_string(), "model".to_string(), vec![])),
            Ok(ChatCompletionChunk::new("3".to_string(), "model".to_string(), vec![])),
        ];

        let stream = iter(chunks);
        let mut batched = StreamingUtils::batch_chunks(stream, 2);

        let first_batch = batched.next().await.unwrap().unwrap();
        assert_eq!(first_batch.len(), 2);

        let second_batch = batched.next().await.unwrap().unwrap();
        assert_eq!(second_batch.len(), 1);
    }

    #[tokio::test]
    async fn test_filter_empty_content() {
        use tokio_stream::iter;
        use crate::models::response::{ChatCompletionChunk, ChatCompletionChunkChoice};
        use crate::models::common::ChatMessage;

        let chunks = vec![
            Ok(ChatCompletionChunk::new("1".to_string(), "model".to_string(), vec![
                ChatCompletionChunkChoice {
                    index: 0,
                    delta: ChatMessage::assistant("Hello"),
                    logprobs: None,
                    finish_reason: None,
                }
            ])),
            Ok(ChatCompletionChunk::new("2".to_string(), "model".to_string(), vec![
                ChatCompletionChunkChoice {
                    index: 0,
                    delta: ChatMessage::assistant(""),
                    logprobs: None,
                    finish_reason: None,
                }
            ])),
        ];

        let stream = iter(chunks);
        let mut filtered = StreamingUtils::filter_empty_content(stream);

        let first_chunk = filtered.next().await.unwrap().unwrap();
        assert_eq!(first_chunk.choices[0].delta.content, "Hello");

        // Empty chunk should be filtered out
        assert!(filtered.next().await.is_none());
    }
}