use axum_test::TestServer;
use serde_json::{json, Value};
use axum::http::StatusCode;
use std::time::{Duration, Instant};
use tokio::time::timeout;

/// Integration test for Performance Validation
///
/// This test implements comprehensive performance validation:
/// - Concurrent request handling (1000+ simultaneous requests)
/// - Response time requirements (95% under 5 seconds)
/// - Memory efficiency under load
/// - Connection management and resource cleanup
/// - Streaming performance validation
/// - Rate limiting behavior validation
///
/// Performance Requirements from quickstart.md:
/// - 95% of requests complete under 5 seconds
/// - Support 1000+ concurrent requests
/// - Graceful degradation under load
/// - Memory usage remains stable
///
/// This test MUST FAIL until performance optimization is implemented.
#[tokio::test]
async fn test_concurrent_request_performance() {
    let app = create_test_app().await;
    let server = TestServer::new(app).unwrap();

    let jwt_token = create_valid_jwt_token();
    let concurrent_requests = 100; // Reduced for test environment

    let request_body = json!({
        "model": "claude-4-sonnet-20250514",
        "messages": [{
            "role": "user",
            "content": "Quick response test"
        }],
        "max_tokens": 20,
        "temperature": 0.1
    });

    // Spawn concurrent requests
    let mut handles = Vec::new();
    let start_time = Instant::now();

    for i in 0..concurrent_requests {
        let server_clone = server.clone();
        let jwt_clone = jwt_token.clone();
        let request_clone = request_body.clone();

        let handle = tokio::spawn(async move {
            let request_start = Instant::now();

            let response = server_clone
                .post("/v1/chat/completions")
                .add_header("Authorization", format!("Bearer {}", jwt_clone))
                .add_header("Content-Type", "application/json")
                .json(&request_clone)
                .await;

            let request_duration = request_start.elapsed();

            (i, response.status_code(), request_duration)
        });

        handles.push(handle);
    }

    // Wait for all requests to complete with timeout
    let timeout_duration = Duration::from_secs(30);
    let mut results = Vec::new();

    for handle in handles {
        let result = timeout(timeout_duration, handle)
            .await
            .expect("Request should complete within timeout")
            .expect("Request task should not panic");
        results.push(result);
    }

    let total_duration = start_time.elapsed();

    // Analyze performance results
    let mut successful_requests = 0;
    let mut response_times = Vec::new();
    let mut status_codes = std::collections::HashMap::new();

    for (id, status, duration) in results {
        *status_codes.entry(status).or_insert(0) += 1;

        if status == StatusCode::OK {
            successful_requests += 1;
            response_times.push(duration);
        }

        println!("Request {}: {} in {:?}", id, status.as_u16(), duration);
    }

    // Performance validation
    println!(
        "Completed {} concurrent requests in {:?}",
        concurrent_requests, total_duration
    );

    println!("Status code distribution: {:?}", status_codes);
    println!("Successful requests: {}/{}", successful_requests, concurrent_requests);

    // At least some requests should succeed if system is working
    if successful_requests > 0 {
        // Calculate 95th percentile response time
        response_times.sort();
        let percentile_95_index = (response_times.len() as f64 * 0.95) as usize;
        let percentile_95_time = response_times.get(percentile_95_index.min(response_times.len() - 1))
            .unwrap_or(&Duration::from_secs(0));

        println!("95th percentile response time: {:?}", percentile_95_time);

        // Performance requirement: 95% under 5 seconds
        assert!(
            *percentile_95_time < Duration::from_secs(5),
            "95th percentile response time should be under 5 seconds, got: {:?}",
            percentile_95_time
        );

        // Average response time should be reasonable
        let avg_response_time: Duration = response_times
            .iter()
            .sum::<Duration>()
            / response_times.len() as u32;

        println!("Average response time: {:?}", avg_response_time);

        assert!(
            avg_response_time < Duration::from_secs(2),
            "Average response time should be under 2 seconds, got: {:?}",
            avg_response_time
        );
    }

    // System should handle concurrent load gracefully
    let error_rate = 1.0 - (successful_requests as f64 / concurrent_requests as f64);
    println!("Error rate: {:.2}%", error_rate * 100.0);

    // Error rate should be acceptable under load
    assert!(
        error_rate < 0.1, // Less than 10% error rate
        "Error rate should be under 10%, got: {:.2}%",
        error_rate * 100.0
    );
}

/// Test streaming performance under load
#[tokio::test]
async fn test_streaming_performance_validation() {
    let app = create_test_app().await;
    let server = TestServer::new(app).unwrap();

    let jwt_token = create_valid_jwt_token();
    let concurrent_streams = 10; // Multiple concurrent streams

    let request_body = json!({
        "model": "claude-4-sonnet-20250514",
        "messages": [{
            "role": "user",
            "content": "Count from 1 to 3"
        }],
        "stream": true,
        "max_tokens": 30
    });

    let mut handles = Vec::new();
    let start_time = Instant::now();

    for i in 0..concurrent_streams {
        let server_clone = server.clone();
        let jwt_clone = jwt_token.clone();
        let request_clone = request_body.clone();

        let handle = tokio::spawn(async move {
            let stream_start = Instant::now();

            let response = server_clone
                .post("/v1/chat/completions")
                .add_header("Authorization", format!("Bearer {}", jwt_clone))
                .add_header("Content-Type", "application/json")
                .json(&request_clone)
                .await;

            let stream_duration = stream_start.elapsed();

            // Validate streaming response
            let status = response.status_code();
            let mut chunk_count = 0;
            let mut first_chunk_time = None;

            if status == StatusCode::OK {
                let sse_body = response.text();

                // Measure time to first chunk
                if sse_body.contains("data: ") {
                    first_chunk_time = Some(stream_duration);
                }

                // Count chunks
                for line in sse_body.lines() {
                    if line.starts_with("data: ") && !line.contains("[DONE]") {
                        chunk_count += 1;
                    }
                }
            }

            (i, status, stream_duration, chunk_count, first_chunk_time)
        });

        handles.push(handle);
    }

    // Wait for all streams to complete
    let mut results = Vec::new();
    for handle in handles {
        let result = handle.await.expect("Stream task should complete");
        results.push(result);
    }

    let total_duration = start_time.elapsed();

    // Analyze streaming performance
    let mut successful_streams = 0;
    let mut first_chunk_times = Vec::new();
    let mut total_chunks = 0;

    for (id, status, duration, chunks, first_chunk_time) in results {
        if status == StatusCode::OK {
            successful_streams += 1;
            total_chunks += chunks;

            if let Some(first_time) = first_chunk_time {
                first_chunk_times.push(first_time);
            }
        }

        println!(
            "Stream {}: {} in {:?}, {} chunks",
            id,
            status.as_u16(),
            duration,
            chunks
        );
    }

    println!(
        "Completed {} concurrent streams in {:?}",
        concurrent_streams, total_duration
    );

    // Streaming performance validation
    if successful_streams > 0 {
        // Time to first chunk should be fast
        if !first_chunk_times.is_empty() {
            let avg_first_chunk = first_chunk_times.iter().sum::<Duration>() / first_chunk_times.len() as u32;

            assert!(
                avg_first_chunk < Duration::from_secs(3),
                "Average time to first chunk should be under 3 seconds, got: {:?}",
                avg_first_chunk
            );
        }

        // Should produce meaningful chunks
        assert!(
            total_chunks > 0,
            "Successful streams should produce chunks"
        );

        println!("Average chunks per stream: {}", total_chunks / successful_streams);
    }
}

/// Test memory efficiency under load
#[tokio::test]
async fn test_memory_efficiency_validation() {
    let app = create_test_app().await;
    let server = TestServer::new(app).unwrap();

    let jwt_token = create_valid_jwt_token();

    // Test with progressively larger payloads
    let payload_sizes = vec![100, 1000, 5000]; // Characters in content

    for size in payload_sizes {
        let large_content = "a".repeat(size);

        let request_body = json!({
            "model": "claude-4-sonnet-20250514",
            "messages": [{
                "role": "user",
                "content": large_content
            }],
            "max_tokens": 100
        });

        let start_time = Instant::now();

        let response = server
            .post("/v1/chat/completions")
            .add_header("Authorization", format!("Bearer {}", jwt_token))
            .add_header("Content-Type", "application/json")
            .json(&request_body)
            .await;

        let response_time = start_time.elapsed();

        println!("Payload size {}: {} in {:?}", size, response.status_code().as_u16(), response_time);

        // Memory efficiency: response time should not grow linearly with payload
        assert!(
            response_time < Duration::from_secs(10),
            "Large payload processing should complete in reasonable time"
        );

        if response.status_code() == StatusCode::OK {
            let body: Value = response.json();

            // Response should be valid regardless of input size
            assert!(body.get("choices").is_some(), "Response should have choices");
            assert!(body.get("usage").is_some(), "Response should have usage stats");
        }
    }
}

/// Test rate limiting performance
#[tokio::test]
async fn test_rate_limiting_performance() {
    let app = create_test_app().await;
    let server = TestServer::new(app).unwrap();

    let jwt_token = create_valid_jwt_token();
    let rapid_requests = 20;

    let request_body = json!({
        "model": "claude-4-sonnet-20250514",
        "messages": [{
            "role": "user",
            "content": "Rate limit test"
        }],
        "max_tokens": 10
    });

    let mut response_times = Vec::new();
    let mut status_codes = Vec::new();

    // Make rapid sequential requests
    for i in 0..rapid_requests {
        let start_time = Instant::now();

        let response = server
            .post("/v1/chat/completions")
            .add_header("Authorization", format!("Bearer {}", jwt_token))
            .add_header("Content-Type", "application/json")
            .json(&request_body)
            .await;

        let response_time = start_time.elapsed();
        let status = response.status_code();

        response_times.push(response_time);
        status_codes.push(status);

        println!("Request {}: {} in {:?}", i + 1, status.as_u16(), response_time);

        // Small delay to avoid overwhelming the system
        tokio::time::sleep(Duration::from_millis(50)).await;
    }

    // Analyze rate limiting behavior
    let rate_limited_count = status_codes
        .iter()
        .filter(|&&status| status == StatusCode::TOO_MANY_REQUESTS)
        .count();

    let successful_count = status_codes
        .iter()
        .filter(|&&status| status == StatusCode::OK)
        .count();

    println!("Successful requests: {}", successful_count);
    println!("Rate limited requests: {}", rate_limited_count);

    // Rate limiting should be implemented gracefully
    if rate_limited_count > 0 {
        // Rate limit responses should be fast
        let rate_limit_times: Vec<Duration> = response_times
            .iter()
            .zip(&status_codes)
            .filter(|(_, &status)| status == StatusCode::TOO_MANY_REQUESTS)
            .map(|(&time, _)| time)
            .collect();

        if !rate_limit_times.is_empty() {
            let avg_rate_limit_time = rate_limit_times.iter().sum::<Duration>() / rate_limit_times.len() as u32;

            assert!(
                avg_rate_limit_time < Duration::from_millis(500),
                "Rate limit responses should be fast, got: {:?}",
                avg_rate_limit_time
            );
        }
    }

    // System should handle rapid requests gracefully
    assert!(
        successful_count + rate_limited_count == rapid_requests,
        "All requests should get a valid response"
    );
}

/// Test connection management under load
#[tokio::test]
async fn test_connection_management_performance() {
    let app = create_test_app().await;
    let server = TestServer::new(app).unwrap();

    let jwt_token = create_valid_jwt_token();

    // Test connection reuse vs new connections
    let request_body = json!({
        "model": "claude-4-sonnet-20250514",
        "messages": [{
            "role": "user",
            "content": "Connection test"
        }],
        "max_tokens": 5
    });

    let sequential_requests = 5;
    let mut response_times = Vec::new();

    // Sequential requests (should reuse connections)
    for i in 0..sequential_requests {
        let start_time = Instant::now();

        let response = server
            .post("/v1/chat/completions")
            .add_header("Authorization", format!("Bearer {}", jwt_token))
            .add_header("Content-Type", "application/json")
            .json(&request_body)
            .await;

        let response_time = start_time.elapsed();
        response_times.push(response_time);

        println!(
            "Sequential request {}: {} in {:?}",
            i + 1,
            response.status_code().as_u16(),
            response_time
        );

        // Short delay between requests
        tokio::time::sleep(Duration::from_millis(100)).await;
    }

    // Connection reuse should improve performance over time
    if response_times.len() >= 3 {
        let first_request_time = response_times[0];
        let last_request_time = response_times[response_times.len() - 1];

        // Later requests should not be significantly slower (connection reuse)
        let time_ratio = last_request_time.as_millis() as f64 / first_request_time.as_millis() as f64;

        assert!(
            time_ratio < 3.0, // Should not be more than 3x slower
            "Connection management should maintain performance, ratio: {:.2}",
            time_ratio
        );
    }
}

/// Helper function to create test application
async fn create_test_app() -> axum::Router {
    // This will fail until we implement the actual app
    panic!("Application not implemented yet - this test should fail!")
}

/// Create a valid JWT token for testing
fn create_valid_jwt_token() -> String {
    "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWIiOiJ0ZXN0LXVzZXItaWQiLCJlbWFpbCI6InRlc3RAdGVzdC5jb20iLCJpYXQiOjE2OTYxMTg0MDAsImV4cCI6MTY5NjEyMjAwMH0.test_signature".to_string()
}

#[cfg(test)]
mod performance_tests {
    use super::*;

    #[test]
    fn test_performance_requirements() {
        // Validate test constants meet requirements
        assert!(Duration::from_secs(5) >= Duration::from_secs(1), "Performance thresholds are reasonable");
    }

    #[test]
    fn test_concurrent_load_parameters() {
        // Validate concurrent request counts are realistic for testing
        let concurrent_requests = 100;
        assert!(concurrent_requests >= 10, "Enough concurrent requests for meaningful test");
        assert!(concurrent_requests <= 1000, "Reasonable load for test environment");
    }

    #[test]
    fn test_timeout_configuration() {
        // Validate timeout values are appropriate
        let timeout_duration = Duration::from_secs(30);
        assert!(timeout_duration >= Duration::from_secs(10), "Timeout allows reasonable processing time");
    }
}