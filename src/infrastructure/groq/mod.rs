//! Groq provider implementation for LLM Supabase RS
//! 
//! This module provides comprehensive Groq integration with full tool calling support,
//! ultra-fast streaming capabilities, and seamless integration with the existing infrastructure.
//! 
//! # Features
//! 
//! - **Complete Groq API compatibility** - Full support for chat completions endpoint
//! - **Native tool calling** - Supports both sequential and parallel tool calling (model-dependent)
//! - **Ultra-fast streaming** - Server-Sent Events (SSE) parsing optimized for Groq's speed
//! - **Unified conversion** - Leverages existing tool calling infrastructure for consistency
//! - **Authentication** - API key-based authentication with Bearer token format
//! - **Error handling** - Comprehensive error handling with retries and rate limiting
//! - **Model support** - llama-3.1-70b-versatile, llama-3.1-8b-instant, mixtral-8x7b-32768, gemma2-9b-it
//! - **Performance optimized** - Tuned for Groq's ultra-fast inference speeds
//! 
//! # Usage
//! 
//! ```rust
//! use crate::infrastructure::groq::{GroqClient, GroqConfig};
//! 
//! // Create client from environment
//! let client = GroqClient::from_env().await?;
//! 
//! // Or with custom config
//! let config = GroqConfig {
//!     api_key: "gsk_...".to_string(),
//!     base_url: Some("https://api.groq.com/openai/v1".to_string()),
//!     model: "llama-3.1-70b-versatile".to_string(),
//!     ..Default::default()
//! };
//! let client = GroqClient::new(config).await?;
//! 
//! // Make a chat completion request
//! let request = ChatCompletionRequest {
//!     model: "llama-3.1-70b-versatile".to_string(),
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
pub use auth::{GroqAuth, RateLimitInfo};
pub use client::{GroqClient, PerformanceStats};
pub use converter::{GroqConverter, ModelCapabilities, ToolCallContext};
pub use streaming::{GroqStreamParser, GroqStreamUtils, StreamingMetrics, PerformanceTier};
pub use types::{GroqConfig, GroqModel, GroqSpeedTier, GroqRateLimit};

// Re-export commonly used types from models
pub use crate::models::request::ChatCompletionRequest;
pub use crate::models::response::{ChatCompletionResponse, ChatCompletionChunk};
pub use crate::models::common::{ToolDefinition, ChatMessage, MessageRole};

use anyhow::Result;
use std::sync::Arc;
use tracing::{info, debug, warn, error};

/// Groq provider implementation
/// 
/// This struct encapsulates the Groq provider and integrates it with the
/// existing infrastructure for provider fallback and health monitoring.
#[derive(Clone)]
pub struct GroqProvider {
    client: Arc<GroqClient>,
    converter: Arc<GroqConverter>,
    config: GroqConfig,
}

impl GroqProvider {
    /// Create a new Groq provider
    pub async fn new(config: GroqConfig) -> Result<Self> {
        info!("Initializing Groq provider with model: {}", config.model);

        let client = GroqClient::new(config.clone()).await?;
        let converter = GroqConverter::new();

        let provider = Self {
            client: Arc::new(client),
            converter: Arc::new(converter),
            config,
        };

        info!("Groq provider initialized successfully with API key: {}", 
              provider.client.auth().get_masked_api_key());
        Ok(provider)
    }

    /// Create Groq provider from environment variables
    pub async fn from_env() -> Result<Self> {
        let config = GroqConfig::from_env()?;
        Self::new(config).await
    }

    /// Get the Groq client
    pub fn client(&self) -> &GroqClient {
        &self.client
    }

    /// Get the converter
    pub fn converter(&self) -> &GroqConverter {
        &self.converter
    }

    /// Get the configuration
    pub fn config(&self) -> &GroqConfig {
        &self.config
    }

    /// Test provider health
    pub async fn health_check(&self) -> Result<ProviderHealth> {
        debug!("Performing Groq provider health check");

        match self.client.test_connection().await {
            Ok(true) => {
                info!("Groq provider health check passed");
                
                // Get performance stats for additional health info
                let stats = self.client.get_performance_stats();
                let latency_ms = if stats.total_requests > 0 {
                    Some(stats.average_response_time.as_millis() as u64)
                } else {
                    None
                };
                
                Ok(ProviderHealth {
                    provider: "groq".to_string(),
                    status: HealthStatus::Healthy,
                    latency_ms,
                    error: None,
                })
            }
            Ok(false) => {
                warn!("Groq provider health check failed");
                Ok(ProviderHealth {
                    provider: "groq".to_string(),
                    status: HealthStatus::Unhealthy,
                    latency_ms: None,
                    error: Some("Connection test failed".to_string()),
                })
            }
            Err(e) => {
                warn!("Groq provider health check error: {}", e);
                Ok(ProviderHealth {
                    provider: "groq".to_string(),
                    status: HealthStatus::Error,
                    latency_ms: None,
                    error: Some(e.to_string()),
                })
            }
        }
    }

    /// Get supported models
    pub async fn get_supported_models(&self) -> Result<Vec<SupportedModel>> {
        debug!("Getting supported Groq models");

        let available_models = match self.client.get_models().await {
            Ok(models) => models,
            Err(_) => {
                // Fallback to known models if API call fails
                warn!("Failed to fetch models from API, using known models");
                GroqModel::get_supported_models()
                    .into_iter()
                    .map(|model| model.name)
                    .collect()
            }
        };

        let supported_models = available_models
            .into_iter()
            .filter_map(|model_name| {
                GroqModel::get_model(&model_name).map(|model| {
                    let capabilities = self.converter.get_model_capabilities(&model.name);
                    SupportedModel {
                        name: model.name.clone(),
                        provider: "groq".to_string(),
                        supports_tools: capabilities.supports_tools,
                        supports_streaming: capabilities.supports_streaming,
                        max_tokens: model.max_tokens,
                        cost_per_1k_tokens: self.estimate_model_cost(&model.name),
                    }
                })
            })
            .collect();

        debug!("Found {} supported Groq models", supported_models.len());
        Ok(supported_models)
    }

    /// Estimate cost per 1K tokens for a model
    fn estimate_model_cost(&self, model: &str) -> f64 {
        // Groq pricing estimates (very competitive)
        match model {
            "llama-3.1-8b-instant" => 0.05,
            "llama-3.1-70b-versatile" => 0.27,
            "mixtral-8x7b-32768" => 0.24,
            "gemma2-9b-it" => 0.10,
            "llama3-70b-8192" => 0.27,
            "llama3-8b-8192" => 0.05,
            _ => 0.20, // Default estimate
        }
    }

    /// Get provider capabilities
    pub fn get_capabilities(&self) -> ProviderCapabilities {
        ProviderCapabilities {
            provider: "groq".to_string(),
            supports_chat: true,
            supports_completion: false, // Groq focuses on chat completions
            supports_tools: true,
            supports_streaming: true,
            supports_embeddings: false, // Not available via Groq
            max_context_length: 131072, // Max for Llama 3.1 models
            supported_formats: vec!["text".to_string(), "json".to_string()],
        }
    }

    /// Get performance metrics
    pub fn get_performance_metrics(&self) -> PerformanceStats {
        self.client.get_performance_stats()
    }

    /// Check if provider is performing well (Groq should be very fast)
    pub fn is_performing_well(&self) -> bool {
        let stats = self.get_performance_metrics();
        stats.is_performing_well()
    }

    /// Get recommended model for a use case
    pub fn get_recommended_model(&self, use_case: &str) -> Option<String> {
        self.client.get_recommended_model(use_case)
    }

    /// Make a chat completion request
    pub async fn chat_completion(&self, request: ChatCompletionRequest) -> Result<ChatCompletionResponse> {
        self.client.chat_completion(request).await
    }

    /// Make a streaming chat completion request
    pub async fn chat_completion_stream(
        &self, 
        request: ChatCompletionRequest
    ) -> Result<impl tokio_stream::Stream<Item = Result<ChatCompletionChunk>>> {
        self.client.chat_completion_stream(request).await
    }

    /// Check if client configuration is valid
    pub fn is_configured(&self) -> bool {
        self.client.is_configured()
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

/// Groq provider factory for easy initialization
pub struct GroqProviderFactory;

impl GroqProviderFactory {
    /// Create Groq provider with automatic configuration detection
    pub async fn create() -> Result<GroqProvider> {
        // Try environment variables first
        if let Ok(provider) = GroqProvider::from_env().await {
            return Ok(provider);
        }

        // Fallback to default configuration (will fail if API key not set)
        let config = GroqConfig::default();
        GroqProvider::new(config).await
    }

    /// Create Groq provider with custom configuration
    pub async fn create_with_config(config: GroqConfig) -> Result<GroqProvider> {
        GroqProvider::new(config).await
    }

    /// Create Groq provider with minimal configuration
    pub async fn create_with_api_key(api_key: String) -> Result<GroqProvider> {
        let config = GroqConfig {
            api_key,
            ..Default::default()
        };
        GroqProvider::new(config).await
    }

    /// Create Groq provider optimized for speed
    pub async fn create_ultra_fast() -> Result<GroqProvider> {
        let config = GroqConfig {
            model: "llama-3.1-8b-instant".to_string(), // Fastest model
            timeout: Some(30), // Shorter timeout for speed
            ..Default::default()
        };
        GroqProvider::new(config).await
    }

    /// Create Groq provider optimized for tool calling
    pub async fn create_for_tools() -> Result<GroqProvider> {
        let config = GroqConfig {
            model: "llama-3.1-70b-versatile".to_string(), // Best tool calling model
            timeout: Some(60),
            ..Default::default()
        };
        GroqProvider::new(config).await
    }

    /// Create test provider with mock configuration
    #[cfg(test)]
    pub async fn create_test() -> Result<GroqProvider> {
        let config = GroqConfig {
            api_key: "gsk_test123456789012345678901234567890123456789012345".to_string(),
            base_url: Some("https://api.groq.com/openai/v1".to_string()),
            model: "llama-3.1-70b-versatile".to_string(),
            ..Default::default()
        };
        GroqProvider::new(config).await
    }
}

/// Groq-specific utilities
pub struct GroqUtils;

impl GroqUtils {
    /// Check if a model is available and supports the required features
    pub fn validate_model_for_request(model: &str, requires_tools: bool) -> Result<()> {
        let model_info = GroqModel::get_model(model)
            .ok_or_else(|| anyhow::anyhow!("Unknown Groq model: {}", model))?;

        if requires_tools && !model_info.supports_tools {
            return Err(anyhow::anyhow!(
                "Model '{}' does not support tool calling", model
            ));
        }

        Ok(())
    }

    /// Get the best model for a specific use case
    pub fn get_best_model_for_use_case(use_case: &str) -> Option<GroqModel> {
        GroqModel::get_recommended_for_use_case(use_case)
    }

    /// Estimate response time based on model and request size
    pub fn estimate_response_time(model: &str, estimated_output_tokens: u32) -> std::time::Duration {
        if let Some(model_info) = GroqModel::get_model(model) {
            let base_time = match model_info.speed_tier {
                GroqSpeedTier::UltraFast => std::time::Duration::from_millis(100),
                GroqSpeedTier::VeryFast => std::time::Duration::from_millis(200),
                GroqSpeedTier::Fast => std::time::Duration::from_millis(500),
            };
            
            // Add time based on output length (Groq is very fast)
            let token_time = std::time::Duration::from_millis(estimated_output_tokens as u64 / 100);
            base_time + token_time
        } else {
            std::time::Duration::from_secs(1) // Default estimate
        }
    }

    /// Check if streaming should be used for a request
    pub fn should_use_streaming(request: &ChatCompletionRequest) -> bool {
        // Groq is so fast that streaming is beneficial for most requests
        let estimated_output = request.max_tokens.unwrap_or(1000);
        
        // Use streaming for responses > 50 tokens
        estimated_output > 50
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_provider_creation() {
        let config = GroqConfig {
            api_key: "gsk_test123456789012345678901234567890123456789012345".to_string(),
            ..Default::default()
        };

        let provider = GroqProvider::new(config).await;
        assert!(provider.is_ok());
        
        let provider = provider.unwrap();
        assert!(provider.is_configured());
    }

    #[tokio::test]
    async fn test_provider_capabilities() {
        if let Ok(provider) = GroqProviderFactory::create_test().await {
            let capabilities = provider.get_capabilities();
            
            assert_eq!(capabilities.provider, "groq");
            assert!(capabilities.supports_chat);
            assert!(capabilities.supports_tools);
            assert!(capabilities.supports_streaming);
            assert!(!capabilities.supports_completion);
            assert!(!capabilities.supports_embeddings);
        }
    }

    #[tokio::test]
    async fn test_factory_creation() {
        let result = GroqProviderFactory::create_with_api_key(
            "gsk_test123456789012345678901234567890123456789012345".to_string()
        ).await;
        
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_ultra_fast_factory() {
        let provider = GroqProviderFactory::create_ultra_fast().await;
        if provider.is_ok() {
            let provider = provider.unwrap();
            assert_eq!(provider.config().model, "llama-3.1-8b-instant");
        }
    }

    #[test]
    fn test_model_validation() {
        // Valid models
        assert!(GroqUtils::validate_model_for_request("llama-3.1-70b-versatile", true).is_ok());
        assert!(GroqUtils::validate_model_for_request("llama3-70b-8192", false).is_ok());
        
        // Invalid for tools
        assert!(GroqUtils::validate_model_for_request("llama3-70b-8192", true).is_err());
        
        // Unknown model
        assert!(GroqUtils::validate_model_for_request("unknown-model", false).is_err());
    }

    #[test]
    fn test_response_time_estimation() {
        let fast_time = GroqUtils::estimate_response_time("llama-3.1-8b-instant", 100);
        let slow_time = GroqUtils::estimate_response_time("mixtral-8x7b-32768", 100);
        
        assert!(fast_time < slow_time);
        assert!(fast_time < std::time::Duration::from_millis(500));
    }

    #[test]
    fn test_streaming_decision() {
        let short_request = ChatCompletionRequest {
            max_tokens: Some(10),
            ..Default::default()
        };
        assert!(!GroqUtils::should_use_streaming(&short_request));
        
        let long_request = ChatCompletionRequest {
            max_tokens: Some(500),
            ..Default::default()
        };
        assert!(GroqUtils::should_use_streaming(&long_request));
    }
}