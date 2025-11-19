//! Cohere tool call converter implementation
//! 
//! This module handles the conversion between Cohere's tool calling format
//! and the unified OpenAI format used throughout the system.

use anyhow::Result;
use serde_json::{Value, json};
use std::collections::HashMap;
use tracing::{debug, warn, error};

use crate::infrastructure::common::tools::{
    ToolCallConverter, UnifiedToolCall, ToolCallResult
};
use crate::infrastructure::cohere::types::{
    CohereTool, CohereToolCall, CohereToolResult, CohereModel
};
use crate::models::common::ToolDefinition;
use crate::shared::types::{ToolCall, FunctionCall};

/// Cohere-specific tool call converter
#[derive(Debug, Clone)]
pub struct CohereConverter {
    /// Model capabilities cache
    model_capabilities: HashMap<String, CohereModelCapabilities>,
}

/// Model capabilities for Cohere models
#[derive(Debug, Clone)]
pub struct CohereModelCapabilities {
    /// Whether the model supports tools
    pub supports_tools: bool,
    
    /// Whether the model supports streaming
    pub supports_streaming: bool,
    
    /// Maximum context length
    pub max_context_length: u32,
    
    /// Whether the model supports parallel tool calls
    pub supports_parallel_tools: bool,
}

/// Context for tool call processing
#[derive(Debug, Clone)]
pub struct CohereToolCallContext {
    /// Current model being used
    pub model: String,
    
    /// Whether streaming is enabled
    pub streaming: bool,
    
    /// Request ID for tracking
    pub request_id: Option<String>,
}

impl CohereConverter {
    /// Create a new Cohere converter
    pub fn new() -> Self {
        let mut model_capabilities = HashMap::new();
        
        // Define capabilities for each Cohere model
        model_capabilities.insert("command-r-plus".to_string(), CohereModelCapabilities {
            supports_tools: true,
            supports_streaming: true,
            max_context_length: 128000,
            supports_parallel_tools: true,
        });
        
        model_capabilities.insert("command-r".to_string(), CohereModelCapabilities {
            supports_tools: true,
            supports_streaming: true,
            max_context_length: 128000,
            supports_parallel_tools: true,
        });
        
        model_capabilities.insert("command".to_string(), CohereModelCapabilities {
            supports_tools: false,
            supports_streaming: true,
            max_context_length: 4096,
            supports_parallel_tools: false,
        });
        
        model_capabilities.insert("command-nightly".to_string(), CohereModelCapabilities {
            supports_tools: true,
            supports_streaming: true,
            max_context_length: 128000,
            supports_parallel_tools: true,
        });
        
        Self {
            model_capabilities,
        }
    }
    
    /// Get model capabilities
    pub fn get_model_capabilities(&self, model: &str) -> CohereModelCapabilities {
        self.model_capabilities.get(model)
            .cloned()
            .unwrap_or_else(|| {
                warn!("Unknown Cohere model '{}', using default capabilities", model);
                CohereModelCapabilities {
                    supports_tools: false,
                    supports_streaming: true,
                    max_context_length: 4096,
                    supports_parallel_tools: false,
                }
            })
    }
    
    /// Check if model supports tool calling
    pub fn model_supports_tools(&self, model: &str) -> bool {
        self.get_model_capabilities(model).supports_tools
    }
    
    /// Convert Cohere tool calls to OpenAI format
    pub fn cohere_to_openai_tool_calls(&self, cohere_calls: &[CohereToolCall]) -> Vec<ToolCall> {
        debug!("Converting {} Cohere tool calls to OpenAI format", cohere_calls.len());
        
        cohere_calls.iter().enumerate().map(|(index, call)| {
            // Generate ID if not provided
            let id = call.id.clone().unwrap_or_else(|| {
                format!("call_{}", index)
            });
            
            // Convert parameters to JSON string
            let arguments = serde_json::to_string(&call.parameters)
                .unwrap_or_else(|_| "{}".to_string());
            
            ToolCall {
                id,
                tool_type: "function".to_string(),
                function: FunctionCall {
                    name: call.name.clone(),
                    arguments,
                },
            }
        }).collect()
    }
    
    /// Convert OpenAI tool calls to Cohere format
    pub fn openai_to_cohere_tool_calls(&self, openai_calls: &[ToolCall]) -> Result<Vec<CohereToolCall>> {
        debug!("Converting {} OpenAI tool calls to Cohere format", openai_calls.len());
        
        let mut cohere_calls = Vec::new();
        
        for call in openai_calls {
            // Parse arguments JSON
            let parameters: HashMap<String, Value> = serde_json::from_str(&call.function.arguments)
                .map_err(|e| anyhow::anyhow!("Failed to parse tool call arguments: {}", e))?;
            
            cohere_calls.push(CohereToolCall {
                name: call.function.name.clone(),
                parameters,
                id: Some(call.id.clone()),
            });
        }
        
        Ok(cohere_calls)
    }
    
    /// Convert tool call results to Cohere tool results format
    pub fn results_to_cohere_tool_results(&self, results: &[ToolCallResult], original_calls: &[CohereToolCall]) -> Result<Vec<CohereToolResult>> {
        debug!("Converting {} tool results to Cohere format", results.len());
        
        let mut cohere_results = Vec::new();
        
        for result in results {
            // Find the original call that matches this result
            let matching_call = original_calls.iter()
                .find(|call| {
                    call.id.as_ref().map(|id| id == &result.tool_call_id).unwrap_or(false)
                })
                .ok_or_else(|| anyhow::anyhow!("No matching call found for result ID: {}", result.tool_call_id))?;
            
            // Create output based on result
            let mut output = HashMap::new();
            output.insert("result".to_string(), Value::String(result.content.clone()));
            
            if let Some(error) = &result.error {
                output.insert("error".to_string(), Value::String(error.clone()));
            }
            
            output.insert("success".to_string(), Value::Bool(result.success));
            
            cohere_results.push(CohereToolResult {
                call: matching_call.clone(),
                outputs: vec![output],
            });
        }
        
        Ok(cohere_results)
    }
}

impl ToolCallConverter for CohereConverter {
    /// Convert OpenAI tools to Cohere format
    fn openai_tools_to_provider(&self, tools: &[ToolDefinition]) -> Result<Value> {
        debug!("Converting {} OpenAI tools to Cohere format", tools.len());
        
        let cohere_tools: Vec<CohereTool> = tools.iter()
            .map(|tool| CohereTool::from_openai_tool(tool))
            .collect();
        
        serde_json::to_value(cohere_tools)
            .map_err(|e| anyhow::anyhow!("Failed to serialize Cohere tools: {}", e))
    }
    
    /// Convert Cohere tool calls to unified format
    fn provider_tool_calls_to_unified(&self, provider_data: &Value) -> Result<Vec<UnifiedToolCall>> {
        debug!("Converting Cohere tool calls to unified format");
        
        // Parse Cohere tool calls from the response
        let cohere_calls: Vec<CohereToolCall> = serde_json::from_value(provider_data.clone())
            .map_err(|e| anyhow::anyhow!("Failed to parse Cohere tool calls: {}", e))?;
        
        let unified_calls = cohere_calls.into_iter().enumerate().map(|(index, call)| {
            let id = call.id.unwrap_or_else(|| format!("cohere_call_{}", index));
            
            UnifiedToolCall {
                id: id.clone(),
                function_name: call.name,
                arguments: serde_json::to_value(call.parameters).unwrap_or(json!({})),
                metadata: {
                    let mut metadata = HashMap::new();
                    metadata.insert("provider".to_string(), json!("cohere"));
                    metadata.insert("call_id".to_string(), json!(id));
                    metadata
                },
            }
        }).collect();
        
        debug!("Converted to {} unified tool calls", unified_calls.len());
        Ok(unified_calls)
    }
    
    /// Convert unified tool calls to OpenAI format
    fn unified_to_openai_tool_calls(&self, unified_calls: &[UnifiedToolCall]) -> Vec<ToolCall> {
        debug!("Converting {} unified tool calls to OpenAI format", unified_calls.len());
        
        unified_calls.iter().map(|call| {
            let arguments = serde_json::to_string(&call.arguments)
                .unwrap_or_else(|_| "{}".to_string());
            
            ToolCall {
                id: call.id.clone(),
                tool_type: "function".to_string(),
                function: FunctionCall {
                    name: call.function_name.clone(),
                    arguments,
                },
            }
        }).collect()
    }
    
    /// Convert tool call results to Cohere message format for continuation
    fn tool_results_to_provider_messages(&self, results: &[ToolCallResult]) -> Result<Value> {
        debug!("Converting {} tool results to Cohere message format", results.len());
        
        // In Cohere, tool results are included as tool_results in the next request
        // We need to reconstruct the original calls and create tool results
        let mut tool_results = Vec::new();
        
        for result in results {
            // Create a minimal tool call structure for the result
            let call = CohereToolCall {
                name: "unknown".to_string(), // This should be provided by the calling code
                parameters: HashMap::new(),
                id: Some(result.tool_call_id.clone()),
            };
            
            let mut output = HashMap::new();
            output.insert("result".to_string(), Value::String(result.content.clone()));
            
            if let Some(error) = &result.error {
                output.insert("error".to_string(), Value::String(error.clone()));
            }
            
            output.insert("success".to_string(), Value::Bool(result.success));
            
            tool_results.push(CohereToolResult {
                call,
                outputs: vec![output],
            });
        }
        
        serde_json::to_value(tool_results)
            .map_err(|e| anyhow::anyhow!("Failed to serialize Cohere tool results: {}", e))
    }
}

impl Default for CohereConverter {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    
    #[test]
    fn test_converter_creation() {
        let converter = CohereConverter::new();
        assert!(converter.model_supports_tools("command-r-plus"));
        assert!(!converter.model_supports_tools("command"));
    }
    
    #[test]
    fn test_model_capabilities() {
        let converter = CohereConverter::new();
        
        let caps = converter.get_model_capabilities("command-r-plus");
        assert!(caps.supports_tools);
        assert!(caps.supports_streaming);
        assert!(caps.supports_parallel_tools);
        assert_eq!(caps.max_context_length, 128000);
        
        let unknown_caps = converter.get_model_capabilities("unknown-model");
        assert!(!unknown_caps.supports_tools);
    }
    
    #[test]
    fn test_cohere_to_openai_tool_calls() {
        let converter = CohereConverter::new();
        
        let mut params = HashMap::new();
        params.insert("query".to_string(), json!("test search"));
        
        let cohere_calls = vec![
            CohereToolCall {
                name: "search".to_string(),
                parameters: params,
                id: Some("call_123".to_string()),
            }
        ];
        
        let openai_calls = converter.cohere_to_openai_tool_calls(&cohere_calls);
        
        assert_eq!(openai_calls.len(), 1);
        assert_eq!(openai_calls[0].id, "call_123");
        assert_eq!(openai_calls[0].function.name, "search");
    }
    
    #[test]
    fn test_openai_to_cohere_tool_calls() {
        let converter = CohereConverter::new();
        
        let openai_calls = vec![
            ToolCall {
                id: "call_123".to_string(),
                r#type: "function".to_string(),
                function: FunctionCall {
                    name: "search".to_string(),
                    arguments: r#"{"query":"test"}"#.to_string(),
                },
            }
        ];
        
        let cohere_calls = converter.openai_to_cohere_tool_calls(&openai_calls).unwrap();
        
        assert_eq!(cohere_calls.len(), 1);
        assert_eq!(cohere_calls[0].name, "search");
        assert_eq!(cohere_calls[0].id, Some("call_123".to_string()));
    }
    
    #[test]
    fn test_openai_tools_to_provider() {
        let converter = CohereConverter::new();
        
        let tools = vec![
            ToolDefinition {
                r#type: "function".to_string(),
                function: crate::models::common::FunctionDefinition {
                    name: "search".to_string(),
                    description: Some("Search for information".to_string()),
                    parameters: Some(json!({
                        "type": "object",
                        "properties": {
                            "query": {
                                "type": "string",
                                "description": "Search query"
                            }
                        },
                        "required": ["query"]
                    })),
                },
            }
        ];
        
        let provider_tools = converter.openai_tools_to_provider(&tools).unwrap();
        assert!(provider_tools.is_array());
        
        let cohere_tools: Vec<CohereTool> = serde_json::from_value(provider_tools).unwrap();
        assert_eq!(cohere_tools.len(), 1);
        assert_eq!(cohere_tools[0].name, "search");
    }
    
    #[test]
    fn test_provider_tool_calls_to_unified() {
        let converter = CohereConverter::new();
        
        let provider_data = json!([
            {
                "name": "search",
                "parameters": {"query": "test"},
                "id": "call_123"
            }
        ]);
        
        let unified_calls = converter.provider_tool_calls_to_unified(&provider_data).unwrap();
        
        assert_eq!(unified_calls.len(), 1);
        assert_eq!(unified_calls[0].function_name, "search");
        assert_eq!(unified_calls[0].id, "call_123");
    }
}