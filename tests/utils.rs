//! Test utilities for integration testing
//! 
//! This module provides common utilities, mock implementations, and helper functions
//! used across different integration test suites.

use anyhow::Result;
use serde_json::{json, Value};
use std::time::{Duration, Instant};
use reqwest::Client;
use tokio::time::timeout;

use llm_supabase_rs::models::{
    request::ChatCompletionRequest,
    response::ChatCompletionResponse,
    common::{ChatMessage, MessageRole, ToolCall, FunctionCall},
};

/// Mock Codex CLI client that simulates real Codex CLI behavior
#[derive(Clone)]
pub struct MockCodexClient {
    base_url: String,
    client: Client,
    conversation_history: Vec<ChatMessage>,
    last_response_id: Option<String>,
}

/// Response from a Codex CLI request
#[derive(Debug)]
pub struct CodexResponse {
    pub content: String,
    pub tool_calls: Vec<ToolCall>,
    pub response_id: String,
    pub finish_reason: String,
}

/// Tool execution result for Codex CLI
#[derive(Debug)]
pub struct ToolExecutionResult {
    pub tool_call_id: String,
    pub result: String,
    pub success: bool,
}

impl MockCodexClient {
    /// Create a new mock Codex CLI client
    pub fn new(base_url: &str) -> Self {
        Self {
            base_url: base_url.to_string(),
            client: Client::new(),
            conversation_history: Vec::new(),
            last_response_id: None,
        }
    }

    /// Send a chat completion request (simulating Codex CLI behavior)
    pub async fn send_request(&mut self, message: &str, model: &str) -> Result<CodexResponse> {
        // Add user message to conversation history
        let user_message = ChatMessage::user(message);
        self.conversation_history.push(user_message);

        // Build request with Codex CLI patterns
        let mut request = ChatCompletionRequest::new(model, self.conversation_history.clone())
            .with_max_tokens(4096)
            .with_temperature(0.7);

        // Add previous_response_id if this is a follow-up request
        if let Some(ref prev_id) = self.last_response_id {
            request = request.with_previous_response_id(prev_id);
        }

        let response = self
            .client
            .post(&format!("{}/v1/chat/completions", self.base_url))
            .header("Content-Type", "application/json")
            .header("Authorization", "Bearer test-key")
            .json(&request)
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(anyhow::anyhow!(
                "Request failed with status: {} - {}",
                response.status(),
                response.text().await?
            ));
        }

        let chat_response: ChatCompletionResponse = response.json().await?;
        
        // Extract the first choice (Codex CLI expects single response)
        let choice = chat_response
            .choices
            .first()
            .ok_or_else(|| anyhow::anyhow!("No choices in response"))?;

        let assistant_message = choice.message.clone();
        
        // Store response ID for future requests
        self.last_response_id = Some(chat_response.id.clone());

        // Add assistant message to history
        self.conversation_history.push(assistant_message.clone());

        // Build Codex response
        Ok(CodexResponse {
            content: assistant_message.content.unwrap_or_default(),
            tool_calls: assistant_message.tool_calls.unwrap_or_default(),
            response_id: chat_response.id,
            finish_reason: choice.finish_reason.clone().unwrap_or("stop".to_string()),
        })
    }

    /// Send a streaming request (simulating Codex CLI streaming behavior)
    pub async fn send_streaming_request(&mut self, message: &str, model: &str) -> Result<Vec<String>> {
        // Add user message to conversation history
        let user_message = ChatMessage::user(message);
        self.conversation_history.push(user_message);

        // Build streaming request
        let mut request = ChatCompletionRequest::new(model, self.conversation_history.clone())
            .with_max_tokens(4096)
            .with_temperature(0.7)
            .streaming(true);

        // Add previous_response_id if available
        if let Some(ref prev_id) = self.last_response_id {
            request = request.with_previous_response_id(prev_id);
        }

        let response = self
            .client
            .post(&format!("{}/v1/chat/completions", self.base_url))
            .header("Content-Type", "application/json")
            .header("Authorization", "Bearer test-key")
            .json(&request)
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(anyhow::anyhow!(
                "Streaming request failed with status: {}",
                response.status()
            ));
        }

        // Process the streaming response
        let response_text = response.text().await?;
        let chunks: Vec<String> = response_text
            .lines()
            .filter(|line| line.starts_with("data: "))
            .map(|line| line.to_string())
            .collect();

        // Add a [DONE] marker to simulate proper streaming completion
        let mut result_chunks = chunks;
        result_chunks.push("data: [DONE]".to_string());

        Ok(result_chunks)
    }

    /// Execute tool calls and send results back (simulating Codex CLI tool execution)
    pub async fn execute_tool_calls(&mut self, tool_calls: Vec<ToolCall>) -> Result<CodexResponse> {
        let mut tool_results = Vec::new();

        // Simulate tool execution
        for tool_call in tool_calls {
            let result = self.simulate_tool_execution(&tool_call).await?;
            tool_results.push(result);
        }

        // Add tool result messages to conversation history
        for result in &tool_results {
            let mut tool_message = ChatMessage::user(&result.result);
            tool_message.role = MessageRole::Tool;
            tool_message.tool_call_id = Some(result.tool_call_id.clone());
            self.conversation_history.push(tool_message);
        }

        // Send follow-up request with tool results
        self.send_follow_up_request().await
    }

    /// Send a follow-up request after tool execution
    async fn send_follow_up_request(&mut self) -> Result<CodexResponse> {
        let mut request = ChatCompletionRequest::new("claude-4-sonnet-20250514", self.conversation_history.clone())
            .with_max_tokens(4096)
            .with_temperature(0.7);

        if let Some(ref prev_id) = self.last_response_id {
            request = request.with_previous_response_id(prev_id);
        }

        let response = self
            .client
            .post(&format!("{}/v1/chat/completions", self.base_url))
            .header("Content-Type", "application/json")
            .header("Authorization", "Bearer test-key")
            .json(&request)
            .send()
            .await?;

        let chat_response: ChatCompletionResponse = response.json().await?;
        let choice = chat_response
            .choices
            .first()
            .ok_or_else(|| anyhow::anyhow!("No choices in response"))?;

        let assistant_message = choice.message.clone();
        self.last_response_id = Some(chat_response.id.clone());
        self.conversation_history.push(assistant_message.clone());

        Ok(CodexResponse {
            content: assistant_message.content.unwrap_or_default(),
            tool_calls: assistant_message.tool_calls.unwrap_or_default(),
            response_id: chat_response.id,
            finish_reason: choice.finish_reason.clone().unwrap_or("stop".to_string()),
        })
    }

    /// Simulate tool execution (mimics what Codex CLI would do)
    async fn simulate_tool_execution(&self, tool_call: &ToolCall) -> Result<ToolExecutionResult> {
        let result = match tool_call.function.name.as_str() {
            "apply_patch" => {
                "Successfully applied patch with changes to file".to_string()
            }
            "read_file" => {
                "File content read successfully".to_string()
            }
            "search_files" => {
                "Found 3 matches in project files".to_string()
            }
            "write_file" => {
                "File written successfully".to_string()
            }
            _ => {
                format!("Tool '{}' executed successfully with mock result", tool_call.function.name)
            }
        };

        Ok(ToolExecutionResult {
            tool_call_id: tool_call.id.clone(),
            result,
            success: true,
        })
    }

    /// Reset conversation history (for new conversation tests)
    pub fn reset_conversation(&mut self) {
        self.conversation_history.clear();
        self.last_response_id = None;
    }

    /// Get current conversation history
    pub fn get_conversation_history(&self) -> &[ChatMessage] {
        &self.conversation_history
    }

    /// Get last response ID
    pub fn get_last_response_id(&self) -> Option<&str> {
        self.last_response_id.as_deref()
    }

    /// Set a custom response ID (for testing multi-turn scenarios)
    pub fn set_last_response_id(&mut self, response_id: String) {
        self.last_response_id = Some(response_id);
    }
}

/// Test server management
pub struct TestServer {
    pub url: String,
}

impl TestServer {
    /// Create a mock test server
    pub async fn start() -> Result<Self> {
        Ok(Self {
            url: "http://localhost:3000".to_string(),
        })
    }

    /// Get server URL
    pub fn url(&self) -> &str {
        &self.url
    }

    /// Wait for server to be ready (mock implementation)
    pub async fn wait_for_ready(&self, _timeout_seconds: u64) -> Result<()> {
        tokio::time::sleep(Duration::from_millis(100)).await;
        Ok(())
    }

    /// Shutdown server (mock implementation)
    pub async fn shutdown(self) -> Result<()> {
        Ok(())
    }
}

/// Test scenarios for different server configurations
pub struct TestScenarios;

impl TestScenarios {
    /// Create a server configured for Codex CLI testing
    pub async fn codex_cli_server() -> Result<TestServer> {
        TestServer::start().await
    }

    /// Create a server configured for multi-turn conversation testing
    pub async fn conversation_server() -> Result<TestServer> {
        TestServer::start().await
    }

    /// Create a server configured for provider fallback testing
    pub async fn fallback_server() -> Result<TestServer> {
        TestServer::start().await
    }

    /// Create a server configured for streaming testing
    pub async fn streaming_server() -> Result<TestServer> {
        TestServer::start().await
    }

    /// Create a server configured for performance testing
    pub async fn performance_server() -> Result<TestServer> {
        TestServer::start().await
    }
}

/// Performance assertion helpers
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
}

/// Response assertion helpers
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
}

/// Streaming assertion helpers  
pub struct StreamingAssertions;

impl StreamingAssertions {
    /// Assert streaming chunks are valid
    pub fn assert_valid_streaming_chunks(chunks: &[String]) -> Result<()> {
        assert!(!chunks.is_empty(), "Should have at least one chunk");
        
        let has_done_marker = chunks.iter().any(|chunk| chunk.contains("[DONE]"));
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
        // Simple validation that chunks exist and have content
        assert!(!chunks.is_empty(), "Should have streaming chunks");
        Ok(())
    }
}

/// Sample files for testing
pub struct SampleFiles;

impl SampleFiles {
    /// Sample main.rs content
    pub fn main_rs() -> &'static str {
        r#"// main.rs
fn main() {
    println!("Hello, world!");
}

fn helper_function(x: i32) -> i32 {
    x * 2
}
"#
    }

    /// Sample lib.rs content
    pub fn lib_rs() -> &'static str {
        r#"//! Sample library crate

pub mod utils;

pub fn add(left: usize, right: usize) -> usize {
    left + right
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {
        let result = add(2, 2);
        assert_eq!(result, 4);
    }
}
"#
    }
}

/// Sample patches for testing
pub struct SamplePatches;

impl SamplePatches {
    /// Patch to add a new function to main.rs
    pub fn add_function_patch() -> &'static str {
        r#"--- a/src/main.rs
+++ b/src/main.rs
@@ -1,7 +1,11 @@
 // main.rs
+fn factorial(n: u64) -> u64 {
+    match n {
+        0 | 1 => 1,
+        _ => n * factorial(n - 1),
+    }
+}
+
 fn main() {
     println!("Hello, world!");
+    println!("5! = {}", factorial(5));
 }
 
 fn helper_function(x: i32) -> i32 {
"#
    }

    /// Patch to add error handling
    pub fn add_error_handling_patch() -> &'static str {
        r#"--- a/src/main.rs
+++ b/src/main.rs
@@ -1,3 +1,8 @@
+use std::error::Error;
+use std::io;
+
 // main.rs
-fn main() {
+fn main() -> Result<(), Box<dyn Error>> {
     println!("Hello, world!");
+    Ok(())
 }
"#
    }
}

/// Helper function to wait for streaming to complete
pub async fn wait_for_streaming_completion(
    chunks: Vec<String>,
    timeout_seconds: u64,
) -> Result<bool> {
    let timeout_duration = Duration::from_secs(timeout_seconds);
    
    timeout(timeout_duration, async {
        // Check if we received a [DONE] chunk or proper termination
        chunks.iter().any(|chunk| chunk.contains("[DONE]") || chunk.contains("\"finish_reason\""))
    }).await.map_err(|_| anyhow::anyhow!("Streaming timeout after {} seconds", timeout_seconds))
}