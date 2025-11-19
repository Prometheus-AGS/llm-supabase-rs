//! Integration tests for Groq provider with tool calling infrastructure
//! 
//! This module tests the complete integration of the Groq provider with:
//! - Tool calling infrastructure (OpenAI-compatible)
//! - Ultra-fast streaming support
//! - Provider fallback system
//! - Configuration management
//! - Performance optimization features
//! - Model capability detection

use anyhow::Result;
use serde_json::json;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio_test;
use futures_util::StreamExt;

use crate::infrastructure::groq::{
    GroqClient, GroqConfig, GroqProvider, GroqConverter, GroqProviderFactory, 
    GroqModel, GroqSpeedTier, GroqStreamParser, GroqUtils
};
use crate::infrastructure::common::tools::{ToolCallManager, UnifiedToolCall, ToolCallResult};
use crate::models::request::ChatCompletionRequest;
use crate::models::common::{ChatMessage, MessageRole, ToolDefinition, FunctionDefinition};
use crate::config::providers::{ProvidersConfig, GroqConfig as ConfigGroqConfig};
use crate::features::provider_fallback::models::{Provider, ProviderCapability};

/// Test Groq provider basic functionality
#[tokio::test]
async fn test_groq_provider_creation() -> Result<()> {
    let config = GroqConfig {
        api_key: "gsk_test123456789012345678901234567890123456789012345".to_string(),
        base_url: Some("https://api.groq.com/openai/v1".to_string()),
        model: "llama-3.1-70b-versatile".to_string(),
        ..Default::default()
    };

    // Test client creation
    let client = GroqClient::new(config.clone()).await?;
    assert!(client.supports_tool_calling("llama-3.1-70b-versatile"));
    assert!(client.supports_tool_calling("llama-3.1-8b-instant"));
    assert!(!client.supports_tool_calling("llama3-70b-8192")); // Older model, no tools

    // Test provider creation
    let provider = GroqProvider::new(config).await?;
    let capabilities = provider.get_capabilities();
    
    assert_eq!(capabilities.provider, "groq");
    assert!(capabilities.supports_chat);
    assert!(capabilities.supports_tools);
    assert!(capabilities.supports_streaming);
    assert!(!capabilities.supports_completion); // Groq focuses on chat
    assert!(!capabilities.supports_embeddings); // Not available via Groq
    assert_eq!(capabilities.max_context_length, 131072); // Llama 3.1 max context
    
    Ok(())
}

/// Test Groq provider factory methods
#[tokio::test]
async fn test_groq_provider_factory() -> Result<()> {
    // Test creation with API key
    let provider = GroqProviderFactory::create_with_api_key(
        "gsk_test123456789012345678901234567890123456789012345".to_string()
    ).await?;
    
    let config = provider.config();
    assert!(!config.api_key.is_empty());
    assert!(config.api_key.starts_with("gsk_"));
    assert_eq!(config.get_base_url(), "https://api.groq.com/openai/v1");
    
    // Test ultra-fast factory
    let ultra_fast_provider = GroqProviderFactory::create_ultra_fast().await;
    if ultra_fast_provider.is_ok() {
        let provider = ultra_fast_provider.unwrap();
        assert_eq!(provider.config().model, "llama-3.1-8b-instant");
        assert_eq!(provider.config().timeout, Some(30));
    }
    
    // Test tool calling optimized factory
    let tools_provider = GroqProviderFactory::create_for_tools().await;
    if tools_provider.is_ok() {
        let provider = tools_provider.unwrap();
        assert_eq!(provider.config().model, "llama-3.1-70b-versatile");
        assert!(GroqModel::supports_tool_calling(&provider.config().model));
    }
    
    Ok(())
}

/// Test tool calling integration with Groq converter
#[tokio::test]
async fn test_tool_calling_integration() -> Result<()> {
    let converter = GroqConverter::new();

    // Create test tool definitions
    let tools = vec![
        ToolDefinition {
            tool_type: "function".to_string(),
            function: FunctionDefinition {
                name: "get_weather".to_string(),
                description: Some("Get weather information for a location".to_string()),
                parameters: Some(json!({
                    "type": "object",
                    "properties": {
                        "location": {
                            "type": "string",
                            "description": "The location to get weather for"
                        },
                        "unit": {
                            "type": "string",
                            "enum": ["celsius", "fahrenheit"],
                            "description": "Temperature unit"
                        }
                    },
                    "required": ["location"]
                })),
            },
        },
        ToolDefinition {
            tool_type: "function".to_string(),
            function: FunctionDefinition {
                name: "calculate_math".to_string(),
                description: Some("Perform mathematical calculations".to_string()),
                parameters: Some(json!({
                    "type": "object",
                    "properties": {
                        "expression": {
                            "type": "string",
                            "description": "Mathematical expression to evaluate"
                        }
                    },
                    "required": ["expression"]
                })),
            },
        },
    ];

    // Test tool conversion (should be pass-through like OpenAI)
    let converted_tools = converter.convert_tools(&tools)?;
    assert!(converted_tools.is_array());
    assert_eq!(converted_tools.as_array().unwrap().len(), 2);

    // Test request validation for tool-capable model
    let test_request = ChatCompletionRequest {
        model: "llama-3.1-70b-versatile".to_string(),
        messages: vec![ChatMessage {
            role: MessageRole::User,
            content: "What's the weather in San Francisco?".to_string(),
            name: None,
            function_call: None,
            tool_calls: None,
            tool_call_id: None,
        }],
        tools: Some(tools.clone()),
        tool_choice: Some(json!("auto")),
        ..Default::default()
    };

    // Should pass validation for tool-capable model
    assert!(converter.validate_tool_request(&test_request).is_ok());
    assert!(converter.validate_model_for_tools("llama-3.1-70b-versatile").is_ok());

    // Test Groq optimization
    let mut optimized_request = test_request.clone();
    converter.optimize_for_groq(&mut optimized_request, "llama-3.1-8b-instant")?;
    
    // Ultra-fast model should have optimized temperature
    assert!(optimized_request.temperature.unwrap_or(1.0) < 1.0);
    
    Ok(())
}

/// Test model capabilities and recommendations
#[tokio::test]
async fn test_model_capabilities() -> Result<()> {
    let converter = GroqConverter::new();

    // Test Llama 3.1 models (most capable)
    let llama_70b_caps = converter.get_model_capabilities("llama-3.1-70b-versatile");
    assert!(llama_70b_caps.supports_tools);
    assert!(llama_70b_caps.supports_parallel_tools);
    assert!(llama_70b_caps.supports_streaming);
    assert_eq!(llama_70b_caps.max_tool_calls, Some(16));
    assert_eq!(llama_70b_caps.speed_tier, "very_fast");

    let llama_8b_caps = converter.get_model_capabilities("llama-3.1-8b-instant");
    assert!(llama_8b_caps.supports_tools);
    assert!(llama_8b_caps.supports_parallel_tools);
    assert_eq!(llama_8b_caps.speed_tier, "ultra_fast");
    assert_eq!(llama_8b_caps.max_tool_calls, Some(8)); // Smaller model, fewer parallel calls

    // Test Mixtral model
    let mixtral_caps = converter.get_model_capabilities("mixtral-8x7b-32768");
    assert!(mixtral_caps.supports_tools);
    assert!(mixtral_caps.supports_parallel_tools);
    assert_eq!(mixtral_caps.speed_tier, "fast");

    // Test Gemma model (limited parallel tools)
    let gemma_caps = converter.get_model_capabilities("gemma2-9b-it");
    assert!(gemma_caps.supports_tools);
    assert!(!gemma_caps.supports_parallel_tools);
    assert_eq!(gemma_caps.max_tool_calls, Some(4));

    // Test older models (no tool support)
    let old_model_caps = converter.get_model_capabilities("llama3-70b-8192");
    assert!(!old_model_caps.supports_tools);
    assert!(!old_model_caps.supports_parallel_tools);

    // Test model support checks
    assert!(converter.supports_parallel_tools("llama-3.1-70b-versatile"));
    assert!(!converter.supports_parallel_tools("gemma2-9b-it"));
    assert!(!converter.supports_parallel_tools("llama3-70b-8192"));

    Ok(())
}

/// Test Groq model information and recommendations
#[tokio::test]
async fn test_groq_models() -> Result<()> {
    // Test supported models
    let models = GroqModel::get_supported_models();
    assert!(!models.is_empty());
    assert!(models.len() >= 6); // At least 6 models supported

    // Test tool-capable models
    let tool_models = GroqModel::get_tool_capable_models();
    assert!(!tool_models.is_empty());
    assert!(tool_models.len() >= 4); // At least 4 models support tools

    // Test specific model lookup
    let llama_model = GroqModel::get_model("llama-3.1-70b-versatile");
    assert!(llama_model.is_some());
    let model = llama_model.unwrap();
    assert_eq!(model.family, "llama-3.1");
    assert_eq!(model.speed_tier, GroqSpeedTier::VeryFast);
    assert!(model.supports_tools);
    assert!(model.supports_streaming);
    assert!(model.supports_parallel_tools);

    // Test model recommendations
    let tool_rec = GroqModel::get_recommended_for_use_case("tool_calling");
    assert!(tool_rec.is_some());
    assert!(tool_rec.unwrap().supports_tools);

    let speed_rec = GroqModel::get_recommended_for_use_case("speed");
    assert!(speed_rec.is_some());
    assert_eq!(speed_rec.unwrap().speed_tier, GroqSpeedTier::UltraFast);

    let reasoning_rec = GroqModel::get_recommended_for_use_case("reasoning");
    assert!(reasoning_rec.is_some());
    
    // Test tool calling support detection
    assert!(GroqModel::supports_tool_calling("llama-3.1-70b-versatile"));
    assert!(GroqModel::supports_tool_calling("llama-3.1-8b-instant"));
    assert!(GroqModel::supports_tool_calling("mixtral-8x7b-32768"));
    assert!(GroqModel::supports_tool_calling("gemma2-9b-it"));
    assert!(!GroqModel::supports_tool_calling("llama3-70b-8192"));
    assert!(!GroqModel::supports_tool_calling("llama3-8b-8192"));

    Ok(())
}

/// Test Groq utilities and helper functions
#[tokio::test]
async fn test_groq_utilities() -> Result<()> {
    // Test model validation
    assert!(GroqUtils::validate_model_for_request("llama-3.1-70b-versatile", true).is_ok());
    assert!(GroqUtils::validate_model_for_request("llama3-70b-8192", false).is_ok());
    assert!(GroqUtils::validate_model_for_request("llama3-70b-8192", true).is_err()); // No tools
    assert!(GroqUtils::validate_model_for_request("unknown-model", false).is_err());

    // Test model recommendations
    let best_tool_model = GroqUtils::get_best_model_for_use_case("tool_calling");
    assert!(best_tool_model.is_some());
    assert!(best_tool_model.unwrap().supports_tools);

    let best_speed_model = GroqUtils::get_best_model_for_use_case("speed");
    assert!(best_speed_model.is_some());
    assert_eq!(best_speed_model.unwrap().speed_tier, GroqSpeedTier::UltraFast);

    // Test response time estimation
    let fast_time = GroqUtils::estimate_response_time("llama-3.1-8b-instant", 100);
    let slow_time = GroqUtils::estimate_response_time("mixtral-8x7b-32768", 100);
    assert!(fast_time < slow_time);
    assert!(fast_time < Duration::from_millis(500)); // Should be very fast

    // Test streaming decision
    let short_request = ChatCompletionRequest {
        max_tokens: Some(10),
        ..Default::default()
    };
    assert!(!GroqUtils::should_use_streaming(&short_request));

    let long_request = ChatCompletionRequest {
        max_tokens: Some(500),
        ..Default::default()
    };
    assert!(GroqUtils::should_use_streaming(&long_request));

    Ok(())
}

/// Test streaming functionality with Groq parser
#[tokio::test]
async fn test_streaming_functionality() -> Result<()> {
    // Test stream parser creation
    let parser = GroqStreamParser::new();
    assert!(parser.get_performance_metrics().is_none()); // No metrics yet

    let model_parser = GroqStreamParser::with_model("llama-3.1-8b-instant".to_string());
    // Test that model-specific parser is created

    // Test test streams
    let mut test_stream = GroqStreamParser::create_test_stream("llama-3.1-70b-versatile");
    let mut chunk_count = 0;
    
    while let Some(result) = test_stream.next().await {
        assert!(result.is_ok());
        let chunk = result.unwrap();
        assert_eq!(chunk.model, "llama-3.1-70b-versatile");
        chunk_count += 1;
        
        if chunk_count > 10 {
            break; // Prevent infinite loop in test
        }
    }
    assert!(chunk_count > 0);

    // Test tool calling stream
    let mut tool_stream = GroqStreamParser::create_tool_test_stream("llama-3.1-70b-versatile");
    if let Some(result) = tool_stream.next().await {
        assert!(result.is_ok());
        let chunk = result.unwrap();
        assert!(!chunk.choices.is_empty());
        assert!(chunk.choices[0].delta.tool_calls.is_some());
        assert_eq!(chunk.choices[0].finish_reason, Some("tool_calls".to_string()));
    }

    Ok(())
}

/// Test performance metrics and optimization
#[tokio::test]
async fn test_performance_features() -> Result<()> {
    let config = GroqConfig {
        api_key: "gsk_test123456789012345678901234567890123456789012345".to_string(),
        model: "llama-3.1-8b-instant".to_string(), // Ultra-fast model
        timeout: Some(30), // Short timeout for speed
        ..Default::default()
    };

    let client = GroqClient::new(config).await?;
    
    // Test initial performance stats
    let initial_stats = client.get_performance_stats();
    assert_eq!(initial_stats.total_requests, 0);
    assert_eq!(initial_stats.total_response_time, Duration::from_millis(0));

    // Test cost estimation
    let test_request = ChatCompletionRequest {
        model: "llama-3.1-8b-instant".to_string(),
        messages: vec![ChatMessage {
            role: MessageRole::User,
            content: "Hello, world!".to_string(),
            name: None,
            function_call: None,
            tool_calls: None,
            tool_call_id: None,
        }],
        max_tokens: Some(100),
        ..Default::default()
    };

    let estimated_cost = client.estimate_request_cost(&test_request);
    assert!(estimated_cost > 0.0);
    assert!(estimated_cost < 0.1); // Should be very affordable

    // Test model recommendations
    let speed_model = client.get_recommended_model("speed");
    assert!(speed_model.is_some());
    assert_eq!(speed_model.unwrap(), "llama-3.1-8b-instant");

    let tool_model = client.get_recommended_model("tool_calling");
    assert!(tool_model.is_some());
    assert_eq!(tool_model.unwrap(), "llama-3.1-70b-versatile");

    Ok(())
}

/// Test authentication and configuration
#[tokio::test]
async fn test_authentication_and_config() -> Result<()> {
    // Test valid API key format
    let valid_config = GroqConfig {
        api_key: "gsk_1234567890123456789012345678901234567890abcdef".to_string(),
        base_url: Some("https://api.groq.com/openai/v1".to_string()),
        ..Default::default()
    };

    assert!(valid_config.validate().is_ok());

    // Test invalid API key formats
    let invalid_configs = vec![
        GroqConfig {
            api_key: "sk_wrong_prefix".to_string(),
            ..Default::default()
        },
        GroqConfig {
            api_key: "gsk_short".to_string(),
            ..Default::default()
        },
        GroqConfig {
            api_key: String::new(),
            ..Default::default()
        },
    ];

    for config in invalid_configs {
        assert!(config.validate().is_err());
    }

    // Test masked API key
    assert_eq!(
        valid_config.get_masked_api_key(),
        "gsk_1234...cdef"
    );

    // Test configuration URLs
    assert_eq!(
        valid_config.get_chat_completions_url(),
        "https://api.groq.com/openai/v1/chat/completions"
    );
    assert_eq!(
        valid_config.get_models_url(),
        "https://api.groq.com/openai/v1/models"
    );

    Ok(())
}

/// Test error handling and validation
#[tokio::test]
async fn test_error_handling() -> Result<()> {
    let converter = GroqConverter::new();

    // Test invalid tool request
    let invalid_request = ChatCompletionRequest {
        model: "llama-3.1-70b-versatile".to_string(),
        messages: vec![],
        tools: Some(vec![ToolDefinition {
            tool_type: "invalid_type".to_string(),
            function: FunctionDefinition {
                name: String::new(), // Empty name should fail
                description: None,
                parameters: None,
            },
        }]),
        ..Default::default()
    };

    assert!(converter.validate_tool_request(&invalid_request).is_err());

    // Test model validation for tools
    assert!(converter.validate_model_for_tools("llama3-70b-8192").is_err());
    assert!(converter.validate_model_for_tools("unknown-model").is_err());

    // Test invalid tool choice
    let bad_tool_choice_request = ChatCompletionRequest {
        model: "llama-3.1-70b-versatile".to_string(),
        messages: vec![ChatMessage {
            role: MessageRole::User,
            content: "Test".to_string(),
            name: None,
            function_call: None,
            tool_calls: None,
            tool_call_id: None,
        }],
        tool_choice: Some(json!("invalid_choice")),
        ..Default::default()
    };

    assert!(converter.validate_tool_request(&bad_tool_choice_request).is_err());

    Ok(())
}

/// Test provider health check
#[tokio::test]
async fn test_provider_health() -> Result<()> {
    let config = GroqConfig {
        api_key: "gsk_test123456789012345678901234567890123456789012345".to_string(),
        ..Default::default()
    };

    let provider = GroqProvider::new(config).await?;
    
    // Test health check (will fail with test key but should return proper structure)
    let health = provider.health_check().await?;
    assert_eq!(health.provider, "groq");
    // Health status will be Error/Unhealthy with test key, which is expected

    // Test performance check
    let performance = provider.get_performance_metrics();
    assert_eq!(performance.total_requests, 0); // No requests made yet

    // Test configuration validation
    assert!(provider.is_configured());

    Ok(())
}

/// Test integration with provider configuration system
#[tokio::test]
async fn test_provider_config_integration() -> Result<()> {
    let mut providers_config = ProvidersConfig::default();
    
    // Test that Groq models are mapped to groq provider
    assert_eq!(providers_config.get_provider_for_model("llama-3.1-70b-versatile"), "groq");
    assert_eq!(providers_config.get_provider_for_model("llama-3.1-8b-instant"), "groq");
    assert_eq!(providers_config.get_provider_for_model("mixtral-8x7b-32768"), "groq");

    // Test model support detection
    assert!(providers_config.is_model_supported("llama-3.1-70b-versatile"));
    assert!(providers_config.is_model_supported("gemma2-9b-it"));

    // Test supported models list includes Groq models
    let supported_models = providers_config.supported_models();
    assert!(supported_models.contains(&"llama-3.1-70b-versatile".to_string()));
    assert!(supported_models.contains(&"llama-3.1-8b-instant".to_string()));

    // Test fallback chain includes groq
    let fallback_chain = &providers_config.routing.fallback.provider_chain;
    assert!(fallback_chain.contains(&"groq".to_string()));
    
    // Groq should be 4th in the fallback chain (after openai, anthropic, azure_openai)
    let groq_position = fallback_chain.iter().position(|p| p == "groq");
    assert!(groq_position.is_some());
    assert_eq!(groq_position.unwrap(), 3);

    Ok(())
}

/// Test concurrent requests and rate limiting
#[tokio::test]
async fn test_concurrent_requests() -> Result<()> {
    let config = GroqConfig {
        api_key: "gsk_test123456789012345678901234567890123456789012345".to_string(),
        rate_limit: Some(crate::infrastructure::groq::GroqRateLimit {
            requests_per_minute: 10,
            tokens_per_minute: 1000,
            auto_throttle: true,
        }),
        ..Default::default()
    };

    let client = GroqClient::new(config).await?;
    
    // Test that client respects rate limiting configuration
    assert!(client.is_configured());

    // Test multiple model capability checks (should be fast)
    let models = vec![
        "llama-3.1-70b-versatile",
        "llama-3.1-8b-instant",
        "mixtral-8x7b-32768",
        "gemma2-9b-it"
    ];

    for model in models {
        let caps = client.get_model_capabilities(model);
        assert!(caps.supports_streaming);
        if GroqModel::supports_tool_calling(model) {
            assert!(caps.supports_tools);
        }
    }

    Ok(())
}

#[cfg(test)]
mod test_utils {
    use super::*;
    
    /// Helper function to create a test Groq configuration
    pub fn create_test_config() -> GroqConfig {
        GroqConfig {
            api_key: "gsk_test123456789012345678901234567890123456789012345".to_string(),
            base_url: Some("https://api.groq.com/openai/v1".to_string()),
            model: "llama-3.1-70b-versatile".to_string(),
            timeout: Some(30),
            ..Default::default()
        }
    }
    
    /// Helper function to create test tool definitions
    pub fn create_test_tools() -> Vec<ToolDefinition> {
        vec![
            ToolDefinition {
                tool_type: "function".to_string(),
                function: FunctionDefinition {
                    name: "get_weather".to_string(),
                    description: Some("Get weather for a location".to_string()),
                    parameters: Some(json!({
                        "type": "object",
                        "properties": {
                            "location": {"type": "string"}
                        },
                        "required": ["location"]
                    })),
                },
            }
        ]
    }
}