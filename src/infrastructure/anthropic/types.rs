//! Anthropic API types and configuration structures
//! 
//! This module defines all the types needed for Anthropic API integration,
//! including configuration, request/response structures, and model definitions.

use anyhow::{anyhow, Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::env;
use tracing::{debug, warn};

/// Anthropic API configuration
#[derive(Debug, Clone)]
pub struct AnthropicConfig {
    /// API key for Anthropic authentication
    pub api_key: String,
    
    /// Base URL for Anthropic API (defaults to https://api.anthropic.com/v1)
    pub base_url: Option<String>,
    
    /// Request timeout in seconds (defaults to 120)
    pub timeout: Option<u64>,
    
    /// Maximum retries for failed requests (defaults to 3)
    pub max_retries: Option<u32>,
    
    /// Default model to use when none is specified
    pub default_model: Option<String>,
    
    /// Organization ID (optional)
    pub organization_id: Option<String>,
    
    /// Custom headers to include with requests
    pub custom_headers: Option<HashMap<String, String>>,
}

impl Default for AnthropicConfig {
    fn default() -> Self {
        Self {
            api_key: String::new(),
            base_url: Some("https://api.anthropic.com/v1".to_string()),
            timeout: Some(120),
            max_retries: Some(3),
            default_model: Some("claude-3-5-sonnet-20241022".to_string()),
            organization_id: None,
            custom_headers: None,
        }
    }
}

impl AnthropicConfig {
    /// Create configuration from environment variables
    pub fn from_env() -> Result<Self> {
        let api_key = env::var("ANTHROPIC_API_KEY")
            .context("ANTHROPIC_API_KEY environment variable not set")?;

        let base_url = env::var("ANTHROPIC_BASE_URL")
            .ok()
            .or_else(|| Some("https://api.anthropic.com/v1".to_string()));

        let timeout = env::var("ANTHROPIC_TIMEOUT")
            .ok()
            .and_then(|t| t.parse::<u64>().ok())
            .or(Some(120));

        let max_retries = env::var("ANTHROPIC_MAX_RETRIES")
            .ok()
            .and_then(|r| r.parse::<u32>().ok())
            .or(Some(3));

        let default_model = env::var("ANTHROPIC_DEFAULT_MODEL")
            .ok()
            .or_else(|| Some("claude-3-5-sonnet-20241022".to_string()));

        let organization_id = env::var("ANTHROPIC_ORGANIZATION_ID").ok();

        debug!("Loaded Anthropic configuration from environment");

        Ok(Self {
            api_key,
            base_url,
            timeout,
            max_retries,
            default_model,
            organization_id,
            custom_headers: None,
        })
    }

    /// Get the messages endpoint URL
    pub fn get_messages_url(&self) -> String {
        let default_base = "https://api.anthropic.com/v1".to_string();
        let base = self.base_url.as_ref().unwrap_or(&default_base);
        format!("{}/messages", base)
    }

    /// Validate the configuration
    pub fn validate(&self) -> Result<()> {
        if self.api_key.is_empty() {
            return Err(anyhow!("API key is required"));
        }

        if !self.api_key.starts_with("sk-ant-") {
            warn!("API key does not start with expected prefix 'sk-ant-'");
        }

        if let Some(timeout) = self.timeout {
            if timeout == 0 {
                return Err(anyhow!("Timeout must be greater than 0"));
            }
        }

        if let Some(max_retries) = self.max_retries {
            if max_retries > 10 {
                warn!("Max retries is very high ({}), this may cause long delays", max_retries);
            }
        }

        Ok(())
    }
}

/// Anthropic model enumeration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AnthropicModel {
    #[serde(rename = "claude-3-5-sonnet-20241022")]
    Claude35Sonnet20241022,
    
    #[serde(rename = "claude-3-5-haiku-20241022")]
    Claude35Haiku20241022,
    
    #[serde(rename = "claude-3-opus-20240229")]
    Claude3Opus20240229,
    
    #[serde(rename = "claude-3-sonnet-20240229")]
    Claude3Sonnet20240229,
    
    #[serde(rename = "claude-3-haiku-20240307")]
    Claude3Haiku20240307,
}

impl AnthropicModel {
    /// Get the string representation of the model
    pub fn as_str(&self) -> &'static str {
        match self {
            AnthropicModel::Claude35Sonnet20241022 => "claude-3-5-sonnet-20241022",
            AnthropicModel::Claude35Haiku20241022 => "claude-3-5-haiku-20241022",
            AnthropicModel::Claude3Opus20240229 => "claude-3-opus-20240229",
            AnthropicModel::Claude3Sonnet20240229 => "claude-3-sonnet-20240229",
            AnthropicModel::Claude3Haiku20240307 => "claude-3-haiku-20240307",
        }
    }

    /// Check if the model supports tool calling
    pub fn supports_tools(&self) -> bool {
        match self {
            AnthropicModel::Claude35Sonnet20241022 |
            AnthropicModel::Claude35Haiku20241022 |
            AnthropicModel::Claude3Opus20240229 |
            AnthropicModel::Claude3Sonnet20240229 |
            AnthropicModel::Claude3Haiku20240307 => true,
        }
    }

    /// Get the maximum number of tokens for the model
    pub fn max_tokens(&self) -> u32 {
        match self {
            AnthropicModel::Claude35Sonnet20241022 |
            AnthropicModel::Claude35Haiku20241022 |
            AnthropicModel::Claude3Opus20240229 |
            AnthropicModel::Claude3Sonnet20240229 |
            AnthropicModel::Claude3Haiku20240307 => 200000,
        }
    }

    /// Get the approximate cost per 1K tokens (input tokens)
    pub fn cost_per_1k_tokens(&self) -> f64 {
        match self {
            AnthropicModel::Claude35Sonnet20241022 => 3.0,
            AnthropicModel::Claude35Haiku20241022 => 0.25,
            AnthropicModel::Claude3Opus20240229 => 15.0,
            AnthropicModel::Claude3Sonnet20240229 => 3.0,
            AnthropicModel::Claude3Haiku20240307 => 0.25,
        }
    }
}

impl From<&str> for AnthropicModel {
    fn from(s: &str) -> Self {
        match s {
            "claude-3-5-sonnet-20241022" => AnthropicModel::Claude35Sonnet20241022,
            "claude-3-5-haiku-20241022" => AnthropicModel::Claude35Haiku20241022,
            "claude-3-opus-20240229" => AnthropicModel::Claude3Opus20240229,
            "claude-3-sonnet-20240229" => AnthropicModel::Claude3Sonnet20240229,
            "claude-3-haiku-20240307" => AnthropicModel::Claude3Haiku20240307,
            _ => AnthropicModel::Claude35Sonnet20241022, // Default fallback
        }
    }
}

impl From<String> for AnthropicModel {
    fn from(s: String) -> Self {
        AnthropicModel::from(s.as_str())
    }
}

/// Anthropic Messages API request structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnthropicMessagesRequest {
    /// Model to use for the completion
    pub model: String,
    
    /// Maximum number of tokens to generate
    pub max_tokens: u32,
    
    /// List of messages in the conversation
    pub messages: Vec<AnthropicMessage>,
    
    /// System message (optional)
    pub system: Option<String>,
    
    /// Available tools for the model to use (optional)
    pub tools: Option<Vec<AnthropicTool>>,
    
    /// Whether to stream the response (optional)
    pub stream: Option<bool>,
    
    /// Sampling temperature (0.0 to 1.0, optional)
    pub temperature: Option<f32>,
    
    /// Top-p sampling parameter (0.0 to 1.0, optional)
    pub top_p: Option<f32>,
    
    /// Top-k sampling parameter (optional)
    pub top_k: Option<u32>,
    
    /// Stop sequences (optional)
    pub stop_sequences: Option<Vec<String>>,
    
    /// Additional metadata (optional)
    pub metadata: Option<serde_json::Value>,
}

/// Anthropic message structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnthropicMessage {
    /// Role of the message sender
    pub role: String,
    
    /// Content of the message
    pub content: AnthropicContent,
}

/// Anthropic content structure (can be text or mixed content)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum AnthropicContent {
    /// Simple text content
    Text(String),
    
    /// Array of content blocks
    Blocks(Vec<AnthropicContentBlock>),
}

/// Anthropic content block structure
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum AnthropicContentBlock {
    #[serde(rename = "text")]
    Text {
        text: String,
    },
    
    #[serde(rename = "tool_use")]
    ToolUse {
        id: String,
        name: String,
        input: serde_json::Value,
    },
    
    #[serde(rename = "tool_result")]
    ToolResult {
        tool_use_id: String,
        content: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        is_error: Option<bool>,
    },
    
    #[serde(rename = "image")]
    Image {
        source: ImageSource,
    },
}

/// Image source for image content blocks
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImageSource {
    #[serde(rename = "type")]
    pub source_type: String, // "base64"
    pub media_type: String,  // "image/jpeg", "image/png", etc.
    pub data: String,        // base64 encoded image data
}

/// Anthropic tool definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnthropicTool {
    /// Name of the tool
    pub name: String,
    
    /// Description of what the tool does
    pub description: String,
    
    /// Input schema for the tool (JSON Schema)
    pub input_schema: serde_json::Value,
}

/// Anthropic Messages API response structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnthropicMessagesResponse {
    /// Unique identifier for the response
    pub id: String,
    
    /// Object type (should be "message")
    #[serde(rename = "type")]
    pub response_type: String,
    
    /// Role of the response (should be "assistant")
    pub role: String,
    
    /// Content of the response
    pub content: Vec<AnthropicContentBlock>,
    
    /// Model used for the response
    pub model: String,
    
    /// Stop reason
    pub stop_reason: Option<String>,
    
    /// Stop sequence that caused the stop (if any)
    pub stop_sequence: Option<String>,
    
    /// Token usage information
    pub usage: AnthropicUsage,
}

/// Token usage information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnthropicUsage {
    /// Number of input tokens
    pub input_tokens: u32,
    
    /// Number of output tokens
    pub output_tokens: u32,
}

/// Anthropic streaming response chunk
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum AnthropicStreamChunk {
    #[serde(rename = "message_start")]
    MessageStart {
        message: AnthropicMessageStart,
    },
    
    #[serde(rename = "content_block_start")]
    ContentBlockStart {
        index: u32,
        content_block: AnthropicContentBlock,
    },
    
    #[serde(rename = "content_block_delta")]
    ContentBlockDelta {
        index: u32,
        delta: AnthropicDelta,
    },
    
    #[serde(rename = "content_block_stop")]
    ContentBlockStop {
        index: u32,
    },
    
    #[serde(rename = "message_delta")]
    MessageDelta {
        delta: AnthropicMessageDelta,
        usage: Option<AnthropicUsage>,
    },
    
    #[serde(rename = "message_stop")]
    MessageStop,
    
    #[serde(rename = "ping")]
    Ping,
    
    #[serde(rename = "error")]
    Error {
        error: AnthropicError,
    },
}

/// Message start information in streaming
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnthropicMessageStart {
    pub id: String,
    #[serde(rename = "type")]
    pub message_type: String,
    pub role: String,
    pub content: Vec<serde_json::Value>,
    pub model: String,
    pub stop_reason: Option<String>,
    pub stop_sequence: Option<String>,
    pub usage: AnthropicUsage,
}

/// Delta information for streaming content
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum AnthropicDelta {
    #[serde(rename = "text_delta")]
    TextDelta {
        text: String,
    },
    
    #[serde(rename = "input_json_delta")]
    InputJsonDelta {
        partial_json: String,
    },
}

/// Message delta for streaming
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnthropicMessageDelta {
    pub stop_reason: Option<String>,
    pub stop_sequence: Option<String>,
}

/// Anthropic API error structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnthropicError {
    #[serde(rename = "type")]
    pub error_type: String,
    
    pub message: String,
}

/// Anthropic API error response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnthropicErrorResponse {
    pub error: AnthropicError,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_anthropic_config_default() {
        let config = AnthropicConfig::default();
        assert_eq!(config.base_url, Some("https://api.anthropic.com/v1".to_string()));
        assert_eq!(config.timeout, Some(120));
        assert_eq!(config.max_retries, Some(3));
        assert_eq!(config.default_model, Some("claude-3-5-sonnet-20241022".to_string()));
    }

    #[test]
    fn test_anthropic_model_conversions() {
        let model = AnthropicModel::from("claude-3-5-sonnet-20241022");
        assert_eq!(model.as_str(), "claude-3-5-sonnet-20241022");
        assert!(model.supports_tools());
        assert_eq!(model.max_tokens(), 200000);
    }

    #[test]
    fn test_anthropic_model_properties() {
        let sonnet = AnthropicModel::Claude35Sonnet20241022;
        let haiku = AnthropicModel::Claude35Haiku20241022;
        let opus = AnthropicModel::Claude3Opus20240229;

        assert!(sonnet.supports_tools());
        assert!(haiku.supports_tools());
        assert!(opus.supports_tools());

        // Test cost differences
        assert!(opus.cost_per_1k_tokens() > sonnet.cost_per_1k_tokens());
        assert!(sonnet.cost_per_1k_tokens() > haiku.cost_per_1k_tokens());
    }

    #[test]
    fn test_messages_url_generation() {
        let config = AnthropicConfig::default();
        assert_eq!(config.get_messages_url(), "https://api.anthropic.com/v1/messages");

        let custom_config = AnthropicConfig {
            base_url: Some("https://custom.api.com/v2".to_string()),
            ..Default::default()
        };
        assert_eq!(custom_config.get_messages_url(), "https://custom.api.com/v2/messages");
    }

    #[test]
    fn test_content_serialization() {
        let text_content = AnthropicContent::Text("Hello, world!".to_string());
        let json = serde_json::to_string(&text_content).unwrap();
        assert_eq!(json, "\"Hello, world!\"");

        let block_content = AnthropicContent::Blocks(vec![
            AnthropicContentBlock::Text {
                text: "Hello".to_string(),
            }
        ]);
        let json = serde_json::to_string(&block_content).unwrap();
        assert!(json.contains("\"type\":\"text\""));
    }

    #[test]
    fn test_config_validation() {
        let mut config = AnthropicConfig::default();
        config.api_key = "".to_string();
        assert!(config.validate().is_err());

        config.api_key = "sk-ant-valid-key".to_string();
        assert!(config.validate().is_ok());

        config.timeout = Some(0);
        assert!(config.validate().is_err());
    }
}