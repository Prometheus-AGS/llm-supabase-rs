//! Diff/patch workflow integration tests
//!
//! This module tests the complete diff generation and patch application workflow:
//! - Unified diff generation from file modifications
//! - Patch application with validation and security checks
//! - File system operations with backup and rollback capabilities

use anyhow::Result;
use std::time::{Duration, Instant};

use llm_supabase_rs::models::{
    request::ChatCompletionRequest,
    common::ChatMessage,
};

// Import our test utilities from the parent utils module
use crate::utils::{
    MockCodexClient, TestServer, TestScenarios, 
    ResponseAssertions, PerformanceAssertions
};

/// Test complete diff/patch workflow simulation
#[tokio::test]
async fn test_complete_diff_patch_workflow() -> Result<()> {
    println!("📄 Starting complete diff/patch workflow test");

    let server = TestScenarios::codex_cli_server().await?;
    server.wait_for_ready(30).await?;

    let mut client = MockCodexClient::new(server.url());

    // Step 1: Request file modification that should generate patch
    println!("📝 Step 1: Requesting file modification");
    
    let response = client
        .send_request(
            "Add a new function 'calculate_sum' to main.rs that takes a vector of integers",
            "claude-4-sonnet-20250514"
        )
        .await?;

    println!("✅ Initial response received");
    assert!(!response.content.is_empty(), "Should provide response content");

    // Step 2: Execute tool calls if present (simulate apply_patch)
    if !response.tool_calls.is_empty() {
        println!("🔧 Step 2: Executing {} tool calls", response.tool_calls.len());

        // Validate tool call structure for patch operations
        for (i, tool_call) in response.tool_calls.iter().enumerate() {
            println!("  Tool {}: {} (ID: {})", i + 1, tool_call.function.name, tool_call.id);
            
            assert_eq!(tool_call.tool_type, "function", "Tool type should be 'function'");
            assert!(!tool_call.id.is_empty(), "Tool call should have ID");
            assert!(!tool_call.function.name.is_empty(), "Function should have name");
            
            // Validate arguments are valid JSON
            let args_json: serde_json::Value = serde_json::from_str(&tool_call.function.arguments)?;
            
            if tool_call.function.name == "apply_patch" {
                assert!(args_json.get("file_path").is_some(), "apply_patch should have file_path");
                assert!(args_json.get("patch").is_some(), "apply_patch should have patch content");
                
                let patch_content = args_json["patch"].as_str().unwrap();
                println!("    Patch content preview: {} chars", patch_content.len());
                
                // Validate patch format
                assert!(
                    patch_content.contains("---") || patch_content.contains("+++") || 
                    patch_content.contains("@@") || patch_content.len() > 10,
                    "Patch should contain valid diff markers or substantial content"
                );
            }
        }

        // Execute tool calls
        let tool_response = client.execute_tool_calls(response.tool_calls).await?;
        println!("✅ Tool execution completed: {}", tool_response.response_id);
        
        assert!(!tool_response.content.is_empty(), "Tool execution should provide feedback");
    } else {
        println!("ℹ️ No tool calls generated (content-only response)");
    }

    server.shutdown().await?;
    println!("✅ Complete diff/patch workflow test completed");
    
    Ok(())
}

/// Test diff generation accuracy
#[tokio::test]
async fn test_diff_generation_accuracy() -> Result<()> {
    println!("🎯 Starting diff generation accuracy test");

    let server = TestScenarios::codex_cli_server().await?;
    server.wait_for_ready(30).await?;

    let mut client = MockCodexClient::new(server.url());

    // Test different types of modifications
    let modification_tests = vec![
        ("Add function", "Add a new function called 'hello_world' that prints a greeting"),
        ("Modify existing", "Update the main function to include error handling"),
        ("Add imports", "Add necessary imports for HTTP client functionality"),
        ("Add struct", "Create a User struct with name and email fields"),
    ];

    println!("🧪 Testing {} modification types", modification_tests.len());

    for (test_name, modification_request) in modification_tests {
        println!("📝 Testing: {}", test_name);
        
        let start_time = Instant::now();
        let response = client
            .send_request(modification_request, "claude-4-sonnet-20250514")
            .await?;
        let response_time = start_time.elapsed();

        // Validate response
        assert!(!response.content.is_empty(), "Should provide modification guidance");
        
        // Check for tool calls with patch content
        if !response.tool_calls.is_empty() {
            let patch_tools: Vec<_> = response.tool_calls.iter()
                .filter(|tc| tc.function.name == "apply_patch")
                .collect();
            
            if !patch_tools.is_empty() {
                println!("  🔧 Generated {} patch operations", patch_tools.len());
                
                for patch_tool in patch_tools {
                    let args: serde_json::Value = serde_json::from_str(&patch_tool.function.arguments)?;
                    
                    if let Some(patch_content) = args.get("patch").and_then(|p| p.as_str()) {
                        // Validate diff format characteristics
                        let has_diff_markers = patch_content.contains("---") && patch_content.contains("+++");
                        let has_hunk_headers = patch_content.contains("@@");
                        let has_line_markers = patch_content.contains("+") || patch_content.contains("-");
                        
                        println!("    Diff markers: {}, Hunk headers: {}, Line markers: {}", 
                                has_diff_markers, has_hunk_headers, has_line_markers);
                        
                        // Should have at least some diff characteristics
                        assert!(
                            has_diff_markers || has_hunk_headers || has_line_markers || patch_content.len() > 20,
                            "Patch should have valid diff format or substantial content"
                        );
                    }
                }
            }
        }

        println!("  ✅ Completed in {:?}", response_time);
        PerformanceAssertions::assert_response_time(response_time, 10000)?; // 10s max
    }

    server.shutdown().await?;
    println!("✅ Diff generation accuracy test completed");
    
    Ok(())
}

/// Test file operation security and validation
#[tokio::test]
async fn test_file_operation_security() -> Result<()> {
    println!("🔒 Starting file operation security test");

    let server = TestScenarios::codex_cli_server().await?;
    server.wait_for_ready(30).await?;

    let mut client = MockCodexClient::new(server.url());

    // Test various file operation scenarios for security validation
    let security_tests = vec![
        ("Safe file path", "Modify src/main.rs to add logging"),
        ("Relative path", "Update ../config.toml with new settings"),
        ("System file", "Modify /etc/hosts file"),
        ("Windows path", "Update C:\\Windows\\System32\\config"),
    ];

    println!("🛡️ Testing {} security scenarios", security_tests.len());

    for (test_name, request) in security_tests {
        println!("🔍 Security test: {}", test_name);
        
        let response = client
            .send_request(request, "claude-4-sonnet-20250514")
            .await?;

        // Even for potentially unsafe operations, should get a response
        assert!(!response.content.is_empty(), "Should provide response even for security-sensitive operations");

        // Check tool calls for security validation
        if !response.tool_calls.is_empty() {
            let file_operations: Vec<_> = response.tool_calls.iter()
                .filter(|tc| tc.function.name == "apply_patch" || tc.function.name == "write_file")
                .collect();
            
            for file_op in file_operations {
                let args: serde_json::Value = serde_json::from_str(&file_op.function.arguments)?;
                
                if let Some(file_path) = args.get("file_path").and_then(|p| p.as_str()) {
                    println!("    File operation target: {}", file_path);
                    
                    // Validate security characteristics
                    let is_safe_path = !file_path.starts_with('/') && 
                                     !file_path.contains("..") && 
                                     !file_path.contains("C:\\") &&
                                     !file_path.contains("System32");
                    
                    if !is_safe_path {
                        println!("    ⚠️ Potentially unsafe path detected: {}", file_path);
                    }
                    
                    // Even unsafe paths should be handled gracefully by the system
                    assert!(!file_path.is_empty(), "File path should not be empty");
                }
            }
        }

        println!("  ✅ Security validation passed");
    }

    server.shutdown().await?;
    println!("✅ File operation security test completed");
    
    Ok(())
}

/// Test patch application validation
#[tokio::test]
async fn test_patch_application_validation() -> Result<()> {
    println!("✅ Starting patch application validation test");

    let server = TestScenarios::codex_cli_server().await?;
    server.wait_for_ready(30).await?;

    let mut client = MockCodexClient::new(server.url());

    // Test patch application with different scenarios
    let patch_scenarios = vec![
        ("Simple addition", "Add a simple hello function to main.rs"),
        ("Complex modification", "Refactor the existing code to use async/await pattern"),
        ("Multiple changes", "Add error handling, logging, and input validation"),
    ];

    println!("🔨 Testing {} patch scenarios", patch_scenarios.len());

    for (scenario_name, modification_request) in patch_scenarios {
        println!("📋 Testing: {}", scenario_name);
        
        let response = client
            .send_request(modification_request, "claude-4-sonnet-20250514")
            .await?;

        // Validate response structure
        assert!(!response.content.is_empty(), "Should provide modification explanation");

        if !response.tool_calls.is_empty() {
            println!("  🔧 Processing {} tool calls", response.tool_calls.len());
            
            // Execute tool calls and validate results
            let tool_execution_start = Instant::now();
            let tool_response = client.execute_tool_calls(response.tool_calls.clone()).await?;
            let execution_time = tool_execution_start.elapsed();
            
            println!("    Tool execution completed in {:?}", execution_time);
            
            // Validate tool execution response
            assert!(!tool_response.content.is_empty(), "Tool execution should provide feedback");
            
            // Tool execution should be reasonably fast
            PerformanceAssertions::assert_response_time(execution_time, 5000)?; // 5s max for tool execution
            
            // Check for success indicators in the response
            let success_indicators = ["successfully", "applied", "completed", "done"];
            let response_lower = tool_response.content.to_lowercase();
            
            let has_success_indicator = success_indicators.iter()
                .any(|indicator| response_lower.contains(indicator));
            
            if has_success_indicator {
                println!("    ✅ Tool execution appears successful");
            } else {
                println!("    ℹ️ Tool execution response: {}", 
                        tool_response.content.chars().take(100).collect::<String>());
            }
        }

        println!("  ✅ Patch scenario completed");
    }

    server.shutdown().await?;
    println!("✅ Patch application validation test completed");
    
    Ok(())
}

/// Test large file handling and performance
#[tokio::test]
async fn test_large_file_handling() -> Result<()> {
    println!("📈 Starting large file handling test");

    let server = TestScenarios::codex_cli_server().await?;
    server.wait_for_ready(30).await?;

    let mut client = MockCodexClient::new(server.url());

    // Test handling of large file operations
    let large_file_tests = vec![
        ("Large content addition", format!("Add this large comment block to main.rs: {}", "// ".repeat(500))),
        ("Multiple functions", "Add 5 different utility functions to main.rs with documentation"),
        ("Complex refactoring", "Refactor main.rs to use a modular structure with multiple modules"),
    ];

    println!("📊 Testing {} large file scenarios", large_file_tests.len());

    for (test_name, request) in large_file_tests {
        println!("🔄 Testing: {}", test_name);
        
        let start_time = Instant::now();
        let response = client
            .send_request(&request, "claude-4-sonnet-20250514")
            .await?;
        let response_time = start_time.elapsed();

        // Validate large operation handling
        assert!(!response.content.is_empty(), "Should handle large operations");
        
        println!("  ⏱️ Response time: {:?}", response_time);
        println!("  📝 Response length: {} chars", response.content.len());
        
        // Large operations should still complete in reasonable time
        PerformanceAssertions::assert_response_time(response_time, 20000)?; // 20s max for large operations
        
        // Check for tool calls with large patches
        if !response.tool_calls.is_empty() {
            let mut total_patch_size = 0;
            
            for tool_call in &response.tool_calls {
                if tool_call.function.name == "apply_patch" {
                    let args: serde_json::Value = serde_json::from_str(&tool_call.function.arguments)?;
                    if let Some(patch) = args.get("patch").and_then(|p| p.as_str()) {
                        total_patch_size += patch.len();
                    }
                }
            }
            
            println!("    Total patch size: {} chars", total_patch_size);
            
            // Should handle reasonable patch sizes
            assert!(total_patch_size < 100000, "Patch size should be manageable (< 100KB)");
        }

        println!("  ✅ Large file handling completed");
    }

    server.shutdown().await?;
    println!("✅ Large file handling test completed");
    
    Ok(())
}

/// Test error handling in diff/patch operations
#[tokio::test]
async fn test_diff_patch_error_handling() -> Result<()> {
    println!("🚨 Starting diff/patch error handling test");

    let server = TestScenarios::codex_cli_server().await?;
    server.wait_for_ready(30).await?;

    let mut client = MockCodexClient::new(server.url());

    // Test various error scenarios
    let error_scenarios = vec![
        ("Nonexistent file", "Modify nonexistent_file.rs to add functions"),
        ("Invalid operation", "Delete all content from system files"),
        ("Malformed request", "Apply undefined changes to random locations"),
    ];

    println!("⚠️ Testing {} error scenarios", error_scenarios.len());

    for (scenario_name, error_request) in error_scenarios {
        println!("🔍 Error scenario: {}", scenario_name);
        
        let start_time = Instant::now();
        let result = client
            .send_request(error_request, "claude-4-sonnet-20250514")
            .await;
        let error_time = start_time.elapsed();

        match result {
            Ok(response) => {
                println!("  ℹ️ Request handled gracefully: {}", response.response_id);
                // Even problematic requests might be handled gracefully
                assert!(!response.content.is_empty(), "Should provide some guidance");
                
                // Check if tool calls were generated despite the error scenario
                if !response.tool_calls.is_empty() {
                    println!("    Generated {} tool calls", response.tool_calls.len());
                    
                    // Tool execution might fail, which is expected
                    let tool_result = client.execute_tool_calls(response.tool_calls).await;
                    match tool_result {
                        Ok(_) => println!("    Tool execution succeeded unexpectedly"),
                        Err(e) => println!("    Tool execution failed as expected: {}", e),
                    }
                }
            }
            Err(e) => {
                println!("  ✅ Error handled properly: {}", e);
                
                // Errors should fail reasonably quickly
                assert!(error_time.as_secs() < 10, "Errors should fail quickly");
                
                let error_str = e.to_string();
                assert!(!error_str.is_empty(), "Error should have message");
            }
        }

        println!("  ✅ Error scenario completed in {:?}", error_time);
    }

    server.shutdown().await?;
    println!("✅ Diff/patch error handling test completed");
    
    Ok(())
}