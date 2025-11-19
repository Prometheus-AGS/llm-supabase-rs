#!/usr/bin/env tsx

/**
 * Simple diagnostic script to test connectivity to the LLM Supabase service
 * This helps identify the root cause of connection failures
 */

import chalk from 'chalk';

// Use Node.js built-in fetch (Node 18+)
const fetch = globalThis.fetch;

const SERVICE_URL = 'http://localhost:8080';

async function diagnoseConnection(): Promise<void> {
  console.log(chalk.cyan('🔍 Diagnosing Connection to LLM Supabase Service'));
  console.log(chalk.cyan('=================================================='));
  
  // Test 1: Basic connectivity to localhost:8080
  console.log(chalk.blue('\n1. Testing basic connectivity...'));
  try {
    console.log(chalk.blue(`   Attempting to connect to: ${SERVICE_URL}`));
    const response = await fetch(SERVICE_URL, { 
      method: 'GET',
      signal: AbortSignal.timeout(5000) // 5 second timeout
    });
    console.log(chalk.green(`   ✅ Connection successful! Status: ${response.status}`));
    
    const headersObj: Record<string, string> = {};
    response.headers.forEach((value, key) => {
      headersObj[key] = value;
    });
    console.log(chalk.blue(`   Response headers: ${JSON.stringify(headersObj, null, 2)}`));
    
    const responseText = await response.text();
    console.log(chalk.blue(`   Response body (first 200 chars): ${responseText.substring(0, 200)}...`));
    
  } catch (error: any) {
    console.log(chalk.red(`   ❌ Connection failed!`));
    console.log(chalk.red(`   Error: ${error.message}`));
    console.log(chalk.red(`   Error type: ${error?.constructor?.name || 'Unknown'}`));
    if (error?.code) {
      console.log(chalk.red(`   Error code: ${error.code}`));
    }
    if (error?.cause) {
      console.log(chalk.red(`   Error cause: ${error.cause}`));
    }
  }
  
  // Test 2: Health endpoint
  console.log(chalk.blue('\n2. Testing health endpoint...'));
  try {
    console.log(chalk.blue(`   Attempting to connect to: ${SERVICE_URL}/health`));
    const response = await fetch(`${SERVICE_URL}/health`, {
      method: 'GET',
      signal: AbortSignal.timeout(5000)
    });
    console.log(chalk.green(`   ✅ Health endpoint accessible! Status: ${response.status}`));
    
    const healthData = await response.text();
    console.log(chalk.blue(`   Health response: ${healthData}`));
    
  } catch (error: any) {
    console.log(chalk.red(`   ❌ Health endpoint failed!`));
    console.log(chalk.red(`   Error: ${error.message}`));
    console.log(chalk.red(`   Error code: ${error?.code || 'N/A'}`));
  }
  
  // Test 3: Chat completions endpoint
  console.log(chalk.blue('\n3. Testing chat completions endpoint...'));
  try {
    console.log(chalk.blue(`   Attempting to connect to: ${SERVICE_URL}/v1/chat/completions`));
    const response = await fetch(`${SERVICE_URL}/v1/chat/completions`, {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
      },
      body: JSON.stringify({
        model: 'claude-sonnet-4@20250514',
        messages: [{ role: 'user', content: 'Hello' }],
        max_tokens: 10
      }),
      signal: AbortSignal.timeout(10000) // 10 second timeout for API calls
    });
    console.log(chalk.green(`   ✅ Chat endpoint accessible! Status: ${response.status}`));
    
    if (response.ok) {
      const chatData = await response.json();
      console.log(chalk.blue(`   Chat response: ${JSON.stringify(chatData, null, 2)}`));
    } else {
      const errorText = await response.text();
      console.log(chalk.yellow(`   Response body: ${errorText}`));
    }
    
  } catch (error: any) {
    console.log(chalk.red(`   ❌ Chat endpoint failed!`));
    console.log(chalk.red(`   Error: ${error.message}`));
    console.log(chalk.red(`   Error code: ${error?.code || 'N/A'}`));
  }
  
  // Test 4: Check if service is running on different ports
  console.log(chalk.blue('\n4. Checking common alternative ports...'));
  const commonPorts = [3000, 8000, 8081, 8082, 8888, 9000];
  
  for (const port of commonPorts) {
    try {
      const testUrl = `http://localhost:${port}`;
      console.log(chalk.blue(`   Testing port ${port}...`));
      const response = await fetch(testUrl, {
        method: 'GET',
        signal: AbortSignal.timeout(2000) // Quick 2 second timeout
      });
      console.log(chalk.green(`   ✅ Found service on port ${port}! Status: ${response.status}`));
      
      // Try health endpoint on this port
      try {
        const healthResponse = await fetch(`${testUrl}/health`, {
          signal: AbortSignal.timeout(2000)
        });
        if (healthResponse.ok) {
          console.log(chalk.green(`   ✅ Health endpoint also works on port ${port}!`));
        }
      } catch {
        // Health endpoint might not exist, that's ok
      }
      
    } catch (error: any) {
      console.log(chalk.gray(`   Port ${port}: ${error.code || error.message}`));
    }
  }
  
  console.log(chalk.cyan('\n📋 Diagnosis Summary:'));
  console.log(chalk.cyan('===================='));
  console.log('If all tests failed with ECONNREFUSED:');
  console.log('  → The service is not running on localhost:8080');
  console.log('  → Start your service with: cargo run (or your preferred method)');
  console.log('');
  console.log('If a different port showed success:');
  console.log('  → Update SERVICE_URL in test-tool-calling.ts to use the correct port');
  console.log('');
  console.log('If basic connectivity works but endpoints fail:');
  console.log('  → The service is running but may not have the expected API endpoints');
  console.log('  → Check your service configuration and routes');
}

// Run diagnosis
diagnoseConnection().catch(console.error);