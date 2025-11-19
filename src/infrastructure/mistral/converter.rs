//! Mistral AI tool calling converter - OpenAI-compatible pass-through implementation
//!
//! Since Mistral AI uses OpenAI-compatible API format, this converter acts as a pass-through
//! with minimal transformation. It implements the ToolCallConverter trait to maintain
//! consistency with other providers while leveraging Mistral's native OpenAI compatibility.

use anyhow::{Context, Result};
use serde_json::Value;
use std::collections::HashMap;
use tracing::{debug, trace, warn};

use crate::infrastructure::common::tools::{ToolCallConverter, UnifiedToolCall, ToolCallResult};
use crate::models::common::ToolDefinition;
use crate::shared::types::{ToolCall, FunctionCall};

use super::types::{MistralModel, MistralConfig};

/// Mistral tool calling converter with OpenAI compatibility
/// 
/// This converter handles tool calling format conversion between the unified format
/// and Mistral's OpenAI-compatible API. Since Mistral uses the same format as OpenAI,
/// most operations are pass-through with optional Mistral-specific enhancements.
pub struct MistralConverter {
    /// Model capabilities cache
    model_capabilities: HashMap<String, ModelCapabilities>,
    
    /// Configuration for European compliance features
    config: Option<MistralConfig>,
}

/// Model capabilities for tool calling and features
#[derive(Debug, Clone)]
pub struct ModelCapabilities {
    /// Whether this model supports tool calling
    pub supports_tools: bool,
    
    /// Whether this model supports streaming
    pub supports_streaming: bool,
    
    /// Whether this model supports function calling (legacy)
    pub supports_functions: bool,
    
    /// Maximum number of tools per request
    pub max_tools: Option<u32>,
    
    /// Whether this model is optimized for code generation
    pub code_optimized: bool,
    
    /// Whether this model is available in EU data centers
    pub eu_available: bool,
    
    /// Tool calling format version
    pub tool_format_version: String,
}

/// Tool calling context for request processing
#[derive(Debug, Clone)]
pub struct ToolCallContext {
    /// Request ID for tracking
    pub request_id: String,
    
    /// Model being used
    pub model: String,
    
    /// Whether EU data residency is required
    pub eu_residency: bool,
    
    /// Whether GDPR compliance is enabled
    pub gdpr_mode: bool,
    
    /// Number of tools in the request
    pub tool_count: usize,
    
    /// Estimated token usage
    pub estimated_tokens: u32,
}

impl MistralConverter {
    /// Create a new Mistral converter
    pub fn new() -> Self {
        let mut converter = Self {
            model_capabilities: HashMap::new(),
            config: None,
        };
        
        // Initialize model capabilities cache
        converter.initialize_model_capabilities();
        
        debug!("Initialized Mistral converter with OpenAI-compatible format");
        converter
    }

    /// Create converter with configuration for European compliance
    pub fn with_config(config: MistralConfig) -> Self {
        let mut converter = Self::new();
        converter.config = Some(config);
        
        debug!("Initialized Mistral converter with European compliance features");
        converter
    }

    /// Initialize model capabilities cache
    fn initialize_model_capabilities(&mut self) {
        let models = MistralModel::get_all_models();
        
        for model in models {
            let capabilities = ModelCapabilities {
                supports_tools: model.supports_tools,
                supports_streaming: model.supports_streaming,
                supports_functions: model.supports_functions,
                max_tools: Some(128), // Mistral supports many tools like OpenAI
                code_optimized: model.supports_code,
                eu_available: model.eu_available,
                tool_format_version: "openai-compatible".to_string(),
            };
            
            self.model_capabilities.insert(model.name, capabilities);
        }
        
        trace!("Initialized {} model capabilities", self.model_capabilities.len());
    }

    /// Get model capabilities
    pub fn get_model_capabilities(&self, model: &str) -> ModelCapabilities {
        self.model_capabilities.get(model).cloned().unwrap_or_else(|| {
            warn!("Unknown model: {}, using default capabilities", model);
            ModelCapabilities {
                supports_tools: true, // Assume tool support for unknown models
                supports_streaming: true,
                supports_functions: true,
                max_tools: Some(128),
                code_optimized: false,
                eu_available: true, // Mistral is European, assume EU availability
                tool_format_version: "openai-compatible".to_string(),
            }
        })
    }

    /// Check if a model supports tool calling
    pub fn supports_tool_calling(&self, model: &str) -> bool {
        self.get_model_capabilities(model).supports_tools
    }

    /// Check if a model supports streaming
    pub fn supports_streaming(&self, model: &str) -> bool {
        self.get_model_capabilities(model).supports_streaming
    }

    /// Check if a model is available in EU data centers
    pub fn supports_eu_residency(&self, model: &str) -> bool {
        self.get_model_capabilities(model).eu_available
    }

    /// Validate tool calling request for Mistral-specific constraints
    pub fn validate_tool_request(&self, tools: &[ToolDefinition], model: &str) -> Result<()> {
        let capabilities = self.get_model_capabilities(model);
        
        if !capabilities.supports_tools {
            return Err(anyhow::anyhow!(
                "Model {} does not support tool calling", model
            ));
        }
        
        if let Some(max_tools) = capabilities.max_tools {
            if tools.len() > max_tools as usize {
                return Err(anyhow::anyhow!(
                    "Too many tools: {} > {} for model {}", 
                    tools.len(), max_tools, model
                ));
            }
        }
        
        // Validate each tool definition
        for (i, tool) in tools.iter().enumerate() {
            if tool.function.name.is_empty() {
                return Err(anyhow::anyhow!(
                    "Tool {} has empty name", i
                ));
            }
            
            if tool.function.name.len() > 64 {
                return Err(anyhow::anyhow!(
                    "Tool {} name too long: {} > 64 characters", 
                    i, tool.function.name.len()
                ));
            }
        }
        
        debug!("Validated {} tools for model {}", tools.len(), model);
        Ok(())
    }

    /// Create tool calling context
    pub fn create_context(&self, model: &str, tools: &[ToolDefinition]) -> ToolCallContext {
        let request_id = format!("mistral_{}", uuid::Uuid::new_v4().simple());
        
        ToolCallContext {
            request_id,
            model: model.to_string(),
            eu_residency: self.config.as_ref()
                .map(|c| c.prefers_eu_residency())
                .unwrap_or(false),
            gdpr_mode: self.config.as_ref()
                .map(|c| c.uses_gdpr_mode())
                .unwrap_or(true),
            tool_count: tools.len(),
            estimated_tokens: Self::estimate_tool_tokens(tools),
        }
    }

    /// Estimate token usage for tools (rough approximation)
    fn estimate_tool_tokens(tools: &[ToolDefinition]) -> u32 {
        tools.iter().map(|tool| {
            // Rough estimate: function name + description + parameters
            let name_tokens = (tool.function.name.len() / 4) as u32;
            let desc_tokens = tool.function.description.as_ref()
                .map(|d| (d.len() / 4) as u32)
                .unwrap_or(0);
            let params_tokens = 50; // Rough estimate for parameters JSON
            
            name_tokens + desc_tokens + params_tokens
        }).sum()
    }
}

impl Default for MistralConverter {
    fn default() -> Self {
        Self::new()
    }
}

impl ToolCallConverter for MistralConverter {
    /// Convert OpenAI tools to Mistral format (pass-through since they're compatible)
    fn openai_tools_to_provider(&self, tools: &[ToolDefinition]) -> Result<Value> {
        // Mistral uses OpenAI-compatible format, so this is a direct pass-through
        let tools_value = serde_json::to_value(tools)
            .context("Failed to serialize tools for Mistral")?;
        
        debug!("Converting {} tools to Mistral format (OpenAI-compatible)", tools.len());
        trace!("Tools: {}", tools_value);
        
        Ok(tools_value)
    }
    
    /// Convert Mistral tool calls to unified format (pass-through since they're OpenAI-compatible)
    fn provider_tool_calls_to_unified(&self, provider_data: &Value) -> Result<Vec<UnifiedToolCall>> {
        let tool_calls: Vec<ToolCall> = serde_json::from_value(provider_data.clone())
            .context("Failed to parse Mistral tool calls")?;
        
        let unified_calls: Vec<UnifiedToolCall> = tool_calls.into_iter().map(|tc| {
            let arguments = serde_json::from_str(&tc.function.arguments)
                .unwrap_or_else(|_| serde_json::Value::String(tc.function.arguments.clone()));
            
            let mut metadata = HashMap::new();
            metadata.insert("provider".to_string(), serde_json::Value::String("mistral".to_string()));
            metadata.insert("format".to_string(), serde_json::Value::String("openai-compatible".to_string()));
            
            UnifiedToolCall {
                id: tc.id,
                function_name: tc.function.name,
                arguments,
                metadata,
            }
        }).collect();
        
        debug!("Converted {} Mistral tool calls to unified format", unified_calls.len());
        Ok(unified_calls)
    }
    
    /// Convert unified tool calls to OpenAI format (for Mistral compatibility)
    fn unified_to_openai_tool_calls(&self, unified_calls: &[UnifiedToolCall]) -> Vec<ToolCall> {
        let tool_calls: Vec<ToolCall> = unified_calls.iter().map(|uc| {
            let arguments = match &uc.arguments {
                Value::String(s) => s.clone(),
                other => serde_json::to_string(other).unwrap_or_default(),
            };
            
            ToolCall {
                id: uc.id.clone(),
                tool_type: "function".to_string(),
                function: FunctionCall {
                    name: uc.function_name.clone(),
                    arguments,
                },
            }
        }).collect();
        
        debug!("Converted {} unified tool calls to Mistral format", tool_calls.len());
        tool_calls
    }
    
    /// Convert tool call results to Mistral-compatible messages for continuation
    fn tool_results_to_provider_messages(&self, results: &[ToolCallResult]) -> Result<Value> {
        let messages: Vec<Value> = results.iter().map(|result| {
            serde_json::json!({
                "role": "tool",
                "tool_call_id": result.tool_call_id,
                "content": if result.success {
                    result.content.clone()
                } else {
                    format!("Error: {}", result.error.as_ref().unwrap_or(&result.content))
                }
            })
        }).collect();
        
        debug!("Converted {} tool results to Mistral message format", results.len());
        Ok(serde_json::Value::Array(messages))
    }
}

/// Mistral-specific tool calling utilities
pub struct MistralToolUtils;

impl MistralToolUtils {
    /// Check if response contains tool calls
    pub fn has_tool_calls(response: &Value) -> bool {
        response.get("choices")
            .and_then(|choices| choices.as_array())
            .and_then(|arr| arr.first())
            .and_then(|choice| choice.get("message"))
            .and_then(|msg| msg.get("tool_calls"))
            .and_then(|tc| tc.as_array())
            .map(|arr| !arr.is_empty())
            .unwrap_or(false)
    }

    /// Extract tool calls from Mistral response
    pub fn extract_tool_calls(response: &Value) -> Result<Vec<ToolCall>> {
        let tool_calls_value = response.get("choices")
            .and_then(|choices| choices.as_array())
            .and_then(|arr| arr.first())
            .and_then(|choice| choice.get("message"))
            .and_then(|msg| msg.get("tool_calls"))
            .ok_or_else(|| anyhow::anyhow!("No tool calls found in response"))?;
        
        let tool_calls: Vec<ToolCall> = serde_json::from_value(tool_calls_value.clone())
            .context("Failed to parse tool calls from Mistral response")?;
        
        debug!("Extracted {} tool calls from Mistral response", tool_calls.len());
        Ok(tool_calls)
    }

    /// Check if model is code-optimized
    pub fn is_code_model(model: &str) -> bool {
        model.contains("codestral") || 
        model.contains("code") ||
        MistralModel::get_by_name(model)
            .map(|m| m.supports_code)
            .unwrap_or(false)
    }

    /// Get recommended model for tool calling workload
    pub fn recommend_model_for_tools(tool_count: usize, code_heavy: bool) -> String {
        match (tool_count, code_heavy) {
            (_, true) => "codestral-latest".to_string(),
            (1..=5, false) => "mistral-small-latest".to_string(),
            (6..=15, false) => "mistral-medium-latest".to_string(),
            _ => "mistral-large-latest".to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::common::{ToolDefinition, FunctionDefinition};
    use serde_json::json;

    fn create_test_tool() -> ToolDefinition {
        ToolDefinition {
            tool_type: "function".to_string(),
            function: FunctionDefinition {
                name: "get_weather".to_string(),
                description: Some("Get current weather".to_string()),
                parameters: json!({
                    "type": "object",
                    "properties": {
                        "location": {
                            "type": "string",
                            "description": "The city name"
                        }
                    },
                    "required": ["location"]
                }),
            },
        }
    }

    #[test]
    fn test_converter_creation() {
        let converter = MistralConverter::new();
        assert!(!converter.model_capabilities.is_empty());
    }

    #[test]
    fn test_model_capabilities() {
        let converter = MistralConverter::new();
        
        let capabilities = converter.get_model_capabilities("mistral-large-latest");
        assert!(capabilities.supports_tools);
        assert!(capabilities.supports_streaming);
        
        let code_capabilities = converter.get_model_capabilities("codestral-latest");
        assert!(code_capabilities.code_optimized);
    }

    #[test]
    fn test_tool_validation() {
        let converter = MistralConverter::new();
        let tools = vec![create_test_tool()];
        
        let result = converter.validate_tool_request(&tools, "mistral-large-latest");
        assert!(result.is_ok());
    }

    #[test]
    fn test_tool_conversion() {
        let converter = MistralConverter::new();
        let tools = vec![create_test_tool()];
        
        let result = converter.openai_tools_to_provider(&tools);
        assert!(result.is_ok());
        
        let tools_value = result.unwrap();
        assert!(tools_value.is_array());
    }

    #[test]
    fn test_tool_call_extraction() {
        let response = json!({
            "choices": [{
                "message": {
                    "tool_calls": [{
                        "id": "call_123",
                        "type": "function",
                        "function": {
                            "name": "get_weather",
                            "arguments": "{\"location\": \"Paris\"}"
                        }
                    }]
                }
            }]
        });
        
        assert!(MistralToolUtils::has_tool_calls(&response));
        
        let tool_calls = MistralToolUtils::extract_tool_calls(&response);
        assert!(tool_calls.is_ok());
        assert_eq!(tool_calls.unwrap().len(), 1);
    }

    #[test]
    fn test_model_recommendations() {
        assert_eq!(
            MistralToolUtils::recommend_model_for_tools(3, false),
            "mistral-small-latest"
        );
        
        assert_eq!(
            MistralToolUtils::recommend_model_for_tools(10, false),
            "mistral-medium-latest"
        );
        
        assert_eq!(
            MistralToolUtils::recommend_model_for_tools(5, true),
            "codestral-latest"
        );
    }

    #[test]
    fn test_code_model_detection() {
        assert!(MistralToolUtils::is_code_model("codestral-latest"));
        assert!(!MistralToolUtils::is_code_model("mistral-large-latest"));
    }

    #[test]
    fn test_context_creation() {
        let converter = MistralConverter::new();
        let tools = vec![create_test_tool()];
        
        let context = converter.create_context("mistral-large-latest", &tools);
        assert_eq!(context.model, "mistral-large-latest");
        assert_eq!(context.tool_count, 1);
        assert!(!context.request_id.is_empty());
    }
}