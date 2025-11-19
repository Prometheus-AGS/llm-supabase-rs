// src/config/providers.rs
//
// Provider configuration types for AI service providers

use serde::{Deserialize, Serialize};
use crate::features::provider_fallback::RetryConfig;
use std::collections::HashMap;

/// Configuration for AI service providers
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ProvidersConfig {
    /// Vertex AI provider configuration
    pub vertex_ai: VertexAiConfig,

    /// OpenAI provider configuration
    pub openai: OpenAIConfig,

    /// Azure OpenAI provider configuration
    pub azure_openai: AzureOpenAIConfig,

    /// Anthropic provider configuration
    pub anthropic: AnthropicConfig,

    /// Groq provider configuration
    pub groq: GroqConfig,

    /// Mistral AI provider configuration
    pub mistral: MistralConfig,

    /// AWS Bedrock provider configuration
    pub aws_bedrock: AwsBedrockConfig,

    /// Cohere provider configuration
    pub cohere: CohereConfig,

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

/// OpenAI provider configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenAIConfig {
    /// OpenAI API key
    pub api_key: String,

    /// OpenAI base URL (defaults to https://api.openai.com/v1)
    pub base_url: Option<String>,

    /// Organization ID (optional)
    pub organization: Option<String>,

    /// Default model configuration
    pub default_model: OpenAIModelConfig,

    /// Model-specific configurations
    pub models: HashMap<String, OpenAIModelConfig>,

    /// API configuration
    pub api: OpenAIApiConfig,
}

/// Azure OpenAI provider configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AzureOpenAIConfig {
    /// Azure OpenAI API key
    pub api_key: String,

    /// Azure OpenAI endpoint (e.g., "https://your-resource.openai.azure.com")
    pub endpoint: String,

    /// Deployment name (maps to model)
    pub deployment: String,

    /// API version (defaults to "2024-02-15-preview")
    pub api_version: String,

    /// Default model configuration
    pub default_model: AzureOpenAIModelConfig,

    /// Model-specific configurations (deployment name -> config)
    pub models: HashMap<String, AzureOpenAIModelConfig>,

    /// Deployment mappings (model name -> deployment name)
    pub deployment_mappings: Option<HashMap<String, String>>,

    /// API configuration
    pub api: AzureOpenAIApiConfig,

    /// Azure AD authentication settings (optional)
    pub azure_ad: Option<AzureADAuthConfig>,
}

/// Anthropic provider configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnthropicConfig {
    /// Anthropic API key
    pub api_key: String,

    /// Anthropic base URL (defaults to https://api.anthropic.com/v1)
    pub base_url: Option<String>,

    /// Organization ID (optional)
    pub organization_id: Option<String>,

    /// Default model configuration
    pub default_model: AnthropicModelConfig,

    /// Model-specific configurations
    pub models: HashMap<String, AnthropicModelConfig>,

    /// API configuration
    pub api: AnthropicApiConfig,
}

/// Groq provider configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GroqConfig {
    /// Groq API key (must start with 'gsk_')
    pub api_key: String,

    /// Groq base URL (defaults to https://api.groq.com/openai/v1)
    pub base_url: Option<String>,

    /// Default model configuration
    pub default_model: GroqModelConfig,

    /// Model-specific configurations
    pub models: HashMap<String, GroqModelConfig>,

    /// API configuration
    pub api: GroqApiConfig,

    /// Rate limiting configuration (Groq has specific limits)
    pub rate_limit: GroqRateLimitConfig,
}

/// Mistral AI provider configuration with European compliance features
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MistralConfig {
    /// Mistral API key
    pub api_key: String,

    /// Mistral base URL (defaults to https://api.mistral.ai/v1)
    pub base_url: Option<String>,

    /// Default model configuration
    pub default_model: MistralModelConfig,

    /// Model-specific configurations
    pub models: HashMap<String, MistralModelConfig>,

    /// API configuration
    pub api: MistralApiConfig,

    /// Rate limiting configuration (Mistral has specific limits)
    pub rate_limit: MistralRateLimitConfig,

    /// European compliance settings
    pub compliance: MistralComplianceConfig,
}

/// Mistral model configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MistralModelConfig {
    /// Model name (e.g., "mistral-large-latest", "codestral-latest")
    pub model_name: String,

    /// Model capabilities
    pub capabilities: MistralModelCapabilities,

    /// Default generation parameters
    pub default_parameters: MistralModelParameters,

    /// Request timeout in seconds
    pub timeout_seconds: u64,

    /// Maximum retries for failed requests
    pub max_retries: u32,

    /// Cost configuration (European pricing)
    pub cost_config: MistralCostConfig,

    /// Whether this model is available in EU data centers
    pub eu_available: bool,

    /// Whether this model is code-optimized
    pub code_optimized: bool,
}

/// Mistral model capabilities
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MistralModelCapabilities {
    /// Whether model supports tool calling
    pub supports_tools: bool,

    /// Whether model supports streaming
    pub supports_streaming: bool,

    /// Whether model supports function calling (legacy)
    pub supports_functions: bool,

    /// Maximum context length
    pub max_context_length: u32,

    /// Maximum number of tools per request
    pub max_tools: Option<u32>,

    /// Supported response formats
    pub response_formats: Vec<String>,
}

/// Mistral model parameters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MistralModelParameters {
    /// Default temperature (0.0 to 1.0)
    pub temperature: f64,

    /// Default top_p (0.0 to 1.0)
    pub top_p: f64,

    /// Default max output tokens
    pub max_tokens: u32,

    /// Default stop sequences
    pub stop: Vec<String>,

    /// Random seed for reproducible outputs
    pub random_seed: Option<u32>,

    /// Whether to enable safe mode
    pub safe_mode: bool,
}

/// Mistral API configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MistralApiConfig {
    /// API version to use
    pub version: String,

    /// Connection timeout in seconds
    pub connection_timeout: u64,

    /// Request timeout in seconds
    pub request_timeout: u64,

    /// Maximum number of concurrent requests
    pub max_concurrent_requests: u32,

    /// User agent string
    pub user_agent: Option<String>,

    /// Custom headers to include in requests
    pub custom_headers: HashMap<String, String>,

    /// Retry configuration
    pub retry: RetryConfig,
}

/// Mistral rate limiting configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MistralRateLimitConfig {
    /// Requests per minute limit
    pub requests_per_minute: u32,

    /// Tokens per minute limit
    pub tokens_per_minute: u32,

    /// Whether to enable automatic throttling
    pub auto_throttle: bool,

    /// Burst allowance for temporary spikes
    pub burst_allowance: u32,

    /// Rate limit window in seconds
    pub window_seconds: u64,
}

/// European compliance configuration for Mistral
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MistralComplianceConfig {
    /// Enable GDPR compliance mode
    pub gdpr_mode: bool,

    /// Prefer EU data residency
    pub eu_residency: bool,

    /// Data processing region preference
    pub preferred_region: Option<String>,

    /// Enable compliance logging
    pub compliance_logging: bool,

    /// Data retention settings
    pub data_retention: MistralDataRetentionConfig,

    /// Privacy settings
    pub privacy: MistralPrivacyConfig,
}

/// Data retention configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MistralDataRetentionConfig {
    /// Retain request logs (in days, 0 = no retention)
    pub request_logs_days: u32,

    /// Retain conversation history (in days, 0 = no retention)
    pub conversation_history_days: u32,

    /// Enable automatic data purging
    pub auto_purge: bool,

    /// Purge schedule (cron format)
    pub purge_schedule: Option<String>,
}

/// Privacy configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MistralPrivacyConfig {
    /// Enable request anonymization
    pub anonymize_requests: bool,

    /// Enable response filtering for PII
    pub filter_pii: bool,

    /// Enable consent tracking
    pub consent_tracking: bool,

    /// Data subject access controls
    pub access_controls: bool,
}

/// Cost configuration for Mistral models
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MistralCostConfig {
    /// Cost per 1K input tokens (EUR)
    pub input_cost_per_1k_tokens: f64,

    /// Cost per 1K output tokens (EUR)
    pub output_cost_per_1k_tokens: f64,

    /// Currency (EUR for European pricing)
    pub currency: String,

    /// Cost tracking enabled
    pub tracking_enabled: bool,

    /// Budget limits
    pub budget_limits: Option<MistralBudgetLimits>,
}

/// Budget limits for cost control
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MistralBudgetLimits {
    /// Daily budget limit (EUR)
    pub daily_limit: Option<f64>,

    /// Monthly budget limit (EUR)
    pub monthly_limit: Option<f64>,

    /// Per-request cost limit (EUR)
    pub per_request_limit: Option<f64>,

    /// Alert thresholds (percentage of limit)
    pub alert_thresholds: Vec<f64>,
}

/// AWS Bedrock provider configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AwsBedrockConfig {
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
    pub default_model: AwsBedrockModelConfig,

    /// Model-specific configurations
    pub models: HashMap<String, AwsBedrockModelConfig>,

    /// API configuration
    pub api: AwsBedrockApiConfig,

    /// Authentication method preference
    pub auth_method: AwsBedrockAuthMethod,

    /// Timeout settings
    pub timeouts: AwsBedrockTimeoutConfig,

    /// Whether to use AWS STS for credential validation
    pub validate_credentials: bool,
}

/// Cohere provider configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CohereConfig {
    /// Cohere API key
    pub api_key: String,

    /// Cohere base URL (defaults to https://api.cohere.ai/v1)
    pub base_url: Option<String>,

    /// Default model configuration
    pub default_model: CohereModelConfig,

    /// Model-specific configurations
    pub models: HashMap<String, CohereModelConfig>,

    /// API configuration
    pub api: CohereApiConfig,

    /// Rate limiting configuration
    pub rate_limit: CohereRateLimitConfig,
}

/// Cohere model configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CohereModelConfig {
    /// Cohere model name (e.g., "command-r-plus", "command-r")
    pub model_name: String,

    /// Model version or variant (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,

    /// Default generation parameters
    pub default_parameters: CohereModelParameters,

    /// Request timeout in seconds
    pub timeout_seconds: u64,

    /// Maximum retries for failed requests
    pub max_retries: u32,

    /// Whether this model supports tool calling
    pub supports_tools: bool,

    /// Whether this model supports streaming
    pub supports_streaming: bool,

    /// Maximum context length for this model
    pub max_context_length: u32,
}

/// Cohere model parameters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CohereModelParameters {
    /// Default temperature (0.0 to 1.0)
    pub temperature: f64,

    /// Default top_p (0.0 to 1.0)
    pub top_p: f64,

    /// Default top_k (1 to 500)
    pub top_k: u32,

    /// Default max output tokens
    pub max_output_tokens: u32,

    /// Default stop sequences
    pub stop_sequences: Vec<String>,

    /// Default preamble for system instructions
    pub preamble: Option<String>,
}

/// Cohere API configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CohereApiConfig {
    /// API timeout in seconds
    pub timeout: u64,

    /// Maximum number of retries
    pub max_retries: u32,

    /// Retry delay in milliseconds
    pub retry_delay_ms: u64,

    /// Enable request logging
    pub enable_logging: bool,

    /// Custom headers to include in requests
    pub custom_headers: Option<HashMap<String, String>>,

    /// Connection pool settings
    pub connection_pool: CohereConnectionPoolConfig,
}

/// Cohere connection pool configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CohereConnectionPoolConfig {
    /// Maximum number of idle connections
    pub max_idle: Option<usize>,

    /// Maximum number of connections per host
    pub max_per_host: Option<usize>,

    /// Connection timeout in seconds
    pub connect_timeout: u64,

    /// Keep-alive timeout in seconds
    pub keep_alive_timeout: u64,
}

/// Cohere rate limiting configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CohereRateLimitConfig {
    /// Requests per minute limit
    pub requests_per_minute: u32,

    /// Tokens per minute limit
    pub tokens_per_minute: u32,

    /// Concurrent requests limit
    pub concurrent_requests: u32,

    /// Enable rate limiting
    pub enabled: bool,

    /// Rate limit enforcement strategy
    pub strategy: RateLimitStrategy,
}

/// Rate limiting strategy
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RateLimitStrategy {
    /// Block requests when limit exceeded
    Block,

    /// Queue requests when limit exceeded
    Queue,

    /// Drop requests when limit exceeded
    Drop,

    /// Use exponential backoff
    ExponentialBackoff,
}

/// AWS Bedrock authentication method preference
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AwsBedrockAuthMethod {
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

/// Configuration for a specific AWS Bedrock model
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AwsBedrockModelConfig {
    /// Bedrock model identifier (e.g., "anthropic.claude-3-5-sonnet-20241022-v2:0")
    pub model_id: String,

    /// Model family (anthropic, meta, mistral, amazon, cohere)
    pub model_family: String,

    /// Whether this model supports tool calling
    pub supports_tools: bool,

    /// Whether this model supports streaming
    pub supports_streaming: bool,

    /// Maximum context length for this model
    pub max_context_length: u32,

    /// Maximum output tokens for this model
    pub max_output_tokens: u32,

    /// Default generation parameters
    pub default_parameters: AwsBedrockModelParameters,

    /// Request timeout in seconds
    pub timeout_seconds: u64,

    /// Maximum retries for failed requests
    pub max_retries: u32,

    /// Rate limiting configuration
    pub rate_limit: RateLimitConfig,
}

/// Default parameters for AWS Bedrock model generation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AwsBedrockModelParameters {
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
}

/// AWS Bedrock API configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AwsBedrockApiConfig {
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
}

/// Timeout configuration for AWS Bedrock
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AwsBedrockTimeoutConfig {
    /// Connection timeout in seconds
    pub connect: u64,

    /// Read timeout in seconds
    pub read: u64,

    /// Write timeout in seconds
    pub write: u64,

    /// Total request timeout in seconds
    pub total: u64,
}

/// Configuration for a specific OpenAI model
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenAIModelConfig {
    /// OpenAI model name (e.g., "gpt-4o", "gpt-3.5-turbo")
    pub model_name: String,

    /// Default generation parameters
    pub default_parameters: OpenAIModelParameters,

    /// Request timeout in seconds
    pub timeout_seconds: u64,

    /// Maximum retries for failed requests
    pub max_retries: u32,

    /// Rate limiting configuration
    pub rate_limit: RateLimitConfig,
}

/// Default parameters for OpenAI model generation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenAIModelParameters {
    /// Default temperature (0.0 to 2.0)
    pub temperature: f64,

    /// Default top_p (0.0 to 1.0)
    pub top_p: f64,

    /// Default max tokens
    pub max_tokens: Option<u32>,

    /// Default presence penalty (-2.0 to 2.0)
    pub presence_penalty: f64,

    /// Default frequency penalty (-2.0 to 2.0)
    pub frequency_penalty: f64,

    /// Default stop sequences
    pub stop: Vec<String>,
}

/// OpenAI API configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenAIApiConfig {
    /// Connection timeout in seconds
    pub connection_timeout: u64,

    /// Request timeout in seconds
    pub request_timeout: u64,

    /// Maximum retries
    pub max_retries: u32,

    /// Rate limit requests per minute
    pub rate_limit_requests_per_minute: Option<u32>,
}

/// Configuration for a specific Azure OpenAI model/deployment
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AzureOpenAIModelConfig {
    /// Azure OpenAI deployment name
    pub deployment_name: String,

    /// Model family (e.g., "gpt-4o", "gpt-4", "gpt-3.5-turbo")
    pub model_family: String,

    /// Default generation parameters
    pub default_parameters: AzureOpenAIModelParameters,

    /// Request timeout in seconds
    pub timeout_seconds: u64,

    /// Maximum retries for failed requests
    pub max_retries: u32,

    /// Rate limiting configuration
    pub rate_limit: RateLimitConfig,
}

/// Default parameters for Azure OpenAI model generation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AzureOpenAIModelParameters {
    /// Default temperature (0.0 to 2.0)
    pub temperature: f64,

    /// Default top_p (0.0 to 1.0)
    pub top_p: f64,

    /// Default max tokens
    pub max_tokens: Option<u32>,

    /// Default presence penalty (-2.0 to 2.0)
    pub presence_penalty: f64,

    /// Default frequency penalty (-2.0 to 2.0)
    pub frequency_penalty: f64,

    /// Default stop sequences
    pub stop: Vec<String>,
}

/// Azure OpenAI API configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AzureOpenAIApiConfig {
    /// Connection timeout in seconds
    pub connection_timeout: u64,

    /// Request timeout in seconds
    pub request_timeout: u64,

    /// Maximum retries
    pub max_retries: u32,

    /// Rate limit requests per minute
    pub rate_limit_requests_per_minute: Option<u32>,

    /// Exponential backoff for retries
    pub exponential_backoff: bool,

    /// Base delay between retries in milliseconds
    pub retry_delay_ms: u64,
}

/// Azure AD authentication configuration for Azure OpenAI
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AzureADAuthConfig {
    /// Azure AD tenant ID
    pub tenant_id: String,

    /// Azure AD client ID
    pub client_id: String,

    /// Azure AD client secret
    pub client_secret: String,

    /// Azure AD scope (defaults to "https://cognitiveservices.azure.com/.default")
    pub scope: Option<String>,
}

/// Configuration for a specific Anthropic model
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnthropicModelConfig {
    /// Anthropic model name (e.g., "claude-3-5-sonnet-20241022")
    pub model_name: String,

    /// Default generation parameters
    pub default_parameters: AnthropicModelParameters,

    /// Request timeout in seconds
    pub timeout_seconds: u64,

    /// Maximum retries for failed requests
    pub max_retries: u32,

    /// Rate limiting configuration
    pub rate_limit: RateLimitConfig,
}

/// Default parameters for Anthropic model generation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnthropicModelParameters {
    /// Default temperature (0.0 to 1.0)
    pub temperature: f64,

    /// Default top_p (0.0 to 1.0)
    pub top_p: f64,

    /// Default top_k (1 to 200)
    pub top_k: Option<u32>,

    /// Default max tokens
    pub max_tokens: u32,

    /// Default stop sequences
    pub stop_sequences: Vec<String>,
}

/// Anthropic API configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnthropicApiConfig {
    /// Connection timeout in seconds
    pub connection_timeout: u64,

    /// Request timeout in seconds
    pub request_timeout: u64,

    /// Maximum retries
    pub max_retries: u32,

    /// Rate limit requests per minute
    pub rate_limit_requests_per_minute: Option<u32>,

    /// Anthropic API version
    pub api_version: String,
}

/// Configuration for a specific Groq model
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GroqModelConfig {
    /// Groq model name (e.g., "llama-3.1-70b-versatile", "llama-3.1-8b-instant")
    pub model_name: String,

    /// Model family (llama, mixtral, gemma)
    pub model_family: String,

    /// Default generation parameters
    pub default_parameters: GroqModelParameters,

    /// Request timeout in seconds (Groq is fast, shorter timeouts)
    pub timeout_seconds: u64,

    /// Maximum retries for failed requests
    pub max_retries: u32,

    /// Rate limiting configuration
    pub rate_limit: RateLimitConfig,

    /// Whether this model supports tool calling
    pub supports_tools: bool,

    /// Whether this model supports streaming
    pub supports_streaming: bool,

    /// Maximum context length for this model
    pub max_context_length: u32,
}

/// Default parameters for Groq model generation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GroqModelParameters {
    /// Default temperature (0.0 to 2.0, but Groq may have model-specific limits)
    pub temperature: f64,

    /// Default top_p (0.0 to 1.0)
    pub top_p: f64,

    /// Default max tokens
    pub max_tokens: Option<u32>,

    /// Default stop sequences
    pub stop: Vec<String>,

    /// Frequency penalty (Groq may support this)
    pub frequency_penalty: Option<f64>,

    /// Presence penalty (Groq may support this)
    pub presence_penalty: Option<f64>,
}

/// Groq API configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GroqApiConfig {
    /// Connection timeout in seconds (fast connections for Groq)
    pub connection_timeout: u64,

    /// Request timeout in seconds (Groq is very fast)
    pub request_timeout: u64,

    /// Maximum retries
    pub max_retries: u32,

    /// Rate limit requests per minute (Groq has specific limits)
    pub rate_limit_requests_per_minute: Option<u32>,

    /// Rate limit tokens per minute
    pub rate_limit_tokens_per_minute: Option<u32>,

    /// Whether to use exponential backoff
    pub exponential_backoff: bool,

    /// Base delay between retries in milliseconds
    pub retry_delay_ms: u64,
}

/// Groq-specific rate limiting configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GroqRateLimitConfig {
    /// Requests per minute limit
    pub requests_per_minute: u32,

    /// Tokens per minute limit
    pub tokens_per_minute: u32,

    /// Whether to automatically throttle requests
    pub auto_throttle: bool,

    /// Concurrent requests limit
    pub max_concurrent_requests: u32,
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

/// Unified model configuration enum for different providers
#[derive(Debug, Clone)]
pub enum ModelConfig {
    VertexAI(VertexModelConfig),
    OpenAI(OpenAIModelConfig),
    AzureOpenAI(AzureOpenAIModelConfig),
    Anthropic(AnthropicModelConfig),
    Groq(GroqModelConfig),
    AwsBedrock(AwsBedrockModelConfig),
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

impl Default for OpenAIConfig {
    fn default() -> Self {
        Self {
            api_key: std::env::var("OPENAI_API_KEY").unwrap_or_default(),
            base_url: std::env::var("OPENAI_BASE_URL").ok()
                .or_else(|| Some("https://api.openai.com/v1".to_string())),
            organization: std::env::var("OPENAI_ORGANIZATION").ok(),
            default_model: OpenAIModelConfig::default(),
            models: HashMap::new(),
            api: OpenAIApiConfig::default(),
        }
    }
}

impl OpenAIConfig {
    /// Create OpenAI configuration from environment variables
    pub fn from_env() -> anyhow::Result<Self> {
        let api_key = std::env::var("OPENAI_API_KEY")
            .map_err(|_| anyhow::anyhow!("OPENAI_API_KEY environment variable is required"))?;

        Ok(Self {
            api_key,
            base_url: std::env::var("OPENAI_BASE_URL").ok()
                .or_else(|| Some("https://api.openai.com/v1".to_string())),
            organization: std::env::var("OPENAI_ORGANIZATION").ok(),
            default_model: OpenAIModelConfig::default(),
            models: HashMap::new(),
            api: OpenAIApiConfig::default(),
        })
    }

    /// Get base URL for OpenAI API
    pub fn get_base_url(&self) -> String {
        self.base_url.clone().unwrap_or_else(|| "https://api.openai.com/v1".to_string())
    }

    /// Get chat completions endpoint URL
    pub fn get_chat_completions_url(&self) -> String {
        format!("{}/chat/completions", self.get_base_url())
    }

    /// Get timeout in seconds
    pub fn timeout(&self) -> Option<u64> {
        Some(self.api.request_timeout)
    }

    /// Get max retries
    pub fn max_retries(&self) -> Option<u32> {
        Some(self.api.max_retries)
    }

    /// Get default model name
    pub fn default_model(&self) -> Option<String> {
        Some(self.default_model.model_name.clone())
    }
}

impl Default for AnthropicConfig {
    fn default() -> Self {
        Self {
            api_key: std::env::var("ANTHROPIC_API_KEY").unwrap_or_default(),
            base_url: std::env::var("ANTHROPIC_BASE_URL").ok()
                .or_else(|| Some("https://api.anthropic.com/v1".to_string())),
            organization_id: std::env::var("ANTHROPIC_ORGANIZATION_ID").ok(),
            default_model: AnthropicModelConfig::default(),
            models: HashMap::new(),
            api: AnthropicApiConfig::default(),
        }
    }
}

impl Default for OpenAIModelConfig {
    fn default() -> Self {
        Self {
            model_name: std::env::var("OPENAI_DEFAULT_MODEL")
                .unwrap_or_else(|_| "gpt-4o".to_string()),
            default_parameters: OpenAIModelParameters::default(),
            timeout_seconds: 120,
            max_retries: 3,
            rate_limit: RateLimitConfig::default(),
        }
    }
}

impl Default for OpenAIModelParameters {
    fn default() -> Self {
        Self {
            temperature: 0.7,
            top_p: 1.0,
            max_tokens: None,
            presence_penalty: 0.0,
            frequency_penalty: 0.0,
            stop: vec![],
        }
    }
}

impl Default for AnthropicModelConfig {
    fn default() -> Self {
        Self {
            model_name: std::env::var("ANTHROPIC_DEFAULT_MODEL")
                .unwrap_or_else(|_| "claude-3-5-sonnet-20241022".to_string()),
            default_parameters: AnthropicModelParameters::default(),
            timeout_seconds: 120,
            max_retries: 3,
            rate_limit: RateLimitConfig::default(),
        }
    }
}

impl Default for AnthropicModelParameters {
    fn default() -> Self {
        Self {
            temperature: 0.7,
            top_p: 1.0,
            top_k: None,
            max_tokens: 4096,
            stop_sequences: vec![],
        }
    }
}

impl Default for AnthropicApiConfig {
    fn default() -> Self {
        Self {
            connection_timeout: 30,
            request_timeout: 120,
            max_retries: 3,
            rate_limit_requests_per_minute: Some(50),
            api_version: "2023-06-01".to_string(),
        }
    }
}

impl Default for OpenAIApiConfig {
    fn default() -> Self {
        Self {
            connection_timeout: 30,
            request_timeout: 120,
            max_retries: 3,
            rate_limit_requests_per_minute: Some(60),
        }
    }
}

impl Default for AwsBedrockConfig {
    fn default() -> Self {
        Self {
            region: std::env::var("AWS_REGION")
                .or_else(|_| std::env::var("AWS_DEFAULT_REGION"))
                .unwrap_or_else(|_| "us-east-1".to_string()),
            access_key: std::env::var("AWS_ACCESS_KEY_ID").ok(),
            secret_key: std::env::var("AWS_SECRET_ACCESS_KEY").ok(),
            session_token: std::env::var("AWS_SESSION_TOKEN").ok(),
            profile: std::env::var("AWS_PROFILE").ok(),
            endpoint_url: None,
            default_model: AwsBedrockModelConfig::default(),
            models: HashMap::new(),
            api: AwsBedrockApiConfig::default(),
            auth_method: AwsBedrockAuthMethod::Default,
            timeouts: AwsBedrockTimeoutConfig::default(),
            validate_credentials: true,
        }
    }
}

impl Default for AwsBedrockModelConfig {
    fn default() -> Self {
        Self {
            model_id: "anthropic.claude-3-5-sonnet-20241022-v2:0".to_string(),
            model_family: "anthropic".to_string(),
            supports_tools: true,
            supports_streaming: true,
            max_context_length: 200000,
            max_output_tokens: 4096,
            default_parameters: AwsBedrockModelParameters::default(),
            timeout_seconds: 60,
            max_retries: 3,
            rate_limit: RateLimitConfig::default(),
        }
    }
}

impl Default for AwsBedrockModelParameters {
    fn default() -> Self {
        Self {
            temperature: Some(0.7),
            top_p: Some(0.9),
            top_k: None,
            max_tokens: Some(4096),
            stop_sequences: vec![],
        }
    }
}

impl Default for AwsBedrockApiConfig {
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
        }
    }
}

impl Default for AwsBedrockTimeoutConfig {
    fn default() -> Self {
        Self {
            connect: 30,
            read: 120,
            write: 30,
            total: 300,
        }
    }
}

impl Default for ProviderRoutingConfig {
    fn default() -> Self {
        let mut model_mapping = HashMap::new();
        
        // Vertex AI models
        model_mapping.insert("claude-sonnet-4-5@20250929".to_string(), "vertex_ai".to_string());
        model_mapping.insert("claude-3-5-haiku@20241022".to_string(), "vertex_ai".to_string());
        
        // OpenAI models
        model_mapping.insert("gpt-4o".to_string(), "openai".to_string());
        model_mapping.insert("gpt-4o-mini".to_string(), "openai".to_string());
        model_mapping.insert("gpt-4-turbo".to_string(), "openai".to_string());
        model_mapping.insert("gpt-3.5-turbo".to_string(), "openai".to_string());

        // Azure OpenAI models (using deployment names)
        model_mapping.insert("azure-gpt-4o".to_string(), "azure_openai".to_string());
        model_mapping.insert("azure-gpt-4".to_string(), "azure_openai".to_string());
        model_mapping.insert("azure-gpt-35-turbo".to_string(), "azure_openai".to_string());
        model_mapping.insert("gpt-4o-azure".to_string(), "azure_openai".to_string());
        model_mapping.insert("gpt-4-azure".to_string(), "azure_openai".to_string());
        model_mapping.insert("gpt-35-turbo-azure".to_string(), "azure_openai".to_string());

        // Anthropic models
        model_mapping.insert("claude-3-5-sonnet-20241022".to_string(), "anthropic".to_string());
        model_mapping.insert("claude-3-5-haiku-20241022".to_string(), "anthropic".to_string());
        model_mapping.insert("claude-3-opus-20240229".to_string(), "anthropic".to_string());
        model_mapping.insert("claude-3-sonnet-20240229".to_string(), "anthropic".to_string());
        model_mapping.insert("claude-3-haiku-20240307".to_string(), "anthropic".to_string());

        // Groq models
        model_mapping.insert("llama-3.1-70b-versatile".to_string(), "groq".to_string());
        model_mapping.insert("llama-3.1-8b-instant".to_string(), "groq".to_string());
        model_mapping.insert("mixtral-8x7b-32768".to_string(), "groq".to_string());
        model_mapping.insert("gemma2-9b-it".to_string(), "groq".to_string());
        model_mapping.insert("llama3-70b-8192".to_string(), "groq".to_string());
        model_mapping.insert("llama3-8b-8192".to_string(), "groq".to_string());

        // AWS Bedrock models
        model_mapping.insert("anthropic.claude-3-5-sonnet-20241022-v2:0".to_string(), "aws_bedrock".to_string());
        model_mapping.insert("anthropic.claude-3-haiku-20240307-v1:0".to_string(), "aws_bedrock".to_string());
        model_mapping.insert("anthropic.claude-3-opus-20240229-v1:0".to_string(), "aws_bedrock".to_string());
        model_mapping.insert("meta.llama3-1-70b-instruct-v1:0".to_string(), "aws_bedrock".to_string());
        model_mapping.insert("meta.llama3-1-8b-instruct-v1:0".to_string(), "aws_bedrock".to_string());
        model_mapping.insert("mistral.mistral-7b-instruct-v0:2".to_string(), "aws_bedrock".to_string());

        Self {
            default_provider: "openai".to_string(), // OpenAI as default since it's more widely accessible
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
            max_attempts: 3,
            provider_chain: vec![
                "openai".to_string(),      // Most reliable
                "anthropic".to_string(),   // Direct Claude access
                "aws_bedrock".to_string(), // Enterprise AWS option with multi-model support
                "azure_openai".to_string(), // Enterprise option
                "groq".to_string(),        // Fastest inference
                "vertex_ai".to_string(),   // Google's proxy
            ],
            trigger_conditions: vec![
                FallbackTrigger::Timeout,
                FallbackTrigger::ServerError,
                FallbackTrigger::RateLimit,
                FallbackTrigger::AuthError,
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

    /// Get Vertex AI model configuration for a specific model
    pub fn get_vertex_model_config(&self, model_name: &str) -> Option<&VertexModelConfig> {
        self.vertex_ai.models.get(model_name)
            .or(Some(&self.vertex_ai.default_model))
    }

    /// Get OpenAI model configuration for a specific model
    pub fn get_openai_model_config(&self, model_name: &str) -> Option<&OpenAIModelConfig> {
        self.openai.models.get(model_name)
            .or(Some(&self.openai.default_model))
    }

    /// Get Azure OpenAI model configuration for a specific model
    pub fn get_azure_openai_model_config(&self, model_name: &str) -> Option<&AzureOpenAIModelConfig> {
        // First try direct deployment name lookup
        if let Some(config) = self.azure_openai.models.get(model_name) {
            return Some(config);
        }
        
        // Then try deployment mapping
        if let Some(ref mappings) = self.azure_openai.deployment_mappings {
            if let Some(deployment) = mappings.get(model_name) {
                if let Some(config) = self.azure_openai.models.get(deployment) {
                    return Some(config);
                }
            }
        }
        
        // Fallback to default model
        Some(&self.azure_openai.default_model)
    }

    /// Get Anthropic model configuration for a specific model
    pub fn get_anthropic_model_config(&self, model_name: &str) -> Option<&AnthropicModelConfig> {
        self.anthropic.models.get(model_name)
            .or(Some(&self.anthropic.default_model))
    }

    /// Get Groq model configuration for a specific model
    pub fn get_groq_model_config(&self, model_name: &str) -> Option<&GroqModelConfig> {
        self.groq.models.get(model_name)
            .or(Some(&self.groq.default_model))
    }

    /// Get model configuration for any provider
    pub fn get_model_config(&self, model_name: &str) -> Option<ModelConfig> {
        let provider = self.get_provider_for_model(model_name);
        match provider {
            "vertex_ai" => self.get_vertex_model_config(model_name)
                .map(|config| ModelConfig::VertexAI(config.clone())),
            "openai" => self.get_openai_model_config(model_name)
                .map(|config| ModelConfig::OpenAI(config.clone())),
            "azure_openai" => self.get_azure_openai_model_config(model_name)
                .map(|config| ModelConfig::AzureOpenAI(config.clone())),
            "anthropic" => self.get_anthropic_model_config(model_name)
                .map(|config| ModelConfig::Anthropic(config.clone())),
            "groq" => self.get_groq_model_config(model_name)
                .map(|config| ModelConfig::Groq(config.clone())),
            _ => None,
        }
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
        self.openai.models.contains_key(model_name) ||
        self.azure_openai.models.contains_key(model_name) ||
        self.anthropic.models.contains_key(model_name) ||
        model_name == self.vertex_ai.default_model.model_name ||
        model_name == self.openai.default_model.model_name ||
        model_name == self.azure_openai.default_model.deployment_name ||
        model_name == self.anthropic.default_model.model_name ||
        // Check Azure OpenAI deployment mappings
        (self.azure_openai.deployment_mappings.as_ref()
            .map(|m| m.contains_key(model_name))
            .unwrap_or(false))
    }

    /// Get all supported model names
    pub fn supported_models(&self) -> Vec<String> {
        let mut models = Vec::new();

        // Add models from routing configuration
        models.extend(self.routing.model_mapping.keys().cloned());

        // Add models from Vertex AI configuration
        models.extend(self.vertex_ai.models.keys().cloned());

        // Add models from OpenAI configuration
        models.extend(self.openai.models.keys().cloned());

        // Add models from Azure OpenAI configuration
        models.extend(self.azure_openai.models.keys().cloned());

        // Add models from Anthropic configuration
        models.extend(self.anthropic.models.keys().cloned());

        // Add default models
        models.push(self.vertex_ai.default_model.model_name.clone());
        models.push(self.openai.default_model.model_name.clone());
        models.push(self.azure_openai.default_model.deployment_name.clone());
        models.push(self.anthropic.default_model.model_name.clone());

        // Add Azure OpenAI mapped models
        if let Some(ref mappings) = self.azure_openai.deployment_mappings {
            models.extend(mappings.keys().cloned());
        }

        // Deduplicate and sort
        models.sort();
        models.dedup();
        models
    }

    /// Get Azure OpenAI deployment name for a model
    pub fn get_azure_openai_deployment(&self, model_name: &str) -> Option<String> {
        // First check if it's already a deployment name
        if self.azure_openai.models.contains_key(model_name) {
            return Some(model_name.to_string());
        }
        
        // Then check deployment mappings
        if let Some(ref mappings) = self.azure_openai.deployment_mappings {
            if let Some(deployment) = mappings.get(model_name) {
                return Some(deployment.clone());
            }
        }
        
        // Fall back to default deployment
        Some(self.azure_openai.deployment.clone())
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
