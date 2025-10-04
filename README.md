# LLM Supabase Tool Calling Test Suite

This TypeScript test suite comprehensively tests the tool calling capabilities of your LLM Supabase service running in debug mode.

## Features

- **Comprehensive Tool Calling Tests**: Tests both modern (`tools`/`tool_calls`) and legacy (`functions`/`function_call`) OpenAI API formats
- **Streaming Support**: Tests both streaming and non-streaming responses
- **Client Detection**: Tests adaptive behavior based on User-Agent headers
- **Tool Choice Validation**: Tests `auto`, `none`, `required`, and specific tool choice strategies
- **Error Handling**: Tests various error scenarios and edge cases
- **Parallel Execution**: Tests parallel tool execution with sequential presentation for legacy clients
- **Mock Tool Implementations**: Includes realistic mock tools for testing

## Prerequisites

1. **Service Running**: Your LLM Supabase service should be running on `http://localhost:8080`
2. **Node.js**: Version 18 or higher
3. **Dependencies**: Install using npm or yarn

## Installation

```bash
# Install dependencies
npm install

# Or using yarn
yarn install
```

## Usage

### Run All Tests

```bash
# Using npm
npm run test

# Or using tsx directly
npx tsx test-tool-calling.ts

# Or make it executable and run directly
chmod +x test-tool-calling.ts
./test-tool-calling.ts
```

### Watch Mode (for development)

```bash
npm run test:watch
```

## Test Categories

### 1. Modern Tool Calling Tests
- Single tool execution
- Multiple tool execution
- Parallel tool execution with batch presentation

### 2. Legacy Function Calling Tests
- Single function execution
- Sequential presentation for legacy clients

### 3. Tool Choice Strategy Tests
- `tool_choice: "auto"` - Model decides whether to use tools
- `tool_choice: "none"` - Tools disabled, text-only response
- `tool_choice: "required"` - Model must use at least one tool
- `tool_choice: { type: "function", function: { name: "specific_tool" } }` - Specific tool required

### 4. Streaming Tests
- Modern streaming with `tool_calls` array
- Legacy streaming with `function_call` object
- Proper SSE (Server-Sent Events) handling

### 5. Client Detection Tests
- Modern SDK detection (OpenAI Python v1.x)
- Legacy SDK detection (OpenAI Python v0.x)
- Adaptive response formatting based on client capabilities

### 6. Error Handling Tests
- Invalid tool names
- Invalid parameters
- Missing required tools
- Malformed requests

## Mock Tools

The test suite includes several mock tools for comprehensive testing:

### `get_weather`
```typescript
// Gets weather information for a location
{
  "location": "San Francisco, CA"
}
// Returns: "Weather in San Francisco, CA: sunny, 22°C"
```

### `search_web`
```typescript
// Searches the web for information
{
  "query": "TypeScript best practices"
}
// Returns: "Search results for 'TypeScript best practices': Found 42 relevant results..."
```

### `calculate`
```typescript
// Performs mathematical calculations
{
  "expression": "15 * 23 + 10"
}
// Returns: "Result: 355"
```

### `get_time`
```typescript
// Gets current time in specified timezone
{
  "timezone": "America/New_York"
}
// Returns: "Current time in America/New_York: 2024-01-15T10:30:00.000Z"
```

## Configuration

The test script uses the following default configuration:

```typescript
const SERVICE_URL = 'http://localhost:8080';
const CHAT_ENDPOINT = `${SERVICE_URL}/v1/chat/completions`;
```

To test against a different service URL, modify the `SERVICE_URL` constant in `test-tool-calling.ts`.

## Expected Service Behavior

### Modern Client (OpenAI SDK v1.x)
- Supports `tools` array and `tool_calls` response format
- Supports `parallel_tool_calls: true`
- Returns all tool calls in a single response

### Legacy Client (OpenAI SDK v0.x)
- Uses `functions` array and `function_call` response format
- Does not support parallel execution
- Returns one function call per response, queues remaining calls

### Adaptive Behavior
The service should automatically detect client capabilities and adapt:
- **Request Analysis**: Examines `functions` vs `tools`, `parallel_tool_calls` parameter
- **User-Agent Detection**: Identifies SDK version from headers
- **Response Formatting**: Returns appropriate format based on detected capabilities

## Test Output

The test suite provides detailed output including:

```
🚀 Starting Tool Calling Tests
================================
Service URL: http://localhost:8080

🧪 Running: Service Health Check
✅ PASS: Service Health Check (45ms)
   Service is healthy

🧪 Running: Modern Tool Calling - Single Tool
✅ PASS: Modern Tool Calling - Single Tool (234ms)
   Tool called: get_weather with args: {"location":"San Francisco"}

🧪 Running: Modern Tool Calling - Multiple Tools
✅ PASS: Modern Tool Calling - Multiple Tools (456ms)
   2 tool(s) called: get_weather, search_web

📊 Test Summary
================
Total tests: 15
Passed: 15
Failed: 0

Total duration: 3456ms
```

## Troubleshooting

### Service Not Running
```
❌ FAIL: Service Health Check
   Error: HTTP 500: connect ECONNREFUSED 127.0.0.1:8080
```
**Solution**: Ensure your service is running on port 8080

### Tool Not Recognized
```
❌ FAIL: Modern Tool Calling - Single Tool
   Error: HTTP 400: Tool 'get_weather' not found
```
**Solution**: Check that your service properly handles the tool definitions in the request

### Streaming Issues
```
❌ FAIL: Streaming - Modern Tool Calls
   Error: Streaming request timed out
```
**Solution**: Check that your service properly implements Server-Sent Events for streaming responses

## Development

### Adding New Tests

To add new test cases, extend the `ToolCallingTests` class:

```typescript
private async testCustomScenario(): Promise<void> {
  const request: ChatCompletionRequest = {
    model: 'claude-sonnet-4@20250514',
    messages: [{ role: 'user', content: 'Your test prompt' }],
    tools: [/* your tools */],
    // ... other parameters
  };

  const response = await makeRequest(request);
  
  // Your assertions
  if (!response.choices?.[0]?.message?.tool_calls) {
    throw new Error('Expected tool calls');
  }
  
  console.log(chalk.green('   Custom test passed'));
}
```

Then add it to the `runAllTests` method:

```typescript
await this.runner.runTest('Custom Scenario', () => this.testCustomScenario());
```

### Adding New Mock Tools

Add new tools to the `MOCK_TOOLS` object and create corresponding `ToolDefinition` objects:

```typescript
const MOCK_TOOLS: Record<string, (args: any) => Promise<string>> = {
  // ... existing tools
  your_new_tool: async (args: { param: string }) => {
    // Your implementation
    return `Result: ${args.param}`;
  }
};

const YOUR_NEW_TOOL: ToolDefinition = {
  type: 'function',
  function: {
    name: 'your_new_tool',
    description: 'Description of your tool',
    parameters: {
      type: 'object',
      properties: {
        param: { type: 'string', description: 'Parameter description' }
      },
      required: ['param']
    }
  }
};
```

## License

MIT License - Feel free to use and modify for your testing needs.