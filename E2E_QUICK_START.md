# E2E Tool Calling - Quick Start

## What Was Created

Three TypeScript test files for end-to-end tool calling:

### 1. **test-simple-tool-e2e.ts** ⭐ Start Here
The simplest possible example - perfect for verifying tool calling works.

```bash
npm run test:simple
```

**What it does:**
- Asks LLM: "What is 24 multiplied by 37?"
- LLM calls `calculate` tool
- Tool executes locally (24 × 37 = 888)
- Result sent back to LLM
- LLM provides natural language answer

**Expected output:**
```
🔬 Minimal E2E Tool Calling Test

Step 1: User Query
  "What is 24 multiplied by 37?"

Step 2: Sending to LLM with calculator tool...
Step 3: Checking LLM response...
  ✓ Tool requested: calculate
  ✓ Arguments: {"operation":"multiply","a":24,"b":37}

Step 4: Executing calculator tool...
  ✓ Calculation: 24 multiply 37 = 888

Step 5: Getting final answer from LLM...
  ✓ Final Answer:

    The result of 24 multiplied by 37 is 888.

✅ E2E Tool Calling Test PASSED
```

### 2. **test-tool-calling-e2e.ts** 🚀 Comprehensive Tests
Full test suite with multiple scenarios and tools.

```bash
npm run test:e2e
```

**Includes 5 test scenarios:**
1. Weather query (get_weather)
2. Math calculation (calculate)
3. Time query (get_current_time)
4. Stock price (get_stock_price)
5. Database search (search_database)

### 3. **test-tool-calling.ts** 🔍 Unit Tests
Existing comprehensive unit tests for tool calling features.

```bash
npm run test:tools
```

## Running Your First Test

1. **Start the service:**
   ```bash
   cargo run
   ```

2. **In another terminal, run the simple test:**
   ```bash
   npm run test:simple
   ```

3. **If that works, run the full suite:**
   ```bash
   npm run test:e2e
   ```

## What Each Test Verifies

### Simple Test
- ✅ LLM receives tool definitions
- ✅ LLM decides to call a tool
- ✅ Tool call parameters are valid JSON
- ✅ Tool executes successfully
- ✅ Tool result sent back to LLM
- ✅ LLM generates final answer with result

### Comprehensive Test
All of the above, plus:
- ✅ Multiple different tools
- ✅ Different parameter types
- ✅ Optional parameters
- ✅ Error handling
- ✅ Multiple test scenarios

## Troubleshooting

### "Service health check failed"
**Problem:** Service isn't running
**Solution:** Run `cargo run` in another terminal

### "No tool calls requested by LLM"
**Problem:** LLM didn't use the tool
**Possible causes:**
- Query too vague
- Tool description unclear
- Model configuration issue

**Solution:** Check service logs for errors

### "HTTP 500" or other errors
**Problem:** Service error
**Solution:** Check cargo run output for stack traces

## Next Steps

1. ✅ Run `npm run test:simple` to verify basic functionality
2. ✅ Run `npm run test:e2e` for comprehensive testing
3. ✅ Read `E2E_TOOL_CALLING_GUIDE.md` for detailed documentation
4. ✅ Customize tests for your use case

## File Structure

```
.
├── test-simple-tool-e2e.ts          # Minimal example (START HERE)
├── test-tool-calling-e2e.ts         # Full test suite
├── test-tool-calling.ts             # Unit tests
├── E2E_QUICK_START.md              # This file
├── E2E_TOOL_CALLING_GUIDE.md       # Detailed guide
└── package.json                     # Updated with new scripts
```

## Available npm Scripts

```bash
npm run test:simple    # Run minimal E2E test
npm run test:e2e       # Run comprehensive E2E suite
npm run test:tools     # Run unit tests
npm run diagnose       # Diagnose connection issues
```

## The Complete Flow

```
User Query
    ↓
LLM + Tool Definitions
    ↓
Tool Call Request
    ↓
Local Tool Execution
    ↓
Tool Result
    ↓
LLM + Previous Context + Tool Result
    ↓
Final Natural Language Answer
```

## Example Tool Definition

```typescript
{
  type: 'function',
  function: {
    name: 'calculate',
    description: 'Perform a mathematical calculation',
    parameters: {
      type: 'object',
      properties: {
        operation: { type: 'string', enum: ['add', 'multiply'] },
        a: { type: 'number' },
        b: { type: 'number' }
      },
      required: ['operation', 'a', 'b']
    }
  }
}
```

## Success Criteria

Your E2E test is successful when:
1. LLM requests the expected tool
2. Tool executes with correct parameters
3. Tool returns valid JSON result
4. LLM incorporates result into final answer
5. All steps complete without errors

---

**Ready to test?** Run `npm run test:simple` now! 🚀
