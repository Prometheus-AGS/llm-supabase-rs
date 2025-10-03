// src/infrastructure/common/adaptive_formatter.rs
//
// Adaptive response formatter that switches between OpenAI legacy and modern formats
// Based on detected client capabilities

use anyhow::Result;
use serde_json::{json, Value};
use std::collections::VecDeque;
use tracing::{debug, info};

use crate::infrastructure::vertex::ToolCallExecution;
use crate::models::{
    common::{ChatMessage, MessageRole, FinishReason},
    response::{ChatCompletionResponse, ChatCompletionChunk, ChatCompletionChoice, ChatCompletionChunkChoice},
};
use crate::shared::types::{ToolCall, FunctionCall};
use super::client_detection::{ClientCapabilities, ToolFormat, ToolChoiceStrategy, StreamingBehavior};

/// Different response format types
#[derive(Debug, Clone)]
pub enum ResponseFormat {
    /// Text-only response (no tools)
    TextOnly(String),
    
    /// Legacy function_call format (single tool)
    LegacyFunctionCall {
        function_call: FunctionCall,
        content: String,
    },
    
    /// Modern tool_calls array format (multiple tools)
    ModernToolCalls {
        tool_calls: Vec<ToolCall>,
        content: String,
    },
}

/// Queue system for managing multi-tool responses for legacy clients
#[derive(Debug)]
pub struct ToolCallQueue {
    /// Pending tool calls by conversation ID
    pending_calls: std::collections::HashMap<String, VecDeque<ToolCallExecution>>,
    
    /// Conversation contexts
    contexts: std::collections::HashMap<String, ConversationContext>,
}

/// Context for managing conversation state
#[derive(Debug, Clone)]
pub struct ConversationContext {
    pub request_id: String,
    pub model: String,
    pub original_tool_count: usize,
    pub completed_tools: usize,
}

/// Adaptive response formatter that handles all OpenAI client types
pub struct AdaptiveResponseFormatter {
    /// Detected client capabilities
    capabilities: ClientCapabilities,
    
    /// Tool choice strategy from request
    tool_choice_strategy: ToolChoiceStrategy,
    
    /// Queue for managing legacy client multi-tool scenarios
    tool_queue: ToolCallQueue,
}

impl ToolCallQueue {
    pub fn new() -> Self {
        Self {
            pending_calls: std::collections::HashMap::new(),
            contexts: std::collections::HashMap::new(),
        }
    }

    /// Queue remaining tool calls for legacy sequential processing
    pub fn queue_remaining_calls(
        &mut self,
        conversation_id: &str,
        executions: Vec<ToolCallExecution>,
        context: ConversationContext,
    ) {
        if executions.len() > 1 {
            let mut queue = VecDeque::from(executions);
            queue.pop_front(); // Remove first call (already returned)
            
            self.pending_calls.insert(conversation_id.to_string(), queue);
            self.contexts.insert(conversation_id.to_string(), context);
            
            info!("Queued {} remaining tool calls for conversation {}", 
                  self.pending_calls.get(conversation_id).unwrap().len(), 
                  conversation_id);
        }
    }

    /// Get next queued tool call for a conversation
    pub fn get_next_call(&mut self, conversation_id: &str) -> Option<ToolCallExecution> {
        if let Some(queue) = self.pending_calls.get_mut(conversation_id) {
            let next_call = queue.pop_front();
            
            // Clean up empty queues
            if queue.is_empty() {
                self.pending_calls.remove(conversation_id);
                self.contexts.remove(conversation_id);
            }
            
            next_call
        } else {
            None
        }
    }

    /// Check if there are pending calls for a conversation
    pub fn has_pending_calls(&self, conversation_id: &str) -> bool {
        self.pending_calls.get(conversation_id).map_or(false, |q| !q.is_empty())
    }
}

impl AdaptiveResponseFormatter {
    /// Create new formatter with detected capabilities
    pub fn new(
        capabilities: ClientCapabilities,
        tool_choice_strategy: ToolChoiceStrategy,
    ) -> Self {
        Self {
            capabilities,
            tool_choice_strategy,
            tool_queue: ToolCallQueue::new(),
        }
    }

    /// Format response based on client capabilities and tool executions
    pub fn format_response(
        &mut self,
        tool_executions: &[ToolCallExecution],
        text_content: Option<String>,
        request_id: &str,
        model: &str,
    ) -> Result<ChatCompletionResponse> {
        let content = text_content.unwrap_or_default();
        
        // Handle tool_choice strategy first
        match self.tool_choice_strategy {
            ToolChoiceStrategy::None => {
                debug!("tool_choice is 'none' - returning text-only response");
                return self.create_text_only_response(content, request_id, model);
            }
            ToolChoiceStrategy::Required => {
                if tool_executions.is_empty() {
                    return Err(anyhow::anyhow!("tool_choice is 'required' but no tools were called"));
                }
            }
            ToolChoiceStrategy::Specific { ref name } => {
                if !tool_executions.iter().any(|e| e.name == *name) {
                    return Err(anyhow::anyhow!("tool_choice requires '{}' but it was not called", name));
                }
            }
            ToolChoiceStrategy::Auto => {
                // Continue with normal processing
            }
        }

        // If no tool executions, return text response
        if tool_executions.is_empty() {
            return self.create_text_only_response(content, request_id, model);
        }

        // Format based on detected client capabilities
        match self.capabilities.tool_format {
            ToolFormat::Legacy => {
                self.format_legacy_response(tool_executions, content, request_id, model)
            }
            ToolFormat::Modern => {
                self.format_modern_response(tool_executions, content, request_id, model)
            }
            ToolFormat::Adaptive => {
                self.format_adaptive_response(tool_executions, content, request_id, model)
            }
        }
    }

    /// Format streaming chunk based on client capabilities
    pub fn format_streaming_chunk(
        &self,
        tool_executions: &[ToolCallExecution],
        text_content: Option<String>,
        request_id: &str,
        model: &str,
        is_final: bool,
    ) -> Result<ChatCompletionChunk> {
        let content = text_content.unwrap_or_default();
        
        if !tool_executions.is_empty() {
            // Tool call chunk
            match self.capabilities.tool_format {
                ToolFormat::Legacy => {
                    self.create_legacy_tool_chunk(&tool_executions[0], request_id, model)
                }
                ToolFormat::Modern => {
                    self.create_modern_tool_chunk(tool_executions, request_id, model)
                }
                ToolFormat::Adaptive => {
                    if tool_executions.len() == 1 {
                        self.create_legacy_tool_chunk(&tool_executions[0], request_id, model)
                    } else {
                        self.create_modern_tool_chunk(tool_executions, request_id, model)
                    }
                }
            }
        } else if !content.is_empty() {
            // Content chunk
            self.create_content_chunk(content, request_id, model)
        } else if is_final {
            // Final chunk
            self.create_final_chunk(request_id, model)
        } else {
            // Empty chunk - shouldn't happen
            Err(anyhow::anyhow!("Cannot create chunk with no content or tool calls"))
        }
    }

    /// Create text-only response
    fn create_text_only_response(
        &self,
        content: String,
        request_id: &str,
        model: &str,
    ) -> Result<ChatCompletionResponse> {
        let message = ChatMessage {
            role: MessageRole::Assistant,
            content,
            name: None,
            function_call: None,
            tool_calls: None,
            tool_call_id: None,
        };

        let choice = ChatCompletionChoice {
            index: 0,
            message,
            logprobs: None,
            finish_reason: FinishReason::Stop,
        };

        Ok(ChatCompletionResponse::new(
            request_id.to_string(),
            model.to_string(),
            vec![choice],
            Default::default(),
        ))
    }

    /// Format response for legacy clients (single function_call)
    fn format_legacy_response(
        &mut self,
        tool_executions: &[ToolCallExecution],
        content: String,
        request_id: &str,
        model: &str,
    ) -> Result<ChatCompletionResponse> {
        let first_execution = &tool_executions[0];
        
        // Queue remaining calls if any
        if tool_executions.len() > 1 {
            let context = ConversationContext {
                request_id: request_id.to_string(),
                model: model.to_string(),
                original_tool_count: tool_executions.len(),
                completed_tools: 1,
            };
            
            self.tool_queue.queue_remaining_calls(
                request_id,
                tool_executions.to_vec(),
                context,
            );
        }

        let function_call = FunctionCall {
            name: first_execution.name.clone(),
            arguments: serde_json::to_string(&first_execution.input)?,
        };

        let message = ChatMessage {
            role: MessageRole::Assistant,
            content,
            name: None,
            function_call: Some(serde_json::to_value(function_call)?),
            tool_calls: None,
            tool_call_id: None,
        };

        let choice = ChatCompletionChoice {
            index: 0,
            message,
            logprobs: None,
            finish_reason: FinishReason::FunctionCall,
        };

        Ok(ChatCompletionResponse::new(
            request_id.to_string(),
            model.to_string(),
            vec![choice],
            Default::default(),
        ))
    }

    /// Format response for modern clients (tool_calls array)
    fn format_modern_response(
        &self,
        tool_executions: &[ToolCallExecution],
        content: String,
        request_id: &str,
        model: &str,
    ) -> Result<ChatCompletionResponse> {
        let tool_calls: Vec<Value> = tool_executions
            .iter()
            .map(|execution| {
                json!({
                    "id": execution.id,
                    "type": "function",
                    "function": {
                        "name": execution.name,
                        "arguments": serde_json::to_string(&execution.input).unwrap_or("{}".to_string())
                    }
                })
            })
            .collect();

        let message = ChatMessage {
            role: MessageRole::Assistant,
            content,
            name: None,
            function_call: None,
            tool_calls: Some(tool_calls),
            tool_call_id: None,
        };

        let choice = ChatCompletionChoice {
            index: 0,
            message,
            logprobs: None,
            finish_reason: FinishReason::ToolCalls,
        };

        Ok(ChatCompletionResponse::new(
            request_id.to_string(),
            model.to_string(),
            vec![choice],
            Default::default(),
        ))
    }

    /// Format response adaptively based on content
    fn format_adaptive_response(
        &mut self,
        tool_executions: &[ToolCallExecution],
        content: String,
        request_id: &str,
        model: &str,
    ) -> Result<ChatCompletionResponse> {
        if tool_executions.len() == 1 {
            // Single tool call - use legacy format for maximum compatibility
            self.format_legacy_response(tool_executions, content, request_id, model)
        } else {
            // Multiple tool calls - use modern format
            self.format_modern_response(tool_executions, content, request_id, model)
        }
    }

    /// Create content streaming chunk
    fn create_content_chunk(
        &self,
        content: String,
        request_id: &str,
        model: &str,
    ) -> Result<ChatCompletionChunk> {
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

        Ok(ChatCompletionChunk::new(
            request_id.to_string(),
            model.to_string(),
            vec![choice],
        ))
    }

    /// Create legacy tool call chunk (function_call)
    fn create_legacy_tool_chunk(
        &self,
        execution: &ToolCallExecution,
        request_id: &str,
        model: &str,
    ) -> Result<ChatCompletionChunk> {
        let function_call = json!({
            "name": execution.name,
            "arguments": serde_json::to_string(&execution.input)?
        });

        let delta = ChatMessage {
            role: MessageRole::Assistant,
            content: String::new(),
            name: None,
            function_call: Some(function_call),
            tool_calls: None,
            tool_call_id: None,
        };

        let choice = ChatCompletionChunkChoice {
            index: 0,
            delta,
            logprobs: None,
            finish_reason: Some(FinishReason::FunctionCall),
        };

        Ok(ChatCompletionChunk::new(
            request_id.to_string(),
            model.to_string(),
            vec![choice],
        ))
    }

    /// Create modern tool calls chunk (tool_calls array)
    fn create_modern_tool_chunk(
        &self,
        executions: &[ToolCallExecution],
        request_id: &str,
        model: &str,
    ) -> Result<ChatCompletionChunk> {
        let tool_calls: Vec<Value> = executions
            .iter()
            .map(|execution| {
                json!({
                    "id": execution.id,
                    "type": "function",
                    "function": {
                        "name": execution.name,
                        "arguments": serde_json::to_string(&execution.input).unwrap_or("{}".to_string())
                    }
                })
            })
            .collect();

        let delta = ChatMessage {
            role: MessageRole::Assistant,
            content: String::new(),
            name: None,
            function_call: None,
            tool_calls: Some(tool_calls),
            tool_call_id: None,
        };

        let choice = ChatCompletionChunkChoice {
            index: 0,
            delta,
            logprobs: None,
            finish_reason: Some(FinishReason::ToolCalls),
        };

        Ok(ChatCompletionChunk::new(
            request_id.to_string(),
            model.to_string(),
            vec![choice],
        ))
    }

    /// Create final streaming chunk
    fn create_final_chunk(
        &self,
        request_id: &str,
        model: &str,
    ) -> Result<ChatCompletionChunk> {
        let choice = ChatCompletionChunkChoice {
            index: 0,
            delta: ChatMessage::empty(),
            logprobs: None,
            finish_reason: Some(FinishReason::Stop),
        };

        Ok(ChatCompletionChunk::new(
            request_id.to_string(),
            model.to_string(),
            vec![choice],
        ))
    }

    /// Get client capabilities
    pub fn get_capabilities(&self) -> &ClientCapabilities {
        &self.capabilities
    }

    /// Get tool choice strategy
    pub fn get_tool_choice_strategy(&self) -> &ToolChoiceStrategy {
        &self.tool_choice_strategy
    }

    /// Check if streaming should stop after tool calls
    pub fn should_stop_streaming_after_tools(&self) -> bool {
        match self.capabilities.streaming_behavior {
            StreamingBehavior::LegacyStop => true,
            StreamingBehavior::ModernBatch => true,
            StreamingBehavior::Adaptive => {
                // Adaptive: stop for tool calls, continue for text
                true
            }
        }
    }

    /// Get mutable reference to tool queue
    pub fn get_tool_queue_mut(&mut self) -> &mut ToolCallQueue {
        &mut self.tool_queue
    }

    /// Get reference to tool queue
    pub fn get_tool_queue(&self) -> &ToolCallQueue {
        &self.tool_queue
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::infrastructure::common::client_detection::{ToolFormat, StreamingBehavior, ToolChoiceSupport};

    fn create_test_execution() -> ToolCallExecution {
        ToolCallExecution {
            id: "call_123".to_string(),
            name: "get_weather".to_string(),
            input: json!({"location": "San Francisco"}),
            result: Some("Sunny, 22°C".to_string()),
            error: None,
            execution_time_ms: 150,
        }
    }

    fn create_legacy_capabilities() -> ClientCapabilities {
        ClientCapabilities {
            tool_format: ToolFormat::Legacy,
            supports_parallel_execution: false,
            sdk_version: Some("openai-python/0.28.0".to_string()),
            streaming_behavior: StreamingBehavior::LegacyStop,
            tool_choice_support: ToolChoiceSupport::Basic,
        }
    }

    fn create_modern_capabilities() -> ClientCapabilities {
        ClientCapabilities {
            tool_format: ToolFormat::Modern,
            supports_parallel_execution: true,
            sdk_version: Some("openai-python/1.35.0".to_string()),
            streaming_behavior: StreamingBehavior::ModernBatch,
            tool_choice_support: ToolChoiceSupport::Full,
        }
    }

    #[test]
    fn test_legacy_formatting() {
        let capabilities = create_legacy_capabilities();
        let strategy = ToolChoiceStrategy::Auto;
        let mut formatter = AdaptiveResponseFormatter::new(capabilities, strategy);
        
        let executions = vec![create_test_execution()];
        let response = formatter.format_response(
            &executions,
            Some("I'll check the weather.".to_string()),
            "req_123",
            "claude-4-sonnet",
        ).unwrap();
        
        let choice = &response.choices[0];
        assert!(choice.message.function_call.is_some());
        assert!(choice.message.tool_calls.is_none());
        assert_eq!(choice.finish_reason, FinishReason::FunctionCall);
    }

    #[test]
    fn test_modern_formatting() {
        let capabilities = create_modern_capabilities();
        let strategy = ToolChoiceStrategy::Auto;
        let formatter = AdaptiveResponseFormatter::new(capabilities, strategy);
        
        let executions = vec![
            create_test_execution(),
            ToolCallExecution {
                id: "call_456".to_string(),
                name: "search_web".to_string(),
                input: json!({"query": "weather forecast"}),
                result: Some("Search results...".to_string()),
                error: None,
                execution_time_ms: 200,
            }
        ];
        
        let response = formatter.format_response(
            &executions,
            Some("I'll help with that.".to_string()),
            "req_123",
            "claude-4-sonnet",
        ).unwrap();
        
        let choice = &response.choices[0];
        assert!(choice.message.function_call.is_none());
        assert!(choice.message.tool_calls.is_some());
        assert_eq!(choice.message.tool_calls.as_ref().unwrap().len(), 2);
        assert_eq!(choice.finish_reason, FinishReason::ToolCalls);
    }

    #[test]
    fn test_tool_choice_none() {
        let capabilities = create_modern_capabilities();
        let strategy = ToolChoiceStrategy::None;
        let formatter = AdaptiveResponseFormatter::new(capabilities, strategy);
        
        let executions = vec![create_test_execution()]; // Should be ignored
        let response = formatter.format_response(
            &executions,
            Some("Just text response.".to_string()),
            "req_123",
            "claude-4-sonnet",
        ).unwrap();
        
        let choice = &response.choices[0];
        assert!(choice.message.function_call.is_none());
        assert!(choice.message.tool_calls.is_none());
        assert_eq!(choice.message.content, "Just text response.");
        assert_eq!(choice.finish_reason, FinishReason::Stop);
    }

    #[test]
    fn test_tool_choice_required_validation() {
        let capabilities = create_modern_capabilities();
        let strategy = ToolChoiceStrategy::Required;
        let formatter = AdaptiveResponseFormatter::new(capabilities, strategy);
        
        // Should error when no tools executed
        let result = formatter.format_response(
            &[],
            Some("No tools used.".to_string()),
            "req_123",
            "claude-4-sonnet",
        );
        
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("required"));
    }

    #[test]
    fn test_tool_queue_management() {
        let mut queue = ToolCallQueue::new();
        
        let executions = vec![
            create_test_execution(),
            ToolCallExecution {
                id: "call_456".to_string(),
                name: "search_web".to_string(),
                input: json!({"query": "test"}),
                result: Some("Result".to_string()),
                error: None,
                execution_time_ms: 100,
            }
        ];
        
        let context = ConversationContext {
            request_id: "req_123".to_string(),
            model: "claude-4-sonnet".to_string(),
            original_tool_count: 2,
            completed_tools: 1,
        };
        
        queue.queue_remaining_calls("conv_123", executions, context);
        
        assert!(queue.has_pending_calls("conv_123"));
        
        let next_call = queue.get_next_call("conv_123");
        assert!(next_call.is_some());
        assert_eq!(next_call.unwrap().name, "search_web");
        
        // Should be empty now
        assert!(!queue.has_pending_calls("conv_123"));
    }
}