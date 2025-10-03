# Tool Calling Implementation Guide

This document describes the comprehensive tool calling implementation that provides full OpenAI API compatibility while handling the differences between OpenAI and Claude/Vertex AI tool calling formats.

## 🎯 Key Features

### **✅ Complete OpenAI Compatibility**
- **Tool Definitions** - Full support for OpenAI tool/function definitions
- **Tool Calls** - Proper tool call detection and forwarding
- **Tool Results** - Seamless handling of tool execution results
- **Streaming Support** - Tool calls work in both streaming and non-streaming modes
- **Multi-turn Conversations** - Proper state management for tool call sequences

### **✅ Provider Abstraction**
- **Unified Interface** - Same API for all providers (Vertex AI, Bedrock, OpenAI)
- **Format Translation** - Automatic conversion between OpenAI and provider formats
- **Error Handling** - Consistent error handling across providers

## 🏗️ Architecture Overview

```
┌─────────────────────────────────────────────────────────────┐
│                    OpenAI API Client                       │
│                  (Your Application)                        │
└─────────────────────┬───────────────────────────────────────┘
                      │
                      │ 1. Send request with tools
                      ▼
┌─────────────────────────────────────────────────────────────┐
│                 Chat Handler                                │
│  • Detects tools in request                                 │
│  • Initializes ToolCallManager                             │
│  • Routes to streaming/non-streaming                       │
└─────────────────────┬───────────────────────────────────────┘
                      │
                      │ 2. Convert & send to provider
                      ▼
┌─────────────────────────────────────────────────────────────┐
│              Provider (Vertex AI/Claude)                   │
│  • Receives Claude-format tools                            │
│  • Makes tool calls                                        │
│  • Returns tool call requests                              │
└─────────────────────┬───────────────────────────────────────┘
                      │
                      │ 3. Tool calls detected
                      ▼
┌─────────────────────────────────────────────────────────────┐
│               ToolCallManager                               │
│  • Converts provider format to OpenAI                      │
│  • Manages tool call state                                 │
│  • Forwards to client                                      │
└─────────────────────┬───────────────────────────────────────┘
                      │
                      │ 4. Return tool calls to client
                      ▼
┌─────────────────────────────────────────────────────────────┐
│                OpenAI API Client                           │
│  • Receives tool calls                                     │
│  • Executes tools (search, API calls, etc.)               │
│  • Sends results back                                      │
└─────────────────────┬───────────────────────────────────────┘
                      │
                      │ 5. Continue conversation
                      ▼
                   [Repeat cycle]
```

## 🔧 Implementation Details

### **Tool Call Flow**

#### **1. Request with Tools**
```json
{
  "model": "claude-4-sonnet-20250514",
  "messages": [
    {
      "role": "user",
      "content": "Search for information about Rust programming"
    }
  ],
  "tools": [
    {
      "type": "function",
      "function": {
        "name": "search_web",
        "description": "Search the web for information",
        "parameters": {
          "type": "object",
          "properties": {
            "query": {
              "type": "string",
              "description": "Search query"
            }
          },
          "required": ["query"]
        }
      }
    }
  ]
}
```

#### **2. Provider Conversion**
The system automatically converts OpenAI tools to Claude format:

**OpenAI Format:**
```json
{
  "type": "function",
  "function": {
    "name": "search_web",
    "description": "Search the web",
    "parameters": { ... }
  }
}
```

**Claude Format:**
```json
{
  "name": "search_web",
  "description": "Search the web",
  "input_schema": { ... }
}
```

#### **3. Tool Call Detection**
When the provider makes tool calls, they're detected and converted:

**Claude Tool Call:**
```json
{
  "type": "tool_use",
  "id": "toolu_123",
  "name": "search_web",
  "input": {
    "query": "Rust programming language"
  }
}
```

**OpenAI Tool Call:**
```json
{
  "id": "call_123",
  "type": "function",
  "function": {
    "name": "search_web",
    "arguments": {
      "query": "Rust programming language"
    }
  }
}
```

#### **4. Client Response**
Your application receives standard OpenAI tool calls and responds with results:

```json
{
  "role": "tool",
  "tool_call_id": "call_123",
  "content": "Rust is a systems programming language..."
}
```

#### **5. Continuation**
The system converts tool results back to provider format and continues the conversation.

## 📋 Usage Examples

### **Basic Tool Calling**

```bash
curl -X POST http://localhost:8080/v1/chat/completions \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer your-token" \
  -d '{
    "model": "claude-4-sonnet-20250514",
    "messages": [
      {
        "role": "user",
        "content": "What is the weather in San Francisco?"
      }
    ],
    "tools": [
      {
        "type": "function",
        "function": {
          "name": "get_weather",
          "description": "Get current weather for a location",
          "parameters": {
            "type": "object",
            "properties": {
              "location": {
                "type": "string",
                "description": "City name"
              }
            },
            "required": ["location"]
          }
        }
      }
    ]
  }'
```

### **Streaming Tool Calls**

```bash
curl -X POST http://localhost:8080/v1/chat/completions \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer your-token" \
  -d '{
    "model": "claude-4-sonnet-20250514",
    "stream": true,
    "messages": [
      {
        "role": "user",
        "content": "Search for recent AI news"
      }
    ],
    "tools": [
      {
        "type": "function",
        "function": {
          "name": "search_web",
          "description": "Search the web",
          "parameters": {
            "type": "object",
            "properties": {
              "query": {"type": "string"}
            }
          }
        }
      }
    ]
  }'
```

### **Multi-turn Tool Conversation**

```json
// 1. Initial request with tools
{
  "model": "claude-4-sonnet-20250514",
  "messages": [
    {
      "role": "user",
      "content": "Find information about the latest iPhone and then get its price"
    }
  ],
  "tools": [
    {
      "type": "function",
      "function": {
        "name": "search_web",
        "description": "Search the web",
        "parameters": {
          "type": "object",
          "properties": {
            "query": {"type": "string"}
          }
        }
      }
    },
    {
      "type": "function", 
      "function": {
        "name": "get_product_price",
        "description": "Get product price",
        "parameters": {
          "type": "object",
          "properties": {
            "product": {"type": "string"}
          }
        }
      }
    }
  ]
}

// 2. Response with tool calls
{
  "choices": [
    {
      "message": {
        "role": "assistant",
        "content": null,
        "tool_calls": [
          {
            "id": "call_123",
            "type": "function",
            "function": {
              "name": "search_web",
              "arguments": {
                "query": "latest iPhone 2024"
              }
            }
          }
        ]
      },
      "finish_reason": "tool_calls"
    }
  ]
}

// 3. Continue with tool results
{
  "model": "claude-4-sonnet-20250514",
  "messages": [
    {
      "role": "user",
      "content": "Find information about the latest iPhone and then get its price"
    },
    {
      "role": "assistant",
      "content": null,
      "tool_calls": [
        {
          "id": "call_123",
          "type": "function",
          "function": {
            "name": "search_web",
            "arguments": {
              "query": "latest iPhone 2024"
            }
          }
        }
      ]
    },
    {
      "role": "tool",
      "tool_call_id": "call_123",
      "content": "The latest iPhone is the iPhone 15 Pro Max released in 2024..."
    }
  ],
  "tools": [/* same tools */]
}
```

## 🔍 Key Differences Handled

### **OpenAI vs Claude Tool Formats**

| Aspect | OpenAI | Claude/Vertex AI |
|--------|--------|------------------|
| **Tool Definition** | `tools[].function` | `tools[].input_schema` |
| **Tool Call Format** | `tool_calls[].function` | `content[].tool_use` |
| **Tool Results** | `role: "tool"` | `content[].tool_result` |
| **Streaming** | Incremental `tool_calls` | Complete `tool_use` blocks |

### **Automatic Conversions**

1. **Tool Definitions**: OpenAI `function.parameters` ↔ Claude `input_schema`
2. **Tool Calls**: OpenAI `tool_calls` ↔ Claude `tool_use` content blocks
3. **Tool Results**: OpenAI `tool` messages ↔ Claude `tool_result` content blocks
4. **Streaming**: OpenAI incremental chunks ↔ Claude complete blocks

## 🧪 Testing Tool Calls

### **Test Script Enhancement**

The test script (`test_streaming.sh`) includes tool calling tests:

```bash
# Test tool calling
./test_streaming.sh tool_calls

# Test streaming with tools
./test_streaming.sh streaming_tools
```

### **Manual Testing**

```bash
# 1. Start server
cargo run

# 2. Test tool calling
curl -X POST http://localhost:8080/v1/chat/completions \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer test-token" \
  -d @test_tool_request.json

# 3. Check logs for tool call detection
# Look for: "Tools detected, initializing tool call manager"
```

## 🎯 Integration with MCP Servers

The system is designed to work seamlessly with MCP (Model Context Protocol) servers:

### **Tavily Integration**
```json
{
  "type": "function",
  "function": {
    "name": "tavily_search",
    "description": "Search the web using Tavily",
    "parameters": {
      "type": "object",
      "properties": {
        "query": {"type": "string"},
        "max_results": {"type": "integer", "default": 5}
      }
    }
  }
}
```

### **Context7 Integration**
```json
{
  "type": "function", 
  "function": {
    "name": "context7_analyze",
    "description": "Analyze context using Context7",
    "parameters": {
      "type": "object",
      "properties": {
        "text": {"type": "string"},
        "analysis_type": {"type": "string", "enum": ["sentiment", "entities", "summary"]}
      }
    }
  }
}
```

## 🚀 Benefits

### **For Developers**
- **Drop-in Replacement** - Works exactly like OpenAI API
- **Provider Agnostic** - Switch between providers without code changes
- **Full Compatibility** - All OpenAI tool calling features supported
- **Streaming Support** - Real-time tool calls in streaming mode

### **For Applications**
- **Unified Interface** - One API for all providers
- **Cost Optimization** - Use cheaper providers with same interface
- **Reliability** - Fallback between providers
- **Performance** - Optimized for each provider's strengths

This implementation ensures that your application gets exactly the OpenAI tool calling behavior it expects, regardless of which AI provider is actually handling the requests! 🎉
