// src/models/request.rs
//
// OpenAI API compatible request models

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use super::common::{ChatMessage, FunctionDefinition, ResponseFormat, ToolDefinition};

/// Chat completion request structure
/// Matches OpenAI API v1/chat/completions specification exactly
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatCompletionRequest {
    /// ID of the model to use (claude-4-sonnet-20250514)
    pub model: String,

    /// List of messages comprising the conversation so far
    pub messages: Vec<ChatMessage>,

    /// Maximum number of tokens to generate (1-200,000)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_tokens: Option<u32>,

    /// Sampling temperature between 0 and 2 (default: 1)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f64>,

    /// Nucleus sampling parameter between 0 and 1 (default: 1)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_p: Option<f64>,

    /// Number of chat completion choices to generate (default: 1)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub n: Option<u32>,

    /// Whether to stream back partial message deltas
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stream: Option<bool>,

    /// Stream options for streaming requests
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stream_options: Option<StreamOptions>,

    /// Up to 4 sequences where the API will stop generating
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stop: Option<Stop>,

    /// Presence penalty between -2.0 and 2.0
    #[serde(skip_serializing_if = "Option::is_none")]
    pub presence_penalty: Option<f64>,

    /// Frequency penalty between -2.0 and 2.0
    #[serde(skip_serializing_if = "Option::is_none")]
    pub frequency_penalty: Option<f64>,

    /// Modify likelihood of specified tokens appearing
    #[serde(skip_serializing_if = "Option::is_none")]
    pub logit_bias: Option<HashMap<String, f64>>,

    /// Whether to return log probabilities
    #[serde(skip_serializing_if = "Option::is_none")]
    pub logprobs: Option<bool>,

    /// Number of most likely tokens to return (0-20)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_logprobs: Option<u32>,

    /// Unique identifier representing your end-user
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user: Option<String>,

    /// Deprecated: list of functions the model may generate JSON inputs for
    #[serde(skip_serializing_if = "Option::is_none")]
    pub functions: Option<Vec<FunctionDefinition>>,

    /// Deprecated: controls which function is called by the model
    #[serde(skip_serializing_if = "Option::is_none")]
    pub function_call: Option<serde_json::Value>,

    /// List of tools the model may call
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tools: Option<Vec<ToolDefinition>>,

    /// Controls which tool is called by the model
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_choice: Option<serde_json::Value>,

    /// Whether to enable parallel tool calls
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parallel_tool_calls: Option<bool>,

    /// Format that the model must output
    #[serde(skip_serializing_if = "Option::is_none")]
    pub response_format: Option<ResponseFormat>,

    /// Random seed for deterministic sampling
    #[serde(skip_serializing_if = "Option::is_none")]
    pub seed: Option<i64>,

    /// Developer-defined tags for filtering
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<HashMap<String, String>>,

    /// Service tier for processing (auto, default, or null)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub service_tier: Option<String>,

    /// Whether to store the conversation for future reference
    #[serde(skip_serializing_if = "Option::is_none")]
    pub store: Option<bool>,
}

/// Stream options for streaming chat completions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamOptions {
    /// Whether to include usage statistics in streaming
    #[serde(skip_serializing_if = "Option::is_none")]
    pub include_usage: Option<bool>,
}

/// Stop sequences - either a string or array of strings
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Stop {
    String(String),
    Array(Vec<String>),
}

impl ChatCompletionRequest {
    /// Create a new chat completion request with required fields
    pub fn new<S: Into<String>>(model: S, messages: Vec<ChatMessage>) -> Self {
        Self {
            model: model.into(),
            messages,
            max_tokens: None,
            temperature: None,
            top_p: None,
            n: None,
            stream: None,
            stream_options: None,
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
        }
    }

    /// Set max_tokens for the request
    pub fn with_max_tokens(mut self, max_tokens: u32) -> Self {
        self.max_tokens = Some(max_tokens);
        self
    }

    /// Set temperature for the request
    pub fn with_temperature(mut self, temperature: f64) -> Self {
        self.temperature = Some(temperature);
        self
    }

    /// Set streaming mode
    pub fn with_stream(mut self, stream: bool) -> Self {
        self.stream = Some(stream);
        self
    }

    /// Set top_p for the request
    pub fn with_top_p(mut self, top_p: f64) -> Self {
        self.top_p = Some(top_p);
        self
    }

    /// Set stop sequences
    pub fn with_stop<S: Into<String>>(mut self, stop: S) -> Self {
        self.stop = Some(Stop::String(stop.into()));
        self
    }

    /// Set multiple stop sequences
    pub fn with_stop_array<S: Into<String>>(mut self, stops: Vec<S>) -> Self {
        let stop_strings: Vec<String> = stops.into_iter().map(|s| s.into()).collect();
        self.stop = Some(Stop::Array(stop_strings));
        self
    }

    /// Enable or disable streaming
    pub fn streaming(mut self, enabled: bool) -> Self {
        self.stream = Some(enabled);
        if enabled {
            self.stream_options = Some(StreamOptions {
                include_usage: Some(true),
            });
        }
        self
    }

    /// Validate request parameters according to OpenAI API constraints
    pub fn validate(&self) -> Result<(), String> {
        // Model validation
        if self.model.is_empty() {
            return Err("Model cannot be empty".to_string());
        }

        // Messages validation
        if self.messages.is_empty() {
            return Err("Messages array cannot be empty".to_string());
        }

        // Max tokens validation (1 to 200,000)
        if let Some(max_tokens) = self.max_tokens {
            if max_tokens == 0 || max_tokens > 200_000 {
                return Err("max_tokens must be between 1 and 200,000".to_string());
            }
        }

        // Temperature validation (0 to 2.0)
        if let Some(temperature) = self.temperature {
            if !(0.0..=2.0).contains(&temperature) {
                return Err("temperature must be between 0.0 and 2.0".to_string());
            }
        }

        // Top_p validation (0 to 1.0)
        if let Some(top_p) = self.top_p {
            if !(0.0..=1.0).contains(&top_p) {
                return Err("top_p must be between 0.0 and 1.0".to_string());
            }
        }

        // N validation (1 to 128)
        if let Some(n) = self.n {
            if n == 0 || n > 128 {
                return Err("n must be between 1 and 128".to_string());
            }
        }

        // Presence penalty validation (-2.0 to 2.0)
        if let Some(penalty) = self.presence_penalty {
            if !(-2.0..=2.0).contains(&penalty) {
                return Err("presence_penalty must be between -2.0 and 2.0".to_string());
            }
        }

        // Frequency penalty validation (-2.0 to 2.0)
        if let Some(penalty) = self.frequency_penalty {
            if !(-2.0..=2.0).contains(&penalty) {
                return Err("frequency_penalty must be between -2.0 and 2.0".to_string());
            }
        }

        // Top logprobs validation (0 to 20)
        if let Some(top_logprobs) = self.top_logprobs {
            if top_logprobs > 20 {
                return Err("top_logprobs must be between 0 and 20".to_string());
            }
        }

        Ok(())
    }

    /// Check if this is a streaming request
    pub fn is_streaming(&self) -> bool {
        self.stream.unwrap_or(false)
    }

    /// Get the effective max_tokens (with default if not specified)
    pub fn effective_max_tokens(&self) -> u32 {
        self.max_tokens.unwrap_or(16_384) // Default max tokens
    }

    /// Get the effective temperature (with default if not specified)
    pub fn effective_temperature(&self) -> f64 {
        self.temperature.unwrap_or(1.0) // Default temperature
    }

    /// Get the effective top_p (with default if not specified)
    pub fn effective_top_p(&self) -> f64 {
        self.top_p.unwrap_or(1.0) // Default top_p
    }

    /// Get the effective n (with default if not specified)
    pub fn effective_n(&self) -> u32 {
        self.n.unwrap_or(1) // Default number of choices
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::common::{ChatMessage, MessageRole};

    #[test]
    fn test_basic_request_creation() {
        let messages = vec![ChatMessage::user("Hello, world!")];
        let request = ChatCompletionRequest::new("claude-4-sonnet-20250514", messages);

        assert_eq!(request.model, "claude-4-sonnet-20250514");
        assert_eq!(request.messages.len(), 1);
        assert_eq!(request.messages[0].content, "Hello, world!");
    }

    #[test]
    fn test_builder_pattern() {
        let messages = vec![ChatMessage::user("Hello")];
        let request = ChatCompletionRequest::new("claude-4-sonnet-20250514", messages)
            .with_max_tokens(100)
            .with_temperature(0.7)
            .with_stream(true);

        assert_eq!(request.max_tokens, Some(100));
        assert_eq!(request.temperature, Some(0.7));
        assert_eq!(request.stream, Some(true));
        assert!(request.is_streaming());
    }

    #[test]
    fn test_validation_empty_messages() {
        let request = ChatCompletionRequest::new("claude-4-sonnet-20250514", vec![]);
        assert!(request.validate().is_err());
    }

    #[test]
    fn test_validation_max_tokens() {
        let messages = vec![ChatMessage::user("Hello")];
        let mut request = ChatCompletionRequest::new("claude-4-sonnet-20250514", messages);

        // Valid max_tokens
        request.max_tokens = Some(1000);
        assert!(request.validate().is_ok());

        // Invalid max_tokens (too high)
        request.max_tokens = Some(300_000);
        assert!(request.validate().is_err());

        // Invalid max_tokens (zero)
        request.max_tokens = Some(0);
        assert!(request.validate().is_err());
    }

    #[test]
    fn test_validation_temperature() {
        let messages = vec![ChatMessage::user("Hello")];
        let mut request = ChatCompletionRequest::new("claude-4-sonnet-20250514", messages);

        // Valid temperature
        request.temperature = Some(0.7);
        assert!(request.validate().is_ok());

        // Invalid temperature (too high)
        request.temperature = Some(3.0);
        assert!(request.validate().is_err());

        // Invalid temperature (negative)
        request.temperature = Some(-1.0);
        assert!(request.validate().is_err());
    }

    #[test]
    fn test_stop_sequences() {
        let messages = vec![ChatMessage::user("Hello")];
        let request = ChatCompletionRequest::new("claude-4-sonnet-20250514", messages)
            .with_stop("\\n");

        if let Some(Stop::String(stop_str)) = &request.stop {
            assert_eq!(stop_str, "\\n");
        } else {
            panic!("Expected Stop::String");
        }

        let request2 = ChatCompletionRequest::new("claude-4-sonnet-20250514", vec![ChatMessage::user("Hello")])
            .with_stop_array(vec!["\\n", "\\n\\n"]);

        if let Some(Stop::Array(stops)) = &request2.stop {
            assert_eq!(stops.len(), 2);
            assert_eq!(stops[0], "\\n");
            assert_eq!(stops[1], "\\n\\n");
        } else {
            panic!("Expected Stop::Array");
        }
    }

    #[test]
    fn test_serialization() {
        let messages = vec![
            ChatMessage::system("You are a helpful assistant"),
            ChatMessage::user("Hello!"),
        ];
        let request = ChatCompletionRequest::new("claude-4-sonnet-20250514", messages)
            .with_max_tokens(100)
            .with_temperature(0.7)
            .streaming(true);

        let json = serde_json::to_string(&request).unwrap();

        // Should contain all the fields we set
        assert!(json.contains("claude-4-sonnet-20250514"));
        assert!(json.contains("\"max_tokens\":100"));
        assert!(json.contains("\"temperature\":0.7"));
        assert!(json.contains("\"stream\":true"));

        // Should be deserializable back
        let deserialized: ChatCompletionRequest = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.model, "claude-4-sonnet-20250514");
        assert_eq!(deserialized.max_tokens, Some(100));
        assert_eq!(deserialized.temperature, Some(0.7));
        assert_eq!(deserialized.stream, Some(true));
    }

    #[test]
    fn test_defaults() {
        let messages = vec![ChatMessage::user("Hello")];
        let request = ChatCompletionRequest::new("claude-4-sonnet-20250514", messages);

        assert_eq!(request.effective_max_tokens(), 16_384);
        assert_eq!(request.effective_temperature(), 1.0);
        assert_eq!(request.effective_top_p(), 1.0);
        assert_eq!(request.effective_n(), 1);
        assert!(!request.is_streaming());
    }
}