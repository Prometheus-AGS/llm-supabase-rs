# Multi-Provider Architecture

This document describes the new multi-provider architecture that makes it easy to support multiple AI providers (Vertex AI, Bedrock, OpenAI, etc.) with a unified interface.

## 🏗️ Architecture Overview

The multi-provider architecture is built around common traits that each provider implements:

```
┌─────────────────────────────────────────────────────────────┐
│                    OpenAI API Layer                        │
│                  (Unified Interface)                       │
└─────────────────────┬───────────────────────────────────────┘
                      │
┌─────────────────────▼───────────────────────────────────────┐
│                Common Traits                               │
│  • ProviderConverter                                       │
│  • StreamingConverter                                      │
│  • ProviderErrorHandler                                    │
└─────────────────────┬───────────────────────────────────────┘
                      │
        ┌─────────────┼─────────────┬─────────────┐
        │             │             │             │
┌───────▼──────┐ ┌────▼────┐ ┌──────▼──────┐ ┌───▼────┐
│ VertexAI     │ │ Bedrock │ │   OpenAI    │ │  Azure │
│ Converter    │ │Converter│ │  Converter  │ │Converter│
└──────────────┘ └─────────┘ └─────────────┘ └────────┘
```

## 🎯 Key Benefits

### **1. Unified Interface**
- All providers implement the same `ProviderConverter` trait
- Consistent error handling across providers
- Easy to switch between providers

### **2. Easy to Extend**
- Adding a new provider requires implementing just one trait
- Common functionality is shared (streaming, error handling, etc.)
- Provider-specific logic is isolated

### **3. Type Safety**
- Each provider defines its own request/response types
- Compile-time guarantees for format conversion
- No runtime type errors

### **4. Streaming Support**
- Common streaming interface for all providers
- Provider-specific streaming logic handled transparently
- Unified Server-Sent Events output

## 📋 Core Traits

### **ProviderConverter**
The main trait that all providers must implement:

```rust
#[async_trait]
pub trait ProviderConverter: Send + Sync {
    type ProviderRequest: Send + Sync;
    type ProviderResponse: Send + Sync;
    type ProviderStreamChunk: Send + Sync;
    type ProviderError: std::error::Error + Send + Sync;

    fn openai_to_provider_request(&self, request: &ChatCompletionRequest) -> Result<Self::ProviderRequest>;
    fn provider_to_openai_response(&self, response: &Self::ProviderResponse, ...) -> Result<ChatCompletionResponse>;
    fn provider_chunk_to_openai_chunk(&self, chunk: &Self::ProviderStreamChunk, ...) -> Result<Option<ChatCompletionChunk>>;
    
    fn provider_name(&self) -> &'static str;
    fn supported_models(&self) -> Vec<String>;
    fn normalize_model_name(&self, model: &str) -> String;
}
```

### **StreamingConverter**
For providers that support streaming:

```rust
#[async_trait]
pub trait StreamingConverter: ProviderConverter {
    type ProviderStream: Stream<Item = Result<Self::ProviderStreamChunk, Self::ProviderError>> + Send;

    async fn process_stream(&self, stream: Self::ProviderStream, ...) -> impl Stream<Item = Result<ChatCompletionChunk, anyhow::Error>> + Send;
}
```

### **ProviderErrorHandler**
For consistent error handling:

```rust
pub trait ProviderErrorHandler {
    type ProviderError: std::error::Error + Send + Sync;

    fn handle_provider_error(&self, error: Self::ProviderError) -> anyhow::Error;
    fn is_retryable_error(&self, error: &Self::ProviderError) -> bool;
    fn retry_delay(&self, attempt: u32) -> std::time::Duration;
}
```

## 🔧 Implementation Example: Vertex AI

Here's how the Vertex AI provider implements the traits:

```rust
pub struct VertexAIConverter {
    anthropic_version: String,
}

#[async_trait]
impl ProviderConverter for VertexAIConverter {
    type ProviderRequest = VertexPredictRequest;
    type ProviderResponse = VertexPredictResponse;
    type ProviderStreamChunk = VertexStreamChunk;
    type ProviderError = VertexError;

    fn openai_to_provider_request(&self, request: &ChatCompletionRequest) -> Result<Self::ProviderRequest> {
        // Convert OpenAI format to Vertex AI format
        let vertex_messages = self.convert_messages(&request.messages)?;
        Ok(VertexPredictRequest {
            anthropic_version: self.anthropic_version.clone(),
            messages: vertex_messages,
            max_tokens: request.max_tokens.unwrap_or(16384),
            temperature: request.temperature,
            // ... other fields
        })
    }

    fn provider_chunk_to_openai_chunk(&self, chunk: &Self::ProviderStreamChunk, ...) -> Result<Option<ChatCompletionChunk>> {
        match chunk.event_type.as_str() {
            "message" => {
                // Vertex AI specific: handle "message" event type
                if let Some(content) = chunk.get_content() {
                    self.create_content_chunk(content, request_id, model)
                } else {
                    Ok(None)
                }
            }
            "content_block_delta" => {
                // Standard Anthropic format
                // ... handle content delta
            }
            // ... other event types
        }
    }
}
```

## 🚀 Usage in Handlers

The chat handler can now use any provider through the common interface:

```rust
pub async fn chat_completions_streaming(
    State(state): State<AppState>,
    request: Request,
) -> Result<impl IntoResponse, AppError> {
    // Parse request
    let chat_request: ChatCompletionRequest = parse_request(request).await?;
    
    // Get the appropriate converter for the model
    let converter = state.get_converter_for_model(&chat_request.model)?;
    
    // Convert to provider format
    let provider_request = converter.openai_to_provider_streaming_request(&chat_request)?;
    
    // Make streaming call
    let provider_stream = state.client.predict_streaming(&chat_request.model, provider_request).await?;
    
    // Process stream using the converter
    let openai_stream = converter.process_stream(provider_stream, request_id, model).await;
    
    // Return SSE response
    Ok(create_sse_response(openai_stream))
}
```

## 📦 Adding New Providers

To add a new provider (e.g., AWS Bedrock):

### **1. Define Provider Types**
```rust
// src/infrastructure/bedrock/types.rs
pub struct BedrockRequest { /* ... */ }
pub struct BedrockResponse { /* ... */ }
pub struct BedrockStreamChunk { /* ... */ }
pub struct BedrockError { /* ... */ }
```

### **2. Implement ProviderConverter**
```rust
// src/infrastructure/bedrock/bedrock_converter.rs
pub struct BedrockConverter;

#[async_trait]
impl ProviderConverter for BedrockConverter {
    type ProviderRequest = BedrockRequest;
    type ProviderResponse = BedrockResponse;
    type ProviderStreamChunk = BedrockStreamChunk;
    type ProviderError = BedrockError;

    fn openai_to_provider_request(&self, request: &ChatCompletionRequest) -> Result<Self::ProviderRequest> {
        // Convert OpenAI format to Bedrock format
        // ...
    }

    fn provider_chunk_to_openai_chunk(&self, chunk: &Self::ProviderStreamChunk, ...) -> Result<Option<ChatCompletionChunk>> {
        // Convert Bedrock chunks to OpenAI format
        // Handle Bedrock-specific event types
        // ...
    }

    fn provider_name(&self) -> &'static str { "bedrock" }
    fn supported_models(&self) -> Vec<String> { /* ... */ }
}
```

### **3. Register Provider**
```rust
// In your app state or configuration
let bedrock_converter = BedrockConverter::new();
state.register_converter("bedrock", Box::new(bedrock_converter));
```

## 🔍 Streaming Fix Applied

The new architecture specifically fixes the streaming issue you encountered:

### **Problem**
- Vertex AI returns `"type": "message"` instead of `"content_block_delta"`
- Original converter only handled standard Anthropic event types
- No content was extracted from "message" events

### **Solution**
- Provider-specific converter handles Vertex AI's unique format
- Fallback logic for unknown event types
- Generic content extraction from various JSON structures

```rust
match chunk.event_type.as_str() {
    "message" => {
        // Vertex AI specific: handle "message" event type
        if let Some(content) = chunk.get_content() {
            self.create_content_chunk(content, request_id, model)
        } else if let Some(usage) = &chunk.usage {
            self.create_usage_chunk(usage, request_id, model)
        } else {
            Ok(None)
        }
    }
    // ... other standard event types
}
```

## 🧪 Testing

Each provider converter can be tested independently:

```rust
#[test]
fn test_vertex_converter() {
    let converter = VertexAIConverter::new();
    
    // Test request conversion
    let openai_request = create_test_request();
    let vertex_request = converter.openai_to_provider_request(&openai_request).unwrap();
    assert_eq!(vertex_request.anthropic_version, "vertex-2023-10-16");
    
    // Test chunk conversion
    let vertex_chunk = create_test_chunk();
    let openai_chunk = converter.provider_chunk_to_openai_chunk(&vertex_chunk, "test", "model").unwrap();
    assert!(openai_chunk.is_some());
}
```

## 🎯 Future Enhancements

### **1. Dynamic Provider Selection**
```rust
// Route requests to different providers based on model
let provider = match model {
    "claude-*" => ProviderType::VertexAI,
    "gpt-*" => ProviderType::OpenAI,
    "anthropic.*" => ProviderType::Bedrock,
    _ => ProviderType::Default,
};
```

### **2. Provider Health Checks**
```rust
pub trait ProviderHealth {
    async fn health_check(&self) -> Result<ProviderStatus>;
    fn is_available(&self) -> bool;
}
```

### **3. Load Balancing**
```rust
pub struct LoadBalancer {
    providers: Vec<Box<dyn ProviderConverter>>,
    strategy: LoadBalancingStrategy,
}
```

### **4. Caching Layer**
```rust
pub trait ProviderCache {
    async fn get_cached_response(&self, request_hash: &str) -> Option<ChatCompletionResponse>;
    async fn cache_response(&self, request_hash: &str, response: &ChatCompletionResponse);
}
```

This architecture makes your API truly provider-agnostic while maintaining the exact OpenAI API compatibility! 🚀
