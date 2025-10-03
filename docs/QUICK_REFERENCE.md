# Quick Reference Guide

**Project:** Universal AI Server Proxy  
**For:** Developers using Claude Code and GitHub Speckit

## Document Purpose

This is a quick reference for AI coding assistants (like Claude Code) and developers using GitHub Speckit to implement the Universal AI Server Proxy. It provides high-level pointers to detailed documentation.

---

## 📚 Documentation Map

### Primary Documents (Read in Order)
1. **[FUNCTIONAL_SPECIFICATION.md](./FUNCTIONAL_SPECIFICATION.md)** → What to build
2. **[TECHNICAL_ARCHITECTURE.md](./TECHNICAL_ARCHITECTURE.md)** → How to build it
3. **[IMPLEMENTATION_GUIDE.md](./IMPLEMENTATION_GUIDE.md)** → Step-by-step instructions

### Supporting Documents
- **[README.md](./README.md)** → Documentation index
- **progress.md** → Current status
- **ADR-001** → Architecture decisions

---

## 🎯 Quick Start for AI Assistants

### Context Loading Priority

When starting a new task, load context in this order:

1. **Task-Specific Section** from Functional Spec
   - Example: For provider work → Section 4 "Provider Support"
   
2. **Relevant Architecture** from Technical Architecture
   - Example: For provider work → Section 6 "Provider Integration"

3. **Implementation Steps** from Implementation Guide
   - Example: For provider work → Phase 2 tasks

### Key Principles

1. **Follow Clean Architecture**
   - Domain layer is provider-agnostic
   - Infrastructure implements providers
   - API layer handles HTTP only

2. **Use Trait-Based Design**
   ```rust
   // Domain defines the contract
   pub trait LLMProvider: Send + Sync {
       async fn execute(&self, req: ProviderRequest) -> Result<ProviderResponse>;
   }
   
   // Infrastructure implements it
   pub struct VertexAIProvider { ... }
   impl LLMProvider for VertexAIProvider { ... }
   ```

3. **Error Handling**
   - Use `AppError` from `shared::error`
   - Convert external errors early
   - Provide context in error messages

4. **Testing**
   - Unit tests in same file
   - Integration tests in `tests/`
   - Always test error paths

---

## 🗺️ Module Structure

```
src/
├── api/                    # HTTP layer
│   ├── handlers/          # Request handlers
│   ├── middleware/        # Auth, logging, etc.
│   └── routes.rs          # Route definitions
│
├── domain/                # Business logic
│   ├── providers/         # Provider abstraction
│   │   ├── traits.rs      # LLMProvider trait
│   │   ├── registry.rs    # Provider registry
│   │   └── selector.rs    # Selection logic
│   ├── tools/             # Tool abstraction
│   ├── models/            # Domain models
│   └── services/          # Business services
│
├── infrastructure/        # External integrations
│   ├── providers/         # Concrete providers
│   │   ├── vertex/
│   │   ├── bedrock/
│   │   ├── openai/
│   │   └── local/
│   ├── mcp/               # MCP client/server
│   ├── database/          # DB access
│   └── webhooks/          # Webhook system
│
├── config/                # Configuration
├── shared/                # Shared utilities
│   ├── error.rs           # Error types
│   └── types.rs           # Common types
│
├── app.rs                 # App state
├── lib.rs                 # Library root
└── main.rs                # Entry point
```

---

## 🔌 Provider Implementation Pattern

### When Adding a New Provider

**1. Define Configuration**
```rust
// src/config/providers.rs
#[derive(Debug, Clone, Deserialize)]
pub struct NewProviderConfig {
    pub enabled: bool,
    pub api_key: String,
    // ... provider-specific fields
}
```

**2. Create Provider Module**
```rust
// src/infrastructure/providers/new_provider/mod.rs
pub mod client;
pub mod converter;

pub use client::NewProvider;
```

**3. Implement Provider Trait**
```rust
// src/infrastructure/providers/new_provider/client.rs
use crate::domain::providers::traits::LLMProvider;

pub struct NewProvider {
    client: reqwest::Client,
    config: NewProviderConfig,
}

#[async_trait]
impl LLMProvider for NewProvider {
    fn name(&self) -> &str { "new-provider" }
    
    async fn execute(&self, req: ProviderRequest) -> Result<ProviderResponse> {
        // 1. Convert request format
        let native_req = self.convert_request(&req)?;
        
        // 2. Call provider API
        let response = self.client
            .post(&self.endpoint_url())
            .json(&native_req)
            .send()
            .await?;
            
        // 3. Convert response format
        let native_res = response.json().await?;
        self.convert_response(native_res)
    }
    
    // Implement other trait methods...
}
```

**4. Register Provider**
```rust
// src/domain/providers/registry.rs
if config.new_provider.enabled {
    let provider = NewProvider::new(&config.new_provider).await?;
    registry.providers.insert("new-provider".to_string(), Arc::new(provider));
}
```

---

## 🛠️ Common Patterns

### Async Error Handling
```rust
use crate::shared::{AppError, Result};

pub async fn some_operation() -> Result<String> {
    let result = external_call()
        .await
        .map_err(|e| AppError::ProviderError(e.to_string()))?;
        
    Ok(result)
}
```

### State Management
```rust
// Always use Arc for shared state
pub struct AppState {
    pub config: Arc<AppConfig>,
    pub providers: Arc<ProviderRegistry>,
    pub tools: Arc<ToolRegistry>,
}

// Clone is cheap (just Arc clone)
let state = state.clone();
```

### Database Access
```rust
// Use repository pattern
pub struct WebhookRepository {
    pool: Arc<PgPool>,
}

impl WebhookRepository {
    pub async fn find_by_user(&self, user_id: Uuid) -> Result<Vec<Webhook>> {
        sqlx::query_as!(
            Webhook,
            r#"SELECT * FROM webhooks WHERE user_id = $1"#,
            user_id
        )
        .fetch_all(&*self.pool)
        .await
        .map_err(AppError::from)
    }
}
```

### Webhook Publishing
```rust
// Non-blocking event publishing
dispatcher.publish(WebhookEvent::ProcessingStart {
    request_id,
    model: req.model.clone(),
    provider: provider_name.clone(),
    timestamp: Utc::now(),
    user_id: auth.user_id,
    prompt_tokens: token_count,
})?;
```

---

## 📋 Implementation Checklist

### For Each New Feature

- [ ] **Specification**
  - [ ] Feature clearly defined in functional spec
  - [ ] Architecture documented
  - [ ] API contract specified

- [ ] **Implementation**
  - [ ] Code follows module structure
  - [ ] Traits used for abstraction
  - [ ] Error handling implemented
  - [ ] Configuration added

- [ ] **Testing**
  - [ ] Unit tests written
  - [ ] Integration tests added
  - [ ] Error cases tested
  - [ ] Manual testing done

- [ ] **Documentation**
  - [ ] Code comments added
  - [ ] API docs updated
  - [ ] README updated if needed

---

## 🚦 Decision Tree for Common Tasks

### "I need to add a new provider"
1. Check: Does it support OpenAI format?
   - Yes → Minimal conversion needed
   - No → Full converter implementation

2. Go to: [Provider Implementation Pattern](#provider-implementation-pattern)

3. Reference: Functional Spec Section 4.2, Technical Architecture Section 6

### "I need to add a new API endpoint"
1. Add to: `src/api/handlers/`
2. Define types in: `src/domain/models/`
3. Register in: `src/api/routes.rs`
4. Reference: Implementation Guide Step 6

### "I need to implement tool execution"
1. Understand: MCP protocol (Functional Spec Section 7)
2. Implement: Tool registry (Technical Architecture Section 7)
3. Follow: Phase 3 tasks in roadmap

### "I need to add webhooks"
1. Define event in: `src/domain/models/webhook.rs`
2. Publish in: Relevant handler
3. Implement delivery in: `src/infrastructure/webhooks/`
4. Reference: Functional Spec Section 6, Technical Architecture Section 8

### "I need to add a database table"
1. Create migration in: `src/infrastructure/database/migrations/`
2. Define model in: `src/infrastructure/database/models.rs`
3. Create repository in: `src/infrastructure/database/repositories/`
4. Reference: Technical Architecture Section 9

---

## 🎨 Code Style Guide

### Naming Conventions
- **Files**: `snake_case.rs`
- **Modules**: `snake_case`
- **Types**: `PascalCase`
- **Functions**: `snake_case`
- **Constants**: `SCREAMING_SNAKE_CASE`

### Imports
```rust
// Standard library
use std::sync::Arc;

// External crates (alphabetical)
use axum::extract::State;
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;

// Internal crates (alphabetical)
use crate::domain::models::chat::*;
use crate::shared::{AppError, Result};
```

### Error Handling
```rust
// Prefer ? operator
let result = operation().await?;

// Add context when converting errors
.map_err(|e| AppError::ProviderError(
    format!("Failed to call provider: {}", e)
))?;
```

### Async Functions
```rust
// Use async-trait for trait methods
#[async_trait]
pub trait MyTrait {
    async fn my_method(&self) -> Result<String>;
}

// Regular async functions don't need macro
pub async fn my_function() -> Result<String> {
    // implementation
}
```

---

## 🔍 Where to Find Things

### "Where is...?"

| What | Location |
|------|----------|
| OpenAI types | `src/domain/models/chat.rs` |
| Provider trait | `src/domain/providers/traits.rs` |
| Error types | `src/shared/error.rs` |
| Config structs | `src/config/` |
| API handlers | `src/api/handlers/` |
| Database schema | `docs/TECHNICAL_ARCHITECTURE.md` Section 9 |
| Provider specs | `docs/FUNCTIONAL_SPECIFICATION.md` Section 4 |
| MCP protocol | `docs/FUNCTIONAL_SPECIFICATION.md` Section 7 |
| Webhook events | `src/domain/models/webhook.rs` |
| Authentication | `src/api/middleware/auth.rs` |

---

## 🐛 Debugging Tips

### Common Issues

**Issue: "Provider not found"**
```rust
// Check: Is provider registered?
// File: src/domain/providers/registry.rs
// Look for: registry.providers.insert(...)
```

**Issue: "Database connection failed"**
```bash
# Check: Is DATABASE_URL set?
export DATABASE_URL=postgres://user:pass@localhost/dbname

# Check: Are migrations run?
sqlx migrate run
```

**Issue: "JWT validation failed"**
```rust
// Check: Is JWT secret correct?
// File: .env or config.yaml
// Look for: SUPABASE_JWT_SECRET
```

**Issue: "Compilation errors with async"**
```rust
// Add: #[async_trait] to trait definitions
// Import: use async_trait::async_trait;
```

### Logging
```bash
# Enable debug logging
RUST_LOG=debug cargo run

# Enable specific module logging
RUST_LOG=llm_supabase_rs::domain::providers=trace cargo run

# Pretty print structures
dbg!(&my_struct);

# Use tracing
tracing::debug!("Value: {:?}", value);
tracing::info!("Request started");
tracing::error!("Error occurred: {}", err);
```

---

## 📦 Dependencies Quick Reference

### Core Dependencies
```toml
axum = "0.7"              # Web framework
tokio = "1.0"             # Async runtime
serde = "1.0"             # Serialization
sqlx = "0.7"              # Database
reqwest = "0.11"          # HTTP client
tracing = "0.1"           # Logging
jsonwebtoken = "9.2"      # JWT
anyhow = "1.0"            # Error handling
```

### When to Use What

| Need | Use | Example |
|------|-----|---------|
| Web server | `axum` | Routes, handlers |
| Async runtime | `tokio` | `#[tokio::main]` |
| Serialization | `serde` | JSON parsing |
| Database | `sqlx` | Queries |
| HTTP requests | `reqwest` | Provider APIs |
| Logging | `tracing` | Debug output |
| JWT | `jsonwebtoken` | Auth validation |
| Errors | `anyhow` | Error context |

---

## 🎯 Priority Guidelines

### Task Priority
- **P0**: Core functionality, blocking
- **P1**: Important features, high value
- **P2**: Nice to have, quality of life
- **P3**: Future enhancements

### Implementation Order
1. **Foundation** (P0)
   - Project structure
   - Configuration
   - Error handling
   - Basic API

2. **Core Features** (P0-P1)
   - Authentication
   - Provider abstraction
   - At least one provider working
   - Request/response flow

3. **Extended Features** (P1-P2)
   - Multiple providers
   - Tool calling
   - Webhooks
   - Streaming

4. **Advanced Features** (P2-P3)
   - Local models
   - Knowledge base
   - Performance optimization
   - Agent support (future)

---

## 📞 Getting Help

### For AI Assistants (Claude Code, etc.)

1. **Load relevant context**
   - Functional spec section
   - Technical architecture section
   - Implementation guide steps

2. **Follow patterns**
   - Use existing code as examples
   - Match project structure
   - Follow error handling patterns

3. **Generate complete code**
   - Include all imports
   - Add error handling
   - Include basic tests

### For Developers

1. **Check documentation first**
2. **Look at existing implementations**
3. **Run tests to validate**
4. **Ask specific questions with context**

---

## ✅ Quality Checklist

Before considering a feature "done":

- [ ] Code compiles without warnings
- [ ] All tests pass
- [ ] Error handling implemented
- [ ] Logging added for key operations
- [ ] Code formatted (`cargo fmt`)
- [ ] Linter passes (`cargo clippy`)
- [ ] Documentation updated
- [ ] Manual testing completed
- [ ] Integration tests added (if applicable)

---

## 🚀 Deployment Quick Reference

### Environment Variables
```bash
# Required
SUPABASE_URL=...
SUPABASE_JWT_SECRET=...
GCP_PROJECT_ID=...
GOOGLE_APPLICATION_CREDENTIALS=...

# Optional
DATABASE_URL=...
REDIS_URL=...
HF_MODEL_PATH=...
```

### Build Commands
```bash
# Development
cargo build

# Production
cargo build --release

# Docker
docker build -t llm-proxy .

# Run
cargo run
./target/release/llm-supabase-rs
```

---

## 📚 Additional Resources

### External Documentation
- [Rust Book](https://doc.rust-lang.org/book/)
- [Tokio Tutorial](https://tokio.rs/tokio/tutorial)
- [Axum Docs](https://docs.rs/axum)
- [OpenAI API Reference](https://platform.openai.com/docs/api-reference)
- [Anthropic API Docs](https://docs.anthropic.com/)

### Internal Documentation
- Full specs in `docs/FUNCTIONAL_SPECIFICATION.md`
- Architecture in `docs/TECHNICAL_ARCHITECTURE.md`
- How-to guide in `docs/IMPLEMENTATION_GUIDE.md`
- Current status in `docs/progress.md`

---

**Last Updated:** October 2, 2025  
**For Questions:** Check documentation first, then create GitHub issue  
**Document Status:** Reference for AI assistants and developers
