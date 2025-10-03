// tests/integration/test_adaptive_tool_calling.rs
//
// Integration tests for adaptive OpenAI tool calling compatibility
// Tests legacy function_call vs modern tool_calls array behavior

use anyhow::Result;
use axum::http::HeaderMap;
use serde_json::{json, Value};

use llm_supabase_rs::{
    infrastructure::common::{
        ClientCapabilityDetector, ToolChoiceStrategy, AdaptiveResponseFormatter,
        ToolFormat, StreamingBehavior,
    },
    infrastructure::vertex::{ToolCallOrchestrator, ToolCallExecution},
    models::{
        request::ChatCompletionRequest,
        common::{ChatMessage, MessageRole, ToolDefinition, FunctionDefinition},
    },
};

/// Test client capability detection from request parameters
#[tokio::test]
async fn test_client_capability_detection() -> Result<()> {
    let detector = ClientCapabilityDetector::new();
    
    // Test legacy client detection (functions parameter)
    let mut legacy_request = ChatCompletionRequest::new("claude-4-sonnet", vec![
        ChatMessage::user("Hello")
    ]);
    legacy_request.functions = Some(vec![
        FunctionDefinition {
            name: "get_weather".to_string(),
            description: Some("Get weather".to_string()),
            parameters: Some(json!({"type": "object"})),
        }
    ]);
    
    let headers = HeaderMap::new();
    let capabilities = detector.detect_capabilities(&legacy_request, &headers);
    
    assert_eq!(capabilities.tool_format, ToolFormat::Legacy);
    assert!(!capabilities.supports_parallel_execution);
    println!("✅ Legacy client detection works correctly");
    
    // Test modern client detection (parallel_tool_calls: true)
    let mut modern_request = ChatCompletionRequest::new("claude-4-sonnet", vec![
        ChatMessage::user("Hello")
    ]);
    modern_request.tools = Some(vec![
        ToolDefinition {
            tool_type: "function".to_string(),
            function: FunctionDefinition {
                name: "get_weather".to_string(),
                description: Some("Get weather".to_string()),
                parameters: Some(json!({"type": "object"})),
            },
        }
    ]);
    modern_request.parallel_tool_calls = Some(true);
    
    let capabilities = detector.detect_capabilities(&modern_request, &headers);
    
    assert_eq!(capabilities.tool_format, ToolFormat::Modern);
    assert!(capabilities.supports_parallel_execution);
    println!("✅ Modern client detection works correctly");
    
    Ok(())
}

/// Test tool choice strategy parsing
#[tokio::test]
async fn test_tool_choice_strategy_parsing() -> Result<()> {
    // Test auto strategy
    let mut request = ChatCompletionRequest::new("claude-4-sonnet", vec![]);
    request.tool_choice = Some(json!("auto"));
    
    let strategy = ToolChoiceStrategy::from_request(&request)?;
    assert_eq!(strategy, ToolChoiceStrategy::Auto);
    
    // Test none strategy
    request.tool_choice = Some(json!("none"));
    let strategy = ToolChoiceStrategy::from_request(&request)?;
    assert_eq!(strategy, ToolChoiceStrategy::None);
    
    // Test required strategy
    request.tool_choice = Some(json!("required"));
    let strategy = ToolChoiceStrategy::from_request(&request)?;
    assert_eq!(strategy, ToolChoiceStrategy::Required);
    
    // Test specific tool strategy
    request.tool_choice = Some(json!({
        "type": "function",
        "function": {"name": "get_weather"}
    }));
    let strategy = ToolChoiceStrategy::from_request(&request)?;
    assert_eq!(strategy, ToolChoiceStrategy::Specific { name: "get_weather".to_string() });
    
    println!("✅ Tool choice strategy parsing works for all formats");
    Ok(())
}

/// Test adaptive response formatting for different client types
#[tokio::test]
async fn test_adaptive_response_formatting() -> Result<()> {
    // Create test tool executions
    let executions = vec![
        ToolCallExecution {
            id: "call_1".to_string(),
            name: "get_weather".to_string(),
            input: json!({"location": "San Francisco"}),
            result: Some("Sunny, 22°C".to_string()),
            error: None,
            execution_time_ms: 150,
        },
        ToolCallExecution {
            id: "call_2".to_string(),
            name: "search_web".to_string(),
            input: json!({"query": "weather forecast"}),
            result: Some("Search results...".to_string()),
            error: None,
            execution_time_ms: 200,
        },
    ];
    
    // Test legacy formatting (should return single function_call)
    let legacy_capabilities = llm_supabase_rs::infrastructure::common::ClientCapabilities {
        tool_format: ToolFormat::Legacy,
        supports_parallel_execution: false,
        sdk_version: None,
        streaming_behavior: StreamingBehavior::LegacyStop,
        tool_choice_support: llm_supabase_rs::infrastructure::common::ToolChoiceSupport::Basic,
    };
    
    let mut legacy_formatter = AdaptiveResponseFormatter::new(
        legacy_capabilities,
        ToolChoiceStrategy::Auto,
    );
    
    let legacy_response = legacy_formatter.format_response(
        &executions,
        Some("I'll help with that.".to_string()),
        "req_123",
        "claude-4-sonnet",
    )?;
    
    let legacy_choice = &legacy_response.choices[0];
    assert!(legacy_choice.message.function_call.is_some());
    assert!(legacy_choice.message.tool_calls.is_none());
    
    // Should queue the remaining call
    assert!(legacy_formatter.get_tool_queue().has_pending_calls("req_123"));
    
    println!("✅ Legacy formatting: single function_call with queued remaining calls");
    
    // Test modern formatting (should return tool_calls array)
    let modern_capabilities = llm_supabase_rs::infrastructure::common::ClientCapabilities {
        tool_format: ToolFormat::Modern,
        supports_parallel_execution: true,
        sdk_version: None,
        streaming_behavior: StreamingBehavior::ModernBatch,
        tool_choice_support: llm_supabase_rs::infrastructure::common::ToolChoiceSupport::Full,
    };
    
    let mut modern_formatter = AdaptiveResponseFormatter::new(
        modern_capabilities,
        ToolChoiceStrategy::Auto,
    );
    
    let modern_response = modern_formatter.format_response(
        &executions,
        Some("I'll help with that.".to_string()),
        "req_456",
        "claude-4-sonnet",
    )?;
    
    let modern_choice = &modern_response.choices[0];
    assert!(modern_choice.message.function_call.is_none());
    assert!(modern_choice.message.tool_calls.is_some());
    assert_eq!(modern_choice.message.tool_calls.as_ref().unwrap().len(), 2);
    
    println!("✅ Modern formatting: tool_calls array with all calls");
    
    Ok(())
}

/// Test tool choice enforcement scenarios
#[tokio::test]
async fn test_tool_choice_enforcement() -> Result<()> {
    let capabilities = llm_supabase_rs::infrastructure::common::ClientCapabilities::default();
    
    // Test tool_choice: "none" - should ignore tool executions
    let mut none_formatter = AdaptiveResponseFormatter::new(
        capabilities.clone(),
        ToolChoiceStrategy::None,
    );
    
    let executions = vec![
        ToolCallExecution {
            id: "call_1".to_string(),
            name: "get_weather".to_string(),
            input: json!({"location": "Boston"}),
            result: Some("Cold, 5°C".to_string()),
            error: None,
            execution_time_ms: 100,
        }
    ];
    
    let none_response = none_formatter.format_response(
        &executions,
        Some("Just text response.".to_string()),
        "req_none",
        "claude-4-sonnet",
    )?;
    
    let none_choice = &none_response.choices[0];
    assert!(none_choice.message.function_call.is_none());
    assert!(none_choice.message.tool_calls.is_none());
    assert_eq!(none_choice.message.content, "Just text response.");
    
    println!("✅ tool_choice: 'none' correctly ignores tool executions");
    
    // Test tool_choice: "required" - should error when no tools executed
    let mut required_formatter = AdaptiveResponseFormatter::new(
        capabilities.clone(),
        ToolChoiceStrategy::Required,
    );
    
    let required_result = required_formatter.format_response(
        &[], // No tool executions
        Some("No tools used.".to_string()),
        "req_required",
        "claude-4-sonnet",
    );
    
    assert!(required_result.is_err());
    assert!(required_result.unwrap_err().to_string().contains("required"));
    
    println!("✅ tool_choice: 'required' correctly validates tool usage");
    
    // Test tool_choice: specific tool
    let mut specific_formatter = AdaptiveResponseFormatter::new(
        capabilities,
        ToolChoiceStrategy::Specific { name: "get_weather".to_string() },
    );
    
    let specific_response = specific_formatter.format_response(
        &executions,
        Some("Weather checked.".to_string()),
        "req_specific",
        "claude-4-sonnet",
    )?;
    
    // Should succeed since get_weather was called
    assert!(specific_response.choices[0].message.function_call.is_some());
    
    println!("✅ tool_choice: specific tool correctly validates required tool usage");
    
    Ok(())
}

/// Test parallel execution with sequential presentation
#[tokio::test]
async fn test_parallel_execution_sequential_presentation() -> Result<()> {
    println!("🚀 Testing parallel execution with sequential presentation...");
    
    // Create multiple tool definitions
    let tools = vec![
        ToolDefinition {
            tool_type: "function".to_string(),
            function: FunctionDefinition {
                name: "get_weather".to_string(),
                description: Some("Get weather".to_string()),
                parameters: Some(json!({"type": "object", "properties": {"location": {"type": "string"}}})),
            },
        },
        ToolDefinition {
            tool_type: "function".to_string(),
            function: FunctionDefinition {
                name: "search_web".to_string(),
                description: Some("Search web".to_string()),
                parameters: Some(json!({"type": "object", "properties": {"query": {"type": "string"}}})),
            },
        },
        ToolDefinition {
            tool_type: "function".to_string(),
            function: FunctionDefinition {
                name: "get_time".to_string(),
                description: Some("Get time".to_string()),
                parameters: Some(json!({"type": "object", "properties": {"timezone": {"type": "string"}}})),
            },
        },
    ];
    
    // Create orchestrator and add multiple tool calls
    let mut orchestrator = ToolCallOrchestrator::new(tools);
    
    let tool_calls = vec![
        llm_supabase_rs::infrastructure::vertex::ProcessedChunk::ToolUse {
            id: "call_1".to_string(),
            name: "get_weather".to_string(),
            input: json!({"location": "New York"}),
        },
        llm_supabase_rs::infrastructure::vertex::ProcessedChunk::ToolUse {
            id: "call_2".to_string(),
            name: "search_web".to_string(),
            input: json!({"query": "NYC weather"}),
        },
        llm_supabase_rs::infrastructure::vertex::ProcessedChunk::ToolUse {
            id: "call_3".to_string(),
            name: "get_time".to_string(),
            input: json!({"timezone": "America/New_York"}),
        },
    ];
    
    // Add all tool calls
    for tool_call in tool_calls {
        orchestrator.add_tool_call(tool_call)?;
    }
    
    // Execute in parallel (performance optimization)
    let start_time = std::time::Instant::now();
    let executions = orchestrator.execute_pending_calls().await?;
    let parallel_execution_time = start_time.elapsed();
    
    assert_eq!(executions.len(), 3);
    assert!(parallel_execution_time.as_millis() < 1000); // Should be fast due to parallel execution
    
    println!("✅ Parallel execution completed {} tools in {:?}", executions.len(), parallel_execution_time);
    
    // Test sequential presentation for legacy client
    let legacy_capabilities = llm_supabase_rs::infrastructure::common::ClientCapabilities {
        tool_format: ToolFormat::Legacy,
        supports_parallel_execution: false,
        sdk_version: None,
        streaming_behavior: StreamingBehavior::LegacyStop,
        tool_choice_support: llm_supabase_rs::infrastructure::common::ToolChoiceSupport::Basic,
    };
    
    let mut legacy_formatter = AdaptiveResponseFormatter::new(
        legacy_capabilities,
        ToolChoiceStrategy::Auto,
    );
    
    let legacy_response = legacy_formatter.format_response(
        &executions,
        Some("I'll help with that.".to_string()),
        "req_legacy",
        "claude-4-sonnet",
    )?;
    
    // Should return single function_call
    let legacy_choice = &legacy_response.choices[0];
    assert!(legacy_choice.message.function_call.is_some());
    assert!(legacy_choice.message.tool_calls.is_none());
    
    // Should queue remaining calls
    assert!(legacy_formatter.get_tool_queue().has_pending_calls("req_legacy"));
    
    // Test modern presentation for modern client
    let modern_capabilities = llm_supabase_rs::infrastructure::common::ClientCapabilities {
        tool_format: ToolFormat::Modern,
        supports_parallel_execution: true,
        sdk_version: None,
        streaming_behavior: StreamingBehavior::ModernBatch,
        tool_choice_support: llm_supabase_rs::infrastructure::common::ToolChoiceSupport::Full,
    };
    
    let mut modern_formatter = AdaptiveResponseFormatter::new(
        modern_capabilities,
        ToolChoiceStrategy::Auto,
    );
    
    let modern_response = modern_formatter.format_response(
        &executions,
        Some("I'll help with that.".to_string()),
        "req_modern",
        "claude-4-sonnet",
    )?;
    
    // Should return tool_calls array
    let modern_choice = &modern_response.choices[0];
    assert!(modern_choice.message.function_call.is_none());
    assert!(modern_choice.message.tool_calls.is_some());
    assert_eq!(modern_choice.message.tool_calls.as_ref().unwrap().len(), 3);
    
    println!("✅ Sequential presentation for legacy, batch presentation for modern");
    println!("✅ Performance optimization: parallel execution with adaptive presentation");
    
    Ok(())
}

/// Test User-Agent based detection
#[tokio::test]
async fn test_user_agent_detection() -> Result<()> {
    let detector = ClientCapabilityDetector::new();
    let request = ChatCompletionRequest::new("claude-4-sonnet", vec![]);
    
    // Test OpenAI Python SDK v1.x (modern)
    let mut headers = HeaderMap::new();
    headers.insert("user-agent", "openai-python/1.35.0".parse().unwrap());
    
    let capabilities = detector.detect_capabilities(&request, &headers);
    assert_eq!(capabilities.tool_format, ToolFormat::Modern);
    assert!(capabilities.supports_parallel_execution);
    
    // Test OpenAI Python SDK v0.x (legacy)
    headers.clear();
    headers.insert("user-agent", "openai-python/0.28.0".parse().unwrap());
    
    let capabilities = detector.detect_capabilities(&request, &headers);
    assert_eq!(capabilities.tool_format, ToolFormat::Legacy);
    assert!(!capabilities.supports_parallel_execution);
    
    println!("✅ User-Agent detection correctly identifies SDK versions");
    Ok(())
}

/// Test complete adaptive workflow
#[tokio::test]
async fn test_complete_adaptive_workflow() -> Result<()> {
    println!("🔄 Testing complete adaptive workflow...");
    
    // Simulate a request that would trigger multiple tool calls
    let mut request = ChatCompletionRequest::new("claude-4-sonnet", vec![
        ChatMessage::user("Get weather for NYC and Boston, then search for travel info between them")
    ]);
    
    request.tools = Some(vec![
        ToolDefinition {
            tool_type: "function".to_string(),
            function: FunctionDefinition {
                name: "get_weather".to_string(),
                description: Some("Get weather information".to_string()),
                parameters: Some(json!({
                    "type": "object",
                    "properties": {
                        "location": {"type": "string"}
                    },
                    "required": ["location"]
                })),
            },
        },
        ToolDefinition {
            tool_type: "function".to_string(),
            function: FunctionDefinition {
                name: "search_travel".to_string(),
                description: Some("Search travel information".to_string()),
                parameters: Some(json!({
                    "type": "object",
                    "properties": {
                        "from": {"type": "string"},
                        "to": {"type": "string"}
                    },
                    "required": ["from", "to"]
                })),
            },
        },
    ]);
    
    // Test with different client types
    let test_scenarios = vec![
        ("Legacy Client", ToolFormat::Legacy, false),
        ("Modern Client", ToolFormat::Modern, true),
        ("Adaptive Client", ToolFormat::Adaptive, true),
    ];
    
    for (scenario_name, tool_format, parallel_support) in test_scenarios {
        println!("Testing scenario: {}", scenario_name);
        
        let capabilities = llm_supabase_rs::infrastructure::common::ClientCapabilities {
            tool_format,
            supports_parallel_execution: parallel_support,
            sdk_version: None,
            streaming_behavior: if parallel_support { 
                StreamingBehavior::ModernBatch 
            } else { 
                StreamingBehavior::LegacyStop 
            },
            tool_choice_support: llm_supabase_rs::infrastructure::common::ToolChoiceSupport::Full,
        };
        
        // Simulate tool executions (would come from Claude)
        let executions = vec![
            ToolCallExecution {
                id: "call_nyc".to_string(),
                name: "get_weather".to_string(),
                input: json!({"location": "New York"}),
                result: Some("NYC: 18°C, cloudy".to_string()),
                error: None,
                execution_time_ms: 120,
            },
            ToolCallExecution {
                id: "call_boston".to_string(),
                name: "get_weather".to_string(),
                input: json!({"location": "Boston"}),
                result: Some("Boston: 15°C, rainy".to_string()),
                error: None,
                execution_time_ms: 130,
            },
            ToolCallExecution {
                id: "call_travel".to_string(),
                name: "search_travel".to_string(),
                input: json!({"from": "New York", "to": "Boston"}),
                result: Some("Multiple flights and trains available".to_string()),
                error: None,
                execution_time_ms: 200,
            },
        ];
        
        let mut formatter = AdaptiveResponseFormatter::new(capabilities, ToolChoiceStrategy::Auto);
        
        let response = formatter.format_response(
            &executions,
            Some("I'll get that information for you.".to_string()),
            &format!("req_{}", scenario_name.replace(" ", "_").to_lowercase()),
            "claude-4-sonnet",
        )?;
        
        let choice = &response.choices[0];
        
        match tool_format {
            ToolFormat::Legacy => {
                assert!(choice.message.function_call.is_some());
                assert!(choice.message.tool_calls.is_none());
                println!("  ✅ Legacy: Single function_call returned");
            }
            ToolFormat::Modern => {
                assert!(choice.message.function_call.is_none());
                assert!(choice.message.tool_calls.is_some());
                assert_eq!(choice.message.tool_calls.as_ref().unwrap().len(), 3);
                println!("  ✅ Modern: tool_calls array with all {} calls", executions.len());
            }
            ToolFormat::Adaptive => {
                // Should choose modern format for multiple calls
                assert!(choice.message.tool_calls.is_some());
                println!("  ✅ Adaptive: Chose modern format for multiple calls");
            }
        }
    }
    
    println!("🎉 Complete adaptive workflow test passed for all client types!");
    Ok(())
}

/// Test error handling scenarios
#[tokio::test]
async fn test_error_handling_scenarios() -> Result<()> {
    let capabilities = llm_supabase_rs::infrastructure::common::ClientCapabilities::default();
    
    // Test invalid tool_choice parameter
    let mut request = ChatCompletionRequest::new("claude-4-sonnet", vec![]);
    request.tool_choice = Some(json!("invalid_choice"));
    
    let result = ToolChoiceStrategy::from_request(&request);
    assert!(result.is_err());
    
    // Test tool_choice: required with no tools defined
    request.tool_choice = Some(json!("required"));
    request.tools = None;
    
    let strategy = ToolChoiceStrategy::from_request(&request)?;
    let validation_result = strategy.validate_against_tools(&request.tools);
    assert!(validation_result.is_err());
    
    // Test tool_choice: specific with undefined tool
    request.tool_choice = Some(json!({
        "type": "function",
        "function": {"name": "undefined_tool"}
    }));
    request.tools = Some(vec![
        ToolDefinition {
            tool_type: "function".to_string(),
            function: FunctionDefinition {
                name: "get_weather".to_string(),
                description: Some("Get weather".to_string()),
                parameters: Some(json!({"type": "object"})),
            },
        }
    ]);
    
    let strategy = ToolChoiceStrategy::from_request(&request)?;
    let validation_result = strategy.validate_against_tools(&request.tools);
    assert!(validation_result.is_err());
    
    println!("✅ Error handling works correctly for all invalid scenarios");
    Ok(())
}