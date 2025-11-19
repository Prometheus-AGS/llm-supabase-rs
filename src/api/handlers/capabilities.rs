// src/api/handlers/capabilities.rs
//
// Feature detection endpoint for client capability discovery
// Allows clients to query server capabilities for optimal integration

use axum::{
    extract::{Request, State},
    response::Json,
};
use serde::{Deserialize, Serialize};

use crate::app::AppState;
use crate::infrastructure::common::ClientCapabilityDetector;
use crate::shared::AppError;

/// Server capability information response
#[derive(Debug, Serialize, Deserialize)]
pub struct ServerCapabilities {
    /// Server version
    pub version: String,
    
    /// Supported tool calling formats
    pub tool_calling: ToolCallingCapabilities,
    
    /// Streaming capabilities
    pub streaming: StreamingCapabilities,
    
    /// Supported models
    pub models: Vec<String>,
    
    /// Performance characteristics
    pub performance: PerformanceCapabilities,
}

/// Tool calling capability details
#[derive(Debug, Serialize, Deserialize)]
pub struct ToolCallingCapabilities {
    /// Supports legacy function_call format
    pub supports_function_call: bool,
    
    /// Supports modern tool_calls array format
    pub supports_tool_calls_array: bool,
    
    /// Supports parallel tool execution
    pub supports_parallel_execution: bool,
    
    /// Maximum tool calls per request
    pub max_tool_calls_per_turn: Option<u32>,
    
    /// Supported tool_choice values
    pub supported_tool_choice: Vec<String>,
    
    /// Supports parallel_tool_calls parameter
    pub supports_parallel_tool_calls_param: bool,
}

/// Streaming capability details
#[derive(Debug, Serialize, Deserialize)]
pub struct StreamingCapabilities {
    /// Supports Server-Sent Events streaming
    pub supports_sse_streaming: bool,
    
    /// Supports tool calls in streaming
    pub supports_streaming_tool_calls: bool,
    
    /// Adaptive streaming behavior based on client
    pub adaptive_streaming_behavior: bool,
    
    /// Stream termination patterns supported
    pub stream_termination_patterns: Vec<String>,
}

/// Performance capability details
#[derive(Debug, Serialize, Deserialize)]
pub struct PerformanceCapabilities {
    /// Internal parallel tool execution
    pub internal_parallel_execution: bool,
    
    /// Adaptive response formatting
    pub adaptive_response_formatting: bool,
    
    /// Client capability auto-detection
    pub automatic_client_detection: bool,
    
    /// Average tool call latency (ms)
    pub avg_tool_call_latency_ms: Option<u32>,
}

/// Client capability detection request
#[derive(Debug, Deserialize)]
pub struct CapabilityDetectionRequest {
    /// Sample request for capability detection
    #[serde(flatten)]
    pub sample_request: serde_json::Value,
}

/// Client capability detection response
#[derive(Debug, Serialize)]
pub struct CapabilityDetectionResponse {
    /// Detected client capabilities
    pub detected_capabilities: ClientCapabilitiesInfo,
    
    /// Recommended settings for optimal performance
    pub recommendations: ClientRecommendations,
}

/// Client capabilities information
#[derive(Debug, Serialize)]
pub struct ClientCapabilitiesInfo {
    /// Detected tool format preference
    pub tool_format: String,
    
    /// Parallel execution support
    pub supports_parallel_execution: bool,
    
    /// Detected SDK version
    pub sdk_version: Option<String>,
    
    /// Streaming behavior expectation
    pub streaming_behavior: String,
    
    /// Tool choice support level
    pub tool_choice_support: String,
}

/// Recommendations for client optimization
#[derive(Debug, Serialize)]
pub struct ClientRecommendations {
    /// Recommended tool_choice setting
    pub recommended_tool_choice: String,
    
    /// Recommended parallel_tool_calls setting
    pub recommended_parallel_tool_calls: bool,
    
    /// Recommended streaming approach
    pub recommended_streaming_approach: String,
    
    /// Performance tips
    pub performance_tips: Vec<String>,
}

/// Get server capabilities
///
/// Returns comprehensive information about server capabilities
/// for client optimization and feature detection
pub async fn get_capabilities(
    State(_state): State<AppState>,
) -> Result<Json<ServerCapabilities>, AppError> {
    let capabilities = ServerCapabilities {
        version: env!("CARGO_PKG_VERSION").to_string(),
        tool_calling: ToolCallingCapabilities {
            supports_function_call: true,
            supports_tool_calls_array: true,
            supports_parallel_execution: true,
            max_tool_calls_per_turn: Some(10), // Reasonable limit
            supported_tool_choice: vec![
                "auto".to_string(),
                "none".to_string(),
                "required".to_string(),
                "specific".to_string(),
            ],
            supports_parallel_tool_calls_param: true,
        },
        streaming: StreamingCapabilities {
            supports_sse_streaming: true,
            supports_streaming_tool_calls: true,
            adaptive_streaming_behavior: true,
            stream_termination_patterns: vec![
                "function_call".to_string(),
                "tool_calls".to_string(),
                "stop".to_string(),
            ],
        },
        models: vec![
            "claude-4-sonnet-20250514".to_string(),
            "claude-sonnet-4-5@20250929".to_string(),
            "claude-3-5-sonnet@20241022".to_string(),
            "claude-3-5-haiku@20241022".to_string(),
        ],
        performance: PerformanceCapabilities {
            internal_parallel_execution: true,
            adaptive_response_formatting: true,
            automatic_client_detection: true,
            avg_tool_call_latency_ms: Some(150), // Estimated average
        },
    };
    
    Ok(Json(capabilities))
}

/// Detect client capabilities from sample request
/// 
/// Analyzes a sample request to determine client capabilities
/// and provide optimization recommendations
pub async fn detect_client_capabilities(
    State(_state): State<AppState>,
    request: Request,
) -> Result<Json<CapabilityDetectionResponse>, AppError> {
    
    
    // Extract headers
    let headers = request.headers().clone();
    
    // Extract request body
    let body_bytes = match axum::body::to_bytes(request.into_body(), usize::MAX).await {
        Ok(bytes) => bytes,
        Err(e) => {
            return Err(AppError::validation(format!("Failed to read request body: {}", e)));
        }
    };
    
    // Parse as ChatCompletionRequest for analysis
    let chat_request: crate::models::request::ChatCompletionRequest = 
        match serde_json::from_slice(&body_bytes) {
            Ok(req) => req,
            Err(e) => {
                return Err(AppError::validation(format!("Invalid request format: {}", e)));
            }
        };
    
    // Detect capabilities
    let detector = ClientCapabilityDetector::new();
    let capabilities = detector.detect_capabilities(&chat_request, &headers);
    
    // Convert to response format
    let capabilities_info = ClientCapabilitiesInfo {
        tool_format: format!("{:?}", capabilities.tool_format),
        supports_parallel_execution: capabilities.supports_parallel_execution,
        sdk_version: capabilities.sdk_version.clone(),
        streaming_behavior: format!("{:?}", capabilities.streaming_behavior),
        tool_choice_support: format!("{:?}", capabilities.tool_choice_support),
    };
    
    // Generate recommendations
    let recommendations = ClientRecommendations {
        recommended_tool_choice: if capabilities.tool_choice_support == 
            crate::infrastructure::common::ToolChoiceSupport::Full {
            "auto".to_string()
        } else {
            "none".to_string()
        },
        recommended_parallel_tool_calls: capabilities.supports_parallel_execution,
        recommended_streaming_approach: format!("{:?}", capabilities.streaming_behavior),
        performance_tips: vec![
            "Use parallel_tool_calls: true for better performance".to_string(),
            "Consider tool_calls array format for multiple tools".to_string(),
            "Enable streaming for better user experience".to_string(),
        ],
    };
    
    let response = CapabilityDetectionResponse {
        detected_capabilities: capabilities_info,
        recommendations,
    };
    
    
    Ok(Json(response))
}

/// Health check with capability summary
pub async fn capabilities_health(
    State(_state): State<AppState>,
) -> Result<Json<serde_json::Value>, AppError> {
    let health_info = serde_json::json!({
        "status": "healthy",
        "adaptive_tool_calling": {
            "enabled": true,
            "formats_supported": ["function_call", "tool_calls"],
            "detection_methods": ["request_parameters", "user_agent", "headers"],
            "parallel_execution": true
        },
        "streaming": {
            "enabled": true,
            "adaptive_behavior": true,
            "tool_call_streaming": true
        },
        "performance": {
            "parallel_tool_execution": true,
            "adaptive_formatting": true,
            "automatic_detection": true
        }
    });
    
    Ok(Json(health_info))
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::HeaderMap;

    #[tokio::test]
    async fn test_capabilities_endpoint() {
        // Test that capabilities endpoint returns expected structure
        let capabilities = ServerCapabilities {
            version: "1.0.0".to_string(),
            tool_calling: ToolCallingCapabilities {
                supports_function_call: true,
                supports_tool_calls_array: true,
                supports_parallel_execution: true,
                max_tool_calls_per_turn: Some(10),
                supported_tool_choice: vec!["auto".to_string(), "none".to_string()],
                supports_parallel_tool_calls_param: true,
            },
            streaming: StreamingCapabilities {
                supports_sse_streaming: true,
                supports_streaming_tool_calls: true,
                adaptive_streaming_behavior: true,
                stream_termination_patterns: vec!["function_call".to_string()],
            },
            models: vec!["claude-4-sonnet".to_string()],
            performance: PerformanceCapabilities {
                internal_parallel_execution: true,
                adaptive_response_formatting: true,
                automatic_client_detection: true,
                avg_tool_call_latency_ms: Some(150),
            },
        };
        
        // Verify serialization works
        let json = serde_json::to_string(&capabilities).unwrap();
        assert!(json.contains("supports_function_call"));
        assert!(json.contains("supports_tool_calls_array"));
        assert!(json.contains("supports_parallel_execution"));
    }

    #[test]
    fn test_client_capabilities_info_serialization() {
        let info = ClientCapabilitiesInfo {
            tool_format: "Modern".to_string(),
            supports_parallel_execution: true,
            sdk_version: Some("openai-python/1.35.0".to_string()),
            streaming_behavior: "ModernBatch".to_string(),
            tool_choice_support: "Full".to_string(),
        };
        
        let json = serde_json::to_string(&info).unwrap();
        assert!(json.contains("Modern"));
        assert!(json.contains("openai-python"));
    }
}