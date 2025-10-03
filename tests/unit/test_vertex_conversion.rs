// tests/unit/test_vertex_conversion.rs
//
// Unit tests for OpenAI ↔ Vertex AI conversion flow
// Tests the complete pipeline: OpenAI Request → Vertex AI → OpenAI Response

use serde_json::json;
use llm_supabase_rs::models::{
    request::ChatCompletionRequest,
    response::{ChatCompletionResponse, ChatCompletionChunk},
    common::{ChatMessage, MessageRole, Usage, FinishReason},
};
use llm_supabase_rs::infrastructure::vertex::{
    converter::FormatConverter,
    types::{VertexPredictRequest, VertexPredictResponse, VertexStreamChunk, VertexMessage, VertexRole, VertexContent, VertexUsage},
};

/// Test the complete OpenAI → Vertex AI → OpenAI conversion flow
#[test]
fn test_openai_to_vertex_to_openai_conversion() {
    // 1. Create OpenAI-style request (what your API receives)
    let openai_request = ChatCompletionRequest {
        model: "claude-sonnet-4-5@20250929".to_string(),
        messages: vec![
            ChatMessage {
                role: MessageRole::System,
                content: "You are a helpful assistant.".to_string(),
                name: None,
                function_call: None,
                tool_calls: None,
                tool_call_id: None,
            },
            ChatMessage {
                role: MessageRole::User,
                content: "Hello! Can you help me test this API?".to_string(),
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
        stream_options: None,
    };

    // 2. Convert OpenAI request to Vertex AI format
    let vertex_request = FormatConverter::openai_to_vertex_v2(&openai_request)
        .expect("Should convert OpenAI request to Vertex AI format");

    // 3. Validate Vertex AI request structure
    assert_eq!(vertex_request.anthropic_version, "vertex-2023-10-16");
    assert_eq!(vertex_request.max_tokens, 100);
    assert_eq!(vertex_request.temperature, Some(0.7));
    assert_eq!(vertex_request.top_p, Some(0.9));
    assert_eq!(vertex_request.system, Some("You are a helpful assistant.".to_string()));
    
    // Should have only user message (system is handled separately)
    assert_eq!(vertex_request.messages.len(), 1);
    assert!(matches!(vertex_request.messages[0].role, VertexRole::User));
    assert_eq!(vertex_request.messages[0].content, "Hello! Can you help me test this API?");

    // 4. Simulate Vertex AI response (what you'd get back from Vertex AI)
    let vertex_response = VertexPredictResponse {
        id: "msg_123456".to_string(),
        response_type: "message".to_string(),
        role: "assistant".to_string(),
        content: vec![
            VertexContent {
                content_type: "text".to_string(),
                text: "Hello! I'd be happy to help you test this API. ".to_string(),
            },
            VertexContent {
                content_type: "text".to_string(),
                text: "Everything looks like it's working correctly!".to_string(),
            },
        ],
        model: "claude-sonnet-4-5@20250929".to_string(),
        stop_reason: Some("end_turn".to_string()),
        stop_sequence: None,
        usage: VertexUsage {
            input_tokens: 25,
            output_tokens: 15,
        },
    };

    // 5. Convert Vertex AI response back to OpenAI format
    let openai_response = FormatConverter::vertex_to_openai_v2(
        &vertex_response,
        "chatcmpl-123456",
        "claude-sonnet-4-5@20250929",
        &openai_request,
    ).expect("Should convert Vertex AI response to OpenAI format");

    // 6. Validate OpenAI response structure
    assert_eq!(openai_response.id, "chatcmpl-123456");
    assert_eq!(openai_response.object, "chat.completion");
    assert_eq!(openai_response.model, "claude-sonnet-4-5@20250929");
    assert_eq!(openai_response.choices.len(), 1);

    let choice = &openai_response.choices[0];
    assert_eq!(choice.index, 0);
    assert_eq!(choice.message.role, MessageRole::Assistant);
    assert_eq!(choice.message.content, "Hello! I'd be happy to help you test this API. Everything looks like it's working correctly!");
    assert_eq!(choice.finish_reason, FinishReason::Stop);

    // Validate usage statistics
    assert_eq!(openai_response.usage.prompt_tokens, 25);
    assert_eq!(openai_response.usage.completion_tokens, 15);
    assert_eq!(openai_response.usage.total_tokens, 40);

    println!("✅ OpenAI → Vertex AI → OpenAI conversion test passed!");
}

/// Test streaming conversion flow
#[test]
fn test_streaming_conversion_flow() {
    // 1. Create streaming OpenAI request
    let openai_request = ChatCompletionRequest {
        model: "claude-sonnet-4-5@20250929".to_string(),
        messages: vec![
            ChatMessage {
                role: MessageRole::User,
                content: "Count from 1 to 3".to_string(),
                name: None,
                function_call: None,
                tool_calls: None,
                tool_call_id: None,
            },
        ],
        max_tokens: Some(50),
        temperature: Some(0.7),
        stream: Some(true),
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
        stream_options: None,
    };

    // 2. Convert to Vertex AI streaming request
    let vertex_request = FormatConverter::openai_to_vertex_streaming(&openai_request)
        .expect("Should convert to streaming request");

    assert_eq!(vertex_request.stream, true);
    assert_eq!(vertex_request.max_tokens, 50);

    // 3. Simulate streaming chunks from Vertex AI
    let streaming_chunks = vec![
        // First chunk - message start
        VertexStreamChunk {
            event_type: "message_start".to_string(),
            index: None,
            delta: None,
            message: None,
            content_block: None,
            usage: None,
        },
        // Content chunk 1
        VertexStreamChunk {
            event_type: "content_block_delta".to_string(),
            index: Some(0),
            delta: Some(llm_supabase_rs::infrastructure::vertex::types::VertexDelta {
                delta_type: "text_delta".to_string(),
                text: Some("1, ".to_string()),
                stop_reason: None,
            }),
            message: None,
            content_block: None,
            usage: None,
        },
        // Content chunk 2
        VertexStreamChunk {
            event_type: "content_block_delta".to_string(),
            index: Some(0),
            delta: Some(llm_supabase_rs::infrastructure::vertex::types::VertexDelta {
                delta_type: "text_delta".to_string(),
                text: Some("2, ".to_string()),
                stop_reason: None,
            }),
            message: None,
            content_block: None,
            usage: None,
        },
        // Content chunk 3
        VertexStreamChunk {
            event_type: "content_block_delta".to_string(),
            index: Some(0),
            delta: Some(llm_supabase_rs::infrastructure::vertex::types::VertexDelta {
                delta_type: "text_delta".to_string(),
                text: Some("3".to_string()),
                stop_reason: None,
            }),
            message: None,
            content_block: None,
            usage: None,
        },
        // Final chunk with usage
        VertexStreamChunk {
            event_type: "message_stop".to_string(),
            index: None,
            delta: None,
            message: None,
            content_block: None,
            usage: Some(VertexUsage {
                input_tokens: 10,
                output_tokens: 8,
            }),
        },
    ];

    // 4. Convert each chunk to OpenAI format
    let mut openai_chunks = Vec::new();
    for chunk in streaming_chunks {
        if let Ok(Some(openai_chunk)) = FormatConverter::vertex_chunk_to_openai_v2(
            &chunk,
            "chatcmpl-stream-123",
            "claude-sonnet-4-5@20250929",
        ) {
            openai_chunks.push(openai_chunk);
        }
    }

    // 5. Validate streaming response
    assert!(!openai_chunks.is_empty(), "Should have converted chunks");

    // Check that we have content chunks
    let content_chunks: Vec<_> = openai_chunks.iter()
        .filter(|chunk| !chunk.choices[0].delta.content.is_empty())
        .collect();
    
    assert!(!content_chunks.is_empty(), "Should have content chunks");

    // Verify final chunk has usage
    let final_chunk = openai_chunks.last().unwrap();
    if let Some(usage) = &final_chunk.usage {
        assert_eq!(usage.prompt_tokens, 10);
        assert_eq!(usage.completion_tokens, 8);
        assert_eq!(usage.total_tokens, 18);
    }

    println!("✅ Streaming conversion test passed!");
}

/// Test error handling in conversion
#[test]
fn test_conversion_error_handling() {
    // Test with invalid request
    let invalid_request = ChatCompletionRequest {
        model: "claude-sonnet-4-5@20250929".to_string(),
        messages: vec![], // Empty messages should be handled gracefully
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
        stream_options: None,
    };

    // Should handle empty messages gracefully
    let result = FormatConverter::openai_to_vertex_v2(&invalid_request);
    assert!(result.is_ok(), "Should handle empty messages gracefully");

    let vertex_request = result.unwrap();
    assert_eq!(vertex_request.messages.len(), 0);
    assert_eq!(vertex_request.max_tokens, 100);

    println!("✅ Error handling test passed!");
}

/// Test JSON serialization/deserialization
#[test]
fn test_json_serialization() {
    // Test Vertex AI request serialization
    let messages = vec![
        VertexMessage {
            role: VertexRole::User,
            content: "Hello Claude".to_string(),
        },
    ];

    let vertex_request = VertexPredictRequest::new(messages, 100)
        .with_system("You are helpful")
        .with_temperature(0.7);

    // Should serialize to JSON
    let json_str = serde_json::to_string(&vertex_request)
        .expect("Should serialize to JSON");

    // Should contain required fields
    assert!(json_str.contains("anthropic_version"));
    assert!(json_str.contains("vertex-2023-10-16"));
    assert!(json_str.contains("messages"));
    assert!(json_str.contains("max_tokens"));
    assert!(json_str.contains("system"));
    assert!(json_str.contains("temperature"));

    // Should NOT contain stream field (it's skipped)
    assert!(!json_str.contains("stream"));

    // Should deserialize back
    let deserialized: VertexPredictRequest = serde_json::from_str(&json_str)
        .expect("Should deserialize from JSON");

    assert_eq!(deserialized.max_tokens, 100);
    assert_eq!(deserialized.temperature, Some(0.7));
    assert_eq!(deserialized.system, Some("You are helpful".to_string()));

    println!("✅ JSON serialization test passed!");
}

#[cfg(test)]
mod integration_helpers {
    use super::*;

    /// Helper to create a realistic test scenario
    pub fn create_test_scenario() -> (ChatCompletionRequest, VertexPredictResponse) {
        let openai_request = ChatCompletionRequest {
            model: "claude-sonnet-4-5@20250929".to_string(),
            messages: vec![
                ChatMessage {
                    role: MessageRole::User,
                    content: "What is the capital of France?".to_string(),
                    name: None,
                    function_call: None,
                    tool_calls: None,
                    tool_call_id: None,
                },
            ],
            max_tokens: Some(50),
            temperature: Some(0.5),
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
            stream_options: None,
        };

        let vertex_response = VertexPredictResponse {
            id: "msg_test_123".to_string(),
            response_type: "message".to_string(),
            role: "assistant".to_string(),
            content: vec![
                VertexContent {
                    content_type: "text".to_string(),
                    text: "The capital of France is Paris.".to_string(),
                },
            ],
            model: "claude-sonnet-4-5@20250929".to_string(),
            stop_reason: Some("end_turn".to_string()),
            stop_sequence: None,
            usage: VertexUsage {
                input_tokens: 12,
                output_tokens: 8,
            },
        };

        (openai_request, vertex_response)
    }

    #[test]
    fn test_realistic_scenario() {
        let (openai_request, vertex_response) = create_test_scenario();

        // Convert request
        let vertex_request = FormatConverter::openai_to_vertex_v2(&openai_request)
            .expect("Should convert request");

        // Convert response
        let openai_response = FormatConverter::vertex_to_openai_v2(
            &vertex_response,
            "chatcmpl-realistic-test",
            "claude-sonnet-4-5@20250929",
            &openai_request,
        ).expect("Should convert response");

        // Validate round-trip
        assert_eq!(openai_response.model, openai_request.model);
        assert_eq!(openai_response.choices[0].message.content, "The capital of France is Paris.");
        assert_eq!(openai_response.usage.total_tokens, 20);

        println!("✅ Realistic scenario test passed!");
    }
}
