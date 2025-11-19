use serde::{Deserialize, Serialize};
use std::env;
use std::time::Duration;

// Import fallback configuration types
use crate::features::provider_fallback::models::{FallbackConfig, Provider};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub host: String,
    pub port: u16,
    pub log_level: String,
    pub supabase: SupabaseConfig,
    pub vertex: super::VertexConfig,
    pub embedding: super::EmbeddingConfig,
    pub model_cache_dir: String,
    pub fallback: FallbackConfigExt,
}

/// Extended fallback configuration with environment loading
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FallbackConfigExt {
    pub enabled: bool,
    pub provider_priority: Vec<String>, // Provider names as strings for config
    pub retry_max_attempts: u32,
    pub retry_base_delay_ms: u64,
    pub retry_max_delay_ms: u64,
    pub retry_exponential_base: f64,
    pub retry_jitter: bool,
    pub circuit_breaker_failure_threshold: u32,
    pub circuit_breaker_timeout_seconds: u64,
    pub circuit_breaker_success_threshold: u32,
    pub health_check_interval_seconds: u64,
    pub provider_timeout_seconds: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SupabaseConfig {
    pub url: String,
    pub anon_key: String,
    pub service_role_key: String,
    pub jwt_secret: String,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            host: "0.0.0.0".to_string(),
            port: 8080,
            log_level: "info".to_string(),
            supabase: SupabaseConfig::default(),
            vertex: super::VertexConfig::default(),
            embedding: super::EmbeddingConfig::default(),
            model_cache_dir: "./models".to_string(),
            fallback: FallbackConfigExt::default(),
        }
    }
}

impl Default for FallbackConfigExt {
    fn default() -> Self {
        Self {
            enabled: true,
            provider_priority: vec!["vertex_ai".to_string(), "groq".to_string()],
            retry_max_attempts: 3,
            retry_base_delay_ms: 1000,
            retry_max_delay_ms: 30000,
            retry_exponential_base: 2.0,
            retry_jitter: true,
            circuit_breaker_failure_threshold: 5,
            circuit_breaker_timeout_seconds: 60,
            circuit_breaker_success_threshold: 3,
            health_check_interval_seconds: 30,
            provider_timeout_seconds: 30,
        }
    }
}

impl Default for SupabaseConfig {
    fn default() -> Self {
        Self {
            url: "".to_string(),
            anon_key: "".to_string(),
            service_role_key: "".to_string(),
            jwt_secret: "".to_string(),
        }
    }
}

impl AppConfig {
    pub fn from_env() -> anyhow::Result<Self> {
        dotenv::dotenv().ok(); // Load .env file if present

        let config = Self {
            host: env::var("HOST").unwrap_or_else(|_| "0.0.0.0".to_string()),
            port: env::var("PORT")
                .unwrap_or_else(|_| "8080".to_string())
                .parse()
                .unwrap_or(8080),
            log_level: env::var("LOG_LEVEL").unwrap_or_else(|_| "info".to_string()),
            supabase: SupabaseConfig {
                url: env::var("SUPABASE_URL")?,
                anon_key: env::var("SUPABASE_ANON_KEY")?,
                service_role_key: env::var("SUPABASE_SERVICE_ROLE_KEY")?,
                jwt_secret: env::var("SUPABASE_JWT_SECRET")?,
            },
            vertex: super::VertexConfig::from_env()?,
            embedding: super::EmbeddingConfig::from_env()?,
            model_cache_dir: env::var("MODEL_CACHE_DIR").unwrap_or_else(|_| "./models".to_string()),
            fallback: FallbackConfigExt::from_env(),
        };

        // Validate configuration
        config.validate()?;

        Ok(config)
    }

    /// Validate the configuration for completeness and correctness
    pub fn validate(&self) -> anyhow::Result<()> {
        use anyhow::Context;

        // Validate host
        if self.host.is_empty() {
            anyhow::bail!("Host cannot be empty");
        }

        // Validate port range
        if self.port == 0 {
            anyhow::bail!("Port must be greater than 0");
        }

        // Validate log level
        match self.log_level.to_lowercase().as_str() {
            "trace" | "debug" | "info" | "warn" | "error" => {},
            _ => anyhow::bail!("Invalid log level: {}. Must be one of: trace, debug, info, warn, error", self.log_level),
        }

        // Validate Supabase configuration
        self.supabase.validate()
            .context("Supabase configuration validation failed")?;

        // Validate Vertex AI configuration
        self.vertex.validate()
            .context("Vertex AI configuration validation failed")?;

        // Validate fallback configuration
        self.fallback.validate()
            .context("Fallback configuration validation failed")?;

        // Validate model cache directory
        if self.model_cache_dir.is_empty() {
            anyhow::bail!("Model cache directory cannot be empty");
        }

        // Ensure model cache directory exists or can be created
        std::fs::create_dir_all(&self.model_cache_dir)
            .with_context(|| format!("Failed to create model cache directory: {}", self.model_cache_dir))?;

        tracing::info!("Configuration validation passed");
        Ok(())
    }

    /// Get the complete server address
    pub fn server_address(&self) -> String {
        format!("{}:{}", self.host, self.port)
    }

    /// Check if the configuration is ready for production
    pub fn is_production_ready(&self) -> bool {
        // Check critical configuration for production
        !self.supabase.url.is_empty() &&
        !self.supabase.jwt_secret.is_empty() &&
        !self.vertex.project_id.is_empty() &&
        !self.vertex.credentials_path.is_empty() &&
        self.log_level != "trace" &&
        self.log_level != "debug"
    }

    /// Get configuration summary for logging
    pub fn summary(&self) -> ConfigSummary {
        ConfigSummary {
            server_address: self.server_address(),
            log_level: self.log_level.clone(),
            supabase_configured: !self.supabase.url.is_empty(),
            vertex_project: self.vertex.project_id.clone(),
            vertex_region: self.vertex.location.clone(),
            model_cache_dir: self.model_cache_dir.clone(),
            production_ready: self.is_production_ready(),
            fallback_enabled: self.fallback.enabled,
            fallback_providers: self.fallback.provider_priority.clone(),
        }
    }

    /// Convert to provider fallback configuration
    pub fn to_fallback_config(&self) -> anyhow::Result<FallbackConfig> {
        let provider_priority = self.fallback.parse_provider_priority()?;
        
        Ok(FallbackConfig {
            enabled: self.fallback.enabled,
            routing_strategy: crate::features::provider_fallback::models::RoutingStrategy::Balanced,
            provider_priority,
            retry_config: crate::features::provider_fallback::models::RetryConfig {
                max_attempts: self.fallback.retry_max_attempts,
                base_delay: Duration::from_millis(self.fallback.retry_base_delay_ms),
                max_delay: Duration::from_millis(self.fallback.retry_max_delay_ms),
                exponential_base: self.fallback.retry_exponential_base,
                jitter: self.fallback.retry_jitter,
            },
            circuit_breaker_config: crate::features::provider_fallback::models::CircuitBreakerConfig {
                failure_threshold: self.fallback.circuit_breaker_failure_threshold,
                timeout: Duration::from_secs(self.fallback.circuit_breaker_timeout_seconds),
                success_threshold: self.fallback.circuit_breaker_success_threshold,
            },
            health_check_interval: Duration::from_secs(self.fallback.health_check_interval_seconds),
            provider_timeout: Duration::from_secs(self.fallback.provider_timeout_seconds),
            intelligent_routing: crate::features::provider_fallback::models::IntelligentRoutingConfig::default(),
        })
    }
}

/// Configuration summary for safe logging (excludes sensitive data)
#[derive(Debug, Serialize)]
pub struct ConfigSummary {
    pub server_address: String,
    pub log_level: String,
    pub supabase_configured: bool,
    pub vertex_project: String,
    pub vertex_region: String,
    pub model_cache_dir: String,
    pub production_ready: bool,
    pub fallback_enabled: bool,
    pub fallback_providers: Vec<String>,
}

impl SupabaseConfig {
    /// Validate Supabase configuration
    pub fn validate(&self) -> anyhow::Result<()> {
        if self.url.is_empty() {
            anyhow::bail!("Supabase URL is required");
        }

        if !self.url.starts_with("https://") {
            anyhow::bail!("Supabase URL must start with https://");
        }

        if self.anon_key.is_empty() {
            anyhow::bail!("Supabase anonymous key is required");
        }

        if self.service_role_key.is_empty() {
            anyhow::bail!("Supabase service role key is required");
        }

        if self.jwt_secret.is_empty() {
            anyhow::bail!("Supabase JWT secret is required");
        }

        if self.jwt_secret.len() < 32 {
            anyhow::bail!("Supabase JWT secret must be at least 32 characters long");
        }

        Ok(())
    }

    /// Check if this is a valid Supabase configuration
    pub fn is_configured(&self) -> bool {
        !self.url.is_empty() &&
        !self.anon_key.is_empty() &&
        !self.service_role_key.is_empty() &&
        !self.jwt_secret.is_empty()
    }
}

impl FallbackConfigExt {
    /// Load fallback configuration from environment variables
    pub fn from_env() -> Self {
        Self {
            enabled: env::var("FALLBACK_ENABLED")
                .unwrap_or_else(|_| "true".to_string())
                .parse()
                .unwrap_or(true),
            provider_priority: env::var("FALLBACK_PROVIDER_PRIORITY")
                .unwrap_or_else(|_| "vertex_ai,groq".to_string())
                .split(',')
                .map(|s| s.trim().to_string())
                .collect(),
            retry_max_attempts: env::var("FALLBACK_RETRY_MAX_ATTEMPTS")
                .unwrap_or_else(|_| "3".to_string())
                .parse()
                .unwrap_or(3),
            retry_base_delay_ms: env::var("FALLBACK_RETRY_BASE_DELAY_MS")
                .unwrap_or_else(|_| "1000".to_string())
                .parse()
                .unwrap_or(1000),
            retry_max_delay_ms: env::var("FALLBACK_RETRY_MAX_DELAY_MS")
                .unwrap_or_else(|_| "30000".to_string())
                .parse()
                .unwrap_or(30000),
            retry_exponential_base: env::var("FALLBACK_RETRY_EXPONENTIAL_BASE")
                .unwrap_or_else(|_| "2.0".to_string())
                .parse()
                .unwrap_or(2.0),
            retry_jitter: env::var("FALLBACK_RETRY_JITTER")
                .unwrap_or_else(|_| "true".to_string())
                .parse()
                .unwrap_or(true),
            circuit_breaker_failure_threshold: env::var("FALLBACK_CB_FAILURE_THRESHOLD")
                .unwrap_or_else(|_| "5".to_string())
                .parse()
                .unwrap_or(5),
            circuit_breaker_timeout_seconds: env::var("FALLBACK_CB_TIMEOUT_SECONDS")
                .unwrap_or_else(|_| "60".to_string())
                .parse()
                .unwrap_or(60),
            circuit_breaker_success_threshold: env::var("FALLBACK_CB_SUCCESS_THRESHOLD")
                .unwrap_or_else(|_| "3".to_string())
                .parse()
                .unwrap_or(3),
            health_check_interval_seconds: env::var("FALLBACK_HEALTH_CHECK_INTERVAL_SECONDS")
                .unwrap_or_else(|_| "30".to_string())
                .parse()
                .unwrap_or(30),
            provider_timeout_seconds: env::var("FALLBACK_PROVIDER_TIMEOUT_SECONDS")
                .unwrap_or_else(|_| "30".to_string())
                .parse()
                .unwrap_or(30),
        }
    }

    /// Parse provider priority strings into Provider enum values
    pub fn parse_provider_priority(&self) -> anyhow::Result<Vec<Provider>> {
        let mut providers = Vec::new();
        
        for provider_str in &self.provider_priority {
            let provider = match provider_str.to_lowercase().as_str() {
                "vertex_ai" | "vertexai" | "vertex" => Provider::VertexAI,
                "groq" => Provider::Groq,
                _ => {
                    tracing::warn!("Unknown provider '{}' in priority list, skipping", provider_str);
                    continue;
                }
            };
            providers.push(provider);
        }

        if providers.is_empty() {
            anyhow::bail!("No valid providers found in priority list: {:?}", self.provider_priority);
        }

        Ok(providers)
    }

    /// Validate fallback configuration
    pub fn validate(&self) -> anyhow::Result<()> {
        if self.retry_max_attempts == 0 {
            anyhow::bail!("retry_max_attempts must be greater than 0");
        }

        if self.retry_base_delay_ms == 0 {
            anyhow::bail!("retry_base_delay_ms must be greater than 0");
        }

        if self.retry_max_delay_ms < self.retry_base_delay_ms {
            anyhow::bail!("retry_max_delay_ms must be >= retry_base_delay_ms");
        }

        if self.retry_exponential_base <= 0.0 {
            anyhow::bail!("retry_exponential_base must be greater than 0");
        }

        if self.circuit_breaker_failure_threshold == 0 {
            anyhow::bail!("circuit_breaker_failure_threshold must be greater than 0");
        }

        if self.circuit_breaker_timeout_seconds == 0 {
            anyhow::bail!("circuit_breaker_timeout_seconds must be greater than 0");
        }

        if self.circuit_breaker_success_threshold == 0 {
            anyhow::bail!("circuit_breaker_success_threshold must be greater than 0");
        }

        if self.health_check_interval_seconds == 0 {
            anyhow::bail!("health_check_interval_seconds must be greater than 0");
        }

        if self.provider_timeout_seconds == 0 {
            anyhow::bail!("provider_timeout_seconds must be greater than 0");
        }

        // Validate provider priority list
        self.parse_provider_priority()?;

        Ok(())
    }
}

/// Application state configuration status
#[derive(Debug, Clone)]
pub struct AppStateHealth {
    pub config_valid: bool,
    pub supabase_configured: bool,
    pub vertex_configured: bool,
    pub production_ready: bool,
    pub startup_time: std::time::SystemTime,
}

impl AppStateHealth {
    pub fn new(config: &AppConfig) -> Self {
        Self {
            config_valid: config.validate().is_ok(),
            supabase_configured: config.supabase.is_configured(),
            vertex_configured: !config.vertex.project_id.is_empty(),
            production_ready: config.is_production_ready(),
            startup_time: std::time::SystemTime::now(),
        }
    }

    pub fn uptime_seconds(&self) -> u64 {
        self.startup_time
            .elapsed()
            .map(|d| d.as_secs())
            .unwrap_or(0)
    }
}
