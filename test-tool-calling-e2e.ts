#!/usr/bin/env tsx

/**
 * End-to-End Tool Calling Test
 * 
 * This script demonstrates the complete tool calling flow:
 * 1. User asks a question that requires tool usage
 * 2. LLM responds with a tool call request
 * 3. Tool is executed locally
 * 4. Tool result is sent back to the LLM
 * 5. LLM provides a final answer using the tool result
 * 
 * Usage: npm run test:e2e or tsx test-tool-calling-e2e.ts
 */

import chalk from 'chalk';

// Use Node.js built-in fetch (Node 18+)
const fetch = globalThis.fetch;

// Configuration
const SERVICE_URL = 'http://localhost:8080';
const CHAT_ENDPOINT = `${SERVICE_URL}/v1/chat/completions`;

// Types for OpenAI API compatibility
interface ChatMessage {
  role: 'system' | 'user' | 'assistant' | 'tool';
  content: string | null;
  name?: string;
  tool_calls?: ToolCall[];
  tool_call_id?: string;
}

interface ToolCall {
  id: string;
  type: 'function';
  function: {
    name: string;
    arguments: string;
  };
}

interface ToolDefinition {
  type: 'function';
  function: {
    name: string;
    description: string;
    parameters: {
      type: 'object';
      properties: Record<string, any>;
      required?: string[];
    };
  };
}

interface ChatCompletionRequest {
  model: string;
  messages: ChatMessage[];
  temperature?: number;
  max_tokens?: number;
  stream?: boolean;
  tools?: ToolDefinition[];
  tool_choice?: 'auto' | 'none' | 'required' | { type: 'function'; function: { name: string } };
}

interface ChatCompletionResponse {
  id: string;
  object: string;
  created: number;
  model: string;
  choices: {
    index: number;
    message: {
      role: string;
      content: string | null;
      tool_calls?: ToolCall[];
    };
    finish_reason: string;
  }[];
  usage?: {
    prompt_tokens: number;
    completion_tokens: number;
    total_tokens: number;
  };
}

// Tool Implementations
// These simulate real tools that would be called in a production environment
const TOOLS: Record<string, (args: any) => Promise<string>> = {
  get_weather: async (args: { location: string; unit?: string }) => {
    console.log(chalk.blue(`   🔧 Executing tool: get_weather(${JSON.stringify(args)})`));
    await new Promise(resolve => setTimeout(resolve, 500)); // Simulate API call
    
    // Simulate realistic weather data
    const weatherConditions = ['sunny', 'partly cloudy', 'cloudy', 'rainy', 'snowy'];
    const condition = weatherConditions[Math.floor(Math.random() * weatherConditions.length)];
    const temp = Math.floor(Math.random() * 30) + 5;
    const unit = args.unit || 'celsius';
    
    return JSON.stringify({
      location: args.location,
      temperature: temp,
      unit: unit,
      condition: condition,
      humidity: Math.floor(Math.random() * 40) + 40,
      wind_speed: Math.floor(Math.random() * 20) + 5
    });
  },
  
  calculate: async (args: { operation: string; a: number; b: number }) => {
    console.log(chalk.blue(`   🔧 Executing tool: calculate(${JSON.stringify(args)})`));
    await new Promise(resolve => setTimeout(resolve, 200));
    
    let result: number;
    switch (args.operation) {
      case 'add':
        result = args.a + args.b;
        break;
      case 'subtract':
        result = args.a - args.b;
        break;
      case 'multiply':
        result = args.a * args.b;
        break;
      case 'divide':
        result = args.b !== 0 ? args.a / args.b : NaN;
        break;
      default:
        throw new Error(`Unknown operation: ${args.operation}`);
    }
    
    return JSON.stringify({
      operation: args.operation,
      operand1: args.a,
      operand2: args.b,
      result: result
    });
  },
  
  get_current_time: async (args: { timezone?: string }) => {
    console.log(chalk.blue(`   🔧 Executing tool: get_current_time(${JSON.stringify(args)})`));
    await new Promise(resolve => setTimeout(resolve, 100));
    
    const now = new Date();
    const timezone = args.timezone || 'UTC';
    
    return JSON.stringify({
      timezone: timezone,
      timestamp: now.toISOString(),
      unix_timestamp: Math.floor(now.getTime() / 1000),
      readable: now.toLocaleString('en-US', { timeZone: timezone })
    });
  },
  
  search_database: async (args: { query: string; table?: string }) => {
    console.log(chalk.blue(`   🔧 Executing tool: search_database(${JSON.stringify(args)})`));
    await new Promise(resolve => setTimeout(resolve, 300));
    
    // Simulate database search results
    const mockResults = [
      { id: 1, title: 'First Result', relevance: 0.95 },
      { id: 2, title: 'Second Result', relevance: 0.87 },
      { id: 3, title: 'Third Result', relevance: 0.72 }
    ];
    
    return JSON.stringify({
      query: args.query,
      table: args.table || 'default',
      results: mockResults,
      total_count: mockResults.length
    });
  },
  
  get_stock_price: async (args: { symbol: string }) => {
    console.log(chalk.blue(`   🔧 Executing tool: get_stock_price(${JSON.stringify(args)})`));
    await new Promise(resolve => setTimeout(resolve, 400));
    
    // Simulate stock price data
    const basePrice = 100 + Math.random() * 200;
    const change = (Math.random() - 0.5) * 10;
    
    return JSON.stringify({
      symbol: args.symbol.toUpperCase(),
      price: parseFloat(basePrice.toFixed(2)),
      change: parseFloat(change.toFixed(2)),
      percent_change: parseFloat(((change / basePrice) * 100).toFixed(2)),
      timestamp: new Date().toISOString()
    });
  }
};

// Tool Definitions for the LLM
const TOOL_DEFINITIONS: ToolDefinition[] = [
  {
    type: 'function',
    function: {
      name: 'get_weather',
      description: 'Get current weather information for a specific location',
      parameters: {
        type: 'object',
        properties: {
          location: {
            type: 'string',
            description: 'The city and state or country, e.g., "San Francisco, CA" or "London, UK"'
          },
          unit: {
            type: 'string',
            enum: ['celsius', 'fahrenheit'],
            description: 'The temperature unit to use'
          }
        },
        required: ['location']
      }
    }
  },
  {
    type: 'function',
    function: {
      name: 'calculate',
      description: 'Perform mathematical calculations',
      parameters: {
        type: 'object',
        properties: {
          operation: {
            type: 'string',
            enum: ['add', 'subtract', 'multiply', 'divide'],
            description: 'The mathematical operation to perform'
          },
          a: {
            type: 'number',
            description: 'The first operand'
          },
          b: {
            type: 'number',
            description: 'The second operand'
          }
        },
        required: ['operation', 'a', 'b']
      }
    }
  },
  {
    type: 'function',
    function: {
      name: 'get_current_time',
      description: 'Get the current time in a specific timezone',
      parameters: {
        type: 'object',
        properties: {
          timezone: {
            type: 'string',
            description: 'IANA timezone identifier (e.g., "America/New_York", "Europe/London")'
          }
        }
      }
    }
  },
  {
    type: 'function',
    function: {
      name: 'search_database',
      description: 'Search for records in the database',
      parameters: {
        type: 'object',
        properties: {
          query: {
            type: 'string',
            description: 'The search query'
          },
          table: {
            type: 'string',
            description: 'The database table to search (optional)'
          }
        },
        required: ['query']
      }
    }
  },
  {
    type: 'function',
    function: {
      name: 'get_stock_price',
      description: 'Get the current stock price for a given ticker symbol',
      parameters: {
        type: 'object',
        properties: {
          symbol: {
            type: 'string',
            description: 'The stock ticker symbol (e.g., "AAPL", "GOOGL")'
          }
        },
        required: ['symbol']
      }
    }
  }
];

// HTTP client utility
async function makeRequest(request: ChatCompletionRequest): Promise<ChatCompletionResponse> {
  console.log(chalk.gray(`\n   → Sending request to LLM...`));
  
  const response = await fetch(CHAT_ENDPOINT, {
    method: 'POST',
    headers: {
      'Content-Type': 'application/json',
      'User-Agent': 'e2e-tool-calling-test/1.0.0'
    },
    body: JSON.stringify(request)
  });

  if (!response.ok) {
    const errorText = await response.text();
    throw new Error(`HTTP ${response.status}: ${errorText}`);
  }

  const result = await response.json() as ChatCompletionResponse;
  console.log(chalk.gray(`   ← Received response from LLM`));
  
  return result;
}

// Execute a tool locally
async function executeTool(toolCall: ToolCall): Promise<string> {
  const toolName = toolCall.function.name;
  const toolFunction = TOOLS[toolName];
  
  if (!toolFunction) {
    throw new Error(`Tool '${toolName}' not found`);
  }
  
  const args = JSON.parse(toolCall.function.arguments);
  return await toolFunction(args);
}

// Main E2E test scenarios
class E2EToolCallingTest {
  
  async runTest(
    testName: string,
    userQuery: string,
    expectedToolName?: string
  ): Promise<void> {
    console.log(chalk.cyan(`\n${'='.repeat(80)}`));
    console.log(chalk.cyan(`🧪 Test: ${testName}`));
    console.log(chalk.cyan('='.repeat(80)));
    
    const startTime = Date.now();
    const messages: ChatMessage[] = [];
    
    try {
      // Step 1: Initial user query
      console.log(chalk.yellow(`\n📤 Step 1: User Query`));
      console.log(chalk.white(`   "${userQuery}"`));
      
      messages.push({
        role: 'user',
        content: userQuery
      });
      
      // Step 2: LLM decides to use a tool
      console.log(chalk.yellow(`\n📤 Step 2: Request to LLM (with tools available)`));
      
      const firstRequest: ChatCompletionRequest = {
        model: 'claude-sonnet-4@20250514',
        messages: messages,
        tools: TOOL_DEFINITIONS,
        tool_choice: 'auto',
        max_tokens: 2000,
        temperature: 0.7
      };
      
      const firstResponse = await makeRequest(firstRequest);
      
      // Check if LLM wants to use tools
      const assistantMessage = firstResponse.choices[0].message;
      
      if (!assistantMessage.tool_calls || assistantMessage.tool_calls.length === 0) {
        console.log(chalk.yellow(`\n⚠️  LLM chose not to use tools`));
        console.log(chalk.white(`   Response: "${assistantMessage.content}"`));
        throw new Error('LLM did not request any tool calls');
      }
      
      // Step 3: Display tool calls requested
      console.log(chalk.yellow(`\n📋 Step 3: LLM Requested Tool Calls`));
      console.log(chalk.green(`   ✓ ${assistantMessage.tool_calls.length} tool call(s) requested`));
      
      for (const toolCall of assistantMessage.tool_calls) {
        console.log(chalk.blue(`   • ${toolCall.function.name}`));
        console.log(chalk.gray(`     Args: ${toolCall.function.arguments}`));
        
        if (expectedToolName && toolCall.function.name !== expectedToolName) {
          throw new Error(`Expected tool '${expectedToolName}', but got '${toolCall.function.name}'`);
        }
      }
      
      // Add assistant message with tool calls to conversation
      messages.push({
        role: 'assistant',
        content: assistantMessage.content,
        tool_calls: assistantMessage.tool_calls
      });
      
      // Step 4: Execute tools locally
      console.log(chalk.yellow(`\n🔧 Step 4: Execute Tools Locally`));
      
      const toolResults: ChatMessage[] = [];
      
      for (const toolCall of assistantMessage.tool_calls) {
        const result = await executeTool(toolCall);
        console.log(chalk.green(`   ✓ Result: ${result}`));
        
        toolResults.push({
          role: 'tool',
          tool_call_id: toolCall.id,
          content: result
        });
      }
      
      // Add tool results to conversation
      messages.push(...toolResults);
      
      // Step 5: Send tool results back to LLM for final answer
      console.log(chalk.yellow(`\n📤 Step 5: Send Tool Results to LLM`));
      
      const secondRequest: ChatCompletionRequest = {
        model: 'claude-sonnet-4@20250514',
        messages: messages,
        tools: TOOL_DEFINITIONS,
        max_tokens: 2000,
        temperature: 0.7
      };
      
      const finalResponse = await makeRequest(secondRequest);
      
      // Step 6: Display final answer
      console.log(chalk.yellow(`\n💬 Step 6: Final Answer from LLM`));
      
      const finalMessage = finalResponse.choices[0].message;
      
      if (!finalMessage.content) {
        throw new Error('LLM did not provide a final answer');
      }
      
      console.log(chalk.green(`\n   ${finalMessage.content}\n`));
      
      // Success summary
      const duration = Date.now() - startTime;
      console.log(chalk.green(`\n✅ Test Passed (${duration}ms)`));
      console.log(chalk.green(`   • User query processed`));
      console.log(chalk.green(`   • ${assistantMessage.tool_calls.length} tool(s) executed`));
      console.log(chalk.green(`   • Final answer generated`));
      
    } catch (error) {
      const duration = Date.now() - startTime;
      console.log(chalk.red(`\n❌ Test Failed (${duration}ms)`));
      console.log(chalk.red(`   Error: ${error instanceof Error ? error.message : String(error)}`));
      throw error;
    }
  }
  
  async runAllTests(): Promise<void> {
    console.log(chalk.cyan('\n🚀 E2E Tool Calling Test Suite'));
    console.log(chalk.cyan('================================'));
    console.log(`Service URL: ${SERVICE_URL}\n`);
    
    let passed = 0;
    let failed = 0;
    
    const tests = [
      {
        name: 'Weather Query',
        query: 'What is the weather like in Tokyo, Japan? Please give me a detailed description.',
        expectedTool: 'get_weather'
      },
      {
        name: 'Mathematical Calculation',
        query: 'What is 457 multiplied by 823? Show me the exact result.',
        expectedTool: 'calculate'
      },
      {
        name: 'Current Time Query',
        query: 'What time is it right now in New York?',
        expectedTool: 'get_current_time'
      },
      {
        name: 'Stock Price Query',
        query: 'What is the current stock price of Apple (AAPL)?',
        expectedTool: 'get_stock_price'
      },
      {
        name: 'Database Search',
        query: 'Search the database for records containing "user authentication"',
        expectedTool: 'search_database'
      }
    ];
    
    for (const test of tests) {
      try {
        await this.runTest(test.name, test.query, test.expectedTool);
        passed++;
      } catch (error) {
        failed++;
        console.error(chalk.red(`\nTest "${test.name}" failed:`, error));
      }
    }
    
    // Final summary
    console.log(chalk.cyan(`\n${'='.repeat(80)}`));
    console.log(chalk.cyan('📊 Test Summary'));
    console.log(chalk.cyan('='.repeat(80)));
    console.log(`Total tests: ${passed + failed}`);
    console.log(chalk.green(`Passed: ${passed}`));
    console.log(chalk.red(`Failed: ${failed}`));
    
    if (failed === 0) {
      console.log(chalk.green('\n🎉 All tests passed!'));
    } else {
      console.log(chalk.red(`\n❌ ${failed} test(s) failed`));
      process.exit(1);
    }
  }
}

// Health check
async function checkServiceHealth(): Promise<void> {
  console.log(chalk.blue('🏥 Checking service health...'));
  
  try {
    const response = await fetch(`${SERVICE_URL}/health`, {
      signal: AbortSignal.timeout(5000)
    });
    
    if (!response.ok) {
      throw new Error(`Health check failed: ${response.status}`);
    }
    
    console.log(chalk.green('✓ Service is healthy\n'));
  } catch (error) {
    console.log(chalk.red('❌ Service health check failed!'));
    console.log(chalk.red(`   Error: ${error instanceof Error ? error.message : String(error)}`));
    console.log(chalk.yellow('\n💡 Make sure the service is running on http://localhost:8080'));
    console.log(chalk.yellow('   Start with: cargo run\n'));
    process.exit(1);
  }
}

// Main execution
async function main(): Promise<void> {
  try {
    await checkServiceHealth();
    
    const testRunner = new E2EToolCallingTest();
    await testRunner.runAllTests();
    
  } catch (error) {
    console.error(chalk.red('\n💥 Test execution failed:'), error);
    process.exit(1);
  }
}

// Run tests if this file is executed directly
if (import.meta.url === `file://${process.argv[1]}`) {
  main().catch(console.error);
}

export { E2EToolCallingTest, TOOLS, TOOL_DEFINITIONS };
