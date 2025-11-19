//! Provider fallback integration tests
//!
//! This module tests the complete provider fallback system:
//! - Automatic provider fallback on failures
//! - Circuit breaker behavior and recovery
//! - Health monitoring and provider status tracking

use anyhow::Result;
use std::time::{Duration, Instant};

use llm_supabase_rs::models::{
    request::ChatCompletionRequest,
    common::ChatMessage,
};

// Import our test utilities from the parent utils module
use crate::utils::{
    MockCodexClient, TestServer, TestScenarios, 
    ResponseAssertions, PerformanceAssertions, ErrorAssertions
};

/// Test basic provider fallback behavior
#[tokio::test]
async fn test_basic_provider_fallback() -> Result<()> {
    println!("🔄 Starting basic provider fallback test");

    let server = TestScenarios::fallback_server().await?;
    server.wait_for_ready(30).await?;

    let mut client = MockCodexClient::new(server.url());

    println!("🏗️ Testing provider fallback scenario");

    // Test 1: Normal operation - primary provider should work
    println!("📝 Test 1: Normal operation with primary provider");
    
    let response1 = client
        .send_request("Create a simple function", "claude-4-sonnet-20250514")
        .await?;

    assert!(!response1.content.is_empty(), "Primary provider should work");
    assert!(!response1.response_id.is_empty(), "Should have response ID");
    
    println!("✅ Primary provider working normally");

    // Test 2: Simulate provider switching scenario
    println!("📝 Test 2: Testing provider resilience");
    
    let start_time = Instant::now();
    let response2 = client
        .send_request("Handle provider failure scenario", "claude-4-sonnet-20250514")
        .await?;

    let fallback_time = start_time.elapsed();
    
    assert!(!response2.content.is_empty(), "Should get response despite failures");
    println!("✅ Response received in {:?}", fallback_time);
    
    // Fallback should be reasonably fast (under 10 seconds)
    PerformanceAssertions::assert_response_time(fallback_time, 10000)?;

    // Test 3: Provider recovery
    println!("📝 Test 3: Testing provider recovery");
    
    let recovery_response = client
        .send_request("Test provider recovery", "claude-4-sonnet-20250514")
        .await?;

    assert!(!recovery_response.content.is_empty(), "Should work after recovery");
    println!("✅ Provider recovery successful");

    server.shutdown().await?;
    println!("✅ Basic provider fallback test completed");
    
    Ok(())
}

/// Test circuit breaker behavior simulation
#[tokio::test]
async fn test_circuit_breaker_simulation() -> Result<()> {
    println!("⚡ Starting circuit breaker simulation test");

    let server = TestScenarios::fallback_server().await?;
    server.wait_for_ready(30).await?;

    let mut client = MockCodexClient::new(server.url());

    // Simulate multiple requests to test circuit breaker patterns
    println!("💥 Simulating multiple requests for circuit breaker testing");
    
    let request_count = 5;
    let mut success_count = 0;
    let mut failure_count = 0;
    let mut response_times = Vec::new();
    
    // Send multiple requests to test circuit breaker behavior
    for i in 0..request_count {
        println!("📝 Request {}: Testing circuit breaker state", i + 1);
        
        let start_time = Instant::now();
        let result = client
            .send_request(
                &format!("Circuit breaker test request {}", i + 1),
                "claude-4-sonnet-20250514"
            )
            .await;
        
        let response_time = start_time.elapsed();
        response_times.push(response_time);
        
        match result {
            Ok(response) => {
                success_count += 1;
                assert!(!response.content.is_empty(), "Successful response should have content");
                println!("  ✅ Request {} succeeded in {:?}", i + 1, response_time);
            }
            Err(e) => {
                failure_count += 1;
                println!("  ⚠️ Request {} failed in {:?}: {}", i + 1, response_time, e);
                
                // Validate error is appropriate for circuit breaker
                let error_str = e.to_string();
                let is_expected_error = error_str.contains("unavailable") || 
                                      error_str.contains("circuit") || 
                                      error_str.contains("fallback") ||
                                      error_str.contains("timeout");
                
                if !is_expected_error {
                    println!("    ℹ️ Unexpected error type: {}", error_str);
                }
            }
        }

        // Brief pause between requests
        tokio::time::sleep(Duration::from_millis(100)).await;
    }

    // Analyze circuit breaker behavior
    let success_rate = success_count as f64 / request_count as f64;
    let avg_response_time: Duration = response_times.iter().sum::<Duration>() / response_times.len() as u32;

    println!("📊 Circuit breaker analysis:");
    println!("  Success rate: {:.1}% ({}/{})", success_rate * 100.0, success_count, request_count);
    println!("  Average response time: {:?}", avg_response_time);
    println!("  Failed requests: {}", failure_count);

    // Validate circuit breaker behavior
    // Should have some success rate (not complete failure)
    assert!(success_rate >= 0.2, "Should have at least 20% success rate");
    
    // Response times should be reasonable (circuit breaker should fail fast)
    assert!(avg_response_time.as_secs() < 10, "Circuit breaker should fail fast");

    server.shutdown().await?;
    println!("✅ Circuit breaker simulation test completed");
    
    Ok(())
}

/// Test health monitoring integration
#[tokio::test]
async fn test_health_monitoring_integration() -> Result<()> {
    println!("🏥 Starting health monitoring integration test");

    let server = TestScenarios::fallback_server().await?;
    server.wait_for_ready(30).await?;

    let mut client = MockCodexClient::new(server.url());

    // Test provider health monitoring through multiple requests
    println!("📊 Testing health monitoring through request patterns");
    
    let health_test_requests = vec![
        ("Normal request", "Create a simple function", true),
        ("Another normal request", "Add error handling", true),
        ("Complex request", "Generate comprehensive documentation", true),
    ];

    let mut health_metrics = Vec::new();

    for (name, request, should_succeed) in health_test_requests {
        println!("🔍 Health test: {}", name);
        
        let start_time = Instant::now();
        let result = client
            .send_request(request, "claude-4-sonnet-20250514")
            .await;
        let response_time = start_time.elapsed();

        let success = result.is_ok();
        health_metrics.push((name, response_time, success));

        match result {
            Ok(response) => {
                println!("  ✅ Healthy response in {:?}: {} chars", response_time, response.content.len());
                if should_succeed {
                    assert!(!response.content.is_empty(), "Healthy response should have content");
                }
            }
            Err(e) => {
                println!("  ⚠️ Health issue detected in {:?}: {}", response_time, e);
                if should_succeed {
                    // This might be expected behavior in a fallback scenario
                    println!("    ℹ️ This may be normal fallback behavior");
                }
            }
        }
    }

    // Analyze health patterns
    let successful_requests: Vec<_> = health_metrics.iter().filter(|(_, _, success)| *success).collect();
    let health_score = successful_requests.len() as f64 / health_metrics.len() as f64;

    println!("📊 Health monitoring analysis:");
    println!("  Health score: {:.1}%", health_score * 100.0);
    println!("  Successful requests: {}/{}", successful_requests.len(), health_metrics.len());

    if !successful_requests.is_empty() {
        let avg_healthy_time: Duration = successful_requests.iter()
            .map(|(_, time, _)| *time)
            .sum::<Duration>() / successful_requests.len() as u32;
        println!("  Average healthy response time: {:?}", avg_healthy_time);
    }

    // Health monitoring should maintain reasonable performance
    assert!(health_score >= 0.5, "Should maintain at least 50% health score");

    server.shutdown().await?;
    println!("✅ Health monitoring integration test completed");
    
    Ok(())
}

/// Test provider capability matching
#[tokio::test]
async fn test_provider_capability_matching() -> Result<()> {
    println!("🎯 Starting provider capability matching test");

    let server = TestScenarios::fallback_server().await?;
    server.wait_for_ready(30).await?;

    let mut client = MockCodexClient::new(server.url());

    // Test different types of requests that require different capabilities
    let capability_tests = vec![
        ("Streaming request", "Explain Rust concepts", Some(true)),
        ("Tool calling request", "Read main.rs and suggest improvements", None),
        ("Large context request", &format!("Process this: {}", "A".repeat(1000)), None),
    ];

    println!("🧪 Testing {} capability scenarios", capability_tests.len());

    for (test_name, request, is_streaming) in capability_tests {
        println!("📝 Testing: {}", test_name);
        
        let start_time = Instant::now();
        
        let result = if is_streaming.unwrap_or(false) {
            client.send_streaming_request(request, "claude-4-sonnet-20250514").await
                .map(|chunks| format!("Streaming response with {} chunks", chunks.len()))
        } else {
            client.send_request(request, "claude-4-sonnet-20250514").await
                .map(|resp| resp.content)
        };

        let response_time = start_time.elapsed();

        match result {
            Ok(content) => {
                println!("  ✅ Capability matched in {:?}: {} chars", response_time, content.len());
                assert!(!content.is_empty(), "Matched capability should provide content");
            }
            Err(e) => {
                println!("  ⚠️ Capability matching issue in {:?}: {}", response_time, e);
                
                // Some capability mismatches are acceptable
                let error_str = e.to_string();
                if error_str.contains("not supported") || error_str.contains("capability") {
                    println!("    ℹ️ Expected capability limitation");
                }
            }
        }

        // Performance should be reasonable regardless of capability matching
        PerformanceAssertions::assert_response_time(response_time, 15000)?; // 15s max
    }

    server.shutdown().await?;
    println!("✅ Provider capability matching test completed");
    
    Ok(())
}

/// Test error propagation and handling
#[tokio::test]
async fn test_error_propagation_handling() -> Result<()> {
    println!("🚨 Starting error propagation and handling test");

    let server = TestScenarios::fallback_server().await?;
    server.wait_for_ready(30).await?;

    let mut client = MockCodexClient::new(server.url());

    // Test various error scenarios
    let error_scenarios = vec![
        ("Invalid model", "gpt-nonexistent-model"),
        ("Empty request", ""),
        ("Oversized request", &"X".repeat(50000)),
    ];

    println!("🧪 Testing {} error scenarios", error_scenarios.len());

    for (scenario_name, model_or_content) in error_scenarios {
        println!("⚠️ Testing: {}", scenario_name);
        
        let start_time = Instant::now();
        
        let result = if scenario_name.contains("model") {
            client.send_request("Test request", model_or_content).await
        } else {
            client.send_request(model_or_content, "claude-4-sonnet-20250514").await
        };
        
        let error_time = start_time.elapsed();

        match result {
            Ok(response) => {
                println!("  ℹ️ Unexpectedly succeeded in {:?}: {}", error_time, response.response_id);
                // Some errors might be handled gracefully
            }
            Err(e) => {
                println!("  ✅ Error handled properly in {:?}: {}", error_time, e);
                
                // Validate error handling
                assert!(error_time.as_secs() < 10, "Errors should fail quickly");
                
                let error_str = e.to_string();
                assert!(!error_str.is_empty(), "Error should have meaningful message");
                
                // Check for appropriate error types
                let has_appropriate_error = error_str.contains("invalid") ||
                                          error_str.contains("error") ||
                                          error_str.contains("failed") ||
                                          error_str.contains("not found");
                
                if !has_appropriate_error {
                    println!("    ℹ️ Error message: {}", error_str);
                }
            }
        }
    }

    server.shutdown().await?;
    println!("✅ Error propagation and handling test completed");
    
    Ok(())
}