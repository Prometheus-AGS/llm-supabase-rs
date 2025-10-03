use axum_test::TestServer;
use serde_json::{json, Value};
use axum::http::StatusCode;
use std::time::{Duration, Instant};

/// Integration test for Streaming Chat Completion E2E
///
/// This test implements Test 4 from quickstart.md:
/// "Streaming Chat Completion"
///
/// Validates complete streaming flow:
/// - Server-Sent Events (SSE) format compliance
/// - OpenAI streaming protocol adherence
/// - Chunk-by-chunk response processing
/// - Proper stream termination with [DONE]
/// - Performance under streaming load
///
/// This test MUST FAIL until the complete streaming pipeline is implemented.
#[tokio::test]
async fn test_streaming_chat_completion_e2e() {
    let app = create_test_app().await;
    let server = TestServer::new(app).unwrap();

    let jwt_token = create_valid_jwt_token();

    // Streaming request from quickstart.md
    let request_body = json!({
        "model": "claude-4-sonnet-20250514",
        "messages": [
            {
                "role": "user",
                "content": "Count from 1 to 5 slowly."
            }
        ],
        "stream": true,
        "max_tokens": 50
    });

    let start_time = Instant::now();

    let response = server
        .post("/v1/chat/completions")
        .add_header("Authorization", format!("Bearer {}", jwt_token))
        .add_header("Content-Type", "application/json")
        .json(&request_body)
        .await;

    let response_time = start_time.elapsed();

    // Validate response status and headers
    assert_eq!(
        response.status_code(),
        StatusCode::OK,
        "Streaming chat should return 200 OK"
    );

    assert_eq!(
        response.header("content-type"),
        "text/event-stream",
        "Streaming response should be text/event-stream"
    );

    assert_eq!(
        response.header("cache-control"),
        "no-cache",
        "Streaming should have no-cache header"
    );

    // Validate performance (streaming should start quickly)
    assert!(
        response_time < Duration::from_secs(3),
        "Streaming should start quickly, took: {:?}",
        response_time
    );

    // Get and parse SSE response
    let sse_body = response.text();

    // Validate SSE format and OpenAI streaming compliance
    validate_sse_streaming_response(&sse_body)
        .expect("SSE response should be valid");

    validate_openai_streaming_compliance(&sse_body)
        .expect("Response should comply with OpenAI streaming format");

    // Parse chunks and validate content progression
    let chunks = parse_sse_chunks(&sse_body)?;

    // Should have multiple chunks for "count from 1 to 5"
    assert!(chunks.len() >= 2, "Should have multiple streaming chunks");

    // First chunk should establish role
    let first_chunk = &chunks[0];
    let first_choice = &first_chunk["choices"][0];
    let first_delta = &first_choice["delta"];

    assert_eq!(
        first_delta["role"].as_str().unwrap(),
        "assistant",
        "First chunk should establish assistant role"
    );

    // Last chunk should have finish_reason
    let last_chunk = &chunks[chunks.len() - 1];
    let last_choice = &last_chunk["choices"][0];

    assert!(
        last_choice.get("finish_reason").is_some(),
        "Last chunk should have finish_reason"
    );

    let finish_reason = last_choice["finish_reason"].as_str().unwrap();
    assert!(
        matches!(finish_reason, "stop" | "length"),
        "Valid finish reason: {}",
        finish_reason
    );
}

/// Test streaming with different content types
#[tokio::test]
async fn test_streaming_content_variations() {
    let app = create_test_app().await;
    let server = TestServer::new(app).unwrap();

    let jwt_token = create_valid_jwt_token();

    // Test short response
    let request_body = json!({
        "model": "claude-4-sonnet-20250514",
        "messages": [{"role": "user", "content": "Say 'hi'"}],
        "stream": true,
        "max_tokens": 5
    });

    let response = server
        .post("/v1/chat/completions")
        .add_header("Authorization", format!("Bearer {}", jwt_token))
        .add_header("Content-Type", "application/json")
        .json(&request_body)
        .await;

    if response.status_code() == StatusCode::OK {
        assert_eq!(response.header("content-type"), "text/event-stream");

        let sse_body = response.text();
        let chunks = parse_sse_chunks(&sse_body)?;

        // Even short responses should have proper streaming structure
        assert!(!chunks.is_empty(), "Should have at least one chunk");

        // Validate all chunks have consistent ID and model
        if chunks.len() > 1 {
            let first_id = chunks[0]["id"].as_str().unwrap();
            let first_model = chunks[0]["model"].as_str().unwrap();

            for chunk in &chunks {
                assert_eq!(
                    chunk["id"].as_str().unwrap(),
                    first_id,
                    "All chunks should have same ID"
                );
                assert_eq!(
                    chunk["model"].as_str().unwrap(),
                    first_model,
                    "All chunks should have same model"
                );
            }
        }
    }
}

/// Test streaming authentication and error scenarios
#[tokio::test]
async fn test_streaming_authentication_errors() {
    let app = create_test_app().await;
    let server = TestServer::new(app).unwrap();

    // Test streaming without authentication
    let request_body = json!({
        "model": "claude-4-sonnet-20250514",
        "messages": [{"role": "user", "content": "Hello"}],
        "stream": true
    });

    let response = server
        .post("/v1/chat/completions")
        .add_header("Content-Type", "application/json")
        .json(&request_body)
        .await;

    // Should return 401, not start streaming
    assert_eq!(response.status_code(), StatusCode::UNAUTHORIZED);
    assert_eq!(response.header("content-type"), "application/json");

    // Should be error response, not SSE
    let body: Value = response.json();
    assert!(body.get("error").is_some());

    // Test streaming with invalid token
    let response = server
        .post("/v1/chat/completions")
        .add_header("Authorization", "Bearer invalid_token")
        .add_header("Content-Type", "application/json")
        .json(&request_body)
        .await;

    assert_eq!(response.status_code(), StatusCode::UNAUTHORIZED);
    assert_eq!(response.header("content-type"), "application/json");
}

/// Test streaming parameter validation
#[tokio::test]
async fn test_streaming_parameter_validation() {
    let app = create_test_app().await;
    let server = TestServer::new(app).unwrap();

    let jwt_token = create_valid_jwt_token();

    // Test invalid parameters with streaming
    let request_body = json!({
        "model": "claude-4-sonnet-20250514",
        "messages": [], // Empty messages
        "stream": true
    });

    let response = server
        .post("/v1/chat/completions")
        .add_header("Authorization", format!("Bearer {}", jwt_token))
        .add_header("Content-Type", "application/json")
        .json(&request_body)
        .await;

    // Should return 400, not start streaming
    assert_eq!(response.status_code(), StatusCode::BAD_REQUEST);
    assert_eq!(response.header("content-type"), "application/json");

    // Test temperature out of bounds with streaming
    let request_body = json!({
        "model": "claude-4-sonnet-20250514",
        "messages": [{"role": "user", "content": "Hello"}],
        "stream": true,
        "temperature": 3.0 // Over limit
    });

    let response = server
        .post("/v1/chat/completions")
        .add_header("Authorization", format!("Bearer {}", jwt_token))
        .add_header("Content-Type", "application/json")
        .json(&request_body)
        .await;

    assert_eq!(response.status_code(), StatusCode::BAD_REQUEST);
}

/// Test streaming performance and concurrency
#[tokio::test]
async fn test_streaming_performance() {
    let app = create_test_app().await;
    let server = TestServer::new(app).unwrap();

    let jwt_token = create_valid_jwt_token();

    let request_body = json!({
        "model": "claude-4-sonnet-20250514",
        "messages": [{"role": "user", "content": "Quick response"}],
        "stream": true,
        "max_tokens": 20
    });

    // Test multiple concurrent streaming requests
    let mut handles = Vec::new();

    for i in 0..3 {
        let server_clone = server.clone();
        let jwt_clone = jwt_token.clone();
        let request_clone = request_body.clone();

        let handle = tokio::spawn(async move {
            let start_time = Instant::now();

            let response = server_clone
                .post("/v1/chat/completions")
                .add_header("Authorization", format!("Bearer {}", jwt_clone))
                .add_header("Content-Type", "application/json")
                .json(&request_clone)
                .await;

            let response_time = start_time.elapsed();

            (i, response.status_code(), response_time)
        });

        handles.push(handle);
    }

    // Wait for all streaming requests
    let mut results = Vec::new();
    for handle in handles {
        let result = handle.await.unwrap();
        results.push(result);
    }

    // Validate concurrent streaming performance
    for (i, status, time) in results {
        if status == StatusCode::OK {
            assert!(
                time < Duration::from_secs(5),
                "Concurrent stream {} should start quickly: {:?}",
                i,
                time
            );
        }
    }
}

/// Test streaming connection handling
#[tokio::test]
async fn test_streaming_connection_management() {
    let app = create_test_app().await;
    let server = TestServer::new(app).unwrap();

    let jwt_token = create_valid_jwt_token();

    let request_body = json!({
        "model": "claude-4-sonnet-20250514",
        "messages": [{"role": "user", "content": "Test streaming"}],
        "stream": true,
        "max_tokens": 30
    });

    let response = server
        .post("/v1/chat/completions")
        .add_header("Authorization", format!("Bearer {}", jwt_token))
        .add_header("Content-Type", "application/json")
        .json(&request_body)
        .await;

    if response.status_code() == StatusCode::OK {
        // Validate streaming headers
        assert_eq!(response.header("content-type"), "text/event-stream");
        assert_eq!(response.header("cache-control"), "no-cache");

        // Connection should be keep-alive for streaming
        if response.header("connection") == "keep-alive" {
            // Connection management is working
        }

        // Validate proper stream termination
        let sse_body = response.text();
        assert!(
            sse_body.contains("data: [DONE]"),
            "Stream should properly terminate with [DONE]"
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

/// Validate SSE streaming response format
fn validate_sse_streaming_response(sse_body: &str) -> Result<(), String> {
    let lines: Vec<&str> = sse_body.lines().collect();

    if lines.is_empty() {
        return Err("SSE response cannot be empty".to_string());
    }

    let mut found_data = false;
    let mut found_done = false;

    for line in lines {
        if line.starts_with("data: ") {
            found_data = true;
            let data_content = &line[6..];

            if data_content == "[DONE]" {
                found_done = true;
            } else if !data_content.is_empty() {
                // Should be valid JSON
                serde_json::from_str::<Value>(data_content)
                    .map_err(|e| format!("Invalid JSON in SSE: {}", e))?;
            }
        }
        // Allow empty lines and other SSE fields
        else if line.is_empty() || line.starts_with("event:") || line.starts_with("id:") {
            continue;
        } else {
            return Err(format!("Invalid SSE line: {}", line));
        }
    }

    if !found_data {
        return Err("SSE must contain data lines".to_string());
    }

    if !found_done {
        return Err("SSE stream must end with [DONE]".to_string());
    }

    Ok(())
}

/// Validate OpenAI streaming protocol compliance
fn validate_openai_streaming_compliance(sse_body: &str) -> Result<(), String> {
    let chunks = parse_sse_chunks(sse_body)?;

    if chunks.is_empty() {
        return Err("Must have at least one streaming chunk".to_string());
    }

    // Validate each chunk structure
    for (i, chunk) in chunks.iter().enumerate() {
        validate_streaming_chunk_structure(chunk, i)?;
    }

    // Validate streaming sequence logic
    validate_streaming_sequence(&chunks)?;

    Ok(())
}

/// Parse SSE chunks from response body
fn parse_sse_chunks(sse_body: &str) -> Result<Vec<Value>, String> {
    let mut chunks = Vec::new();

    for line in sse_body.lines() {
        if line.starts_with("data: ") {
            let data_content = &line[6..];

            if data_content != "[DONE]" && !data_content.is_empty() {
                let chunk: Value = serde_json::from_str(data_content)
                    .map_err(|e| format!("Failed to parse chunk JSON: {}", e))?;
                chunks.push(chunk);
            }
        }
    }

    Ok(chunks)
}

/// Validate individual streaming chunk structure
fn validate_streaming_chunk_structure(chunk: &Value, index: usize) -> Result<(), String> {
    let obj = chunk.as_object()
        .ok_or("Chunk must be an object")?;

    // Required fields
    let _id = obj.get("id")
        .and_then(|v| v.as_str())
        .ok_or("Missing chunk field: id")?;

    let object_type = obj.get("object")
        .and_then(|v| v.as_str())
        .ok_or("Missing chunk field: object")?;

    if object_type != "chat.completion.chunk" {
        return Err(format!("object must be 'chat.completion.chunk', got: {}", object_type));
    }

    let choices = obj.get("choices")
        .and_then(|v| v.as_array())
        .ok_or("Missing chunk field: choices")?;

    if choices.len() != 1 {
        return Err("Streaming chunks should have exactly one choice".to_string());
    }

    let choice = choices[0].as_object()
        .ok_or("Choice must be an object")?;

    let _delta = choice.get("delta")
        .and_then(|v| v.as_object())
        .ok_or("Choice missing delta field")?;

    // First chunk should have role, others may have content
    if index == 0 {
        let delta = choice["delta"].as_object().unwrap();
        if !delta.contains_key("role") {
            return Err("First chunk delta should contain role".to_string());
        }
    }

    Ok(())
}

/// Validate streaming sequence logic
fn validate_streaming_sequence(chunks: &[Value]) -> Result<(), String> {
    if chunks.is_empty() {
        return Err("Cannot validate empty chunk sequence".to_string());
    }

    // All chunks should have same ID and model
    let first_id = chunks[0]["id"].as_str().unwrap();
    let first_model = chunks[0]["model"].as_str().unwrap();

    for chunk in chunks {
        if chunk["id"].as_str().unwrap() != first_id {
            return Err("All chunks should have same ID".to_string());
        }
        if chunk["model"].as_str().unwrap() != first_model {
            return Err("All chunks should have same model".to_string());
        }
    }

    // Last chunk should have finish_reason
    let last_choice = chunks[chunks.len() - 1]["choices"][0].as_object().unwrap();
    if !last_choice.contains_key("finish_reason") ||
       last_choice["finish_reason"].is_null() {
        return Err("Last chunk should have non-null finish_reason".to_string());
    }

    Ok(())
}

#[cfg(test)]
mod streaming_e2e_tests {
    use super::*;

    #[test]
    fn test_sse_parsing() {
        let sse_body = r#"data: {"id":"test","object":"chat.completion.chunk","choices":[{"delta":{"role":"assistant"}}]}

data: {"id":"test","object":"chat.completion.chunk","choices":[{"delta":{"content":"Hello"}}]}

data: [DONE]
"#;

        assert!(validate_sse_streaming_response(sse_body).is_ok());

        let chunks = parse_sse_chunks(sse_body).unwrap();
        assert_eq!(chunks.len(), 2);
    }

    #[test]
    fn test_chunk_structure_validation() {
        let valid_chunk = json!({
            "id": "test",
            "object": "chat.completion.chunk",
            "created": 1696118400_u64,
            "model": "claude-4-sonnet-20250514",
            "choices": [
                {
                    "index": 0,
                    "delta": {"role": "assistant"},
                    "finish_reason": null
                }
            ]
        });

        assert!(validate_streaming_chunk_structure(&valid_chunk, 0).is_ok());
    }
}