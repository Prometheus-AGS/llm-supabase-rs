#!/usr/bin/env tsx

/**
 * Direct HTTP Test Suite for LLM Supabase Service
 * 
 * This test uses direct HTTP calls to test both streaming and non-streaming
 * functionality without complex client logic that can introduce bugs.
 */

import chalk from 'chalk';

const SERVICE_URL = 'http://localhost:8080';
const CHAT_ENDPOINT = `${SERVICE_URL}/v1/chat/completions`;

// Simple HTTP request function
async function httpRequest(url: string, options: RequestInit): Promise<Response> {
  const response = await fetch(url, options);
  if (!response.ok) {
    const errorText = await response.text();
    throw new Error(`HTTP ${response.status}: ${errorText}`);
  }
  return response;
}

// Test 1: Non-streaming calculation
async function testNonStreamingCalculation(): Promise<void> {
  console.log(chalk.cyan('\n🧪 Test: Non-streaming Calculation'));
  console.log(chalk.cyan('====================================='));
  
  try {
    // Step 1: Initial request with tools
    const response1 = await httpRequest(CHAT_ENDPOINT, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({
        model: 'claude-sonnet-4@20250514',
        messages: [
          { role: 'user', content: 'Calculate 15 * 23' }
        ],
        tools: [{
          type: 'function',
          function: {
            name: 'calculate',
            description: 'Perform mathematical calculations',
            parameters: {
              type: 'object',
              properties: {
                operation: { type: 'string', enum: ['add', 'subtract', 'multiply', 'divide'] },
                a: { type: 'number' },
                b: { type: 'number' }
              },
              required: ['operation', 'a', 'b']
            }
          }
        }],
        max_tokens: 200,
        stream: false
      })
    });

    const result1 = await response1.json();
    console.log(chalk.green('✓ Step 1: LLM response received'));
    
    // Check for tool calls
    const message1 = result1.choices[0].message;
    if (!message1.tool_calls || message1.tool_calls.length === 0) {
      throw new Error('No tool calls in first response');
    }
    
    const toolCall = message1.tool_calls[0];
    console.log(chalk.blue(`✓ Tool call: ${toolCall.function.name}`));
    console.log(chalk.gray(`  Args: ${toolCall.function.arguments}`));
    
    // Parse and execute tool locally
    const args = JSON.parse(toolCall.function.arguments);
    let result: number;
    switch (args.operation) {
      case 'multiply': result = args.a * args.b; break;
      case 'add': result = args.a + args.b; break;
      case 'subtract': result = args.a - args.b; break;
      case 'divide': result = args.a / args.b; break;
      default: throw new Error(`Unknown operation: ${args.operation}`);
    }
    
    const toolResult = JSON.stringify({ operation: args.operation, operand1: args.a, operand2: args.b, result });
    console.log(chalk.green(`✓ Tool executed locally: ${result}`));
    
    // Step 2: Send tool result back
    const response2 = await httpRequest(CHAT_ENDPOINT, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({
        model: 'claude-sonnet-4@20250514',
        messages: [
          { role: 'user', content: 'Calculate 15 * 23' },
          { role: 'assistant', content: message1.content || 'I will calculate that for you.', tool_calls: message1.tool_calls },
          { role: 'tool', tool_call_id: toolCall.id, content: toolResult }
        ],
        max_tokens: 200,
        stream: false
      })
    });

    const result2 = await response2.json();
    const message2 = result2.choices[0].message;
    
    console.log(chalk.yellow('Step 2: Final response'));
    console.log(chalk.gray(`Response: ${JSON.stringify(result2, null, 2)}`));
    
    if (!message2.content) {
      throw new Error('No final content in response');
    }
    
    console.log(chalk.green(`✓ Final answer: ${message2.content}`));
    console.log(chalk.green('✅ Non-streaming calculation test PASSED'));
    
  } catch (error) {
    console.log(chalk.red(`❌ Non-streaming calculation test FAILED: ${error}`));
    throw error;
  }
}

// Test 2: Streaming calculation
async function testStreamingCalculation(): Promise<void> {
  console.log(chalk.cyan('\n🧪 Test: Streaming Calculation'));
  console.log(chalk.cyan('=================================='));
  
  try {
    // Step 1: Streaming request with tools
    const response1 = await httpRequest(CHAT_ENDPOINT, {
      method: 'POST',
      headers: { 
        'Content-Type': 'application/json',
        'Accept': 'text/event-stream'
      },
      body: JSON.stringify({
        model: 'claude-sonnet-4@20250514',
        messages: [
          { role: 'user', content: 'Calculate 25 * 17' }
        ],
        tools: [{
          type: 'function',
          function: {
            name: 'calculate',
            description: 'Perform mathematical calculations',
            parameters: {
              type: 'object',
              properties: {
                operation: { type: 'string', enum: ['add', 'subtract', 'multiply', 'divide'] },
                a: { type: 'number' },
                b: { type: 'number' }
              },
              required: ['operation', 'a', 'b']
            }
          }
        }],
        max_tokens: 200,
        stream: true
      })
    });

    // Parse SSE stream
    const reader = response1.body?.getReader();
    if (!reader) throw new Error('No response body reader');
    
    const decoder = new TextDecoder();
    let buffer = '';
    let content = '';
    let toolCalls: any[] = [];
    
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
          const chunk = JSON.parse(data);
          const delta = chunk.choices?.[0]?.delta;
          if (!delta) continue;
          
          if (delta.content) {
            content += delta.content;
          }
          
          if (delta.tool_calls) {
            for (const toolCall of delta.tool_calls) {
              const index = toolCall.index ?? 0;
              if (!toolCalls[index]) {
                toolCalls[index] = {
                  id: toolCall.id || `call_${index}`,
                  type: 'function',
                  function: { name: '', arguments: '' }
                };
              }
              
              if (toolCall.id) toolCalls[index].id = toolCall.id;
              if (toolCall.function?.name) toolCalls[index].function.name = toolCall.function.name;
              if (toolCall.function?.arguments) toolCalls[index].function.arguments = toolCall.function.arguments;
            }
          }
        } catch (e) {
          // Skip invalid chunks
        }
      }
    }
    
    console.log(chalk.green('✓ Step 1: Streaming response received'));
    console.log(chalk.gray(`  Content: ${content}`));
    console.log(chalk.gray(`  Tool calls: ${toolCalls.length}`));
    
    if (toolCalls.length === 0) {
      throw new Error('No tool calls in streaming response');
    }
    
    const toolCall = toolCalls[0];
    console.log(chalk.blue(`✓ Tool call: ${toolCall.function.name}`));
    console.log(chalk.gray(`  Args: ${toolCall.function.arguments}`));
    
    // Execute tool locally
    const args = JSON.parse(toolCall.function.arguments);
    const result = args.a * args.b; // Simple multiplication
    const toolResult = JSON.stringify({ operation: args.operation, operand1: args.a, operand2: args.b, result });
    console.log(chalk.green(`✓ Tool executed locally: ${result}`));
    
    // Step 2: Send tool result back (streaming)
    const response2 = await httpRequest(CHAT_ENDPOINT, {
      method: 'POST',
      headers: { 
        'Content-Type': 'application/json',
        'Accept': 'text/event-stream'
      },
      body: JSON.stringify({
        model: 'claude-sonnet-4@20250514',
        messages: [
          { role: 'user', content: 'Calculate 25 * 17' },
          { role: 'assistant', content: content || 'I will calculate that for you.', tool_calls: toolCalls },
          { role: 'tool', tool_call_id: toolCall.id, content: toolResult }
        ],
        max_tokens: 200,
        stream: true
      })
    });

    // Parse final streaming response
    const reader2 = response2.body?.getReader();
    if (!reader2) throw new Error('No response body reader for final response');
    
    let finalContent = '';
    let finalBuffer = '';
    
    while (true) {
      const { done, value } = await reader2.read();
      if (done) break;
      
      finalBuffer += decoder.decode(value, { stream: true });
      const lines = finalBuffer.split('\n');
      finalBuffer = lines.pop() || '';
      
      for (const line of lines) {
        if (!line.startsWith('data: ')) continue;
        const data = line.slice(6).trim();
        if (data === '[DONE]') continue;
        
        try {
          const chunk = JSON.parse(data);
          const delta = chunk.choices?.[0]?.delta;
          if (delta?.content) {
            finalContent += delta.content;
          }
        } catch (e) {
          // Skip invalid chunks
        }
      }
    }
    
    console.log(chalk.yellow('Step 2: Final streaming response'));
    if (!finalContent.trim()) {
      throw new Error('No final content in streaming response');
    }
    
    console.log(chalk.green(`✓ Final answer: ${finalContent}`));
    console.log(chalk.green('✅ Streaming calculation test PASSED'));
    
  } catch (error) {
    console.log(chalk.red(`❌ Streaming calculation test FAILED: ${error}`));
    throw error;
  }
}

// Health check
async function checkHealth(): Promise<void> {
  console.log(chalk.blue('🏥 Checking service health...'));
  const response = await httpRequest(`${SERVICE_URL}/health`, { method: 'GET' });
  const health = await response.json();
  if (health.status !== 'healthy') {
    throw new Error(`Service not healthy: ${health.status}`);
  }
  console.log(chalk.green('✓ Service is healthy\n'));
}

// Main test runner
async function main(): Promise<void> {
  try {
    await checkHealth();
    
    console.log(chalk.cyan('🚀 Direct HTTP Test Suite'));
    console.log(chalk.cyan('=========================='));
    
    let passed = 0;
    let failed = 0;
    
    // Test non-streaming
    try {
      await testNonStreamingCalculation();
      passed++;
    } catch (error) {
      failed++;
      console.error(chalk.red('Non-streaming test failed:'), error);
    }
    
    // Test streaming
    try {
      await testStreamingCalculation();
      passed++;
    } catch (error) {
      failed++;
      console.error(chalk.red('Streaming test failed:'), error);
    }
    
    // Summary
    console.log(chalk.cyan('\n📊 Test Summary'));
    console.log(chalk.cyan('================'));
    console.log(`Total tests: ${passed + failed}`);
    console.log(chalk.green(`Passed: ${passed}`));
    console.log(chalk.red(`Failed: ${failed}`));
    
    if (failed === 0) {
      console.log(chalk.green('\n🎉 All tests passed!'));
    } else {
      console.log(chalk.red(`\n❌ ${failed} test(s) failed`));
      process.exit(1);
    }
    
  } catch (error) {
    console.error(chalk.red('Test execution failed:'), error);
    process.exit(1);
  }
}

// Run if executed directly
if (import.meta.url === `file://${process.argv[1]}`) {
  main().catch(console.error);
}