// src/infrastructure/common/converter.rs
//
// Common converter trait for all AI providers
// Provides a unified interface for converting between OpenAI format and provider-specific formats

use anyhow::Result;
use async_trait::async_trait;
use tokio_stream::Stream;

use crate::models::{
    request::ChatCompletionRequest,
    response::{ChatCompletionResponse, ChatCompletionChunk},
};

/// Common trait for converting between OpenAI API format and provider-specific formats
/// 
/// This trait provides a unified interface that all AI providers (Vertex AI, Bedrock, etc.)
/// must implement to handle format conversion and streaming.
#[async_trait]
pub trait ProviderConverter: Send + Sync {
    /// Provider-specific request type
    type ProviderRequest: Send + Sync;
    
    /// Provider-specific response type
    type ProviderResponse: Send + Sync;
    
    /// Provider-specific streaming chunk type
    type ProviderStreamChunk: Send + Sync;
    
    /// Provider-specific error type
    type ProviderError: std::error::Error + Send + Sync;

    /// Convert OpenAI chat completion request to provider-specific format
    fn openai_to_provider_request(
        &self,
        request: &ChatCompletionRequest,
    ) -> Result<Self::ProviderRequest>;

    /// Convert OpenAI chat completion request to provider-specific streaming format
    fn openai_to_provider_streaming_request(
        &self,
        request: &ChatCompletionRequest,
    ) -> Result<Self::ProviderRequest>;

    /// Convert provider response to OpenAI chat completion format
    fn provider_to_openai_response(
        &self,
        response: &Self::ProviderResponse,
        request_id: &str,
        model: &str,
        original_request: &ChatCompletionRequest,
    ) -> Result<ChatCompletionResponse>;

    /// Convert provider streaming chunk to OpenAI chat completion chunk format
    fn provider_chunk_to_openai_chunk(
        &self,
        chunk: &Self::ProviderStreamChunk,
        request_id: &str,
        model: &str,
    ) -> Result<Option<ChatCompletionChunk>>;

    /// Get the provider name for logging and identification
    fn provider_name(&self) -> &'static str;

    /// Validate that the request is supported by this provider
    fn validate_request(&self, request: &ChatCompletionRequest) -> Result<()> {
        // Default implementation - providers can override
        request.validate().map_err(|e| anyhow::anyhow!(e))
    }

    /// Get supported models for this provider
    fn supported_models(&self) -> Vec<String>;

    /// Normalize model name for this provider
    fn normalize_model_name(&self, model: &str) -> String {
        // Default implementation - providers can override
        model.to_string()
    }
}

/// Streaming converter trait for providers that support streaming
#[async_trait]
pub trait StreamingConverter: ProviderConverter {
    /// Stream type returned by the provider
    type ProviderStream: Stream<Item = Result<Self::ProviderStreamChunk, Self::ProviderError>> + Send;

    /// Process a stream of provider chunks into OpenAI chunks
    async fn process_stream(
        &self,
        stream: Self::ProviderStream,
        request_id: String,
        model: String,
    ) -> impl Stream<Item = Result<ChatCompletionChunk, anyhow::Error>> + Send;
}

/// Error handling trait for provider-specific errors
pub trait ProviderErrorHandler {
    type ProviderError: std::error::Error + Send + Sync;

    /// Convert provider error to standardized error format
    fn handle_provider_error(&self, error: Self::ProviderError) -> anyhow::Error;

    /// Check if error is retryable
    fn is_retryable_error(&self, _error: &Self::ProviderError) -> bool {
        false // Default: don't retry
    }

    /// Get retry delay for retryable errors
    fn retry_delay(&self, attempt: u32) -> std::time::Duration {
        std::time::Duration::from_millis(1000 * 2_u64.pow(attempt.min(5)))
    }
}

/// Utility functions for common conversion tasks
pub mod utils {
    use crate::models::common::{MessageRole, FinishReason};

    /// Convert common role strings to OpenAI MessageRole
    pub fn normalize_role(role: &str) -> MessageRole {
        match role.to_lowercase().as_str() {
            "user" => MessageRole::User,
            "assistant" => MessageRole::Assistant,
            "system" => MessageRole::System,
            "function" => MessageRole::Function,
            "tool" => MessageRole::Tool,
            _ => MessageRole::User, // Default fallback
        }
    }

    /// Convert common finish reason strings to OpenAI FinishReason
    pub fn normalize_finish_reason(reason: &str) -> FinishReason {
        match reason.to_lowercase().as_str() {
            "stop" | "end_turn" | "complete" => FinishReason::Stop,
            "length" | "max_tokens" => FinishReason::Length,
            "content_filter" | "safety" => FinishReason::ContentFilter,
            "tool_calls" | "function_call" => FinishReason::ToolCalls,
            _ => FinishReason::Stop, // Default fallback
        }
    }

    /// Extract text content from various possible JSON structures
    pub fn extract_text_content(json: &serde_json::Value) -> Option<String> {
        // Try common locations where text content might be stored
        json.get("text")
            .or_else(|| json.get("content"))
            .or_else(|| json.get("delta").and_then(|d| d.get("text")))
            .or_else(|| json.get("message").and_then(|m| m.get("content")))
            .or_else(|| json.get("choices")
                .and_then(|c| c.as_array())
                .and_then(|arr| arr.first())
                .and_then(|first| first.get("text")))
            .and_then(|t| t.as_str())
            .map(|s| s.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::common::{MessageRole, FinishReason};

    #[test]
    fn test_normalize_role() {
        assert_eq!(utils::normalize_role("user"), MessageRole::User);
        assert_eq!(utils::normalize_role("ASSISTANT"), MessageRole::Assistant);
        assert_eq!(utils::normalize_role("System"), MessageRole::System);
        assert_eq!(utils::normalize_role("unknown"), MessageRole::User);
    }

    #[test]
    fn test_normalize_finish_reason() {
        assert_eq!(utils::normalize_finish_reason("stop"), FinishReason::Stop);
        assert_eq!(utils::normalize_finish_reason("MAX_TOKENS"), FinishReason::Length);
        assert_eq!(utils::normalize_finish_reason("content_filter"), FinishReason::ContentFilter);
    }

    #[test]
    fn test_extract_text_content() {
        let json1 = serde_json::json!({"text": "hello world"});
        assert_eq!(utils::extract_text_content(&json1), Some("hello world".to_string()));

        let json2 = serde_json::json!({"delta": {"text": "streaming text"}});
        assert_eq!(utils::extract_text_content(&json2), Some("streaming text".to_string()));

        let json3 = serde_json::json!({"unknown": "field"});
        assert_eq!(utils::extract_text_content(&json3), None);
    }
}
