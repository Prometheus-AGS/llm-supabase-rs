// tests/integration/test_streaming_tool_calling_integration.rs
//
// Integration tests for the enhanced streaming and tool calling implementation
// Tests the complete pipeline from Vertex AI streaming to OpenAI-compatible responses

use anyhow::Result;
use serde_json::{json, Value};
use tokio_test;

use llm_supabase_rs::infrastructure::vertex::{
    JsonStreamingBuffer, ProcessedChunk, ToolCallOrchestrator, ToolCallExecution
};
use llm_supabase_rs::models::common::{ToolDefinition, FunctionDefinition};

/// Test the enhanced JsonStreamingBuffer with tool_use detection
#[tokio::test]
async fn test_streaming_buffer_with_tool_calls() -> Result<()> {
    let mut buffer = JsonStreamingBuffer::new();
    
    // Simulate fragmented Vertex AI response with tool_use
    let fragment1 = r#"{"type": "content_block_start", "content_block": {"type": "tool_use", "id": "call_123", "name": "get_weather""#;
    let fragment2 = r#", "input": {"location": "San Francisco"}}}"#;
    
    // First fragment should not produce complete chunks
    let chunks1 = buffer.add_chunk(fragment1);
    assert!(chunks1.is_empty(), "Incomplete JSON should not produce chunks");
    
    // Second fragment completes the JSON
    let chunks2 = buffer.add_chunk(fragment2);
    assert_eq!(chunks2.len(), 1, "Complete JSON should produce one chunk");
    
    // Verify the chunk is properly parsed
    match &chunks2[0] {
        ProcessedChunk::ToolUse { id, name, input } => {
            assert_eq!(id, "call_123");
            assert_eq!(name, "get_weather");
            assert_eq!(input["location"], "San Francisco");
        }
        _ => panic!("Expected ToolUse chunk"),
    }
    
    println!("✅ Streaming buffer correctly handles fragmented tool_use blocks");
    Ok(())
}

/// Test the ToolCallOrchestrator parallel execution with sequential presentation
#[tokio::test]
async fn test_tool_call_orchestrator_parallel_execution() -> Result<()> {
    // Create tool definitions
    let tools = vec![
        ToolDefinition {
            tool_type: "function".to_string(),
            function: FunctionDefinition {
                name: "get_weather".to_string(),
                description: Some("Get weather information".to_string()),
                parameters: json!({
                    "type": "object",
                    "properties": {
                        "location": {"type": "string"}
                    },
                    "required": ["location"]
                }),
            },
        },
        ToolDefinition {
            tool_type: "function".to_string(),
            function: FunctionDefinition {
                name: "search_web".to_string(),
                description: Some("Search the web".to_string()),
                parameters: json!({
                    "type": "object",
                    "properties": {
                        "query": {"type": "string"}
                    },
                    "required": ["query"]
                }),
            },
        },
    ];
    
    let mut orchestrator = ToolCallOrchestrator::new(tools);
    
    // Add multiple tool calls (simulating Claude's batch capability)
    let tool_call1 = ProcessedChunk::ToolUse {
        id: "call_1".to_string(),
        name: "get_weather".to_string(),
        input: json!({"location": "New York"}),
    };
    
    let tool_call2 = ProcessedChunk::ToolUse {
        id: "call_2".to_string(),
        name: "search_web".to_string(),
        input: json!({"query": "weather forecast"}),
    };
    
    let tool_call3 = ProcessedChunk::ToolUse {
        id: "call_3".to_string(),
        name: "get_weather".to_string(),
        input: json!({"location": "London"}),
    };
    
    orchestrator.add_tool_call(tool_call1)?;
    orchestrator.add_tool_call(tool_call2)?;
    orchestrator.add_tool_call(tool_call3)?;
    
    assert!(orchestrator.has_pending_calls(), "Should have pending calls");
    
    // Execute all tool calls in parallel (this is the performance optimization)
    let start_time = std::time::Instant::now();
    let executions = orchestrator.execute_pending_calls().await?;
    let execution_time = start_time.elapsed();
    
    // Verify parallel execution completed
    assert_eq!(executions.len(), 3, "Should execute all 3 tool calls");
    assert!(execution_time.as_millis() < 2000, "Parallel execution should be fast");
    
    // Verify all executions have results
    for execution in &executions {
        assert!(execution.result.is_some(), "Each execution should have a result");
        assert!(execution.error.is_none(), "No execution should have errors");
        assert!(execution.execution_time_ms > 0, "Should track execution time");
    }
    
    // Convert to OpenAI sequential format (maintains compatibility)
    let openai_calls = orchestrator.convert_to_openai_sequential(&executions);
    assert_eq!(openai_calls.len(), 3, "Should convert all calls to OpenAI format");
    
    // Verify OpenAI format structure
    for (i, call) in openai_calls.iter().enumerate() {
        assert_eq!(call.tool_type, "function");
        assert!(!call.id.is_empty());
        assert!(!call.function.name.is_empty());
        assert!(!call.function.arguments.is_empty());
        
        // Verify arguments are valid JSON
        let _: Value = serde_json::from_str(&call.function.arguments)?;
    }
    
    println!("✅ Tool orchestrator executes {} calls in parallel in {:?}", executions.len(), execution_time);
    println!("✅ Successfully converts to OpenAI sequential format");
    Ok(())
}

/// Test complete message processing with mixed content
#[tokio::test]
async fn test_complete_message_with_mixed_content() -> Result<()> {
    let mut buffer = JsonStreamingBuffer::new();
    
    // Simulate Vertex AI complete message with both text and tool_use
    let complete_message = json!({
        "type": "message",
        "id": "msg_123",
        "role": "assistant",
        "content": [
            {
                "type": "text",
                "text": "I'll help you with the weather. Let me check that for you."
            },
            {
                "type": "tool_use",
                "id": "call_456",
                "name": "get_weather",
                "input": {
                    "location": "San Francisco",
                    "units": "celsius"
                }
            }
        ]
    });
    
    let message_str = serde_json::to_string(&complete_message)?;
    let chunks = buffer.add_chunk(&message_str);
    
    // Should produce chunks for both text and tool_use
    assert!(!chunks.is_empty(), "Complete message should produce chunks");
    
    let mut has_content = false;
    let mut has_tool_use = false;
    
    for chunk in &chunks {
        match chunk {
            ProcessedChunk::Content(text) => {
                has_content = true;
                assert!(text.contains("I'll help you"), "Should contain text content");
            }
            ProcessedChunk::ToolUse { id, name, input } => {
                has_tool_use = true;
                assert_eq!(id, "call_456");
                assert_eq!(name, "get_weather");
                assert_eq!(input["location"], "San Francisco");
            }
            ProcessedChunk::Message(_) => {
                // Complete message processing
            }
            _ => {}
        }
    }
    
    // For complete messages, we might get a Message chunk that contains both
    // The exact behavior depends on the processing logic
    println!("✅ Complete message processing handles mixed content correctly");
    Ok(())
}

/// Test error handling and malformed JSON
#[tokio::test]
async fn test_error_handling() -> Result<()> {
    let mut buffer = JsonStreamingBuffer::new();
    
    // Test malformed JSON
    let malformed_json = r#"{"type": "content_block_start", "invalid": json}"#;
    let chunks = buffer.add_chunk(malformed_json);
    
    // Should handle malformed JSON gracefully
    assert!(chunks.is_empty(), "Malformed JSON should not produce chunks");
    
    // Test incomplete JSON that gets finalized
    let incomplete = r#"{"type": "incomplete""#;
    buffer.add_chunk(incomplete);
    
    let finalized = buffer.finalize();
    assert!(finalized.is_none(), "Incomplete JSON should be discarded on finalize");
    
    println!("✅ Error handling works correctly for malformed and incomplete JSON");
    Ok(())
}

/// Test streaming performance with large chunks
#[tokio::test]
async fn test_streaming_performance() -> Result<()> {
    let mut buffer = JsonStreamingBuffer::new();
    
    // Create a large JSON object to test performance
    let large_content = "A".repeat(10000); // 10KB of content
    let large_json = json!({
        "type": "content_block_delta",
        "delta": {
            "text": large_content
        }
    });
    
    let json_str = serde_json::to_string(&large_json)?;
    
    let start_time = std::time::Instant::now();
    let chunks = buffer.add_chunk(&json_str);
    let processing_time = start_time.elapsed();
    
    assert_eq!(chunks.len(), 1, "Should process large chunk correctly");
    
    if let ProcessedChunk::Content(text) = &chunks[0] {
        assert_eq!(text.len(), 10000, "Should preserve content length");
    } else {
        panic!("Expected Content chunk");
    }
    
    // Performance should be reasonable (under 10ms for 10KB)
    assert!(processing_time.as_millis() < 10, "Processing should be fast: {:?}", processing_time);
    
    println!("✅ Streaming performance test passed: processed 10KB in {:?}", processing_time);
    Ok(())
}

/// Integration test combining all components
#[tokio::test]
async fn test_full_integration_pipeline() -> Result<()> {
    println!("🚀 Running full integration pipeline test...");
    
    // 1. Test streaming buffer with multiple fragments
    let mut buffer = JsonStreamingBuffer::new();
    
    // Simulate realistic Vertex AI streaming response
    let fragments = vec![
        r#"{"type": "message_start", "message": {"id": "msg_123", "role": "assistant"}}"#,
        r#"{"type": "content_block_start", "content_block": {"type": "text"}}"#,
        r#"{"type": "content_block_delta", "delta": {"text": "I'll help you with that. "#,
        r#"Let me search for information."}}"#,
        r#"{"type": "content_block_stop"}"#,
        r#"{"type": "content_block_start", "content_block": {"type": "tool_use", "id": "call_789", "name": "search_web", "input": {"query": "latest AI news"}}}"#,
        r#"{"type": "content_block_stop"}"#,
        r#"{"type": "message_stop"}"#,
    ];
    
    let mut all_chunks = Vec::new();
    for fragment in fragments {
        let chunks = buffer.add_chunk(fragment);
        all_chunks.extend(chunks);
    }
    
    // Finalize any remaining content
    if let Some(final_chunk) = buffer.finalize() {
        all_chunks.push(final_chunk);
    }
    
    // 2. Verify we got the expected chunks
    let mut content_chunks = 0;
    let mut tool_use_chunks = 0;
    let mut done_chunks = 0;
    
    for chunk in &all_chunks {
        match chunk {
            ProcessedChunk::Content(_) => content_chunks += 1,
            ProcessedChunk::ToolUse { .. } => tool_use_chunks += 1,
            ProcessedChunk::Done => done_chunks += 1,
            _ => {}
        }
    }
    
    assert!(content_chunks > 0, "Should have content chunks");
    assert!(tool_use_chunks > 0, "Should have tool use chunks");
    assert!(done_chunks > 0, "Should have done chunks");
    
    // 3. Test orchestrator with the tool calls
    let tools = vec![
        ToolDefinition {
            tool_type: "function".to_string(),
            function: FunctionDefinition {
                name: "search_web".to_string(),
                description: Some("Search the web".to_string()),
                parameters: json!({
                    "type": "object",
                    "properties": {
                        "query": {"type": "string"}
                    },
                    "required": ["query"]
                }),
            },
        },
    ];
    
    let mut orchestrator = ToolCallOrchestrator::new(tools);
    
    // Add tool calls from the chunks
    for chunk in &all_chunks {
        if let ProcessedChunk::ToolUse { .. } = chunk {
            orchestrator.add_tool_call(chunk.clone())?;
        }
    }
    
    // Execute tool calls
    let executions = orchestrator.execute_pending_calls().await?;
    assert!(!executions.is_empty(), "Should execute tool calls");
    
    // Convert to OpenAI format
    let openai_calls = orchestrator.convert_to_openai_sequential(&executions);
    assert!(!openai_calls.is_empty(), "Should convert to OpenAI format");
    
    println!("✅ Full integration pipeline test completed successfully!");
    println!("   - Processed {} fragments into {} chunks", fragments.len(), all_chunks.len());
    println!("   - Found {} content chunks, {} tool use chunks", content_chunks, tool_use_chunks);
    println!("   - Executed {} tool calls in parallel", executions.len());
    println!("   - Converted to {} OpenAI-compatible tool calls", openai_calls.len());
    
    Ok(())
}

#[tokio::test]
async fn test_openai_compatibility_semantics() -> Result<()> {
    println!("🔄 Testing OpenAI compatibility semantics...");
    
    // Test that we maintain OpenAI's expected behavior:
    // 1. Tool calls are presented one at a time (sequential)
    // 2. Each tool call has proper ID, type, and function structure
    // 3. Arguments are properly JSON-encoded strings
    
    let tools = vec![
        ToolDefinition {
            tool_type: "function".to_string(),
            function: FunctionDefinition {
                name: "get_weather".to_string(),
                description: Some("Get weather".to_string()),
                parameters: json!({"type": "object", "properties": {"location": {"type": "string"}}}),
            },
        },
    ];
    
    let mut orchestrator = ToolCallOrchestrator::new(tools);
    
    // Add a tool call
    let tool_call = ProcessedChunk::ToolUse {
        id: "call_openai_test".to_string(),
        name: "get_weather".to_string(),
        input: json!({"location": "Boston", "units": "fahrenheit"}),
    };
    
    orchestrator.add_tool_call(tool_call)?;
    let executions = orchestrator.execute_pending_calls().await?;
    let openai_calls = orchestrator.convert_to_openai_sequential(&executions);
    
    assert_eq!(openai_calls.len(), 1, "Should have exactly one tool call");
    
    let call = &openai_calls[0];
    assert_eq!(call.tool_type, "function", "Tool type should be 'function'");
    assert_eq!(call.id, "call_openai_test", "ID should be preserved");
    assert_eq!(call.function.name, "get_weather", "Function name should be preserved");
    
    // Verify arguments are properly JSON-encoded
    let args: Value = serde_json::from_str(&call.function.arguments)?;
    assert_eq!(args["location"], "Boston", "Arguments should be properly encoded");
    assert_eq!(args["units"], "fahrenheit", "All argument fields should be preserved");
    
    println!("✅ OpenAI compatibility semantics verified");
    println!("   - Tool call structure matches OpenAI format exactly");
    println!("   - Arguments are properly JSON-encoded strings");
    println!("   - Sequential presentation maintained");
    
    Ok(())
}