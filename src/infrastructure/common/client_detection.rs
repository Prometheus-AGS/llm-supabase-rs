// src/infrastructure/common/client_detection.rs
//
// Client capability detection for adaptive OpenAI compatibility
// Automatically detects legacy vs modern OpenAI client behavior

use anyhow::Result;
use axum::http::HeaderMap;
use std::collections::HashMap;
use tracing::{debug, info};

use crate::models::request::ChatCompletionRequest;

/// Represents detected client capabilities and expected behavior
#[derive(Debug, Clone, PartialEq)]
pub struct ClientCapabilities {
    /// Tool calling format preference
    pub tool_format: ToolFormat,
    
    /// Whether client supports parallel tool execution
    pub supports_parallel_execution: bool,
    
    /// Detected SDK version if available
    pub sdk_version: Option<String>,
    
    /// Streaming behavior expectations
    pub streaming_behavior: StreamingBehavior,
    
    /// Tool choice handling capability
    pub tool_choice_support: ToolChoiceSupport,
}

/// Tool calling format preferences
#[derive(Debug, Clone, PartialEq)]
pub enum ToolFormat {
    /// Legacy OpenAI function_call format (single call per response)
    Legacy,
    
    /// Modern OpenAI tool_calls array format (multiple calls supported)
    Modern,
    
    /// Adaptive - choose based on response content
    Adaptive,
}

/// Streaming behavior expectations
#[derive(Debug, Clone, PartialEq)]
pub enum StreamingBehavior {
    /// Legacy: Stream stops at function_call, new request needed
    LegacyStop,
    
    /// Modern: Stream stops at tool_calls array, single continuation
    ModernBatch,
    
    /// Adaptive: Choose based on tool call count
    Adaptive,
}

/// Tool choice parameter support level
#[derive(Debug, Clone, PartialEq)]
pub enum ToolChoiceSupport {
    /// Basic support (auto, none)
    Basic,
    
    /// Full support (auto, none, required, specific tool)
    Full,
    
    /// No support (ignore tool_choice)
    None,
}

/// Tool choice strategy parsed from request
#[derive(Debug, Clone, PartialEq)]
pub enum ToolChoiceStrategy {
    /// Let model decide whether to use tools
    Auto,
    
    /// Do not use any tools
    None,
    
    /// Must use at least one tool
    Required,
    
    /// Must use specific tool
    Specific { name: String },
}

/// Client capability detector service
pub struct ClientCapabilityDetector {
    /// Known SDK patterns for detection
    sdk_patterns: HashMap<String, ClientCapabilities>,
}

impl Default for ClientCapabilities {
    fn default() -> Self {
        Self {
            tool_format: ToolFormat::Legacy, // Safe default
            supports_parallel_execution: false,
            sdk_version: None,
            streaming_behavior: StreamingBehavior::LegacyStop,
            tool_choice_support: ToolChoiceSupport::Basic,
        }
    }
}

impl ClientCapabilityDetector {
    /// Create a new client detector with known patterns
    pub fn new() -> Self {
        let mut sdk_patterns = HashMap::new();
        
        // OpenAI SDK patterns
        sdk_patterns.insert("openai-python/1.".to_string(), ClientCapabilities {
            tool_format: ToolFormat::Modern,
            supports_parallel_execution: true,
            sdk_version: None,
            streaming_behavior: StreamingBehavior::ModernBatch,
            tool_choice_support: ToolChoiceSupport::Full,
        });
        
        sdk_patterns.insert("openai-python/0.".to_string(), ClientCapabilities {
            tool_format: ToolFormat::Legacy,
            supports_parallel_execution: false,
            sdk_version: None,
            streaming_behavior: StreamingBehavior::LegacyStop,
            tool_choice_support: ToolChoiceSupport::Basic,
        });
        
        // Node.js SDK patterns
        sdk_patterns.insert("openai-node/4.".to_string(), ClientCapabilities {
            tool_format: ToolFormat::Modern,
            supports_parallel_execution: true,
            sdk_version: None,
            streaming_behavior: StreamingBehavior::ModernBatch,
            tool_choice_support: ToolChoiceSupport::Full,
        });
        
        Self { sdk_patterns }
    }

    /// Detect client capabilities from request and headers
    pub fn detect_capabilities(
        &self,
        request: &ChatCompletionRequest,
        headers: &HeaderMap,
    ) -> ClientCapabilities {
        let mut capabilities = ClientCapabilities::default();
        
        // 1. Analyze request parameters for capability signals
        capabilities = self.analyze_request_parameters(request, capabilities);
        
        // 2. Analyze headers for SDK version information
        capabilities = self.analyze_headers(headers, capabilities);
        
        // 3. Apply heuristics for edge cases
        capabilities = self.apply_detection_heuristics(request, capabilities);
        
        info!(
            tool_format = ?capabilities.tool_format,
            parallel_support = capabilities.supports_parallel_execution,
            streaming_behavior = ?capabilities.streaming_behavior,
            "Detected client capabilities"
        );
        
        capabilities
    }

    /// Analyze request parameters for capability indicators
    fn analyze_request_parameters(
        &self,
        request: &ChatCompletionRequest,
        mut capabilities: ClientCapabilities,
    ) -> ClientCapabilities {
        // Check for legacy function calling
        if request.functions.is_some() && request.tools.is_none() {
            debug!("Detected legacy functions parameter - using legacy format");
            capabilities.tool_format = ToolFormat::Legacy;
            capabilities.supports_parallel_execution = false;
            capabilities.streaming_behavior = StreamingBehavior::LegacyStop;
            return capabilities;
        }
        
        // Check for modern tool calling indicators
        if request.tools.is_some() {
            debug!("Detected tools parameter - analyzing modern capabilities");
            
            // Check parallel_tool_calls parameter
            if let Some(parallel) = request.parallel_tool_calls {
                capabilities.supports_parallel_execution = parallel;
                if parallel {
                    capabilities.tool_format = ToolFormat::Modern;
                    capabilities.streaming_behavior = StreamingBehavior::ModernBatch;
                }
            }
            
            // Check tool_choice format
            if let Some(tool_choice) = &request.tool_choice {
                if tool_choice.is_object() {
                    debug!("Detected object-format tool_choice - modern client");
                    capabilities.tool_format = ToolFormat::Modern;
                    capabilities.tool_choice_support = ToolChoiceSupport::Full;
                }
            }
        }
        
        capabilities
    }

    /// Analyze headers for SDK version and client information
    fn analyze_headers(
        &self,
        headers: &HeaderMap,
        mut capabilities: ClientCapabilities,
    ) -> ClientCapabilities {
        if let Some(user_agent) = headers.get("user-agent") {
            if let Ok(user_agent_str) = user_agent.to_str() {
                debug!("Analyzing User-Agent: {}", user_agent_str);
                
                // Check against known SDK patterns
                for (pattern, pattern_capabilities) in &self.sdk_patterns {
                    if user_agent_str.contains(pattern) {
                        debug!("Matched SDK pattern: {}", pattern);
                        capabilities = pattern_capabilities.clone();
                        capabilities.sdk_version = Some(user_agent_str.to_string());
                        break;
                    }
                }
                
                // Additional heuristics
                if user_agent_str.contains("openai") {
                    capabilities.tool_choice_support = ToolChoiceSupport::Full;
                }
            }
        }
        
        capabilities
    }

    /// Apply final heuristics for edge cases
    fn apply_detection_heuristics(
        &self,
        request: &ChatCompletionRequest,
        mut capabilities: ClientCapabilities,
    ) -> ClientCapabilities {
        // If we have tools but no clear modern indicators, use adaptive
        if request.tools.is_some() && capabilities.tool_format == ToolFormat::Legacy {
            debug!("Tools present but no modern indicators - using adaptive format");
            capabilities.tool_format = ToolFormat::Adaptive;
            capabilities.streaming_behavior = StreamingBehavior::Adaptive;
        }
        
        // If no tools at all, format doesn't matter
        if request.tools.is_none() && request.functions.is_none() {
            debug!("No tools in request - format irrelevant");
            capabilities.tool_format = ToolFormat::Adaptive;
        }
        
        capabilities
    }
}

impl ToolChoiceStrategy {
    /// Parse tool_choice parameter from request
    pub fn from_request(request: &ChatCompletionRequest) -> Result<Self> {
        match &request.tool_choice {
            None => {
                // Default behavior based on tools presence
                if request.tools.is_some() || request.functions.is_some() {
                    Ok(ToolChoiceStrategy::Auto)
                } else {
                    Ok(ToolChoiceStrategy::None)
                }
            }
            Some(value) => {
                if let Some(s) = value.as_str() {
                    match s {
                        "auto" => Ok(ToolChoiceStrategy::Auto),
                        "none" => Ok(ToolChoiceStrategy::None),
                        "required" => Ok(ToolChoiceStrategy::Required),
                        _ => Err(anyhow::anyhow!("Invalid tool_choice string: {}", s)),
                    }
                } else if let Some(obj) = value.as_object() {
                    // Handle object format: {"type": "function", "function": {"name": "tool_name"}}
                    if obj.get("type").and_then(|t| t.as_str()) == Some("function") {
                        if let Some(function) = obj.get("function") {
                            if let Some(name) = function.get("name").and_then(|n| n.as_str()) {
                                return Ok(ToolChoiceStrategy::Specific {
                                    name: name.to_string(),
                                });
                            }
                        }
                    }
                    Err(anyhow::anyhow!("Invalid tool_choice object format"))
                } else {
                    Err(anyhow::anyhow!("Invalid tool_choice type"))
                }
            }
        }
    }

    /// Validate strategy against available tools
    pub fn validate_against_tools(&self, tools: &Option<Vec<crate::models::common::ToolDefinition>>) -> Result<()> {
        match self {
            ToolChoiceStrategy::None => {
                // None is always valid
                Ok(())
            }
            ToolChoiceStrategy::Auto => {
                // Auto is always valid
                Ok(())
            }
            ToolChoiceStrategy::Required => {
                if tools.is_none() || tools.as_ref().unwrap().is_empty() {
                    Err(anyhow::anyhow!("tool_choice is 'required' but no tools are defined"))
                } else {
                    Ok(())
                }
            }
            ToolChoiceStrategy::Specific { name } => {
                if let Some(tool_list) = tools {
                    if tool_list.iter().any(|t| t.function.name == *name) {
                        Ok(())
                    } else {
                        Err(anyhow::anyhow!("tool_choice specifies '{}' but tool is not defined", name))
                    }
                } else {
                    Err(anyhow::anyhow!("tool_choice specifies '{}' but no tools are defined", name))
                }
            }
        }
    }

    /// Check if this strategy should prevent tool usage
    pub fn prevents_tool_usage(&self) -> bool {
        matches!(self, ToolChoiceStrategy::None)
    }

    /// Check if this strategy requires tool usage
    pub fn requires_tool_usage(&self) -> bool {
        matches!(self, ToolChoiceStrategy::Required | ToolChoiceStrategy::Specific { .. })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::common::{ToolDefinition, FunctionDefinition};
    use serde_json::json;

    fn create_test_request_with_tools() -> ChatCompletionRequest {
        let mut request = ChatCompletionRequest::new("claude-4-sonnet", vec![]);
        request.tools = Some(vec![
            ToolDefinition {
                tool_type: "function".to_string(),
                function: FunctionDefinition {
                    name: "get_weather".to_string(),
                    description: Some("Get weather".to_string()),
                    parameters: Some(json!({"type": "object"})),
                },
            }
        ]);
        request
    }

    #[test]
    fn test_legacy_detection() {
        let detector = ClientCapabilityDetector::new();
        let mut request = ChatCompletionRequest::new("claude-4-sonnet", vec![]);
        request.functions = Some(vec![]); // Legacy functions parameter
        
        let headers = HeaderMap::new();
        let capabilities = detector.detect_capabilities(&request, &headers);
        
        assert_eq!(capabilities.tool_format, ToolFormat::Legacy);
        assert!(!capabilities.supports_parallel_execution);
    }

    #[test]
    fn test_modern_detection() {
        let detector = ClientCapabilityDetector::new();
        let mut request = create_test_request_with_tools();
        request.parallel_tool_calls = Some(true);
        
        let headers = HeaderMap::new();
        let capabilities = detector.detect_capabilities(&request, &headers);
        
        assert_eq!(capabilities.tool_format, ToolFormat::Modern);
        assert!(capabilities.supports_parallel_execution);
    }

    #[test]
    fn test_tool_choice_parsing() {
        // Test string values
        let mut request = create_test_request_with_tools();
        request.tool_choice = Some(json!("auto"));
        
        let strategy = ToolChoiceStrategy::from_request(&request).unwrap();
        assert_eq!(strategy, ToolChoiceStrategy::Auto);
        
        // Test object format
        request.tool_choice = Some(json!({
            "type": "function",
            "function": {"name": "get_weather"}
        }));
        
        let strategy = ToolChoiceStrategy::from_request(&request).unwrap();
        assert_eq!(strategy, ToolChoiceStrategy::Specific { name: "get_weather".to_string() });
    }

    #[test]
    fn test_tool_choice_validation() {
        let request = create_test_request_with_tools();
        
        // Valid specific tool
        let strategy = ToolChoiceStrategy::Specific { name: "get_weather".to_string() };
        assert!(strategy.validate_against_tools(&request.tools).is_ok());
        
        // Invalid specific tool
        let strategy = ToolChoiceStrategy::Specific { name: "nonexistent".to_string() };
        assert!(strategy.validate_against_tools(&request.tools).is_err());
        
        // Required with no tools
        let empty_request = ChatCompletionRequest::new("claude-4-sonnet", vec![]);
        let strategy = ToolChoiceStrategy::Required;
        assert!(strategy.validate_against_tools(&empty_request.tools).is_err());
    }

    #[test]
    fn test_user_agent_detection() {
        let detector = ClientCapabilityDetector::new();
        let request = create_test_request_with_tools();
        
        let mut headers = HeaderMap::new();
        headers.insert("user-agent", "openai-python/1.35.0".parse().unwrap());
        
        let capabilities = detector.detect_capabilities(&request, &headers);
        
        assert_eq!(capabilities.tool_format, ToolFormat::Modern);
        assert!(capabilities.supports_parallel_execution);
        assert!(capabilities.sdk_version.is_some());
    }

    #[test]
    fn test_adaptive_fallback() {
        let detector = ClientCapabilityDetector::new();
        let request = create_test_request_with_tools(); // Has tools but no clear indicators
        
        let headers = HeaderMap::new();
        let capabilities = detector.detect_capabilities(&request, &headers);
        
        // Should fall back to adaptive behavior
        assert_eq!(capabilities.tool_format, ToolFormat::Adaptive);
        assert_eq!(capabilities.streaming_behavior, StreamingBehavior::Adaptive);
    }
}