#!/usr/bin/env tsx

/**
 * Minimal End-to-End Tool Calling Example
 * 
 * This is the simplest possible demonstration of the complete tool calling flow.
 * Perfect for quick verification that tool calling works end-to-end.
 * 
 * Usage: tsx test-simple-tool-e2e.ts
 */

import chalk from 'chalk';

const SERVICE_URL = 'http://localhost:8080';
const CHAT_ENDPOINT = `${SERVICE_URL}/v1/chat/completions`;

interface Message {
  role: 'user' | 'assistant' | 'tool';
  content: string | null;
  tool_calls?: Array<{
    id: string;
    type: 'function';
    function: { name: string; arguments: string };
  }>;
  tool_call_id?: string;
}

// Simple calculator tool
function calculate(operation: string, a: number, b: number): number {
  switch (operation) {
    case 'add': return a + b;
    case 'subtract': return a - b;
    case 'multiply': return a * b;
    case 'divide': return a / b;
    default: throw new Error(`Unknown operation: ${operation}`);
  }
}

async function runSimpleE2ETest(): Promise<void> {
  console.log(chalk.cyan('\n🔬 Minimal E2E Tool Calling Test\n'));
  
  const messages: Message[] = [];
  
  // Step 1: User asks for calculation
  console.log(chalk.yellow('Step 1: User Query'));
  const userQuery = 'What is 24 multiplied by 37?';
  console.log(`  "${userQuery}"\n`);
  
  messages.push({
    role: 'user',
    content: userQuery
  });
  
  // Step 2: Request with tool definition
  console.log(chalk.yellow('Step 2: Sending to LLM with calculator tool...'));
  
  const firstResponse = await fetch(CHAT_ENDPOINT, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({
      model: 'claude-sonnet-4@20250514',
      messages: messages,
      max_tokens: 1000,
      tools: [{
        type: 'function',
        function: {
          name: 'calculate',
          description: 'Perform a mathematical calculation',
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
      }]
    })
  });
  
  if (!firstResponse.ok) {
    throw new Error(`HTTP ${firstResponse.status}: ${await firstResponse.text()}`);
  }
  
  const firstResult = await firstResponse.json();
  const assistantMessage = firstResult.choices[0].message;
  
  // Step 3: Check for tool calls
  console.log(chalk.yellow('Step 3: Checking LLM response...'));
  
  if (!assistantMessage.tool_calls || assistantMessage.tool_calls.length === 0) {
    console.log(chalk.red('  ❌ No tool calls requested by LLM'));
    console.log(chalk.red(`  Response: ${assistantMessage.content}`));
    process.exit(1);
  }
  
  const toolCall = assistantMessage.tool_calls[0];
  console.log(chalk.green(`  ✓ Tool requested: ${toolCall.function.name}`));
  console.log(chalk.blue(`  ✓ Arguments: ${toolCall.function.arguments}\n`));
  
  messages.push({
    role: 'assistant',
    content: assistantMessage.content,
    tool_calls: assistantMessage.tool_calls
  });
  
  // Step 4: Execute tool
  console.log(chalk.yellow('Step 4: Executing calculator tool...'));
  
  const args = JSON.parse(toolCall.function.arguments);
  const result = calculate(args.operation, args.a, args.b);
  
  console.log(chalk.green(`  ✓ Calculation: ${args.a} ${args.operation} ${args.b} = ${result}\n`));
  
  messages.push({
    role: 'tool',
    tool_call_id: toolCall.id,
    content: JSON.stringify({ result })
  });
  
  // Step 5: Get final answer
  console.log(chalk.yellow('Step 5: Getting final answer from LLM...'));
  
  const secondResponse = await fetch(CHAT_ENDPOINT, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({
      model: 'claude-sonnet-4@20250514',
      messages: messages,
      max_tokens: 1000
    })
  });
  
  if (!secondResponse.ok) {
    throw new Error(`HTTP ${secondResponse.status}: ${await secondResponse.text()}`);
  }
  
  const finalResult = await secondResponse.json();
  const finalAnswer = finalResult.choices[0].message.content;
  
  console.log(chalk.green('  ✓ Final Answer:\n'));
  console.log(chalk.white(`    ${finalAnswer}\n`));
  
  // Success!
  console.log(chalk.green('✅ E2E Tool Calling Test PASSED\n'));
  console.log(chalk.cyan('Flow verified:'));
  console.log(chalk.cyan('  User Query → LLM → Tool Call → Execution → Result → Final Answer'));
}

// Run the test
async function main() {
  try {
    // Health check first
    const health = await fetch(`${SERVICE_URL}/health`);
    if (!health.ok) {
      throw new Error('Service not healthy');
    }
    
    await runSimpleE2ETest();
  } catch (error) {
    console.log(chalk.red('\n❌ Test Failed'));
    console.log(chalk.red(`Error: ${error instanceof Error ? error.message : String(error)}\n`));
    process.exit(1);
  }
}

main();
