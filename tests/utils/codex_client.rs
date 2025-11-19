
//! Mock Codex CLI client for testing integration workflows across all providers
//! 
//! This module provides a test client that simulates the behavior of the actual
//! Codex CLI when interacting with our OpenAI-compatible proxy, supporting all 8 providers.

use anyhow::Result;
use reqwest::Client;
use serde_json::{json, Value};
use std::collections::HashMap;
use tokio::time::{timeout, Duration};
use futures_util::StreamExt;

use crate::models::{
    request::ChatCompletionRequest,
    response::ChatCompletionResponse,
    common::{ChatMessage, MessageRole, ToolDefinition, FunctionDefinition, ToolCall, FunctionCall},
};

/// Provider configuration for testing
#[derive(Debug, Clone)]
pub struct ProviderConfig {
    pub name: String,
    pub default_model: String,
    pub supports_streaming: bool,
    pub supports_tools: bool,
    pub expected_response_time_ms: u64,
    pub custom_headers: HashMap<String, String>,
}

impl ProviderConfig {
    /// Create OpenAI provider configuration
    pub fn openai() -> Self {
        Self {
            name: "openai".to_string(),
            default_model: "gpt-4".to_string(),
            supports_streaming: true,
            supports_tools: true,
            expected_response_time_ms: 2000,
            custom_headers: HashMap::new(),
        }
    }

    /// Create Anthropic provider configuration
    pub fn anthropic() -> Self {
        Self {
            name: "anthropic".to_string(),
            default_model: "claude-3-sonnet-20240229".to_string(),
            supports_streaming: true,
            supports_tools: true,
            expected_response_time_ms: 2500,
            custom_headers: HashMap::new(),
        }
    }

    /// Create Vertex AI provider configuration
    pub fn vertex() -> Self {
        Self {
            name: "vertex".to_string(),
            default_model: "gemini-pro".to_string(),
            supports_streaming: true,
            supports_tools: true,
            expected_response_time_ms: 2200,
            custom_headers: HashMap::new(),
        }
    }

    /// Create Groq provider configuration
    pub fn groq() -> Self {
        Self {
            name: "groq".to_string(),
            default_model: "mixtral-8x7b-32768".to_string(),
            supports_streaming: true,
            supports_tools: false, // Limited tool support
            expected_response_time_ms: 500, // Very fast
            custom_headers: HashMap::new(),
        }
    }

    /// Create Azure OpenAI provider configuration
    pub fn azure_openai() -> Self {
        Self {
            name: "azure_openai".to_string(),
            default_model: "gpt-4".to_string(),
            supports_streaming: true,
            supports_tools: true,
            expected_response_time_ms: 2100,
            custom_headers: HashMap::new(),
        }
    }

    /// Create AWS Bedrock provider configuration
    pub fn aws_bedrock() -> Self {
        Self {
            name: "aws_bedrock".to_string(),
            default_model: "anthropic.claude-3-sonnet-20240229-v1:0".to_string(),
            supports_streaming: true,
            supports_tools: true,
            expected_response_time_ms: 2800,
            custom_headers: HashMap::new(),
        }
    }

    /// Create Cohere provider configuration
    pub fn cohere() -> Self {
        Self {
            name: "cohere".to_string(),
            default_model: "command".to_string(),
            supports_streaming: true,
            supports_tools: true,
            expected_response_time_ms: 2300,
            custom_headers: HashMap::new(),
        }
    }

    /// Create Mistral AI provider configuration
    pub fn mistral() -> Self {
        Self {
            name: "mistral".to_string(),
            default_model: "mistral-large-latest".to_string(),
            supports_streaming: true,
            supports_tools: true,
            expected_response_time_ms: 1800,
            custom_headers: HashMap::new(),
        }
    }

    /// Get all provider configurations
    pub fn all_providers() -> Vec<Self> {
        vec![
            Self::openai(),
            Self::anthropic(),
            Self::vertex(),
            Self::groq(),
            Self::azure_openai(),
            Self::aws_bedrock(),
            Self::cohere(),
            Self::mistral(),
        ]
    }
}

/// Mock Codex CLI client that simulates real Codex CLI behavior across all providers
#[derive(Clone)]
pub struct MockCodexClient {
    base_url: String,
    client: Client,
    conversation_history: Vec<ChatMessage>,
    last_response_id: Option<String>,
    provider_config: Option<ProviderConfig>,
    custom_headers: HashMap<String, String>,
}

/// Response from a Codex CLI request with provider information
#[derive(Debug, Clone)]
pub struct CodexResponse {
    pub content: String,
    pub tool_calls: Vec<ToolCall>,
    pub response_id: String,
    pub finish_reason: String,
    pub usage: Option<Usage>,
    pub provider_name: Option<String>,
    pub model_used: String,
    pub response_time_ms: u64,
}

/// Token usage statistics
#[derive(Debug)]
pub struct Usage {
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
    pub total_tokens: u32,
}

/// Tool execution result for Codex CLI
#[derive(Debug)]
pub struct ToolExecutionResult {
    pub tool_call_id: String,
    pub result: String,
    pub success: bool,
}

/// Multi-provider test session for comprehensive testing
pub struct MultiProviderTestSession {
    pub clients: HashMap<String, MockCodexClient>,
    pub results: HashMap<String, Vec<CodexResponse>>,
    pub errors: HashMap<String, Vec<String>>,
}

impl MultiProviderTestSession {
    /// Create a new multi-provider test session
    pub fn new(base_url: &str) -> Self {
        let mut clients = HashMap::new();
        
        for config in ProviderConfig::all_providers() {
            let mut client = MockCodexClient::new(base_url);
            client.set_provider_config(config.clone());
            clients.insert(config.name.clone(), client);
        }
        
        Self {
            clients,
            results: HashMap::new(),
            errors: HashMap::new(),
        }
    }

    /// Run the same test across all providers
    pub async fn test_all_providers(&mut self, message: &str) -> Result<HashMap<String, CodexResponse>> {
        let mut results = HashMap::new();
        
        for (provider_name, client) in &mut self.clients {
            match client.send_request_with_provider(message).await {
                Ok(response) => {
                    self.results.entry(provider_name.clone()).or_insert_with(Vec::new).push(response.clone());
                    results.insert(provider_name.clone(), response);
                }
                Err(e) => {
                    let error_msg = e.to_string();
                    self.errors.entry(provider_name.clone()).or_insert_with(Vec::new).push(error_msg);
                }
            }
        }
        
        Ok(results)
    }

    /// Test streaming across all providers that support it
    pub async fn test_streaming_all_providers(&mut self, message: &str) -> Result<HashMap<String, Vec<String>>> {
        let mut results = HashMap::new();
        
        for (provider_name, client) in &mut self.clients {
            if let Some(config) = &client.provider_config {
                if config.supports_streaming {
                    match client.send_streaming_request_with_provider(message).await {
                        Ok(chunks) => {
                            results.insert(provider_name.clone(), chunks);
                        }
                        Err(e) => {
                            let error_msg = e.to_string();
                            self.errors.entry(provider_name.clone()).or_insert_with(Vec::new).push(error_msg);
                        }
                    }
                }
            }
        }
        
        Ok(results)
    }

    /// Get performance comparison across providers
    pub fn get_performance_comparison(&self) -> Vec<(String, f64)> {
        let mut performance = Vec::new();
        
        for (provider_name, responses) in &self.results {
            if !responses.is_empty() {
                let avg_response_time = responses.iter()
                    .map(|r| r.response_time_ms as f64)
                    .sum::<f64>() / responses.len() as f64;
                performance.push((provider_name.clone(), avg_response_time));
            }
        }
        
        performance.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));
        performance
    }

    /// Get success rates for each provider
    pub fn get_success_rates(&self) -> HashMap<String, f64> {
        let mut success_rates = HashMap::new();
        
        for provider_name in self.clients.keys() {
            let success_count = self.results.get(provider_name).map(|r| r.len()).unwrap_or(0);
            let error_count = self.errors.get(provider_name).map(|e| e.len()).unwrap_or(0);
            let total_attempts = success_count + error_count;
            
            let success_rate = if total_attempts > 0 {
                success_count as f64 / total_attempts as f64
            } else {
                0.0
            };
            
            success_rates.insert(provider_name.clone(), success_rate);
        }
        
        success_rates
    }
}

impl MockCodexClient {
    /// Create a new mock Codex CLI client
    pub fn new(base_url: &str) -> Self {
        Self {
            base_url: base_url.to_string(),
            client: Client::new(),
            conversation_history: Vec::new(),
            last_response_id: None,
            provider_config: None,
            custom_headers: HashMap::new(),
        }
    }

    /// Create a client with specific provider configuration
    pub fn with_provider_config(base_url: &str, config: ProviderConfig) -> Self {
        let mut client = Self::new(base_url);
        client.set_provider_config(config);
        client
    }

    /// Set provider configuration
    pub fn set_provider_config(&mut self, config: ProviderConfig) {
        self.custom_headers = config.custom_headers.clone();
        self.provider_config = Some(config);
    }

    /// Get the current provider name
    pub fn get_provider_name(&self) -> Option<&str> {
        self.provider_config.as_ref().map(|c| c.name.as_str())
    }

    /// Send a chat completion request using provider-specific settings
    pub async fn send_request_with_provider(&mut self, message: &str) -> Result<CodexResponse> {
        let model = self.provider_config.as_ref()
            .map(|c| c.default_model.clone())
            .unwrap_or_else(|| "gpt-4".to_string());
        
        let start_time = std::time::Instant::now();
        let mut response = self.send_request(message, &model).await?;
        response.response_time_ms = start_time.elapsed().as_millis() as u64;
        response.provider_name = self.provider_config.as_ref().map(|c| c.name.clone());
        response.model_used = model;
        
        Ok(response)
    }

    /// Send a streaming request using provider-specific settings
    pub async fn send_streaming_request_with_provider(&mut self, message: &str) -> Result<Vec<String>> {
        let model = self.provider_config.as_ref()
            .map(|c| c.default_model.clone())
            .unwrap_or_else(|| "gpt-4".to_string());
        
        self.send_streaming_request(message, &model).await
    }

    /// Send a chat completion request (simulating Codex CLI behavior)
    pub async fn send_request(&mut self, message: &str, model: &str) -> Result<CodexResponse> {
        // Add user message to conversation history
        let user_message = ChatMessage {
            role: MessageRole::User,
            content: message.to_string(),
            name: None,
            tool_calls: None,
            tool_call_id: None,
        };
        self.conversation_history.push(user_message);

        // Build request with Codex CLI patterns
        let mut request = ChatCompletionRequest::new(model, self.conversation_history.clone())
            .with_max_tokens(4096)
            .with_temperature(0.7);

        // Add previous_response_id if this is a follow-up request
        if let Some(ref prev_id) = self.last_response_id {
            request = request.with_previous_response_id(prev_id);
        }

        // Add tools if provider supports them
        if self.provider_config.as_ref().map(|c| c.supports_tools).unwrap_or(true) {
            request.tools = Some(self.get_codex_tools());
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
            usage: chat_response.usage.map(|u| Usage {
                prompt_tokens: u.prompt_tokens,
                completion_tokens: u.completion_tokens,
                total_tokens: u.total_tokens,
            }),
            provider_name: self.provider_config.as_ref().map(|c| c.name.clone()),
            model_used: model.to_string(),
            response_time_ms: 0, // Will be set by caller
        })
    }

    /// Send a streaming request (simulating Codex CLI streaming behavior)
    pub async fn send_streaming_request(&mut self, message: &str, model: &str) -> Result<Vec<String>> {
        // Check if provider supports streaming
        if let Some(config) = &self.provider_config {
            if !config.supports_streaming {
                return Err(anyhow::anyhow!("Provider {} does not support streaming", config.name));
            }
        }

        // Add user message to conversation history
        let user_message = ChatMessage {
            role: MessageRole::User,
            content: message.to_string(),
            name: None,
            tool_calls: None,
            tool_call_id: None,
        };
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

        // Add tools if supported
        if self.provider_config.as_ref().map(|c| c.supports_tools).unwrap_or(true) {
            request.tools = Some(self.get_codex_tools());
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
        let mut stream = response.bytes_stream();
        let mut chunks = Vec::new();
        let mut accumulated_content = String::new();
        let mut tool_calls = Vec::new();

        while let Some(chunk_result) = stream.next().await {
            let chunk = chunk_result?;
            let chunk_str = String::from_utf8_lossy(&chunk);
            
            // Parse Server-Sent Events format
            for line in chunk_str.lines() {
                if line.starts_with("data: ") {
                    let data = &line[6..]; // Remove "data: " prefix
                    
                    if data == "[DONE]" {
                        break;
                    }
                    
                    if let Ok(json_data) = serde_json::from_str::<Value>(data) {
                        chunks.push(data.to_string());
                        
                        // Extract content and tool calls
                        if let Some(choices) = json_data.get("choices").and_then(|c| c.as_array()) {
                            if let Some(choice) = choices.first() {
                                if let Some(delta) = choice.get("delta") {
                                    if let Some(content) = delta.get("content").and_then(|c| c.as_str()) {
                                        accumulated_content.push_str(content);
                                    }
                                    
                                    if let Some(calls) = delta.get("tool_calls").and_then(|tc| tc.as_array()) {
                                        for call in calls {
                                            // Handle tool call parsing in streaming format
                                            if let Ok(tool_call) = serde_json::from_value::<ToolCall>(call.clone()) {
                                                tool_calls.push(tool_call);
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        // Add assistant response to conversation history if we got content
        if !accumulated_content.is_empty() || !tool_calls.is_empty() {
            let assistant_message = ChatMessage {
                role: MessageRole::Assistant,
                content: if accumulated_content.is_empty() { None } else { Some(accumulated_content) },
                name: None,
                tool_calls: if tool_calls.is_empty() { None } else { Some(tool_calls) },
                tool_call_id: None,
            };
            self.conversation_history.push(assistant_message);
        }

        Ok(chunks)
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
            let tool_message = ChatMessage {
                role: MessageRole::Tool,
                content: result.result.clone(),
                name: None,
                tool_calls: None,
                tool_call_id: Some(result.tool_call_id.clone()),
            };
            self.conversation_history.push(tool_message);
        }

        // Send follow-up request with tool results
        self.send_follow_up_request().await
    }

    /// Send a follow-up request after tool execution
    async fn send_follow_up_request(&mut self) -> Result<CodexResponse> {
        let model = self.provider_config.as_ref()
            .map(|c| c.default_model.clone())
            .unwrap_or_else(|| "gpt-4".to_string());
            
        let mut request = ChatCompletionRequest::new(&model, self.conversation_history.clone())
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
            usage: chat_response.usage.map(|u| Usage {
                prompt_tokens: u.prompt_tokens,
                completion_tokens: u.completion_tokens,
                total_tokens: u.total_tokens,
            }),
            provider_name: self.provider_config.as_ref().map(|c| c.name.clone()),
            model_used: model,
            response_time_ms: 0,
        })
    }

    /// Simulate tool execution (mimics what Codex CLI would do)
    async fn simulate_tool_execution(&self, tool_call: &ToolCall) -> Result<ToolExecutionResult> {
        let result = match tool_call.function.name.as_str() {
            "apply_patch" => {
                format!(
                    "Successfully applied patch with {} changes to file: {}",
                    self.count_patch_changes(&tool_call.function.arguments),
                    self.extract_file_path(&tool_call.function.arguments).unwrap_or("unknown")
                )
            }
            "read_file" => {
                let file_path = self.extract_file_path(&tool_call.function.arguments).unwrap_or("test.txt");
                format!("File content of {}:\n// This is simulated file content\nfunction example() {{\n    return 'Hello, World!';\n}}", file_path)
            }
            "search_files" => {
                "Found 3 matches:\n1. main.rs:15: function main()\n2. lib.rs:42: pub fn helper()\n3. test.rs:8: fn test_example()"
            }
            "write_file" => {
                let file_path = self.extract_file_path(&tool_call.function.arguments).unwrap_or("new_file.txt");
                format!("Successfully wrote {} bytes to {}", 
                    self.extract_content_length(&tool_call.function.arguments),
                    file_path)
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

    /// Get the common tools that Codex CLI uses
    fn get_codex_tools(&self) -> Vec<ToolDefinition> {
        vec![
            ToolDefinition {
                tool_type: "function".to_string(),
                function: FunctionDefinition {
                    name: "apply_patch".to_string(),
                    description: Some("Apply a unified diff patch to a file".to_string()),
                    parameters: json!({
                        "type": "object",
                        "properties": {
                            "file_path": {
                                "type": "string",
                                "description": "Path to the file to patch"
                            },
                            "patch": {
                                "type": "string",
                                "description": "Unified diff patch content"
                            }
                        },
                        "required": ["file_path", "patch"]
                    }),
                },
            },
            ToolDefinition {
                tool_type: "function".to_string(),
                function: FunctionDefinition {
                    name: "read_file".to_string(),
                    description: Some("Read the contents of a file".to_string()),
                    parameters: json!({
                        "type": "object",
                        "properties": {
                            "file_path": {
                                "type": "string",
                                "description": "Path to the file to read"
                            }
                        },
                        "required": ["file_path"]
                    }),
                },
            },
            ToolDefinition {
                tool_type: "function".to_string(),
                function: FunctionDefinition {
                    name: "search_files".to_string(),
                    description: Some("Search for patterns in files".to_string()),
                    parameters: json!({
                        "type": "object",
                        "properties": {
                            "pattern": {
                                "type": "string",
                                "description": "Pattern to search for"
                            },
                            "directory": {
                                "type": "string",
                                "description": "Directory to search in"
                            }
                        },
                        "required": ["pattern"]
                    }),
                },
            },
            ToolDefinition {
                tool_type: "function".to_string(),
                function: FunctionDefinition {
                    name: "write_file".to_string(),
                    description: Some("Write content to a file".to_string()),
                    parameters: json!({
                        "type": "object",
                        "properties": {
                            "file_path": {
                                "type": "string",
                                "description": "Path to the file to write"
                            },
                            "content": {
                                "type": "string",
                                "description": "Content to write to the file"
                            }
                        },
                        "required": ["file_path", "content"]
                    }),
                },
            },
        ]
    }

    /// Helper to extract file path from tool arguments
    fn extract_file_path(&self, arguments: &str) -> Option<String> {
        if let Ok(args) = serde_json::from_str::<Value>(arguments) {
            args.get("file_path")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string())
        } else {
            None
        }
    }

    /// Helper to count patch changes
    fn count_patch_changes(&self, arguments: &str) -> usize {
        if let Ok(args) = serde_json::from_str::<Value>(arguments) {
            if let Some(patch) = args.get("patch").and_then(|v| v.as_str()) {
                patch.lines().filter(|line| line.starts_with('+') || line.starts_with('-')).count()
            } else {
                0
            }
        } else {
            0
        }
    }

    /// Helper to extract content length
    fn extract_content_length(&self, arguments: &str) -> usize {
        if let Ok(args) = serde_json::from_str::<Value>(arguments) {
            if let Some(content) = args.get("content").and_then(|v| v.as_str()) {
                content.len()
            } else {
                0
            }
        } else {
            0
        }
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

/// Helper function to wait for streaming to complete
pub async fn wait_for_streaming_completion(
    chunks: Vec<String>,
    timeout_seconds: u64,
) -> Result<bool> {
    let timeout_duration = Duration::from_secs(timeout_seconds);
    
    timeout(timeout_duration, async {
        // Check if we received a [DONE] chunk or proper termination
        chunks.iter().any(|chunk| chunk.contains("[DONE]") || chunk.contains("\"finish_reason\""))
    })
    .await
    .map_err(|_| anyhow::anyhow!("Streaming completion timeout"))
}

/// Utility for cross-provider testing scenarios
pub struct CrossProviderTesting;

impl CrossProviderTesting {
    /// Test the same message across all providers and compare results
    pub async fn compare_providers(base_url: &str, message: &str) -> Result<HashMap<String, CodexResponse>> {
        let mut session = MultiProviderTestSession::new(base_url);
        session.test_all_providers(message).await
    }

    /// Test streaming across all providers that support it
    pub async fn compare_streaming(base_url: &str, message: &str) -> Result<HashMap<String, Vec<String>>> {
        let mut session = MultiProviderTestSession::new(base_url);
        session.test_streaming_all_providers(message).await
    }

    /// Get expected capabilities for provider validation
    pub fn get_expected_capabilities() -> HashMap<String, (bool, bool)> {
        ProviderConfig::all_providers()
            .into_iter()
            .map(|config| (config.name.clone(), (config.supports_streaming, config.supports_tools)))
            .collect()
    }
}