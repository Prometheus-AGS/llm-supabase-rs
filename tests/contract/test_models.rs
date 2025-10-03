use axum_test::TestServer;
use serde_json::Value;
use axum::http::StatusCode;

/// Contract test for GET /v1/models endpoint
///
/// This test validates the exact API contract specified in openai-api.yaml:
/// - Endpoint: GET /v1/models
/// - Requires Bearer Authentication (Supabase JWT)
/// - Response: 200 OK with models list in OpenAI format
/// - Must include Claude 4 Sonnet model
///
/// This test MUST FAIL until the models endpoint is implemented.
#[tokio::test]
async fn test_models_endpoint_contract() {
    let app = create_test_app().await;
    let server = TestServer::new(app).unwrap();

    // Test with valid Bearer token
    let response = server
        .get("/v1/models")
        .add_header("Authorization", "Bearer fake_jwt_token_for_testing")
        .await;

    // Validate response status
    assert_eq!(
        response.status_code(),
        StatusCode::OK,
        "Models endpoint should return 200 OK with valid authentication"
    );

    // Validate response headers
    assert_eq!(
        response.header("content-type"),
        "application/json",
        "Models endpoint should return JSON content type"
    );

    // Parse and validate response body
    let body: Value = response.json();
    validate_models_response(&body).expect("Response should match OpenAI models list format");

    // Verify Claude 4 Sonnet model is included
    let models_data = &body["data"];
    let models_array = models_data.as_array().unwrap();

    let claude_model = models_array
        .iter()
        .find(|model| {
            model["id"].as_str() == Some("claude-4-sonnet-20250514")
        });

    assert!(
        claude_model.is_some(),
        "Models list must include claude-4-sonnet-20250514"
    );

    let claude_model = claude_model.unwrap();
    assert_eq!(
        claude_model["owned_by"].as_str().unwrap(),
        "anthropic-vertex",
        "Claude model should be owned by anthropic-vertex"
    );
}

/// Test models endpoint authentication requirement
#[tokio::test]
async fn test_models_authentication_required() {
    let app = create_test_app().await;
    let server = TestServer::new(app).unwrap();

    // Test without Authorization header
    let response = server.get("/v1/models").await;

    // Should return 401 Unauthorized
    assert_eq!(
        response.status_code(),
        StatusCode::UNAUTHORIZED,
        "Models endpoint should return 401 without authentication"
    );

    // Validate error response format
    assert_eq!(
        response.header("content-type"),
        "application/json",
        "Error response should be JSON"
    );

    let body: Value = response.json();
    validate_error_response(&body).expect("401 response should match OpenAI error format");

    let error = &body["error"];
    assert_eq!(
        error["type"].as_str().unwrap(),
        "authentication_error",
        "Error type should be authentication_error"
    );
}

/// Test models endpoint with invalid token
#[tokio::test]
async fn test_models_invalid_token() {
    let app = create_test_app().await;
    let server = TestServer::new(app).unwrap();

    // Test with malformed token
    let response = server
        .get("/v1/models")
        .add_header("Authorization", "Bearer invalid_malformed_token")
        .await;

    // Should return 401 Unauthorized
    assert_eq!(
        response.status_code(),
        StatusCode::UNAUTHORIZED,
        "Models endpoint should return 401 with invalid token"
    );

    let body: Value = response.json();
    let error = &body["error"];
    assert_eq!(error["type"], "authentication_error");
}

/// Test models endpoint with expired token
#[tokio::test]
async fn test_models_expired_token() {
    let app = create_test_app().await;
    let server = TestServer::new(app).unwrap();

    // Test with properly formatted but expired JWT
    // This is a fake expired JWT for testing
    let expired_jwt = "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWIiOiIxMjM0NTY3ODkwIiwibmFtZSI6IkpvaG4gRG9lIiwiaWF0IjoxNTE2MjM5MDIyLCJleHAiOjE1MTYyMzkwMjJ9.expired";

    let response = server
        .get("/v1/models")
        .add_header("Authorization", format!("Bearer {}", expired_jwt))
        .await;

    // Should return 401 Unauthorized
    assert_eq!(response.status_code(), StatusCode::UNAUTHORIZED);

    let body: Value = response.json();
    let error = &body["error"];
    assert_eq!(error["type"], "authentication_error");
}

/// Test models endpoint response consistency
#[tokio::test]
async fn test_models_response_consistency() {
    let app = create_test_app().await;
    let server = TestServer::new(app).unwrap();

    // Make multiple requests to ensure consistent response
    for i in 0..3 {
        let response = server
            .get("/v1/models")
            .add_header("Authorization", format!("Bearer test_token_{}", i))
            .await;

        if response.status_code() == StatusCode::OK {
            let body: Value = response.json();
            validate_models_response(&body)
                .expect("All successful responses should have consistent format");

            // Verify structure is consistent
            assert_eq!(body["object"], "list");
            assert!(body["data"].is_array());
        }
    }
}

/// Test models endpoint with different HTTP methods
#[tokio::test]
async fn test_models_method_validation() {
    let app = create_test_app().await;
    let server = TestServer::new(app).unwrap();

    // POST to /v1/models should not be allowed
    let response = server
        .post("/v1/models")
        .add_header("Authorization", "Bearer fake_jwt_token")
        .await;

    // Should return 405 Method Not Allowed or 404 Not Found
    assert!(
        matches!(response.status_code(), StatusCode::METHOD_NOT_ALLOWED | StatusCode::NOT_FOUND),
        "POST to /v1/models should not be allowed"
    );

    // PUT to /v1/models should not be allowed
    let response = server
        .put("/v1/models")
        .add_header("Authorization", "Bearer fake_jwt_token")
        .await;

    assert!(
        matches!(response.status_code(), StatusCode::METHOD_NOT_ALLOWED | StatusCode::NOT_FOUND),
        "PUT to /v1/models should not be allowed"
    );
}

/// Helper function to create test application
async fn create_test_app() -> axum::Router {
    // This will fail until we implement the actual app
    panic!("Application not implemented yet - this test should fail!")
}

/// Validate models list response format
fn validate_models_response(response: &Value) -> Result<(), String> {
    let obj = response.as_object()
        .ok_or("Response must be an object")?;

    // Required fields according to OpenAPI spec
    let object_type = obj.get("object")
        .and_then(|v| v.as_str())
        .ok_or("Missing required field: object")?;

    if object_type != "list" {
        return Err(format!("object field must be 'list', got: {}", object_type));
    }

    let data = obj.get("data")
        .and_then(|v| v.as_array())
        .ok_or("Missing required field: data (array)")?;

    if data.is_empty() {
        return Err("data array cannot be empty".to_string());
    }

    // Validate each model in the list
    for (i, model) in data.iter().enumerate() {
        validate_model_object(model)
            .map_err(|e| format!("Model {} validation failed: {}", i, e))?;
    }

    Ok(())
}

/// Validate individual model object
fn validate_model_object(model: &Value) -> Result<(), String> {
    let obj = model.as_object()
        .ok_or("Model must be an object")?;

    // Required fields according to OpenAPI spec
    let _id = obj.get("id")
        .and_then(|v| v.as_str())
        .ok_or("Model missing required field: id")?;

    let object_type = obj.get("object")
        .and_then(|v| v.as_str())
        .ok_or("Model missing required field: object")?;

    if object_type != "model" {
        return Err(format!("Model object field must be 'model', got: {}", object_type));
    }

    let _created = obj.get("created")
        .and_then(|v| v.as_u64())
        .ok_or("Model missing required field: created (unix timestamp)")?;

    let _owned_by = obj.get("owned_by")
        .and_then(|v| v.as_str())
        .ok_or("Model missing required field: owned_by")?;

    // Optional fields validation
    if let Some(permission) = obj.get("permission") {
        if !permission.is_array() {
            return Err("Model permission field must be array if present".to_string());
        }
    }

    if let Some(root) = obj.get("root") {
        if !root.is_string() {
            return Err("Model root field must be string if present".to_string());
        }
    }

    if let Some(parent) = obj.get("parent") {
        if !parent.is_string() && !parent.is_null() {
            return Err("Model parent field must be string or null if present".to_string());
        }
    }

    Ok(())
}

/// Validate error response format (reused from other contract tests)
fn validate_error_response(response: &Value) -> Result<(), String> {
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

    let valid_types = [
        "invalid_request_error",
        "authentication_error",
        "permission_error",
        "rate_limit_error",
        "server_error"
    ];

    if !valid_types.contains(&error_type) {
        return Err(format!("Invalid error type: {}", error_type));
    }

    Ok(())
}

/// Test helper for model ID validation
#[cfg(test)]
mod model_validation_tests {
    use super::*;

    #[test]
    fn test_claude_model_id_format() {
        // Test that our expected model ID follows the right format
        let model_id = "claude-4-sonnet-20250514";

        assert!(model_id.starts_with("claude-"));
        assert!(model_id.contains("sonnet"));
        assert!(model_id.contains("20250514")); // Date format

        // Should be a valid model identifier
        assert!(!model_id.is_empty());
        assert!(!model_id.contains(' '));
        assert!(!model_id.contains('\n'));
    }

    #[test]
    fn test_owned_by_format() {
        let owned_by = "anthropic-vertex";

        // Should indicate both the original company and the provider
        assert!(owned_by.contains("anthropic"));
        assert!(owned_by.contains("vertex"));
    }
}