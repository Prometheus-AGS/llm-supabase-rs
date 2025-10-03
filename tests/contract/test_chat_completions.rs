use axum_test::TestServer;
use serde_json::{json, Value};
use axum::http::StatusCode;

/// Contract test for POST /v1/chat/completions (non-streaming)
///
/// This test validates the exact API contract specified in openai-api.yaml:
/// - Endpoint: POST /v1/chat/completions
/// - Requires Bearer Authentication (Supabase JWT)
/// - Request/Response format must match OpenAI API exactly
///
/// This test MUST FAIL until the chat completions endpoint is implemented.
#[tokio::test]
async fn test_chat_completions_non_streaming_contract() {
    let app = create_test_app().await;
    let server = TestServer::new(app).unwrap();

    // Valid chat completion request matching OpenAPI spec
    let request_body = json!({
        "model": "claude-4-sonnet-20250514",
        "messages": [
            {
                "role": "user",
                "content": "Hello! Can you help me test this API?"
            }
        ],
        "max_tokens": 100,
        "temperature": 0.7,
        "stream": false
    });

    // Test with valid Bearer token (will fail until auth is implemented)
    let response = server
        .post("/v1/chat/completions")
        .add_header("Authorization", "Bearer fake_jwt_token_for_testing")
        .add_header("Content-Type", "application/json")
        .json(&request_body)
        .await;

    // Validate response status (should be 200 OK when properly implemented)
    assert_eq!(
        response.status_code(),
        StatusCode::OK,
        "Chat completions should return 200 OK with valid request"
    );

    // Validate response headers
    assert_eq!(
        response.header("content-type"),
        "application/json",
        "Chat completions should return JSON content type"
    );

    // Parse and validate response body structure
    let body: Value = response.json();
    validate_chat_completion_response(&body).expect("Response should match OpenAI chat completion format");
}

/// Test authentication requirement
#[tokio::test]
async fn test_chat_completions_authentication_required() {
    let app = create_test_app().await;
    let server = TestServer::new(app).unwrap();

    let request_body = json!({
        "model": "claude-4-sonnet-20250514",
        "messages": [{"role": "user", "content": "Hello"}]
    });

    // Test without Authorization header
    let response = server
        .post("/v1/chat/completions")
        .add_header("Content-Type", "application/json")
        .json(&request_body)
        .await;

    // Should return 401 Unauthorized
    assert_eq!(
        response.status_code(),
        StatusCode::UNAUTHORIZED,
        "Chat completions should return 401 without authentication"
    );

    let body: Value = response.json();
    validate_error_response(&body).expect("401 response should match OpenAI error format");
}

/// Test invalid request validation
#[tokio::test]
async fn test_chat_completions_invalid_request() {
    let app = create_test_app().await;
    let server = TestServer::new(app).unwrap();

    // Empty messages array (invalid according to OpenAPI spec)
    let invalid_request = json!({
        "model": "claude-4-sonnet-20250514",
        "messages": [],
        "temperature": 3.0  // Also invalid - max is 2.0
    });

    let response = server
        .post("/v1/chat/completions")
        .add_header("Authorization", "Bearer fake_jwt_token")
        .add_header("Content-Type", "application/json")
        .json(&invalid_request)
        .await;

    // Should return 400 Bad Request
    assert_eq!(
        response.status_code(),
        StatusCode::BAD_REQUEST,
        "Invalid request should return 400 Bad Request"
    );

    let body: Value = response.json();
    validate_error_response(&body).expect("400 response should match OpenAI error format");

    // Validate specific error details
    let error_obj = body["error"].as_object().unwrap();
    assert_eq!(error_obj["type"].as_str().unwrap(), "invalid_request_error");
    assert!(error_obj.contains_key("param"), "Error should specify which parameter is invalid");
}

/// Test model validation
#[tokio::test]
async fn test_chat_completions_invalid_model() {
    let app = create_test_app().await;
    let server = TestServer::new(app).unwrap();

    let request_body = json!({
        "model": "unsupported-model-name",
        "messages": [{"role": "user", "content": "Hello"}]
    });

    let response = server
        .post("/v1/chat/completions")
        .add_header("Authorization", "Bearer fake_jwt_token")
        .add_header("Content-Type", "application/json")
        .json(&request_body)
        .await;

    // Should return 400 Bad Request for unsupported model
    assert_eq!(
        response.status_code(),
        StatusCode::BAD_REQUEST,
        "Unsupported model should return 400 Bad Request"
    );

    let body: Value = response.json();
    let error = &body["error"];
    assert_eq!(error["type"], "invalid_request_error");
    assert_eq!(error["param"], "model");
}

/// Test parameter validation boundaries
#[tokio::test]
async fn test_chat_completions_parameter_boundaries() {
    let app = create_test_app().await;
    let server = TestServer::new(app).unwrap();

    // Test max_tokens boundary (max 200,000 according to OpenAPI spec)
    let request_body = json!({
        "model": "claude-4-sonnet-20250514",
        "messages": [{"role": "user", "content": "Hello"}],
        "max_tokens": 200001  // One over the limit
    });

    let response = server
        .post("/v1/chat/completions")
        .add_header("Authorization", "Bearer fake_jwt_token")
        .add_header("Content-Type", "application/json")
        .json(&request_body)
        .await;

    assert_eq!(response.status_code(), StatusCode::BAD_REQUEST);

    // Test temperature boundary (max 2.0)
    let request_body = json!({
        "model": "claude-4-sonnet-20250514",
        "messages": [{"role": "user", "content": "Hello"}],
        "temperature": 2.1  // Over the limit
    });

    let response = server
        .post("/v1/chat/completions")
        .add_header("Authorization", "Bearer fake_jwt_token")
        .add_header("Content-Type", "application/json")
        .json(&request_body)
        .await;

    assert_eq!(response.status_code(), StatusCode::BAD_REQUEST);
}

/// Helper function to create test application
async fn create_test_app() -> axum::Router {
    // This will fail until we implement the actual app
    panic!("Application not implemented yet - this test should fail!")
}

/// Validate chat completion response format
fn validate_chat_completion_response(response: &Value) -> Result<(), String> {
    let obj = response.as_object()
        .ok_or("Response must be an object")?;

    // Required fields according to OpenAPI spec
    let _id = obj.get("id")
        .and_then(|v| v.as_str())
        .ok_or("Missing required field: id")?;

    let object_type = obj.get("object")
        .and_then(|v| v.as_str())
        .ok_or("Missing required field: object")?;

    if object_type != "chat.completion" {
        return Err(format!("object field must be 'chat.completion', got: {}", object_type));
    }

    let _created = obj.get("created")
        .and_then(|v| v.as_u64())
        .ok_or("Missing required field: created (unix timestamp)")?;

    let _model = obj.get("model")
        .and_then(|v| v.as_str())
        .ok_or("Missing required field: model")?;

    let choices = obj.get("choices")
        .and_then(|v| v.as_array())
        .ok_or("Missing required field: choices (array)")?;

    if choices.is_empty() {
        return Err("choices array cannot be empty".to_string());
    }

    // Validate first choice structure
    let choice = choices[0].as_object()
        .ok_or("Choice must be an object")?;

    let _index = choice.get("index")
        .and_then(|v| v.as_u64())
        .ok_or("Choice missing required field: index")?;

    let message = choice.get("message")
        .and_then(|v| v.as_object())
        .ok_or("Choice missing required field: message")?;

    let role = message.get("role")
        .and_then(|v| v.as_str())
        .ok_or("Message missing required field: role")?;

    if role != "assistant" {
        return Err(format!("Message role must be 'assistant', got: {}", role));
    }

    let _content = message.get("content")
        .and_then(|v| v.as_str())
        .ok_or("Message missing required field: content")?;

    let finish_reason = choice.get("finish_reason")
        .and_then(|v| v.as_str());

    if let Some(reason) = finish_reason {
        if !matches!(reason, "stop" | "length" | "content_filter") {
            return Err(format!("Invalid finish_reason: {}", reason));
        }
    }

    // Usage field is required for non-streaming responses
    let usage = obj.get("usage")
        .and_then(|v| v.as_object())
        .ok_or("Missing required field: usage")?;

    let _prompt_tokens = usage.get("prompt_tokens")
        .and_then(|v| v.as_u64())
        .ok_or("Usage missing required field: prompt_tokens")?;

    let _completion_tokens = usage.get("completion_tokens")
        .and_then(|v| v.as_u64())
        .ok_or("Usage missing required field: completion_tokens")?;

    let _total_tokens = usage.get("total_tokens")
        .and_then(|v| v.as_u64())
        .ok_or("Usage missing required field: total_tokens")?;

    Ok(())
}

/// Validate error response format
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

    // Optional fields: code, param
    if let Some(code) = error.get("code") {
        if !code.is_string() {
            return Err("error.code must be a string if present".to_string());
        }
    }

    if let Some(param) = error.get("param") {
        if !param.is_string() && !param.is_null() {
            return Err("error.param must be a string or null if present".to_string());
        }
    }

    Ok(())
}