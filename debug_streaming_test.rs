// debug_streaming_test.rs
// Debug streaming to see what chunks are being processed

use serde_json::json;
use llm_supabase_rs::{
    infrastructure::vertex::{FormatConverter, VertexStreamChunk, VertexDelta, VertexContent},
    models::{
        request::ChatCompletionRequest,
        common::{ChatMessage, MessageRole},
    },
};

fn main() {
    println!("🔍 Debug Streaming Chunk Processing");
    println!("=====================================");
    
    // Create test streaming request
    let request = ChatCompletionRequest {
        model: "claude-sonnet-4@20250514".to_string(),
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
        max_tokens: Some(50),
        temperature: Some(0.1),
        top_p: None,
        n: None,
        stream: Some(true),
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
    
    // Convert to streaming format
    println!("📤 Converting OpenAI request to Vertex AI streaming format...");
    let vertex_request = FormatConverter::openai_to_vertex_streaming(&request)
        .expect("Should convert to streaming format");
    
    println!("✅ Streaming conversion successful!");
    println!("📋 Stream flag: {}", vertex_request.stream);
    println!("📋 Max tokens: {}", vertex_request.max_tokens);
    
    // Test various chunk types that might come from Vertex AI
    let test_chunks = vec![
        // 1. Message start chunk
        VertexStreamChunk {
            event_type: "message_start".to_string(),
            id: Some("msg_01ABC".to_string()),
            role: Some("assistant".to_string()),
            model: Some("claude-sonnet-4@20250514".to_string()),
            content: None,
            index: None,
            delta: None,
            message: None,
            content_block: None,
            usage: None,
        },
        
        // 2. Content block start
        VertexStreamChunk {
            event_type: "content_block_start".to_string(),
            id: None,
            role: None,
            model: None,
            content: Some(vec![VertexContent {
                content_type: "text".to_string(),
                text: "".to_string(),
            }]),
            index: Some(0),
            delta: None,
            message: None,
            content_block: None,
            usage: None,
        },
        
        // 3. Content block delta with actual text
        VertexStreamChunk {
            event_type: "content_block_delta".to_string(),
            id: None,
            role: None,
            model: None,
            content: None,
            index: Some(0),
            delta: Some(VertexDelta {
                delta_type: "text_delta".to_string(),
                text: Some("1".to_string()),
                stop_reason: None,
            }),
            message: None,
            content_block: None,
            usage: None,
        },
        
        // 4. More content
        VertexStreamChunk {
            event_type: "content_block_delta".to_string(),
            id: None,
            role: None,
            model: None,
            content: None,
            index: Some(0),
            delta: Some(VertexDelta {
                delta_type: "text_delta".to_string(),
                text: Some(", 2".to_string()),
                stop_reason: None,
            }),
            message: None,
            content_block: None,
            usage: None,
        },
        
        // 5. Final content
        VertexStreamChunk {
            event_type: "content_block_delta".to_string(),
            id: None,
            role: None,
            model: None,
            content: None,
            index: Some(0),
            delta: Some(VertexDelta {
                delta_type: "text_delta".to_string(),
                text: Some(", 3".to_string()),
                stop_reason: None,
            }),
            message: None,
            content_block: None,
            usage: None,
        },
        
        // 6. Content block stop
        VertexStreamChunk {
            event_type: "content_block_stop".to_string(),
            id: None,
            role: None,
            model: None,
            content: None,
            index: Some(0),
            delta: None,
            message: None,
            content_block: None,
            usage: None,
        },
        
        // 7. Message delta with finish reason
        VertexStreamChunk {
            event_type: "message_delta".to_string(),
            id: None,
            role: None,
            model: None,
            content: None,
            index: None,
            delta: Some(VertexDelta {
                delta_type: "message_delta".to_string(),
                text: None,
                stop_reason: Some("end_turn".to_string()),
            }),
            message: None,
            content_block: None,
            usage: None,
        },
        
        // 8. Message stop with usage
        VertexStreamChunk {
            event_type: "message_stop".to_string(),
            id: None,
            role: None,
            model: None,
            content: None,
            index: None,
            delta: None,
            message: None,
            content_block: None,
            usage: Some(llm_supabase_rs::infrastructure::vertex::VertexUsage {
                input_tokens: 10,
                output_tokens: 8,
            }),
        },
    ];
    
    println!("\n📦 Processing {} test chunks...", test_chunks.len());
    println!("==========================================");
    
    let mut content_chunks = 0;
    let mut skipped_chunks = 0;
    
    for (i, chunk) in test_chunks.iter().enumerate() {
        println!("\n🔧 Processing chunk {} - Type: '{}'", i + 1, chunk.event_type);
        
        match FormatConverter::vertex_chunk_to_openai_v2(
            chunk,
            "test-streaming-123",
            "claude-sonnet-4@20250514"
        ) {
            Ok(Some(openai_chunk)) => {
                content_chunks += 1;
                println!("✅ Chunk converted successfully!");
                println!("📋 OpenAI chunk ID: {}", openai_chunk.id);
                println!("📋 Choices: {}", openai_chunk.choices.len());
                
                if let Some(choice) = openai_chunk.choices.first() {
                    if !choice.delta.content.is_empty() {
                        println!("📝 Content: '{}'", choice.delta.content);
                    }
                    if let Some(finish_reason) = &choice.finish_reason {
                        println!("🏁 Finish reason: {:?}", finish_reason);
                    }
                }
                
                // Simulate what would be sent to client
                if let Ok(json_str) = serde_json::to_string(&openai_chunk) {
                    println!("📤 SSE Data: data: {}", json_str);
                }
            },
            Ok(None) => {
                skipped_chunks += 1;
                println!("⏭️  Chunk skipped (None returned)");
            },
            Err(e) => {
                println!("❌ Chunk conversion failed: {}", e);
            }
        }
    }
    
    println!("\n📊 SUMMARY");
    println!("==========");
    println!("📦 Total chunks processed: {}", test_chunks.len());
    println!("✅ Content chunks generated: {}", content_chunks);
    println!("⏭️  Chunks skipped: {}", skipped_chunks);
    println!("❌ Failed chunks: {}", test_chunks.len() - content_chunks - skipped_chunks);
    
    if content_chunks > 0 {
        println!("\n🎉 SUCCESS: Content chunks are being generated!");
        println!("💡 The streaming conversion logic is working correctly.");
    } else {
        println!("\n⚠️  WARNING: No content chunks were generated!");
        println!("🔍 This might be why you're only seeing DONE messages.");
    }
    
    println!("\nNow test with a real request to see if chunks contain content...");
}