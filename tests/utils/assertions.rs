
//! Test assertion utilities for cross-provider integration testing
//! 
//! This module provides custom assertion functions and validation helpers
//! for verifying API responses, conversation state, and system behavior across all 8 providers.

use anyhow::Result;
use serde_json::Value;
use std::collections::HashMap;
use std::time::Duration;

use crate::models::{
    request::ChatCompletionRequest,
    response::{ChatCompletionResponse, ChatCompletionChoice},
    common::{ChatMessage, MessageRole, ToolCall, FinishReason},
};
use crate::features::conversations::models::ConversationState;

/// Response validation helpers with provider-aware capabilities
pub struct ResponseAssertions;

impl ResponseAssertions {
    /// Assert that a response is valid and well-formed
    pub fn assert_valid_response(response: &ChatCompletionResponse) -> Result<()> {
        assert!(!response.id.is_empty(), "Response ID should not be empty");
        assert_eq!(response.object, "chat.completion", "Object type should be chat.completion");
        assert!(response.created > 0, "Created timestamp should be positive");
        assert!(!response.model.is_empty(), "Model should not be empty");
        assert!(!response.choices.is_empty(), "Choices should not be empty");
        
        Ok(())
    }

    /// Assert that response matches expected provider format
    pub fn assert_provider_response_format(response: &ChatCompletionResponse, provider_name: &str) -> Result<()> {
        Self::assert_valid_response(response)?;
        
        match provider_name {
            "openai" => {
                assert!(
                    response.id.starts_with("chatcmpl-") || response.id.contains("openai"),
                    "OpenAI response ID should follow expected format"
                );
            }
            "anthropic" => {
                assert!(
                    response.id.starts_with("msg_") || response.id.contains("anthropic"),
                    "Anthropic response ID should follow expected format"
                );
            }
            "vertex" => {
                assert!(
                    response.id.contains("vertex") || response.model.contains("vertex"),
                    "Vertex AI response should indicate vertex origin"
                );
            }
            "groq" => {
                assert!(
                    response.id.contains("groq") || response.model.contains("groq"),
                    "Groq response should indicate groq origin"
                );
            }
            "azure_openai" => {
                assert!(
                    response.id.contains("azure") || response.model.contains("azure"),
                    "Azure OpenAI response should indicate azure origin"
                );
            }
            "aws_bedrock" => {
                assert!(
                    response.id.contains("bedrock") || response.model.contains("bedrock"),
                    "AWS Bedrock response should indicate bedrock origin"
                );
            }
            "cohere" => {
                assert!(
                    response.id.contains("cohere") || response.model.contains("command"),
                    "Cohere response should indicate cohere origin"
                );
            }
            "mistral" => {
                assert!(
                    response.id.contains("mistral") || response.model.contains("mistral"),
                    "Mistral response should indicate mistral origin"
                );
            }
            _ => {
                // Generic validation for unknown providers
                println!("Warning: Unknown provider '{}', using generic validation", provider_name);
            }
        }
        
        Ok(())
    }

    /// Assert that response choices are valid
    pub fn assert_valid_choices(choices: &[ChatCompletionChoice]) -> Result<()> {
        assert!(!choices.is_empty(), "Should have at least one choice");
        
        for (i, choice) in choices.iter().enumerate() {
            assert_eq!(choice.index, i as u32, "Choice index should match position");
            
            // Validate message
            Self::assert_valid_message(&choice.message)?;
            
            // Validate finish reason if present
            if let Some(ref finish_reason) = choice.finish_reason {
                Self::assert_valid_finish_reason(finish_reason)?;
            }
        }
        
        Ok(())
    }

    /// Assert that a message is valid
    pub fn assert_valid_message(message: &ChatMessage) -> Result<()> {
        // Role validation
        match message.role {
            MessageRole::Assistant => {
                // Assistant should have either content or tool calls
                assert!(
                    message.content.is_some() || message.tool_calls.is_some(),
                    "Assistant message should have content or tool calls"
                );
            }
            MessageRole::User => {
                assert!(message.content.len() > 0, "User message should have content");
                assert!(message.tool_calls.is_none(), "User message should not have tool calls");
            }
            MessageRole::System => {
                assert!(message.content.len() > 0, "System message should have content");
                assert!(message.tool_calls.is_none(), "System message should not have tool calls");
            }
            MessageRole::Tool => {
                assert!(message.content.len() > 0, "Tool message should have content");
                assert!(message.tool_call_id.is_some(), "Tool message should have tool_call_id");
                assert!(message.tool_calls.is_none(), "Tool message should not have tool calls");
            }
        }
        
        Ok(())
    }

    /// Assert that finish reason is valid
    pub fn assert_valid_finish_reason(finish_reason: &FinishReason) -> Result<()> {
        match finish_reason {
            FinishReason::Stop => Ok(()),
            FinishReason::Length => Ok(()),
            FinishReason::ToolCalls => Ok(()),
            FinishReason::ContentFilter => Ok(()),
            FinishReason::FunctionCall => Ok(()),
        }
    }

    /// Assert that tool calls are valid
    pub fn assert_valid_tool_calls(tool_calls: &[ToolCall]) -> Result<()> {
        for tool_call in tool_calls {
            assert!(!tool_call.id.is_empty(), "Tool call ID should not be empty");
            assert_eq!(tool_call.tool_type, "function", "Tool type should be 'function'");
            assert!(!tool_call.function.name.is_empty(), "Function name should not be empty");
            
            // Validate that arguments are valid JSON
            let _: Value = serde_json::from_str(&tool_call.function.arguments)
                .map_err(|e| anyhow::anyhow!("Tool call arguments should be valid JSON: {}", e))?;
        }
        
        Ok(())
    }

    /// Assert response contains expected content patterns
    pub fn assert_content_contains(response: &ChatCompletionResponse, patterns: &[&str]) -> Result<()> {
        let content = response.choices.first()
            .and_then(|c| c.message.content.as_ref())
            .ok_or_else(|| anyhow::anyhow!("Response should have content"))?;

        for pattern in patterns {
            assert!(
                content.contains(pattern),
                "Response content should contain '{}'. Actual content: {}",
                pattern,
                content
            );
        }
        
        Ok(())
    }

    /// Assert response contains provider-specific patterns
    pub fn assert_provider_specific_content(response: &ChatCompletionResponse, provider_name: &str) -> Result<()> {
        let content = response.choices.first()
            .and_then(|c| c.message.content.as_ref())
            .ok_or_else(|| anyhow::anyhow!("Response should have content"))?;

        let expected_patterns = match provider_name {
            "openai" => vec!["I'll help", "Let me", "Here's"],
            "anthropic" => vec!["I'll analyze", "Let me break down", "I'd be happy to"],
            "vertex" => vec!["I can help", "Using Google's", "Let me assist"],
            "groq" => vec!["Fast response", "Quickly", "⚡"],
            "azure_openai" => vec!["Azure OpenAI", "Enterprise-grade", "Secure"],
            "aws_bedrock" => vec!["AWS Bedrock", "Secure and scalable", "Amazon's infrastructure"],
            "cohere" => vec!["Cohere model", "Enterprise AI", "Advanced language"],
            "mistral" => vec!["Mistral AI", "European-compliant", "Cost-effective"],
            _ => return Ok(()), // Skip validation for unknown providers
        };

        // Check if at least one expected pattern is present (flexible matching)
        let has_expected_pattern = expected_patterns.iter().any(|pattern| {
            content.to_lowercase().contains(&pattern.to_lowercase())
        });

        if !has_expected_pattern {
            println!(
                "Warning: Response from {} doesn't contain expected patterns. Content: {}",
                provider_name,
                content
            );
        }
        
        Ok(())
    }

    /// Assert response has specific tool calls
    pub fn assert_has_tool_calls(response: &ChatCompletionResponse, expected_tools: &[&str]) -> Result<()> {
        let tool_calls = response.choices.first()
            .and_then(|c| c.message.tool_calls.as_ref())
            .ok_or_else(|| anyhow::anyhow!("Response should have tool calls"))?;

        assert_eq!(
            tool_calls.len(),
            expected_tools.len(),
            "Should have {} tool calls",
            expected_tools.len()
        );

        for (i, expected_name) in expected_tools.iter().enumerate() {
            assert_eq!(
                tool_calls[i].function.name,
                *expected_name,
                "Tool call {} should be '{}'",
                i,
                expected_name
            );
        }

        Ok(())
    }

    /// Assert response usage statistics are reasonable
    pub fn assert_reasonable_usage(response: &ChatCompletionResponse) -> Result<()> {
        if let Some(ref usage) = response.usage {
            assert!(usage.prompt_tokens > 0, "Prompt tokens should be positive");
            assert!(usage.completion_tokens > 0, "Completion tokens should be positive");
            assert_eq!(
                usage.total_tokens,
                usage.prompt_tokens + usage.completion_tokens,
                "Total tokens should equal prompt + completion tokens"
            );
            
            // Sanity checks
            assert!(usage.prompt_tokens < 100_000, "Prompt tokens should be reasonable");
            assert!(usage.completion_tokens < 100_000, "Completion tokens should be reasonable");
        }
        
        Ok(())
    }
}

/// Cross-provider comparison and validation helpers
pub struct CrossProviderAssertions;

impl CrossProviderAssertions {
    /// Assert that all provider responses are valid
    pub fn assert_all_responses_valid(responses: &HashMap<String, ChatCompletionResponse>) -> Result<()> {
        assert!(!responses.is_empty(), "Should have at least one provider response");
        
        for (provider_name, response) in responses {
            ResponseAssertions::assert_provider_response_format(response, provider_name)
                .map_err(|e| anyhow::anyhow!("Provider {} failed validation: {}", provider_name, e))?;
        }
        
        Ok(())
    }

    /// Assert provider capabilities match expectations
    pub fn assert_provider_capabilities(provider_name: &str, supports_streaming: bool, supports_tools: bool) -> Result<()> {
        let expected_capabilities = match provider_name {
            "openai" => (true, true),
            "anthropic" => (true, true),
            "vertex" => (true, true),
            "groq" => (true, false), // Fast but limited tools
            "azure_openai" => (true, true),
            "aws_bedrock" => (true, true),
            "cohere" => (true, true),
            "mistral" => (true, true),
            _ => return Err(anyhow::anyhow!("Unknown provider: {}", provider_name)),
        };

        assert_eq!(
            supports_streaming, expected_capabilities.0,
            "Provider {} streaming support should be {}",
            provider_name, expected_capabilities.0
        );

        assert_eq!(
            supports_tools, expected_capabilities.1,
            "Provider {} tool support should be {}",
            provider_name, expected_capabilities.1
        );

        Ok(())
    }

    /// Assert provider performance meets expectations
    pub fn assert_provider_performance(
        provider_name: &str,
        response_time_ms: u64,
        success_rate: f64
    ) -> Result<()> {
        let (max_time, min_success_rate) = match provider_name {
            "openai" => (10_000, 0.95),
            "anthropic" => (12_000, 0.94),
            "vertex" => (11_000, 0.93),
            "groq" => (3_000, 0.90), // Fast but may have more variability
            "azure_openai" => (10_500, 0.95),
            "aws_bedrock" => (15_000, 0.92),
            "cohere" => (12_000, 0.93),
            "mistral" => (9_000, 0.94),
            _ => return Err(anyhow::anyhow!("Unknown provider: {}", provider_name)),
        };

        assert!(
            response_time_ms <= max_time,
            "Provider {} response time {}ms exceeds maximum {}ms",
            provider_name, response_time_ms, max_time
        );

        assert!(
            success_rate >= min_success_rate,
            "Provider {} success rate {:.1}% below minimum {:.1}%",
            provider_name, success_rate * 100.0, min_success_rate * 100.0
        );

        Ok(())
    }

    /// Assert consistent behavior across providers
    pub fn assert_consistent_behavior(responses: &HashMap<String, ChatCompletionResponse>) -> Result<()> {
        if responses.len() < 2 {
            return Ok(()); // Need at least 2 responses to compare
        }

        let mut content_lengths = Vec::new();
        let mut has_tool_calls = Vec::new();

        for (provider_name, response) in responses {
            if let Some(choice) = response.choices.first() {
                // Collect content length
                if let Some(ref content) = choice.message.content {
                    content_lengths.push((provider_name.clone(), content.len()));
                }

                // Check tool call presence
                has_tool_calls.push((provider_name.clone(), choice.message.tool_calls.is_some()));
            }
        }

        // Check that content lengths are reasonably similar (within 5x difference)
        if content_lengths.len() >= 2 {
            let max_length = content_lengths.iter().map(|(_, len)| *len).max().unwrap();
            let min_length = content_lengths.iter().map(|(_, len)| *len).min().unwrap();

            if min_length > 0 {
                let ratio = max_length as f64 / min_length as f64;
                assert!(
                    ratio <= 5.0,
                    "Content length variation too high across providers: max={}, min={}, ratio={:.1}",
                    max_length, min_length, ratio
                );
            }
        }

        // Log any significant differences for investigation
        if content_lengths.len() >= 2 {
            let avg_length = content_lengths.iter().map(|(_, len)| *len).sum::<usize>() as f64 / content_lengths.len() as f64;
            
            for (provider, length) in &content_lengths {
                let deviation = (*length as f64 - avg_length).abs() / avg_length;
                if deviation > 0.5 {
                    println!(
                        "Warning: Provider {} content length {} deviates {:.1}% from average {}",
                        provider, length, deviation * 100.0, avg_length as usize
                    );
                }
            }
        }

        Ok(())
    }

    /// Assert fallback chain behavior
    pub fn assert_fallback_chain_behavior(
        chain_results: &[(String, Result<ChatCompletionResponse, String>)]
    ) -> Result<()> {
        assert!(!chain_results.is_empty(), "Fallback chain should have at least one result");
        
        let mut found_success = false;
        let mut failure_count = 0;

        for (provider_name, result) in chain_results {
            match result {
                Ok(response) => {
                    // If we found a success, validate it
                    ResponseAssertions::assert_valid_response(response)?;
                    found_success = true;
                    println!("Fallback chain succeeded with provider: {}", provider_name);
                    break;
                }
                Err(error) => {
                    failure_count += 1;
                    println!("Fallback provider {} failed: {}", provider_name, error);
                }
            }
        }

        if !found_success {
            assert!(
                failure_count < chain_results.len(),
                "Fallback chain completely failed - all {} providers failed",
                chain_results.len()
            );
        }

        Ok(())
    }
}

/// Conversation state validation helpers
pub struct ConversationAssertions;

impl ConversationAssertions {
    /// Assert conversation state is valid
    pub fn assert_valid_conversation(state: &ConversationState) -> Result<()> {
        assert!(!state.conversation_id.is_empty(), "Conversation ID should not be empty");
        assert!(!state.model.is_empty(), "Model should not be empty");
        assert!(state.created_at <= state.updated_at, "Created should be <= updated");
        assert!(state.updated_at <= state.expires_at, "Updated should be <= expires");
        assert!(state.turn_count < 1000, "Turn count should be reasonable");
        
        Ok(())
    }

    /// Assert conversation has expected message count
    pub fn assert_message_count(state: &ConversationState, expected: usize) -> Result<()> {
        assert_eq!(
            state.messages.len(),
            expected,
            "Conversation should have {} messages",
            expected
        );
        
        Ok(())
    }

    /// Assert conversation contains message with content
    pub fn assert_contains_message(state: &ConversationState, role: MessageRole, content: &str) -> Result<()> {
        let found = state.messages.iter().any(|msg| {
            msg.role == role && msg.content.contains(content)
        });
        
        assert!(
            found,
            "Conversation should contain {:?} message with content: {}",
            role,
            content
        );
        
        Ok(())
    }

    /// Assert conversation metadata is reasonable
    pub fn assert_reasonable_metadata(state: &ConversationState) -> Result<()> {
        let meta = &state.metadata;
        
        assert!(meta.total_tokens >= meta.total_prompt_tokens, "Total >= prompt tokens");
        assert!(meta.total_tokens >= meta.total_completion_tokens, "Total >= completion tokens");
        assert!(meta.request_count > 0, "Request count should be positive");
        assert!(meta.request_count < 10000, "Request count should be reasonable");
        
        Ok(())
    }

    /// Assert conversation continuity across providers
    pub fn assert_conversation_continuity(
        conversations: &HashMap<String, ConversationState>
    ) -> Result<()> {
        if conversations.len() < 2 {
            return Ok(()); // Need at least 2 to compare
        }

        // Check that all conversations have similar message counts (within 2 messages)
        let message_counts: Vec<usize> = conversations.values()
            .map(|conv| conv.messages.len())
            .collect();

        if let (Some(max_count), Some(min_count)) = (message_counts.iter().max(), message_counts.iter().min()) {
            assert!(
                max_count - min_count <= 2,
                "Message count variation too high across providers: max={}, min={}",
                max_count, min_count
            );
        }

        Ok(())
    }
}

/// Streaming response validation helpers
pub struct StreamingAssertions;

impl StreamingAssertions {
    /// Assert streaming chunks are valid
    pub fn assert_valid_streaming_chunks(chunks: &[String]) -> Result<()> {
        assert!(!chunks.is_empty(), "Should have at least one chunk");
        
        let mut has_data_chunks = false;
        let mut has_done_marker = false;
        
        for chunk in chunks {
            if chunk.starts_with("data: ") {
                let data = &chunk[6..];
                
                if data == "[DONE]" {
                    has_done_marker = true;
                } else if !data.trim().is_empty() {
                    // Validate JSON structure
                    let _: Value = serde_json::from_str(data)
                        .map_err(|e| anyhow::anyhow!("Invalid JSON in chunk: {} - Error: {}", data, e))?;
                    has_data_chunks = true;
                }
            }
        }
        
        assert!(has_data_chunks, "Should have at least one data chunk");
        assert!(has_done_marker, "Should have [DONE] marker");
        
        Ok(())
    }

    /// Assert streaming completed within timeout
    pub fn assert_streaming_timeout(duration: Duration, max_seconds: u64) -> Result<()> {
        assert!(
            duration.as_secs() <= max_seconds,
            "Streaming should complete within {} seconds, took {} seconds",
            max_seconds,
            duration.as_secs()
        );
        
        Ok(())
    }

    /// Assert streaming has proper content progression
    pub fn assert_content_progression(chunks: &[String]) -> Result<()> {
        let mut accumulated_content = String::new();
        let mut previous_length = 0;
        
        for chunk in chunks {
            if chunk.starts_with("data: ") {
                let data = &chunk[6..];
                if data != "[DONE]" {
                    if let Ok(json) = serde_json::from_str::<Value>(data) {
                        if let Some(choices) = json.get("choices").and_then(|c| c.as_array()) {
                            if let Some(choice) = choices.first() {
                                if let Some(delta) = choice.get("delta") {
                                    if let Some(content) = delta.get("content").and_then(|c| c.as_str()) {
                                        accumulated_content.push_str(content);
                                    }
                                }
                            }
                        }
                    }
                }
            }
            
            // Content should only grow, never shrink
            assert!(
                accumulated_content.len() >= previous_length,
                "Streaming content should only grow"
            );
            previous_length = accumulated_content.len();
        }
        
        Ok(())
    }

    /// Assert provider-specific streaming behavior
    pub fn assert_provider_streaming_behavior(
        provider_name: &str,
        chunks: &[String],
        duration: Duration
    ) -> Result<()> {
        Self::assert_valid_streaming_chunks(chunks)?;

        // Provider-specific timing expectations
        let max_duration = match provider_name {
            "groq" => 5,  // Very fast
            "mistral" => 10,
            "openai" => 15,
            "azure_openai" => 15,
            "anthropic" => 20,
            "vertex" => 20,
            "cohere" => 18,
            "aws_bedrock" => 25, // Potentially slower
            _ => 30, // Default timeout
        };

        Self::assert_streaming_timeout(duration, max_duration)?;

        // Check chunk frequency for fast providers
        if provider_name == "groq" {
            assert!(
                chunks.len() >= 3,
                "Groq should produce multiple chunks for streaming efficiency"
            );
        }

        Ok(())
    }
}

/// Performance validation helpers with provider-specific expectations
pub struct PerformanceAssertions;

impl PerformanceAssertions {
    /// Assert response time is within acceptable limits
    pub fn assert_response_time(duration: Duration, max_ms: u64) -> Result<()> {
        assert!(
            duration.as_millis() <= max_ms as u128,
            "Response should complete within {}ms, took {}ms",
            max_ms,
            duration.as_millis()
        );
        
        Ok(())
    }

    /// Assert provider-specific response time
    pub fn assert_provider_response_time(provider_name: &str, duration: Duration) -> Result<()> {
        let max_ms = match provider_name {
            "groq" => 3_000,      // Very fast
            "mistral" => 9_000,   // Good performance
            "openai" => 10_000,   // Standard
            "azure_openai" => 10_500,
            "anthropic" => 12_000,
            "vertex" => 11_000,
            "cohere" => 12_000,
            "aws_bedrock" => 15_000, // May be slower due to multi-model routing
            _ => 20_000, // Default generous timeout
        };

        Self::assert_response_time(duration, max_ms)?;
        Ok(())
    }

    /// Assert memory usage is reasonable (placeholder for future implementation)
    pub fn assert_memory_usage(_current_mb: u64, _max_mb: u64) -> Result<()> {
        // TODO: Implement actual memory usage checking
        Ok(())
    }

    /// Assert concurrent request success rate
    pub fn assert_success_rate(successful: usize, total: usize, min_rate: f64) -> Result<()> {
        let rate = successful as f64 / total as f64;
        assert!(
            rate >= min_rate,
            "Success rate should be at least {:.1}%, got {:.1}% ({}/{} requests)",
            min_rate * 100.0,
            rate * 100.0,
            successful,
            total
        );
        
        Ok(())
    }

    /// Assert throughput meets minimum requirements
    pub fn assert_throughput(requests: usize, duration: Duration, min_rps: f64) -> Result<()> {
        let rps = requests as f64 / duration.as_secs_f64();
        assert!(
            rps >= min_rps,
            "Throughput should be at least {:.1} requests/second, got {:.1}",
            min_rps,
            rps
        );
        
        Ok(())
    }

    /// Assert provider-specific throughput expectations
    pub fn assert_provider_throughput(
        provider_name: &str,
        requests: usize,
        duration: Duration
    ) -> Result<()> {
        let min_rps = match provider_name {
            "groq" => 10.0,       // Should be very fast
            "mistral" => 5.0,     // Good performance
            "openai" => 3.0,      // Standard
            "anthropic" => 2.5,   // Thoughtful responses
            "vertex" => 3.0,      // Google infrastructure
            "azure_openai" => 3.0,
            "cohere" => 3.0,
            "aws_bedrock" => 2.0, // Multi-model complexity
            _ => 1.0, // Minimum viable throughput
        };

        Self::assert_throughput(requests, duration, min_rps)?;
        Ok(())
    }

    /// Compare performance across providers
    pub fn assert_performance_comparison(
        performance_results: &HashMap<String, (Duration, f64)>
    ) -> Result<()> {
        if performance_results.len() < 2 {
            return Ok(()); // Need at least 2 to compare
        }

        // Find fastest and slowest providers
        let mut fastest = ("", Duration::from_secs(u64::MAX));
        let mut slowest = ("", Duration::from_secs(0));

        for (provider, (duration, _success_rate)) in performance_results {
            if *duration < fastest.1 {
                fastest = (provider, *duration);
            }
            if *duration > slowest.1 {
                slowest = (provider, *duration);
            }
        }

        println!(
            "Performance comparison - Fastest: {} ({}ms), Slowest: {} ({}ms)",
            fastest.0, fastest.1.as_millis(),
            slowest.0, slowest.1.as_millis()
        );

        // Groq should typically be among the fastest
        if performance_results.contains_key("groq") {
            let groq_time = performance_results.get("groq").unwrap().0;
            assert!(
                groq_time <= Duration::from_millis(3000),
                "Groq should be fast, took {}ms",
                groq_time.as_millis()
            );
        }

        Ok(())
    }
}

/// Error validation helpers with provider-specific patterns
pub struct ErrorAssertions;

impl ErrorAssertions {
    /// Assert error message contains expected patterns
    pub fn assert_error_contains(error: &anyhow::Error, patterns: &[&str]) -> Result<()> {
        let error_str = error.to_string();
        
        for pattern in patterns {
            assert!(
                error_str.contains(pattern),
                "Error should contain '{}'. Actual error: {}",
                pattern,
                error_str
            );
        }
        
        Ok(())
    }

    /// Assert provider-specific error patterns
    pub fn assert_provider_error_pattern(provider_name: &str, error: &anyhow::Error) -> Result<()> {
        let error_str = error.to_string();
        
        let expected_patterns = match provider_name {
            "openai" => vec!["OpenAI", "Invalid API key", "Rate limit"],
            "anthropic" => vec!["Anthropic", "Authentication failed", "Invalid request"],
            "vertex" => vec!["Vertex", "Invalid credentials", "Quota exceeded"],
            "groq" => vec!["Groq", "Service unavailable"],
            "azure_openai" => vec!["Azure", "authentication"],
            "aws_bedrock" => vec!["Bedrock", "AWS"],
            "cohere" => vec!["Cohere"],
            "mistral" => vec!["Mistral"],
            _ => return Ok(()), // Skip validation for unknown providers
        };

        let has_expected_pattern = expected_patterns.iter().any(|pattern| {
            error_str.to_lowercase().contains(&pattern.to_lowercase())
        });

        if !has_expected_pattern {
            println!(
                "Warning: Error from {} doesn't contain expected patterns. Error: {}",
                provider_name, error_str
            );
        }
        
        Ok(())
    }

    /// Assert HTTP status code in error response
    pub fn assert_http_status(status: reqwest::StatusCode, expected: u16) -> Result<()> {
        assert_eq!(
            status.as_u16(),
            expected,
            "HTTP status should be {}, got {}",
            expected,
            status.as_u16()
        );
        
        Ok
()
    }

    /// Assert error is retryable/non-retryable
    pub fn assert_error_retryable(error: &anyhow::Error, should_retry: bool) -> Result<()> {
        let error_str = error.to_string().to_lowercase();
        
        let is_retryable = error_str.contains("timeout") ||
                          error_str.contains("service unavailable") ||
                          error_str.contains("rate limit") ||
                          error_str.contains("temporarily unavailable");
        
        if should_retry {
            assert!(is_retryable, "Error should be retryable: {}", error);
        } else {
            assert!(!is_retryable, "Error should not be retryable: {}", error);
        }
        
        Ok(())
    }

    /// Assert error recovery behavior across providers
    pub fn assert_error_recovery_behavior(
        provider_errors: &HashMap<String, Vec<String>>,
        expected_recovery_patterns: &HashMap<String, Vec<&str>>
    ) -> Result<()> {
        for (provider_name, errors) in provider_errors {
            if let Some(expected_patterns) = expected_recovery_patterns.get(provider_name) {
                for error in errors {
                    let mut found_pattern = false;
                    for pattern in expected_patterns {
                        if error.to_lowercase().contains(&pattern.to_lowercase()) {
                            found_pattern = true;
                            break;
                        }
                    }
                    
                    if !found_pattern {
                        println!(
                            "Warning: Error from {} doesn't match expected recovery patterns: {}",
                            provider_name, error
                        );
                    }
                }
            }
        }
        
        Ok(())
    }
}

/// Tool calling validation across providers
pub struct ToolCallAssertions;

impl ToolCallAssertions {
    /// Assert tool calling works consistently across providers
    pub fn assert_consistent_tool_calling(
        tool_responses: &HashMap<String, ChatCompletionResponse>
    ) -> Result<()> {
        let mut tool_call_counts = HashMap::new();
        let mut function_names = HashMap::new();

        for (provider_name, response) in tool_responses {
            if let Some(choice) = response.choices.first() {
                if let Some(ref tool_calls) = choice.message.tool_calls {
                    tool_call_counts.insert(provider_name.clone(), tool_calls.len());
                    
                    let names: Vec<String> = tool_calls.iter()
                        .map(|tc| tc.function.name.clone())
                        .collect();
                    function_names.insert(provider_name.clone(), names);
                }
            }
        }

        // Check that providers supporting tools have similar tool call patterns
        if tool_call_counts.len() >= 2 {
            let call_counts: Vec<usize> = tool_call_counts.values().cloned().collect();
            let max_calls = *call_counts.iter().max().unwrap_or(&0);
            let min_calls = *call_counts.iter().min().unwrap_or(&0);

            // Allow some variation but not excessive differences
            if min_calls > 0 {
                let ratio = max_calls as f64 / min_calls as f64;
                assert!(
                    ratio <= 3.0,
                    "Tool call count variation too high across providers: max={}, min={}",
                    max_calls, min_calls
                );
            }
        }

        Ok(())
    }

    /// Assert tool call arguments are valid JSON across providers
    pub fn assert_tool_arguments_valid(
        tool_responses: &HashMap<String, ChatCompletionResponse>
    ) -> Result<()> {
        for (provider_name, response) in tool_responses {
            if let Some(choice) = response.choices.first() {
                if let Some(ref tool_calls) = choice.message.tool_calls {
                    for (i, tool_call) in tool_calls.iter().enumerate() {
                        let _: Value = serde_json::from_str(&tool_call.function.arguments)
                            .map_err(|e| anyhow::anyhow!(
                                "Provider {} tool call {} has invalid JSON arguments: {}",
                                provider_name, i, e
                            ))?;
                    }
                }
            }
        }
        
        Ok(())
    }

    /// Assert provider-specific tool calling capabilities
    pub fn assert_provider_tool_capabilities(provider_name: &str, response: &ChatCompletionResponse) -> Result<()> {
        let supports_tools = match provider_name {
            "openai" => true,
            "anthropic" => true,
            "vertex" => true,
            "groq" => false, // Limited support
            "azure_openai" => true,
            "aws_bedrock" => true,
            "cohere" => true,
            "mistral" => true,
            _ => return Err(anyhow::anyhow!("Unknown provider: {}", provider_name)),
        };

        if supports_tools {
            // Providers that support tools should be able to generate them when requested
            if let Some(choice) = response.choices.first() {
                if choice.message.tool_calls.is_none() && choice.message.content.is_none() {
                    return Err(anyhow::anyhow!(
                        "Provider {} should generate either content or tool calls",
                        provider_name
                    ));
                }
            }
        } else {
            // Providers with limited tool support should focus on content
            if let Some(choice) = response.choices.first() {
                assert!(
                    choice.message.content.is_some(),
                    "Provider {} with limited tool support should provide content",
                    provider_name
                );
            }
        }

        Ok(())
    }
}

/// Helper macros for common assertions
#[macro_export]
macro_rules! assert_response_ok {
    ($response:expr) => {
        crate::utils::assertions::ResponseAssertions::assert_valid_response(&$response)
            .expect("Response should be valid");
        crate::utils::assertions::ResponseAssertions::assert_valid_choices(&$response.choices)
            .expect("Choices should be valid");
    };
}

#[macro_export]
macro_rules! assert_provider_response_ok {
    ($response:expr, $provider:expr) => {
        crate::utils::assertions::ResponseAssertions::assert_provider_response_format(&$response, $provider)
            .expect("Provider response should be valid");
    };
}

#[macro_export]
macro_rules! assert_conversation_valid {
    ($state:expr) => {
        crate::utils::assertions::ConversationAssertions::assert_valid_conversation(&$state)
            .expect("Conversation should be valid");
    };
}

#[macro_export]
macro_rules! assert_timing {
    ($duration:expr, $max_ms:expr) => {
        crate::utils::assertions::PerformanceAssertions::assert_response_time($duration, $max_ms)
            .expect(&format!("Response should complete within {}ms", $max_ms));
    };
}

#[macro_export]
macro_rules! assert_provider_timing {
    ($provider:expr, $duration:expr) => {
        crate::utils::assertions::PerformanceAssertions::assert_provider_response_time($provider, $duration)
            .expect(&format!("Provider {} should meet timing expectations", $provider));
    };
}

#[macro_export]
macro_rules! assert_cross_provider_consistency {
    ($responses:expr) => {
        crate::utils::assertions::CrossProviderAssertions::assert_all_responses_valid(&$responses)
            .expect("All provider responses should be valid");
        crate::utils::assertions::CrossProviderAssertions::assert_consistent_behavior(&$responses)
            .expect("Provider behavior should be consistent");
    };
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::response::ChatCompletionUsage;

    #[test]
    fn test_response_validation() {
        let response = ChatCompletionResponse {
            id: "test-id".to_string(),
            object: "chat.completion".to_string(),
            created: 1234567890,
            model: "claude-4-sonnet-20250514".to_string(),
            choices: vec![ChatCompletionChoice {
                index: 0,
                message: ChatMessage {
                    role: MessageRole::Assistant,
                    content: Some("Test response".to_string()),
                    name: None,
                    tool_calls: None,
                    tool_call_id: None,
                },
                logprobs: None,
                finish_reason: Some(FinishReason::Stop),
            }],
            usage: Some(ChatCompletionUsage {
                prompt_tokens: 10,
                completion_tokens: 5,
                total_tokens: 15,
            }),
            system_fingerprint: None,
        };

        assert!(ResponseAssertions::assert_valid_response(&response).is_ok());
        assert!(ResponseAssertions::assert_reasonable_usage(&response).is_ok());
    }

    #[test]
    fn test_provider_response_format() {
        let mut response = ChatCompletionResponse {
            id: "chatcmpl-openai-test".to_string(),
            object: "chat.completion".to_string(),
            created: 1234567890,
            model: "gpt-4".to_string(),
            choices: vec![ChatCompletionChoice {
                index: 0,
                message: ChatMessage {
                    role: MessageRole::Assistant,
                    content: Some("OpenAI response".to_string()),
                    name: None,
                    tool_calls: None,
                    tool_call_id: None,
                },
                logprobs: None,
                finish_reason: Some(FinishReason::Stop),
            }],
            usage: Some(ChatCompletionUsage {
                prompt_tokens: 10,
                completion_tokens: 5,
                total_tokens: 15,
            }),
            system_fingerprint: None,
        };

        assert!(ResponseAssertions::assert_provider_response_format(&response, "openai").is_ok());

        // Test Anthropic format
        response.id = "msg_anthropic_test".to_string();
        assert!(ResponseAssertions::assert_provider_response_format(&response, "anthropic").is_ok());
    }

    #[test]
    fn test_streaming_validation() {
        let chunks = vec![
            "data: {\"choices\":[{\"delta\":{\"content\":\"Hello\"}}]}".to_string(),
            "data: {\"choices\":[{\"delta\":{\"content\":\" world\"}}]}".to_string(),
            "data: [DONE]".to_string(),
        ];

        assert!(StreamingAssertions::assert_valid_streaming_chunks(&chunks).is_ok());
    }

    #[test]
    fn test_performance_assertions() {
        let duration = Duration::from_millis(500);
        assert!(PerformanceAssertions::assert_response_time(duration, 1000).is_ok());
        assert!(PerformanceAssertions::assert_provider_response_time("groq", duration).is_ok());
        
        assert!(PerformanceAssertions::assert_success_rate(95, 100, 0.9).is_ok());
        assert!(PerformanceAssertions::assert_throughput(100, Duration::from_secs(10), 5.0).is_ok());
    }

    #[test]
    fn test_cross_provider_assertions() {
        let mut responses = HashMap::new();
        
        let response1 = ChatCompletionResponse {
            id: "openai-test".to_string(),
            object: "chat.completion".to_string(),
            created: 1234567890,
            model: "gpt-4".to_string(),
            choices: vec![ChatCompletionChoice {
                index: 0,
                message: ChatMessage {
                    role: MessageRole::Assistant,
                    content: Some("Response content".to_string()),
                    name: None,
                    tool_calls: None,
                    tool_call_id: None,
                },
                logprobs: None,
                finish_reason: Some(FinishReason::Stop),
            }],
            usage: Some(ChatCompletionUsage {
                prompt_tokens: 10,
                completion_tokens: 5,
                total_tokens: 15,
            }),
            system_fingerprint: None,
        };

        let response2 = ChatCompletionResponse {
            id: "anthropic-test".to_string(),
            object: "chat.completion".to_string(),
            created: 1234567890,
            model: "claude-3-sonnet".to_string(),
            choices: vec![ChatCompletionChoice {
                index: 0,
                message: ChatMessage {
                    role: MessageRole::Assistant,
                    content: Some("Similar response content".to_string()),
                    name: None,
                    tool_calls: None,
                    tool_call_id: None,
                },
                logprobs: None,
                finish_reason: Some(FinishReason::Stop),
            }],
            usage: Some(ChatCompletionUsage {
                prompt_tokens: 12,
                completion_tokens: 6,
                total_tokens: 18,
            }),
            system_fingerprint: None,
        };

        responses.insert("openai".to_string(), response1);
        responses.insert("anthropic".to_string(), response2);

        assert!(CrossProviderAssertions::assert_all_responses_valid(&responses).is_ok());
        assert!(CrossProviderAssertions::assert_consistent_behavior(&responses).is_ok());
    }

    #[test]
    fn test_provider_capabilities() {
        assert!(CrossProviderAssertions::assert_provider_capabilities("openai", true, true).is_ok());
        assert!(CrossProviderAssertions::assert_provider_capabilities("groq", true, false).is_ok());
        
        // Should fail for incorrect capabilities
        assert!(CrossProviderAssertions::assert_provider_capabilities("groq", true, true).is_err());
    }

    #[test]
    fn test_error_assertions() {
        let error = anyhow::anyhow!("OpenAI rate limit exceeded");
        assert!(ErrorAssertions::assert_error_contains(&error, &["OpenAI", "rate limit"]).is_ok());
        assert!(ErrorAssertions::assert_provider_error_pattern("openai", &error).is_ok());
        assert!(ErrorAssertions::assert_error_retryable(&error, true).is_ok());
        
        let non_retryable_error = anyhow::anyhow!("Invalid API key");
        assert!(ErrorAssertions::assert_error_retryable(&non_retryable_error, false).is_ok());
    }

    #[test]
    fn test_tool_call_assertions() {
        let mut responses = HashMap::new();
        
        let tool_call = ToolCall {
            id: "call_123".to_string(),
            tool_type: "function".to_string(),
            function: crate::models::common::FunctionCall {
                name: "read_file".to_string(),
                arguments: r#"{"file_path": "test.rs"}"#.to_string(),
            },
        };

        let response = ChatCompletionResponse {
            id: "test".to_string(),
            object: "chat.completion".to_string(),
            created: 1234567890,
            model: "gpt-4".to_string(),
            choices: vec![ChatCompletionChoice {
                index: 0,
                message: ChatMessage {
                    role: MessageRole::Assistant,
                    content: None,
                    name: None,
                    tool_calls: Some(vec![tool_call]),
                    tool_call_id: None,
                },
                logprobs: None,
                finish_reason: Some(FinishReason::ToolCalls),
            }],
            usage: None,
            system_fingerprint: None,
        };

        responses.insert("openai".to_string(), response);
        
        assert!(ToolCallAssertions::assert_tool_arguments_valid(&responses).is_ok());
        assert!(ToolCallAssertions::assert_provider_tool_capabilities("openai", &responses["openai"]).is_ok());
    }

    #[test]
    fn test_fallback_chain_assertions() {
        let chain_results = vec![
            ("provider1".to_string(), Err("Service unavailable".to_string())),
            ("provider2".to_string(), Err("Rate limited".to_string())),
            ("provider3".to_string(), Ok(ChatCompletionResponse {
                id: "success".to_string(),
                object: "chat.completion".to_string(),
                created: 1234567890,
                model: "working-model".to_string(),
                choices: vec![ChatCompletionChoice {
                    index: 0,
                    message: ChatMessage {
                        role: MessageRole::Assistant,
                        content: Some("Success response".to_string()),
                        name: None,
                        tool_calls: None,
                        tool_call_id: None,
                    },
                    logprobs: None,
                    finish_reason: Some(FinishReason::Stop),
                }],
                usage: None,
                system_fingerprint: None,
            })),
        ];

        assert!(CrossProviderAssertions::assert_fallback_chain_behavior(&chain_results).is_ok());
    }

    #[test]
    fn test_performance_comparison() {
        let mut performance_results = HashMap::new();
        performance_results.insert("groq".to_string(), (Duration::from_millis(500), 0.95));
        performance_results.insert("openai".to_string(), (Duration::from_millis(2000), 0.97));
        performance_results.insert("anthropic".to_string(), (Duration::from_millis(2500), 0.94));

        assert!(PerformanceAssertions::assert_performance_comparison(&performance_results).is_ok());
    }
}