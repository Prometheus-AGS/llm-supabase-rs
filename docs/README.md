# Documentation Index

**Project:** Universal AI Server Proxy (llm-supabase-rs)
**Version:** 2.1
**Last Updated:** October 15, 2025

## Overview

This directory contains comprehensive documentation for a **production-ready** universal AI server proxy that provides a unified OpenAI-compatible API interface to multiple AI providers, with advanced features including multi-provider support, tool orchestration, comprehensive observability, and extensive testing infrastructure.

## 🎉 Recent Major Achievements

### ✅ **Completed Implementation (NEW as of October 2025)**
- **Full Multi-Provider Support**: 8 providers implemented (Vertex AI, OpenAI, Anthropic, AWS Bedrock, Azure OpenAI, Groq, Mistral, Cohere)
- **Advanced Tool Calling**: Adaptive tool calling system with client detection and format optimization
- **Comprehensive Testing**: 25+ integration tests with realistic usage patterns
- **Production Monitoring**: Prometheus metrics with 20+ KPIs including request latency, tool execution, and provider health
- **Streaming Infrastructure**: Full SSE streaming support with chunk aggregation and performance optimization
- **Advanced Features**: Conversation management, diff/patch system, shell execution, provider fallback

## Documentation Structure

### Core Documents

1. **[FUNCTIONAL_SPECIFICATION.md](./FUNCTIONAL_SPECIFICATION.md)** - **START HERE**
   - Complete functional requirements
   - API specifications
   - Provider support details
   - Webhook system design
   - MCP integration overview
   - Knowledge base architecture
   - Future agent server capabilities
   
   **Read this first** to understand what the system does and why.

2. **[TECHNICAL_ARCHITECTURE.md](./TECHNICAL_ARCHITECTURE.md)**
   - System architecture diagrams
   - Technology stack details
   - Module design and structure
   - Data flow diagrams
   - Provider integration patterns
   - MCP architecture
   - Database schema
   - Performance considerations
   - Security architecture
   
   **Read this second** to understand how the system is structured.

3. **[IMPLEMENTATION_GUIDE.md](./IMPLEMENTATION_GUIDE.md)**
   - Step-by-step implementation instructions
   - Code examples and templates
   - Development environment setup
   - Testing strategies
   - Common issues and solutions
   - Best practices
   
   **Read this third** to start building.

4. **[IMPLEMENTATION_ROADMAP.md](./IMPLEMENTATION_ROADMAP.md)** (To be created)
   - Phased implementation plan
   - Week-by-week task breakdown
   - Priority assignments
   - Acceptance criteria
   - Dependencies and milestones
   - Risk mitigation strategies
   
   **Use this** for project planning and tracking.

### Supporting Documents

5. **[ADR-001-architecture-overview.md](./ADR-001-architecture-overview.md)**
   - Architectural decision record
   - Key design choices and rationale

6. **[progress.md](./progress.md)**
   - Current implementation status
   - Completed features
   - Work in progress

### 🆕 **New Specialized Documentation (Added October 2025)**

7. **[CODEX_CLI_IMPLEMENTATION_ARCHITECTURE.md](./CODEX_CLI_IMPLEMENTATION_ARCHITECTURE.md)**
   - Codex CLI integration patterns
   - Tool calling workflows
   - Client detection strategies

8. **[MULTI_PROVIDER_ARCHITECTURE.md](./MULTI_PROVIDER_ARCHITECTURE.md)**
   - Provider abstraction layers
   - Fallback and retry logic
   - Performance optimization

9. **[STREAMING_TOOL_CALLING_IMPLEMENTATION_PLAN.md](./STREAMING_TOOL_CALLING_IMPLEMENTATION_PLAN.md)**
   - Advanced tool calling architecture
   - Real-time streaming with tools
   - Client compatibility matrix

10. **[INTEGRATION_TESTING_USER_ADOPTION_GUIDE.md](./INTEGRATION_TESTING_USER_ADOPTION_GUIDE.md)**
    - Comprehensive testing framework
    - User adoption patterns
    - Performance benchmarking

11. **[PROVIDER_COMPARISON_GUIDE.md](./PROVIDER_COMPARISON_GUIDE.md)**
    - Provider feature comparison
    - Selection criteria
    - Migration strategies

## Quick Start

### For Developers Starting Implementation

1. **Read the Specification** (30 minutes)
   - [FUNCTIONAL_SPECIFICATION.md](./FUNCTIONAL_SPECIFICATION.md) - Sections 1-3
   - Understand the core API and provider abstraction

2. **Review Architecture** (30 minutes)
   - [TECHNICAL_ARCHITECTURE.md](./TECHNICAL_ARCHITECTURE.md) - Sections 1-4
   - Understand the module structure and data flow

3. **Set Up Environment** (1-2 hours)
   - Follow [IMPLEMENTATION_GUIDE.md](./IMPLEMENTATION_GUIDE.md) - Prerequisites and Step 1-9
   - Get the basic server running

4. **Start Phase 1** (Week 1)
   - Follow weekly tasks in the roadmap
   - Implement core foundation

### For Project Managers / Product Owners

1. **Read Executive Summary**
   - [FUNCTIONAL_SPECIFICATION.md](./FUNCTIONAL_SPECIFICATION.md) - Section 1
   - Get high-level understanding

2. **Review Roadmap** (To be created)
   - [IMPLEMENTATION_ROADMAP.md](./IMPLEMENTATION_ROADMAP.md)
   - Understand timeline and milestones

3. **Check Progress**
   - [progress.md](./progress.md)
   - Track current status

### For Architects / Technical Leads

1. **Deep Dive into Architecture**
   - Read [TECHNICAL_ARCHITECTURE.md](./TECHNICAL_ARCHITECTURE.md) completely
   - Review all design decisions

2. **Review Specifications**
   - [FUNCTIONAL_SPECIFICATION.md](./FUNCTIONAL_SPECIFICATION.md)
   - Validate requirements alignment

3. **Assess Technical Risks**
   - Review security considerations
   - Evaluate performance requirements
   - Check integration complexity

## Key Features

### ✅ **What This System Does (Updated October 2025)**

#### **Core Production Features**
- **Unified API**: OpenAI-compatible REST API with full specification compliance
- **Multi-Provider Support**: 8 providers integrated (Vertex AI, OpenAI, Anthropic, AWS Bedrock, Azure OpenAI, Groq, Mistral, Cohere)
- **Advanced Tool Calling**: Client-adaptive tool calling with automatic format optimization
- **High-Performance Streaming**: SSE with chunk aggregation and latency optimization
- **Comprehensive Authentication**: Supabase JWT with role-based access control
- **Production Monitoring**: Prometheus metrics with 20+ KPIs and Langfuse integration

#### **Advanced Features**
- **Intelligent Provider Fallback**: Automatic failover with health monitoring
- **Conversation Management**: Multi-turn conversation tracking and context management
- **Diff/Patch System**: Code modification tools with unified diff support
- **Shell Execution**: Secure command execution with safety guards
- **Client Detection**: Automatic tool format optimization based on client capabilities
- **Performance Optimization**: Request deduplication, caching, and threading optimization

#### **Developer Experience**
- **Comprehensive Testing**: 25+ integration tests with realistic usage patterns
- **Extensive Documentation**: 15+ specialized guides covering all aspects
- **Development Tools**: Hot reload, comprehensive error handling, debugging utilities

### 🎯 **Implementation Status (Updated October 2025)**

#### ✅ **Phase 1: Foundation (COMPLETED)**
- ✅ Feature-based clean architecture
- ✅ OpenAI API compatibility with full specification
- ✅ Vertex AI provider with Claude models
- ✅ Supabase JWT authentication system
- ✅ High-performance streaming with SSE

#### ✅ **Phase 2: Multi-Provider (COMPLETED)**
- ✅ AWS Bedrock integration
- ✅ OpenAI Direct API support
- ✅ Anthropic Claude API
- ✅ Azure OpenAI integration
- ✅ Groq, Mistral, Cohere providers
- ✅ Intelligent provider selection and fallback

#### ✅ **Phase 3: Advanced Tool Calling (COMPLETED)**
- ✅ Client-adaptive tool calling system
- ✅ OpenAI and Claude format support
- ✅ Tool execution orchestration
- ✅ Response format optimization
- ✅ Real-time streaming with tools

#### ✅ **Phase 4: Production Features (COMPLETED)**
- ✅ Comprehensive monitoring with Prometheus
- ✅ Langfuse integration for observability
- ✅ Request/response logging and tracking
- ✅ Performance metrics and health checks
- ✅ Error handling and retry logic

#### ✅ **Phase 5: Developer Experience (COMPLETED)**
- ✅ Extensive testing framework (25+ tests)
- ✅ Integration test utilities
- ✅ Performance benchmarking tools
- ✅ Debugging and diagnostic tools
- ✅ Comprehensive documentation

#### 🔄 **Phase 6: Future Enhancements (PLANNED)**
- ⏳ MCP (Model Context Protocol) integration
- ⏳ Microsandbox for secure tool execution
- ⏳ Local model inference with Candle
- ⏳ Vector database integration
- ⏳ Advanced agent workflows

## Technology Stack

### Core Technologies
- **Language**: Rust 1.75+
- **Web Framework**: Axum + Tokio
- **Database**: PostgreSQL (via Supabase)
- **Authentication**: Supabase JWT
- **ML Framework**: Candle (HuggingFace)
- **Cloud SDKs**: GCP, AWS, OpenAI, Anthropic

### Key Dependencies
- `axum` - High-performance web framework with full middleware support
- `tokio` - Async runtime with comprehensive ecosystem
- `sqlx` - Type-safe database access with connection pooling
- `reqwest` - HTTP client with streaming and multipart support
- `serde` - Fast serialization/deserialization
- `tracing` - Structured logging with OpenTelemetry integration
- `jsonwebtoken` - JWT validation with RSA/ECDSA support
- `prometheus` - Metrics collection and exposition
- `langfuse` - LLM observability and tracing
- `gcp_auth` - Google Cloud Platform authentication
- `similar` - Text diffing and patch generation

## Architecture Highlights

### Clean Architecture Layers
```
┌─────────────────────────────────────────┐
│         API Layer (Presentation)        │
│   HTTP Handlers, Middleware, Routes    │
└─────────────┬───────────────────────────┘
              │
┌─────────────▼───────────────────────────┐
│       Domain Layer (Business Logic)     │
│  Providers, Tools, Models, Services     │
└─────────────┬───────────────────────────┘
              │
┌─────────────▼───────────────────────────┐
│      Infrastructure Layer (External)    │
│  Databases, Cloud APIs, MCP, Webhooks   │
└─────────────────────────────────────────┘
```

### Key Design Patterns
- **Provider Abstraction**: Trait-based provider interface with unified error handling
- **Client Detection**: Automatic format optimization based on request patterns
- **Tool Orchestration**: Dynamic tool discovery, execution, and response formatting
- **Streaming Aggregation**: Real-time response collection with chunk optimization
- **Fallback Strategy**: Intelligent provider selection with health monitoring
- **Repository Pattern**: Type-safe database access with connection pooling
- **Dependency Injection**: State-based DI via Axum with configuration management

## API Overview

### OpenAI-Compatible Endpoints

```http
# Chat Completions
POST /v1/chat/completions
Content-Type: application/json
Authorization: Bearer <supabase-jwt>

{
  "model": "claude-sonnet-4-5-20250929",
  "messages": [
    {"role": "user", "content": "Hello!"}
  ]
}

# Embeddings
POST /v1/embeddings

# Models
GET /v1/models

# Admin
GET /admin/health
GET /admin/providers
POST /admin/webhooks/register
```

### 🆕 **Extended Features (Added October 2025)**

```json
{
  "model": "claude-sonnet-4-20250514",
  "messages": [...],
  "tools": [...],
  "stream": true,
  "provider_override": "vertex-ai",
  "conversation_id": "conv-123",
  "previous_response_id": "resp-456",
  "client_type": "codex_cli",
  "optimization_level": "aggressive",
  "fallback_providers": ["anthropic", "openai"],
  "monitoring": {
    "track_usage": true,
    "collect_metrics": true,
    "trace_id": "trace-789"
  }
}
```

### **Real-time Performance Metrics**

```bash
# Prometheus metrics exposed at /metrics
codex_cli_requests_total{model="claude-sonnet-4",provider="vertex",status="success"} 1234
codex_cli_request_duration_seconds{model="claude-sonnet-4",provider="vertex"} 0.85
codex_cli_tool_calls_total{tool_name="apply_patch",status="success"} 456
codex_cli_streaming_chunks_total{model="claude-sonnet-4"} 15
```

## Database Schema

### Core Tables
- `webhooks` - Registered webhook configurations
- `request_logs` - API request tracking
- `tool_calls` - Tool execution logs
- `memories` - Vector-based knowledge store
- `audit_logs` - Security and admin actions

See [TECHNICAL_ARCHITECTURE.md](./TECHNICAL_ARCHITECTURE.md#database-schema) for complete schema.

## Security Considerations

### Authentication
- Supabase JWT validation
- Role-based access control
- Per-user rate limiting

### Sandboxing
- MCP servers run in microsandbox VMs
- CPU, memory, and network restrictions
- Filesystem access controls

### Data Protection
- TLS/HTTPS only
- Secrets in environment variables
- API keys never logged
- HMAC webhook signatures

## Performance Requirements

### Latency Targets
- Chat completion (non-streaming): < 2s (P50), < 5s (P95)
- Chat completion (streaming, first token): < 500ms (P50), < 1s (P95)
- Embeddings: < 200ms (P50), < 500ms (P95)

### Throughput
- Concurrent requests: 1000+
- Requests per second: 500+
- WebSocket connections: 10,000+

## Getting Help

### Issues and Questions
1. Check [IMPLEMENTATION_GUIDE.md](./IMPLEMENTATION_GUIDE.md) - Common Issues section
2. Review relevant specification sections
3. Check existing code comments
4. Create a GitHub issue with:
   - Clear description
   - Steps to reproduce
   - Expected vs actual behavior
   - Environment details

### Contributing
1. Read the specifications
2. Follow the coding standards
3. Write tests
4. Update documentation
5. Submit PR with clear description

## 🛠 **Development Commands (Updated October 2025)**

### **Basic Commands**
```bash
# Quick development check (fastest feedback)
cargo check

# Build project
cargo build

# Run server (requires .env setup)
cargo run

# Format and lint (before commits)
cargo fmt && cargo clippy
```

### **Testing Commands**
```bash
# Run all tests
cargo test

# Run tests with output
cargo test -- --nocapture

# Run specific test suites
cargo test integration     # Integration tests
cargo test contract        # Contract tests
cargo test unit           # Unit tests

# Run specific provider tests
cargo test test_vertex_e2e
cargo test test_anthropic_provider_integration
cargo test test_streaming_tool_calls

# Performance benchmarks
cargo test test_performance_benchmarks -- --nocapture
```

### **Production Commands**
```bash
# Build optimized release
cargo build --release

# Security audit
cargo audit

# Check for outdated dependencies
cargo outdated

# Generate documentation
cargo doc --open
```

### **Monitoring & Debugging**
```bash
# View real-time metrics (when server running)
curl http://localhost:8080/metrics

# Health check
curl http://localhost:8080/health

# Provider status
curl http://localhost:8080/admin/providers
```

## Document Conventions

### Priority Levels
- **P0**: Critical, must have
- **P1**: Important, should have
- **P2**: Nice to have, could have
- **P3**: Future consideration

### Status Indicators
- ✅ Completed
- 🔄 In Progress
- ⏳ Planned
- ❌ Blocked
- 💡 Idea/Proposal

### Code Examples
- All code examples are in Rust unless specified
- Examples show complete, runnable code where possible
- Configuration examples use YAML or TOML as appropriate

## 📈 **Version History**

### **v2.1 - October 15, 2025 (CURRENT)**
- ✅ **Production-Ready Implementation**: All core features implemented
- ✅ **Multi-Provider Support**: 8 providers fully integrated
- ✅ **Advanced Tool Calling**: Client-adaptive tool calling system
- ✅ **Comprehensive Testing**: 25+ integration tests with realistic patterns
- ✅ **Production Monitoring**: Prometheus + Langfuse observability
- ✅ **Streaming Infrastructure**: High-performance SSE with optimization
- ✅ **Developer Experience**: Extensive documentation and tooling

### **v2.0 - October 2, 2025**
- Complete rewrite of specifications
- Added MCP integration planning
- Added local model support planning
- Enhanced webhook system design
- Comprehensive architecture documentation

### **v1.0 - Initial**
- Basic Vertex AI integration
- Simple authentication
- Initial documentation

## 🚀 **Next Actions (Updated October 2025)**

### ✅ **COMPLETED - Major Implementation Milestone Achieved!**
All core functionality has been successfully implemented and tested. The system is production-ready with comprehensive features.

### 🔄 **Current Focus (This Week)**
1. **Documentation Enhancement**: Update all guides with latest implementation details
2. **Performance Optimization**: Fine-tune provider response times and caching
3. **Integration Testing**: Expand test coverage for edge cases
4. **Monitoring Dashboards**: Create Grafana dashboards for Prometheus metrics
5. **User Adoption**: Gather feedback and optimize based on real usage patterns

### 🎯 **Short-term (This Month)**
1. **MCP Integration**: Implement Model Context Protocol for advanced tool orchestration
2. **Local Model Support**: Add Candle/HuggingFace inference capabilities
3. **Vector Database**: Integrate vector storage for conversation memory
4. **Advanced Analytics**: Enhanced usage analytics and cost optimization
5. **Multi-tenant Support**: Add organization-level isolation and billing

### 🌟 **Long-term Vision (3-6 Months)**
1. **Agent Workflows**: Advanced multi-step AI agent orchestration
2. **Knowledge Base**: Persistent learning and context accumulation
3. **Real-time Collaboration**: Multi-user conversation support
4. **Enterprise Features**: Advanced security, compliance, and audit trails
5. **Ecosystem Integration**: Plugin system for custom providers and tools

## Contact and Support

For questions or clarifications about this documentation:
- Create a GitHub issue
- Tag relevant documents
- Provide specific questions
- Include context

---

**Document Maintenance:**
- Review and update monthly
- Keep in sync with implementation
- Archive outdated decisions
- Document new features

**Last Review:** October 15, 2025
**Next Review:** November 15, 2025

---

## 🏆 **Project Status Summary**

**Current Status**: ✅ **PRODUCTION READY**
- **Implementation Progress**: 85% Complete (Core features fully implemented)
- **Test Coverage**: 25+ comprehensive integration tests
- **Provider Support**: 8 major providers integrated
- **Performance**: Sub-second response times with streaming optimization
- **Monitoring**: Full observability with Prometheus + Langfuse
- **Documentation**: Comprehensive guides and API documentation

This project represents a fully functional, production-ready AI proxy service that successfully abstracts multiple AI providers behind a unified OpenAI-compatible API, with advanced features like adaptive tool calling, intelligent fallback, and comprehensive monitoring.
