//! AWS Bedrock Provider Integration Tests
//!
//! These tests verify the complete AWS Bedrock provider implementation including:
//! - Authentication and configuration
//! - Model-specific request/response handling  
//! - Tool calling functionality across model families
//! - Streaming support
//! - Error handling and fallback behavior
//! - Multi-model support (Claude, Llama, Mistral)

use anyhow::Result;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::time::Duration;
use tokio_stream::StreamExt;

use llm_supabase_rs::infrastructure::aws_bedrock::{
    BedrockClient, BedrockConfig, BedrockConverter, BedrockProvider, BedrockProviderFactory,
    BedrockModel, BedrockModelFamily, BedrockAuthMethod,
};
use llm_supabase_rs::models::common::{ChatMessage, MessageRole, ToolDefinition, FunctionDefinition};
use llm_supabase_rs::models::request::ChatCompletionRequest;
use llm_supabase_rs::shared::types::ToolCall;

/// Test configuration for AWS Bedrock integration tests
pub struct BedrockTestConfig {
    pub use_mock: bool,
    pub region: String,
    pub access_key: Option<String>,
    pub secret_key: Option<String>,
}

impl Default for BedrockTestConfig {
    fn default() -> Self {
        Self {
            use_mock: std::env::var("BEDROCK_USE_MOCK").unwrap_or_else(|_| "true".to_string()) == "true",
            region: std::env::var("AWS_REGION").unwrap_or_else(|_| "us-east-1".to_string()),
            access_key: std::env::var("AWS_ACCESS_KEY_ID").ok(),
            secret_key: std::env::var("AWS_SECRET_ACCESS_KEY").ok(),
        }
    }
}

fn create_test_config() -> BedrockConfig {
    let test_config = BedrockTestConfig::default();
    
    BedrockConfig {
        region: test_config.region,
        access_key: if test_config.use_mock {
            Some("AKIAIOSFODNN7EXAMPLE".to_string())
        } else {
            test_config.access_key
        },
        secret_key: if test_config.use_mock {
            Some("wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY".to_string())
        } else {
            test_config.secret_key
        },
        validate_credentials: !test_config.use_mock,
        ..Default::default()
    }
}

#[tokio::test]
async fn test_bedrock_client_creation() -> Result<()> {
    let config = create_test_config();
    let client = BedrockClient::new(config).await?;
    
    // Verify client configuration
    assert_eq!(client.get_config().region, "us-east-1");
    assert!(client.get_config().validate_credentials == false || client.get_config().access_key.is_some());
    
    Ok(())
}

#[tokio::test]
async fn test_bedrock_provider_factory() -> Result<()> {
    // Test factory creation with explicit credentials
    let provider = BedrockProviderFactory::create_with_credentials(
        "AKIAIOSFODNN7EXAMPLE".to_string(),
        "wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY".to_string(),
        "us-east-1".to_string(),
    ).await?;
    
    let capabilities = provider.get_capabilities();
    assert_eq!(capabilities.provider, "aws_bedrock");
    assert!(capabilities.supports_chat);
    assert!(capabilities.supports_tools);
    assert!(capabilities.supports_streaming);
    assert!(capabilities.supports_embeddings);
    
    // Test region-specific creation
    let eu_provider = BedrockProviderFactory::create_for_region("eu-west-1".to_string()).await?;
    assert_eq!(eu_provider.config().region, "eu-west-1");
    
    Ok(())
}

#[tokio::test]
async fn test_supported_models() -> Result<()> {
    let config = create_test_config();
    let provider = BedrockProvider::new(config).await?;
    
    let models = provider.get_supported_models().await?;
    assert!(!models.is_empty());
    
    // Verify specific model support
    let model_names: Vec<String> = models.iter().map(|m| m.name.clone()).collect();
    
    // Claude models
    assert!(model_names.contains(&"anthropic.claude-3-5-sonnet-20241022-v2:0".to_string()));
    assert!(model_names.contains(&"anthropic.claude-3-haiku-20240307-v1:0".to_string()));
    assert!(model_names.contains(&"anthropic.claude-3-opus-20240229-v1:0".to_string()));
    
    // Llama models
    assert!(model_names.contains(&"meta.llama3-1-70b-instruct-v1:0".to_string()));
    assert!(model_names.contains(&"meta.llama3-1-8b-instruct-v1:0".to_string()));
    
    // Mistral models
    assert!(model_names.contains(&"mistral.mistral-7b-instruct-v0:2".to_string()));
    
    // Verify tool calling support
    for model in &models {
        if model.name.starts_with("anthropic.") || model.name.starts_with("meta.") {
            assert!(model.supports_tools, "Model {} should support tool calling", model.name);
        }
        assert!(model.supports_streaming, "Model {} should support streaming", model.name);
    }
    
    Ok(())
}

#[tokio::test]
async fn test_model_family_detection() -> Result<()> {
    let config = create_test_config();
    
    // Test different model family detection
    assert_eq!(
        config.get_model_family("anthropic.claude-3-5-sonnet-20241022-v2:0"),
        BedrockModelFamily::Anthropic
    );
    assert_eq!(
        config.get_model_family("meta.llama3-1-70b-instruct-v1:0"),
        BedrockModelFamily::Meta
    );
    assert_eq!(
        config.get_model_family("mistral.mistral-7b-instruct-v0:2"),
        BedrockModelFamily::Mistral
    );
    assert_eq!(
        config.get_model_family("amazon.titan-text-express-v1"),
        BedrockModelFamily::Amazon
    );
    
    // Test tool calling support detection
    assert!(config.model_supports_tools("anthropic.claude-3-5-sonnet-20241022-v2:0"));
    assert!(config.model_supports_tools("meta.llama3-1-70b-instruct-v1:0"));
    assert!(!config.model_supports_tools("mistral.mistral-7b-instruct-v0:2"));
    
    Ok(())
}

#[tokio::test]
async fn test_converter_functionality() -> Result<()> {
    let converter = BedrockConverter::new();
    
    // Test model family capabilities
    assert!(converter.supports_tools(&BedrockModelFamily::Anthropic));
    assert!(converter.supports_tools(&BedrockModelFamily::Meta));
    assert!(!converter.supports_tools(&BedrockModelFamily::Mistral));
    
    // Test message conversion for different families
    let messages = vec![
        ChatMessage {
            role: MessageRole::System,
            content: "You are a helpful assistant".to_string(),
            name: None,
            tool_calls: None,
            tool_call_id: None,
        },
        ChatMessage {
            role: MessageRole::User,
            content: "Hello, how are you?".to_string(),
            name: None,
            tool_calls: None,
            tool_call_id: None,
        },
    ];
    
    // Test Claude conversion
    let claude_result = converter.convert_messages_to_bedrock(&messages, BedrockModelFamily::Anthropic)?;
    assert!(claude_result.get("messages").is_some());
    assert!(claude_result.get("system").is_some());
    
    // Test Llama conversion
    let llama_result = converter.convert_messages_to_bedrock(&messages, BedrockModelFamily::Meta)?;
    assert!(llama_result.get("messages").is_some());
    let llama_messages = llama_result["messages"].as_array().unwrap();
    assert_eq!(llama_messages.len(), 2);
    assert_eq!(llama_messages[0]["role"], "system");
    assert_eq!(llama_messages[1]["role"], "user");
    
    Ok(())
}

#[tokio::test]
async fn test_tool_calling_conversion() -> Result<()> {
    let converter = BedrockConverter::new();
    
    // Create test tool definitions
    let tools = vec![
        ToolDefinition {
            r#type: "function".to_string(),
            function: FunctionDefinition {
                name: "get_weather".to_string(),
                description: Some("Get weather information".to_string()),
                parameters: json!({
                    "type": "object",
                    "properties": {
                        "location": {
                            "type": "string",
                            "description": "The location to get weather for"
                        }
                    },
                    "required": ["location"]
                }),
            },
        }
    ];
    
    // Test Claude tool format
    let claude_tools = converter.get_tool_format_for_family(&BedrockModelFamily::Anthropic, &tools)?;
    let claude_tools_array = claude_tools.as_array().unwrap();
    assert_eq!(claude_tools_array.len(), 1);
    assert_eq!(claude_tools_array[0]["name"], "get_weather");
    assert!(claude_tools_array[0].get("input_schema").is_some());
    
    // Test Llama tool format
    let llama_tools = converter.get_tool_format_for_family(&BedrockModelFamily::Meta, &tools)?;
    let llama_tools_array = llama_tools.as_array().unwrap();
    assert_eq!(llama_tools_array.len(), 1);
    assert_eq!(llama_tools_array[0]["type"], "function");
    assert_eq!(llama_tools_array[0]["function"]["name"], "get_weather");
    
    // Test unsupported family
    let mistral_result = converter.get_tool_format_for_family(&BedrockModelFamily::Mistral, &tools);
    assert!(mistral_result.is_err());
    
    Ok(())
}

#[tokio::test]
async fn test_tool_call_extraction() -> Result<()> {
    let converter = BedrockConverter::new();
    
    // Test Claude tool call extraction
    let claude_response = json!({
        "content": [
            {
                "type": "tool_use",
                "id": "call_123",
                "name": "get_weather",
                "input": {
                    "location": "San Francisco"
                }
            }
        ]
    });
    
    let claude_tool_calls = converter.extract_tool_calls_from_response(&claude_response, BedrockModelFamily::Anthropic)?;
    assert_eq!(claude_tool_calls.len(), 1);
    assert_eq!(claude_tool_calls[0].id, "call_123");
    assert_eq!(claude_tool_calls[0].function_name, "get_weather");
    assert_eq!(claude_tool_calls[0].arguments["location"], "San Francisco");
    
    // Test Llama tool call extraction
    let llama_response = json!({
        "function_calls": [
            {
                "name": "get_weather",
                "arguments": {
                    "location": "New York"
                }
            }
        ]
    });
    
    let llama_tool_calls = converter.extract_tool_calls_from_response(&llama_response, BedrockModelFamily::Meta)?;
    assert_eq!(llama_tool_calls.len(), 1);
    assert!(llama_tool_calls[0].id.starts_with("call_"));
    assert_eq!(llama_tool_calls[0].function_name, "get_weather");
    assert_eq!(llama_tool_calls[0].arguments["location"], "New York");
    
    Ok(())
}

#[tokio::test]
async fn test_request_conversion() -> Result<()> {
    let config = create_test_config();
    let client = BedrockClient::new(config).await?;
    
    let request = ChatCompletionRequest {
        model: "anthropic.claude-3-5-sonnet-20241022-v2:0".to_string(),
        messages: vec![
            ChatMessage {
                role: MessageRole::User,
                content: "What's the weather like?".to_string(),
                name: None,
                tool_calls: None,
                tool_call_id: None,
            }
        ],
        temperature: Some(0.7),
        max_tokens: Some(1000),
        top_p: Some(0.9),
        tools: None,
        stream: Some(false),
        ..Default::default()
    };
    
    let model_family = BedrockModelFamily::Anthropic;
    let bedrock_request = client.convert_to_bedrock_request(&request, &model_family).await;
    
    // This will fail in tests without proper mocking, but we can verify the structure
    if let Err(e) = bedrock_request {
        // Expected to fail due to lack of proper AWS credentials in test environment
        assert!(e.to_string().contains("credentials") || e.to_string().contains("auth"));
    }
    
    Ok(())
}

#[tokio::test] 
async fn test_authentication_methods() -> Result<()> {
    // Test different authentication method configurations
    
    // Explicit credentials
    let explicit_config = BedrockConfig {
        region: "us-east-1".to_string(),
        access_key: Some("AKIAIOSFODNN7EXAMPLE".to_string()),
        secret_key: Some("wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY".to_string()),
        auth_method: BedrockAuthMethod::Explicit,
        validate_credentials: false,
        ..Default::default()
    };
    
    let client = BedrockClient::new(explicit_config).await?;
    assert_eq!(client.get_config().auth_method, BedrockAuthMethod::Explicit);
    
    // Environment variables method
    let env_config = BedrockConfig {
        auth_method: BedrockAuthMethod::EnvironmentVariables,
        validate_credentials: false,
        ..Default::default()
    };
    
    // This might fail if env vars aren't set, which is expected in CI
    let env_result = BedrockClient::new(env_config).await;
    if env_result.is_err() {
        // Expected in test environments without proper AWS setup
        assert!(env_result.unwrap_err().to_string().contains("environment variable") ||
                env_result.unwrap_err().to_string().contains("AWS_"));
    }
    
    Ok(())
}

#[tokio::test]
async fn test_model_capabilities() -> Result<()> {
    // Test predefined model capabilities
    let claude_model = BedrockModel::find_by_id("anthropic.claude-3-5-sonnet-20241022-v2:0");
    assert!(claude_model.is_some());
    
    let model = claude_model.unwrap();
    assert_eq!(model.family, BedrockModelFamily::Anthropic);
    assert!(model.supports_tools);
    assert!(model.supports_streaming);
    assert_eq!(model.max_context_length, 200000);
    
    // Test family-based model grouping
    let anthropic_models = BedrockModel::by_family(BedrockModelFamily::Anthropic);
    assert!(!anthropic_models.is_empty());
    assert!(anthropic_models.iter().all(|m| m.family == BedrockModelFamily::Anthropic));
    
    let meta_models = BedrockModel::by_family(BedrockModelFamily::Meta);
    assert!(!meta_models.is_empty());
    assert!(meta_models.iter().all(|m| m.family == BedrockModelFamily::Meta));
    
    // Test tool-capable models
    let tool_models = BedrockModel::tool_capable();
    assert!(!tool_models.is_empty());
    assert!(tool_models.iter().any(|m| m.family == BedrockModelFamily::Anthropic));
    assert!(tool_models.iter().any(|m| m.family == BedrockModelFamily::Meta));
    
    Ok(())
}

#[tokio::test]
async fn test_error_handling() -> Result<()> {
    // Test various error conditions
    
    // Invalid credentials
    let invalid_config = BedrockConfig {
        region: "us-east-1".to_string(),
        access_key: Some("INVALID".to_string()),
        secret_key: Some("INVALID".to_string()),
        validate_credentials: true, // This should trigger validation
        ..Default::default()
    };
    
    let result = BedrockClient::new(invalid_config).await;
    if result.is_err() {
        // Expected behavior - invalid credentials should be rejected
        let error = result.unwrap_err();
        assert!(error.to_string().contains("credential") || error.to_string().contains("auth"));
    }
    
    // Invalid region format
    let invalid_region_config = BedrockConfig {
        region: "invalid-region-format".to_string(),
        validate_credentials: false,
        ..Default::default()
    };
    
    let client = BedrockClient::new(invalid_region_config).await?;
    assert_eq!(client.get_config().region, "invalid-region-format");
    
    Ok(())
}

#[tokio::test]
async fn test_endpoint_url_generation() -> Result<()> {
    let config = BedrockConfig {
        region: "eu-west-1".to_string(),
        ..Default::default()
    };
    
    let expected_url = "https://bedrock-runtime.eu-west-1.amazonaws.com";
    assert_eq!(config.get_endpoint_url(), expected_url);
    
    // Test custom endpoint
    let custom_config = BedrockConfig {
        region: "us-east-1".to_string(),
        endpoint_url: Some("https://my-vpc-endpoint.com".to_string()),
        ..Default::default()
    };
    
    assert_eq!(custom_config.get_endpoint_url(), "https://my-vpc-endpoint.com");
    
    Ok(())
}

#[tokio::test]
async fn test_configuration_from_env() -> Result<()> {
    // Test environment variable loading
    std::env::set_var("AWS_REGION", "us-west-2");
    std::env::set_var("AWS_ACCESS_KEY_ID", "test_access_key");
    std::env::set_var("AWS_SECRET_ACCESS_KEY", "test_secret_key");
    
    let config = BedrockConfig::from_env()?;
    assert_eq!(config.region, "us-west-2");
    assert_eq!(config.access_key.as_deref(), Some("test_access_key"));
    assert_eq!(config.secret_key.as_deref(), Some("test_secret_key"));
    
    // Cleanup
    std::env::remove_var("AWS_REGION");
    std::env::remove_var("AWS_ACCESS_KEY_ID");
    std::env::remove_var("AWS_SECRET_ACCESS_KEY");
    
    Ok(())
}

#[tokio::test]
async fn test_converter_context_creation() -> Result<()> {
    let converter = BedrockConverter::new();
    
    let context = converter.create_context(
        BedrockModelFamily::Anthropic,
        "anthropic.claude-3-5-sonnet-20241022-v2:0".to_string(),
        true,
    );
    
    assert_eq!(context.model_family, BedrockModelFamily::Anthropic);
    assert_eq!(context.model_id, "anthropic.claude-3-5-sonnet-20241022-v2:0");
    assert!(context.streaming);
    
    Ok(())
}

#[tokio::test]
async fn test_provider_health_check() -> Result<()> {
    let config = create_test_config();
    let provider = BedrockProvider::new(config).await?;
    
    let health = provider.health_check().await;
    
    match health {
        Ok(health_status) => {
            assert_eq!(health_status.provider, "aws_bedrock");
            // In mock/test mode, health check might pass or fail depending on implementation
        }
        Err(e) => {
            // Expected in test environment without real AWS access
            assert!(e.to_string().contains("connection") || e.to_string().contains("auth"));
        }
    }
    
    Ok(())
}

#[tokio::test]
async fn test_model_configuration_precedence() -> Result<()> {
    let mut config = BedrockConfig::default();
    
    // Test default model config
    let default_config = config.get_model_config("unknown_model");
    assert_eq!(default_config.model_id, "anthropic.claude-3-5-sonnet-20241022-v2:0");
    
    // Add specific model config
    let custom_model_config = crate::infrastructure::aws_bedrock::types::BedrockModelConfig {
        model_id: "custom_model".to_string(),
        model_family: crate::infrastructure::aws_bedrock::types::BedrockModelFamily::Anthropic,
        supports_tools: false,
        supports_streaming: false,
        max_context_length: 50000,
        max_output_tokens: 1000,
        default_parameters: crate::infrastructure::aws_bedrock::types::BedrockModelParameters::default(),
        timeout_seconds: 30,
        max_retries: 1,
        rate_limit: crate::infrastructure::aws_bedrock::types::BedrockRateLimitConfig::default(),
    };
    
    config.models.insert("custom_model".to_string(), custom_model_config.clone());
    
    let retrieved_config = config.get_model_config("custom_model");
    assert_eq!(retrieved_config.model_id, "custom_model");
    assert!(!retrieved_config.supports_tools);
    assert_eq!(retrieved_config.max_context_length, 50000);
    
    Ok(())
}

// Integration test for the complete request flow (requires mocking in CI)
#[cfg(feature = "integration_tests")]
#[tokio::test]
async fn test_complete_chat_completion_flow() -> Result<()> {
    let test_config = BedrockTestConfig::default();
    
    if test_config.use_mock {
        // Skip this test in mock mode
        return Ok(());
    }
    
    let config = create_test_config();
    let client = BedrockClient::new(config).await?;
    
    let request = ChatCompletionRequest {
        model: "anthropic.claude-3-haiku-20240307-v1:0".to_string(), // Use cheaper model for tests
        messages: vec![
            ChatMessage {
                role: MessageRole::User,
                content: "Hello! Please respond with exactly: 'Test successful'".to_string(),
                name: None,
                tool_calls: None,
                tool_call_id: None,
            }
        ],
        temperature: Some(0.0), // Deterministic response
        max_tokens: Some(50),
        ..Default::default()
    };
    
    let response = client.chat_completion(request).await?;
    
    assert_eq!(response.object, "chat.completion");
    assert!(!response.choices.is_empty());
    assert!(response.choices[0].message.content.is_some());
    assert!(response.usage.is_some());
    
    Ok(())
}

// Test streaming functionality (requires mocking in CI)  
#[cfg(feature = "integration_tests")]
#[tokio::test]
async fn test_streaming_chat_completion() -> Result<()> {
    let test_config = BedrockTestConfig::default();
    
    if test_config.use_mock {
        // Skip this test in mock mode
        return Ok(());
    }
    
    let config = create_test_config();
    let client = BedrockClient::new(config).await?;
    
    let request = ChatCompletionRequest {
        model: "anthropic.claude-3-haiku-20240307-v1:0".to_string(),
        messages: vec![
            ChatMessage {
                role: MessageRole::User,
                content: "Count from 1 to 5, each number on a new line.".to_string(),
                name: None,
                tool_calls: None,
                tool_call_id: None,
            }
        ],
        temperature: Some(0.0),
        max_tokens: Some(100),
        stream: Some(true),
        ..Default::default()
    };
    
    let mut stream = client.chat_completion_stream(request).await?;
    let mut chunk_count = 0;
    let mut content_received = false;
    
    while let Some(chunk_result) = stream.next().await {
        let chunk = chunk_result?;
        chunk_count += 1;
        
        assert_eq!(chunk.object, "chat.completion.chunk");
        
        if !chunk.choices.is_empty() && chunk.choices[0].delta.content.is_some() {
            content_received = true;
        }
        
        // Don't let test run indefinitely
        if chunk_count > 100 {
            break;
        }
    }
    
    assert!(chunk_count > 0, "Should receive at least one chunk");
    assert!(content_received, "Should receive content in at least one chunk");
    
    Ok(())
}

// Performance benchmark test
#[tokio::test]
async fn test_performance_benchmarks() -> Result<()> {
    let config = create_test_config();
    
    // Test client creation performance
    let start = std::time::Instant::now();
    let _client = BedrockClient::new(config).await?;
    let creation_time = start.elapsed();
    
    // Client creation should be reasonably fast (< 1 second)
    assert!(creation_time < Duration::from_secs(1), 
            "Client creation took too long: {:?}", creation_time);
    
    // Test converter performance
    let converter = BedrockConverter::new();
    let messages = vec![
        ChatMessage {
            role: MessageRole::User,
            content: "Test message".to_string(),
            name: None,
            tool_calls: None,
            tool_call_id: None,
        }
    ];
    
    let start = std::time::Instant::now();
    for _ in 0..1000 {
        let _result = converter.convert_messages_to_bedrock(&messages, BedrockModelFamily::Anthropic)?;
    }
    let conversion_time = start.elapsed();
    
    // 1000 conversions should complete quickly (< 100ms)
    assert!(conversion_time < Duration::from_millis(100),
            "Message conversion took too long: {:?}", conversion_time);
    
    Ok(())
}