# Universal AI Server Proxy (llm-supabase-rs)

A high-performance, production-ready Rust web service that provides a unified OpenAI-compatible API interface to multiple AI providers. The service acts as an intelligent proxy with comprehensive authentication, tool orchestration, webhooks, and local model inference capabilities.

## 🎯 Overview

This service transforms the complexity of working with multiple AI providers into a single, familiar OpenAI-compatible REST API. It supports Vertex AI, AWS Bedrock, OpenAI, Anthropic, and local models while providing enterprise-grade features like Supabase authentication, MCP (Model Context Protocol) tool orchestration, comprehensive logging, and webhook integrations.

## ✨ Key Features

### 🚀 Core Capabilities
- **OpenAI API Compatibility**: Full REST API compatibility with OpenAI's v1 specification
- **Multi-Provider Support**: Seamless integration with Vertex AI, AWS Bedrock, OpenAI, Anthropic, Groq, OpenRouter, and Ollama
- **Local Model Inference**: GPU-accelerated inference using HuggingFace models with Candle
- **Streaming Support**: Server-Sent Events (SSE) with real-time response streaming
- **Tool Orchestration**: MCP client/server for dynamic tool execution and sandboxing

### 🔐 Enterprise Features
- **Supabase Authentication**: JWT-based auth with role-based access control
- **Webhook System**: Comprehensive lifecycle hooks for monitoring and integration
- **Request Logging**: Detailed tracking with user attribution and performance metrics
- **Observability**: Structured logging, metrics, and distributed tracing
- **Security**: TLS-only, secrets management, and sandboxed tool execution

### 🏗️ Architecture Highlights
- **Clean Architecture**: Feature-based modular design with clear separation of concerns
- **Async/Performance**: Built on Tokio with connection pooling and non-blocking I/O
- **Type Safety**: Strong typing with comprehensive error handling and HTTP status mapping
- **Configuration**: Environment-based configuration with validation and hot reloading

## 📚 Documentation

### 📖 Essential Reading
Our comprehensive documentation is organized for different roles and use cases:

#### For Developers
- **[docs/FUNCTIONAL_SPECIFICATION.md](docs/FUNCTIONAL_SPECIFICATION.md)** - Complete requirements and API specifications *(Start here)*
- **[docs/TECHNICAL_ARCHITECTURE.md](docs/TECHNICAL_ARCHITECTURE.md)** - System architecture and design patterns
- **[docs/IMPLEMENTATION_GUIDE.md](docs/IMPLEMENTATION_GUIDE.md)** - Step-by-step development guide
- **[docs/README.md](docs/README.md)** - Documentation index and quick start

#### Architecture & Design
- **[docs/ADR-001-architecture-overview.md](docs/ADR-001-architecture-overview.md)** - Architectural decisions and rationale
- **[docs/MULTI_PROVIDER_ARCHITECTURE.md](docs/MULTI_PROVIDER_ARCHITECTURE.md)** - Provider abstraction design
- **[docs/threading/](docs/threading/)** - Threading model and performance optimizations

#### Implementation Plans
- **[docs/IMPLEMENTATION_PLAN.md](docs/IMPLEMENTATION_PLAN.md)** - High-level implementation roadmap
- **[docs/STREAMING_TOOL_CALLING_IMPLEMENTATION_PLAN.md](docs/STREAMING_TOOL_CALLING_IMPLEMENTATION_PLAN.md)** - Tool calling implementation
- **[docs/CLIENT_DETECTION_IMPLEMENTATION.md](docs/CLIENT_DETECTION_IMPLEMENTATION.md)** - Client detection and compatibility

#### Specialized Features
- **[docs/streams/STREAMING_OVERVIEW.md](docs/streams/STREAMING_OVERVIEW.md)** - Streaming implementation details
- **[docs/tools/TOOL_RULES.md](docs/tools/TOOL_RULES.md)** - Tool execution and security rules
- **[docs/LOGGING.md](docs/LOGGING.md)** - Logging architecture and configuration
- **[docs/future/REALTIME_AI_VIDEO.md](docs/future/REALTIME_AI_VIDEO.md)** - Future capabilities roadmap

#### Status & Progress
- **[docs/progress.md](docs/progress.md)** - Current implementation status
- **[docs/IMPLEMENTATION_COMPLETE.md](docs/IMPLEMENTATION_COMPLETE.md)** - Completion tracking

## 🚀 Quick Start

### Prerequisites
```bash
# Install Rust (1.75+)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Install additional tools
cargo install sqlx-cli
```

### Setup
```bash
# Clone the repository
git clone <repository-url>
cd llm-supabase-rs

# Configure environment
cp .env.example .env
# Edit .env with your configuration (see Configuration section below)

# Set up GCP credentials (if using Vertex AI)
cp /path/to/your/gcp-credentials.json ./gcp-credentials.json
```

### Development
```bash
# Quick development check (fastest feedback)
cargo check

# Run the application
cargo run

# Run tests
cargo test

# Format and lint (before commits)
cargo fmt && cargo clippy

# Build for production
cargo build --release
```

## ⚙️ Configuration

The service uses environment-based configuration. Copy `.env.example` to `.env` and configure:

```env
# Server Configuration
HOST=0.0.0.0
PORT=8080
LOG_LEVEL=info

# Supabase Configuration (Authentication)
SUPABASE_URL=https://your-project.supabase.co
SUPABASE_ANON_KEY=your_anon_key_here
SUPABASE_SERVICE_ROLE_KEY=your_service_role_key_here
SUPABASE_JWT_SECRET=your_jwt_secret_here

# Google Cloud Platform (Vertex AI)
GCP_PROJECT_ID=your-gcp-project-id
GCP_LOCATION=us-central1
GOOGLE_APPLICATION_CREDENTIALS=./gcp-credentials.json

# Model Configuration
DEFAULT_MODEL=claude-sonnet-4-20250514

# Optional: Other AI Providers
OPENAI_API_KEY=sk-...
ANTHROPIC_API_KEY=sk-ant-...
AWS_ACCESS_KEY_ID=your-aws-key
AWS_SECRET_ACCESS_KEY=your-aws-secret
```

## 🛠️ Technology Stack

### Core Framework
- **[Axum](https://github.com/tokio-rs/axum)**: High-performance web framework with middleware support
- **[Tokio](https://tokio.rs/)**: Async runtime for non-blocking I/O and concurrency
- **[SQLx](https://github.com/launchbadge/sqlx)**: Type-safe SQL database integration

### AI & ML Integration
- **[Candle](https://github.com/huggingface/candle)**: HuggingFace-compatible ML framework for local inference
- **Google Cloud SDK**: Vertex AI integration with service account authentication
- **AWS SDK**: Bedrock integration for AWS-hosted models
- **HTTP Clients**: Direct integration with OpenAI, Anthropic, and other API providers

### Data & Security
- **[Serde](https://serde.rs/)**: JSON serialization with OpenAI API compatibility
- **[jsonwebtoken](https://github.com/Keats/jsonwebtoken)**: JWT validation for Supabase authentication
- **[Tracing](https://tracing.rs/)**: Structured logging with request correlation

## 📊 Project Structure

```
src/
├── main.rs                     # Application entry point and server bootstrap
├── lib.rs                      # Library exports and public API
├── app.rs                      # Axum router setup and middleware configuration
├── config/                     # Configuration management
│   ├── app.rs                  # Main application configuration
│   ├── embedding.rs            # Multi-provider embedding configuration
│   └── vertex.rs               # Vertex AI specific configuration
├── features/                   # Feature-based business logic modules
│   └── auth/                   # Authentication and authorization
│       ├── jwt.rs              # JWT token validation and parsing
│       ├── middleware.rs       # Authentication middleware
│       └── supabase.rs         # Supabase client integration
├── infrastructure/             # External service integrations
│   ├── vertex/                 # Google Vertex AI provider
│   │   ├── client.rs           # Main Vertex AI client
│   │   ├── claude.rs           # Claude model specific logic
│   │   └── converter.rs        # OpenAI ↔ Vertex format conversion
│   ├── supabase.rs             # Supabase database client
│   └── logging.rs              # Structured logging infrastructure
├── shared/                     # Cross-cutting concerns and utilities
│   ├── error.rs                # Centralized error types and HTTP mapping
│   ├── types.rs                # Common type definitions
│   └── utils.rs                # Utility functions and helpers
docs/                           # Comprehensive project documentation
├── README.md                   # Documentation index and navigation
├── FUNCTIONAL_SPECIFICATION.md # Complete requirements and API specs
├── TECHNICAL_ARCHITECTURE.md   # Architecture and design patterns
├── IMPLEMENTATION_GUIDE.md     # Step-by-step development guide
├── threading/                  # Threading and performance documentation
├── streams/                    # Streaming implementation details
├── tools/                      # Tool orchestration documentation
└── future/                     # Future capabilities and roadmap
```

## 🌐 API Endpoints

### Core OpenAI-Compatible Routes
```http
# Health Check
GET /health

# Chat Completions (streaming and non-streaming)
POST /v1/chat/completions
Authorization: Bearer <supabase-jwt>
Content-Type: application/json

# List Available Models
GET /v1/models
Authorization: Bearer <supabase-jwt>

# Generate Embeddings
POST /v1/embeddings
Authorization: Bearer <supabase-jwt>
```

### Extended Features
```json
{
  "model": "claude-sonnet-4.5",
  "messages": [
    {"role": "user", "content": "Hello, world!"}
  ],
  "stream": true,
  "provider_override": "vertex-ai",
  "webhooks": {
    "on_start": "https://your-app.com/webhook/start",
    "on_end": "https://your-app.com/webhook/end",
    "on_tool_call": "https://your-app.com/webhook/tool"
  },
  "mcp_config": {
    "allow_external_tools": true,
    "sandbox_timeout": 30
  }
}
```

### Admin Endpoints
```http
# System Health and Diagnostics
GET /admin/health
GET /admin/providers
GET /admin/metrics

# Webhook Management
POST /admin/webhooks/register
GET /admin/webhooks/list
DELETE /admin/webhooks/{id}
```

## 🔐 Authentication

The service supports multiple authentication methods through Supabase:

1. **User JWT Tokens**: For authenticated Supabase users
2. **Anonymous Keys**: For public access with rate limiting
3. **Service Role Keys**: For administrative and backend access

All requests include the `Authorization: Bearer <token>` header. The service automatically validates tokens and provides user context for logging and access control.

## 🧪 Testing

### Running Tests
```bash
# Run all tests
cargo test

# Run with detailed output
cargo test -- --nocapture

# Run specific test module
cargo test auth::tests

# Run tests with logging
RUST_LOG=debug cargo test

# Integration tests (requires GCP credentials)
cargo test --ignored  # Tests marked with #[ignore]
```

### Test Organization
- **Unit Tests**: Embedded in source files with `#[cfg(test)]`
- **Integration Tests**: Located in `tests/` directory
- **Mock Configuration**: Test utilities in individual modules

## 📈 Performance & Monitoring

### Performance Targets
- **Chat Completions**: < 2s (P50), < 5s (P95) non-streaming
- **Streaming First Token**: < 500ms (P50), < 1s (P95)
- **Embeddings**: < 200ms (P50), < 500ms (P95)
- **Concurrent Requests**: 1000+ simultaneous connections

### Observability Features
- **Structured Logging**: JSON-formatted logs with request correlation
- **Request Tracking**: Unique request IDs and comprehensive timing
- **Error Reporting**: Detailed error context with stack traces
- **Metrics Collection**: Performance metrics and health indicators

## 🔄 Development Roadmap

### Phase 1: Foundation ✅
- [x] Project structure and configuration
- [x] OpenAI API compatibility layer
- [x] Vertex AI provider integration
- [x] Supabase authentication
- [x] Basic streaming support

### Phase 2: Multi-Provider 🔄
- [ ] AWS Bedrock integration
- [ ] Direct OpenAI provider
- [ ] Anthropic provider integration
- [ ] Provider selection and fallback logic
- [ ] Performance optimization

### Phase 3: Tool Orchestration ⏳
- [ ] MCP (Model Context Protocol) client
- [ ] Tool registry and management
- [ ] Sandboxed tool execution
- [ ] MCP server capabilities

### Phase 4: Webhook System ⏳
- [ ] Event dispatcher architecture
- [ ] Webhook delivery system
- [ ] Supabase integration hooks
- [ ] Admin management interface

### Phase 5: Local Models ⏳
- [ ] HuggingFace model integration
- [ ] Candle inference engine
- [ ] GPU acceleration support
- [ ] Model caching and optimization

### Phase 6: Production Hardening 💡
- [ ] Rate limiting and quotas
- [ ] Comprehensive caching
- [ ] Docker containerization
- [ ] Kubernetes deployment
- [ ] Performance benchmarking

## 🤝 Contributing

### Development Workflow
1. **Fork and Clone**: Fork the repository and clone locally
2. **Read Documentation**: Review [docs/README.md](docs/README.md) for comprehensive guidance
3. **Setup Environment**: Follow the Quick Start guide
4. **Make Changes**: Implement features following the architecture patterns
5. **Test Thoroughly**: Run tests and add new test coverage
6. **Code Quality**: Run `cargo fmt` and `cargo clippy`
7. **Documentation**: Update relevant documentation
8. **Submit PR**: Create pull request with clear description

### Coding Standards
- Follow Rust naming conventions and best practices
- Use structured error handling with `AppError`
- Include comprehensive logging with request context
- Write unit tests for all business logic
- Document public APIs and complex functions

## 📄 License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## 🆘 Support & Community

### Getting Help
1. **Check Documentation**: Start with [docs/README.md](docs/README.md)
2. **Review Issues**: Search existing GitHub issues
3. **Create Issue**: Use issue templates for bug reports and features
4. **Join Discussions**: Participate in GitHub Discussions

### Reporting Issues
When reporting issues, please include:
- Clear description of the problem
- Steps to reproduce
- Expected vs actual behavior
- Environment details (OS, Rust version, etc.)
- Relevant log output

---

**Production Ready**: This service is designed for enterprise-grade deployment with comprehensive security, monitoring, and scalability features. See the [documentation](docs/) for deployment guides and production considerations.

**Active Development**: This project is under active development. Check [docs/progress.md](docs/progress.md) for current status and upcoming features.