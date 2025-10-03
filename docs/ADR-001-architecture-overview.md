# ADR-001: Architecture Overview for OpenAI-Compatible LLM Service

## Status
Accepted

## Context
We need to build a Rust-based web service that implements the OpenAI API specification while routing requests to Google Vertex AI (specifically Claude models) and supporting multiple embedding providers. The service must handle authentication, logging, streaming responses, and maintain high performance.

## Decision
We will implement a feature-based clean architecture using Rust Axum with the following key decisions:

### 1. Web Framework: Axum
- **Choice**: Axum with tokio async runtime
- **Rationale**: Axum provides excellent performance, type-safe routing, middleware support, and excellent streaming capabilities with Server-Sent Events (SSE)
- **Implementation**: Use `axum::response::sse::Sse` for streaming responses

### 2. Authentication Strategy
- **Choice**: JWT middleware with Supabase integration
- **Rationale**: Support for Supabase user tokens, anon keys, and service role keys
- **Implementation**: Custom middleware extracting and validating JWT tokens, with user lookup in Supabase for user tokens

### 3. Vertex AI Integration
- **Choice**: Google Cloud SDK with Application Default Credentials
- **Rationale**: Service account authentication using credentials file at `/Users/gqadonis/.gcp/credentials.json`
- **Implementation**: Convert Vertex AI responses to OpenAI format, handle both streaming and non-streaming

### 4. Streaming Strategy
- **Choice**: Server-Sent Events (SSE) with chunk aggregation
- **Rationale**: OpenAI-compatible streaming format with full response logging
- **Implementation**: Accumulate chunks during streaming, log complete response when finished

### 5. Embedding Providers
- **Choice**: Multi-provider architecture with configuration-driven selection
- **Providers**: 
  - Vertex AI embedding models (cloud)
  - OpenAI embedding models (cloud)
  - HuggingFace models via Candle (local inference)
- **Implementation**: Trait-based provider system with runtime selection

### 6. Project Structure
```
src/
├── main.rs                 # Application entry point
├── lib.rs                  # Library exports
├── config/                 # Configuration management
│   ├── mod.rs
│   ├── app.rs              # Application config
│   ├── embedding.rs        # Embedding provider config
│   └── vertex.rs           # Vertex AI config
├── features/               # Feature-based modules
│   ├── auth/               # Authentication
│   │   ├── mod.rs
│   │   ├── jwt.rs          # JWT handling
│   │   ├── middleware.rs   # Auth middleware
│   │   └── supabase.rs     # Supabase integration
│   ├── chat/               # Chat completions
│   │   ├── mod.rs
│   │   ├── handler.rs      # HTTP handlers
│   │   ├── service.rs      # Business logic
│   │   └── types.rs        # Request/response types
│   ├── embeddings/         # Embedding functionality
│   │   ├── mod.rs
│   │   ├── handler.rs
│   │   ├── service.rs
│   │   ├── providers/      # Provider implementations
│   │   │   ├── mod.rs
│   │   │   ├── vertex.rs
│   │   │   ├── openai.rs
│   │   │   └── candle.rs
│   │   └── types.rs
│   ├── models/             # Model enumeration
│   │   ├── mod.rs
│   │   ├── handler.rs
│   │   └── service.rs
│   └── streaming/          # Streaming support
│       ├── mod.rs
│       ├── aggregator.rs   # Chunk aggregation
│       └── sse.rs          # SSE utilities
├── infrastructure/         # External service integration
│   ├── mod.rs
│   ├── vertex/             # Vertex AI client
│   │   ├── mod.rs
│   │   ├── client.rs
│   │   ├── claude.rs       # Claude-specific integration
│   │   └── converter.rs    # OpenAI format conversion
│   ├── supabase/           # Supabase client
│   │   ├── mod.rs
│   │   ├── client.rs
│   │   └── auth.rs
│   └── logging/            # Structured logging
│       ├── mod.rs
│       ├── logger.rs
│       └── metrics.rs
├── shared/                 # Shared utilities
│   ├── mod.rs
│   ├── error.rs            # Error handling
│   ├── types.rs            # Common types
│   └── utils.rs            # Utilities
└── app.rs                  # Application setup
```

## Consequences

### Positive
- Clean separation of concerns with feature-based architecture
- High performance with Rust and async/await
- OpenAI API compatibility
- Multi-provider embedding support
- Comprehensive logging and monitoring
- Streaming support with aggregation

### Negative
- Complex initial setup
- Multiple external dependencies
- Requires careful error handling across async boundaries
- Memory management for streaming responses

## Implementation Notes

### Dependencies
Key Rust crates:
- `axum` - Web framework
- `tokio` - Async runtime
- `serde` - Serialization
- `reqwest` - HTTP client
- `jsonwebtoken` - JWT handling
- `tracing` - Structured logging
- `anyhow` - Error handling
- `gcp_auth` - Google Cloud authentication
- `candle-core`, `candle-nn`, `candle-transformers` - Local ML inference
- `uuid` - ID generation

### Configuration
Environment-based configuration with support for:
- Vertex AI project and region
- Supabase URL and keys
- Embedding provider selection
- Model download directories
- Logging levels

### Security Considerations
- JWT validation with proper key management
- Service account credential security
- Input validation and sanitization
- Rate limiting (future implementation)