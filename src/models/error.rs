// src/models/error.rs
//
// OpenAI API compatible error response models

use serde::{Deserialize, Serialize};
use std::time::SystemTime;

// Import provider fallback types
use crate::features::provider_fallback::models::{Provider, ProviderErrorType};

/// OpenAI API error response structure
/// Matches the exact format returned by OpenAI API for all error conditions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorResponse {
    pub error: ErrorDetails,
}

/// Detailed error information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorDetails {
    /// Human-readable error message
    pub message: String,

    /// Error type classification
    #[serde(rename = "type")]
    pub error_type: ErrorType,

    /// Machine-readable error code (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub code: Option<String>,

    /// Parameter that caused the error (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub param: Option<String>,
}

/// OpenAI API error type enumeration
/// These match the exact error types returned by the OpenAI API
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ErrorType {
    /// Invalid request parameters or format
    InvalidRequestError,

    /// Authentication failed or missing
    AuthenticationError,

    /// Insufficient permissions for the requested operation
    PermissionError,

    /// Rate limit exceeded
    RateLimitError,

    /// Internal server error
    ServerError,

    /// Service temporarily overloaded
    OverloadedError,

    /// Model not found or unavailable
    NotFoundError,

    /// Request timeout
    TimeoutError,

    /// Conflict with current state
    ConflictError,

    /// Unprocessable entity (validation errors)
    UnprocessableEntityError,
}

impl ErrorResponse {
    /// Create a new error response
    pub fn new(error_type: ErrorType, message: String) -> Self {
        Self {
            error: ErrorDetails {
                message,
                error_type,
                code: None,
                param: None,
            },
        }
    }

    /// Create an error response with a specific parameter
    pub fn with_param(error_type: ErrorType, message: String, param: String) -> Self {
        Self {
            error: ErrorDetails {
                message,
                error_type,
                code: None,
                param: Some(param),
            },
        }
    }

    /// Create an error response with a specific error code
    pub fn with_code(error_type: ErrorType, message: String, code: String) -> Self {
        Self {
            error: ErrorDetails {
                message,
                error_type,
                code: Some(code),
                param: None,
            },
        }
    }

    /// Create a full error response with all fields
    pub fn full(
        error_type: ErrorType,
        message: String,
        code: Option<String>,
        param: Option<String>,
    ) -> Self {
        Self {
            error: ErrorDetails {
                message,
                error_type,
                code,
                param,
            },
        }
    }

    // Convenience methods for common error types

    /// Invalid request error
    pub fn invalid_request<S: Into<String>>(message: S) -> Self {
        Self::new(ErrorType::InvalidRequestError, message.into())
    }

    /// Invalid request error with parameter
    pub fn invalid_request_param<S: Into<String>>(message: S, param: S) -> Self {
        Self::with_param(ErrorType::InvalidRequestError, message.into(), param.into())
    }

    /// Authentication error
    pub fn authentication<S: Into<String>>(message: S) -> Self {
        Self::new(ErrorType::AuthenticationError, message.into())
    }

    /// Authentication error with code
    pub fn authentication_code<S: Into<String>>(message: S, code: S) -> Self {
        Self::with_code(ErrorType::AuthenticationError, message.into(), code.into())
    }

    /// Permission error
    pub fn permission<S: Into<String>>(message: S) -> Self {
        Self::new(ErrorType::PermissionError, message.into())
    }

    /// Rate limit error
    pub fn rate_limit<S: Into<String>>(message: S) -> Self {
        Self::new(ErrorType::RateLimitError, message.into())
    }

    /// Server error
    pub fn server<S: Into<String>>(message: S) -> Self {
        Self::new(ErrorType::ServerError, message.into())
    }

    /// Overloaded error (503)
    pub fn overloaded<S: Into<String>>(message: S) -> Self {
        Self::new(ErrorType::OverloadedError, message.into())
    }

    /// Not found error
    pub fn not_found<S: Into<String>>(message: S) -> Self {
        Self::new(ErrorType::NotFoundError, message.into())
    }

    /// Timeout error
    pub fn timeout<S: Into<String>>(message: S) -> Self {
        Self::new(ErrorType::TimeoutError, message.into())
    }

    /// Model-specific errors
    /// Unsupported model error
    pub fn unsupported_model<S: Into<String>>(model: S) -> Self {
        Self::invalid_request_param(
            format!("The model '{}' is not supported", model.into()),
            "model".to_string(),
        )
    }

    /// Empty messages error
    pub fn empty_messages() -> Self {
        Self::invalid_request_param(
            "Messages array cannot be empty".to_string(),
            "messages".to_string(),
        )
    }

    /// Invalid temperature error
    pub fn invalid_temperature(value: f64) -> Self {
        Self::invalid_request_param(
            format!("Temperature must be between 0.0 and 2.0, got: {}", value),
            "temperature".to_string(),
        )
    }

    /// Invalid max_tokens error
    pub fn invalid_max_tokens(value: u32) -> Self {
        Self::invalid_request_param(
            format!("max_tokens must be between 1 and 200,000, got: {}", value),
            "max_tokens".to_string(),
        )
    }

    /// Invalid top_p error
    pub fn invalid_top_p(value: f64) -> Self {
        Self::invalid_request_param(
            format!("top_p must be between 0.0 and 1.0, got: {}", value),
            "top_p".to_string(),
        )
    }

    /// Missing authorization error
    pub fn missing_authorization() -> Self {
        Self::authentication_code(
            "Missing Authorization header".to_string(),
            "missing_authorization".to_string(),
        )
    }

    /// Invalid authorization format error
    pub fn invalid_authorization_format() -> Self {
        Self::authentication_code(
            "Authorization header must be in format 'Bearer <token>'".to_string(),
            "invalid_authorization_format".to_string(),
        )
    }

    /// Invalid JWT token error
    pub fn invalid_jwt_token<S: Into<String>>(details: S) -> Self {
        Self::authentication_code(
            format!("Invalid JWT token: {}", details.into()),
            "invalid_jwt_token".to_string(),
        )
    }

    /// Expired JWT token error
    pub fn expired_jwt_token() -> Self {
        Self::authentication_code(
            "JWT token has expired".to_string(),
            "expired_jwt_token".to_string(),
        )
    }

    /// Malformed JSON error
    pub fn malformed_json<S: Into<String>>(details: S) -> Self {
        Self::invalid_request(format!("Malformed JSON: {}", details.into()))
    }

    /// Missing required field error
    pub fn missing_field<S: Into<String>>(field: S) -> Self {
        let field_str = field.into();
        Self::invalid_request_param(
            format!("Missing required field: {}", field_str),
            field_str,
        )
    }

    /// Provider error (when Vertex AI fails)
    pub fn provider_error<S: Into<String>>(details: S) -> Self {
        Self::server(format!("Provider error: {}", details.into()))
    }

    /// Provider unavailable error
    pub fn provider_unavailable() -> Self {
        Self::overloaded("AI provider is temporarily unavailable. Please try again later.".to_string())
    }

    /// Create error from provider-specific error type
    pub fn from_provider_error(provider: Provider, error: ProviderErrorType) -> Self {
        match error {
            ProviderErrorType::Authentication => {
                Self::authentication_code(
                    format!("Authentication failed with {}", provider.display_name()),
                    "provider_authentication_failed".to_string(),
                )
            }
            ProviderErrorType::Authorization => {
                Self::permission(format!(
                    "Insufficient permissions for {} provider",
                    provider.display_name()
                ))
            }
            ProviderErrorType::RateLimit { reset_time } => {
                let reset_seconds = reset_time
                    .duration_since(SystemTime::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs();
                Self::full(
                    ErrorType::RateLimitError,
                    format!(
                        "Rate limit exceeded for {}. Reset at: {}",
                        provider.display_name(),
                        reset_seconds
                    ),
                    Some("rate_limit_exceeded".to_string()),
                    None,
                )
            }
            ProviderErrorType::QuotaExceeded => {
                Self::rate_limit(format!(
                    "Quota exceeded for {} provider",
                    provider.display_name()
                ))
            }
            ProviderErrorType::ModelNotAvailable => {
                Self::not_found(format!(
                    "Requested model is not available on {}",
                    provider.display_name()
                ))
            }
            ProviderErrorType::ServiceUnavailable => {
                Self::overloaded(format!(
                    "{} service is temporarily unavailable",
                    provider.display_name()
                ))
            }
            ProviderErrorType::Timeout => {
                Self::timeout(format!(
                    "Request to {} timed out",
                    provider.display_name()
                ))
            }
            ProviderErrorType::NetworkError => {
                Self::server(format!(
                    "Network error communicating with {}",
                    provider.display_name()
                ))
            }
            ProviderErrorType::InvalidRequest => {
                Self::invalid_request(format!(
                    "Invalid request format for {}",
                    provider.display_name()
                ))
            }
            ProviderErrorType::InternalError => {
                Self::server(format!(
                    "Internal error in {} provider",
                    provider.display_name()
                ))
            }
            ProviderErrorType::Unknown { message } => {
                Self::server(format!(
                    "Unknown error in {}: {}",
                    provider.display_name(),
                    message
                ))
            }
        }
    }

    /// Provider fallback exhausted error
    pub fn fallback_exhausted<S: Into<String>>(last_errors: Vec<(Provider, ProviderErrorType)>) -> Self {
        let error_summary: Vec<String> = last_errors
            .iter()
            .map(|(provider, error)| {
                format!("{}: {:?}", provider.display_name(), error)
            })
            .collect();

        Self::overloaded(format!(
            "All providers failed. Last errors: [{}]",
            error_summary.join(", ")
        ))
    }

    /// Circuit breaker open error
    pub fn circuit_breaker_open<S: Into<String>>(provider: Provider) -> Self {
        Self::overloaded(format!(
            "{} is currently unavailable due to repeated failures (circuit breaker open)",
            provider.display_name()
        ))
    }

    /// Provider routing error
    pub fn provider_routing_failed<S: Into<String>>(reason: S) -> Self {
        Self::server(format!("Provider routing failed: {}", reason.into()))
    }

    /// Fallback configuration error
    pub fn fallback_config_error<S: Into<String>>(details: S) -> Self {
        Self::server(format!("Fallback configuration error: {}", details.into()))
    }

    /// Model mapping error (when requested model isn't available on selected provider)
    pub fn model_mapping_error<S: Into<String>>(requested_model: S, provider: Provider, available_model: S) -> Self {
        Self::invalid_request_param(
            format!(
                "Model '{}' not available on {}. Using '{}' instead",
                requested_model.into(),
                provider.display_name(),
                available_model.into()
            ),
            "model".to_string(),
        )
    }

    /// Get the HTTP status code for this error type
    pub fn status_code(&self) -> u16 {
        match self.error.error_type {
            ErrorType::InvalidRequestError => 400,
            ErrorType::AuthenticationError => 401,
            ErrorType::PermissionError => 403,
            ErrorType::NotFoundError => 404,
            ErrorType::ConflictError => 409,
            ErrorType::UnprocessableEntityError => 422,
            ErrorType::RateLimitError => 429,
            ErrorType::ServerError => 500,
            ErrorType::OverloadedError => 503,
            ErrorType::TimeoutError => 408,
        }
    }

    /// Convert to JSON string for HTTP response
    pub fn to_json(&self) -> String {
        serde_json::to_string(self).unwrap_or_else(|_| {
            r#"{"error":{"message":"Internal error serializing error response","type":"server_error"}}"#
                .to_string()
        })
    }
}

impl std::fmt::Display for ErrorResponse {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.error.error_type_str(), self.error.message)
    }
}

impl ErrorDetails {
    /// Get a string representation of the error type
    fn error_type_str(&self) -> &str {
        match self.error_type {
            ErrorType::InvalidRequestError => "invalid_request_error",
            ErrorType::AuthenticationError => "authentication_error",
            ErrorType::PermissionError => "permission_error",
            ErrorType::RateLimitError => "rate_limit_error",
            ErrorType::ServerError => "server_error",
            ErrorType::OverloadedError => "overloaded_error",
            ErrorType::NotFoundError => "not_found_error",
            ErrorType::TimeoutError => "timeout_error",
            ErrorType::ConflictError => "conflict_error",
            ErrorType::UnprocessableEntityError => "unprocessable_entity_error",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_error_creation() {
        let error = ErrorResponse::invalid_request("Test error message");

        assert_eq!(error.error.message, "Test error message");
        assert!(matches!(error.error.error_type, ErrorType::InvalidRequestError));
        assert_eq!(error.status_code(), 400);
    }

    #[test]
    fn test_error_with_parameter() {
        let error = ErrorResponse::invalid_request_param("Invalid temperature", "temperature");

        assert_eq!(error.error.param, Some("temperature".to_string()));
        assert!(matches!(error.error.error_type, ErrorType::InvalidRequestError));
    }

    #[test]
    fn test_authentication_errors() {
        let missing_auth = ErrorResponse::missing_authorization();
        assert_eq!(missing_auth.status_code(), 401);
        assert_eq!(missing_auth.error.code, Some("missing_authorization".to_string()));

        let invalid_format = ErrorResponse::invalid_authorization_format();
        assert_eq!(invalid_format.status_code(), 401);
        assert_eq!(invalid_format.error.code, Some("invalid_authorization_format".to_string()));

        let expired_token = ErrorResponse::expired_jwt_token();
        assert_eq!(expired_token.status_code(), 401);
        assert_eq!(expired_token.error.code, Some("expired_jwt_token".to_string()));
    }

    #[test]
    fn test_model_specific_errors() {
        let unsupported = ErrorResponse::unsupported_model("gpt-4");
        assert_eq!(unsupported.error.param, Some("model".to_string()));
        assert!(unsupported.error.message.contains("gpt-4"));

        let empty_messages = ErrorResponse::empty_messages();
        assert_eq!(empty_messages.error.param, Some("messages".to_string()));

        let invalid_temp = ErrorResponse::invalid_temperature(3.0);
        assert_eq!(invalid_temp.error.param, Some("temperature".to_string()));
        assert!(invalid_temp.error.message.contains("3"));
    }

    #[test]
    fn test_status_codes() {
        assert_eq!(ErrorResponse::invalid_request("").status_code(), 400);
        assert_eq!(ErrorResponse::authentication("").status_code(), 401);
        assert_eq!(ErrorResponse::permission("").status_code(), 403);
        assert_eq!(ErrorResponse::not_found("").status_code(), 404);
        assert_eq!(ErrorResponse::rate_limit("").status_code(), 429);
        assert_eq!(ErrorResponse::server("").status_code(), 500);
        assert_eq!(ErrorResponse::overloaded("").status_code(), 503);
        assert_eq!(ErrorResponse::timeout("").status_code(), 408);
    }

    #[test]
    fn test_serialization() {
        let error = ErrorResponse::invalid_request_param("Test message", "test_param");
        let json = serde_json::to_string(&error).unwrap();

        // Should contain proper structure
        assert!(json.contains("\"error\":{"));
        assert!(json.contains("\"type\":\"invalid_request_error\""));
        assert!(json.contains("\"message\":\"Test message\""));
        assert!(json.contains("\"param\":\"test_param\""));

        // Should be deserializable
        let deserialized: ErrorResponse = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.error.message, "Test message");
        assert_eq!(deserialized.error.param, Some("test_param".to_string()));
    }

    #[test]
    fn test_provider_errors() {
        let provider_error = ErrorResponse::provider_error("Vertex AI timeout");
        assert_eq!(provider_error.status_code(), 500);
        assert!(provider_error.error.message.contains("Vertex AI timeout"));

        let unavailable = ErrorResponse::provider_unavailable();
        assert_eq!(unavailable.status_code(), 503);
        assert!(unavailable.error.message.contains("temporarily unavailable"));
    }

    #[test]
    fn test_to_json() {
        let error = ErrorResponse::invalid_request("Test");
        let json = error.to_json();

        // Should be valid JSON
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert!(parsed["error"]["message"].as_str().unwrap() == "Test");
    }

    #[test]
    fn test_display() {
        let error = ErrorResponse::invalid_request("Test message");
        let display_str = format!("{}", error);

        assert!(display_str.contains("invalid_request_error"));
        assert!(display_str.contains("Test message"));
    }

    #[test]
    fn test_provider_error_conversion() {
        use crate::features::provider_fallback::models::{Provider, ProviderErrorType};
        use std::time::SystemTime;

        // Test authentication error
        let auth_error = ErrorResponse::from_provider_error(
            Provider::VertexAI,
            ProviderErrorType::Authentication,
        );
        assert_eq!(auth_error.status_code(), 401);
        assert!(auth_error.error.message.contains("Authentication failed"));
        assert!(auth_error.error.message.contains("Google Vertex AI"));

        // Test rate limit error
        let rate_limit_error = ErrorResponse::from_provider_error(
            Provider::Groq,
            ProviderErrorType::RateLimit {
                reset_time: SystemTime::UNIX_EPOCH + std::time::Duration::from_secs(1700000000),
            },
        );
        assert_eq!(rate_limit_error.status_code(), 429);
        assert!(rate_limit_error.error.message.contains("Rate limit exceeded"));
        assert!(rate_limit_error.error.message.contains("Groq"));

        // Test service unavailable
        let unavailable_error = ErrorResponse::from_provider_error(
            Provider::VertexAI,
            ProviderErrorType::ServiceUnavailable,
        );
        assert_eq!(unavailable_error.status_code(), 503);
        assert!(unavailable_error.error.message.contains("temporarily unavailable"));
    }

    #[test]
    fn test_fallback_exhausted_error() {
        use crate::features::provider_fallback::models::{Provider, ProviderErrorType};

        let last_errors = vec![
            (Provider::VertexAI, ProviderErrorType::Timeout),
            (Provider::Groq, ProviderErrorType::ServiceUnavailable),
        ];

        let error = ErrorResponse::fallback_exhausted(last_errors);
        assert_eq!(error.status_code(), 503);
        assert!(error.error.message.contains("All providers failed"));
        assert!(error.error.message.contains("Google Vertex AI"));
        assert!(error.error.message.contains("Groq"));
    }

    #[test]
    fn test_circuit_breaker_error() {
        use crate::features::provider_fallback::models::Provider;

        let error = ErrorResponse::circuit_breaker_open(Provider::VertexAI);
        assert_eq!(error.status_code(), 503);
        assert!(error.error.message.contains("circuit breaker open"));
        assert!(error.error.message.contains("Google Vertex AI"));
    }

    #[test]
    fn test_model_mapping_error() {
        use crate::features::provider_fallback::models::Provider;

        let error = ErrorResponse::model_mapping_error(
            "gpt-4",
            Provider::VertexAI,
            "claude-3-5-sonnet@20241022",
        );
        assert_eq!(error.status_code(), 400);
        assert_eq!(error.error.param, Some("model".to_string()));
        assert!(error.error.message.contains("gpt-4"));
        assert!(error.error.message.contains("claude-3-5-sonnet@20241022"));
    }
}