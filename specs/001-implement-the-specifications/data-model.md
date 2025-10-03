# Data Model: Vertex AI and Claude 4 Sonnet Integration

**Date**: October 2, 2025
**Feature**: Vertex AI and Claude 4 Sonnet Priority Implementation

## Core Entities

### 1. Chat Request
**Purpose**: Represents incoming OpenAI-compatible chat completion requests

**Fields**:
- `model: String` - Model identifier (e.g., "claude-4-sonnet-20250514")
- `messages: Vec<Message>` - Array of conversation messages
- `max_tokens: Option<u32>` - Maximum tokens to generate (1-200000)
- `temperature: Option<f64>` - Randomness control (0.0-2.0)
- `top_p: Option<f64>` - Nucleus sampling (0.0-1.0)
- `stream: Option<bool>` - Enable streaming response
- `user: Option<String>` - End-user identifier for tracking

**Validation Rules**:
- `messages` must not be empty
- `messages.len()` must not exceed 100
- `max_tokens` must be between 1 and 200,000
- `temperature` must be between 0.0 and 2.0
- `top_p` must be between 0.0 and 1.0
- `model` must be a supported model identifier

**Relationships**:
- Contains multiple `Message` entities
- Maps to `VertexAIRequest` for provider calls
- Associated with `AuthContext` for authenticated requests

### 2. Message
**Purpose**: Individual message in chat conversation

**Fields**:
- `role: MessageRole` - Message sender (system, user, assistant)
- `content: String` - Message text content
- `name: Option<String>` - Optional participant name

**Validation Rules**:
- `content` must not be empty
- `content.len()` must not exceed 1,000,000 characters
- `role` must be valid MessageRole variant
- `name` is optional, max 64 characters if provided

**State Transitions**:
- Messages are immutable once created
- Conversation history flows: system → user → assistant → user...

### 3. Chat Response
**Purpose**: OpenAI-compatible response structure

**Fields**:
- `id: String` - Unique response identifier
- `object: String` - Always "chat.completion" or "chat.completion.chunk"
- `created: u64` - Unix timestamp
- `model: String` - Model used for generation
- `choices: Vec<Choice>` - Response choices (typically 1)
- `usage: Option<Usage>` - Token usage statistics

**Validation Rules**:
- `id` must be unique per request
- `created` must be valid unix timestamp
- `choices` must contain at least one choice
- `usage` required for non-streaming responses

**Relationships**:
- Contains multiple `Choice` entities
- Contains one `Usage` entity (for non-streaming)
- Maps from `VertexAIResponse`

### 4. Choice
**Purpose**: Individual response choice from the model

**Fields**:
- `index: u32` - Choice index (0-based)
- `message: Option<Message>` - Complete message (non-streaming)
- `delta: Option<Delta>` - Incremental content (streaming)
- `finish_reason: Option<String>` - Why generation stopped
- `logprobs: Option<Value>` - Log probabilities (not supported)

**Validation Rules**:
- `index` starts at 0
- Exactly one of `message` or `delta` must be present
- `finish_reason` values: "stop", "length", "content_filter", null

### 5. Delta
**Purpose**: Incremental content for streaming responses

**Fields**:
- `role: Option<MessageRole>` - Role (only in first chunk)
- `content: Option<String>` - Incremental text content

**Validation Rules**:
- `role` only present in first streaming chunk
- `content` may be empty for role-only chunks

### 6. Usage
**Purpose**: Token usage statistics

**Fields**:
- `prompt_tokens: u32` - Tokens in input
- `completion_tokens: u32` - Tokens in output
- `total_tokens: u32` - Sum of prompt + completion tokens

**Validation Rules**:
- `total_tokens` must equal `prompt_tokens + completion_tokens`
- All values must be non-negative

### 7. Authentication Context
**Purpose**: User authentication and authorization information

**Fields**:
- `user_id: Option<Uuid>` - Supabase user ID from JWT
- `email: Option<String>` - User email from JWT claims
- `role: Option<String>` - User role for authorization
- `metadata: Map<String, Value>` - Additional user metadata
- `is_authenticated: bool` - Authentication status

**Validation Rules**:
- `user_id` must be valid UUID format if present
- `email` must be valid email format if present
- `is_authenticated` must match presence of valid JWT

**Relationships**:
- Associated with each request for logging and rate limiting
- Used for authorization decisions

### 8. Provider Configuration
**Purpose**: Configuration for external AI providers

**Fields**:
- `name: String` - Provider identifier ("vertex-ai", "openai", etc.)
- `enabled: bool` - Whether provider is active
- `priority: u8` - Provider selection priority (lower = higher priority)
- `base_url: Option<String>` - API base URL
- `api_key: Option<String>` - Authentication key
- `project_id: Option<String>` - Cloud project identifier
- `location: Option<String>` - Geographic location
- `max_concurrent_requests: u32` - Concurrency limit
- `timeout_seconds: u32` - Request timeout

**Validation Rules**:
- `name` must be unique per configuration
- `priority` must be unique per enabled provider
- `max_concurrent_requests` must be > 0
- `timeout_seconds` must be between 1 and 300

**Relationships**:
- Defines available providers for request routing
- Used by provider selection logic

### 9. Request Log
**Purpose**: Audit trail and analytics for API requests

**Fields**:
- `id: Uuid` - Unique log entry identifier
- `request_id: Uuid` - Request correlation ID
- `user_id: Option<Uuid>` - Authenticated user ID
- `model: String` - Requested model
- `provider: String` - Provider used for request
- `endpoint: String` - API endpoint called
- `prompt_tokens: Option<u32>` - Input token count
- `completion_tokens: Option<u32>` - Output token count
- `total_tokens: Option<u32>` - Total token count
- `duration_ms: u32` - Request duration in milliseconds
- `status: RequestStatus` - Request outcome
- `error_message: Option<String>` - Error details if failed
- `created_at: DateTime<Utc>` - Timestamp

**Validation Rules**:
- `request_id` must be unique per request
- `duration_ms` must be > 0
- `status` must be valid RequestStatus variant
- `error_message` required if status is error

**State Transitions**:
- Created → In Progress → Completed/Failed
- Immutable once request completes

## Enumerations

### MessageRole
```rust
pub enum MessageRole {
    System,    // System instructions
    User,      // User input
    Assistant, // Model response
}
```

### RequestStatus
```rust
pub enum RequestStatus {
    Success,          // Request completed successfully
    Error,            // Request failed with error
    Timeout,          // Request exceeded time limit
    RateLimited,      // Request blocked by rate limiting
    ProviderError,    // Provider returned error
}
```

## Provider-Specific Models

### VertexAI Request
**Purpose**: Native Vertex AI request format for Claude models

**Fields**:
- `anthropic_version: String` - API version ("bedrock-2023-05-31")
- `max_tokens: u32` - Maximum tokens to generate
- `messages: Vec<VertexMessage>` - Conversation messages
- `temperature: Option<f64>` - Generation temperature
- `top_p: Option<f64>` - Top-p sampling
- `stop_sequences: Option<Vec<String>>` - Stop generation triggers
- `stream: bool` - Enable streaming

### VertexAI Response
**Purpose**: Native Vertex AI response format

**Fields**:
- `content: Vec<ContentBlock>` - Response content blocks
- `id: String` - Response identifier
- `model: String` - Model used
- `role: String` - Response role ("assistant")
- `stop_reason: Option<String>` - Generation stop reason
- `stop_sequence: Option<String>` - Matched stop sequence
- `usage: VertexUsage` - Token usage

### ContentBlock
**Purpose**: Individual content block in Vertex AI response

**Fields**:
- `type: String` - Block type ("text")
- `text: String` - Text content

## Conversion Mappings

### OpenAI → Vertex AI
- `ChatCompletionRequest` → `VertexAIRequest`
- `Message` → `VertexMessage` (role mapping)
- Parameters: direct mapping with validation

### Vertex AI → OpenAI
- `VertexAIResponse` → `ChatCompletionResponse`
- `ContentBlock` → `Message.content`
- `VertexUsage` → `Usage`
- Error codes: mapped to OpenAI error format

## Database Schema (Supabase)

### webhooks
- Configuration for webhook delivery
- User-specific webhook endpoints
- Event filtering and authentication

### request_logs
- Audit trail for all API requests
- Performance and usage analytics
- Error tracking and debugging

### tool_calls
- Future extension for tool/function calling
- Integration with MCP servers

### memories
- Future extension for context/memory
- Vector embeddings for semantic search

## Performance Considerations

### 1. Memory Management
- Streaming responses: process chunks incrementally
- Large requests: validate size limits early
- Connection pooling: reuse HTTP connections

### 2. Concurrency
- Request handling: fully async with Tokio
- Provider calls: parallel execution where possible
- Database operations: connection pooling

### 3. Caching
- Model metadata: cache provider capabilities
- Configuration: cache with TTL refresh
- Authentication: cache JWT validation results briefly

## Error Handling Strategy

### 1. Validation Errors
- Input validation: return 400 Bad Request
- Authentication: return 401 Unauthorized
- Authorization: return 403 Forbidden

### 2. Provider Errors
- Timeout: attempt fallback provider
- Rate limit: return 429 Too Many Requests
- Server error: return 503 Service Unavailable

### 3. System Errors
- Configuration: fail fast at startup
- Database: retry with exponential backoff
- Network: circuit breaker pattern

---

**Status**: Data model complete - entities, relationships, and validation rules defined
**Next**: API Contracts generation