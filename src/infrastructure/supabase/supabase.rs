use reqwest::Client;
use serde::{Deserialize, Serialize};
use anyhow::Result;
use crate::config::SupabaseConfig;

pub struct SupabaseClient {
    client: Client,
    base_url: String,
    anon_key: String,
    service_role_key: String,
    jwt_secret: String,
}

impl SupabaseClient {
    pub fn new(config: &SupabaseConfig) -> Self {
        Self {
            client: Client::new(),
            base_url: config.url.clone(),
            anon_key: config.anon_key.clone(),
            service_role_key: config.service_role_key.clone(),
            jwt_secret: config.jwt_secret.clone(),
        }
    }
    
    // Generic query builder
    fn rest_url(&self, table: &str) -> String {
        format!("{}/rest/v1/{}", self.base_url, table)
    }
    
    // Use service role key for admin operations
    fn auth_header(&self, use_service_role: bool) -> String {
        if use_service_role {
            format!("Bearer {}", self.service_role_key)
        } else {
            format!("Bearer {}", self.anon_key)
        }
    }
}

// Database Operations via REST API
impl SupabaseClient {
    // SELECT operations
    pub async fn select<T: for<'de> Deserialize<'de>>(
        &self,
        table: &str,
        query: SupabaseQuery,
    ) -> Result<Vec<T>> {
        let mut url = self.rest_url(table);
        
        // Build query string
        let mut params = vec![];
        if let Some(select) = query.select {
            params.push(format!("select={}", select));
        }
        if let Some(filter) = query.filter {
            params.push(filter);
        }
        if let Some(order) = query.order {
            params.push(format!("order={}", order));
        }
        if let Some(limit) = query.limit {
            params.push(format!("limit={}", limit));
        }
        
        if !params.is_empty() {
            url = format!("{}?{}", url, params.join("&"));
        }
        
        let response = self.client
            .get(&url)
            .header("apikey", &self.anon_key)
            .header("Authorization", self.auth_header(query.use_service_role))
            .send()
            .await?;
            
        if !response.status().is_success() {
            let error_text = response.text().await?;
            return Err(anyhow::anyhow!("Supabase query failed: {}", error_text));
        }
        
        Ok(response.json().await?)
    }
    
    // INSERT operations
    pub async fn insert<T: Serialize>(
        &self,
        table: &str,
        data: &T,
        use_service_role: bool,
    ) -> Result<serde_json::Value> {
        let response = self.client
            .post(&self.rest_url(table))
            .header("apikey", &self.anon_key)
            .header("Authorization", self.auth_header(use_service_role))
            .header("Content-Type", "application/json")
            .header("Prefer", "return=representation")
            .json(data)
            .send()
            .await?;
            
        if !response.status().is_success() {
            let error_text = response.text().await?;
            return Err(anyhow::anyhow!("Supabase insert failed: {}", error_text));
        }
        
        Ok(response.json().await?)
    }
    
    // UPDATE operations
    pub async fn update<T: Serialize>(
        &self,
        table: &str,
        filter: &str,
        data: &T,
        use_service_role: bool,
    ) -> Result<serde_json::Value> {
        let url = format!("{}?{}", self.rest_url(table), filter);
        
        let response = self.client
            .patch(&url)
            .header("apikey", &self.anon_key)
            .header("Authorization", self.auth_header(use_service_role))
            .header("Content-Type", "application/json")
            .header("Prefer", "return=representation")
            .json(data)
            .send()
            .await?;
            
        if !response.status().is_success() {
            let error_text = response.text().await?;
            return Err(anyhow::anyhow!("Supabase update failed: {}", error_text));
        }
        
        Ok(response.json().await?)
    }
    
    // DELETE operations
    pub async fn delete(
        &self,
        table: &str,
        filter: &str,
        use_service_role: bool,
    ) -> Result<()> {
        let url = format!("{}?{}", self.rest_url(table), filter);
        
        let response = self.client
            .delete(&url)
            .header("apikey", &self.anon_key)
            .header("Authorization", self.auth_header(use_service_role))
            .send()
            .await?;
            
        if !response.status().is_success() {
            let error_text = response.text().await?;
            return Err(anyhow::anyhow!("Supabase delete failed: {}", error_text));
        }
        
        Ok(())
    }
    
    // Call Edge Function
    pub async fn call_function(
        &self,
        function_name: &str,
        payload: serde_json::Value,
    ) -> Result<serde_json::Value> {
        let url = format!("{}/functions/v1/{}", self.base_url, function_name);
        
        let response = self.client
            .post(&url)
            .header("Authorization", format!("Bearer {}", self.service_role_key))
            .header("Content-Type", "application/json")
            .json(&payload)
            .send()
            .await?;
            
        if !response.status().is_success() {
            let error_text = response.text().await?;
            return Err(anyhow::anyhow!("Edge function failed: {}", error_text));
        }
        
        Ok(response.json().await?)
    }
    
    // JWT Validation
    pub fn validate_jwt(&self, token: &str) -> Result<Claims> {
        use jsonwebtoken::{decode, DecodingKey, Validation, Algorithm};

        let validation = Validation::new(Algorithm::HS256);
        let token_data = decode::<Claims>(
            token,
            &DecodingKey::from_secret(self.jwt_secret.as_bytes()),
            &validation
        )?;

        Ok(token_data.claims)
    }

    /// Health check for Supabase service
    /// Tests connectivity by making a simple request to the REST API
    pub async fn health_check(&self) -> Result<bool> {
        use tracing::debug;

        debug!("Performing Supabase health check");

        // Test connectivity with a simple request to the REST API health endpoint
        let url = format!("{}/rest/v1/", self.base_url);

        let response = self.client
            .get(&url)
            .header("Authorization", format!("Bearer {}", self.anon_key))
            .header("apikey", &self.anon_key)
            .send()
            .await?;

        let is_healthy = response.status().is_success() || response.status().as_u16() == 404;

        if is_healthy {
            debug!("Supabase health check passed");
        } else {
            debug!("Supabase health check failed with status: {}", response.status());
        }

        Ok(is_healthy)
    }
}

// Helper types
#[derive(Default)]
pub struct SupabaseQuery {
    pub select: Option<String>,
    pub filter: Option<String>,
    pub order: Option<String>,
    pub limit: Option<usize>,
    pub use_service_role: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Claims {
    pub sub: String,        // Subject (user ID)
    pub email: Option<String>,
    pub role: Option<String>,
    pub iat: u64,           // Issued at
    pub exp: u64,           // Expiration
}