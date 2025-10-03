// src/models/common.rs
//
// Common types and structures shared across OpenAI API models

use serde::{Deserialize, Serialize};

/// Chat message structure compatible with OpenAI API
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: MessageRole,
    pub content: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub function_call: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<Vec<serde_json::Value>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_call_id: Option<String>,
}

/// Message roles as defined by OpenAI API
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MessageRole {
    System,
    User,
    Assistant,
    Tool,
    Function,
}

/// Function definition for function calling
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionDefinition {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parameters: Option<serde_json::Value>,
}

/// Tool definition for tool calling
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolDefinition {
    #[serde(rename = "type")]
    pub tool_type: String,
    pub function: FunctionDefinition,
}

/// Response format specification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResponseFormat {
    #[serde(rename = "type")]
    pub format_type: ResponseFormatType,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub json_schema: Option<serde_json::Value>,
}

/// Response format types
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ResponseFormatType {
    Text,
    JsonObject,
    JsonSchema,
}

/// Usage statistics structure
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Usage {
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
    pub total_tokens: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prompt_tokens_details: Option<PromptTokensDetails>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub completion_tokens_details: Option<CompletionTokensDetails>,
}

/// Detailed prompt token usage
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromptTokensDetails {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cached_tokens: Option<u32>,
}

/// Detailed completion token usage
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompletionTokensDetails {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reasoning_tokens: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub accepted_prediction_tokens: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rejected_prediction_tokens: Option<u32>,
}

/// Finish reason enumeration
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FinishReason {
    Stop,
    Length,
    FunctionCall,
    ToolCalls,
    ContentFilter,
}

impl std::fmt::Display for FinishReason {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FinishReason::Stop => write!(f, "stop"),
            FinishReason::Length => write!(f, "length"),
            FinishReason::FunctionCall => write!(f, "function_call"),
            FinishReason::ToolCalls => write!(f, "tool_calls"),
            FinishReason::ContentFilter => write!(f, "content_filter"),
        }
    }
}

impl Default for ResponseFormat {
    fn default() -> Self {
        Self {
            format_type: ResponseFormatType::Text,
            json_schema: None,
        }
    }
}

impl ChatMessage {
    /// Create a new system message
    pub fn system<S: Into<String>>(content: S) -> Self {
        Self {
            role: MessageRole::System,
            content: content.into(),
            name: None,
            function_call: None,
            tool_calls: None,
            tool_call_id: None,
        }
    }

    /// Create a new user message
    pub fn user<S: Into<String>>(content: S) -> Self {
        Self {
            role: MessageRole::User,
            content: content.into(),
            name: None,
            function_call: None,
            tool_calls: None,
            tool_call_id: None,
        }
    }

    /// Create a new assistant message
    pub fn assistant<S: Into<String>>(content: S) -> Self {
        Self {
            role: MessageRole::Assistant,
            content: content.into(),
            name: None,
            function_call: None,
            tool_calls: None,
            tool_call_id: None,
        }
    }

    /// Create an empty assistant message (for streaming chunks)
    pub fn empty() -> Self {
        Self {
            role: MessageRole::Assistant,
            content: String::new(),
            name: None,
            function_call: None,
            tool_calls: None,
            tool_call_id: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_chat_message_creation() {
        let system_msg = ChatMessage::system("You are a helpful assistant");
        assert!(matches!(system_msg.role, MessageRole::System));
        assert_eq!(system_msg.content, "You are a helpful assistant");

        let user_msg = ChatMessage::user("Hello, world!");
        assert!(matches!(user_msg.role, MessageRole::User));
        assert_eq!(user_msg.content, "Hello, world!");

        let assistant_msg = ChatMessage::assistant("Hi there!");
        assert!(matches!(assistant_msg.role, MessageRole::Assistant));
        assert_eq!(assistant_msg.content, "Hi there!");
    }

    #[test]
    fn test_usage_calculations() {
        let usage = Usage {
            prompt_tokens: 10,
            completion_tokens: 20,
            total_tokens: 30,
            prompt_tokens_details: None,
            completion_tokens_details: None,
        };

        assert_eq!(usage.prompt_tokens + usage.completion_tokens, usage.total_tokens);
    }

    #[test]
    fn test_serialization() {
        let message = ChatMessage::user("Test message");
        let json = serde_json::to_string(&message).unwrap();

        // Should serialize role as lowercase
        assert!(json.contains(r#""role":"user""#));
        assert!(json.contains(r#""content":"Test message""#));
    }
}