use axum_test::TestServer;
use serde_json::Value;
use axum::http::StatusCode;
use std::time::{Duration, Instant};

/// Integration test for Health Check Validation
///
/// This test implements Test 1 from quickstart.md:
/// "Health Check (No Authentication Required)"
///
/// Validates:
/// - Service is running and responds to health checks
/// - Provider availability is reported correctly
/// - Response times are within acceptable limits
/// - Health check works without authentication
///
/// This test MUST FAIL until the health endpoint is fully implemented.
#[tokio::test]
async fn test_health_check_integration() {
    let app = create_test_app().await;
    let server = TestServer::new(app).unwrap();

    let start_time = Instant::now();
    let response = server.get("/health").await;
    let response_time = start_time.elapsed();

    // Validate basic response
    assert_eq!(
        response.status_code(),
        StatusCode::OK,
        "Health endpoint should return 200 OK"
    );

    assert_eq!(
        response.header("content-type"),
        "application/json",
        "Health endpoint should return JSON"
    );

    // Parse response body
    let body: Value = response.json();

    // Validate response structure matches quickstart.md expectations
    validate_health_response_structure(&body)
        .expect("Health response should match expected structure");

    // Validate response time (should be fast for health checks)
    assert!(
        response_time < Duration::from_secs(2),
        "Health check should respond quickly, took: {:?}",
        response_time
    );

    // Test specific success criteria from quickstart.md
    let status = body["status"].as_str().unwrap();
    assert_eq!(
        status, "healthy",
        "Health status should be 'healthy' for running service"
    );

    let version = body["version"].as_str().unwrap();
    assert_eq!(
        version, "0.1.0",
        "Version should match Cargo.toml version"
    );

    // Validate provider status
    if let Some(providers) = body.get("providers") {
        let providers_obj = providers.as_object().unwrap();

        // Should include Vertex AI provider
        assert!(
            providers_obj.contains_key("vertex-ai"),
            "Health response should include vertex-ai provider status"
        );

        let vertex_status = &providers_obj["vertex-ai"];
        assert!(
            vertex_status.get("status").is_some(),
            "Vertex AI provider should have status field"
        );

        let status_str = vertex_status["status"].as_str().unwrap();
        assert!(
            matches!(status_str, "available" | "unavailable" | "degraded"),
            "Vertex AI status should be valid: {}",
            status_str
        );

        // If available, should have latency information
        if status_str == "available" {
            if let Some(latency) = vertex_status.get("latency_ms") {
                let latency_val = latency.as_u64().unwrap();
                assert!(
                    latency_val < 5000,
                    "Vertex AI latency should be reasonable: {}ms",
                    latency_val
                );
            }
        }
    }
}

/// Test health check under different conditions
#[tokio::test]
async fn test_health_check_conditions() {
    let app = create_test_app().await;
    let server = TestServer::new(app).unwrap();

    // Test multiple rapid health checks (simulating monitoring)
    for i in 0..5 {
        let response = server.get("/health").await;
        assert_eq!(
            response.status_code(),
            StatusCode::OK,
            "Health check {} should succeed",
            i
        );

        let body: Value = response.json();
        assert!(
            body.get("timestamp").is_some(),
            "Each health check should have a timestamp"
        );
    }
}

/// Test health check response consistency
#[tokio::test]
async fn test_health_check_consistency() {
    let app = create_test_app().await;
    let server = TestServer::new(app).unwrap();

    let mut responses = Vec::new();

    // Collect multiple responses
    for _ in 0..3 {
        let response = server.get("/health").await;
        let body: Value = response.json();
        responses.push(body);

        // Small delay between requests
        tokio::time::sleep(Duration::from_millis(10)).await;
    }

    // Validate consistency
    for response in &responses {
        assert_eq!(response["status"], "healthy");
        assert_eq!(response["version"], "0.1.0");

        // Structure should be consistent
        validate_health_response_structure(response)
            .expect("All health responses should have consistent structure");
    }
}

/// Test health check with various HTTP methods
#[tokio::test]
async fn test_health_check_method_validation() {
    let app = create_test_app().await;
    let server = TestServer::new(app).unwrap();

    // GET should work
    let response = server.get("/health").await;
    assert_eq!(response.status_code(), StatusCode::OK);

    // POST should not be allowed for health
    let response = server.post("/health").await;
    assert!(
        matches!(
            response.status_code(),
            StatusCode::METHOD_NOT_ALLOWED | StatusCode::NOT_FOUND
        ),
        "POST to /health should not be allowed"
    );

    // PUT should not be allowed
    let response = server.put("/health").await;
    assert!(
        matches!(
            response.status_code(),
            StatusCode::METHOD_NOT_ALLOWED | StatusCode::NOT_FOUND
        ),
        "PUT to /health should not be allowed"
    );
}

/// Test health check without authentication (should work)
#[tokio::test]
async fn test_health_check_no_auth_required() {
    let app = create_test_app().await;
    let server = TestServer::new(app).unwrap();

    // Health should work without any authentication headers
    let response = server.get("/health").await;

    assert_eq!(
        response.status_code(),
        StatusCode::OK,
        "Health check should work without authentication"
    );

    let body: Value = response.json();
    assert_eq!(
        body["status"].as_str().unwrap(),
        "healthy",
        "Health status should be healthy without auth"
    );
}

/// Test health check stress scenario
#[tokio::test]
async fn test_health_check_concurrent_requests() {
    let app = create_test_app().await;
    let server = TestServer::new(app).unwrap();

    // Spawn multiple concurrent health check requests
    let mut handles = Vec::new();

    for i in 0..10 {
        let server_clone = server.clone();
        let handle = tokio::spawn(async move {
            let response = server_clone.get("/health").await;
            (i, response.status_code())
        });
        handles.push(handle);
    }

    // Wait for all requests to complete
    let mut results = Vec::new();
    for handle in handles {
        let result = handle.await.unwrap();
        results.push(result);
    }

    // All should succeed
    for (i, status) in results {
        assert_eq!(
            status,
            StatusCode::OK,
            "Concurrent health check {} should succeed",
            i
        );
    }
}

/// Helper function to create test application
async fn create_test_app() -> axum::Router {
    // This will fail until we implement the actual app
    panic!("Application not implemented yet - this test should fail!")
}

/// Validate health response structure matches expectations
fn validate_health_response_structure(response: &Value) -> Result<(), String> {
    let obj = response.as_object()
        .ok_or("Health response must be an object")?;

    // Required fields from quickstart.md
    let _status = obj.get("status")
        .and_then(|v| v.as_str())
        .ok_or("Missing required field: status")?;

    let _version = obj.get("version")
        .and_then(|v| v.as_str())
        .ok_or("Missing required field: version")?;

    // Optional timestamp field
    if let Some(timestamp) = obj.get("timestamp") {
        if !timestamp.is_string() {
            return Err("timestamp field must be string if present".to_string());
        }

        // Validate ISO 8601 format if present
        let ts_str = timestamp.as_str().unwrap();
        if !is_valid_iso8601(ts_str) {
            return Err("timestamp must be valid ISO 8601 format".to_string());
        }
    }

    // Optional providers field
    if let Some(providers) = obj.get("providers") {
        let providers_obj = providers.as_object()
            .ok_or("providers field must be object if present")?;

        for (provider_name, provider_info) in providers_obj {
            let info = provider_info.as_object()
                .ok_or_else(|| format!("Provider '{}' info must be object", provider_name))?;

            // Provider must have status
            let _status = info.get("status")
                .and_then(|v| v.as_str())
                .ok_or_else(|| format!("Provider '{}' missing status field", provider_name))?;

            // Optional latency field
            if let Some(latency) = info.get("latency_ms") {
                if !latency.is_number() {
                    return Err(format!("Provider '{}' latency_ms must be number", provider_name));
                }
            }
        }
    }

    Ok(())
}

/// Validate ISO 8601 timestamp format
fn is_valid_iso8601(timestamp: &str) -> bool {
    chrono::DateTime::parse_from_rfc3339(timestamp).is_ok()
}

#[cfg(test)]
mod health_validation_tests {
    use super::*;

    #[test]
    fn test_iso8601_validation() {
        assert!(is_valid_iso8601("2025-10-02T10:00:00Z"));
        assert!(is_valid_iso8601("2025-10-02T10:00:00.123Z"));
        assert!(is_valid_iso8601("2025-10-02T10:00:00+00:00"));

        assert!(!is_valid_iso8601("invalid-date"));
        assert!(!is_valid_iso8601("2025-10-02"));
        assert!(!is_valid_iso8601(""));
    }

    #[test]
    fn test_health_structure_validation() {
        let valid_response = serde_json::json!({
            "status": "healthy",
            "version": "0.1.0",
            "timestamp": "2025-10-02T10:00:00Z",
            "providers": {
                "vertex-ai": {
                    "status": "available",
                    "latency_ms": 150
                }
            }
        });

        assert!(validate_health_response_structure(&valid_response).is_ok());

        let invalid_response = serde_json::json!({
            "status": "healthy"
            // Missing version
        });

        assert!(validate_health_response_structure(&invalid_response).is_err());
    }
}