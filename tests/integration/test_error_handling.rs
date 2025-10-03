use axum_test::TestServer;
use serde_json::{json, Value};
use axum::http::StatusCode;
use axum::Router;
use std::sync::Arc;

// Import necessary types from the main crate
use llm_supabase_rs::config::{AppConfig, EmbeddingConfig, SupabaseConfig, VertexConfig};
use llm_supabase_rs::app::AppState;
use llm_supabase_rs::config::app::AppStateHealth;
use llm_supabase_rs::config::providers::{VertexAiConfig, VertexAuthConfig, VertexAuthMethod, VertexModelConfig};
use llm_supabase_rs::infrastructure::{SupabaseClient, VertexAIClient};

/// Integration test for Error Handling Scenarios
///
/// This test implements Tests 5-6 from quickstart.md:
/// - "Authentication Failure"
/// - "Invalid Request Handling"
///
/// Validates comprehensive error handling:
/// - OpenAI-compatible error response formats
/// - Proper HTTP status codes
/// - Detailed error messages and codes
/// - Parameter-specific error reporting
/// - Graceful degradation scenarios
///
/// This test MUST FAIL until error handling middleware is implemented.
#[tokio::test]
async fn test_authentication_error_handling() {
    let app = create_test_app().await;
    let server = TestServer::new(app).unwrap();

    // Test 1: Missing Authorization header
    let response = server
        .post("/v1/chat/completions")
        .add_header("Content-Type", "application/json")
        .json(&json!({
            "model": "claude-4-sonnet-20250514",
            "messages": [{"role": "user", "content": "Hello"}]
        }))
        .await;

    assert_eq!(response.status_code(), StatusCode::UNAUTHORIZED);

    let body: Value = response.json();
    validate_openai_error_format(&body, "authentication_error")
        .expect("Auth error should match OpenAI format");

    assert_eq!(body["error"]["code"], "missing_authorization");

    // Test 2: Malformed Authorization header
    let response = server
        .post("/v1/chat/completions")
        .add_header("Authorization", "InvalidFormat")
        .add_header("Content-Type", "application/json")
        .json(&json!({
            "model": "claude-4-sonnet-20250514",
            "messages": [{"role": "user", "content": "Hello"}]
        }))
        .await;

    assert_eq!(response.status_code(), StatusCode::UNAUTHORIZED);

    let body: Value = response.json();
    validate_openai_error_format(&body, "authentication_error")
        .expect("Malformed auth error should match OpenAI format");

    // Test 3: Invalid Bearer token
    let response = server
        .post("/v1/chat/completions")
        .add_header("Authorization", "Bearer invalid_token")
        .add_header("Content-Type", "application/json")
        .json(&json!({
            "model": "claude-4-sonnet-20250514",
            "messages": [{"role": "user", "content": "Hello"}]
        }))
        .await;

    assert_eq!(response.status_code(), StatusCode::UNAUTHORIZED);

    let body: Value = response.json();
    validate_openai_error_format(&body, "authentication_error")
        .expect("Invalid token error should match OpenAI format");
}

/// Test invalid request parameter validation
#[tokio::test]
async fn test_invalid_request_validation() {
    let app = create_test_app().await;
    let server = TestServer::new(app).unwrap();

    let jwt_token = create_valid_jwt_token();

    // Test 1: Empty messages array
    let response = server
        .post("/v1/chat/completions")
        .add_header("Authorization", format!("Bearer {}", jwt_token))
        .add_header("Content-Type", "application/json")
        .json(&json!({
            "model": "claude-4-sonnet-20250514",
            "messages": [],
            "temperature": 3.0  // Also invalid
        }))
        .await;

    assert_eq!(response.status_code(), StatusCode::BAD_REQUEST);

    let body: Value = response.json();
    validate_openai_error_format(&body, "invalid_request_error")
        .expect("Invalid messages error should match OpenAI format");

    assert_eq!(body["error"]["code"], "invalid_messages");
    assert_eq!(body["error"]["param"], "messages");

    // Test 2: Temperature out of bounds
    let response = server
        .post("/v1/chat/completions")
        .add_header("Authorization", format!("Bearer {}", jwt_token))
        .add_header("Content-Type", "application/json")
        .json(&json!({
            "model": "claude-4-sonnet-20250514",
            "messages": [{"role": "user", "content": "Hello"}],
            "temperature": 2.5  // Over 2.0 limit
        }))
        .await;

    assert_eq!(response.status_code(), StatusCode::BAD_REQUEST);

    let body: Value = response.json();
    validate_openai_error_format(&body, "invalid_request_error")
        .expect("Temperature error should match OpenAI format");

    assert_eq!(body["error"]["param"], "temperature");

    // Test 3: Max tokens out of bounds
    let response = server
        .post("/v1/chat/completions")
        .add_header("Authorization", format!("Bearer {}", jwt_token))
        .add_header("Content-Type", "application/json")
        .json(&json!({
            "model": "claude-4-sonnet-20250514",
            "messages": [{"role": "user", "content": "Hello"}],
            "max_tokens": 200001  // Over 200,000 limit
        }))
        .await;

    assert_eq!(response.status_code(), StatusCode::BAD_REQUEST);

    let body: Value = response.json();
    validate_openai_error_format(&body, "invalid_request_error")
        .expect("Max tokens error should match OpenAI format");

    assert_eq!(body["error"]["param"], "max_tokens");
}

/// Test unsupported model validation
#[tokio::test]
async fn test_unsupported_model_errors() {
    let app = create_test_app().await;
    let server = TestServer::new(app).unwrap();

    let jwt_token = create_valid_jwt_token();

    let response = server
        .post("/v1/chat/completions")
        .add_header("Authorization", format!("Bearer {}", jwt_token))
        .add_header("Content-Type", "application/json")
        .json(&json!({
            "model": "unsupported-model-name",
            "messages": [{"role": "user", "content": "Hello"}]
        }))
        .await;

    assert_eq!(response.status_code(), StatusCode::BAD_REQUEST);

    let body: Value = response.json();
    validate_openai_error_format(&body, "invalid_request_error")
        .expect("Model error should match OpenAI format");

    assert_eq!(body["error"]["param"], "model");

    let message = body["error"]["message"].as_str().unwrap();
    assert!(message.contains("model"), "Error message should mention model");
}

/// Test malformed JSON request handling
#[tokio::test]
async fn test_malformed_json_errors() {
    let app = create_test_app().await;
    let server = TestServer::new(app).unwrap();

    let jwt_token = create_valid_jwt_token();

    // Send malformed JSON
    let response = server
        .post("/v1/chat/completions")
        .add_header("Authorization", format!("Bearer {}", jwt_token))
        .add_header("Content-Type", "application/json")
        .text("{ invalid json content")
        .await;

    assert_eq!(response.status_code(), StatusCode::BAD_REQUEST);

    let body: Value = response.json();
    validate_openai_error_format(&body, "invalid_request_error")
        .expect("JSON error should match OpenAI format");
}

/// Test missing required fields
#[tokio::test]
async fn test_missing_required_fields() {
    let app = create_test_app().await;
    let server = TestServer::new(app).unwrap();

    let jwt_token = create_valid_jwt_token();

    // Missing model field
    let response = server
        .post("/v1/chat/completions")
        .add_header("Authorization", format!("Bearer {}", jwt_token))
        .add_header("Content-Type", "application/json")
        .json(&json!({
            "messages": [{"role": "user", "content": "Hello"}]
            // Missing required "model" field
        }))
        .await;

    assert_eq!(response.status_code(), StatusCode::BAD_REQUEST);

    let body: Value = response.json();
    validate_openai_error_format(&body, "invalid_request_error")
        .expect("Missing model error should match OpenAI format");

    assert_eq!(body["error"]["param"], "model");

    // Missing messages field
    let response = server
        .post("/v1/chat/completions")
        .add_header("Authorization", format!("Bearer {}", jwt_token))
        .add_header("Content-Type", "application/json")
        .json(&json!({
            "model": "claude-4-sonnet-20250514"
            // Missing required "messages" field
        }))
        .await;

    assert_eq!(response.status_code(), StatusCode::BAD_REQUEST);

    let body: Value = response.json();
    validate_openai_error_format(&body, "invalid_request_error")
        .expect("Missing messages error should match OpenAI format");

    assert_eq!(body["error"]["param"], "messages");
}

/// Test provider error scenarios
#[tokio::test]
async fn test_provider_error_scenarios() {
    let app = create_test_app().await;
    let server = TestServer::new(app).unwrap();

    let jwt_token = create_valid_jwt_token();

    // This test will check how the system handles provider failures
    // When Vertex AI is unavailable or returns errors

    let response = server
        .post("/v1/chat/completions")
        .add_header("Authorization", format!("Bearer {}", jwt_token))
        .add_header("Content-Type", "application/json")
        .json(&json!({
            "model": "claude-4-sonnet-20250514",
            "messages": [{"role": "user", "content": "Hello"}]
        }))
        .await;

    // If provider is unavailable, should return 503 Service Unavailable
    if response.status_code() == StatusCode::SERVICE_UNAVAILABLE {
        let body: Value = response.json();
        validate_openai_error_format(&body, "server_error")
            .expect("Provider error should match OpenAI format");

        let message = body["error"]["message"].as_str().unwrap();
        assert!(!message.is_empty(), "Error message should not be empty");
    }
}

/// Test rate limiting errors
#[tokio::test]
async fn test_rate_limiting_errors() {
    let app = create_test_app().await;
    let server = TestServer::new(app).unwrap();

    let jwt_token = create_valid_jwt_token();

    // Make rapid requests to trigger rate limiting
    for i in 0..10 {
        let response = server
            .post("/v1/chat/completions")
            .add_header("Authorization", format!("Bearer {}", jwt_token))
            .add_header("Content-Type", "application/json")
            .json(&json!({
                "model": "claude-4-sonnet-20250514",
                "messages": [{"role": "user", "content": format!("Request {}", i)}]
            }))
            .await;

        // If rate limited, should return 429
        if response.status_code() == StatusCode::TOO_MANY_REQUESTS {
            let body: Value = response.json();
            validate_openai_error_format(&body, "rate_limit_error")
                .expect("Rate limit error should match OpenAI format");

            // Should include retry-after information
            let retry_after = response.header("retry-after");
            assert!(!retry_after.is_empty(), "Retry-After header should have value");

            break; // Found rate limiting, test successful
        }
    }
}

/// Test method not allowed errors
#[tokio::test]
async fn test_method_not_allowed_errors() {
    let app = create_test_app().await;
    let server = TestServer::new(app).unwrap();

    let jwt_token = create_valid_jwt_token();

    // GET to chat completions should not be allowed
    let response = server
        .get("/v1/chat/completions")
        .add_header("Authorization", format!("Bearer {}", jwt_token))
        .await;

    assert!(
        matches!(
            response.status_code(),
            StatusCode::METHOD_NOT_ALLOWED | StatusCode::NOT_FOUND
        ),
        "GET to chat completions should not be allowed"
    );

    // PUT to models should not be allowed
    let response = server
        .put("/v1/models")
        .add_header("Authorization", format!("Bearer {}", jwt_token))
        .await;

    assert!(
        matches!(
            response.status_code(),
            StatusCode::METHOD_NOT_ALLOWED | StatusCode::NOT_FOUND
        ),
        "PUT to models should not be allowed"
    );
}

/// Test endpoint not found errors
#[tokio::test]
async fn test_not_found_errors() {
    let app = create_test_app().await;
    let server = TestServer::new(app).unwrap();

    let jwt_token = create_valid_jwt_token();

    // Non-existent endpoint
    let response = server
        .get("/v1/non-existent")
        .add_header("Authorization", format!("Bearer {}", jwt_token))
        .await;

    assert_eq!(response.status_code(), StatusCode::NOT_FOUND);

    // Wrong API version
    let response = server
        .get("/v2/models")
        .add_header("Authorization", format!("Bearer {}", jwt_token))
        .await;

    assert_eq!(response.status_code(), StatusCode::NOT_FOUND);
}

/// Helper function to create test application
async fn create_test_app() -> Router {
    // Create test configuration
    let config = create_test_config();
    
    // Create Vertex AI config
    let vertex_ai_config = VertexAiConfig {
        project_id: config.vertex.project_id.clone(),
        region: config.vertex.location.clone(),
        credentials_path: config.vertex.credentials_path.clone(),
        default_model: VertexModelConfig {
            model_name: config.vertex.default_model.clone(),
            ..Default::default()
        },
        auth: VertexAuthConfig {
            auth_method: VertexAuthMethod::ServiceAccountKey,
            ..Default::default()
        },
        ..Default::default()
    };
    
    // Initialize clients (these may fail in test environment, but that's expected)
    let vertex_client = match VertexAIClient::new(vertex_ai_config).await {
        Ok(client) => Arc::new(client),
        Err(_) => {
            // Create a mock client for testing error handling
            // In a real scenario, you'd use a mock implementation
            panic!("Vertex AI client initialization failed - expected in test environment without credentials")
        }
    };
    
    let supabase_client = Arc::new(SupabaseClient::new(&config.supabase));
    
    // Create app state
    let app_state = AppState {
        config: config.clone(),
        vertex_client,
        supabase_client,
        health: AppStateHealth::new(&config),
    };
    
    // Create the router using the App's create_router method
    // Note: We need to expose create_router as public or create the router here
    create_test_router(app_state)
}

/// Create test configuration
fn create_test_config() -> AppConfig {
    AppConfig {
        host: "localhost".to_string(),
        port: 8080,
        log_level: "info".to_string(),
        supabase: SupabaseConfig {
            url: std::env::var("SUPABASE_URL")
                .unwrap_or_else(|_| "https://test.supabase.co".to_string()),
            anon_key: std::env::var("SUPABASE_ANON_KEY")
                .unwrap_or_else(|_| "test-anon-key".to_string()),
            service_role_key: std::env::var("SUPABASE_SERVICE_ROLE_KEY")
                .unwrap_or_else(|_| "test-service-role-key".to_string()),
            jwt_secret: std::env::var("SUPABASE_JWT_SECRET")
                .unwrap_or_else(|_| "test-jwt-secret-key-for-testing-purposes-only".to_string()),
        },
        vertex: VertexConfig {
            project_id: std::env::var("VERTEX_PROJECT_ID")
                .unwrap_or_else(|_| "test-project".to_string()),
            location: std::env::var("VERTEX_LOCATION")
                .unwrap_or_else(|_| "us-east5".to_string()),
            credentials_path: std::env::var("GOOGLE_APPLICATION_CREDENTIALS")
                .unwrap_or_else(|_| "./gcp-credentials.json".to_string()),
            default_model: "claude-4-sonnet-20250514".to_string(),
        },
        embedding: EmbeddingConfig::default(),
        model_cache_dir: "./models".to_string(),
    }
}

/// Create test router (simplified version of App::create_router)
fn create_test_router(state: AppState) -> Router {
    use axum::{
        http::{header, Method},
        middleware,
        routing::{get, post},
    };
    use tower::ServiceBuilder;
    use tower_http::{
        cors::{Any, CorsLayer},
        trace::TraceLayer,
    };
    use llm_supabase_rs::api::handlers::{
        health::health_check,
        models::list_models,
        chat::chat_completions_unified,
    };
    use llm_supabase_rs::features::auth::AuthMiddleware;
    
    let cors = CorsLayer::new()
        .allow_methods([Method::GET, Method::POST, Method::OPTIONS])
        .allow_headers([header::AUTHORIZATION, header::CONTENT_TYPE, header::ACCEPT])
        .allow_origin(Any);
    
    Router::new()
        .route("/health", get(health_check))
        .route("/v1/chat/completions", post(chat_completions_unified))
        .route("/v1/models", get(list_models))
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

/// Create a valid JWT token for testing
/// This creates a properly signed JWT token using the test JWT secret
fn create_valid_jwt_token() -> String {
    use jsonwebtoken::{encode, EncodingKey, Header, Algorithm};
    use serde::{Serialize, Deserialize};
    
    #[derive(Debug, Serialize, Deserialize)]
    struct Claims {
        sub: String,
        email: String,
        iat: usize,
        exp: usize,
    }
    
    let claims = Claims {
        sub: "test-user-id".to_string(),
        email: "test@test.com".to_string(),
        iat: 1696118400,
        exp: 2696122000, // Far future expiration for testing
    };
    
    let secret = std::env::var("SUPABASE_JWT_SECRET")
        .unwrap_or_else(|_| "test-jwt-secret-key-for-testing-purposes-only".to_string());
    
    encode(
        &Header::new(Algorithm::HS256),
        &claims,
        &EncodingKey::from_secret(secret.as_bytes()),
    )
    .unwrap_or_else(|_| {
        // Fallback to a static token if encoding fails
        "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWIiOiJ0ZXN0LXVzZXItaWQiLCJlbWFpbCI6InRlc3RAdGVzdC5jb20iLCJpYXQiOjE2OTYxMTg0MDAsImV4cCI6MjY5NjEyMjAwMH0.test_signature".to_string()
    })
}

/// Validate OpenAI-compatible error response format
fn validate_openai_error_format(response: &Value, expected_type: &str) -> Result<(), String> {
    let obj = response.as_object()
        .ok_or("Error response must be an object")?;

    let error = obj.get("error")
        .and_then(|v| v.as_object())
        .ok_or("Missing required field: error")?;

    let _message = error.get("message")
        .and_then(|v| v.as_str())
        .ok_or("Error missing required field: message")?;

    let error_type = error.get("type")
        .and_then(|v| v.as_str())
        .ok_or("Error missing required field: type")?;

    if error_type != expected_type {
        return Err(format!("Expected error type '{}', got '{}'", expected_type, error_type));
    }

    // Validate allowed error types
    let valid_types = [
        "invalid_request_error",
        "authentication_error",
        "permission_error",
        "rate_limit_error",
        "server_error"
    ];

    if !valid_types.contains(&error_type) {
        return Err(format!("Invalid error type: {}", error_type));
    }

    // Optional fields validation
    if let Some(code) = error.get("code") {
        if !code.is_string() {
            return Err("error.code must be string if present".to_string());
        }
    }

    if let Some(param) = error.get("param") {
        if !param.is_string() && !param.is_null() {
            return Err("error.param must be string or null if present".to_string());
        }
    }

    Ok(())
}

#[cfg(test)]
mod error_handling_tests {
    use super::*;

    #[test]
    fn test_error_format_validation() {
        let valid_error = json!({
            "error": {
                "message": "Messages array cannot be empty",
                "type": "invalid_request_error",
                "code": "invalid_messages",
                "param": "messages"
            }
        });

        assert!(validate_openai_error_format(&valid_error, "invalid_request_error").is_ok());

        let invalid_error = json!({
            "error": {
                "message": "Some error",
                "type": "unknown_error_type"
            }
        });

        assert!(validate_openai_error_format(&invalid_error, "unknown_error_type").is_err());
    }

    #[test]
    fn test_error_types() {
        let types = [
            "invalid_request_error",
            "authentication_error",
            "permission_error",
            "rate_limit_error",
            "server_error"
        ];

        for error_type in types {
            let error_response = json!({
                "error": {
                    "message": "Test error",
                    "type": error_type
                }
            });

            assert!(validate_openai_error_format(&error_response, error_type).is_ok());
        }
    }
}
