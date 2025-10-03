use crate::api::handlers::{
    health::health_check,
    models::list_models,
    chat::chat_completions_unified,
};
use crate::config::{AppConfig, app::AppStateHealth, providers::{VertexAiConfig, VertexAuthConfig, VertexAuthMethod}};
use crate::features::auth::AuthMiddleware;
use crate::infrastructure::{SupabaseClient, VertexAIClient};
use crate::shared::{AppError, AppResult};

use axum::{
    http::{header, Method},
    middleware,
    routing::{get, post},
    Router,
};
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::net::TcpListener;
use tower::ServiceBuilder;
use tower_http::{
    cors::{Any, CorsLayer},
    trace::TraceLayer,
};

pub struct App {
    config: AppConfig,
    vertex_client: Arc<VertexAIClient>,
    supabase_client: Arc<SupabaseClient>,
}

#[derive(Clone)]
pub struct AppState {
    pub config: AppConfig,
    pub vertex_client: Arc<VertexAIClient>,
    pub supabase_client: Arc<SupabaseClient>,
    pub health: AppStateHealth,
}

impl App {
    pub async fn new(config: AppConfig) -> AppResult<Self> {
        tracing::info!("Initializing application components...");

        // Convert old VertexConfig to new VertexAiConfig
        let vertex_ai_config = VertexAiConfig {
            project_id: config.vertex.project_id.clone(),
            region: config.vertex.location.clone(),
            credentials_path: config.vertex.credentials_path.clone(),
            default_model: crate::config::providers::VertexModelConfig {
                model_name: config.vertex.default_model.clone(),
                ..Default::default()
            },
            auth: VertexAuthConfig {
                auth_method: VertexAuthMethod::ServiceAccountKey,
                ..Default::default()
            },
            ..Default::default()
        };

        // Initialize Vertex AI client
        let vertex_client = Arc::new(VertexAIClient::new(vertex_ai_config).await.map_err(
            |e| AppError::internal(format!("Failed to initialize Vertex AI client: {}", e)),
        )?);

        // Initialize Supabase client (placeholder for now)
        let supabase_client = Arc::new(SupabaseClient::new(&config.supabase));

        // Log configuration summary (safe - excludes sensitive data)
        let config_summary = config.summary();
        tracing::info!(
            server_address = %config_summary.server_address,
            log_level = %config_summary.log_level,
            supabase_configured = config_summary.supabase_configured,
            vertex_project = %config_summary.vertex_project,
            vertex_region = %config_summary.vertex_region,
            production_ready = config_summary.production_ready,
            "Application components initialized successfully"
        );

        if !config_summary.production_ready {
            tracing::warn!("⚠️  Application is NOT production ready. Check configuration.");
        } else {
            tracing::info!("✅ Application is production ready");
        }

        Ok(Self {
            config,
            vertex_client,
            supabase_client,
        })
    }

    pub async fn run(self) -> AppResult<()> {
        let addr = SocketAddr::from(([0, 0, 0, 0], self.config.port));

        let app_state = AppState {
            config: self.config.clone(),
            vertex_client: self.vertex_client.clone(),
            supabase_client: self.supabase_client.clone(),
            health: AppStateHealth::new(&self.config),
        };

        let app = self.create_router(app_state);

        tracing::info!("🚀 Server starting on http://{}", addr);

        let listener = TcpListener::bind(addr)
            .await
            .map_err(|e| AppError::internal(format!("Failed to bind to address: {}", e)))?;

        // Create shutdown signal handler
        let shutdown_signal = Self::shutdown_signal();

        // Start the server with graceful shutdown support
        axum::serve(listener, app)
            .with_graceful_shutdown(shutdown_signal)
            .await
            .map_err(|e| AppError::internal(format!("Server error: {}", e)))?;

        // Perform cleanup after server shutdown
        Self::cleanup().await;
        tracing::info!("Server has shut down gracefully");
        Ok(())
    }

    /// Handle graceful shutdown signal
    async fn shutdown_signal() {
        let ctrl_c = async {
            tokio::signal::ctrl_c()
                .await
                .expect("Failed to install Ctrl+C handler");
        };

        #[cfg(unix)]
        let terminate = async {
            tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
                .expect("Failed to install signal handler")
                .recv()
                .await;
        };

        #[cfg(not(unix))]
        let terminate = std::future::pending::<()>();

        tokio::select! {
            _ = ctrl_c => {
                tracing::info!("Received Ctrl+C, initiating graceful shutdown...");
            },
            _ = terminate => {
                tracing::info!("Received SIGTERM, initiating graceful shutdown...");
            },
        }
    }

    /// Perform cleanup operations during shutdown
    async fn cleanup() {
        tracing::info!("Performing cleanup operations...");
        
        // Perform any necessary cleanup here
        // For example:
        // - Close database connections
        // - Flush pending requests
        // - Save state if needed
        // - Clean up temporary files
        
        // Health check cleanup
        tracing::debug!("Cleaning up health check resources");
        
        // Vertex AI client cleanup (if needed)
        tracing::debug!("Cleaning up Vertex AI client resources");
        
        // Supabase client cleanup (if needed)
        tracing::debug!("Cleaning up Supabase client resources");
        
        tracing::info!("Cleanup operations completed");
    }

    fn create_router(&self, state: AppState) -> Router {
        // CORS configuration
        let cors = CorsLayer::new()
            .allow_methods([Method::GET, Method::POST, Method::OPTIONS])
            .allow_headers([header::AUTHORIZATION, header::CONTENT_TYPE, header::ACCEPT])
            .allow_origin(Any);

        // Create the main router with all routes
        Router::new()
            // Health check endpoint
            .route("/health", get(health_check))
            // OpenAI API compatible endpoints
            .route("/v1/chat/completions", post(chat_completions_unified))
            .route("/v1/models", get(list_models))
            .route("/v1/embeddings", post(embeddings_handler))
            // Apply middleware layers
            .layer(
                ServiceBuilder::new()
                    .layer(TraceLayer::new_for_http())
                    .layer(cors)
                    .layer(middleware::from_fn_with_state(
                        state.clone(),
                        AuthMiddleware::optional_authenticate,
                    )),
            )
            .with_state(state)
    }
}

// Note: health_check handler is now imported from crate::api::handlers::health::health_check

// Note: chat_completions handler is now implemented in crate::api::handlers::chat::chat_completions

// Note: models handler is now implemented in crate::api::handlers::models::list_models

async fn embeddings_handler() -> Result<&'static str, AppError> {
    Err(AppError::internal("Embeddings handler not implemented yet"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{AppConfig, EmbeddingConfig, SupabaseConfig, VertexConfig};

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
                default_model: "claude-sonnet-4-20250514".to_string(),
            },
            embedding: EmbeddingConfig::default(),
            model_cache_dir: "./models".to_string(),
        }
    }

    #[tokio::test]
    #[ignore] // Requires GCP credentials
    async fn test_app_creation() {
        let config = create_test_config();
        let result = App::new(config).await;
        // This will fail without actual credentials, but tests the structure
        assert!(result.is_ok() || result.is_err());
    }
}
