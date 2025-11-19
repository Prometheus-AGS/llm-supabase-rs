//! AWS Bedrock provider implementation for LLM Supabase RS
//! 
//! This module provides comprehensive AWS Bedrock integration with full tool calling support,
//! streaming capabilities, and seamless integration with the existing infrastructure.
//! 
//! # Features
//! 
//! - **Complete AWS Bedrock API compatibility** - Full support for multiple model families
//! - **Multi-model tool calling** - Supports Claude, Llama, and Mistral models with conversion to OpenAI format
//! - **AWS SigV4 authentication** - Native AWS authentication with multiple credential sources
//! - **Event streaming support** - AWS Bedrock event streams for real-time responses
//! - **Unified conversion** - Leverages existing tool calling infrastructure for consistency
//! - **Regional support** - Support for all AWS regions and data residency requirements
//! - **Enterprise features** - IAM roles, VPC endpoints, CloudTrail integration
//! - **Model support** - Claude 3.5 Sonnet, Haiku, Opus, Llama 3.1, Mistral models
//! 
//! # Usage
//! 
//! ```rust
//! use crate::infrastructure::aws_bedrock::{BedrockClient, BedrockConfig};
//! 
//! // Create client from environment
//! let client = BedrockClient::from_env().await?;
//! 
//! // Or with custom config
//! let config = BedrockConfig {
//!     region: "us-east-1".to_string(),
//!     access_key: Some("AKIAIOSFODNN7EXAMPLE".to_string()),
//!     secret_key: Some("wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY".to_string()),
//!     ..Default::default()
//! };
//! let client = BedrockClient::new(config).await?;
//! 
//! // Make a chat completion request
//! let request = ChatCompletionRequest {
//!     model: "anthropic.claude-3-5-sonnet-20241022-v2:0".to_string(),
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
pub use auth::BedrockAuth;
pub use client::BedrockClient;
pub use converter::{BedrockConverter, BedrockModelCapabilities, BedrockToolCallContext};
pub use streaming::{BedrockStreamParser, BedrockStreamUtils};
pub use types::{BedrockConfig, BedrockModel, BedrockRegion};

// Re-export commonly used types from models
pub use crate::models::request::ChatCompletionRequest;
pub use crate::models::response::{ChatCompletionResponse, ChatCompletionChunk};
pub use crate::models::common::{ToolDefinition, ChatMessage, MessageRole};

use anyhow::Result;
use std::sync::Arc;
use tracing::{info, debug, warn};

/// AWS Bedrock provider implementation
/// 
/// This struct encapsulates the AWS Bedrock provider and integrates it with the
/// existing infrastructure for provider fallback and health monitoring.
#[derive(Clone)]
pub struct BedrockProvider {
    client: Arc<BedrockClient>,
    converter: Arc<BedrockConverter>,
    config: BedrockConfig,
}

impl BedrockProvider {
    /// Create a new AWS Bedrock provider
    pub async fn new(config: BedrockConfig) -> Result<Self> {
        info!("Initializing AWS Bedrock provider for region: {}", config.region);

        let client = BedrockClient::new(config.clone()).await?;
        let converter = BedrockConverter::new();

        let provider = Self {
            client: Arc::new(client),
            converter: Arc::new(converter),
            config,
        };

        info!("AWS Bedrock provider initialized successfully");
        Ok(provider)
    }

    /// Create AWS Bedrock provider from environment variables
    pub async fn from_env() -> Result<Self> {
        let config = BedrockConfig::from_env()?;
        Self::new(config).await
    }

    /// Get the Bedrock client
    pub fn client(&self) -> &BedrockClient {
        &self.client
    }

    /// Get the converter
    pub fn converter(&self) -> &BedrockConverter {
        &self.converter
    }

    /// Get the configuration
    pub fn config(&self) -> &BedrockConfig {
        &self.config
    }

    /// Test provider health
    pub async fn health_check(&self) -> Result<ProviderHealth> {
        debug!("Performing AWS Bedrock provider health check");

        match self.client.test_connection().await {
            Ok(true) => {
                info!("AWS Bedrock provider health check passed");
                Ok(ProviderHealth {
                    provider: "aws_bedrock".to_string(),
                    status: HealthStatus::Healthy,
                    latency_ms: None, // Could be enhanced to measure actual latency
                    error: None,
                })
            }
            Ok(false) => {
                warn!("AWS Bedrock provider health check failed");
                Ok(ProviderHealth {
                    provider: "aws_bedrock".to_string(),
                    status: HealthStatus::Unhealthy,
                    latency_ms: None,
                    error: Some("Connection test failed".to_string()),
                })
            }
            Err(e) => {
                warn!("AWS Bedrock provider health check error: {}", e);
                Ok(ProviderHealth {
                    provider: "aws_bedrock".to_string(),
                    status: HealthStatus::Error,
                    latency_ms: None,
                    error: Some(e.to_string()),
                })
            }
        }
    }

    /// Get supported models
    pub async fn get_supported_models(&self) -> Result<Vec<SupportedModel>> {
        debug!("Getting supported AWS Bedrock models");

        // AWS Bedrock models with tool calling capabilities
        let supported_models = vec![
            SupportedModel {
                name: "anthropic.claude-3-5-sonnet-20241022-v2:0".to_string(),
                provider: "aws_bedrock".to_string(),
                supports_tools: true,
                supports_streaming: true,
                max_tokens: 200000,
                cost_per_1k_tokens: 3.0, // Approximate Bedrock pricing
            },
            SupportedModel {
                name: "anthropic.claude-3-haiku-20240307-v1:0".to_string(),
                provider: "aws_bedrock".to_string(),
                supports_tools: true,
                supports_streaming: true,
                max_tokens: 200000,
                cost_per_1k_tokens: 0.25, // Approximate Bedrock pricing
            },
            SupportedModel {
                name: "anthropic.claude-3-opus-20240229-v1:0".to_string(),
                provider: "aws_bedrock".to_string(),
                supports_tools: true,
                supports_streaming: true,
                max_tokens: 200000,
                cost_per_1k_tokens: 15.0, // Approximate Bedrock pricing
            },
            SupportedModel {
                name: "meta.llama3-1-70b-instruct-v1:0".to_string(),
                provider: "aws_bedrock".to_string(),
                supports_tools: true,
                supports_streaming: true,
                max_tokens: 128000,
                cost_per_1k_tokens: 2.65, // Approximate Bedrock pricing
            },
            SupportedModel {
                name: "meta.llama3-1-8b-instruct-v1:0".to_string(),
                provider: "aws_bedrock".to_string(),
                supports_tools: true,
                supports_streaming: true,
                max_tokens: 128000,
                cost_per_1k_tokens: 0.22, // Approximate Bedrock pricing
            },
            SupportedModel {
                name: "mistral.mistral-7b-instruct-v0:2".to_string(),
                provider: "aws_bedrock".to_string(),
                supports_tools: false, // Most Mistral models don't support tool calling yet
                supports_streaming: true,
                max_tokens: 32000,
                cost_per_1k_tokens: 0.15, // Approximate Bedrock pricing
            },
        ];

        debug!("Found {} supported models", supported_models.len());
        Ok(supported_models)
    }

    /// Get provider capabilities
    pub fn get_capabilities(&self) -> ProviderCapabilities {
        ProviderCapabilities {
            provider: "aws_bedrock".to_string(),
            supports_chat: true,
            supports_completion: false, // Bedrock uses invoke model API
            supports_tools: true,
            supports_streaming: true,
            supports_embeddings: true, // Bedrock supports embedding models
            max_context_length: 200000, // Depends on model, Claude has highest
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

/// AWS Bedrock provider factory for easy initialization
pub struct BedrockProviderFactory;

impl BedrockProviderFactory {
    /// Create AWS Bedrock provider with automatic configuration detection
    pub async fn create() -> Result<BedrockProvider> {
        // Try environment variables first
        if let Ok(provider) = BedrockProvider::from_env().await {
            return Ok(provider);
        }

        // Fallback to default configuration (will use AWS credential chain)
        let config = BedrockConfig::default();
        BedrockProvider::new(config).await
    }

    /// Create AWS Bedrock provider with custom configuration
    pub async fn create_with_config(config: BedrockConfig) -> Result<BedrockProvider> {
        BedrockProvider::new(config).await
    }

    /// Create AWS Bedrock provider with access key and secret
    pub async fn create_with_credentials(
        access_key: String,
        secret_key: String,
        region: String,
    ) -> Result<BedrockProvider> {
        let config = BedrockConfig {
            access_key: Some(access_key),
            secret_key: Some(secret_key),
            region,
            ..Default::default()
        };
        BedrockProvider::new(config).await
    }

    /// Create AWS Bedrock provider for specific region
    pub async fn create_for_region(region: String) -> Result<BedrockProvider> {
        let config = BedrockConfig {
            region,
            ..Default::default()
        };
        BedrockProvider::new(config).await
    }

    /// Create test provider with mock configuration
    #[cfg(test)]
    pub async fn create_test() -> Result<BedrockProvider> {
        let config = BedrockConfig {
            access_key: Some("AKIAIOSFODNN7EXAMPLE".to_string()),
            secret_key: Some("wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY".to_string()),
            region: "us-east-1".to_string(),
            ..Default::default()
        };
        BedrockProvider::new(config).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_provider_creation() {
        let config = BedrockConfig {
            access_key: Some("AKIAIOSFODNN7EXAMPLE".to_string()),
            secret_key: Some("wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY".to_string()),
            region: "us-east-1".to_string(),
            ..Default::default()
        };

        let provider = BedrockProvider::new(config).await;
        assert!(provider.is_ok());
    }

    #[tokio::test]
    async fn test_provider_capabilities() {
        if let Ok(provider) = BedrockProviderFactory::create_test().await {
            let capabilities = provider.get_capabilities();
            
            assert_eq!(capabilities.provider, "aws_bedrock");
            assert!(capabilities.supports_chat);
            assert!(capabilities.supports_tools);
            assert!(capabilities.supports_streaming);
            assert!(capabilities.supports_embeddings);
        }
    }

    #[tokio::test]
    async fn test_factory_creation() {
        let result = BedrockProviderFactory::create_with_credentials(
            "AKIAIOSFODNN7EXAMPLE".to_string(),
            "wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY".to_string(),
            "us-east-1".to_string(),
        ).await;
        
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_supported_models() {
        if let Ok(provider) = BedrockProviderFactory::create_test().await {
            let models = provider.get_supported_models().await;
            assert!(models.is_ok());
            
            let models = models.unwrap();
            assert!(!models.is_empty());
            
            // Check for specific Bedrock models
            let model_names: Vec<String> = models.iter().map(|m| m.name.clone()).collect();
            assert!(model_names.contains(&"anthropic.claude-3-5-sonnet-20241022-v2:0".to_string()));
            assert!(model_names.contains(&"anthropic.claude-3-haiku-20240307-v1:0".to_string()));
            assert!(model_names.contains(&"meta.llama3-1-70b-instruct-v1:0".to_string()));
        }
    }

    #[tokio::test]
    async fn test_region_based_creation() {
        let result = BedrockProviderFactory::create_for_region("eu-west-1".to_string()).await;
        assert!(result.is_ok());
        
        if let Ok(provider) = result {
            assert_eq!(provider.config().region, "eu-west-1");
        }
    }
}