// src/api/handlers/models.rs
//
// Models endpoint handler implementation
// Provides list of available models in OpenAI-compatible format

use axum::{
    extract::State,
    response::Json,
    http::StatusCode,
};
use tracing::{debug, instrument};

use crate::app::AppState;
use crate::models::response::ModelsResponse;
use crate::shared::AppError;

/// Models list handler
///
/// Returns list of available models in OpenAI-compatible format
/// This endpoint lists the Claude 4 Sonnet model available through Vertex AI
#[instrument(skip(state))]
pub async fn list_models(State(state): State<AppState>) -> Result<Json<ModelsResponse>, AppError> {
    debug!("Processing models list request");

    // Get available models from Vertex AI client
    let available_models = match state.vertex_client.list_models().await {
        Ok(models) => {
            debug!(model_count = models.len(), "Retrieved models from Vertex AI");
            models
        }
        Err(e) => {
            // If we can't get models from Vertex AI, return default Claude model
            debug!(error = %e, "Failed to retrieve models from Vertex AI, using default");
            vec!["claude-sonnet-4-5@20250929".to_string()]
        }
    };

    // Create OpenAI-compatible models response
    let mut models_response = ModelsResponse::claude_models();

    // Add any additional models from Vertex AI if different from default
    for model_id in available_models {
        if model_id != "claude-sonnet-4-5@20250929" {
            models_response = models_response.add_model(
                crate::models::response::Model::new(model_id, "anthropic".to_string())
            );
        }
    }

    debug!(
        model_count = models_response.data.len(),
        "Returning models list"
    );

    Ok(Json(models_response))
}

/// Model details handler (for future expansion)
///
/// This would provide detailed information about a specific model
/// Currently not required by OpenAI API but could be useful for debugging
#[instrument(skip(_state))]
pub async fn get_model_details(
    State(_state): State<AppState>,
    // Path(model_id): Path<String>, // Uncomment when needed
) -> Result<StatusCode, AppError> {
    debug!("Model details endpoint called");

    // For now, return not implemented
    // In the future, this could return detailed model information
    Err(AppError::internal("Model details endpoint not implemented"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::{App, AppState};
    use crate::config::{AppConfig, EmbeddingConfig, SupabaseConfig, VertexConfig};
    use std::sync::Arc;

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
            vertex: VertexConfig {
                project_id: "test-project".to_string(),
                location: "us-east5".to_string(),
                credentials_path: "./gcp-credentials.json".to_string(),
                default_model: "claude-sonnet-4-5@20250929".to_string(),
            },
            embedding: EmbeddingConfig::default(),
            model_cache_dir: "./models".to_string(),
        }
    }

    #[tokio::test]
    async fn test_list_models_response_format() {
        // Test that the models response has the correct format
        let models_response = ModelsResponse::claude_models();

        assert_eq!(models_response.object, "list");
        assert_eq!(models_response.data.len(), 1);
        assert_eq!(models_response.data[0].id, "claude-sonnet-4-5@20250929");
        assert_eq!(models_response.data[0].object, "model");
        assert_eq!(models_response.data[0].owned_by, "anthropic");
    }

    #[tokio::test]
    async fn test_models_response_serialization() {
        let models_response = ModelsResponse::claude_models();
        let json = serde_json::to_string(&models_response).unwrap();

        // Should contain required OpenAI API fields
        assert!(json.contains("\"object\":\"list\""));
        assert!(json.contains("\"data\":["));
        assert!(json.contains("\"id\":\"claude-sonnet-4-5@20250929\""));
        assert!(json.contains("\"owned_by\":\"anthropic\""));

        // Should be deserializable
        let deserialized: ModelsResponse = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.object, "list");
        assert_eq!(deserialized.data.len(), 1);
    }

    #[test]
    fn test_add_model() {
        let mut models_response = ModelsResponse::claude_models();
        let initial_count = models_response.data.len();

        models_response = models_response.add_model(
            crate::models::response::Model::new("gpt-4".to_string(), "openai".to_string())
        );

        assert_eq!(models_response.data.len(), initial_count + 1);
        assert!(models_response.data.iter().any(|m| m.id == "gpt-4"));
        assert!(models_response.data.iter().any(|m| m.owned_by == "openai"));
    }

    // Integration test would require actual Vertex AI setup
    // #[tokio::test]
    // #[ignore]
    // async fn test_list_models_integration() {
    //     let config = create_test_config();
    //     let app = App::new(config).await.unwrap();
    //     let state = AppState {
    //         config: app.config.clone(),
    //         vertex_client: app.vertex_client.clone(),
    //         supabase_client: app.supabase_client.clone(),
    //     };
    //
    //     let result = list_models(State(state)).await;
    //     assert!(result.is_ok());
    //
    //     let response = result.unwrap().0;
    //     assert_eq!(response.object, "list");
    //     assert!(!response.data.is_empty());
    // }
}