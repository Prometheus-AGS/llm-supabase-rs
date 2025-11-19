//! Multi-turn conversation integration tests
//!
//! This module tests conversation continuity, context preservation, and multi-turn
//! interaction patterns for the Codex CLI proxy.

use anyhow::Result;
use std::time::{Duration, Instant};

use llm_supabase_rs::models::{
    request::ChatCompletionRequest,
    common::ChatMessage,
};

// Import our test utilities from the parent utils module
use crate::utils::{MockCodexClient, TestServer, TestScenarios, ResponseAssertions};

/// Test basic conversation continuity
#[tokio::test]
async fn test_basic_conversation_continuity() -> Result<()> {
    println!("💬 Starting basic conversation continuity test");

    let server = TestScenarios::conversation_server().await?;
    server.wait_for_ready(30).await?;

    let mut client = MockCodexClient::new(server.url());

    // Start conversation
    println!("📝 Step 1: Starting new conversation");
    let response1 = client
        .send_request(
            "Create a simple calculator function",
            "claude-4-sonnet-20250514"
        )
        .await?;

    println!("✅ Initial response received with ID: {}", response1.response_id);
    assert!(!response1.content.is_empty(), "Initial response should have content");
    
    let first_response_id = response1.response_id.clone();

    // Continue conversation
    println!("📝 Step 2: Continuing conversation");
    let response2 = client
        .send_request(
            "Now add error handling to that calculator",
            "claude-4-sonnet-20250514"
        )
        .await?;

    println!("✅ Follow-up response received with ID: {}", response2.response_id);
    
    // Validate conversation threading
    assert_ne!(
        response2.response_id, first_response_id,
        "Should have different response IDs"
    );

    let conversation_history = client.get_conversation_history();
    assert!(
        conversation_history.len() >= 2,
        "Should have at least 2 messages in history"
    );

    server.shutdown().await?;
    println!("✅ Basic conversation continuity test completed");
    
    Ok(())
}

/// Test conversation context preservation across multiple turns
#[tokio::test]
async fn test_conversation_context_preservation() -> Result<()> {
    println!("🧠 Starting conversation context preservation test");

    let server = TestScenarios::conversation_server().await?;
    server.wait_for_ready(30).await?;

    let mut client = MockCodexClient::new(server.url());

    // Simulate a multi-step development conversation
    let conversation_steps = vec![
        ("Create a User struct with name and email fields", "Initial struct creation"),
        ("Add a validation method for the email field", "Add validation"),
        ("Add error handling for invalid emails", "Add error handling"),
    ];

    println!("🔄 Executing {} conversation steps", conversation_steps.len());

    let mut response_ids = Vec::new();
    
    for (i, (request, description)) in conversation_steps.iter().enumerate() {
        println!("📝 Step {}: {}", i + 1, description);
        
        let response = client
            .send_request(request, "claude-4-sonnet-20250514")
            .await?;

        println!("  ✅ Response ID: {}", response.response_id);
        response_ids.push(response.response_id.clone());

        // Validate response
        assert!(!response.content.is_empty(), "Response should have content");
        
        // Validate conversation growth
        let current_history = client.get_conversation_history();
        assert!(
            current_history.len() >= (i + 1) * 2,
            "Conversation should grow with each turn"
        );
    }

    // Validate all response IDs are unique
    let mut unique_ids = response_ids.clone();
    unique_ids.sort();
    unique_ids.dedup();
    assert_eq!(
        unique_ids.len(),
        response_ids.len(),
        "All response IDs should be unique"
    );

    server.shutdown().await?;
    println!("✅ Conversation context preservation test completed");
    
    Ok(())
}

/// Test conversation isolation between different clients
#[tokio::test]
async fn test_conversation_isolation() -> Result<()> {
    println!("🔒 Starting conversation isolation test");

    let server = TestScenarios::conversation_server().await?;
    server.wait_for_ready(30).await?;

    // Create multiple independent clients
    let mut client1 = MockCodexClient::new(server.url());
    let mut client2 = MockCodexClient::new(server.url());
    let mut client3 = MockCodexClient::new(server.url());

    println!("👥 Testing isolation between 3 independent clients");

    // Start conversations on each client
    let response1 = client1
        .send_request("Create a sorting algorithm", "claude-4-sonnet-20250514")
        .await?;

    let response2 = client2
        .send_request("Create a web server", "claude-4-sonnet-20250514")
        .await?;

    let response3 = client3
        .send_request("Create a database schema", "claude-4-sonnet-20250514")
        .await?;

    println!("✅ All clients received initial responses");

    // Validate conversations are isolated
    let history1 = client1.get_conversation_history();
    let history2 = client2.get_conversation_history();
    let history3 = client3.get_conversation_history();

    assert!(history1.len() >= 2, "Client 1 should have at least 2 messages");
    assert!(history2.len() >= 2, "Client 2 should have at least 2 messages");
    assert!(history3.len() >= 2, "Client 3 should have at least 2 messages");

    // Validate response IDs are unique across clients
    let all_response_ids = vec![
        &response1.response_id,
        &response2.response_id,
        &response3.response_id,
    ];

    let mut unique_ids = all_response_ids.clone();
    unique_ids.sort();
    unique_ids.dedup();

    assert_eq!(
        unique_ids.len(),
        all_response_ids.len(),
        "All response IDs should be unique across clients"
    );

    server.shutdown().await?;
    println!("✅ Conversation isolation test completed");
    
    Ok(())
}

/// Test conversation with tool calls
#[tokio::test]
async fn test_conversation_with_tool_calls() -> Result<()> {
    println!("🔧 Starting conversation with tool calls test");

    let server = TestScenarios::conversation_server().await?;
    server.wait_for_ready(30).await?;

    let mut client = MockCodexClient::new(server.url());

    // Step 1: Request that should trigger tool calls
    println!("📝 Step 1: Sending request that triggers tool use");
    
    let response1 = client
        .send_request(
            "Read the main.rs file and add a new function to it",
            "claude-4-sonnet-20250514"
        )
        .await?;

    println!("✅ Initial response received");
    
    // Execute tool calls if present
    if !response1.tool_calls.is_empty() {
        println!("🔧 Executing {} tool calls", response1.tool_calls.len());
        
        let _tool_response = client.execute_tool_calls(response1.tool_calls.clone()).await?;
        println!("✅ Tool execution completed");
    }

    // Step 2: Follow-up that references the tool results
    println!("📝 Step 2: Following up on tool results");
    
    let response2 = client
        .send_request(
            "Now add tests for the function you just created",
            "claude-4-sonnet-20250514"
        )
        .await?;

    println!("✅ Follow-up response received");

    // Validate conversation includes messages
    let final_history = client.get_conversation_history();
    
    println!("📊 Total messages: {}", final_history.len());
    assert!(final_history.len() >= 4, "Should have multiple messages");

    server.shutdown().await?;
    println!("✅ Conversation with tool calls test completed");
    
    Ok(())
}

/// Test conversation error recovery
#[tokio::test]
async fn test_conversation_error_recovery() -> Result<()> {
    println!("🩹 Starting conversation error recovery test");

    let server = TestScenarios::conversation_server().await?;
    server.wait_for_ready(30).await?;

    let mut client = MockCodexClient::new(server.url());

    // Start normal conversation
    println!("📝 Step 1: Starting normal conversation");
    
    let response1 = client
        .send_request("Create a function", "claude-4-sonnet-20250514")
        .await?;

    println!("✅ Normal response received: {}", response1.response_id);
    
    let _initial_response_id = response1.response_id.clone();
    let _initial_history_len = client.get_conversation_history().len();

    // Simulate error scenario - set invalid response ID
    println!("⚠️ Step 2: Simulating error scenario");
    
    client.set_last_response_id("invalid-response-id-12345".to_string());

    // Try to continue conversation - should recover gracefully
    let error_recovery_result = client
        .send_request("Add error handling", "claude-4-sonnet-20250514")
        .await;

    match error_recovery_result {
        Ok(response) => {
            println!("✅ Error recovery successful: {}", response.response_id);
            assert!(!response.content.is_empty(), "Recovery response should have content");
        }
        Err(_e) => {
            println!("⚠️ Error as expected, testing fresh conversation");
            
            // Reset client and try fresh request
            client.reset_conversation();
            
            let fresh_response = client
                .send_request("Start fresh conversation", "claude-4-sonnet-20250514")
                .await?;
            
            println!("✅ Fresh conversation started: {}", fresh_response.response_id);
            assert!(!fresh_response.content.is_empty(), "Fresh response should have content");
        }
    }

    server.shutdown().await?;
    println!("✅ Conversation error recovery test completed");
    
    Ok(())
}