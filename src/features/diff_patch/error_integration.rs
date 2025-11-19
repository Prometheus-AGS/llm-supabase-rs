// src/features/diff_patch/error_integration.rs
//
// Integration of diff/patch errors with the existing error system

use crate::features::diff_patch::models::PatchError;
use crate::models::error::{ErrorDetails, ErrorResponse, ErrorType};
use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
};

/// Convert PatchError to HTTP response
impl IntoResponse for PatchError {
    fn into_response(self) -> Response {
        let (status, error_type, message) = match &self {
            PatchError::IoError { message } => {
                (StatusCode::INTERNAL_SERVER_ERROR, ErrorType::ServerError, message.clone())
            }
            PatchError::InvalidPatchFormat { message } => {
                (StatusCode::BAD_REQUEST, ErrorType::InvalidRequestError, message.clone())
            }
            PatchError::InvalidChunk { message } => {
                (StatusCode::BAD_REQUEST, ErrorType::InvalidRequestError, message.clone())
            }
            PatchError::FileNotFound { path } => {
                (
                    StatusCode::NOT_FOUND,
                    ErrorType::NotFoundError,
                    format!("File not found: {}", path),
                )
            }
            PatchError::PermissionDenied { message } => {
                (StatusCode::FORBIDDEN, ErrorType::PermissionError, message.clone())
            }
            PatchError::PatchApplicationFailed { message } => {
                (StatusCode::UNPROCESSABLE_ENTITY, ErrorType::UnprocessableEntityError, message.clone())
            }
            PatchError::LineMismatch { line_number, expected, actual } => {
                (
                    StatusCode::CONFLICT,
                    ErrorType::ConflictError,
                    format!(
                        "Line mismatch at line {}: expected '{}', found '{}'",
                        line_number, expected, actual
                    ),
                )
            }
            PatchError::ContextMismatch { message } => {
                (StatusCode::CONFLICT, ErrorType::ConflictError, message.clone())
            }
            PatchError::BinaryFileNotSupported { path } => {
                (
                    StatusCode::UNPROCESSABLE_ENTITY,
                    ErrorType::UnprocessableEntityError,
                    format!("Binary file operations not supported: {}", path),
                )
            }
            PatchError::BackupFailed { message } => {
                (StatusCode::INTERNAL_SERVER_ERROR, ErrorType::ServerError, message.clone())
            }
            PatchError::SecurityViolation { path } => {
                (
                    StatusCode::FORBIDDEN,
                    ErrorType::PermissionError,
                    format!("Security violation detected: {}", path),
                )
            }
            PatchError::FileSizeExceeded { path, size } => {
                (
                    StatusCode::PAYLOAD_TOO_LARGE,
                    ErrorType::InvalidRequestError,
                    format!("File size exceeded for {}: {} bytes", path, size),
                )
            }
        };

        let error_response = ErrorResponse::new(error_type, message);
        (status, axum::Json(error_response)).into_response()
    }
}

/// Convert PatchError to OpenAI-compatible error response
impl From<PatchError> for ErrorResponse {
    fn from(error: PatchError) -> Self {
        let (error_type, message) = match error {
            PatchError::IoError { message } => (ErrorType::ServerError, message),
            PatchError::InvalidPatchFormat { message } => (ErrorType::InvalidRequestError, message),
            PatchError::InvalidChunk { message } => (ErrorType::InvalidRequestError, message),
            PatchError::FileNotFound { path } => {
                (ErrorType::NotFoundError, format!("File not found: {}", path))
            }
            PatchError::PermissionDenied { message } => (ErrorType::PermissionError, message),
            PatchError::PatchApplicationFailed { message } => {
                (ErrorType::UnprocessableEntityError, message)
            }
            PatchError::LineMismatch { line_number, expected, actual } => {
                (
                    ErrorType::ConflictError,
                    format!(
                        "Line mismatch at line {}: expected '{}', found '{}'",
                        line_number, expected, actual
                    ),
                )
            }
            PatchError::ContextMismatch { message } => (ErrorType::ConflictError, message),
            PatchError::BinaryFileNotSupported { path } => {
                (
                    ErrorType::UnprocessableEntityError,
                    format!("Binary file operations not supported: {}", path),
                )
            }
            PatchError::BackupFailed { message } => (ErrorType::ServerError, message),
            PatchError::SecurityViolation { path } => {
                (
                    ErrorType::PermissionError,
                    format!("Security violation detected: {}", path),
                )
            }
            PatchError::FileSizeExceeded { path, size } => {
                (
                    ErrorType::InvalidRequestError,
                    format!("File size exceeded for {}: {} bytes", path, size),
                )
            }
        };

        ErrorResponse::new(error_type, message)
    }
}

/// Error context for patch operations
#[derive(Debug, Clone)]
pub struct PatchErrorContext {
    /// Operation that was being performed
    pub operation: String,
    /// File path being operated on
    pub file_path: Option<String>,
    /// Additional context information
    pub context: std::collections::HashMap<String, String>,
}

impl PatchErrorContext {
    /// Create a new error context
    pub fn new(operation: String) -> Self {
        Self {
            operation,
            file_path: None,
            context: std::collections::HashMap::new(),
        }
    }

    /// Set the file path
    pub fn with_file_path(mut self, path: String) -> Self {
        self.file_path = Some(path);
        self
    }

    /// Add context information
    pub fn with_context<K: Into<String>, V: Into<String>>(mut self, key: K, value: V) -> Self {
        self.context.insert(key.into(), value.into());
        self
    }

    /// Convert to error details
    pub fn to_error_details(&self, base_message: &str) -> ErrorDetails {
        let mut message = format!("{}: {}", self.operation, base_message);
        
        if let Some(ref file_path) = self.file_path {
            message = format!("{} (file: {})", message, file_path);
        }

        if !self.context.is_empty() {
            let context_str: Vec<String> = self.context.iter()
                .map(|(k, v)| format!("{}={}", k, v))
                .collect();
            message = format!("{} [{}]", message, context_str.join(", "));
        }

        ErrorDetails {
            message,
            error_type: ErrorType::ServerError,
            code: None,
            param: self.file_path.clone(),
        }
    }

    /// Create an error response with context
    pub fn create_error_response(&self, error_type: ErrorType, base_message: &str) -> ErrorResponse {
        let details = self.to_error_details(base_message);
        ErrorResponse {
            error: ErrorDetails {
                error_type,
                ..details
            },
        }
    }
}

/// Result type for patch operations with context
pub type PatchResult<T> = Result<T, PatchErrorWithContext>;

/// PatchError with additional context
#[derive(Debug)]
pub struct PatchErrorWithContext {
    pub error: PatchError,
    pub context: Option<PatchErrorContext>,
}

impl PatchErrorWithContext {
    /// Create a new error with context
    pub fn new(error: PatchError, context: PatchErrorContext) -> Self {
        Self {
            error,
            context: Some(context),
        }
    }

    /// Create without context
    pub fn without_context(error: PatchError) -> Self {
        Self {
            error,
            context: None,
        }
    }

    /// Add context to an existing error
    pub fn with_context(mut self, context: PatchErrorContext) -> Self {
        self.context = Some(context);
        self
    }
}

impl From<PatchError> for PatchErrorWithContext {
    fn from(error: PatchError) -> Self {
        Self::without_context(error)
    }
}

impl IntoResponse for PatchErrorWithContext {
    fn into_response(self) -> Response {
        if let Some(context) = self.context {
            let error_response = match self.error {
                PatchError::IoError { ref message } => {
                    context.create_error_response(ErrorType::ServerError, message)
                }
                PatchError::InvalidPatchFormat { ref message } => {
                    context.create_error_response(ErrorType::InvalidRequestError, message)
                }
                PatchError::FileNotFound { ref path } => {
                    context.create_error_response(
                        ErrorType::NotFoundError,
                        &format!("File not found: {}", path),
                    )
                }
                _ => ErrorResponse::from(self.error),
            };

            let status = match self.error {
                PatchError::IoError { .. } => StatusCode::INTERNAL_SERVER_ERROR,
                PatchError::InvalidPatchFormat { .. } => StatusCode::BAD_REQUEST,
                PatchError::FileNotFound { .. } => StatusCode::NOT_FOUND,
                PatchError::PermissionDenied { .. } => StatusCode::FORBIDDEN,
                PatchError::SecurityViolation { .. } => StatusCode::FORBIDDEN,
                PatchError::LineMismatch { .. } => StatusCode::CONFLICT,
                PatchError::ContextMismatch { .. } => StatusCode::CONFLICT,
                PatchError::FileSizeExceeded { .. } => StatusCode::PAYLOAD_TOO_LARGE,
                _ => StatusCode::UNPROCESSABLE_ENTITY,
            };

            (status, axum::Json(error_response)).into_response()
        } else {
            self.error.into_response()
        }
    }
}

/// Helper macros for creating contextualized errors
#[macro_export]
macro_rules! patch_error_with_context {
    ($error:expr, $operation:expr) => {
        PatchErrorWithContext::new($error, PatchErrorContext::new($operation.to_string()))
    };
    
    ($error:expr, $operation:expr, $file:expr) => {
        PatchErrorWithContext::new(
            $error,
            PatchErrorContext::new($operation.to_string()).with_file_path($file.to_string())
        )
    };
}

/// Utility functions for error handling
pub mod utils {
    use super::*;

    /// Log patch errors with context
    pub fn log_patch_error(error: &PatchErrorWithContext) {
        if let Some(ref context) = error.context {
            tracing::error!(
                operation = %context.operation,
                file_path = ?context.file_path,
                context = ?context.context,
                error = %error.error,
                "Patch operation failed"
            );
        } else {
            tracing::error!(error = %error.error, "Patch operation failed");
        }
    }

    /// Create a standardized error message for tool calls
    pub fn format_tool_call_error(
        tool_name: &str,
        error: &PatchError,
        file_path: Option<&str>,
    ) -> String {
        let base_message = match error {
            PatchError::FileNotFound { path } => format!("File not found: {}", path),
            PatchError::PermissionDenied { message } => format!("Permission denied: {}", message),
            PatchError::SecurityViolation { path } => format!("Security violation: {}", path),
            PatchError::InvalidPatchFormat { message } => format!("Invalid patch format: {}", message),
            PatchError::PatchApplicationFailed { message } => format!("Patch failed: {}", message),
            PatchError::LineMismatch { line_number, expected, actual } => {
                format!(
                    "Line mismatch at line {}: expected '{}', found '{}'",
                    line_number,
                    expected.chars().take(50).collect::<String>(),
                    actual.chars().take(50).collect::<String>()
                )
            }
            _ => error.to_string(),
        };

        if let Some(file) = file_path {
            format!("Tool '{}' failed for file '{}': {}", tool_name, file, base_message)
        } else {
            format!("Tool '{}' failed: {}", tool_name, base_message)
        }
    }

    /// Extract file path from patch error
    pub fn extract_file_path(error: &PatchError) -> Option<&str> {
        match error {
            PatchError::FileNotFound { path } => Some(path),
            PatchError::SecurityViolation { path } => Some(path),
            PatchError::BinaryFileNotSupported { path } => Some(path),
            PatchError::FileSizeExceeded { path, .. } => Some(path),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_patch_error_conversion() {
        let error = PatchError::FileNotFound {
            path: "test.txt".to_string(),
        };
        
        let error_response: ErrorResponse = error.into();
        assert!(matches!(error_response.error.error_type, ErrorType::NotFoundError));
        assert!(error_response.error.message.contains("test.txt"));
    }

    #[test]
    fn test_patch_error_context() {
        let context = PatchErrorContext::new("apply_patch".to_string())
            .with_file_path("src/main.rs".to_string())
            .with_context("line", "42")
            .with_context("operation", "insert");

        let error_details = context.to_error_details("Line insertion failed");
        
        assert!(error_details.message.contains("apply_patch"));
        assert!(error_details.message.contains("src/main.rs"));
        assert!(error_details.message.contains("line=42"));
        assert!(error_details.message.contains("operation=insert"));
        assert_eq!(error_details.param, Some("src/main.rs".to_string()));
    }

    #[test]
    fn test_error_with_context_macro() {
        let base_error = PatchError::InvalidPatchFormat {
            message: "Bad format".to_string(),
        };
        
        let error_with_context = patch_error_with_context!(
            base_error,
            "apply_patch",
            "test.rs"
        );

        assert!(error_with_context.context.is_some());
        let context = error_with_context.context.unwrap();
        assert_eq!(context.operation, "apply_patch");
        assert_eq!(context.file_path, Some("test.rs".to_string()));
    }

    #[test]
    fn test_format_tool_call_error() {
        let error = PatchError::LineMismatch {
            line_number: 42,
            expected: "original line".to_string(),
            actual: "different line".to_string(),
        };

        let formatted = utils::format_tool_call_error(
            "apply_patch",
            &error,
            Some("main.rs")
        );

        assert!(formatted.contains("apply_patch"));
        assert!(formatted.contains("main.rs"));
        assert!(formatted.contains("Line mismatch at line 42"));
    }

    #[test]
    fn test_extract_file_path() {
        let error1 = PatchError::FileNotFound {
            path: "missing.txt".to_string(),
        };
        assert_eq!(utils::extract_file_path(&error1), Some("missing.txt"));

        let error2 = PatchError::InvalidPatchFormat {
            message: "Bad format".to_string(),
        };
        assert_eq!(utils::extract_file_path(&error2), None);
    }
}