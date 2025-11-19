// src/infrastructure/common/streaming_v2.rs
//
// Provider-specific streaming processors for OpenAI API compliance

use anyhow::Result;
use async_stream::stream;
use async_trait::async_trait;
use futures_util::Stream;
use serde_json::Value;
use std::collections::HashMap;
use std::pin::Pin;
use tokio_stream::StreamExt;
use tracing::{debug, error, info, warn};

use crate::models::{
    common::{ChatMessage, MessageRole, FinishReason, Usage},
    response::{ChatCompletionChunk, ChatCompletionChunkChoice},
};
use super::tools::{UnifiedToolCall, ToolCallManager};

/// Streaming behavior patterns for different providers
#[derive(Debug, Clone)]
pub enum StreamingBehavior {
    /// Token-by-token streaming (OpenAI, Groq, Together, Fireworks)
    TokenByToken { 
        supports_delta_tool_calls: bool 
    },
    /// Complete blocks buffering (Vertex AI, Anthropic)
    CompleteBlocks { 
        buffer_until_complete: bool 
    },
    /// JSON lines streaming (AWS Bedrock)
    JsonLines { 
        parse_each_line: bool 
    },
}

/// Normalized chunk that all processors produce
#[derive(Debug, Clone)]
pub struct NormalizedChunk {
    pub content: Option<String>,
    pub tool_calls: Option<Vec<UnifiedToolCall>>,
    pub finish_reason: Option<FinishReason>,
    pub usage: Option<Usage>,
    pub is_final: bool,
}

/// Trait for provider-specific streaming processors
#[async_trait]
pub trait StreamProcessor: Send + Sync {
    /// Process a raw chunk from the provider
    async fn process_chunk(&mut self, chunk: &[u8]) -> Result<Vec<NormalizedChunk>>;
    
    /// Finalize the stream and return any remaining chunks
    async fn finalize(&mut self) -> Result<Vec<NormalizedChunk>>;
    
    /// Get the streaming behavior for this processor
    fn get_behavior(&self) -> &StreamingBehavior;
    
    /// Check if tool calls are detected and stream should pause
    fn has_pending_tool_calls(&self) -> bool;
    
    /// Get pending tool calls if any
    fn get_pending_tool_calls(&mut self) -> Option<Vec<UnifiedToolCall>>;
}

/// Stream normalizer that converts provider chunks to OpenAI format
pub struct StreamNormalizer {
    request_id: String,
    model: String,
    tool_manager: Option<ToolCallManager>,
}

impl StreamNormalizer {
    pub fn new(request_id: String, model: String, tool_manager: Option<ToolCallManager>) -> Self {
        Self {
            request_id,
            model,
            tool_manager,
        }
    }
    
    /// Normalize a stream of provider chunks to OpenAI format
    pub fn normalize_stream<S>(
        &self,
        mut processor: Box<dyn StreamProcessor>,
        raw_stream: S,
    ) -> impl Stream<Item = Result<ChatCompletionChunk>> + '_
    where
        S: Stream<Item = Result<bytes::Bytes, reqwest::Error>> + Send + 'static,
    {
        stream! {
            tokio::pin!(raw_stream);
            
            while let Some(chunk_result) = raw_stream.next().await {
                match chunk_result {
                    Ok(bytes) => {
                        match processor.process_chunk(&bytes).await {
                            Ok(normalized_chunks) => {
                                for normalized in normalized_chunks {
                                    match self.convert_to_openai_chunk(&normalized) {
                                        Ok(Some(openai_chunk)) => yield Ok(openai_chunk),
                                        Ok(None) => continue,
                                        Err(e) => {
                                            error!("Failed to convert normalized chunk: {}", e);
                                            yield Err(e);
                                        }
                                    }
                                }
                                
                                // Check if we have tool calls and need to pause the stream
                                if processor.has_pending_tool_calls() {
                                    info!("Tool calls detected, terminating stream as per OpenAI spec");
                                    
                                    // Send final tool call chunk
                                    if let Some(tool_calls) = processor.get_pending_tool_calls() {
                                        match self.create_tool_call_chunk(&tool_calls) {
                                            Ok(tool_chunk) => yield Ok(tool_chunk),
                                            Err(e) => yield Err(e),
                                        }
                                    }
                                    
                                    // Stream should terminate here - client will send tool results
                                    // in a new request (continuation pattern)
                                    break;
                                }
                            }
                            Err(e) => {
                                error!("Stream processing error: {}", e);
                                yield Err(e);
                                break;
                            }
                        }
                    }
                    Err(e) => {
                        error!("Raw stream error: {}", e);
                        yield Err(anyhow::anyhow!("Stream error: {}", e));
                        break;
                    }
                }
            }
            
            // Finalize the processor
            match processor.finalize().await {
                Ok(final_chunks) => {
                    for normalized in final_chunks {
                        match self.convert_to_openai_chunk(&normalized) {
                            Ok(Some(openai_chunk)) => yield Ok(openai_chunk),
                            Ok(None) => continue,
                            Err(e) => {
                                error!("Failed to convert final chunk: {}", e);
                                yield Err(e);
                            }
                        }
                    }
                }
                Err(e) => {
                    error!("Stream finalization error: {}", e);
                    yield Err(e);
                }
            }
        }
    }
    
    /// Convert normalized chunk to OpenAI format
    fn convert_to_openai_chunk(&self, normalized: &NormalizedChunk) -> Result<Option<ChatCompletionChunk>> {
        if normalized.content.is_none() && normalized.tool_calls.is_none() && normalized.usage.is_none() {
            return Ok(None);
        }
        
        let delta = ChatMessage {
            role: MessageRole::Assistant,
            content: normalized.content.clone().unwrap_or_default(),
            name: None,
            function_call: None,
            tool_calls: normalized.tool_calls.as_ref().map(|calls| {
                calls.iter().map(|call| serde_json::to_value(call).unwrap_or_default()).collect()
            }),
            tool_call_id: None,
        };
        
        let choice = ChatCompletionChunkChoice {
            index: 0,
            delta,
            logprobs: None,
            finish_reason: normalized.finish_reason.clone(),
        };
        
        let chunk = if let Some(usage) = &normalized.usage {
            ChatCompletionChunk::new_with_usage(
                self.request_id.clone(),
                self.model.clone(),
                vec![choice],
                usage.clone(),
            )
        } else {
            ChatCompletionChunk::new(
                self.request_id.clone(),
                self.model.clone(),
                vec![choice],
            )
        };
        
        Ok(Some(chunk))
    }
    
    /// Create a chunk specifically for tool calls
    fn create_tool_call_chunk(&self, tool_calls: &[UnifiedToolCall]) -> Result<ChatCompletionChunk> {
        let tool_calls_json: Vec<Value> = tool_calls.iter()
            .map(|call| serde_json::to_value(call))
            .collect::<Result<Vec<_>, _>>()?;
        
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
            finish_reason: Some(FinishReason::ToolCalls),
        };
        
        Ok(ChatCompletionChunk::new(
            self.request_id.clone(),
            self.model.clone(),
            vec![choice],
        ))
    }
}

/// OpenAI/Groq token-by-token streaming processor
pub struct OpenAIStreamProcessor {
    behavior: StreamingBehavior,
    line_buffer: String,
    tool_call_builder: ToolCallBuilder,
    pending_tool_calls: Option<Vec<UnifiedToolCall>>,
}

impl OpenAIStreamProcessor {
    pub fn new() -> Self {
        Self {
            behavior: StreamingBehavior::TokenByToken { 
                supports_delta_tool_calls: true 
            },
            line_buffer: String::new(),
            tool_call_builder: ToolCallBuilder::new(),
            pending_tool_calls: None,
        }
    }
}

#[async_trait]
impl StreamProcessor for OpenAIStreamProcessor {
    async fn process_chunk(&mut self, chunk: &[u8]) -> Result<Vec<NormalizedChunk>> {
        let chunk_str = String::from_utf8_lossy(chunk);
        self.line_buffer.push_str(&chunk_str);
        
        let mut results = Vec::new();
        
        while let Some(line_end) = self.line_buffer.find('\n') {
            let line = self.line_buffer[..line_end].trim().to_string();
            self.line_buffer.drain(..line_end + 1);
            
            if line.is_empty() {
                continue;
            }
            
            if let Some(normalized) = self.process_line(&line).await? {
                results.push(normalized);
            }
        }
        
        Ok(results)
    }
    
    async fn finalize(&mut self) -> Result<Vec<NormalizedChunk>> {
        let mut results = Vec::new();
        
        // Process any remaining buffered content
        if !self.line_buffer.is_empty() {
            let line_buffer = self.line_buffer.clone();
            self.line_buffer.clear();
            if let Some(normalized) = self.process_line(&line_buffer).await? {
                results.push(normalized);
            }
        }
        
        // Finalize any pending tool calls
        if let Some(tool_calls) = self.tool_call_builder.finalize() {
            self.pending_tool_calls = Some(tool_calls);
        }
        
        Ok(results)
    }
    
    fn get_behavior(&self) -> &StreamingBehavior {
        &self.behavior
    }
    
    fn has_pending_tool_calls(&self) -> bool {
        self.pending_tool_calls.is_some()
    }
    
    fn get_pending_tool_calls(&mut self) -> Option<Vec<UnifiedToolCall>> {
        self.pending_tool_calls.take()
    }
}

impl OpenAIStreamProcessor {
    async fn process_line(&mut self, line: &str) -> Result<Option<NormalizedChunk>> {
        // Handle SSE format
        let json_str = if line.starts_with("data: ") {
            let data = line.strip_prefix("data: ").unwrap();
            if data.trim() == "[DONE]" {
                return Ok(Some(NormalizedChunk {
                    content: None,
                    tool_calls: None,
                    finish_reason: Some(FinishReason::Stop),
                    usage: None,
                    is_final: true,
                }));
            }
            data
        } else {
            line
        };
        
        let json: Value = serde_json::from_str(json_str)?;
        
        // Extract content from choices[0].delta
        if let Some(choices) = json.get("choices").and_then(|c| c.as_array()) {
            if let Some(choice) = choices.first() {
                let mut normalized = NormalizedChunk {
                    content: None,
                    tool_calls: None,
                    finish_reason: None,
                    usage: None,
                    is_final: false,
                };
                
                // Extract delta content
                if let Some(delta) = choice.get("delta") {
                    if let Some(content) = delta.get("content").and_then(|c| c.as_str()) {
                        normalized.content = Some(content.to_string());
                    }
                    
                    // Handle incremental tool calls
                    if let Some(tool_calls) = delta.get("tool_calls") {
                        self.tool_call_builder.add_delta(tool_calls)?;
                    }
                }
                
                // Extract finish reason
                if let Some(finish_reason) = choice.get("finish_reason").and_then(|f| f.as_str()) {
                    normalized.finish_reason = match finish_reason {
                        "stop" => Some(FinishReason::Stop),
                        "length" => Some(FinishReason::Length),
                        "tool_calls" => {
                            // Finalize tool calls and mark for stream termination
                            if let Some(tool_calls) = self.tool_call_builder.finalize() {
                                self.pending_tool_calls = Some(tool_calls);
                            }
                            Some(FinishReason::ToolCalls)
                        }
                        "function_call" => Some(FinishReason::FunctionCall),
                        _ => None,
                    };
                }
                
                // Extract usage if present
                if let Some(usage) = json.get("usage") {
                    normalized.usage = Some(Usage {
                        prompt_tokens: usage.get("prompt_tokens").and_then(|v| v.as_u64()).map(|v| v as u32).unwrap_or(0),
                        completion_tokens: usage.get("completion_tokens").and_then(|v| v.as_u64()).map(|v| v as u32).unwrap_or(0),
                        total_tokens: usage.get("total_tokens").and_then(|v| v.as_u64()).map(|v| v as u32).unwrap_or(0),
                        prompt_tokens_details: None,
                        completion_tokens_details: None,
                    });
                }
                
                return Ok(Some(normalized));
            }
        }
        
        Ok(None)
    }
}

/// Tool call builder for incremental streaming
pub struct ToolCallBuilder {
    active_calls: HashMap<String, PartialToolCall>,
}

#[derive(Debug, Clone)]
struct PartialToolCall {
    id: String,
    function_name: String,
    arguments: String,
}

impl ToolCallBuilder {
    pub fn new() -> Self {
        Self {
            active_calls: HashMap::new(),
        }
    }
    
    pub fn add_delta(&mut self, tool_calls_delta: &Value) -> Result<()> {
        if let Some(calls_array) = tool_calls_delta.as_array() {
            for call_delta in calls_array {
                if let Some(index) = call_delta.get("index").and_then(|i| i.as_u64()) {
                    let index_str = index.to_string();
                    
                    // Get or create partial call
                    let partial_call = self.active_calls.entry(index_str.clone()).or_insert_with(|| {
                        PartialToolCall {
                            id: String::new(),
                            function_name: String::new(),
                            arguments: String::new(),
                        }
                    });
                    
                    // Update ID
                    if let Some(id) = call_delta.get("id").and_then(|i| i.as_str()) {
                        partial_call.id = id.to_string();
                    }
                    
                    // Update function details
                    if let Some(function) = call_delta.get("function") {
                        if let Some(name) = function.get("name").and_then(|n| n.as_str()) {
                            partial_call.function_name = name.to_string();
                        }
                        
                        if let Some(args) = function.get("arguments").and_then(|a| a.as_str()) {
                            partial_call.arguments.push_str(args);
                        }
                    }
                }
            }
        }
        
        Ok(())
    }
    
    pub fn finalize(&mut self) -> Option<Vec<UnifiedToolCall>> {
        if self.active_calls.is_empty() {
            return None;
        }
        
        let mut tool_calls = Vec::new();
        
        for (_, partial) in self.active_calls.drain() {
            if !partial.id.is_empty() && !partial.function_name.is_empty() {
                let arguments = serde_json::from_str(&partial.arguments)
                    .unwrap_or_else(|_| serde_json::Value::String(partial.arguments));
                
                tool_calls.push(UnifiedToolCall {
                    id: partial.id,
                    function_name: partial.function_name,
                    arguments,
                    metadata: HashMap::new(),
                });
            }
        }
        
        if tool_calls.is_empty() {
            None
        } else {
            Some(tool_calls)
        }
    }
}

/// Anthropic streaming processor (buffers until complete tool_use blocks)
pub struct AnthropicStreamProcessor {
    behavior: StreamingBehavior,
    text_buffer: String,
    tool_use_buffer: Option<Value>,
    line_buffer: String,
    pending_tool_calls: Option<Vec<UnifiedToolCall>>,
}

impl AnthropicStreamProcessor {
    pub fn new() -> Self {
        Self {
            behavior: StreamingBehavior::CompleteBlocks { 
                buffer_until_complete: true 
            },
            text_buffer: String::new(),
            tool_use_buffer: None,
            line_buffer: String::new(),
            pending_tool_calls: None,
        }
    }
}

#[async_trait]
impl StreamProcessor for AnthropicStreamProcessor {
    async fn process_chunk(&mut self, chunk: &[u8]) -> Result<Vec<NormalizedChunk>> {
        let chunk_str = String::from_utf8_lossy(chunk);
        self.line_buffer.push_str(&chunk_str);
        
        let mut results = Vec::new();
        
        while let Some(line_end) = self.line_buffer.find('\n') {
            let line = self.line_buffer[..line_end].trim().to_string();
            self.line_buffer.drain(..line_end + 1);
            
            if line.is_empty() {
                continue;
            }
            
            if let Some(normalized) = self.process_line(&line).await? {
                results.push(normalized);
            }
        }
        
        Ok(results)
    }
    
    async fn finalize(&mut self) -> Result<Vec<NormalizedChunk>> {
        let mut results = Vec::new();
        
        // Emit any buffered text content
        if !self.text_buffer.is_empty() {
            results.push(NormalizedChunk {
                content: Some(self.text_buffer.clone()),
                tool_calls: None,
                finish_reason: None,
                usage: None,
                is_final: false,
            });
            self.text_buffer.clear();
        }
        
        // Emit any complete tool use
        if let Some(tool_use) = self.tool_use_buffer.take() {
            if let Some(tool_calls) = self.convert_tool_use_to_unified(&tool_use)? {
                self.pending_tool_calls = Some(tool_calls);
            }
        }
        
        Ok(results)
    }
    
    fn get_behavior(&self) -> &StreamingBehavior {
        &self.behavior
    }
    
    fn has_pending_tool_calls(&self) -> bool {
        self.pending_tool_calls.is_some()
    }
    
    fn get_pending_tool_calls(&mut self) -> Option<Vec<UnifiedToolCall>> {
        self.pending_tool_calls.take()
    }
}

impl AnthropicStreamProcessor {
    async fn process_line(&mut self, line: &str) -> Result<Option<NormalizedChunk>> {
        // Handle SSE format
        let json_str = if line.starts_with("data: ") {
            line.strip_prefix("data: ").unwrap()
        } else {
            line
        };
        
        let json: Value = serde_json::from_str(json_str)?;
        
        match json.get("type").and_then(|t| t.as_str()) {
            Some("content_block_delta") => {
                // Buffer text content
                if let Some(delta) = json.get("delta") {
                    if let Some(text) = delta.get("text").and_then(|t| t.as_str()) {
                        self.text_buffer.push_str(text);
                        
                        // Emit incremental text
                        return Ok(Some(NormalizedChunk {
                            content: Some(text.to_string()),
                            tool_calls: None,
                            finish_reason: None,
                            usage: None,
                            is_final: false,
                        }));
                    }
                }
            }
            Some("content_block_start") => {
                if let Some(content_block) = json.get("content_block") {
                    if content_block.get("type").and_then(|t| t.as_str()) == Some("tool_use") {
                        // Start buffering tool use
                        self.tool_use_buffer = Some(content_block.clone());
                    }
                }
            }
            Some("content_block_stop") => {
                // Tool use complete, convert and emit
                if let Some(tool_use) = self.tool_use_buffer.take() {
                    if let Some(tool_calls) = self.convert_tool_use_to_unified(&tool_use)? {
                        self.pending_tool_calls = Some(tool_calls);
                        
                        return Ok(Some(NormalizedChunk {
                            content: None,
                            tool_calls: None,
                            finish_reason: Some(FinishReason::ToolCalls),
                            usage: None,
                            is_final: false,
                        }));
                    }
                }
            }
            Some("message_stop") => {
                return Ok(Some(NormalizedChunk {
                    content: None,
                    tool_calls: None,
                    finish_reason: Some(FinishReason::Stop),
                    usage: None,
                    is_final: true,
                }));
            }
            _ => {}
        }
        
        Ok(None)
    }
    
    fn convert_tool_use_to_unified(&self, tool_use: &Value) -> Result<Option<Vec<UnifiedToolCall>>> {
        if let (Some(id), Some(name), Some(input)) = (
            tool_use.get("id").and_then(|i| i.as_str()),
            tool_use.get("name").and_then(|n| n.as_str()),
            tool_use.get("input"),
        ) {
            let unified_call = UnifiedToolCall {
                id: id.to_string(),
                function_name: name.to_string(),
                arguments: input.clone(),
                metadata: HashMap::new(),
            };
            
            Ok(Some(vec![unified_call]))
        } else {
            Ok(None)
        }
    }
}

/// Bedrock streaming processor (JSON lines format)
pub struct BedrockStreamProcessor {
    behavior: StreamingBehavior,
    line_buffer: String,
    pending_tool_calls: Option<Vec<UnifiedToolCall>>,
}

impl BedrockStreamProcessor {
    pub fn new() -> Self {
        Self {
            behavior: StreamingBehavior::JsonLines { 
                parse_each_line: true 
            },
            line_buffer: String::new(),
            pending_tool_calls: None,
        }
    }
}

#[async_trait]
impl StreamProcessor for BedrockStreamProcessor {
    async fn process_chunk(&mut self, chunk: &[u8]) -> Result<Vec<NormalizedChunk>> {
        let chunk_str = String::from_utf8_lossy(chunk);
        self.line_buffer.push_str(&chunk_str);
        
        let mut results = Vec::new();
        
        while let Some(line_end) = self.line_buffer.find('\n') {
            let line = self.line_buffer[..line_end].trim().to_string();
            self.line_buffer.drain(..line_end + 1);
            
            if line.is_empty() {
                continue;
            }
            
            if let Some(normalized) = self.process_json_line(&line).await? {
                results.push(normalized);
            }
        }
        
        Ok(results)
    }
    
    async fn finalize(&mut self) -> Result<Vec<NormalizedChunk>> {
        Ok(Vec::new())
    }
    
    fn get_behavior(&self) -> &StreamingBehavior {
        &self.behavior
    }
    
    fn has_pending_tool_calls(&self) -> bool {
        self.pending_tool_calls.is_some()
    }
    
    fn get_pending_tool_calls(&mut self) -> Option<Vec<UnifiedToolCall>> {
        self.pending_tool_calls.take()
    }
}

impl BedrockStreamProcessor {
    async fn process_json_line(&mut self, line: &str) -> Result<Option<NormalizedChunk>> {
        let json: Value = serde_json::from_str(line)?;
        
        // Extract text content
        if let Some(delta) = json.get("delta") {
            if let Some(text) = delta.get("text").and_then(|t| t.as_str()) {
                return Ok(Some(NormalizedChunk {
                    content: Some(text.to_string()),
                    tool_calls: None,
                    finish_reason: None,
                    usage: None,
                    is_final: false,
                }));
            }
        }
        
        // Check for tool use
        if let Some(content_block) = json.get("contentBlock") {
            if let Some(tool_use) = content_block.get("toolUse") {
                if let Some(tool_calls) = self.convert_bedrock_tool_use(tool_use)? {
                    self.pending_tool_calls = Some(tool_calls);
                    
                    return Ok(Some(NormalizedChunk {
                        content: None,
                        tool_calls: None,
                        finish_reason: Some(FinishReason::ToolCalls),
                        usage: None,
                        is_final: false,
                    }));
                }
            }
        }
        
        // Check for completion
        if let Some(stop_reason) = json.get("stopReason").and_then(|s| s.as_str()) {
            let finish_reason = match stop_reason {
                "end_turn" => FinishReason::Stop,
                "tool_use" => FinishReason::ToolCalls,
                "max_tokens" => FinishReason::Length,
                _ => FinishReason::Stop,
            };
            
            return Ok(Some(NormalizedChunk {
                content: None,
                tool_calls: None,
                finish_reason: Some(finish_reason),
                usage: None,
                is_final: true,
            }));
        }
        
        Ok(None)
    }
    
    fn convert_bedrock_tool_use(&self, tool_use: &Value) -> Result<Option<Vec<UnifiedToolCall>>> {
        if let (Some(tool_use_id), Some(name), Some(input)) = (
            tool_use.get("toolUseId").and_then(|i| i.as_str()),
            tool_use.get("name").and_then(|n| n.as_str()),
            tool_use.get("input"),
        ) {
            let unified_call = UnifiedToolCall {
                id: tool_use_id.to_string(),
                function_name: name.to_string(),
                arguments: input.clone(),
                metadata: HashMap::new(),
            };
            
            Ok(Some(vec![unified_call]))
        } else {
            Ok(None)
        }
    }
}

/// Factory for creating provider-specific stream processors
pub struct StreamProcessorFactory;

impl StreamProcessorFactory {
    pub fn create_processor(provider: &str) -> Box<dyn StreamProcessor> {
        match provider.to_lowercase().as_str() {
            "openai" | "groq" | "together" | "fireworks" => {
                Box::new(OpenAIStreamProcessor::new())
            }
            "anthropic" | "claude" => {
                Box::new(AnthropicStreamProcessor::new())
            }
            "bedrock" | "aws" => {
                Box::new(BedrockStreamProcessor::new())
            }
            _ => {
                // Default to OpenAI format
                Box::new(OpenAIStreamProcessor::new())
            }
        }
    }
}