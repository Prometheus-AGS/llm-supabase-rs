// src/config/providers.rs
//
// Provider configuration types for AI service providers

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Configuration for AI service providers
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProvidersConfig {
    /// Vertex AI provider configuration
    pub vertex_ai: VertexAiConfig,

    /// Provider selection and routing configuration
    pub routing: ProviderRoutingConfig,

    /// Global provider settings
    pub global: GlobalProviderConfig,
}

/// Vertex AI specific configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VertexAiConfig {
    /// GCP project ID
    pub project_id: String,

    /// GCP region for Vertex AI API
    pub region: String,

    /// Service account key file path
    pub credentials_path: String,

    /// Default model configuration
    pub default_model: VertexModelConfig,

    /// Model-specific configurations
    pub models: HashMap<String, VertexModelConfig>,

    /// API endpoint configuration
    pub endpoint: VertexEndpointConfig,

    /// Authentication configuration
    pub auth: VertexAuthConfig,
}

/// Configuration for a specific Vertex AI model
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VertexModelConfig {
    /// Vertex AI model name (e.g., "claude-4-sonnet-20250514")
    pub model_name: String,

    /// Model version or endpoint ID
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,

    /// Default generation parameters
    pub default_parameters: VertexModelParameters,

    /// Request timeout in seconds
    pub timeout_seconds: u64,

    /// Maximum retries for failed requests
    pub max_retries: u32,

    /// Rate limiting configuration
    pub rate_limit: RateLimitConfig,
}

/// Default parameters for Vertex AI model generation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VertexModelParameters {
    /// Default temperature (0.0 to 1.0)
    pub temperature: f64,

    /// Default top_p (0.0 to 1.0)
    pub top_p: f64,

    /// Default top_k (1 to 40)
    pub top_k: u32,

    /// Default max output tokens
    pub max_output_tokens: u32,

    /// Default stop sequences
    pub stop_sequences: Vec<String>,
}

/// Vertex AI API endpoint configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VertexEndpointConfig {
    /// Base API URL (defaults to Vertex AI API)
    pub base_url: String,

    /// API version
    pub api_version: String,

    /// Custom endpoint path template
    #[serde(skip_serializing_if = "Option::is_none")]
    pub endpoint_template: Option<String>,

    /// Connection timeout in seconds
    pub connection_timeout: u64,

    /// Request timeout in seconds
    pub request_timeout: u64,
}

/// Vertex AI authentication configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VertexAuthConfig {
    /// Authentication method
    pub auth_method: VertexAuthMethod,

    /// Token refresh interval in seconds
    pub token_refresh_interval: u64,

    /// OAuth scopes required
    pub scopes: Vec<String>,
}

/// Vertex AI authentication methods
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VertexAuthMethod {
    /// Use service account key file
    ServiceAccountKey,

    /// Use application default credentials
    ApplicationDefault,

    /// Use workload identity (for GKE)
    WorkloadIdentity,
}

/// Provider routing and selection configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderRoutingConfig {
    /// Default provider for unknown models
    pub default_provider: String,

    /// Model to provider mapping
    pub model_mapping: HashMap<String, String>,

    /// Fallback configuration
    pub fallback: FallbackConfig,

    /// Load balancing configuration
    pub load_balancing: LoadBalancingConfig,
}

/// Fallback configuration when primary provider fails
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FallbackConfig {
    /// Enable fallback to alternative providers
    pub enabled: bool,

    /// Maximum fallback attempts
    pub max_attempts: u32,

    /// Fallback provider chain (in order of preference)
    pub provider_chain: Vec<String>,

    /// Conditions that trigger fallback
    pub trigger_conditions: Vec<FallbackTrigger>,
}

/// Conditions that trigger fallback to another provider
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FallbackTrigger {
    /// HTTP timeout
    Timeout,

    /// HTTP 5xx errors
    ServerError,

    /// Rate limit exceeded (429)
    RateLimit,

    /// Authentication failures
    AuthError,

    /// Network connectivity issues
    NetworkError,
}

/// Load balancing configuration for multiple provider instances
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoadBalancingConfig {
    /// Load balancing strategy
    pub strategy: LoadBalancingStrategy,

    /// Health check configuration
    pub health_check: HealthCheckConfig,

    /// Circuit breaker configuration
    pub circuit_breaker: CircuitBreakerConfig,
}

/// Load balancing strategies
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LoadBalancingStrategy {
    /// Round robin distribution
    RoundRobin,

    /// Least connections
    LeastConnections,

    /// Weighted round robin
    WeightedRoundRobin,

    /// Random selection
    Random,
}

/// Health check configuration for providers
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthCheckConfig {
    /// Enable health checks
    pub enabled: bool,

    /// Health check interval in seconds
    pub interval_seconds: u64,

    /// Health check timeout in seconds
    pub timeout_seconds: u64,

    /// Number of consecutive failures before marking unhealthy
    pub failure_threshold: u32,

    /// Number of consecutive successes before marking healthy
    pub success_threshold: u32,
}

/// Circuit breaker configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CircuitBreakerConfig {
    /// Enable circuit breaker
    pub enabled: bool,

    /// Failure rate threshold (0.0 to 1.0)
    pub failure_rate_threshold: f64,

    /// Minimum number of requests before circuit breaker can trip
    pub minimum_throughput: u32,

    /// Circuit breaker open duration in seconds
    pub open_duration_seconds: u64,

    /// Half-open state max requests
    pub half_open_max_requests: u32,
}

/// Rate limiting configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RateLimitConfig {
    /// Enable rate limiting
    pub enabled: bool,

    /// Requests per minute limit
    pub requests_per_minute: u32,

    /// Burst capacity
    pub burst_capacity: u32,

    /// Rate limit strategy
    pub strategy: RateLimitStrategy,
}

/// Rate limiting strategies
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RateLimitStrategy {
    /// Token bucket algorithm
    TokenBucket,

    /// Sliding window
    SlidingWindow,

    /// Fixed window
    FixedWindow,
}

/// Global provider configuration settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GlobalProviderConfig {
    /// Default request timeout in seconds
    pub default_timeout: u64,

    /// Default max retries
    pub default_max_retries: u32,

    /// Enable metrics collection
    pub metrics_enabled: bool,

    /// Enable distributed tracing
    pub tracing_enabled: bool,

    /// Log level for provider operations
    pub log_level: String,

    /// Enable request/response logging (be careful with sensitive data)
    pub log_requests: bool,
}

impl Default for ProvidersConfig {
    fn default() -> Self {
        Self {
            vertex_ai: VertexAiConfig::default(),
            routing: ProviderRoutingConfig::default(),
            global: GlobalProviderConfig::default(),
        }
    }
}

impl Default for VertexAiConfig {
    fn default() -> Self {
        Self {
            project_id: std::env::var("GCP_PROJECT_ID")
                .unwrap_or_else(|_| "your-gcp-project".to_string()),
            region: std::env::var("GCP_LOCATION")
                .unwrap_or_else(|_| "us-east5".to_string()),
            credentials_path: std::env::var("GOOGLE_APPLICATION_CREDENTIALS")
                .unwrap_or_else(|_| "./gcp-credentials.json".to_string()),
            default_model: VertexModelConfig::default(),
            models: HashMap::new(),
            endpoint: VertexEndpointConfig::default(),
            auth: VertexAuthConfig::default(),
        }
    }
}

impl Default for VertexModelConfig {
    fn default() -> Self {
        Self {
            model_name: std::env::var("DEFAULT_MODEL")
                .unwrap_or_else(|_| "claude-sonnet-4-5@20250929".to_string()),
            version: None,
            default_parameters: VertexModelParameters::default(),
            timeout_seconds: 30,
            max_retries: 3,
            rate_limit: RateLimitConfig::default(),
        }
    }
}

impl Default for VertexModelParameters {
    fn default() -> Self {
        Self {
            temperature: 0.7,
            top_p: 0.9,
            top_k: 20,
            max_output_tokens: 4000,
            stop_sequences: vec![],
        }
    }
}

impl Default for VertexEndpointConfig {
    fn default() -> Self {
        Self {
            base_url: "https://aiplatform.googleapis.com".to_string(),
            api_version: "v1".to_string(),
            endpoint_template: None,
            connection_timeout: 10,
            request_timeout: 30,
        }
    }
}

impl Default for VertexAuthConfig {
    fn default() -> Self {
        Self {
            auth_method: VertexAuthMethod::ServiceAccountKey,
            token_refresh_interval: 3300, // 55 minutes (tokens expire in 1 hour)
            scopes: vec![
                "https://www.googleapis.com/auth/cloud-platform".to_string(),
                "https://www.googleapis.com/auth/cloud-platform.read-only".to_string(),
            ],
        }
    }
}

impl Default for ProviderRoutingConfig {
    fn default() -> Self {
        let mut model_mapping = HashMap::new();
        model_mapping.insert("claude-sonnet-4-5@20250929".to_string(), "vertex_ai".to_string());
        model_mapping.insert("claude-3-5-haiku@20241022".to_string(), "vertex_ai".to_string()); // Backward compatibility

        Self {
            default_provider: "vertex_ai".to_string(),
            model_mapping,
            fallback: FallbackConfig::default(),
            load_balancing: LoadBalancingConfig::default(),
        }
    }
}

impl Default for FallbackConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            max_attempts: 2,
            provider_chain: vec!["vertex_ai".to_string()],
            trigger_conditions: vec![
                FallbackTrigger::Timeout,
                FallbackTrigger::ServerError,
                FallbackTrigger::NetworkError,
            ],
        }
    }
}

impl Default for LoadBalancingConfig {
    fn default() -> Self {
        Self {
            strategy: LoadBalancingStrategy::RoundRobin,
            health_check: HealthCheckConfig::default(),
            circuit_breaker: CircuitBreakerConfig::default(),
        }
    }
}

impl Default for HealthCheckConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            interval_seconds: 30,
            timeout_seconds: 5,
            failure_threshold: 3,
            success_threshold: 2,
        }
    }
}

impl Default for CircuitBreakerConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            failure_rate_threshold: 0.5,
            minimum_throughput: 10,
            open_duration_seconds: 60,
            half_open_max_requests: 3,
        }
    }
}

impl Default for RateLimitConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            requests_per_minute: 60,
            burst_capacity: 10,
            strategy: RateLimitStrategy::TokenBucket,
        }
    }
}

impl Default for GlobalProviderConfig {
    fn default() -> Self {
        Self {
            default_timeout: 30,
            default_max_retries: 3,
            metrics_enabled: true,
            tracing_enabled: true,
            log_level: "info".to_string(),
            log_requests: false,
        }
    }
}

impl ProvidersConfig {
    /// Create a new providers configuration with Vertex AI
    pub fn new() -> Self {
        Self::default()
    }

    /// Get model configuration for a specific model
    pub fn get_model_config(&self, model_name: &str) -> Option<&VertexModelConfig> {
        self.vertex_ai.models.get(model_name)
            .or_else(|| Some(&self.vertex_ai.default_model))
    }

    /// Get the provider name for a specific model
    pub fn get_provider_for_model(&self, model_name: &str) -> &str {
        self.routing.model_mapping.get(model_name)
            .map(|s| s.as_str())
            .unwrap_or(&self.routing.default_provider)
    }

    /// Check if a model is supported
    pub fn is_model_supported(&self, model_name: &str) -> bool {
        self.routing.model_mapping.contains_key(model_name) ||
        self.vertex_ai.models.contains_key(model_name) ||
        model_name == self.vertex_ai.default_model.model_name
    }

    /// Get all supported model names
    pub fn supported_models(&self) -> Vec<String> {
        let mut models = Vec::new();

        // Add models from routing configuration
        models.extend(self.routing.model_mapping.keys().cloned());

        // Add models from Vertex AI configuration
        models.extend(self.vertex_ai.models.keys().cloned());

        // Add default model
        models.push(self.vertex_ai.default_model.model_name.clone());

        // Deduplicate and sort
        models.sort();
        models.dedup();
        models
    }
}

impl VertexAiConfig {
    /// Get region-specific base URL for Vertex AI
    fn get_regional_base_url(&self) -> String {
        // Vertex AI requires region in the URL subdomain
        format!("https://{}-aiplatform.googleapis.com", self.region)
    }

    /// Get the full Vertex AI endpoint URL for predictions
    pub fn prediction_endpoint(&self, model_name: &str) -> String {
        // Normalize model name for Vertex AI endpoint
        let normalized_model = self.normalize_model_name(model_name);
        
        let default_template = format!(
            "{}/v1/projects/{}/locations/{}/publishers/anthropic/models/{}:streamRawPredict",
            self.get_regional_base_url(),
            self.project_id,
            self.region,
            "{model}"
        );

        let template = self.endpoint.endpoint_template.as_ref()
            .unwrap_or(&default_template);

        template.replace("{model}", &normalized_model)
            .replace("{project}", &self.project_id)
            .replace("{region}", &self.region)
    }

    /// Get the streaming endpoint URL
    pub fn streaming_endpoint(&self, model_name: &str) -> String {
        // Normalize model name for Vertex AI endpoint
        let normalized_model = self.normalize_model_name(model_name);
        
        let endpoint = format!(
            "{}/v1/projects/{}/locations/{}/publishers/anthropic/models/{}:streamRawPredict",
            self.get_regional_base_url(),
            self.project_id,
            self.region,
            normalized_model
        );
        
        endpoint
    }

    /// Normalize model name for Vertex AI API endpoints
    fn normalize_model_name(&self, model_name: &str) -> String {
        match model_name {
            // Map common model names to their Vertex AI equivalents
            "claude-4-sonnet" => "claude-sonnet-4-5@20250929".to_string(),
            "claude-3-5-haiku" => "claude-3-5-haiku@20241022".to_string(),
            "claude-4-5-sonnet" => "claude-sonnet-4-5@20250929".to_string(),
            "claude-4-1-opus" => "claude-opus-4-1@20250805".to_string(), // Fallback to available model
            _ => {
                // If model contains @ use as-is, otherwise try to convert
                if model_name.contains('@') {
                    model_name.to_string()
                } else {
                    // Default fallback
                    "claude-3-5-haiku@20241022".to_string()
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_providers_config_default() {
        let config = ProvidersConfig::default();

        assert_eq!(config.vertex_ai.default_model.model_name, "claude-sonnet-4-5@20250929");
        assert_eq!(config.routing.default_provider, "vertex_ai");
        assert!(config.global.metrics_enabled);
    }

    #[test]
    fn test_model_support_detection() {
        let config = ProvidersConfig::default();

        assert!(config.is_model_supported("claude-sonnet-4-5@20250929"));
        assert!(!config.is_model_supported("unsupported-model"));
    }

    #[test]
    fn test_provider_routing() {
        let config = ProvidersConfig::default();

        let provider = config.get_provider_for_model("claude-sonnet-4-5@20250929");
        assert_eq!(provider, "vertex_ai");

        let unknown_provider = config.get_provider_for_model("unknown-model");
        assert_eq!(unknown_provider, "vertex_ai"); // Should use default
    }

    #[test]
    fn test_vertex_ai_endpoints() {
        let config = VertexAiConfig {
            project_id: "prometheus-461323".to_string(),
            region: "us-east5".to_string(),
            ..Default::default()
        };

        let endpoint = config.prediction_endpoint("claude-sonnet-4-5@20250929");
        assert!(endpoint.contains("prometheus-461323"));
        assert!(endpoint.contains("us-east5"));
        assert!(endpoint.contains("claude-4-sonnet"));
        assert!(endpoint.contains(":streamRawPredict"));

        let streaming_endpoint = config.streaming_endpoint("claude-sonnet-4-5@20250929");
        assert!(streaming_endpoint.contains(":streamRawPredict"));
    }

    #[test]
    fn test_model_configuration() {
        let mut config = ProvidersConfig::default();

        // Add a custom model configuration
        let custom_model = VertexModelConfig {
            model_name: "custom-claude".to_string(),
            timeout_seconds: 60,
            ..Default::default()
        };

        config.vertex_ai.models.insert("custom-claude".to_string(), custom_model);

        let model_config = config.get_model_config("custom-claude");
        assert!(model_config.is_some());
        assert_eq!(model_config.unwrap().timeout_seconds, 60);

        // Should fall back to default for unknown models
        let default_config = config.get_model_config("unknown-model");
        assert!(default_config.is_some());
        assert_eq!(default_config.unwrap().model_name, "claude-sonnet-4-5@20250929");
    }

    #[test]
    fn test_supported_models_list() {
        let config = ProvidersConfig::default();
        let models = config.supported_models();

        assert!(models.contains(&"claude-sonnet-4-5@20250929".to_string()));
        assert!(!models.is_empty());
    }

    #[test]
    fn test_serialization() {
        let config = ProvidersConfig::default();
        let json = serde_json::to_string(&config).unwrap();

        // Should contain key configuration fields
        assert!(json.contains("vertex_ai"));
        assert!(json.contains("claude-sonnet-4-5@20250929"));
        assert!(json.contains("routing"));

        // Should be deserializable
        let deserialized: ProvidersConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.vertex_ai.default_model.model_name, "claude-sonnet-4-5@20250929");
    }

    #[test]
    fn test_auth_methods() {
        let auth_config = VertexAuthConfig::default();
        assert!(matches!(auth_config.auth_method, VertexAuthMethod::ServiceAccountKey));
        assert!(auth_config.scopes.len() >= 2);
    }

    #[test]
    fn test_rate_limiting_config() {
        let rate_limit = RateLimitConfig::default();
        assert!(rate_limit.enabled);
        assert_eq!(rate_limit.requests_per_minute, 60);
        assert!(matches!(rate_limit.strategy, RateLimitStrategy::TokenBucket));
    }

    #[test]
    fn test_fallback_configuration() {
        let fallback = FallbackConfig::default();
        assert!(fallback.enabled);
        assert!(!fallback.provider_chain.is_empty());
        assert!(!fallback.trigger_conditions.is_empty());
    }
}
