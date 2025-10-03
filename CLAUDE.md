# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

This is a production-ready Rust web service that provides an OpenAI-compatible API for LLM interactions. It acts as a proxy that routes requests to Google Vertex AI (specifically Claude models) while providing Supabase authentication, comprehensive logging, and multi-provider embedding support. The service is built with a feature-based clean architecture using Axum and Tokio.

## Development Commands

### Essential Rust Commands
```bash
# Quick development check (fastest feedback)
cargo check

# Build the project
cargo build

# Run the application (requires .env setup)
cargo run

# Run tests
cargo test

# Run tests with output
cargo test -- --nocapture

# Run specific test
cargo test test_name

# Format and lint (before commits)
cargo fmt && cargo clippy

# Build for production
cargo build --release
```

### Development Setup
1. Copy environment configuration: `cp .env.example .env`
2. Edit `.env` with your Supabase and GCP credentials
3. Place GCP service account key at `./gcp-credentials.json`
4. Run: `cargo run`

## Architecture Overview

### Clean Architecture Layers
- **Features** (`src/features/`): Domain-specific business logic
  - `auth/`: JWT authentication with Supabase integration
- **Infrastructure** (`src/infrastructure/`): External service integrations
  - `vertex/`: Vertex AI client, Claude-specific logic, format conversion
  - `supabase.rs`: Supabase client integration
  - `logging.rs`: Structured logging setup
- **Shared** (`src/shared/`): Common utilities across layers
  - `error.rs`: Centralized error types with HTTP status mapping
  - `types.rs`: Shared type definitions
  - `utils.rs`: Common utility functions
- **Config** (`src/config/`): Environment-based configuration management

### Key Architectural Patterns
- **OpenAI API Compatibility**: All endpoints match OpenAI v1 specification
- **Format Conversion**: Automatic translation between OpenAI and Vertex AI formats (`src/infrastructure/vertex/converter.rs`)
- **Authentication Flow**: JWT validation → Supabase user lookup → request authorization
- **Error Handling**: Custom `AppError` type with context and HTTP status mapping
- **State Management**: Shared `AppState` containing clients and configuration

### Entry Points
- `src/main.rs`: Application bootstrap, logging initialization, config loading
- `src/app.rs`: Axum router setup, middleware configuration, handler routing
- Handler implementations are currently placeholder stubs awaiting implementation

## Configuration Management

### Required Environment Variables
```bash
# Server
HOST=0.0.0.0
PORT=8080
LOG_LEVEL=info

# Supabase (authentication)
SUPABASE_URL=https://your-project.supabase.co
SUPABASE_ANON_KEY=your_anon_key
SUPABASE_SERVICE_ROLE_KEY=your_service_role_key
SUPABASE_JWT_SECRET=your_jwt_secret

# Google Cloud Platform (Vertex AI)
GCP_PROJECT_ID=your-gcp-project-id
GCP_LOCATION=us-central1
GOOGLE_APPLICATION_CREDENTIALS=./gcp-credentials.json

# Model Configuration
DEFAULT_MODEL=claude-sonnet-4-20250514
```

### Configuration Structure
Configuration is managed through strongly-typed structs in `src/config/`:
- `AppConfig`: Main application configuration
- `SupabaseConfig`: Supabase connection details
- `VertexConfig`: Google Vertex AI settings
- `EmbeddingConfig`: Multi-provider embedding configuration

## API Endpoints

### Current Routes (as defined in `src/app.rs:81-113`)
```
GET  /health                  # Health check
POST /v1/chat/completions     # OpenAI-compatible chat completions
GET  /v1/models               # List available models
POST /v1/embeddings           # Generate embeddings
```

### Authentication Middleware
All API endpoints use optional authentication via `AuthMiddleware::optional_authenticate` which validates JWT tokens from the `Authorization: Bearer <token>` header.

## Dependencies and Key Libraries

### Core Web Framework
- **axum**: HTTP server framework with routing and middleware
- **tokio**: Async runtime for high-performance I/O
- **tower-http**: HTTP middleware (CORS, tracing)

### Authentication & Security
- **jsonwebtoken**: JWT token validation for Supabase users
- **gcp_auth**: Google Cloud Platform authentication

### Data & Serialization
- **serde**: JSON serialization/deserialization
- **sqlx**: PostgreSQL database integration (for future use)
- **reqwest**: HTTP client for external API calls

### Error Handling & Logging
- **anyhow** & **thiserror**: Structured error handling
- **tracing**: Structured logging with context

## Testing Strategy

### Test Organization
- Unit tests: Embedded in source files with `#[cfg(test)]`
- Integration tests: Would be placed in `tests/` directory
- Test configuration: Mock config in `src/app.rs:140-160`

### Running Tests
```bash
# All tests
cargo test

# Tests requiring GCP credentials are marked with #[ignore]
cargo test -- --ignored  # Run only ignored tests
```

## Development Patterns

### Error Handling
- Use `AppResult<T>` (alias for `Result<T, AppError>`) for all functions that can fail
- `AppError` provides context and automatic HTTP status mapping
- Chain errors with `.map_err()` for context propagation

### Async Patterns
- All handlers and clients are async
- Use `Arc<Client>` for shared state across handlers
- Database connections and HTTP clients should use connection pooling

### Module Organization
- Feature-based modules prevent circular dependencies
- Infrastructure modules handle external integrations
- Shared modules provide cross-cutting concerns

## Key Files for Development

When implementing new features, focus on:
- `src/app.rs:122-132`: Handler function stubs (primary implementation targets)
- `src/infrastructure/vertex/`: Vertex AI integration logic
- `src/features/auth/`: Authentication and authorization logic
- `src/shared/error.rs`: Error type definitions and HTTP mapping

## Specify Integration

This project uses Specify for project management with templates in `.specify/templates/`. The constitution is defined in `.specify/memory/constitution.md` for consistent development practices.
