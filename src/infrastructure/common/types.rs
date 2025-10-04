// src/infrastructure/common/types.rs
//
// Common types and enums used across all providers

use serde::{Deserialize, Serialize};

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_provider_from_string() {
        assert_eq!(AIProvider::parse("vertex_ai"), Some(AIProvider::VertexAI));
        assert_eq!(AIProvider::parse("VERTEX"), Some(AIProvider::VertexAI));
        assert_eq!(AIProvider::parse("gcp"), Some(AIProvider::VertexAI));
        assert_eq!(AIProvider::parse("bedrock"), Some(AIProvider::Bedrock));
        assert_eq!(AIProvider::parse("aws"), Some(AIProvider::Bedrock));
        assert_eq!(AIProvider::parse("invalid"), None);
    }

    #[test]
    fn test_provider_as_str() {
        assert_eq!(AIProvider::VertexAI.as_str(), "vertex_ai");
        assert_eq!(AIProvider::Bedrock.as_str(), "bedrock");
        assert_eq!(AIProvider::OpenAI.as_str(), "openai");
    }

    #[test]
    fn test_supported_models() {
        let vertex_models = AIProvider::VertexAI.supported_models();
        assert!(vertex_models.contains(&"claude-sonnet-4-5@20250929"));
        
        let openai_models = AIProvider::OpenAI.supported_models();
        assert!(openai_models.contains(&"gpt-4o"));
    }

    #[test]
    fn test_model_mapping() {
        let mapping = ModelMapping::new(
            "claude-4-sonnet",
            "claude-sonnet-4-5@20250929",
            AIProvider::VertexAI,
            ProviderCapabilities {
                streaming: true,
                function_calling: true,
                vision: true,
                max_tokens: Some(200_000),
                max_context_length: Some(200_000),
                input_types: vec![InputType::Text],
                output_formats: vec![OutputFormat::Text],
            },
        );

        assert_eq!(mapping.openai_name, "claude-4-sonnet");
        assert_eq!(mapping.provider_name, "claude-sonnet-4-5@20250929");
        assert_eq!(mapping.provider, AIProvider::VertexAI);
        assert!(mapping.capabilities.streaming);
    }
}
