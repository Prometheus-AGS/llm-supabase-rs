# Implementation Guide

**Project:** Universal AI Server Proxy (llm-supabase-rs)  
**Version:** 2.0  
**Date:** October 2, 2025

## Document Purpose

This guide provides practical, step-by-step instructions for implementing the Universal AI Server Proxy. It complements the Functional Specification and Technical Architecture documents by focusing on "how to build" rather than "what to build."

## Prerequisites

### Required Skills
- Rust programming (intermediate to advanced)
- Async programming with Tokio
- REST API design
- Cloud services (GCP, AWS)
- Database design (PostgreSQL)
- Docker/Kubernetes basics

### Development Environment

#### 1. Install Rust
```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
rustup update stable
rustup default stable
```

#### 2. Install Development Tools
```bash
# SQLx CLI for database migrations
cargo install sqlx-cli --no-default-features --features postgres

# Additional tools
cargo install cargo-watch    # Auto-reload on changes
cargo install cargo-nextest  # Better test runner
cargo install cargo-audit    # Security auditing
cargo install cargo-deny     # Dependency checking
```

#### 3. Install Required Services
```bash
# PostgreSQL (via Docker)
docker run -d \
  --name postgres \
  -e POSTGRES_PASSWORD=postgres \
  -e POSTGRES_DB=llm_proxy \
  -p 5432:5432 \
  postgres:15

# Redis (optional, for caching)
docker run -d \
  --name redis \
  -p 6379:6379 \
  redis:7
```

---

## Phase 1: Project Foundation (Week 1-4)

### Step 1: Initialize Project Structure

```bash
# Create project
cargo new --lib llm-supabase-rs
cd llm-supabase-rs

# Create directory structure
mkdir -p src/{api,domain,infrastructure,config,shared}
mkdir -p src/api/{handlers,middleware}
mkdir -p src/domain/{providers,tools,models,services}
mkdir -p src/infrastructure/{providers,database,webhooks,mcp}
mkdir -p tests/{unit,integration}
mkdir -p docs
mkdir -p examples
```

### Step 2: Update Cargo.toml

Add dependencies progressively. Start with core dependencies:

```toml
[package]
name = "llm-supabase-rs"
version = "0.1.0"
edition = "2021"

[dependencies]
# Web framework
axum = { version = "0.7", features = ["macros"] }
tokio = { version = "1.0", features = ["full"] }
tower = "0.4"
tower-http = { version = "0.5", features = ["trace", "cors"] }

# Serialization
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"

# HTTP client
reqwest = { version = "0.11", features = ["json", "stream"] }

# Authentication
jsonwebtoken = "9.2"

# Error handling
anyhow = "1.0"
thiserror = "1.0"

# Logging
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter"] }

# Async utilities
futures = "0.3"
async-trait = "0.1"

# Configuration
config = "0.14"
dotenv = "0.15"

# Database
sqlx = { version = "0.7", features = ["runtime-tokio-rustls", "postgres", "uuid", "chrono"] }

# Utilities
uuid = { version = "1.6", features = ["v4", "serde"] }
chrono = { version = "0.4", features = ["serde"] }
dashmap = "5.5"

[dev-dependencies]
mockall = "0.12"
tokio-test = "0.4"
```

### Step 3: Create Configuration System

**File: `src/config/mod.rs`**
```rust
pub mod app;
pub mod providers;

pub use app::AppConfig;
pub use providers::ProvidersConfig;
```

**File: `src/config/app.rs`**
```rust
use serde::Deserialize;
use std::net::SocketAddr;

#[derive(Debug, Clone, Deserialize)]
pub struct AppConfig {
    pub server: ServerConfig,
    pub supabase: SupabaseConfig,
    pub providers: ProvidersConfig,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ServerConfig {
    pub host: String,
    pub port: u16,
}

impl ServerConfig {
    pub fn addr(&self) -> SocketAddr {
        format!("{}:{}", self.host, self.port)
            .parse()
            .expect("Invalid server address")
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct SupabaseConfig {
    pub url: String,
    pub anon_key: String,
    pub service_role_key: String,
    pub jwt_secret: String,
}

impl AppConfig {
    pub fn load() -> anyhow::Result<Self> {
        dotenv::dotenv().ok();
        
        let config = config::Config::builder()
            .add_source(config::File::with_name("config").required(false))
            .add_source(config::Environment::with_prefix("APP"))
            .build()?;
        
        Ok(config.try_deserialize()?)
    }
}
```

**File: `config.yaml`**
```yaml
server:
  host: "0.0.0.0"
  port: 8080

supabase:
  url: "${SUPABASE_URL}"
  anon_key: "${SUPABASE_ANON_KEY}"
  service_role_key: "${SUPABASE_SERVICE_ROLE_KEY}"
  jwt_secret: "${SUPABASE_JWT_SECRET}"

providers:
  vertex_ai:
    enabled: true
    project_id: "${GCP_PROJECT_ID}"
    location: "us-central1"
```

**File: `.env`**
```bash
# Server
APP_SERVER__HOST=0.0.0.0
APP_SERVER__PORT=8080

# Supabase
SUPABASE_URL=https://your-project.supabase.co
SUPABASE_ANON_KEY=your_anon_key
SUPABASE_SERVICE_ROLE_KEY=your_service_role_key
SUPABASE_JWT_SECRET=your_jwt_secret

# GCP
GCP_PROJECT_ID=your-project-id
GOOGLE_APPLICATION_CREDENTIALS=./gcp-credentials.json
```

### Step 4: Create Error Types

**File: `src/shared/error.rs`**
```rust
use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde_json::json;

#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("Authentication failed: {0}")]
    AuthError(String),
    
    #[error("Provider error: {0}")]
    ProviderError(String),
    
    #[error("Configuration error: {0}")]
    ConfigError(String),
    
    #[error("Database error: {0}")]
    DatabaseError(#[from] sqlx::Error),
    
    #[error("Invalid request: {0}")]
    ValidationError(String),
    
    #[error("Internal server error: {0}")]
    InternalError(String),
    
    #[error("Not found: {0}")]
    NotFoundError(String),
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, error_message) = match self {
            AppError::AuthError(msg) => (StatusCode::UNAUTHORIZED, msg),
            AppError::ValidationError(msg) => (StatusCode::BAD_REQUEST, msg),
            AppError::NotFoundError(msg) => (StatusCode::NOT_FOUND, msg),
            AppError::ProviderError(msg) => (StatusCode::BAD_GATEWAY, msg),
            _ => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Internal server error".to_string(),
            ),
        };

        let body = Json(json!({
            "error": {
                "message": error_message,
                "type": self.error_type(),
            }
        }));

        (status, body).into_response()
    }
}

impl AppError {
    fn error_type(&self) -> &str {
        match self {
            AppError::AuthError(_) => "authentication_error",
            AppError::ProviderError(_) => "provider_error",
            AppError::ConfigError(_) => "configuration_error",
            AppError::DatabaseError(_) => "database_error",
            AppError::ValidationError(_) => "validation_error",
            AppError::InternalError(_) => "internal_error",
            AppError::NotFoundError(_) => "not_found",
        }
    }
}

pub type Result<T> = std::result::Result<T, AppError>;
```

**File: `src/shared/mod.rs`**
```rust
pub mod error;

pub use error::{AppError, Result};
```

### Step 5: Create OpenAI API Types

**File: `src/domain/models/chat.rs`**
```rust
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatCompletionRequest {
    pub model: String,
    pub messages: Vec<Message>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_tokens: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stream: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tools: Option<Vec<Tool>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_choice: Option<ToolChoice>,
    
    // Extended fields
    #[serde(skip_serializing_if = "Option::is_none")]
    pub provider_override: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub role: String,
    pub content: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<Vec<ToolCall>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_call_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tool {
    pub r#type: String,
    pub function: FunctionDefinition,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionDefinition {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub parameters: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCall {
    pub id: String,
    pub r#type: String,
    pub function: FunctionCall,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionCall {
    pub name: String,
    pub arguments: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ToolChoice {
    Auto,
    None,
    Required,
    Specific { r#type: String, function: FunctionName },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionName {
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatCompletionResponse {
    pub id: String,
    pub object: String,
    pub created: i64,
    pub model: String,
    pub choices: Vec<Choice>,
    pub usage: Usage,
    
    // Extended fields
    #[serde(skip_serializing_if = "Option::is_none")]
    pub provider: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Choice {
    pub index: usize,
    pub message: Message,
    pub finish_reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Usage {
    pub prompt_tokens: usize,
    pub completion_tokens: usize,
    pub total_tokens: usize,
}

impl ChatCompletionRequest {
    pub fn validate(&self) -> crate::shared::Result<()> {
        if self.messages.is_empty() {
            return Err(crate::shared::AppError::ValidationError(
                "Messages cannot be empty".to_string(),
            ));
        }
        
        if self.messages.len() > 100 {
            return Err(crate::shared::AppError::ValidationError(
                "Too many messages (max: 100)".to_string(),
            ));
        }
        
        if let Some(temp) = self.temperature {
            if !(0.0..=2.0).contains(&temp) {
                return Err(crate::shared::AppError::ValidationError(
                    "Temperature must be between 0 and 2".to_string(),
                ));
            }
        }
        
        Ok(())
    }
}
```

### Step 6: Create Basic API Structure

**File: `src/api/mod.rs`**
```rust
pub mod handlers;
pub mod middleware;
pub mod routes;

pub use routes::create_router;
```

**File: `src/api/routes.rs`**
```rust
use axum::{
    routing::{get, post},
    Router,
};
use crate::app::AppState;

pub fn create_router(state: AppState) -> Router {
    Router::new()
        .route("/health", get(health_check))
        .route("/v1/chat/completions", post(super::handlers::chat::chat_completions))
        .route("/v1/models", get(super::handlers::models::list_models))
        .with_state(state)
}

async fn health_check() -> &'static str {
    "OK"
}
```

**File: `src/api/handlers/mod.rs`**
```rust
pub mod chat;
pub mod models;
```

**File: `src/api/handlers/chat.rs`**
```rust
use axum::{
    extract::State,
    Json,
};
use crate::{
    app::AppState,
    domain::models::chat::{ChatCompletionRequest, ChatCompletionResponse},
    shared::Result,
};

pub async fn chat_completions(
    State(state): State<AppState>,
    Json(req): Json<ChatCompletionRequest>,
) -> Result<Json<ChatCompletionResponse>> {
    // Validate request
    req.validate()?;
    
    // TODO: Implement chat completion logic
    
    Err(crate::shared::AppError::InternalError(
        "Not implemented yet".to_string(),
    ))
}
```

### Step 7: Create Application State

**File: `src/app.rs`**
```rust
use crate::config::AppConfig;
use std::sync::Arc;

#[derive(Clone)]
pub struct AppState {
    pub config: Arc<AppConfig>,
}

impl AppState {
    pub fn new(config: AppConfig) -> Self {
        Self {
            config: Arc::new(config),
        }
    }
}
```

### Step 8: Create Main Entry Point

**File: `src/main.rs`**
```rust
use llm_supabase_rs::{app::AppState, api::create_router, config::AppConfig};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize tracing
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "llm_supabase_rs=debug,tower_http=debug".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    // Load configuration
    let config = AppConfig::load()?;
    let addr = config.server.addr();

    tracing::info!("Loading configuration...");
    tracing::info!("Server will listen on {}", addr);

    // Create application state
    let state = AppState::new(config);

    // Create router
    let app = create_router(state);

    // Create TCP listener
    let listener = tokio::net::TcpListener::bind(addr).await?;
    
    tracing::info!("🚀 Server started on {}", addr);

    // Run server
    axum::serve(listener, app).await?;

    Ok(())
}
```

**File: `src/lib.rs`**
```rust
pub mod api;
pub mod app;
pub mod config;
pub mod domain;
pub mod infrastructure;
pub mod shared;

pub use app::AppState;
```

### Step 9: Test Basic Setup

```bash
# Build the project
cargo build

# Run the server
cargo run

# In another terminal, test the health endpoint
curl http://localhost:8080/health
# Should return: OK
```

---

## Development Workflow

### Daily Development Process

1. **Start with Tests**
   ```bash
   # Write a failing test first
   cargo test --test your_test_name
   ```

2. **Implement Feature**
   ```bash
   # Use cargo watch for auto-reload
   cargo watch -x check -x test
   ```

3. **Check Code Quality**
   ```bash
   # Format code
   cargo fmt
   
   # Run linter
   cargo clippy -- -D warnings
   
   # Check for security issues
   cargo audit
   ```

4. **Commit Changes**
   ```bash
   git add .
   git commit -m "feat: implement feature X"
   ```

### Testing Strategy

#### Unit Tests
Place unit tests in the same file as the code:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validation() {
        let req = ChatCompletionRequest {
            model: "claude-sonnet-4.5".to_string(),
            messages: vec![],
            ..Default::default()
        };
        
        assert!(req.validate().is_err());
    }
}
```

#### Integration Tests
Place in `tests/integration/`:

```rust
// tests/integration/api_test.rs
use llm_supabase_rs::*;

#[tokio::test]
async fn test_health_endpoint() {
    // Test implementation
}
```

### Debugging Tips

1. **Enable Detailed Logging**
   ```bash
   RUST_LOG=debug cargo run
   ```

2. **Use Rust Analyzer**
   Install in VS Code for better IDE support

3. **Print Debug Information**
   ```rust
   dbg!(&variable);
   tracing::debug!("Value: {:?}", variable);
   ```

---

## Next Steps

After completing the foundation:

1. **Implement Authentication** (Week 2)
   - JWT validation middleware
   - Supabase integration
   - Role-based access control

2. **Vertex AI Provider** (Week 3)
   - GCP authentication
   - Format conversion
   - API integration

3. **Follow Implementation Roadmap**
   - See `IMPLEMENTATION_ROADMAP.md` for detailed weekly tasks
   - Complete each phase sequentially
   - Write tests for each feature

---

## Common Issues and Solutions

### Issue: Compilation Errors with async-trait

**Solution:**
```rust
use async_trait::async_trait;

#[async_trait]
trait MyTrait {
    async fn my_method(&self);
}
```

### Issue: sqlx Compile-Time Verification Failing

**Solution:**
```bash
# Set DATABASE_URL
export DATABASE_URL=postgres://postgres:postgres@localhost:5432/llm_proxy

# Or use offline mode
export SQLX_OFFLINE=true
```

### Issue: Provider API Rate Limits

**Solution:**
- Implement exponential backoff
- Add request queuing
- Use circuit breakers

---

## Resources

### Documentation
- [Rust Book](https://doc.rust-lang.org/book/)
- [Tokio Tutorial](https://tokio.rs/tokio/tutorial)
- [Axum Documentation](https://docs.rs/axum)
- [SQLx Documentation](https://docs.rs/sqlx)

### Community
- [Rust Discord](https://discord.gg/rust-lang)
- [r/rust](https://reddit.com/r/rust)

### Tools
- [Rust Analyzer](https://rust-analyzer.github.io/)
- [cargo-watch](https://github.com/watchexec/cargo-watch)
- [Postman](https://www.postman.com/) for API testing

---

## Conclusion

This implementation guide provides the foundation for building the Universal AI Server Proxy. Follow the weekly roadmap in `IMPLEMENTATION_ROADMAP.md` for detailed task breakdowns.

**Key Principles:**
1. Test-driven development
2. Incremental implementation
3. Regular commits
4. Code review
5. Documentation as you go

Good luck with the implementation!
