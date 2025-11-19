//! Mistral AI provider implementation for LLM Supabase RS
//! 
//! This module provides comprehensive Mistral AI integration with full tool calling support,
//! streaming capabilities, European compliance features, and seamless integration with the 
//! existing infrastructure.
//! 
//! # Features
//! 
//! - **Complete Mistral API compatibility** - Full support for chat completions endpoint
//! - **OpenAI-compatible format** - Uses OpenAI-compatible API format for seamless integration
//! - **Native tool calling** - Supports modern parallel tool calling with OpenAI format
//! - **Streaming support** - Server-Sent Events (SSE) parsing for real-time responses
//! - **European compliance** - Built-in GDPR compliance and EU data residency options
//! - **Unified conversion** - Leverages existing tool calling infrastructure for consistency
//! - **Authentication** - API key-based authentication with Bearer token
//! - **Error handling** - Comprehensive error handling with retries and rate limiting
//! - **Model support** - mistral-large-latest, mistral-medium-latest, mistral-small-latest, codestral-latest
//! - **Cost optimization** - European pricing and efficient token usage
//! 
//! # Usage
//! 
//! ```rust
//! use crate::infrastructure::mistral::{MistralClient, MistralConfig};
//! 
//! // Create client from environment
//! let client = MistralClient::from_env().await?;
//! 
//! // Or with custom config
//! let config = MistralConfig {
//!     api_key: "your-mistral-api-key".to_string(),
//!     base_url: Some("https://api.mistral.ai/v1".to_string()),
//!     eu_residency: Some(true),
//!     gdpr_mode: Some(true),
//!     ..Default::default()
//! };
//! let client = MistralClient::new(config).await?;
//! 
//! // Make a chat completion request
//! let request = ChatCompletionRequest {
//!     model: "mistral-large-latest".to_string(),
//!     messages: vec![/* messages */],
//!     tools: Some(vec![/* tool definitions */]),
//!     ..Default::default()
//! };
//! 
//! let response = client.chat_completion(request).await?;
//! ```
//! 
//! # European Compliance
//! 
//! Mistral AI is a European AI company that provides:
//! - GDPR compliance by design
//! - European data residency options
//! - Transparent AI development
//! - Competitive European pricing
//! 
//! Enable compliance features in configuration:
//! ```rust
//! let config = MistralConfig {
//!     eu_residency: Some(true),  // Prefer EU data centers
//!     gdpr_mode: Some(true),     // Enable GDPR compliance mode
//!     ..Default::default()
//! };
//! ```

pub mod auth;
pub mod client;
pub mod converter;
pub mod streaming;
pub mod types;

// Re-export main types and client for convenience
pub use auth::{MistralAuth, RateLimitInfo, ComplianceStats, RateLimitStatus};
pub use client::{MistralClient, ClientPerformanceMetrics};
pub use converter::{MistralConverter, ModelCapabilities, ToolCallContext, MistralToolUtils};
pub use streaming::{
    MistralStreamParser, MistralStreamUtils, MistralStreamChunk, StreamingMetrics,
    StreamingPerformanceReport
};
pub use types::{MistralConfig, MistralModel, MistralRateLimit, MistralApiError};

// Re-export commonly used types from models
pub use crate::models::request::ChatCompletionRequest;
pub use crate::models::response::{ChatCompletionResponse, ChatCompletionChunk};
pub use crate::models::common::{ToolDefinition, ChatMessage, MessageRole};

use anyhow::Result;
use std::sync::Arc;
use tracing::{info, debug, warn};

/// Mistral provider implementation
/// 
/// This struct encapsulates the Mistral provider and integrates it with the
/// existing infrastructure for provider fallback and health monitoring.
/// Includes European compliance features and cost optimization.
#[derive(Clone)]
pub struct MistralProvider {
    client: Arc<MistralClient>,
    converter: Arc<MistralConverter>,
    config: MistralConfig,
}

impl MistralProvider {
    /// Create a new Mistral provider
    pub async fn new(config: MistralConfig) -> Result<Self> {
        info!("Initializing Mistral provider with European compliance features");

        let client = MistralClient::new(config.clone()).await?;
        let converter = MistralConverter::with_config(config.clone());

        let provider = Self {
            client: Arc::new(client),
            converter: Arc::new(converter),
            config,
        };

        info!("Mistral provider initialized successfully (EU residency: {}, GDPR: {})",
              provider.config.prefers_eu_residency(),
              provider.config.uses_gdpr_mode());
        Ok(provider)
    }

    /// Create Mistral provider from environment variables
    pub async fn from_env() -> Result<Self> {
        let config = MistralConfig::from_env()?;
        Self::new(config).await
    }

    /// Get the Mistral client
    pub fn client(&self) -> &MistralClient {
        &self.client
    }

    /// Get the converter
    pub fn converter(&self) -> &MistralConverter {
        &self.converter
    }

    /// Get the configuration
    pub fn config(&self) -> &MistralConfig {
        &self.config
    }

    /// Test provider health
    pub async fn health_check(&self) -> Result<ProviderHealth> {
        debug!("Performing Mistral provider health check");

        match self.client.test_connection().await {
            Ok(true) => {
                info!("Mistral provider health check passed");
                Ok(ProviderHealth {
                    provider: "mistral".to_string(),
                    status: HealthStatus::Healthy,
                    latency_ms: None, // Could be enhanced to measure actual latency
                    error: None,
                    compliance: Some(ComplianceStatus {
                        eu_residency: self.config.prefers_eu_residency(),
                        gdpr_compliant: self.config.uses_gdpr_mode(),
                        region: if self.config.prefers_eu_residency() { 
                            Some("EU".to_string()) 
                        } else { 
                            None 
                        },
                    }),
                })
            }
            Ok(false) => {
                warn!("Mistral provider health check failed");
                Ok(ProviderHealth {
                    provider: "mistral".to_string(),
                    status: HealthStatus::Unhealthy,
                    latency_ms: None,
                    error: Some("Connection test failed".to_string()),
                    compliance: None,
                })
            }
            Err(e) => {
                warn!("Mistral provider health check error: {}", e);
                Ok(ProviderHealth {
                    provider: "mistral".to_string(),
                    status: HealthStatus::Error,
                    latency_ms: None,
                    error: Some(e.to_string()),
                    compliance: None,
                })
            }
        }
    }

    /// Get supported models
    pub async fn get_supported_models(&self) -> Result<Vec<SupportedModel>> {
        debug!("Getting supported Mistral models");

        let available_models = match self.client.get_models().await {
            Ok(models) => models,
            Err(_) => {
                // Fallback to known models if API call fails
                warn!("Failed to fetch models from API, using known models");
                MistralModel::get_all_models()
                    .into_iter()
                    .map(|m| m.name)
                    .collect()
            }
        };

        let supported_models: Vec<_> = available_models
            .into_iter()
            .filter_map(|model_name| {
                if let Some(model_info) = MistralModel::get_by_name(&model_name) {
                    let capabilities = self.converter.get_model_capabilities(&model_name);
                    Some(SupportedModel {
                        name: model_name.clone(),
                        provider: "mistral".to_string(),
                        supports_tools: capabilities.supports_tools,
                        supports_streaming: capabilities.supports_streaming,
                        max_tokens: model_info.context_length,
                        cost_per_1k_input_tokens: model_info.cost_per_1k_input_tokens,
                        cost_per_1k_output_tokens: model_info.cost_per_1k_output_tokens,
                        eu_available: model_info.eu_available,
                        code_optimized: model_info.supports_code,
                        description: Some(model_info.description),
                    })
                } else {
                    None
                }
            })
            .collect();

        debug!("Found {} supported Mistral models", supported_models.len());
        Ok(supported_models)
    }

    /// Get provider capabilities
    pub fn get_capabilities(&self) -> ProviderCapabilities {
        ProviderCapabilities {
            provider: "mistral".to_string(),
            supports_chat: true,
            supports_completion: false, // Mistral focuses on chat completions
            supports_tools: true,
            supports_streaming: true,
            supports_embeddings: false, // Not implemented in this provider
            max_context_length: 128000, // Mistral Large context length
            supported_formats: vec!["text".to_string(), "json".to_string()],
            european_compliance: Some(EuropeanCompliance {
                gdpr_compliant: self.config.uses_gdpr_mode(),
                eu_data_residency: self.config.prefers_eu_residency(),
                european_company: true, // Mistral is European
                data_processing_regions: vec!["EU".to_string(), "France".to_string()],
            }),
            cost_optimization: Some(CostOptimization {
                competitive_pricing: true,
                european_pricing: true,
                cost_per_1k_tokens_range: (0.25, 9.0), // Range from Mistral Tiny to Large
                supports_cost_effective_models: true,
            }),
        }
    }

    /// Get European compliance statistics
    pub async fn get_compliance_stats(&self) -> Result<ComplianceStats> {
        self.client.get_compliance_stats().await
    }

    /// Get performance metrics
    pub fn get_performance_metrics(&self) -> ClientPerformanceMetrics {
        self.client.get_performance_metrics()
    }

    /// Get rate limit status
    pub async fn get_rate_limit_status(&self) -> Option<RateLimitStatus> {
        self.client.get_rate_limit_status().await
    }
}

/// Provider health status with European compliance information
#[derive(Debug, Clone)]
pub struct ProviderHealth {
    pub provider: String,
    pub status: HealthStatus,
    pub latency_ms: Option<u64>,
    pub error: Option<String>,
    pub compliance: Option<ComplianceStatus>,
}

/// European compliance status
#[derive(Debug, Clone)]
pub struct ComplianceStatus {
    pub eu_residency: bool,
    pub gdpr_compliant: bool,
    pub region: Option<String>,
}

/// Health status enumeration
#[derive(Debug, Clone, PartialEq)]
pub enum HealthStatus {
    Healthy,
    Unhealthy,
    Error,
}

/// Supported model information with European and cost features
#[derive(Debug, Clone)]
pub struct SupportedModel {
    pub name: String,
    pub provider: String,
    pub supports_tools: bool,
    pub supports_streaming: bool,
    pub max_tokens: u32,
    pub cost_per_1k_input_tokens: f64,
    pub cost_per_1k_output_tokens: f64,
    pub eu_available: bool,
    pub code_optimized: bool,
    pub description: Option<String>,
}

/// Provider capabilities with European compliance features
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
    pub european_compliance: Option<EuropeanCompliance>,
    pub cost_optimization: Option<CostOptimization>,
}

/// European compliance features
#[derive(Debug, Clone)]
pub struct EuropeanCompliance {
    pub gdpr_compliant: bool,
    pub eu_data_residency: bool,
    pub european_company: bool,
    pub data_processing_regions: Vec<String>,
}

/// Cost optimization features
#[derive(Debug, Clone)]
pub struct CostOptimization {
    pub competitive_pricing: bool,
    pub european_pricing: bool,
    pub cost_per_1k_tokens_range: (f64, f64), // (min, max)
    pub supports_cost_effective_models: bool,
}

/// Mistral provider factory for easy initialization
pub struct MistralProviderFactory;

impl MistralProviderFactory {
    /// Create Mistral provider with automatic configuration detection
    pub async fn create() -> Result<MistralProvider> {
        // Try environment variables first
        if let Ok(provider) = MistralProvider::from_env().await {
            return Ok(provider);
        }

        // Fallback to default configuration (will fail if API key not set)
        let config = MistralConfig::default();
        MistralProvider::new(config).await
    }

    /// Create Mistral provider with custom configuration
    pub async fn create_with_config(config: MistralConfig) -> Result<MistralProvider> {
        MistralProvider::new(config).await
    }

    /// Create Mistral provider with minimal configuration
    pub async fn create_with_api_key(api_key: String) -> Result<MistralProvider> {
        let config = MistralConfig {
            api_key,
            ..Default::default()
        };
        MistralProvider::new(config).await
    }

    /// Create European-compliant Mistral provider
    pub async fn create_eu_compliant(api_key: String) -> Result<MistralProvider> {
        let config = MistralConfig {
            api_key,
            eu_residency: Some(true),
            gdpr_mode: Some(true),
            ..Default::default()
        };
        MistralProvider::new(config).await
    }

    /// Create cost-optimized Mistral provider
    pub async fn create_cost_optimized(api_key: String) -> Result<MistralProvider> {
        let config = MistralConfig {
            api_key,
            model: "mistral-small-latest".to_string(), // Most cost-effective
            ..Default::default()
        };
        MistralProvider::new(config).await
    }

    /// Create code-optimized Mistral provider
    pub async fn create_code_optimized(api_key: String) -> Result<MistralProvider> {
        let config = MistralConfig {
            api_key,
            model: "codestral-latest".to_string(), // Code-specialized model
            ..Default::default()
        };
        MistralProvider::new(config).await
    }

    /// Create test provider with mock configuration
    #[cfg(test)]
    pub async fn create_test() -> Result<MistralProvider> {
        let config = MistralConfig {
            api_key: "test-mistral-key-123456789012345678901234567890".to_string(),
            base_url: Some("https://api.mistral.ai/v1".to_string()),
            ..Default::default()
        };
        MistralProvider::new(config).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_provider_creation() {
        let config = MistralConfig {
            api_key: "test-mistral-key-123456789012345678901234567890".to_string(),
            ..Default::default()
        };

        let provider = MistralProvider::new(config).await;
        assert!(provider.is_ok());
    }

    #[tokio::test]
    async fn test_provider_capabilities() {
        if let Ok(provider) = MistralProviderFactory::create_test().await {
            let capabilities = provider.get_capabilities();
            
            assert_eq!(capabilities.provider, "mistral");
            assert!(capabilities.supports_chat);
            assert!(capabilities.supports_tools);
            assert!(capabilities.supports_streaming);
            assert!(!capabilities.supports_completion);
            assert!(capabilities.european_compliance.is_some());
            assert!(capabilities.cost_optimization.is_some());
        }
    }

    #[tokio::test]
    async fn test_factory_creation() {
        let result = MistralProviderFactory::create_with_api_key(
            "test-mistral-key-123456789012345678901234567890".to_string()
        ).await;
        
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_eu_compliant_creation() {
        let result = MistralProviderFactory::create_eu_compliant(
            "test-mistral-key-123456789012345678901234567890".to_string()
        ).await;
        
        assert!(result.is_ok());
        
        if let Ok(provider) = result {
            assert!(provider.config().prefers_eu_residency());
            assert!(provider.config().uses_gdpr_mode());
        }
    }

    #[tokio::test]
    async fn test_cost_optimized_creation() {
        let result = MistralProviderFactory::create_cost_optimized(
            "test-mistral-key-123456789012345678901234567890".to_string()
        ).await;
        
        assert!(result.is_ok());
        
        if let Ok(provider) = result {
            assert_eq!(provider.config().model, "mistral-small-latest");
        }
    }

    #[tokio::test]
    async fn test_code_optimized_creation() {
        let result = MistralProviderFactory::create_code_optimized(
            "test-mistral-key-123456789012345678901234567890".to_string()
        ).await;
        
        assert!(result.is_ok());
        
        if let Ok(provider) = result {
            assert_eq!(provider.config().model, "codestral-latest");
        }
    }

    #[test]
    fn test_supported_model_features() {
        let model = SupportedModel {
            name: "mistral-large-latest".to_string(),
            provider: "mistral".to_string(),
            supports_tools: true,
            supports_streaming: true,
            max_tokens: 128000,
            cost_per_1k_input_tokens: 3.0,
            cost_per_1k_output_tokens: 9.0,
            eu_available: true,
            code_optimized: false,
            description: Some("Most capable Mistral model".to_string()),
        };

        assert!(model.supports_tools);
        assert!(model.eu_available);
        assert_eq!(model.provider, "mistral");
    }

    #[test]
    fn test_european_compliance_features() {
        let compliance = EuropeanCompliance {
            gdpr_compliant: true,
            eu_data_residency: true,
            european_company: true,
            data_processing_regions: vec!["EU".to_string(), "France".to_string()],
        };

        assert!(compliance.gdpr_compliant);
        assert!(compliance.eu_data_residency);
        assert!(compliance.european_company);
    }
}