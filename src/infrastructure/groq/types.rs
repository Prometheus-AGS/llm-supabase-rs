//! Groq types and configuration structures
//!
//! This module defines the Groq-specific types, configuration, and models.
//! Since Groq uses OpenAI-compatible API format, most structures are similar
//! but with Groq-specific endpoints, models, and capabilities.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::env;
use tracing::{debug, warn};

/// Groq configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GroqConfig {
    /// Groq API key
    pub api_key: String,

    /// Groq base URL (default: "https://api.groq.com/openai/v1")
    pub base_url: Option<String>,

    /// Default model to use (default: "llama-3.1-70b-versatile")
    pub model: String,

    /// Request timeout in seconds (default: 60)
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
    pub rate_limit: Option<GroqRateLimit>,
}

/// Groq rate limiting configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GroqRateLimit {
    /// Requests per minute limit (default: 30)
    pub requests_per_minute: u32,

    /// Tokens per minute limit (default: 14400)
    pub tokens_per_minute: u32,

    /// Whether to automatically throttle requests
    pub auto_throttle: bool,
}

/// Groq model information with capabilities
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GroqModel {
    /// Model identifier
    pub name: String,

    /// Model family (llama, mixtral, gemma, etc.)
    pub family: String,

    /// Maximum context length
    pub max_tokens: u32,

    /// Whether the model supports tool calling
    pub supports_tools: bool,

    /// Whether the model supports streaming
    pub supports_streaming: bool,

    /// Whether the model supports parallel tool calls
    pub supports_parallel_tools: bool,

    /// Model's strengths/use cases
    pub use_cases: Vec<String>,

    /// Expected inference speed relative to other models
    pub speed_tier: GroqSpeedTier,

    /// Whether this is a chat-optimized model
    pub is_chat_model: bool,
}

/// Groq model speed tiers (Groq's main advantage is speed)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum GroqSpeedTier {
    /// Ultra-fast inference (8B models)
    UltraFast,
    /// Very fast inference (most models)
    VeryFast,
    /// Fast inference (large models)
    Fast,
}

impl Default for GroqConfig {
    fn default() -> Self {
        Self {
            api_key: String::new(),
            base_url: Some("https://api.groq.com/openai/v1".to_string()),
            model: "llama-3.1-70b-versatile".to_string(),
            timeout: Some(60),  // Groq is fast, shorter timeout than other providers
            max_retries: Some(3),
            retry_delay_ms: Some(1000),
            exponential_backoff: Some(true),
            user_agent: Some("LLM-Supabase-RS/1.0 (Groq)".to_string()),
            rate_limit: Some(GroqRateLimit::default()),
        }
    }
}

impl Default for GroqRateLimit {
    fn default() -> Self {
        Self {
            requests_per_minute: 30,
            tokens_per_minute: 14400,
            auto_throttle: true,
        }
    }
}

impl GroqConfig {
    /// Create configuration from environment variables
    pub fn from_env() -> Result<Self> {
        debug!("Loading Groq configuration from environment variables");

        let api_key = env::var("GROQ_API_KEY")
            .context("GROQ_API_KEY environment variable is required")?;

        let base_url = env::var("GROQ_BASE_URL")
            .ok()
            .or_else(|| Some("https://api.groq.com/openai/v1".to_string()));

        let model = env::var("GROQ_MODEL")
            .unwrap_or_else(|_| "llama-3.1-70b-versatile".to_string());

        let timeout = env::var("GROQ_TIMEOUT")
            .ok()
            .and_then(|t| t.parse().ok());

        let max_retries = env::var("GROQ_MAX_RETRIES")
            .ok()
            .and_then(|r| r.parse().ok());

        let retry_delay_ms = env::var("GROQ_RETRY_DELAY_MS")
            .ok()
            .and_then(|d| d.parse().ok());

        let exponential_backoff = env::var("GROQ_EXPONENTIAL_BACKOFF")
            .ok()
            .and_then(|b| b.parse().ok());

        let user_agent = env::var("GROQ_USER_AGENT").ok();

        // Parse rate limit configuration if provided
        let rate_limit = if env::var("GROQ_RATE_LIMIT_RPM").is_ok() || env::var("GROQ_RATE_LIMIT_TPM").is_ok() {
            let requests_per_minute = env::var("GROQ_RATE_LIMIT_RPM")
                .ok()
                .and_then(|rpm| rpm.parse().ok())
                .unwrap_or(30);

            let tokens_per_minute = env::var("GROQ_RATE_LIMIT_TPM")
                .ok()
                .and_then(|tpm| tpm.parse().ok())
                .unwrap_or(14400);

            let auto_throttle = env::var("GROQ_AUTO_THROTTLE")
                .ok()
                .and_then(|at| at.parse().ok())
                .unwrap_or(true);

            Some(GroqRateLimit {
                requests_per_minute,
                tokens_per_minute,
                auto_throttle,
            })
        } else {
            Some(GroqRateLimit::default())
        };

        debug!("Successfully loaded Groq configuration");

        Ok(Self {
            api_key,
            base_url,
            model,
            timeout,
            max_retries,
            retry_delay_ms,
            exponential_backoff,
            user_agent,
            rate_limit,
        })
    }

    /// Get the chat completions URL
    pub fn get_chat_completions_url(&self) -> String {
        let default_base = "https://api.groq.com/openai/v1";
        let base = self.base_url.as_ref()
            .map(|s| s.as_str())
            .unwrap_or(default_base);

        format!("{}/chat/completions", base.trim_end_matches('/'))
    }

    /// Get the models list URL
    pub fn get_models_url(&self) -> String {
        let default_base = "https://api.groq.com/openai/v1";
        let base = self.base_url.as_ref()
            .map(|s| s.as_str())
            .unwrap_or(default_base);

        format!("{}/models", base.trim_end_matches('/'))
    }

    /// Validate the configuration
    pub fn validate(&self) -> Result<()> {
        if self.api_key.is_empty() {
            return Err(anyhow::anyhow!("API key cannot be empty"));
        }

        // Groq API keys have a specific format
        if !self.api_key.starts_with("gsk_") {
            return Err(anyhow::anyhow!("Groq API key must start with 'gsk_'"));
        }

        if self.api_key.len() < 40 {
            return Err(anyhow::anyhow!("Groq API key appears to be too short"));
        }

        if let Some(ref base_url) = self.base_url {
            if !base_url.starts_with("https://") {
                return Err(anyhow::anyhow!("Base URL must use HTTPS"));
            }
        }

        if self.model.is_empty() {
            return Err(anyhow::anyhow!("Model name cannot be empty"));
        }

        // Validate timeout ranges (Groq is fast, so shorter timeouts are reasonable)
        if let Some(timeout) = self.timeout {
            if timeout == 0 || timeout > 300 {
                return Err(anyhow::anyhow!("Timeout must be between 1 and 300 seconds"));
            }
        }

        // Validate retry configuration
        if let Some(max_retries) = self.max_retries {
            if max_retries > 10 {
                return Err(anyhow::anyhow!("Max retries cannot exceed 10"));
            }
        }

        if let Some(retry_delay) = self.retry_delay_ms {
            if retry_delay > 30000 {
                return Err(anyhow::anyhow!("Retry delay cannot exceed 30 seconds"));
            }
        }

        debug!("Groq configuration validation passed");
        Ok(())
    }

    /// Get masked API key for logging
    pub fn get_masked_api_key(&self) -> String {
        if self.api_key.len() > 8 {
            format!("gsk_{}...{}", 
                &self.api_key[4..8], 
                &self.api_key[self.api_key.len()-4..]
            )
        } else {
            "gsk_****".to_string()
        }
    }

    /// Get the effective base URL
    pub fn get_base_url(&self) -> String {
        self.base_url.as_ref()
            .map(|s| s.clone())
            .unwrap_or_else(|| "https://api.groq.com/openai/v1".to_string())
    }
}

impl GroqModel {
    /// Get all supported Groq models with their capabilities
    pub fn get_supported_models() -> Vec<Self> {
        vec![
            // Llama 3.1 models - most capable
            Self {
                name: "llama-3.1-70b-versatile".to_string(),
                family: "llama-3.1".to_string(),
                max_tokens: 131072,
                supports_tools: true,
                supports_streaming: true,
                supports_parallel_tools: true,
                use_cases: vec![
                    "complex reasoning".to_string(),
                    "tool calling".to_string(),
                    "code generation".to_string(),
                ],
                speed_tier: GroqSpeedTier::VeryFast,
                is_chat_model: true,
            },
            Self {
                name: "llama-3.1-8b-instant".to_string(),
                family: "llama-3.1".to_string(),
                max_tokens: 131072,
                supports_tools: true,
                supports_streaming: true,
                supports_parallel_tools: true,
                use_cases: vec![
                    "fast responses".to_string(),
                    "simple tool calling".to_string(),
                    "basic reasoning".to_string(),
                ],
                speed_tier: GroqSpeedTier::UltraFast,
                is_chat_model: true,
            },
            
            // Mixtral models - good for complex tasks
            Self {
                name: "mixtral-8x7b-32768".to_string(),
                family: "mixtral".to_string(),
                max_tokens: 32768,
                supports_tools: true,
                supports_streaming: true,
                supports_parallel_tools: true,
                use_cases: vec![
                    "complex reasoning".to_string(),
                    "multilingual".to_string(),
                    "tool calling".to_string(),
                ],
                speed_tier: GroqSpeedTier::Fast,
                is_chat_model: true,
            },
            
            // Gemma models - efficient and fast
            Self {
                name: "gemma2-9b-it".to_string(),
                family: "gemma2".to_string(),
                max_tokens: 8192,
                supports_tools: true,
                supports_streaming: true,
                supports_parallel_tools: false,
                use_cases: vec![
                    "efficient inference".to_string(),
                    "basic tool calling".to_string(),
                    "instruction following".to_string(),
                ],
                speed_tier: GroqSpeedTier::VeryFast,
                is_chat_model: true,
            },
            
            // Llama 3.0 models - still supported but older
            Self {
                name: "llama3-70b-8192".to_string(),
                family: "llama3".to_string(),
                max_tokens: 8192,
                supports_tools: false,
                supports_streaming: true,
                supports_parallel_tools: false,
                use_cases: vec![
                    "general conversation".to_string(),
                    "text generation".to_string(),
                ],
                speed_tier: GroqSpeedTier::VeryFast,
                is_chat_model: true,
            },
            Self {
                name: "llama3-8b-8192".to_string(),
                family: "llama3".to_string(),
                max_tokens: 8192,
                supports_tools: false,
                supports_streaming: true,
                supports_parallel_tools: false,
                use_cases: vec![
                    "fast responses".to_string(),
                    "simple conversations".to_string(),
                ],
                speed_tier: GroqSpeedTier::UltraFast,
                is_chat_model: true,
            },
        ]
    }

    /// Get model by name
    pub fn get_model(name: &str) -> Option<Self> {
        Self::get_supported_models()
            .into_iter()
            .find(|model| model.name == name)
    }

    /// Check if model supports tool calling
    pub fn supports_tool_calling(model_name: &str) -> bool {
        Self::get_model(model_name)
            .map(|model| model.supports_tools)
            .unwrap_or(false)
    }

    /// Get models that support tool calling
    pub fn get_tool_capable_models() -> Vec<Self> {
        Self::get_supported_models()
            .into_iter()
            .filter(|model| model.supports_tools)
            .collect()
    }

    /// Check if this model supports a specific feature
    pub fn supports_feature(&self, feature: &str) -> bool {
        match feature {
            "tools" | "function_calling" => self.supports_tools,
            "streaming" => self.supports_streaming,
            "parallel_tools" => self.supports_parallel_tools,
            "chat" => self.is_chat_model,
            _ => false,
        }
    }

    /// Get recommended model for a use case
    pub fn get_recommended_for_use_case(use_case: &str) -> Option<Self> {
        let models = Self::get_supported_models();
        
        match use_case {
            "tool_calling" | "tools" => {
                // Return most capable tool-calling model
                models.into_iter()
                    .filter(|m| m.supports_tools)
                    .max_by_key(|m| m.max_tokens)
            },
            "speed" | "fast" => {
                // Return fastest model
                models.into_iter()
                    .filter(|m| m.speed_tier == GroqSpeedTier::UltraFast)
                    .next()
            },
            "reasoning" | "complex" => {
                // Return most capable model
                models.into_iter()
                    .filter(|m| m.use_cases.contains(&"complex reasoning".to_string()))
                    .max_by_key(|m| m.max_tokens)
            },
            _ => {
                // Default to versatile model
                Self::get_model("llama-3.1-70b-versatile")
            }
        }
    }
}

/// Groq error response structure (OpenAI-compatible)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GroqError {
    pub error: GroqErrorDetails,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GroqErrorDetails {
    pub code: Option<String>,
    pub message: String,
    #[serde(rename = "type")]
    pub error_type: Option<String>,
    pub param: Option<String>,
    /// Groq-specific error context
    pub context: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_default() {
        let config = GroqConfig::default();
        assert_eq!(config.model, "llama-3.1-70b-versatile");
        assert_eq!(config.get_base_url(), "https://api.groq.com/openai/v1");
        assert_eq!(config.timeout, Some(60));
    }

    #[test]
    fn test_chat_completions_url() {
        let config = GroqConfig {
            base_url: Some("https://api.groq.com/openai/v1".to_string()),
            ..Default::default()
        };

        let url = config.get_chat_completions_url();
        assert_eq!(url, "https://api.groq.com/openai/v1/chat/completions");
    }

    #[test]
    fn test_config_validation() {
        let valid_config = GroqConfig {
            api_key: "gsk_1234567890123456789012345678901234567890".to_string(),
            ..Default::default()
        };

        assert!(valid_config.validate().is_ok());

        let invalid_config = GroqConfig {
            api_key: "sk_wrong_prefix".to_string(),
            ..Default::default()
        };

        assert!(invalid_config.validate().is_err());
    }

    #[test]
    fn test_masked_api_key() {
        let config = GroqConfig {
            api_key: "gsk_1234567890abcdef".to_string(),
            ..Default::default()
        };

        assert_eq!(config.get_masked_api_key(), "gsk_1234...cdef");
    }

    #[test]
    fn test_supported_models() {
        let models = GroqModel::get_supported_models();
        assert!(!models.is_empty());
        
        // Check that we have tool-capable models
        let tool_models = GroqModel::get_tool_capable_models();
        assert!(!tool_models.is_empty());
        
        // Check specific models
        assert!(GroqModel::supports_tool_calling("llama-3.1-70b-versatile"));
        assert!(GroqModel::supports_tool_calling("llama-3.1-8b-instant"));
        assert!(!GroqModel::supports_tool_calling("llama3-70b-8192"));
    }

    #[test]
    fn test_model_recommendations() {
        let tool_model = GroqModel::get_recommended_for_use_case("tool_calling");
        assert!(tool_model.is_some());
        assert!(tool_model.unwrap().supports_tools);

        let speed_model = GroqModel::get_recommended_for_use_case("speed");
        assert!(speed_model.is_some());
        assert_eq!(speed_model.unwrap().speed_tier, GroqSpeedTier::UltraFast);
    }

    #[test]
    fn test_model_features() {
        let model = GroqModel::get_model("llama-3.1-70b-versatile").unwrap();
        assert!(model.supports_feature("tools"));
        assert!(model.supports_feature("streaming"));
        assert!(model.supports_feature("parallel_tools"));
        assert!(model.supports_feature("chat"));
        assert!(!model.supports_feature("unknown_feature"));
    }
}