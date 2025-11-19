//! Integration tests for Azure OpenAI provider with tool calling infrastructure
//! 
//! This module tests the complete integration of the Azure OpenAI provider with:
//! - Tool calling infrastructure
//! - Streaming support
//! - Provider fallback system
//! - Configuration management
//! - Azure-specific features (deployments, Azure AD auth)

use anyhow::Result;
use serde_json::json;
use std::sync::Arc;
use tokio_test;

use crate::infrastructure::azure_openai::{
    AzureOpenAIClient, AzureOpenAIConfig, AzureOpenAIProvider, AzureOpenAIConverter, 
    AzureOpenAIProviderFactory, AzureOpenAIAuth
};
use crate::infrastructure::common::tools::{ToolCallManager, UnifiedToolCall, ToolCallResult};
use crate::models::request::ChatCompletionRequest;
use crate::models::common::{ChatMessage, MessageRole, ToolDefinition, FunctionDefinition};
use crate::config::providers::{ProvidersConfig, AzureOpenAIConfig as ConfigAzureOpenAIConfig};
use crate::features::provider_fallback::models::{Provider, ProviderCapability};

/// Test Azure OpenAI provider basic functionality
#[tokio::test]
async fn test_azure_openai_provider_creation() -> Result<()> {
    let config = AzureOpenAIConfig {
        api_key: "test-api-key-1234567890123456789012345678901234567890".to_string(),
        endpoint: "https://test-resource.openai.azure.com".to_string(),
        deployment: "gpt-4o".to_string(),
        api_version: "2024-02-15-preview".to_string(),
        ..Default::default()
    };

    // Test client creation
    let client = AzureOpenAIClient::new(config.clone()).await?;
    assert!(client.supports_tool_calling("gpt-4o"));
    assert!(client.supports_streaming("gpt-4o"));
    assert_eq!(client.get_resource_name(), Some("test-resource".to_string()));

    // Test provider creation
    let provider = AzureOpenAIProvider::new(config).await?;
    let capabilities = provider.get_capabilities();
    
    assert_eq!(capabilities.provider, "azure_openai");
    assert!(capabilities.supports_chat);
    assert!(capabilities.supports_tools);
    assert!(capabilities.supports_streaming);
    
    Ok(())
}

/// Test Azure OpenAI provider factory
#[tokio::test]
async fn test_azure_openai_provider_factory() -> Result<()> {
    // Test creation with credentials
    let provider = AzureOpenAIProviderFactory::create_with_credentials(
        "test-api-key-1234567890123456789012345678901234567890".to_string(),
        "https://test-resource.openai.azure.com".to_string(),
        "gpt-4o".to_string()
    ).await?;
    
    let config = provider.config();
    assert!(!config.api_key.is_empty());
    assert_eq!(config.endpoint, "https://test-resource.openai.azure.com");
    assert_eq!(config.deployment, "gpt-4o");
    assert_eq!(config.api_version, "2024-02-15-preview");
    
    Ok(())
}

/// Test Azure OpenAI configuration validation
#[tokio::test]
async fn test_azure_openai_config_validation() -> Result<()> {
    // Valid configuration
    let valid_config = AzureOpenAIConfig {
        api_key: "test-api-key-1234567890123456789012345678901234567890".to_string(),
        endpoint: "https://test-resource.openai.azure.com".to_string(),
        deployment: "gpt-4o".to_string(),
        api_version: "2024-02-15-preview".to_string(),
        ..Default::default()
    };
    assert!(valid_config.validate().is_ok());

    // Invalid API key
    let invalid_api_key_config = AzureOpenAIConfig {
        api_key: "short".to_string(),
        endpoint: "https://test-resource.openai.azure.com".to_string(),
        deployment: "gpt-4o".to_string(),
        ..Default::default()
    };
    assert!(invalid_api_key_config.validate().is_err());

    // Invalid endpoint
    let invalid_endpoint_config = AzureOpenAIConfig {
        api_key: "test-api-key-1234567890123456789012345678901234567890".to_string(),
        endpoint: "http://invalid-endpoint".to_string(), // Not HTTPS
        deployment: "gpt-4o".to_string(),
        ..Default::default()
    };
    assert!(invalid_endpoint_config.validate().is_err());

    // Empty deployment
    let invalid_deployment_config = AzureOpenAIConfig {
        api_key: "test-api-key-1234567890123456789012345678901234567890".to_string(),
        endpoint: "https://test-resource.openai.azure.com".to_string(),
        deployment: "".to_string(),
        ..Default::default()
    };
    assert!(invalid_deployment_config.validate().is_err());

    Ok(())
}

/// Test tool calling integration with Azure OpenAI converter
#[tokio::test]
async fn test_azure_openai_tool_calling_integration() -> Result<()> {
    let converter = AzureOpenAIConverter::new();

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
                name: "calculate_sum".to_string(),
                description: Some("Calculate the sum of two numbers".to_string()),
                parameters: Some(json!({
                    "type": "object",
                    "properties": {
                        "a": {"type": "number"},
                        "b": {"type": "number"}
                    },
                    "required": ["a", "b"]
                })),
            },
        },
    ];

    // Test tool conversion (should be pass-through for Azure OpenAI)
    let converted_tools = converter.convert_tools(&tools)?;
    assert!(converted_tools.is_array());
    assert_eq!(converted_tools.as_array().unwrap().len(), 2);

    // Test request validation
    let test_request = ChatCompletionRequest {
        model: "gpt-4o".to_string(),
        messages: vec![
            ChatMessage {
                role: MessageRole::User,
                content: "What's the weather in Paris?".to_string(),
                name: None,
                function_call: None,
                tool_calls: None,
                tool_call_id: None,
            }
        ],
        tools: Some(tools.clone()),
        tool_choice: Some(json!("auto")),
        ..Default::default()
    };

    assert!(converter.validate_tool_request(&test_request).is_ok());

    // Test invalid tool request
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

    assert!(converter.validate_tool_request(&invalid_request).is_err());

    Ok(())
}

/// Test Azure OpenAI model capabilities detection
#[tokio::test]
async fn test_azure_openai_model_capabilities() -> Result<()> {
    let converter = AzureOpenAIConverter::new();

    // Test GPT-4o capabilities
    let gpt4o_caps = converter.get_model_capabilities("gpt-4o-deployment");
    assert!(gpt4o_caps.supports_tools);
    assert!(gpt4o_caps.supports_parallel_tools);
    assert!(gpt4o_caps.supports_streaming);
    assert_eq!(gpt4o_caps.max_tool_calls, Some(16));

    // Test GPT-4 capabilities
    let gpt4_caps = converter.get_model_capabilities("gpt-4-turbo-deployment");
    assert!(gpt4_caps.supports_tools);
    assert!(gpt4_caps.supports_parallel_tools);
    assert!(gpt4_caps.supports_streaming);

    // Test GPT-3.5 capabilities (legacy)
    let gpt35_caps = converter.get_model_capabilities("gpt-35-turbo");
    assert!(gpt35_caps.supports_tools);
    assert!(!gpt35_caps.supports_parallel_tools); // Sequential only
    assert!(gpt35_caps.supports_streaming);
    assert_eq!(gpt35_caps.max_tool_calls, Some(1));

    // Test unknown deployment
    let unknown_caps = converter.get_model_capabilities("unknown-deployment");
    assert!(!unknown_caps.supports_tools);
    assert!(!unknown_caps.supports_parallel_tools);

    Ok(())
}

/// Test Azure OpenAI deployment mappings
#[tokio::test]
async fn test_azure_openai_deployment_mappings() -> Result<()> {
    let mut mappings = std::collections::HashMap::new();
    mappings.insert("gpt-4".to_string(), "my-gpt4-deployment".to_string());
    mappings.insert("gpt-4o".to_string(), "my-gpt4o-deployment".to_string());

    let config = AzureOpenAIConfig {
        api_key: "test-api-key-1234567890123456789012345678901234567890".to_string(),
        endpoint: "https://test-resource.openai.azure.com".to_string(),
        deployment: "default-deployment".to_string(),
        deployment_mappings: Some(mappings),
        ..Default::default()
    };

    let client = AzureOpenAIClient::new(config).await?;

    // Test model to deployment mapping
    assert_eq!(client.map_model_to_deployment("gpt-4"), "my-gpt4-deployment");
    assert_eq!(client.map_model_to_deployment("gpt-4o"), "my-gpt4o-deployment");
    assert_eq!(client.map_model_to_deployment("unknown-model"), "default-deployment");

    Ok(())
}

/// Test Azure OpenAI authentication
#[tokio::test]
async fn test_azure_openai_authentication() -> Result<()> {
    let config = AzureOpenAIConfig {
        api_key: "test-api-key-1234567890123456789012345678901234567890".to_string(),
        endpoint: "https://test-resource.openai.azure.com".to_string(),
        deployment: "gpt-4o".to_string(),
        ..Default::default()
    };

    let auth = AzureOpenAIAuth::new(&config)?;
    
    // Test API key validation
    assert!(auth.validate_api_key().is_ok());
    assert_eq!(auth.get_masked_api_key(), "test...7890");
    assert_eq!(auth.get_auth_method(), "API Key");
    assert!(!auth.has_azure_ad_config());

    // Test header creation
    let headers = auth.create_headers()?;
    assert!(headers.contains_key("api-key"));
    assert!(headers.contains_key("content-type"));
    assert!(headers.contains_key("user-agent"));

    // Test streaming headers
    let streaming_headers = auth.create_streaming_headers()?;
    assert!(streaming_headers.contains_key("api-key"));
    assert!(streaming_headers.contains_key("accept"));
    assert_eq!(
        streaming_headers.get("accept").unwrap().to_str().unwrap(),
        "text/event-stream"
    );

    Ok(())
}

/// Test Azure OpenAI URL generation
#[tokio::test]
async fn test_azure_openai_url_generation() -> Result<()> {
    let config = AzureOpenAIConfig {
        api_key: "test-api-key-1234567890123456789012345678901234567890".to_string(),
        endpoint: "https://test-resource.openai.azure.com".to_string(),
        deployment: "gpt-4o".to_string(),
        api_version: "2024-02-15-preview".to_string(),
        ..Default::default()
    };

    // Test chat completions URL
    let chat_url = config.get_chat_completions_url();
    assert_eq!(
        chat_url,
        "https://test-resource.openai.azure.com/openai/deployments/gpt-4o/chat/completions?api-version=2024-02-15-preview"
    );

    // Test models URL (deployments endpoint)
    let models_url = config.get_models_url();
    assert_eq!(
        models_url,
        "https://test-resource.openai.azure.com/openai/deployments?api-version=2024-02-15-preview"
    );

    // Test resource name extraction
    assert_eq!(config.get_resource_name(), Some("test-resource".to_string()));

    Ok(())
}

/// Test Azure OpenAI provider health check
#[tokio::test]
async fn test_azure_openai_provider_health_check() -> Result<()> {
    let config = AzureOpenAIConfig {
        api_key: "test-api-key-1234567890123456789012345678901234567890".to_string(),
        endpoint: "https://test-resource.openai.azure.com".to_string(),
        deployment: "gpt-4o".to_string(),
        ..Default::default()
    };

    let provider = AzureOpenAIProvider::new(config).await?;
    
    // Note: This will fail in tests since we don't have a real API key/endpoint
    // But we can test that the health check method exists and returns proper structure
    let health_result = provider.health_check().await;
    
    // Should return a result (either Ok or Err, but not panic)
    assert!(health_result.is_ok());
    
    if let Ok(health) = health_result {
        assert_eq!(health.provider, "azure_openai");
        // Status will be Error or Unhealthy since we're using test credentials
        assert!(matches!(health.status, 
            crate::infrastructure::azure_openai::HealthStatus::Error | 
            crate::infrastructure::azure_openai::HealthStatus::Unhealthy
        ));
    }

    Ok(())
}

/// Test Azure OpenAI supported models
#[tokio::test]
async fn test_azure_openai_supported_models() -> Result<()> {
    let config = AzureOpenAIConfig {
        api_key: "test-api-key-1234567890123456789012345678901234567890".to_string(),
        endpoint: "https://test-resource.openai.azure.com".to_string(),
        deployment: "gpt-4o".to_string(),
        ..Default::default()
    };

    let provider = AzureOpenAIProvider::new(config).await?;
    
    // Get supported models (will fallback to configured deployment)
    let models = provider.get_supported_models().await?;
    
    assert!(!models.is_empty());
    assert!(models.iter().any(|m| m.name == "gpt-4o"));
    assert!(models.iter().all(|m| m.provider == "azure_openai"));
    assert!(models.iter().all(|m| m.supports_tools));
    assert!(models.iter().all(|m| m.supports_streaming));

    Ok(())
}

/// Test Azure OpenAI request validation
#[tokio::test]
async fn test_azure_openai_request_validation() -> Result<()> {
    let config = AzureOpenAIConfig {
        api_key: "test-api-key-1234567890123456789012345678901234567890".to_string(),
        endpoint: "https://test-resource.openai.azure.com".to_string(),
        deployment: "gpt-4o".to_string(),
        ..Default::default()
    };

    let client = AzureOpenAIClient::new(config).await?;

    // Valid request
    let valid_request = ChatCompletionRequest {
        model: "gpt-4o".to_string(),
        messages: vec![
            ChatMessage {
                role: MessageRole::User,
                content: "Hello".to_string(),
                name: None,
                function_call: None,
                tool_calls: None,
                tool_call_id: None,
            }
        ],
        max_tokens: Some(100),
        ..Default::default()
    };

    assert!(client.validate_request(&valid_request).is_ok());

    // Invalid request - too many tokens
    let invalid_request = ChatCompletionRequest {
        model: "gpt-4o".to_string(),
        messages: vec![],
        max_tokens: Some(200000), // Exceeds limit
        ..Default::default()
    };

    assert!(client.validate_request(&invalid_request).is_err());

    Ok(())
}

/// Test Azure OpenAI tool result processing
#[tokio::test]
async fn test_azure_openai_tool_result_processing() -> Result<()> {
    let converter = AzureOpenAIConverter::new();

    // Create test tool results
    let tool_results = vec![
        ToolCallResult {
            tool_call_id: "call_123".to_string(),
            content: "The weather in Paris is 20°C and sunny".to_string(),
            success: true,
            error: None,
        },
        ToolCallResult {
            tool_call_id: "call_456".to_string(),
            content: "The sum is 42".to_string(),
            success: true,
            error: None,
        },
    ];

    // Convert to chat messages
    let result_messages = converter.convert_tool_results(&tool_results)?;
    
    assert_eq!(result_messages.len(), 2);
    assert!(result_messages.iter().all(|m| m.role == MessageRole::Tool));
    assert_eq!(result_messages[0].tool_call_id, Some("call_123".to_string()));
    assert_eq!(result_messages[1].tool_call_id, Some("call_456".to_string()));
    assert_eq!(result_messages[0].content, "The weather in Paris is 20°C and sunny");
    assert_eq!(result_messages[1].content, "The sum is 42");

    Ok(())
}

/// Test Azure OpenAI configuration from environment
#[tokio::test]
async fn test_azure_openai_config_from_env() -> Result<()> {
    // Set test environment variables
    std::env::set_var("AZURE_OPENAI_API_KEY", "test-key-1234567890123456789012345678901234567890");
    std::env::set_var("AZURE_OPENAI_ENDPOINT", "https://test.openai.azure.com");
    std::env::set_var("AZURE_OPENAI_DEPLOYMENT", "test-deployment");
    std::env::set_var("AZURE_OPENAI_API_VERSION", "2024-02-15-preview");

    let config = AzureOpenAIConfig::from_env();
    assert!(config.is_ok());

    let config = config.unwrap();
    assert_eq!(config.api_key, "test-key-1234567890123456789012345678901234567890");
    assert_eq!(config.endpoint, "https://test.openai.azure.com");
    assert_eq!(config.deployment, "test-deployment");
    assert_eq!(config.api_version, "2024-02-15-preview");

    // Clean up environment variables
    std::env::remove_var("AZURE_OPENAI_API_KEY");
    std::env::remove_var("AZURE_OPENAI_ENDPOINT");
    std::env::remove_var("AZURE_OPENAI_DEPLOYMENT");
    std::env::remove_var("AZURE_OPENAI_API_VERSION");

    Ok(())
}

/// Test Azure OpenAI provider integration with provider config
#[tokio::test]
async fn test_azure_openai_provider_config_integration() -> Result<()> {
    let mut providers_config = ProvidersConfig::default();

    // Test Azure OpenAI config integration
    assert!(providers_config.get_azure_openai_model_config("gpt-4o").is_some());
    assert!(providers_config.is_model_supported("azure-gpt-4o"));
    
    let deployment = providers_config.get_azure_openai_deployment("gpt-4o");
    assert!(deployment.is_some());

    // Test supported models includes Azure OpenAI
    let supported_models = providers_config.supported_models();
    assert!(supported_models.iter().any(|m| m.contains("azure")));

    Ok(())
}

/// Test Azure OpenAI streaming parser
#[tokio::test]
async fn test_azure_openai_streaming_parser() -> Result<()> {
    use crate::infrastructure::azure_openai::streaming::{AzureOpenAIStreamParser, AzureOpenAIStreamUtils};
    use tokio_stream::StreamExt;

    let parser = AzureOpenAIStreamParser::new();

    // Test stream creation
    let test_stream = AzureOpenAIStreamParser::create_test_stream();
    let chunks: Vec<_> = test_stream.collect().await;
    
    assert!(!chunks.is_empty());
    assert!(chunks.iter().all(|chunk| chunk.is_ok()));

    // Test tool call stream
    let tool_stream = AzureOpenAIStreamParser::create_test_tool_call_stream();
    let tool_chunks: Vec<_> = tool_stream.collect().await;
    
    assert!(!tool_chunks.is_empty());
    assert!(tool_chunks.iter().any(|chunk| {
        if let Ok(chunk) = chunk {
            parser.chunk_has_tool_calls(chunk)
        } else {
            false
        }
    }));

    Ok(())
}

/// Test Azure OpenAI error handling
#[tokio::test]
async fn test_azure_openai_error_handling() -> Result<()> {
    // Test with invalid configuration
    let invalid_config = AzureOpenAIConfig {
        api_key: "invalid".to_string(),
        endpoint: "invalid".to_string(),
        deployment: "".to_string(),
        ..Default::default()
    };

    // Should fail validation
    assert!(invalid_config.validate().is_err());

    // Client creation should fail with invalid config
    let client_result = AzureOpenAIClient::new(invalid_config).await;
    assert!(client_result.is_err());

    Ok(())
}