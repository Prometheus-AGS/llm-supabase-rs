// src/infrastructure/vertex/types.rs
//
// Vertex AI API types for Claude models (Anthropic Messages API format)

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Helper function to skip serializing stream field when false
fn should_skip_stream(stream: &bool) -> bool {
    !stream
}

/// Vertex AI request for Claude models (Anthropic Messages API format)
/// This matches the format expected by Vertex AI's Anthropic endpoint
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VertexPredictRequest {
    /// Required: Anthropic API version for Vertex AI
    pub anthropic_version: String,
    
    /// The conversation messages
    pub messages: Vec<VertexMessage>,

    /// Maximum tokens to generate
    pub max_tokens: u32,

    /// System prompt (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub system: Option<String>,

    /// Sampling temperature (0.0 to 1.0)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f64>,

    /// Top-p nucleus sampling (0.0 to 1.0)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_p: Option<f64>,

    /// Top-k sampling (1 to 40)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_k: Option<u32>,

    /// List of stop sequences
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stop_sequences: Option<Vec<String>>,

    /// Tools available for the model to use
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tools: Option<Vec<VertexTool>>,

    /// Tool choice configuration
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_choice: Option<VertexToolChoice>,

    /// Whether to stream the response (required for streaming endpoint)
    #[serde(skip_serializing_if = "should_skip_stream")]
    pub stream: bool,
}

/// Tool definition for Vertex AI (Anthropic format)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VertexTool {
    pub name: String,
    pub description: String,
    pub input_schema: Value,
}

/// Tool choice configuration for Vertex AI (Anthropic format)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum VertexToolChoice {
    /// Auto tool choice - serializes as object with type "auto"
    Auto {
        #[serde(rename = "type")]
        tool_type: String
    },
    /// Any tool choice (required) - serializes as object with type "any"
    Any {
        #[serde(rename = "type")]
        tool_type: String
    },
    /// Specific tool choice - serializes as object with type "tool" and name
    Tool {
        #[serde(rename = "type")]
        tool_type: String,
        name: String
    },
}

/// Message format for Claude in Vertex AI
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VertexMessage {
    pub role: VertexRole,
    pub content: String,
}

/// Message roles supported by Claude
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum VertexRole {
    User,
    Assistant,
}

/// Vertex AI response from Claude models
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VertexPredictResponse {
    pub id: String,
    
    #[serde(rename = "type")]
    pub response_type: String,
    
    pub role: String,
    
    pub content: Vec<VertexContent>,
    
    pub model: String,
    
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stop_reason: Option<String>,
    
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stop_sequence: Option<String>,
    
    pub usage: VertexUsage,
}

impl VertexPredictResponse {
    /// Get text content from the response, combining all text blocks
    pub fn get_text(&self) -> String {
        self.content
            .iter()
            .filter_map(|content| content.get_text())
            .collect::<Vec<&str>>()
            .join("")
    }
    
    /// Get all tool use blocks from the response
    pub fn get_tool_uses(&self) -> Vec<&VertexContent> {
        self.content
            .iter()
            .filter(|content| content.is_tool_use())
            .collect()
    }
}

/// Content block in the response
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum VertexContent {
    #[serde(rename = "text")]
    Text {
        text: String,
    },
    #[serde(rename = "tool_use")]
    ToolUse {
        id: String,
        name: String,
        input: Value,
    },
}

impl VertexContent {
    /// Get text content if this is a text block
    pub fn get_text(&self) -> Option<&str> {
        match self {
            VertexContent::Text { text } => Some(text),
            _ => None,
        }
    }
    
    /// Check if this is a tool use block
    pub fn is_tool_use(&self) -> bool {
        matches!(self, VertexContent::ToolUse { .. })
    }
}

/// Token usage statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VertexUsage {
    pub input_tokens: u32,
    pub output_tokens: u32,
}

/// Individual prediction result (for internal use)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VertexPrediction {
    /// Generated content from Claude
    pub content: String,

    /// Token usage information
    #[serde(skip_serializing_if = "Option::is_none")]
    pub usage: Option<VertexUsageInternal>,

    /// Safety ratings (if applicable)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub safety_ratings: Option<Vec<VertexSafetyRating>>,

    /// Finish reason
    #[serde(skip_serializing_if = "Option::is_none")]
    pub finish_reason: Option<VertexFinishReason>,
}

/// Internal usage format for backward compatibility
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VertexUsageInternal {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub input_tokens: Option<u32>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub output_tokens: Option<u32>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub total_tokens: Option<u32>,
}

/// Safety rating for content filtering
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VertexSafetyRating {
    pub category: String,
    pub probability: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub blocked: Option<bool>,
}

/// Reason why generation finished
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VertexFinishReason {
    /// Natural stop
    #[serde(rename = "end_turn")]
    Stop,
    /// Maximum tokens reached
    #[serde(rename = "max_tokens")]
    MaxTokens,
    /// Safety filter triggered
    Safety,
    /// Recitation filter triggered
    Recitation,
    /// Other reason
    Other,
}

/// Vertex AI Streaming response chunk
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VertexStreamChunk {
    #[serde(rename = "type")]
    pub event_type: String,
    
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    
    #[serde(skip_serializing_if = "Option::is_none")]
    pub role: Option<String>,
    
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<Vec<VertexContent>>,
    
    #[serde(skip_serializing_if = "Option::is_none")]
    pub index: Option<usize>,
    
    #[serde(skip_serializing_if = "Option::is_none")]
    pub delta: Option<VertexDelta>,
    
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<VertexStreamMessage>,
    
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content_block: Option<VertexContentBlock>,
    
    #[serde(skip_serializing_if = "Option::is_none")]
    pub usage: Option<VertexUsage>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VertexDelta {
    #[serde(rename = "type")]
    pub delta_type: String,
    
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stop_reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VertexStreamMessage {
    pub id: String,
    
    #[serde(rename = "type")]
    pub message_type: String,
    
    pub role: String,
    
    pub content: Vec<serde_json::Value>,
    
    pub model: String,
    
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stop_reason: Option<String>,
    
    pub usage: VertexUsage,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VertexContentBlock {
    #[serde(rename = "type")]
    pub block_type: String,
    
    pub text: String,
}

/// Vertex AI error response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VertexError {
    #[serde(rename = "type")]
    pub error_type: String,
    
    pub error: VertexErrorDetails,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VertexErrorDetails {
    #[serde(rename = "type")]
    pub error_type: String,
    
    pub message: String,
}

impl std::fmt::Display for VertexError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Vertex AI Error ({}): {}", self.error_type, self.error.message)
    }
}

impl std::error::Error for VertexError {}

impl std::fmt::Display for VertexErrorDetails {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.error_type, self.message)
    }
}

impl std::error::Error for VertexErrorDetails {}

impl VertexPredictRequest {
    /// Create a new prediction request
    pub fn new(messages: Vec<VertexMessage>, max_tokens: u32) -> Self {
        Self {
            anthropic_version: "vertex-2023-10-16".to_string(),
            messages,
            max_tokens,
            system: None,
            temperature: None,
            top_p: None,
            top_k: None,
            stop_sequences: None,
            tools: None,
            tool_choice: None,
            stream: false,
        }
    }

    /// Add system prompt
    pub fn with_system<S: Into<String>>(mut self, system: S) -> Self {
        self.system = Some(system.into());
        self
    }

    /// Set temperature
    pub fn with_temperature(mut self, temperature: f64) -> Self {
        self.temperature = Some(temperature);
        self
    }

    /// Set top_p
    pub fn with_top_p(mut self, top_p: f64) -> Self {
        self.top_p = Some(top_p);
        self
    }

    /// Set top_k
    pub fn with_top_k(mut self, top_k: u32) -> Self {
        self.top_k = Some(top_k);
        self
    }

    /// Set stop sequences
    pub fn with_stop_sequences(mut self, sequences: Vec<String>) -> Self {
        self.stop_sequences = Some(sequences);
        self
    }

    /// Enable streaming
    pub fn with_streaming(mut self, stream: bool) -> Self {
        self.stream = stream;
        self
    }
}

impl VertexMessage {
    /// Create a user message
    pub fn user<S: Into<String>>(content: S) -> Self {
        Self {
            role: VertexRole::User,
            content: content.into(),
        }
    }

    /// Create an assistant message
    pub fn assistant<S: Into<String>>(content: S) -> Self {
        Self {
            role: VertexRole::Assistant,
            content: content.into(),
        }
    }
}

impl VertexPredictResponse {
    
    /// Convert to internal prediction format for backward compatibility
    pub fn to_prediction(&self) -> VertexPrediction {
        VertexPrediction {
            content: self.get_text(),
            usage: Some(VertexUsageInternal {
                input_tokens: Some(self.usage.input_tokens),
                output_tokens: Some(self.usage.output_tokens),
                total_tokens: Some(self.usage.input_tokens + self.usage.output_tokens),
            }),
            safety_ratings: None,
            finish_reason: self.stop_reason.as_ref().map(|r| match r.as_str() {
                "end_turn" => VertexFinishReason::Stop,
                "max_tokens" => VertexFinishReason::MaxTokens,
                "stop_sequence" => VertexFinishReason::Stop,
                _ => VertexFinishReason::Other,
            }),
        }
    }
}

impl VertexStreamChunk {
    /// Extract text content from the chunk
    /// This handles multiple possible locations where content might be found
    pub fn get_content(&self) -> Option<String> {
        // Try direct content field first (Vertex AI message format)
        if let Some(content_array) = &self.content {
            for content in content_array {
                if let Some(text) = content.get_text() {
                    if !text.is_empty() {
                        return Some(text.to_string());
                    }
                }
            }
        }
        
        // Try delta.text (standard Anthropic format)
        if let Some(delta) = &self.delta {
            if let Some(text) = &delta.text {
                if !text.is_empty() {
                    return Some(text.clone());
                }
            }
        }
        
        // Try content_block.text (Vertex AI specific)
        if let Some(content_block) = &self.content_block {
            if !content_block.text.is_empty() {
                return Some(content_block.text.clone());
            }
        }
        
        // Try message.content (full message format)
        if let Some(message) = &self.message {
            for content in &message.content {
                if let Some(text) = content.get("text").and_then(|t| t.as_str()) {
                    if !text.is_empty() {
                        return Some(text.to_string());
                    }
                }
            }
        }
        
        None
    }
    
    /// Get finish reason if available
    pub fn get_finish_reason(&self) -> Option<VertexFinishReason> {
        // Try delta.stop_reason first
        if let Some(delta) = &self.delta {
            if let Some(reason) = &delta.stop_reason {
                return Some(match reason.as_str() {
                    "end_turn" => VertexFinishReason::Stop,
                    "max_tokens" => VertexFinishReason::MaxTokens,
                    "stop_sequence" => VertexFinishReason::Stop,
                    _ => VertexFinishReason::Other,
                });
            }
        }
        
        // Try message.stop_reason
        if let Some(message) = &self.message {
            if let Some(reason) = &message.stop_reason {
                return Some(match reason.as_str() {
                    "end_turn" => VertexFinishReason::Stop,
                    "max_tokens" => VertexFinishReason::MaxTokens,
                    "stop_sequence" => VertexFinishReason::Stop,
                    _ => VertexFinishReason::Other,
                });
            }
        }
        
        None
    }
    
    /// Check if this chunk has any content at all
    pub fn has_content(&self) -> bool {
        self.get_content().is_some()
    }
    
    /// Check if this is a final chunk
    pub fn is_final(&self) -> bool {
        self.event_type == "message_stop" || 
        self.get_finish_reason().is_some() ||
        self.usage.is_some()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vertex_message_creation() {
        let user_msg = VertexMessage::user("Hello Claude");
        assert!(matches!(user_msg.role, VertexRole::User));
        assert_eq!(user_msg.content, "Hello Claude");

        let assistant_msg = VertexMessage::assistant("Hello there!");
        assert!(matches!(assistant_msg.role, VertexRole::Assistant));
        assert_eq!(assistant_msg.content, "Hello there!");
    }

    #[test]
    fn test_vertex_request_creation() {
        let messages = vec![
            VertexMessage::user("Hello"),
            VertexMessage::assistant("Hi!"),
        ];

        let request = VertexPredictRequest::new(messages, 1000)
            .with_system("You are Claude")
            .with_temperature(0.7)
            .with_top_p(0.9);

        assert_eq!(request.max_tokens, 1000);
        assert_eq!(request.system, Some("You are Claude".to_string()));
        assert_eq!(request.temperature, Some(0.7));
        assert_eq!(request.top_p, Some(0.9));
        assert_eq!(request.anthropic_version, "vertex-2023-10-16");
    }

    #[test]
    fn test_serialization() {
        let messages = vec![
            VertexMessage::user("Hello"),
            VertexMessage::assistant("Hi!"),
        ];
        let request = VertexPredictRequest::new(messages, 100);

        let json = serde_json::to_string(&request).unwrap();

        // Should include anthropic_version
        assert!(json.contains("\"anthropic_version\":\"vertex-2023-10-16\""));
        
        // Should include messages and max_tokens
        assert!(json.contains("\"messages\""));
        assert!(json.contains("\"max_tokens\":100"));

        // Should not include stream field (it's skipped)
        assert!(!json.contains("\"stream\""));

        // Should be deserializable
        let deserialized: VertexPredictRequest = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.max_tokens, 100);
        assert_eq!(deserialized.messages.len(), 2);
    }

    #[test]
    fn test_response_text_extraction() {
        let response = VertexPredictResponse {
            id: "msg_123".to_string(),
            response_type: "message".to_string(),
            role: "assistant".to_string(),
            content: vec![
                VertexContent::Text {
                    text: "Hello ".to_string(),
                },
                VertexContent::Text {
                    text: "there!".to_string(),
                },
            ],
            model: "claude-sonnet-4-5@20250929".to_string(),
            stop_reason: Some("end_turn".to_string()),
            stop_sequence: None,
            usage: VertexUsage {
                input_tokens: 10,
                output_tokens: 20,
            },
        };

        assert_eq!(response.get_text(), "Hello there!");
        
        let prediction = response.to_prediction();
        assert_eq!(prediction.content, "Hello there!");
        assert_eq!(prediction.usage.as_ref().unwrap().input_tokens, Some(10));
        assert_eq!(prediction.usage.as_ref().unwrap().output_tokens, Some(20));
    }
}
