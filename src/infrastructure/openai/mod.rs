//! OpenAI provider implementation for LLM Supabase RS
//! 
//! This module provides comprehensive OpenAI integration with full tool calling support,
//! streaming capabilities, and seamless integration with the existing infrastructure.
//! 
//! # Features
//! 
//! - **Complete OpenAI API compatibility** - Full support for chat completions endpoint
//! - **Native tool calling** - Supports both legacy (sequential) and modern (parallel) tool calling
//! - **Streaming support** - Server-Sent Events (SSE) parsing for real-time responses
//! - **Unified conversion** - Leverages existing tool calling infrastructure for consistency
//! - **Authentication** - API key-based authentication with validation
//! - **Error handling** - Comprehensive error handling with retries and rate limiting
//! - **Model support** - gpt-4o, gpt-4o-mini, gpt-4-turbo, gpt-3.5-turbo
//! 
//! # Usage
//! 
//! ```rust
//! use crate::infrastructure::openai::{OpenAIClient, OpenAIConfig};
//! 
//! // Create client from environment
//! let client = OpenAIClient::from_env().await?;
//! 
//! // Or with custom config
//! let config = OpenAIConfig {
//!     api_key: "sk-...".to_string(),
//!     base_url: Some("https://api.openai.com/v1".to_string()),
//!     ..Default::default()
//! };
//! let client = OpenAIClient::new(config).await?;
//! 
//! // Make a chat completion request
//! let request = ChatCompletionRequest {
//!     model: "gpt-4o".to_string(),
//!     messages: vec![/* messages */],
//!     tools: Some(vec![/* tool definitions */]),
//!     ..Default::default()
//! };
//! 
//! let response = client.chat_completion(request).await?;
//! ```

pub mod auth;
pub mod client;
pub mod converter;
pub mod streaming;
pub mod types;

// Re-export main types and client for convenience
pub use auth::OpenAIAuth;
pub use client::OpenAIClient;
pub use converter::{OpenAIConverter, ModelCapabilities, ToolCallContext};
pub use streaming::{OpenAIStreamParser, OpenAIStreamUtils};
pub use types::OpenAIModel;
pub use crate::config::providers::OpenAIConfig;

// Re-export commonly used types from models
pub use crate::models::request::ChatCompletionRequest;
pub use crate::models::response::{ChatCompletionResponse, ChatCompletionChunk};
pub use crate::models::common::{ToolDefinition, ChatMessage, MessageRole};

use anyhow::Result;
use std::sync::Arc;
use tracing::{info, debug, warn};

/// OpenAI provider implementation
/// 
/// This struct encapsulates the OpenAI provider and integrates it with the
/// existing infrastructure for provider fallback and health monitoring.
#[derive(Clone)]
pub struct OpenAIProvider {
    client: Arc<OpenAIClient>,
    converter: Arc<OpenAIConverter>,
    config: OpenAIConfig,
}

impl OpenAIProvider {
    /// Create a new OpenAI provider
    pub async fn new(config: OpenAIConfig) -> Result<Self> {
        info!("Initializing OpenAI provider");

        let client = OpenAIClient::new(config.clone()).await?;
        let converter = OpenAIConverter::new();

        let provider = Self {
            client: Arc::new(client),
            converter: Arc::new(converter),
            config,
        };

        info!("OpenAI provider initialized successfully");
        Ok(provider)
    }

    /// Create OpenAI provider from environment variables
    pub async fn from_env() -> Result<Self> {
        let config = OpenAIConfig::from_env()?;
        Self::new(config).await
    }

    /// Get the OpenAI client
    pub fn client(&self) -> &OpenAIClient {
        &self.client
    }

    /// Get the converter
    pub fn converter(&self) -> &OpenAIConverter {
        &self.converter
    }

    /// Get the configuration
    pub fn config(&self) -> &OpenAIConfig {
        &self.config
    }

    /// Test provider health
    pub async fn health_check(&self) -> Result<ProviderHealth> {
        debug!("Performing OpenAI provider health check");

        match self.client.test_connection().await {
            Ok(true) => {
                info!("OpenAI provider health check passed");
                Ok(ProviderHealth {
                    provider: "openai".to_string(),
                    status: HealthStatus::Healthy,
                    latency_ms: None, // Could be enhanced to measure actual latency
                    error: None,
                })
            }
            Ok(false) => {
                warn!("OpenAI provider health check failed");
                Ok(ProviderHealth {
                    provider: "openai".to_string(),
                    status: HealthStatus::Unhealthy,
                    latency_ms: None,
                    error: Some("Connection test failed".to_string()),
                })
            }
            Err(e) => {
                warn!("OpenAI provider health check error: {}", e);
                Ok(ProviderHealth {
                    provider: "openai".to_string(),
                    status: HealthStatus::Error,
                    latency_ms: None,
                    error: Some(e.to_string()),
                })
            }
        }
    }

    /// Get supported models
    pub async fn get_supported_models(&self) -> Result<Vec<SupportedModel>> {
        debug!("Getting supported OpenAI models");

        let available_models = match self.client.get_models().await {
            Ok(models) => models,
            Err(_) => {
                // Fallback to known models if API call fails
                warn!("Failed to fetch models from API, using known models");
                vec![
                    "gpt-4o".to_string(),
                    "gpt-4o-mini".to_string(),
                    "gpt-4-turbo".to_string(),
                    "gpt-3.5-turbo".to_string(),
                ]
            }
        };

        let supported_models: Vec<_> = available_models
            .into_iter()
            .filter(|model| self.client.supports_tool_calling(model))
            .map(|model| {
                let capabilities = self.converter.get_model_capabilities(&model);
                SupportedModel {
                    name: model.clone(),
                    provider: "openai".to_string(),
                    supports_tools: capabilities.supports_tools,
                    supports_streaming: capabilities.supports_streaming,
                    max_tokens: match model.as_str() {
                        "gpt-4o" | "gpt-4o-mini" | "gpt-4-turbo" => 128000,
                        "gpt-3.5-turbo" => 16385,
                        _ => 4096,
                    },
                    cost_per_1k_tokens: match model.as_str() {
                        "gpt-4o" => 5.0,
                        "gpt-4o-mini" => 0.15,
                        "gpt-4-turbo" => 10.0,
                        "gpt-3.5-turbo" => 1.5,
                        _ => 2.0,
                    },
                }
            })
            .collect();

        debug!("Found {} supported models", supported_models.len());
        Ok(supported_models)
    }

    /// Get provider capabilities
    pub fn get_capabilities(&self) -> ProviderCapabilities {
        ProviderCapabilities {
            provider: "openai".to_string(),
            supports_chat: true,
            supports_completion: false, // OpenAI deprecated completions
            supports_tools: true,
            supports_streaming: true,
            supports_embeddings: false, // Not implemented in this provider
            max_context_length: 128000,
            supported_formats: vec!["text".to_string(), "json".to_string()],
        }
    }
}

/// Provider health status
#[derive(Debug, Clone)]
pub struct ProviderHealth {
    pub provider: String,
    pub status: HealthStatus,
    pub latency_ms: Option<u64>,
    pub error: Option<String>,
}

/// Health status enumeration
#[derive(Debug, Clone, PartialEq)]
pub enum HealthStatus {
    Healthy,
    Unhealthy,
    Error,
}

/// Supported model information
#[derive(Debug, Clone)]
pub struct SupportedModel {
    pub name: String,
    pub provider: String,
    pub supports_tools: bool,
    pub supports_streaming: bool,
    pub max_tokens: u32,
    pub cost_per_1k_tokens: f64,
}

/// Provider capabilities
#[derive(Debug, Clone)]
pub struct ProviderCapabilities {
    pub provider: String,
    pub supports_chat: bool,
    pub supports_completion: bool,
    pub supports_tools: bool,
    pub supports_streaming: bool,
    pub supports_embeddings: bool,
    pub max_context_length: u32,
    pub supported_formats: Vec<String>,
}

/// OpenAI provider factory for easy initialization
pub struct OpenAIProviderFactory;

impl OpenAIProviderFactory {
    /// Create OpenAI provider with automatic configuration detection
    pub async fn create() -> Result<OpenAIProvider> {
        // Try environment variables first
        if let Ok(provider) = OpenAIProvider::from_env().await {
            return Ok(provider);
        }

        // Fallback to default configuration (will fail if API key not set)
        let config = OpenAIConfig::default();
        OpenAIProvider::new(config).await
    }

    /// Create OpenAI provider with custom configuration
    pub async fn create_with_config(config: OpenAIConfig) -> Result<OpenAIProvider> {
        OpenAIProvider::new(config).await
    }

    /// Create OpenAI provider with minimal configuration
    pub async fn create_with_api_key(api_key: String) -> Result<OpenAIProvider> {
        let config = OpenAIConfig {
            api_key,
            ..Default::default()
        };
        OpenAIProvider::new(config).await
    }

    /// Create test provider with mock configuration
    #[cfg(test)]
    pub async fn create_test() -> Result<OpenAIProvider> {
        let config = OpenAIConfig {
            api_key: "sk-test123456789012345678901234567890123456789012345".to_string(),
            base_url: Some("https://api.openai.com/v1".to_string()),
            ..Default::default()
        };
        OpenAIProvider::new(config).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_provider_creation() {
        let config = OpenAIConfig {
            api_key: "sk-test123456789012345678901234567890123456789012345".to_string(),
            ..Default::default()
        };

        let provider = OpenAIProvider::new(config).await;
        assert!(provider.is_ok());
    }

    #[tokio::test]
    async fn test_provider_capabilities() {
        if let Ok(provider) = OpenAIProviderFactory::create_test().await {
            let capabilities = provider.get_capabilities();
            
            assert_eq!(capabilities.provider, "openai");
            assert!(capabilities.supports_chat);
            assert!(capabilities.supports_tools);
            assert!(capabilities.supports_streaming);
            assert!(!capabilities.supports_completion);
        }
    }

    #[tokio::test]
    async fn test_factory_creation() {
        let result = OpenAIProviderFactory::create_with_api_key(
            "sk-test123456789012345678901234567890123456789012345".to_string()
        ).await;
        
        assert!(result.is_ok());
    }
}