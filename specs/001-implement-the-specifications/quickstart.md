# Quickstart Guide: Vertex AI and Claude 4 Sonnet Integration

**Date**: October 2, 2025
**Feature**: Vertex AI and Claude 4 Sonnet Priority Implementation

## Overview

This quickstart guide validates the core functionality of the OpenAI-compatible API service that routes requests to Google Vertex AI's Claude 4 Sonnet model. Follow these steps to verify the implementation meets all acceptance criteria.

## Prerequisites

### 1. Environment Setup
```bash
# Clone and navigate to repository
git clone <repository-url>
cd llm-supabase-rs
git checkout 001-implement-the-specifications

# Install Rust (if not installed)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source ~/.cargo/env

# Verify Rust version
cargo --version  # Should be 1.75+
```

### 2. Configuration
```bash
# Copy environment template
cp .env.example .env

# Edit .env with your credentials
nano .env
```

Required environment variables:
```bash
# Server Configuration
HOST=0.0.0.0
PORT=8080
LOG_LEVEL=info

# Supabase (Authentication)
SUPABASE_URL=https://your-project.supabase.co
SUPABASE_ANON_KEY=your_anon_key
SUPABASE_SERVICE_ROLE_KEY=your_service_role_key
SUPABASE_JWT_SECRET=your_jwt_secret

# Google Cloud Platform (Vertex AI)
GCP_PROJECT_ID=your-gcp-project-id
GCP_LOCATION=us-central1
GOOGLE_APPLICATION_CREDENTIALS=./gcp-credentials.json

# Model Configuration
DEFAULT_MODEL=claude-4-sonnet-20250514
```

### 3. Google Cloud Service Account
```bash
# Place your GCP service account key file
cp /path/to/your/service-account-key.json ./gcp-credentials.json

# Set permissions (secure the file)
chmod 600 gcp-credentials.json
```

### 4. Build and Run
```bash
# Build the project
cargo build

# Run the service
cargo run
```

Expected output:
```
INFO llm_supabase_rs: Starting server on 0.0.0.0:8080
INFO llm_supabase_rs: Vertex AI client initialized for project: your-gcp-project-id
INFO llm_supabase_rs: Health check endpoint available at /health
```

## Validation Tests

### Test 1: Health Check (No Authentication Required)
**Purpose**: Verify service is running and providers are available

```bash
curl http://localhost:8080/health
```

**Expected Response**:
```json
{
  "status": "healthy",
  "version": "0.1.0",
  "timestamp": "2025-10-02T10:00:00Z",
  "providers": {
    "vertex-ai": {
      "status": "available",
      "latency_ms": 150
    }
  }
}
```

**Success Criteria**: Status is "healthy" and vertex-ai provider is "available"

### Test 2: Models Endpoint (With Authentication)
**Purpose**: Verify authentication and model listing

First, get a Supabase JWT token:
```bash
# Using Supabase CLI or your authentication method
SUPABASE_JWT="your-jwt-token-here"
```

Test the models endpoint:
```bash
curl -H "Authorization: Bearer $SUPABASE_JWT" \
     http://localhost:8080/v1/models
```

**Expected Response**:
```json
{
  "object": "list",
  "data": [
    {
      "id": "claude-4-sonnet-20250514",
      "object": "model",
      "created": 1696118400,
      "owned_by": "anthropic-vertex"
    }
  ]
}
```

**Success Criteria**: Returns Claude 4 Sonnet model information

### Test 3: Simple Chat Completion (Non-Streaming)
**Purpose**: Verify basic chat completion functionality

```bash
curl -X POST \
     -H "Authorization: Bearer $SUPABASE_JWT" \
     -H "Content-Type: application/json" \
     -d '{
       "model": "claude-4-sonnet-20250514",
       "messages": [
         {
           "role": "user",
           "content": "Hello! Can you help me test this API?"
         }
       ],
       "max_tokens": 100,
       "temperature": 0.7
     }' \
     http://localhost:8080/v1/chat/completions
```

**Expected Response Structure**:
```json
{
  "id": "chatcmpl-123abc",
  "object": "chat.completion",
  "created": 1696118400,
  "model": "claude-4-sonnet-20250514",
  "choices": [
    {
      "index": 0,
      "message": {
        "role": "assistant",
        "content": "Hello! I'd be happy to help you test this API..."
      },
      "finish_reason": "stop"
    }
  ],
  "usage": {
    "prompt_tokens": 15,
    "completion_tokens": 25,
    "total_tokens": 40
  }
}
```

**Success Criteria**:
- Response status: 200 OK
- Response time: < 5 seconds (95% requirement)
- Contains valid OpenAI-compatible structure
- Model field matches requested model
- Usage statistics included

### Test 4: Streaming Chat Completion
**Purpose**: Verify Server-Sent Events streaming functionality

```bash
curl -X POST \
     -H "Authorization: Bearer $SUPABASE_JWT" \
     -H "Content-Type: application/json" \
     -d '{
       "model": "claude-4-sonnet-20250514",
       "messages": [
         {
           "role": "user",
           "content": "Count from 1 to 5 slowly."
         }
       ],
       "stream": true,
       "max_tokens": 50
     }' \
     http://localhost:8080/v1/chat/completions
```

**Expected Response Format**:
```
data: {"id":"chatcmpl-456def","object":"chat.completion.chunk","created":1696118400,"model":"claude-4-sonnet-20250514","choices":[{"index":0,"delta":{"role":"assistant"},"finish_reason":null}]}

data: {"id":"chatcmpl-456def","object":"chat.completion.chunk","created":1696118400,"model":"claude-4-sonnet-20250514","choices":[{"index":0,"delta":{"content":"1"},"finish_reason":null}]}

data: {"id":"chatcmpl-456def","object":"chat.completion.chunk","created":1696118400,"model":"claude-4-sonnet-20250514","choices":[{"index":0,"delta":{"content":"..."},"finish_reason":null}]}

data: {"id":"chatcmpl-456def","object":"chat.completion.chunk","created":1696118400,"model":"claude-4-sonnet-20250514","choices":[{"index":0,"delta":{},"finish_reason":"stop"}]}

data: [DONE]
```

**Success Criteria**:
- Content-Type: text/event-stream
- SSE format with "data:" prefix
- First chunk contains role
- Subsequent chunks contain incremental content
- Final chunk has finish_reason
- Ends with "data: [DONE]"

### Test 5: Authentication Failure
**Purpose**: Verify proper authentication error handling

```bash
curl -X POST \
     -H "Content-Type: application/json" \
     -d '{
       "model": "claude-4-sonnet-20250514",
       "messages": [{"role": "user", "content": "Hello"}]
     }' \
     http://localhost:8080/v1/chat/completions
```

**Expected Response**:
```json
{
  "error": {
    "message": "Authentication required",
    "type": "authentication_error",
    "code": "missing_authorization"
  }
}
```

**Success Criteria**:
- Response status: 401 Unauthorized
- Proper error response format

### Test 6: Invalid Request Handling
**Purpose**: Verify input validation

```bash
curl -X POST \
     -H "Authorization: Bearer $SUPABASE_JWT" \
     -H "Content-Type: application/json" \
     -d '{
       "model": "claude-4-sonnet-20250514",
       "messages": [],
       "temperature": 3.0
     }' \
     http://localhost:8080/v1/chat/completions
```

**Expected Response**:
```json
{
  "error": {
    "message": "Messages array cannot be empty",
    "type": "invalid_request_error",
    "code": "invalid_messages",
    "param": "messages"
  }
}
```

**Success Criteria**:
- Response status: 400 Bad Request
- Specific validation error message

### Test 7: Performance Validation
**Purpose**: Verify concurrent request handling

```bash
# Install Apache Bench (if not available, use alternative load testing tool)
# Create test script for concurrent requests
cat > test_concurrent.sh << 'EOF'
#!/bin/bash
SUPABASE_JWT="your-jwt-token-here"

# Test 10 concurrent requests
ab -n 50 -c 10 \
   -H "Authorization: Bearer $SUPABASE_JWT" \
   -H "Content-Type: application/json" \
   -p test_payload.json \
   http://localhost:8080/v1/chat/completions
EOF

# Create test payload
cat > test_payload.json << 'EOF'
{
  "model": "claude-4-sonnet-20250514",
  "messages": [
    {
      "role": "user",
      "content": "What is 2+2?"
    }
  ],
  "max_tokens": 10
}
EOF

chmod +x test_concurrent.sh
./test_concurrent.sh
```

**Success Criteria**:
- All requests complete successfully
- 95% of requests complete within 5 seconds
- No connection errors under concurrent load

## Provider Fallback Testing

### Test 8: Provider Unavailable Scenario
**Purpose**: Verify fallback behavior when Vertex AI is unavailable

This test requires temporarily disabling Vertex AI access or using a test configuration.

**Setup**:
```bash
# Temporarily modify configuration to use invalid credentials
# or block access to Vertex AI endpoints
```

**Test Request**:
```bash
curl -X POST \
     -H "Authorization: Bearer $SUPABASE_JWT" \
     -H "Content-Type: application/json" \
     -d '{
       "model": "claude-4-sonnet-20250514",
       "messages": [{"role": "user", "content": "Hello"}]
     }' \
     http://localhost:8080/v1/chat/completions
```

**Expected Behavior**:
- If fallback provider configured: Request succeeds with fallback
- If no fallback configured: 503 Service Unavailable response

## Logging Verification

### Test 9: Request Logging
**Purpose**: Verify comprehensive request logging

After running previous tests, check the logs:

```bash
# Check application logs
tail -f logs/app.log  # or wherever logs are configured

# Look for structured log entries like:
# INFO request_completed: duration_ms=1250 user_id=uuid tokens_used=45 status=success
```

**Success Criteria**:
- All requests logged with timing information
- Authentication context captured
- Token usage tracked
- Error conditions logged with context

## Troubleshooting

### Common Issues

1. **GCP Authentication Errors**:
   - Verify service account key file exists and has correct permissions
   - Ensure service account has Vertex AI permissions
   - Check GCP project ID matches configuration

2. **Supabase JWT Errors**:
   - Verify JWT secret matches Supabase project configuration
   - Ensure JWT token is valid and not expired
   - Check token format (should be valid JWT)

3. **Connection Errors**:
   - Verify network connectivity to Google Cloud
   - Check firewall rules and proxy settings
   - Ensure DNS resolution works for googleapis.com

4. **Performance Issues**:
   - Monitor system resources (CPU, memory)
   - Check connection pool configuration
   - Verify adequate system limits (file descriptors, etc.)

### Debug Commands

```bash
# Check service health in detail
curl -v http://localhost:8080/health

# Test with verbose curl output
curl -v -X POST ... (add -v to any test command)

# Check logs for specific request ID
grep "request_id:abc123" logs/app.log

# Monitor resource usage
htop
# or
docker stats  # if running in container
```

## Success Checklist

After completing all tests, verify:

- [ ] Service starts successfully with valid configuration
- [ ] Health endpoint returns healthy status
- [ ] Authentication works with valid Supabase JWT tokens
- [ ] Authentication properly rejects invalid tokens
- [ ] Non-streaming chat completions work with Claude 4 Sonnet
- [ ] Streaming chat completions work with proper SSE format
- [ ] Response times meet 95% < 5 seconds requirement
- [ ] Concurrent requests handled successfully (up to 1000)
- [ ] Input validation prevents invalid requests
- [ ] Provider fallback works when configured
- [ ] Comprehensive logging captures all required information
- [ ] Error responses match OpenAI format
- [ ] Model listing endpoint works

## Next Steps

Once all tests pass:

1. **Production Deployment**: Deploy to production environment
2. **Monitoring Setup**: Configure dashboards and alerts
3. **Performance Tuning**: Optimize based on production load
4. **Additional Providers**: Implement fallback provider support
5. **Enhanced Features**: Add additional OpenAI API endpoints

---

**Validation Status**: Ready for implementation testing
**Required Success Rate**: 100% of acceptance criteria must pass