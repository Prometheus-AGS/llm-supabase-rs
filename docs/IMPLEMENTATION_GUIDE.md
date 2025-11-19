# Implementation Guide

**Project:** Universal AI Server Proxy (llm-supabase-rs)
**Version:** 2.1
**Date:** October 15, 2025
**Status:** ✅ PRODUCTION READY

## Document Purpose

This guide provides practical instructions for **using and extending** the production-ready Universal AI Server Proxy. The system is fully implemented with comprehensive multi-provider support, advanced tool calling, and extensive observability.

---

## 🚀 **Quick Start (Production Ready System)**

### **Prerequisites**
- Rust 1.75+
- Docker (optional, for local development)
- Supabase account (for authentication)
- Google Cloud Platform account (for Vertex AI)

### **Setup Instructions (< 30 minutes)**

#### 1. **Clone and Setup**
```bash
# Clone the repository
git clone <repository-url>
cd llm-supabase-rs

# Install Rust dependencies
cargo build

# Copy environment template
cp .env.example .env
```

#### 2. **Configure Environment Variables**
Edit `.env` with your credentials:

```bash
# Server Configuration
HOST=0.0.0.0
PORT=8080
LOG_LEVEL=info

# Supabase Authentication
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

# Optional Provider API Keys (for direct access)
OPENAI_API_KEY=your_openai_key
ANTHROPIC_API_KEY=your_anthropic_key
```

#### 3. **Add GCP Service Account**
Place your GCP service account JSON file at `./gcp-credentials.json`

#### 4. **Run the Server**
```bash
# Development mode with hot reload
cargo run

# Production build
cargo build --release
./target/release/llm-supabase-rs
```

#### 5. **Verify Installation**
```bash
# Health check
curl http://localhost:8080/health

# Check available providers
curl http://localhost:8080/admin/providers

# Test chat completion
curl -X POST http://localhost:8080/v1/chat/completions \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer your-supabase-jwt" \
  -d '{"model":"claude-sonnet-4-20250514","messages":[{"role":"user","content":"Hello!"}]}'
```

---

## 📊 **Current Implementation Status**

### ✅ **Fully Implemented Features**
- **8 AI Providers**: Vertex AI, OpenAI, Anthropic, AWS Bedrock, Azure OpenAI, Groq, Mistral, Cohere
- **Advanced Tool Calling**: Client-adaptive system with format optimization
- **High-Performance Streaming**: SSE with chunk aggregation
- **Production Monitoring**: Prometheus metrics + Langfuse integration
- **Comprehensive Testing**: 25+ integration tests
- **Complete Documentation**: 15+ specialized guides

### ✅ **Available API Endpoints**
```
GET  /health                    # System health check
GET  /metrics                   # Prometheus metrics
POST /v1/chat/completions       # OpenAI-compatible chat
GET  /v1/models                 # List available models
POST /v1/embeddings             # Generate embeddings
GET  /admin/providers           # Provider status
GET  /admin/health              # Detailed health info
```

---

## 🛠 **Development Commands**

### **Essential Commands**
```bash
# Quick development check (fastest feedback)
cargo check

# Build and run
cargo build && cargo run

# Format and lint (before commits)
cargo fmt && cargo clippy

# Run all tests
cargo test

# Run with hot reload during development
cargo watch -x run
```

### **Testing Commands**
```bash
# Run all tests with output
cargo test -- --nocapture

# Run specific test categories
cargo test integration          # Integration tests
cargo test contract             # Contract tests
cargo test unit                # Unit tests

# Run provider-specific tests
cargo test test_vertex_e2e
cargo test test_anthropic_provider
cargo test test_streaming_tool

# Performance benchmarks
cargo test test_performance_benchmarks -- --nocapture
```

### **Production Commands**
```bash
# Build optimized release
cargo build --release

# Security audit
cargo audit

# Generate documentation
cargo doc --open

# Run with production config
RUST_LOG=info ./target/release/llm-supabase-rs
```

---

## 🔧 **Configuration Guide**

### **Environment Variables Reference**

#### **Core Server Settings**
```bash
HOST=0.0.0.0                    # Server bind address
PORT=8080                       # Server port
LOG_LEVEL=info                  # Logging level (trace,debug,info,warn,error)
RUST_LOG=llm_supabase_rs=debug  # Rust-specific logging
```

#### **Authentication (Supabase)**
```bash
SUPABASE_URL=https://your-project.supabase.co
SUPABASE_ANON_KEY=eyJ...        # For public access
SUPABASE_SERVICE_ROLE_KEY=eyJ... # For admin operations
SUPABASE_JWT_SECRET=your-secret  # For JWT validation
```

#### **Primary Provider (Vertex AI)**
```bash
GCP_PROJECT_ID=your-project-id
GCP_LOCATION=us-central1        # GCP region
GOOGLE_APPLICATION_CREDENTIALS=./gcp-credentials.json
```

#### **Optional Provider API Keys**
```bash
# OpenAI Direct API
OPENAI_API_KEY=sk-...
OPENAI_ORG_ID=org-...

# Anthropic Claude API
ANTHROPIC_API_KEY=sk-ant-...

# AWS Bedrock (uses AWS credentials)
AWS_ACCESS_KEY_ID=AKIA...
AWS_SECRET_ACCESS_KEY=...
AWS_REGION=us-east-1

# Azure OpenAI
AZURE_OPENAI_API_KEY=...
AZURE_OPENAI_ENDPOINT=https://...

# Other providers
GROQ_API_KEY=gsk_...
MISTRAL_API_KEY=...
COHERE_API_KEY=...
```

#### **Feature Flags**
```bash
# Enable/disable features
ENABLE_METRICS=true             # Prometheus metrics
ENABLE_LANGFUSE=true            # LLM observability
ENABLE_STREAMING=true           # SSE streaming
ENABLE_TOOL_CALLING=true        # Tool execution
ENABLE_PROVIDER_FALLBACK=true   # Automatic failover
```

### **Model Configuration**
```bash
DEFAULT_MODEL=claude-sonnet-4-20250514
DEFAULT_PROVIDER=vertex-ai
MAX_TOKENS_DEFAULT=4096
TEMPERATURE_DEFAULT=0.7
```

---

## 🏗 **Architecture Overview**

### **Project Structure**
```
src/
├── api/                    # HTTP handlers and routes
│   ├── handlers/          # Request handlers
│   ├── middleware/        # Authentication, CORS, etc.
│   └── routes.rs          # Route configuration
├── config/                # Configuration management
│   ├── app.rs            # Main app config
│   ├── providers.rs      # Provider configurations
│   └── mod.rs            # Config module
├── features/              # Business logic modules
│   ├── auth/             # Authentication system
│   ├── conversations/    # Conversation management
│   ├── diff_patch/       # Code modification tools
│   ├── provider_fallback/ # Provider failover
│   └── shell_execution/  # Secure command execution
├── infrastructure/        # External integrations
│   ├── anthropic/        # Anthropic Claude API
│   ├── aws_bedrock/      # AWS Bedrock integration
│   ├── azure_openai/     # Azure OpenAI service
│   ├── cohere/           # Cohere API
│   ├── groq/             # Groq API
│   ├── mistral/          # Mistral AI API
│   ├── openai/           # OpenAI Direct API
│   ├── vertex/           # Google Vertex AI
│   ├── common/           # Shared infrastructure
│   └── supabase/         # Supabase integration
├── models/               # Data structures
│   ├── request.rs        # Request types
│   ├── response.rs       # Response types
│   ├── common.rs         # Shared types
│   └── error.rs          # Error types
├── monitoring/           # Observability
│   ├── metrics.rs        # Prometheus metrics
│   └── langfuse_integration.rs # LLM observability
└── main.rs              # Application entry point
```

### **Key Design Patterns**
- **Provider Abstraction**: Unified interface for all AI providers
- **Client Detection**: Automatic format optimization
- **Tool Orchestration**: Dynamic tool execution and formatting
- **Streaming Aggregation**: Optimized real-time responses
- **Fallback Strategy**: Intelligent provider failover
- **Dependency Injection**: State-based configuration

---

## 🧪 **Testing Guide**

### **Test Categories**

#### **1. Unit Tests**
```bash
# Run unit tests
cargo test --lib

# Individual test
cargo test test_vertex_conversion -- --nocapture
```

#### **2. Integration Tests**
```bash
# All integration tests
cargo test integration

# Provider-specific
cargo test test_vertex_e2e
cargo test test_anthropic_provider_integration
cargo test test_openai_provider_integration
```

#### **3. Contract Tests**
```bash
# API contract validation
cargo test contract

# Specific contracts
cargo test contract_health
cargo test contract_chat
cargo test contract_streaming
```

#### **4. Performance Tests**
```bash
# Performance benchmarks
cargo test test_performance_benchmarks -- --nocapture

# Load testing
cargo test test_concurrent_requests -- --nocapture
```

### **Test Utilities**
The project includes comprehensive test utilities in `tests/utils/`:
- **MockCodexClient**: Simulates client interactions
- **TestFixtures**: Provides realistic test data
- **PerformanceAssertions**: Validates response times
- **ErrorAssertions**: Tests error handling

---

## 📊 **Monitoring & Observability**

### **Prometheus Metrics**
Available at `http://localhost:8080/metrics`:

```bash
# Request metrics
codex_cli_requests_total{model,provider,status}
codex_cli_request_duration_seconds{model,provider}

# Tool calling metrics
codex_cli_tool_calls_total{tool_name,status}
codex_cli_tool_execution_duration_seconds{tool_name}

# Streaming metrics
codex_cli_streaming_requests_total
codex_cli_streaming_chunks_total{model}

# Provider metrics
codex_cli_provider_requests_total{provider,model,status}
codex_cli_provider_response_time_seconds{provider}
codex_cli_provider_availability_ratio{provider}
```

### **Health Checks**
```bash
# Basic health
curl http://localhost:8080/health

# Detailed health with provider status
curl http://localhost:8080/admin/health

# Provider-specific status
curl http://localhost:8080/admin/providers
```

### **Logging**
Structured logging with multiple levels:
```bash
# Set logging level
export RUST_LOG=llm_supabase_rs=debug

# Component-specific logging
export RUST_LOG=llm_supabase_rs::infrastructure::vertex=trace
```

---

## 🔗 **API Usage Examples**

### **Basic Chat Completion**
```bash
curl -X POST http://localhost:8080/v1/chat/completions \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer your-jwt-token" \
  -d '{
    "model": "claude-sonnet-4-20250514",
    "messages": [
      {"role": "user", "content": "Hello, how are you?"}
    ],
    "max_tokens": 1000,
    "temperature": 0.7
  }'
```

### **Streaming Chat**
```bash
curl -X POST http://localhost:8080/v1/chat/completions \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer your-jwt-token" \
  -d '{
    "model": "claude-sonnet-4-20250514",
    "messages": [
      {"role": "user", "content": "Write a simple Rust function"}
    ],
    "stream": true
  }'
```

### **Tool Calling (Advanced)**
```bash
curl -X POST http://localhost:8080/v1/chat/completions \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer your-jwt-token" \
  -d '{
    "model": "claude-sonnet-4-20250514",
    "messages": [
      {"role": "user", "content": "Read the file main.rs and analyze it"}
    ],
    "tools": [
      {
        "type": "function",
        "function": {
          "name": "read_file",
          "description": "Read a file from the filesystem",
          "parameters": {
            "type": "object",
            "properties": {
              "file_path": {"type": "string"}
            }
          }
        }
      }
    ],
    "client_type": "codex_cli"
  }'
```

### **Provider Override**
```bash
curl -X POST http://localhost:8080/v1/chat/completions \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer your-jwt-token" \
  -d '{
    "model": "gpt-4",
    "provider_override": "openai",
    "messages": [
      {"role": "user", "content": "Hello from OpenAI!"}
    ]
  }'
```

---

## 🚀 **Deployment Guide**

### **Production Deployment**

#### **1. Build Release**
```bash
cargo build --release
```

#### **2. Environment Setup**
```bash
# Production environment file
cp .env.example .env.production

# Set production values
export LOG_LEVEL=warn
export RUST_LOG=llm_supabase_rs=info
```

#### **3. Security Considerations**
- Use proper JWT secrets
- Secure API keys in environment variables
- Enable HTTPS/TLS
- Configure proper CORS settings
- Set up rate limiting

#### **4. Monitoring Setup**
- Configure Prometheus scraping
- Set up Grafana dashboards
- Configure alerting rules
- Monitor key metrics

### **Docker Deployment**
```dockerfile
FROM rust:1.75 as builder
WORKDIR /app
COPY . .
RUN cargo build --release

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y ca-certificates
COPY --from=builder /app/target/release/llm-supabase-rs /usr/local/bin/
EXPOSE 8080
CMD ["llm-supabase-rs"]
```

---

## 🔧 **Extending the System**

### **Adding New Providers**

1. **Create Provider Module**
```rust
// src/infrastructure/newprovider/mod.rs
pub mod client;
pub mod auth;
pub mod converter;
pub mod types;
```

2. **Implement Provider Trait**
```rust
use crate::infrastructure::common::traits::AIProvider;

impl AIProvider for NewProvider {
    async fn chat_completion(&self, request: ChatRequest) -> Result<ChatResponse> {
        // Implementation
    }
}
```

3. **Add Configuration**
```rust
// src/config/providers.rs
pub struct NewProviderConfig {
    pub api_key: String,
    pub base_url: String,
}
```

4. **Register Provider**
```rust
// src/app.rs - Add to provider factory
```

### **Adding New Tools**
Tools are automatically discovered from the request. To add custom tool execution:

1. **Create Tool Handler**
```rust
// src/features/tools/custom_tool.rs
pub async fn execute_custom_tool(params: serde_json::Value) -> Result<String> {
    // Tool implementation
}
```

2. **Register Tool**
```rust
// Register in tool registry
```

### **Adding Metrics**
```rust
// src/monitoring/metrics.rs
let custom_metric = Counter::new("custom_metric_total", "Description")?;
registry.register(Box::new(custom_metric.clone()))?;
```

---

## 🐛 **Troubleshooting**

### **Common Issues**

#### **1. Authentication Errors**
```bash
# Verify JWT token
curl -H "Authorization: Bearer token" http://localhost:8080/health

# Check Supabase configuration
echo $SUPABASE_JWT_SECRET
```

#### **2. Provider Connection Issues**
```bash
# Check provider status
curl http://localhost:8080/admin/providers

# Test credentials
gcloud auth application-default login  # For GCP
```

#### **3. Performance Issues**
```bash
# Check metrics
curl http://localhost:8080/metrics | grep duration

# Monitor logs
tail -f logs/app.log
```

#### **4. Build Issues**
```bash
# Clean and rebuild
cargo clean && cargo build

# Check dependencies
cargo tree
```

### **Debug Mode**
```bash
# Enable debug logging
export RUST_LOG=llm_supabase_rs=debug

# Trace level for specific modules
export RUST_LOG=llm_supabase_rs::infrastructure::vertex=trace
```

---

## 📚 **Additional Resources**

### **Related Documentation**
- [API Reference](./API_REFERENCE.md) - Complete API documentation
- [Architecture Guide](./TECHNICAL_ARCHITECTURE.md) - System architecture
- [Provider Comparison](./PROVIDER_COMPARISON_GUIDE.md) - Provider feature matrix
- [Performance Guide](./PERFORMANCE_OPTIMIZATION.md) - Performance tuning

### **External Resources**
- [OpenAI API Documentation](https://platform.openai.com/docs/api-reference)
- [Anthropic Claude API](https://docs.anthropic.com/claude/reference)
- [Google Vertex AI](https://cloud.google.com/vertex-ai/docs)
- [Supabase Authentication](https://supabase.com/docs/guides/auth)

---

## 🏆 **System Status**

**Current Status**: ✅ **PRODUCTION READY**

The Universal AI Server Proxy is fully implemented and production-ready with:
- ✅ 8 AI providers integrated
- ✅ Advanced tool calling system
- ✅ Comprehensive monitoring
- ✅ Extensive testing (25+ tests)
- ✅ Complete documentation
- ✅ High-performance streaming
- ✅ Intelligent provider fallback

**Next Steps**: Focus on advanced features like MCP integration, local model support, and enterprise features.

---

**Last Updated**: October 15, 2025
**Version**: 2.1
**Maintainer**: Development Team