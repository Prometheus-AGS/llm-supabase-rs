// src/infrastructure/common/tools.rs
//
// Common tool calling abstractions for all providers
// Handles the differences between OpenAI and Claude/Anthropic tool calling formats

use anyhow::Result;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

use crate::models::common::{ToolDefinition};
use crate::shared::types::{ToolCall, FunctionCall};

/// Unified tool calling interface that abstracts provider differences
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnifiedToolCall {
    /// Unique identifier for this tool call
    pub id: String,
    
    /// Name of the function/tool to call
    pub function_name: String,
    
    /// Arguments to pass to the function (as JSON)
    pub arguments: Value,
    
    /// Provider-specific metadata
    pub metadata: HashMap<String, Value>,
}

/// Tool call result from the calling application
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCallResult {
    /// ID of the tool call this result corresponds to
    pub tool_call_id: String,
    
    /// Result content (success or error)
    pub content: String,
    
    /// Whether the call was successful
    pub success: bool,
    
    /// Optional error details
    pub error: Option<String>,
}

/// Tool calling state for managing multi-turn conversations
#[derive(Debug, Clone)]
pub enum ToolCallState {
    /// No tool calls in progress
    None,
    
    /// Tool calls have been made, waiting for results
    WaitingForResults {
        tool_calls: Vec<UnifiedToolCall>,
        request_id: String,
    },
    
    /// Tool call results received, ready to continue
    ResultsReceived {
        results: Vec<ToolCallResult>,
        request_id: String,
    },
}

/// Trait for converting between provider-specific and unified tool formats
pub trait ToolCallConverter {
    /// Convert OpenAI tools to provider-specific format
    fn openai_tools_to_provider(&self, tools: &[ToolDefinition]) -> Result<Value>;
    
    /// Convert provider tool calls to unified format
    fn provider_tool_calls_to_unified(&self, provider_data: &Value) -> Result<Vec<UnifiedToolCall>>;
    
    /// Convert unified tool calls to OpenAI format
    fn unified_to_openai_tool_calls(&self, unified_calls: &[UnifiedToolCall]) -> Vec<ToolCall>;
    
    /// Convert tool call results to provider format for continuation
    fn tool_results_to_provider_messages(&self, results: &[ToolCallResult]) -> Result<Value>;
}

/// OpenAI tool call converter (pass-through since we use OpenAI as the standard)
#[derive(Debug, Clone)]
pub struct OpenAIToolConverter;

impl ToolCallConverter for OpenAIToolConverter {
    fn openai_tools_to_provider(&self, tools: &[ToolDefinition]) -> Result<Value> {
        // OpenAI format is our standard, so pass through
        Ok(serde_json::to_value(tools)?)
    }
    
    fn provider_tool_calls_to_unified(&self, provider_data: &Value) -> Result<Vec<UnifiedToolCall>> {
        let tool_calls: Vec<ToolCall> = serde_json::from_value(provider_data.clone())?;
        
        let unified_calls = tool_calls.into_iter().map(|tc| {
            let arguments = serde_json::from_str(&tc.function.arguments)
                .unwrap_or_else(|_| serde_json::Value::String(tc.function.arguments.clone()));
            
            UnifiedToolCall {
                id: tc.id,
                function_name: tc.function.name,
                arguments,
                metadata: HashMap::new(),
            }
        }).collect();
        
        Ok(unified_calls)
    }
    
    fn unified_to_openai_tool_calls(&self, unified_calls: &[UnifiedToolCall]) -> Vec<ToolCall> {
        unified_calls.iter().map(|uc| {
            let arguments_str = if uc.arguments.is_string() {
                uc.arguments.as_str().unwrap_or("{}").to_string()
            } else {
                serde_json::to_string(&uc.arguments).unwrap_or("{}".to_string())
            };
            
            ToolCall {
                id: uc.id.clone(),
                tool_type: "function".to_string(),
                function: FunctionCall {
                    name: uc.function_name.clone(),
                    arguments: arguments_str,
                },
            }
        }).collect()
    }
    
    fn tool_results_to_provider_messages(&self, results: &[ToolCallResult]) -> Result<Value> {
        let messages: Vec<Value> = results.iter().map(|result| {
            serde_json::json!({
                "role": "tool",
                "tool_call_id": result.tool_call_id,
                "content": result.content
            })
        }).collect();
        
        Ok(serde_json::to_value(messages)?)
    }
}

/// Claude/Anthropic tool call converter
pub struct ClaudeToolConverter;

impl ToolCallConverter for ClaudeToolConverter {
    fn openai_tools_to_provider(&self, tools: &[ToolDefinition]) -> Result<Value> {
        // Convert OpenAI tools format to Claude tools format
        let claude_tools: Vec<Value> = tools.iter().map(|tool| {
            serde_json::json!({
                "name": tool.function.name,
                "description": tool.function.description,
                "input_schema": tool.function.parameters
            })
        }).collect();
        
        Ok(serde_json::to_value(claude_tools)?)
    }
    
    fn provider_tool_calls_to_unified(&self, provider_data: &Value) -> Result<Vec<UnifiedToolCall>> {
        // Claude returns tool use in content blocks
        let mut unified_calls = Vec::new();
        
        if let Some(content_array) = provider_data.get("content").and_then(|c| c.as_array()) {
            for content_block in content_array {
                if let Some(tool_use) = content_block.get("tool_use") {
                    if let (Some(id), Some(name), Some(input)) = (
                        tool_use.get("id").and_then(|v| v.as_str()),
                        tool_use.get("name").and_then(|v| v.as_str()),
                        tool_use.get("input")
                    ) {
                        unified_calls.push(UnifiedToolCall {
                            id: id.to_string(),
                            function_name: name.to_string(),
                            arguments: input.clone(),
                            metadata: HashMap::new(),
                        });
                    }
                }
            }
        }
        
        Ok(unified_calls)
    }
    
    fn unified_to_openai_tool_calls(&self, unified_calls: &[UnifiedToolCall]) -> Vec<ToolCall> {
        // Convert unified format to OpenAI format
        unified_calls.iter().map(|uc| {
            let arguments_str = if uc.arguments.is_string() {
                uc.arguments.as_str().unwrap_or("{}").to_string()
            } else {
                serde_json::to_string(&uc.arguments).unwrap_or("{}".to_string())
            };
            
            ToolCall {
                id: uc.id.clone(),
                tool_type: "function".to_string(),
                function: FunctionCall {
                    name: uc.function_name.clone(),
                    arguments: arguments_str,
                },
            }
        }).collect()
    }
    
    fn tool_results_to_provider_messages(&self, results: &[ToolCallResult]) -> Result<Value> {
        // Claude expects tool results in a specific format
        let mut content_blocks = Vec::new();
        
        for result in results {
            content_blocks.push(serde_json::json!({
                "type": "tool_result",
                "tool_use_id": result.tool_call_id,
                "content": result.content,
                "is_error": !result.success
            }));
        }
        
        Ok(serde_json::json!({
            "role": "user",
            "content": content_blocks
        }))
    }
}

/// Tool call manager that handles the complete tool calling lifecycle
pub struct ToolCallManager {
    converter: Box<dyn ToolCallConverter + Send + Sync>,
    state: ToolCallState,
}

impl ToolCallManager {
    pub fn new(converter: Box<dyn ToolCallConverter + Send + Sync>) -> Self {
        Self {
            converter,
            state: ToolCallState::None,
        }
    }
    
    /// Create a manager for OpenAI format
    pub fn openai() -> Self {
        Self::new(Box::new(OpenAIToolConverter))
    }
    
    /// Create a manager for Claude format
    pub fn claude() -> Self {
        Self::new(Box::new(ClaudeToolConverter))
    }
    
    /// Convert OpenAI tools to provider format
    pub fn convert_tools_to_provider(&self, tools: &[ToolDefinition]) -> Result<Value> {
        self.converter.openai_tools_to_provider(tools)
    }
    
    /// Process tool calls from provider response
    pub fn process_tool_calls(&mut self, provider_data: &Value, request_id: String) -> Result<Option<Vec<UnifiedToolCall>>> {
        let tool_calls = self.converter.provider_tool_calls_to_unified(provider_data)?;
        
        if !tool_calls.is_empty() {
            self.state = ToolCallState::WaitingForResults {
                tool_calls: tool_calls.clone(),
                request_id,
            };
            Ok(Some(tool_calls))
        } else {
            Ok(None)
        }
    }
    
    /// Convert unified tool calls to OpenAI format for the client
    pub fn to_openai_format(&self, unified_calls: &[UnifiedToolCall]) -> Vec<ToolCall> {
        self.converter.unified_to_openai_tool_calls(unified_calls)
    }
    
    /// Process tool call results from the client
    pub fn process_tool_results(&mut self, results: Vec<ToolCallResult>) -> Result<Value> {
        let provider_messages = self.converter.tool_results_to_provider_messages(&results)?;
        
        self.state = ToolCallState::ResultsReceived {
            results,
            request_id: match &self.state {
                ToolCallState::WaitingForResults { request_id, .. } => request_id.clone(),
                _ => "unknown".to_string(),
            },
        };
        
        Ok(provider_messages)
    }
    
    /// Get current tool call state
    pub fn get_state(&self) -> &ToolCallState {
        &self.state
    }
    
    /// Reset the tool call state
    pub fn reset(&mut self) {
        self.state = ToolCallState::None;
    }

    /// Access the converter
    pub fn converter(&self) -> &dyn ToolCallConverter {
        &*self.converter
    }
}

/// Shell execution manager integration with tool calling
pub struct ShellToolExecutor {
    shell_manager: crate::features::shell_execution::ShellExecutionManager,
}

impl ShellToolExecutor {
    /// Create a new shell tool executor
    pub fn new() -> Self {
        let config = crate::features::shell_execution::ShellConfig::default();
        let shell_manager = crate::features::shell_execution::ShellExecutionManager::new(config);
        Self { shell_manager }
    }

    /// Create with custom shell configuration
    pub fn with_config(config: crate::features::shell_execution::ShellConfig) -> Self {
        let shell_manager = crate::features::shell_execution::ShellExecutionManager::new(config);
        Self { shell_manager }
    }

    /// Execute a tool call using shell execution
    pub async fn execute_tool_call(&self, tool_call: &UnifiedToolCall) -> Result<ToolCallResult, anyhow::Error> {
        // Convert unified tool call to shell execution format
        let mut cmd_array = vec![tool_call.function_name.clone()];
        let args = self.parse_arguments_to_array(&tool_call.arguments)?;
        cmd_array.extend(args);
        
        let tool_call_json = serde_json::json!({
            "cmd": cmd_array
        });

        // Execute the command
        match self.shell_manager.execute_from_tool_call(&tool_call_json).await {
            Ok(result) => {
                Ok(ToolCallResult {
                    tool_call_id: tool_call.id.clone(),
                    content: if result.success {
                        result.stdout
                    } else {
                        format!("Error: {}", result.stderr)
                    },
                    success: result.success,
                    error: if result.success { None } else { Some(result.stderr) },
                })
            }
            Err(e) => {
                Ok(ToolCallResult {
                    tool_call_id: tool_call.id.clone(),
                    content: String::new(),
                    success: false,
                    error: Some(e.to_string()),
                })
            }
        }
    }

    /// Parse tool call arguments to array format expected by shell executor
    fn parse_arguments_to_array(&self, arguments: &Value) -> Result<Vec<String>, anyhow::Error> {
        match arguments {
            Value::Object(obj) => {
                // For apply_patch, we expect a specific format
                if let Some(patch_content) = obj.get("patch_content") {
                    if let Some(patch_str) = patch_content.as_str() {
                        return Ok(vec![patch_str.to_string()]);
                    }
                }
                
                // For other commands, convert object to key=value pairs
                let mut args = Vec::new();
                for (key, value) in obj {
                    match value {
                        Value::String(s) => args.push(format!("{}={}", key, s)),
                        Value::Number(n) => args.push(format!("{}={}", key, n)),
                        Value::Bool(b) => args.push(format!("{}={}", key, b)),
                        _ => args.push(format!("{}={}", key, value.to_string())),
                    }
                }
                Ok(args)
            }
            Value::Array(arr) => {
                let mut args = Vec::new();
                for item in arr {
                    match item {
                        Value::String(s) => args.push(s.clone()),
                        _ => args.push(item.to_string()),
                    }
                }
                Ok(args)
            }
            Value::String(s) => Ok(vec![s.clone()]),
            _ => Ok(vec![arguments.to_string()]),
        }
    }
}

impl Default for ShellToolExecutor {
    fn default() -> Self {
        Self::new()
    }
}

/// Enhanced tool call manager with shell execution support
pub struct EnhancedToolCallManager {
    base_manager: ToolCallManager,
    shell_executor: ShellToolExecutor,
}

impl EnhancedToolCallManager {
    /// Create a new enhanced tool call manager
    pub fn new(converter: Box<dyn ToolCallConverter + Send + Sync>) -> Self {
        Self {
            base_manager: ToolCallManager::new(converter),
            shell_executor: ShellToolExecutor::new(),
        }
    }

    /// Create for OpenAI format with shell support
    pub fn openai_with_shell() -> Self {
        Self::new(Box::new(OpenAIToolConverter))
    }

    /// Create for Claude format with shell support
    pub fn claude_with_shell() -> Self {
        Self::new(Box::new(ClaudeToolConverter))
    }

    /// Process tool calls and execute shell commands
    pub async fn process_and_execute_tool_calls(
        &mut self,
        provider_data: &Value,
        request_id: String
    ) -> Result<Option<Vec<ToolCallResult>>, anyhow::Error> {
        // First, use base manager to process tool calls
        if let Some(tool_calls) = self.base_manager.process_tool_calls(provider_data, request_id)? {
            let mut results = Vec::new();
            
            for tool_call in &tool_calls {
                // Check if this is a shell command
                if self.is_shell_command(&tool_call.function_name) {
                    // Execute using shell executor
                    let result = self.shell_executor.execute_tool_call(tool_call).await?;
                    results.push(result);
                } else {
                    // Create a placeholder result for non-shell commands
                    results.push(ToolCallResult {
                        tool_call_id: tool_call.id.clone(),
                        content: format!("Tool '{}' not implemented", tool_call.function_name),
                        success: false,
                        error: Some("Tool not implemented".to_string()),
                    });
                }
            }
            
            Ok(Some(results))
        } else {
            Ok(None)
        }
    }

    /// Check if a function name represents a shell command
    fn is_shell_command(&self, function_name: &str) -> bool {
        matches!(function_name,
            "apply_patch" |
            "read_file" |
            "write_file" |
            "copy_file" |
            "move_file" |
            "create_directory" |
            "file_exists"
        )
    }

    /// Get the underlying base manager
    pub fn base_manager(&self) -> &ToolCallManager {
        &self.base_manager
    }

    /// Get mutable reference to base manager
    pub fn base_manager_mut(&mut self) -> &mut ToolCallManager {
        &mut self.base_manager
    }
}

/// Utility functions for tool calling
pub mod utils {
    use super::*;
    
    /// Extract tool calls from streaming chunks
    pub fn extract_tool_calls_from_stream_chunk(chunk: &Value) -> Option<Vec<UnifiedToolCall>> {
        // Check for OpenAI format tool calls in streaming
        if let Some(choices) = chunk.get("choices").and_then(|c| c.as_array()) {
            if let Some(choice) = choices.first() {
                if let Some(delta) = choice.get("delta") {
                    if let Some(_tool_calls) = delta.get("tool_calls") {
                        // Handle incremental tool call building in streaming
                        // This is complex as tool calls can be split across multiple chunks
                        return Some(Vec::new()); // TODO: Implement incremental building
                    }
                }
            }
        }
        
        // Check for Claude format tool use
        if let Some(content_block) = chunk.get("content_block") {
            if content_block.get("type").and_then(|t| t.as_str()) == Some("tool_use") {
                // Claude tool use block
                return Some(Vec::new()); // TODO: Implement Claude tool use extraction
            }
        }
        
        None
    }
    
    /// Check if a chunk indicates tool calls are complete
    pub fn is_tool_calls_complete(chunk: &Value) -> bool {
        // OpenAI: check if finish_reason is "tool_calls"
        if let Some(choices) = chunk.get("choices").and_then(|c| c.as_array()) {
            if let Some(choice) = choices.first() {
                if let Some(finish_reason) = choice.get("finish_reason").and_then(|f| f.as_str()) {
                    return finish_reason == "tool_calls";
                }
            }
        }
        
        // Claude: check for content_block_stop with tool_use type
        if chunk.get("type").and_then(|t| t.as_str()) == Some("content_block_stop") {
            if let Some(content_block) = chunk.get("content_block") {
                if content_block.get("type").and_then(|t| t.as_str()) == Some("tool_use") {
                    return true;
                }
            }
        }
        
        false
    }
    
    /// Validate tool call arguments against schema
    pub fn validate_tool_arguments(
        function_name: &str,
        arguments: &Value,
        tools: &[ToolDefinition],
    ) -> Result<()> {
        // Find the tool definition
        let _tool_def = tools.iter()
            .find(|t| t.function.name == function_name)
            .ok_or_else(|| anyhow::anyhow!("Unknown function: {}", function_name))?;
        
        // Basic validation - in a real implementation, you'd use a JSON schema validator
        if !arguments.is_object() {
            return Err(anyhow::anyhow!("Tool arguments must be an object"));
        }
        
        // TODO: Implement proper JSON schema validation against tool_def.function.parameters
        
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use crate::models::common::FunctionDefinition;
    
    #[test]
    fn test_openai_tool_converter() {
        let converter = OpenAIToolConverter;
        
        // Test tool definition conversion
        let tools = vec![ToolDefinition {
            tool_type: "function".to_string(),
            function: FunctionDefinition {
                name: "get_weather".to_string(),
                description: Some("Get weather information".to_string()),
                parameters: Some(json!({
                    "type": "object",
                    "properties": {
                        "location": {"type": "string"}
                    }
                })),
            },
        }];
        
        let provider_tools = converter.openai_tools_to_provider(&tools).unwrap();
        assert!(provider_tools.is_array());
    }
    
    #[test]
    fn test_claude_tool_converter() {
        let converter = ClaudeToolConverter;
        
        // Test OpenAI to Claude conversion
        let tools = vec![ToolDefinition {
            tool_type: "function".to_string(),
            function: FunctionDefinition {
                name: "search_web".to_string(),
                description: Some("Search the web".to_string()),
                parameters: Some(json!({
                    "type": "object",
                    "properties": {
                        "query": {"type": "string"}
                    }
                })),
            },
        }];
        
        let claude_tools = converter.openai_tools_to_provider(&tools).unwrap();
        let tools_array = claude_tools.as_array().unwrap();
        assert_eq!(tools_array[0]["name"], "search_web");
        assert_eq!(tools_array[0]["description"], "Search the web");
        assert!(tools_array[0]["input_schema"].is_object());
    }
    
    #[test]
    fn test_tool_call_manager() {
        let mut manager = ToolCallManager::openai();
        
        // Test initial state
        assert!(matches!(manager.get_state(), ToolCallState::None));
        
        // Test processing tool calls
        let tool_call_data = json!([{
            "id": "call_123",
            "type": "function",
            "function": {
                "name": "get_weather",
                "arguments": {"location": "San Francisco"}
            }
        }]);
        
        let result = manager.process_tool_calls(&tool_call_data, "req_123".to_string()).unwrap();
        assert!(result.is_some());
        assert!(matches!(manager.get_state(), ToolCallState::WaitingForResults { .. }));
    }
}
