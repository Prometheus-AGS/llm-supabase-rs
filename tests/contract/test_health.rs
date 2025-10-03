use axum_test::TestServer;
use serde_json::Value;
use axum::http::StatusCode;

/// Contract test for GET /health endpoint
///
/// This test validates the exact API contract specified in openai-api.yaml:
/// - Endpoint: GET /health
/// - No authentication required
/// - Response: 200 OK with specific JSON schema
///
/// This test MUST FAIL until the health endpoint is implemented.
#[tokio::test]
async fn test_health_endpoint_contract() {
    // Create test server - this will fail until app is properly implemented
    let app = create_test_app().await;
    let server = TestServer::new(app).unwrap();

    // Test GET /health endpoint
    let response = server.get("/health").await;

    // Validate response status
    assert_eq!(response.status_code(), StatusCode::OK, "Health endpoint should return 200 OK");

    // Validate response headers
    assert_eq!(
        response.header("content-type"),
        "application/json",
        "Health endpoint should return JSON content type"
    );

    // Parse response body
    let body: Value = response.json();

    // Validate required fields according to OpenAPI spec
    assert!(body.is_object(), "Response should be a JSON object");

    let health_response = body.as_object().unwrap();

    // Required fields: status, version
    assert!(
        health_response.contains_key("status"),
        "Response must contain 'status' field"
    );
    assert!(
        health_response.contains_key("version"),
        "Response must contain 'version' field"
    );

    // Status must be one of: "healthy", "degraded", "unhealthy"
    let status = health_response["status"].as_str().unwrap();
    assert!(
        matches!(status, "healthy" | "degraded" | "unhealthy"),
        "Status must be 'healthy', 'degraded', or 'unhealthy', got: {}",
        status
    );

    // Version should be a string
    assert!(
        health_response["version"].is_string(),
        "Version field must be a string"
    );

    // Optional fields validation
    if let Some(timestamp) = health_response.get("timestamp") {
        assert!(
            timestamp.is_string(),
            "Timestamp field must be a string if present"
        );
        // TODO: Validate ISO 8601 format
    }

    if let Some(providers) = health_response.get("providers") {
        assert!(
            providers.is_object(),
            "Providers field must be an object if present"
        );

        // Validate provider structure
        let providers_obj = providers.as_object().unwrap();
        for (provider_name, provider_info) in providers_obj {
            assert!(
                provider_info.is_object(),
                "Provider '{}' info must be an object",
                provider_name
            );

            let info = provider_info.as_object().unwrap();

            // Provider must have status field
            assert!(
                info.contains_key("status"),
                "Provider '{}' must have status field",
                provider_name
            );

            let provider_status = info["status"].as_str().unwrap();
            assert!(
                matches!(provider_status, "available" | "unavailable" | "degraded"),
                "Provider '{}' status must be 'available', 'unavailable', or 'degraded', got: {}",
                provider_name,
                provider_status
            );

            // Optional latency_ms field
            if let Some(latency) = info.get("latency_ms") {
                assert!(
                    latency.is_number(),
                    "Provider '{}' latency_ms must be a number if present",
                    provider_name
                );
            }
        }
    }
}

/// Test health endpoint when service is unhealthy
/// Should return 503 Service Unavailable according to OpenAPI spec
#[tokio::test]
async fn test_health_endpoint_unhealthy_contract() {
    // This test will be updated once we can simulate unhealthy state
    // For now, we assume healthy service returns 200

    let app = create_test_app().await;
    let server = TestServer::new(app).unwrap();

    let response = server.get("/health").await;

    // For now, expect healthy response
    // TODO: Implement unhealthy state simulation
    assert!(
        matches!(response.status_code(), StatusCode::OK | StatusCode::SERVICE_UNAVAILABLE),
        "Health endpoint should return 200 or 503"
    );

    if response.status_code() == StatusCode::SERVICE_UNAVAILABLE {
        let body: Value = response.json();

        // Should return error response format
        assert!(body.get("error").is_some(), "503 response should contain error field");
    }
}

/// Helper function to create test application
/// This will fail until the main application is implemented
async fn create_test_app() -> axum::Router {
    // This will fail until we implement the actual app
    // For now, return a placeholder that will cause tests to fail
    panic!("Application not implemented yet - this test should fail!")
}

#[cfg(test)]
mod contract_validation_helpers {
    use super::*;

    /// Validate ISO 8601 timestamp format
    pub fn is_valid_iso8601(timestamp: &str) -> bool {
        // Basic ISO 8601 validation
        // Format: YYYY-MM-DDTHH:MM:SSZ or YYYY-MM-DDTHH:MM:SS.sssZ
        chrono::DateTime::parse_from_rfc3339(timestamp).is_ok()
    }

    /// Validate that response matches exact OpenAPI schema
    pub fn validate_health_response_schema(response: &Value) -> Result<(), String> {
        let obj = response.as_object()
            .ok_or("Response must be an object")?;

        // Required fields
        let _status = obj.get("status")
            .and_then(|v| v.as_str())
            .ok_or("Missing required field: status")?;

        let _version = obj.get("version")
            .and_then(|v| v.as_str())
            .ok_or("Missing required field: version")?;

        // Optional fields validation
        if let Some(timestamp) = obj.get("timestamp") {
            let ts_str = timestamp.as_str()
                .ok_or("timestamp field must be string")?;
            if !is_valid_iso8601(ts_str) {
                return Err("timestamp must be valid ISO 8601 format".to_string());
            }
        }

        Ok(())
    }
}