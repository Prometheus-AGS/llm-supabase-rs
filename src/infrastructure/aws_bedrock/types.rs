//! AWS Bedrock specific types and data structures
//!
//! This module contains all AWS Bedrock-specific types, configurations,
//! and model definitions for the Bedrock provider implementation.

use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::env;

/// AWS Bedrock provider configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BedrockConfig {
    /// AWS region (e.g., "us-east-1")
    pub region: String,
    
    /// AWS access key ID (optional, can use default credential chain)
    pub access_key: Option<String>,
    
    /// AWS secret access key (optional, can use default credential chain)
    pub secret_key: Option<String>,
    
    /// AWS session token for temporary credentials (optional)
    pub session_token: Option<String>,
    
    /// AWS profile name (optional)
    pub profile: Option<String>,
    
    /// Custom endpoint URL (for VPC endpoints, optional)
    pub endpoint_url: Option<String>,
    
    /// Default model configuration
    pub default_model: BedrockModelConfig,
    
    /// Model-specific configurations
    pub models: HashMap<String, BedrockModelConfig>,
    
    /// API configuration
    pub api: BedrockApiConfig,
    
    /// Authentication method preference
    pub auth_method: BedrockAuthMethod,
    
    /// Timeout settings
    pub timeouts: BedrockTimeoutConfig,
    
    /// Whether to use AWS STS for credential validation
    pub validate_credentials: bool,
}

/// AWS Bedrock authentication method preference
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BedrockAuthMethod {
    /// Use environment variables (AWS_ACCESS_KEY_ID, AWS_SECRET_ACCESS_KEY)
    EnvironmentVariables,
    
    /// Use AWS credentials file (~/.aws/credentials)
    CredentialsFile,
    
    /// Use IAM roles for EC2 instances
    InstanceProfile,
    
    /// Use IAM roles for ECS tasks
    EcsTaskRole,
    
    /// Use IAM roles for Lambda functions
    LambdaRole,
    
    /// Use explicit access key and secret
    Explicit,
    
    /// Use AWS SDK default credential chain (recommended)
    Default,
}

/// Configuration for a specific Bedrock model
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BedrockModelConfig {
    /// Bedrock model identifier (e.g., "anthropic.claude-3-5-sonnet-20241022-v2:0")
    pub model_id: String,
    
    /// Model family (anthropic, meta, mistral, amazon, cohere)
    pub model_family: BedrockModelFamily,
    
    /// Whether this model supports tool calling
    pub supports_tools: bool,
    
    /// Whether this model supports streaming
    pub supports_streaming: bool,
    
    /// Maximum context length for this model
    pub max_context_length: u32,
    
    /// Maximum output tokens for this model
    pub max_output_tokens: u32,
    
    /// Default generation parameters
    pub default_parameters: BedrockModelParameters,
    
    /// Request timeout in seconds
    pub timeout_seconds: u64,
    
    /// Maximum retries for failed requests
    pub max_retries: u32,
    
    /// Rate limiting configuration
    pub rate_limit: BedrockRateLimitConfig,
}

/// Bedrock model family enumeration
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum BedrockModelFamily {
    /// Anthropic Claude models
    Anthropic,
    
    /// Meta Llama models
    Meta,
    
    /// Mistral AI models
    Mistral,
    
    /// Amazon Titan models
    Amazon,
    
    /// Cohere Command models
    Cohere,
    
    /// AI21 Labs Jurassic models
    Ai21,
    
    /// Stability AI models
    Stability,
}

/// Default parameters for Bedrock model generation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BedrockModelParameters {
    /// Temperature for randomness (0.0 to 1.0)
    pub temperature: Option<f64>,
    
    /// Top-p for nucleus sampling (0.0 to 1.0)
    pub top_p: Option<f64>,
    
    /// Top-k for top-k sampling
    pub top_k: Option<u32>,
    
    /// Maximum tokens to generate
    pub max_tokens: Option<u32>,
    
    /// Stop sequences
    pub stop_sequences: Vec<String>,
    
    /// Model-specific parameters (varies by family)
    pub model_specific: HashMap<String, serde_json::Value>,
}

/// Bedrock API configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BedrockApiConfig {
    /// Connection timeout in seconds
    pub connection_timeout: u64,
    
    /// Request timeout in seconds
    pub request_timeout: u64,
    
    /// Maximum retries
    pub max_retries: u32,
    
    /// Rate limit requests per minute (varies by model)
    pub rate_limit_requests_per_minute: Option<u32>,
    
    /// Exponential backoff for retries
    pub exponential_backoff: bool,
    
    /// Base delay between retries in milliseconds
    pub retry_delay_ms: u64,
    
    /// Maximum retry delay in milliseconds
    pub max_retry_delay_ms: u64,
    
    /// User agent for requests
    pub user_agent: String,
    
    /// Additional headers to include in requests
    pub additional_headers: HashMap<String, String>,
}

/// Rate limiting configuration for Bedrock
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BedrockRateLimitConfig {
    /// Requests per minute limit
    pub requests_per_minute: Option<u32>,
    
    /// Tokens per minute limit
    pub tokens_per_minute: Option<u32>,
    
    /// Burst allowance
    pub burst_limit: Option<u32>,
    
    /// Rate limiting strategy
    pub strategy: BedrockRateLimitStrategy,
}

/// Rate limiting strategy
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BedrockRateLimitStrategy {
    /// No rate limiting
    None,
    
    /// Token bucket algorithm
    TokenBucket,
    
    /// Fixed window
    FixedWindow,
    
    /// Sliding window
    SlidingWindow,
}

/// Timeout configuration for Bedrock
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BedrockTimeoutConfig {
    /// Connection timeout in seconds
    pub connect: u64,
    
    /// Read timeout in seconds
    pub read: u64,
    
    /// Write timeout in seconds
    pub write: u64,
    
    /// Total request timeout in seconds
    pub total: u64,
}

/// AWS region enumeration for Bedrock
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum BedrockRegion {
    /// US East (N. Virginia)
    UsEast1,
    
    /// US West (Oregon)
    UsWest2,
    
    /// Europe (Ireland)
    EuWest1,
    
    /// Asia Pacific (Tokyo)
    ApNortheast1,
    
    /// Custom region
    Custom(String),
}

impl BedrockRegion {
    /// Convert region to AWS region string
    pub fn as_str(&self) -> &str {
        match self {
            BedrockRegion::UsEast1 => "us-east-1",
            BedrockRegion::UsWest2 => "us-west-2",
            BedrockRegion::EuWest1 => "eu-west-1",
            BedrockRegion::ApNortheast1 => "ap-northeast-1",
            BedrockRegion::Custom(region) => region.as_str(),
        }
    }
    
    /// Parse region from string
    pub fn from_str(s: &str) -> Self {
        match s {
            "us-east-1" => BedrockRegion::UsEast1,
            "us-west-2" => BedrockRegion::UsWest2,
            "eu-west-1" => BedrockRegion::EuWest1,
            "ap-northeast-1" => BedrockRegion::ApNortheast1,
            custom => BedrockRegion::Custom(custom.to_string()),
        }
    }
}

/// Bedrock model identifier with family and capabilities
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BedrockModel {
    /// Model ID as used in Bedrock API
    pub id: String,
    
    /// Human-readable model name
    pub name: String,
    
    /// Model family
    pub family: BedrockModelFamily,
    
    /// Model version
    pub version: Option<String>,
    
    /// Whether model supports tool calling
    pub supports_tools: bool,
    
    /// Whether model supports streaming
    pub supports_streaming: bool,
    
    /// Maximum context length
    pub max_context_length: u32,
    
    /// Maximum output tokens
    pub max_output_tokens: u32,
    
    /// Model-specific parameters schema
    pub parameter_schema: HashMap<String, serde_json::Value>,
}

/// Bedrock API request structure (generic)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BedrockRequest {
    /// Model ID
    pub model_id: String,
    
    /// Request body (model-specific format)
    pub body: serde_json::Value,
    
    /// Content type (usually "application/json")
    pub content_type: String,
    
    /// Accept header for response
    pub accept: String,
}

/// Bedrock API response structure (generic)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BedrockResponse {
    /// Response body
    pub body: serde_json::Value,
    
    /// Content type of response
    pub content_type: String,
    
    /// Response metadata
    pub metadata: HashMap<String, String>,
}

/// Bedrock streaming event types
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BedrockStreamEvent {
    /// Content chunk event
    ContentBlockStart,
    
    /// Content delta event
    ContentBlockDelta,
    
    /// Content block stop event
    ContentBlockStop,
    
    /// Message start event
    MessageStart,
    
    /// Message delta event
    MessageDelta,
    
    /// Message stop event
    MessageStop,
    
    /// Error event
    Error,
    
    /// Unknown event type
    Unknown(String),
}

/// Error types specific to Bedrock
#[derive(Debug, thiserror::Error)]
pub enum BedrockError {
    #[error("Authentication failed: {0}")]
    AuthenticationError(String),
    
    #[error("Invalid model: {0}")]
    InvalidModel(String),
    
    #[error("Rate limit exceeded: {0}")]
    RateLimitExceeded(String),
    
    #[error("Request timeout: {0}")]
    Timeout(String),
    
    #[error("AWS API error: {0}")]
    AwsApiError(String),
    
    #[error("Streaming error: {0}")]
    StreamingError(String),
    
    #[error("Tool calling not supported for model: {0}")]
    ToolCallingNotSupported(String),
    
    #[error("Configuration error: {0}")]
    ConfigurationError(String),
}

impl Default for BedrockConfig {
    fn default() -> Self {
        Self {
            region: "us-east-1".to_string(),
            access_key: None,
            secret_key: None,
            session_token: None,
            profile: None,
            endpoint_url: None,
            default_model: BedrockModelConfig::default(),
            models: HashMap::new(),
            api: BedrockApiConfig::default(),
            auth_method: BedrockAuthMethod::Default,
            timeouts: BedrockTimeoutConfig::default(),
            validate_credentials: true,
        }
    }
}

impl Default for BedrockModelConfig {
    fn default() -> Self {
        Self {
            model_id: "anthropic.claude-3-5-sonnet-20241022-v2:0".to_string(),
            model_family: BedrockModelFamily::Anthropic,
            supports_tools: true,
            supports_streaming: true,
            max_context_length: 200000,
            max_output_tokens: 4096,
            default_parameters: BedrockModelParameters::default(),
            timeout_seconds: 60,
            max_retries: 3,
            rate_limit: BedrockRateLimitConfig::default(),
        }
    }
}

impl Default for BedrockModelParameters {
    fn default() -> Self {
        Self {
            temperature: Some(0.7),
            top_p: Some(0.9),
            top_k: None,
            max_tokens: Some(4096),
            stop_sequences: vec![],
            model_specific: HashMap::new(),
        }
    }
}

impl Default for BedrockApiConfig {
    fn default() -> Self {
        Self {
            connection_timeout: 30,
            request_timeout: 120,
            max_retries: 3,
            rate_limit_requests_per_minute: None,
            exponential_backoff: true,
            retry_delay_ms: 1000,
            max_retry_delay_ms: 60000,
            user_agent: "llm-supabase-rs/1.0.0".to_string(),
            additional_headers: HashMap::new(),
        }
    }
}

impl Default for BedrockRateLimitConfig {
    fn default() -> Self {
        Self {
            requests_per_minute: None,
            tokens_per_minute: None,
            burst_limit: None,
            strategy: BedrockRateLimitStrategy::None,
        }
    }
}

impl Default for BedrockTimeoutConfig {
    fn default() -> Self {
        Self {
            connect: 30,
            read: 120,
            write: 30,
            total: 300,
        }
    }
}

impl BedrockConfig {
    /// Create configuration from environment variables
    pub fn from_env() -> Result<Self> {
        let region = env::var("AWS_REGION")
            .or_else(|_| env::var("AWS_DEFAULT_REGION"))
            .unwrap_or_else(|_| "us-east-1".to_string());
            
        let access_key = env::var("AWS_ACCESS_KEY_ID").ok();
        let secret_key = env::var("AWS_SECRET_ACCESS_KEY").ok();
        let session_token = env::var("AWS_SESSION_TOKEN").ok();
        let profile = env::var("AWS_PROFILE").ok();
        
        // Validate that if access key is provided, secret key is also provided
        if access_key.is_some() && secret_key.is_none() {
            return Err(anyhow!("AWS_SECRET_ACCESS_KEY must be provided if AWS_ACCESS_KEY_ID is set"));
        }
        
        if access_key.is_none() && secret_key.is_some() {
            return Err(anyhow!("AWS_ACCESS_KEY_ID must be provided if AWS_SECRET_ACCESS_KEY is set"));
        }
        
        Ok(Self {
            region,
            access_key,
            secret_key,
            session_token,
            profile,
            ..Default::default()
        })
    }
    
    /// Get the Bedrock endpoint URL for the configured region
    pub fn get_endpoint_url(&self) -> String {
        if let Some(endpoint) = &self.endpoint_url {
            endpoint.clone()
        } else {
            format!("https://bedrock-runtime.{}.amazonaws.com", self.region)
        }
    }
    
    /// Get model configuration for a specific model
    pub fn get_model_config(&self, model_id: &str) -> &BedrockModelConfig {
        self.models.get(model_id).unwrap_or(&self.default_model)
    }
    
    /// Check if a model supports tool calling
    pub fn model_supports_tools(&self, model_id: &str) -> bool {
        self.get_model_config(model_id).supports_tools
    }
    
    /// Check if a model supports streaming
    pub fn model_supports_streaming(&self, model_id: &str) -> bool {
        self.get_model_config(model_id).supports_streaming
    }
    
    /// Get model family for a model ID
    pub fn get_model_family(&self, model_id: &str) -> BedrockModelFamily {
        if model_id.starts_with("anthropic.") {
            BedrockModelFamily::Anthropic
        } else if model_id.starts_with("meta.") {
            BedrockModelFamily::Meta
        } else if model_id.starts_with("mistral.") {
            BedrockModelFamily::Mistral
        } else if model_id.starts_with("amazon.") {
            BedrockModelFamily::Amazon
        } else if model_id.starts_with("cohere.") {
            BedrockModelFamily::Cohere
        } else if model_id.starts_with("ai21.") {
            BedrockModelFamily::Ai21
        } else if model_id.starts_with("stability.") {
            BedrockModelFamily::Stability
        } else {
            // Default to the configured model family
            self.get_model_config(model_id).model_family.clone()
        }
    }
}

/// Predefined Bedrock models with their capabilities
impl BedrockModel {
    /// Get all supported Bedrock models
    pub fn all_supported() -> Vec<BedrockModel> {
        vec![
            BedrockModel {
                id: "anthropic.claude-3-5-sonnet-20241022-v2:0".to_string(),
                name: "Claude 3.5 Sonnet v2".to_string(),
                family: BedrockModelFamily::Anthropic,
                version: Some("v2:0".to_string()),
                supports_tools: true,
                supports_streaming: true,
                max_context_length: 200000,
                max_output_tokens: 4096,
                parameter_schema: HashMap::new(),
            },
            BedrockModel {
                id: "anthropic.claude-3-haiku-20240307-v1:0".to_string(),
                name: "Claude 3 Haiku".to_string(),
                family: BedrockModelFamily::Anthropic,
                version: Some("v1:0".to_string()),
                supports_tools: true,
                supports_streaming: true,
                max_context_length: 200000,
                max_output_tokens: 4096,
                parameter_schema: HashMap::new(),
            },
            BedrockModel {
                id: "anthropic.claude-3-opus-20240229-v1:0".to_string(),
                name: "Claude 3 Opus".to_string(),
                family: BedrockModelFamily::Anthropic,
                version: Some("v1:0".to_string()),
                supports_tools: true,
                supports_streaming: true,
                max_context_length: 200000,
                max_output_tokens: 4096,
                parameter_schema: HashMap::new(),
            },
            BedrockModel {
                id: "meta.llama3-1-70b-instruct-v1:0".to_string(),
                name: "Llama 3.1 70B Instruct".to_string(),
                family: BedrockModelFamily::Meta,
                version: Some("v1:0".to_string()),
                supports_tools: true,
                supports_streaming: true,
                max_context_length: 128000,
                max_output_tokens: 2048,
                parameter_schema: HashMap::new(),
            },
            BedrockModel {
                id: "meta.llama3-1-8b-instruct-v1:0".to_string(),
                name: "Llama 3.1 8B Instruct".to_string(),
                family: BedrockModelFamily::Meta,
                version: Some("v1:0".to_string()),
                supports_tools: true,
                supports_streaming: true,
                max_context_length: 128000,
                max_output_tokens: 2048,
                parameter_schema: HashMap::new(),
            },
            BedrockModel {
                id: "mistral.mistral-7b-instruct-v0:2".to_string(),
                name: "Mistral 7B Instruct".to_string(),
                family: BedrockModelFamily::Mistral,
                version: Some("v0:2".to_string()),
                supports_tools: false,
                supports_streaming: true,
                max_context_length: 32000,
                max_output_tokens: 2048,
                parameter_schema: HashMap::new(),
            },
        ]
    }
    
    /// Find a model by ID
    pub fn find_by_id(model_id: &str) -> Option<BedrockModel> {
        Self::all_supported().into_iter().find(|m| m.id == model_id)
    }
    
    /// Get models by family
    pub fn by_family(family: BedrockModelFamily) -> Vec<BedrockModel> {
        Self::all_supported().into_iter().filter(|m| m.family == family).collect()
    }
    
    /// Get models that support tool calling
    pub fn tool_capable() -> Vec<BedrockModel> {
        Self::all_supported().into_iter().filter(|m| m.supports_tools).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_defaults() {
        let config = BedrockConfig::default();
        assert_eq!(config.region, "us-east-1");
        assert_eq!(config.auth_method, BedrockAuthMethod::Default);
        assert!(config.validate_credentials);
    }

    #[test]
    fn test_region_conversion() {
        assert_eq!(BedrockRegion::UsEast1.as_str(), "us-east-1");
        assert_eq!(BedrockRegion::from_str("us-west-2"), BedrockRegion::UsWest2);
        
        let custom = BedrockRegion::from_str("eu-central-1");
        if let BedrockRegion::Custom(region) = custom {
            assert_eq!(region, "eu-central-1");
        } else {
            panic!("Expected custom region");
        }
    }

    #[test]
    fn test_model_family_detection() {
        let config = BedrockConfig::default();
        
        assert_eq!(
            config.get_model_family("anthropic.claude-3-5-sonnet-20241022-v2:0"),
            BedrockModelFamily::Anthropic
        );
        assert_eq!(
            config.get_model_family("meta.llama3-1-70b-instruct-v1:0"),
            BedrockModelFamily::Meta
        );
        assert_eq!(
            config.get_model_family("mistral.mistral-7b-instruct-v0:2"),
            BedrockModelFamily::Mistral
        );
    }

    #[test]
    fn test_endpoint_url_generation() {
        let config = BedrockConfig {
            region: "us-west-2".to_string(),
            ..Default::default()
        };
        
        assert_eq!(
            config.get_endpoint_url(),
            "https://bedrock-runtime.us-west-2.amazonaws.com"
        );
        
        let custom_config = BedrockConfig {
            endpoint_url: Some("https://my-vpc-endpoint.com".to_string()),
            ..Default::default()
        };
        
        assert_eq!(
            custom_config.get_endpoint_url(),
            "https://my-vpc-endpoint.com"
        );
    }

    #[test]
    fn test_model_capabilities() {
        let claude_model = BedrockModel::find_by_id("anthropic.claude-3-5-sonnet-20241022-v2:0");
        assert!(claude_model.is_some());
        
        let model = claude_model.unwrap();
        assert_eq!(model.family, BedrockModelFamily::Anthropic);
        assert!(model.supports_tools);
        assert!(model.supports_streaming);
        
        let tool_capable_models = BedrockModel::tool_capable();
        assert!(!tool_capable_models.is_empty());
        
        // Ensure Claude models support tools
        assert!(tool_capable_models.iter().any(|m| m.id.contains("claude")));
        
        // Check Llama models support tools
        assert!(tool_capable_models.iter().any(|m| m.id.contains("llama")));
    }

    #[test]
    fn test_model_by_family() {
        let anthropic_models = BedrockModel::by_family(BedrockModelFamily::Anthropic);
        assert!(!anthropic_models.is_empty());
        assert!(anthropic_models.iter().all(|m| m.id.starts_with("anthropic.")));
        
        let meta_models = BedrockModel::by_family(BedrockModelFamily::Meta);
        assert!(!meta_models.is_empty());
        assert!(meta_models.iter().all(|m| m.id.starts_with("meta.")));
    }
}