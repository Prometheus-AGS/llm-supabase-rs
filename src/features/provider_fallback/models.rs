
// src/features/provider_fallback/models.rs
//
// Provider fallback system models and types

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::{Duration, Instant, SystemTime};
use crate::models::error::ErrorType;

/// Supported AI providers
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Provider {
    VertexAI,
    OpenAI,
    AzureOpenAI,
    Anthropic,
    Groq,
    Mistral,
    AwsBedrock,
    Cohere,
}

impl Provider {
    /// Get all supported providers
    pub fn all() -> Vec<Provider> {
        vec![
            Provider::OpenAI,
            Provider::Anthropic,
            Provider::AzureOpenAI,
            Provider::Cohere,
            Provider::Mistral,
            Provider::AwsBedrock,
            Provider::Groq,
            Provider::VertexAI,
        ]
    }

    /// Get provider name as string
    pub fn name(&self) -> &'static str {
        match self {
            Provider::VertexAI => "vertex_ai",
            Provider::OpenAI => "openai",
            Provider::AzureOpenAI => "azure_openai",
            Provider::Anthropic => "anthropic",
            Provider::Groq => "groq",
            Provider::Mistral => "mistral",
            Provider::AwsBedrock => "aws_bedrock",
            Provider::Cohere => "cohere",
        }
    }

    /// Get provider display name
    pub fn display_name(&self) -> &'static str {
        match self {
            Provider::VertexAI => "Google Vertex AI",
            Provider::OpenAI => "OpenAI",
            Provider::AzureOpenAI => "Azure OpenAI",
            Provider::Anthropic => "Anthropic Claude",
            Provider::Groq => "Groq",
            Provider::Mistral => "Mistral AI",
            Provider::AwsBedrock => "AWS Bedrock",
            Provider::Cohere => "Cohere",
        }
    }

    /// Get provider base URL
    pub fn base_url(&self) -> &'static str {
        match self {
            Provider::VertexAI => "https://us-central1-aiplatform.googleapis.com",
            Provider::OpenAI => "https://api.openai.com/v1",
            Provider::AzureOpenAI => "https://{resource}.openai.azure.com",
            Provider::Anthropic => "https://api.anthropic.com/v1",
            Provider::Groq => "https://api.groq.com",
            Provider::Mistral => "https://api.mistral.ai/v1",
            Provider::AwsBedrock => "https://bedrock-runtime.us-east-1.amazonaws.com",
            Provider::Cohere => "https://api.cohere.ai/v1",
        }
    }

    /// Get supported models for this provider
    pub fn supported_models(&self) -> Vec<&'static str> {
        match self {
            Provider::VertexAI => vec![
                "claude-3-5-sonnet@20241022",
                "claude-3-haiku@20240307",
                "claude-3-opus@20240229",
            ],
            Provider::OpenAI => vec![
                "gpt-4o",
                "gpt-4o-mini",
                "gpt-4-turbo",
                "gpt-4-turbo-preview",
                "gpt-4-0125-preview",
                "gpt-4-1106-preview",
                "gpt-3.5-turbo",
                "gpt-3.5-turbo-0125",
            ],
            Provider::AzureOpenAI => vec![
                "gpt-4o",
                "gpt-4o-mini",
                "gpt-4-turbo",
                "gpt-4",
                "gpt-3.5-turbo",
            ],
            Provider::Anthropic => vec![
                "claude-3-5-sonnet-20241022",
                "claude-3-5-haiku-20241022",
                "claude-3-opus-20240229",
                "claude-3-sonnet-20240229",
                "claude-3-haiku-20240307",
            ],
            Provider::Groq => vec![
                "llama-3.1-70b-versatile",
                "llama-3.1-8b-instant",
                "llama-3.2-1b-preview",
                "llama-3.2-3b-preview",
                "llama3-groq-70b-8192-tool-use-preview",
                "llama3-groq-8b-8192-tool-use-preview",
                "mixtral-8x7b-32768",
                "gemma2-9b-it",
            ],
            Provider::Mistral => vec![
                "mistral-large-latest",
                "mistral-medium-latest",
                "mistral-small-latest",
                "codestral-latest",
                "mistral-tiny",
                "open-mistral-7b",
                "open-mistral-nemo",
                "open-codestral-mamba",
            ],
            Provider::AwsBedrock => vec![
                "anthropic.claude-3-5-sonnet-20241022-v2:0",
                "anthropic.claude-3-haiku-20240307-v1:0",
                "anthropic.claude-3-opus-20240229-v1:0",
                "anthropic.claude-3-sonnet-20240229-v1:0",
                "meta.llama3-1-70b-instruct-v1:0",
                "meta.llama3-1-8b-instruct-v1:0",
                "mistral.mistral-7b-instruct-v0:2",
                "mistral.mistral-large-2402-v1:0",
                "cohere.command-r-plus-v1:0",
                "cohere.command-r-v1:0",
            ],
            Provider::Cohere => vec![
                "command-r-plus",
                "command-r",
                "command",
                "command-nightly",
                "command-light",
                "command-light-nightly",
            ],
        }
    }

    /// Check if provider supports a specific model
    pub fn supports_model(&self, model: &str) -> bool {
        self.supported_models().contains(&model)
    }

    /// Get default model for this provider
    pub fn default_model(&self) -> &'static str {
        match self {
            Provider::VertexAI => "claude-3-5-sonnet@20241022",
            Provider::OpenAI => "gpt-4o",
            Provider::AzureOpenAI => "gpt-4o",
            Provider::Anthropic => "claude-3-5-sonnet-20241022",
            Provider::Groq => "llama-3.1-70b-versatile",
            Provider::Mistral => "mistral-large-latest",
            Provider::AwsBedrock => "anthropic.claude-3-5-sonnet-20241022-v2:0",
            Provider::Cohere => "command-r-plus",
        }
    }

    /// Get provider performance characteristics
    pub fn performance_characteristics(&self) -> ProviderPerformanceProfile {
        match self {
            Provider::VertexAI => ProviderPerformanceProfile {
                average_response_time: Duration::from_millis(2000),
                reliability_score: 0.95,
                speed_score: 0.75,
                quality_score: 0.95,
                cost_efficiency: 0.85,
                enterprise_grade: true,
            },
            Provider::OpenAI => ProviderPerformanceProfile {
                average_response_time: Duration::from_millis(1500),
                reliability_score: 0.98,
                speed_score: 0.85,
                quality_score: 0.95,
                cost_efficiency: 0.80,
                enterprise_grade: true,
            },
            Provider::AzureOpenAI => ProviderPerformanceProfile {
                average_response_time: Duration::from_millis(1600),
                reliability_score: 0.97,
                speed_score: 0.83,
                quality_score: 0.95,
                cost_efficiency: 0.78,
                enterprise_grade: true,
            },
            Provider::Anthropic => ProviderPerformanceProfile {
                average_response_time: Duration::from_millis(1800),
                reliability_score: 0.96,
                speed_score: 0.78,
                quality_score: 0.97,
                cost_efficiency: 0.82,
                enterprise_grade: true,
            },
            Provider::Groq => ProviderPerformanceProfile {
                average_response_time: Duration::from_millis(400),
                reliability_score: 0.92,
                speed_score: 0.98,
                quality_score: 0.85,
                cost_efficiency: 0.95,
                enterprise_grade: false,
            },
            Provider::Mistral => ProviderPerformanceProfile {
                average_response_time: Duration::from_millis(1200),
                reliability_score: 0.93,
                speed_score: 0.88,
                quality_score: 0.88,
                cost_efficiency: 0.92,
                enterprise_grade: true,
            },
            Provider::AwsBedrock => ProviderPerformanceProfile {
                average_response_time: Duration::from_millis(2200),
                reliability_score: 0.96,
                speed_score: 0.70,
                quality_score: 0.93,
                cost_efficiency: 0.75,
                enterprise_grade: true,
            },
            Provider::Cohere => ProviderPerformanceProfile {
                average_response_time: Duration::from_millis(1400),
                reliability_score: 0.94,
                speed_score: 0.82,
                quality_score: 0.90,
                cost_efficiency: 0.88,
                enterprise_grade: true,
            },
        }
    }

    /// Check if provider is European/GDPR compliant
    pub fn is_european_compliant(&self) -> bool {
        match self {
            Provider::Mistral => true, // French company, EU-native
            Provider::Cohere => true,  // Offers EU data residency
            Provider::OpenAI => false, // US-based, limited EU options
            Provider::Anthropic => false, // US-based
            Provider::AzureOpenAI => true, // Microsoft has EU data centers
            Provider::VertexAI => true, // Google has EU data centers
            Provider::AwsBedrock => true, // AWS has EU regions
            Provider::Groq => false, // US-based, limited compliance
        }
    }
}

impl std::fmt::Display for Provider {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.name())
    }
}

/// Provider performance profile for intelligent routing
#[derive(Debug, Clone, PartialEq)]
pub struct ProviderPerformanceProfile {
    pub average_response_time: Duration,
    pub reliability_score: f64,  // 0.0 to 1.0
    pub speed_score: f64,        // 0.0 to 1.0
    pub quality_score: f64,      // 0.0 to 1.0
    pub cost_efficiency: f64,    // 0.0 to 1.0
    pub enterprise_grade: bool,
}

/// Provider capabilities
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProviderCapability {
    pub streaming: bool,
    pub tool_calling: bool,
    pub vision: bool,
    pub max_tokens: u32,
    pub max_context_length: u32,
    pub supports_system_messages: bool,
    pub supports_function_calling: bool,
    pub supports_json_mode: bool,
}

impl ProviderCapability {
    /// Get capabilities for a specific provider
    pub fn for_provider(provider: Provider) -> Self {
        match provider {
            Provider::VertexAI => Self {
                streaming: true,
                tool_calling: true,
                vision: true,
                max_tokens: 200_000,
                max_context_length: 200_000,
                supports_system_messages: true,
                supports_function_calling: true,
                supports_json_mode: false,
            },
            Provider::OpenAI => Self {
                streaming: true,
                tool_calling: true,
                vision: false, // Vision not implemented yet in our system
                max_tokens: 128_000,
                max_context_length: 128_000,
                supports_system_messages: true,
                supports_function_calling: true,
                supports_json_mode: true,
            },
            Provider::AzureOpenAI => Self {
                streaming: true,
                tool_calling: true,
                vision: false, // Vision not implemented yet in our system
                max_tokens: 128_000,
                max_context_length: 128_000,
                supports_system_messages: true,
                supports_function_calling: true,
                supports_json_mode: true,
            },
            Provider::Anthropic => Self {
                streaming: true,
                tool_calling: true,
                vision: true,
                max_tokens: 200_000,
                max_context_length: 200_000,
                supports_system_messages: true,
                supports_function_calling: true,
                supports_json_mode: false,
            },
            Provider::Groq => Self {
                streaming: true,
                tool_calling: true,
                vision: false,
                max_tokens: 32_768,
                max_context_length: 128_000,
                supports_system_messages: true,
                supports_function_calling: true,
                supports_json_mode: true,
            },
            Provider::Mistral => Self {
                streaming: true,
                tool_calling: true,
                vision: false,
                max_tokens: 32_768,
                max_context_length: 128_000,
                supports_system_messages: true,
                supports_function_calling: true,
                supports_json_mode: true,
            },
            Provider::AwsBedrock => Self {
                streaming: true,
                tool_calling: true, // Depends on model family (Claude, Llama)
                vision: false, // Not implemented yet, but Claude models support vision
                max_tokens: 200_000, // Varies by model, Claude has highest
                max_context_length: 200_000,
                supports_system_messages: true,
                supports_function_calling: true,
                supports_json_mode: false,
            },
            Provider::Cohere => Self {
                streaming: true,
                tool_calling: true,
                vision: false,
                max_tokens: 4_096,
                max_context_length: 128_000,
                supports_system_messages: true,
                supports_function_calling: true,
                supports_json_mode: false,
            },
        }
    }

    /// Check if provider meets requirements
    pub fn meets_requirements(&self, requirements: &ProviderRequirements) -> bool {
        (!requirements.streaming || self.streaming) &&
        (!requirements.tool_calling || self.tool_calling) &&
        (!requirements.vision || self.vision) &&
        (!requirements.system_messages || self.supports_system_messages) &&
        (!requirements.function_calling || self.supports_function_calling) &&
        (!requirements.json_mode || self.supports_json_mode) &&
        self.max_tokens >= requirements.min_max_tokens &&
        self.max_context_length >= requirements.min_context_length
    }
}

/// Requirements for provider selection
#[derive(Debug, Clone, Default)]
pub struct ProviderRequirements {
    pub streaming: bool,
    pub tool_calling: bool,
    pub vision: bool,
    pub system_messages: bool,
    pub function_calling: bool,
    pub json_mode: bool,
    pub min_max_tokens: u32,
    pub min_context_length: u32,
    pub european_compliance: bool,
    pub enterprise_grade: bool,
}

/// Routing strategy for intelligent provider selection
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum RoutingStrategy {
    /// Balance speed, quality, and reliability (default)
    Balanced,
    /// Optimize for fastest response times
    Speed,
    /// Optimize for highest quality responses
    Quality,
    /// Optimize for enterprise requirements (reliability, compliance)
    Enterprise,
    /// Optimize for cost efficiency
    Cost,
    /// European compliance and data residency
    European,
}

impl RoutingStrategy {
    /// Get the fallback chain for this routing strategy
    pub fn fallback_chain(&self) -> Vec<Provider> {
        match self {
            RoutingStrategy::Balanced => vec![
                Provider::OpenAI,      // Most reliable, industry standard
                Provider::Anthropic,   // Strong reasoning, safety
                Provider::Cohere,      // Enterprise reliability
                Provider::Mistral,     // Cost-effective, European
                Provider::AzureOpenAI, // Enterprise infrastructure
                Provider::AwsBedrock,  // Multi-model platform
                Provider::Groq,        // Ultra-fast fallback
                Provider::VertexAI,    // Google integration
            ],
            RoutingStrategy::Speed => vec![
                Provider::Groq,        // Sub-second responses
                Provider::Mistral,     // Fast European option
                Provider::Cohere,      // Good performance
                Provider::OpenAI,      // Fast and reliable
                Provider::AzureOpenAI, // Microsoft performance
                Provider::Anthropic,   // Quality backup
                Provider::AwsBedrock,  // AWS infrastructure
                Provider::VertexAI,    // Google fallback
            ],
            RoutingStrategy::Quality => vec![
                Provider::OpenAI,      // Industry-leading quality
                Provider::Anthropic,   // Superior reasoning
                Provider::VertexAI,    // Google's quality models
                Provider::AzureOpenAI, // Enterprise OpenAI
                Provider::AwsBedrock,  // Multi-model quality
                Provider::Cohere,      // Enterprise quality
                Provider::Mistral,     // European quality
                Provider::Groq,        // Speed with quality
            ],
            RoutingStrategy::Enterprise => vec![
                Provider::AzureOpenAI, // Microsoft enterprise
                Provider::AwsBedrock,  // Amazon enterprise
                Provider::Cohere,      // Enterprise focus
                Provider::VertexAI,    // Google enterprise
                Provider::OpenAI,      // Industry standard
                Provider::Mistral,     // European enterprise
                Provider::Anthropic,   // Safety-focused
                Provider::Groq,        // Performance option
            ],
            RoutingStrategy::Cost => vec![
                Provider::Mistral,     // Competitive European pricing
                Provider::Groq,        // Cost-effective speed
                Provider::Cohere,      // Good value
                Provider::VertexAI,    // Google pricing
                Provider::AwsBedrock,  // AWS cost optimization
                Provider::OpenAI,      // Standard pricing
                Provider::Anthropic,   // Premium option
                Provider::AzureOpenAI, // Enterprise pricing
            ],
            RoutingStrategy::European => vec![
                Provider::Mistral,     // French, EU-native
                Provider::Cohere,      // EU data residency
                Provider::AzureOpenAI, // Microsoft EU data centers
                Provider::VertexAI,    // Google EU regions
                Provider::AwsBedrock,  // AWS EU regions
                Provider::OpenAI,      // Limited EU compliance
                Provider::Anthropic,   // US-based
                Provider::Groq,        // US-based
            ],
        }
    }

    /// Get scoring weights for provider selection
    pub fn scoring_weights(&self) -> ScoringWeights {
        match self {
            RoutingStrategy::Balanced => ScoringWeights {
                reliability: 0.3,
                speed: 0.25,
                quality: 0.25,
                cost: 0.1,
                enterprise: 0.1,
            },
            RoutingStrategy::Speed => ScoringWeights {
                reliability: 0.15,
                speed: 0.5,
                quality: 0.15,
                cost: 0.1,
                enterprise: 0.1,
            },
            RoutingStrategy::Quality => ScoringWeights {
                reliability: 0.25,
                speed: 0.1,
                quality: 0.5,
                cost: 0.05,
                enterprise: 0.1,
            },
            RoutingStrategy::Enterprise => ScoringWeights {
                reliability: 0.4,
                speed: 0.1,
                quality: 0.2,
                cost: 0.05,
                enterprise: 0.25,
            },
            RoutingStrategy::Cost => ScoringWeights {
                reliability: 0.2,
                speed: 0.15,
                quality: 0.15,
                cost: 0.4,
                enterprise: 0.1,
            },
            RoutingStrategy::European => ScoringWeights {
                reliability: 0.25,
                speed: 0.15,
                quality: 0.25,
                cost: 0.15,
                enterprise: 0.2,
            },
        }
    }
}

/// Scoring weights for provider selection
#[derive(Debug, Clone, Copy)]
pub struct ScoringWeights {
    pub reliability: f64,
    pub speed: f64,
    pub quality: f64,
    pub cost: f64,
    pub enterprise: f64,
}

/// Provider health metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderHealth {
    pub provider: Provider,
    pub status: HealthStatus,
    pub success_rate: f64,
    pub average_response_time: Duration,
    pub consecutive_failures: u32,
    pub last_success: Option<SystemTime>,
    pub last_failure: Option<SystemTime>,
    pub rate_limit_reset: Option<SystemTime>,
    pub circuit_breaker_state: CircuitBreakerState,
    pub total_requests: u64,
    pub successful_requests: u64,
    pub failed_requests: u64,
}

impl ProviderHealth {
    /// Create new health metrics for a provider
    pub fn new(provider: Provider) -> Self {
        Self {
            provider,
            status: HealthStatus::Healthy,
            success_rate: 1.0,
            average_response_time: Duration::from_millis(1000),
            consecutive_failures: 0,
            last_success: None,
            last_failure: None,
            rate_limit_reset: None,
            circuit_breaker_state: CircuitBreakerState::Closed,
            total_requests: 0,
            successful_requests: 0,
            failed_requests: 0,
        }
    }

    /// Update health metrics with a successful request
    pub fn record_success(&mut self, response_time: Duration) {
        self.consecutive_failures = 0;
        self.last_success = Some(SystemTime::now());
        self.total_requests += 1;
        self.successful_requests += 1;
        
        // Update average response time with exponential moving average
        let alpha = 0.1; // Smoothing factor
        let current_ms = self.average_response_time.as_millis() as f64;
        let new_ms = response_time.as_millis() as f64;
        let updated_ms = (alpha * new_ms + (1.0 - alpha) * current_ms) as u64;
        self.average_response_time = Duration::from_millis(updated_ms);

        // Update success rate
        self.success_rate = self.successful_requests as f64 / self.total_requests as f64;

        // Update status based on metrics
        self.update_status();
    }

    /// Update health metrics with a failed request
    pub fn record_failure(&mut self, error_type: &ProviderErrorType) {
        self.consecutive_failures += 1;
        self.last_failure = Some(SystemTime::now());
        self.total_requests += 1;
        self.failed_requests += 1;

        // Handle rate limiting
        if matches!(error_type, ProviderErrorType::RateLimit { reset_time }) {
            if let ProviderErrorType::RateLimit { reset_time } = error_type {
                self.rate_limit_reset = Some(*reset_time);
            }
        }

        // Update success rate
        self.success_rate = if self.total_requests > 0 {
            self.successful_requests as f64 / self.total_requests as f64
        } else {
            0.0
        };

        // Update status based on metrics
        self.update_status();
    }

    /// Update health status based on current metrics
    fn update_status(&mut self) {
        if self.consecutive_failures >= 5 || self.success_rate < 0.5 {
            self.status = HealthStatus::Unhealthy;
        } else if self.consecutive_failures >= 3 || self.success_rate < 0.8 {
            self.status = HealthStatus::Degraded;
        } else if self.rate_limit_reset.is_some() {
            if let Some(reset_time) = self.rate_limit_reset {
                if SystemTime::now() < reset_time {
                    self.status = HealthStatus::RateLimited;
                } else {
                    self.rate_limit_reset = None;
                    self.status = HealthStatus::Healthy;
                }
            }
        } else {
            self.status = HealthStatus::Healthy;
        }
    }

    /// Check if provider is available for requests
    pub fn is_available(&self) -> bool {
        matches!(self.status, HealthStatus::Healthy | HealthStatus::Degraded) &&
        !matches!(self.circuit_breaker_state, CircuitBreakerState::Open)
    }

    /// Get provider priority score (higher is better)
    pub fn priority_score(&self) -> f64 {
        let base_score = match self.status {
            HealthStatus::Healthy => 100.0,
            HealthStatus::Degraded => 60.0,
            HealthStatus::RateLimited => 20.0,
            HealthStatus::Unhealthy => 0.0,
        };

        let circuit_breaker_penalty = match self.circuit_breaker_state {
            CircuitBreakerState::Closed => 0.0,
            CircuitBreakerState::HalfOpen => -20.0,
            CircuitBreakerState::Open => -100.0,
        };

        let response_time_penalty = (self.average_response_time.as_millis() as f64 / 1000.0) * -5.0;
        let success_rate_bonus = self.success_rate * 20.0;

        (base_score + circuit_breaker_penalty + response_time_penalty + success_rate_bonus).max(0.0)
    }
}

/// Provider health status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum HealthStatus {
    Healthy,
    Degraded,
    RateLimited,
    Unhealthy,
}

/// Circuit breaker state
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CircuitBreakerState {
    Closed,   // Normal operation
    HalfOpen, // Testing if service recovered
    Open,     // Blocking requests
}

/// Provider-specific error types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ProviderErrorType {
    Authentication,
    Authorization,
    RateLimit { reset_time: SystemTime },
    QuotaExceeded,
    ModelNotAvailable,
    ServiceUnavailable,
    Timeout,
    NetworkError,
    InvalidRequest,
    InternalError,
    Unknown { message: String },
}

impl std::fmt::Display for ProviderErrorType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ProviderErrorType::Authentication => write!(f, "Authentication error"),
            ProviderErrorType::Authorization => write!(f, "Authorization error"),
            ProviderErrorType::RateLimit { reset_time } => {
                write!(f, "Rate limit exceeded, resets at {:?}", reset_time)
            }
            ProviderErrorType::QuotaExceeded => write!(f, "Quota exceeded"),
            ProviderErrorType::ModelNotAvailable => write!(f, "Model not available"),
            ProviderErrorType::ServiceUnavailable => write!(f, "Service unavailable"),
            ProviderErrorType::Timeout => write!(f, "Request timeout"),
            ProviderErrorType::NetworkError => write!(f, "Network error"),
            ProviderErrorType::InvalidRequest => write!(f, "Invalid request"),
            ProviderErrorType::InternalError => write!(f, "Internal error"),
            ProviderErrorType::Unknown { message } => write!(f, "Unknown error: {}", message),
        }
    }
}

impl ProviderErrorType {
    /// Convert to OpenAI-compatible error type
    pub fn to_openai_error(&self) -> ErrorType {
        match self {
            Self::Authentication => ErrorType::AuthenticationError,
            Self::Authorization => ErrorType::PermissionError,
            Self::RateLimit { .. } => ErrorType::RateLimitError,
            Self::QuotaExceeded => ErrorType::RateLimitError,
            Self::ModelNotAvailable => ErrorType::NotFoundError,
            Self::ServiceUnavailable => ErrorType::OverloadedError,
            Self::Timeout => ErrorType::TimeoutError,
            Self::NetworkError => ErrorType::ServerError,
            Self::InvalidRequest => ErrorType::InvalidRequestError,
            Self::InternalError => ErrorType::ServerError,
            Self::Unknown { .. } => ErrorType::ServerError,
        }
    }

    /// Check if error is retryable
    pub fn is_retryable(&self) -> bool {
        matches!(
            self,
            Self::RateLimit { .. } |
            Self::ServiceUnavailable |
            Self::Timeout |
            Self::NetworkError |
            Self::InternalError
        )
    }
}

/// Provider choice for a request
#[derive(Debug, Clone)]
pub struct ProviderChoice {
    pub provider: Provider,
    pub model: String,
    pub reason: String,
    pub confidence: f64,
    pub estimated_response_time: Duration,
    pub routing_strategy: RoutingStrategy,
}

/// Fallback action after provider failure
#[derive(Debug, Clone)]
pub enum FallbackAction {
    Retry {
        provider: Provider,
        delay: Duration,
        attempt: u32,
    },
    Switch {
        from: Provider,
        to: Provider,
        reason: String,
    },
    Fail {
        reason: String,
        last_errors: Vec<(Provider, ProviderErrorType)>,
    },
}

/// Retry configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetryConfig {
    pub max_attempts: u32,
    pub base_delay: Duration,
    pub max_delay: Duration,
    pub exponential_base: f64,
    pub jitter: bool,
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            max_attempts: 3,
            base_delay: Duration::from_millis(1000),
            max_delay: Duration::from_secs(30),
            exponential_base: 2.0,
            jitter: true,
        }
    }
}

/// Circuit breaker configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CircuitBreakerConfig {
    pub failure_threshold: u32,
    pub timeout: Duration,
    pub success_threshold: u32,
}

impl Default for CircuitBreakerConfig {
    fn default() -> Self {
        Self {
            failure_threshold: 5,
            timeout: Duration::from_secs(60),
            success_threshold: 3,
        }
    }
}

/// Fallback system configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FallbackConfig {
    pub enabled: bool,
    pub routing_strategy: RoutingStrategy,
    pub provider_priority: Vec<Provider>,
    pub retry_config: RetryConfig,
    pub circuit_breaker_config: CircuitBreakerConfig,
    pub health_check_interval: Duration,
    pub provider_timeout: Duration,
    pub intelligent_routing: IntelligentRoutingConfig,
}

impl Default for FallbackConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            routing_strategy: RoutingStrategy::Balanced,
            provider_priority: RoutingStrategy::Balanced.fallback_chain(),
            retry_config: RetryConfig::default(),
            circuit_breaker_config: CircuitBreakerConfig::default(),
            health_check_interval: Duration::from_secs(30),
            provider_timeout: Duration::from_secs(30),
            intelligent_routing: IntelligentRoutingConfig::default(),
        }
    }
}

/// Configuration for intelligent routing features
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntelligentRoutingConfig {
    /// Enable dynamic provider ordering based on health
    pub dynamic_ordering: bool,
    /// Enable cost-aware routing
    pub cost_aware: bool,
    /// Enable regional compliance routing
    pub regional_compliance: bool,
    /// Enable performance-based routing
    pub performance_based: bool,
    /// Minimum confidence threshold for provider selection
    pub min_confidence_threshold: f64,
}

impl Default for IntelligentRoutingConfig {
    fn default() -> Self {
        Self {
            dynamic_ordering: true,
            cost_aware: true,
            regional_compliance: true,
            performance_based: true,
            min_confidence_threshold: 0.7,
        }
    }
}

/// Request metrics for monitoring
#[derive(Debug, Clone)]
pub struct RequestMetrics {
    pub request_id: String,
    pub provider: Provider,
    pub model: String,
    pub start_time: Instant,
    pub end_time: Option<Instant>,
    pub response_time: Option<Duration>,
    pub success: bool,
    pub error: Option<ProviderErrorType>,
    pub retry_count: u32,
    pub fallback_used: bool,
    pub routing_strategy: RoutingStrategy,
}

impl RequestMetrics {
    pub fn new(request_id: String, provider: Provider, model: String) -> Self {
        Self {
            request_id,
            provider,
            model,
            start_time: Instant::now(),
            end_time: None,
            response_time: None,
            success: false,
            error: None,
            retry_count: 0,
            fallback_used: false,
            routing_strategy: RoutingStrategy::Balanced,
        }
    }

    pub fn complete_success(&mut self) {
        let now = Instant::now();
        self.end_time = Some(now);
        self.response_time = Some(now.duration_since(self.start_time));
        self.success = true;
    }

    pub fn complete_failure(&mut self, error: ProviderErrorType) {
        let now = Instant::now();
        self.end_time = Some(now);
        self.response_time = Some(now.duration_since(self.start_time));
        self.success = false;
        self.error = Some(error);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_all_providers_included() {
        let providers = Provider::all();
        assert_eq!(providers.len(), 8);
        assert!(providers.contains(&Provider::OpenAI));
        assert!(providers.contains(&Provider::Anthropic));
        assert!(providers.contains(&Provider::AzureOpenAI));
        assert!(providers.contains(&Provider::Groq));
        assert!(providers.contains(&Provider::Mistral));
        assert!(providers.contains(&Provider::AwsBedrock));
        assert!(providers.contains(&Provider::Cohere));
        assert!(providers.contains(&Provider::VertexAI));
    }

    #[test]
    fn test_provider_capabilities() {
        let vertex_caps = ProviderCapability::for_provider(Provider::VertexAI);
        assert!(vertex_caps.streaming);
        assert!(vertex_caps.tool_calling);
        assert!(vertex_caps.vision);

        let groq_caps = ProviderCapability::for_provider(Provider::Groq);
        assert!(groq_caps.streaming);
        assert!(groq_caps.tool_calling);
        assert!(!groq_caps.vision); // Groq doesn't support vision

        let azure_caps = ProviderCapability::for_provider(Provider::AzureOpenAI);
        assert!(azure_caps.streaming);
        assert!(azure_caps.tool_calling);
        assert!(azure_caps.supports_json_mode);

        let mistral_caps = ProviderCapability::for_provider(Provider::Mistral);
        assert!(mistral_caps.streaming);
        assert!(mistral_caps.tool_calling);
        assert!(mistral_caps.supports_json_mode);

        let cohere_caps = ProviderCapability::for_provider(Provider::Cohere);
        assert!(cohere_caps.streaming);
        assert!(cohere_caps.tool_calling);
        assert!(!cohere_caps.supports_json_mode);
    }

    #[test]
    fn test_provider_model_support() {
        assert!(Provider::VertexAI.supports_model("claude-3-5-sonnet@20241022"));
        assert!(Provider::Groq.supports_model("llama-3.1-70b-versatile"));
        assert!(Provider::AzureOpenAI.supports_model("gpt-4o"));
        assert!(Provider::Mistral.supports_model("mistral-large-latest"));
        assert!(Provider::Cohere.supports_model("command-r-plus"));
        assert!(!Provider::VertexAI.supports_model("gpt-4o"));
    }

    #[test]
    fn test_routing_strategies() {
        let balanced_chain = RoutingStrategy::Balanced.fallback_chain();
        assert_eq!(balanced_chain[0], Provider::OpenAI);
        assert_eq!(balanced_chain[1], Provider::Anthropic);

        let speed_chain = RoutingStrategy::Speed.fallback_chain();
        assert_eq!(speed_chain[0], Provider::Groq);
        assert_eq!(speed_chain[1], Provider::Mistral);

        let enterprise_chain = RoutingStrategy::Enterprise.fallback_chain();
        assert_eq!(enterprise_chain[0], Provider::AzureOpenAI);
        assert_eq!(enterprise_chain[1], Provider::AwsBedrock);

        let european_chain = RoutingStrategy::European.fallback_chain();
        assert_eq!(european_chain[0], Provider::Mistral);
        assert_eq!(european_chain[1], Provider::Cohere);
    }

    #[test]
    fn test_european_compliance() {
        assert!(Provider::Mistral.is_european_compliant());
        assert!(Provider::Cohere.is_european_compliant());
        assert!(Provider::AzureOpenAI.is_european_compliant());
        assert!(Provider::VertexAI.is_european_compliant());
        assert!(Provider::AwsBedrock.is_european_compliant());
        assert!(!Provider::OpenAI.is_european_compliant());
        assert!(!Provider::Anthropic.is_european_compliant());
        assert!(!Provider::Groq.is_european_compliant());
    }

    #[test]
    fn test_provider_health_metrics() {
        let mut health = ProviderHealth::new(Provider::VertexAI);
        assert_eq!(health.status, HealthStatus::Healthy);
        assert_eq!(health.consecutive_failures, 0);

        // Record failures
        health.record_failure(&ProviderErrorType::ServiceUnavailable);
        health.record_failure(&ProviderErrorType::Timeout);
        health.record_failure(&ProviderErrorType::NetworkError);

        assert_eq!(health.consecutive_failures, 3);
        assert_eq!(health.status, HealthStatus::Degraded);

        // Record success
        health.record_success(Duration::from_millis(500));
        assert_eq!(health.consecutive_failures, 0);
        assert_eq!(health.status, HealthStatus::Healthy);
    }

    #[test]
    fn test_error_type_conversion() {
        let auth_error = ProviderErrorType::Authentication;
        assert_eq!(auth_error.to_openai_error(), ErrorType::AuthenticationError);
        assert!(!auth_error.is_retryable());

        let timeout_error = ProviderErrorType::Timeout;
        assert_eq!(timeout_error.to_openai_error(), ErrorType::TimeoutError);
        assert!(timeout_error.is_retryable());

        let rate_limit_error = ProviderErrorType::RateLimit {
            reset_time: SystemTime::now() + Duration::from_secs(60)
        };
        assert_eq!(rate_limit_error.to_openai_error(), ErrorType::RateLimitError);
        assert!(rate_limit_error.is_retryable());
    }

    #[test]
    fn test_provider_priority_score() {
        let mut health = ProviderHealth::new(Provider::VertexAI);
        let initial_score = health.priority_score();

        // Record failures to decrease score
        health.record_failure(&ProviderErrorType::Timeout);
        health.record_failure(&ProviderErrorType::Timeout);
        health.record_failure(&ProviderErrorType::Timeout);

        let degraded_score = health.priority_score();
        assert!(degraded_score < initial_score);

        // Record success to increase score
        health.record_success(Duration::from_millis(200));
        let recovered_score = health.priority_score();
        assert!(recovered_score > degraded_score);
    }

    #[test]
    fn test_provider_requirements_matching() {
        let requirements = ProviderRequirements {
            streaming: true,
            tool_calling: true,
            vision: false,
            system_messages: true,
            function_calling: true,
            json_mode: false,
            min_max_tokens: 1000,
            min_context_length: 8000,
            european_compliance: false,
            enterprise_grade: false,
        };

        let openai_caps = ProviderCapability::for_provider(Provider::OpenAI);
        assert!(openai_caps.meets_requirements(&requirements));

        let vision_requirements = ProviderRequirements {
            vision: true,
            ..requirements
        };

        let vertex_caps = ProviderCapability::for_provider(Provider::VertexAI);
        assert!(vertex_caps.meets_requirements(&vision_requirements));

        let groq_caps = ProviderCapability::for_provider(Provider::Groq);
        assert!(!groq_caps.meets_requirements(&vision_requirements)); // Groq doesn't support vision
    }

    #[test]
    fn test_performance_characteristics() {
        let groq_perf = Provider::Groq.performance_characteristics();
        assert!(groq_perf.speed_score > 0.95); // Groq should have highest speed score
        assert!(!groq_perf.enterprise_grade); // But not enterprise grade

        let openai_perf = Provider::OpenAI.performance_characteristics();
        assert!(openai_perf.reliability_score > 0.95); // OpenAI should have high reliability
        assert!(openai_perf.enterprise_grade); // And be enterprise grade

        let mistral_perf = Provider::Mistral.performance_characteristics();
        assert!(mistral_perf.cost_efficiency > 0.90); // Mistral should be cost efficient
        assert!(mistral_perf.enterprise_grade); // And enterprise grade
    }

    #[test]
    fn test_routing_strategy_weights() {
        let speed_weights = RoutingStrategy::Speed.scoring_weights();
        assert!(speed_weights.speed > 0.4); // Speed strategy prioritizes speed

        let quality_weights = RoutingStrategy::Quality.scoring_weights();
        assert!(quality_weights.quality > 0.4); // Quality strategy prioritizes quality

        let enterprise_weights = RoutingStrategy::Enterprise.scoring_weights();
        assert!(enterprise_weights.reliability > 0.3); // Enterprise prioritizes reliability
        assert!(enterprise_weights.enterprise > 0.2); // And enterprise features

        let cost_weights = RoutingStrategy::Cost.scoring_weights();
        assert!(cost_weights.cost > 0.3); // Cost strategy prioritizes cost
    }
}