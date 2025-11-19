//! Anthropic Claude provider implementation for LLM Supabase RS
//! 
//! This module provides comprehensive Anthropic Claude integration with full tool calling support,
//! streaming capabilities, and seamless integration with the existing infrastructure.
//! 
//! # Features
//! 
//! - **Complete Anthropic API compatibility** - Full support for messages endpoint
//! - **Native tool calling** - Supports Claude's tool use format with conversion to OpenAI format
//! - **Streaming support** - Server-Sent Events (SSE) parsing for real-time responses
//! - **Unified conversion** - Leverages existing tool calling infrastructure for consistency
//! - **Authentication** - API key-based authentication via x-api-key header
//! - **Error handling** - Comprehensive error handling with retries and rate limiting
//! - **Model support** - claude-3-5-sonnet-20241022, claude-3-5-haiku-20241022, claude-3-opus-20240229
//! 
//! # Usage
//! 
//! ```rust
//! use crate::infrastructure::anthropic::{AnthropicClient, AnthropicConfig};
//! 
//! // Create client from environment
//! let client = AnthropicClient::from_env().await?;
//! 
//! // Or with custom config
//! let config = AnthropicConfig {
//!     api_key: "sk-ant-...".to_string(),
//!     base_url: Some("https://api.anthropic.com/v1".to_string()),
//!     ..Default::default()
//! };
//! let client = AnthropicClient::new(config).await?;
//! 
//! // Make a chat completion request
//! let request = ChatCompletionRequest {
//!     model: "claude-3-5-sonnet-20241022".to_string(),
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
pub use auth::AnthropicAuth;
pub use client::AnthropicClient;
pub use converter::{AnthropicConverter, AnthropicModelCapabilities, AnthropicToolCallContext};
pub use streaming::{AnthropicStreamParser, AnthropicStreamUtils};
pub use types::{AnthropicConfig, AnthropicModel};

// Re-export commonly used types from models
pub use crate::models::request::ChatCompletionRequest;
pub use crate::models::response::{ChatCompletionResponse, ChatCompletionChunk};
pub use crate::models::common::{ToolDefinition, ChatMessage, MessageRole};

use anyhow::Result;
use std::sync::Arc;
use tracing::{info, debug, warn};

/// Anthropic provider implementation
/// 
/// This struct encapsulates the Anthropic provider and integrates it with the
/// existing infrastructure for provider fallback and health monitoring.
#[derive(Clone)]
pub struct AnthropicProvider {
    client: Arc<AnthropicClient>,
    converter: Arc<AnthropicConverter>,
    config: AnthropicConfig,
}

impl AnthropicProvider {
    /// Create a new Anthropic provider
    pub async fn new(config: AnthropicConfig) -> Result<Self> {
        info!("Initializing Anthropic provider");

        let client = AnthropicClient::new(config.clone()).await?;
        let converter = AnthropicConverter::new();

        let provider = Self {
            client: Arc::new(client),
            converter: Arc::new(converter),
            config,
        };

        info!("Anthropic provider initialized successfully");
        Ok(provider)
    }

    /// Create Anthropic provider from environment variables
    pub async fn from_env() -> Result<Self> {
        let config = AnthropicConfig::from_env()?;
        Self::new(config).await
    }

    /// Get the Anthropic client
    pub fn client(&self) -> &AnthropicClient {
        &self.client
    }

    /// Get the converter
    pub fn converter(&self) -> &AnthropicConverter {
        &self.converter
    }

    /// Get the configuration
    pub fn config(&self) -> &AnthropicConfig {
        &self.config
    }

    /// Test provider health
    pub async fn health_check(&self) -> Result<ProviderHealth> {
        debug!("Performing Anthropic provider health check");

        match self.client.test_connection().await {
            Ok(true) => {
                info!("Anthropic provider health check passed");
                Ok(ProviderHealth {
                    provider: "anthropic".to_string(),
                    status: HealthStatus::Healthy,
                    latency_ms: None, // Could be enhanced to measure actual latency
                    error: None,
                })
            }
            Ok(false) => {
                warn!("Anthropic provider health check failed");
                Ok(ProviderHealth {
                    provider: "anthropic".to_string(),
                    status: HealthStatus::Unhealthy,
                    latency_ms: None,
                    error: Some("Connection test failed".to_string()),
                })
            }
            Err(e) => {
                warn!("Anthropic provider health check error: {}", e);
                Ok(ProviderHealth {
                    provider: "anthropic".to_string(),
                    status: HealthStatus::Error,
                    latency_ms: None,
                    error: Some(e.to_string()),
                })
            }
        }
    }

    /// Get supported models
    pub async fn get_supported_models(&self) -> Result<Vec<SupportedModel>> {
        debug!("Getting supported Anthropic models");

        // Anthropic doesn't provide a models endpoint, so we return known models
        let supported_models = vec![
            SupportedModel {
                name: "claude-3-5-sonnet-20241022".to_string(),
                provider: "anthropic".to_string(),
                supports_tools: true,
                supports_streaming: true,
                max_tokens: 200000,
                cost_per_1k_tokens: 3.0, // Approximate pricing
            },
            SupportedModel {
                name: "claude-3-5-haiku-20241022".to_string(),
                provider: "anthropic".to_string(),
                supports_tools: true,
                supports_streaming: true,
                max_tokens: 200000,
                cost_per_1k_tokens: 0.25, // Approximate pricing
            },
            SupportedModel {
                name: "claude-3-opus-20240229".to_string(),
                provider: "anthropic".to_string(),
                supports_tools: true,
                supports_streaming: true,
                max_tokens: 200000,
                cost_per_1k_tokens: 15.0, // Approximate pricing
            },
        ];

        debug!("Found {} supported models", supported_models.len());
        Ok(supported_models)
    }

    /// Get provider capabilities
    pub fn get_capabilities(&self) -> ProviderCapabilities {
        ProviderCapabilities {
            provider: "anthropic".to_string(),
            supports_chat: true,
            supports_completion: false, // Anthropic uses messages API
            supports_tools: true,
            supports_streaming: true,
            supports_embeddings: false, // Not implemented in this provider
            max_context_length: 200000,
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

/// Anthropic provider factory for easy initialization
pub struct AnthropicProviderFactory;

impl AnthropicProviderFactory {
    /// Create Anthropic provider with automatic configuration detection
    pub async fn create() -> Result<AnthropicProvider> {
        // Try environment variables first
        if let Ok(provider) = AnthropicProvider::from_env().await {
            return Ok(provider);
        }

        // Fallback to default configuration (will fail if API key not set)
        let config = AnthropicConfig::default();
        AnthropicProvider::new(config).await
    }

    /// Create Anthropic provider with custom configuration
    pub async fn create_with_config(config: AnthropicConfig) -> Result<AnthropicProvider> {
        AnthropicProvider::new(config).await
    }

    /// Create Anthropic provider with minimal configuration
    pub async fn create_with_api_key(api_key: String) -> Result<AnthropicProvider> {
        let config = AnthropicConfig {
            api_key,
            ..Default::default()
        };
        AnthropicProvider::new(config).await
    }

    /// Create test provider with mock configuration
    #[cfg(test)]
    pub async fn create_test() -> Result<AnthropicProvider> {
        let config = AnthropicConfig {
            api_key: "sk-ant-test123456789012345678901234567890123456789012345".to_string(),
            base_url: Some("https://api.anthropic.com/v1".to_string()),
            ..Default::default()
        };
        AnthropicProvider::new(config).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_provider_creation() {
        let config = AnthropicConfig {
            api_key: "sk-ant-test123456789012345678901234567890123456789012345".to_string(),
            ..Default::default()
        };

        let provider = AnthropicProvider::new(config).await;
        assert!(provider.is_ok());
    }

    #[tokio::test]
    async fn test_provider_capabilities() {
        if let Ok(provider) = AnthropicProviderFactory::create_test().await {
            let capabilities = provider.get_capabilities();
            
            assert_eq!(capabilities.provider, "anthropic");
            assert!(capabilities.supports_chat);
            assert!(capabilities.supports_tools);
            assert!(capabilities.supports_streaming);
            assert!(!capabilities.supports_completion);
        }
    }

    #[tokio::test]
    async fn test_factory_creation() {
        let result = AnthropicProviderFactory::create_with_api_key(
            "sk-ant-test123456789012345678901234567890123456789012345".to_string()
        ).await;
        
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_supported_models() {
        if let Ok(provider) = AnthropicProviderFactory::create_test().await {
            let models = provider.get_supported_models().await;
            assert!(models.is_ok());
            
            let models = models.unwrap();
            assert!(!models.is_empty());
            
            // Check for specific Claude models
            let model_names: Vec<String> = models.iter().map(|m| m.name.clone()).collect();
            assert!(model_names.contains(&"claude-3-5-sonnet-20241022".to_string()));
            assert!(model_names.contains(&"claude-3-5-haiku-20241022".to_string()));
            assert!(model_names.contains(&"claude-3-opus-20240229".to_string()));
        }
    }
}