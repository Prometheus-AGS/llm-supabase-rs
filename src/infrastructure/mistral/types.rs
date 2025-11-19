//! Mistral AI types and configuration structures
//!
//! This module defines Mistral-specific types, configuration, and models.
//! Since Mistral uses OpenAI-compatible API format, most structures are similar
//! but with Mistral-specific endpoints, models, and European compliance features.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::env;
use tracing::{debug, warn};

/// Mistral AI configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MistralConfig {
    /// Mistral API key
    pub api_key: String,

    /// Mistral base URL (default: "https://api.mistral.ai/v1")
    pub base_url: Option<String>,

    /// Default model to use (default: "mistral-large-latest")
    pub model: String,

    /// Request timeout in seconds (default: 120)
    pub timeout: Option<u64>,

    /// Maximum retries for failed requests (default: 3)
    pub max_retries: Option<u32>,

    /// Base delay between retries in milliseconds (default: 1000)
    pub retry_delay_ms: Option<u64>,

    /// Whether to use exponential backoff (default: true)
    pub exponential_backoff: Option<bool>,

    /// Custom user agent string (optional)
    pub user_agent: Option<String>,

    /// Rate limiting configuration
    pub rate_limit: Option<MistralRateLimit>,

    /// European data residency preference (default: false)
    pub eu_residency: Option<bool>,

    /// GDPR compliance mode (default: true)
    pub gdpr_mode: Option<bool>,
}

/// Mistral rate limiting configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MistralRateLimit {
    /// Requests per minute limit (default: 60)
    pub requests_per_minute: u32,

    /// Tokens per minute limit (default: 200000)
    pub tokens_per_minute: u32,

    /// Whether to automatically throttle requests
    pub auto_throttle: bool,
}

/// Mistral model information with capabilities and European compliance features
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MistralModel {
    /// Model identifier
    pub name: String,

    /// Display name for the model
    pub display_name: String,

    /// Maximum context length
    pub context_length: u32,

    /// Whether this model supports tool calling
    pub supports_tools: bool,

    /// Whether this model supports streaming
    pub supports_streaming: bool,

    /// Whether this model supports function calling (legacy)
    pub supports_functions: bool,

    /// Cost per 1K input tokens (EUR)
    pub cost_per_1k_input_tokens: f64,

    /// Cost per 1K output tokens (EUR)
    pub cost_per_1k_output_tokens: f64,

    /// Whether this model is available in EU
    pub eu_available: bool,

    /// Whether this model supports code generation
    pub supports_code: bool,

    /// Model capabilities description
    pub description: String,

    /// Whether this model is deprecated
    pub deprecated: bool,
}

impl MistralConfig {
    /// Create configuration from environment variables
    pub fn from_env() -> Result<Self> {
        let api_key = env::var("MISTRAL_API_KEY")
            .context("MISTRAL_API_KEY environment variable not set")?;

        let base_url = env::var("MISTRAL_BASE_URL").ok();
        let model = env::var("MISTRAL_MODEL")
            .unwrap_or_else(|_| "mistral-large-latest".to_string());

        let timeout = env::var("MISTRAL_TIMEOUT")
            .ok()
            .and_then(|v| v.parse().ok());

        let max_retries = env::var("MISTRAL_MAX_RETRIES")
            .ok()
            .and_then(|v| v.parse().ok());

        let retry_delay_ms = env::var("MISTRAL_RETRY_DELAY_MS")
            .ok()
            .and_then(|v| v.parse().ok());

        let exponential_backoff = env::var("MISTRAL_EXPONENTIAL_BACKOFF")
            .ok()
            .and_then(|v| v.parse().ok());

        let eu_residency = env::var("MISTRAL_EU_RESIDENCY")
            .ok()
            .and_then(|v| v.parse().ok());

        let gdpr_mode = env::var("MISTRAL_GDPR_MODE")
            .ok()
            .and_then(|v| v.parse().ok());

        Ok(Self {
            api_key,
            base_url,
            model,
            timeout,
            max_retries,
            retry_delay_ms,
            exponential_backoff,
            user_agent: None,
            rate_limit: None,
            eu_residency,
            gdpr_mode,
        })
    }

    /// Validate the configuration
    pub fn validate(&self) -> Result<()> {
        if self.api_key.trim().is_empty() {
            return Err(anyhow::anyhow!("Mistral API key cannot be empty"));
        }

        // Validate API key format (should not be too short)
        if self.api_key.len() < 10 {
            return Err(anyhow::anyhow!("Invalid Mistral API key format"));
        }

        if let Some(timeout) = self.timeout {
            if timeout == 0 {
                return Err(anyhow::anyhow!("Timeout must be greater than 0"));
            }
        }

        if let Some(max_retries) = self.max_retries {
            if max_retries > 10 {
                warn!("High retry count configured: {}", max_retries);
            }
        }

        debug!("Mistral configuration validation passed");
        Ok(())
    }

    /// Get the base URL for Mistral API
    pub fn get_base_url(&self) -> String {
        self.base_url
            .clone()
            .unwrap_or_else(|| "https://api.mistral.ai/v1".to_string())
    }

    /// Get the chat completions endpoint
    pub fn get_chat_endpoint(&self) -> String {
        format!("{}/chat/completions", self.get_base_url())
    }

    /// Get the models endpoint
    pub fn get_models_endpoint(&self) -> String {
        format!("{}/models", self.get_base_url())
    }

    /// Get timeout duration
    pub fn get_timeout(&self) -> u64 {
        self.timeout.unwrap_or(120)
    }

    /// Get max retries
    pub fn get_max_retries(&self) -> u32 {
        self.max_retries.unwrap_or(3)
    }

    /// Get retry delay
    pub fn get_retry_delay_ms(&self) -> u64 {
        self.retry_delay_ms.unwrap_or(1000)
    }

    /// Check if exponential backoff is enabled
    pub fn uses_exponential_backoff(&self) -> bool {
        self.exponential_backoff.unwrap_or(true)
    }

    /// Check if EU residency is preferred
    pub fn prefers_eu_residency(&self) -> bool {
        self.eu_residency.unwrap_or(false)
    }

    /// Check if GDPR mode is enabled
    pub fn uses_gdpr_mode(&self) -> bool {
        self.gdpr_mode.unwrap_or(true)
    }
}

impl Default for MistralConfig {
    fn default() -> Self {
        Self {
            api_key: String::new(),
            base_url: Some("https://api.mistral.ai/v1".to_string()),
            model: "mistral-large-latest".to_string(),
            timeout: Some(120),
            max_retries: Some(3),
            retry_delay_ms: Some(1000),
            exponential_backoff: Some(true),
            user_agent: Some("LLM-Supabase-RS/1.0 (Mistral)".to_string()),
            rate_limit: Some(MistralRateLimit::default()),
            eu_residency: Some(false),
            gdpr_mode: Some(true),
        }
    }
}

impl Default for MistralRateLimit {
    fn default() -> Self {
        Self {
            requests_per_minute: 60,
            tokens_per_minute: 200000,
            auto_throttle: true,
        }
    }
}

impl MistralModel {
    /// Get all available Mistral models with their capabilities
    pub fn get_all_models() -> Vec<Self> {
        vec![
            Self {
                name: "mistral-large-latest".to_string(),
                display_name: "Mistral Large (Latest)".to_string(),
                context_length: 128000,
                supports_tools: true,
                supports_streaming: true,
                supports_functions: true,
                cost_per_1k_input_tokens: 3.0,
                cost_per_1k_output_tokens: 9.0,
                eu_available: true,
                supports_code: true,
                description: "Most capable Mistral model with advanced reasoning and tool calling".to_string(),
                deprecated: false,
            },
            Self {
                name: "mistral-medium-latest".to_string(),
                display_name: "Mistral Medium (Latest)".to_string(),
                context_length: 32000,
                supports_tools: true,
                supports_streaming: true,
                supports_functions: true,
                cost_per_1k_input_tokens: 2.5,
                cost_per_1k_output_tokens: 7.5,
                eu_available: true,
                supports_code: true,
                description: "Balanced performance and cost for most applications".to_string(),
                deprecated: false,
            },
            Self {
                name: "mistral-small-latest".to_string(),
                display_name: "Mistral Small (Latest)".to_string(),
                context_length: 32000,
                supports_tools: true,
                supports_streaming: true,
                supports_functions: true,
                cost_per_1k_input_tokens: 1.0,
                cost_per_1k_output_tokens: 3.0,
                eu_available: true,
                supports_code: false,
                description: "Fast and cost-effective for simple tasks".to_string(),
                deprecated: false,
            },
            Self {
                name: "codestral-latest".to_string(),
                display_name: "Codestral (Latest)".to_string(),
                context_length: 32000,
                supports_tools: true,
                supports_streaming: true,
                supports_functions: true,
                cost_per_1k_input_tokens: 1.0,
                cost_per_1k_output_tokens: 3.0,
                eu_available: true,
                supports_code: true,
                description: "Specialized for code generation and programming tasks".to_string(),
                deprecated: false,
            },
            Self {
                name: "mistral-tiny".to_string(),
                display_name: "Mistral Tiny".to_string(),
                context_length: 32000,
                supports_tools: false, // Tiny model may not support tool calling
                supports_streaming: true,
                supports_functions: false,
                cost_per_1k_input_tokens: 0.25,
                cost_per_1k_output_tokens: 0.75,
                eu_available: true,
                supports_code: false,
                description: "Ultra-fast and cost-effective for basic text tasks".to_string(),
                deprecated: false,
            },
        ]
    }

    /// Check if a model supports tool calling
    pub fn supports_tool_calling(model_name: &str) -> bool {
        Self::get_all_models()
            .iter()
            .find(|m| m.name == model_name)
            .map(|m| m.supports_tools)
            .unwrap_or(false)
    }

    /// Get model by name
    pub fn get_by_name(name: &str) -> Option<Self> {
        Self::get_all_models()
            .into_iter()
            .find(|m| m.name == name)
    }

    /// Get models that support tool calling
    pub fn get_tool_capable_models() -> Vec<Self> {
        Self::get_all_models()
            .into_iter()
            .filter(|m| m.supports_tools)
            .collect()
    }

    /// Get models available in EU
    pub fn get_eu_models() -> Vec<Self> {
        Self::get_all_models()
            .into_iter()
            .filter(|m| m.eu_available)
            .collect()
    }

    /// Get models that support code generation
    pub fn get_code_models() -> Vec<Self> {
        Self::get_all_models()
            .into_iter()
            .filter(|m| m.supports_code)
            .collect()
    }
}

/// Mistral API response wrapper for better error handling
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MistralApiResponse<T> {
    pub data: Option<T>,
    pub error: Option<MistralApiError>,
}

/// Mistral API error structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MistralApiError {
    pub message: String,
    pub type_: Option<String>,
    pub param: Option<String>,
    pub code: Option<String>,
}

/// Mistral model list response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MistralModelsResponse {
    pub object: String,
    pub data: Vec<MistralModelInfo>,
}

/// Mistral model information from API
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MistralModelInfo {
    pub id: String,
    pub object: String,
    pub created: Option<u64>,
    pub owned_by: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_validation() {
        let mut config = MistralConfig::default();
        config.api_key = "valid_api_key_123".to_string();
        
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_invalid_config_validation() {
        let config = MistralConfig {
            api_key: String::new(),
            ..Default::default()
        };
        
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_model_capabilities() {
        let large_model = MistralModel::get_by_name("mistral-large-latest").unwrap();
        assert!(large_model.supports_tools);
        assert!(large_model.supports_streaming);
        assert!(large_model.eu_available);
    }

    #[test]
    fn test_tool_capable_models() {
        let tool_models = MistralModel::get_tool_capable_models();
        assert!(!tool_models.is_empty());
        
        for model in tool_models {
            assert!(model.supports_tools);
        }
    }

    #[test]
    fn test_eu_models() {
        let eu_models = MistralModel::get_eu_models();
        assert!(!eu_models.is_empty());
        
        for model in eu_models {
            assert!(model.eu_available);
        }
    }

    #[test]
    fn test_endpoints() {
        let config = MistralConfig::default();
        assert_eq!(config.get_chat_endpoint(), "https://api.mistral.ai/v1/chat/completions");
        assert_eq!(config.get_models_endpoint(), "https://api.mistral.ai/v1/models");
    }

    #[test]
    fn test_config_from_env_missing_key() {
        // This will fail because MISTRAL_API_KEY is not set in test environment
        let result = MistralConfig::from_env();
        assert!(result.is_err());
    }
}