use crate::infrastructure::database::supabase::{SupabaseClient, SupabaseQuery};
use uuid::Uuid;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Webhook {
    pub id: Uuid,
    pub user_id: Option<Uuid>,
    pub events: Vec<String>,
    pub endpoint: String,
    pub auth_header: Option<String>,
    pub enabled: bool,
    pub metadata: serde_json::Value,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

pub struct WebhookRepository {
    client: SupabaseClient,
}

impl WebhookRepository {
    pub fn new(client: SupabaseClient) -> Self {
        Self { client }
    }
    
    // Find webhooks by user
    pub async fn find_by_user(&self, user_id: Uuid) -> Result<Vec<Webhook>> {
        let query = SupabaseQuery {
            select: Some("*".to_string()),
            filter: Some(format!("user_id=eq.{}&enabled=eq.true", user_id)),
            order: Some("created_at.desc".to_string()),
            use_service_role: true,
            ..Default::default()
        };
        
        self.client.select("webhooks", query).await
    }
    
    // Find webhooks by event type
    pub async fn find_by_event(&self, event_type: &str) -> Result<Vec<Webhook>> {
        let query = SupabaseQuery {
            select: Some("*".to_string()),
            filter: Some(format!("events=cs.{{{}}}&enabled=eq.true", event_type)),
            use_service_role: true,
            ..Default::default()
        };
        
        self.client.select("webhooks", query).await
    }
    
    // Create webhook
    pub async fn create(&self, webhook: &WebhookCreate) -> Result<Webhook> {
        let result = self.client
            .insert("webhooks", webhook, true)
            .await?;
            
        Ok(serde_json::from_value(result[0].clone())?)
    }
    
    // Update webhook
    pub async fn update(&self, id: Uuid, updates: &WebhookUpdate) -> Result<Webhook> {
        let filter = format!("id=eq.{}", id);
        let result = self.client
            .update("webhooks", &filter, updates, true)
            .await?;
            
        Ok(serde_json::from_value(result[0].clone())?)
    }
    
    // Delete webhook
    pub async fn delete(&self, id: Uuid) -> Result<()> {
        let filter = format!("id=eq.{}", id);
        self.client.delete("webhooks", &filter, true).await
    }
}

#[derive(Serialize)]
pub struct WebhookCreate {
    pub user_id: Option<Uuid>,
    pub events: Vec<String>,
    pub endpoint: String,
    pub auth_header: Option<String>,
    pub enabled: bool,
    pub metadata: serde_json::Value,
}

#[derive(Serialize)]
pub struct WebhookUpdate {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub events: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub endpoint: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enabled: Option<bool>,
}