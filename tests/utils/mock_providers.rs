//! Mock AI providers for integration testing
//! 
//! This module provides mock implementations of all 8 AI providers with configurable behaviors.

use anyhow::Result;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use tokio::time::{sleep, Duration};

use crate::models::{
    request::ChatCompletionRequest,
    response::{ChatCompletionResponse, ChatCompletionChoice, ChatCompletionUsage},
    common::{ChatMessage, MessageRole, ToolCall, FunctionCall, FinishReason},
};

/// Configuration for mock provider behavior
#[derive(Debug, Clone)]
pub struct MockProviderConfig {
    pub name: String,
    pub should_fail: bool,
    pub failure_rate: f64,
    pub response_delay_ms: u64,
    pub max_response_tokens: u32,
    pub supports_streaming: bool,
    pub supports_tools: bool,
    pub custom_responses: HashMap<String, String>,
    pub model_prefix: String,
}

impl Default for MockProviderConfig {
    fn default() -> Self {
        Self {
            name: "mock".to_string(),
            should_fail: false,
            failure_rate: 0.0,
            response_delay_ms: 100,
            max_response_tokens: 4096,
            supports_streaming: true,
            supports_tools: true,
            custom_responses: HashMap::new(),
            model_prefix: "mock".to_string(),
        }
    }
}

/// Mock provider response behavior
#[derive(Debug, Clone)]
pub enum MockResponseBehavior {
    Success,
    AuthError,
    RateLimitError,
    TimeoutError,
    ServiceUnavailable,
    ModelNotFound,
    QuotaExceeded,
    CustomError(String),
}

/// Shared state for mock providers
#[derive(Debug)]
pub struct MockProviderState {
    pub request_count: u32,
    pub last_request: Option<ChatCompletionRequest>,
    pub response_behavior: MockResponseBehavior,
    pub custom_data: HashMap<String, Value>,
}

impl Default for MockProviderState {
    fn default() -> Self {
        Self {
            request_count: 0,
            last_request: None,
            response_behavior: MockResponseBehavior::Success,
            custom_data: HashMap::new(),
        }
    }
}

/// Generic mock provider
pub struct MockProvider {
    config: MockProviderConfig,
    state: Arc<Mutex<MockProviderState>>,
}

impl MockProvider {
    pub fn new(config: MockProviderConfig) -> Self {
        Self {
            config,
            state: Arc::new(Mutex::new(MockProviderState::default())),
        }
    }

    pub async fn chat_completions(&self, request: ChatCompletionRequest) -> Result<ChatCompletionResponse> {
        {
            if let Ok(mut state) = self.state.lock() {
                state.request_count += 1;
                state.last_request = Some(request.clone());
            }
        }

        if self.config.response_delay_ms > 0 {
            sleep(Duration::from_millis(self.config.response_delay_ms)).await;
        }

        let should_fail = {
            let state = self.state.lock().unwrap();
            !matches!(state.response_behavior, MockResponseBehavior::Success)
        };

        if should_fail {
            return self.generate_error_response().await;
        }

        self.generate_success_response(request).await
    }

    pub fn get_request_count(&self) -> u32 {
        self.state.lock().map(|s| s.request_count).unwrap_or(0)
    }

    pub fn set_response_behavior(&self, behavior: MockResponseBehavior) {
        if let Ok(mut state) = self.state.lock() {
            state.response_behavior = behavior;
        }
    }

    pub fn reset_state(&self) {
        if let Ok(mut state) = self.state.lock() {
            *state = MockProviderState::default();
        }
    }

    pub fn get_provider_name(&self) -> &str {
        &self.config.name
    }

    pub fn get_last_request(&self) -> Option<ChatCompletionRequest> {
        self.state.lock().ok()?.last_request.clone()
    }

    async fn generate_success_response(&self, request: ChatCompletionRequest) -> Result<ChatCompletionResponse> {
        let response_id = format!("chatcmpl-{}-{}", self.config.name, uuid::Uuid::new_v4());
        let model = format!("{}/{}", self.config.model_prefix, request.model);

        let content = self.generate_content_for_provider(&request);
        let tool_calls = if self.config.supports_tools && request.tools.is_some() {
            self.generate_tool_calls(&request)
        } else {
            None
        };

        let assistant_message = ChatMessage {
            role: MessageRole::Assistant,
            content: if tool_calls.is_some() { None } else { Some(content.clone()) },
            name: None,
            tool_calls,
            tool_call_id: None,
        };

        let choice = ChatCompletionChoice {
            index: 0,
            message: assistant_message,
            logprobs: None,
            finish_reason: Some(FinishReason::Stop),
        };

        let usage = ChatCompletionUsage {
            prompt_tokens: self.estimate_tokens(&request.messages),
            completion_tokens: if tool_calls.is_some() { 50 } else { content.split_whitespace().count() as u32 },
            total_tokens: 0,
        };

        let final_usage = ChatCompletionUsage {
            total_tokens: usage.prompt_tokens + usage.completion_tokens,
            ..usage
        };

        Ok(ChatCompletionResponse {
            id: response_id,
            object: "chat.completion".to_string(),
            created: chrono::Utc::now().timestamp() as u64,
            model,
            choices: vec![choice],
            usage: Some(final_usage),
            system_fingerprint: Some(format!("fp-{}-mock", self.config.name)),
        })
    }

    fn generate_content_for_provider(&self, request: &ChatCompletionRequest) -> String {
        if let Some(last_message) = request.messages.last() {
            if let Some(custom_response) = self.config.custom_responses.get(&last_message.content) {
                return custom_response.clone();
            }
        }

        match self.config.name.as_str() {
            "openai" => self.generate_openai_content(request),
            "anthropic" => self.generate_anthropic_content(request),
            "vertex" => self.generate_vertex_content(request),
            "groq" => self.generate_groq_content(request),
            "azure_openai" => self.generate_azure_content(request),
            "aws_bedrock" => self.generate_bedrock_content(request),
            "cohere" => self.generate_cohere_content(request),
            "mistral" => self.generate_mistral_content(request),
            _ => "I'm a mock AI provider. How can I help you?".to_string(),
        }
    }

    fn generate_openai_content(&self, request: &ChatCompletionRequest) -> String {
        if let Some(last_message) = request.messages.last() {
            match last_message.content.to_lowercase().as_str() {
                content if content.contains("code") => {
                    "I'll help you write that code. Let me create the implementation for you.".to_string()
                }
                content if content.contains("test") => {
                    "I'll generate comprehensive tests for that functionality.".to_string()
                }
                _ => "I understand your request. How can I assist you today?".to_string()
            }
        } else {
            "Hello! I'm OpenAI's assistant. How can I help you?".to_string()
        }
    }

    fn generate_anthropic_content(&self, request: &ChatCompletionRequest) -> String {
        if let Some(last_message) = request.messages.last() {
            match last_message.content.to_lowercase().as_str() {
                content if content.contains("analyze") => {
                    "I'll analyze this carefully and provide a thorough breakdown of the key components.".to_string()
                }
                content if content.contains("explain") => {
                    "I'd be happy to explain this concept. Let me break it down into clear parts.".to_string()
                }
                _ => "I understand what you're looking for. Let me provide a thoughtful response.".to_string()
            }
        } else {
            "Hello! I'm Claude, an AI assistant created by Anthropic. How can I help you?".to_string()
        }
    }

    fn generate_vertex_content(&self, _request: &ChatCompletionRequest) -> String {
        "I'm a Google Vertex AI model. I can help with AI tasks using Google's advanced capabilities.".to_string()
    }

    fn generate_groq_content(&self, _request: &ChatCompletionRequest) -> String {
        "⚡ Fast Groq response: I'm optimized for speed and efficiency. Here's your quick answer.".to_string()
    }

    fn generate_azure_content(&self, _request: &ChatCompletionRequest) -> String {
        "I'm an Azure OpenAI service model, providing enterprise-grade AI capabilities with Microsoft's security features.".to_string()
    }

    fn generate_bedrock_content(&self, _request: &ChatCompletionRequest) -> String {
        "I'm an AWS Bedrock model, providing secure and scalable AI capabilities on Amazon's infrastructure.".to_string()
    }

    fn generate_cohere_content(&self, _request: &ChatCompletionRequest) -> String {
        "I'm a Cohere model, designed for enterprise AI applications with advanced language understanding capabilities.".to_string()
    }

    fn generate_mistral_content(&self, _request: &ChatCompletionRequest) -> String {
        "I'm a Mistral AI model, offering European-compliant AI solutions with cost-effective performance.".to_string()
    }

    fn generate_tool_calls(&self, request: &ChatCompletionRequest) -> Option<Vec<ToolCall>> {
        if let Some(tools) = &request.tools {
            if let Some(last_message) = request.messages.last() {
                let content = last_message.content.to_lowercase();
                let mut tool_calls = Vec::new();
                
                if content.contains("read") && tools.iter().any(|t| t.function.name == "read_file") {
                    tool_calls.push(ToolCall {
                        id: format!("call_{}", uuid::Uuid::new_v4()),
                        tool_type: "function".to_string(),
                        function: FunctionCall {
                            name: "read_file".to_string(),
                            arguments: json!({"file_path": "src/main.rs"}).to_string(),
                        },
                    });
                }
                
                if content.contains("patch") && tools.iter().any(|t| t.function.name == "apply_patch") {
                    tool_calls.push(ToolCall {
                        id: format!("call_{}", uuid::Uuid::new_v4()),
                        tool_type: "function".to_string(),
                        function: FunctionCall {
                            name: "apply_patch".to_string(),
                            arguments: json!({
                                "file_path": "src/main.rs",
                                "patch": "--- a/src/main.rs\n+++ b/src/main.rs\n@@ -1,3 +1,7 @@\n+fn hello() {\n+    println!(\"Hello!\");\n+}\n+\n fn main() {\n     println!(\"Hello, world!\");\n }"
                            }).to_string(),
                        },
                    });
                }
                
                if !tool_calls.is_empty() {
                    return Some(tool_calls);
                }
            }
        }
        None
    }

    fn estimate_tokens(&self, messages: &[ChatMessage]) -> u32 {
        messages.iter()
            .map(|msg| msg.content.split_whitespace().count() as u32 * 1.3)
            .sum::<u32>()
            .min(self.config.max_response_tokens)
    }

    async fn generate_error_response(&self) -> Result<ChatCompletionResponse> {
        let behavior = {
            let state = self.state.lock().unwrap();
            state.response_behavior.clone()
        };

        match behavior {
            MockResponseBehavior::AuthError => {
                Err(anyhow::anyhow!("{} authentication failed: Invalid API key", self.config.name))
            }
            MockResponseBehavior::RateLimitError => {
                Err(anyhow::anyhow!("{} rate limit exceeded", self.config.name))
            }
            MockResponseBehavior::ModelNotFound => {
                Err(anyhow::anyhow!("{} model not found", self.config.name))
            }
            MockResponseBehavior::QuotaExceeded => {
                Err(anyhow::anyhow!("{} quota exceeded", self.config.name))
            }
            MockResponseBehavior::TimeoutError => {
                Err(anyhow::anyhow!("{} request timeout", self.config.name))
            }
            MockResponseBehavior::ServiceUnavailable => {
                Err(anyhow::anyhow!("{} service unavailable", self.config.name))
            }
            MockResponseBehavior::CustomError(msg) => {
                Err(anyhow::anyhow!("{} error: {}", self.config.name, msg))
            }
            MockResponseBehavior::Success => {
                unreachable!("This should not be called for success behavior")
            }
        }
    }
}

/// Mock failing provider for testing fallback scenarios
pub struct MockFailingProvider {
    config: MockProviderConfig,
    state: Arc<Mutex<MockProviderState>>,
}

impl MockFailingProvider {
    pub fn new() -> Self {
        let mut config = MockProviderConfig::default();
        config.name = "failing".to_string();
        config.should_fail = true;
        config.failure_rate = 1.0;
        
        Self {
            config,
            state: Arc::new(Mutex::new(MockProviderState {
                response_behavior: MockResponseBehavior::ServiceUnavailable,
                ..Default::default()
            })),
        }
    }

    pub async fn chat_completions(&self, request: ChatCompletionRequest) -> Result<ChatCompletionResponse> {
        {
            if let Ok(mut state) = self.state.lock() {
                state.request_count += 1;
                state.last_request = Some(request);
            }
        }

        Err(anyhow::anyhow!("Mock provider always fails"))
    }

    pub fn get_request_count(&self) -> u32 {
        self.state.lock().map(|s| s.request_count).unwrap_or(0)
    }
}

/// Factory for creating preconfigured mock providers
pub struct MockProviderFactory;

impl MockProviderFactory {
    /// Create OpenAI mock provider
    pub fn create_openai() -> MockProvider {
        let config = MockProviderConfig {
            name: "openai".to_string(),
            model_prefix: "gpt".to_string(),
            response_delay_ms: 150,
            supports_streaming: true,
            supports_tools: true,
            ..Default::default()
        };
        MockProvider::new(config)
    }

    /// Create Anthropic Claude mock provider
    pub fn create_anthropic() -> MockProvider {
        let config = MockProviderConfig {
            name: "anthropic".to_string(),
            model_prefix: "claude".to_string(),
            response_delay_ms: 200,
            supports_streaming: true,
            supports_tools: true,
            ..Default::default()
        };
        MockProvider::new(config)
    }

    /// Create Vertex AI mock provider
    pub fn create_vertex() -> MockProvider {
        let config = MockProviderConfig {
            name: "vertex".to_string(),
            model_prefix: "vertex".to_string(),
            response_delay_ms: 180,
            supports_streaming: true,
            supports_tools: true,
            ..Default::default()
        };
        MockProvider::new(config)
    }

    /// Create Groq mock provider (fast)
    pub fn create_groq() -> MockProvider {
        let config = MockProviderConfig {
            name: "groq".to_string(),
            model_prefix: "groq".to_string(),
            response_delay_ms: 50, // Very fast
            supports_streaming: true,
            supports_tools: false, // Limited tool support
            ..Default::default()
        };
        MockProvider::new(config)
    }

    /// Create Azure OpenAI mock provider
    pub fn create_azure_openai() -> MockProvider {
        let config = MockProviderConfig {
            name: "azure_openai".to_string(),
            model_prefix: "azure-gpt".to_string(),
            response_delay_ms: 160,
            supports_streaming: true,
            supports_tools: true,
            ..Default::default()
        };
        MockProvider::new(config)
    }

    /// Create AWS Bedrock mock provider
    pub fn create_aws_bedrock() -> MockProvider {
        let config = MockProviderConfig {
            name: "aws_bedrock".to_string(),
            model_prefix: "bedrock".to_string(),
            response_delay_ms: 220,
            supports_streaming: true,
            supports_tools: true,
            ..Default::default()
        };
        MockProvider::new(config)
    }

    /// Create Cohere mock provider
    pub fn create_cohere() -> MockProvider {
        let config = MockProviderConfig {
            name: "cohere".to_string(),
            model_prefix: "command".to_string(),
            response_delay_ms: 170,
            supports_streaming: true,
            supports_tools: true,
            ..Default::default()
        };
        MockProvider::new(config)
    }

    /// Create Mistral AI mock provider
    pub fn create_mistral() -> MockProvider {
        let config = MockProviderConfig {
            name: "mistral".to_string(),
            model_prefix: "mistral".to_string(),
            response_delay_ms: 140,
            supports_streaming: true,
            supports_tools: true,
            ..Default::default()
        };
        MockProvider::new(config)
    }

    /// Create all 8 providers for comprehensive testing
    pub fn create_all_providers() -> HashMap<String, MockProvider> {
        let mut providers = HashMap::new();
        
        providers.insert("openai".to_string(), Self::create_openai());
        providers.insert("anthropic".to_string(), Self::create_anthropic());
        providers.insert("vertex".to_string(), Self::create_vertex());
        providers.insert("groq".to_string(), Self::create_groq());
        providers.insert("azure_openai".to_string(), Self::create_azure_openai());
        providers.insert("aws_bedrock".to_string(), Self::create_aws_bedrock());
        providers.insert("cohere".to_string(), Self::create_cohere());
        providers.insert("mistral".to_string(), Self::create_mistral());
        
        providers
    }

    /// Create a slow provider for timeout testing
    pub fn create_slow_provider() -> MockProvider {
        let config = MockProviderConfig {
            name: "slow".to_string(),
            model_prefix: "slow".to_string(),
            response_delay_ms: 5000, // 5 second delay
            ..Default::default()
        };
        MockProvider::new(config)
    }

    /// Create an unreliable provider for fallback testing
    pub fn create_unreliable_provider() -> MockProvider {
        let config = MockProviderConfig {
            name: "unreliable".to_string(),
            model_prefix: "unreliable".to_string(),
            failure_rate: 0.7, // 70% failure rate
            ..Default::default()
        };
        MockProvider::new(config)
    }

    /// Create providers with specific capabilities for testing
    pub fn create_provider_with_capabilities(name: &str, supports_streaming: bool, supports_tools: bool) -> MockProvider {
        let config = MockProviderConfig {
            name: name.to_string(),
            model_prefix: name.to_string(),
            supports_streaming,
            supports_tools,
            ..Default::default()
        };
        MockProvider::new(config)
    }
}

/// Provider capability test helpers
pub struct ProviderCapabilityMatrix;

impl ProviderCapabilityMatrix {
    /// Get expected capabilities for each provider
    pub fn get_provider_capabilities() -> HashMap<String, (bool, bool)> {
        let mut capabilities = HashMap::new();
        
        // (supports_streaming, supports_tools)
        capabilities.insert("openai".to_string(), (true, true));
        capabilities.insert("anthropic".to_string(), (true, true));
        capabilities.insert("vertex".to_string(), (true, true));
        capabilities.insert("groq".to_string(), (true, false)); // Fast but limited tools
        capabilities.insert("azure_openai".to_string(), (true, true));
        capabilities.insert("aws_bedrock".to_string(), (true, true));
        capabilities.insert("cohere".to_string(), (true, true));
        capabilities.insert("mistral".to_string(), (true, true));
        
        capabilities
    }

    /// Get expected response times (in ms) for each provider
    pub fn get_provider_response_times() -> HashMap<String, u64> {
        let mut response_times = HashMap::new();
        
        response_times.insert("openai".to_string(), 150);
        response_times.insert("anthropic".to_string(), 200);
        response_times.insert("vertex".to_string(), 180);
        response_times.insert("groq".to_string(), 50); // Fastest
        response_times.insert("azure_openai".to_string(), 160);
        response_times.insert("aws_bedrock".to_string(), 220);
        response_times.insert("cohere".to_string(), 170);
        response_times.insert("mistral".to_string(), 140);
        
        response_times
    }

    /// Get provider-specific model names for testing
    pub fn get_provider_test_models() -> HashMap<String, Vec<String>> {
        let mut models = HashMap::new();
        
        models.insert("openai".to_string(), vec![
            "gpt-4".to_string(),
            "gpt-3.5-turbo".to_string(),
        ]);
        
        models.insert("anthropic".to_string(), vec![
            "claude-3-opus-20240229".to_string(),
            "claude-3-sonnet-20240229".to_string(),
        ]);
        
        models.insert("vertex".to_string(), vec![
            "gemini-pro".to_string(),
            "claude-3-sonnet@20240229".to_string(),
        ]);
        
        models.insert("groq".to_string(), vec![
            "mixtral-8x7b-32768".to_string(),
            "llama2-70b-4096".to_string(),
        ]);
        
        models.insert("azure_openai".to_string(), vec![
            "gpt-4".to_string(),
            "gpt-35-turbo".to_string(),
        ]);
        
        models.insert("aws_bedrock".to_string(), vec![
            "anthropic.claude-3-sonnet-20240229-v1:0".to_string(),
            "meta.llama2-70b-chat-v1".to_string(),
        ]);
        
        models.insert("cohere".to_string(), vec![
            "command".to_string(),
            "command-light".to_string(),
        ]);
        
        models.insert("mistral".to_string(), vec![
            "mistral-large-latest".to_string(),
            "mistral-medium-latest".to_string(),
        ]);
        
        models
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::common::ChatMessage;

    #[tokio::test]
    async fn test_all_providers_creation() {
        let providers = MockProviderFactory::create_all_providers();
        assert_eq!(providers.len(), 8);
        
        let expected_providers = vec![
            "openai", "anthropic", "vertex", "groq", 
            "azure_openai", "aws_bedrock", "cohere", "mistral"
        ];
        
        for provider_name in expected_providers {
            assert!(providers.contains_key(provider_name));
        }
    }

    #[tokio::test]
    async fn test_provider_capabilities() {
        let capabilities = ProviderCapabilityMatrix::get_provider_capabilities();
        
        // Test Groq has limited tool support
        let groq_caps = capabilities.get("groq").unwrap();
        assert_eq!(groq_caps.0, true);  // supports streaming
        assert_eq!(groq_caps.1, false); // limited tool support
        
        // Test OpenAI has full capabilities
        let openai_caps = capabilities.get("openai").unwrap();
        assert_eq!(openai_caps.0, true);  // supports streaming
        assert_eq!(openai_caps.1, true);  // supports tools
    }

    #[tokio::test]
    async fn test_mock_openai_provider() {
        let provider = MockProviderFactory::create_openai();
        
        let request = ChatCompletionRequest::new(
            "gpt-4",
            vec![ChatMessage::user("Hello, world!")]
        );

        let response = provider.chat_completions(request).await.unwrap();
        
        assert_eq!(response.choices.len(), 1);
        assert!(response.choices[0].message.content.is_some());
        assert_eq!(provider.get_request_count(), 1);
        assert!(response.id.contains("openai"));
    }

    #[tokio::test]
    async fn test_mock_anthropic_provider() {
        let provider = MockProviderFactory::create_anthropic();
        
        let request = ChatCompletionRequest::new(
            "claude-3-sonnet-20240229",
            vec![ChatMessage::user("Analyze this code")]
        );

        let response = provider.chat_completions(request).await.unwrap();
        
        assert_eq!(response.choices.len(), 1);
        assert!(response.choices[0].message.content.as_ref().unwrap().contains("analyze"));
        assert_eq!(provider.get_request_count(), 1);
        assert!(response.id.contains("anthropic"));
    }

    #[tokio::test]
    async fn test_mock_groq_provider_speed() {
        use std::time::Instant;
        
        let provider = MockProviderFactory::create_groq();
        
        let request = ChatCompletionRequest::new(
            "mixtral-8x7b-32768",
            vec![ChatMessage::user("Fast response please")]
        );

        let start = Instant::now();
        let response = provider.chat_completions(request).await.unwrap();
        let duration = start.elapsed();
        
        assert!(duration.as_millis() < 100); // Should be very fast
        assert!(response.choices[0].message.content.as_ref().unwrap().contains("Fast"));
    }

    #[tokio::test]
    async fn test_mock_failing_provider() {
        let provider = MockFailingProvider::new();
        
        let request = ChatCompletionRequest::new(
            "any-model",
            vec![ChatMessage::user("This should fail")]
        );

        let result = provider.chat_completions(request).await;
        assert!(result.is_err());
        assert_eq!(provider.get_request_count(), 1);
    }

    #[tokio::test]
    async fn test_response_behavior() {
        let provider = MockProviderFactory::create_vertex();
        provider.set_response_behavior(MockResponseBehavior::RateLimitError);
        
        let request = ChatCompletionRequest::new(
            "gemini-pro",
            vec![ChatMessage::user("Test")]
        );

        let result = provider.chat_completions(request).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("rate limit"));
    }

    #[tokio::test]
    async fn test_provider_model_mapping() {
        let models = ProviderCapabilityMatrix::get_provider_test_models();
        
        // Test each provider has expected models
        assert!(models.get("openai").unwrap().contains(&"gpt-4".to_string()));
        assert!(models.get("anthropic").unwrap().contains(&"claude-3-opus-20240229".to_string()));
        assert!(models.get("groq").unwrap().contains(&"mixtral-8x7b-32768".to_string()));
    }
}