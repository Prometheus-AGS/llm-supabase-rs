use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde_json::json;
use thiserror::Error;

pub type AppResult<T> = Result<T, AppError>;

#[derive(Error, Debug)]
pub enum AppError {
    #[error("Authentication error: {message}")]
    Authentication { message: String },

    #[error("Authorization error: {message}")]
    Authorization { message: String },

    #[error("Validation error: {message}")]
    Validation { message: String },

    #[error("Vertex AI error: {message}")]
    VertexAI { message: String },

    #[error("Supabase error: {message}")]
    Supabase { message: String },

    #[error("Embedding provider error: {message}")]
    EmbeddingProvider { message: String },

    #[error("Model not found: {model}")]
    ModelNotFound { model: String },

    #[error("Rate limit exceeded")]
    RateLimit,

    #[error("Internal server error: {message}")]
    Internal { message: String },

    #[error("External service error: {service} - {message}")]
    ExternalService { service: String, message: String },

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("HTTP client error: {0}")]
    HttpClient(#[from] reqwest::Error),

    #[error("JWT error: {0}")]
    Jwt(#[from] jsonwebtoken::errors::Error),

    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

impl AppError {
    pub fn authentication(message: impl Into<String>) -> Self {
        Self::Authentication {
            message: message.into(),
        }
    }

    pub fn authorization(message: impl Into<String>) -> Self {
        Self::Authorization {
            message: message.into(),
        }
    }

    pub fn validation(message: impl Into<String>) -> Self {
        Self::Validation {
            message: message.into(),
        }
    }

    pub fn vertex_ai(message: impl Into<String>) -> Self {
        Self::VertexAI {
            message: message.into(),
        }
    }

    pub fn supabase(message: impl Into<String>) -> Self {
        Self::Supabase {
            message: message.into(),
        }
    }

    pub fn embedding_provider(message: impl Into<String>) -> Self {
        Self::EmbeddingProvider {
            message: message.into(),
        }
    }

    pub fn model_not_found(model: impl Into<String>) -> Self {
        Self::ModelNotFound {
            model: model.into(),
        }
    }

    pub fn internal(message: impl Into<String>) -> Self {
        Self::Internal {
            message: message.into(),
        }
    }

    pub fn external_service(service: impl Into<String>, message: impl Into<String>) -> Self {
        Self::ExternalService {
            service: service.into(),
            message: message.into(),
        }
    }
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, error_message) = match &self {
            AppError::Authentication { .. } => (StatusCode::UNAUTHORIZED, "Authentication failed"),
            AppError::Authorization { .. } => (StatusCode::FORBIDDEN, "Authorization failed"),
            AppError::Validation { .. } => (StatusCode::BAD_REQUEST, "Validation failed"),
            AppError::ModelNotFound { .. } => (StatusCode::NOT_FOUND, "Model not found"),
            AppError::RateLimit => (StatusCode::TOO_MANY_REQUESTS, "Rate limit exceeded"),
            AppError::VertexAI { .. } => (StatusCode::BAD_GATEWAY, "Vertex AI service error"),
            AppError::Supabase { .. } => (StatusCode::BAD_GATEWAY, "Supabase service error"),
            AppError::EmbeddingProvider { .. } => {
                (StatusCode::BAD_GATEWAY, "Embedding service error")
            }
            AppError::ExternalService { .. } => (StatusCode::BAD_GATEWAY, "External service error"),
            AppError::Serialization(_) => {
                (StatusCode::INTERNAL_SERVER_ERROR, "Serialization error")
            }
            AppError::HttpClient(_) => (StatusCode::BAD_GATEWAY, "HTTP client error"),
            AppError::Jwt(_) => (StatusCode::UNAUTHORIZED, "JWT error"),
            AppError::Database(_) => (StatusCode::INTERNAL_SERVER_ERROR, "Database error"),
            AppError::Io(_) => (StatusCode::INTERNAL_SERVER_ERROR, "IO error"),
            AppError::Internal { .. } => {
                (StatusCode::INTERNAL_SERVER_ERROR, "Internal server error")
            }
        };

        let error_response = json!({
            "error": {
                "message": error_message,
                "type": format!("{:?}", self).split('(').next().unwrap_or("Unknown"),
                "details": self.to_string()
            }
        });

        tracing::error!("Application error: {} - {}", status, self);

        (status, Json(error_response)).into_response()
    }
}

// Convert anyhow::Error to AppError
impl From<anyhow::Error> for AppError {
    fn from(err: anyhow::Error) -> Self {
        AppError::internal(err.to_string())
    }
}
