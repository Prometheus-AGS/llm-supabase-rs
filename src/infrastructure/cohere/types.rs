//! Cohere API types and configuration
//! 
//! This module defines all the Cohere-specific types, request/response structures,
//! and configuration needed for the Cohere provider implementation.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use anyhow::Result;
use crate::models::common::{ToolDefinition, ChatMessage};

/// Cohere provider configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CohereConfig {
    /// Cohere API key
    pub api_key: String,
    
    /// Optional base URL override (defaults to https://api.cohere.ai/v1)
    pub base_url: Option<String>,
    
    /// Default model to use
    pub default_model: Option<String>,
    
    /// Request timeout in seconds
    pub timeout_seconds: Option<u64>,
    
    /// Maximum number of retries
    pub max_retries: Option<u32>,
    
    /// Rate limiting configuration
    pub rate_limit: Option<RateLimitConfig>,
}

/// Rate limiting configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RateLimitConfig {
    /// Requests per minute
    pub requests_per_minute: u32,
    
    /// Tokens per minute
    pub tokens_per_minute: u32,
}

/// Available Cohere models
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum CohereModel {
    #[serde(rename = "command-r-plus")]
    CommandRPlus,
    
    #[serde(rename = "command-r")]
    CommandR,
    
    #[serde(rename = "command")]
    Command,
    
    #[serde(rename = "command-nightly")]
    CommandNightly,
}

impl CohereModel {
    /// Get the string representation of the model
    pub fn as_str(&self) -> &'static str {
        match self {
            CohereModel::CommandRPlus => "command-r-plus",
            CohereModel::CommandR => "command-r",
            CohereModel::Command => "command",
            CohereModel::CommandNightly => "command-nightly",
        }
    }
    
    /// Parse model from string
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "command-r-plus" => Some(CohereModel::CommandRPlus),
            "command-r" => Some(CohereModel::CommandR),
            "command" => Some(CohereModel::Command),
            "command-nightly" => Some(CohereModel::CommandNightly),
            _ => None,
        }
    }
    
    /// Check if model supports tool calling
    pub fn supports_tools(&self) -> bool {
        match self {
            CohereModel::CommandRPlus | CohereModel::CommandR | CohereModel::CommandNightly => true,
            CohereModel::Command => false,
        }
    }
    
    /// Get maximum context length for the model
    pub fn max_context_length(&self) -> u32 {
        match self {
            CohereModel::CommandRPlus => 128000,
            CohereModel::CommandR => 128000,
            CohereModel::Command => 4096,
            CohereModel::CommandNightly => 128000,
        }
    }
}

impl Default for CohereModel {
    fn default() -> Self {
        CohereModel::CommandRPlus
    }
}

/// Cohere chat request structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CohereChatRequest {
    /// The model to use for generation
    pub model: String,
    
    /// The message to send to the model
    pub message: String,
    
    /// Chat history
    #[serde(skip_serializing_if = "Option::is_none")]
    pub chat_history: Option<Vec<CohereChatMessage>>,
    
    /// Tools available for the model to use
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tools: Option<Vec<CohereTool>>,
    
    /// Tool results from previous tool calls
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_results: Option<Vec<CohereToolResult>>,
    
    /// Whether to stream the response
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stream: Option<bool>,
    
    /// Generation parameters
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f64>,
    
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_tokens: Option<u32>,
    
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_p: Option<f64>,
    
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_k: Option<u32>,
    
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stop_sequences: Option<Vec<String>>,
    
    /// Preamble to set context
    #[serde(skip_serializing_if = "Option::is_none")]
    pub preamble: Option<String>,
}

/// Cohere chat message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CohereChatMessage {
    /// Role of the message sender
    pub role: CohereRole,
    
    /// Content of the message
    pub message: String,
    
    /// Tool calls made in this message
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<Vec<CohereToolCall>>,
}

/// Message roles in Cohere
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum CohereRole {
    User,
    Chatbot,
    System,
    Tool,
}

/// Cohere tool definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CohereTool {
    /// Name of the tool
    pub name: String,
    
    /// Description of what the tool does
    pub description: String,
    
    /// Parameters schema for the tool
    pub parameter_definitions: HashMap<String, CohereParameter>,
}

/// Cohere parameter definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CohereParameter {
    /// Description of the parameter
    pub description: String,
    
    /// Type of the parameter
    #[serde(rename = "type")]
    pub param_type: String,
    
    /// Whether the parameter is required
    pub required: bool,
}

/// Cohere tool call
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CohereToolCall {
    /// Name of the tool being called
    pub name: String,
    
    /// Parameters for the tool call
    pub parameters: HashMap<String, serde_json::Value>,
    
    /// Optional tool call ID
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
}

/// Cohere tool result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CohereToolResult {
    /// Call information
    pub call: CohereToolCall,
    
    /// Outputs from the tool execution
    pub outputs: Vec<HashMap<String, serde_json::Value>>,
}

/// Cohere chat response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CohereChatResponse {
    /// Generated text
    pub text: String,
    
    /// Generation ID
    pub generation_id: Option<String>,
    
    /// Tool calls made by the model
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<Vec<CohereToolCall>>,
    
    /// Citations for the response
    #[serde(skip_serializing_if = "Option::is_none")]
    pub citations: Option<Vec<CohereCitation>>,
    
    /// Response metadata
    #[serde(skip_serializing_if = "Option::is_none")]
    pub meta: Option<CohereResponseMeta>,
    
    /// Usage statistics
    #[serde(skip_serializing_if = "Option::is_none")]
    pub usage: Option<CohereUsage>,
}

/// Cohere citation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CohereCitation {
    /// Start position in the text
    pub start: usize,
    
    /// End position in the text
    pub end: usize,
    
    /// Text that was cited
    pub text: String,
    
    /// Documents that support this citation
    pub document_ids: Vec<String>,
}

/// Response metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CohereResponseMeta {
    /// API version used
    pub api_version: Option<String>,
    
    /// Billing information
    pub billed_units: Option<CohereBilledUnits>,
}

/// Billed units for usage tracking
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CohereBilledUnits {
    /// Input tokens
    pub input_tokens: Option<u32>,
    
    /// Output tokens
    pub output_tokens: Option<u32>,
}

/// Usage statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CohereUsage {
    /// Total tokens used
    pub tokens: u32,
    
    /// Input tokens
    pub input_tokens: u32,
    
    /// Output tokens
    pub output_tokens: u32,
}

/// Cohere streaming response chunk
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CohereStreamChunk {
    /// Event type
    #[serde(rename = "event_type")]
    pub event_type: CohereEventType,
    
    /// Text delta for text generation
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    
    /// Tool calls for tool-call-generation events
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<Vec<CohereToolCall>>,
    
    /// Final response for stream-end events
    #[serde(skip_serializing_if = "Option::is_none")]
    pub response: Option<CohereChatResponse>,
    
    /// Error information
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<CohereError>,
}

/// Cohere streaming event types
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum CohereEventType {
    /// Stream started
    StreamStart,
    
    /// Text generation in progress
    TextGeneration,
    
    /// Tool calls generated
    ToolCallsGeneration,
    
    /// Stream ended
    StreamEnd,
    
    /// Error occurred
    Error,
}

/// Cohere API error
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CohereError {
    /// Error message
    pub message: String,
    
    /// Error type
    #[serde(rename = "type")]
    pub error_type: Option<String>,
    
    /// Error code
    pub code: Option<String>,
}

impl CohereConfig {
    /// Create configuration from environment variables
    pub fn from_env() -> Result<Self> {
        let api_key = std::env::var("COHERE_API_KEY")
            .map_err(|_| anyhow::anyhow!("COHERE_API_KEY environment variable not set"))?;
            
        Ok(Self {
            api_key,
            base_url: std::env::var("COHERE_BASE_URL").ok(),
            default_model: std::env::var("COHERE_DEFAULT_MODEL").ok(),
            timeout_seconds: std::env::var("COHERE_TIMEOUT_SECONDS")
                .ok()
                .and_then(|s| s.parse().ok()),
            max_retries: std::env::var("COHERE_MAX_RETRIES")
                .ok()
                .and_then(|s| s.parse().ok()),
            rate_limit: None, // Can be extended to read from env
        })
    }
    
    /// Get the base URL for API requests
    pub fn get_base_url(&self) -> String {
        self.base_url
            .clone()
            .unwrap_or_else(|| "https://api.cohere.ai/v1".to_string())
    }
    
    /// Get the default model
    pub fn get_default_model(&self) -> String {
        self.default_model
            .clone()
            .unwrap_or_else(|| CohereModel::default().as_str().to_string())
    }
    
    /// Get timeout in seconds
    pub fn get_timeout_seconds(&self) -> u64 {
        self.timeout_seconds.unwrap_or(30)
    }
    
    /// Get maximum retries
    pub fn get_max_retries(&self) -> u32 {
        self.max_retries.unwrap_or(3)
    }
}

impl Default for CohereConfig {
    fn default() -> Self {
        Self {
            api_key: String::new(),
            base_url: None,
            default_model: None,
            timeout_seconds: None,
            max_retries: None,
            rate_limit: None,
        }
    }
}

/// Helper functions for converting between formats
impl CohereTool {
    /// Convert from OpenAI tool definition
    pub fn from_openai_tool(tool: &ToolDefinition) -> Self {
        let mut parameter_definitions = HashMap::new();
        
        if let Some(params) = &tool.function.parameters {
            if let Some(properties) = params.get("properties") {
                if let Some(props_obj) = properties.as_object() {
                    for (name, prop) in props_obj {
                        if let Some(prop_obj) = prop.as_object() {
                            let param_type = prop_obj.get("type")
                                .and_then(|t| t.as_str())
                                .unwrap_or("string")
                                .to_string();
                                
                            let description = prop_obj.get("description")
                                .and_then(|d| d.as_str())
                                .unwrap_or("")
                                .to_string();
                            
                            // Check if parameter is required
                            let required = if let Some(required_array) = params.get("required") {
                                required_array.as_array()
                                    .map(|arr| arr.iter().any(|v| v.as_str() == Some(name)))
                                    .unwrap_or(false)
                            } else {
                                false
                            };
                            
                            parameter_definitions.insert(name.clone(), CohereParameter {
                                description,
                                param_type,
                                required,
                            });
                        }
                    }
                }
            }
        }
        
        Self {
            name: tool.function.name.clone(),
            description: tool.function.description.clone().unwrap_or_default(),
            parameter_definitions,
        }
    }
}

impl CohereChatMessage {
    /// Convert from generic chat message
    pub fn from_chat_message(msg: &ChatMessage) -> Self {
        let role = match msg.role.as_str() {
            "user" => CohereRole::User,
            "assistant" => CohereRole::Chatbot,
            "system" => CohereRole::System,
            "tool" => CohereRole::Tool,
            _ => CohereRole::User,
        };
        
        Self {
            role,
            message: msg.content.clone(),
            tool_calls: None, // Will be populated separately if needed
        }
    }
}