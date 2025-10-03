# Functional Specification: Universal AI Server Proxy

**Version:** 2.0  
**Status:** Draft  
**Last Updated:** October 2, 2025  
**Project:** llm-supabase-rs

## Executive Summary

This document specifies a universal AI server proxy built in Rust that provides a unified OpenAI-compatible API interface to multiple AI providers, LLMs, and execution environments. The system acts as an intelligent routing layer with authentication, webhooks, tool orchestration via MCP (Model Context Protocol), and future agent execution capabilities.

## Table of Contents

1. [Overview](#overview)
2. [Core Capabilities](#core-capabilities)
3. [API Specification](#api-specification)
4. [Provider Support](#provider-support)
5. [Authentication & Authorization](#authentication--authorization)
6. [Webhook System](#webhook-system)
7. [MCP Integration](#mcp-integration)
8. [Local Model Support](#local-model-support)
9. [Knowledge Base & Memory](#knowledge-base--memory)
10. [Future: Agent Server](#future-agent-server)
11. [Performance Requirements](#performance-requirements)
12. [Security Considerations](#security-considerations)

---

## 1. Overview

### 1.1 Purpose

Create a high-performance Rust-based proxy server that:
- Exposes a single OpenAI-compatible REST API
- Routes requests to multiple AI providers with automatic protocol conversion
- Manages tools and function calling via MCP servers
- Provides comprehensive webhook integration with Supabase
- Supports local model inference with GPU acceleration
- Acts as both MCP client and MCP server
- Enables future agent orchestration capabilities

### 1.2 Goals

- **Unified Interface**: Single API for all AI providers
- **Protocol Agnostic**: Transparent conversion between OpenAI and native provider formats
- **Tool Orchestration**: Automatic tool discovery and routing via MCP
- **Observable**: Complete webhook lifecycle for monitoring and integration
- **Performant**: Rust-based with GPU support and efficient resource management
- **Extensible**: Plugin architecture for providers, tools, and agents

### 1.3 Non-Goals (Current Phase)

- Multi-tenancy with resource isolation (future phase)
- Built-in UI/dashboard (API-only)
- Model training or fine-tuning
- Data storage beyond caching and configuration

---

## 2. Core Capabilities

### 2.1 OpenAI API Compatibility

The server exposes the following OpenAI v1 API endpoints:

#### 2.1.1 Chat Completions
```
POST /v1/chat/completions
```

**Features:**
- Standard chat completions
- Streaming responses (SSE)
- Tool/function calling
- Multi-turn conversations
- Vision support (image inputs)
- System prompts and multi-role messages

#### 2.1.2 Embeddings
```
POST /v1/embeddings
```

**Features:**
- Text embeddings generation
- Batch embedding support
- Multiple embedding providers
- Dimension normalization

#### 2.1.3 Models
```
GET /v1/models
GET /v1/models/{model_id}
```

**Features:**
- Dynamic model enumeration from all configured providers
- Provider capability metadata (streaming, vision, tools, max tokens)
- Model aliases and routing configuration
- Real-time availability status

### 2.2 Request Flow

```
Client Request (OpenAI format)
    ↓
Authentication & Validation
    ↓
Webhook: Processing Start
    ↓
Provider Selection & Route Determination
    ↓
Protocol Conversion (OpenAI → Native)
    ↓
Tool Call Detection & MCP Routing
    ↓
Provider Execution (with tool resolution loop)
    ↓
Protocol Conversion (Native → OpenAI)
    ↓
Webhook: Processing End
    ↓
Client Response (OpenAI format)
```

---

## 3. API Specification

### 3.1 Request Format Extensions

Beyond standard OpenAI parameters, the server accepts:

```json
{
  // Standard OpenAI fields
  "model": "claude-sonnet-4-5-20250929",
  "messages": [...],
  
  // Extended fields
  "provider_override": "vertex-ai",  // Force specific provider
  "webhooks": {
    "on_start": "https://project.supabase.co/functions/v1/llm-start",
    "on_end": "https://project.supabase.co/functions/v1/llm-end",
    "on_tool_call": "https://project.supabase.co/functions/v1/tool-call",
    "on_tool_resolution": "https://project.supabase.co/functions/v1/tool-resolve",
    "on_error": "https://project.supabase.co/functions/v1/error-handler"
  },
  "mcp_config": {
    "allow_external_tools": true,
    "tool_timeout_ms": 30000,
    "pass_through_unmatched": true
  },
  "metadata": {
    "user_id": "uuid",
    "session_id": "session-uuid",
    "tags": ["production", "customer-support"]
  }
}
```

### 3.2 Response Format Extensions

```json
{
  // Standard OpenAI response
  "id": "chatcmpl-...",
  "object": "chat.completion",
  "created": 1234567890,
  "model": "claude-sonnet-4-5-20250929",
  "choices": [...],
  
  // Extended metadata
  "provider": "vertex-ai",
  "usage": {
    "prompt_tokens": 100,
    "completion_tokens": 50,
    "total_tokens": 150,
    "cached_tokens": 20  // If prompt caching used
  },
  "timing": {
    "total_ms": 1234,
    "provider_ms": 1100,
    "conversion_ms": 50,
    "tool_calls_ms": 84
  },
  "tools_used": [
    {
      "name": "web_search",
      "provider": "mcp-server-brave",
      "duration_ms": 84
    }
  ]
}
```

### 3.3 Administrative Endpoints

```
GET  /admin/health              # Health check
GET  /admin/providers           # List configured providers
GET  /admin/mcp/servers         # List MCP servers
POST /admin/webhooks/register   # Register persistent webhooks
GET  /admin/webhooks            # List registered webhooks
POST /admin/models/download     # Trigger HuggingFace download
GET  /admin/models/status       # Check model download status
```

---

## 4. Provider Support

### 4.1 Provider Architecture

Each provider implements a common trait:

```rust
#[async_trait]
pub trait LLMProvider: Send + Sync {
    /// Provider identifier
    fn name(&self) -> &str;
    
    /// Convert OpenAI request to native format
    async fn convert_request(&self, req: OpenAIRequest) -> Result<ProviderRequest>;
    
    /// Execute native request
    async fn execute(&self, req: ProviderRequest) -> Result<ProviderResponse>;
    
    /// Convert native response to OpenAI format
    async fn convert_response(&self, res: ProviderResponse) -> Result<OpenAIResponse>;
    
    /// List available models
    async fn list_models(&self) -> Result<Vec<ModelInfo>>;
    
    /// Check if provider supports capability
    fn supports(&self, capability: Capability) -> bool;
    
    /// Streaming support
    async fn execute_stream(&self, req: ProviderRequest) -> Result<StreamingResponse>;
}
```

### 4.2 Supported Providers

#### 4.2.1 Vertex AI (Google Cloud) - Priority 1
- **Models**: Gemini, Claude on Vertex
- **Auth**: GCP service account
- **Features**: Streaming, vision, tools, prompt caching
- **Endpoint**: Regional API endpoints

#### 4.2.2 AWS Bedrock - Priority 2
- **Models**: Claude, Llama, Titan, Mistral
- **Auth**: AWS IAM credentials
- **Features**: Streaming, vision, tools, guardrails
- **Endpoint**: Regional Bedrock APIs

#### 4.2.3 OpenAI - Priority 2
- **Models**: GPT-4, GPT-3.5, embeddings
- **Auth**: API key
- **Features**: Native format (no conversion needed)
- **Endpoint**: api.openai.com

#### 4.2.4 Anthropic Direct - Priority 2
- **Models**: Claude family
- **Auth**: API key
- **Features**: Streaming, vision, tools, prompt caching
- **Endpoint**: api.anthropic.com

#### 4.2.5 OpenRouter - Priority 3
- **Models**: 200+ models from multiple providers
- **Auth**: API key
- **Features**: Model discovery, fallback routing
- **Endpoint**: openrouter.ai

#### 4.2.6 Groq - Priority 3
- **Models**: Llama, Mixtral (high-speed inference)
- **Auth**: API key
- **Features**: Ultra-fast inference, streaming
- **Endpoint**: api.groq.com

#### 4.2.7 Ollama - Priority 3
- **Models**: Any GGUF model
- **Auth**: None (local)
- **Features**: Local inference, model management
- **Endpoint**: Configurable (default: localhost:11434)

#### 4.2.8 OpenAI-Compatible - Priority 3
- **Models**: Any API-compatible service
- **Auth**: Configurable
- **Features**: Generic OpenAI protocol
- **Endpoint**: Configurable

#### 4.2.9 HuggingFace/Candle (Local) - Priority 3
- **Models**: Any GGUF or supported format
- **Auth**: None (local)
- **Features**: GPU acceleration, model download, caching
- **Storage**: Configurable via `HF_MODEL_PATH` env var

### 4.3 Provider Selection Logic

```rust
pub enum ProviderSelection {
    /// Explicitly specified in request
    Explicit(String),
    
    /// Based on model name pattern
    ModelBased(String),
    
    /// Based on capabilities needed
    CapabilityBased(Vec<Capability>),
    
    /// Load balancing across multiple providers
    LoadBalanced(Vec<String>),
    
    /// Fallback chain
    Fallback(Vec<String>),
}
```

**Selection Priority:**
1. `provider_override` in request
2. Model name prefix/suffix matching (e.g., `claude-*` → Anthropic/Vertex)
3. Capability requirements (tools, vision, streaming)
4. Load balancing configuration
5. Fallback chain with retry logic

### 4.4 Model Configuration (YAML)

```yaml
providers:
  vertex_ai:
    enabled: true
    project_id: ${GCP_PROJECT_ID}
    location: us-central1
    credentials: ${GOOGLE_APPLICATION_CREDENTIALS}
    models:
      - name: claude-sonnet-4-5-20250929
        alias: ["claude-4.5-sonnet", "claude-sonnet-4.5"]
        capabilities: [streaming, vision, tools, prompt_caching]
        max_tokens: 200000
        
  bedrock:
    enabled: true
    region: us-east-1
    models:
      - name: anthropic.claude-3-5-sonnet-20241022-v2:0
        alias: ["claude-3.5-sonnet-bedrock"]
        
  openai:
    enabled: true
    api_key: ${OPENAI_API_KEY}
    
  huggingface_local:
    enabled: true
    model_path: ${HF_MODEL_PATH}
    models:
      - name: llama-3.1-8b-instruct
        repo: meta-llama/Llama-3.1-8B-Instruct
        files:
          - model.safetensors
          - config.json
          - tokenizer.json
        download_on_startup: true
        gpu_layers: -1  # All layers on GPU
```

---

## 5. Authentication & Authorization

### 5.1 Supabase JWT Authentication

The server validates all requests using Supabase JWT tokens:

```
Authorization: Bearer <supabase-jwt-token>
```

**Token Types:**
1. **User JWT**: Authenticated user tokens
2. **Anonymous Key**: `anon` key for public access
3. **Service Role Key**: Administrative access

### 5.2 JWT Payload Extraction

```rust
pub struct AuthContext {
    pub user_id: Option<Uuid>,
    pub role: Role,
    pub email: Option<String>,
    pub metadata: serde_json::Value,
    pub permissions: Vec<Permission>,
}

pub enum Role {
    Anonymous,
    Authenticated,
    ServiceRole,
}
```

### 5.3 Permission Model

```yaml
permissions:
  models:
    - role: anonymous
      allowed_providers: [ollama]
      rate_limit: 10/hour
      
    - role: authenticated
      allowed_providers: [vertex_ai, openai, anthropic]
      rate_limit: 1000/hour
      
    - role: service_role
      allowed_providers: [all]
      rate_limit: unlimited
```

### 5.4 Webhook Authentication

Outgoing webhooks include authentication:

```http
POST /functions/v1/llm-start
Authorization: Bearer <service-role-key>
Content-Type: application/json
X-Webhook-Signature: <hmac-sha256>
X-Request-ID: <uuid>
```

---

## 6. Webhook System

### 6.1 Webhook Lifecycle Events

```rust
pub enum WebhookEvent {
    /// LLM processing started
    ProcessingStart {
        request_id: Uuid,
        model: String,
        provider: String,
        timestamp: DateTime<Utc>,
        user_id: Option<Uuid>,
        prompt_tokens: usize,
    },
    
    /// LLM processing completed
    ProcessingEnd {
        request_id: Uuid,
        duration_ms: u64,
        completion_tokens: usize,
        total_cost: Option<f64>,
        cached: bool,
    },
    
    /// Tool call initiated
    ToolCallStart {
        request_id: Uuid,
        tool_name: String,
        tool_id: String,
        arguments: serde_json::Value,
        timestamp: DateTime<Utc>,
    },
    
    /// Tool resolution request
    ToolResolution {
        request_id: Uuid,
        tool_id: String,
        tool_name: String,
        arguments: serde_json::Value,
        /// If true, expects response from webhook
        requires_response: bool,
    },
    
    /// Tool call completed
    ToolCallEnd {
        request_id: Uuid,
        tool_id: String,
        duration_ms: u64,
        success: bool,
        result: Option<serde_json::Value>,
        error: Option<String>,
    },
    
    /// Error occurred
    Error {
        request_id: Uuid,
        error_type: String,
        error_message: String,
        stage: ProcessingStage,
        timestamp: DateTime<Utc>,
    },
}
```

### 6.2 Webhook Registration

#### 6.2.1 Per-Request Webhooks

Specified in each API request (see Section 3.1).

#### 6.2.2 Persistent Webhooks

Registered via admin API:

```json
POST /admin/webhooks/register
{
  "user_id": "uuid",  // Optional, for user-specific hooks
  "events": ["processing_start", "processing_end", "tool_call_start"],
  "endpoint": "https://project.supabase.co/functions/v1/llm-webhook",
  "auth_header": "Bearer <token>",
  "enabled": true,
  "metadata": {
    "environment": "production"
  }
}
```

#### 6.2.3 Webhook Storage (Supabase)

```sql
CREATE TABLE webhooks (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID REFERENCES auth.users(id),
    events TEXT[] NOT NULL,
    endpoint TEXT NOT NULL,
    auth_header TEXT,
    enabled BOOLEAN DEFAULT true,
    metadata JSONB DEFAULT '{}'::jsonb,
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW()
);

CREATE INDEX idx_webhooks_user_enabled ON webhooks(user_id, enabled);
```

### 6.3 Webhook Delivery

```rust
pub struct WebhookDelivery {
    /// Async delivery with retry
    pub async fn deliver(&self, event: WebhookEvent) -> Result<()>;
    
    /// Retry configuration
    pub max_retries: usize,
    pub retry_delay_ms: u64,
    pub timeout_ms: u64,
    
    /// Dead letter queue for failed deliveries
    pub dlq_enabled: bool,
}
```

**Retry Policy:**
- Max retries: 3
- Exponential backoff: 1s, 2s, 4s
- Timeout per attempt: 10s
- Failed webhooks logged to Supabase

### 6.4 Tool Resolution Webhooks

For custom tool handling:

```
POST <tool_resolution_endpoint>
{
  "request_id": "uuid",
  "tool_name": "custom_search",
  "tool_id": "call_abc123",
  "arguments": {
    "query": "rust async patterns",
    "limit": 10
  }
}

Response:
{
  "result": {
    "results": [...]
  }
}
```

The server waits for the response and includes it in the next LLM call.

---

## 7. MCP Integration

### 7.1 MCP Overview

Model Context Protocol (MCP) enables:
- Standardized tool/resource definitions
- Dynamic tool discovery
- Sandboxed execution
- Streaming communication

### 7.2 MCP Client (Server → MCP Servers)

#### 7.2.1 Configuration

```json
{
  "mcpServers": {
    "brave-search": {
      "command": "microsandbox",
      "args": ["run", "--config", "./mcp-servers/brave-search.json"],
      "env": {
        "BRAVE_API_KEY": "${BRAVE_API_KEY}"
      },
      "tools": ["web_search"],
      "enabled": true
    },
    "github": {
      "command": "microsandbox",
      "args": ["run", "--config", "./mcp-servers/github.json"],
      "env": {
        "GITHUB_TOKEN": "${GITHUB_TOKEN}"
      },
      "tools": ["search_repos", "create_issue", "list_prs"],
      "enabled": true
    },
    "filesystem": {
      "command": "microsandbox",
      "args": ["run", "--config", "./mcp-servers/filesystem.json"],
      "tools": ["read_file", "write_file", "list_directory"],
      "allowed_paths": ["/workspace"],
      "enabled": false  // Disabled by default for security
    }
  }
}
```

#### 7.2.2 Tool Discovery

On startup:
1. Load MCP configuration
2. Launch MCP servers in microsandbox VMs
3. Query each server for available tools
4. Build tool registry with routing information
5. Register tools in OpenAI function schema

#### 7.2.3 Tool Routing

```rust
pub enum ToolResolution {
    /// Handle via MCP server
    MCPServer { server_name: String, tool_name: String },
    
    /// Pass back to caller via webhook
    ExternalResolution { webhook_url: String },
    
    /// Built-in tool (rare)
    Internal { handler: String },
}

pub struct ToolRegistry {
    /// Map tool name to resolution strategy
    tools: HashMap<String, ToolResolution>,
    
    /// MCP server connections
    mcp_clients: HashMap<String, MCPClient>,
}
```

#### 7.2.4 Execution Flow

```
LLM responds with tool call
    ↓
Webhook: ToolCallStart
    ↓
Check tool registry
    ↓
If MCP Tool:
    Route to MCP server (via microsandbox)
    Execute in sandboxed environment
    Return result
    ↓
If External Tool:
    POST to tool resolution webhook
    Wait for response (with timeout)
    Return result
    ↓
Webhook: ToolCallEnd
    ↓
Continue LLM processing with tool result
```

### 7.3 MCP Server (Server ← External Clients)

The proxy itself acts as an MCP server for external tools.

#### 7.3.1 Exposed Tools

```json
{
  "tools": [
    {
      "name": "llm_completion",
      "description": "Generate text using any configured LLM provider",
      "inputSchema": {
        "type": "object",
        "properties": {
          "prompt": { "type": "string" },
          "model": { "type": "string" },
          "max_tokens": { "type": "number" }
        }
      }
    },
    {
      "name": "generate_embedding",
      "description": "Generate embeddings for text",
      "inputSchema": {
        "type": "object",
        "properties": {
          "text": { "type": "string" },
          "model": { "type": "string" }
        }
      }
    },
    {
      "name": "list_available_models",
      "description": "List all available models across providers",
      "inputSchema": { "type": "object", "properties": {} }
    }
  ]
}
```

#### 7.3.2 MCP Server Protocol

Implement HTTP streaming transport:

```
POST /mcp/v1/invoke
Content-Type: application/json

{
  "method": "tools/call",
  "params": {
    "name": "llm_completion",
    "arguments": {
      "prompt": "Explain async Rust",
      "model": "claude-sonnet-4.5",
      "max_tokens": 1000
    }
  }
}

Response (SSE stream):
data: {"type":"progress","content":"Generating..."}

data: {"type":"content","text":"Async Rust..."}

data: {"type":"complete","result":"..."}
```

#### 7.3.3 Security (Microsandbox)

All MCP server executions run in microsandbox VMs:

```rust
pub struct MicrosandboxConfig {
    /// CPU limit
    pub cpu_limit: f64,  // e.g., 0.5 for 50% of one core
    
    /// Memory limit in MB
    pub memory_limit: usize,  // e.g., 512
    
    /// Execution timeout
    pub timeout_ms: u64,  // e.g., 30000
    
    /// Network access
    pub network_enabled: bool,
    
    /// Allowed network endpoints
    pub allowed_hosts: Vec<String>,
    
    /// Filesystem access
    pub allowed_paths: Vec<PathBuf>,
}
```

---

## 8. Local Model Support

### 8.1 HuggingFace Integration

#### 8.1.1 Model Configuration

```yaml
huggingface_models:
  - name: llama-3.1-8b-instruct
    repo_id: meta-llama/Llama-3.1-8B-Instruct
    files:
      - model.safetensors
      - config.json
      - tokenizer.json
      - tokenizer_config.json
    download_on_startup: true
    priority: high
    
  - name: mistral-7b-instruct
    repo_id: mistralai/Mistral-7B-Instruct-v0.3
    files:
      - model.safetensors
      - config.json
      - tokenizer.json
    download_on_startup: false
    priority: medium
```

#### 8.1.2 Download Process

```rust
pub struct HuggingFaceDownloader {
    /// Download models on startup
    pub async fn download_configured_models(&self) -> Result<Vec<ModelStatus>>;
    
    /// Background download with progress tracking
    pub async fn download_model(
        &self, 
        repo_id: &str,
        files: Vec<String>,
        progress_callback: impl Fn(DownloadProgress)
    ) -> Result<PathBuf>;
    
    /// Verify model integrity
    pub async fn verify_model(&self, model_path: &Path) -> Result<bool>;
}

pub struct DownloadProgress {
    pub model_name: String,
    pub file: String,
    pub bytes_downloaded: u64,
    pub total_bytes: u64,
    pub percentage: f32,
}
```

**Storage Structure:**
```
$HF_MODEL_PATH/
├── meta-llama--Llama-3.1-8B-Instruct/
│   ├── model.safetensors
│   ├── config.json
│   └── tokenizer.json
├── mistralai--Mistral-7B-Instruct-v0.3/
│   └── ...
└── .cache/
    └── download_status.json
```

#### 8.1.3 Download Status API

```
GET /admin/models/status

Response:
{
  "models": [
    {
      "name": "llama-3.1-8b-instruct",
      "repo_id": "meta-llama/Llama-3.1-8B-Instruct",
      "status": "downloaded",
      "size_gb": 15.2,
      "path": "/models/meta-llama--Llama-3.1-8B-Instruct",
      "downloaded_at": "2025-10-02T10:00:00Z"
    },
    {
      "name": "mistral-7b-instruct",
      "status": "downloading",
      "progress": 45.2,
      "eta_seconds": 420
    }
  ]
}
```

### 8.2 Candle Inference

#### 8.2.1 GPU Detection

```rust
pub struct GPUDetector {
    /// Detect available CUDA devices
    pub fn detect_cuda(&self) -> Result<Vec<CudaDevice>>;
    
    /// Detect Metal (Apple Silicon)
    pub fn detect_metal(&self) -> Result<Option<MetalDevice>>;
    
    /// Select best device
    pub fn select_device(&self) -> Device;
}
```

#### 8.2.2 Model Loading

```rust
pub struct CandleModel {
    /// Load model with GPU support
    pub async fn load(
        &self,
        model_path: &Path,
        device: &Device,
        dtype: DType,
    ) -> Result<LoadedModel>;
    
    /// Quantization options
    pub quantization: Option<Quantization>,
}

pub enum Quantization {
    Q4_0,  // 4-bit quantization
    Q5_0,
    Q8_0,
    F16,   // Half precision
}
```

#### 8.2.3 Inference

```rust
pub struct CandleInference {
    /// Generate completion
    pub async fn generate(
        &self,
        prompt: &str,
        params: GenerationParams,
    ) -> Result<String>;
    
    /// Stream generation
    pub async fn generate_stream(
        &self,
        prompt: &str,
        params: GenerationParams,
    ) -> Result<impl Stream<Item = Result<String>>>;
}

pub struct GenerationParams {
    pub max_tokens: usize,
    pub temperature: f32,
    pub top_p: f32,
    pub top_k: Option<usize>,
    pub repetition_penalty: f32,
    pub stop_sequences: Vec<String>,
}
```

### 8.3 Mistral.rs Integration (Alternative)

If Mistral.rs exposes library crates:

```rust
// Use mistral-rs as inference backend
pub struct MistralRsProvider {
    /// Create from model path
    pub fn from_model_path(path: &Path, device: Device) -> Result<Self>;
    
    /// Execute inference
    pub async fn infer(&self, request: InferenceRequest) -> Result<InferenceResponse>;
}
```

**Benefits:**
- Optimized inference engine
- Built-in quantization
- GPU acceleration
- Production-ready

### 8.4 Performance Optimization

```rust
pub struct ModelCache {
    /// Keep models in memory
    loaded_models: HashMap<String, Arc<LoadedModel>>,
    
    /// LRU eviction when memory pressure
    max_memory_gb: usize,
    
    /// Unload least recently used
    pub async fn evict_lru(&mut self) -> Result<()>;
}

pub struct BatchProcessor {
    /// Batch multiple requests for efficiency
    pub async fn batch_infer(
        &self,
        requests: Vec<InferenceRequest>
    ) -> Result<Vec<InferenceResponse>>;
    
    /// Max batch size
    pub max_batch_size: usize,
    
    /// Batch timeout
    pub batch_timeout_ms: u64,
}
```

---

## 9. Knowledge Base & Memory

**Priority:** Lower (Phase 2)

### 9.1 Architecture

```rust
pub enum MemoryScope {
    /// Global knowledge available to all users
    Global,
    
    /// User-specific memory
    User(Uuid),
    
    /// Session-specific memory
    Session(String),
}

pub struct MemoryStore {
    /// Vector store for semantic search
    vector_store: Arc<dyn VectorStore>,
    
    /// Metadata store (Supabase)
    metadata_store: SupabaseClient,
    
    /// Cache for frequently accessed memories
    cache: Arc<MemoryCache>,
}
```

### 9.2 Vector Storage

```sql
-- Supabase vector extension
CREATE EXTENSION IF NOT EXISTS vector;

CREATE TABLE memories (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID REFERENCES auth.users(id),
    scope TEXT NOT NULL,  -- 'global', 'user', 'session'
    content TEXT NOT NULL,
    embedding vector(1536),  -- Adjust dimension based on model
    metadata JSONB DEFAULT '{}'::jsonb,
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW()
);

CREATE INDEX idx_memories_embedding ON memories 
  USING ivfflat (embedding vector_cosine_ops)
  WITH (lists = 100);

CREATE INDEX idx_memories_user_scope ON memories(user_id, scope);
```

### 9.3 Memory Operations

```rust
pub struct MemoryOperations {
    /// Store new memory
    pub async fn store(
        &self,
        content: String,
        scope: MemoryScope,
        metadata: serde_json::Value,
    ) -> Result<Uuid>;
    
    /// Semantic search
    pub async fn search(
        &self,
        query: &str,
        scope: MemoryScope,
        limit: usize,
    ) -> Result<Vec<Memory>>;
    
    /// Update memory
    pub async fn update(&self, id: Uuid, content: String) -> Result<()>;
    
    /// Delete memory
    pub async fn delete(&self, id: Uuid) -> Result<()>;
}
```

### 9.4 Automatic Memory Integration

When enabled, automatically inject relevant memories:

```rust
pub struct MemoryInjection {
    /// Extract query from user message
    fn extract_query(&self, messages: &[Message]) -> String;
    
    /// Search relevant memories
    async fn get_relevant_memories(
        &self,
        query: &str,
        scope: MemoryScope,
    ) -> Result<Vec<Memory>>;
    
    /// Inject as system message
    fn inject_into_request(
        &self,
        request: &mut ChatRequest,
        memories: Vec<Memory>,
    );
}
```

---

## 10. Future: Agent Server

**Priority:** Future Phase

### 10.1 Agent Architecture

```rust
pub trait Agent {
    /// Agent metadata
    fn metadata(&self) -> AgentMetadata;
    
    /// Execute agent task
    async fn execute(&self, task: AgentTask) -> Result<AgentResult>;
    
    /// Stream agent execution
    async fn execute_stream(&self, task: AgentTask) 
        -> Result<impl Stream<Item = AgentEvent>>;
}

pub struct AgentMetadata {
    pub name: String,
    pub version: String,
    pub description: String,
    pub capabilities: Vec<Capability>,
    pub required_tools: Vec<String>,
    pub language: AgentLanguage,
}

pub enum AgentLanguage {
    Rust,
    Python,
    JavaScript,
    Go,
    Any,  // Language-agnostic (via WASM)
}
```

### 10.2 Agent Execution (Microsandbox)

All agents run in isolated microsandbox VMs:

```rust
pub struct AgentExecutor {
    /// Load agent from path
    pub async fn load_agent(&self, path: &Path) -> Result<Box<dyn Agent>>;
    
    /// Execute in sandbox
    pub async fn execute_sandboxed(
        &self,
        agent: &dyn Agent,
        task: AgentTask,
        sandbox_config: MicrosandboxConfig,
    ) -> Result<AgentResult>;
}
```

### 10.3 Agent-to-Agent (A2A) Protocol

```rust
pub struct A2AMessage {
    pub from_agent: String,
    pub to_agent: String,
    pub message_type: A2AMessageType,
    pub payload: serde_json::Value,
    pub correlation_id: Uuid,
}

pub enum A2AMessageType {
    Request,
    Response,
    Event,
    Error,
}
```

### 10.4 Agent Registry

```sql
CREATE TABLE agents (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    name TEXT NOT NULL,
    version TEXT NOT NULL,
    language TEXT NOT NULL,
    capabilities JSONB DEFAULT '[]'::jsonb,
    wasm_path TEXT,  -- For WASM agents
    enabled BOOLEAN DEFAULT true,
    created_at TIMESTAMPTZ DEFAULT NOW()
);
```

---

## 11. Performance Requirements

### 11.1 Latency Targets

| Operation | Target (P50) | Target (P95) | Target (P99) |
|-----------|--------------|--------------|--------------|
| Chat completion (non-streaming) | < 2s | < 5s | < 10s |
| Chat completion (streaming, first token) | < 500ms | < 1s | < 2s |
| Embeddings | < 200ms | < 500ms | < 1s |
| Model list | < 50ms | < 100ms | < 200ms |
| Tool execution (MCP) | < 1s | < 3s | < 5s |
| Webhook delivery | < 100ms | < 500ms | < 1s |

### 11.2 Throughput Targets

- **Concurrent requests:** 1000+
- **Requests per second:** 500+
- **WebSocket connections:** 10,000+
- **MCP servers:** 50+ simultaneous

### 11.3 Resource Limits

```yaml
resources:
  cpu:
    min: 2 cores
    recommended: 8 cores
    max: 32 cores
    
  memory:
    min: 4 GB
    recommended: 16 GB
    max: 128 GB
    
  gpu:
    optional: true
    vram_min: 8 GB  # For local models
    vram_recommended: 24 GB
    
  storage:
    models: 100 GB+  # For local model cache
    logs: 10 GB
    cache: 5 GB
```

### 11.4 Optimization Strategies

1. **Connection Pooling**: Reuse HTTP connections to providers
2. **Request Batching**: Batch embedding requests
3. **Prompt Caching**: Use provider-native prompt caching
4. **Model Caching**: Keep frequently used local models in memory
5. **Async I/O**: Non-blocking operations throughout
6. **Zero-Copy**: Minimize data copying in hot paths
7. **GPU Utilization**: Maximize GPU usage for local inference

---

## 12. Security Considerations

### 12.1 Authentication & Authorization

- ✅ JWT validation on every request
- ✅ Role-based access control (RBAC)
- ✅ Per-user provider access restrictions
- ✅ Rate limiting per user/role
- ✅ API key rotation support

### 12.2 Sandboxing

- ✅ MCP servers run in microsandbox VMs
- ✅ Resource limits (CPU, memory, network)
- ✅ Filesystem access restrictions
- ✅ Network access controls
- ✅ Execution timeouts

### 12.3 Data Protection

- ✅ TLS/HTTPS only
- ✅ Secrets stored in environment variables
- ✅ API keys never logged
- ✅ PII detection in logs
- ✅ Data encryption at rest (for cached data)

### 12.4 Webhook Security

- ✅ HMAC signature verification
- ✅ Request ID tracking
- ✅ Replay attack prevention
- ✅ Webhook URL allowlisting
- ✅ Timeout protection

### 12.5 Input Validation

- ✅ Request size limits
- ✅ Token limits
- ✅ Schema validation
- ✅ SQL injection prevention (parameterized queries)
- ✅ XSS prevention

### 12.6 Audit Logging

All security-relevant events logged to Supabase:

```sql
CREATE TABLE audit_logs (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    timestamp TIMESTAMPTZ DEFAULT NOW(),
    user_id UUID,
    action TEXT NOT NULL,
    resource TEXT,
    success BOOLEAN,
    ip_address INET,
    user_agent TEXT,
    metadata JSONB DEFAULT '{}'::jsonb
);

CREATE INDEX idx_audit_logs_timestamp ON audit_logs(timestamp DESC);
CREATE INDEX idx_audit_logs_user ON audit_logs(user_id);
```

---

## 13. Deployment Considerations

### 13.1 Environment Variables

```bash
# Server
HOST=0.0.0.0
PORT=8080
LOG_LEVEL=info
RUST_LOG=llm_supabase_rs=debug

# Supabase
SUPABASE_URL=https://project.supabase.co
SUPABASE_ANON_KEY=...
SUPABASE_SERVICE_ROLE_KEY=...
SUPABASE_JWT_SECRET=...

# Providers
GCP_PROJECT_ID=...
GCP_LOCATION=us-central1
GOOGLE_APPLICATION_CREDENTIALS=./gcp-credentials.json

AWS_REGION=us-east-1
AWS_ACCESS_KEY_ID=...
AWS_SECRET_ACCESS_KEY=...

OPENAI_API_KEY=...
ANTHROPIC_API_KEY=...
OPENROUTER_API_KEY=...
GROQ_API_KEY=...

# Local Models
HF_MODEL_PATH=/data/models
HF_TOKEN=...  # For gated models

# MCP
MCP_CONFIG_PATH=./mcp-servers.json

# Performance
MAX_CONCURRENT_REQUESTS=1000
REQUEST_TIMEOUT_MS=60000
POOL_SIZE=50
```

### 13.2 Docker

```dockerfile
FROM rust:1.75 as builder
WORKDIR /app
COPY . .
RUN cargo build --release

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y ca-certificates libssl3 && rm -rf /var/lib/apt/lists/*
COPY --from=builder /app/target/release/llm-supabase-rs /usr/local/bin/
EXPOSE 8080
CMD ["llm-supabase-rs"]
```

### 13.3 Kubernetes

```yaml
apiVersion: apps/v1
kind: Deployment
metadata:
  name: llm-proxy
spec:
  replicas: 3
  selector:
    matchLabels:
      app: llm-proxy
  template:
    spec:
      containers:
      - name: llm-proxy
        image: llm-proxy:latest
        resources:
          requests:
            memory: "4Gi"
            cpu: "2"
          limits:
            memory: "16Gi"
            cpu: "8"
        env:
        - name: SUPABASE_URL
          valueFrom:
            secretKeyRef:
              name: llm-proxy-secrets
              key: supabase-url
```

---

## 14. Testing Strategy

### 14.1 Unit Tests

- Provider conversion logic
- Authentication/authorization
- Tool routing
- Webhook delivery
- Model configuration parsing

### 14.2 Integration Tests

- End-to-end API flows
- Provider integration
- MCP server communication
- Webhook callbacks
- Database operations

### 14.3 Performance Tests

- Load testing (wrk, k6)
- Streaming performance
- Concurrent request handling
- Memory usage profiling

### 14.4 Security Tests

- Authentication bypass attempts
- SQL injection
- Rate limit enforcement
- Sandbox escape attempts

---

## 15. Monitoring & Observability

### 15.1 Metrics

```rust
pub struct Metrics {
    // Request metrics
    pub requests_total: Counter,
    pub requests_duration: Histogram,
    pub requests_active: Gauge,
    
    // Provider metrics
    pub provider_requests: Counter,
    pub provider_errors: Counter,
    pub provider_latency: Histogram,
    
    // Model metrics
    pub model_usage: Counter,
    pub tokens_processed: Counter,
    pub cache_hits: Counter,
    
    // Tool metrics
    pub tool_calls: Counter,
    pub tool_duration: Histogram,
    pub tool_errors: Counter,
    
    // Resource metrics
    pub memory_usage: Gauge,
    pub gpu_utilization: Gauge,
    pub connection_pool: Gauge,
}
```

### 15.2 Tracing

Distributed tracing with OpenTelemetry:

```rust
#[tracing::instrument(skip(self))]
pub async fn handle_request(&self, req: Request) -> Result<Response> {
    // Automatic span creation
}
```

### 15.3 Health Checks

```
GET /admin/health

Response:
{
  "status": "healthy",
  "version": "0.1.0",
  "uptime_seconds": 3600,
  "providers": {
    "vertex_ai": "healthy",
    "openai": "healthy",
    "ollama": "degraded"
  },
  "mcp_servers": {
    "brave-search": "healthy",
    "github": "healthy"
  },
  "database": "healthy",
  "gpu": {
    "available": true,
    "utilization": 45.2,
    "memory_used_gb": 8.5
  }
}
```

---

## 16. Next Steps

### Phase 1: Core Infrastructure (Weeks 1-4)
1. ✅ Project setup with Cargo workspace
2. ✅ OpenAI API types and validation
3. ✅ Authentication middleware
4. ✅ Vertex AI provider implementation
5. 🔄 Streaming support
6. 🔄 Tool calling foundation

### Phase 2: Multi-Provider (Weeks 5-8)
1. Bedrock provider
2. OpenAI provider
3. Anthropic Direct provider
4. Provider abstraction trait
5. Model registry and routing
6. Configuration management

### Phase 3: MCP Integration (Weeks 9-12)
1. MCP client implementation
2. Microsandbox integration
3. Tool registry and routing
4. MCP server implementation
5. Tool execution monitoring

### Phase 4: Webhooks & Observability (Weeks 13-16)
1. Webhook delivery system
2. Supabase integration for storage
3. Metrics and tracing
4. Health checks and status
5. Admin API

### Phase 5: Local Models (Weeks 17-20)
1. HuggingFace downloader
2. Candle integration
3. GPU detection and utilization
4. Model caching
5. Batch processing

### Phase 6: Advanced Features (Weeks 21+)
1. Knowledge base and memory
2. Additional providers (Groq, OpenRouter, Ollama)
3. Performance optimization
4. Agent server (future)

---

## Appendix A: Glossary

- **MCP**: Model Context Protocol - standardized protocol for tool/resource access
- **Microsandbox**: Lightweight VM for secure code execution
- **GGUF**: GPT-Generated Unified Format - model file format
- **Candle**: Rust ML framework by HuggingFace
- **SSE**: Server-Sent Events - streaming protocol
- **A2A**: Agent-to-Agent communication protocol

## Appendix B: References

- [OpenAI API Documentation](https://platform.openai.com/docs/api-reference)
- [Anthropic API Documentation](https://docs.anthropic.com/)
- [Model Context Protocol](https://modelcontextprotocol.io/)
- [Supabase Documentation](https://supabase.com/docs)
- [Candle Framework](https://github.com/huggingface/candle)
- [Vertex AI Documentation](https://cloud.google.com/vertex-ai/docs)

---

**Document Status:** Draft for Implementation  
**Next Review:** After Phase 1 completion  
**Approval Required:** Architecture team, Security team
