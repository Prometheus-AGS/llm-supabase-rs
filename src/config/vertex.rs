use serde::{Deserialize, Serialize};
use std::env;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VertexConfig {
    pub project_id: String,
    pub location: String,
    pub credentials_path: String,
    pub default_model: String,
}

impl Default for VertexConfig {
    fn default() -> Self {
        Self {
            project_id: "".to_string(),
            location: "us-east5".to_string(),
            credentials_path: "./gcp-credentials.json".to_string(),
            default_model: "claude-sonnet-4-5@20250929".to_string(),
        }
    }
}

impl VertexConfig {
    pub fn from_env() -> anyhow::Result<Self> {
        let config = Self {
            project_id: env::var("GCP_PROJECT_ID")?,
            location: env::var("GCP_LOCATION").unwrap_or_else(|_| "us-east5".to_string()),
            credentials_path: env::var("GOOGLE_APPLICATION_CREDENTIALS")
                .unwrap_or_else(|_| "./gcp-credentials.json".to_string()),
            default_model: env::var("DEFAULT_MODEL")
                .unwrap_or_else(|_| "claude-sonnet-4-5@20250929".to_string()),
        };

        Ok(config)
    }

    /// Validate Vertex AI configuration
    pub fn validate(&self) -> anyhow::Result<()> {
        if self.project_id.is_empty() {
            anyhow::bail!("GCP project ID is required");
        }

        if self.location.is_empty() {
            anyhow::bail!("GCP location is required");
        }

        // Validate location format (should be like us-central1, europe-west1, etc.)
        if !self.location.contains('-') {
            anyhow::bail!("Invalid GCP location format: {}", self.location);
        }

        if self.credentials_path.is_empty() {
            anyhow::bail!("GCP credentials path is required");
        }

        // Check if credentials file exists
        if !std::path::Path::new(&self.credentials_path).exists() {
            anyhow::bail!("GCP credentials file not found: {}", self.credentials_path);
        }

        if self.default_model.is_empty() {
            anyhow::bail!("Default model is required");
        }

        // Validate that it's a Claude model
        if !self.default_model.contains("claude") {
            anyhow::bail!("Default model must be a Claude model, got: {}", self.default_model);
        }

        Ok(())
    }
}
