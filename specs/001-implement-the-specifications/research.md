# Research: Vertex AI and Claude 4 Sonnet Integration

**Date**: October 2, 2025
**Feature**: Vertex AI and Claude 4 Sonnet Priority Implementation

## Key Research Areas

### 1. Vertex AI Anthropic Models Integration

**Decision**: Use Google Cloud Vertex AI's Anthropic Claude 4 Sonnet model via REST API
**Model ID**: `claude-4-sonnet-20250514`

**Rationale**:
- Vertex AI provides enterprise-grade scaling and reliability for Claude models
- Integrated with Google Cloud ecosystem for auth, monitoring, and billing
- Supports streaming responses required for real-time chat
- Model identifier confirmed through clarification session

**Alternatives Considered**:
- Direct Anthropic API: Rejected due to requirement for Vertex AI integration
- Claude 3.5 Sonnet: Rejected as clarifications specified Claude 4 Sonnet specifically

**Implementation Notes**:
- Endpoint: `https://{location}-aiplatform.googleapis.com/v1/projects/{project}/locations/{location}/publishers/anthropic/models/claude-4-sonnet-20250514:streamRawPredict`
- Authentication: Google Cloud Service Account with Vertex AI permissions
- Streaming: Use `streamRawPredict` for SSE streaming responses

### 2. OpenAI API Compatibility

**Decision**: Implement full OpenAI v1 Chat Completions API compatibility
**Format**: Match OpenAI's exact request/response structure and SSE streaming format

**Rationale**:
- Enables drop-in replacement for existing OpenAI integrations
- Large ecosystem of SDKs and tools expect OpenAI format
- Clarifications specified "OpenAI's exact format" for streaming

**Alternatives Considered**:
- Custom API format: Rejected due to compatibility requirements
- Subset implementation: Rejected for comprehensive compatibility

**Implementation Notes**:
- Request conversion: OpenAI ChatCompletionRequest → Vertex AI native format
- Response conversion: Vertex AI response → OpenAI ChatCompletionResponse
- Streaming: SSE with `data:` prefix following OpenAI's exact format
- Error mapping: Vertex AI errors → OpenAI-compatible error responses

### 3. Supabase JWT Authentication

**Decision**: Use Supabase JWT token validation with `jsonwebtoken` crate
**Flow**: Extract Bearer token → Validate JWT signature → Extract user context

**Rationale**:
- Supabase provides enterprise authentication with user management
- JWT tokens contain user identity and metadata needed for authorization
- Existing project already configured for Supabase integration

**Alternatives Considered**:
- Custom authentication: Rejected due to Supabase requirement
- API keys: Rejected in favor of user-based JWT authentication

**Implementation Notes**:
- JWT secret from `SUPABASE_JWT_SECRET` environment variable
- Extract user ID from `sub` claim for request logging
- Optional authentication for flexibility in development/testing

### 4. Axum Web Framework Architecture

**Decision**: Use Axum with Tower middleware stack for HTTP service
**Architecture**: Layered middleware → routing → handlers → business logic

**Rationale**:
- Axum provides excellent async performance with Tokio runtime
- Tower middleware enables composable authentication, logging, CORS
- Type-safe request/response handling with extractors
- Already specified in project dependencies

**Alternatives Considered**:
- Actix-web: Rejected as Axum already chosen
- Warp: Rejected for Axum's better ecosystem integration

**Implementation Notes**:
- Middleware stack: CORS → Tracing → Auth → Rate Limiting → Handlers
- State management: `Arc<AppState>` with shared clients and configuration
- Error handling: Custom `AppError` type with HTTP status mapping

### 5. Concurrent Request Handling

**Decision**: Support 1000 concurrent requests using Tokio async runtime
**Strategy**: Connection pooling + async request processing + backpressure

**Rationale**:
- Clarifications specified 1000 concurrent requests as target
- Tokio runtime provides excellent concurrency for I/O-bound workloads
- Async nature allows high concurrency without blocking

**Alternatives Considered**:
- Thread-per-request: Rejected for scalability limitations
- Lower concurrency: Rejected based on specified requirements

**Implementation Notes**:
- HTTP client connection pooling with reqwest
- Async handlers throughout the request pipeline
- Rate limiting per user to prevent abuse
- Graceful degradation when limits approached

### 6. Error Handling and Fallback

**Decision**: Implement provider fallback with configurable alternative providers
**Strategy**: Primary Vertex AI → Fallback to configured alternative → Error response

**Rationale**:
- Clarifications specified fallback to alternative providers when Vertex AI unavailable
- Improves reliability and availability of the service
- Allows gradual migration between providers if needed

**Alternatives Considered**:
- Immediate error on failure: Rejected due to fallback requirement
- Retry-only strategy: Rejected for fallback requirement

**Implementation Notes**:
- Provider trait abstraction for multiple implementations
- Configuration-driven provider selection
- Automatic failover with circuit breaker pattern
- 503 Service Unavailable when all providers fail

### 7. Performance and Observability

**Decision**: Comprehensive observability with structured logging and metrics
**Target**: 95% of requests under 5 seconds response time

**Rationale**:
- Performance target specified in clarifications
- Enterprise-grade service requires comprehensive monitoring
- Debugging and optimization need detailed telemetry

**Alternatives Considered**:
- Basic logging: Rejected for enterprise requirements
- No performance monitoring: Rejected due to SLA requirements

**Implementation Notes**:
- Structured logging with `tracing` crate
- Request/response timing and token usage tracking
- Prometheus metrics for monitoring dashboards
- Distributed tracing for request flow analysis

## Technical Dependencies Analysis

### Core Dependencies (from Cargo.toml)
- **axum 0.8.6**: Web framework - latest stable version ✓
- **tokio 1.0**: Async runtime - mature and well-supported ✓
- **reqwest 0.12.23**: HTTP client - supports connection pooling and streaming ✓
- **jsonwebtoken 10.0.0**: JWT handling - latest version with security updates ✓
- **gcp_auth 0.12.3**: Google Cloud authentication - supports service accounts ✓
- **serde/serde_json**: JSON serialization - standard choice for API services ✓
- **tracing/tracing-subscriber**: Structured logging - industry standard ✓

### Missing Dependencies (to be added)
- **anyhow 1.0**: Error handling - already in Cargo.toml ✓
- **thiserror 2.0**: Custom error types - already in Cargo.toml ✓
- **uuid 1.6**: Request ID generation - already in Cargo.toml ✓
- **chrono 0.4**: Timestamp handling - already in Cargo.toml ✓

## Integration Patterns

### 1. Request Flow Pattern
```
Client Request → Auth Middleware → Rate Limiting → Handler → Provider → Response Conversion → Client
```

### 2. Provider Abstraction Pattern
```rust
trait LLMProvider {
    async fn chat_completion(&self, request: OpenAIRequest) -> Result<OpenAIResponse>;
    async fn chat_completion_stream(&self, request: OpenAIRequest) -> Result<Stream<SSEEvent>>;
}
```

### 3. Error Handling Pattern
```rust
// Centralized error type with HTTP status mapping
enum AppError {
    AuthenticationFailed,
    ProviderUnavailable,
    InvalidRequest(String),
    InternalError(String),
}
```

### 4. Configuration Pattern
```rust
// Environment-based configuration with validation
struct AppConfig {
    vertex_ai: VertexAIConfig,
    supabase: SupabaseConfig,
    server: ServerConfig,
}
```

## Security Considerations

### 1. Authentication Security
- JWT signature validation with proper algorithm checking
- Token expiration validation
- User context extraction for authorization

### 2. Request Validation
- Input sanitization for chat messages
- Parameter validation according to OpenAI API spec
- Rate limiting per authenticated user

### 3. Provider Security
- Service account key rotation capability
- Secure credential storage (environment variables)
- Request/response logging with PII filtering

## Performance Optimizations

### 1. Connection Management
- HTTP client connection pooling
- Keep-alive connections to Vertex AI
- Connection timeout configuration

### 2. Caching Strategy
- Model metadata caching
- Configuration caching with refresh capability
- Response caching for identical requests (optional)

### 3. Streaming Optimization
- Efficient SSE streaming implementation
- Backpressure handling for slow clients
- Memory-efficient large response handling

## Deployment Considerations

### 1. Container Deployment
- Multi-stage Docker build for optimized image size
- Health check endpoints for load balancer integration
- Graceful shutdown handling

### 2. Monitoring and Alerting
- Prometheus metrics exposure
- Health check endpoints
- Log aggregation compatibility

### 3. Configuration Management
- Environment-based configuration
- Configuration validation at startup
- Hot-reload capability for non-critical settings

---

**Status**: Research complete - all clarifications resolved, technical decisions documented
**Next Phase**: Design & Contracts (Phase 1)