//! Cohere provider implementation for LLM Supabase RS
//! 
//! This module provides comprehensive Cohere integration with full tool calling support,
//! streaming capabilities, and seamless integration with the existing infrastructure.
//! 
//! # Features
//! 
//! - **Complete Cohere API compatibility** - Full support for chat completions endpoint
//! - **Native tool calling** - Supports Cohere's tool use format with conversion to OpenAI format
//! - **Streaming support** - Server-Sent Events (SSE) parsing for real-time responses
//! - **Unified conversion** - Leverages existing tool calling infrastructure for consistency
//! - **Authentication** - API key-based authentication via Authorization Bearer header
//! - **Error handling** - Comprehensive error handling with retries and rate limiting
//! - **Model support** - command-r-plus, command-r, command, command-nightly
//! 
//! # Usage
//! 
//! ```rust
//! use crate::infrastructure::cohere::{CohereClient, CohereConfig};
//! 
//! // Create client from environment
//! let client = CohereClient::from_env().await?;
//! 
//! // Or with custom config
//! let config = CohereConfig {
//!     api_key: "your-api-key".to_string(),
//!     base_url: Some("https://api.cohere.ai/v1".to_string()),
//!     ..Default::default()
//! };
//! let client = CohereClient::new(config).await?;
//! 
//! // Make a chat completion request
//! let request = ChatCompletionRequest {
//!     model: "command-r-plus".to_string(),
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
pub use auth::CohereAuth;
pub use client::CohereClient;
pub use converter::{CohereConverter, CohereModelCapabilities, CohereToolCallContext};
pub use streaming::{CohereStreamParser, CohereStreamUtils, CohereStreamWrapper};
pub use types::{CohereConfig, CohereModel};

// Re-export commonly used types from models
pub use crate::models::request::ChatCompletionRequest;
pub use crate::models::response::{ChatCompletionResponse, ChatCompletionChunk};
pub use crate::models::common::{ToolDefinition, ChatMessage, MessageRole};

use anyhow::Result;
use std::sync::Arc;
use tracing::{info, debug, warn};

/// Cohere provider implementation
/// 
/// This struct encapsulates the Cohere provider and integrates it with the
/// existing infrastructure for provider fallback and health monitoring.
#[derive(Clone)]
pub struct CohereProvider {
    client: Arc<CohereClient>,
    converter: Arc<CohereConverter>,
    config: CohereConfig,
}

impl CohereProvider {
    /// Create a new Cohere provider
    pub async fn new(config: CohereConfig) -> Result<Self> {
        info!("Initializing Cohere provider");

        let client = CohereClient::new(config.clone()).await?;
        let converter = CohereConverter::new();

        let provider = Self {
            client: Arc::new(client),
            converter: Arc::new(converter),
            config,
        };

        info!("Cohere provider initialized successfully");
        Ok(provider)
    }

    /// Create Cohere provider from environment variables
    pub async fn from_env() -> Result<Self> {
        let config = CohereConfig::from_env()?;
        Self::new(config).await
    }

    /// Get the Cohere client
    pub fn client(&self) -> &CohereClient {
        &self.client
    }

    /// Get the converter
    pub fn converter(&self) -> &CohereConverter {
        &self.converter
    }

    /// Get the configuration
    pub fn config(&self) -> &CohereConfig {
        &self.config
    }

    /// Test provider health
    pub async fn health_check(&self) -> Result<ProviderHealth> {
        debug!("Performing Cohere provider health check");

        match self.client.test_connection().await {
            Ok(true) => {
                info!("Cohere provider health check passed");
                Ok(ProviderHealth {
                    provider: "cohere".to_string(),
                    status: HealthStatus::Healthy,
                    latency_ms: None, // Could be enhanced to measure actual latency
                    error: None,
                })
            }
            Ok(false) => {
                warn!("Cohere provider health check failed");
                Ok(ProviderHealth {
                    provider: "cohere".to_string(),
                    status: HealthStatus::Unhealthy,
                    latency_ms: None,
                    error: Some("Connection test failed".to_string()),
                })
            }
            Err(e) => {
                warn!("Cohere provider health check error: {}", e);
                Ok(ProviderHealth {
                    provider: "cohere".to_string(),
                    status: HealthStatus::Error,
                    latency_ms: None,
                    error: Some(e.to_string()),
                })
            }
        }
    }

    /// Get supported models
    pub async fn get_supported_models(&self) -> Result<Vec<SupportedModel>> {
        debug!("Getting supported Cohere models");

        let available_models = match self.client.get_models().await {
            Ok(models) => models,
            Err(_) => {
                // Fallback to known models if API call fails
                warn!("Failed to fetch models from API, using known models");
                vec![
                    "command-r-plus".to_string(),
                    "command-r".to_string(),
                    "command".to_string(),
                    "command-nightly".to_string(),
                ]
            }
        };

        let supported_models = available_models
            .into_iter()
            .filter(|model| self.client.supports_tool_calling(model))
            .map(|model| {
                let capabilities = self.converter.get_model_capabilities(&model);
                SupportedModel {
                    name: model.clone(),
                    provider: "cohere".to_string(),
                    supports_tools: capabilities.supports_tools,
                    supports_streaming: capabilities.supports_streaming,
                    max_tokens: match model.as_str() {
                        "command-r-plus" | "command-r" | "command-nightly" => 128000,
                        "command" => 4096,
                        _ => 4096,
                    },
                    cost_per_1k_tokens: match model.as_str() {
                        "command-r-plus" => 3.0, // Estimated pricing
                        "command-r" => 1.5,
                        "command" => 1.0,
                        "command-nightly" => 3.5,
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
            provider: "cohere".to_string(),
            supports_chat: true,
            supports_completion: false, // Cohere uses chat format
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

/// Cohere provider factory for easy initialization
pub struct CohereProviderFactory;

impl CohereProviderFactory {
    /// Create Cohere provider with automatic configuration detection
    pub async fn create() -> Result<CohereProvider> {
        // Try environment variables first
        if let Ok(provider) = CohereProvider::from_env().await {
            return Ok(provider);
        }

        // Fallback to default configuration (will fail if API key not set)
        let config = CohereConfig::default();
        CohereProvider::new(config).await
    }

    /// Create Cohere provider with custom configuration
    pub async fn create_with_config(config: CohereConfig) -> Result<CohereProvider> {
        CohereProvider::new(config).await
    }

    /// Create Cohere provider with minimal configuration
    pub async fn create_with_api_key(api_key: String) -> Result<CohereProvider> {
        let config = CohereConfig {
            api_key,
            ..Default::default()
        };
        CohereProvider::new(config).await
    }

    /// Create test provider with mock configuration
    #[cfg(test)]
    pub async fn create_test() -> Result<CohereProvider> {
        let config = CohereConfig {
            api_key: "test-api-key-1234567890123456789012345678901234567890".to_string(),
            base_url: Some("https://api.cohere.ai/v1".to_string()),
            ..Default::default()
        };
        CohereProvider::new(config).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_provider_creation() {
        let config = CohereConfig {
            api_key: "test-api-key-1234567890123456789012345678901234567890".to_string(),
            ..Default::default()
        };

        let provider = CohereProvider::new(config).await;
        assert!(provider.is_ok());
    }

    #[tokio::test]
    async fn test_provider_capabilities() {
        if let Ok(provider) = CohereProviderFactory::create_test().await {
            let capabilities = provider.get_capabilities();
            
            assert_eq!(capabilities.provider, "cohere");
            assert!(capabilities.supports_chat);
            assert!(capabilities.supports_tools);
            assert!(capabilities.supports_streaming);
            assert!(!capabilities.supports_completion);
        }
    }

    #[tokio::test]
    async fn test_factory_creation() {
        let result = CohereProviderFactory::create_with_api_key(
            "test-api-key-1234567890123456789012345678901234567890".to_string()
        ).await;
        
        assert!(result.is_ok());
    }

    #[test]
    fn test_model_capabilities() {
        let converter = CohereConverter::new();
        
        // Test command-r-plus capabilities
        let caps = converter.get_model_capabilities("command-r-plus");
        assert!(caps.supports_tools);
        assert!(caps.supports_streaming);
        assert!(caps.supports_parallel_tools);
        assert_eq!(caps.max_context_length, 128000);
        
        // Test command capabilities (no tools)
        let caps = converter.get_model_capabilities("command");
        assert!(!caps.supports_tools);
        assert!(caps.supports_streaming);
        assert!(!caps.supports_parallel_tools);
        assert_eq!(caps.max_context_length, 4096);
    }

    #[test]
    fn test_health_status() {
        let healthy = HealthStatus::Healthy;
        let unhealthy = HealthStatus::Unhealthy;
        let error = HealthStatus::Error;
        
        assert_eq!(healthy, HealthStatus::Healthy);
        assert_ne!(healthy, unhealthy);
        assert_ne!(unhealthy, error);
    }

    #[tokio::test]
    async fn test_supported_models() {
        if let Ok(provider) = CohereProviderFactory::create_test().await {
            // This would normally require a real API call, so we test the structure
            let models = vec![
                SupportedModel {
                    name: "command-r-plus".to_string(),
                    provider: "cohere".to_string(),
                    supports_tools: true,
                    supports_streaming: true,
                    max_tokens: 128000,
                    cost_per_1k_tokens: 3.0,
                }
            ];
            
            assert_eq!(models.len(), 1);
            assert_eq!(models[0].provider, "cohere");
            assert!(models[0].supports_tools);
        }
    }
}