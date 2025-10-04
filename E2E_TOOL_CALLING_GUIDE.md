# End-to-End Tool Calling Test Guide

This guide explains how to run and understand the comprehensive end-to-end tool calling test.

## Overview

The E2E test (`test-tool-calling-e2e.ts`) demonstrates the **complete tool calling flow**:

1. **User Query** → User asks a question that requires external data
2. **Tool Call Request** → LLM analyzes the query and requests specific tool(s)
3. **Tool Execution** → Tools are executed locally with the requested parameters
4. **Result Processing** → Tool results are formatted and sent back to the LLM
5. **Final Answer** → LLM generates a natural language response using the tool results

## Prerequisites

### 1. Service Running

Ensure your LLM service is running on `http://localhost:8080`:

```bash
cargo run
```

### 2. Dependencies Installed

```bash
npm install
```

### 3. Environment Variables

Make sure you have the necessary environment variables configured in `.env`:

```bash
GCP_PROJECT_ID=your-project-id
GCP_LOCATION=us-east5
GOOGLE_APPLICATION_CREDENTIALS=./gcp-credentials.json
```

## Running the Test

### Run All E2E Tests

```bash
npm run test:e2e
```

### Run with TypeScript Directly

```bash
tsx test-tool-calling-e2e.ts
```

### Make Executable and Run

```bash
chmod +x test-tool-calling-e2e.ts
./test-tool-calling-e2e.ts
```

## Test Scenarios

The E2E test suite includes the following scenarios:

### 1. Weather Query
- **Query**: "What is the weather like in Tokyo, Japan?"
- **Expected Tool**: `get_weather`
- **Flow**: LLM → weather tool → formatted weather data → natural answer

### 2. Mathematical Calculation
- **Query**: "What is 457 multiplied by 823?"
- **Expected Tool**: `calculate`
- **Flow**: LLM → calculator → computation → result explanation

### 3. Current Time Query
- **Query**: "What time is it right now in New York?"
- **Expected Tool**: `get_current_time`
- **Flow**: LLM → time tool → timezone conversion → readable time

### 4. Stock Price Query
- **Query**: "What is the current stock price of Apple (AAPL)?"
- **Expected Tool**: `get_stock_price`
- **Flow**: LLM → stock API → price data → market analysis

### 5. Database Search
- **Query**: "Search the database for records containing 'user authentication'"
- **Expected Tool**: `search_database`
- **Flow**: LLM → database query → results → summary

## Understanding the Output

### Successful Test Output

```
🧪 Test: Weather Query
================================================================================

📤 Step 1: User Query
   "What is the weather like in Tokyo, Japan?"

📤 Step 2: Request to LLM (with tools available)
   → Sending request to LLM...
   ← Received response from LLM

📋 Step 3: LLM Requested Tool Calls
   ✓ 1 tool call(s) requested
   • get_weather
     Args: {"location":"Tokyo, Japan","unit":"celsius"}

🔧 Step 4: Execute Tools Locally
   🔧 Executing tool: get_weather({"location":"Tokyo, Japan","unit":"celsius"})
   ✓ Result: {"location":"Tokyo, Japan","temperature":18,"unit":"celsius",...}

📤 Step 5: Send Tool Results to LLM
   → Sending request to LLM...
   ← Received response from LLM

💬 Step 6: Final Answer from LLM

   Based on the current weather data for Tokyo, Japan, it's currently 18°C 
   with partly cloudy conditions. The humidity is at 65% and there's a 
   gentle wind of about 12 km/h. It's a pleasant day in Tokyo!

✅ Test Passed (3421ms)
   • User query processed
   • 1 tool(s) executed
   • Final answer generated
```

## Available Tools

The test suite implements the following tools:

### get_weather
Gets current weather information for a location.

**Parameters**:
- `location` (required): City and country
- `unit` (optional): "celsius" or "fahrenheit"

**Returns**: Weather data including temperature, condition, humidity, wind speed

### calculate
Performs mathematical operations.

**Parameters**:
- `operation` (required): "add", "subtract", "multiply", "divide"
- `a` (required): First operand
- `b` (required): Second operand

**Returns**: Calculation result

### get_current_time
Gets current time in a specific timezone.

**Parameters**:
- `timezone` (optional): IANA timezone identifier

**Returns**: Current time with timestamp and readable format

### search_database
Searches database records.

**Parameters**:
- `query` (required): Search query
- `table` (optional): Database table name

**Returns**: Search results with relevance scores

### get_stock_price
Gets current stock price.

**Parameters**:
- `symbol` (required): Stock ticker symbol

**Returns**: Price, change, and percent change

## Troubleshooting

### Service Not Running

If you see:
```
❌ Service health check failed!
   Error: fetch failed
```

**Solution**: Start your service with `cargo run`

### Tool Not Called

If the LLM doesn't call any tools:
```
⚠️  LLM chose not to use tools
```

**Possible causes**:
- Query is too vague
- LLM has the answer without needing tools
- Tool descriptions aren't clear enough

**Solution**: Make the query more specific or adjust tool descriptions

### Invalid Tool Call

If the wrong tool is called:
```
Expected tool 'get_weather', but got 'search_database'
```

**Possible causes**:
- Query is ambiguous
- Tool descriptions overlap
- Temperature is too high (causing creative interpretations)

**Solution**: Adjust query wording or tool descriptions

### Timeout Errors

If requests timeout:
```
Error: Request timeout
```

**Solution**: 
- Check service logs for errors
- Increase timeout in the test
- Verify GCP credentials are valid

## Customizing Tests

### Add a New Test Scenario

Edit `test-tool-calling-e2e.ts` and add to the `tests` array:

```typescript
{
  name: 'Your Test Name',
  query: 'Your user query here',
  expectedTool: 'expected_tool_name'
}
```

### Add a New Tool

1. **Implement the tool** in the `TOOLS` object:

```typescript
your_tool_name: async (args: { param: string }) => {
  // Your implementation
  return JSON.stringify({ result: 'data' });
}
```

2. **Add tool definition** in `TOOL_DEFINITIONS`:

```typescript
{
  type: 'function',
  function: {
    name: 'your_tool_name',
    description: 'What your tool does',
    parameters: {
      type: 'object',
      properties: {
        param: {
          type: 'string',
          description: 'Parameter description'
        }
      },
      required: ['param']
    }
  }
}
```

## Integration with CI/CD

### GitHub Actions Example

```yaml
- name: Run E2E Tool Calling Tests
  run: |
    cargo run &
    sleep 5  # Wait for service to start
    npm run test:e2e
```

### Docker Compose

```yaml
services:
  llm-service:
    build: .
    ports:
      - "8080:8080"
    healthcheck:
      test: ["CMD", "curl", "-f", "http://localhost:8080/health"]
      interval: 5s
      timeout: 3s
      retries: 3
  
  e2e-tests:
    image: node:18
    depends_on:
      llm-service:
        condition: service_healthy
    command: npm run test:e2e
```

## Best Practices

1. **Always check service health** before running tests
2. **Use realistic tool implementations** that simulate actual API behavior
3. **Keep test queries specific** to ensure predictable tool usage
4. **Set appropriate timeouts** for different operations
5. **Log tool executions** for debugging and visibility
6. **Validate tool results** before sending to LLM
7. **Handle errors gracefully** and provide clear error messages
8. **Test both success and failure paths** in production

## Next Steps

- Add streaming support to E2E tests
- Implement parallel tool execution tests
- Add authentication and authorization tests
- Create performance benchmarks
- Add error recovery scenarios
- Test rate limiting and throttling

## Related Documentation

- [Tool Calling Guide](TOOL_CALLING_GUIDE.md)
- [Testing Guide](TEST_GUIDE.md)
- [OpenAI Compatibility](OPENAI_COMPATIBILITY.md)
- [Streaming Fix Instructions](STREAMING_FIX_INSTRUCTIONS.md)
