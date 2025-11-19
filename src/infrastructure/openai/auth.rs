use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, warn, error};

use crate::models::error::ErrorResponse;

/// Authentication handler for OpenAI API
#[derive(Debug, Clone)]
pub struct OpenAIAuth {
    pub api_key: String,
    pub organization: Option<String>,
    pub project: Option<String>,
}

impl OpenAIAuth {
    /// Create a new OpenAI authentication handler
    pub fn new(api_key: String) -> Self {
        Self {
            api_key,
            organization: None,
            project: None,
        }
    }

    /// Create OpenAI auth from configuration
    pub fn from_config(config: &crate::config::providers::OpenAIConfig) -> anyhow::Result<Self> {
        if config.api_key.is_empty() {
            return Err(anyhow::anyhow!("OpenAI API key is empty"));
        }

        Ok(Self {
            api_key: config.api_key.clone(),
            organization: config.organization.clone(),
            project: None,
        })
    }

    /// Validate API key format and presence
    pub fn validate_api_key(&self) -> anyhow::Result<()> {
        if self.api_key.trim().is_empty() {
            return Err(anyhow::anyhow!("OpenAI API key is required"));
        }

        if !self.api_key.starts_with("sk-") && !self.api_key.starts_with("sk-proj-") {
            warn!("OpenAI API key format may be invalid - should start with 'sk-' or 'sk-proj-'");
        }

        debug!(
            organization = ?self.organization,
            project = ?self.project,
            "OpenAI authentication validated"
        );

        Ok(())
    }

    /// Get a masked version of the API key for logging
    pub fn get_masked_api_key(&self) -> String {
        if self.api_key.len() > 10 {
            format!("{}...{}", &self.api_key[..7], &self.api_key[self.api_key.len()-4..])
        } else {
            "***".to_string()
        }
    }

    /// Create OpenAI auth with organization ID
    pub fn with_organization(mut self, organization: String) -> Self {
        self.organization = Some(organization);
        self
    }

    /// Create OpenAI auth with project ID
    pub fn with_project(mut self, project: String) -> Self {
        self.project = Some(project);
        self
    }

    /// Get the API key for authentication
    pub fn get_api_key(&self) -> &str {
        &self.api_key
    }

    /// Get the organization ID if set
    pub fn get_organization(&self) -> Option<&str> {
        self.organization.as_deref()
    }

    /// Get the project ID if set
    pub fn get_project(&self) -> Option<&str> {
        self.project.as_deref()
    }

    /// Validate the authentication configuration
    pub fn validate(&self) -> Result<(), String> {
        if self.api_key.trim().is_empty() {
            return Err("OpenAI API key is required".to_string());
        }

        if !self.api_key.starts_with("sk-") && !self.api_key.starts_with("sk-proj-") {
            warn!("OpenAI API key format may be invalid - should start with 'sk-' or 'sk-proj-'");
        }

        debug!(
            organization = ?self.organization,
            project = ?self.project,
            "OpenAI authentication validated"
        );

        Ok(())
    }

    /// Create authorization header value
    pub fn auth_header(&self) -> String {
        format!("Bearer {}", self.api_key)
    }

    /// Create OpenAI-Organization header if organization is set
    pub fn organization_header(&self) -> Option<String> {
        self.organization.clone()
    }

    /// Create OpenAI-Project header if project is set
    pub fn project_header(&self) -> Option<String> {
        self.project.clone()
    }

    /// Create HTTP headers for OpenAI API requests
    pub fn create_headers(&self) -> anyhow::Result<reqwest::header::HeaderMap> {
        let mut headers = reqwest::header::HeaderMap::new();

        // Authorization header
        let auth_value = reqwest::header::HeaderValue::from_str(&self.auth_header())
            .map_err(|e| anyhow::anyhow!("Invalid authorization header: {}", e))?;
        headers.insert(reqwest::header::AUTHORIZATION, auth_value);

        // Content-Type header
        let content_type = reqwest::header::HeaderValue::from_static("application/json");
        headers.insert(reqwest::header::CONTENT_TYPE, content_type);

        // Optional organization header
        if let Some(org) = &self.organization {
            let org_value = reqwest::header::HeaderValue::from_str(org)
                .map_err(|e| anyhow::anyhow!("Invalid organization header: {}", e))?;
            headers.insert("OpenAI-Organization", org_value);
        }

        // Optional project header
        if let Some(project) = &self.project {
            let project_value = reqwest::header::HeaderValue::from_str(project)
                .map_err(|e| anyhow::anyhow!("Invalid project header: {}", e))?;
            headers.insert("OpenAI-Project", project_value);
        }

        Ok(headers)
    }
}

/// Thread-safe OpenAI authentication wrapper
pub type OpenAIAuthRef = Arc<RwLock<OpenAIAuth>>;

/// Create a new thread-safe OpenAI authentication reference
pub fn new_auth_ref(api_key: String) -> OpenAIAuthRef {
    Arc::new(RwLock::new(OpenAIAuth::new(api_key)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_openai_auth_creation() {
        let auth = OpenAIAuth::new("sk-test-key".to_string());
        assert_eq!(auth.get_api_key(), "sk-test-key");
        assert!(auth.get_organization().is_none());
        assert!(auth.get_project().is_none());
    }

    #[test]
    fn test_openai_auth_with_organization() {
        let auth = OpenAIAuth::new("sk-test-key".to_string())
            .with_organization("org-123".to_string());
        
        assert_eq!(auth.get_organization(), Some("org-123"));
    }

    #[test]
    fn test_openai_auth_with_project() {
        let auth = OpenAIAuth::new("sk-test-key".to_string())
            .with_project("proj-456".to_string());
        
        assert_eq!(auth.get_project(), Some("proj-456"));
    }

    #[test]
    fn test_auth_header() {
        let auth = OpenAIAuth::new("sk-test-key".to_string());
        assert_eq!(auth.auth_header(), "Bearer sk-test-key");
    }

    #[test]
    fn test_auth_validation() {
        let auth = OpenAIAuth::new("sk-test-key".to_string());
        assert!(auth.validate().is_ok());

        let empty_auth = OpenAIAuth::new("".to_string());
        assert!(empty_auth.validate().is_err());
    }
}