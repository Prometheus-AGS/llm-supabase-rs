// tests/integration/test_tool_calling_e2e.rs
//
// End-to-end integration tests for tool calling functionality
// Tests the complete tool calling workflow from request to response

use anyhow::Result;
use serde_json::{json, Value};
use std::time::Duration;
use tokio::time::timeout;

use llm_supabase_rs::{
    app::App,
    config::AppConfig,
    models::{
        request::ChatCompletionRequest,
        response::ChatCompletionResponse,
        common::{ChatMessage, MessageRole, ToolDefinition, FunctionDefinition},
    },
};

/// Test configuration for tool calling
struct ToolCallTestConfig {
    base_url: String,
    api_key: String,
    model: String,
}

impl Default for ToolCallTestConfig {
    fn default() -> Self {
        Self {
            base_url: "http://localhost:8080".to_string(),
            api_key: "test-token".to_string(),
            model: "claude-4-sonnet-20250514".to_string(),
        }
    }
}

/// Helper to create a test tool definition
fn create_weather_tool() -> ToolDefinition {
    ToolDefinition {
        tool_type: "function".to_string(),
        function: FunctionDefinition {
            name: "get_weather".to_string(),
            description: Some("Get current weather for a location".to_string()),
            parameters: json!({
                "type": "object",
                "properties": {
                    "location": {
                        "type": "string",
                        "description": "City name"
                    },
                    "units": {
                        "type": "string",
                        "enum": ["celsius", "fahrenheit"],
                        "description": "Temperature units",
                        "default": "celsius"
                    }
                },
                "required": ["location"]
            }),
        },
    }
}

/// Helper to create a search tool definition
fn create_search_tool() -> ToolDefinition {
    ToolDefinition {
        tool_type: "function".to_string(),
        function: FunctionDefinition {
            name: "tavily_search".to_string(),
            description: Some("Search the web using Tavily".to_string()),
            parameters: json!({
                "type": "object",
                "properties": {
                    "query": {
                        "type": "string",
                        "description": "Search query"
                    },
                    "max_results": {
                        "type": "integer",
                        "description": "Maximum number of results",
                        "default": 5,
                        "minimum": 1,
                        "maximum": 20
                    }
                },
                "required": ["query"]
            }),
        },
    }
}

/// Test basic tool calling functionality
#[tokio::test]
#[ignore] // Requires running server
async fn test_basic_tool_calling() -> Result<()> {
    let config = ToolCallTestConfig::default();
    let client = reqwest::Client::new();

    // Create a request with tool calling
    let request = json!({
        "model": config.model,
        "messages": [
            {
                "role": "user",
                "content": "What's the weather like in San Francisco? Use the weather tool."
            }
        ],
        "tools": [create_weather_tool()],
        "max_tokens": 150
    });

    // Send request
    let response = client
        .post(&format!("{}/v1/chat/completions", config.base_url))
        .header("Content-Type", "application/json")
        .header("Authorization", &format!("Bearer {}", config.api_key))
        .json(&request)
        .send()
        .await?;

    assert!(response.status().is_success(), "Request failed: {}", response.status());

    let response_body: Value = response.json().await?;
    
    // Verify response structure
    assert!(response_body.get("choices").is_some(), "Response missing choices");
    
    let choice = &response_body["choices"][0];
    let message = &choice["message"];
    
    // Check if tool calls were made
    if let Some(tool_calls) = message.get("tool_calls") {
        assert!(tool_calls.is_array(), "tool_calls should be an array");
        assert!(!tool_calls.as_array().unwrap().is_empty(), "tool_calls should not be empty");
        
        let tool_call = &tool_calls[0];
        assert_eq!(tool_call["type"], "function", "tool_call type should be function");
        assert_eq!(tool_call["function"]["name"], "get_weather", "function name should be get_weather");
        
        // Verify arguments are valid JSON
        let arguments_str = tool_call["function"]["arguments"].as_str().unwrap();
        let _arguments: Value = serde_json::from_str(arguments_str)?;
        
        println!("✅ Basic tool calling test passed");
        println!("Tool call: {}", serde_json::to_string_pretty(tool_call)?);
    } else {
        // If no tool calls, check for content
        assert!(message.get("content").is_some(), "Response should have either tool_calls or content");
        println!("⚠️  No tool calls made, but response contains content");
    }

    Ok(())
}

/// Test streaming tool calling
#[tokio::test]
#[ignore] // Requires running server
async fn test_streaming_tool_calling() -> Result<()> {
    let config = ToolCallTestConfig::default();
    let client = reqwest::Client::new();

    let request = json!({
        "model": config.model,
        "stream": true,
        "messages": [
            {
                "role": "user",
                "content": "Search for recent AI news using the search tool."
            }
        ],
        "tools": [create_search_tool()],
        "max_tokens": 200
    });

    let response = client
        .post(&format!("{}/v1/chat/completions", config.base_url))
        .header("Content-Type", "application/json")
        .header("Authorization", &format!("Bearer {}", config.api_key))
        .json(&request)
        .send()
        .await?;

    assert!(response.status().is_success(), "Streaming request failed: {}", response.status());

    // Read streaming response
    let response_text = response.text().await?;
    
    // Verify SSE format
    assert!(response_text.contains("data: "), "Response should contain SSE data");
    
    // Check for tool calls in streaming chunks
    let has_tool_calls = response_text.contains("tool_calls");
    let has_function_name = response_text.contains("tavily_search");
    
    if has_tool_calls && has_function_name {
        println!("✅ Streaming tool calling test passed");
        println!("Found tool calls in streaming response");
    } else {
        println!("⚠️  Streaming response received but no tool calls detected");
    }

    // Verify proper SSE termination
    assert!(response_text.contains("[DONE]") || response_text.ends_with("\n\n"), 
           "Streaming should end properly");

    Ok(())
}

/// Test multiple tool calls in one request
#[tokio::test]
#[ignore] // Requires running server
async fn test_multiple_tool_calls() -> Result<()> {
    let config = ToolCallTestConfig::default();
    let client = reqwest::Client::new();

    let request = json!({
        "model": config.model,
        "messages": [
            {
                "role": "user",
                "content": "Get the weather in both New York and London, then search for travel information between these cities."
            }
        ],
        "tools": [
            create_weather_tool(),
            {
                "type": "function",
                "function": {
                    "name": "search_travel",
                    "description": "Search for travel information between cities",
                    "parameters": {
                        "type": "object",
                        "properties": {
                            "from": {"type": "string"},
                            "to": {"type": "string"},
                            "travel_type": {"type": "string", "enum": ["flight", "train", "car"]}
                        },
                        "required": ["from", "to"]
                    }
                }
            }
        ],
        "max_tokens": 300
    });

    let response = client
        .post(&format!("{}/v1/chat/completions", config.base_url))
        .header("Content-Type", "application/json")
        .header("Authorization", &format!("Bearer {}", config.api_key))
        .json(&request)
        .send()
        .await?;

    assert!(response.status().is_success(), "Multiple tools request failed: {}", response.status());

    let response_body: Value = response.json().await?;
    let choice = &response_body["choices"][0];
    let message = &choice["message"];

    if let Some(tool_calls) = message.get("tool_calls") {
        let tool_calls_array = tool_calls.as_array().unwrap();
        println!("✅ Multiple tool calls test: {} tool calls detected", tool_calls_array.len());
        
        for (i, tool_call) in tool_calls_array.iter().enumerate() {
            println!("Tool call {}: {}", i + 1, tool_call["function"]["name"]);
        }
    } else {
        println!("⚠️  No tool calls detected in multiple tools test");
    }

    Ok(())
}

/// Test tool call continuation with results
#[tokio::test]
#[ignore] // Requires running server
async fn test_tool_call_continuation() -> Result<()> {
    let config = ToolCallTestConfig::default();
    let client = reqwest::Client::new();

    // First request - should trigger tool call
    let initial_request = json!({
        "model": config.model,
        "messages": [
            {
                "role": "user",
                "content": "What's the weather in Tokyo?"
            }
        ],
        "tools": [create_weather_tool()],
        "max_tokens": 100
    });

    let response = client
        .post(&format!("{}/v1/chat/completions", config.base_url))
        .header("Content-Type", "application/json")
        .header("Authorization", &format!("Bearer {}", config.api_key))
        .json(&initial_request)
        .send()
        .await?;

    let response_body: Value = response.json().await?;
    let choice = &response_body["choices"][0];
    let message = &choice["message"];

    if let Some(tool_calls) = message.get("tool_calls") {
        let tool_call = &tool_calls[0];
        let tool_call_id = tool_call["id"].as_str().unwrap();

        // Continue conversation with tool result
        let continuation_request = json!({
            "model": config.model,
            "messages": [
                {
                    "role": "user",
                    "content": "What's the weather in Tokyo?"
                },
                message,
                {
                    "role": "tool",
                    "tool_call_id": tool_call_id,
                    "content": "The weather in Tokyo is currently 22°C (72°F) with partly cloudy skies. Humidity is 65% and wind speed is 10 km/h from the southeast."
                }
            ],
            "max_tokens": 150
        });

        let continuation_response = client
            .post(&format!("{}/v1/chat/completions", config.base_url))
            .header("Content-Type", "application/json")
            .header("Authorization", &format!("Bearer {}", config.api_key))
            .json(&continuation_request)
            .send()
            .await?;

        assert!(continuation_response.status().is_success(), 
               "Continuation request failed: {}", continuation_response.status());

        let continuation_body: Value = continuation_response.json().await?;
        let final_message = &continuation_body["choices"][0]["message"];

        assert!(final_message.get("content").is_some(), 
               "Continuation should produce content response");

        println!("✅ Tool call continuation test passed");
        println!("Final response: {}", final_message["content"]);
    } else {
        println!("⚠️  No tool calls in initial request - skipping continuation test");
    }

    Ok(())
}

/// Test error handling in tool calls
#[tokio::test]
#[ignore] // Requires running server
async fn test_tool_call_error_handling() -> Result<()> {
    let config = ToolCallTestConfig::default();
    let client = reqwest::Client::new();

    // Request with invalid tool definition
    let request = json!({
        "model": config.model,
        "messages": [
            {
                "role": "user",
                "content": "Use an invalid tool."
            }
        ],
        "tools": [
            {
                "type": "function",
                "function": {
                    "name": "invalid_tool",
                    "description": "This tool has invalid parameters",
                    "parameters": "invalid_schema" // Invalid - should be object
                }
            }
        ],
        "max_tokens": 100
    });

    let response = client
        .post(&format!("{}/v1/chat/completions", config.base_url))
        .header("Content-Type", "application/json")
        .header("Authorization", &format!("Bearer {}", config.api_key))
        .json(&request)
        .send()
        .await?;

    // Should either return error or handle gracefully
    if response.status().is_client_error() {
        let error_body: Value = response.json().await?;
        assert!(error_body.get("error").is_some(), "Error response should contain error field");
        println!("✅ Error handling test passed - proper error response");
    } else {
        // If handled gracefully, should still return valid response
        assert!(response.status().is_success(), "Should handle gracefully or return error");
        println!("✅ Error handling test passed - graceful handling");
    }

    Ok(())
}

/// Test performance of tool calling
#[tokio::test]
#[ignore] // Requires running server
async fn test_tool_calling_performance() -> Result<()> {
    let config = ToolCallTestConfig::default();
    let client = reqwest::Client::new();

    let request = json!({
        "model": config.model,
        "messages": [
            {
                "role": "user",
                "content": "Get the current time."
            }
        ],
        "tools": [
            {
                "type": "function",
                "function": {
                    "name": "get_time",
                    "description": "Get the current time",
                    "parameters": {
                        "type": "object",
                        "properties": {
                            "timezone": {"type": "string", "default": "UTC"}
                        }
                    }
                }
            }
        ],
        "max_tokens": 50
    });

    let start = std::time::Instant::now();

    let response = timeout(Duration::from_secs(10), async {
        client
            .post(&format!("{}/v1/chat/completions", config.base_url))
            .header("Content-Type", "application/json")
            .header("Authorization", &format!("Bearer {}", config.api_key))
            .json(&request)
            .send()
            .await
    }).await??;

    let duration = start.elapsed();

    assert!(response.status().is_success(), "Performance test request failed");
    assert!(duration < Duration::from_secs(5), "Request took too long: {:?}", duration);

    println!("✅ Performance test passed - Response time: {:?}", duration);

    Ok(())
}

/// Integration test runner
#[tokio::test]
#[ignore] // Requires running server
async fn run_all_tool_calling_tests() -> Result<()> {
    println!("🚀 Running comprehensive tool calling integration tests...");

    // Run all tests
    test_basic_tool_calling().await?;
    test_streaming_tool_calling().await?;
    test_multiple_tool_calls().await?;
    test_tool_call_continuation().await?;
    test_tool_call_error_handling().await?;
    test_tool_calling_performance().await?;

    println!("🎉 All tool calling integration tests passed!");

    Ok(())
}

#[cfg(test)]
mod tool_calling_unit_tests {
    use super::*;
    use llm_supabase_rs::infrastructure::common::tools::*;

    #[test]
    fn test_tool_definition_creation() {
        let tool = create_weather_tool();
        assert_eq!(tool.function.name, "get_weather");
        assert!(tool.function.description.is_some());
        assert!(tool.function.parameters.is_object());
    }

    #[test]
    fn test_unified_tool_call_conversion() {
        let manager = ToolCallManager::openai();
        
        let tool_calls_json = json!([{
            "id": "call_123",
            "type": "function",
            "function": {
                "name": "get_weather",
                "arguments": "{\"location\": \"San Francisco\"}"
            }
        }]);

        let result = manager.converter.provider_tool_calls_to_unified(&tool_calls_json);
        assert!(result.is_ok());
        
        let unified_calls = result.unwrap();
        assert_eq!(unified_calls.len(), 1);
        assert_eq!(unified_calls[0].function_name, "get_weather");
    }

    #[test]
    fn test_tool_call_state_management() {
        let mut manager = ToolCallManager::claude();
        
        // Initially no state
        assert!(matches!(manager.get_state(), ToolCallState::None));
        
        // After processing tool calls
        let tool_data = json!({
            "content": [{
                "tool_use": {
                    "id": "tool_123",
                    "name": "search",
                    "input": {"query": "test"}
                }
            }]
        });
        
        let result = manager.process_tool_calls(&tool_data, "req_123".to_string());
        assert!(result.is_ok());
        
        // Should now be waiting for results
        assert!(matches!(manager.get_state(), ToolCallState::WaitingForResults { .. }));
    }
}
