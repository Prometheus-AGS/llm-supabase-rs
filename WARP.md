# WARP.md

This file provides guidance to WARP (warp.dev) when working with code in this repository.

## Project Overview

This is a production-ready Rust project that implements an OpenAI-compatible API service. It routes requests to Google Vertex AI (specifically Claude models) while providing comprehensive authentication via Supabase, structured logging, and multi-provider embedding support. The service is built with a feature-based clean architecture for scalability and maintainability.

## Development Commands

### Core Rust Commands
```bash
# Build the project
cargo build

# Run the application
cargo run

# Build with optimizations (release mode)
cargo build --release

# Run in release mode
cargo run --release

# Check code without building
cargo check

# Run tests
cargo test

# Run tests with output
cargo test -- --nocapture

# Run a specific test
cargo test test_name

# Format code
cargo fmt

# Lint code
cargo clippy

# Update dependencies
cargo update

# Generate documentation
cargo doc --open
```

### Development Workflow
```bash
# Quick development check (fast feedback loop)
cargo check && cargo clippy

# Full development cycle
cargo fmt && cargo clippy && cargo test && cargo build
```

## Project Architecture

### Current Structure
- `src/main.rs`: Application entry point with logging and configuration setup
- `src/lib.rs`: Library exports and module organization
- `src/app.rs`: Main Axum application with routing and middleware
- `src/config/`: Environment-based configuration management
- `src/features/auth/`: JWT authentication with Supabase integration
- `src/infrastructure/vertex/`: Vertex AI client and format conversion
- `src/shared/`: Common types, errors, and utilities
- `Cargo.toml`: Production dependencies including Axum, Tokio, and cloud SDKs

### Expected Architecture (Based on Project Name)
This project will likely evolve to include:

1. **LLM Integration Layer**: Modules for interfacing with language models (OpenAI, Anthropic, local models)
2. **Supabase Client**: Database operations, authentication, real-time subscriptions
3. **API Layer**: REST/GraphQL endpoints or CLI interface
4. **Configuration Management**: Environment variables, API keys, database connections
5. **Error Handling**: Robust error types for LLM and database operations

### Recommended Project Structure
```
src/
├── main.rs              # Application entry point
├── lib.rs               # Library root (if building a library)
├── config/              # Configuration management
├── llm/                 # LLM integration modules
│   ├── providers/       # Different LLM providers (OpenAI, etc.)
│   └── types.rs         # LLM-related types
├── supabase/            # Supabase integration
│   ├── client.rs        # Supabase client setup
│   ├── auth.rs          # Authentication handling
│   └── database.rs      # Database operations
├── api/                 # API layer (if applicable)
└── error.rs             # Error types and handling
```

## Development Environment Setup

### Prerequisites
- Rust toolchain (installed via rustup)
- Supabase CLI (for local development): `npm install -g @supabase/cli`

### Environment Variables (Expected)
Create a `.env` file in the project root:
```env
SUPABASE_URL=your_supabase_project_url
SUPABASE_ANON_KEY=your_supabase_anon_key
SUPABASE_SERVICE_ROLE_KEY=your_service_role_key
OPENAI_API_KEY=your_openai_api_key  # or other LLM provider keys
```

### Recommended Dependencies
Common crates likely needed for this project:
- `tokio` - Async runtime
- `serde` - Serialization/deserialization
- `reqwest` - HTTP client
- `postgrest` - Supabase PostgREST client
- `anyhow` or `thiserror` - Error handling
- `clap` - CLI argument parsing (if building CLI)
- `dotenv` - Environment variable management

## Testing Strategy

### Test Organization
```bash
# Unit tests (in src/ files)
cargo test unit

# Integration tests (in tests/ directory)
cargo test integration

# Run tests with specific feature flags
cargo test --features "integration-tests"
```

### Test Database
For Supabase integration testing, consider:
- Use Supabase local development environment
- Separate test database/project
- Mock Supabase responses for unit tests

## Key Development Practices

### Async/Await Patterns
- Use `tokio` for async runtime
- Implement proper error handling in async contexts
- Consider connection pooling for database operations

### Configuration Management
- Use strongly-typed configuration structs
- Validate configuration on startup
- Support both environment variables and config files

### Error Handling
- Create custom error types for domain-specific errors
- Use `Result<T, E>` consistently
- Implement proper error context with libraries like `anyhow`

### Security Considerations
- Never commit API keys or secrets
- Use service role keys only on the server side
- Implement proper authentication and authorization
- Validate and sanitize all external inputs (especially for LLM prompts)

## Debugging and Logging

### Logging Setup
Consider using `tracing` or `log` crates:
```bash
# Run with debug logging
RUST_LOG=debug cargo run

# Run with trace logging for specific modules
RUST_LOG=llm_supabase_rs::llm=trace cargo run
```

### Common Issues
- Database connection timeouts
- LLM API rate limits and token limits
- JSON serialization/deserialization errors
- Async runtime issues