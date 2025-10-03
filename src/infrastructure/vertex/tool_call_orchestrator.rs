// src/infrastructure/vertex/tool_call_orchestrator.rs
//
// Tool Call Orchestrator for managing OpenAI vs Claude semantic differences
// Executes tool calls in parallel internally but presents them sequentially for OpenAI compatibility

use anyhow::Result;
use serde_json::Value;
use std::collections::HashMap;
use tokio::time::{timeout, Duration};
use tracing::{debug, info, warn};

use crate::infrastructure::vertex::streaming_buffer::ProcessedChunk;
use crate::models::common::ToolDefinition;
use crate::shared::types::{ToolCall, FunctionCall};

/// Represents a tool call execution result
#[derive(Debug, Clone)]
pub struct ToolCallExecution {
    pub id: String,
    pub name: String,
    pub input: Value,
    pub result: Option<String>,
    pub error: Option<String>,
    pub execution_time_ms: u64,
}

/// Manages the execution and presentation of tool calls
/// Handles parallel execution internally while maintaining OpenAI sequential semantics
pub struct ToolCallOrchestrator {
    /// Available tool definitions
    tool_definitions: Vec<ToolDefinition>,
    
    /// Pending tool calls waiting for execution
    pending_calls: Vec<ProcessedChunk>,
    
    /// Completed tool call executions
    completed_executions: HashMap<String, ToolCallExecution>,
    
    /// Current batch being processed
    current_batch: Option<ToolCallBatch>,
}

/// Represents a batch of tool calls that can be executed in parallel
#[derive(Debug)]
struct ToolCallBatch {
    #[allow(dead_code)] // Used for debugging and potential future batch management
    calls: Vec<ProcessedChunk>,
    started_at: std::time::Instant,
    batch_id: String,
}

impl ToolCallOrchestrator {
    /// Create a new orchestrator with tool definitions
    pub fn new(tool_definitions: Vec<ToolDefinition>) -> Self {
        Self {
            tool_definitions,
            pending_calls: Vec::new(),
            completed_executions: HashMap::new(),
            current_batch: None,
        }
    }

    /// Add a tool call to the orchestrator
    pub fn add_tool_call(&mut self, tool_call: ProcessedChunk) -> Result<()> {
        if let ProcessedChunk::ToolUse { id, name, .. } = &tool_call {
            debug!("Adding tool call to orchestrator: {} ({})", name, id);
            self.pending_calls.push(tool_call);
        }
        Ok(())
    }

    /// Check if there are pending tool calls
    pub fn has_pending_calls(&self) -> bool {
        !self.pending_calls.is_empty() || self.current_batch.is_some()
    }

    /// Start executing pending tool calls in parallel
    /// This executes all pending calls simultaneously for performance
    pub async fn execute_pending_calls(&mut self) -> Result<Vec<ToolCallExecution>> {
        if self.pending_calls.is_empty() {
            return Ok(Vec::new());
        }

        let batch_id = uuid::Uuid::new_v4().to_string();
        let calls = std::mem::take(&mut self.pending_calls);
        
        info!("Starting parallel execution of {} tool calls in batch {}", calls.len(), batch_id);
        
        let batch = ToolCallBatch {
            calls: calls.clone(),
            started_at: std::time::Instant::now(),
            batch_id: batch_id.clone(),
        };
        
        self.current_batch = Some(batch);

        // Execute all tool calls in parallel
        let mut execution_futures = Vec::new();
        
        for tool_call in calls {
            if let ProcessedChunk::ToolUse { id, name, input } = tool_call {
                let execution_future = self.execute_single_tool_call(id, name, input);
                execution_futures.push(execution_future);
            }
        }

        // Wait for all executions to complete with timeout
        let timeout_duration = Duration::from_secs(30); // 30 second timeout
        let results = match timeout(timeout_duration, futures::future::join_all(execution_futures)).await {
            Ok(results) => results,
            Err(_) => {
                warn!("Tool call batch {} timed out after 30 seconds", batch_id);
                return Err(anyhow::anyhow!("Tool call execution timed out"));
            }
        };

        let mut executions = Vec::new();
        for result in results {
            match result {
                Ok(execution) => {
                    self.completed_executions.insert(execution.id.clone(), execution.clone());
                    executions.push(execution);
                }
                Err(e) => {
                    warn!("Tool call execution failed: {}", e);
                }
            }
        }

        let total_time = self.current_batch.as_ref().unwrap().started_at.elapsed();
        info!("Completed parallel execution of {} tool calls in {:?}", executions.len(), total_time);
        
        self.current_batch = None;
        Ok(executions)
    }

    /// Execute a single tool call (mock implementation for now)
    async fn execute_single_tool_call(
        &self,
        id: String,
        name: String,
        input: Value,
    ) -> Result<ToolCallExecution> {
        let start_time = std::time::Instant::now();
        
        debug!("Executing tool call: {} with input: {:?}", name, input);
        
        // Mock execution - in real implementation, this would call the actual tool
        // For now, simulate some processing time and return a mock result
        tokio::time::sleep(Duration::from_millis(100 + (rand::random::<u64>() % 500))).await;
        
        let mock_result = match name.as_str() {
            "get_weather" => {
                if let Some(location) = input.get("location").and_then(|l| l.as_str()) {
                    format!("The weather in {} is sunny with a temperature of 22°C", location)
                } else {
                    "Weather information unavailable - location not specified".to_string()
                }
            }
            "tavily_search" => {
                if let Some(query) = input.get("query").and_then(|q| q.as_str()) {
                    format!("Search results for '{}': Found 5 relevant articles about this topic.", query)
                } else {
                    "Search failed - no query provided".to_string()
                }
            }
            _ => {
                format!("Tool '{}' executed successfully with input: {}", name, input)
            }
        };

        let execution_time = start_time.elapsed();
        
        Ok(ToolCallExecution {
            id,
            name,
            input,
            result: Some(mock_result),
            error: None,
            execution_time_ms: execution_time.as_millis() as u64,
        })
    }

    /// Convert tool call executions to OpenAI format for sequential presentation
    /// This maintains OpenAI compatibility by presenting one tool call at a time
    pub fn convert_to_openai_sequential(&self, executions: &[ToolCallExecution]) -> Vec<ToolCall> {
        executions.iter().map(|execution| {
            let arguments = serde_json::to_string(&execution.input).unwrap_or("{}".to_string());
            
            ToolCall {
                id: execution.id.clone(),
                tool_type: "function".to_string(),
                function: FunctionCall {
                    name: execution.name.clone(),
                    arguments,
                },
            }
        }).collect()
    }

    /// Get the next tool call result for sequential processing
    /// This allows the OpenAI client to process one result at a time
    pub fn get_next_sequential_result(&mut self) -> Option<ToolCallExecution> {
        // For now, return the first completed execution
        // In a full implementation, this would maintain proper ordering
        if let Some((_, execution)) = self.completed_executions.iter().next() {
            let execution = execution.clone();
            self.completed_executions.remove(&execution.id);
            Some(execution)
        } else {
            None
        }
    }

    /// Check if a tool is defined in the available tools
    pub fn is_tool_defined(&self, tool_name: &str) -> bool {
        self.tool_definitions.iter().any(|tool| tool.function.name == tool_name)
    }

    /// Get tool definition by name
    pub fn get_tool_definition(&self, tool_name: &str) -> Option<&ToolDefinition> {
        self.tool_definitions.iter().find(|tool| tool.function.name == tool_name)
    }

    /// Get execution statistics
    pub fn get_execution_stats(&self) -> ExecutionStats {
        ExecutionStats {
            pending_calls: self.pending_calls.len(),
            completed_executions: self.completed_executions.len(),
            is_batch_running: self.current_batch.is_some(),
            batch_id: self.current_batch.as_ref().map(|b| b.batch_id.clone()),
        }
    }

    /// Reset the orchestrator state
    pub fn reset(&mut self) {
        self.pending_calls.clear();
        self.completed_executions.clear();
        self.current_batch = None;
    }
}

/// Execution statistics for monitoring
#[derive(Debug)]
pub struct ExecutionStats {
    pub pending_calls: usize,
    pub completed_executions: usize,
    pub is_batch_running: bool,
    pub batch_id: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn create_test_tool_definition() -> ToolDefinition {
        ToolDefinition {
            tool_type: "function".to_string(),
            function: crate::models::common::FunctionDefinition {
                name: "get_weather".to_string(),
                description: Some("Get weather information".to_string()),
                parameters: Some(json!({
                    "type": "object",
                    "properties": {
                        "location": {"type": "string"}
                    },
                    "required": ["location"]
                })),
            },
        }
    }

    #[tokio::test]
    async fn test_orchestrator_creation() {
        let tools = vec![create_test_tool_definition()];
        let orchestrator = ToolCallOrchestrator::new(tools);
        
        assert!(!orchestrator.has_pending_calls());
        assert!(orchestrator.is_tool_defined("get_weather"));
        assert!(!orchestrator.is_tool_defined("unknown_tool"));
    }

    #[tokio::test]
    async fn test_add_tool_call() {
        let tools = vec![create_test_tool_definition()];
        let mut orchestrator = ToolCallOrchestrator::new(tools);
        
        let tool_call = ProcessedChunk::ToolUse {
            id: "call_123".to_string(),
            name: "get_weather".to_string(),
            input: json!({"location": "San Francisco"}),
        };
        
        orchestrator.add_tool_call(tool_call).unwrap();
        assert!(orchestrator.has_pending_calls());
    }

    #[tokio::test]
    async fn test_parallel_execution() {
        let tools = vec![create_test_tool_definition()];
        let mut orchestrator = ToolCallOrchestrator::new(tools);
        
        // Add multiple tool calls
        for i in 0..3 {
            let tool_call = ProcessedChunk::ToolUse {
                id: format!("call_{}", i),
                name: "get_weather".to_string(),
                input: json!({"location": format!("City_{}", i)}),
            };
            orchestrator.add_tool_call(tool_call).unwrap();
        }
        
        let executions = orchestrator.execute_pending_calls().await.unwrap();
        assert_eq!(executions.len(), 3);
        
        // Verify all executions completed
        for execution in &executions {
            assert!(execution.result.is_some());
            assert!(execution.error.is_none());
            assert!(execution.execution_time_ms > 0);
        }
    }

    #[test]
    fn test_openai_conversion() {
        let tools = vec![create_test_tool_definition()];
        let orchestrator = ToolCallOrchestrator::new(tools);
        
        let executions = vec![
            ToolCallExecution {
                id: "call_123".to_string(),
                name: "get_weather".to_string(),
                input: json!({"location": "San Francisco"}),
                result: Some("Sunny, 22°C".to_string()),
                error: None,
                execution_time_ms: 150,
            }
        ];
        
        let openai_calls = orchestrator.convert_to_openai_sequential(&executions);
        assert_eq!(openai_calls.len(), 1);
        assert_eq!(openai_calls[0].id, "call_123");
        assert_eq!(openai_calls[0].function.name, "get_weather");
    }
}