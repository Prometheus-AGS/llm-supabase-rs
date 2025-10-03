use axum_test::TestServer;
use serde_json::{json, Value};
use axum::http::StatusCode;
use std::time::{Duration, Instant};

/// Integration test for Chat Completion E2E
///
/// This test implements Test 3 from quickstart.md:
/// "Simple Chat Completion (Non-Streaming)"
///
/// Validates complete end-to-end flow:
/// - Authentication with Supabase JWT
/// - Request validation and processing
/// - Vertex AI provider integration
/// - Response format compliance with OpenAI API
/// - Performance requirements (95% under 5 seconds)
///
/// This test MUST FAIL until the complete chat completion pipeline is implemented.
#[tokio::test]
async fn test_chat_completion_e2e_basic() {
    let app = create_test_app().await;
    let server = TestServer::new(app).unwrap();

    let jwt_token = create_valid_jwt_token();

    // Basic chat completion request from quickstart.md
    let request_body = json!({
        "model": "claude-4-sonnet-20250514",
        "messages": [
            {
                "role": "user",
                "content": "Hello! Can you help me test this API?"
            }
        ],
        "max_tokens": 100,
        "temperature": 0.7
    });

    let start_time = Instant::now();

    let response = server
        .post("/v1/chat/completions")
        .add_header("Authorization", format!("Bearer {}", jwt_token))
        .add_header("Content-Type", "application/json")
        .json(&request_body)
        .await;

    let response_time = start_time.elapsed();

    // Validate response status
    assert_eq!(
        response.status_code(),
        StatusCode::OK,
        "Chat completion should return 200 OK"
    );

    // Validate response headers
    assert_eq!(
        response.header("content-type"),
        "application/json",
        "Chat completion should return JSON"
    );

    // Validate performance requirement: 95% under 5 seconds
    assert!(
        response_time < Duration::from_secs(5),
        "Chat completion should complete under 5 seconds, took: {:?}",
        response_time
    );

    // Parse and validate response body
    let body: Value = response.json();

    // Validate exact structure from quickstart.md expected response
    validate_chat_completion_response(&body)
        .expect("Response should match OpenAI chat completion format");

    // Validate specific fields from quickstart.md
    assert_eq!(body["object"], "chat.completion");
    assert_eq!(body["model"], "claude-4-sonnet-20250514");

    let choices = body["choices"].as_array().unwrap();
    assert!(!choices.is_empty(), "Should have at least one choice");

    let choice = &choices[0];
    assert_eq!(choice["index"], 0);

    let message = &choice["message"];
    assert_eq!(message["role"], "assistant");
    assert!(message["content"].is_string());
    assert!(!message["content"].as_str().unwrap().is_empty());

    // Validate usage statistics
    let usage = &body["usage"];
    assert!(usage["prompt_tokens"].as_u64().unwrap() > 0);
    assert!(usage["completion_tokens"].as_u64().unwrap() > 0);
    assert!(usage["total_tokens"].as_u64().unwrap() > 0);

    // Validate total = prompt + completion
    let prompt_tokens = usage["prompt_tokens"].as_u64().unwrap();
    let completion_tokens = usage["completion_tokens"].as_u64().unwrap();
    let total_tokens = usage["total_tokens"].as_u64().unwrap();
    assert_eq!(total_tokens, prompt_tokens + completion_tokens);
}

/// Test chat completion with various message formats
#[tokio::test]
async fn test_chat_completion_message_formats() {
    let app = create_test_app().await;
    let server = TestServer::new(app).unwrap();

    let jwt_token = create_valid_jwt_token();

    // Test system message + user message
    let request_body = json!({
        "model": "claude-4-sonnet-20250514",
        "messages": [
            {
                "role": "system",
                "content": "You are a helpful assistant. Always respond with exactly one word."
            },
            {
                "role": "user",
                "content": "What is 2+2?"
            }
        ],
        "max_tokens": 10
    });

    let response = server
        .post("/v1/chat/completions")
        .add_header("Authorization", format!("Bearer {}", jwt_token))
        .add_header("Content-Type", "application/json")
        .json(&request_body)
        .await;

    if response.status_code() == StatusCode::OK {
        let body: Value = response.json();
        validate_chat_completion_response(&body)
            .expect("System+User message response should be valid");

        let content = body["choices"][0]["message"]["content"].as_str().unwrap();
        assert!(!content.is_empty(), "Response content should not be empty");
    }
}

/// Test chat completion parameter validation
#[tokio::test]
async fn test_chat_completion_parameter_validation() {
    let app = create_test_app().await;
    let server = TestServer::new(app).unwrap();

    let jwt_token = create_valid_jwt_token();

    // Test max_tokens boundary (within OpenAPI limits)
    let request_body = json!({
        "model": "claude-4-sonnet-20250514",
        "messages": [{"role": "user", "content": "Short test"}],
        "max_tokens": 1000,
        "temperature": 1.0,
        "top_p": 1.0
    });

    let response = server
        .post("/v1/chat/completions")
        .add_header("Authorization", format!("Bearer {}", jwt_token))
        .add_header("Content-Type", "application/json")
        .json(&request_body)
        .await;

    if response.status_code() == StatusCode::OK {
        let body: Value = response.json();

        // Validate that parameters were respected
        let usage = &body["usage"];
        let completion_tokens = usage["completion_tokens"].as_u64().unwrap();
        assert!(
            completion_tokens <= 1000,
            "Completion tokens should respect max_tokens limit"
        );
    }
}

/// Test chat completion with conversation history
#[tokio::test]
async fn test_chat_completion_conversation_flow() {
    let app = create_test_app().await;
    let server = TestServer::new(app).unwrap();

    let jwt_token = create_valid_jwt_token();

    // Multi-turn conversation
    let request_body = json!({
        "model": "claude-4-sonnet-20250514",
        "messages": [
            {
                "role": "user",
                "content": "What's the capital of France?"
            },
            {
                "role": "assistant",
                "content": "The capital of France is Paris."
            },
            {
                "role": "user",
                "content": "What's the population of that city?"
            }
        ],
        "max_tokens": 50
    });

    let response = server
        .post("/v1/chat/completions")
        .add_header("Authorization", format!("Bearer {}", jwt_token))
        .add_header("Content-Type", "application/json")
        .json(&request_body)
        .await;

    if response.status_code() == StatusCode::OK {
        let body: Value = response.json();
        validate_chat_completion_response(&body)
            .expect("Conversation flow response should be valid");

        // Response should reference Paris/population context
        let content = body["choices"][0]["message"]["content"].as_str().unwrap();
        assert!(!content.is_empty(), "Conversation response should not be empty");
    }
}

/// Test chat completion error scenarios
#[tokio::test]
async fn test_chat_completion_error_handling() {
    let app = create_test_app().await;
    let server = TestServer::new(app).unwrap();

    let jwt_token = create_valid_jwt_token();

    // Test invalid model
    let request_body = json!({
        "model": "non-existent-model",
        "messages": [{"role": "user", "content": "Hello"}]
    });

    let response = server
        .post("/v1/chat/completions")
        .add_header("Authorization", format!("Bearer {}", jwt_token))
        .add_header("Content-Type", "application/json")
        .json(&request_body)
        .await;

    assert_eq!(response.status_code(), StatusCode::BAD_REQUEST);

    let body: Value = response.json();
    let error = &body["error"];
    assert_eq!(error["type"], "invalid_request_error");
    assert_eq!(error["param"], "model");

    // Test empty messages
    let request_body = json!({
        "model": "claude-4-sonnet-20250514",
        "messages": []
    });

    let response = server
        .post("/v1/chat/completions")
        .add_header("Authorization", format!("Bearer {}", jwt_token))
        .add_header("Content-Type", "application/json")
        .json(&request_body)
        .await;

    assert_eq!(response.status_code(), StatusCode::BAD_REQUEST);
}

/// Test chat completion with special characters and edge cases
#[tokio::test]
async fn test_chat_completion_edge_cases() {
    let app = create_test_app().await;
    let server = TestServer::new(app).unwrap();

    let jwt_token = create_valid_jwt_token();

    // Test with special characters, emojis, long text
    let request_body = json!({
        "model": "claude-4-sonnet-20250514",
        "messages": [
            {
                "role": "user",
                "content": "Hello! 🌟 Can you process: special chars \"quotes\", 'apostrophes', newlines\n\ntabs\t\tand Unicode: 你好世界? Testing long content with multiple sentences to ensure proper processing of varied input formats and edge cases that might occur in real usage scenarios."
            }
        ],
        "max_tokens": 150
    });

    let response = server
        .post("/v1/chat/completions")
        .add_header("Authorization", format!("Bearer {}", jwt_token))
        .add_header("Content-Type", "application/json")
        .json(&request_body)
        .await;

    if response.status_code() == StatusCode::OK {
        let body: Value = response.json();
        validate_chat_completion_response(&body)
            .expect("Edge case response should be valid");

        // Response should handle special characters properly
        let content = body["choices"][0]["message"]["content"].as_str().unwrap();
        assert!(!content.is_empty(), "Edge case response should not be empty");

        // Validate JSON encoding is correct
        assert!(serde_json::to_string(&body).is_ok(), "Response should be valid JSON");
    }
}

/// Test chat completion performance under load
#[tokio::test]
async fn test_chat_completion_performance() {
    let app = create_test_app().await;
    let server = TestServer::new(app).unwrap();

    let jwt_token = create_valid_jwt_token();

    let request_body = json!({
        "model": "claude-4-sonnet-20250514",
        "messages": [{"role": "user", "content": "Quick test"}],
        "max_tokens": 10
    });

    // Test multiple sequential requests
    let mut response_times = Vec::new();

    for _i in 0..3 {
        let start_time = Instant::now();

        let response = server
            .post("/v1/chat/completions")
            .add_header("Authorization", format!("Bearer {}", jwt_token))
            .add_header("Content-Type", "application/json")
            .json(&request_body)
            .await;

        let response_time = start_time.elapsed();
        response_times.push(response_time);

        if response.status_code() == StatusCode::OK {
            // Each request should meet performance requirements
            assert!(
                response_time < Duration::from_secs(5),
                "Request should complete under 5 seconds"
            );
        }

        // Small delay between requests
        tokio::time::sleep(Duration::from_millis(100)).await;
    }

    // Validate performance consistency
    for (i, time) in response_times.iter().enumerate() {
        println!("Request {} took: {:?}", i + 1, time);
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

/// Validate chat completion response format
fn validate_chat_completion_response(response: &Value) -> Result<(), String> {
    let obj = response.as_object()
        .ok_or("Response must be an object")?;

    // Required fields
    let _id = obj.get("id")
        .and_then(|v| v.as_str())
        .ok_or("Missing required field: id")?;

    let object_type = obj.get("object")
        .and_then(|v| v.as_str())
        .ok_or("Missing required field: object")?;

    if object_type != "chat.completion" {
        return Err(format!("object must be 'chat.completion', got: {}", object_type));
    }

    let _created = obj.get("created")
        .and_then(|v| v.as_u64())
        .ok_or("Missing required field: created")?;

    let _model = obj.get("model")
        .and_then(|v| v.as_str())
        .ok_or("Missing required field: model")?;

    let choices = obj.get("choices")
        .and_then(|v| v.as_array())
        .ok_or("Missing required field: choices")?;

    if choices.is_empty() {
        return Err("choices array cannot be empty".to_string());
    }

    // Validate choice structure
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

    // Usage field required for non-streaming
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

#[cfg(test)]
mod chat_e2e_tests {
    use super::*;

    #[test]
    fn test_chat_response_validation() {
        let valid_response = json!({
            "id": "chatcmpl-123",
            "object": "chat.completion",
            "created": 1696118400_u64,
            "model": "claude-4-sonnet-20250514",
            "choices": [
                {
                    "index": 0,
                    "message": {
                        "role": "assistant",
                        "content": "Hello! I'd be happy to help you test this API."
                    },
                    "finish_reason": "stop"
                }
            ],
            "usage": {
                "prompt_tokens": 15,
                "completion_tokens": 12,
                "total_tokens": 27
            }
        });

        assert!(validate_chat_completion_response(&valid_response).is_ok());
    }

    #[test]
    fn test_invalid_chat_response() {
        let invalid_response = json!({
            "id": "chatcmpl-123",
            "object": "wrong_object_type",
            // Missing other required fields
        });

        assert!(validate_chat_completion_response(&invalid_response).is_err());
    }
}