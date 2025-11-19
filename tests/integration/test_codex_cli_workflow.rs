//! Codex CLI workflow integration tests across all 8 providers
//!
//! This module tests complete end-to-end Codex CLI workflows:
//! - Code generation and modification requests across all providers
//! - Multi-turn development sessions with provider comparisons
//! - Tool call execution (apply_patch, read_file, etc.) compatibility
//! - OpenAI API compatibility validation for all providers
//! - Provider-specific performance and capability testing

use anyhow::Result;
use std::collections::HashMap;
use std::time::{Duration, Instant};

use llm_supabase_rs::models::{
    request::ChatCompletionRequest,
    common::ChatMessage,
};

// Import our enhanced test utilities with multi-provider support
use crate::utils::{
    MockCodexClient, ProviderConfig, MultiProviderTestSession,
    CrossProviderTesting, TestScenarios, TestFixtures,
    ResponseAssertions, CrossProviderAssertions, PerformanceAssertions,
    assert_response_ok, assert_provider_response_ok, assert_provider_timing,
    assert_cross_provider_consistency,
};

/// Test basic Codex CLI code generation workflow across all providers
#[tokio::test]
async fn test_codex_cli_code_generation_all_providers() -> Result<()> {
    println!("🛠️ Starting Codex CLI code generation workflow test across all providers");

    let server = TestScenarios::codex_cli_server().await?;
    server.wait_for_ready(30).await?;

    // Create multi-provider test session
    let mut session = MultiProviderTestSession::new(server.url());

    println!("📝 Testing code generation across all providers");
    let responses = session.test_all_providers("Create a simple HTTP server in Rust using axum").await?;

    // Validate all responses
    assert_cross_provider_consistency!(responses);

    // Check provider-specific behavior
    for (provider_name, response) in &responses {
        assert_provider_response_ok!(response, provider_name);
        
        // Validate provider-specific content patterns
        ResponseAssertions::assert_provider_specific_content(response, provider_name)?;
        
        println!("✅ Provider {} generated valid code response", provider_name);
    }

    // Get performance comparison
    let performance = session.get_performance_comparison();
    println!("📊 Performance ranking (fastest to slowest):");
    for (provider, avg_time) in performance {
        println!("  {}: {:.0}ms", provider, avg_time);
    }

    // Validate success rates
    let success_rates = session.get_success_rates();
    for (provider_name, success_rate) in success_rates {
        assert!(
            success_rate >= 0.8,
            "Provider {} success rate {:.1}% below minimum 80%",
            provider_name, success_rate * 100.0
        );
    }

    server.shutdown().await?;
    println!("✅ Codex CLI code generation workflow test completed across all providers");
    
    Ok(())
}

/// Test Codex CLI tool execution workflow across providers that support tools
#[tokio::test]
async fn test_codex_cli_tool_execution_all_providers() -> Result<()> {
    println!("🔧 Starting Codex CLI tool execution workflow test across providers");

    let server = TestScenarios::codex_cli_server().await?;
    server.wait_for_ready(30).await?;

    // Get providers that support tools
    let tool_supporting_providers = vec![
        "openai", "anthropic", "vertex", "azure_openai",
        "aws_bedrock", "cohere", "mistral"
    ]; // Note: Groq has limited tool support

    let mut successful_providers = 0;
    let mut tool_responses = HashMap::new();

    for provider_name in tool_supporting_providers {
        let config = match provider_name {
            "openai" => ProviderConfig::openai(),
            "anthropic" => ProviderConfig::anthropic(),
            "vertex" => ProviderConfig::vertex(),
            "azure_openai" => ProviderConfig::azure_openai(),
            "aws_bedrock" => ProviderConfig::aws_bedrock(),
            "cohere" => ProviderConfig::cohere(),
            "mistral" => ProviderConfig::mistral(),
            _ => continue,
        };

        let mut client = MockCodexClient::with_provider_config(server.url(), config);

        println!("📝 Testing tool execution with provider: {}", provider_name);
        
        match client.send_request_with_provider("Read the main.rs file and add a new function to it").await {
            Ok(response) => {
                assert_provider_response_ok!(response, provider_name);
                
                // Check for tool calls
                if !response.tool_calls.is_empty() {
                    println!("🔧 Provider {} executed {} tool calls", provider_name, response.tool_calls.len());
                    
                    // Validate tool call structure
                    ResponseAssertions::assert_valid_tool_calls(&response.tool_calls)?;
                    
                    // Execute the tool calls
                    let tool_result = client.execute_tool_calls(response.tool_calls.clone()).await?;
                    assert!(!tool_result.content.is_empty(), "Tool execution should provide response");
                    
                    tool_responses.insert(provider_name.to_string(), response);
                } else {
                    println!("ℹ️ Provider {} provided content without tool calls", provider_name);
                }
                
                successful_providers += 1;
            }
            Err(e) => {
                println!("❌ Provider {} failed tool execution: {}", provider_name, e);
            }
        }
    }

    // Validate tool calling consistency across providers
    if !tool_responses.is_empty() {
        crate::utils::ToolCallAssertions::assert_consistent_tool_calling(&tool_responses)?;
        crate::utils::ToolCallAssertions::assert_tool_arguments_valid(&tool_responses)?;
        
        println!("✅ Tool calling validated across {} providers", tool_responses.len());
    }

    assert!(
        successful_providers >= 5, // Most providers should succeed
        "At least 5 providers should support tool execution, got {}",
        successful_providers
    );

    server.shutdown().await?;
    println!("✅ Codex CLI tool execution workflow test completed");
    
    Ok(())
}

/// Test OpenAI API compatibility across all providers
#[tokio::test]
async fn test_openai_api_compatibility_all_providers() -> Result<()> {
    println!("🔌 Starting OpenAI API compatibility test across all providers");

    let server = TestScenarios::codex_cli_server().await?;
    server.wait_for_ready(30).await?;

    let all_providers = ProviderConfig::all_providers();
    let mut compatibility_results = HashMap::new();

    for config in all_providers {
        let mut client = MockCodexClient::with_provider_config(server.url(), config.clone());

        println!("🔍 Testing OpenAI compatibility for provider: {}", config.name);
        
        match client.send_request_with_provider("Write a simple HTTP server in Rust using axum").await {
            Ok(response) => {
                // Validate OpenAI-compatible response structure
                assert_response_ok!(response);
                
                // Check response ID format (should be compatible)
                assert!(!response.response_id.is_empty(), "Should have response ID");
                
                // Validate usage statistics if present
                if let Some(usage) = &response.usage {
                    println!("📊 Provider {} token usage - Prompt: {}, Completion: {}, Total: {}", 
                            config.name, usage.prompt_tokens, usage.completion_tokens, usage.total_tokens);
                    
                    ResponseAssertions::assert_reasonable_usage(&response)?;
                }
                
                // Test tool call format compatibility if present
                if !response.tool_calls.is_empty() {
                    println!("🔧 Validating tool call format for provider: {}", config.name);
                    ResponseAssertions::assert_valid_tool_calls(&response.tool_calls)?;
                }
                
                compatibility_results.insert(config.name.clone(), true);
                println!("✅ Provider {} passed OpenAI compatibility test", config.name);
            }
            Err(e) => {
                println!("❌ Provider {} failed compatibility test: {}", config.name, e);
                compatibility_results.insert(config.name.clone(), false);
            }
        }
    }

    // Calculate compatibility rate
    let compatible_count = compatibility_results.values().filter(|&&v| v).count();
    let total_count = compatibility_results.len();
    let compatibility_rate = compatible_count as f64 / total_count as f64;

    println!("📊 OpenAI API Compatibility Results:");
    for (provider, compatible) in &compatibility_results {
        println!("  {}: {}", provider, if *compatible { "✅ Compatible" } else { "❌ Issues" });
    }
    println!("  Overall compatibility rate: {:.1}% ({}/{})", 
             compatibility_rate * 100.0, compatible_count, total_count);

    assert!(
        compatibility_rate >= 0.75, // At least 75% should be compatible
        "OpenAI compatibility rate {:.1}% below minimum 75%",
        compatibility_rate * 100.0
    );

    server.shutdown().await?;
    println!("✅ OpenAI API compatibility test completed successfully");
    
    Ok(())
}

/// Test realistic development session across multiple providers
#[tokio::test]
async fn test_realistic_development_session_cross_provider() -> Result<()> {
    println!("💼 Starting realistic development session test with provider comparison");

    let server = TestScenarios::codex_cli_server().await?;
    server.wait_for_ready(30).await?;

    // Test with a subset of providers for performance
    let test_providers = vec![
        ("openai", ProviderConfig::openai()),
        ("anthropic", ProviderConfig::anthropic()),
        ("groq", ProviderConfig::groq()), // Test speed difference
        ("mistral", ProviderConfig::mistral()),
    ];

    let development_tasks = vec![
        ("Create a User struct with validation", "Initial struct creation"),
        ("Add database persistence methods", "Database integration"),
        ("Implement user authentication", "Security features"),
        ("Add comprehensive error handling", "Error management"),
        ("Generate integration tests", "Testing"),
    ];

    println!("🔄 Simulating {} development tasks across {} providers", 
             development_tasks.len(), test_providers.len());

    let mut provider_results = HashMap::new();

    for (provider_name, config) in test_providers {
        println!("📋 Testing development session with provider: {}", provider_name);
        
        let mut client = MockCodexClient::with_provider_config(server.url(), config);
        let session_start = Instant::now();
        let mut task_times = Vec::new();
        let mut successful_tasks = 0;

        for (i, (task, description)) in development_tasks.iter().enumerate() {
            println!("  Task {}: {} ({})", i + 1, description, provider_name);
            
            let task_start = Instant::now();
            match client.send_request_with_provider(task).await {
                Ok(response) => {
                    let task_duration = task_start.elapsed();
                    task_times.push(task_duration);
                    successful_tasks += 1;
                    
                    println!("    ✅ Completed in {:?}: {} chars", 
                             task_duration, response.content.len());
                    
                    // Validate response quality
                    assert!(!response.content.is_empty(), "Each task should generate content");
                    assert!(response.content.len() > 50, "Responses should be substantial");

                    // Execute any tool calls if provider supports them
                    if !response.tool_calls.is_empty() && config.supports_tools {
                        println!("    🔧 Executing {} tool calls", response.tool_calls.len());
                        let _tool_response = client.execute_tool_calls(response.tool_calls).await?;
                    }
                }
                Err(e) => {
                    println!("    ❌ Failed: {}", e);
                    task_times.push(Duration::from_secs(30)); // Penalty time
                }
            }
        }

        let total_session_time = session_start.elapsed();
        let avg_task_time = if !task_times.is_empty() {
            task_times.iter().sum::<Duration>() / task_times.len() as u32
        } else {
            Duration::from_secs(30)
        };

        println!("📊 Provider {} Development Session Results:", provider_name);
        println!("    Total time: {:?}", total_session_time);
        println!("    Average task time: {:?}", avg_task_time);
        println!("    Successful tasks: {}/{}", successful_tasks, development_tasks.len());

        // Performance assertions based on provider expectations
        match provider_name {
            "groq" => {
                // Groq should be fastest
                PerformanceAssertions::assert_response_time(avg_task_time, 5000)?; 
            }
            "mistral" => {
                PerformanceAssertions::assert_response_time(avg_task_time, 8000)?;
            }
            _ => {
                PerformanceAssertions::assert_response_time(avg_task_time, 12000)?; 
            }
        }

        provider_results.insert(provider_name.to_string(), (total_session_time, successful_tasks));
    }

    // Compare results across providers
    println!("📊 Cross-Provider Development Session Comparison:");
    let mut performance_ranking: Vec<_> = provider_results.iter().collect();
    performance_ranking.sort_by_key(|(_, (duration, _))| *duration);

    for (rank, (provider, (duration, success_count))) in performance_ranking.iter().enumerate() {
        println!("  {}. {}: {:?} ({}/{} tasks)", 
                 rank + 1, provider, duration, success_count, development_tasks.len());
    }

    // Validate that most providers completed most tasks
    let avg_success_rate = provider_results.values()
        .map(|(_, success)| *success as f64 / development_tasks.len() as f64)
        .sum::<f64>() / provider_results.len() as f64;

    assert!(
        avg_success_rate >= 0.7,
        "Average success rate across providers {:.1}% below minimum 70%",
        avg_success_rate * 100.0
    );

    server.shutdown().await?;
    println!("✅ Realistic development session test completed successfully");
    
    Ok(())
}

/// Test concurrent Codex CLI requests across providers
#[tokio::test]
async fn test_concurrent_codex_requests_all_providers() -> Result<()> {
    println!("⚡ Starting concurrent Codex CLI requests test across providers");

    let server = TestScenarios::codex_cli_server().await?;
    server.wait_for_ready(30).await?;

    // Test concurrent requests with multiple providers
    let concurrent_providers = vec![
        ("openai", ProviderConfig::openai()),
        ("anthropic", ProviderConfig::anthropic()),
        ("groq", ProviderConfig::groq()),
    ];

    let concurrent_count = 3;
    let mut provider_results = HashMap::new();

    for (provider_name, config) in concurrent_providers {
        println!("🔀 Testing {} concurrent requests with provider: {}", concurrent_count, provider_name);

        let mut tasks = Vec::new();
        let start_time = Instant::now();

        // Create concurrent tasks
        for i in 0..concurrent_count {
            let server_url = server.url().to_string();
            let config_clone = config.clone();
            let request_msg = format!("Create a {} function for data processing", 
                                    match i { 0 => "sorting", 1 => "filtering", _ => "validation" });

            let task = tokio::spawn(async move {
                let mut client = MockCodexClient::with_provider_config(&server_url, config_clone);
                client.send_request_with_provider(&request_msg).await
            });
            
            tasks.push(task);
        }

        // Wait for all tasks to complete
        let mut successful_requests = 0;
        let mut total_response_time = Duration::from_secs(0);

        for (i, task) in tasks.into_iter().enumerate() {
            match task.await {
                Ok(Ok(response)) => {
                    successful_requests += 1;
                    total_response_time += Duration::from_millis(response.response_time_ms);
                    println!("    ✅ Request {} succeeded: {} chars", i, response.content.len());
                    
                    assert!(!response.content.is_empty(), "Response should have content");
                    assert!(!response.response_id.is_empty(), "Response should have ID");
                    assert_provider_timing!(provider_name, Duration::from_millis(response.response_time_ms));
                }
                Ok(Err(e)) => {
                    println!("    ❌ Request {} failed: {}", i, e);
                }
                Err(e) => {
                    println!("    ❌ Request {} task failed: {}", i, e);
                }
            }
        }

        let concurrent_time = start_time.elapsed();
        let avg_response_time = if successful_requests > 0 {
            total_response_time / successful_requests as u32
        } else {
            Duration::from_secs(30)
        };

        println!("📊 Provider {} Concurrent Results:", provider_name);
        println!("    Success rate: {}/{} ({:.1}%)", 
                successful_requests, concurrent_count,
                successful_requests as f64 / concurrent_count as f64 * 100.0);
        println!("    Total time: {:?}", concurrent_time);
        println!("    Average response time: {:?}", avg_response_time);

        // Validate performance based on provider expectations
        let expected_performance = match provider_name {
            "groq" => (0.8, 5000), // (min_success_rate, max_avg_time_ms)
            "openai" => (0.9, 10000),
            "anthropic" => (0.85, 12000),
            _ => (0.7, 15000),
        };

        PerformanceAssertions::assert_success_rate(
            successful_requests, concurrent_count, expected_performance.0
        )?;

        if successful_requests > 0 {
            PerformanceAssertions::assert_response_time(avg_response_time, expected_performance.1)?;
        }

        provider_results.insert(
            provider_name.to_string(), 
            (successful_requests, concurrent_time, avg_response_time)
        );
    }

    // Compare concurrent performance across providers
    println!("📊 Concurrent Performance Comparison:");
    for (provider, (success, total_time, avg_time)) in &provider_results {
        println!("  {}: {}/{} success, avg {:?}", 
                 provider, success, concurrent_count, avg_time);
    }

    server.shutdown().await?;
    println!("✅ Concurrent requests test completed successfully across all providers");
    
    Ok(())
}

/// Test provider-specific features and capabilities
#[tokio::test]
async fn test_provider_specific_capabilities() -> Result<()> {
    println!("🎯 Testing provider-specific capabilities and features");

    let server = TestScenarios::codex_cli_server().await?;
    server.wait_for_ready(30).await?;

    let all_providers = ProviderConfig::all_providers();
    let mut capability_results = HashMap::new();

    for config in all_providers {
        println!("🔍 Testing capabilities for provider: {}", config.name);
        
        let mut client = MockCodexClient::with_provider_config(server.url(), config.clone());
        let mut provider_capabilities = HashMap::new();

        // Test streaming capability
        if config.supports_streaming {
            println!("  Testing streaming capability...");
            match client.send_streaming_request_with_provider("Explain sorting algorithms step by step").await {
                Ok(chunks) => {
                    assert!(!chunks.is_empty(), "Streaming should produce chunks");
                    provider_capabilities.insert("streaming".to_string(), true);
                    println!("    ✅ Streaming works: {} chunks", chunks.len());
                }
                Err(e) => {
                    println!("    ❌ Streaming failed: {}", e);
                    provider_capabilities.insert("streaming".to_string(), false);
                }
            }
        } else {
            println!("  ⏭️  Streaming not supported");
            provider_capabilities.insert("streaming".to_string(), false);
        }

        // Test tool calling capability
        if config.supports_tools {
            println!("  Testing tool calling capability...");
            match client.send_request_with_provider("Read a file and modify it").await {
                Ok(response) => {
                    let has_tools = !response.tool_calls.is_empty();
                    provider_capabilities.insert("tools".to_string(), has_tools);
                    
                    if has_tools {
                        println!("    ✅ Tool calling works: {} tools", response.tool_calls.len());
                    } else {
                        println!("    ℹ️ Tool calling available but not used in this response");
                    }
                }
                Err(e) => {
                    println!("    ❌ Tool calling test failed: {}", e);
                    provider_capabilities.insert("tools".to_string(), false);
                }
            }
        } else {
            println!("  ⏭️  Tool calling not supported");
            provider_capabilities.insert("tools".to_string(), false);
        }

        // Test response speed (provider-specific expectations)
        println!("  Testing response speed...");
        let speed_start = Instant::now();
        match client.send_request_with_provider("Write a hello world function").await {
            Ok(response) => {
                let response_time = speed_start.elapsed();
                let is_fast = response_time.as_millis() <= config.expected_response_time_ms as u128;
                provider_capabilities.insert("speed".to_string(), is_fast);
                
                println!("    Response time: {:?} (expected: {}ms) {}", 
                         response_time, config.expected_response_time_ms,
                         if is_fast { "✅" } else { "⚠️" });
            }
            Err(e) => {
                println!("    ❌ Speed test failed: {}", e);
                provider_capabilities.insert("speed".to_string(), false);
            }
        }

        capability_results.insert(config.name.clone(), provider_capabilities);
    }

    // Analyze capability results
    println!("📊 Provider Capability Summary:");
    println!("  Provider     | Streaming | Tools | Speed");
    println!("  -------------|-----------|-------|-------");
    
    for (provider_name, capabilities) in &capability_results {
        let streaming = if capabilities.get("streaming").unwrap_or(&false) { "✅" } else { "❌" };
        let tools = if capabilities.get("tools").unwrap_or(&false) { "✅" } else { "❌" };
        let speed = if capabilities.get("speed").unwrap_or(&false) { "✅" } else { "⚠️" };
        
        println!("  {:12} | {:9} | {:5} | {}", provider_name, streaming, tools, speed);
    }

    // Validate that we have good capability coverage
    let streaming_count = capability_results.values()
        .filter(|caps| caps.get("streaming").unwrap_or(&false))
        .count();
    
    let tools_count = capability_results.values()
        .filter(|caps| caps.get("tools").unwrap_or(&false))
        .count();

    assert!(streaming_count >= 6, "At least 6 providers should support streaming");
    assert!(tools_count >= 6, "At least 6 providers should support tools"); 

    server.shutdown().await?;
    println!("✅ Provider-specific capabilities test completed");
    
    Ok(())
}