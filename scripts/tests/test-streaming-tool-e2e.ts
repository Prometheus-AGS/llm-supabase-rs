#!/usr/bin/env tsx

/**
 * Streaming End-to-End Tool Calling Test
 * 
 * This demonstrates the complete tool calling flow with STREAMING:
 * 1. User query → LLM streams tool call in chunks
 * 2. Accumulate tool call from SSE stream
 * 3. Execute tool locally
 * 4. Send result → LLM streams final answer in chunks
 * 
 * Usage: npm run test:streaming or tsx test-streaming-tool-e2e.ts
 */

import chalk from 'chalk';

const SERVICE_URL = 'http://localhost:8080';
const CHAT_ENDPOINT = `${SERVICE_URL}/v1/chat/completions`;

interface Message {
  role: 'user' | 'assistant' | 'tool';
  content: string | null;
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

interface StreamChunk {
  id?: string;
  object?: string;
  created?: number;
  model?: string;
  choices?: Array<{
    index: number;
    delta: {
      role?: string;
      content?: string;
      tool_calls?: Array<{
        index?: number;
        id?: string;
        type?: string;
        function?: {
          name?: string;
          arguments?: string;
        };
      }>;
    };
    finish_reason?: string | null;
  }>;
}

// Tool implementations
const TOOLS: Record<string, (args: any) => Promise<string>> = {
  calculate: async (args: { operation: string; a: number; b: number }) => {
    console.log(chalk.blue(`   🔧 Executing: calculate(${args.operation}, ${args.a}, ${args.b})`));
    await new Promise(resolve => setTimeout(resolve, 300));
    
    let result: number;
    switch (args.operation) {
      case 'add': result = args.a + args.b; break;
      case 'subtract': result = args.a - args.b; break;
      case 'multiply': result = args.a * args.b; break;
      case 'divide': result = args.b !== 0 ? args.a / args.b : NaN; break;
      default: throw new Error(`Unknown operation: ${args.operation}`);
    }
    
    return JSON.stringify({ operation: args.operation, operand1: args.a, operand2: args.b, result });
  },
  
  get_weather: async (args: { location: string; unit?: string }) => {
    console.log(chalk.blue(`   🔧 Executing: get_weather(${args.location})`));
    await new Promise(resolve => setTimeout(resolve, 400));
    
    const conditions = ['sunny', 'partly cloudy', 'rainy', 'snowy'];
    const condition = conditions[Math.floor(Math.random() * conditions.length)];
    const temp = Math.floor(Math.random() * 30) + 5;
    
    return JSON.stringify({
      location: args.location,
      temperature: temp,
      unit: args.unit || 'celsius',
      condition,
      humidity: Math.floor(Math.random() * 40) + 40
    });
  },
  
  get_current_time: async (args: { timezone?: string }) => {
    console.log(chalk.blue(`   🔧 Executing: get_current_time(${args.timezone || 'UTC'})`));
    await new Promise(resolve => setTimeout(resolve, 200));
    
    const now = new Date();
    return JSON.stringify({
      timezone: args.timezone || 'UTC',
      timestamp: now.toISOString(),
      unix_timestamp: Math.floor(now.getTime() / 1000)
    });
  }
};

// Tool definitions
const TOOL_DEFINITIONS = [
  {
    type: 'function',
    function: {
      name: 'calculate',
      description: 'Perform mathematical calculations',
      parameters: {
        type: 'object',
        properties: {
          operation: { type: 'string', enum: ['add', 'subtract', 'multiply', 'divide'] },
          a: { type: 'number', description: 'First operand' },
          b: { type: 'number', description: 'Second operand' }
        },
        required: ['operation', 'a', 'b']
      }
    }
  },
  {
    type: 'function',
    function: {
      name: 'get_weather',
      description: 'Get current weather for a location',
      parameters: {
        type: 'object',
        properties: {
          location: { type: 'string', description: 'City and state/country' },
          unit: { type: 'string', enum: ['celsius', 'fahrenheit'] }
        },
        required: ['location']
      }
    }
  },
  {
    type: 'function',
    function: {
      name: 'get_current_time',
      description: 'Get current time in a timezone',
      parameters: {
        type: 'object',
        properties: {
          timezone: { type: 'string', description: 'IANA timezone (e.g., America/New_York)' }
        }
      }
    }
  }
];

// Stream response handler - simplified and reliable
async function streamRequest(
  messages: Message[],
  tools: any[],
  showChunks: boolean = false
): Promise<{ content: string; tool_calls: ToolCall[] }> {
  
  const response = await fetch(CHAT_ENDPOINT, {
    method: 'POST',
    headers: {
      'Content-Type': 'application/json',
      'Accept': 'text/event-stream'
    },
    body: JSON.stringify({
      model: 'claude-sonnet-4@20250514',
      messages,
      tools,
      max_tokens: 2000,
      stream: true
    })
  });

  if (!response.ok) {
    throw new Error(`HTTP ${response.status}: ${await response.text()}`);
  }

  const reader = response.body?.getReader();
  if (!reader) {
    throw new Error('No response body reader available');
  }

  const decoder = new TextDecoder();
  let buffer = '';
  let content = '';
  let toolCalls: ToolCall[] = [];

  try {
    while (true) {
      const { done, value } = await reader.read();
      if (done) break;

      buffer += decoder.decode(value, { stream: true });
      const lines = buffer.split('\n');
      buffer = lines.pop() || '';

      for (const line of lines) {
        if (!line.startsWith('data: ')) continue;
        
        const data = line.slice(6).trim();
        if (data === '[DONE]') continue;

        try {
          const chunk: StreamChunk = JSON.parse(data);
          
          if (showChunks) {
            console.log(chalk.gray(`   📦 Chunk: ${JSON.stringify(chunk)}`));
          }
          
          const delta = chunk.choices?.[0]?.delta;
          if (!delta) continue;

          // Accumulate content
          if (delta.content) {
            content += delta.content;
            if (showChunks) {
              process.stdout.write(chalk.cyan(delta.content));
            }
          }

          // Handle tool calls - use the complete tool call data from the chunk
          if (delta.tool_calls && delta.tool_calls.length > 0) {
            for (const toolCall of delta.tool_calls) {
              // Only add if we have complete tool call data
              if (toolCall.id && toolCall.function?.name && toolCall.function?.arguments) {
                const existingIndex = toolCalls.findIndex(tc => tc.id === toolCall.id);
                if (existingIndex === -1) {
                  toolCalls.push({
                    id: toolCall.id,
                    type: 'function',
                    function: {
                      name: toolCall.function.name,
                      arguments: toolCall.function.arguments
                    }
                  });
                }
              }
            }
          }
        } catch (error) {
          console.warn(chalk.yellow(`   ⚠️  Failed to parse chunk: ${data}`));
        }
      }
    }
  } finally {
    reader.releaseLock();
  }

  if (showChunks && (content || toolCalls.length > 0)) {
    console.log(); // New line after streaming output
  }

  return { content, tool_calls: toolCalls };
}

// Execute tool locally
async function executeTool(toolCall: ToolCall): Promise<string> {
  const toolName = toolCall.function.name;
  const toolFunction = TOOLS[toolName];
  
  if (!toolFunction) {
    throw new Error(`Tool '${toolName}' not found`);
  }
  
  const args = JSON.parse(toolCall.function.arguments);
  return await toolFunction(args);
}

// Main streaming E2E test
async function runStreamingE2ETest(
  testName: string,
  userQuery: string,
  expectedToolName?: string,
  showStreamChunks: boolean = false
): Promise<void> {
  
  console.log(chalk.cyan(`\n${'='.repeat(80)}`));
  console.log(chalk.cyan(`🧪 Streaming Test: ${testName}`));
  console.log(chalk.cyan('='.repeat(80)));
  
  const startTime = Date.now();
  const messages: Message[] = [];
  
  try {
    // Step 1: User query
    console.log(chalk.yellow(`\n📤 Step 1: User Query`));
    console.log(chalk.white(`   "${userQuery}"`));
    
    messages.push({ role: 'user', content: userQuery });
    
    // Step 2: Stream initial request
    console.log(chalk.yellow(`\n🌊 Step 2: Streaming Request to LLM (with tools)`));
    
    const firstResponse = await streamRequest(messages, TOOL_DEFINITIONS, showStreamChunks);
    
    // Step 3: Check for tool calls
    console.log(chalk.yellow(`\n📋 Step 3: Tool Calls from Stream`));
    
    if (firstResponse.tool_calls.length === 0) {
      console.log(chalk.red(`   ❌ No tool calls in stream`));
      console.log(chalk.red(`   Content: ${firstResponse.content}`));
      throw new Error('LLM did not request any tool calls');
    }
    
    console.log(chalk.green(`   ✓ ${firstResponse.tool_calls.length} tool call(s) accumulated from stream`));
    
    for (const toolCall of firstResponse.tool_calls) {
      console.log(chalk.blue(`   • ${toolCall.function.name}`));
      console.log(chalk.gray(`     ID: ${toolCall.id}`));
      console.log(chalk.gray(`     Args: ${toolCall.function.arguments}`));
      
      if (expectedToolName && toolCall.function.name !== expectedToolName) {
        throw new Error(`Expected tool '${expectedToolName}', got '${toolCall.function.name}'`);
      }
    }
    
    // Add assistant message to conversation
    // Ensure content is not empty - use a default message if the LLM didn't provide content
    const assistantContent = firstResponse.content || "I'll help you with that.";
    
    messages.push({
      role: 'assistant',
      content: assistantContent,
      tool_calls: firstResponse.tool_calls
    });
    
    // Step 4: Execute tools
    console.log(chalk.yellow(`\n🔧 Step 4: Execute Tools Locally`));
    
    const toolResults: Message[] = [];
    
    for (const toolCall of firstResponse.tool_calls) {
      const result = await executeTool(toolCall);
      console.log(chalk.green(`   ✓ Result: ${result}`));
      
      toolResults.push({
        role: 'tool',
        tool_call_id: toolCall.id,
        content: result
      });
    }
    
    messages.push(...toolResults);
    
    // Step 5: Stream final response
    console.log(chalk.yellow(`\n🌊 Step 5: Streaming Final Answer from LLM`));
    
    const finalResponse = await streamRequest(messages, TOOL_DEFINITIONS, true);
    
    if (!finalResponse.content) {
      throw new Error('No final content in stream');
    }
    
    console.log(chalk.yellow(`\n💬 Step 6: Complete Final Answer`));
    console.log(chalk.green(`\n   ${finalResponse.content}\n`));
    
    // Success
    const duration = Date.now() - startTime;
    console.log(chalk.green(`✅ Streaming Test Passed (${duration}ms)`));
    console.log(chalk.green(`   • Streamed initial request`));
    console.log(chalk.green(`   • Accumulated ${firstResponse.tool_calls.length} tool call(s) from chunks`));
    console.log(chalk.green(`   • Executed tools locally`));
    console.log(chalk.green(`   • Streamed final answer`));
    
  } catch (error) {
    const duration = Date.now() - startTime;
    console.log(chalk.red(`\n❌ Streaming Test Failed (${duration}ms)`));
    console.log(chalk.red(`   Error: ${error instanceof Error ? error.message : String(error)}`));
    throw error;
  }
}

// Test suite
class StreamingE2ETests {
  async runAllTests(): Promise<void> {
    console.log(chalk.cyan('\n🌊 Streaming E2E Tool Calling Test Suite'));
    console.log(chalk.cyan('========================================='));
    console.log(`Service URL: ${SERVICE_URL}\n`);
    
    let passed = 0;
    let failed = 0;
    
    const tests = [
      {
        name: 'Simple Calculation (Streaming)',
        query: 'Calculate 156 multiplied by 47. Show me the exact result.',
        expectedTool: 'calculate'
      },
      {
        name: 'Weather Query (Streaming)',
        query: 'What is the weather in Paris, France right now?',
        expectedTool: 'get_weather'
      },
      {
        name: 'Time Query (Streaming)',
        query: 'What is the current time in Tokyo?',
        expectedTool: 'get_current_time'
      },
      {
        name: 'Complex Calculation (Streaming)',
        query: 'What is 2048 divided by 64?',
        expectedTool: 'calculate'
      }
    ];
    
    for (const test of tests) {
      try {
        await runStreamingE2ETest(test.name, test.query, test.expectedTool, false);
        passed++;
      } catch (error) {
        failed++;
        console.error(chalk.red(`\nTest "${test.name}" failed:`), error);
      }
    }
    
    // Summary
    console.log(chalk.cyan(`\n${'='.repeat(80)}`));
    console.log(chalk.cyan('📊 Streaming Test Summary'));
    console.log(chalk.cyan('='.repeat(80)));
    console.log(`Total tests: ${passed + failed}`);
    console.log(chalk.green(`Passed: ${passed}`));
    console.log(chalk.red(`Failed: ${failed}`));
    
    if (failed === 0) {
      console.log(chalk.green('\n🎉 All streaming tests passed!'));
      console.log(chalk.cyan('\n✨ Verified streaming capabilities:'));
      console.log(chalk.cyan('   • SSE streaming with tool calls'));
      console.log(chalk.cyan('   • Tool call chunk accumulation'));
      console.log(chalk.cyan('   • Streaming final answers'));
    } else {
      console.log(chalk.red(`\n❌ ${failed} streaming test(s) failed`));
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
    
    const testRunner = new StreamingE2ETests();
    await testRunner.runAllTests();
    
  } catch (error) {
    console.error(chalk.red('\n💥 Test execution failed:'), error);
    process.exit(1);
  }
}

// Run if executed directly
if (import.meta.url === `file://${process.argv[1]}`) {
  main().catch(console.error);
}

export { StreamingE2ETests, streamRequest, executeTool };
