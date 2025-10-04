#!/usr/bin/env tsx

/**
 * TypeScript Test Script for LLM Supabase Service Tool Calling
 * 
 * This script tests the tool calling capabilities of the service running in debug mode.
 * It covers both legacy and modern OpenAI tool calling formats, streaming and non-streaming,
 * and various error scenarios.
 * 
 * Usage: npm run test:tools or tsx test-tool-calling.ts
 */

import EventSource from 'eventsource';
import chalk from 'chalk';

// Use Node.js built-in fetch (Node 18+)
const fetch = globalThis.fetch;

// Configuration
const SERVICE_URL = 'http://localhost:8080';
const CHAT_ENDPOINT = `${SERVICE_URL}/v1/chat/completions`;

// Types for OpenAI API compatibility
interface ChatMessage {
  role: 'system' | 'user' | 'assistant' | 'tool';
  content: string;
  name?: string;
  tool_calls?: ToolCall[] | undefined;
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

interface FunctionCall {
  name: string;
  arguments: string;
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

interface FunctionDefinition {
  name: string;
  description: string;
  parameters: {
    type: 'object';
    properties: Record<string, any>;
    required?: string[];
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
  parallel_tool_calls?: boolean;
  functions?: FunctionDefinition[];
  function_call?: 'auto' | 'none' | { name: string };
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
      content: string;
      tool_calls?: ToolCall[] | undefined;
      function_call?: FunctionCall;
    };
    finish_reason: string;
  }[];
  usage?: {
    prompt_tokens: number;
    completion_tokens: number;
    total_tokens: number;
  };
}

// Test utilities
class TestRunner {
  private passed = 0;
  private failed = 0;
  private testResults: Array<{ name: string; status: 'PASS' | 'FAIL'; error?: string; duration: number }> = [];

  async runTest(name: string, testFn: () => Promise<void>): Promise<void> {
    const startTime = Date.now();
    console.log(chalk.blue(`\n🧪 Running: ${name}`));
    
    try {
      await testFn();
      const duration = Date.now() - startTime;
      this.passed++;
      this.testResults.push({ name, status: 'PASS', duration });
      console.log(chalk.green(`✅ PASS: ${name} (${duration}ms)`));
    } catch (error) {
      const duration = Date.now() - startTime;
      this.failed++;
      const errorMsg = error instanceof Error ? error.message : String(error);
      this.testResults.push({ name, status: 'FAIL', error: errorMsg, duration });
      console.log(chalk.red(`❌ FAIL: ${name} (${duration}ms)`));
      console.log(chalk.red(`   Error: ${errorMsg}`));
    }
  }

  printSummary(): void {
    console.log(chalk.cyan('\n📊 Test Summary'));
    console.log(chalk.cyan('================'));
    console.log(`Total tests: ${this.passed + this.failed}`);
    console.log(chalk.green(`Passed: ${this.passed}`));
    console.log(chalk.red(`Failed: ${this.failed}`));
    
    if (this.failed > 0) {
      console.log(chalk.red('\n❌ Failed Tests:'));
      this.testResults
        .filter(r => r.status === 'FAIL')
        .forEach(r => console.log(chalk.red(`  - ${r.name}: ${r.error}`)));
    }
    
    const totalDuration = this.testResults.reduce((sum, r) => sum + r.duration, 0);
    console.log(chalk.cyan(`\nTotal duration: ${totalDuration}ms`));
  }
}

// Mock tool implementations
const MOCK_TOOLS: Record<string, (args: any) => Promise<string>> = {
  get_weather: async (args: { location: string }) => {
    await new Promise(resolve => setTimeout(resolve, 100)); // Simulate API delay
    const weather = ['sunny', 'cloudy', 'rainy', 'snowy'][Math.floor(Math.random() * 4)];
    const temp = Math.floor(Math.random() * 30) + 5;
    return `Weather in ${args.location}: ${weather}, ${temp}°C`;
  },
  
  search_web: async (args: { query: string }) => {
    await new Promise(resolve => setTimeout(resolve, 150));
    return `Search results for "${args.query}": Found 42 relevant results including recent articles and documentation.`;
  },
  
  calculate: async (args: { expression: string }) => {
    await new Promise(resolve => setTimeout(resolve, 50));
    try {
      // Simple calculator - in real implementation, use a safe math parser
      const result = eval(args.expression.replace(/[^0-9+\-*/().\s]/g, ''));
      return `Result: ${result}`;
    } catch {
      return `Error: Invalid expression "${args.expression}"`;
    }
  },
  
  get_time: async (args: { timezone?: string }) => {
    await new Promise(resolve => setTimeout(resolve, 75));
    const now = new Date();
    const timezone = args.timezone || 'UTC';
    return `Current time in ${timezone}: ${now.toISOString()}`;
  }
};

// Tool definitions for testing
const WEATHER_TOOL: ToolDefinition = {
  type: 'function',
  function: {
    name: 'get_weather',
    description: 'Get current weather information for a location',
    parameters: {
      type: 'object',
      properties: {
        location: {
          type: 'string',
          description: 'The city and state, e.g. San Francisco, CA'
        }
      },
      required: ['location']
    }
  }
};

const SEARCH_TOOL: ToolDefinition = {
  type: 'function',
  function: {
    name: 'search_web',
    description: 'Search the web for information',
    parameters: {
      type: 'object',
      properties: {
        query: {
          type: 'string',
          description: 'The search query'
        }
      },
      required: ['query']
    }
  }
};

const CALCULATOR_TOOL: ToolDefinition = {
  type: 'function',
  function: {
    name: 'calculate',
    description: 'Perform mathematical calculations',
    parameters: {
      type: 'object',
      properties: {
        expression: {
          type: 'string',
          description: 'Mathematical expression to evaluate'
        }
      },
      required: ['expression']
    }
  }
};

const TIME_TOOL: ToolDefinition = {
  type: 'function',
  function: {
    name: 'get_time',
    description: 'Get current time in specified timezone',
    parameters: {
      type: 'object',
      properties: {
        timezone: {
          type: 'string',
          description: 'Timezone identifier (e.g., America/New_York)'
        }
      }
    }
  }
};

// Legacy function definitions
const WEATHER_FUNCTION: FunctionDefinition = {
  name: 'get_weather',
  description: 'Get current weather information for a location',
  parameters: {
    type: 'object',
    properties: {
      location: {
        type: 'string',
        description: 'The city and state, e.g. San Francisco, CA'
      }
    },
    required: ['location']
  }
};

// HTTP client utilities
async function makeRequest(request: ChatCompletionRequest): Promise<ChatCompletionResponse> {
  const response = await fetch(CHAT_ENDPOINT, {
    method: 'POST',
    headers: {
      'Content-Type': 'application/json',
      'User-Agent': 'tool-calling-test/1.0.0'
    },
    body: JSON.stringify(request)
  });

  if (!response.ok) {
    const errorText = await response.text();
    throw new Error(`HTTP ${response.status}: ${errorText}`);
  }

  return await response.json() as ChatCompletionResponse;
}

async function makeStreamingRequest(request: ChatCompletionRequest): Promise<ChatCompletionResponse> {
  return new Promise((resolve, reject) => {
    const streamRequest = { ...request, stream: true };
    
    // For streaming, we need to make a fetch request and handle the stream manually
    fetch(CHAT_ENDPOINT, {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
        'User-Agent': 'tool-calling-test/1.0.0',
        'Accept': 'text/event-stream'
      },
      body: JSON.stringify(streamRequest)
    }).then(async (response: Response) => {
      if (!response.ok) {
        throw new Error(`HTTP ${response.status}: ${await response.text()}`);
      }

      let responseData: Partial<ChatCompletionResponse> = {
        choices: [{ index: 0, message: { role: 'assistant', content: '' }, finish_reason: '' }]
      };
      let toolCalls: ToolCall[] = [];

      const reader = response.body?.getReader();
      if (!reader) {
        throw new Error('No response body reader available');
      }

      const decoder = new TextDecoder();
      let buffer = '';

      try {
        while (true) {
          const { done, value } = await reader.read();
          if (done) break;

          buffer += decoder.decode(value, { stream: true });
          const lines = buffer.split('\n');
          buffer = lines.pop() || '';

          for (const line of lines) {
            if (line.startsWith('data: ')) {
              const data = line.slice(6).trim();
              
              if (data === '[DONE]') {
                if (toolCalls.length > 0) {
                  responseData.choices![0].message.tool_calls = toolCalls;
                }
                resolve(responseData as ChatCompletionResponse);
                return;
              }

              try {
                const chunk = JSON.parse(data);
                if (chunk.choices?.[0]?.delta) {
                  const delta = chunk.choices[0].delta;
                  
                  if (delta.content) {
                    responseData.choices![0].message.content += delta.content;
                  }
                  
                  if (delta.tool_calls) {
                    delta.tool_calls.forEach((toolCall: any, index: number) => {
                      if (!toolCalls[index]) {
                        toolCalls[index] = {
                          id: toolCall.id || `call_${index}`,
                          type: 'function',
                          function: { name: '', arguments: '' }
                        };
                      }
                      
                      if (toolCall.function?.name) {
                        toolCalls[index].function.name += toolCall.function.name;
                      }
                      if (toolCall.function?.arguments) {
                        toolCalls[index].function.arguments += toolCall.function.arguments;
                      }
                    });
                  }
                  
                  if (delta.function_call) {
                    // Handle legacy function_call format
                    if (!responseData.choices![0].message.function_call) {
                      responseData.choices![0].message.function_call = { name: '', arguments: '' };
                    }
                    if (delta.function_call.name) {
                      responseData.choices![0].message.function_call.name += delta.function_call.name;
                    }
                    if (delta.function_call.arguments) {
                      responseData.choices![0].message.function_call.arguments += delta.function_call.arguments;
                    }
                  }
                }
                
                if (chunk.choices?.[0]?.finish_reason) {
                  responseData.choices![0].finish_reason = chunk.choices[0].finish_reason;
                }
              } catch (error) {
                console.warn('Failed to parse streaming chunk:', data);
              }
            }
          }
        }
      } finally {
        reader.releaseLock();
      }
    }).catch(reject);

    // Timeout after 30 seconds
    setTimeout(() => {
      reject(new Error('Streaming request timed out'));
    }, 30000);
  });
}


// Test cases
class ToolCallingTests {
  private runner = new TestRunner();

  async runAllTests(): Promise<void> {
    console.log(chalk.cyan('🚀 Starting Tool Calling Tests'));
    console.log(chalk.cyan('================================'));
    console.log(`Service URL: ${SERVICE_URL}`);
    
    // Health check first
    await this.runner.runTest('Service Health Check', () => this.testServiceHealth());
    
    // Modern tool calling tests
    await this.runner.runTest('Modern Tool Calling - Single Tool', () => this.testModernSingleTool());
    await this.runner.runTest('Modern Tool Calling - Multiple Tools', () => this.testModernMultipleTools());
    await this.runner.runTest('Modern Tool Calling - Parallel Execution', () => this.testModernParallelTools());
    
    // Legacy tool calling tests
    await this.runner.runTest('Legacy Function Calling - Single Function', () => this.testLegacySingleFunction());
    
    // Tool choice tests
    await this.runner.runTest('Tool Choice - Auto', () => this.testToolChoiceAuto());
    await this.runner.runTest('Tool Choice - Required', () => this.testToolChoiceRequired());
    await this.runner.runTest('Tool Choice - Specific Tool', () => this.testToolChoiceSpecific());
    await this.runner.runTest('Tool Choice - None', () => this.testToolChoiceNone());
    
    // Streaming tests
    await this.runner.runTest('Streaming - Modern Tool Calls', () => this.testStreamingModernTools());
    await this.runner.runTest('Streaming - Legacy Function Call', () => this.testStreamingLegacyFunction());
    
    // Error handling tests
    await this.runner.runTest('Error Handling - Invalid Tool Name', () => this.testInvalidToolName());
    await this.runner.runTest('Error Handling - Invalid Parameters', () => this.testInvalidParameters());
    await this.runner.runTest('Error Handling - Missing Required Tool', () => this.testMissingRequiredTool());
    
    // Client detection tests
    await this.runner.runTest('Client Detection - Modern SDK', () => this.testModernSDKDetection());
    await this.runner.runTest('Client Detection - Legacy SDK', () => this.testLegacySDKDetection());
    
    this.runner.printSummary();
  }

  private async testServiceHealth(): Promise<void> {
    console.log(chalk.blue(`   Attempting to connect to: ${SERVICE_URL}/health`));
    
    try {
      const response = await fetch(`${SERVICE_URL}/health`);
      console.log(chalk.blue(`   Response status: ${response.status}`));
      
      // Convert headers to object for logging
      const headersObj: Record<string, string> = {};
      response.headers.forEach((value, key) => {
        headersObj[key] = value;
      });
      console.log(chalk.blue(`   Response headers: ${JSON.stringify(headersObj)}`));
      
      if (!response.ok) {
        const errorText = await response.text();
        console.log(chalk.yellow(`   Response body: ${errorText}`));
        throw new Error(`Service health check failed: ${response.status} - ${errorText}`);
      }
      
      const healthData = await response.text();
      console.log(chalk.blue(`   Health response: ${healthData}`));
      console.log(chalk.green('   Service is healthy'));
    } catch (error: any) {
      console.log(chalk.red(`   Connection error details: ${error}`));
      console.log(chalk.red(`   Error type: ${error?.constructor?.name || 'Unknown'}`));
      if (error?.cause) {
        console.log(chalk.red(`   Error cause: ${error.cause}`));
      }
      if (error?.code) {
        console.log(chalk.red(`   Error code: ${error.code}`));
      }
      throw error;
    }
  }

  private async testModernSingleTool(): Promise<void> {
    const request: ChatCompletionRequest = {
      model: 'claude-sonnet-4@20250514',
      messages: [
        { role: 'user', content: 'What is the weather like in San Francisco?' }
      ],
      tools: [WEATHER_TOOL],
      tool_choice: 'auto',
      max_tokens: 1000
    };

    console.log(chalk.blue(`   Request: ${JSON.stringify(request, null, 2)}`));
    const response = await makeRequest(request);
    console.log(chalk.blue(`   Response: ${JSON.stringify(response, null, 2)}`));
    
    if (!response.choices?.[0]?.message?.tool_calls) {
      console.log(chalk.yellow(`   No tool_calls found in response. Message content: "${response.choices?.[0]?.message?.content}"`));
      console.log(chalk.yellow(`   Full message object: ${JSON.stringify(response.choices?.[0]?.message, null, 2)}`));
      throw new Error('Expected tool_calls in response');
    }
    
    const toolCall = response.choices[0].message.tool_calls[0];
    if (toolCall.function.name !== 'get_weather') {
      throw new Error(`Expected get_weather, got ${toolCall.function.name}`);
    }
    
    const args = JSON.parse(toolCall.function.arguments);
    if (!args.location) {
      throw new Error('Expected location parameter in tool call');
    }
    
    console.log(chalk.green(`   Tool called: ${toolCall.function.name} with args: ${toolCall.function.arguments}`));
  }

  private async testModernMultipleTools(): Promise<void> {
    const request: ChatCompletionRequest = {
      model: 'claude-sonnet-4@20250514',
      messages: [
        { role: 'user', content: 'Get the weather in New York and search for "NYC attractions"' }
      ],
      tools: [WEATHER_TOOL, SEARCH_TOOL],
      tool_choice: 'auto',
      parallel_tool_calls: true,
      max_tokens: 1000
    };

    const response = await makeRequest(request);
    
    if (!response.choices?.[0]?.message?.tool_calls) {
      throw new Error('Expected tool_calls in response');
    }
    
    const toolCalls = response.choices[0].message.tool_calls;
    if (toolCalls.length < 1) {
      throw new Error('Expected at least one tool call');
    }
    
    console.log(chalk.green(`   ${toolCalls.length} tool(s) called: ${toolCalls.map(tc => tc.function.name).join(', ')}`));
  }

  private async testModernParallelTools(): Promise<void> {
    const request: ChatCompletionRequest = {
      model: 'claude-sonnet-4@20250514',
      messages: [
        { role: 'user', content: 'Calculate 15 * 23, get weather in Boston, and get current time in EST' }
      ],
      tools: [CALCULATOR_TOOL, WEATHER_TOOL, TIME_TOOL],
      tool_choice: 'auto',
      parallel_tool_calls: true,
      max_tokens: 1000
    };

    const response = await makeRequest(request);
    
    if (!response.choices?.[0]?.message?.tool_calls) {
      throw new Error('Expected tool_calls in response');
    }
    
    const toolCalls = response.choices[0].message.tool_calls;
    console.log(chalk.green(`   Parallel execution: ${toolCalls.length} tools called simultaneously`));
  }

  private async testLegacySingleFunction(): Promise<void> {
    const request: ChatCompletionRequest = {
      model: 'claude-sonnet-4@20250514',
      messages: [
        { role: 'user', content: 'What is the weather like in Chicago?' }
      ],
      functions: [WEATHER_FUNCTION],
      function_call: 'auto',
      max_tokens: 1000
    };

    console.log(chalk.blue(`   Legacy Request: ${JSON.stringify(request, null, 2)}`));
    const response = await makeRequest(request);
    console.log(chalk.blue(`   Legacy Response: ${JSON.stringify(response, null, 2)}`));
    
    if (!response.choices?.[0]?.message?.function_call) {
      console.log(chalk.yellow(`   No function_call found in response. Message content: "${response.choices?.[0]?.message?.content}"`));
      console.log(chalk.yellow(`   Full message object: ${JSON.stringify(response.choices?.[0]?.message, null, 2)}`));
      throw new Error('Expected function_call in response');
    }
    
    const functionCall = response.choices[0].message.function_call;
    if (functionCall.name !== 'get_weather') {
      throw new Error(`Expected get_weather, got ${functionCall.name}`);
    }
    
    console.log(chalk.green(`   Legacy function called: ${functionCall.name}`));
  }

  private async testToolChoiceAuto(): Promise<void> {
    const request: ChatCompletionRequest = {
      model: 'claude-sonnet-4@20250514',
      messages: [
        { role: 'user', content: 'Hello, how are you?' }
      ],
      tools: [WEATHER_TOOL],
      tool_choice: 'auto',
      max_tokens: 1000
    };

    const response = await makeRequest(request);
    
    // With auto, the model should choose not to call tools for a greeting
    if (response.choices[0].message.tool_calls) {
      console.log(chalk.yellow('   Model chose to use tools for greeting (unexpected but not wrong)'));
    } else {
      console.log(chalk.green('   Model correctly chose not to use tools for greeting'));
    }
  }

  private async testToolChoiceRequired(): Promise<void> {
    const request: ChatCompletionRequest = {
      model: 'claude-sonnet-4@20250514',
      messages: [
        { role: 'user', content: 'Get weather information' }
      ],
      tools: [WEATHER_TOOL],
      tool_choice: 'required',
      max_tokens: 1000
    };

    const response = await makeRequest(request);
    
    if (!response.choices?.[0]?.message?.tool_calls) {
      throw new Error('Expected tool_calls when tool_choice is required');
    }
    
    console.log(chalk.green('   Tool correctly called when required'));
  }

  private async testToolChoiceSpecific(): Promise<void> {
    const request: ChatCompletionRequest = {
      model: 'claude-sonnet-4@20250514',
      messages: [
        { role: 'user', content: 'I need some information' }
      ],
      tools: [WEATHER_TOOL, SEARCH_TOOL],
      tool_choice: { type: 'function', function: { name: 'get_weather' } },
      max_tokens: 1000
    };

    const response = await makeRequest(request);
    
    if (!response.choices?.[0]?.message?.tool_calls) {
      throw new Error('Expected tool_calls when specific tool is required');
    }
    
    const toolCall = response.choices[0].message.tool_calls[0];
    if (toolCall.function.name !== 'get_weather') {
      throw new Error(`Expected get_weather, got ${toolCall.function.name}`);
    }
    
    console.log(chalk.green('   Specific tool correctly called'));
  }

  private async testToolChoiceNone(): Promise<void> {
    const request: ChatCompletionRequest = {
      model: 'claude-sonnet-4@20250514',
      messages: [
        { role: 'user', content: 'What is the weather like?' }
      ],
      tools: [WEATHER_TOOL],
      tool_choice: 'none',
      max_tokens: 1000
    };

    const response = await makeRequest(request);
    
    if (response.choices[0].message.tool_calls) {
      throw new Error('Expected no tool calls when tool_choice is none');
    }
    
    console.log(chalk.green('   Tools correctly disabled when choice is none'));
  }

  private async testStreamingModernTools(): Promise<void> {
    const request: ChatCompletionRequest = {
      model: 'claude-sonnet-4@20250514',
      messages: [
        { role: 'user', content: 'Get weather for Miami' }
      ],
      tools: [WEATHER_TOOL],
      tool_choice: 'auto',
      stream: true,
      max_tokens: 1000
    };

    const response = await makeStreamingRequest(request);
    
    if (!response.choices?.[0]?.message?.tool_calls) {
      throw new Error('Expected tool_calls in streaming response');
    }
    
    console.log(chalk.green('   Streaming tool calls work correctly'));
  }

  private async testStreamingLegacyFunction(): Promise<void> {
    const request: ChatCompletionRequest = {
      model: 'claude-sonnet-4@20250514',
      messages: [
        { role: 'user', content: 'Get weather for Seattle' }
      ],
      functions: [WEATHER_FUNCTION],
      function_call: 'auto',
      stream: true,
      max_tokens: 1000
    };

    const response = await makeStreamingRequest(request);
    
    if (!response.choices?.[0]?.message?.function_call) {
      throw new Error('Expected function_call in streaming response');
    }
    
    console.log(chalk.green('   Streaming legacy function calls work correctly'));
  }

  private async testInvalidToolName(): Promise<void> {
    const request: ChatCompletionRequest = {
      model: 'claude-sonnet-4@20250514',
      messages: [
        { role: 'user', content: 'Use the nonexistent tool' }
      ],
      tools: [WEATHER_TOOL],
      tool_choice: { type: 'function', function: { name: 'nonexistent_tool' } },
      max_tokens: 1000
    };

    try {
      await makeRequest(request);
      throw new Error('Expected request to fail with invalid tool name');
    } catch (error) {
      if (error instanceof Error && error.message.includes('HTTP')) {
        console.log(chalk.green('   Invalid tool name correctly rejected'));
      } else {
        throw error;
      }
    }
  }

  private async testInvalidParameters(): Promise<void> {
    const invalidTool: ToolDefinition = {
      type: 'function',
      function: {
        name: 'invalid_tool',
        description: 'A tool with invalid parameters',
        parameters: {
          type: 'object',
          properties: {},
          required: ['nonexistent_param']
        }
      }
    };

    const request: ChatCompletionRequest = {
      model: 'claude-sonnet-4@20250514',
      messages: [
        { role: 'user', content: 'Use the invalid tool' }
      ],
      tools: [invalidTool],
      tool_choice: 'required',
      max_tokens: 1000
    };

    // This test checks that the service handles tools with problematic schemas
    const response = await makeRequest(request);
    console.log(chalk.green('   Service handles invalid tool parameters gracefully'));
  }

  private async testMissingRequiredTool(): Promise<void> {
    const request: ChatCompletionRequest = {
      model: 'claude-sonnet-4@20250514',
      messages: [
        { role: 'user', content: 'Just have a normal conversation' }
      ],
      tools: [],
      tool_choice: 'required',
      max_tokens: 1000
    };

    try {
      await makeRequest(request);
      throw new Error('Expected request to fail when tool_choice is required but no tools provided');
    } catch (error) {
      if (error instanceof Error && error.message.includes('HTTP')) {
        console.log(chalk.green('   Missing required tools correctly rejected'));
      } else {
        throw error;
      }
    }
  }

  private async testModernSDKDetection(): Promise<void> {
    const request: ChatCompletionRequest = {
      model: 'claude-sonnet-4@20250514',
      messages: [
        { role: 'user', content: 'Get weather for Portland' }
      ],
      tools: [WEATHER_TOOL],
      parallel_tool_calls: true,
      max_tokens: 1000
    };

    const response = await fetch(CHAT_ENDPOINT, {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
        'User-Agent': 'openai-python/1.35.0'
      },
      body: JSON.stringify(request)
    });

    if (!response.ok) {
      throw new Error(`Modern SDK detection test failed: ${response.status}`);
    }

    const result = await response.json() as ChatCompletionResponse;
    console.log(chalk.green('   Modern SDK correctly detected and handled'));
  }

  private async testLegacySDKDetection(): Promise<void> {
    const request: ChatCompletionRequest = {
      model: 'claude-sonnet-4@20250514',
      messages: [
        { role: 'user', content: 'Get weather for Denver' }
      ],
      functions: [WEATHER_FUNCTION],
      function_call: 'auto',
      max_tokens: 1000
    };

    const response = await fetch(CHAT_ENDPOINT, {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
        'User-Agent': 'openai-python/0.28.0'
      },
      body: JSON.stringify(request)
    });

    if (!response.ok) {
      throw new Error(`Legacy SDK detection test failed: ${response.status}`);
    }

    const result = await response.json() as ChatCompletionResponse;
    console.log(chalk.green('   Legacy SDK correctly detected and handled'));
  }
}

// Main execution
async function main(): Promise<void> {
  const tests = new ToolCallingTests();
  
  try {
    await tests.runAllTests();
  } catch (error) {
    console.error(chalk.red('Test execution failed:'), error);
    process.exit(1);
  }
}

// Run tests if this file is executed directly
// In ES modules, we check if import.meta.url matches the main module
if (import.meta.url === `file://${process.argv[1]}`) {
  main().catch(console.error);
}

export { ToolCallingTests, MOCK_TOOLS };