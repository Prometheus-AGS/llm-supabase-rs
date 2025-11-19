//! Azure OpenAI types and configuration structures
//!
//! This module defines the Azure OpenAI-specific types, configuration, and models
//! that differ from the standard OpenAI implementation.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::env;
use tracing::{debug, warn};

/// Azure OpenAI configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AzureOpenAIConfig {
    /// Azure OpenAI API key
    pub api_key: String,

    /// Azure OpenAI endpoint (e.g., "https://your-resource.openai.azure.com")
    pub endpoint: String,

    /// Deployment name (maps to model)
    pub deployment: String,

    /// API version to use (default: "2024-02-15-preview")
    pub api_version: String,

    /// Request timeout in seconds (default: 120)
    pub timeout: Option<u64>,

    /// Maximum retries for failed requests (default: 3)
    pub max_retries: Option<u32>,

    /// Base delay between retries in milliseconds (default: 1000)
    pub retry_delay_ms: Option<u64>,

    /// Whether to use exponential backoff (default: true)
    pub exponential_backoff: Option<bool>,

    /// Custom user agent string (optional)
    pub user_agent: Option<String>,

    /// Additional deployment mappings (model name -> deployment name)
    pub deployment_mappings: Option<std::collections::HashMap<String, String>>,

    /// Azure AD authentication settings (optional)
    pub azure_ad: Option<AzureADConfig>,
}

/// Azure AD authentication configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AzureADConfig {
    /// Azure AD tenant ID
    pub tenant_id: String,

    /// Azure AD client ID
    pub client_id: String,

    /// Azure AD client secret
    pub client_secret: String,

    /// Azure AD scope (default: "https://cognitiveservices.azure.com/.default")
    pub scope: Option<String>,
}

/// Azure OpenAI model information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AzureOpenAIModel {
    /// Model/deployment name
    pub name: String,

    /// Model family (gpt-4, gpt-3.5-turbo, etc.)
    pub family: String,

    /// Maximum context length
    pub max_tokens: u32,

    /// Whether the model supports tool calling
    pub supports_tools: bool,

    /// Whether the model supports streaming
    pub supports_streaming: bool,

    /// Whether the model supports parallel tool calls
    pub supports_parallel_tools: bool,

    /// Azure-specific model version
    pub version: Option<String>,

    /// Model availability region
    pub region: Option<String>,
}

impl Default for AzureOpenAIConfig {
    fn default() -> Self {
        Self {
            api_key: String::new(),
            endpoint: String::new(),
            deployment: "gpt-4o".to_string(),
            api_version: "2024-02-15-preview".to_string(),
            timeout: Some(120),
            max_retries: Some(3),
            retry_delay_ms: Some(1000),
            exponential_backoff: Some(true),
            user_agent: Some("LLM-Supabase-RS/1.0 (Azure OpenAI)".to_string()),
            deployment_mappings: None,
            azure_ad: None,
        }
    }
}

impl AzureOpenAIConfig {
    /// Create configuration from environment variables
    pub fn from_env() -> Result<Self> {
        debug!("Loading Azure OpenAI configuration from environment variables");

        let api_key = env::var("AZURE_OPENAI_API_KEY")
            .context("AZURE_OPENAI_API_KEY environment variable is required")?;

        let endpoint = env::var("AZURE_OPENAI_ENDPOINT")
            .context("AZURE_OPENAI_ENDPOINT environment variable is required")?;

        let deployment = env::var("AZURE_OPENAI_DEPLOYMENT")
            .context("AZURE_OPENAI_DEPLOYMENT environment variable is required")?;

        let api_version = env::var("AZURE_OPENAI_API_VERSION")
            .unwrap_or_else(|_| "2024-02-15-preview".to_string());

        let timeout = env::var("AZURE_OPENAI_TIMEOUT")
            .ok()
            .and_then(|t| t.parse().ok());

        let max_retries = env::var("AZURE_OPENAI_MAX_RETRIES")
            .ok()
            .and_then(|r| r.parse().ok());

        let retry_delay_ms = env::var("AZURE_OPENAI_RETRY_DELAY_MS")
            .ok()
            .and_then(|d| d.parse().ok());

        let exponential_backoff = env::var("AZURE_OPENAI_EXPONENTIAL_BACKOFF")
            .ok()
            .and_then(|b| b.parse().ok());

        let user_agent = env::var("AZURE_OPENAI_USER_AGENT").ok();

        // Parse deployment mappings if provided (JSON format)
        let deployment_mappings = env::var("AZURE_OPENAI_DEPLOYMENT_MAPPINGS")
            .ok()
            .and_then(|mappings_str| {
                serde_json::from_str(&mappings_str)
                    .map_err(|e| {
                        warn!("Failed to parse AZURE_OPENAI_DEPLOYMENT_MAPPINGS: {}", e);
                        e
                    })
                    .ok()
            });

        // Parse Azure AD configuration if provided
        let azure_ad = if env::var("AZURE_AD_TENANT_ID").is_ok() {
            Some(AzureADConfig::from_env()?)
        } else {
            None
        };

        debug!("Successfully loaded Azure OpenAI configuration");

        Ok(Self {
            api_key,
            endpoint,
            deployment,
            api_version,
            timeout,
            max_retries,
            retry_delay_ms,
            exponential_backoff,
            user_agent,
            deployment_mappings,
            azure_ad,
        })
    }

    /// Get the chat completions URL for Azure OpenAI
    pub fn get_chat_completions_url(&self) -> String {
        format!(
            "{}/openai/deployments/{}/chat/completions?api-version={}",
            self.endpoint.trim_end_matches('/'),
            self.deployment,
            self.api_version
        )
    }

    /// Get the models list URL (Azure OpenAI doesn't support this, returns deployments endpoint)
    pub fn get_models_url(&self) -> String {
        format!(
            "{}/openai/deployments?api-version={}",
            self.endpoint.trim_end_matches('/'),
            self.api_version
        )
    }

    /// Map a model name to a deployment name
    pub fn get_deployment_for_model(&self, model: &str) -> String {
        if let Some(ref mappings) = self.deployment_mappings {
            mappings.get(model).cloned().unwrap_or_else(|| {
                debug!("No mapping found for model '{}', using default deployment", model);
                self.deployment.clone()
            })
        } else {
            self.deployment.clone()
        }
    }

    /// Validate the configuration
    pub fn validate(&self) -> Result<()> {
        if self.api_key.is_empty() {
            return Err(anyhow::anyhow!("API key cannot be empty"));
        }

        if self.api_key.len() < 32 {
            return Err(anyhow::anyhow!("API key appears to be too short"));
        }

        if self.endpoint.is_empty() {
            return Err(anyhow::anyhow!("Endpoint cannot be empty"));
        }

        if !self.endpoint.starts_with("https://") {
            return Err(anyhow::anyhow!("Endpoint must use HTTPS"));
        }

        if !self.endpoint.contains(".openai.azure.com") {
            return Err(anyhow::anyhow!("Endpoint must be an Azure OpenAI endpoint"));
        }

        if self.deployment.is_empty() {
            return Err(anyhow::anyhow!("Deployment name cannot be empty"));
        }

        if self.api_version.is_empty() {
            return Err(anyhow::anyhow!("API version cannot be empty"));
        }

        // Validate timeout ranges
        if let Some(timeout) = self.timeout {
            if timeout == 0 || timeout > 3600 {
                return Err(anyhow::anyhow!("Timeout must be between 1 and 3600 seconds"));
            }
        }

        // Validate retry configuration
        if let Some(max_retries) = self.max_retries {
            if max_retries > 10 {
                return Err(anyhow::anyhow!("Max retries cannot exceed 10"));
            }
        }

        if let Some(retry_delay) = self.retry_delay_ms {
            if retry_delay > 30000 {
                return Err(anyhow::anyhow!("Retry delay cannot exceed 30 seconds"));
            }
        }

        debug!("Azure OpenAI configuration validation passed");
        Ok(())
    }

    /// Get masked API key for logging
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

    /// Get the resource name from the endpoint
    pub fn get_resource_name(&self) -> Option<String> {
        if let Some(start) = self.endpoint.find("://") {
            if let Some(end) = self.endpoint[start + 3..].find(".openai.azure.com") {
                return Some(self.endpoint[start + 3..start + 3 + end].to_string());
            }
        }
        None
    }
}

impl AzureADConfig {
    /// Create Azure AD configuration from environment variables
    pub fn from_env() -> Result<Self> {
        let tenant_id = env::var("AZURE_AD_TENANT_ID")
            .context("AZURE_AD_TENANT_ID environment variable is required for Azure AD auth")?;

        let client_id = env::var("AZURE_AD_CLIENT_ID")
            .context("AZURE_AD_CLIENT_ID environment variable is required for Azure AD auth")?;

        let client_secret = env::var("AZURE_AD_CLIENT_SECRET")
            .context("AZURE_AD_CLIENT_SECRET environment variable is required for Azure AD auth")?;

        let scope = env::var("AZURE_AD_SCOPE")
            .ok()
            .or_else(|| Some("https://cognitiveservices.azure.com/.default".to_string()));

        Ok(Self {
            tenant_id,
            client_id,
            client_secret,
            scope,
        })
    }

    /// Get the OAuth2 token endpoint URL
    pub fn get_token_url(&self) -> String {
        format!(
            "https://login.microsoftonline.com/{}/oauth2/v2.0/token",
            self.tenant_id
        )
    }
}

impl AzureOpenAIModel {
    /// Create model from deployment name and inferred capabilities
    pub fn from_deployment(deployment: &str) -> Self {
        let (family, max_tokens, supports_tools, supports_parallel_tools) = 
            Self::infer_model_properties(deployment);

        Self {
            name: deployment.to_string(),
            family,
            max_tokens,
            supports_tools,
            supports_streaming: true, // All Azure OpenAI models support streaming
            supports_parallel_tools,
            version: None,
            region: None,
        }
    }

    /// Infer model properties from deployment name
    fn infer_model_properties(deployment: &str) -> (String, u32, bool, bool) {
        let deployment_lower = deployment.to_lowercase();
        
        if deployment_lower.contains("gpt-4o") {
            ("gpt-4o".to_string(), 128000, true, true)
        } else if deployment_lower.contains("gpt-4") {
            ("gpt-4".to_string(), 128000, true, true)
        } else if deployment_lower.contains("gpt-3.5") || deployment_lower.contains("gpt-35") {
            ("gpt-3.5-turbo".to_string(), 16385, true, false)
        } else {
            // Default for unknown models
            ("unknown".to_string(), 4096, false, false)
        }
    }

    /// Check if this model supports a specific feature
    pub fn supports_feature(&self, feature: &str) -> bool {
        match feature {
            "tools" | "function_calling" => self.supports_tools,
            "streaming" => self.supports_streaming,
            "parallel_tools" => self.supports_parallel_tools,
            _ => false,
        }
    }
}

/// Azure OpenAI error response structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AzureOpenAIError {
    pub error: AzureOpenAIErrorDetails,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AzureOpenAIErrorDetails {
    pub code: Option<String>,
    pub message: String,
    #[serde(rename = "type")]
    pub error_type: Option<String>,
    pub param: Option<String>,
    /// Azure-specific error details
    pub inner_error: Option<AzureInnerError>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AzureInnerError {
    pub code: Option<String>,
    pub content_filter_result: Option<ContentFilterResult>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContentFilterResult {
    pub hate: Option<ContentFilterSeverity>,
    pub self_harm: Option<ContentFilterSeverity>,
    pub sexual: Option<ContentFilterSeverity>,
    pub violence: Option<ContentFilterSeverity>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContentFilterSeverity {
    pub filtered: bool,
    pub severity: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;

    #[test]
    fn test_config_default() {
        let config = AzureOpenAIConfig::default();
        assert_eq!(config.deployment, "gpt-4o");
        assert_eq!(config.api_version, "2024-02-15-preview");
        assert_eq!(config.timeout, Some(120));
    }

    #[test]
    fn test_chat_completions_url() {
        let config = AzureOpenAIConfig {
            endpoint: "https://test-resource.openai.azure.com".to_string(),
            deployment: "gpt-4o".to_string(),
            api_version: "2024-02-15-preview".to_string(),
            ..Default::default()
        };

        let url = config.get_chat_completions_url();
        assert_eq!(
            url, 
            "https://test-resource.openai.azure.com/openai/deployments/gpt-4o/chat/completions?api-version=2024-02-15-preview"
        );
    }

    #[test]
    fn test_deployment_mapping() {
        let mut mappings = std::collections::HashMap::new();
        mappings.insert("gpt-4".to_string(), "my-gpt4-deployment".to_string());

        let config = AzureOpenAIConfig {
            deployment: "default-deployment".to_string(),
            deployment_mappings: Some(mappings),
            ..Default::default()
        };

        assert_eq!(config.get_deployment_for_model("gpt-4"), "my-gpt4-deployment");
        assert_eq!(config.get_deployment_for_model("unknown-model"), "default-deployment");
    }

    #[test]
    fn test_config_validation() {
        let valid_config = AzureOpenAIConfig {
            api_key: "valid-api-key-1234567890123456789012345678901234567890".to_string(),
            endpoint: "https://test-resource.openai.azure.com".to_string(),
            deployment: "gpt-4o".to_string(),
            api_version: "2024-02-15-preview".to_string(),
            ..Default::default()
        };

        assert!(valid_config.validate().is_ok());

        let invalid_config = AzureOpenAIConfig {
            api_key: "short".to_string(),
            ..valid_config.clone()
        };

        assert!(invalid_config.validate().is_err());
    }

    #[test]
    fn test_model_from_deployment() {
        let model = AzureOpenAIModel::from_deployment("gpt-4o-deployment");
        assert_eq!(model.family, "gpt-4o");
        assert_eq!(model.max_tokens, 128000);
        assert!(model.supports_tools);
        assert!(model.supports_parallel_tools);

        let model35 = AzureOpenAIModel::from_deployment("gpt-35-turbo");
        assert_eq!(model35.family, "gpt-3.5-turbo");
        assert_eq!(model35.max_tokens, 16385);
        assert!(model35.supports_tools);
        assert!(!model35.supports_parallel_tools);
    }

    #[test]
    fn test_masked_api_key() {
        let config = AzureOpenAIConfig {
            api_key: "1234567890abcdef".to_string(),
            ..Default::default()
        };

        assert_eq!(config.get_masked_api_key(), "1234...cdef");
    }

    #[test]
    fn test_resource_name_extraction() {
        let config = AzureOpenAIConfig {
            endpoint: "https://my-resource.openai.azure.com".to_string(),
            ..Default::default()
        };

        assert_eq!(config.get_resource_name(), Some("my-resource".to_string()));
    }
}