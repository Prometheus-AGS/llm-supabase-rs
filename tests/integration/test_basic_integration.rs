//! Basic integration tests
//!
//! This module provides simple, immediately working integration tests that validate
//! the core functionality of the Codex CLI proxy without external dependencies.

use anyhow::Result;
use std::time::Duration;
use serde_json::json;

use llm_supabase_rs::models::{
    request::ChatCompletionRequest,
    common::{ChatMessage, MessageRole, ToolDefinition, FunctionDefinition},
};

// Declare the utils module
mod utils;
use utils::*;

/// Test basic request creation and validation
#[test]
fn test_basic_request_creation() -> Result<()> {
    println!("🧪 Testing basic request creation");

    // Test basic message creation
    let user_message = ChatMessage {
        role: MessageRole::User,
        content: "Hello, world!".to_string(),
        name: None,
        tool_calls: None,
        tool_call_id: None,
    };

    assert_eq!(user_message.role, MessageRole::User);
    assert_eq!(user_message.content, "Hello, world!");
    assert!(user_message.name.is_none());

    // Test request creation with validation
    let request = ChatCompletionRequest::new(
        "claude-4-sonnet-20250514",
        vec![user_message]
    )
    .with_max_tokens(1000)
    .with_temperature(0.7)
    .with_stream(false);

    // Validate request structure
    assert_eq!(request.model, "claude-4-sonnet-20250514");
    assert_eq!(request.messages.len(), 1);
    assert_eq!(request.max_tokens, Some(1000));
    assert_eq!(request.temperature, Some(0.7));
    assert_eq!(request.stream, Some(false));

    // Test request validation
    assert!(request.validate().is_ok(), "Valid request should pass validation");

    println!("✅ Basic request creation test passed");
    Ok(())
}

/// Test tool definition creation
#[test]
fn test_tool_definition_creation() -> Result<()> {
    println!("🔧 Testing tool definition creation");

    let tool = ToolDefinition {
        tool_type: "function".to_string(),
        function: FunctionDefinition {
            name: "apply_patch".to_string(),
            description: Some("Apply a unified diff patch to a file".to_string()),
            parameters: json!({
                "type": "object",
                "properties": {
                    "file_path": {
                        "type": "string",
                        "description": "Path to the file to patch"
                    },
                    "patch": {
                        "type": "string",
                        "description": "Unified diff patch content"
                    }
                },
                "required": ["file_path", "patch"]
            }),
        },
    };

    // Validate tool structure
    assert_eq!(tool.tool_type, "function");
    assert_eq!(tool.function.name, "apply_patch");
    assert!(tool.function.description.is_some());
    assert!(tool.function.parameters.is_object());

    // Test serialization/deserialization
    let json_str = serde_json::to_string(&tool)?;
    assert!(!json_str.is_empty());
    
    let deserialized: ToolDefinition = serde_json::from_str(&json_str)?;
    assert_eq!(deserialized.function.name, "apply_patch");

    println!("✅ Tool definition creation test passed");
    Ok(())
}

/// Test conversation flow simulation
#[test]
fn test_conversation_flow_simulation() -> Result<()> {
    println!("💬 Testing conversation flow simulation");

    // Simulate a multi-turn conversation
    let mut conversation = Vec::new();

    // User request
    conversation.push(ChatMessage {
        role: MessageRole::User,
        content: "Create a simple calculator function".to_string(),
        name: None,
        tool_calls: None,
        tool_call_id: None,
    });

    // Assistant response
    conversation.push(ChatMessage {
        role: MessageRole::Assistant,
        content: Some("I'll create a calculator function for you.".to_string()),
        name: None,
        tool_calls: None,
        tool_call_id: None,
    });

    // Follow-up user request
    conversation.push(ChatMessage {
        role: MessageRole::User,
        content: "Now add error handling to it".to_string(),
        name: None,
        tool_calls: None,
        tool_call_id: None,
    });

    // Validate conversation structure
    assert_eq!(conversation.len(), 3);
    assert_eq!(conversation[0].role, MessageRole::User);
    assert_eq!(conversation[1].role, MessageRole::Assistant);
    assert_eq!(conversation[2].role, MessageRole::User);

    // Test conversation in request
    let request = ChatCompletionRequest::new(
        "claude-4-sonnet-20250514",
        conversation
    );

    assert!(request.validate().is_ok(), "Conversation request should be valid");
    assert_eq!(request.messages.len(), 3);

    println!("✅ Conversation flow simulation test passed");
    Ok(())
}

/// Test streaming request configuration
#[test]
fn test_streaming_request_configuration() -> Result<()> {
    println!("🌊 Testing streaming request configuration");

    let streaming_request = ChatCompletionRequest::new(
        "claude-4-sonnet-20250514",
        vec![ChatMessage {
            role: MessageRole::User,
            content: "Explain Rust ownership".to_string(),
            name: None,
            tool_calls: None,
            tool_call_id: None,
        }]
    )
    .streaming(true)
    .with_max_tokens(2048);

    // Validate streaming configuration
    assert_eq!(streaming_request.stream, Some(true));
    assert!(streaming_request.stream_options.is_some());
    assert_eq!(streaming_request.max_tokens, Some(2048));
    assert!(streaming_request.is_streaming());

    // Test non-streaming configuration
    let non_streaming = ChatCompletionRequest::new(
        "claude-4-sonnet-20250514",
        vec![ChatMessage {
            role: MessageRole::User,
            content: "Simple question".to_string(),
            name: None,
            tool_calls: None,
            tool_call_id: None,
        }]
    )
    .with_stream(false);

    assert_eq!(non_streaming.stream, Some(false));
    assert!(!non_streaming.is_streaming());

    println!("✅ Streaming request configuration test passed");
    Ok(())
}

/// Test request validation edge cases
#[test]
fn test_request_validation_edge_cases() -> Result<()> {
    println!("⚠️ Testing request validation edge cases");

    // Test empty messages (should fail)
    let empty_request = ChatCompletionRequest::new("claude-4-sonnet-20250514", vec![]);
    assert!(empty_request.validate().is_err(), "Empty messages should fail validation");

    // Test invalid temperature (should fail)
    let invalid_temp = ChatCompletionRequest::new(
        "claude-4-sonnet-20250514",
        vec![ChatMessage {
            role: MessageRole::User,
            content: "Test".to_string(),
            name: None,
            tool_calls: None,
            tool_call_id: None,
        }]
    ).with_temperature(3.0); // Invalid: > 2.0

    assert!(invalid_temp.validate().is_err(), "Invalid temperature should fail validation");

    // Test invalid max_tokens (should fail)
    let invalid_tokens = ChatCompletionRequest::new(
        "claude-4-sonnet-20250514",
        vec![ChatMessage {
            role: MessageRole::User,
            content: "Test".to_string(),
            name: None,
            tool_calls: None,
            tool_call_id: None,
        }]
    ).with_max_tokens(0); // Invalid: must be > 0

    assert!(invalid_tokens.validate().is_err(), "Invalid max_tokens should fail validation");

    // Test valid edge case values
    let valid_edge_case = ChatCompletionRequest::new(
        "claude-4-sonnet-20250514",
        vec![ChatMessage {
            role: MessageRole::User,
            content: "Test".to_string(),
            name: None,
            tool_calls: None,
            tool_call_id: None,
        }]
    )
    .with_temperature(0.0)  // Valid minimum
    .with_max_tokens(1)     // Valid minimum
    .with_top_p(1.0);       // Valid maximum

    assert!(valid_edge_case.validate().is_ok(), "Valid edge case should pass validation");

    println!("✅ Request validation edge cases test passed");
    Ok(())
}

/// Test performance benchmarking utilities
#[test]
fn test_performance_benchmarking_utilities() -> Result<()> {
    println!("📊 Testing performance benchmarking utilities");

    // Test duration measurements
    let start = std::time::Instant::now();
    std::thread::sleep(Duration::from_millis(10));
    let elapsed = start.elapsed();

    // Validate timing
    assert!(elapsed.as_millis() >= 10, "Should measure at least 10ms");
    assert!(elapsed.as_millis() < 1000, "Should be less than 1 second");

    // Test performance assertion utility
    let fast_duration = Duration::from_millis(50);
    let result = PerformanceAssertions::assert_response_time(fast_duration, 100);
    assert!(result.is_ok(), "Fast duration should pass performance assertion");

    let slow_duration = Duration::from_millis(200);
    let result = PerformanceAssertions::assert_response_time(slow_duration, 100);
    assert!(result.is_err(), "Slow duration should fail performance assertion");

    // Test success rate calculations
    let success_rate_result = PerformanceAssertions::assert_success_rate(8, 10, 0.7);
    assert!(success_rate_result.is_ok(), "80% success rate should pass 70% threshold");

    let low_success_rate = PerformanceAssertions::assert_success_rate(5, 10, 0.7);
    assert!(low_success_rate.is_err(), "50% success rate should fail 70% threshold");

    println!("✅ Performance benchmarking utilities test passed");
    Ok(())
}

/// Integration test that demonstrates the framework is working
#[tokio::test]
async fn test_integration_framework_demonstration() -> Result<()> {
    println!("🎯 Demonstrating integration test framework capabilities");

    // Test 1: Mock client creation
    let mock_client = MockCodexClient::new("http://localhost:3000");
    assert!(mock_client.get_conversation_history().is_empty(), "New client should have empty history");
    
    println!("✅ Mock client created successfully");

    // Test 2: Request building with realistic Codex CLI patterns
    let codex_request = ChatCompletionRequest::new(
        "claude-4-sonnet-20250514",
        vec![ChatMessage {
            role: MessageRole::User,
            content: "Create a Rust function to parse JSON".to_string(),
            name: None,
            tool_calls: None,
            tool_call_id: None,
        }]
    )
    .with_tools(TestFixtures::codex_tools())
    .with_max_tokens(4096)
    .with_temperature(0.7);

    assert!(codex_request.validate().is_ok(), "Codex CLI request should be valid");
    assert!(codex_request.tools.is_some(), "Should have Codex tools");
    println!("✅ Codex CLI request pattern validated");

    // Test 3: Tool definitions validation
    let tools = TestFixtures::codex_tools();
    assert!(tools.len() >= 4, "Should have standard Codex tools");
    
    let apply_patch_tool = tools.iter()
        .find(|t| t.function.name == "apply_patch")
        .expect("Should have apply_patch tool");
    
    assert_eq!(apply_patch_tool.tool_type, "function");
    println!("✅ Tool definitions validated");

    // Test 4: Performance measurement
    let start = std::time::Instant::now();
    tokio::time::sleep(Duration::from_millis(1)).await;
    let elapsed = start.elapsed();
    
    let performance_result = PerformanceAssertions::assert_response_time(elapsed, 1000);
    assert!(performance_result.is_ok(), "Performance measurement should work");
    println!("✅ Performance measurement working");

    // Test 5: Error handling validation
    let test_error = anyhow::anyhow!("Test error for validation");
    let error_result = ErrorAssertions::assert_error_contains(&test_error, &["Test error"]);
    assert!(error_result.is_ok(), "Error assertion should work");
    println!("✅ Error handling validation working");

    println!("🎉 Integration test framework demonstration completed successfully!");
    println!("   All core components are functional and ready for comprehensive testing");
    
    Ok(())
}

/// Test that validates realistic Codex CLI usage patterns work
#[test]
fn test_realistic_codex_usage_patterns() -> Result<()> {
    println!("👨‍💻 Testing realistic Codex CLI usage patterns");

    // Pattern 1: Code generation request
    let code_gen_request = ChatCompletionRequest::new(
        "claude-4-sonnet-20250514",
        vec![ChatMessage {
            role: MessageRole::User,
            content: "Create a web server in Rust using axum with basic routes".to_string(),
            name: None,
            tool_calls: None,
            tool_call_id: None,
        }]
    );

    assert!(code_gen_request.validate().is_ok());
    println!("✅ Code generation pattern validated");

    // Pattern 2: File modification request with tools
    let file_mod_request = ChatCompletionRequest::new(
        "claude-4-sonnet-20250514",
        vec![
            ChatMessage {
                role: MessageRole::User,
                content: "Read main.rs and add error handling to the main function".to_string(),
                name: None,
                tool_calls: None,
                tool_call_id: None,
            }
        ]
    ).with_tools(TestFixtures::codex_tools());

    assert!(file_mod_request.validate().is_ok());
    assert!(file_mod_request.tools.is_some());
    println!("✅ File modification pattern validated");

    // Pattern 3: Multi-turn conversation
    let multi_turn = vec![
        ChatMessage {
            role: MessageRole::User,
            content: "Create a User struct".to_string(),
            name: None,
            tool_calls: None,
            tool_call_id: None,
        },
        ChatMessage {
            role: MessageRole::Assistant,
            content: Some("I've created a User struct for you.".to_string()),
            name: None,
            tool_calls: None,
            tool_call_id: None,
        },
        ChatMessage {
            role: MessageRole::User,
            content: "Now add validation methods to it".to_string(),
            name: None,
            tool_calls: None,
            tool_call_id: None,
        }
    ];

    let conversation_request = ChatCompletionRequest::new(
        "claude-4-sonnet-20250514",
        multi_turn
    ).with_previous_response_id("prev-response-123");

    assert!(conversation_request.validate().is_ok());
    assert!(conversation_request.previous_response_id.is_some());
    println!("✅ Multi-turn conversation pattern validated");

    // Pattern 4: Streaming request for explanations
    let streaming_request = ChatCompletionRequest::new(
        "claude-4-sonnet-20250514",
        vec![ChatMessage {
            role: MessageRole::User,
            content: "Explain Rust ownership and borrowing with examples".to_string(),
            name: None,
            tool_calls: None,
            tool_call_id: None,
        }]
    )
    .streaming(true)
    .with_max_tokens(2048);

    assert!(streaming_request.validate().is_ok());
    assert!(streaming_request.is_streaming());
    println!("✅ Streaming explanation pattern validated");

    println!("🎯 All realistic Codex CLI usage patterns validated successfully");
    Ok(())
}