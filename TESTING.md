# Testing Guide

This document explains how to run the comprehensive test suite for the LLM Supabase Rust server, including real end-to-end tests with Vertex AI.

## Test Categories

### 1. Unit Tests (Always Available)
These test the conversion logic without requiring external services:

```bash
# Test OpenAI ↔ Vertex AI conversion logic
cargo test infrastructure::vertex::test_conversion --lib

# Test specific conversion scenarios
cargo test test_openai_vertex_conversion_basic --lib -- --nocapture
cargo test test_real_world_scenario --lib -- --nocapture
```

### 2. Integration Tests (Require Setup)
These test the complete flow with real Vertex AI API calls.

## Setup for Real E2E Tests

### Prerequisites

1. **Google Cloud Project** with Vertex AI API enabled
2. **Service Account** with Vertex AI permissions
3. **Environment Configuration**

### Step 1: Enable Vertex AI API

```bash
# Enable the Vertex AI API
gcloud services enable aiplatform.googleapis.com

# Enable Claude models (if not already available)
# Visit: https://console.cloud.google.com/vertex-ai/model-garden
```

### Step 2: Create Service Account

```bash
# Create service account
gcloud iam service-accounts create vertex-ai-test \
    --description="Service account for Vertex AI testing" \
    --display-name="Vertex AI Test"

# Grant necessary permissions
gcloud projects add-iam-policy-binding YOUR_PROJECT_ID \
    --member="serviceAccount:vertex-ai-test@YOUR_PROJECT_ID.iam.gserviceaccount.com" \
    --role="roles/aiplatform.user"

# Create and download key
gcloud iam service-accounts keys create ./gcp-credentials.json \
    --iam-account=vertex-ai-test@YOUR_PROJECT_ID.iam.gserviceaccount.com
```

### Step 3: Configure Environment

Create a `.env` file in the project root:

```bash
# Copy the example
cp .env.example .env

# Edit with your values
nano .env
```

Required environment variables:
```env
# Google Cloud Platform Configuration
GCP_PROJECT_ID=your-actual-project-id
GCP_LOCATION=us-east5
GOOGLE_APPLICATION_CREDENTIALS=./gcp-credentials.json

# Model Configuration
DEFAULT_MODEL=claude-sonnet-4-5@20250929
```

### Step 4: Verify Setup

```bash
# Test configuration loading
cargo test test_env_config_loading --test integration_vertex_e2e

# Test instructions
cargo test test_instructions --test integration_vertex_e2e -- --nocapture
```

## Running E2E Tests

### Basic End-to-End Test
Tests the complete OpenAI → Vertex AI → OpenAI conversion flow:

```bash
cargo test test_real_vertex_ai_e2e_flow --test integration_vertex_e2e --ignored -- --nocapture
```

**What this tests:**
- ✅ Environment configuration loading
- ✅ Vertex AI client initialization  
- ✅ OpenAI request → Vertex AI request conversion
- ✅ Real API call to Vertex AI
- ✅ Vertex AI response → OpenAI response conversion
- ✅ Complete JSON serialization/deserialization
- ✅ Usage statistics and token counting

### Streaming Test
Tests real-time streaming responses:

```bash
cargo test test_real_vertex_ai_streaming_e2e --test integration_vertex_e2e --ignored -- --nocapture
```

**What this tests:**
- ✅ Streaming request conversion
- ✅ Real streaming API call
- ✅ Chunk-by-chunk processing
- ✅ OpenAI streaming format compliance

### Health Check Test
Tests client health and model availability:

```bash
cargo test test_real_vertex_ai_health_check --test integration_vertex_e2e --ignored -- --nocapture
```

**What this tests:**
- ✅ Authentication validation
- ✅ API connectivity
- ✅ Model listing
- ✅ Health check functionality

### Error Handling Test
Tests error scenarios and edge cases:

```bash
cargo test test_real_vertex_ai_error_handling --test integration_vertex_e2e --ignored -- --nocapture
```

**What this tests:**
- ✅ Invalid model name handling
- ✅ Error response parsing
- ✅ Proper error propagation

### Performance Test
Tests response times and performance:

```bash
cargo test test_real_vertex_ai_performance --test integration_vertex_e2e --ignored -- --nocapture
```

**What this tests:**
- ✅ Response time measurement
- ✅ Performance expectations
- ✅ Token usage efficiency

### Image Chat Completion Test
Tests vision capabilities with real images:

```bash
cargo test test_real_vertex_ai_image_chat --test integration_vertex_e2e --ignored -- --nocapture
```

**What this tests:**
- ✅ Image encoding and processing
- ✅ Vision model integration
- ✅ Image analysis accuracy
- ✅ Higher token usage for images
- ✅ Programming concept recognition

### Streaming Image Chat Test
Tests real-time image analysis streaming:

```bash
cargo test test_real_vertex_ai_streaming_image_chat --test integration_vertex_e2e --ignored -- --nocapture
```

**What this tests:**
- ✅ Streaming image processing
- ✅ Real-time vision analysis
- ✅ Chunk-by-chunk image responses
- ✅ Content validation for images

### Run All E2E Tests
```bash
# Run all real Vertex AI tests
cargo test test_real_vertex_ai --test integration_vertex_e2e --ignored -- --nocapture

# Run all tests (unit + integration)
cargo test --test integration_vertex_e2e --ignored -- --nocapture
```

## Expected Output

### Successful E2E Test Output
```
🚀 Starting real Vertex AI end-to-end test...
📋 Configuration loaded:
  Project ID: your-project-123
  Region: us-east5
  Model: claude-sonnet-4-5@20250929
✅ Vertex AI client initialized successfully
📤 OpenAI Request created:
  Model: claude-sonnet-4-5@20250929
  Messages: 2 messages
  Max tokens: Some(50)
🔄 Converted to Vertex AI format:
  Anthropic version: vertex-2023-10-16
  Max tokens: 50
  System: Some("You are a helpful assistant. Keep responses brief.")
  Messages: 1 messages
📡 Making API call to Vertex AI...
✅ Received response from Vertex AI:
  Response ID: msg_01ABC123
  Model: claude-sonnet-4-5@20250929
  Content blocks: 1
  Usage - Input: 25, Output: 8
🔄 Converted back to OpenAI format:
  Response ID: chatcmpl-e2e-test
  Object: chat.completion
  Model: claude-sonnet-4-5@20250929
  Choices: 1
💬 Assistant response: 'Test successful'
📋 Final OpenAI API response:
{
  "id": "chatcmpl-e2e-test",
  "object": "chat.completion",
  "created": 1759482420,
  "model": "claude-sonnet-4-5@20250929",
  "choices": [
    {
      "index": 0,
      "message": {
        "role": "assistant",
        "content": "Test successful"
      },
      "finish_reason": "stop"
    }
  ],
  "usage": {
    "prompt_tokens": 25,
    "completion_tokens": 8,
    "total_tokens": 33
  }
}
🎉 End-to-end test completed successfully!
✅ Complete flow validated: OpenAI Request → Vertex AI → OpenAI Response
```

## Troubleshooting

### Common Issues

#### 1. Authentication Errors
```
Error: Failed to initialize Vertex AI client
```
**Solution:**
- Verify `GOOGLE_APPLICATION_CREDENTIALS` points to valid JSON file
- Check service account has `roles/aiplatform.user` permission
- Ensure project ID is correct

#### 2. Model Not Available
```
Error: Model claude-sonnet-4-5@20250929 not found
```
**Solution:**
- Visit [Vertex AI Model Garden](https://console.cloud.google.com/vertex-ai/model-garden)
- Enable Claude models for your project
- Check model name spelling and version

#### 3. Region Issues
```
Error: Location us-east5 not supported
```
**Solution:**
- Use supported regions: `us-east5`, `us-central1`, `europe-west1`
- Update `GCP_LOCATION` in `.env` file

#### 4. API Not Enabled
```
Error: Vertex AI API not enabled
```
**Solution:**
```bash
gcloud services enable aiplatform.googleapis.com
```

### Debug Mode
Run tests with debug output:
```bash
RUST_LOG=debug cargo test test_real_vertex_ai_e2e_flow --test integration_vertex_e2e --ignored -- --nocapture
```

## Test Development

### Adding New E2E Tests

1. **Create test function** in `tests/integration/test_vertex_e2e.rs`
2. **Mark with `#[ignore]`** for manual execution
3. **Use `dotenv::dotenv().ok()`** to load environment
4. **Follow the pattern:**
   ```rust
   #[tokio::test]
   #[ignore = "requires real GCP credentials and API access"]
   async fn test_your_scenario() {
       dotenv::dotenv().ok();
       let config = create_vertex_config_from_env().expect("Config failed");
       // ... your test logic
   }
   ```

### Best Practices

1. **Keep tests focused** - test one specific scenario per function
2. **Use descriptive names** - clearly indicate what's being tested
3. **Include validation** - assert on all important response fields
4. **Handle errors gracefully** - provide clear error messages
5. **Log progress** - use `println!` for test progress tracking

## CI/CD Integration

For automated testing in CI/CD:

```yaml
# GitHub Actions example
- name: Run Unit Tests
  run: cargo test --lib

- name: Run E2E Tests (if credentials available)
  run: |
    if [ -n "$GCP_SERVICE_ACCOUNT_KEY" ]; then
      echo "$GCP_SERVICE_ACCOUNT_KEY" > gcp-credentials.json
      cargo test --test integration_vertex_e2e --ignored
    fi
  env:
    GCP_PROJECT_ID: ${{ secrets.GCP_PROJECT_ID }}
    GCP_LOCATION: us-east5
    GOOGLE_APPLICATION_CREDENTIALS: ./gcp-credentials.json
```

This comprehensive testing setup ensures your OpenAI-compatible API works correctly with Vertex AI in all scenarios!
