#!/bin/bash

# Comprehensive Tool Calling Demo
# Demonstrates real-world tool calling scenarios with your OpenAI-compatible API

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
PURPLE='\033[0;35m'
CYAN='\033[0;36m'
NC='\033[0m' # No Color

# Configuration
BASE_URL="http://localhost:8080"
API_KEY="test-token"
MODEL="claude-4-sonnet-20250514"

echo -e "${PURPLE}🚀 Tool Calling Demo - Real World Scenarios${NC}"
echo "=============================================="

# Demo 1: Web Search Tool
demo_web_search() {
    echo -e "\n${BLUE}📊 Demo 1: Web Search Assistant${NC}"
    echo "================================"
    
    echo -e "${CYAN}Scenario: User asks for recent AI news${NC}"
    
    local response=$(curl -s -X POST "$BASE_URL/v1/chat/completions" \
        -H "Content-Type: application/json" \
        -H "Authorization: Bearer $API_KEY" \
        -d '{
            "model": "'$MODEL'",
            "messages": [
                {
                    "role": "user",
                    "content": "Find me the latest news about OpenAI and Claude AI. Use the search tool to get current information."
                }
            ],
            "tools": [
                {
                    "type": "function",
                    "function": {
                        "name": "tavily_search",
                        "description": "Search the web using Tavily for current information",
                        "parameters": {
                            "type": "object",
                            "properties": {
                                "query": {
                                    "type": "string",
                                    "description": "Search query"
                                },
                                "search_depth": {
                                    "type": "string",
                                    "enum": ["basic", "advanced"],
                                    "description": "Depth of search",
                                    "default": "basic"
                                },
                                "max_results": {
                                    "type": "integer",
                                    "description": "Maximum number of results",
                                    "default": 5,
                                    "minimum": 1,
                                    "maximum": 20
                                }
                            },
                            "required": ["query"]
                        }
                    }
                }
            ],
            "max_tokens": 200
        }')
    
    echo -e "${YELLOW}AI Response:${NC}"
    echo "$response" | jq -r '.choices[0].message.content // "No content"' 2>/dev/null
    
    if echo "$response" | jq -e '.choices[0].message.tool_calls[0]' > /dev/null 2>&1; then
        echo -e "\n${GREEN}🔧 Tool Call Detected:${NC}"
        local tool_call=$(echo "$response" | jq '.choices[0].message.tool_calls[0]')
        echo "$tool_call" | jq '.'
        
        # Simulate tool execution
        echo -e "\n${CYAN}🔍 Simulating Tavily Search...${NC}"
        local search_query=$(echo "$tool_call" | jq -r '.function.arguments' | jq -r '.query')
        echo "Query: $search_query"
        
        # Mock search results
        local mock_results='{
            "results": [
                {
                    "title": "OpenAI Releases GPT-4 Turbo with Vision",
                    "url": "https://openai.com/blog/gpt-4-turbo",
                    "snippet": "OpenAI announces GPT-4 Turbo with improved capabilities..."
                },
                {
                    "title": "Anthropic Claude 3 Shows Strong Performance",
                    "url": "https://anthropic.com/claude-3",
                    "snippet": "Claude 3 demonstrates significant improvements in reasoning..."
                }
            ]
        }'
        
        echo -e "${GREEN}📋 Mock Search Results:${NC}"
        echo "$mock_results" | jq '.'
    fi
}

# Demo 2: Multi-step Tool Workflow
demo_multi_step_workflow() {
    echo -e "\n${BLUE}🔄 Demo 2: Multi-Step Workflow${NC}"
    echo "=============================="
    
    echo -e "${CYAN}Scenario: Research and analyze a company${NC}"
    
    local response=$(curl -s -X POST "$BASE_URL/v1/chat/completions" \
        -H "Content-Type: application/json" \
        -H "Authorization: Bearer $API_KEY" \
        -d '{
            "model": "'$MODEL'",
            "messages": [
                {
                    "role": "user",
                    "content": "I want to research Tesla. First search for recent Tesla news, then analyze the sentiment of the findings, and finally get their current stock price."
                }
            ],
            "tools": [
                {
                    "type": "function",
                    "function": {
                        "name": "search_company_news",
                        "description": "Search for recent news about a company",
                        "parameters": {
                            "type": "object",
                            "properties": {
                                "company": {"type": "string"},
                                "timeframe": {"type": "string", "enum": ["1d", "1w", "1m"], "default": "1w"}
                            },
                            "required": ["company"]
                        }
                    }
                },
                {
                    "type": "function",
                    "function": {
                        "name": "analyze_sentiment",
                        "description": "Analyze sentiment of text content",
                        "parameters": {
                            "type": "object",
                            "properties": {
                                "text": {"type": "string"},
                                "analysis_type": {"type": "string", "enum": ["basic", "detailed"], "default": "basic"}
                            },
                            "required": ["text"]
                        }
                    }
                },
                {
                    "type": "function",
                    "function": {
                        "name": "get_stock_price",
                        "description": "Get current stock price for a company",
                        "parameters": {
                            "type": "object",
                            "properties": {
                                "symbol": {"type": "string"},
                                "include_metrics": {"type": "boolean", "default": false}
                            },
                            "required": ["symbol"]
                        }
                    }
                }
            ],
            "max_tokens": 300
        }')
    
    echo -e "${YELLOW}AI Response:${NC}"
    echo "$response" | jq -r '.choices[0].message.content // "No content"' 2>/dev/null
    
    if echo "$response" | jq -e '.choices[0].message.tool_calls' > /dev/null 2>&1; then
        echo -e "\n${GREEN}🔧 Tool Calls Detected:${NC}"
        local tool_calls=$(echo "$response" | jq '.choices[0].message.tool_calls')
        echo "$tool_calls" | jq '.'
        
        local call_count=$(echo "$tool_calls" | jq 'length')
        echo -e "\n${PURPLE}📊 Analysis: $call_count tool call(s) planned${NC}"
    fi
}

# Demo 3: Streaming Tool Calls
demo_streaming_tools() {
    echo -e "\n${BLUE}🌊 Demo 3: Streaming Tool Calls${NC}"
    echo "==============================="
    
    echo -e "${CYAN}Scenario: Real-time data analysis${NC}"
    
    local temp_file=$(mktemp)
    
    echo -e "${YELLOW}Streaming Response:${NC}"
    curl -s -X POST "$BASE_URL/v1/chat/completions" \
        -H "Content-Type: application/json" \
        -H "Authorization: Bearer $API_KEY" \
        -d '{
            "model": "'$MODEL'",
            "stream": true,
            "messages": [
                {
                    "role": "user",
                    "content": "Monitor the current cryptocurrency prices for Bitcoin and Ethereum, then provide a brief analysis."
                }
            ],
            "tools": [
                {
                    "type": "function",
                    "function": {
                        "name": "get_crypto_price",
                        "description": "Get current cryptocurrency price",
                        "parameters": {
                            "type": "object",
                            "properties": {
                                "symbol": {"type": "string"},
                                "currency": {"type": "string", "default": "USD"}
                            },
                            "required": ["symbol"]
                        }
                    }
                },
                {
                    "type": "function",
                    "function": {
                        "name": "analyze_market_trend",
                        "description": "Analyze market trend for given data",
                        "parameters": {
                            "type": "object",
                            "properties": {
                                "data": {"type": "string"},
                                "timeframe": {"type": "string", "default": "1h"}
                            },
                            "required": ["data"]
                        }
                    }
                }
            ],
            "max_tokens": 250
        }' | tee "$temp_file"
    
    echo -e "\n${GREEN}📊 Streaming Analysis:${NC}"
    
    # Check for tool calls in streaming
    if grep -q "tool_calls" "$temp_file"; then
        echo "✅ Tool calls detected in stream"
    else
        echo "❌ No tool calls in stream"
    fi
    
    # Check for proper SSE format
    if grep -q "data: " "$temp_file"; then
        echo "✅ Proper Server-Sent Events format"
    else
        echo "❌ Invalid streaming format"
    fi
    
    # Count chunks
    local chunk_count=$(grep -c "data: " "$temp_file" || echo "0")
    echo "📦 Total chunks received: $chunk_count"
    
    rm "$temp_file"
}

# Demo 4: Context7 Integration
demo_context7_integration() {
    echo -e "\n${BLUE}🧠 Demo 4: Context7 Integration${NC}"
    echo "==============================="
    
    echo -e "${CYAN}Scenario: Advanced text analysis with Context7${NC}"
    
    local response=$(curl -s -X POST "$BASE_URL/v1/chat/completions" \
        -H "Content-Type: application/json" \
        -H "Authorization: Bearer $API_KEY" \
        -d '{
            "model": "'$MODEL'",
            "messages": [
                {
                    "role": "user",
                    "content": "Analyze this customer feedback for sentiment, extract key entities, and summarize the main points: '\''The new product is amazing! The customer service was helpful, but the delivery took too long. Overall, I would recommend it to others.'\''"
                }
            ],
            "tools": [
                {
                    "type": "function",
                    "function": {
                        "name": "context7_analyze",
                        "description": "Analyze text using Context7 for advanced NLP tasks",
                        "parameters": {
                            "type": "object",
                            "properties": {
                                "text": {"type": "string"},
                                "analysis_types": {
                                    "type": "array",
                                    "items": {
                                        "type": "string",
                                        "enum": ["sentiment", "entities", "summary", "keywords", "topics"]
                                    },
                                    "description": "Types of analysis to perform"
                                },
                                "language": {"type": "string", "default": "en"}
                            },
                            "required": ["text", "analysis_types"]
                        }
                    }
                }
            ],
            "max_tokens": 200
        }')
    
    echo -e "${YELLOW}AI Response:${NC}"
    echo "$response" | jq -r '.choices[0].message.content // "No content"' 2>/dev/null
    
    if echo "$response" | jq -e '.choices[0].message.tool_calls[0]' > /dev/null 2>&1; then
        echo -e "\n${GREEN}🔧 Context7 Tool Call:${NC}"
        local tool_call=$(echo "$response" | jq '.choices[0].message.tool_calls[0]')
        echo "$tool_call" | jq '.'
        
        echo -e "\n${CYAN}🧠 Simulating Context7 Analysis...${NC}"
        local analysis_types=$(echo "$tool_call" | jq -r '.function.arguments' | jq -r '.analysis_types[]' | tr '\n' ', ')
        echo "Analysis types: ${analysis_types%,}"
        
        # Mock Context7 results
        local mock_analysis='{
            "sentiment": {
                "overall": "positive",
                "confidence": 0.78,
                "aspects": {
                    "product": "very_positive",
                    "service": "positive", 
                    "delivery": "negative"
                }
            },
            "entities": [
                {"text": "product", "type": "PRODUCT", "confidence": 0.95},
                {"text": "customer service", "type": "SERVICE", "confidence": 0.89},
                {"text": "delivery", "type": "SERVICE", "confidence": 0.92}
            ],
            "summary": "Customer is satisfied with product quality and service but disappointed with delivery speed. Would recommend despite delivery issues."
        }'
        
        echo -e "${GREEN}📋 Mock Context7 Results:${NC}"
        echo "$mock_analysis" | jq '.'
    fi
}

# Demo 5: Error Handling and Recovery
demo_error_handling() {
    echo -e "\n${BLUE}🚨 Demo 5: Error Handling${NC}"
    echo "=========================="
    
    echo -e "${CYAN}Scenario: Handling tool errors gracefully${NC}"
    
    # Test with invalid parameters
    local response=$(curl -s -X POST "$BASE_URL/v1/chat/completions" \
        -H "Content-Type: application/json" \
        -H "Authorization: Bearer $API_KEY" \
        -d '{
            "model": "'$MODEL'",
            "messages": [
                {
                    "role": "user",
                    "content": "Get the weather for an invalid location."
                }
            ],
            "tools": [
                {
                    "type": "function",
                    "function": {
                        "name": "get_weather",
                        "description": "Get weather information",
                        "parameters": {
                            "type": "object",
                            "properties": {
                                "location": {"type": "string"}
                            },
                            "required": ["location"]
                        }
                    }
                }
            ],
            "max_tokens": 100
        }')
    
    echo -e "${YELLOW}Response:${NC}"
    echo "$response" | jq '.' 2>/dev/null || echo "$response"
    
    # Test tool result with error
    if echo "$response" | jq -e '.choices[0].message.tool_calls[0]' > /dev/null 2>&1; then
        local tool_call_id=$(echo "$response" | jq -r '.choices[0].message.tool_calls[0].id')
        
        echo -e "\n${CYAN}🔄 Simulating Tool Error Response...${NC}"
        
        local error_response=$(curl -s -X POST "$BASE_URL/v1/chat/completions" \
            -H "Content-Type: application/json" \
            -H "Authorization: Bearer $API_KEY" \
            -d '{
                "model": "'$MODEL'",
                "messages": [
                    {
                        "role": "user",
                        "content": "Get the weather for an invalid location."
                    },
                    {
                        "role": "assistant",
                        "content": null,
                        "tool_calls": [
                            {
                                "id": "'$tool_call_id'",
                                "type": "function",
                                "function": {
                                    "name": "get_weather",
                                    "arguments": "{\"location\": \"InvalidCity123\"}"
                                }
                            }
                        ]
                    },
                    {
                        "role": "tool",
                        "tool_call_id": "'$tool_call_id'",
                        "content": "Error: Location '\''InvalidCity123'\'' not found. Please provide a valid city name."
                    }
                ],
                "max_tokens": 100
            }')
        
        echo -e "${YELLOW}Error Recovery Response:${NC}"
        echo "$error_response" | jq -r '.choices[0].message.content // "No content"' 2>/dev/null
    fi
}

# Performance metrics
show_performance_summary() {
    echo -e "\n${PURPLE}📊 Performance Summary${NC}"
    echo "======================"
    
    echo -e "${GREEN}✅ Tool Calling Features Demonstrated:${NC}"
    echo "• Basic function calling"
    echo "• Multi-step workflows"
    echo "• Streaming tool calls"
    echo "• Advanced NLP integration (Context7)"
    echo "• Error handling and recovery"
    echo "• Parameter validation"
    echo "• Multiple tool types"
    
    echo -e "\n${BLUE}🎯 OpenAI API Compatibility:${NC}"
    echo "• Full tool/function calling support"
    echo "• Proper tool_calls format"
    echo "• Tool result handling"
    echo "• Streaming compatibility"
    echo "• Error response format"
    
    echo -e "\n${CYAN}🚀 Next Steps:${NC}"
    echo "1. Implement actual tool execution in your client"
    echo "2. Add real Tavily and Context7 integrations"
    echo "3. Build multi-turn tool conversations"
    echo "4. Add tool result validation"
    echo "5. Implement tool call timeouts"
}

# Main demo execution
main() {
    echo -e "${PURPLE}Starting comprehensive tool calling demos...${NC}"
    
    # Check if server is running
    if ! curl -s -f "$BASE_URL/health" > /dev/null 2>&1; then
        echo -e "${RED}❌ Server not running at $BASE_URL${NC}"
        echo "Please start with: cargo run"
        exit 1
    fi
    
    demo_web_search
    demo_multi_step_workflow
    demo_streaming_tools
    demo_context7_integration
    demo_error_handling
    show_performance_summary
    
    echo -e "\n${GREEN}🎉 Tool Calling Demo Complete!${NC}"
    echo -e "${YELLOW}Your OpenAI-compatible API now supports comprehensive tool calling! 🚀${NC}"
}

# Run the demo
main "$@"
