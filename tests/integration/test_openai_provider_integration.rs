//! Integration tests for OpenAI provider with tool calling infrastructure
//! 
//! This module tests the complete integration of the OpenAI provider with:
//! - Tool calling infrastructure
//! - Streaming support
//! - Provider fallback system
//! - Configuration management

use anyhow::Result;
use serde_json::json;
use std::sync::Arc;
use tokio_test;

use crate::infrastructure::openai::{
    OpenAIClient, OpenAIConfig, OpenAIProvider, OpenAIConverter, OpenAIProviderFactory
};
use crate::infrastructure::common::tools::{ToolCallManager, UnifiedToolCall, ToolCallResult};
use crate::models::request::ChatCompletionRequest;
use crate::models::common::{ChatMessage, MessageRole, ToolDefinition, FunctionDefinition};
use crate::config::providers::{ProvidersConfig, OpenAIConfig as ConfigOpenAIConfig};
use crate::features::provider_fallback::models::{Provider, ProviderCapability};

/// Test OpenAI provider basic functionality
#[tokio::test]
async fn test_openai_provider_creation() -> Result<()> {
    let config = OpenAIConfig {
        api_key: "sk-test123456789012345678901234567890123456789012345".to_string(),
        base_url: Some("https://api.openai.com/v1".to_string()),
        ..Default::default()
    };

    // Test client creation
    let client = OpenAIClient::new(config.clone()).await?;
    assert!(client.supports_tool_calling("gpt-4o"));
    assert!(client.supports_streaming("gpt-4o"));

    // Test provider creation
    let provider = OpenAIProvider::new(config).await?;
    let capabilities = provider.get_capabilities();
    
    assert_eq!(capabilities.provider, "openai");
    assert!(capabilities.supports_chat);
    assert!(capabilities.supports_tools);
    assert!(capabilities.supports_streaming);
    
    Ok(())
}

/// Test OpenAI provider factory
#[tokio::test]
async fn test_openai_provider_factory() -> Result<()> {
    // Test creation with API key
    let provider = OpenAIProviderFactory::create_with_api_key(
        "sk-test123456789012345678901234567890123456789012345".to_string()
    ).await?;
    
    let config = provider.config();
    assert!(!config.api_key.is_empty());
    assert_eq!(config.get_base_url(), "https://api.openai.com/v1");
    
    Ok(())
}

/// Test tool calling integration with OpenAI converter
#[tokio::test]
async fn test_tool_calling_integration() -> Result<()> {
    let converter = OpenAIConverter::new();

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
    ];

    // Test tool conversion
    let converted_tools = converter.convert_tools(&tools)?;
    assert!(converted_tools.is_array());
    assert_eq!(converted_tools.as_array().unwrap().len(), 1);

    // Test request validation
    let request = ChatCompletionRequest {
        model: "gpt-4o".to_string(),
        messages: vec![
            ChatMessage {
                role: MessageRole::User,
                content: "What's the weather in San Francisco?".to_string(),
                name: None,
                function_call: None,
                tool_calls: None,
                tool_call_id: None,
            }
        ],
        tools: Some(tools),
        tool_choice: Some(json!("auto")),
        ..Default::default()
    };

    let validation_result = converter.validate_tool_request(&request);
    assert!(validation_result.is_ok());

    Ok(())
}

/// Test unified tool call conversion
#[tokio::test]
async fn test_unified_tool_call_conversion() -> Result<()> {
    let converter = OpenAIConverter::new();
    let tool_manager = ToolCallManager::openai();

    // Test unified tool call creation
    let unified_calls = vec![
        UnifiedToolCall {
            id: "call_123".to_string(),
            function_name: "get_weather".to_string(),
            arguments: json!({
                "location": "San Francisco",
                "unit": "celsius"
            }),
            metadata: std::collections::HashMap::new(),
        },
    ];

    // Test conversion to OpenAI format
    let openai_calls = converter.unified_to_openai_calls(&unified_calls);
    assert_eq!(openai_calls.len(), 1);

    let call = &openai_calls[0];
    assert!(call.get("id").is_some());
    assert_eq!(call.get("type").unwrap(), "function");

    // Test tool result creation
    let tool_results = vec![
        ToolCallResult {
            tool_call_id: "call_123".to_string(),
            content: "The weather in San Francisco is 22°C and sunny.".to_string(),
            success: true,
            error: None,
        },
    ];

    let result_messages = converter.convert_tool_results(&tool_results)?;
    assert_eq!(result_messages.len(), 1);
    assert_eq!(result_messages[0].role, MessageRole::Tool);

    Ok(())
}

/// Test model capabilities detection
#[tokio::test]
async fn test_model_capabilities() -> Result<()> {
    let converter = OpenAIConverter::new();

    // Test GPT-4o capabilities (modern model with parallel tools)
    let gpt4o_caps = converter.get_model_capabilities("gpt-4o");
    assert!(gpt4o_caps.supports_tools);
    assert!(gpt4o_caps.supports_parallel_tools);
    assert!(gpt4o_caps.supports_streaming);

    // Test GPT-3.5-turbo capabilities (legacy model with sequential tools)
    let gpt35_caps = converter.get_model_capabilities("gpt-3.5-turbo");
    assert!(gpt35_caps.supports_tools);
    assert!(!gpt35_caps.supports_parallel_tools); // Sequential only
    assert!(gpt35_caps.supports_streaming);

    // Test unknown model capabilities
    let unknown_caps = converter.get_model_capabilities("unknown-model");
    assert!(!unknown_caps.supports_tools);
    assert!(!unknown_caps.supports_parallel_tools);

    Ok(())
}

/// Test provider configuration integration
#[tokio::test]
async fn test_provider_config_integration() -> Result<()> {
    let mut providers_config = ProvidersConfig::default();
    
    // Test OpenAI config is included
    assert!(!providers_config.openai.api_key.is_empty() || std::env::var("OPENAI_API_KEY").is_err());
    
    // Test model mapping includes OpenAI models
    assert_eq!(providers_config.get_provider_for_model("gpt-4o"), "openai");
    assert_eq!(providers_config.get_provider_for_model("gpt-3.5-turbo"), "openai");
    
    // Test model support detection
    assert!(providers_config.is_model_supported("gpt-4o"));
    assert!(providers_config.is_model_supported("gpt-4o-mini"));
    
    // Test OpenAI model configuration
    if let Some(config) = providers_config.get_openai_model_config("gpt-4o") {
        assert_eq!(config.model_name, "gpt-4o");
        assert!(config.timeout_seconds > 0);
    }

    Ok(())
}

/// Test provider fallback integration
#[tokio::test]
async fn test_provider_fallback_integration() -> Result<()> {
    // Test that OpenAI is included in provider enum
    let all_providers = Provider::all();
    assert!(all_providers.contains(&Provider::OpenAI));
    
    // Test OpenAI provider properties
    assert_eq!(Provider::OpenAI.name(), "openai");
    assert_eq!(Provider::OpenAI.display_name(), "OpenAI");
    assert_eq!(Provider::OpenAI.base_url(), "https://api.openai.com/v1");
    assert_eq!(Provider::OpenAI.default_model(), "gpt-4o");
    
    // Test OpenAI model support
    assert!(Provider::OpenAI.supports_model("gpt-4o"));
    assert!(Provider::OpenAI.supports_model("gpt-3.5-turbo"));
    assert!(!Provider::OpenAI.supports_model("claude-3-5-sonnet@20241022"));
    
    // Test OpenAI capabilities
    let capabilities = ProviderCapability::for_provider(Provider::OpenAI);
    assert!(capabilities.streaming);
    assert!(capabilities.tool_calling);
    assert_eq!(capabilities.max_tokens, 128_000);
    
    Ok(())
}

/// Test tool call context and continuation
#[tokio::test]
async fn test_tool_call_context() -> Result<()> {
    let converter = OpenAIConverter::new();

    let messages = vec![
        ChatMessage {
            role: MessageRole::User,
            content: "What's the weather like?".to_string(),
            name: None,
            function_call: None,
            tool_calls: None,
            tool_call_id: None,
        }
    ];

    let tool_calls = vec![
        UnifiedToolCall {
            id: "call_123".to_string(),
            function_name: "get_weather".to_string(),
            arguments: json!({"location": "San Francisco"}),
            metadata: std::collections::HashMap::new(),
        }
    ];

    // Create context
    let context = crate::infrastructure::openai::ToolCallContext::new(
        "req_123".to_string(),
        "gpt-4o".to_string(),
        tool_calls,
        messages,
    );

    // Test continuation request creation
    let tool_results = vec![
        ToolCallResult {
            tool_call_id: "call_123".to_string(),
            content: "It's sunny and 22°C".to_string(),
            success: true,
            error: None,
        }
    ];

    let continuation_request = context.create_continuation_request(&converter, tool_results)?;
    assert_eq!(continuation_request.model, "gpt-4o");
    assert!(continuation_request.messages.len() > 1); // Original + tool result

    Ok(())
}

/// Test streaming integration (mock test since we can't make real API calls)
#[tokio::test]
async fn test_streaming_integration() -> Result<()> {
    use crate::infrastructure::openai::streaming::{OpenAIStreamParser, OpenAIStreamUtils};

    let parser = OpenAIStreamParser::new();
    
    // Test creating a test stream
    let test_stream = OpenAIStreamParser::create_test_stream();
    let chunks = OpenAIStreamUtils::collect_chunks(test_stream).await?;
    
    assert!(chunks.len() > 0);
    
    let content = OpenAIStreamUtils::extract_content(&chunks);
    assert_eq!(content, "Hello world!");
    
    assert!(OpenAIStreamUtils::is_stream_finished(&chunks));
    assert_eq!(OpenAIStreamUtils::get_finish_reason(&chunks), Some("stop".to_string()));
    
    Ok(())
}

/// Test error handling and validation
#[tokio::test]
async fn test_error_handling() -> Result<()> {
    let converter = OpenAIConverter::new();

    // Test invalid tool type validation
    let invalid_request = ChatCompletionRequest {
        model: "gpt-4o".to_string(),
        messages: vec![],
        tools: Some(vec![
            ToolDefinition {
                tool_type: "invalid_type".to_string(),
                function: FunctionDefinition {
                    name: "test".to_string(),
                    description: None,
                    parameters: None,
                },
            }
        ]),
        ..Default::default()
    };

    let validation_result = converter.validate_tool_request(&invalid_request);
    assert!(validation_result.is_err());

    // Test empty function name validation
    let empty_name_request = ChatCompletionRequest {
        model: "gpt-4o".to_string(),
        messages: vec![],
        tools: Some(vec![
            ToolDefinition {
                tool_type: "function".to_string(),
                function: FunctionDefinition {
                    name: "".to_string(),
                    description: None,
                    parameters: None,
                },
            }
        ]),
        ..Default::default()
    };

    let validation_result = converter.validate_tool_request(&empty_name_request);
    assert!(validation_result.is_err());

    Ok(())
}

/// Integration test for the complete workflow
#[tokio::test]
async fn test_complete_workflow_integration() -> Result<()> {
    // This test verifies the complete integration without making actual API calls
    
    // 1. Create provider
    let provider = OpenAIProviderFactory::create_with_api_key(
        "sk-test123456789012345678901234567890123456789012345".to_string()
    ).await?;

    // 2. Test health check (will fail without real API key, but shouldn't panic)
    let health = provider.health_check().await;
    assert!(health.is_ok()); // Should return health status, not error

    // 3. Test model listing
    let models = provider.get_supported_models().await?;
    assert!(models.len() > 0);
    assert!(models.iter().any(|m| m.name == "gpt-4o"));

    // 4. Test capabilities
    let capabilities = provider.get_capabilities();
    assert!(capabilities.supports_tools);
    assert!(capabilities.supports_streaming);

    // 5. Test converter integration
    let converter = provider.converter();
    let test_tools = vec![
        ToolDefinition {
            tool_type: "function".to_string(),
            function: FunctionDefinition {
                name: "test_function".to_string(),
                description: Some("Test function".to_string()),
                parameters: Some(json!({"type": "object"})),
            },
        }
    ];

    let converted = converter.convert_tools(&test_tools)?;
    assert!(converted.is_array());

    Ok(())
}

/// Test configuration validation and defaults
#[tokio::test]
async fn test_configuration_validation() -> Result<()> {
    // Test default configuration
    let default_config = OpenAIConfig::default();
    assert_eq!(default_config.get_base_url(), "https://api.openai.com/v1");
    
    // Test configuration from mock environment
    std::env::set_var("OPENAI_API_KEY", "sk-test123");
    std::env::set_var("OPENAI_BASE_URL", "https://custom.openai.com/v1");
    std::env::set_var("OPENAI_DEFAULT_MODEL", "gpt-3.5-turbo");
    
    let env_config = OpenAIConfig::from_env()?;
    assert_eq!(env_config.api_key, "sk-test123");
    assert_eq!(env_config.base_url, Some("https://custom.openai.com/v1".to_string()));
    
    // Clean up environment
    std::env::remove_var("OPENAI_API_KEY");
    std::env::remove_var("OPENAI_BASE_URL");
    std::env::remove_var("OPENAI_DEFAULT_MODEL");
    
    Ok(())
}