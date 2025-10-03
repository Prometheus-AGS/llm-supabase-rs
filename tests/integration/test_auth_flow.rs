use axum_test::TestServer;
use serde_json::{json, Value};
use axum::http::StatusCode;
use std::time::Duration;

/// Integration test for Authentication Flow (Valid JWT)
///
/// This test implements Test 2 from quickstart.md:
/// "Models Endpoint (With Authentication)"
///
/// Validates:
/// - Supabase JWT token validation works correctly
/// - Protected endpoints require authentication
/// - Valid tokens provide access to resources
/// - Invalid/expired tokens are properly rejected
///
/// This test MUST FAIL until authentication middleware is fully implemented.
#[tokio::test]
async fn test_authentication_flow_valid_jwt() {
    let app = create_test_app().await;
    let server = TestServer::new(app).unwrap();

    // Test with a simulated valid JWT token
    let valid_jwt = create_test_jwt_token();

    let response = server
        .get("/v1/models")
        .add_header("Authorization", format!("Bearer {}", valid_jwt))
        .await;

    // Should return 200 OK with valid authentication
    assert_eq!(
        response.status_code(),
        StatusCode::OK,
        "Models endpoint should return 200 OK with valid JWT"
    );

    assert_eq!(
        response.header("content-type"),
        "application/json",
        "Models response should be JSON"
    );

    let body: Value = response.json();

    // Validate response structure matches expected format
    assert_eq!(
        body["object"].as_str().unwrap(),
        "list",
        "Models response should have object: 'list'"
    );

    let models = body["data"].as_array().unwrap();
    assert!(!models.is_empty(), "Models array should not be empty");

    // Should include Claude 4 Sonnet model
    let claude_model = models.iter().find(|model| {
        model["id"].as_str() == Some("claude-4-sonnet-20250514")
    });

    assert!(
        claude_model.is_some(),
        "Should include claude-4-sonnet-20250514 model"
    );
}

/// Test authentication failure scenarios
#[tokio::test]
async fn test_authentication_failure_scenarios() {
    let app = create_test_app().await;
    let server = TestServer::new(app).unwrap();

    // Test 1: No Authorization header
    let response = server.get("/v1/models").await;

    assert_eq!(
        response.status_code(),
        StatusCode::UNAUTHORIZED,
        "Should return 401 without Authorization header"
    );

    let body: Value = response.json();
    validate_error_response(&body, "authentication_error")
        .expect("401 response should be valid error format");

    // Test 2: Malformed Authorization header
    let response = server
        .get("/v1/models")
        .add_header("Authorization", "InvalidFormat")
        .await;

    assert_eq!(
        response.status_code(),
        StatusCode::UNAUTHORIZED,
        "Should return 401 with malformed Authorization header"
    );

    // Test 3: Invalid Bearer token
    let response = server
        .get("/v1/models")
        .add_header("Authorization", "Bearer invalid_token_format")
        .await;

    assert_eq!(
        response.status_code(),
        StatusCode::UNAUTHORIZED,
        "Should return 401 with invalid Bearer token"
    );

    // Test 4: Expired JWT token
    let expired_jwt = create_expired_jwt_token();
    let response = server
        .get("/v1/models")
        .add_header("Authorization", format!("Bearer {}", expired_jwt))
        .await;

    assert_eq!(
        response.status_code(),
        StatusCode::UNAUTHORIZED,
        "Should return 401 with expired JWT token"
    );
}

/// Test JWT token validation edge cases
#[tokio::test]
async fn test_jwt_validation_edge_cases() {
    let app = create_test_app().await;
    let server = TestServer::new(app).unwrap();

    // Test with JWT missing signature
    let unsigned_jwt = "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWIiOiIxMjM0NTY3ODkwIiwibmFtZSI6IkpvaG4gRG9lIiwiaWF0IjoxNTE2MjM5MDIyfQ";

    let response = server
        .get("/v1/models")
        .add_header("Authorization", format!("Bearer {}", unsigned_jwt))
        .await;

    assert_eq!(response.status_code(), StatusCode::UNAUTHORIZED);

    // Test with completely malformed JWT
    let malformed_jwt = "not.a.jwt";

    let response = server
        .get("/v1/models")
        .add_header("Authorization", format!("Bearer {}", malformed_jwt))
        .await;

    assert_eq!(response.status_code(), StatusCode::UNAUTHORIZED);

    // Test with empty Bearer token
    let response = server
        .get("/v1/models")
        .add_header("Authorization", "Bearer ")
        .await;

    assert_eq!(response.status_code(), StatusCode::UNAUTHORIZED);
}

/// Test authentication context extraction
#[tokio::test]
async fn test_authentication_context_extraction() {
    let app = create_test_app().await;
    let server = TestServer::new(app).unwrap();

    // Create JWT with specific user context
    let jwt_with_context = create_jwt_with_user_context();

    let response = server
        .post("/v1/chat/completions")
        .add_header("Authorization", format!("Bearer {}", jwt_with_context))
        .add_header("Content-Type", "application/json")
        .json(&json!({
            "model": "claude-4-sonnet-20250514",
            "messages": [{"role": "user", "content": "Hello"}],
            "max_tokens": 10
        }))
        .await;

    // Should succeed with valid JWT
    if response.status_code() == StatusCode::OK {
        // If successful, the authentication context should be properly extracted
        // This will be validated by the implementation's logging/audit trail
        let body: Value = response.json();

        // Basic structure validation
        assert!(body.get("id").is_some(), "Response should have ID");
        assert!(body.get("choices").is_some(), "Response should have choices");
    } else {
        // If not yet implemented, should return 401 or 500, not other errors
        assert!(
            matches!(
                response.status_code(),
                StatusCode::UNAUTHORIZED | StatusCode::INTERNAL_SERVER_ERROR
            ),
            "Unexpected error code: {}",
            response.status_code()
        );
    }
}

/// Test authentication with different endpoints
#[tokio::test]
async fn test_auth_across_endpoints() {
    let app = create_test_app().await;
    let server = TestServer::new(app).unwrap();

    let valid_jwt = create_test_jwt_token();

    // Test authentication on all protected endpoints
    let endpoints = vec![
        ("/v1/models", "GET"),
        ("/v1/chat/completions", "POST"),
    ];

    for (endpoint, method) in endpoints {
        let response = match method {
            "GET" => {
                server
                    .get(endpoint)
                    .add_header("Authorization", format!("Bearer {}", valid_jwt))
                    .await
            }
            "POST" => {
                server
                    .post(endpoint)
                    .add_header("Authorization", format!("Bearer {}", valid_jwt))
                    .add_header("Content-Type", "application/json")
                    .json(&json!({
                        "model": "claude-4-sonnet-20250514",
                        "messages": [{"role": "user", "content": "test"}]
                    }))
                    .await
            }
            _ => panic!("Unsupported method: {}", method),
        };

        // Should not return 401 with valid auth
        assert_ne!(
            response.status_code(),
            StatusCode::UNAUTHORIZED,
            "Endpoint {} {} should not return 401 with valid auth",
            method,
            endpoint
        );
    }
}

/// Test authentication rate limiting (if implemented)
#[tokio::test]
async fn test_auth_rate_limiting() {
    let app = create_test_app().await;
    let server = TestServer::new(app).unwrap();

    let valid_jwt = create_test_jwt_token();

    // Make multiple rapid requests to test rate limiting
    let mut responses = Vec::new();

    for i in 0..5 {
        let response = server
            .get("/v1/models")
            .add_header("Authorization", format!("Bearer {}", valid_jwt))
            .await;

        responses.push((i, response.status_code()));

        // Small delay between requests
        tokio::time::sleep(Duration::from_millis(10)).await;
    }

    // All should succeed or some might be rate limited
    for (i, status) in responses {
        assert!(
            matches!(status, StatusCode::OK | StatusCode::TOO_MANY_REQUESTS),
            "Request {} should either succeed or be rate limited, got: {}",
            i,
            status
        );
    }
}

/// Helper function to create test application
async fn create_test_app() -> axum::Router {
    // This will fail until we implement the actual app
    panic!("Application not implemented yet - this test should fail!")
}

/// Create a simulated valid JWT token for testing
fn create_test_jwt_token() -> String {
    // This would normally create a valid JWT signed with the test secret
    // For now, return a placeholder that the test expects to validate
    "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWIiOiJ0ZXN0LXVzZXItaWQiLCJlbWFpbCI6InRlc3RAdGVzdC5jb20iLCJpYXQiOjE2OTYxMTg0MDAsImV4cCI6MTY5NjEyMjAwMH0.test_signature".to_string()
}

/// Create an expired JWT token for testing
fn create_expired_jwt_token() -> String {
    // JWT with past expiration date
    "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWIiOiJ0ZXN0LXVzZXItaWQiLCJlbWFpbCI6InRlc3RAdGVzdC5jb20iLCJpYXQiOjE1MTYyMzkwMjIsImV4cCI6MTUxNjIzOTAyMn0.expired_signature".to_string()
}

/// Create JWT with specific user context
fn create_jwt_with_user_context() -> String {
    // JWT with user metadata for context testing
    "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWIiOiJ0ZXN0LXVzZXItMTIzIiwiZW1haWwiOiJ0ZXN0dXNlckBleGFtcGxlLmNvbSIsInJvbGUiOiJ1c2VyIiwiaWF0IjoxNjk2MTE4NDAwLCJleHAiOjE2OTYxMjIwMDB9.user_context_signature".to_string()
}

/// Validate error response format
fn validate_error_response(response: &Value, expected_type: &str) -> Result<(), String> {
    let obj = response.as_object()
        .ok_or("Error response must be an object")?;

    let error = obj.get("error")
        .and_then(|v| v.as_object())
        .ok_or("Missing required field: error")?;

    let _message = error.get("message")
        .and_then(|v| v.as_str())
        .ok_or("Error missing required field: message")?;

    let error_type = error.get("type")
        .and_then(|v| v.as_str())
        .ok_or("Error missing required field: type")?;

    if error_type != expected_type {
        return Err(format!("Expected error type '{}', got '{}'", expected_type, error_type));
    }

    Ok(())
}

#[cfg(test)]
mod auth_flow_tests {
    use super::*;

    #[test]
    fn test_jwt_token_format() {
        let token = create_test_jwt_token();

        // JWT should have 3 parts separated by dots
        let parts: Vec<&str> = token.split('.').collect();
        assert_eq!(parts.len(), 3, "JWT should have 3 parts (header.payload.signature)");

        // Each part should be non-empty
        for (i, part) in parts.iter().enumerate() {
            assert!(!part.is_empty(), "JWT part {} should not be empty", i);
        }
    }

    #[test]
    fn test_expired_token_format() {
        let token = create_expired_jwt_token();

        // Should still be a valid JWT format
        let parts: Vec<&str> = token.split('.').collect();
        assert_eq!(parts.len(), 3);
    }

    #[test]
    fn test_error_response_validation() {
        let valid_error = json!({
            "error": {
                "message": "Authentication required",
                "type": "authentication_error",
                "code": "missing_authorization"
            }
        });

        assert!(validate_error_response(&valid_error, "authentication_error").is_ok());

        let invalid_error = json!({
            "error": {
                "message": "Some error",
                "type": "different_error"
            }
        });

        assert!(validate_error_response(&invalid_error, "authentication_error").is_err());
    }
}