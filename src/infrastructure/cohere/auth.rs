//! Cohere authentication implementation
//! 
//! This module handles API key-based authentication for the Cohere API.
//! Cohere uses Bearer token authentication via the Authorization header.

use anyhow::Result;
use reqwest::{Client, RequestBuilder};
use tracing::{debug, warn};
use crate::infrastructure::cohere::types::CohereConfig;

/// Cohere authentication handler
#[derive(Debug, Clone)]
pub struct CohereAuth {
    /// API key for authentication
    api_key: String,
}

impl CohereAuth {
    /// Create new Cohere authentication with API key
    pub fn new(api_key: String) -> Self {
        Self { api_key }
    }
    
    /// Create authentication from configuration
    pub fn from_config(config: &CohereConfig) -> Self {
        Self::new(config.api_key.clone())
    }
    
    /// Create authentication from environment variable
    pub fn from_env() -> Result<Self> {
        let api_key = std::env::var("COHERE_API_KEY")
            .map_err(|_| anyhow::anyhow!("COHERE_API_KEY environment variable not set"))?;
        
        Ok(Self::new(api_key))
    }
    
    /// Apply authentication to a request builder
    pub fn apply_auth(&self, builder: RequestBuilder) -> RequestBuilder {
        debug!("Applying Cohere authentication");
        builder.bearer_auth(&self.api_key)
    }
    
    /// Apply authentication headers to a reqwest client
    pub fn get_authenticated_client(&self) -> Result<Client> {
        let mut headers = reqwest::header::HeaderMap::new();
        
        // Add Authorization header with Bearer token
        let auth_header = format!("Bearer {}", self.api_key);
        headers.insert(
            reqwest::header::AUTHORIZATION,
            auth_header.parse()
                .map_err(|e| anyhow::anyhow!("Failed to create auth header: {}", e))?
        );
        
        // Add Content-Type header
        headers.insert(
            reqwest::header::CONTENT_TYPE,
            "application/json".parse()
                .map_err(|e| anyhow::anyhow!("Failed to create content-type header: {}", e))?
        );
        
        // Add User-Agent header
        headers.insert(
            reqwest::header::USER_AGENT,
            "llm-supabase-rs/1.0".parse()
                .map_err(|e| anyhow::anyhow!("Failed to create user-agent header: {}", e))?
        );
        
        // Build client with headers
        let client = Client::builder()
            .default_headers(headers)
            .build()
            .map_err(|e| anyhow::anyhow!("Failed to create HTTP client: {}", e))?;
        
        debug!("Created authenticated Cohere HTTP client");
        Ok(client)
    }
    
    /// Validate the API key format
    pub fn validate_api_key(&self) -> Result<()> {
        if self.api_key.is_empty() {
            return Err(anyhow::anyhow!("API key cannot be empty"));
        }
        
        // Cohere API keys typically don't have a specific format requirement
        // but we can check for minimum length
        if self.api_key.len() < 10 {
            warn!("Cohere API key seems unusually short, this might cause authentication failures");
        }
        
        debug!("Cohere API key validation passed");
        Ok(())
    }
    
    /// Test authentication by making a simple request
    pub async fn test_authentication(&self) -> Result<bool> {
        debug!("Testing Cohere authentication");
        
        let client = self.get_authenticated_client()?;
        
        // Make a simple request to test auth
        // We'll use a minimal chat request to verify authentication works
        let test_payload = serde_json::json!({
            "model": "command-r",
            "message": "Hello",
            "max_tokens": 1
        });
        
        let response = client
            .post("https://api.cohere.ai/v1/chat")
            .json(&test_payload)
            .send()
            .await;
        
        match response {
            Ok(resp) => {
                if resp.status().is_success() {
                    debug!("Cohere authentication test successful");
                    Ok(true)
                } else if resp.status() == 401 {
                    warn!("Cohere authentication test failed: Unauthorized (401)");
                    Ok(false)
                } else {
                    warn!("Cohere authentication test returned status: {}", resp.status());
                    // Other errors might not be auth-related, so we'll consider auth valid
                    Ok(true)
                }
            }
            Err(e) => {
                warn!("Cohere authentication test failed with network error: {}", e);
                // Network errors don't necessarily mean auth is invalid
                Ok(true)
            }
        }
    }
    
    /// Get the API key (for debugging, returns masked version)
    pub fn get_masked_api_key(&self) -> String {
        if self.api_key.len() > 8 {
            format!("{}...{}", 
                &self.api_key[..4], 
                &self.api_key[self.api_key.len()-4..]
            )
        } else {
            "****".to_string()
        }
    }
    
    /// Check if authentication is configured
    pub fn is_configured(&self) -> bool {
        !self.api_key.is_empty()
    }
}

/// Helper functions for authentication management
impl CohereAuth {
    /// Create a test authentication instance
    #[cfg(test)]
    pub fn create_test() -> Self {
        Self::new("test-api-key-1234567890".to_string())
    }
    
    /// Refresh authentication if needed (Cohere uses static API keys)
    pub async fn refresh(&mut self) -> Result<()> {
        // Cohere uses static API keys, so no refresh is needed
        // But we can re-validate the key
        self.validate_api_key()?;
        debug!("Cohere authentication refreshed (validation only)");
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_auth_creation() {
        let auth = CohereAuth::new("test-key".to_string());
        assert!(auth.is_configured());
    }
    
    #[test]
    fn test_api_key_validation() {
        let auth = CohereAuth::new("valid-api-key".to_string());
        assert!(auth.validate_api_key().is_ok());
        
        let empty_auth = CohereAuth::new(String::new());
        assert!(empty_auth.validate_api_key().is_err());
    }
    
    #[test]
    fn test_masked_api_key() {
        let auth = CohereAuth::new("1234567890abcdef".to_string());
        let masked = auth.get_masked_api_key();
        assert!(masked.contains("1234"));
        assert!(masked.contains("cdef"));
        assert!(masked.contains("..."));
    }
    
    #[test]
    fn test_short_api_key_masking() {
        let auth = CohereAuth::new("short".to_string());
        let masked = auth.get_masked_api_key();
        assert_eq!(masked, "****");
    }
    
    #[tokio::test]
    async fn test_client_creation() {
        let auth = CohereAuth::create_test();
        let client_result = auth.get_authenticated_client();
        assert!(client_result.is_ok());
    }
    
    #[tokio::test]
    async fn test_refresh() {
        let mut auth = CohereAuth::create_test();
        let result = auth.refresh().await;
        assert!(result.is_ok());
    }
}