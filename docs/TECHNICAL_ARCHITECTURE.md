# Technical Architecture: Universal AI Server Proxy

**Version:** 2.0  
**Date:** October 2, 2025  
**Project:** llm-supabase-rs

## Table of Contents

1. [System Overview](#system-overview)
2. [Technology Stack](#technology-stack)
3. [Architecture Layers](#architecture-layers)
4. [Module Design](#module-design)
5. [Data Flow](#data-flow)
6. [Provider Integration](#provider-integration)
7. [MCP Architecture](#mcp-architecture)
8. [Webhook System](#webhook-system)
9. [Database Schema](#database-schema)
10. [Performance Architecture](#performance-architecture)
11. [Security Architecture](#security-architecture)
12. [Deployment Architecture](#deployment-architecture)

---

## 1. System Overview

### 1.1 High-Level Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                        Client Applications                        │
│          (Web, Mobile, CLI, IDEs, Other AI Systems)              │
└────────────────────────┬────────────────────────────────────────┘
                         │ OpenAI API Format
                         │ HTTP/HTTPS + SSE
                         ▼
┌─────────────────────────────────────────────────────────────────┐
│                    Universal AI Proxy Server                     │
│  ┌─────────────────────────────────────────────────────────┐   │
│  │                   API Gateway Layer                      │   │
│  │  • Request Validation  • Auth  • Rate Limiting           │   │
│  └──────────────────────┬──────────────────────────────────┘   │
│                         │                                        │
│  ┌──────────────────────▼──────────────────────────────────┐   │
│  │              Request Processing Pipeline                 │   │
│  │  • Protocol Conversion  • Tool Detection  • Routing      │   │
│  └──────────────────────┬──────────────────────────────────┘   │
│                         │                                        │
│  ┌──────────────────────┼──────────────────────────────────┐   │
│  │              Provider Abstraction Layer                  │   │
│  │  ┌─────────┐ ┌─────────┐ ┌─────────┐ ┌──────────┐      │   │
│  │  │ Vertex  │ │ Bedrock │ │ OpenAI  │ │ Anthropic│ ...  │   │
│  │  └─────────┘ └─────────┘ └─────────┘ └──────────┘      │   │
│  └──────────────────────┬──────────────────────────────────┘   │
│                         │                                        │
│  ┌──────────────────────▼──────────────────────────────────┐   │
│  │                Tool Orchestration Layer                  │   │
│  │  • MCP Client  • Tool Registry  • Tool Execution         │   │
│  └──────────────────────┬──────────────────────────────────┘   │
│                         │                                        │
│  ┌──────────────────────▼──────────────────────────────────┐   │
│  │              Webhook & Observability Layer               │   │
│  │  • Event Publishing  • Metrics  • Tracing                │   │
│  └──────────────────────────────────────────────────────────┘   │
└───────────┬──────────────────────┬──────────────────────────────┘
            │                      │
            │                      │
┌───────────▼──────────┐  ┌────────▼────────────────────────────┐
│   External Services  │  │        MCP Servers (Microsandbox)   │
│  • Cloud Providers   │  │  ┌──────┐ ┌──────┐ ┌──────┐        │
│  • API Services      │  │  │Search│ │GitHub│ │Files │  ...   │
│  • Supabase          │  │  └──────┘ └──────┘ └──────┘        │
└──────────────────────┘  └─────────────────────────────────────┘
```

---

## 2. Technology Stack

### 2.1 Core Dependencies

```toml
[dependencies]
# Web Framework
axum = "0.7"
tokio = { version = "1.0", features = ["full"] }
tower = "0.4"
tower-http = "0.5"

# Serialization
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"

# HTTP Client
reqwest = { version = "0.11", features = ["json", "stream"] }

# Authentication
jsonwebtoken = "9.2"

# Cloud SDKs
gcp_auth = "0.10"
aws-config = "1.0"
aws-sdk-bedrockruntime = "1.0"

# ML/AI
candle-core = "0.9"
candle-nn = "0.9"
candle-transformers = "0.9"
hf-hub = "0.4"
tokenizers = "0.22"

# Database
sqlx = { version = "0.7", features = ["postgres", "runtime-tokio-rustls"] }

# Observability
tracing = "0.1"
tracing-subscriber = "0.3"
prometheus = "0.13"

# Utilities
uuid = { version = "1.6", features = ["v4"] }
chrono = "0.4"
dashmap = "5.5"
```

---

## 3. Module Design

### 3.1 Project Structure

```
src/
├── main.rs                      # Application entry point
├── lib.rs                       # Library root
├── app.rs                       # Application setup
│
├── api/                         # Presentation layer
│   ├── mod.rs
│   ├── routes.rs                # Route definitions
│   ├── handlers/                # Request handlers
│   │   ├── chat.rs              # /v1/chat/completions
│   │   ├── embeddings.rs        # /v1/embeddings
│   │   ├── models.rs            # /v1/models
│   │   └── admin.rs             # /admin/*
│   ├── middleware/              # HTTP middleware
│   │   ├── auth.rs
│   │   ├── rate_limit.rs
│   │   └── tracing.rs
│   └── streaming.rs             # SSE streaming
│
├── domain/                      # Domain layer
│   ├── mod.rs
│   ├── providers/               # Provider abstractions
│   │   ├── traits.rs            # Provider trait
│   │   ├── registry.rs          # Provider registry
│   │   └── selector.rs          # Selection logic
│   ├── tools/                   # Tool abstractions
│   │   ├── registry.rs          # Tool registry
│   │   └── executor.rs          # Tool execution
│   └── models/                  # Business models
│       ├── chat.rs
│       ├── embedding.rs
│       └── webhook.rs
│
├── infrastructure/              # Infrastructure layer
│   ├── providers/               # Provider implementations
│   │   ├── vertex/
│   │   ├── bedrock/
│   │   ├── openai/
│   │   ├── anthropic/
│   │   └── local/               # Candle-based
│   ├── mcp/                     # MCP integration
│   │   ├── client.rs
│   │   ├── server.rs
│   │   └── sandbox.rs
│   ├── database/                # Database layer
│   │   └── supabase.rs
│   └── webhooks/                # Webhook system
│       ├── dispatcher.rs
│       └── delivery.rs
│
├── config/                      # Configuration
│   ├── app.rs
│   ├── providers.rs
│   └── mcp.rs
│
└── shared/                      # Shared utilities
    ├── error.rs
    ├── types.rs
    └── metrics.rs
```

### 3.2 Core Trait Definitions

```rust
// domain/providers/traits.rs

#[async_trait]
pub trait LLMProvider: Send + Sync {
    /// Provider identifier
    fn name(&self) -> &str;
    
    /// List available models
    async fn list_models(&self) -> Result<Vec<ModelInfo>>;
    
    /// Convert OpenAI request to native format
    fn convert_request(
        &self, 
        req: &OpenAIRequest
    ) -> Result<ProviderRequest>;
    
    /// Execute request
    async fn execute(
        &self,
        req: ProviderRequest
    ) -> Result<ProviderResponse>;
    
    /// Execute streaming request
    async fn execute_stream(
        &self,
        req: ProviderRequest
    ) -> Result<impl Stream<Item = Result<ProviderChunk>>>;
    
    /// Convert native response to OpenAI format
    fn convert_response(
        &self,
        res: ProviderResponse
    ) -> Result<OpenAIResponse>;
    
    /// Check capability support
    fn supports(&self, capability: Capability) -> bool;
}

#[async_trait]
pub trait EmbeddingProvider: Send + Sync {
    /// Provider identifier
    fn name(&self) -> &str;
    
    /// Generate embeddings
    async fn generate_embeddings(
        &self,
        texts: Vec<String>,
        model: Option<String>
    ) -> Result<Vec<Vec<f32>>>;
    
    /// Get embedding dimensions
    fn embedding_dimensions(&self, model: &str) -> usize;
}

// domain/tools/traits.rs

#[async_trait]
pub trait ToolExecutor: Send + Sync {
    /// Execute tool call
    async fn execute(
        &self,
        tool_name: &str,
        arguments: serde_json::Value
    ) -> Result<ToolResult>;
    
    /// Check if tool is available
    fn has_tool(&self, tool_name: &str) -> bool;
    
    /// List available tools
    fn list_tools(&self) -> Vec<ToolDefinition>;
}
```

---

## 4. Data Flow

### 4.1 Chat Completion Flow

```
Client Request
    ↓
[1] HTTP Handler (api/handlers/chat.rs)
    ↓
[2] Authentication Middleware
    ↓ (AuthContext)
[3] Request Validation
    ↓ (OpenAIChatRequest)
[4] Webhook: ProcessingStart
    ↓
[5] Provider Selection (domain/providers/selector.rs)
    ↓ (ProviderName)
[6] Tool Detection
    ↓ (Vec<ToolDefinition>)
[7] Request Conversion
    ↓ (ProviderRequest)
[8] Provider Execution
    │
    ├─ If streaming:
    │   ↓
    │   [9a] Stream Response
    │   ↓
    │   [10a] SSE to Client
    │
    └─ If non-streaming:
        ↓
        [9b] Wait for completion
        ↓
        [10b] Tool Resolution Loop
        │   ↓
        │   [11] Execute Tools (MCP/External)
        │   ↓
        │   [12] Webhook: ToolCall events
        │   ↓
        │   [13] Continue LLM with tool results
        ↓
[14] Response Conversion
    ↓ (OpenAIResponse)
[15] Webhook: ProcessingEnd
    ↓
[16] HTTP Response to Client
```

### 4.2 Tool Execution Flow

```
Tool Call Detected
    ↓
[1] Webhook: ToolCallStart
    ↓
[2] Tool Registry Lookup
    ↓
    ├─ If MCP Tool:
    │   ↓
    │   [3a] Route to MCP Server
    │   ↓
    │   [4a] Execute in Microsandbox
    │   ↓
    │   [5a] Collect Result
    │
    ├─ If External Tool:
    │   ↓
    │   [3b] Webhook: ToolResolution
    │   ↓
    │   [4b] Wait for HTTP Response
    │   ↓
    │   [5b] Parse Result
    │
    └─ If Unknown:
        ↓
        [3c] Pass through to caller
        ↓
        [4c] Error or skip
    ↓
[6] Webhook: ToolCallEnd
    ↓
[7] Return result to LLM
```

### 4.3 Sequence Diagram

```
Client    API     Auth    Provider  MCP      Webhook   Supabase
  │        │       │       Selector  Client   Dispatcher │
  │        │       │          │        │         │        │
  ├─POST───>       │          │        │         │        │
  │        ├──────>│          │        │         │        │
  │        │<──────┤          │        │         │        │
  │        │  AuthContext     │        │         │        │
  │        ├─────────────────>│        │         │        │
  │        │     Select       │        │         │        │
  │        │<─────────────────┤        │         │        │
  │        │                  │        │         │        │
  │        ├──────────────────────────────────>  │        │
  │        │           ProcessingStart Event     │        │
  │        │                  │        │<────────┤        │
  │        │                  │        │         ├──────> │
  │        │                  │        │         │  Store │
  │        │                  │        │         │<────── │
  │        │                  │        │         │        │
  │        ├──────────────────>        │         │        │
  │        │    Execute Request        │         │        │
  │        │<──────────────────┤       │         │        │
  │        │   Tool Call Detected      │         │        │
  │        ├────────────────────────── >         │        │
  │        │         Execute Tool      │         │        │
  │        │<──────────────────────────┤         │        │
  │        │         Tool Result       │         │        │
  │<───────┤                           │         │        │
 Response  │                           │         │        │
```

---

## 5. Provider Integration

### 5.1 Provider Architecture

```rust
// infrastructure/providers/mod.rs

pub struct ProviderRegistry {
    providers: DashMap<String, Arc<dyn LLMProvider>>,
    embeddings: DashMap<String, Arc<dyn EmbeddingProvider>>,
}

impl ProviderRegistry {
    pub async fn initialize(config: &Config) -> Result<Self> {
        let mut registry = Self {
            providers: DashMap::new(),
            embeddings: DashMap::new(),
        };
        
        // Initialize enabled providers
        if config.vertex_ai.enabled {
            let provider = VertexAIProvider::new(&config.vertex_ai).await?;
            registry.providers.insert(
                "vertex-ai".to_string(),
                Arc::new(provider)
            );
        }
        
        if config.bedrock.enabled {
            let provider = BedrockProvider::new(&config.bedrock).await?;
            registry.providers.insert(
                "bedrock".to_string(),
                Arc::new(provider)
            );
        }
        
        // ... more providers
        
        Ok(registry)
    }
    
    pub fn get_provider(&self, name: &str) -> Option<Arc<dyn LLMProvider>> {
        self.providers.get(name).map(|p| Arc::clone(&p))
    }
}
```

### 5.2 Vertex AI Provider

```rust
// infrastructure/providers/vertex/client.rs

pub struct VertexAIProvider {
    client: reqwest::Client,
    project_id: String,
    location: String,
    authenticator: Arc<gcp_auth::TokenProvider>,
}

impl VertexAIProvider {
    pub async fn new(config: &VertexAIConfig) -> Result<Self> {
        let authenticator = gcp_auth::provider().await?;
        
        Ok(Self {
            client: reqwest::Client::new(),
            project_id: config.project_id.clone(),
            location: config.location.clone(),
            authenticator: Arc::new(authenticator),
        })
    }
    
    async fn get_token(&self) -> Result<String> {
        let token = self.authenticator
            .token(&["https://www.googleapis.com/auth/cloud-platform"])
            .await?;
        Ok(token.token().to_string())
    }
    
    fn endpoint_url(&self, model: &str) -> String {
        format!(
            "https://{}-aiplatform.googleapis.com/v1/projects/{}/locations/{}/publishers/anthropic/models/{}:streamRawPredict",
            self.location, self.project_id, self.location, model
        )
    }
}

#[async_trait]
impl LLMProvider for VertexAIProvider {
    fn name(&self) -> &str {
        "vertex-ai"
    }
    
    async fn execute(&self, req: ProviderRequest) -> Result<ProviderResponse> {
        let token = self.get_token().await?;
        
        let response = self.client
            .post(&self.endpoint_url(&req.model))
            .bearer_auth(token)
            .json(&req.body)
            .send()
            .await?;
            
        // Parse and convert response
        let body = response.json::<VertexAIResponse>().await?;
        Ok(ProviderResponse::from_vertex(body))
    }
    
    // ... other trait methods
}
```

### 5.3 Local Model Provider (Candle)

```rust
// infrastructure/providers/local/candle_engine.rs

pub struct CandleProvider {
    models: Arc<RwLock<HashMap<String, LoadedModel>>>,
    device: Device,
    model_path: PathBuf,
}

struct LoadedModel {
    model: Arc<dyn CandleModel>,
    tokenizer: Arc<Tokenizer>,
    config: ModelConfig,
    last_used: Instant,
}

impl CandleProvider {
    pub async fn new(config: &LocalConfig) -> Result<Self> {
        let device = Self::select_device()?;
        
        Ok(Self {
            models: Arc::new(RwLock::new(HashMap::new())),
            device,
            model_path: PathBuf::from(&config.model_path),
        })
    }
    
    fn select_device() -> Result<Device> {
        if candle_core::utils::cuda_is_available() {
            Ok(Device::new_cuda(0)?)
        } else if candle_core::utils::metal_is_available() {
            Ok(Device::new_metal(0)?)
        } else {
            Ok(Device::Cpu)
        }
    }
    
    async fn load_model(&self, model_name: &str) -> Result<Arc<LoadedModel>> {
        // Check cache
        {
            let models = self.models.read().await;
            if let Some(model) = models.get(model_name) {
                return Ok(Arc::new(model.clone()));
            }
        }
        
        // Load model
        let model_dir = self.model_path.join(model_name);
        let model = self.load_from_disk(&model_dir).await?;
        
        // Cache it
        let mut models = self.models.write().await;
        models.insert(model_name.to_string(), model.clone());
        
        Ok(Arc::new(model))
    }
    
    async fn generate(
        &self,
        model: &LoadedModel,
        prompt: &str,
        params: &GenerationParams
    ) -> Result<String> {
        // Tokenize
        let tokens = model.tokenizer
            .encode(prompt, false)
            .map_err(|e| anyhow::anyhow!("Tokenization error: {}", e))?;
            
        // Generate
        let mut output_tokens = Vec::new();
        let mut logits = model.model.forward(&tokens.get_ids())?;
        
        for _ in 0..params.max_tokens {
            let next_token = self.sample(&logits, params)?;
            output_tokens.push(next_token);
            
            if model.config.eos_token_id == Some(next_token) {
                break;
            }
            
            logits = model.model.forward(&[next_token])?;
        }
        
        // Decode
        let text = model.tokenizer
            .decode(&output_tokens, true)
            .map_err(|e| anyhow::anyhow!("Decoding error: {}", e))?;
            
        Ok(text)
    }
}
```

---

## 6. MCP Architecture

### 6.1 MCP Client Implementation

```rust
// infrastructure/mcp/client.rs

pub struct MCPClient {
    servers: Arc<RwLock<HashMap<String, MCPServerHandle>>>,
    sandbox_runtime: Arc<MicrosandboxRuntime>,
    config: MCPConfig,
}

struct MCPServerHandle {
    name: String,
    process: Arc<Mutex<Child>>,
    transport: Arc<MCPTransport>,
    tools: Vec<ToolDefinition>,
    status: ServerStatus,
}

impl MCPClient {
    pub async fn new(config: MCPConfig) -> Result<Self> {
        Ok(Self {
            servers: Arc::new(RwLock::new(HashMap::new())),
            sandbox_runtime: Arc::new(MicrosandboxRuntime::new()?),
            config,
        })
    }
    
    pub async fn initialize_servers(&self) -> Result<()> {
        for (name, server_config) in &self.config.servers {
            if !server_config.enabled {
                continue;
            }
            
            let handle = self.launch_server(name, server_config).await?;
            
            let mut servers = self.servers.write().await;
            servers.insert(name.clone(), handle);
        }
        
        Ok(())
    }
    
    async fn launch_server(
        &self,
        name: &str,
        config: &MCPServerConfig
    ) -> Result<MCPServerHandle> {
        // Create sandbox configuration
        let sandbox_config = SandboxConfig {
            cpu_limit: 0.5,
            memory_limit_mb: 512,
            timeout_ms: 30000,
            network_enabled: config.network_enabled.unwrap_or(false),
            allowed_hosts: config.allowed_hosts.clone().unwrap_or_default(),
            env: config.env.clone(),
        };
        
        // Launch in sandbox
        let process = self.sandbox_runtime
            .spawn(&config.command, &config.args, sandbox_config)
            .await?;
            
        // Create transport
        let transport = MCPTransport::new_stdio(
            process.stdin.take().unwrap(),
            process.stdout.take().unwrap()
        );
        
        // Initialize MCP protocol
        transport.send_initialize().await?;
        let init_response = transport.receive_initialize().await?;
        
        // List tools
        let tools = transport.list_tools().await?;
        
        Ok(MCPServerHandle {
            name: name.to_string(),
            process: Arc::new(Mutex::new(process)),
            transport: Arc::new(transport),
            tools,
            status: ServerStatus::Running,
        })
    }
    
    pub async fn execute_tool(
        &self,
        server_name: &str,
        tool_name: &str,
        arguments: serde_json::Value
    ) -> Result<ToolResult> {
        let servers = self.servers.read().await;
        let handle = servers.get(server_name)
            .ok_or_else(|| anyhow::anyhow!("Server not found: {}", server_name))?;
            
        // Send tool call request
        let request = MCPToolCallRequest {
            method: "tools/call".to_string(),
            params: MCPToolCallParams {
                name: tool_name.to_string(),
                arguments,
            },
        };
        
        let response = handle.transport
            .send_request(request)
            .await?;
            
        Ok(response.into())
    }
}
```

### 6.2 MCP Server Implementation

```rust
// infrastructure/mcp/server.rs

pub struct MCPServer {
    tool_handlers: Arc<RwLock<HashMap<String, Box<dyn ToolHandler>>>>,
}

#[async_trait]
pub trait ToolHandler: Send + Sync {
    async fn execute(
        &self,
        arguments: serde_json::Value
    ) -> Result<serde_json::Value>;
}

impl MCPServer {
    pub fn new() -> Self {
        let mut handlers: HashMap<String, Box<dyn ToolHandler>> = HashMap::new();
        
        // Register built-in tools
        handlers.insert(
            "llm_completion".to_string(),
            Box::new(LLMCompletionHandler::new())
        );
        handlers.insert(
            "generate_embedding".to_string(),
            Box::new(EmbeddingHandler::new())
        );
        handlers.insert(
            "list_available_models".to_string(),
            Box::new(ListModelsHandler::new())
        );
        
        Self {
            tool_handlers: Arc::new(RwLock::new(handlers)),
        }
    }
    
    pub async fn handle_request(
        &self,
        request: MCPRequest
    ) -> Result<MCPResponse> {
        match request.method.as_str() {
            "initialize" => self.handle_initialize(request).await,
            "tools/list" => self.handle_list_tools(request).await,
            "tools/call" => self.handle_tool_call(request).await,
            _ => Err(anyhow::anyhow!("Unknown method: {}", request.method)),
        }
    }
    
    async fn handle_tool_call(&self, request: MCPRequest) -> Result<MCPResponse> {
        let params: MCPToolCallParams = serde_json::from_value(request.params)?;
        
        let handlers = self.tool_handlers.read().await;
        let handler = handlers.get(&params.name)
            .ok_or_else(|| anyhow::anyhow!("Tool not found: {}", params.name))?;
            
        let result = handler.execute(params.arguments).await?;
        
        Ok(MCPResponse {
            jsonrpc: "2.0".to_string(),
            id: request.id,
            result: Some(result),
            error: None,
        })
    }
}

// Example tool handler
struct LLMCompletionHandler {
    provider_registry: Arc<ProviderRegistry>,
}

#[async_trait]
impl ToolHandler for LLMCompletionHandler {
    async fn execute(
        &self,
        arguments: serde_json::Value
    ) -> Result<serde_json::Value> {
        let args: LLMCompletionArgs = serde_json::from_value(arguments)?;
        
        // Select provider
        let provider = self.provider_registry
            .get_provider(&args.model.unwrap_or_else(|| "claude-sonnet-4.5".to_string()))
            .ok_or_else(|| anyhow::anyhow!("Provider not found"))?;
            
        // Execute
        let request = ProviderRequest {
            model: args.model.clone(),
            messages: vec![Message {
                role: "user".to_string(),
                content: args.prompt,
            }],
            max_tokens: args.max_tokens,
            temperature: args.temperature,
            ..Default::default()
        };
        
        let response = provider.execute(request).await?;
        
        Ok(serde_json::to_value(response)?)
    }
}
```

### 6.3 Microsandbox Integration

```rust
// infrastructure/mcp/sandbox.rs

pub struct MicrosandboxRuntime {
    // Platform-specific implementation
}

pub struct SandboxConfig {
    pub cpu_limit: f64,
    pub memory_limit_mb: usize,
    pub timeout_ms: u64,
    pub network_enabled: bool,
    pub allowed_hosts: Vec<String>,
    pub env: HashMap<String, String>,
}

impl MicrosandboxRuntime {
    pub fn new() -> Result<Self> {
        // Initialize sandbox runtime
        Ok(Self {})
    }
    
    pub async fn spawn(
        &self,
        command: &str,
        args: &[String],
        config: SandboxConfig
    ) -> Result<Child> {
        use std::process::{Command, Stdio};
        
        // Create sandbox wrapper command
        let mut cmd = Command::new("microsandbox");
        cmd.arg("run")
            .arg("--cpu-limit").arg(config.cpu_limit.to_string())
            .arg("--memory-limit").arg(format!("{}M", config.memory_limit_mb))
            .arg("--timeout").arg(config.timeout_ms.to_string());
            
        if !config.network_enabled {
            cmd.arg("--no-network");
        } else if !config.allowed_hosts.is_empty() {
            cmd.arg("--allowed-hosts")
                .arg(config.allowed_hosts.join(","));
        }
        
        // Add environment variables
        for (key, value) in &config.env {
            cmd.env(key, value);
        }
        
        // Add actual command to execute
        cmd.arg("--").arg(command).args(args);
        
        // Configure stdio
        cmd.stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
            
        // Spawn process
        let child = cmd.spawn()?;
        
        Ok(child)
    }
}
```

---

## 7. Webhook System

### 7.1 Webhook Architecture

```rust
// infrastructure/webhooks/dispatcher.rs

pub struct WebhookDispatcher {
    delivery: Arc<WebhookDelivery>,
    registry: Arc<WebhookRegistry>,
    event_queue: Arc<tokio::sync::mpsc::UnboundedSender<WebhookEvent>>,
}

impl WebhookDispatcher {
    pub fn new(registry: Arc<WebhookRegistry>) -> Self {
        let delivery = Arc::new(WebhookDelivery::new());
        let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
        
        let dispatcher = Self {
            delivery: Arc::clone(&delivery),
            registry: Arc::clone(&registry),
            event_queue: Arc::new(tx),
        };
        
        // Spawn background worker
        tokio::spawn(Self::process_events(
            rx,
            Arc::clone(&delivery),
            Arc::clone(&registry)
        ));
        
        dispatcher
    }
    
    pub fn publish(&self, event: WebhookEvent) -> Result<()> {
        self.event_queue.send(event)?;
        Ok(())
    }
    
    async fn process_events(
        mut rx: tokio::sync::mpsc::UnboundedReceiver<WebhookEvent>,
        delivery: Arc<WebhookDelivery>,
        registry: Arc<WebhookRegistry>
    ) {
        while let Some(event) = rx.recv().await {
            let webhooks = registry.find_webhooks_for_event(&event).await;
            
            for webhook in webhooks {
                let delivery = Arc::clone(&delivery);
                let event = event.clone();
                let webhook = webhook.clone();
                
                // Deliver asynchronously
                tokio::spawn(async move {
                    if let Err(e) = delivery.deliver(&webhook, &event).await {
                        tracing::error!(
                            "Webhook delivery failed: {} - {}",
                            webhook.endpoint,
                            e
                        );
                    }
                });
            }
        }
    }
}
```

### 7.2 Webhook Delivery

```rust
// infrastructure/webhooks/delivery.rs

pub struct WebhookDelivery {
    client: reqwest::Client,
    max_retries: usize,
    retry_delay_ms: u64,
    timeout_ms: u64,
}

impl WebhookDelivery {
    pub fn new() -> Self {
        Self {
            client: reqwest::Client::builder()
                .timeout(Duration::from_secs(10))
                .build()
                .unwrap(),
            max_retries: 3,
            retry_delay_ms: 1000,
            timeout_ms: 10000,
        }
    }
    
    pub async fn deliver(
        &self,
        webhook: &RegisteredWebhook,
        event: &WebhookEvent
    ) -> Result<()> {
        let mut attempt = 0;
        let mut delay = self.retry_delay_ms;
        
        loop {
            attempt += 1;
            
            match self.try_deliver(webhook, event).await {
                Ok(_) => {
                    tracing::info!(
                        "Webhook delivered successfully: {} (attempt {})",
                        webhook.endpoint,
                        attempt
                    );
                    return Ok(());
                }
                Err(e) if attempt < self.max_retries => {
                    tracing::warn!(
                        "Webhook delivery failed (attempt {}): {} - {}",
                        attempt,
                        webhook.endpoint,
                        e
                    );
                    tokio::time::sleep(Duration::from_millis(delay)).await;
                    delay *= 2; // Exponential backoff
                }
                Err(e) => {
                    tracing::error!(
                        "Webhook delivery failed after {} attempts: {} - {}",
                        attempt,
                        webhook.endpoint,
                        e
                    );
                    // Log to dead letter queue
                    self.log_to_dlq(webhook, event, &e).await?;
                    return Err(e);
                }
            }
        }
    }
    
    async fn try_deliver(
        &self,
        webhook: &RegisteredWebhook,
        event: &WebhookEvent
    ) -> Result<()> {
        let payload = serde_json::to_value(event)?;
        
        let mut request = self.client
            .post(&webhook.endpoint)
            .json(&payload)
            .header("Content-Type", "application/json")
            .header("X-Webhook-Event", event.event_type())
            .header("X-Request-ID", event.request_id().to_string());
            
        if let Some(auth) = &webhook.auth_header {
            request = request.header("Authorization", auth);
        }
        
        // Generate HMAC signature if secret is configured
        if let Some(secret) = &webhook.secret {
            let signature = self.generate_signature(&payload, secret)?;
            request = request.header("X-Webhook-Signature", signature);
        }
        
        let response = request.send().await?;
        
        if !response.status().is_success() {
            return Err(anyhow::anyhow!(
                "Webhook returned non-success status: {}",
                response.status()
            ));
        }
        
        Ok(())
    }
    
    fn generate_signature(
        &self,
        payload: &serde_json::Value,
        secret: &str
    ) -> Result<String> {
        use hmac::{Hmac, Mac};
        use sha2::Sha256;
        
        type HmacSha256 = Hmac<Sha256>;
        
        let payload_bytes = serde_json::to_vec(payload)?;
        let mut mac = HmacSha256::new_from_slice(secret.as_bytes())
            .map_err(|e| anyhow::anyhow!("HMAC error: {}", e))?;
        mac.update(&payload_bytes);
        
        let result = mac.finalize();
        Ok(hex::encode(result.into_bytes()))
    }
}
```

---

## 8. Database Schema

### 8.1 Supabase Tables

```sql
-- Webhooks
CREATE TABLE webhooks (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID REFERENCES auth.users(id),
    events TEXT[] NOT NULL,
    endpoint TEXT NOT NULL,
    auth_header TEXT,
    secret TEXT,  -- For HMAC signing
    enabled BOOLEAN DEFAULT true,
    metadata JSONB DEFAULT '{}'::jsonb,
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW()
);

CREATE INDEX idx_webhooks_user_enabled ON webhooks(user_id, enabled);
CREATE INDEX idx_webhooks_events ON webhooks USING GIN(events);

-- Request logs
CREATE TABLE request_logs (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    request_id UUID NOT NULL,
    user_id UUID REFERENCES auth.users(id),
    model TEXT NOT NULL,
    provider TEXT NOT NULL,
    endpoint TEXT NOT NULL,
    prompt_tokens INTEGER,
    completion_tokens INTEGER,
    total_tokens INTEGER,
    cached_tokens INTEGER DEFAULT 0,
    duration_ms INTEGER,
    status TEXT NOT NULL,  -- 'success', 'error', 'partial'
    error_message TEXT,
    metadata JSONB DEFAULT '{}'::jsonb,
    created_at TIMESTAMPTZ DEFAULT NOW()
);

CREATE INDEX idx_request_logs_user ON request_logs(user_id, created_at DESC);
CREATE INDEX idx_request_logs_request_id ON request_logs(request_id);
CREATE INDEX idx_request_logs_model ON request_logs(model, created_at DESC);

-- Tool calls
CREATE TABLE tool_calls (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    request_id UUID NOT NULL,
    tool_id TEXT NOT NULL,
    tool_name TEXT NOT NULL,
    server_name TEXT,  -- MCP server name if applicable
    arguments JSONB NOT NULL,
    result JSONB,
    duration_ms INTEGER,
    success BOOLEAN,
    error_message TEXT,
    created_at TIMESTAMPTZ DEFAULT NOW()
);

CREATE INDEX idx_tool_calls_request ON tool_calls(request_id);
CREATE INDEX idx_tool_calls_tool ON tool_calls(tool_name, created_at DESC);

-- Memory/Knowledge base
CREATE EXTENSION IF NOT EXISTS vector;

CREATE TABLE memories (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID REFERENCES auth.users(id),
    scope TEXT NOT NULL,  -- 'global', 'user', 'session'
    session_id TEXT,
    content TEXT NOT NULL,
    embedding vector(1536),
    metadata JSONB DEFAULT '{}'::jsonb,
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW()
);

CREATE INDEX idx_memories_embedding ON memories 
    USING ivfflat (embedding vector_cosine_ops)
    WITH (lists = 100);
CREATE INDEX idx_memories_user_scope ON memories(user_id, scope);
CREATE INDEX idx_memories_session ON memories(session_id);

-- Audit logs
CREATE TABLE audit_logs (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    timestamp TIMESTAMPTZ DEFAULT NOW(),
    user_id UUID REFERENCES auth.users(id),
    action TEXT NOT NULL,
    resource TEXT,
    success BOOLEAN,
    ip_address INET,
    user_agent TEXT,
    metadata JSONB DEFAULT '{}'::jsonb
);

CREATE INDEX idx_audit_logs_timestamp ON audit_logs(timestamp DESC);
CREATE INDEX idx_audit_logs_user ON audit_logs(user_id, timestamp DESC);
CREATE INDEX idx_audit_logs_action ON audit_logs(action, timestamp DESC);

-- Webhook delivery logs
CREATE TABLE webhook_delivery_logs (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    webhook_id UUID REFERENCES webhooks(id),
    event_type TEXT NOT NULL,
    request_id UUID,
    attempt INTEGER NOT NULL,
    status TEXT NOT NULL,  -- 'success', 'failed', 'retrying'
    response_status INTEGER,
    error_message TEXT,
    duration_ms INTEGER,
    created_at TIMESTAMPTZ DEFAULT NOW()
);

CREATE INDEX idx_webhook_delivery_webhook ON webhook_delivery_logs(webhook_id, created_at DESC);
CREATE INDEX idx_webhook_delivery_request ON webhook_delivery_logs(request_id);
```

### 8.2 Database Migrations

```rust
// infrastructure/database/migrations/001_initial_schema.sql
// Place all CREATE TABLE statements here

// infrastructure/database/mod.rs
pub async fn run_migrations(pool: &PgPool) -> Result<()> {
    sqlx::migrate!("./infrastructure/database/migrations")
        .run(pool)
        .await?;
    Ok(())
}
```

---

## 9. Performance Architecture

### 9.1 Connection Pooling

```rust
// infrastructure/database/pool.rs

pub struct DatabasePool {
    pool: deadpool_postgres::Pool,
}

impl DatabasePool {
    pub async fn new(config: &DatabaseConfig) -> Result<Self> {
        let pool = deadpool_postgres::Config {
            host: Some(config.host.clone()),
            port: Some(config.port),
            dbname: Some(config.database.clone()),
            user: Some(config.user.clone()),
            password: Some(config.password.clone()),
            pool: Some(deadpool_postgres::PoolConfig {
                max_size: config.pool_size,
                timeouts: deadpool_postgres::Timeouts {
                    wait: Some(Duration::from_secs(5)),
                    create: Some(Duration::from_secs(5)),
                    recycle: Some(Duration::from_secs(5)),
                },
            }),
            ..Default::default()
        }
        .create_pool(Some(deadpool_postgres::Runtime::Tokio1), tokio_postgres::NoTls)?;
        
        Ok(Self { pool })
    }
    
    pub async fn get(&self) -> Result<deadpool_postgres::Object> {
        Ok(self.pool.get().await?)
    }
}
```

### 9.2 Caching Layer

```rust
// infrastructure/cache/mod.rs

pub struct CacheLayer {
    responses: Arc<DashMap<String, CachedResponse>>,
    embeddings: Arc<DashMap<String, CachedEmbedding>>,
    models: Arc<DashMap<String, CachedModelList>>,
    max_size_mb: usize,
}

struct CachedResponse {
    data: OpenAIResponse,
    cached_at: Instant,
    ttl: Duration,
}

impl CacheLayer {
    pub fn new(max_size_mb: usize) -> Self {
        Self {
            responses: Arc::new(DashMap::new()),
            embeddings: Arc::new(DashMap::new()),
            models: Arc::new(DashMap::new()),
            max_size_mb,
        }
    }
    
    pub fn get_response(&self, key: &str) -> Option<OpenAIResponse> {
        self.responses.get(key).and_then(|entry| {
            if entry.cached_at.elapsed() < entry.ttl {
                Some(entry.data.clone())
            } else {
                None
            }
        })
    }
    
    pub fn put_response(
        &self,
        key: String,
        response: OpenAIResponse,
        ttl: Duration
    ) {
        self.responses.insert(key, CachedResponse {
            data: response,
            cached_at: Instant::now(),
            ttl,
        });
        
        // TODO: Implement LRU eviction when size exceeds max_size_mb
    }
}
```

### 9.3 Rate Limiting

```rust
// api/middleware/rate_limit.rs

use tower::limit::RateLimit;
use std::time::Duration;

pub struct RateLimiter {
    limits: Arc<DashMap<String, TokenBucket>>,
}

struct TokenBucket {
    tokens: AtomicU64,
    last_refill: Instant,
    rate: u64,  // Tokens per second
    capacity: u64,
}

impl RateLimiter {
    pub fn new() -> Self {
        Self {
            limits: Arc::new(DashMap::new()),
        }
    }
    
    pub async fn check_limit(
        &self,
        user_id: &str,
        role: &Role
    ) -> Result<()> {
        let config = self.get_limit_config(role);
        
        let bucket = self.limits
            .entry(user_id.to_string())
            .or_insert_with(|| TokenBucket::new(config.rate, config.capacity));
            
        if !bucket.try_consume(1) {
            return Err(anyhow::anyhow!("Rate limit exceeded"));
        }
        
        Ok(())
    }
}
```

---

## 10. Security Architecture

### 10.1 Authentication Flow

```rust
// api/middleware/auth.rs

pub async fn auth_middleware(
    State(state): State<AppState>,
    mut request: Request,
    next: Next
) -> Result<Response, StatusCode> {
    // Extract Bearer token
    let token = extract_token(&request)?;
    
    // Validate JWT
    let claims = validate_jwt(&token, &state.config.supabase.jwt_secret)
        .map_err(|_| StatusCode::UNAUTHORIZED)?;
    
    // Build auth context
    let auth_context = AuthContext {
        user_id: claims.sub.parse().ok(),
        role: Role::from_str(&claims.role),
        email: claims.email,
        metadata: claims.user_metadata,
        permissions: Vec::new(),  // Load from config
    };
    
    // Check rate limit
    state.rate_limiter
        .check_limit(&auth_context.user_id.unwrap_or_default().to_string(), &auth_context.role)
        .await
        .map_err(|_| StatusCode::TOO_MANY_REQUESTS)?;
    
    // Insert into request extensions
    request.extensions_mut().insert(auth_context);
    
    Ok(next.run(request).await)
}

fn validate_jwt(token: &str, secret: &str) -> Result<Claims> {
    use jsonwebtoken::{decode, DecodingKey, Validation, Algorithm};
    
    let validation = Validation::new(Algorithm::HS256);
    let token_data = decode::<Claims>(
        token,
        &DecodingKey::from_secret(secret.as_bytes()),
        &validation
    )?;
    
    Ok(token_data.claims)
}
```

### 10.2 Input Validation

```rust
// shared/validation.rs

pub fn validate_chat_request(req: &OpenAIChatRequest) -> Result<()> {
    // Check message count
    if req.messages.is_empty() {
        return Err(anyhow::anyhow!("Messages cannot be empty"));
    }
    
    if req.messages.len() > 100 {
        return Err(anyhow::anyhow!("Too many messages (max: 100)"));
    }
    
    // Check token limits
    if let Some(max_tokens) = req.max_tokens {
        if max_tokens > 200000 {
            return Err(anyhow::anyhow!("max_tokens too large (max: 200000)"));
        }
    }
    
    // Check temperature
    if let Some(temp) = req.temperature {
        if !(0.0..=2.0).contains(&temp) {
            return Err(anyhow::anyhow!("temperature must be between 0 and 2"));
        }
    }
    
    // Validate message content
    for msg in &req.messages {
        if msg.content.is_empty() {
            return Err(anyhow::anyhow!("Message content cannot be empty"));
        }
        
        // Check for excessively long messages
        if msg.content.len() > 1_000_000 {
            return Err(anyhow::anyhow!("Message content too long"));
        }
    }
    
    Ok(())
}
```

---

## 11. Deployment Architecture

### 11.1 Container Configuration

```dockerfile
# Dockerfile
FROM rust:1.75 as builder

WORKDIR /app

# Copy manifests
COPY Cargo.toml Cargo.lock ./

# Build dependencies (cached layer)
RUN mkdir src && \
    echo "fn main() {}" > src/main.rs && \
    cargo build --release && \
    rm -rf src

# Copy source
COPY src ./src

# Build application
RUN cargo build --release

# Runtime stage
FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    && rm -rf /var/lib/apt/lists/*

COPY --from=builder /app/target/release/llm-supabase-rs /usr/local/bin/

EXPOSE 8080

CMD ["llm-supabase-rs"]
```

### 11.2 Kubernetes Deployment

```yaml
# k8s/deployment.yaml
apiVersion: apps/v1
kind: Deployment
metadata:
  name: llm-proxy
  labels:
    app: llm-proxy
spec:
  replicas: 3
  selector:
    matchLabels:
      app: llm-proxy
  template:
    metadata:
      labels:
        app: llm-proxy
    spec:
      containers:
      - name: llm-proxy
        image: llm-proxy:latest
        ports:
        - containerPort: 8080
        resources:
          requests:
            memory: "4Gi"
            cpu: "2"
            nvidia.com/gpu: "1"  # For local model support
          limits:
            memory: "16Gi"
            cpu: "8"
            nvidia.com/gpu: "1"
        env:
        - name: SUPABASE_URL
          valueFrom:
            secretKeyRef:
              name: llm-proxy-secrets
              key: supabase-url
        - name: SUPABASE_SERVICE_ROLE_KEY
          valueFrom:
            secretKeyRef:
              name: llm-proxy-secrets
              key: supabase-service-role-key
        - name: GCP_PROJECT_ID
          valueFrom:
            configMapKeyRef:
              name: llm-proxy-config
              key: gcp-project-id
        volumeMounts:
        - name: gcp-credentials
          mountPath: /etc/gcp
          readOnly: true
        - name: model-storage
          mountPath: /data/models
        livenessProbe:
          httpGet:
            path: /admin/health
            port: 8080
          initialDelaySeconds: 30
          periodSeconds: 10
        readinessProbe:
          httpGet:
            path: /admin/health
            port: 8080
          initialDelaySeconds: 5
          periodSeconds: 5
      volumes:
      - name: gcp-credentials
        secret:
          secretName: gcp-credentials
      - name: model-storage
        persistentVolumeClaim:
          claimName: model-storage-pvc
---
apiVersion: v1
kind: Service
metadata:
  name: llm-proxy
spec:
  selector:
    app: llm-proxy
  ports:
  - port: 80
    targetPort: 8080
  type: LoadBalancer
```

---

## 12. Monitoring & Observability

### 12.1 Metrics Collection

```rust
// shared/metrics.rs

use prometheus::{IntCounter, Histogram, Gauge, Registry};

pub struct Metrics {
    // Request metrics
    pub requests_total: IntCounter,
    pub requests_duration: Histogram,
    pub requests_active: Gauge,
    
    // Provider metrics
    pub provider_requests: IntCounter,
    pub provider_errors: IntCounter,
    pub provider_latency: Histogram,
    
    // Token metrics
    pub tokens_processed: IntCounter,
    pub cache_hits: IntCounter,
    
    // Resource metrics
    pub memory_usage: Gauge,
    pub gpu_utilization: Gauge,
}

impl Metrics {
    pub fn new(registry: &Registry) -> Result<Self> {
        let requests_total = IntCounter::new(
            "requests_total",
            "Total number of requests"
        )?;
        registry.register(Box::new(requests_total.clone()))?;
        
        // ... register other metrics
        
        Ok(Self {
            requests_total,
            // ... other metrics
        })
    }
}
```

### 12.2 Tracing

```rust
// main.rs

use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

fn init_tracing() {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "llm_supabase_rs=debug,tower_http=debug".into())
        )
        .with(tracing_subscriber::fmt::layer())
        .init();
}
```

---

## Appendix: Key Design Decisions

### A1. Why Rust?

- **Performance**: Native code, zero-cost abstractions
- **Safety**: Memory safety, thread safety
- **Async**: Excellent async/await support with Tokio
- **Ecosystem**: Rich ecosystem for ML (Candle), web (Axum), cloud SDKs

### A2. Why Axum?

- **Performance**: Built on Tokio and Hyper
- **Ergonomics**: Type-safe extractors, minimal boilerplate
- **Ecosystem**: Tower middleware, excellent for microservices

### A3. Why DashMap over Mutex<HashMap>?

- **Concurrency**: Lock-free reads, fine-grained locking
- **Performance**: Better scalability under high concurrency

### A4. Why Separate Provider Implementations?

- **Maintainability**: Each provider is isolated
- **Testability**: Mock individual providers
- **Flexibility**: Easy to add/remove providers

---

**Document Status:** Complete for Phase 1 Implementation  
**Next Steps:** Begin implementation following this architecture
