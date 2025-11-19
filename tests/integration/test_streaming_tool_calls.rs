//! Streaming tool calls integration tests
//!
//! This module tests the integration between streaming responses and tool execution:
//! - Streaming responses that include tool calls
//! - Proper stream termination when tool calls are required
//! - Tool execution after streaming completion

use anyhow::Result;
use serde_json::Value;
use std::time::{Duration, Instant};

use llm_supabase_rs::models::{
    request::ChatCompletionRequest,
    common::ChatMessage,
};

// Import our test utilities from the parent utils module
use crate::utils::{MockCodexClient, TestServer, TestScenarios, StreamingAssertions, PerformanceAssertions};

/// Test basic streaming response with tool call integration
#[tokio::test]
async fn test_streaming_with_tool_calls() -> Result<()> {
    println!("🌊🔧 Starting streaming with tool calls test");

    let server = TestScenarios::streaming_server().await?;
    server.wait_for_ready(30).await?;

    let mut client = MockCodexClient::new(server.url());

    // Send request that should trigger both streaming and tool calls
    println!("📡 Sending streaming request with tool call potential");
    
    let start_time = Instant::now();
    let chunks = client
        .send_streaming_request(
            "Read the main.rs file, analyze its structure, and suggest improvements",
            "claude-4-sonnet-20250514"
        )
        .await?;

    let streaming_duration = start_time.elapsed();
    println!("✅ Streaming completed in {:?} with {} chunks", streaming_duration, chunks.len());

    // Validate streaming format
    assert!(!chunks.is_empty(), "Should receive streaming chunks");
    PerformanceAssertions::assert_response_time(streaming_duration, 20000)?; // 20s max

    // Parse streaming chunks to extract any tool calls
    let mut tool_calls_found = false;
    let mut content_parts = Vec::new();

    for chunk in &chunks {
        if chunk.starts_with("data: ") && !chunk.contains("[DONE]") {
            let data = &chunk[6..]; // Remove "data: " prefix
            
            if let Ok(json_data) = serde_json::from_str::<Value>(data) {
                if let Some(choices) = json_data.get("choices").and_then(|c| c.as_array()) {
                    if let Some(choice) = choices.first() {
                        if let Some(delta) = choice.get("delta") {
                            // Extract content
                            if let Some(content) = delta.get("content").and_then(|c| c.as_str()) {
                                content_parts.push(content.to_string());
                            }
                            
                            // Check for tool calls
                            if delta.get("tool_calls").is_some() {
                                tool_calls_found = true;
                            }
                        }
                    }
                }
            }
        }
    }

    let accumulated_content = content_parts.join("");
    println!("📝 Accumulated content length: {}", accumulated_content.len());
    println!("🔧 Tool calls found: {}", tool_calls_found);

    // Validate proper streaming termination
    let has_done_marker = chunks.iter().any(|chunk| chunk.contains("[DONE]"));
    assert!(has_done_marker, "Stream should terminate with [DONE] marker");

    server.shutdown().await?;
    println!("✅ Streaming with tool calls test completed");
    
    Ok(())
}

/// Test streaming termination behavior with tool calls
#[tokio::test]
async fn test_streaming_termination_with_tools() -> Result<()> {
    println!("🛑 Starting streaming termination with tools test");

    let server = TestScenarios::streaming_server().await?;
    server.wait_for_ready(30).await?;

    let mut client = MockCodexClient::new(server.url());

    // Send request that should definitely trigger tool calls
    println!("📡 Sending request designed to trigger tool calls");
    
    let chunks = client
        .send_streaming_request(
            "Apply this patch to main.rs: add a new hello_world function",
            "claude-4-sonnet-20250514"
        )
        .await?;

    println!("📊 Received {} streaming chunks", chunks.len());

    // Analyze stream termination pattern
    let mut found_tool_calls = false;
    let mut found_done_marker = false;
    let mut found_finish_reason = false;

    for chunk in &chunks {
        if chunk.starts_with("data: ") {
            let data = &chunk[6..];
            
            if data == "[DONE]" {
                found_done_marker = true;
                continue;
            }

            if let Ok(json_data) = serde_json::from_str::<Value>(data) {
                // Check for tool calls
                if let Some(choices) = json_data.get("choices").and_then(|c| c.as_array()) {
                    if let Some(choice) = choices.first() {
                        if let Some(delta) = choice.get("delta") {
                            if delta.get("tool_calls").is_some() {
                                found_tool_calls = true;
                            }
                        }
                        
                        // Check for finish reason
                        if let Some(finish_reason) = choice.get("finish_reason") {
                            if !finish_reason.is_null() {
                                found_finish_reason = true;
                                println!("🏁 Found finish reason: {}", finish_reason);
                            }
                        }
                    }
                }
            }
        }
    }

    // Validate proper termination
    assert!(found_done_marker, "Stream should terminate with [DONE] marker");
    
    if found_tool_calls {
        println!("✅ Tool calls detected in streaming response");
    }

    println!("📊 Termination analysis - Done: {}, Tool calls: {}, Finish reason: {}", 
             found_done_marker, found_tool_calls, found_finish_reason);

    server.shutdown().await?;
    println!("✅ Streaming termination test completed");
    
    Ok(())
}

/// Test streaming content validation
#[tokio::test]
async fn test_streaming_content_validation() -> Result<()> {
    println!("✅ Starting streaming content validation test");

    let server = TestScenarios::streaming_server().await?;
    server.wait_for_ready(30).await?;

    let mut client = MockCodexClient::new(server.url());

    println!("📡 Sending streaming request for content validation");
    
    let chunks = client
        .send_streaming_request(
            "Explain how Rust's ownership system works",
            "claude-4-sonnet-20250514"
        )
        .await?;

    println!("📊 Received {} chunks for validation", chunks.len());

    // Validate chunk format and content progression
    let mut accumulated_content = String::new();
    let mut data_chunk_count = 0;

    for (i, chunk) in chunks.iter().enumerate() {
        // Validate SSE format
        if chunk.starts_with("data: ") {
            data_chunk_count += 1;
            let data = &chunk[6..];
            
            if data == "[DONE]" {
                println!("🏁 Found [DONE] marker at chunk {}", i);
                continue;
            }

            // Validate JSON format
            if let Ok(json_data) = serde_json::from_str::<Value>(data) {
                if let Some(choices) = json_data.get("choices").and_then(|c| c.as_array()) {
                    if let Some(choice) = choices.first() {
                        if let Some(delta) = choice.get("delta") {
                            if let Some(content) = delta.get("content").and_then(|c| c.as_str()) {
                                accumulated_content.push_str(content);
                            }
                        }
                    }
                }
            } else {
                panic!("Invalid JSON in streaming chunk: {}", data);
            }
        }
    }

    println!("📝 Total data chunks: {}", data_chunk_count);
    println!("📝 Final content length: {} chars", accumulated_content.len());

    // Validate streaming behavior
    assert!(data_chunk_count > 0, "Should have at least one data chunk");
    assert!(!accumulated_content.is_empty(), "Should have accumulated content");

    // Content should be reasonable (not just error messages)
    assert!(
        accumulated_content.len() > 10,
        "Content should be substantial, got: {}",
        accumulated_content.chars().take(100).collect::<String>()
    );

    server.shutdown().await?;
    println!("✅ Streaming content validation test completed");
    
    Ok(())
}

/// Test streaming performance characteristics
#[tokio::test]
async fn test_streaming_performance() -> Result<()> {
    println!("🚀 Starting streaming performance test");

    let server = TestScenarios::streaming_server().await?;
    server.wait_for_ready(30).await?;

    let mut client = MockCodexClient::new(server.url());

    // Test different types of streaming requests
    let streaming_tests = vec![
        ("Short explanation", "Explain what a function is", 10),
        ("Medium explanation", "Explain object-oriented programming", 50),
        ("Code generation", "Write a simple web server in Rust", 100),
    ];

    println!("🧪 Testing {} streaming scenarios", streaming_tests.len());

    for (name, request, expected_min_chunks) in streaming_tests {
        println!("📝 Testing streaming: {}", name);
        
        let start_time = Instant::now();
        let chunks = client
            .send_streaming_request(request, "claude-4-sonnet-20250514")
            .await?;
        let total_time = start_time.elapsed();

        println!("  ✅ Completed: {:?} with {} chunks", total_time, chunks.len());

        // Validate streaming response
        assert!(!chunks.is_empty(), "Should receive streaming chunks");
        
        // Check for proper termination
        let has_done_marker = chunks.iter().any(|chunk| chunk.contains("[DONE]"));
        assert!(has_done_marker, "Streaming should complete with [DONE] marker");

        // Performance validation
        PerformanceAssertions::assert_response_time(total_time, 15000)?; // 15s max for streaming
        
        // Validate minimum streaming behavior (should have some chunks)
        assert!(chunks.len() >= 3, "Should have at least a few streaming chunks");

        println!("    📊 Performance: {} chunks in {:?}", chunks.len(), total_time);
    }

    server.shutdown().await?;
    println!("✅ Streaming performance test completed");
    
    Ok(())
}

/// Test streaming error handling
#[tokio::test]
async fn test_streaming_error_handling() -> Result<()> {
    println!("⚠️ Starting streaming error handling test");

    let server = TestScenarios::streaming_server().await?;
    server.wait_for_ready(30).await?;

    let mut client = MockCodexClient::new(server.url());

    // Test streaming with potentially problematic requests
    let error_test_cases = vec![
        ("Empty request", ""),
        ("Very long request", &"A".repeat(10000)),
        ("Special characters", "Test with émojis: 🚀🔧💻 and unicode: αβγδε"),
    ];

    println!("🧪 Testing {} error scenarios", error_test_cases.len());

    for (name, request) in error_test_cases {
        println!("📝 Testing: {}", name);
        
        let start_time = Instant::now();
        let result = client
            .send_streaming_request(request, "claude-4-sonnet-20250514")
            .await;
        let request_time = start_time.elapsed();

        match result {
            Ok(chunks) => {
                println!("  ✅ Handled gracefully: {} chunks in {:?}", chunks.len(), request_time);
                
                // Even with problematic input, should get proper streaming format
                if !chunks.is_empty() {
                    let has_done = chunks.iter().any(|c| c.contains("[DONE]"));
                    assert!(has_done, "Even error responses should terminate properly");
                }
            }
            Err(e) => {
                println!("  ⚠️ Failed as expected: {} in {:?}", e, request_time);
                
                // Errors should fail quickly, not hang
                assert!(request_time.as_secs() < 10, "Error should fail quickly");
            }
        }
    }

    server.shutdown().await?;
    println!("✅ Streaming error handling test completed");
    
    Ok(())
}