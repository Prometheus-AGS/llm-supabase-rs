# Documentation Index

**Project:** Universal AI Server Proxy (llm-supabase-rs)  
**Version:** 2.0  
**Last Updated:** October 2, 2025

## Overview

This directory contains comprehensive documentation for implementing a universal AI server proxy that provides a unified OpenAI-compatible API interface to multiple AI providers, with support for tool orchestration via MCP (Model Context Protocol), webhooks, and local model inference.

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

### ✅ What This System Does

- **Unified API**: Single OpenAI-compatible REST API for multiple providers
- **Multi-Provider**: Support for Vertex AI, AWS Bedrock, OpenAI, Anthropic, Groq, OpenRouter, Ollama, and local models
- **Tool Orchestration**: MCP (Model Context Protocol) client and server for tool execution
- **Webhooks**: Comprehensive lifecycle hooks for monitoring and integration
- **Local Models**: GPU-accelerated inference with HuggingFace and Candle
- **Authentication**: Supabase JWT-based auth with role-based access control
- **Observability**: Metrics, tracing, and request logging

### 🚧 Phased Implementation

#### Phase 1: Foundation (Weeks 1-4)
- Project structure
- OpenAI API compatibility
- Vertex AI provider
- Authentication
- Basic streaming

#### Phase 2: Multi-Provider (Weeks 5-8)
- AWS Bedrock
- OpenAI Direct
- Anthropic
- Provider selection logic
- Fallback/retry

#### Phase 3: MCP Integration (Weeks 9-12)
- MCP client
- Microsandbox integration
- Tool registry
- MCP server
- Tool orchestration

#### Phase 4: Webhooks (Weeks 13-16)
- Event dispatcher
- Delivery system
- Supabase integration
- Request logging
- Admin API

#### Phase 5: Local Models (Weeks 17-20)
- HuggingFace downloader
- Candle integration
- GPU support
- Model caching
- Batch processing

#### Phase 6: Advanced (Weeks 21+)
- Knowledge base
- Production hardening
- Performance optimization
- Agent server (future)

## Technology Stack

### Core Technologies
- **Language**: Rust 1.75+
- **Web Framework**: Axum + Tokio
- **Database**: PostgreSQL (via Supabase)
- **Authentication**: Supabase JWT
- **ML Framework**: Candle (HuggingFace)
- **Cloud SDKs**: GCP, AWS, OpenAI, Anthropic

### Key Dependencies
- `axum` - Web framework
- `tokio` - Async runtime
- `sqlx` - Database access
- `reqwest` - HTTP client
- `serde` - Serialization
- `tracing` - Logging
- `jsonwebtoken` - JWT validation
- `candle-core` - ML inference

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
- **Provider Abstraction**: Trait-based provider interface
- **Tool Registry**: Dynamic tool discovery and routing
- **Event Dispatcher**: Async webhook processing
- **Repository Pattern**: Database access abstraction
- **Dependency Injection**: State-based DI via Axum

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

### Extended Features

```json
{
  "model": "claude-sonnet-4.5",
  "messages": [...],
  "provider_override": "vertex-ai",
  "webhooks": {
    "on_start": "https://...",
    "on_end": "https://...",
    "on_tool_call": "https://..."
  },
  "mcp_config": {
    "allow_external_tools": true,
    "pass_through_unmatched": true
  }
}
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

## Development Commands

```bash
# Build project
cargo build

# Run server
cargo run

# Run tests
cargo test

# Watch for changes
cargo watch -x check -x test

# Format code
cargo fmt

# Lint code
cargo clippy -- -D warnings

# Security audit
cargo audit

# Run specific test
cargo test test_name -- --nocapture
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

## Version History

### v2.0 - October 2, 2025
- Complete rewrite of specifications
- Added MCP integration
- Added local model support
- Enhanced webhook system
- Comprehensive architecture documentation

### v1.0 - Initial
- Basic Vertex AI integration
- Simple authentication
- Initial documentation

## Next Actions

### Immediate (This Week)
1. Review all three core documents
2. Set up development environment
3. Create initial project structure
4. Implement configuration system
5. Create basic API endpoints

### Short-term (This Month)
1. Complete Phase 1 foundation
2. Integrate Vertex AI provider
3. Implement authentication
4. Add streaming support
5. Begin Phase 2 planning

### Long-term (3-6 Months)
1. Complete all 5 phases
2. Production deployment
3. Performance optimization
4. Comprehensive testing
5. Documentation updates

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

**Last Review:** October 2, 2025  
**Next Review:** November 2, 2025
