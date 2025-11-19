# Private Langfuse Setup Guide for Codex CLI Integration Testing

## 🎯 Overview

This guide provides complete instructions for setting up a private Langfuse instance for LLM observability in the Codex CLI integration testing environment.

---

## 🚀 Quick Start

### 1. Start Complete Environment (2 minutes)

```bash
# Start all services including private Langfuse
docker-compose -f tests/docker/docker-compose.test.yml up -d

# Wait for services to be ready
docker-compose -f tests/docker/docker-compose.test.yml ps

# Verify Langfuse is accessible
curl http://localhost:3000/api/public/health
```

### 2. Access Langfuse Dashboard (1 minute)

```bash
# Open Langfuse in browser
open http://localhost:3000

# Default login credentials (change in production):
# Email: admin@langfuse.com
# Password: langfuse
```

### 3. Configure Integration Testing (1 minute)

```bash
# Set environment variables for local Langfuse
export LANGFUSE_BASE_URL="http://localhost:3000"
export LANGFUSE_PUBLIC_KEY="pk_lf_test_public_key"
export LANGFUSE_SECRET_KEY="sk_lf_test_secret_key"

# Run tests with monitoring
./tests/run_integration_tests.sh
```

---

## 📋 Complete Service Architecture

### Docker Services

| **Service** | **Port** | **Purpose** | **Health Check** |
|------------|----------|-------------|------------------|
| **langfuse-server** | 3000 | LLM observability dashboard | `curl localhost:3000/api/public/health` |
| **langfuse-db** | 5433 | Langfuse database | `pg_isready -h localhost -p 5433` |
| **postgres** | 5432 | Application database | `pg_isready -h localhost -p 5432` |
| **redis** | 6379 | Caching layer | `redis-cli ping` |
| **integration-tests** | - | Test execution | Automated health checks |

### Service Dependencies

```mermaid
graph TD
    A[integration-tests] --> B[postgres]
    A --> C[redis] 
    A --> D[langfuse-server]
    D --> E[langfuse-db]
    F[performance-tests] --> B
    F --> D
    G[security-tests] --> B
    G --> D
```

---

## ⚙️ Configuration Details

### Langfuse Server Configuration

**Environment Variables:**
```yaml
DATABASE_URL: postgresql://langfuse:langfuse_password@langfuse-db:5432/langfuse
NEXTAUTH_URL: http://localhost:3000
NEXTAUTH_SECRET: langfuse_secret_key_change_in_production
SALT: salt_change_in_production
AUTH_DISABLE_USERNAME_PASSWORD: false
AUTH_DISABLE_SIGNUP: false
LANGFUSE_ENABLE_EXPERIMENTAL_FEATURES: true
TELEMETRY_ENABLED: false
```

**Volumes:**
```yaml
langfuse_uploads: # File upload storage
langfuse_postgres_data: # Database persistence
```

### Integration Test Configuration

**Test Services Use Private Langfuse:**
```yaml
# integration-tests service
LANGFUSE_BASE_URL: http://langfuse-server:3000
LANGFUSE_PUBLIC_KEY: pk_lf_test_public_key
LANGFUSE_SECRET_KEY: sk_lf_test_secret_key
LLM_MONITORING_ENABLED: true

# performance-tests service  
LANGFUSE_BASE_URL: http://langfuse-server:3000
LANGFUSE_PUBLIC_KEY: pk_lf_perf_test_key
LANGFUSE_SECRET_KEY: sk_lf_perf_test_key

# security-tests service
LANGFUSE_BASE_URL: http://langfuse-server:3000
LANGFUSE_PUBLIC_KEY: pk_lf_security_test_key
LANGFUSE_SECRET_KEY: sk_lf_security_test_key
```

---

## 🔧 Usage Instructions

### Running Integration Tests with Langfuse

**1. Start Complete Environment:**
```bash
# Start all services
docker-compose -f tests/docker/docker-compose.test.yml up -d

# Verify all services are healthy
docker-compose -f tests/docker/docker-compose.test.yml ps
```

**2. Configure API Keys (First Time Setup):**
```bash
# Access Langfuse dashboard
open http://localhost:3000

# Create project and get API keys
# Navigate to Settings > API Keys
# Create new key pair for testing
```

**3. Run Tests with Monitoring:**
```bash
# Set local Langfuse configuration
export LANGFUSE_BASE_URL="http://localhost:3000"
export LANGFUSE_PUBLIC_KEY="your_generated_public_key"
export LANGFUSE_SECRET_KEY="your_generated_secret_key"

# Execute tests with full observability
docker-compose -f tests/docker/docker-compose.test.yml up integration-tests

# Or run locally with monitoring
./tests/run_integration_tests.sh
```

### Viewing Test Results in Langfuse

**1. Access Dashboard:**
```
URL: http://localhost:3000
Project: Codex CLI Integration Testing
```

**2. Key Dashboards:**
- **Traces** - Complete request flows with timing
- **Sessions** - Multi-turn conversation analysis
- **Users** - Test user activity patterns
- **Models** - Provider performance comparison
- **Scores** - Test success rates and performance metrics

**3. Custom Views:**
- Filter by `tags: ["codex-cli", "integration-test"]`
- Group by provider for performance comparison
- Analyze tool execution patterns
- Monitor conversation success rates

---

## 📊 Monitoring Integration

### Langfuse Trace Structure

**Trace Hierarchy:**
```
📊 codex_cli_request (Main Trace)
├── 🔧 tool_apply_patch (Tool Execution Span)
├── 🔧 tool_read_file (Tool Execution Span)  
├── 💬 conversation_turn (Conversation Span)
└── 🌊 streaming_response (Streaming Span)
```

**Metadata Captured:**
- Request/response content and timing
- Tool execution details and security validation
- Provider performance and fallback behavior
- Conversation context and turn analysis
- Performance metrics and resource usage

### Integration with Existing Monitoring

**Prometheus + Langfuse:**
- Prometheus provides system metrics
- Langfuse provides LLM-specific observability
- Combined dashboards for complete visibility

**Example Combined Query:**
```promql
# Correlate Prometheus and Langfuse data
rate(codex_cli_requests_total[5m]) * on(provider) 
  group_left(langfuse_success_rate) langfuse_success_rate_by_provider
```

---

## 🛠 Development Workflow

### Local Development with Langfuse

**1. Start Local Services:**
```bash
# Start only required services for development
docker-compose -f tests/docker/docker-compose.test.yml up -d langfuse-server langfuse-db

# Verify Langfuse is ready
curl http://localhost:3000/api/public/health
```

**2. Configure Application:**
```bash
# Set local instance configuration
export LANGFUSE_LOCAL_INSTANCE=true
export LANGFUSE_BASE_URL="http://localhost:3000"

# Run application with monitoring
cargo run --features monitoring
```

**3. Run Tests and Monitor:**
```bash
# Execute tests with observability
cargo test test_codex_cli_code_generation_workflow

# View results in Langfuse dashboard
open http://localhost:3000
```

### Production Deployment

**1. Private Langfuse Deployment:**
```yaml
# docker-compose.production.yml
services:
  langfuse-server:
    image: langfuse/langfuse:latest
    environment:
      DATABASE_URL: postgresql://langfuse:${LANGFUSE_DB_PASSWORD}@langfuse-db:5432/langfuse
      NEXTAUTH_SECRET: ${LANGFUSE_NEXTAUTH_SECRET}
      SALT: ${LANGFUSE_SALT}
      TELEMETRY_ENABLED: false
    depends_on:
      - langfuse-db
```

**2. Application Configuration:**
```bash
# Production environment variables
export LANGFUSE_BASE_URL="https://langfuse.yourdomain.com"
export LANGFUSE_PUBLIC_KEY="${LANGFUSE_PROD_PUBLIC_KEY}"
export LANGFUSE_SECRET_KEY="${LANGFUSE_PROD_SECRET_KEY}"
```

---

## 🔍 Troubleshooting

### Common Issues

**Issue 1: Langfuse Connection Failed**
```bash
# Check service status
docker-compose -f tests/docker/docker-compose.test.yml ps langfuse-server

# Check logs
docker-compose -f tests/docker/docker-compose.test.yml logs langfuse-server

# Verify network connectivity
docker-compose -f tests/docker/docker-compose.test.yml exec integration-tests \
  curl http://langfuse-server:3000/api/public/health
```

**Issue 2: Database Connection Issues**
```bash
# Check Langfuse database
docker-compose -f tests/docker/docker-compose.test.yml exec langfuse-db \
  psql -U langfuse -d langfuse -c "SELECT version();"

# Check application database
docker-compose -f tests/docker/docker-compose.test.yml exec postgres \
  psql -U postgres -d llm_supabase_test -c "SELECT version();"
```

**Issue 3: API Key Authentication**
```bash
# Test API key authentication
curl -H "Authorization: Bearer ${LANGFUSE_SECRET_KEY}" \
  http://localhost:3000/api/public/health

# Verify keys are set correctly
echo "Public Key: $LANGFUSE_PUBLIC_KEY"
echo "Secret Key: ${LANGFUSE_SECRET_KEY:0:10}..."
```

### Health Check Commands

```bash
# Complete system health check
./tests/docker/health_check.sh

# Individual service checks
docker-compose -f tests/docker/docker-compose.test.yml exec langfuse-server curl -f http://localhost:3000/api/public/health
docker-compose -f tests/docker/docker-compose.test.yml exec integration-tests cargo test test_basic_request_creation
```

---

## 📈 Performance Optimization

### Langfuse Performance Tuning

**Database Optimization:**
```sql
-- Optimize Langfuse database for high throughput
ALTER SYSTEM SET shared_buffers = '256MB';
ALTER SYSTEM SET effective_cache_size = '1GB';
ALTER SYSTEM SET maintenance_work_mem = '64MB';
SELECT pg_reload_conf();
```

**Configuration Tuning:**
```yaml
# Optimized Langfuse configuration
langfuse-server:
  environment:
    # Performance tuning
    NODE_ENV: production
    DATABASE_POOL_SIZE: 20
    REDIS_URL: redis://redis:6379
    
    # Monitoring optimization
    LANGFUSE_BATCH_SIZE: 100
    LANGFUSE_FLUSH_INTERVAL: 30
```

### Integration Performance

**Batch Configuration:**
```rust
// Optimize for testing workload
let config = LangfuseConfig {
    batch_size: 50,           // Smaller batches for faster feedback
    flush_interval_seconds: 15, // More frequent flushing
    detailed_tracing: true,    // Full tracing for testing
    ..Default::default()
};
```

---

## 🎯 Next Steps

### Setup Validation

- [ ] Private Langfuse instance running (http://localhost:3000)
- [ ] All Docker services healthy
- [ ] Integration tests connecting to local Langfuse
- [ ] Traces appearing in Langfuse dashboard
- [ ] Performance metrics being collected

### Production Setup

- [ ] Configure production Langfuse deployment
- [ ] Set up proper authentication and security
- [ ] Configure backup and data retention
- [ ] Set up monitoring alerts
- [ ] Validate production integration

---

## 📚 Additional Resources

### Langfuse Documentation
- **Official Docs**: https://langfuse.com/docs
- **Self-Hosting**: https://langfuse.com/docs/deployment/self-host
- **API Reference**: https://langfuse.com/docs/api

### Integration Examples
- `src/monitoring/langfuse_integration.rs` - Complete integration implementation
- `tests/docker/docker-compose.test.yml` - Full environment setup
- `tests/config/test-config.toml` - Configuration examples

### Support
- Check Docker Compose logs for issues
- Validate network connectivity between services
- Monitor resource usage during testing
- Review Langfuse documentation for advanced configuration

---

**Status: ✅ Private Langfuse deployment ready for comprehensive LLM observability**

This setup provides complete, self-hosted LLM monitoring for the Codex CLI integration testing framework with full data privacy and control.