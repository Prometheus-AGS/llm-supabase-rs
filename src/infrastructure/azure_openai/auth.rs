//! Azure OpenAI authentication module
//!
//! This module handles Azure OpenAI-specific authentication, which uses API keys
//! with the `api-key` header instead of the Bearer token format used by OpenAI.

use anyhow::{Context, Result};
use reqwest::header::{HeaderMap, HeaderName, HeaderValue, CONTENT_TYPE, USER_AGENT};
use serde::{Deserialize, Serialize};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tracing::{debug, trace, warn};

use super::types::{AzureOpenAIConfig, AzureADConfig};

/// Azure OpenAI authentication handler
#[derive(Debug, Clone)]
pub struct AzureOpenAIAuth {
    /// API key for authentication
    api_key: String,
    
    /// User agent string
    user_agent: String,
    
    /// Azure AD configuration (optional)
    azure_ad_config: Option<AzureADConfig>,
    
    /// Cached Azure AD token (if using Azure AD auth)
    cached_token: Option<AzureADToken>,
}

/// Azure AD token information
#[derive(Debug, Clone)]
struct AzureADToken {
    /// Access token
    token: String,
    
    /// Token expiration time (Unix timestamp)
    expires_at: u64,
    
    /// Token type (usually "Bearer")
    token_type: String,
}

/// Azure AD token response structure
#[derive(Debug, Deserialize)]
struct AzureADTokenResponse {
    access_token: String,
    expires_in: u64,
    token_type: String,
}

impl AzureOpenAIAuth {
    /// Create a new Azure OpenAI authentication handler
    pub fn new(config: &AzureOpenAIConfig) -> Result<Self> {
        debug!("Initializing Azure OpenAI authentication");
        
        if config.api_key.is_empty() {
            return Err(anyhow::anyhow!("Azure OpenAI API key cannot be empty"));
        }
        
        let user_agent = config.user_agent
            .clone()
            .unwrap_or_else(|| "LLM-Supabase-RS/1.0 (Azure OpenAI)".to_string());
        
        let auth = Self {
            api_key: config.api_key.clone(),
            user_agent,
            azure_ad_config: config.azure_ad.clone(),
            cached_token: None,
        };
        
        debug!("Azure OpenAI authentication initialized with API key: {}", 
               auth.get_masked_api_key());
        
        Ok(auth)
    }
    
    /// Validate the API key format
    pub fn validate_api_key(&self) -> Result<()> {
        debug!("Validating Azure OpenAI API key format");
        
        if self.api_key.is_empty() {
            return Err(anyhow::anyhow!("API key cannot be empty"));
        }
        
        if self.api_key.len() < 32 {
            return Err(anyhow::anyhow!(
                "API key appears to be too short (expected at least 32 characters, got {})",
                self.api_key.len()
            ));
        }
        
        // Azure OpenAI API keys are typically 32-64 characters of alphanumeric characters
        if !self.api_key.chars().all(|c| c.is_alphanumeric()) {
            warn!("API key contains non-alphanumeric characters, which may be invalid");
        }
        
        debug!("API key validation passed");
        Ok(())
    }
    
    /// Create HTTP headers for Azure OpenAI requests
    pub fn create_headers(&self) -> Result<HeaderMap> {
        let mut headers = HeaderMap::new();
        
        // Azure OpenAI uses api-key header instead of Authorization Bearer
        headers.insert(
            HeaderName::from_static("api-key"),
            HeaderValue::from_str(&self.api_key)
                .context("Failed to create api-key header value")?
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
        
        trace!("Created Azure OpenAI request headers (api-key masked)");
        Ok(headers)
    }
    
    /// Create HTTP headers with Azure AD authentication (if configured)
    pub async fn create_headers_with_azure_ad(&mut self) -> Result<HeaderMap> {
        if self.azure_ad_config.is_some() {
            // Use Azure AD authentication
            self.ensure_valid_azure_ad_token().await?;
            
            let mut headers = HeaderMap::new();
            
            if let Some(ref token) = self.cached_token {
                headers.insert(
                    reqwest::header::AUTHORIZATION,
                    HeaderValue::from_str(&format!("{} {}", token.token_type, token.token))
                        .context("Failed to create Authorization header value")?
                );
            }
            
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
            
            trace!("Created Azure AD authenticated headers");
            Ok(headers)
        } else {
            // Fall back to API key authentication
            self.create_headers()
        }
    }
    
    /// Ensure we have a valid Azure AD token (refresh if necessary)
    async fn ensure_valid_azure_ad_token(&mut self) -> Result<()> {
        // Clone config to avoid borrowing issues
        let config = self.azure_ad_config.clone();
        if let Some(config) = config {
            // Check if we need to refresh the token
            let now = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs();
            
            let needs_refresh = match &self.cached_token {
                Some(token) => token.expires_at <= now + 300, // Refresh 5 minutes before expiry
                None => true,
            };
            
            if needs_refresh {
                debug!("Refreshing Azure AD token");
                self.refresh_azure_ad_token(&config).await?;
            }
        }
        
        Ok(())
    }
    
    /// Refresh the Azure AD token
    async fn refresh_azure_ad_token(&mut self, config: &AzureADConfig) -> Result<()> {
        let client = reqwest::Client::new();
        let token_url = config.get_token_url();
        
        // Use a let binding to ensure the scope string lives long enough
        let default_scope = "https://cognitiveservices.azure.com/.default".to_string();
        let scope = config.scope.as_ref().unwrap_or(&default_scope);
        
        let form_data = [
            ("grant_type", "client_credentials"),
            ("client_id", &config.client_id),
            ("client_secret", &config.client_secret),
            ("scope", scope),
        ];
        
        let response = client
            .post(&token_url)
            .form(&form_data)
            .send()
            .await
            .context("Failed to request Azure AD token")?;
        
        if !response.status().is_success() {
            let error_text = response.text().await
                .unwrap_or_else(|_| "Unknown error".to_string());
            return Err(anyhow::anyhow!(
                "Azure AD token request failed: {}", error_text
            ));
        }
        
        let token_response: AzureADTokenResponse = response
            .json()
            .await
            .context("Failed to parse Azure AD token response")?;
        
        let expires_at = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() + token_response.expires_in;
        
        self.cached_token = Some(AzureADToken {
            token: token_response.access_token,
            expires_at,
            token_type: token_response.token_type,
        });
        
        debug!("Successfully refreshed Azure AD token, expires at {}", expires_at);
        Ok(())
    }
    
    /// Get masked API key for logging purposes
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
    
    /// Test the authentication by making a simple request
    pub async fn test_authentication(&self, endpoint: &str, api_version: &str) -> Result<bool> {
        debug!("Testing Azure OpenAI authentication");
        
        let client = reqwest::Client::new();
        let test_url = format!(
            "{}/openai/deployments?api-version={}",
            endpoint.trim_end_matches('/'),
            api_version
        );
        
        let headers = self.create_headers()?;
        
        let response = client
            .get(&test_url)
            .headers(headers)
            .timeout(Duration::from_secs(10))
            .send()
            .await
            .context("Failed to send authentication test request")?;
        
        let success = response.status().is_success() || response.status().as_u16() == 404;
        
        if success {
            debug!("Azure OpenAI authentication test successful");
        } else {
            let status = response.status();
            let error_text = response.text().await
                .unwrap_or_else(|_| "Unknown error".to_string());
            warn!("Authentication test failed with status {}: {}", status, error_text);
        }
        
        Ok(success)
    }
    
    /// Check if Azure AD authentication is configured
    pub fn has_azure_ad_config(&self) -> bool {
        self.azure_ad_config.is_some()
    }
    
    /// Get the authentication method being used
    pub fn get_auth_method(&self) -> &'static str {
        if self.azure_ad_config.is_some() {
            "Azure AD"
        } else {
            "API Key"
        }
    }
    
    /// Create headers for streaming requests (same as regular requests for Azure OpenAI)
    pub fn create_streaming_headers(&self) -> Result<HeaderMap> {
        let mut headers = self.create_headers()?;
        
        // Azure OpenAI streaming uses the same headers as regular requests
        headers.insert(
            HeaderName::from_static("accept"),
            HeaderValue::from_static("text/event-stream")
        );
        
        trace!("Created Azure OpenAI streaming headers");
        Ok(headers)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::infrastructure::azure_openai::types::AzureOpenAIConfig;

    #[test]
    fn test_auth_creation() {
        let config = AzureOpenAIConfig {
            api_key: "test-api-key-1234567890123456789012345678901234567890".to_string(),
            endpoint: "https://test-resource.openai.azure.com".to_string(),
            deployment: "gpt-4o".to_string(),
            ..Default::default()
        };

        let auth = AzureOpenAIAuth::new(&config);
        assert!(auth.is_ok());
        
        let auth = auth.unwrap();
        assert_eq!(auth.get_auth_method(), "API Key");
        assert!(!auth.has_azure_ad_config());
    }

    #[test]
    fn test_api_key_validation() {
        let config = AzureOpenAIConfig {
            api_key: "test-api-key-1234567890123456789012345678901234567890".to_string(),
            ..Default::default()
        };

        let auth = AzureOpenAIAuth::new(&config).unwrap();
        assert!(auth.validate_api_key().is_ok());

        let invalid_config = AzureOpenAIConfig {
            api_key: "short".to_string(),
            ..Default::default()
        };

        let invalid_auth = AzureOpenAIAuth::new(&invalid_config).unwrap();
        assert!(invalid_auth.validate_api_key().is_err());
    }

    #[test]
    fn test_create_headers() {
        let config = AzureOpenAIConfig {
            api_key: "test-api-key-1234567890123456789012345678901234567890".to_string(),
            user_agent: Some("Test-Agent/1.0".to_string()),
            ..Default::default()
        };

        let auth = AzureOpenAIAuth::new(&config).unwrap();
        let headers = auth.create_headers().unwrap();

        assert!(headers.contains_key("api-key"));
        assert!(headers.contains_key("content-type"));
        assert!(headers.contains_key("user-agent"));
        
        assert_eq!(
            headers.get("api-key").unwrap().to_str().unwrap(),
            "test-api-key-1234567890123456789012345678901234567890"
        );
        
        assert_eq!(
            headers.get("user-agent").unwrap().to_str().unwrap(),
            "Test-Agent/1.0"
        );
    }

    #[test]
    fn test_masked_api_key() {
        let config = AzureOpenAIConfig {
            api_key: "1234567890abcdef".to_string(),
            ..Default::default()
        };

        let auth = AzureOpenAIAuth::new(&config).unwrap();
        assert_eq!(auth.get_masked_api_key(), "1234...cdef");
    }

    #[test]
    fn test_streaming_headers() {
        let config = AzureOpenAIConfig {
            api_key: "test-api-key-1234567890123456789012345678901234567890".to_string(),
            ..Default::default()
        };

        let auth = AzureOpenAIAuth::new(&config).unwrap();
        let headers = auth.create_streaming_headers().unwrap();

        assert!(headers.contains_key("api-key"));
        assert!(headers.contains_key("accept"));
        assert_eq!(
            headers.get("accept").unwrap().to_str().unwrap(),
            "text/event-stream"
        );
    }

    #[test]
    fn test_azure_ad_config() {
        let azure_ad = AzureADConfig {
            tenant_id: "test-tenant".to_string(),
            client_id: "test-client".to_string(),
            client_secret: "test-secret".to_string(),
            scope: None,
        };

        let config = AzureOpenAIConfig {
            api_key: "test-api-key-1234567890123456789012345678901234567890".to_string(),
            azure_ad: Some(azure_ad),
            ..Default::default()
        };

        let auth = AzureOpenAIAuth::new(&config).unwrap();
        assert!(auth.has_azure_ad_config());
        assert_eq!(auth.get_auth_method(), "Azure AD");
    }
}