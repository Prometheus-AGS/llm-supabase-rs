use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

// Authentication types
#[derive(Debug, Clone)]
pub struct AuthContext {
    pub user_id: Option<Uuid>,
    pub token_type: TokenType,
    pub is_authenticated: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TokenType {
    User(Uuid),
    Anon,
    ServiceRole,
}

/// Supported AI providers
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum AIProvider {
    #[serde(rename = "vertex_ai")]
    VertexAI,
    #[serde(rename = "bedrock")]
    Bedrock,
    #[serde(rename = "openai")]
    OpenAI,
    #[serde(rename = "anthropic")]
    Anthropic,
    #[serde(rename = "azure")]
    Azure,
}

impl AIProvider {
    /// Get the provider name as a string
    pub fn as_str(&self) -> &'static str {
        match self {
            AIProvider::VertexAI => "vertex_ai",
            AIProvider::Bedrock => "bedrock",
            AIProvider::OpenAI => "openai",
            AIProvider::Anthropic => "anthropic",
            AIProvider::Azure => "azure",
        }
    }

    /// Parse provider from string
    pub fn parse(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "vertex_ai" | "vertex" | "gcp" => Some(AIProvider::VertexAI),
            "bedrock" | "aws" => Some(AIProvider::Bedrock),
            "openai" => Some(AIProvider::OpenAI),
            "anthropic" => Some(AIProvider::Anthropic),
            "azure" => Some(AIProvider::Azure),
            _ => None,
        }
    }

    /// Get supported models for this provider
    pub fn supported_models(&self) -> Vec<&'static str> {
        match self {
            AIProvider::VertexAI => vec![
                "claude-sonnet-4-5@20250929",
                "claude-3-5-haiku@20241022",
                "claude-3-5-sonnet@20241022",
            ],
            AIProvider::Bedrock => vec![
                "anthropic.claude-3-5-sonnet-20241022-v2:0",
                "anthropic.claude-3-5-haiku-20241022-v1:0",
                "anthropic.claude-3-opus-20240229-v1:0",
            ],
            AIProvider::OpenAI => vec![
                "gpt-4o",
                "gpt-4o-mini",
                "gpt-4-turbo",
                "gpt-3.5-turbo",
            ],
            AIProvider::Anthropic => vec![
                "claude-3-5-sonnet-20241022",
                "claude-3-5-haiku-20241022",
                "claude-3-opus-20240229",
            ],
            AIProvider::Azure => vec![
                "gpt-4o",
                "gpt-4-turbo",
                "gpt-35-turbo",
            ],
        }
    }
}

/// Provider capabilities
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderCapabilities {
    /// Whether the provider supports streaming
    pub streaming: bool,
    
    /// Whether the provider supports function calling
    pub function_calling: bool,
    
    /// Whether the provider supports vision/image inputs
    pub vision: bool,
    
    /// Maximum tokens supported
    pub max_tokens: Option<u32>,
    
    /// Maximum context length
    pub max_context_length: Option<u32>,
    
    /// Supported input types
    pub input_types: Vec<InputType>,
    
    /// Supported output formats
    pub output_formats: Vec<OutputFormat>,
}

/// Input types supported by providers
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum InputType {
    Text,
    Image,
    Audio,
    Video,
    Document,
}

/// Output formats supported by providers
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum OutputFormat {
    Text,
    Json,
    Structured,
    FunctionCall,
}

/// Provider configuration trait
pub trait ProviderConfig {
    /// Get the provider type
    fn provider(&self) -> AIProvider;
    
    /// Get provider capabilities
    fn capabilities(&self) -> ProviderCapabilities;
    
    /// Validate the configuration
    fn validate(&self) -> anyhow::Result<()>;
    
    /// Get the base URL for API calls
    fn base_url(&self) -> String;
    
    /// Get authentication headers
    fn auth_headers(&self) -> std::collections::HashMap<String, String>;
}

/// Model mapping for cross-provider compatibility
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelMapping {
    /// OpenAI-compatible model name
    pub openai_name: String,
    
    /// Provider-specific model name
    pub provider_name: String,
    
    /// Provider this mapping applies to
    pub provider: AIProvider,
    
    /// Model capabilities
    pub capabilities: ProviderCapabilities,
}

impl ModelMapping {
    /// Create a new model mapping
    pub fn new(
        openai_name: impl Into<String>,
        provider_name: impl Into<String>,
        provider: AIProvider,
        capabilities: ProviderCapabilities,
    ) -> Self {
        Self {
            openai_name: openai_name.into(),
            provider_name: provider_name.into(),
            provider,
            capabilities,
        }
    }
}

/// Tool call structure matching OpenAI API format
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCall {
    /// Unique identifier for the tool call
    pub id: String,
    
    /// Tool type, typically "function"
    #[serde(rename = "type")]
    pub tool_type: String,
    
    /// Function call details
    pub function: FunctionCall,
}

/// Function call structure matching OpenAI API format
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionCall {
    /// Name of the function to call
    pub name: String,
    
    /// Arguments to pass to the function (as JSON string)
    pub arguments: String,
}

/// Tool call delta for streaming (incremental updates)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCallDelta {
    /// Tool call index in the array
    pub index: u32,
    
    /// Tool call ID
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    
    /// Tool type, typically "function"
    #[serde(rename = "type", skip_serializing_if = "Option::is_none")]
    pub tool_type: Option<String>,
    
    /// Function call delta
    #[serde(skip_serializing_if = "Option::is_none")]
    pub function: Option<FunctionCallDelta>,
}

/// Function call delta for streaming
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionCallDelta {
    /// Function name (may be partial in streaming)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    
    /// Function arguments (may be partial in streaming)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub arguments: Option<String>,
}

/// Message content structure supporting text and multimodal content
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum MessageContent {
    /// Simple text content
    Text(String),
    
    /// Multimodal content with multiple parts
    Parts(Vec<MessagePart>),
}

/// Message part for multimodal content
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessagePart {
    /// Text content (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    
    /// Image URL (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub image_url: Option<ImageUrl>,
}

/// Image URL structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImageUrl {
    /// URL of the image
    pub url: String,
}

/// Default model mappings for common providers
pub fn default_model_mappings() -> Vec<ModelMapping> {
    vec![
        // Vertex AI mappings
        ModelMapping::new(
            "claude-4-sonnet-20250514",
            "claude-sonnet-4-5@20250929",
            AIProvider::VertexAI,
            ProviderCapabilities {
                streaming: true,
                function_calling: true,
                vision: true,
                max_tokens: Some(200_000),
                max_context_length: Some(200_000),
                input_types: vec![InputType::Text, InputType::Image],
                output_formats: vec![OutputFormat::Text, OutputFormat::Json, OutputFormat::FunctionCall],
            },
        ),
        ModelMapping::new(
            "claude-3-5-haiku",
            "claude-3-5-haiku@20241022",
            AIProvider::VertexAI,
            ProviderCapabilities {
                streaming: true,
                function_calling: true,
                vision: true,
                max_tokens: Some(200_000),
                max_context_length: Some(200_000),
                input_types: vec![InputType::Text, InputType::Image],
                output_formats: vec![OutputFormat::Text, OutputFormat::Json, OutputFormat::FunctionCall],
            },
        ),
        // Bedrock mappings
        ModelMapping::new(
            "claude-4-sonnet-20250514",
            "anthropic.claude-3-5-sonnet-20241022-v2:0",
            AIProvider::Bedrock,
            ProviderCapabilities {
                streaming: true,
                function_calling: true,
                vision: true,
                max_tokens: Some(200_000),
                max_context_length: Some(200_000),
                input_types: vec![InputType::Text, InputType::Image],
                output_formats: vec![OutputFormat::Text, OutputFormat::Json, OutputFormat::FunctionCall],
            },
        ),
        // OpenAI mappings (pass-through)
        ModelMapping::new(
            "gpt-4o",
            "gpt-4o",
            AIProvider::OpenAI,
            ProviderCapabilities {
                streaming: true,
                function_calling: true,
                vision: true,
                max_tokens: Some(128_000),
                max_context_length: Some(128_000),
                input_types: vec![InputType::Text, InputType::Image, InputType::Audio],
                output_formats: vec![OutputFormat::Text, OutputFormat::Json, OutputFormat::FunctionCall],
            },
        ),
    ]
}
