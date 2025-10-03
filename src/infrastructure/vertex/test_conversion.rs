// src/infrastructure/vertex/test_conversion.rs
//
// Unit tests for OpenAI ↔ Vertex AI conversion
// Run with: cargo test test_openai_vertex_conversion

#[cfg(test)]
mod tests {
    use super::super::converter::FormatConverter;
    use super::super::types::*;
    use crate::models::{
        request::ChatCompletionRequest,
        common::{ChatMessage, MessageRole},
    };

    #[test]
    fn test_openai_vertex_conversion_basic() {
        println!("🧪 Testing OpenAI → Vertex AI → OpenAI conversion...");

        // 1. Create OpenAI request (what your API receives)
        let openai_request = ChatCompletionRequest {
            model: "claude-sonnet-4-5@20250929".to_string(),
            messages: vec![
                ChatMessage {
                    role: MessageRole::User,
                    content: "Hello! Test message.".to_string(),
                    name: None,
                    function_call: None,
                    tool_calls: None,
                    tool_call_id: None,
                },
            ],
            max_tokens: Some(100),
            temperature: Some(0.7),
            top_p: Some(0.9),
            n: None,
            stream: Some(false),
            stop: None,
            presence_penalty: None,
            frequency_penalty: None,
            logit_bias: None,
            logprobs: None,
            top_logprobs: None,
            user: None,
            functions: None,
            function_call: None,
            tools: None,
            tool_choice: None,
            parallel_tool_calls: None,
            response_format: None,
            seed: None,
            metadata: None,
            service_tier: None,
            store: None,
            stream_options: None,
        };

        // 2. Convert to Vertex AI format
        let vertex_request = FormatConverter::openai_to_vertex_v2(&openai_request)
            .expect("Should convert OpenAI request to Vertex AI");

        // 3. Validate Vertex AI request
        assert_eq!(vertex_request.anthropic_version, "vertex-2023-10-16");
        assert_eq!(vertex_request.max_tokens, 100);
        assert_eq!(vertex_request.temperature, Some(0.7));
        assert_eq!(vertex_request.top_p, Some(0.9));
        assert_eq!(vertex_request.messages.len(), 1);
        assert_eq!(vertex_request.messages[0].content, "Hello! Test message.");

        println!("✅ OpenAI → Vertex AI conversion: PASSED");

        // 4. Create mock Vertex AI response
        let vertex_response = VertexPredictResponse {
            id: "msg_test123".to_string(),
            response_type: "message".to_string(),
            role: "assistant".to_string(),
            content: vec![
                VertexContent {
                    content_type: "text".to_string(),
                    text: "Hello! This is a test response.".to_string(),
                },
            ],
            model: "claude-sonnet-4-5@20250929".to_string(),
            stop_reason: Some("end_turn".to_string()),
            stop_sequence: None,
            usage: VertexUsage {
                input_tokens: 10,
                output_tokens: 8,
            },
        };

        // 5. Convert back to OpenAI format
        let openai_response = FormatConverter::vertex_to_openai_v2(
            &vertex_response,
            "chatcmpl-test123",
            "claude-sonnet-4-5@20250929",
            &openai_request,
        ).expect("Should convert Vertex AI response to OpenAI");

        // 6. Validate OpenAI response
        assert_eq!(openai_response.id, "chatcmpl-test123");
        assert_eq!(openai_response.object, "chat.completion");
        assert_eq!(openai_response.model, "claude-sonnet-4-5@20250929");
        assert_eq!(openai_response.choices.len(), 1);
        assert_eq!(openai_response.choices[0].message.content, "Hello! This is a test response.");
        assert_eq!(openai_response.usage.prompt_tokens, 10);
        assert_eq!(openai_response.usage.completion_tokens, 8);
        assert_eq!(openai_response.usage.total_tokens, 18);

        println!("✅ Vertex AI → OpenAI conversion: PASSED");
        println!("🎉 Complete conversion flow test: SUCCESS!");
    }

    #[test]
    fn test_vertex_request_json_format() {
        println!("🧪 Testing Vertex AI JSON format...");

        let messages = vec![
            VertexMessage::user("What is 2+2?"),
        ];

        let vertex_request = VertexPredictRequest::new(messages, 50)
            .with_system("You are a helpful assistant")
            .with_temperature(0.5);

        // Serialize to JSON
        let json = serde_json::to_string_pretty(&vertex_request)
            .expect("Should serialize to JSON");

        println!("📄 Vertex AI Request JSON:\n{}", json);

        // Validate JSON contains required fields
        assert!(json.contains("\"anthropic_version\": \"vertex-2023-10-16\""));
        assert!(json.contains("\"messages\""));
        assert!(json.contains("\"max_tokens\": 50"));
        assert!(json.contains("\"system\": \"You are a helpful assistant\""));
        assert!(json.contains("\"temperature\": 0.5"));

        // Validate stream field is NOT included (it's skipped)
        assert!(!json.contains("\"stream\""));

        println!("✅ JSON format validation: PASSED");
    }

    #[test]
    fn test_streaming_conversion() {
        println!("🧪 Testing streaming conversion...");

        // Create streaming request
        let openai_request = ChatCompletionRequest {
            model: "claude-sonnet-4-5@20250929".to_string(),
            messages: vec![
                ChatMessage {
                    role: MessageRole::User,
                    content: "Count to 3".to_string(),
                    name: None,
                    function_call: None,
                    tool_calls: None,
                    tool_call_id: None,
                },
            ],
            max_tokens: Some(20),
            stream: Some(true),
            temperature: None,
            top_p: None,
            n: None,
            stop: None,
            presence_penalty: None,
            frequency_penalty: None,
            logit_bias: None,
            logprobs: None,
            top_logprobs: None,
            user: None,
            functions: None,
            function_call: None,
            tools: None,
            tool_choice: None,
            parallel_tool_calls: None,
            response_format: None,
            seed: None,
            metadata: None,
            service_tier: None,
            store: None,
            stream_options: None,
        };

        // Convert to streaming request
        let vertex_request = FormatConverter::openai_to_vertex_streaming(&openai_request)
            .expect("Should convert to streaming");

        assert_eq!(vertex_request.stream, true);
        assert_eq!(vertex_request.max_tokens, 20);

        println!("✅ Streaming request conversion: PASSED");

        // Test streaming chunk conversion
        let vertex_chunk = VertexStreamChunk {
            event_type: "content_block_delta".to_string(),
            index: Some(0),
            delta: Some(VertexDelta {
                delta_type: "text_delta".to_string(),
                text: Some("1, 2, 3".to_string()),
                stop_reason: None,
            }),
            message: None,
            content_block: None,
            usage: None,
        };

        let openai_chunk = FormatConverter::vertex_chunk_to_openai_v2(
            &vertex_chunk,
            "chatcmpl-stream123",
            "claude-sonnet-4-5@20250929",
        ).expect("Should convert chunk").expect("Should have content");

        assert_eq!(openai_chunk.id, "chatcmpl-stream123");
        assert_eq!(openai_chunk.object, "chat.completion.chunk");
        assert_eq!(openai_chunk.choices[0].delta.content, "1, 2, 3");

        println!("✅ Streaming chunk conversion: PASSED");
        println!("🎉 Streaming conversion test: SUCCESS!");
    }

    #[test]
    fn test_error_scenarios() {
        println!("🧪 Testing error handling...");

        // Test with empty messages (should handle gracefully)
        let empty_request = ChatCompletionRequest {
            model: "claude-sonnet-4-5@20250929".to_string(),
            messages: vec![],
            max_tokens: Some(100),
            temperature: None,
            top_p: None,
            n: None,
            stream: Some(false),
            stop: None,
            presence_penalty: None,
            frequency_penalty: None,
            logit_bias: None,
            logprobs: None,
            top_logprobs: None,
            user: None,
            functions: None,
            function_call: None,
            tools: None,
            tool_choice: None,
            parallel_tool_calls: None,
            response_format: None,
            seed: None,
            metadata: None,
            service_tier: None,
            store: None,
            stream_options: None,
        };

        let result = FormatConverter::openai_to_vertex_v2(&empty_request);
        assert!(result.is_ok(), "Should handle empty messages gracefully");

        let vertex_request = result.unwrap();
        assert_eq!(vertex_request.messages.len(), 0);

        println!("✅ Error handling test: PASSED");
    }

    /// This test simulates the exact flow your application will use
    #[test]
    fn test_real_world_scenario() {
        println!("🧪 Testing real-world API scenario...");

        // 1. Simulate incoming OpenAI API request
        let incoming_request = r#"{
            "model": "claude-sonnet-4-5@20250929",
            "messages": [
                {
                    "role": "system",
                    "content": "You are a helpful assistant."
                },
                {
                    "role": "user", 
                    "content": "What's the weather like today?"
                }
            ],
            "max_tokens": 150,
            "temperature": 0.7,
            "stream": false
        }"#;

        // 2. Parse OpenAI request
        let openai_request: ChatCompletionRequest = serde_json::from_str(incoming_request)
            .expect("Should parse OpenAI request JSON");

        // 3. Convert to Vertex AI format
        let vertex_request = FormatConverter::openai_to_vertex_v2(&openai_request)
            .expect("Should convert to Vertex AI format");

        // 4. Serialize for Vertex AI API call
        let vertex_json = serde_json::to_string(&vertex_request)
            .expect("Should serialize for Vertex AI");

        println!("📤 Request to Vertex AI:\n{}", 
            serde_json::to_string_pretty(&vertex_request).unwrap());

        // 5. Simulate Vertex AI response
        let vertex_response_json = r#"{
            "id": "msg_01ABC123",
            "type": "message",
            "role": "assistant",
            "content": [
                {
                    "type": "text",
                    "text": "I don't have access to real-time weather data. To get current weather information, I'd recommend checking a weather app or website like Weather.com or your local meteorological service."
                }
            ],
            "model": "claude-sonnet-4-5@20250929",
            "stop_reason": "end_turn",
            "usage": {
                "input_tokens": 25,
                "output_tokens": 35
            }
        }"#;

        let vertex_response: VertexPredictResponse = serde_json::from_str(vertex_response_json)
            .expect("Should parse Vertex AI response");

        // 6. Convert back to OpenAI format
        let openai_response = FormatConverter::vertex_to_openai_v2(
            &vertex_response,
            "chatcmpl-weather123",
            "claude-sonnet-4-5@20250929",
            &openai_request,
        ).expect("Should convert back to OpenAI format");

        // 7. Serialize for client response
        let response_json = serde_json::to_string_pretty(&openai_response)
            .expect("Should serialize OpenAI response");

        println!("📥 Response to client:\n{}", response_json);

        // 8. Validate response structure
        assert_eq!(openai_response.object, "chat.completion");
        assert_eq!(openai_response.model, "claude-sonnet-4-5@20250929");
        assert!(!openai_response.choices[0].message.content.is_empty());
        assert_eq!(openai_response.usage.total_tokens, 60);

        println!("✅ Real-world scenario test: PASSED");
        println!("🎉 Your OpenAI ↔ Vertex AI conversion is working correctly!");
    }
}
