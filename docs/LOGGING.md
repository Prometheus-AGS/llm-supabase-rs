# Logging Guide for llm-supabase-rs

## Overview

This project uses **`tracing`** and **`tracing-subscriber`** for logging - the modern, production-ready logging framework for async Rust applications.

## Why `tracing` is the Best Choice

1. **Async-First**: Designed specifically for async Rust and Tokio
2. **Structured Logging**: Built-in support for key-value pairs and structured data
3. **Instrumentation**: Tracks async task lifecycles automatically
4. **Zero-cost**: Minimal overhead when logging is disabled
5. **Flexible Output**: JSON, pretty-print, or custom formats
6. **OpenTelemetry**: Native integration for distributed tracing

## Features Implemented

✅ **Multiple Output Formats**
- Pretty colored logs (development)
- Compact logs (production)
- JSON structured logs (for log aggregation like ELK, Datadog, etc.)

✅ **Environment-based Configuration**
- Auto-switches format based on debug/release build
- Configurable via environment variables

✅ **Rich Context**
- Thread names/IDs
- Source file/line numbers
- Module paths
- Timestamps

✅ **Span Tracing**
- Track request lifecycles
- Measure execution time
- Nested context propagation

## Environment Variables

```bash
# Log level (trace, debug, info, warn, error)
LOG_LEVEL=info

# Enable file logging
LOG_TO_FILE=true
LOG_FILE_PATH=./logs/app.log

# Enable request/response logging
LOG_REQUESTS=true
```

## Usage Examples

### Basic Logging

```rust
use tracing::{info, warn, error, debug, trace};

// Simple log
info!("Server starting");

// With structured data
info!(
    port = 8080,
    host = "0.0.0.0",
    "Server bound to address"
);

// With context
error!(
    error = ?err,
    request_id = %req_id,
    "Failed to process request"
);
```

### Instrumentation (Span Tracing)

```rust
use tracing::{instrument, info};

#[instrument(skip(database))]
async fn process_request(user_id: u64, database: &Database) -> Result<()> {
    info!("Processing request"); // Automatically includes user_id in context
    
    // Sub-spans
    let data = fetch_data(user_id).await?;
    let result = transform(data).await?;
    
    Ok(result)
}

#[instrument]
async fn fetch_data(user_id: u64) -> Result<Data> {
    // This span is nested under process_request
    info!("Fetching from database");
    // ...
}
```

### Structured Logging with Fields

```rust
use tracing::{info, warn};

// Add fields to a span
let span = tracing::info_span!(
    "http_request",
    method = "GET",
    path = "/api/chat",
    request_id = %uuid::Uuid::new_v4()
);

let _guard = span.enter();
info!("Request received");
// All logs within this guard will include the span fields
```

### Error Logging

```rust
use tracing::error;

match dangerous_operation().await {
    Ok(result) => info!(?result, "Operation successful"),
    Err(e) => error!(
        error = %e,
        error_debug = ?e,
        "Operation failed"
    ),
}
```

## Log Levels

Use appropriate levels for different situations:

- **`trace`**: Very detailed, verbose debugging (usually disabled)
- **`debug`**: Detailed info for debugging (disabled in production)
- **`info`**: General informational messages
- **`warn`**: Warning conditions that might need attention
- **`error`**: Error conditions that need immediate attention

## Log Formats

### Development (Pretty)
```
2025-10-03T08:21:14.938713Z  INFO llm_supabase_rs: Starting LLM Supabase Service
    at src/main.rs:27
    in main
```

### Production (JSON)
```json
{
  "timestamp": "2025-10-03T08:21:14.938713Z",
  "level": "INFO",
  "target": "llm_supabase_rs",
  "message": "Starting LLM Supabase Service",
  "file": "src/main.rs",
  "line": 27,
  "span": {
    "name": "main"
  }
}
```

## Advanced Features

### Custom Fields

```rust
use tracing::info;

info!(
    user_id = 12345,
    action = "login",
    ip_address = %req.ip(),
    "User action performed"
);
```

### Conditional Logging

```rust
use tracing::{debug, enabled, Level};

if enabled!(Level::DEBUG) {
    let expensive_data = compute_debug_info();
    debug!(?expensive_data, "Debug information");
}
```

### Span Guards

```rust
use tracing::{info_span, warn};

async fn process() {
    let span = info_span!("process_batch", batch_id = 123);
    let _guard = span.enter();
    
    // All logs here include batch_id
    info!("Starting processing");
    
    // Span automatically closed when _guard drops
}
```

## Integration with HTTP Requests

Your project uses `tower-http` tracing:

```rust
use tower_http::trace::TraceLayer;

let app = Router::new()
    .route("/api/chat", post(chat_handler))
    .layer(
        TraceLayer::new_for_http()
            .make_span_with(|request: &Request<_>| {
                tracing::info_span!(
                    "http_request",
                    method = %request.method(),
                    uri = %request.uri(),
                    version = ?request.version(),
                )
            })
    );
```

## Best Practices

1. **Use structured logging** - Add key-value pairs instead of string formatting
   ```rust
   // ✅ Good
   info!(user_id = 123, action = "login", "User logged in");
   
   // ❌ Bad
   info!("User 123 performed action: login");
   ```

2. **Use `instrument` for functions** - Automatically track entry/exit
   ```rust
   #[instrument]
   async fn my_function() { }
   ```

3. **Skip sensitive data** - Don't log passwords, tokens, etc.
   ```rust
   #[instrument(skip(password))]
   fn authenticate(username: &str, password: &str) { }
   ```

4. **Use appropriate levels** - Don't overuse INFO or DEBUG

5. **Add request IDs** - For tracing requests across services
   ```rust
   info!(request_id = %uuid::Uuid::new_v4(), "Processing request");
   ```

## Production Deployment

For production, ensure:

1. Set `LOG_LEVEL=info` or `warn`
2. Use JSON format for log aggregation
3. Disable source location to reduce log size
4. Send logs to a centralized logging system (ELK, Datadog, etc.)

## Log Aggregation Examples

### With Datadog
```bash
# Your app outputs JSON logs
LOG_FORMAT=json cargo run

# Datadog agent picks them up automatically
```

### With ELK Stack
```bash
# App outputs JSON to stdout
# Filebeat collects and sends to Elasticsearch
# Kibana visualizes
```

## Troubleshooting

### No logs appearing?
Check your LOG_LEVEL environment variable.

### Too verbose?
Set `LOG_LEVEL=warn` or `LOG_LEVEL=error`

### Need more detail?
Set `LOG_LEVEL=debug` or `LOG_LEVEL=trace` (temporarily!)

## Migration from `env_logger`

If you used `env_logger` before, `tracing` is similar but better:

```rust
// Old (env_logger)
log::info!("Message");

// New (tracing)
tracing::info!("Message");

// Structured data
tracing::info!(key = value, "Message");
```

## Quick Start

```rust
// In your .env
LOG_LEVEL=info

// In your code
use tracing::{info, error, instrument};

#[instrument]
async fn my_handler() -> Result<()> {
    info!("Handler called");
    Ok(())
}
```

## Resources

- [tracing docs](https://docs.rs/tracing/)
- [tracing-subscriber docs](https://docs.rs/tracing-subscriber/)
- [Tokio tracing guide](https://tokio.rs/tokio/topics/tracing)
