use serde::{Deserialize, Serialize};
use std::env;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub host: String,
    pub port: u16,
    pub log_level: String,
    pub supabase: SupabaseConfig,
    pub vertex: super::VertexConfig,
    pub embedding: super::EmbeddingConfig,
    pub model_cache_dir: String,
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
        }
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
