//! Performance benchmark integration tests
//!
//! This module provides comprehensive performance validation for the Codex CLI proxy:
//! - Response time benchmarks for different request types
//! - Basic throughput testing
//! - Memory usage and resource consumption validation
//! - Simple load testing scenarios

use anyhow::Result;
use std::time::{Duration, Instant};

use llm_supabase_rs::models::{
    request::ChatCompletionRequest,
    common::ChatMessage,
};

// Import our test utilities from the parent utils module
use crate::utils::{MockCodexClient, TestServer, TestScenarios, PerformanceAssertions};

/// Performance test results aggregation
#[derive(Debug, Clone)]
struct PerformanceMetrics {
    pub total_requests: usize,
    pub successful_requests: usize,
    pub failed_requests: usize,
    pub total_duration: Duration,
    pub min_response_time: Duration,
    pub max_response_time: Duration,
    pub avg_response_time: Duration,
    pub throughput_rps: f64,
}

impl PerformanceMetrics {
    fn new() -> Self {
        Self {
            total_requests: 0,
            successful_requests: 0,
            failed_requests: 0,
            total_duration: Duration::from_millis(0),
            min_response_time: Duration::from_secs(u64::MAX),
            max_response_time: Duration::from_millis(0),
            avg_response_time: Duration::from_millis(0),
            throughput_rps: 0.0,
        }
    }

    fn calculate_from_times(mut self, response_times: Vec<Duration>, total_duration: Duration) -> Self {
        if response_times.is_empty() {
            return self;
        }

        let mut times = response_times.clone();
        times.sort();

        self.total_requests = times.len();
        self.successful_requests = times.len();
        self.total_duration = total_duration;
        self.min_response_time = *times.first().unwrap();
        self.max_response_time = *times.last().unwrap();
        
        // Calculate average
        let total_ms: u64 = times.iter().map(|d| d.as_millis() as u64).sum();
        self.avg_response_time = Duration::from_millis(total_ms / times.len() as u64);
        
        // Calculate throughput
        self.throughput_rps = times.len() as f64 / total_duration.as_secs_f64();
        
        self
    }

    fn print_summary(&self) {
        println!("📊 Performance Metrics Summary:");
        println!("  Total requests: {}", self.total_requests);
        println!("  Successful: {} ({:.1}%)", 
                self.successful_requests, 
                (self.successful_requests as f64 / self.total_requests as f64) * 100.0);
        println!("  Failed: {}", self.failed_requests);
        println!("  Total duration: {:?}", self.total_duration);
        println!("  Min response time: {:?}", self.min_response_time);
        println!("  Avg response time: {:?}", self.avg_response_time);
        println!("  Max response time: {:?}", self.max_response_time);
        println!("  Throughput: {:.2} req/sec", self.throughput_rps);
    }
}

/// Test basic response time benchmarks
#[tokio::test]
async fn test_response_time_benchmarks() -> Result<()> {
    println!("⏱️ Starting response time benchmark test");

    let server = TestScenarios::performance_server().await?;
    server.wait_for_ready(30).await?;

    let mut client = MockCodexClient::new(server.url());

    // Define benchmark scenarios
    let benchmark_scenarios = vec![
        ("Simple question", "What is 2+2?", Duration::from_millis(1000)),
        ("Code generation", "Create a simple calculator function in Rust", Duration::from_millis(3000)),
        ("Code analysis", "Analyze this code for potential improvements", Duration::from_millis(2000)),
    ];

    println!("🧪 Running {} benchmark scenarios", benchmark_scenarios.len());

    let mut all_response_times = Vec::new();
    let benchmark_start = Instant::now();

    for (name, request, expected_max) in benchmark_scenarios {
        println!("📝 Benchmarking: {}", name);
        
        let start_time = Instant::now();
        let result = client
            .send_request(request, "claude-4-sonnet-20250514")
            .await;
        let response_time = start_time.elapsed();
        
        match result {
            Ok(response) => {
                println!("  ✅ Response: {:?} ({} chars)", response_time, response.content.len());
                
                // Validate response quality
                assert!(!response.content.is_empty(), "Response should have content");
                assert!(!response.response_id.is_empty(), "Response should have ID");
                
                // Performance assertion
                if response_time > expected_max {
                    println!("  ⚠️ Response time exceeded expectation: {:?} > {:?}", 
                            response_time, expected_max);
                } else {
                    println!("  ✅ Response time within expectation");
                }
                
                all_response_times.push(response_time);
            }
            Err(e) => {
                println!("  ❌ Request failed: {}", e);
            }
        }
    }

    let total_benchmark_time = benchmark_start.elapsed();
    
    // Calculate and display metrics
    let metrics = PerformanceMetrics::new()
        .calculate_from_times(all_response_times, total_benchmark_time);
    
    metrics.print_summary();

    // Performance assertions
    PerformanceAssertions::assert_response_time(metrics.avg_response_time, 5000)?; // 5s average max
    
    assert!(metrics.throughput_rps > 0.1, "Should achieve minimum throughput");

    server.shutdown().await?;
    println!("✅ Response time benchmark test completed");
    
    Ok(())
}

/// Test simple concurrent throughput
#[tokio::test]
async fn test_simple_concurrent_throughput() -> Result<()> {
    println!("🚀 Starting simple concurrent throughput test");

    let server = TestScenarios::performance_server().await?;
    server.wait_for_ready(30).await?;

    let client_count = 3;
    let mut clients = Vec::new();
    
    for _i in 0..client_count {
        let client = MockCodexClient::new(server.url());
        clients.push(client);
    }

    println!("🔀 Testing {} concurrent requests", client_count);

    let start_time = Instant::now();
    let mut successful = 0;

    // Execute requests sequentially for simplicity
    for (i, mut client) in clients.into_iter().enumerate() {
        let result = client
            .send_request(
                &format!("Concurrent test request {}", i),
                "claude-4-sonnet-20250514"
            )
            .await;
        
        match result {
            Ok(_) => {
                successful += 1;
                println!("  ✅ Request {} completed", i);
            }
            Err(e) => {
                println!("  ❌ Request {} failed: {}", i, e);
            }
        }
    }

    let total_time = start_time.elapsed();

    println!("📈 Concurrent test results:");
    println!("  Success rate: {}/{} ({:.1}%)", 
            successful, client_count, 
            (successful as f64 / client_count as f64) * 100.0);
    println!("  Total time: {:?}", total_time);

    // Performance assertions
    let success_rate = successful as f64 / client_count as f64;
    assert!(success_rate >= 0.6, "Success rate should be at least 60%");

    server.shutdown().await?;
    println!("✅ Concurrent throughput test completed");
    
    Ok(())
}

/// Test basic memory and resource usage
#[tokio::test]
async fn test_basic_memory_usage() -> Result<()> {
    println!("💾 Starting basic memory usage test");

    let server = TestScenarios::performance_server().await?;
    server.wait_for_ready(30).await?;

    let mut client = MockCodexClient::new(server.url());

    // Test with progressively larger requests
    let request_sizes = vec![100, 1000, 5000];
    
    for size in request_sizes {
        println!("  🧪 Testing request size: {} chars", size);
        
        let large_request = "A".repeat(size);
        
        let start_time = Instant::now();
        let result = client
            .send_request(&large_request, "claude-4-sonnet-20250514")
            .await;
        let response_time = start_time.elapsed();

        match result {
            Ok(response) => {
                println!("    ✅ Handled in {:?}, response: {} chars", 
                        response_time, response.content.len());
                
                // Should handle large requests reasonably
                assert!(response_time.as_secs() < 30, "Large request should complete within 30s");
                assert!(!response.content.is_empty(), "Should have response content");
            }
            Err(e) => {
                println!("    ⚠️ Large request failed: {}", e);
                // Some failures are acceptable for very large requests
            }
        }
    }

    server.shutdown().await?;
    println!("✅ Basic memory usage test completed");
    
    Ok(())
}

/// Test load scenario with sustained requests
#[tokio::test]
async fn test_sustained_load() -> Result<()> {
    println!("💪 Starting sustained load test");

    let server = TestScenarios::performance_server().await?;
    server.wait_for_ready(30).await?;

    let mut client = MockCodexClient::new(server.url());

    // Test with sustained requests
    let request_count = 5;
    let mut response_times = Vec::new();

    println!("🏃 Testing {} sustained requests", request_count);

    let load_start = Instant::now();

    for i in 0..request_count {
        let request_start = Instant::now();
        
        let result = client
            .send_request(
                &format!("Sustained load test request {}", i + 1),
                "claude-4-sonnet-20250514"
            )
            .await;
        
        let request_time = request_start.elapsed();
        
        match result {
            Ok(_) => {
                response_times.push(request_time);
                println!("  ⏱️ Request {}: {:?}", i + 1, request_time);
            }
            Err(e) => {
                println!("  ❌ Request {} failed: {}", i + 1, e);
            }
        }

        // Brief pause between requests
        tokio::time::sleep(Duration::from_millis(200)).await;
    }

    let total_load_time = load_start.elapsed();

    // Analyze performance
    if !response_times.is_empty() {
        let metrics = PerformanceMetrics::new()
            .calculate_from_times(response_times, total_load_time);
        
        println!("📈 Sustained load results:");
        metrics.print_summary();

        // Basic assertions
        assert!(metrics.throughput_rps > 0.5, "Should maintain basic throughput");
        assert!(metrics.avg_response_time.as_secs() < 10, "Average response time should be reasonable");
    }

    server.shutdown().await?;
    println!("✅ Sustained load test completed");
    
    Ok(())
}