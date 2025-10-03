# LLM Supabase Rust Service

A high-performance, OpenAI-compatible API service built in Rust that routes requests to Google Vertex AI while providing comprehensive authentication, logging, and multi-provider embedding support.

## 🚀 Features

### ✅ Implemented

- **OpenAI API Compatibility**: Full REST API compatibility with OpenAI's v1 specification
- **Vertex AI Integration**: Seamless routing to Google Vertex AI with Claude models as default
- **JWT Authentication**: Support for Supabase user tokens, anonymous keys, and service role tokens
- **Feature-Based Architecture**: Clean, modular codebase with separation of concerns
- **Format Conversion**: Automatic conversion between OpenAI and Vertex AI request/response formats
- **Comprehensive Error Handling**: Structured error types with proper HTTP status codes
- **Configuration Management**: Environment-based configuration with validation
- **Structured Logging**: Built-in tracing and logging infrastructure

### 🚧 In Development

- **Streaming Support**: Server-Sent Events (SSE) with response aggregation
- **Embedding Providers**: Multi-provider support (Vertex AI, OpenAI, HuggingFace/Candle)
- **Tool Calling**: Function calling capabilities with comprehensive logging
- **Request Logging**: User tracking and comprehensive request/response logging

## 📁 Project Structure

```
src/
├── main.rs                 # Application entry point
├── lib.rs                  # Library exports
├── app.rs                  # Main application setup
├── config/                 # Configuration management
│   ├── app.rs              # Application config
│   ├── embedding.rs        # Embedding provider config
│   └── vertex.rs           # Vertex AI config
├── features/               # Feature-based modules
│   └── auth/               # Authentication
│       ├── jwt.rs          # JWT handling
│       ├── middleware.rs   # Auth middleware
│       └── supabase.rs     # Supabase integration
├── infrastructure/         # External service integration
│   ├── vertex/             # Vertex AI client
│   │   ├── client.rs       # Main client
│   │   ├── claude.rs       # Claude-specific logic
│   │   └── converter.rs    # Format conversion
│   ├── supabase.rs         # Supabase client
│   └── logging.rs          # Logging infrastructure
└── shared/                 # Shared utilities
    ├── error.rs            # Error handling
    ├── types.rs            # Common types
    └── utils.rs            # Utility functions
```

## 🛠️ Technology Stack

- **Framework**: Axum with Tokio async runtime
- **Authentication**: JWT with jsonwebtoken crate
- **HTTP Client**: Reqwest with streaming support
- **Configuration**: Environment-based with dotenv
- **Logging**: Tracing with structured output
- **Database**: SQLx with PostgreSQL support
- **Cloud Integration**: Google Cloud Platform with service account auth

## ⚙️ Configuration

Copy `.env.example` to `.env` and configure:

```env
# Server Configuration
HOST=0.0.0.0
PORT=8080
LOG_LEVEL=info

# Supabase Configuration
SUPABASE_URL=https://your-project.supabase.co
SUPABASE_ANON_KEY=your_anon_key_here
SUPABASE_SERVICE_ROLE_KEY=your_service_role_key_here
SUPABASE_JWT_SECRET=your_jwt_secret_here

# Google Cloud Platform
GCP_PROJECT_ID=your-gcp-project-id
GCP_LOCATION=us-central1
GOOGLE_APPLICATION_CREDENTIALS=./gcp-credentials.json

# Model Configuration
DEFAULT_MODEL=claude-sonnet-4-20250514
```

## 🚀 Quick Start

1. **Prerequisites**:
   ```bash
   # Install Rust
   curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
   
   # Install required tools
   cargo install sqlx-cli
   ```

2. **Setup**:
   ```bash
   # Clone the repository
   git clone <repository-url>
   cd llm-supabase-rs
   
   # Copy GCP credentials
   cp /path/to/your/gcp-credentials.json ./gcp-credentials.json
   
   # Configure environment
   cp .env.example .env
   # Edit .env with your configuration
   ```

3. **Development**:
   ```bash
   # Check code
   cargo check
   
   # Run tests
   cargo test
   
   # Run in development mode
   cargo run
   
   # Build for production
   cargo build --release
   ```

## 📚 API Endpoints

### Health Check
```
GET /health
```

### OpenAI Compatible Endpoints
```
POST /v1/chat/completions    # Chat completions (streaming/non-streaming)
GET  /v1/models              # List available models
POST /v1/embeddings          # Generate embeddings
```

## 🔐 Authentication

The service supports multiple authentication methods:

1. **User Tokens**: JWT tokens for authenticated Supabase users
2. **Anonymous Keys**: Supabase anonymous access keys
3. **Service Role Keys**: Administrative access tokens

Authentication is handled via the `Authorization: Bearer <token>` header.

## 🏗️ Architecture Highlights

### Clean Architecture
- **Features**: Domain-specific functionality
- **Infrastructure**: External service integrations
- **Shared**: Common utilities and types

### Error Handling
- Comprehensive error types with context
- Proper HTTP status code mapping
- Structured error responses

### Async/Concurrency
- Tokio async runtime for maximum performance
- Connection pooling and request batching
- Non-blocking I/O throughout

### Type Safety
- Strong typing with serde for serialization
- OpenAI API compatible types
- Vertex AI format conversion

## 🧪 Testing

```bash
# Run all tests
cargo test

# Run with output
cargo test -- --nocapture

# Run specific test
cargo test test_name

# Test with logging
RUST_LOG=debug cargo test
```

## 📊 Monitoring & Logging

The service provides comprehensive observability:

- **Structured Logging**: JSON-formatted logs with tracing
- **Request Tracking**: Unique request IDs and timing
- **Error Reporting**: Detailed error context and stack traces
- **Performance Metrics**: Response times and throughput

## 🔄 Future Roadmap

1. **Complete Streaming Implementation**
2. **Add HuggingFace/Candle Local Inference**
3. **Implement Comprehensive Tool Calling**
4. **Add Rate Limiting and Caching**
5. **Docker Containerization**
6. **Kubernetes Deployment Manifests**
7. **Performance Benchmarking**

## 🤝 Contributing

1. Fork the repository
2. Create a feature branch
3. Make your changes
4. Add tests
5. Run `cargo fmt` and `cargo clippy`
6. Submit a pull request

## 📄 License

This project is licensed under the MIT License.

## 📞 Support

For issues and questions, please open a GitHub issue or contact the development team.

---

**Note**: This service is designed for high-performance production use with enterprise-grade authentication and logging capabilities.