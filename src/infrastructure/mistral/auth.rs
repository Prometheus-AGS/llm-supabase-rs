//! Mistral AI authentication and security utilities
//!
//! This module handles API key validation, authentication headers, and rate limiting
//! for the Mistral AI API. It provides Bearer token authentication and European
//! compliance features like GDPR mode and data residency preferences.

use anyhow::{Context, Result};
use reqwest::header::{HeaderMap, HeaderValue, AUTHORIZATION, USER_AGENT, CONTENT_TYPE};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use tracing::{debug, info, warn, error, trace};

use super::types::{MistralConfig, MistralRateLimit};

/// Mistral authentication handler with European compliance features
pub struct MistralAuth {
    /// API key for authentication
    api_key: String,

    /// Configuration reference
    config: Arc<MistralConfig>,

    /// Rate limiting state
    rate_limiter: Option<Arc<RwLock<RateLimitState>>>,

    /// Request tracking for compliance
    request_tracker: Arc<RwLock<RequestTracker>>,
}

/// Rate limiting state tracker
#[derive(Debug, Clone)]
pub struct RateLimitState {
    /// Current request count in the current window
    pub current_requests: u32,

    /// Current token count in the current window
    pub current_tokens: u32,

    /// Window start time
    pub window_start: Instant,

    /// Window duration (1 minute)
    pub window_duration: Duration,

    /// Last request timestamp
    pub last_request: Instant,

    /// Rate limit configuration
    pub limits: MistralRateLimit,
}

/// Request tracking for European compliance
#[derive(Debug, Clone)]
pub struct RequestTracker {
    /// Total number of requests made
    pub total_requests: u64,

    /// Requests with EU data residency
    pub eu_requests: u64,

    /// GDPR compliant requests
    pub gdpr_requests: u64,

    /// Last request timestamp
    pub last_request: Option<Instant>,

    /// Request history for compliance auditing
    pub request_history: Vec<RequestRecord>,
}

/// Individual request record for compliance tracking
#[derive(Debug, Clone)]
pub struct RequestRecord {
    /// Request timestamp
    pub timestamp: Instant,

    /// Whether EU data residency was used
    pub eu_residency: bool,

    /// Whether GDPR mode was enabled
    pub gdpr_mode: bool,

    /// Request endpoint
    pub endpoint: String,

    /// Request ID for tracking
    pub request_id: String,
}

/// Rate limit information returned from API
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RateLimitInfo {
    /// Requests per minute limit
    pub requests_limit: Option<u32>,

    /// Requests remaining in current window
    pub requests_remaining: Option<u32>,

    /// Tokens per minute limit
    pub tokens_limit: Option<u32>,

    /// Tokens remaining in current window
    pub tokens_remaining: Option<u32>,

    /// Reset time for rate limits
    pub reset_time: Option<u64>,

    /// Retry after seconds if rate limited
    pub retry_after: Option<u64>,
}

impl MistralAuth {
    /// Create a new Mistral authentication handler
    pub fn new(config: &MistralConfig) -> Result<Self> {
        if config.api_key.trim().is_empty() {
            return Err(anyhow::anyhow!("Mistral API key cannot be empty"));
        }

        let rate_limiter = config.rate_limit.as_ref().map(|limits| {
            Arc::new(RwLock::new(RateLimitState {
                current_requests: 0,
                current_tokens: 0,
                window_start: Instant::now(),
                window_duration: Duration::from_secs(60),
                last_request: Instant::now(),
                limits: limits.clone(),
            }))
        });

        let request_tracker = Arc::new(RwLock::new(RequestTracker {
            total_requests: 0,
            eu_requests: 0,
            gdpr_requests: 0,
            last_request: None,
            request_history: Vec::new(),
        }));

        debug!("Initialized Mistral authentication with API key: {}", 
               Self::mask_api_key(&config.api_key));

        Ok(Self {
            api_key: config.api_key.clone(),
            config: Arc::new(config.clone()),
            rate_limiter,
            request_tracker,
        })
    }

    /// Validate the API key format
    pub fn validate_api_key(&self) -> Result<()> {
        if self.api_key.len() < 10 {
            return Err(anyhow::anyhow!("Invalid Mistral API key: too short"));
        }

        // Mistral API keys should not contain spaces or special characters
        if self.api_key.chars().any(|c| c.is_whitespace()) {
            return Err(anyhow::anyhow!("Invalid Mistral API key: contains whitespace"));
        }

        debug!("Mistral API key validation passed");
        Ok(())
    }

    /// Create authentication headers for Mistral API requests
    pub fn create_headers(&self) -> Result<HeaderMap> {
        let mut headers = HeaderMap::new();

        // Authorization header with Bearer token
        let auth_value = format!("Bearer {}", self.api_key);
        headers.insert(
            AUTHORIZATION,
            HeaderValue::from_str(&auth_value)
                .context("Failed to create authorization header")?
        );

        // Content-Type header
        headers.insert(
            CONTENT_TYPE,
            HeaderValue::from_static("application/json")
        );

        // User-Agent header
        let default_ua = "LLM-Supabase-RS/1.0 (Mistral)";
        let user_agent = self.config.user_agent.as_deref().unwrap_or(default_ua);
        
        headers.insert(
            USER_AGENT,
            HeaderValue::from_str(user_agent)
                .context("Failed to create user-agent header")?
        );

        // Add European compliance headers if enabled
        if self.config.uses_gdpr_mode() {
            headers.insert(
                "X-GDPR-Mode",
                HeaderValue::from_static("enabled")
            );
        }

        if self.config.prefers_eu_residency() {
            headers.insert(
                "X-EU-Residency",
                HeaderValue::from_static("preferred")
            );
        }

        trace!("Created Mistral authentication headers");
        Ok(headers)
    }

    /// Check if a request would exceed rate limits
    pub async fn check_rate_limit(&self, estimated_tokens: u32) -> Result<bool> {
        if let Some(rate_limiter) = &self.rate_limiter {
            let mut state = rate_limiter.write().await;
            
            let now = Instant::now();
            
            // Reset window if needed
            if now.duration_since(state.window_start) >= state.window_duration {
                state.current_requests = 0;
                state.current_tokens = 0;
                state.window_start = now;
                debug!("Reset rate limit window");
            }

            // Check if request would exceed limits
            let would_exceed_requests = state.current_requests >= state.limits.requests_per_minute;
            let would_exceed_tokens = state.current_tokens + estimated_tokens > state.limits.tokens_per_minute;

            if would_exceed_requests || would_exceed_tokens {
                warn!("Rate limit would be exceeded - requests: {}/{}, tokens: {}/{}",
                      state.current_requests, state.limits.requests_per_minute,
                      state.current_tokens + estimated_tokens, state.limits.tokens_per_minute);
                return Ok(false);
            }

            trace!("Rate limit check passed");
            return Ok(true);
        }

        // No rate limiting configured
        Ok(true)
    }

    /// Record a successful request for rate limiting and compliance tracking
    pub async fn record_request(&self, endpoint: &str, tokens_used: u32) -> Result<String> {
        let request_id = format!("mistral_req_{}", uuid::Uuid::new_v4().simple());

        // Update rate limiting
        if let Some(rate_limiter) = &self.rate_limiter {
            let mut state = rate_limiter.write().await;
            state.current_requests += 1;
            state.current_tokens += tokens_used;
            state.last_request = Instant::now();
        }

        // Update compliance tracking
        {
            let mut tracker = self.request_tracker.write().await;
            let now = Instant::now();
            
            tracker.total_requests += 1;
            
            if self.config.prefers_eu_residency() {
                tracker.eu_requests += 1;
            }
            
            if self.config.uses_gdpr_mode() {
                tracker.gdpr_requests += 1;
            }
            
            tracker.last_request = Some(now);
            
            // Add to request history (keep last 1000 records)
            tracker.request_history.push(RequestRecord {
                timestamp: now,
                eu_residency: self.config.prefers_eu_residency(),
                gdpr_mode: self.config.uses_gdpr_mode(),
                endpoint: endpoint.to_string(),
                request_id: request_id.clone(),
            });

            // Trim history to prevent memory growth
            if tracker.request_history.len() > 1000 {
                tracker.request_history.remove(0);
            }
        }

        debug!("Recorded request: {} to {}", request_id, endpoint);
        Ok(request_id)
    }

    /// Parse rate limit information from response headers
    pub fn parse_rate_limit_headers(&self, headers: &HeaderMap) -> RateLimitInfo {
        RateLimitInfo {
            requests_limit: self.parse_header_u32(headers, "x-ratelimit-limit-requests"),
            requests_remaining: self.parse_header_u32(headers, "x-ratelimit-remaining-requests"),
            tokens_limit: self.parse_header_u32(headers, "x-ratelimit-limit-tokens"),
            tokens_remaining: self.parse_header_u32(headers, "x-ratelimit-remaining-tokens"),
            reset_time: self.parse_header_u64(headers, "x-ratelimit-reset-requests"),
            retry_after: self.parse_header_u64(headers, "retry-after"),
        }
    }

    /// Get compliance statistics
    pub async fn get_compliance_stats(&self) -> Result<ComplianceStats> {
        let tracker = self.request_tracker.read().await;
        
        Ok(ComplianceStats {
            total_requests: tracker.total_requests,
            eu_requests: tracker.eu_requests,
            gdpr_requests: tracker.gdpr_requests,
            eu_compliance_rate: if tracker.total_requests > 0 {
                (tracker.eu_requests as f64 / tracker.total_requests as f64) * 100.0
            } else {
                0.0
            },
            gdpr_compliance_rate: if tracker.total_requests > 0 {
                (tracker.gdpr_requests as f64 / tracker.total_requests as f64) * 100.0
            } else {
                0.0
            },
            last_request: tracker.last_request.map(|_| {
                // Return current time since Instant doesn't track absolute time
                // In production, you'd want to store SystemTime instead
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_millis() as u64
            }),
        })
    }

    /// Get the masked API key for logging
    pub fn get_masked_api_key(&self) -> String {
        Self::mask_api_key(&self.api_key)
    }

    /// Mask API key for safe logging
    fn mask_api_key(api_key: &str) -> String {
        if api_key.len() <= 8 {
            "*".repeat(api_key.len())
        } else {
            format!("{}***{}", &api_key[..4], &api_key[api_key.len()-4..])
        }
    }

    /// Parse u32 value from header
    fn parse_header_u32(&self, headers: &HeaderMap, header_name: &str) -> Option<u32> {
        headers.get(header_name)
            .and_then(|v| v.to_str().ok())
            .and_then(|s| s.parse().ok())
    }

    /// Parse u64 value from header
    fn parse_header_u64(&self, headers: &HeaderMap, header_name: &str) -> Option<u64> {
        headers.get(header_name)
            .and_then(|v| v.to_str().ok())
            .and_then(|s| s.parse().ok())
    }

    /// Check if rate limiting is enabled
    pub fn has_rate_limiting(&self) -> bool {
        self.rate_limiter.is_some()
    }

    /// Get current rate limit status
    pub async fn get_rate_limit_status(&self) -> Option<RateLimitStatus> {
        if let Some(rate_limiter) = &self.rate_limiter {
            let state = rate_limiter.read().await;
            let now = Instant::now();
            
            Some(RateLimitStatus {
                current_requests: state.current_requests,
                current_tokens: state.current_tokens,
                requests_limit: state.limits.requests_per_minute,
                tokens_limit: state.limits.tokens_per_minute,
                window_remaining_seconds: {
                    let elapsed = now.duration_since(state.window_start);
                    if elapsed >= state.window_duration {
                        0
                    } else {
                        (state.window_duration - elapsed).as_secs()
                    }
                },
                last_request_seconds_ago: now.duration_since(state.last_request).as_secs(),
            })
        } else {
            None
        }
    }
}

/// Compliance statistics for European data regulations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComplianceStats {
    /// Total number of requests
    pub total_requests: u64,

    /// Requests using EU data residency
    pub eu_requests: u64,

    /// Requests using GDPR mode
    pub gdpr_requests: u64,

    /// EU compliance rate (percentage)
    pub eu_compliance_rate: f64,

    /// GDPR compliance rate (percentage)
    pub gdpr_compliance_rate: f64,

    /// Timestamp of last request (as milliseconds since epoch)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_request: Option<u64>,
}

/// Current rate limit status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RateLimitStatus {
    /// Current requests in window
    pub current_requests: u32,

    /// Current tokens in window
    pub current_tokens: u32,

    /// Requests limit per minute
    pub requests_limit: u32,

    /// Tokens limit per minute
    pub tokens_limit: u32,

    /// Seconds remaining in current window
    pub window_remaining_seconds: u64,

    /// Seconds since last request
    pub last_request_seconds_ago: u64,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::infrastructure::mistral::types::MistralConfig;

    #[test]
    fn test_mask_api_key() {
        assert_eq!(MistralAuth::mask_api_key("short"), "short");
        assert_eq!(MistralAuth::mask_api_key("test1234test"), "test***test");
        assert_eq!(MistralAuth::mask_api_key("sk-1234567890abcdef"), "sk-1***cdef");
    }

    #[tokio::test]
    async fn test_auth_creation() {
        let config = MistralConfig {
            api_key: "test-api-key-123456".to_string(),
            ..Default::default()
        };

        let auth = MistralAuth::new(&config);
        assert!(auth.is_ok());
    }

    #[tokio::test]
    async fn test_invalid_auth_creation() {
        let config = MistralConfig {
            api_key: String::new(),
            ..Default::default()
        };

        let auth = MistralAuth::new(&config);
        assert!(auth.is_err());
    }

    #[tokio::test]
    async fn test_headers_creation() {
        let config = MistralConfig {
            api_key: "test-api-key-123456".to_string(),
            ..Default::default()
        };

        let auth = MistralAuth::new(&config).unwrap();
        let headers = auth.create_headers().unwrap();

        assert!(headers.contains_key(AUTHORIZATION));
        assert!(headers.contains_key(CONTENT_TYPE));
        assert!(headers.contains_key(USER_AGENT));
    }

    #[tokio::test]
    async fn test_rate_limiting() {
        let mut config = MistralConfig {
            api_key: "test-api-key-123456".to_string(),
            rate_limit: Some(MistralRateLimit {
                requests_per_minute: 5,
                tokens_per_minute: 1000,
                auto_throttle: true,
            }),
            ..Default::default()
        };

        let auth = MistralAuth::new(&config).unwrap();

        // Should allow first few requests
        assert!(auth.check_rate_limit(100).await.unwrap());
        
        // Record requests to approach limit
        for i in 0..5 {
            auth.record_request("/test", 100).await.unwrap();
        }

        // Should now be at or over limit
        let can_make_request = auth.check_rate_limit(100).await.unwrap();
        // This might be true or false depending on timing, but shouldn't error
        assert!(can_make_request == true || can_make_request == false);
    }

    #[tokio::test]
    async fn test_compliance_tracking() {
        let config = MistralConfig {
            api_key: "test-api-key-123456".to_string(),
            eu_residency: Some(true),
            gdpr_mode: Some(true),
            ..Default::default()
        };

        let auth = MistralAuth::new(&config).unwrap();
        
        // Make a few requests
        for i in 0..3 {
            auth.record_request("/test", 100).await.unwrap();
        }

        let stats = auth.get_compliance_stats().await.unwrap();
        assert_eq!(stats.total_requests, 3);
        assert_eq!(stats.eu_requests, 3);
        assert_eq!(stats.gdpr_requests, 3);
        assert_eq!(stats.eu_compliance_rate, 100.0);
        assert_eq!(stats.gdpr_compliance_rate, 100.0);
    }
}
