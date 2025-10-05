# Streaming E2E Tool Calling Guide

## Overview

The streaming E2E test (`test-streaming-tool-e2e.ts`) demonstrates the complete tool calling flow using **Server-Sent Events (SSE)** streaming, which is critical for building responsive user interfaces.

## Why Streaming Matters

### Non-Streaming Flow
```
User waits... → LLM processes entire response → Tool calls appear → User waits... → Final answer appears
```

**Problems:**
- Long wait times before any response
- Poor user experience
- No progress feedback

### Streaming Flow
```
"Calculat" → "ing..." → Tool call chunk by chunk → Execute → "The" → " result" → " is" → " 888"
```

**Benefits:**
- Immediate feedback as tokens arrive
- Better perceived performance
- Can show "thinking" state to users
- Industry-standard for modern AI apps

## How SSE Streaming Works

### 1. Server-Sent Events Format

```http
data: {"choices":[{"delta":{"content":"The"}}]}

data: {"choices":[{"delta":{"content":" result"}}]}

data: {"choices":[{"delta":{"content":" is"}}]}

data: [DONE]
```

### 2. Tool Call Streaming

Tool calls are streamed incrementally:

```http
data: {"choices":[{"delta":{"tool_calls":[{"index":0,"id":"call_123"}]}}]}

data: {"choices":[{"delta":{"tool_calls":[{"index":0,"function":{"name":"calc"}}]}}]}

data: {"choices":[{"delta":{"tool_calls":[{"index":0,"function":{"name":"ulate"}}]}}]}

data: {"choices":[{"delta":{"tool_calls":[{"index":0,"function":{"arguments":"{\""}}]}}]}

data: {"choices":[{"delta":{"tool_calls":[{"index":0,"function":{"arguments":"a\":"}}]}}]}

data: [DONE]
```

**Accumulated result:**
```json
{
  "id": "call_123",
  "type": "function",
  "function": {
    "name": "calculate",
    "arguments": "{\"a\":24,\"b\":37,\"operation\":\"multiply\"}"
  }
}
```

## Running Streaming Tests

```bash
# Start service
cargo run

# Run streaming E2E tests
npm run test:streaming
```

## Expected Output

```
🌊 Streaming E2E Tool Calling Test Suite
=========================================

🧪 Streaming Test: Simple Calculation (Streaming)
================================================================================

📤 Step 1: User Query
   "Calculate 156 multiplied by 47. Show me the exact result."

🌊 Step 2: Streaming Request to LLM (with tools)
   → Sending request to LLM...
   ← Received response from LLM

📋 Step 3: Tool Calls from Stream
   ✓ 1 tool call(s) accumulated from stream
   • calculate
     ID: call_xyz123
     Args: {"operation":"multiply","a":156,"b":47}

🔧 Step 4: Execute Tools Locally
   🔧 Executing: calculate(multiply, 156, 47)
   ✓ Result: {"operation":"multiply","operand1":156,"operand2":47,"result":7332}

🌊 Step 5: Streaming Final Answer from LLM
   → Sending request to LLM...
   ← Received response from LLM
The result of 156 multiplied by 47 is 7,332.

💬 Step 6: Complete Final Answer

   The result of 156 multiplied by 47 is 7,332.

✅ Streaming Test Passed (2845ms)
   • Streamed initial request
   • Accumulated 1 tool call(s) from chunks
   • Executed tools locally
   • Streamed final answer
```

## Implementation Details

### Accumulating Streamed Tool Calls

```typescript
const toolCallsMap = new Map<number, ToolCall>();

// Process each SSE chunk
for (const chunk of streamChunks) {
  const delta = chunk.choices?.[0]?.delta;
  
  if (delta.tool_calls) {
    for (const toolCallDelta of delta.tool_calls) {
      const index = toolCallDelta.index ?? 0;
      
      // Initialize if first chunk for this tool call
      if (!toolCallsMap.has(index)) {
        toolCallsMap.set(index, {
          id: toolCallDelta.id || `call_${index}`,
          type: 'function',
          function: { name: '', arguments: '' }
        });
      }
      
      const toolCall = toolCallsMap.get(index)!;
      
      // Accumulate name
      if (toolCallDelta.function?.name) {
        toolCall.function.name += toolCallDelta.function.name;
      }
      
      // Accumulate arguments
      if (toolCallDelta.function?.arguments) {
        toolCall.function.arguments += toolCallDelta.function.arguments;
      }
    }
  }
}

// Convert map to array
const toolCalls = Array.from(toolCallsMap.values());
```

### Handling Multiple Parallel Tool Calls

```typescript
// LLM can request multiple tools in parallel via streaming
data: {"choices":[{"delta":{"tool_calls":[{"index":0,"id":"call_1"}]}}]}
data: {"choices":[{"delta":{"tool_calls":[{"index":1,"id":"call_2"}]}}]}
data: {"choices":[{"delta":{"tool_calls":[{"index":0,"function":{"name":"get_weather"}}]}}]}
data: {"choices":[{"delta":{"tool_calls":[{"index":1,"function":{"name":"calculate"}}]}}]}
```

The `index` field tells us which tool call each chunk belongs to.

## Complete Streaming E2E Flow

```
1. User Query
   ↓
2. Stream Request (with tools)
   ↓
3. SSE Stream Starts
   data: {"delta":{"tool_calls":[...]}}
   data: {"delta":{"tool_calls":[...]}}
   data: {"delta":{"tool_calls":[...]}}
   data: [DONE]
   ↓
4. Accumulate Tool Calls
   {"id":"call_123","function":{"name":"calculate","arguments":"{...}"}}
   ↓
5. Execute Tools Locally
   calculate(multiply, 156, 47) = 7332
   ↓
6. Stream Request (with tool results)
   ↓
7. SSE Stream Starts
   data: {"delta":{"content":"The"}}
   data: {"delta":{"content":" result"}}
   data: {"delta":{"content":" is"}}
   data: [DONE]
   ↓
8. Display Final Answer
   "The result of 156 multiplied by 47 is 7,332."
```

## Test Scenarios

### 1. Simple Calculation
- **Query**: "Calculate 156 multiplied by 47"
- **Tool**: `calculate`
- **Tests**: Basic streaming accumulation

### 2. Weather Query
- **Query**: "What is the weather in Paris, France?"
- **Tool**: `get_weather`
- **Tests**: String parameters via streaming

### 3. Time Query
- **Query**: "What is the current time in Tokyo?"
- **Tool**: `get_current_time`
- **Tests**: Optional parameters via streaming

### 4. Complex Calculation
- **Query**: "What is 2048 divided by 64?"
- **Tool**: `calculate`
- **Tests**: Different operations via streaming

## Key Differences from Non-Streaming

| Aspect | Non-Streaming | Streaming |
|--------|--------------|-----------|
| Response | Single complete object | Multiple SSE chunks |
| Tool calls | Immediately available | Must be accumulated |
| Final answer | All at once | Token by token |
| User experience | Wait then see all | See tokens as generated |
| Implementation | Simpler | More complex |
| Use case | Batch processing | Interactive UIs |

## Common Issues

### Issue 1: Incomplete Tool Call Accumulation

**Problem**: Tool call JSON is incomplete
```json
{"function":{"name":"calc","arguments":"{\"a\":"}}  // Missing closing braces
```

**Cause**: Not waiting for `[DONE]` signal

**Solution**: Process all chunks until `[DONE]`

### Issue 2: Mixed Content and Tool Calls

**Problem**: LLM streams both content and tool calls

**Solution**: Accumulate both separately
```typescript
if (delta.content) {
  content += delta.content;
}
if (delta.tool_calls) {
  // Accumulate tool calls
}
```

### Issue 3: Parsing Errors

**Problem**: Chunk is not valid JSON

**Solution**: Gracefully handle parse errors
```typescript
try {
  const chunk = JSON.parse(data);
} catch (error) {
  console.warn('Failed to parse chunk:', data);
  continue; // Skip this chunk
}
```

## Performance Considerations

### Streaming Overhead
- **Latency**: First token arrives faster
- **Bandwidth**: More HTTP overhead due to SSE framing
- **Processing**: Client must accumulate chunks

### When to Use Streaming
✅ **Use streaming when:**
- Building interactive UIs
- Users need immediate feedback
- Responses are long
- Token-by-token display is valuable

❌ **Don't use streaming when:**
- Building batch processing systems
- Complete response needed before processing
- Network is unreliable (chunks can be lost)
- Simpler non-streaming works fine

## Testing Best Practices

1. **Test both streaming and non-streaming**: Different code paths
2. **Verify chunk accumulation**: Ensure no data loss
3. **Test error handling**: Malformed chunks, network errors
4. **Test parallel tool calls**: Multiple tools in one stream
5. **Test large responses**: Memory and performance
6. **Test network issues**: Reconnection, timeout

## Integration Example

```typescript
// React component using streaming tool calls
function ChatWithTools() {
  const [messages, setMessages] = useState([]);
  const [streamingContent, setStreamingContent] = useState('');
  
  async function sendMessage(userMessage) {
    const response = await fetch('/v1/chat/completions', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({
        messages: [...messages, { role: 'user', content: userMessage }],
        tools: TOOL_DEFINITIONS,
        stream: true
      })
    });
    
    const reader = response.body.getReader();
    const decoder = new TextDecoder();
    let buffer = '';
    let toolCalls = new Map();
    
    while (true) {
      const { done, value } = await reader.read();
      if (done) break;
      
      buffer += decoder.decode(value, { stream: true });
      const lines = buffer.split('\n');
      buffer = lines.pop() || '';
      
      for (const line of lines) {
        if (line.startsWith('data: ')) {
          const data = line.slice(6);
          if (data === '[DONE]') break;
          
          const chunk = JSON.parse(data);
          const delta = chunk.choices[0].delta;
          
          // Update UI with streaming content
          if (delta.content) {
            setStreamingContent(prev => prev + delta.content);
          }
          
          // Accumulate tool calls
          if (delta.tool_calls) {
            // ... accumulation logic
          }
        }
      }
    }
    
    // Execute tools if needed
    if (toolCalls.size > 0) {
      const results = await executeTools(Array.from(toolCalls.values()));
      // Stream again with results...
    }
  }
  
  return (/* UI code */);
}
```

## Summary

The streaming E2E test verifies:
- ✅ SSE streaming works correctly
- ✅ Tool calls can be accumulated from chunks
- ✅ Tools can be executed with streamed parameters
- ✅ Final answers can be streamed back
- ✅ Complete request → tool → response flow works via streaming

This is essential for production applications where user experience matters.

## Related Documentation

- [E2E Quick Start](E2E_QUICK_START.md) - Getting started guide
- [E2E Tool Calling Guide](E2E_TOOL_CALLING_GUIDE.md) - Comprehensive guide
- [Streaming Fix Instructions](STREAMING_FIX_INSTRUCTIONS.md) - Debugging streaming issues
