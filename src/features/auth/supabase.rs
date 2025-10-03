use crate::config::AppConfig;
use crate::shared::{AppError, AppResult};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SupabaseUser {
    pub id: Uuid,
    pub email: Option<String>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
    pub user_metadata: serde_json::Value,
    pub app_metadata: serde_json::Value,
}

pub struct SupabaseAuthClient {
    client: Client,
    base_url: String,
    service_role_key: String,
}

impl SupabaseAuthClient {
    pub fn new(config: &AppConfig) -> Self {
        Self {
            client: Client::new(),
            base_url: format!("{}/auth/v1", config.supabase.url),
            service_role_key: config.supabase.service_role_key.clone(),
        }
    }

    pub async fn get_user(&self, user_id: Uuid) -> AppResult<Option<SupabaseUser>> {
        let url = format!("{}/admin/users/{}", self.base_url, user_id);

        let response = self
            .client
            .get(&url)
            .header("Authorization", format!("Bearer {}", self.service_role_key))
            .header("apikey", &self.service_role_key)
            .send()
            .await
            .map_err(|e| AppError::supabase(format!("Failed to call Supabase API: {}", e)))?;

        match response.status().as_u16() {
            200 => {
                let user = response.json::<SupabaseUser>().await.map_err(|e| {
                    AppError::supabase(format!("Failed to parse user response: {}", e))
                })?;
                Ok(Some(user))
            }
            404 => Ok(None),
            status => {
                let error_text = response.text().await.unwrap_or_default();
                Err(AppError::supabase(format!(
                    "Supabase API returned {}: {}",
                    status, error_text
                )))
            }
        }
    }

    pub async fn validate_service_role_token(&self, token: &str) -> AppResult<bool> {
        // For service role tokens, we can validate by making a simple admin API call
        let url = format!("{}/admin/users", self.base_url);

        let response = self
            .client
            .get(&url)
            .header("Authorization", format!("Bearer {}", token))
            .header("apikey", token)
            .query(&[("per_page", "1")]) // Just get one user to test auth
            .send()
            .await
            .map_err(|e| {
                AppError::supabase(format!("Failed to validate service role token: {}", e))
            })?;

        Ok(response.status().is_success())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{AppConfig, SupabaseConfig};

    fn create_test_config() -> AppConfig {
        AppConfig {
            host: "localhost".to_string(),
            port: 8080,
            log_level: "info".to_string(),
            supabase: SupabaseConfig {
                url: "https://test.supabase.co".to_string(),
                anon_key: "test-anon-key".to_string(),
                service_role_key: "test-service-role-key".to_string(),
                jwt_secret: "test-secret".to_string(),
            },
            vertex: crate::config::VertexConfig::default(),
            embedding: crate::config::EmbeddingConfig::default(),
            model_cache_dir: "./models".to_string(),
        }
    }

    #[test]
    fn test_client_creation() {
        let config = create_test_config();
        let client = SupabaseAuthClient::new(&config);
        assert_eq!(client.base_url, "https://test.supabase.co/auth/v1");
        assert_eq!(client.service_role_key, "test-service-role-key");
    }
}
