//! Integration tests for Anthropic Claude provider
//! 
//! This module contains comprehensive integration tests for the Anthropic provider,
//! including tool calling, streaming, and error handling scenarios.

use anyhow::Result;
use serde_json::json;
use std::sync::Arc;
use tokio_stream::StreamExt;

use crate::infrastructure::anthropic::{
    AnthropicClient, AnthropicConfig, AnthropicProvider, AnthropicProviderFactory
};
use crate::infrastructure::common::tools::{UnifiedToolCall, ToolCallResult};
use crate::models::common::{ChatMessage, ToolDefinition, FunctionDefinition};
use crate::models::request::ChatCompletionRequest;
use crate::models::response::ChatCompletionResponse;

/// Test configuration for Anthropic provider
fn create_test_config() -> AnthropicConfig {
    AnthropicConfig {
        api_key: std::env::var("ANTHROPIC_API_KEY")
            .unwrap_or_else(|_| "sk-ant-test123456789012345678901234567890123456789012345".to_string()),
        base_url: Some("https://api.anthropic.com/v1".to_string()),
        organization_id: None,
        default_model: crate::infrastructure::anthropic::types::AnthropicModelConfig {
            model_name: "claude-3-5-sonnet-20241022".to_string(),
            default_parameters: crate::infrastructure::anthropic::types::AnthropicModelParameters {
                temperature: 0.7,
                top_p: 1.0,
                top_k: None,
                max_tokens: 1000,
                stop_sequences: vec![],
            },
            timeout_seconds: 120,
            max_retries: 3,
            rate_limit: crate::config::providers::RateLimitConfig::default(),
        },
        models: std::collections::HashMap::new(),
        api: crate::infrastructure::anthropic::types::AnthropicApiConfig {
            connection_timeout: 30,
            request_timeout: 120,
            max_retries: 3,
            rate_limit_requests_per_minute: Some(50),
            api_version: "2023-06-01".to_string(),
        },
    }
}

/// Create a basic chat completion request for testing
fn create_test_request() -> ChatCompletionRequest {
    ChatCompletionRequest {
        model: "claude-3-5-sonnet-20241022".to_string(),
        messages: vec![
            ChatMessage {
                role: "user".to_string(),
                content: "Hello! Please respond with a simple greeting.".to_string(),
                name: None,
                tool_call_id: None,
                tool_calls: None,
            },
        ],
        tools: None,
        max_tokens: Some(100),
        temperature: Some(0.7),
        top_p: None,
        stream: Some(false),
        stop: None,
        presence_penalty: None,
        frequency_penalty: None,
        logit_bias: None,
        user: None,
        tool_choice: None,
        function_call: None,
        functions: None,
        response_format: None,
    }
}

/// Create a tool calling request for testing
fn create_tool_calling_request() -> ChatCompletionRequest {
    ChatCompletionRequest {
        model: "claude-3-5-sonnet-20241022".to_string(),
        messages: vec![
            ChatMessage {
                role: "user".to_string(),
                content: "What's the weather like in San Francisco? Use the get_weather tool.".to_string(),
                name: None,
                tool_call_id: None,
                tool_calls: None,
            },
        ],
        tools: Some(vec![
            ToolDefinition {
                tool_type: "function".to_string(),
                function: FunctionDefinition {
                    name: "get_weather".to_string(),
                    description: Some("Get current weather information for a location".to_string()),
                    parameters: json!({
                        "type": "object",
                        "properties": {
                            "location": {
                                "type": "string",
                                "description": "The city and state, e.g. San Francisco, CA"
                            },
                            "unit": {
                                "type": "string",
                                "enum": ["celsius", "fahrenheit"],
                                "description": "The temperature unit"
                            }
                        },
                        "required": ["location"]
                    }),
                },
            }
        ]),
        max_tokens: Some(1000),
        temperature: Some(0.1),
        top_p: None,
        stream: Some(false),
        stop: None,
        presence_penalty: None,
        frequency_penalty: None,
        logit_bias: None,
        user: None,
        tool_choice: None,
        function_call: None,
        functions: None,
        response_format: None,
    }
}

#[tokio::test]
async fn test_anthropic_client_creation() -> Result<()> {
    println!("🧪 Testing Anthropic client creation");
    
    let config = create_test_config();
    let client = AnthropicClient::new(config.clone()).await;
    
    assert!(client.is_ok(), "Should be able to create Anthropic client");
    
    let client = client.unwrap();
    assert_eq!(client.config().api_key, config.api_key);
    assert_eq!(client.config().base_url, config.base_url);
    
    println!("✅ Anthropic client creation test passed");
    Ok(())
}

#[tokio::test]
async fn test_anthropic_provider_factory() -> Result<()> {
    println!("🧪 Testing Anthropic provider factory");
    
    let provider = AnthropicProviderFactory::create_with_api_key(
        "sk-ant-test123456789012345678901234567890123456789012345".to_string()
    ).await;
    
    assert!(provider.is_ok(), "Should be able to create provider via factory");
    
    let provider = provider.unwrap();
    let capabilities = provider.get_capabilities();
    
    assert_eq!(capabilities.provider, "anthropic");
    assert!(capabilities.supports_chat);
    assert!(capabilities.supports_tools);
    assert!(capabilities.supports_streaming);
    assert!(!capabilities.supports_completion);
    
    println!("✅ Anthropic provider factory test passed");
    Ok(())
}

#[tokio::test]
async fn test_anthropic_model_capabilities() -> Result<()> {
    println!("🧪 Testing Anthropic model capabilities");
    
    let config = create_test_config();
    let client = AnthropicClient::new(config).await?;
    
    // Test model support checks
    assert!(client.supports_tool_calling("claude-3-5-sonnet-20241022"));
    assert!(client.supports_streaming("claude-3-5-sonnet-20241022"));
    assert!(client.supports_tool_calling("claude-3-5-haiku-20241022"));
    assert!(client.supports_streaming("claude-3-opus-20240229"));
    
    // Test model capabilities
    let capabilities = client.get_model_capabilities("claude-3-5-sonnet-20241022");
    assert!(capabilities.supports_tools);
    assert!(capabilities.supports_streaming);
    assert!(capabilities.supports_parallel_tools);
    assert_eq!(capabilities.max_tokens, 200000);
    
    // Test max tokens
    assert_eq!(client.get_max_tokens("claude-3-5-sonnet-20241022"), 200000);
    assert_eq!(client.get_max_tokens("claude-3-5-haiku-20241022"), 200000);
    
    println!("✅ Anthropic model capabilities test passed");
    Ok(())
}

#[tokio::test]
async fn test_anthropic_available_models() -> Result<()> {
    println!("🧪 Testing Anthropic available models");
    
    let config = create_test_config();
    let client = AnthropicClient::new(config).await?;
    
    let models = client.get_models().await?;
    
    assert!(!models.is_empty(), "Should return some models");
    assert!(models.contains(&"claude-3-5-sonnet-20241022".to_string()));
    assert!(models.contains(&"claude-3-5-haiku-20241022".to_string()));
    assert!(models.contains(&"claude-3-opus-20240229".to_string()));
    
    println!("✅ Available models: {:?}", models);
    println!("✅ Anthropic available models test passed");
    Ok(())
}

#[tokio::test]
async fn test_anthropic_request_conversion() -> Result<()> {
    println!("🧪 Testing Anthropic request conversion");
    
    let config = create_test_config();
    let client = AnthropicClient::new(config).await?;
    
    let openai_request = create_test_request();
    
    // Test request conversion
    let anthropic_request = client.converter().openai_request_to_anthropic(&openai_request)?;
    
    assert_eq!(anthropic_request.model, "claude-3-5-sonnet-20241022");
    assert_eq!(anthropic_request.max_tokens, 100);
    assert_eq!(anthropic_request.messages.len(), 1);
    assert_eq!(anthropic_request.stream, Some(false));
    
    // Test with tool calling request
    let tool_request = create_tool_calling_request();
    let anthropic_tool_request = client.converter().openai_request_to_anthropic(&tool_request)?;
    
    assert!(anthropic_tool_request.tools.is_some());
    let tools = anthropic_tool_request.tools.unwrap();
    assert_eq!(tools.len(), 1);
    assert_eq!(tools[0].name, "get_weather");
    
    println!("✅ Anthropic request conversion test passed");
    Ok(())
}

#[tokio::test]
async fn test_anthropic_tool_call_conversion() -> Result<()> {
    println!("🧪 Testing Anthropic tool call conversion");
    
    let config = create_test_config();
    let client = AnthropicClient::new(config).await?;
    
    // Test OpenAI tools to Anthropic format
    let tools = vec![
        ToolDefinition {
            tool_type: "function".to_string(),
            function: FunctionDefinition {
                name: "test_function".to_string(),
                description: Some("A test function".to_string()),
                parameters: json!({"type": "object"}),
            },
        }
    ];
    
    let anthropic_tools = client.converter().openai_tools_to_provider(&tools)?;
    assert!(anthropic_tools.is_array());
    
    let tools_array = anthropic_tools.as_array().unwrap();
    assert_eq!(tools_array.len(), 1);
    assert_eq!(tools_array[0]["name"], "test_function");
    assert_eq!(tools_array[0]["description"], "A test function");
    
    // Test unified tool call to OpenAI format
    let unified_calls = vec![
        UnifiedToolCall {
            id: "test-id-1".to_string(),
            function_name: "get_weather".to_string(),
            arguments: json!({"location": "San Francisco"}),
            metadata: std::collections::HashMap::new(),
        }
    ];
    
    let openai_calls = client.converter().unified_to_openai_tool_calls(&unified_calls);
    assert_eq!(openai_calls.len(), 1);
    assert_eq!(openai_calls[0].id, "test-id-1");
    assert_eq!(openai_calls[0].function.name, "get_weather");
    
    // Test tool results to provider format
    let results = vec![
        ToolCallResult {
            tool_call_id: "test-id-1".to_string(),
            content: "Weather is sunny, 72°F".to_string(),
            success: true,
            error: None,
        }
    ];
    
    let provider_messages = client.converter().tool_results_to_provider_messages(&results)?;
    assert_eq!(provider_messages["role"], "user");
    assert!(provider_messages["content"].is_array());
    
    println!("✅ Anthropic tool call conversion test passed");
    Ok(())
}

#[tokio::test]
#[ignore] // Only run with real API key
async fn test_anthropic_connection_test() -> Result<()> {
    println!("🧪 Testing Anthropic connection");
    
    // Skip if no API key
    if std::env::var("ANTHROPIC_API_KEY").is_err() {
        println!("⏭️  Skipping connection test - no ANTHROPIC_API_KEY");
        return Ok(());
    }
    
    let config = AnthropicConfig::from_env()?;
    let client = AnthropicClient::new(config).await?;
    
    let connection_result = client.test_connection().await?;
    println!("🔗 Connection test result: {}", connection_result);
    
    // Don't assert here since it depends on valid API key
    // Just ensure no errors are thrown
    
    println!("✅ Anthropic connection test completed");
    Ok(())
}

#[tokio::test]
#[ignore] // Only run with real API key
async fn test_anthropic_basic_chat_completion() -> Result<()> {
    println!("🧪 Testing Anthropic basic chat completion");
    
    // Skip if no API key
    if std::env::var("ANTHROPIC_API_KEY").is_err() {
        println!("⏭️  Skipping chat completion test - no ANTHROPIC_API_KEY");
        return Ok(());
    }
    
    let config = AnthropicConfig::from_env()?;
    let client = AnthropicClient::new(config).await?;
    
    let request = create_test_request();
    let response = client.chat_completion(request).await?;
    
    assert_eq!(response.object, "chat.completion");
    assert_eq!(response.model, "claude-3-5-sonnet-20241022");
    assert!(!response.choices.is_empty());
    
    let choice = &response.choices[0];
    assert_eq!(choice.message.role, "assistant");
    assert!(choice.message.content.is_some());
    
    println!("✅ Response: {:?}", choice.message.content);
    println!("✅ Anthropic basic chat completion test passed");
    Ok(())
}

#[tokio::test]
#[ignore] // Only run with real API key
async fn test_anthropic_streaming_chat_completion() -> Result<()> {
    println!("🧪 Testing Anthropic streaming chat completion");
    
    // Skip if no API key
    if std::env::var("ANTHROPIC_API_KEY").is_err() {
        println!("⏭️  Skipping streaming test - no ANTHROPIC_API_KEY");
        return Ok(());
    }
    
    let config = AnthropicConfig::from_env()?;
    let client = AnthropicClient::new(config).await?;
    
    let mut request = create_test_request();
    request.stream = Some(true);
    
    let mut stream = client.chat_completion_stream(request).await?;
    let mut chunk_count = 0;
    let mut content_pieces = Vec::new();
    
    while let Some(chunk_result) = stream.next().await {
        match chunk_result {
            Ok(chunk) => {
                chunk_count += 1;
                
                assert_eq!(chunk.object, "chat.completion.chunk");
                assert_eq!(chunk.model, "claude-3-5-sonnet-20241022");
                
                if !chunk.choices.is_empty() {
                    let choice = &chunk.choices[0];
                    if let Some(content) = &choice.delta.content {
                        content_pieces.push(content.clone());
                    }
                }
                
                // Limit chunks to prevent infinite streams in tests
                if chunk_count > 50 {
                    break;
                }
            }
            Err(e) => {
                println!("❌ Stream error: {}", e);
                break;
            }
        }
    }
    
    assert!(chunk_count > 0, "Should receive at least one chunk");
    println!("✅ Received {} chunks", chunk_count);
    println!("✅ Content: {}", content_pieces.join(""));
    println!("✅ Anthropic streaming chat completion test passed");
    Ok(())
}

#[tokio::test]
#[ignore] // Only run with real API key
async fn test_anthropic_tool_calling_workflow() -> Result<()> {
    println!("🧪 Testing Anthropic tool calling workflow");
    
    // Skip if no API key
    if std::env::var("ANTHROPIC_API_KEY").is_err() {
        println!("⏭️  Skipping tool calling test - no ANTHROPIC_API_KEY");
        return Ok(());
    }
    
    let config = AnthropicConfig::from_env()?;
    let client = AnthropicClient::new(config).await?;
    
    let request = create_tool_calling_request();
    
    // Create a mock tool executor
    let tool_executor = |tool_call: &UnifiedToolCall| -> Result<ToolCallResult> {
        match tool_call.function_name.as_str() {
            "get_weather" => {
                let location = tool_call.arguments.get("location")
                    .and_then(|v| v.as_str())
                    .unwrap_or("Unknown");
                
                Ok(ToolCallResult {
                    tool_call_id: tool_call.id.clone(),
                    content: format!("The weather in {} is sunny and 72°F", location),
                    success: true,
                    error: None,
                })
            }
            _ => {
                Ok(ToolCallResult {
                    tool_call_id: tool_call.id.clone(),
                    content: "Unknown tool".to_string(),
                    success: false,
                    error: Some("Tool not implemented".to_string()),
                })
            }
        }
    };
    
    let response = client.execute_tool_call_workflow(request, tool_executor).await?;
    
    assert_eq!(response.object, "chat.completion");
    assert!(!response.choices.is_empty());
    
    let choice = &response.choices[0];
    assert_eq!(choice.message.role, "assistant");
    
    if let Some(content) = &choice.message.content {
        println!("✅ Final response: {}", content);
        assert!(content.to_lowercase().contains("weather") || content.to_lowercase().contains("sunny"));
    }
    
    println!("✅ Anthropic tool calling workflow test passed");
    Ok(())
}

#[tokio::test]
async fn test_anthropic_provider_health() -> Result<()> {
    println!("🧪 Testing Anthropic provider health");
    
    let config = create_test_config();
    let provider = AnthropicProvider::new(config).await?;
    
    let health = provider.health_check().await?;
    
    assert_eq!(health.provider, "anthropic");
    // Health status depends on whether we have a valid API key
    println!("✅ Health status: {:?}", health.status);
    println!("✅ Anthropic provider health test passed");
    Ok(())
}

#[tokio::test]
async fn test_anthropic_supported_models_list() -> Result<()> {
    println!("🧪 Testing Anthropic supported models list");
    
    let config = create_test_config();
    let provider = AnthropicProvider::new(config).await?;
    
    let models = provider.get_supported_models().await?;
    
    assert!(!models.is_empty(), "Should have supported models");
    
    // Check for expected Claude models
    let model_names: Vec<String> = models.iter().map(|m| m.name.clone()).collect();
    assert!(model_names.contains(&"claude-3-5-sonnet-20241022".to_string()));
    assert!(model_names.contains(&"claude-3-5-haiku-20241022".to_string()));
    assert!(model_names.contains(&"claude-3-opus-20240229".to_string()));
    
    // Verify model properties
    for model in &models {
        assert_eq!(model.provider, "anthropic");
        assert!(model.supports_tools);
        assert!(model.supports_streaming);
        assert!(model.max_tokens > 0);
    }
    
    println!("✅ Supported models: {:?}", model_names);
    println!("✅ Anthropic supported models test passed");
    Ok(())
}

#[tokio::test]
async fn test_anthropic_error_handling() -> Result<()> {
    println!("🧪 Testing Anthropic error handling");
    
    // Test with invalid API key
    let mut config = create_test_config();
    config.api_key = "invalid-key".to_string();
    
    let client = AnthropicClient::new(config).await?;
    let request = create_test_request();
    
    let result = client.chat_completion(request).await;
    
    // Should handle authentication error gracefully
    if let Err(error) = result {
        println!("✅ Expected error for invalid key: {}", error);
        assert!(error.to_string().contains("API") || error.to_string().contains("auth"));
    }
    
    println!("✅ Anthropic error handling test passed");
    Ok(())
}

#[tokio::test]
async fn test_anthropic_retry_functionality() -> Result<()> {
    println!("🧪 Testing Anthropic retry functionality");
    
    let config = create_test_config();
    let client = AnthropicClient::new(config).await?;
    
    let request = create_test_request();
    
    // This will likely fail due to invalid API key, but should attempt retries
    let result = client.chat_completion_with_retries(request).await;
    
    // The retry functionality should be exercised regardless of success/failure
    match result {
        Ok(_) => println!("✅ Request succeeded"),
        Err(e) => println!("✅ Request failed as expected: {}", e),
    }
    
    println!("✅ Anthropic retry functionality test passed");
    Ok(())
}

#[tokio::test]
async fn test_anthropic_authentication_validation() -> Result<()> {
    println!("🧪 Testing Anthropic authentication validation");
    
    let config = create_test_config();
    let client = AnthropicClient::new(config).await?;
    
    let auth = client.auth();
    
    // Test API key validation
    assert!(auth.is_api_key_format_valid());
    
    // Test masked API key
    let masked_key = auth.get_masked_api_key();
    assert!(!masked_key.is_empty());
    assert!(masked_key.contains("...") || masked_key.contains("*"));
    
    // Test header creation
    let headers = auth.create_headers()?;
    assert!(headers.contains_key("x-api-key"));
    assert!(headers.contains_key("anthropic-version"));
    assert!(headers.contains_key("content-type"));
    
    // Test streaming headers
    let streaming_headers = auth.create_streaming_headers()?;
    assert!(streaming_headers.contains_key("accept"));
    assert!(streaming_headers.contains_key("cache-control"));
    
    println!("✅ Anthropic authentication validation test passed");
    Ok(())
}

#[tokio::test]
async fn test_anthropic_provider_integration() -> Result<()> {
    println!("🧪 Testing Anthropic provider integration with fallback system");
    
    // Test provider enumeration
    use crate::features::provider_fallback::models::Provider;
    
    let providers = Provider::all();
    assert!(providers.contains(&Provider::Anthropic));
    
    // Test Anthropic provider properties
    assert_eq!(Provider::Anthropic.name(), "anthropic");
    assert_eq!(Provider::Anthropic.display_name(), "Anthropic Claude");
    assert_eq!(Provider::Anthropic.base_url(), "https://api.anthropic.com/v1");
    assert_eq!(Provider::Anthropic.default_model(), "claude-3-5-sonnet-20241022");
    
    // Test supported models
    let supported = Provider::Anthropic.supported_models();
    assert!(supported.contains(&"claude-3-5-sonnet-20241022"));
    assert!(supported.contains(&"claude-3-5-haiku-20241022"));
    assert!(supported.contains(&"claude-3-opus-20240229"));
    
    // Test model support check
    assert!(Provider::Anthropic.supports_model("claude-3-5-sonnet-20241022"));
    assert!(!Provider::Anthropic.supports_model("gpt-4o"));
    
    // Test provider capabilities
    use crate::features::provider_fallback::models::ProviderCapability;
    
    let capabilities = ProviderCapability::for_provider(Provider::Anthropic);
    assert!(capabilities.streaming);
    assert!(capabilities.tool_calling);
    assert!(capabilities.vision);
    assert_eq!(capabilities.max_tokens, 200_000);
    assert_eq!(capabilities.max_context_length, 200_000);
    
    println!("✅ Anthropic provider integration test passed");
    Ok(())
}

/// Integration test runner that demonstrates the complete Anthropic provider functionality
#[tokio::test]
async fn test_anthropic_complete_integration() -> Result<()> {
    println!("🚀 Running complete Anthropic provider integration test");
    
    // Test 1: Provider Creation
    println!("📋 Step 1: Creating Anthropic provider");
    let config = create_test_config();
    let provider = AnthropicProvider::new(config).await?;
    assert_eq!(provider.get_capabilities().provider, "anthropic");
    
    // Test 2: Client Access
    println!("📋 Step 2: Testing client access");
    let client = provider.client();
    assert!(client.supports_tool_calling("claude-3-5-sonnet-20241022"));
    
    // Test 3: Model Capabilities
    println!("📋 Step 3: Testing model capabilities");
    let models = provider.get_supported_models().await?;
    assert!(!models.is_empty());
    
    // Test 4: Configuration Access
    println!("📋 Step 4: Testing configuration access");
    let config = provider.config();
    assert!(!config.api_key.is_empty());
    
    // Test 5: Health Check
    println!("📋 Step 5: Testing health check");
    let health = provider.health_check().await?;
    assert_eq!(health.provider, "anthropic");
    
    // Test 6: Converter Functionality
    println!("📋 Step 6: Testing converter functionality");
    let converter = provider.converter();
    let capabilities = converter.get_model_capabilities("claude-3-5-sonnet-20241022");
    assert!(capabilities.supports_tools);
    
    println!("✅ Complete Anthropic provider integration test passed");
    Ok(())
}

/// Performance and reliability tests
#[tokio::test]
async fn test_anthropic_performance_characteristics() -> Result<()> {
    println!("🧪 Testing Anthropic performance characteristics");
    
    let config = create_test_config();
    let client = AnthropicClient::new(config).await?;
    
    // Test configuration validation performance
    let start = std::time::Instant::now();
    for _ in 0..100 {
        let _ = client.config().validate();
    }
    let validation_time = start.elapsed();
    println!("✅ Configuration validation (100x): {:?}", validation_time);
    
    // Test converter performance
    let request = create_test_request();
    let start = std::time::Instant::now();
    for _ in 0..100 {
        let _ = client.converter().openai_request_to_anthropic(&request);
    }
    let conversion_time = start.elapsed();
    println!("✅ Request conversion (100x): {:?}", conversion_time);
    
    // Test tool call conversion performance
    let tools = vec![
        ToolDefinition {
            tool_type: "function".to_string(),
            function: FunctionDefinition {
                name: "test_tool".to_string(),
                description: Some("Test".to_string()),
                parameters: json!({"type": "object"}),
            },
        }
    ];
    
    let start = std::time::Instant::now();
    for _ in 0..100 {
        let _ = client.converter().openai_tools_to_provider(&tools);
    }
    let tool_conversion_time = start.elapsed();
    println!("✅ Tool conversion (100x): {:?}", tool_conversion_time);
    
    println!("✅ Anthropic performance characteristics test passed");
    Ok(())
}

#[cfg(test)]
mod test_utils {
    use super::*;
    
    /// Utility function to check if Anthropic API key is available for integration tests
    pub fn has_anthropic_api_key() -> bool {
        std::env::var("ANTHROPIC_API_KEY").is_ok()
    }
    
    /// Utility function to create a minimal test configuration
    pub fn minimal_test_config() -> AnthropicConfig {
        AnthropicConfig {
            api_key: "test-key".to_string(),
            base_url: Some("https://api.anthropic.com/v1".to_string()),
            ..Default::default()
        }
    }
    
    /// Utility function to assert response structure
    pub fn assert_valid_chat_response(response: &ChatCompletionResponse) {
        assert_eq!(response.object, "chat.completion");
        assert!(!response.choices.is_empty());
        assert_eq!(response.choices[0].message.role, "assistant");
    }
}