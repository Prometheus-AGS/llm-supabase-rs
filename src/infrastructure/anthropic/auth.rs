//! Anthropic API authentication implementation
//! 
//! This module handles Anthropic API authentication using API keys.
//! Anthropic uses x-api-key header authentication instead of Bearer tokens.

use anyhow::{anyhow, Context, Result};
use reqwest::header::{HeaderMap, HeaderName, HeaderValue, CONTENT_TYPE, USER_AGENT};
use std::str::FromStr;
use tracing::{debug, warn};

use super::types::AnthropicConfig;

/// Anthropic authentication handler
/// 
/// Manages API key authentication for Anthropic API requests.
/// Unlike OpenAI which uses Bearer tokens, Anthropic uses x-api-key headers.
#[derive(Debug, Clone)]
pub struct AnthropicAuth {
    /// API key for authentication
    api_key: String,
    
    /// Organization ID (optional)
    organization_id: Option<String>,
    
    /// Additional custom headers
    custom_headers: Option<std::collections::HashMap<String, String>>,
}

impl AnthropicAuth {
    /// Create a new Anthropic authentication handler
    pub fn new(config: &AnthropicConfig) -> Result<Self> {
        if config.api_key.is_empty() {
            return Err(anyhow!("Anthropic API key is required"));
        }

        debug!("Initializing Anthropic authentication");

        Ok(Self {
            api_key: config.api_key.clone(),
            organization_id: config.organization_id.clone(),
            custom_headers: config.custom_headers.clone(),
        })
    }

    /// Validate the API key format
    pub fn validate_api_key(&self) -> Result<()> {
        if self.api_key.is_empty() {
            return Err(anyhow!("API key cannot be empty"));
        }

        // Anthropic API keys typically start with "sk-ant-"
        if !self.api_key.starts_with("sk-ant-") {
            warn!("API key does not start with expected prefix 'sk-ant-'. This may indicate an invalid key.");
        }

        // Check minimum length (Anthropic keys are typically 95+ characters)
        if self.api_key.len() < 40 {
            return Err(anyhow!("API key appears to be too short"));
        }

        // Check for common invalid characters
        if self.api_key.contains(' ') || self.api_key.contains('\t') || self.api_key.contains('\n') {
            return Err(anyhow!("API key contains invalid whitespace characters"));
        }

        debug!("API key validation passed");
        Ok(())
    }

    /// Create HTTP headers for Anthropic API requests
    pub fn create_headers(&self) -> Result<HeaderMap> {
        let mut headers = HeaderMap::new();

        // Add the API key header (x-api-key instead of Authorization)
        let api_key_header = HeaderValue::from_str(&self.api_key)
            .context("Failed to create API key header value")?;
        headers.insert(
            HeaderName::from_str("x-api-key")
                .context("Failed to create x-api-key header name")?,
            api_key_header
        );

        // Add required Anthropic version header
        headers.insert(
            HeaderName::from_str("anthropic-version")
                .context("Failed to create anthropic-version header name")?,
            HeaderValue::from_str("2023-06-01")
                .context("Failed to create anthropic-version header value")?
        );

        // Add standard content type header
        headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));

        // Add user agent
        headers.insert(
            USER_AGENT, 
            HeaderValue::from_static("llm-supabase-rs/1.0.0 (Anthropic Client)")
        );

        // Add organization ID if provided
        if let Some(org_id) = &self.organization_id {
            headers.insert(
                HeaderName::from_str("anthropic-organization")
                    .context("Failed to create anthropic-organization header name")?,
                HeaderValue::from_str(org_id)
                    .context("Failed to create organization header value")?
            );
            debug!("Added organization header: {}", org_id);
        }

        // Add any custom headers
        if let Some(custom_headers) = &self.custom_headers {
            for (key, value) in custom_headers {
                let header_name = HeaderName::from_str(key)
                    .with_context(|| format!("Failed to create custom header name: {}", key))?;
                let header_value = HeaderValue::from_str(value)
                    .with_context(|| format!("Failed to create custom header value: {}", value))?;
                headers.insert(header_name, header_value);
                debug!("Added custom header: {} = {}", key, value);
            }
        }

        debug!("Created Anthropic API headers with {} entries", headers.len());
        Ok(headers)
    }

    /// Get the API key (for logging/debugging purposes)
    /// Returns a masked version for security
    pub fn get_masked_api_key(&self) -> String {
        if self.api_key.len() <= 8 {
            "*".repeat(self.api_key.len())
        } else {
            format!("{}...{}", 
                &self.api_key[..4], 
                &self.api_key[self.api_key.len()-4..]
            )
        }
    }

    /// Get the organization ID
    pub fn get_organization_id(&self) -> Option<&String> {
        self.organization_id.as_ref()
    }

    /// Test if the API key is likely valid (basic format check)
    pub fn is_api_key_format_valid(&self) -> bool {
        !self.api_key.is_empty() &&
        self.api_key.len() >= 40 &&
        !self.api_key.contains(' ') &&
        !self.api_key.contains('\t') &&
        !self.api_key.contains('\n')
    }

    /// Create headers for streaming requests
    /// Same as regular headers but may include additional streaming-specific headers
    pub fn create_streaming_headers(&self) -> Result<HeaderMap> {
        let mut headers = self.create_headers()?;
        
        // Add accept header for streaming
        headers.insert(
            HeaderName::from_str("accept")
                .context("Failed to create accept header name")?,
            HeaderValue::from_str("text/event-stream")
                .context("Failed to create accept header value")?
        );

        // Add cache control for streaming
        headers.insert(
            HeaderName::from_str("cache-control")
                .context("Failed to create cache-control header name")?,
            HeaderValue::from_str("no-cache")
                .context("Failed to create cache-control header value")?
        );

        debug!("Created streaming headers with {} entries", headers.len());
        Ok(headers)
    }

    /// Update the API key (for runtime configuration changes)
    pub fn update_api_key(&mut self, new_api_key: String) -> Result<()> {
        if new_api_key.is_empty() {
            return Err(anyhow!("New API key cannot be empty"));
        }

        let old_masked = self.get_masked_api_key();
        self.api_key = new_api_key;
        
        // Validate the new key
        self.validate_api_key()?;
        
        debug!("Updated API key from {} to {}", old_masked, self.get_masked_api_key());
        Ok(())
    }

    /// Update organization ID
    pub fn update_organization_id(&mut self, org_id: Option<String>) {
        let old_org = self.organization_id.clone();
        self.organization_id = org_id;
        debug!("Updated organization ID from {:?} to {:?}", old_org, self.organization_id);
    }

    /// Add or update a custom header
    pub fn add_custom_header(&mut self, key: String, value: String) {
        if self.custom_headers.is_none() {
            self.custom_headers = Some(std::collections::HashMap::new());
        }
        
        if let Some(headers) = &mut self.custom_headers {
            headers.insert(key.clone(), value.clone());
            debug!("Added custom header: {} = {}", key, value);
        }
    }

    /// Remove a custom header
    pub fn remove_custom_header(&mut self, key: &str) -> Option<String> {
        if let Some(headers) = &mut self.custom_headers {
            let removed = headers.remove(key);
            if removed.is_some() {
                debug!("Removed custom header: {}", key);
            }
            removed
        } else {
            None
        }
    }

    /// Clear all custom headers
    pub fn clear_custom_headers(&mut self) {
        if let Some(headers) = &mut self.custom_headers {
            let count = headers.len();
            headers.clear();
            debug!("Cleared {} custom headers", count);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::infrastructure::anthropic::types::AnthropicConfig;

    fn create_test_config() -> AnthropicConfig {
        AnthropicConfig {
            api_key: "sk-ant-test123456789012345678901234567890123456789012345".to_string(),
            organization_id: Some("org-test123".to_string()),
            ..Default::default()
        }
    }

    #[test]
    fn test_auth_creation() {
        let config = create_test_config();
        let auth = AnthropicAuth::new(&config);
        assert!(auth.is_ok());

        let auth = auth.unwrap();
        assert_eq!(auth.get_organization_id(), Some(&"org-test123".to_string()));
    }

    #[test]
    fn test_auth_creation_empty_key() {
        let mut config = create_test_config();
        config.api_key = "".to_string();
        
        let auth = AnthropicAuth::new(&config);
        assert!(auth.is_err());
    }

    #[test]
    fn test_api_key_validation() {
        let config = create_test_config();
        let auth = AnthropicAuth::new(&config).unwrap();
        
        assert!(auth.validate_api_key().is_ok());
        assert!(auth.is_api_key_format_valid());
    }

    #[test]
    fn test_api_key_validation_invalid() {
        let mut config = create_test_config();
        
        // Test short key
        config.api_key = "short".to_string();
        let auth = AnthropicAuth::new(&config).unwrap();
        assert!(auth.validate_api_key().is_err());
        
        // Test key with whitespace
        config.api_key = "sk-ant-test123456789012345678901234567890123456789012345 ".to_string();
        let auth = AnthropicAuth::new(&config).unwrap();
        assert!(auth.validate_api_key().is_err());
    }

    #[test]
    fn test_headers_creation() {
        let config = create_test_config();
        let auth = AnthropicAuth::new(&config).unwrap();
        
        let headers = auth.create_headers().unwrap();
        
        // Check required headers
        assert!(headers.contains_key("x-api-key"));
        assert!(headers.contains_key("anthropic-version"));
        assert!(headers.contains_key("content-type"));
        assert!(headers.contains_key("user-agent"));
        assert!(headers.contains_key("anthropic-organization"));
        
        // Check values
        assert_eq!(headers.get("anthropic-version").unwrap(), "2023-06-01");
        assert_eq!(headers.get("content-type").unwrap(), "application/json");
    }

    #[test]
    fn test_streaming_headers() {
        let config = create_test_config();
        let auth = AnthropicAuth::new(&config).unwrap();
        
        let headers = auth.create_streaming_headers().unwrap();
        
        // Check streaming-specific headers
        assert!(headers.contains_key("accept"));
        assert!(headers.contains_key("cache-control"));
        
        // Check values
        assert_eq!(headers.get("accept").unwrap(), "text/event-stream");
        assert_eq!(headers.get("cache-control").unwrap(), "no-cache");
    }

    #[test]
    fn test_masked_api_key() {
        let config = create_test_config();
        let auth = AnthropicAuth::new(&config).unwrap();
        
        let masked = auth.get_masked_api_key();
        assert!(masked.contains("sk-a"));
        assert!(masked.contains("..."));
        assert!(masked.contains("2345"));
        assert!(!masked.contains("test123456789012345678901234567890123456789"));
    }

    #[test]
    fn test_custom_headers() {
        let config = create_test_config();
        let mut auth = AnthropicAuth::new(&config).unwrap();
        
        // Add custom header
        auth.add_custom_header("x-custom".to_string(), "custom-value".to_string());
        
        let headers = auth.create_headers().unwrap();
        assert!(headers.contains_key("x-custom"));
        assert_eq!(headers.get("x-custom").unwrap(), "custom-value");
        
        // Remove custom header
        let removed = auth.remove_custom_header("x-custom");
        assert_eq!(removed, Some("custom-value".to_string()));
        
        let headers = auth.create_headers().unwrap();
        assert!(!headers.contains_key("x-custom"));
    }

    #[test]
    fn test_api_key_update() {
        let config = create_test_config();
        let mut auth = AnthropicAuth::new(&config).unwrap();
        
        let old_masked = auth.get_masked_api_key();
        
        let new_key = "sk-ant-new123456789012345678901234567890123456789012345".to_string();
        let result = auth.update_api_key(new_key);
        assert!(result.is_ok());
        
        let new_masked = auth.get_masked_api_key();
        assert_ne!(old_masked, new_masked);
        assert!(new_masked.contains("sk-a"));
        assert!(new_masked.contains("2345"));
    }

    #[test]
    fn test_organization_id_update() {
        let config = create_test_config();
        let mut auth = AnthropicAuth::new(&config).unwrap();
        
        assert_eq!(auth.get_organization_id(), Some(&"org-test123".to_string()));
        
        auth.update_organization_id(Some("org-new456".to_string()));
        assert_eq!(auth.get_organization_id(), Some(&"org-new456".to_string()));
        
        auth.update_organization_id(None);
        assert_eq!(auth.get_organization_id(), None);
    }
}