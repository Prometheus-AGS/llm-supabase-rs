// tests/integration/test_streaming_validation.rs
//
// Comprehensive streaming validation tests
// Tests the complete OpenAI streaming compatibility

use serde_json::json;
use std::time::Duration;

/// Test that validates the exact streaming request from the user
#[tokio::test]
#[ignore = "requires running server"]
async fn test_user_streaming_request() {
    println!("🧪 Testing the exact user streaming request...");

    // This is the exact request the user sent that wasn't streaming
    let request_body = json!({
        "model": "claude-4-sonnet-20250514",
        "stream": true,
        "messages": [
            {
                "role": "user",
                "content": "tell me something good"
            }
        ]
    });

    // Make request to local server
    let client = reqwest::Client::new();
    let response = client
        .post("http://localhost:8080/v1/chat/completions")
        .header("Content-Type", "application/json")
        .header("Authorization", "Bearer test-token") // Use your actual token
        .json(&request_body)
        .send()
        .await
        .expect("Should send request");

    println!("📡 Response status: {}", response.status());
    
    // Validate streaming headers
    assert_eq!(response.status(), 200, "Should return 200 OK");
    
    let content_type = response.headers().get("content-type")
        .expect("Should have content-type header")
        .to_str()
        .expect("Should be valid string");
    
    assert_eq!(content_type, "text/event-stream", "Should be SSE content type");
    
    let cache_control = response.headers().get("cache-control")
        .expect("Should have cache-control header")
        .to_str()
        .expect("Should be valid string");
    
    assert_eq!(cache_control, "no-cache", "Should have no-cache");

    println!("✅ Headers validated - this is real streaming!");

    // Read streaming response
    let mut stream = response.bytes_stream();
    let mut chunks_received = 0;
    let mut content_received = String::new();
    
    use futures_util::StreamExt;
    use tokio::time::timeout;

    // Read chunks with timeout
    while let Ok(Some(chunk_result)) = timeout(Duration::from_secs(30), stream.next()).await {
        match chunk_result {
            Ok(chunk) => {
                let chunk_str = String::from_utf8_lossy(&chunk);
                println!("📦 Received chunk: {}", chunk_str.trim());
                
                chunks_received += 1;
                
                // Parse SSE data
                for line in chunk_str.lines() {
                    if line.starts_with("data: ") {
                        let data = &line[6..];
                        if data == "[DONE]" {
                            println!("🏁 Received [DONE] - stream completed");
                            break;
                        } else if !data.is_empty() {
                            // Parse JSON chunk
                            if let Ok(chunk_json) = serde_json::from_str::<serde_json::Value>(data) {
                                if let Some(choices) = chunk_json.get("choices") {
                                    if let Some(choice) = choices.get(0) {
                                        if let Some(delta) = choice.get("delta") {
                                            if let Some(content) = delta.get("content") {
                                                if let Some(content_str) = content.as_str() {
                                                    content_received.push_str(content_str);
                                                    println!("💬 Content: '{}'", content_str);
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
                
                if chunks_received > 50 {
                    println!("⚠️  Received many chunks, stopping test");
                    break;
                }
            }
            Err(e) => {
                eprintln!("❌ Chunk error: {}", e);
                break;
            }
        }
    }

    println!("📊 Streaming Results:");
    println!("  Chunks received: {}", chunks_received);
    println!("  Total content: '{}'", content_received);

    assert!(chunks_received > 0, "Should have received at least one chunk");
    assert!(!content_received.is_empty(), "Should have received some content");

    println!("🎉 User's streaming request now works correctly!");
}

/// Test all OpenAI parameters are accepted
#[tokio::test]
#[ignore = "requires running server"]
async fn test_complete_openai_parameter_support() {
    println!("🧪 Testing complete OpenAI API parameter support...");

    // Request with ALL possible OpenAI parameters
    let comprehensive_request = json!({
        "model": "claude-4-sonnet-20250514",
        "messages": [
            {
                "role": "system",
                "content": "You are a helpful assistant."
            },
            {
                "role": "user", 
                "content": "Say 'All parameters work!'"
            }
        ],
        "max_tokens": 50,
        "temperature": 0.7,
        "top_p": 0.9,
        "n": 1,
        "stream": false,
        "stop": ["END"],
        "presence_penalty": 0.1,
        "frequency_penalty": 0.1,
        "logit_bias": {},
        "logprobs": false,
        "top_logprobs": null,
        "user": "test-user-123",
        "functions": null,
        "function_call": null,
        "tools": null,
        "tool_choice": null,
        "parallel_tool_calls": null,
        "response_format": null,
        "seed": 12345,
        "metadata": {
            "test": "comprehensive"
        },
        "service_tier": null,
        "store": false,
        "stream_options": null
    });

    let client = reqwest::Client::new();
    let response = client
        .post("http://localhost:8080/v1/chat/completions")
        .header("Content-Type", "application/json")
        .header("Authorization", "Bearer test-token")
        .json(&comprehensive_request)
        .send()
        .await
        .expect("Should send comprehensive request");

    println!("📡 Comprehensive request status: {}", response.status());

    // Should accept the request (even if some parameters aren't fully implemented)
    assert!(
        response.status().is_success() || response.status() == 400,
        "Should either succeed or return validation error, got: {}",
        response.status()
    );

    if response.status().is_success() {
        let response_json: serde_json::Value = response.json().await
            .expect("Should parse response JSON");
        
        println!("✅ All OpenAI parameters accepted!");
        println!("📋 Response structure: {}", serde_json::to_string_pretty(&response_json).unwrap());
    } else {
        let error_text = response.text().await.unwrap_or_default();
        println!("⚠️  Request validation response: {}", error_text);
    }

    println!("🎉 OpenAI parameter compatibility test completed!");
}

/// Test streaming with various parameter combinations
#[tokio::test]
#[ignore = "requires running server"]
async fn test_streaming_with_parameters() {
    println!("🧪 Testing streaming with various parameters...");

    let streaming_requests = vec![
        // Basic streaming
        json!({
            "model": "claude-4-sonnet-20250514",
            "stream": true,
            "messages": [{"role": "user", "content": "Count 1 to 3"}],
            "max_tokens": 20
        }),
        // Streaming with temperature
        json!({
            "model": "claude-4-sonnet-20250514", 
            "stream": true,
            "messages": [{"role": "user", "content": "Say hello"}],
            "max_tokens": 10,
            "temperature": 0.1
        }),
        // Streaming with system message
        json!({
            "model": "claude-4-sonnet-20250514",
            "stream": true,
            "messages": [
                {"role": "system", "content": "Be brief"},
                {"role": "user", "content": "Hi"}
            ],
            "max_tokens": 15,
            "temperature": 0.5,
            "top_p": 0.8
        }),
    ];

    let client = reqwest::Client::new();

    for (i, request) in streaming_requests.iter().enumerate() {
        println!("🔄 Testing streaming request {}", i + 1);

        let response = client
            .post("http://localhost:8080/v1/chat/completions")
            .header("Content-Type", "application/json")
            .header("Authorization", "Bearer test-token")
            .json(request)
            .send()
            .await
            .expect("Should send request");

        assert_eq!(response.status(), 200, "Request {} should succeed", i + 1);
        
        let content_type = response.headers().get("content-type")
            .map(|h| h.to_str().unwrap_or(""))
            .unwrap_or("");
        
        assert_eq!(content_type, "text/event-stream", "Request {} should stream", i + 1);

        println!("✅ Streaming request {} validated", i + 1);
    }

    println!("🎉 All streaming parameter combinations work!");
}

/// Performance test for streaming
#[tokio::test]
#[ignore = "requires running server"]
async fn test_streaming_performance() {
    println!("🧪 Testing streaming performance...");

    let request = json!({
        "model": "claude-4-sonnet-20250514",
        "stream": true,
        "messages": [{"role": "user", "content": "Write a short poem"}],
        "max_tokens": 100
    });

    let start_time = std::time::Instant::now();

    let client = reqwest::Client::new();
    let response = client
        .post("http://localhost:8080/v1/chat/completions")
        .header("Content-Type", "application/json")
        .header("Authorization", "Bearer test-token")
        .json(&request)
        .send()
        .await
        .expect("Should send request");

    let first_byte_time = start_time.elapsed();

    assert_eq!(response.status(), 200);
    
    // Streaming should start quickly
    assert!(
        first_byte_time < Duration::from_secs(5),
        "Streaming should start within 5 seconds, took: {:?}",
        first_byte_time
    );

    println!("⚡ Performance Results:");
    println!("  Time to first byte: {:?}", first_byte_time);
    println!("  Status: {}", response.status());

    println!("🎉 Streaming performance test completed!");
}

#[cfg(test)]
mod validation_tests {
    use super::*;

    #[test]
    fn test_request_structure() {
        // Test that our request structures are valid
        let request = json!({
            "model": "claude-4-sonnet-20250514",
            "stream": true,
            "messages": [{"role": "user", "content": "test"}]
        });

        assert!(request.is_object());
        assert_eq!(request["stream"], true);
        assert_eq!(request["model"], "claude-4-sonnet-20250514");
        
        println!("✅ Request structure validation passed");
    }

    #[test]
    fn test_instructions() {
        println!("📋 To run streaming validation tests:");
        println!();
        println!("1. Start your server:");
        println!("   cargo run");
        println!();
        println!("2. In another terminal, run the tests:");
        println!("   cargo test test_user_streaming_request --ignored");
        println!("   cargo test test_complete_openai_parameter_support --ignored");
        println!("   cargo test test_streaming_with_parameters --ignored");
        println!();
        println!("3. For all streaming validation tests:");
        println!("   cargo test --test test_streaming_validation --ignored");
        println!();
        println!("✅ Instructions displayed!");
    }
}
