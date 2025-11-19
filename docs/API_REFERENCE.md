# API Reference

**Project:** Universal AI Server Proxy (llm-supabase-rs)
**Version:** 2.1
**Date:** October 15, 2025
**Status:** ✅ PRODUCTION READY

## Overview

The Universal AI Server Proxy provides a fully OpenAI-compatible REST API that abstracts multiple AI providers behind a unified interface. This reference documents all available endpoints, request/response formats, and advanced features.

---

## 🌐 **Base Configuration**

### **Server Information**
- **Base URL**: `http://localhost:8080` (development) or your deployed domain
- **Protocol**: HTTP/HTTPS
- **Content-Type**: `application/json`
- **Authentication**: Bearer token (Supabase JWT)

### **Rate Limits**
- **Requests per minute**: 1000 (default, configurable)
- **Concurrent requests**: 100 per user
- **Token limits**: Per model specifications

---

## 🔐 **Authentication**

### **JWT Bearer Token**
All endpoints (except health checks) require authentication via Supabase JWT tokens:

```http
Authorization: Bearer eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9...
```

### **Token Types Supported**
- **User tokens**: For authenticated user requests
- **Anonymous tokens**: For public access (if enabled)
- **Service role tokens**: For admin operations

---

## 📡 **Core API Endpoints**

### **1. Chat Completions**

#### **POST /v1/chat/completions**
OpenAI-compatible chat completion endpoint with multi-provider support.

**Request Body:**
```json
{
  "model": "claude-sonnet-4-20250514",
  "messages": [
    {
      "role": "user",
      "content": "Hello, how are you?"
    }
  ],
  "max_tokens": 4096,
  "temperature": 0.7,
  "stream": false,
  "tools": [...],
  "tool_choice": "auto"
}
```

**Advanced Parameters:**
```json
{
  "model": "claude-sonnet-4-20250514",
  "messages": [...],
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

**Response (Non-streaming):**
```json
{
  "id": "chatcmpl-123",
  "object": "chat.completion",
  "created": 1699461776,
  "model": "claude-sonnet-4-20250514",
  "provider": "vertex-ai",
  "choices": [
    {
      "index": 0,
      "message": {
        "role": "assistant",
        "content": "Hello! I'm doing well, thank you for asking...",
        "tool_calls": null
      },
      "finish_reason": "stop"
    }
  ],
  "usage": {
    "prompt_tokens": 12,
    "completion_tokens": 45,
    "total_tokens": 57
  },
  "conversation_id": "conv-123",
  "response_id": "resp-456"
}
```

**Response (Streaming):**
```
data: {"id":"chatcmpl-123","object":"chat.completion.chunk","created":1699461776,"model":"claude-sonnet-4-20250514","choices":[{"index":0,"delta":{"content":"Hello"},"finish_reason":null}]}

data: {"id":"chatcmpl-123","object":"chat.completion.chunk","created":1699461776,"model":"claude-sonnet-4-20250514","choices":[{"index":0,"delta":{"content":"!"},"finish_reason":null}]}

data: {"id":"chatcmpl-123","object":"chat.completion.chunk","created":1699461776,"model":"claude-sonnet-4-20250514","choices":[{"index":0,"delta":{},"finish_reason":"stop"}],"usage":{"prompt_tokens":12,"completion_tokens":45,"total_tokens":57}}

data: [DONE]
```

**Tool Calling Response:**
```json
{
  "id": "chatcmpl-123",
  "choices": [
    {
      "message": {
        "role": "assistant",
        "content": null,
        "tool_calls": [
          {
            "id": "call_123",
            "type": "function",
            "function": {
              "name": "read_file",
              "arguments": "{\"file_path\": \"main.rs\"}"
            }
          }
        ]
      },
      "finish_reason": "tool_calls"
    }
  ]
}
```

### **2. Models List**

#### **GET /v1/models**
Returns list of available models across all providers.

**Response:**
```json
{
  "object": "list",
  "data": [
    {
      "id": "claude-sonnet-4-20250514",
      "object": "model",
      "created": 1699461776,
      "owned_by": "anthropic",
      "provider": "vertex-ai",
      "capabilities": {
        "chat_completion": true,
        "streaming": true,
        "tool_calling": true,
        "vision": false
      },
      "context_window": 200000,
      "max_output_tokens": 8192
    },
    {
      "id": "gpt-4-turbo",
      "object": "model",
      "created": 1699461776,
      "owned_by": "openai",
      "provider": "openai",
      "capabilities": {
        "chat_completion": true,
        "streaming": true,
        "tool_calling": true,
        "vision": true
      },
      "context_window": 128000,
      "max_output_tokens": 4096
    }
  ]
}
```

### **3. Embeddings**

#### **POST /v1/embeddings**
Generate embeddings for text inputs.

**Request:**
```json
{
  "model": "text-embedding-3-small",
  "input": "Your text to embed",
  "encoding_format": "float",
  "dimensions": 1536
}
```

**Response:**
```json
{
  "object": "list",
  "data": [
    {
      "object": "embedding",
      "embedding": [0.0023064255, -0.009327292, ...],
      "index": 0
    }
  ],
  "model": "text-embedding-3-small",
  "usage": {
    "prompt_tokens": 5,
    "total_tokens": 5
  }
}
```

---

## 🔧 **Admin & Monitoring Endpoints**

### **4. Health Check**

#### **GET /health**
Basic health check endpoint (no authentication required).

**Response:**
```json
{
  "status": "healthy",
  "timestamp": "2025-10-15T10:30:00Z",
  "version": "2.1.0",
  "uptime_seconds": 3600
}
```

### **5. Detailed Health**

#### **GET /admin/health**
Comprehensive health information including provider status.

**Response:**
```json
{
  "status": "healthy",
  "timestamp": "2025-10-15T10:30:00Z",
  "version": "2.1.0",
  "uptime_seconds": 3600,
  "providers": {
    "vertex-ai": {
      "status": "healthy",
      "last_check": "2025-10-15T10:29:30Z",
      "response_time_ms": 245,
      "error_rate": 0.002
    },
    "openai": {
      "status": "healthy",
      "last_check": "2025-10-15T10:29:28Z",
      "response_time_ms": 189,
      "error_rate": 0.001
    },
    "anthropic": {
      "status": "degraded",
      "last_check": "2025-10-15T10:29:25Z",
      "response_time_ms": 1200,
      "error_rate": 0.05,
      "last_error": "Rate limit exceeded"
    }
  },
  "system": {
    "memory_usage_mb": 245,
    "cpu_usage_percent": 12.5,
    "active_connections": 15,
    "active_requests": 3
  },
  "metrics": {
    "requests_total": 12450,
    "requests_per_minute": 125,
    "average_response_time_ms": 890,
    "error_rate": 0.008
  }
}
```

### **6. Provider Status**

#### **GET /admin/providers**
Detailed information about all configured providers.

**Response:**
```json
{
  "providers": [
    {
      "name": "vertex-ai",
      "type": "google_vertex",
      "status": "active",
      "models": [
        "claude-sonnet-4-20250514",
        "claude-haiku-4-20250514",
        "gemini-pro-1.5"
      ],
      "capabilities": {
        "chat_completion": true,
        "streaming": true,
        "tool_calling": true,
        "embeddings": true
      },
      "configuration": {
        "project_id": "your-project",
        "location": "us-central1",
        "authenticated": true
      },
      "metrics": {
        "requests_total": 8450,
        "success_rate": 0.998,
        "average_latency_ms": 245,
        "last_request": "2025-10-15T10:29:45Z"
      }
    }
  ]
}
```

### **7. Prometheus Metrics**

#### **GET /metrics**
Prometheus-formatted metrics for monitoring and alerting.

**Response:**
```
# HELP codex_cli_requests_total Total number of Codex CLI requests
# TYPE codex_cli_requests_total counter
codex_cli_requests_total{model="claude-sonnet-4",provider="vertex",status="success",client_type="codex_cli"} 1234

# HELP codex_cli_request_duration_seconds Request processing duration
# TYPE codex_cli_request_duration_seconds histogram
codex_cli_request_duration_seconds_bucket{model="claude-sonnet-4",provider="vertex",operation_type="chat_completion",le="0.1"} 15
codex_cli_request_duration_seconds_bucket{model="claude-sonnet-4",provider="vertex",operation_type="chat_completion",le="0.5"} 125
codex_cli_request_duration_seconds_bucket{model="claude-sonnet-4",provider="vertex",operation_type="chat_completion",le="1.0"} 890
codex_cli_request_duration_seconds_sum{model="claude-sonnet-4",provider="vertex",operation_type="chat_completion"} 1045.5
codex_cli_request_duration_seconds_count{model="claude-sonnet-4",provider="vertex",operation_type="chat_completion"} 1234

# HELP codex_cli_tool_calls_total Total tool calls executed
# TYPE codex_cli_tool_calls_total counter
codex_cli_tool_calls_total{tool_name="apply_patch",provider="local",status="success"} 456
codex_cli_tool_calls_total{tool_name="read_file",provider="local",status="success"} 789
```

---

## 🛠 **Tool Calling**

### **Tool Definition Format**
```json
{
  "type": "function",
  "function": {
    "name": "read_file",
    "description": "Read contents of a file from the filesystem",
    "parameters": {
      "type": "object",
      "properties": {
        "file_path": {
          "type": "string",
          "description": "Path to the file to read"
        },
        "encoding": {
          "type": "string",
          "description": "File encoding (default: utf-8)",
          "enum": ["utf-8", "ascii", "latin-1"]
        }
      },
      "required": ["file_path"]
    }
  }
}
```

### **Built-in Tools**
The system includes several built-in tools for common operations:

#### **File Operations**
- `read_file`: Read file contents
- `write_file`: Write content to file
- `search_files`: Search for files matching patterns
- `list_directory`: List directory contents

#### **Code Operations**
- `apply_patch`: Apply unified diff patches
- `generate_diff`: Generate diff between texts
- `extract_code_blocks`: Extract code from markdown

#### **System Operations**
- `execute_command`: Run shell commands (with safety restrictions)
- `get_system_info`: Get system information

### **Tool Execution Response**
```json
{
  "role": "tool",
  "content": "File contents here...",
  "tool_call_id": "call_123",
  "name": "read_file"
}
```

---

## 🌊 **Streaming**

### **Server-Sent Events (SSE)**
Streaming responses use the SSE format with `data:` prefixed JSON chunks.

**Connection Headers:**
```http
Accept: text/event-stream
Cache-Control: no-cache
```

**Stream Events:**
- `data: {json}` - Content chunks
- `data: [DONE]` - End of stream marker

### **Streaming with Tools**
Tool calls can be streamed in real-time:

```
data: {"choices":[{"delta":{"tool_calls":[{"index":0,"id":"call_123","type":"function","function":{"name":"read_file","arguments":""}}]}}]}

data: {"choices":[{"delta":{"tool_calls":[{"index":0,"function":{"arguments":"{\"file"}}]}}]}

data: {"choices":[{"delta":{"tool_calls":[{"index":0,"function":{"arguments":"_path\": \"main.rs\"}"}}]}}]}

data: [DONE]
```

---

## 🔄 **Provider Fallback**

### **Automatic Fallback**
The system automatically falls back to alternative providers when:
- Primary provider is unavailable
- Rate limits are exceeded
- Request timeout occurs
- Provider returns error

**Fallback Priority:**
1. **Primary**: Specified provider or default
2. **Secondary**: Best available alternative
3. **Tertiary**: Any remaining healthy provider

### **Manual Provider Selection**
Override provider selection using the `provider_override` parameter:

```json
{
  "model": "gpt-4",
  "provider_override": "openai",
  "fallback_providers": ["azure_openai", "vertex-ai"],
  "messages": [...]
}
```

---

## 🎛 **Advanced Features**

### **Conversation Management**
Track multi-turn conversations using conversation IDs:

```json
{
  "conversation_id": "conv-123",
  "previous_response_id": "resp-456",
  "messages": [...]
}
```

### **Client Optimization**
Automatic optimization based on client type:

```json
{
  "client_type": "codex_cli",
  "optimization_level": "aggressive"
}
```

**Available Client Types:**
- `codex_cli`: Optimized for code generation
- `chat_interface`: Optimized for conversational UI
- `api_client`: Generic API client
- `mobile_app`: Mobile-optimized responses

### **Request Tracing**
Enable detailed request tracing:

```json
{
  "monitoring": {
    "trace_id": "trace-789",
    "collect_metrics": true,
    "track_usage": true
  }
}
```

---

## 📊 **Response Formats**

### **Error Responses**
All errors follow the OpenAI error format:

```json
{
  "error": {
    "message": "The model 'invalid-model' does not exist",
    "type": "invalid_request_error",
    "param": "model",
    "code": "model_not_found"
  }
}
```

**Common Error Codes:**
- `invalid_request_error`: Malformed request
- `authentication_error`: Invalid or missing auth token
- `permission_denied_error`: Insufficient permissions
- `rate_limit_exceeded`: Too many requests
- `provider_error`: Upstream provider error
- `internal_error`: Server error

### **HTTP Status Codes**
- `200`: Success
- `400`: Bad Request
- `401`: Unauthorized
- `403`: Forbidden
- `404`: Not Found
- `429`: Too Many Requests
- `500`: Internal Server Error
- `502`: Bad Gateway (provider error)
- `503`: Service Unavailable

---

## 🔗 **WebSocket Support** (Future)

### **Real-time Chat** (Planned)
```javascript
const ws = new WebSocket('wss://api.example.com/v1/chat/stream');
ws.send(JSON.stringify({
  model: 'claude-sonnet-4-20250514',
  messages: [...],
  stream: true
}));
```

---

## 📋 **Request/Response Examples**

### **Complete Conversation Example**
```bash
# Initial request
curl -X POST http://localhost:8080/v1/chat/completions \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer your-token" \
  -d '{
    "model": "claude-sonnet-4-20250514",
    "messages": [
      {"role": "user", "content": "Write a Rust function to calculate fibonacci"}
    ],
    "conversation_id": "conv-123"
  }'

# Follow-up request
curl -X POST http://localhost:8080/v1/chat/completions \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer your-token" \
  -d '{
    "model": "claude-sonnet-4-20250514",
    "messages": [
      {"role": "user", "content": "Write a Rust function to calculate fibonacci"},
      {"role": "assistant", "content": "Here is a Rust function..."},
      {"role": "user", "content": "Now add error handling"}
    ],
    "conversation_id": "conv-123",
    "previous_response_id": "resp-456"
  }'
```

### **Tool Calling Example**
```bash
curl -X POST http://localhost:8080/v1/chat/completions \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer your-token" \
  -d '{
    "model": "claude-sonnet-4-20250514",
    "messages": [
      {"role": "user", "content": "Read main.rs and add error handling"}
    ],
    "tools": [
      {
        "type": "function",
        "function": {
          "name": "read_file",
          "description": "Read a file from filesystem",
          "parameters": {
            "type": "object",
            "properties": {
              "file_path": {"type": "string"}
            },
            "required": ["file_path"]
          }
        }
      }
    ],
    "client_type": "codex_cli"
  }'
```

---

## 🚀 **Performance Specifications**

### **Latency Targets**
- **Non-streaming responses**: P50 < 1s, P95 < 3s
- **First token (streaming)**: P50 < 500ms, P95 < 1s
- **Tool execution**: P50 < 200ms, P95 < 1s
- **Provider failover**: < 100ms additional latency

### **Throughput Targets**
- **Concurrent requests**: 1000+
- **Requests per second**: 500+
- **Streaming connections**: 10,000+

### **Reliability Targets**
- **Uptime**: 99.9%
- **Error rate**: < 0.1% under normal conditions
- **Provider availability**: 99.5% effective (with fallback)

---

## 🔐 **Security Considerations**

### **Authentication**
- JWT tokens validated on every request
- Token expiration enforced
- Role-based access control

### **Input Validation**
- All inputs sanitized and validated
- Request size limits enforced
- Rate limiting per user/token

### **Data Protection**
- No API keys logged
- Secure credential storage
- TLS/HTTPS enforced in production

---

## 📚 **SDKs and Integration**

### **cURL Examples**
See examples throughout this document.

### **Python SDK** (Future)
```python
import llm_proxy

client = llm_proxy.Client(
    base_url="https://api.example.com",
    auth_token="your-token"
)

response = client.chat.completions.create(
    model="claude-sonnet-4-20250514",
    messages=[{"role": "user", "content": "Hello!"}]
)
```

### **JavaScript SDK** (Future)
```javascript
import { LLMProxy } from 'llm-proxy-js';

const client = new LLMProxy({
  baseURL: 'https://api.example.com',
  authToken: 'your-token'
});

const response = await client.chat.completions.create({
  model: 'claude-sonnet-4-20250514',
  messages: [{ role: 'user', content: 'Hello!' }]
});
```

---

## 🔄 **Changelog**

### **Version 2.1** (October 15, 2025)
- ✅ Full production release
- ✅ 8 provider integrations
- ✅ Advanced tool calling
- ✅ Comprehensive monitoring
- ✅ Performance optimization

### **Version 2.0** (October 2, 2025)
- Initial comprehensive specification
- Multi-provider architecture design
- Tool calling framework

---

**This API reference represents the current production implementation of the Universal AI Server Proxy, providing comprehensive OpenAI-compatible functionality with advanced multi-provider support and extensive monitoring capabilities.**