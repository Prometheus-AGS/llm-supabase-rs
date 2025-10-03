# Testing Guide for OpenAI-Compatible API

## Quick Test Script Usage

I've created a comprehensive test script based on the streaming issue we discovered in your logs. The key finding was:

**🔍 Root Cause Found:** Your logs show `"Unknown streaming event type: message"` - this means Vertex AI is sending event type `"message"` but your converter only handles `"content_block_delta"`, `"message_start"`, etc.

## Running the Tests

### 1. Start Your Server
```bash
# Terminal 1 - Start server with debug logging
RUST_LOG=debug cargo run
```

### 2. Run Tests
```bash
# Terminal 2 - Run all tests
./test_streaming.sh

# Or run specific tests
./test_streaming.sh streaming      # Test your original failing request
./test_streaming.sh health         # Test health endpoint
./test_streaming.sh non-streaming  # Test regular completion
./test_streaming.sh performance    # Test response time
```

### 3. Environment Configuration
```bash
# Custom configuration
BASE_URL=http://localhost:8080 \
API_TOKEN=your-actual-token \
MODEL=claude-4-sonnet-20250514 \
TIMEOUT=30 \
./test_streaming.sh
```

## Expected Test Results

### ✅ What Should Work Now
- **Health Check** - Server responds with 200
- **Non-streaming** - Regular chat completions work
- **Streaming Headers** - Correct `text/event-stream` headers
- **[DONE] Message** - Proper stream termination

### ⚠️ What Might Still Need Fixing
- **Content Chunks** - Based on your logs, you're getting `event_type: "message"` instead of expected types
- **Streaming Content** - The converter needs to handle the `"message"` event type

## The Fix You Need

Based on your server logs showing `"Unknown streaming event type: message"`, you need to update your converter to handle this event type. The fix is to modify `src/infrastructure/vertex/converter.rs`:

```rust
match chunk.event_type.as_str() {
    "message_start" => Ok(None),
    "content_block_delta" | "ping" => {
        // existing code...
    },
    "message" => {  // ADD THIS CASE
        // Handle the "message" event type that Vertex AI is actually sending
        if let Some(content) = chunk.get_content() {
            // Convert to OpenAI format
            // ... same logic as content_block_delta
        } else {
            Ok(None)
        }
    },
    // ... rest of cases
}
```

## Test Script Features

### 🧪 Comprehensive Testing
- **Health Check** - Validates server is running
- **Non-streaming** - Tests regular chat completions
- **Streaming** - Tests your exact failing request
- **Parameters** - Tests all OpenAI parameters
- **Error Handling** - Tests invalid requests
- **Performance** - Measures response times

### 📊 Detailed Analysis
- **HTTP Status Codes** - Validates correct responses
- **Headers** - Checks SSE headers (`text/event-stream`, `no-cache`)
- **JSON Validation** - Ensures valid response format
- **Chunk Counting** - Counts streaming chunks vs content chunks
- **Content Extraction** - Shows actual streaming content

### 🎯 Your Original Issue Testing
The script specifically tests your original failing request:
```json
{
  "model":"claude-4-sonnet-20250514",
  "stream": true,
  "messages": [
    {
      "role": "user",
      "content": "tell me something good"
    }
  ]
}
```

## Sample Output

```bash
🚀 Starting OpenAI-Compatible API Tests
Server: http://localhost:8080
Model: claude-4-sonnet-20250514

=== Test 3: Streaming Chat Completion (Original Issue) ===
[INFO] Making streaming request (your original failing request)...
[SUCCESS] Correct Content-Type: text/event-stream
[SUCCESS] Correct Cache-Control: no-cache
[INFO] Streaming analysis:
  Total data chunks: 1
  Content chunks: 0
  [DONE] chunks: 1
[WARNING] Only received [DONE] - no content chunks (this was your original issue)
[INFO] This suggests the 'message' event type issue we found in logs
```

## Next Steps

1. **Run the test script** to confirm the current behavior
2. **Check server logs** for the "Unknown streaming event type" messages
3. **Update the converter** to handle the actual event types Vertex AI sends
4. **Re-run tests** to validate the fix

The test script will help you validate each step of the fix! 🚀
