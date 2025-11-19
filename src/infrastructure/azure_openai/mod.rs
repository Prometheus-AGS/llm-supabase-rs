//! Azure OpenAI provider implementation for LLM Supabase RS
//! 
//! This module provides comprehensive Azure OpenAI integration with full tool calling support,
//! streaming capabilities, and seamless integration with the existing infrastructure.
//! 
//! # Features
//! 
//! - **Complete Azure OpenAI API compatibility** - Full support for chat completions endpoint
//! - **Native tool calling** - Supports both legacy (sequential) and modern (parallel) tool calling
//! - **Streaming support** - Server-Sent Events (SSE) parsing for real-time responses
//! - **Unified conversion** - Leverages existing tool calling infrastructure for consistency
//! - **Azure Authentication** - API key-based authentication with Azure-specific headers
//! - **Error handling** - Comprehensive error handling with retries and rate limiting
//! - **Model support** - gpt-4o, gpt-4-turbo, gpt-3.5-turbo deployments
//! - **Enterprise ready** - Support for Azure AD authentication and private endpoints
//! 
//! # Azure OpenAI Differences
//! 
//! Azure OpenAI uses the same API format as OpenAI but with key differences:
//! - Uses `api-key` header instead of `Authorization: Bearer`
//! - Different endpoint format: `https://{resource}.openai.azure.com/openai/deployments/{deployment}/chat/completions`
//! - Requires `api-version` query parameter
//! - Uses deployment names instead of model names
//! - Azure-specific error codes and rate limiting
//! 
//! # Usage
//! 
//! ```rust
//! use crate::infrastructure::azure_openai::{AzureOpenAIClient, AzureOpenAIConfig};
//! 
//! // Create client from environment
//! let client = AzureOpenAIClient::from_env().await?;
//! 
//! // Or with custom config
//! let config = AzureOpenAIConfig {
//!     api_key: "your-api-key".to_string(),
//!     endpoint: "https://your-resource.openai.azure.com".to_string(),
//!     deployment: "gpt-4o".to_string(),
//!     api_version: "2024-02-15-preview".to_string(),
//!     ..Default::default()
//! };
//! let client = AzureOpenAIClient::new(config).await?;
//! 
//! // Make a chat completion request
//! let request = ChatCompletionRequest {
//!     model: "gpt-4o".to_string(), // Will be mapped to deployment
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
pub use auth::AzureOpenAIAuth;
pub use client::AzureOpenAIClient;
pub use converter::{AzureOpenAIConverter, ModelCapabilities, ToolCallContext};
pub use streaming::{AzureOpenAIStreamParser, AzureOpenAIStreamUtils};
pub use types::{AzureOpenAIConfig, AzureOpenAIModel};

// Re-export commonly used types from models
pub use crate::models::request::ChatCompletionRequest;
pub use crate::models::response::{ChatCompletionResponse, ChatCompletionChunk};
pub use crate::models::common::{ToolDefinition, ChatMessage, MessageRole};

use anyhow::Result;
use std::sync::Arc;
use tracing::{info, debug, warn};

/// Azure OpenAI provider implementation
/// 
/// This struct encapsulates the Azure OpenAI provider and integrates it with the
/// existing infrastructure for provider fallback and health monitoring.
#[derive(Clone)]
pub struct AzureOpenAIProvider {
    client: Arc<AzureOpenAIClient>,
    converter: Arc<AzureOpenAIConverter>,
    config: AzureOpenAIConfig,
}

impl AzureOpenAIProvider {
    /// Create a new Azure OpenAI provider
    pub async fn new(config: AzureOpenAIConfig) -> Result<Self> {
        info!("Initializing Azure OpenAI provider");

        let client = AzureOpenAIClient::new(config.clone()).await?;
        let converter = AzureOpenAIConverter::new();

        let provider = Self {
            client: Arc::new(client),
            converter: Arc::new(converter),
            config,
        };

        info!("Azure OpenAI provider initialized successfully");
        Ok(provider)
    }

    /// Create Azure OpenAI provider from environment variables
    pub async fn from_env() -> Result<Self> {
        let config = AzureOpenAIConfig::from_env()?;
        Self::new(config).await
    }

    /// Get the Azure OpenAI client
    pub fn client(&self) -> &AzureOpenAIClient {
        &self.client
    }

    /// Get the converter
    pub fn converter(&self) -> &AzureOpenAIConverter {
        &self.converter
    }

    /// Get the configuration
    pub fn config(&self) -> &AzureOpenAIConfig {
        &self.config
    }

    /// Test provider health
    pub async fn health_check(&self) -> Result<ProviderHealth> {
        debug!("Performing Azure OpenAI provider health check");

        match self.client.test_connection().await {
            Ok(true) => {
                info!("Azure OpenAI provider health check passed");
                Ok(ProviderHealth {
                    provider: "azure_openai".to_string(),
                    status: HealthStatus::Healthy,
                    latency_ms: None, // Could be enhanced to measure actual latency
                    error: None,
                })
            }
            Ok(false) => {
                warn!("Azure OpenAI provider health check failed");
                Ok(ProviderHealth {
                    provider: "azure_openai".to_string(),
                    status: HealthStatus::Unhealthy,
                    latency_ms: None,
                    error: Some("Connection test failed".to_string()),
                })
            }
            Err(e) => {
                warn!("Azure OpenAI provider health check error: {}", e);
                Ok(ProviderHealth {
                    provider: "azure_openai".to_string(),
                    status: HealthStatus::Error,
                    latency_ms: None,
                    error: Some(e.to_string()),
                })
            }
        }
    }

    /// Get supported models
    pub async fn get_supported_models(&self) -> Result<Vec<SupportedModel>> {
        debug!("Getting supported Azure OpenAI models");

        // Azure OpenAI doesn't have a models endpoint like OpenAI
        // Instead, we return the known deployments that support tool calling
        let available_deployments = vec![
            self.config.deployment.clone(),
        ];

        let supported_models = available_deployments
            .into_iter()
            .filter(|deployment| self.client.supports_tool_calling(deployment))
            .map(|deployment| {
                let capabilities = self.converter.get_model_capabilities(&deployment);
                SupportedModel {
                    name: deployment.clone(),
                    provider: "azure_openai".to_string(),
                    supports_tools: capabilities.supports_tools,
                    supports_streaming: capabilities.supports_streaming,
                    max_tokens: match deployment.contains("gpt-4") {
                        true => 128000,
                        false => 16385, // gpt-3.5-turbo
                    },
                    cost_per_1k_tokens: match deployment.as_str() {
                        d if d.contains("gpt-4o") => 5.0,
                        d if d.contains("gpt-4") => 10.0,
                        _ => 1.5, // gpt-3.5-turbo
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
            provider: "azure_openai".to_string(),
            supports_chat: true,
            supports_completion: false, // Azure OpenAI deprecated completions
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

/// Azure OpenAI provider factory for easy initialization
pub struct AzureOpenAIProviderFactory;

impl AzureOpenAIProviderFactory {
    /// Create Azure OpenAI provider with automatic configuration detection
    pub async fn create() -> Result<AzureOpenAIProvider> {
        // Try environment variables first
        if let Ok(provider) = AzureOpenAIProvider::from_env().await {
            return Ok(provider);
        }

        // Fallback to default configuration (will fail if required configs not set)
        let config = AzureOpenAIConfig::default();
        AzureOpenAIProvider::new(config).await
    }

    /// Create Azure OpenAI provider with custom configuration
    pub async fn create_with_config(config: AzureOpenAIConfig) -> Result<AzureOpenAIProvider> {
        AzureOpenAIProvider::new(config).await
    }

    /// Create Azure OpenAI provider with minimal configuration
    pub async fn create_with_credentials(
        api_key: String, 
        endpoint: String, 
        deployment: String
    ) -> Result<AzureOpenAIProvider> {
        let config = AzureOpenAIConfig {
            api_key,
            endpoint,
            deployment,
            ..Default::default()
        };
        AzureOpenAIProvider::new(config).await
    }

    /// Create test provider with mock configuration
    #[cfg(test)]
    pub async fn create_test() -> Result<AzureOpenAIProvider> {
        let config = AzureOpenAIConfig {
            api_key: "test-api-key-12345678901234567890123456789012".to_string(),
            endpoint: "https://test-resource.openai.azure.com".to_string(),
            deployment: "gpt-4o".to_string(),
            api_version: "2024-02-15-preview".to_string(),
            ..Default::default()
        };
        AzureOpenAIProvider::new(config).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_provider_creation() {
        let config = AzureOpenAIConfig {
            api_key: "test-api-key-12345678901234567890123456789012".to_string(),
            endpoint: "https://test-resource.openai.azure.com".to_string(),
            deployment: "gpt-4o".to_string(),
            api_version: "2024-02-15-preview".to_string(),
            ..Default::default()
        };

        let provider = AzureOpenAIProvider::new(config).await;
        assert!(provider.is_ok());
    }

    #[tokio::test]
    async fn test_provider_capabilities() {
        if let Ok(provider) = AzureOpenAIProviderFactory::create_test().await {
            let capabilities = provider.get_capabilities();
            
            assert_eq!(capabilities.provider, "azure_openai");
            assert!(capabilities.supports_chat);
            assert!(capabilities.supports_tools);
            assert!(capabilities.supports_streaming);
            assert!(!capabilities.supports_completion);
        }
    }

    #[tokio::test]
    async fn test_factory_creation() {
        let result = AzureOpenAIProviderFactory::create_with_credentials(
            "test-api-key-12345678901234567890123456789012".to_string(),
            "https://test-resource.openai.azure.com".to_string(),
            "gpt-4o".to_string()
        ).await;
        
        assert!(result.is_ok());
    }
}