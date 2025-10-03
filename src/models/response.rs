// src/models/response.rs
//
// OpenAI API compatible response models

use serde::{Deserialize, Serialize};

use super::common::{ChatMessage, FinishReason, Usage};

/// Chat completion response structure
/// Matches OpenAI API v1/chat/completions response specification exactly
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatCompletionResponse {
    /// Unique identifier for the chat completion
    pub id: String,

    /// Object type, always "chat.completion"
    pub object: String,

    /// Unix timestamp of when the chat completion was created
    pub created: u64,

    /// Model used for the chat completion
    pub model: String,

    /// System fingerprint
    #[serde(skip_serializing_if = "Option::is_none")]
    pub system_fingerprint: Option<String>,

    /// List of chat completion choices
    pub choices: Vec<ChatCompletionChoice>,

    /// Usage statistics for the completion request
    pub usage: Usage,
}

/// Individual chat completion choice
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatCompletionChoice {
    /// Index of this choice in the list of choices
    pub index: u32,

    /// The generated message
    pub message: ChatMessage,

    /// Log probability information for the choice
    #[serde(skip_serializing_if = "Option::is_none")]
    pub logprobs: Option<LogProbs>,

    /// Reason the model stopped generating tokens
    pub finish_reason: FinishReason,
}

/// Chat completion streaming response chunk
/// Used for Server-Sent Events streaming
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatCompletionChunk {
    /// Unique identifier for the chat completion
    pub id: String,

    /// Object type, always "chat.completion.chunk"
    pub object: String,

    /// Unix timestamp of when the chunk was created
    pub created: u64,

    /// Model used for the chat completion
    pub model: String,

    /// System fingerprint
    #[serde(skip_serializing_if = "Option::is_none")]
    pub system_fingerprint: Option<String>,

    /// List of chat completion choice deltas
    pub choices: Vec<ChatCompletionChunkChoice>,

    /// Usage statistics (only in the last chunk if stream_options.include_usage is true)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub usage: Option<Usage>,
}

/// Individual streaming chunk choice
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatCompletionChunkChoice {
    /// Index of this choice in the list of choices
    pub index: u32,

    /// Delta containing the incremental message content
    pub delta: ChatMessage,

    /// Log probability information for the choice
    #[serde(skip_serializing_if = "Option::is_none")]
    pub logprobs: Option<LogProbs>,

    /// Reason the model stopped generating tokens (only present in final chunk)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub finish_reason: Option<FinishReason>,
}

/// Log probabilities for tokens
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogProbs {
    /// List of message content tokens with log probability information
    pub content: Vec<TokenLogProb>,
}

/// Log probability information for a token
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenLogProb {
    /// The token
    pub token: String,

    /// Log probability of the token
    pub logprob: f64,

    /// List of bytes representing the token
    pub bytes: Vec<u8>,

    /// List of the most likely tokens and their log probabilities
    pub top_logprobs: Vec<TopTokenLogProb>,
}

/// Top token log probability
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TopTokenLogProb {
    /// The token
    pub token: String,

    /// Log probability of the token
    pub logprob: f64,

    /// List of bytes representing the token
    pub bytes: Vec<u8>,
}

/// Models list response structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelsResponse {
    /// Object type, always "list"
    pub object: String,

    /// List of available models
    pub data: Vec<Model>,
}

/// Individual model information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Model {
    /// Model identifier
    pub id: String,

    /// Object type, always "model"
    pub object: String,

    /// Unix timestamp of when the model was created
    pub created: u64,

    /// Organization that owns the model
    pub owned_by: String,
}

impl ChatCompletionResponse {
    /// Create a new chat completion response
    pub fn new(
        id: String,
        model: String,
        choices: Vec<ChatCompletionChoice>,
        usage: Usage,
    ) -> Self {
        Self {
            id,
            object: "chat.completion".to_string(),
            created: chrono::Utc::now().timestamp() as u64,
            model,
            system_fingerprint: None,
            choices,
            usage,
        }
    }

    /// Get the primary message content (from first choice)
    pub fn content(&self) -> Option<&str> {
        self.choices
            .first()
            .map(|choice| choice.message.content.as_str())
    }

    /// Get the finish reason from the first choice
    pub fn finish_reason(&self) -> Option<&FinishReason> {
        self.choices
            .first()
            .map(|choice| &choice.finish_reason)
    }
}

impl ChatCompletionChunk {
    /// Create a new chat completion chunk
    pub fn new(
        id: String,
        model: String,
        choices: Vec<ChatCompletionChunkChoice>,
    ) -> Self {
        Self {
            id,
            object: "chat.completion.chunk".to_string(),
            created: chrono::Utc::now().timestamp() as u64,
            model,
            system_fingerprint: None,
            choices,
            usage: None,
        }
    }

    /// Create a final chunk with usage statistics
    pub fn final_chunk(
        id: String,
        model: String,
        usage: Usage,
    ) -> Self {
        Self {
            id,
            object: "chat.completion.chunk".to_string(),
            created: chrono::Utc::now().timestamp() as u64,
            model,
            system_fingerprint: None,
            choices: vec![ChatCompletionChunkChoice {
                index: 0,
                delta: ChatMessage {
                    role: crate::models::common::MessageRole::Assistant,
                    content: String::new(),
                    name: None,
                    function_call: None,
                    tool_calls: None,
                    tool_call_id: None,
                },
                logprobs: None,
                finish_reason: Some(FinishReason::Stop),
            }],
            usage: Some(usage),
        }
    }

    /// Create a new chunk with usage information
    pub fn new_with_usage(
        id: String,
        model: String,
        choices: Vec<ChatCompletionChunkChoice>,
        usage: Usage,
    ) -> Self {
        Self {
            id,
            object: "chat.completion.chunk".to_string(),
            created: chrono::Utc::now().timestamp() as u64,
            model,
            system_fingerprint: None,
            choices,
            usage: Some(usage),
        }
    }

    /// Get the content delta from the first choice
    pub fn content_delta(&self) -> Option<&str> {
        self.choices
            .first()
            .map(|choice| choice.delta.content.as_str())
    }

    /// Check if this is the final chunk (has finish_reason)
    pub fn is_final(&self) -> bool {
        self.choices
            .first()
            .map(|choice| choice.finish_reason.is_some())
            .unwrap_or(false)
    }

    /// Create a Server-Sent Event formatted string
    pub fn to_sse(&self) -> String {
        let json = serde_json::to_string(self).unwrap_or_default();
        format!("data: {}\n\n", json)
    }

    /// Create the final [DONE] SSE message
    pub fn done_sse() -> String {
        "data: [DONE]\n\n".to_string()
    }

    /// Create an error chunk for streaming errors
    pub fn error_chunk(id: String, model: String, error_message: String) -> Self {
        Self {
            id,
            object: "chat.completion.chunk".to_string(),
            created: chrono::Utc::now().timestamp() as u64,
            model,
            system_fingerprint: None,
            choices: vec![ChatCompletionChunkChoice {
                index: 0,
                delta: ChatMessage {
                    role: crate::models::common::MessageRole::Assistant,
                    content: format!("Error: {}", error_message),
                    name: None,
                    function_call: None,
                    tool_calls: None,
                    tool_call_id: None,
                },
                logprobs: None,
                finish_reason: Some(FinishReason::Stop),
            }],
            usage: None,
        }
    }

    /// Create a heartbeat chunk (empty content to keep connection alive)
    pub fn heartbeat_chunk(id: String, model: String) -> Self {
        Self {
            id,
            object: "chat.completion.chunk".to_string(),
            created: chrono::Utc::now().timestamp() as u64,
            model,
            system_fingerprint: None,
            choices: vec![ChatCompletionChunkChoice {
                index: 0,
                delta: ChatMessage {
                    role: crate::models::common::MessageRole::Assistant,
                    content: String::new(),
                    name: None,
                    function_call: None,
                    tool_calls: None,
                    tool_call_id: None,
                },
                logprobs: None,
                finish_reason: None,
            }],
            usage: None,
        }
    }
}

impl ModelsResponse {
    /// Create a new models response with Claude 4 Sonnet
    pub fn claude_models() -> Self {
        Self {
            object: "list".to_string(),
            data: vec![
                Model {
                    id: "claude-4-sonnet-20250514".to_string(),
                    object: "model".to_string(),
                    created: 1715702400, // May 14, 2025 timestamp
                    owned_by: "anthropic".to_string(),
                },
            ],
        }
    }

    /// Add a model to the response
    pub fn add_model(mut self, model: Model) -> Self {
        self.data.push(model);
        self
    }
}

impl Model {
    /// Create a new model entry
    pub fn new<S: Into<String>>(id: S, owned_by: S) -> Self {
        Self {
            id: id.into(),
            object: "model".to_string(),
            created: chrono::Utc::now().timestamp() as u64,
            owned_by: owned_by.into(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::common::{ChatMessage, MessageRole};

    #[test]
    fn test_chat_completion_response() {
        let message = ChatMessage::assistant("Hello there!");
        let choice = ChatCompletionChoice {
            index: 0,
            message,
            logprobs: None,
            finish_reason: FinishReason::Stop,
        };
        let usage = Usage {
            prompt_tokens: 10,
            completion_tokens: 5,
            total_tokens: 15,
            prompt_tokens_details: None,
            completion_tokens_details: None,
        };

        let response = ChatCompletionResponse::new(
            "chatcmpl-123".to_string(),
            "claude-4-sonnet-20250514".to_string(),
            vec![choice],
            usage,
        );

        assert_eq!(response.id, "chatcmpl-123");
        assert_eq!(response.object, "chat.completion");
        assert_eq!(response.model, "claude-4-sonnet-20250514");
        assert_eq!(response.content(), Some("Hello there!"));
        assert!(matches!(response.finish_reason(), Some(FinishReason::Stop)));
    }

    #[test]
    fn test_streaming_chunk() {
        let delta = ChatMessage {
            role: MessageRole::Assistant,
            content: "Hello".to_string(),
            name: None,
            function_call: None,
            tool_calls: None,
            tool_call_id: None,
        };

        let choice = ChatCompletionChunkChoice {
            index: 0,
            delta,
            logprobs: None,
            finish_reason: None,
        };

        let chunk = ChatCompletionChunk::new(
            "chatcmpl-123".to_string(),
            "claude-4-sonnet-20250514".to_string(),
            vec![choice],
        );

        assert_eq!(chunk.id, "chatcmpl-123");
        assert_eq!(chunk.object, "chat.completion.chunk");
        assert_eq!(chunk.content_delta(), Some("Hello"));
        assert!(!chunk.is_final());

        let sse = chunk.to_sse();
        assert!(sse.starts_with("data: "));
        assert!(sse.ends_with("\n\n"));
        assert!(sse.contains("chat.completion.chunk"));
    }

    #[test]
    fn test_final_chunk() {
        let usage = Usage {
            prompt_tokens: 10,
            completion_tokens: 5,
            total_tokens: 15,
            prompt_tokens_details: None,
            completion_tokens_details: None,
        };

        let chunk = ChatCompletionChunk::final_chunk(
            "chatcmpl-123".to_string(),
            "claude-4-sonnet-20250514".to_string(),
            usage,
        );

        assert!(chunk.is_final());
        assert!(chunk.usage.is_some());
    }

    #[test]
    fn test_models_response() {
        let models = ModelsResponse::claude_models();

        assert_eq!(models.object, "list");
        assert_eq!(models.data.len(), 1);
        assert_eq!(models.data[0].id, "claude-4-sonnet-20250514");
        assert_eq!(models.data[0].owned_by, "anthropic");
        assert_eq!(models.data[0].object, "model");
    }

    #[test]
    fn test_sse_formatting() {
        let chunk = ChatCompletionChunk::new(
            "test".to_string(),
            "claude-4-sonnet-20250514".to_string(),
            vec![],
        );

        let sse = chunk.to_sse();
        assert!(sse.starts_with("data: {"));
        assert!(sse.ends_with("}\n\n"));

        let done_sse = ChatCompletionChunk::done_sse();
        assert_eq!(done_sse, "data: [DONE]\n\n");
    }

    #[test]
    fn test_serialization() {
        let message = ChatMessage::assistant("Test response");
        let choice = ChatCompletionChoice {
            index: 0,
            message,
            logprobs: None,
            finish_reason: FinishReason::Stop,
        };
        let usage = Usage {
            prompt_tokens: 5,
            completion_tokens: 10,
            total_tokens: 15,
            prompt_tokens_details: None,
            completion_tokens_details: None,
        };

        let response = ChatCompletionResponse::new(
            "test-id".to_string(),
            "claude-4-sonnet-20250514".to_string(),
            vec![choice],
            usage,
        );

        let json = serde_json::to_string(&response).unwrap();

        // Should contain required fields
        assert!(json.contains("\"object\":\"chat.completion\""));
        assert!(json.contains("\"model\":\"claude-4-sonnet-20250514\""));
        assert!(json.contains("\"finish_reason\":\"stop\""));

        // Should be deserializable
        let deserialized: ChatCompletionResponse = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.id, "test-id");
        assert_eq!(deserialized.model, "claude-4-sonnet-20250514");
    }
}