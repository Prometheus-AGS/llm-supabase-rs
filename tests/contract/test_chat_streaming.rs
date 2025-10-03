use axum_test::TestServer;
use serde_json::{json, Value};
use axum::http::StatusCode;

/// Contract test for POST /v1/chat/completions (streaming)
///
/// This test validates the exact Server-Sent Events streaming format
/// specified in openai-api.yaml:
/// - Endpoint: POST /v1/chat/completions with stream: true
/// - Response: text/event-stream with exact SSE format
/// - Must follow OpenAI's streaming protocol exactly
///
/// This test MUST FAIL until streaming chat completions are implemented.
#[tokio::test]
async fn test_chat_completions_streaming_contract() {
    let app = create_test_app().await;
    let server = TestServer::new(app).unwrap();

    // Streaming chat completion request
    let request_body = json!({
        "model": "claude-4-sonnet-20250514",
        "messages": [
            {
                "role": "user",
                "content": "Count from 1 to 3 slowly."
            }
        ],
        "stream": true,
        "max_tokens": 50
    });

    let response = server
        .post("/v1/chat/completions")
        .add_header("Authorization", "Bearer fake_jwt_token_for_testing")
        .add_header("Content-Type", "application/json")
        .json(&request_body)
        .await;

    // Validate response status
    assert_eq!(
        response.status_code(),
        StatusCode::OK,
        "Streaming chat completions should return 200 OK"
    );

    // Validate streaming response headers
    assert_eq!(
        response.header("content-type"),
        "text/event-stream",
        "Streaming response should have text/event-stream content type"
    );

    assert_eq!(
        response.header("cache-control"),
        "no-cache",
        "Streaming response should have no-cache header"
    );

    // Get response text for SSE parsing
    let body = response.text();

    // Validate SSE format
    validate_sse_format(&body).expect("Response should follow Server-Sent Events format");

    // Validate OpenAI streaming format
    validate_openai_streaming_format(&body).expect("Response should follow OpenAI streaming format");
}

/// Test streaming with different message types
#[tokio::test]
async fn test_streaming_message_types() {
    let app = create_test_app().await;
    let server = TestServer::new(app).unwrap();

    // Test system message + user message
    let request_body = json!({
        "model": "claude-4-sonnet-20250514",
        "messages": [
            {
                "role": "system",
                "content": "You are a helpful assistant. Always respond in exactly 5 words."
            },
            {
                "role": "user",
                "content": "Hello!"
            }
        ],
        "stream": true,
        "max_tokens": 10
    });

    let response = server
        .post("/v1/chat/completions")
        .add_header("Authorization", "Bearer fake_jwt_token")
        .add_header("Content-Type", "application/json")
        .json(&request_body)
        .await;

    assert_eq!(response.status_code(), StatusCode::OK);
    assert_eq!(response.header("content-type"), "text/event-stream");
}

/// Test streaming authentication requirement
#[tokio::test]
async fn test_streaming_authentication_required() {
    let app = create_test_app().await;
    let server = TestServer::new(app).unwrap();

    let request_body = json!({
        "model": "claude-4-sonnet-20250514",
        "messages": [{"role": "user", "content": "Hello"}],
        "stream": true
    });

    // No Authorization header
    let response = server
        .post("/v1/chat/completions")
        .add_header("Content-Type", "application/json")
        .json(&request_body)
        .await;

    // Should return 401, not stream
    assert_eq!(response.status_code(), StatusCode::UNAUTHORIZED);
    assert_eq!(response.header("content-type"), "application/json");

    let body: Value = response.json();
    assert!(body.get("error").is_some());
}

/// Test streaming with invalid parameters
#[tokio::test]
async fn test_streaming_invalid_parameters() {
    let app = create_test_app().await;
    let server = TestServer::new(app).unwrap();

    // Invalid: empty messages with streaming
    let request_body = json!({
        "model": "claude-4-sonnet-20250514",
        "messages": [],
        "stream": true
    });

    let response = server
        .post("/v1/chat/completions")
        .add_header("Authorization", "Bearer fake_jwt_token")
        .add_header("Content-Type", "application/json")
        .json(&request_body)
        .await;

    // Should return 400 Bad Request, not stream
    assert_eq!(response.status_code(), StatusCode::BAD_REQUEST);
    assert_eq!(response.header("content-type"), "application/json");

    let body: Value = response.json();
    let error = &body["error"];
    assert_eq!(error["type"], "invalid_request_error");
}

/// Test streaming connection handling
#[tokio::test]
async fn test_streaming_connection_headers() {
    let app = create_test_app().await;
    let server = TestServer::new(app).unwrap();

    let request_body = json!({
        "model": "claude-4-sonnet-20250514",
        "messages": [{"role": "user", "content": "Hi"}],
        "stream": true
    });

    let response = server
        .post("/v1/chat/completions")
        .add_header("Authorization", "Bearer fake_jwt_token")
        .add_header("Content-Type", "application/json")
        .json(&request_body)
        .await;

    if response.status_code() == StatusCode::OK {
        // Validate required streaming headers
        assert_eq!(response.header("content-type"), "text/event-stream");
        assert_eq!(response.header("cache-control"), "no-cache");

        // Optional but recommended headers
        if response.header("connection") == "keep-alive" {
            // Connection keep-alive is present and correct
        }
    }
}

/// Helper function to create test application
async fn create_test_app() -> axum::Router {
    // This will fail until we implement the actual app
    panic!("Application not implemented yet - this test should fail!")
}

/// Validate Server-Sent Events format
fn validate_sse_format(body: &str) -> Result<(), String> {
    let lines: Vec<&str> = body.lines().collect();

    if lines.is_empty() {
        return Err("SSE response cannot be empty".to_string());
    }

    // Each data line should start with "data: "
    let mut has_data_lines = false;
    let mut has_done_marker = false;

    for line in lines {
        if line.starts_with("data: ") {
            has_data_lines = true;

            let data_content = &line[6..]; // Remove "data: " prefix

            if data_content == "[DONE]" {
                has_done_marker = true;
            } else {
                // Should be valid JSON
                serde_json::from_str::<Value>(data_content)
                    .map_err(|e| format!("Invalid JSON in SSE data line: {}", e))?;
            }
        }
        // Allow empty lines (used as separators in SSE)
        else if line.is_empty() {
            continue;
        }
        // Allow event type lines (optional)
        else if line.starts_with("event: ") {
            continue;
        }
        // Allow id lines (optional)
        else if line.starts_with("id: ") {
            continue;
        }
        else {
            return Err(format!("Invalid SSE line format: {}", line));
        }
    }

    if !has_data_lines {
        return Err("SSE response must contain at least one data line".to_string());
    }

    if !has_done_marker {
        return Err("SSE response must end with 'data: [DONE]'".to_string());
    }

    Ok(())
}

/// Validate OpenAI streaming format
fn validate_openai_streaming_format(body: &str) -> Result<(), String> {
    let lines: Vec<&str> = body.lines().collect();
    let mut chunks: Vec<Value> = Vec::new();
    let mut found_done = false;

    // Parse all data chunks
    for line in lines {
        if line.starts_with("data: ") {
            let data_content = &line[6..];

            if data_content == "[DONE]" {
                found_done = true;
            } else {
                let chunk: Value = serde_json::from_str(data_content)
                    .map_err(|e| format!("Invalid JSON chunk: {}", e))?;
                chunks.push(chunk);
            }
        }
    }

    if chunks.is_empty() {
        return Err("Must have at least one streaming chunk".to_string());
    }

    if !found_done {
        return Err("Stream must end with [DONE] marker".to_string());
    }

    // Validate chunk structure
    for (i, chunk) in chunks.iter().enumerate() {
        validate_streaming_chunk(chunk, i)
            .map_err(|e| format!("Chunk {} validation failed: {}", i, e))?;
    }

    // Validate streaming sequence
    validate_streaming_sequence(&chunks)?;

    Ok(())
}

/// Validate individual streaming chunk
fn validate_streaming_chunk(chunk: &Value, index: usize) -> Result<(), String> {
    let obj = chunk.as_object()
        .ok_or("Chunk must be an object")?;

    // Required fields
    let _id = obj.get("id")
        .and_then(|v| v.as_str())
        .ok_or("Chunk missing required field: id")?;

    let object_type = obj.get("object")
        .and_then(|v| v.as_str())
        .ok_or("Chunk missing required field: object")?;

    if object_type != "chat.completion.chunk" {
        return Err(format!("object must be 'chat.completion.chunk', got: {}", object_type));
    }

    let _created = obj.get("created")
        .and_then(|v| v.as_u64())
        .ok_or("Chunk missing required field: created")?;

    let _model = obj.get("model")
        .and_then(|v| v.as_str())
        .ok_or("Chunk missing required field: model")?;

    let choices = obj.get("choices")
        .and_then(|v| v.as_array())
        .ok_or("Chunk missing required field: choices")?;

    if choices.len() != 1 {
        return Err("Streaming chunks should have exactly one choice".to_string());
    }

    // Validate choice structure
    let choice = choices[0].as_object()
        .ok_or("Choice must be an object")?;

    let choice_index = choice.get("index")
        .and_then(|v| v.as_u64())
        .ok_or("Choice missing required field: index")?;

    if choice_index != 0 {
        return Err("Choice index should be 0 for single choice".to_string());
    }

    // Delta field is required for streaming chunks
    let delta = choice.get("delta")
        .and_then(|v| v.as_object())
        .ok_or("Choice missing required field: delta")?;

    // First chunk should have role, subsequent chunks have content
    if index == 0 {
        let role = delta.get("role")
            .and_then(|v| v.as_str());

        if let Some(r) = role {
            if r != "assistant" {
                return Err(format!("First chunk role must be 'assistant', got: {}", r));
            }
        }
    }

    // Validate finish_reason (null for most chunks, set for final chunk)
    if let Some(finish_reason) = choice.get("finish_reason") {
        if !finish_reason.is_null() {
            let reason = finish_reason.as_str()
                .ok_or("finish_reason must be string or null")?;

            if !matches!(reason, "stop" | "length" | "content_filter") {
                return Err(format!("Invalid finish_reason: {}", reason));
            }
        }
    }

    Ok(())
}

/// Validate streaming sequence logic
fn validate_streaming_sequence(chunks: &[Value]) -> Result<(), String> {
    if chunks.is_empty() {
        return Err("Cannot validate empty chunk sequence".to_string());
    }

    // First chunk should have role in delta
    let first_delta = chunks[0]["choices"][0]["delta"].as_object().unwrap();
    if !first_delta.contains_key("role") {
        return Err("First chunk delta should contain role".to_string());
    }

    // Last chunk should have finish_reason
    let last_choice = chunks[chunks.len() - 1]["choices"][0].as_object().unwrap();
    let finish_reason = last_choice.get("finish_reason");

    if let Some(reason) = finish_reason {
        if reason.is_null() {
            return Err("Last chunk should have non-null finish_reason".to_string());
        }
    } else {
        return Err("Last chunk missing finish_reason field".to_string());
    }

    // All chunks should have same id, model, created timestamp
    let first_id = chunks[0]["id"].as_str().unwrap();
    let first_model = chunks[0]["model"].as_str().unwrap();
    let first_created = chunks[0]["created"].as_u64().unwrap();

    for chunk in chunks {
        if chunk["id"].as_str().unwrap() != first_id {
            return Err("All chunks in stream should have same id".to_string());
        }
        if chunk["model"].as_str().unwrap() != first_model {
            return Err("All chunks in stream should have same model".to_string());
        }
        if chunk["created"].as_u64().unwrap() != first_created {
            return Err("All chunks in stream should have same created timestamp".to_string());
        }
    }

    Ok(())
}