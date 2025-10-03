// src/infrastructure/vertex/auth.rs
//
// Vertex AI authentication client for Google Cloud Platform

use anyhow::{Context, Result};
use gcp_auth::TokenProvider;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::sync::RwLock;
use tracing::{debug, info};

use crate::config::providers::{VertexAuthConfig, VertexAuthMethod};

/// Vertex AI authentication client
/// Handles OAuth2 token management for Google Cloud Platform APIs
pub struct VertexAuthClient {
    /// Authentication provider from gcp_auth
    provider: Arc<dyn TokenProvider>,

    /// Cached access token
    cached_token: Arc<RwLock<Option<CachedToken>>>,

    /// Authentication configuration
    config: VertexAuthConfig,
}

/// Cached authentication token with expiry information
#[derive(Debug, Clone)]
struct CachedToken {
    /// The access token
    token: String,

    /// Token expiry time (Unix timestamp)
    expires_at: u64,

    /// Buffer time before expiry (seconds)
    refresh_buffer: u64,
}

/// Authentication context for requests
#[derive(Debug, Clone)]
pub struct AuthContext {
    /// Bearer token for authorization header
    pub bearer_token: String,

    /// Token expiry time
    pub expires_at: u64,

    /// Whether token is close to expiry and should be refreshed
    pub needs_refresh: bool,
}


impl VertexAuthClient {
    /// Create a new Vertex AI authentication client
    pub async fn new(config: VertexAuthConfig) -> Result<Self> {
        // Use the gcp_auth provider() function which automatically selects the best authentication method
        let provider = gcp_auth::provider().await
            .context("Failed to create GCP authentication provider")?;

        info!(
            method = ?config.auth_method,
            "Created Vertex AI authentication client"
        );

        Ok(Self {
            provider,
            cached_token: Arc::new(RwLock::new(None)),
            config,
        })
    }

    /// Get an authentication context with a valid access token
    pub async fn get_auth_context(&self) -> Result<AuthContext> {
        // Check if we have a cached token that's still valid
        {
            let cached = self.cached_token.read().await;
            if let Some(ref token) = *cached {
                if token.is_valid() {
                    debug!("Using cached authentication token");
                    return Ok(AuthContext {
                        bearer_token: token.token.clone(),
                        expires_at: token.expires_at,
                        needs_refresh: token.needs_refresh(),
                    });
                }
            }
        }

        // Need to fetch a new token
        debug!("Fetching new authentication token");
        let token = self.fetch_fresh_token().await?;

        // Cache the new token
        {
            let mut cached = self.cached_token.write().await;
            *cached = Some(token.clone());
        }

        Ok(AuthContext {
            bearer_token: token.token,
            expires_at: token.expires_at,
            needs_refresh: false,
        })
    }

    /// Force refresh the authentication token
    pub async fn refresh_token(&self) -> Result<AuthContext> {
        debug!("Force refreshing authentication token");

        let token = self.fetch_fresh_token().await?;

        // Update cached token
        {
            let mut cached = self.cached_token.write().await;
            *cached = Some(token.clone());
        }

        Ok(AuthContext {
            bearer_token: token.token,
            expires_at: token.expires_at,
            needs_refresh: false,
        })
    }

    /// Fetch a fresh token from the authentication provider
    async fn fetch_fresh_token(&self) -> Result<CachedToken> {
        // Convert Vec<String> to Vec<&str>
        let scopes: Vec<&str> = self.config.scopes.iter().map(|s| s.as_str()).collect();

        let token = self.provider
            .token(&scopes)
            .await
            .context("Failed to get access token from authentication provider")?;

        let expires_at = token.expires_at().timestamp() as u64;

        let cached_token = CachedToken {
            token: token.as_str().to_string(),
            expires_at,
            refresh_buffer: 300, // Refresh 5 minutes before expiry
        };

        info!(
            expires_at = %expires_at,
            "Successfully obtained new authentication token"
        );

        Ok(cached_token)
    }

    /// Check if the current authentication is valid
    pub async fn is_authenticated(&self) -> bool {
        let cached = self.cached_token.read().await;
        cached.as_ref().map(|token| token.is_valid()).unwrap_or(false)
    }

    /// Get the current token expiry time (if any)
    pub async fn token_expires_at(&self) -> Option<u64> {
        let cached = self.cached_token.read().await;
        cached.as_ref().map(|token| token.expires_at)
    }

    /// Validate that authentication is working by making a test request
    pub async fn validate_auth(&self) -> Result<()> {
        let auth_context = self.get_auth_context().await?;

        // Test the token by making a simple request to GCP
        let client = reqwest::Client::new();
        let response = client
            .get("https://oauth2.googleapis.com/tokeninfo")
            .bearer_auth(&auth_context.bearer_token)
            .send()
            .await
            .context("Failed to validate authentication token")?;

        if response.status().is_success() {
            info!("Authentication validation successful");
            Ok(())
        } else {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            anyhow::bail!(
                "Authentication validation failed with status {}: {}",
                status,
                error_text
            );
        }
    }
}

impl CachedToken {
    /// Check if the token is still valid (not expired)
    fn is_valid(&self) -> bool {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        now < self.expires_at
    }

    /// Check if the token needs to be refreshed soon
    fn needs_refresh(&self) -> bool {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        now >= (self.expires_at - self.refresh_buffer)
    }
}

impl AuthContext {
    /// Get the authorization header value for HTTP requests
    pub fn authorization_header(&self) -> String {
        format!("Bearer {}", self.bearer_token)
    }

    /// Check if this auth context is still valid
    pub fn is_valid(&self) -> bool {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        now < self.expires_at
    }

    /// Get time until expiry in seconds
    pub fn time_until_expiry(&self) -> u64 {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        if self.expires_at > now {
            self.expires_at - now
        } else {
            0
        }
    }
}

/// Convenient builder for creating VertexAuthClient with different configurations
pub struct VertexAuthClientBuilder {
    config: VertexAuthConfig,
}

impl VertexAuthClientBuilder {
    /// Create a new builder with default configuration
    pub fn new() -> Self {
        Self {
            config: VertexAuthConfig::default(),
        }
    }

    /// Set the authentication method
    pub fn with_auth_method(mut self, method: VertexAuthMethod) -> Self {
        self.config.auth_method = method;
        self
    }

    /// Set the OAuth scopes
    pub fn with_scopes(mut self, scopes: Vec<String>) -> Self {
        self.config.scopes = scopes;
        self
    }

    /// Set the token refresh interval
    pub fn with_refresh_interval(mut self, seconds: u64) -> Self {
        self.config.token_refresh_interval = seconds;
        self
    }

    /// Build the authentication client
    pub async fn build(self) -> Result<VertexAuthClient> {
        VertexAuthClient::new(self.config).await
    }
}

impl Default for VertexAuthClientBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;

    // Note: These tests require actual GCP credentials to run
    // They are integration tests and should be run with `cargo test --ignored`

    #[tokio::test]
    #[ignore = "requires GCP credentials"]
    async fn test_gcp_auth_provider() {
        let config = VertexAuthConfig::default();

        let client = VertexAuthClient::new(config).await;
        if client.is_err() {
            eprintln!("GCP credentials not available, skipping test");
            return;
        }

        let client = client.unwrap();
        let auth_context = client.get_auth_context().await;
        assert!(auth_context.is_ok(), "Failed to get auth context: {:?}", auth_context.err());

        let context = auth_context.unwrap();
        assert!(!context.bearer_token.is_empty());
        assert!(context.is_valid());
    }

    #[tokio::test]
    #[ignore = "requires GCP credentials"]
    async fn test_application_default_auth() {
        let config = VertexAuthConfig {
            auth_method: VertexAuthMethod::ApplicationDefault,
            ..Default::default()
        };

        let client = VertexAuthClient::new(config).await;
        if client.is_err() {
            eprintln!("Application default credentials not available, skipping test");
            return;
        }

        let client = client.unwrap();
        let auth_context = client.get_auth_context().await;
        assert!(auth_context.is_ok());
    }

    #[test]
    fn test_cached_token_validity() {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        // Valid token
        let valid_token = CachedToken {
            token: "test_token".to_string(),
            expires_at: now + 3600, // 1 hour from now
            refresh_buffer: 300,
        };

        assert!(valid_token.is_valid());
        assert!(!valid_token.needs_refresh());

        // Expired token
        let expired_token = CachedToken {
            token: "test_token".to_string(),
            expires_at: now - 3600, // 1 hour ago
            refresh_buffer: 300,
        };

        assert!(!expired_token.is_valid());
        assert!(expired_token.needs_refresh());

        // Token that needs refresh
        let refresh_token = CachedToken {
            token: "test_token".to_string(),
            expires_at: now + 200, // 200 seconds from now
            refresh_buffer: 300,   // Refresh 300 seconds before expiry
        };

        assert!(refresh_token.is_valid());
        assert!(refresh_token.needs_refresh());
    }

    #[test]
    fn test_auth_context() {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        let context = AuthContext {
            bearer_token: "test_token".to_string(),
            expires_at: now + 3600,
            needs_refresh: false,
        };

        assert_eq!(context.authorization_header(), "Bearer test_token");
        assert!(context.is_valid());
        assert!(context.time_until_expiry() > 3590); // Should be close to 3600
    }

    #[test]
    fn test_builder_pattern() {
        let builder = VertexAuthClientBuilder::new()
            .with_auth_method(VertexAuthMethod::ApplicationDefault)
            .with_scopes(vec!["https://www.googleapis.com/auth/cloud-platform".to_string()])
            .with_refresh_interval(1800);

        assert!(matches!(builder.config.auth_method, VertexAuthMethod::ApplicationDefault));
        assert_eq!(builder.config.scopes.len(), 1);
        assert_eq!(builder.config.token_refresh_interval, 1800);
    }

    #[tokio::test]
    async fn test_token_caching() {
        // Create a mock authentication client for testing token caching behavior
        let config = VertexAuthConfig::default();
        let cached_token = Arc::new(RwLock::new(None));

        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        // Simulate a cached token
        let token = CachedToken {
            token: "cached_token".to_string(),
            expires_at: now + 3600,
            refresh_buffer: 300,
        };

        {
            let mut cache = cached_token.write().await;
            *cache = Some(token);
        }

        // Verify token is cached
        {
            let cache = cached_token.read().await;
            assert!(cache.is_some());
            assert!(cache.as_ref().unwrap().is_valid());
        }
    }
}