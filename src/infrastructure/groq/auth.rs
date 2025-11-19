//! Groq authentication module
//!
//! This module handles Groq-specific authentication using API keys with the
//! Authorization Bearer header format (OpenAI-compatible).

use anyhow::{Context, Result};
use reqwest::header::{HeaderMap, HeaderName, HeaderValue, AUTHORIZATION, CONTENT_TYPE, USER_AGENT};
use std::time::Duration;
use tracing::{debug, trace, warn};

use super::types::GroqConfig;

/// Groq authentication handler
#[derive(Debug, Clone)]
pub struct GroqAuth {
    /// API key for authentication
    api_key: String,
    
    /// User agent string
    user_agent: String,
    
    /// Base URL for API requests
    base_url: String,
}

impl GroqAuth {
    /// Create a new Groq authentication handler
    pub fn new(config: &GroqConfig) -> Result<Self> {
        debug!("Initializing Groq authentication");
        
        if config.api_key.is_empty() {
            return Err(anyhow::anyhow!("Groq API key cannot be empty"));
        }
        
        let user_agent = config.user_agent
            .clone()
            .unwrap_or_else(|| "LLM-Supabase-RS/1.0 (Groq)".to_string());
        
        let base_url = config.get_base_url();
        
        let auth = Self {
            api_key: config.api_key.clone(),
            user_agent,
            base_url,
        };
        
        debug!("Groq authentication initialized with API key: {}", 
               auth.get_masked_api_key());
        
        Ok(auth)
    }
    
    /// Validate the API key format
    pub fn validate_api_key(&self) -> Result<()> {
        debug!("Validating Groq API key format");
        
        if self.api_key.is_empty() {
            return Err(anyhow::anyhow!("API key cannot be empty"));
        }
        
        // Groq API keys should start with 'gsk_'
        if !self.api_key.starts_with("gsk_") {
            return Err(anyhow::anyhow!(
                "Groq API key must start with 'gsk_' prefix"
            ));
        }
        
        if self.api_key.len() < 40 {
            return Err(anyhow::anyhow!(
                "API key appears to be too short (expected at least 40 characters, got {})",
                self.api_key.len()
            ));
        }
        
        // Groq API keys are typically alphanumeric with underscores
        let valid_chars = self.api_key.chars().all(|c| c.is_alphanumeric() || c == '_');
        if !valid_chars {
            return Err(anyhow::anyhow!(
                "API key contains invalid characters (only alphanumeric and underscore allowed)"
            ));
        }
        
        debug!("API key validation passed");
        Ok(())
    }
    
    /// Create HTTP headers for Groq requests
    pub fn create_headers(&self) -> Result<HeaderMap> {
        let mut headers = HeaderMap::new();
        
        // Groq uses Authorization Bearer header (OpenAI-compatible)
        let auth_value = format!("Bearer {}", self.api_key);
        headers.insert(
            AUTHORIZATION,
            HeaderValue::from_str(&auth_value)
                .context("Failed to create Authorization header value")?
        );
        
        // Set content type
        headers.insert(
            CONTENT_TYPE,
            HeaderValue::from_static("application/json")
        );
        
        // Set user agent
        headers.insert(
            USER_AGENT,
            HeaderValue::from_str(&self.user_agent)
                .context("Failed to create user-agent header value")?
        );
        
        trace!("Created Groq request headers (API key masked)");
        Ok(headers)
    }
    
    /// Create headers for streaming requests
    pub fn create_streaming_headers(&self) -> Result<HeaderMap> {
        let mut headers = self.create_headers()?;
        
        // Add streaming-specific headers
        headers.insert(
            HeaderName::from_static("accept"),
            HeaderValue::from_static("text/event-stream")
        );
        
        headers.insert(
            HeaderName::from_static("cache-control"),
            HeaderValue::from_static("no-cache")
        );
        
        trace!("Created Groq streaming headers");
        Ok(headers)
    }
    
    /// Get masked API key for logging purposes
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
    
    /// Test the authentication by making a simple request
    pub async fn test_authentication(&self) -> Result<bool> {
        debug!("Testing Groq authentication");
        
        let client = reqwest::Client::new();
        let test_url = format!("{}/models", self.base_url.trim_end_matches('/'));
        
        let headers = self.create_headers()?;
        
        let response = client
            .get(&test_url)
            .headers(headers)
            .timeout(Duration::from_secs(10))
            .send()
            .await
            .context("Failed to send authentication test request")?;
        
        let success = response.status().is_success();
        
        if success {
            debug!("Groq authentication test successful");
        } else {
            let status = response.status();
            let error_text = response.text().await
                .unwrap_or_else(|_| "Unknown error".to_string());
            warn!("Authentication test failed with status {}: {}", status, error_text);
        }
        
        Ok(success)
    }
    
    /// Get the authentication method being used
    pub fn get_auth_method(&self) -> &'static str {
        "Bearer Token"
    }
    
    /// Get the API key (for internal use only)
    pub(crate) fn get_api_key(&self) -> &str {
        &self.api_key
    }
    
    /// Check if the authentication is properly configured
    pub fn is_configured(&self) -> bool {
        !self.api_key.is_empty() && self.api_key.starts_with("gsk_")
    }
    
    /// Create headers with custom timeout (for long-running requests)
    pub fn create_headers_with_timeout(&self, timeout_seconds: u64) -> Result<HeaderMap> {
        let mut headers = self.create_headers()?;
        
        // Add timeout hint (not standard but some proxies use it)
        if timeout_seconds > 0 {
            headers.insert(
                HeaderName::from_static("x-timeout"),
                HeaderValue::from_str(&timeout_seconds.to_string())
                    .context("Failed to create timeout header value")?
            );
        }
        
        Ok(headers)
    }
    
    /// Validate authentication configuration
    pub fn validate_config(&self) -> Result<()> {
        self.validate_api_key()?;
        
        if self.base_url.is_empty() {
            return Err(anyhow::anyhow!("Base URL cannot be empty"));
        }
        
        if !self.base_url.starts_with("https://") {
            return Err(anyhow::anyhow!("Base URL must use HTTPS"));
        }
        
        if self.user_agent.is_empty() {
            warn!("User agent is empty, this may cause issues with some API endpoints");
        }
        
        debug!("Groq authentication configuration is valid");
        Ok(())
    }
    
    /// Create headers for a specific model request (some models may have different requirements)
    pub fn create_model_headers(&self, model: &str) -> Result<HeaderMap> {
        let mut headers = self.create_headers()?;
        
        // Add model-specific headers if needed
        headers.insert(
            HeaderName::from_static("x-groq-model"),
            HeaderValue::from_str(model)
                .context("Failed to create model header value")?
        );
        
        trace!("Created model-specific headers for: {}", model);
        Ok(headers)
    }
    
    /// Get rate limit information from headers (if available)
    pub fn extract_rate_limit_info(&self, headers: &HeaderMap) -> Option<RateLimitInfo> {
        let remaining = headers.get("x-ratelimit-remaining")?
            .to_str().ok()?
            .parse().ok()?;
        
        let limit = headers.get("x-ratelimit-limit")?
            .to_str().ok()?
            .parse().ok()?;
        
        let reset = headers.get("x-ratelimit-reset")?
            .to_str().ok()?
            .parse().ok()?;
        
        Some(RateLimitInfo {
            remaining,
            limit,
            reset_timestamp: reset,
        })
    }
}

/// Rate limit information from API response headers
#[derive(Debug, Clone)]
pub struct RateLimitInfo {
    /// Remaining requests in current window
    pub remaining: u32,
    
    /// Total limit for current window
    pub limit: u32,
    
    /// Unix timestamp when the limit resets
    pub reset_timestamp: u64,
}

impl RateLimitInfo {
    /// Check if we're approaching the rate limit
    pub fn is_approaching_limit(&self, threshold: f32) -> bool {
        if self.limit == 0 {
            return false;
        }
        
        let usage_percentage = 1.0 - (self.remaining as f32 / self.limit as f32);
        usage_percentage >= threshold
    }
    
    /// Get seconds until rate limit resets
    pub fn seconds_until_reset(&self) -> u64 {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        
        if self.reset_timestamp > now {
            self.reset_timestamp - now
        } else {
            0
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::infrastructure::groq::types::GroqConfig;

    #[test]
    fn test_auth_creation() {
        let config = GroqConfig {
            api_key: "gsk_1234567890123456789012345678901234567890".to_string(),
            ..Default::default()
        };

        let auth = GroqAuth::new(&config);
        assert!(auth.is_ok());
        
        let auth = auth.unwrap();
        assert_eq!(auth.get_auth_method(), "Bearer Token");
        assert!(auth.is_configured());
    }

    #[test]
    fn test_api_key_validation() {
        let config = GroqConfig {
            api_key: "gsk_1234567890123456789012345678901234567890".to_string(),
            ..Default::default()
        };

        let auth = GroqAuth::new(&config).unwrap();
        assert!(auth.validate_api_key().is_ok());

        // Test invalid prefix
        let invalid_config = GroqConfig {
            api_key: "sk_1234567890123456789012345678901234567890".to_string(),
            ..Default::default()
        };

        let invalid_auth = GroqAuth::new(&invalid_config).unwrap();
        assert!(invalid_auth.validate_api_key().is_err());

        // Test short key
        let short_config = GroqConfig {
            api_key: "gsk_short".to_string(),
            ..Default::default()
        };

        let short_auth = GroqAuth::new(&short_config).unwrap();
        assert!(short_auth.validate_api_key().is_err());
    }

    #[test]
    fn test_create_headers() {
        let config = GroqConfig {
            api_key: "gsk_1234567890123456789012345678901234567890".to_string(),
            user_agent: Some("Test-Agent/1.0".to_string()),
            ..Default::default()
        };

        let auth = GroqAuth::new(&config).unwrap();
        let headers = auth.create_headers().unwrap();

        assert!(headers.contains_key("authorization"));
        assert!(headers.contains_key("content-type"));
        assert!(headers.contains_key("user-agent"));
        
        let auth_header = headers.get("authorization").unwrap().to_str().unwrap();
        assert!(auth_header.starts_with("Bearer gsk_"));
        
        assert_eq!(
            headers.get("user-agent").unwrap().to_str().unwrap(),
            "Test-Agent/1.0"
        );
    }

    #[test]
    fn test_masked_api_key() {
        let config = GroqConfig {
            api_key: "gsk_1234567890abcdef".to_string(),
            ..Default::default()
        };

        let auth = GroqAuth::new(&config).unwrap();
        assert_eq!(auth.get_masked_api_key(), "gsk_1234...cdef");
    }

    #[test]
    fn test_streaming_headers() {
        let config = GroqConfig {
            api_key: "gsk_1234567890123456789012345678901234567890".to_string(),
            ..Default::default()
        };

        let auth = GroqAuth::new(&config).unwrap();
        let headers = auth.create_streaming_headers().unwrap();

        assert!(headers.contains_key("authorization"));
        assert!(headers.contains_key("accept"));
        assert!(headers.contains_key("cache-control"));
        
        assert_eq!(
            headers.get("accept").unwrap().to_str().unwrap(),
            "text/event-stream"
        );
    }

    #[test]
    fn test_config_validation() {
        let config = GroqConfig {
            api_key: "gsk_1234567890123456789012345678901234567890".to_string(),
            ..Default::default()
        };

        let auth = GroqAuth::new(&config).unwrap();
        assert!(auth.validate_config().is_ok());
    }

    #[test]
    fn test_model_headers() {
        let config = GroqConfig {
            api_key: "gsk_1234567890123456789012345678901234567890".to_string(),
            ..Default::default()
        };

        let auth = GroqAuth::new(&config).unwrap();
        let headers = auth.create_model_headers("llama-3.1-70b-versatile").unwrap();

        assert!(headers.contains_key("x-groq-model"));
        assert_eq!(
            headers.get("x-groq-model").unwrap().to_str().unwrap(),
            "llama-3.1-70b-versatile"
        );
    }

    #[test]
    fn test_rate_limit_info() {
        let info = RateLimitInfo {
            remaining: 10,
            limit: 100,
            reset_timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs() + 3600,
        };

        assert!(info.is_approaching_limit(0.8)); // 90% used > 80% threshold
        assert!(!info.is_approaching_limit(0.95)); // 90% used < 95% threshold
        assert!(info.seconds_until_reset() > 0);
    }
}