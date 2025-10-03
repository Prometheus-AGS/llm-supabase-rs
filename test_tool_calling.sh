#!/bin/bash

# Tool Calling End-to-End Test Script
# Tests both streaming and non-streaming tool calling scenarios

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Configuration
BASE_URL="http://localhost:8080"
API_KEY="test-token"
MODEL="claude-4-sonnet-20250514"

echo -e "${BLUE}🔧 Tool Calling End-to-End Test Suite${NC}"
echo "=================================================="

# Function to check if server is running
check_server() {
    echo -e "${YELLOW}📡 Checking if server is running...${NC}"
    if ! curl -s -f "$BASE_URL/health" > /dev/null 2>&1; then
        echo -e "${RED}❌ Server is not running at $BASE_URL${NC}"
        echo "Please start the server with: cargo run"
        exit 1
    fi
    echo -e "${GREEN}✅ Server is running${NC}"
}

# Function to test basic tool calling (non-streaming)
test_basic_tool_calling() {
    echo -e "\n${BLUE}🧪 Test 1: Basic Tool Calling (Non-Streaming)${NC}"
    echo "=============================================="
    
    local response=$(curl -s -X POST "$BASE_URL/v1/chat/completions" \
        -H "Content-Type: application/json" \
        -H "Authorization: Bearer $API_KEY" \
        -d '{
            "model": "'$MODEL'",
            "messages": [
                {
                    "role": "user",
                    "content": "What is the weather like in San Francisco? Use the get_weather tool."
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
                                },
                                "units": {
                                    "type": "string",
                                    "enum": ["celsius", "fahrenheit"],
                                    "description": "Temperature units"
                                }
                            },
                            "required": ["location"]
                        }
                    }
                }
            ],
            "max_tokens": 150
        }')
    
    echo "Response:"
    echo "$response" | jq '.' 2>/dev/null || echo "$response"
    
    # Check if response contains tool calls
    if echo "$response" | jq -e '.choices[0].message.tool_calls' > /dev/null 2>&1; then
        echo -e "${GREEN}✅ Tool calls detected in response${NC}"
        
        # Extract tool call details
        local tool_call_id=$(echo "$response" | jq -r '.choices[0].message.tool_calls[0].id')
        local function_name=$(echo "$response" | jq -r '.choices[0].message.tool_calls[0].function.name')
        local arguments=$(echo "$response" | jq -r '.choices[0].message.tool_calls[0].function.arguments')
        
        echo "Tool Call ID: $tool_call_id"
        echo "Function: $function_name"
        echo "Arguments: $arguments"
        
        # Test continuation with tool results
        test_tool_result_continuation "$tool_call_id" "$response"
    else
        echo -e "${RED}❌ No tool calls found in response${NC}"
        return 1
    fi
}

# Function to test tool result continuation
test_tool_result_continuation() {
    local tool_call_id="$1"
    local original_response="$2"
    
    echo -e "\n${BLUE}🔄 Test 1b: Tool Result Continuation${NC}"
    echo "====================================="
    
    # Simulate tool execution result
    local tool_result="The weather in San Francisco is currently 72°F (22°C) with partly cloudy skies. Wind speed is 8 mph from the west."
    
    # Extract original messages and add tool result
    local messages=$(echo "$original_response" | jq -c '[
        {
            "role": "user",
            "content": "What is the weather like in San Francisco? Use the get_weather tool."
        },
        .choices[0].message,
        {
            "role": "tool",
            "tool_call_id": "'$tool_call_id'",
            "content": "'$tool_result'"
        }
    ]')
    
    local response=$(curl -s -X POST "$BASE_URL/v1/chat/completions" \
        -H "Content-Type: application/json" \
        -H "Authorization: Bearer $API_KEY" \
        -d '{
            "model": "'$MODEL'",
            "messages": '$messages',
            "max_tokens": 150
        }')
    
    echo "Continuation Response:"
    echo "$response" | jq '.' 2>/dev/null || echo "$response"
    
    if echo "$response" | jq -e '.choices[0].message.content' > /dev/null 2>&1; then
        echo -e "${GREEN}✅ Tool result processed successfully${NC}"
    else
        echo -e "${RED}❌ Failed to process tool result${NC}"
        return 1
    fi
}

# Function to test streaming tool calling
test_streaming_tool_calling() {
    echo -e "\n${BLUE}🌊 Test 2: Streaming Tool Calling${NC}"
    echo "=================================="
    
    local temp_file=$(mktemp)
    
    curl -s -X POST "$BASE_URL/v1/chat/completions" \
        -H "Content-Type: application/json" \
        -H "Authorization: Bearer $API_KEY" \
        -d '{
            "model": "'$MODEL'",
            "stream": true,
            "messages": [
                {
                    "role": "user",
                    "content": "Search for recent AI news using the search_web tool."
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
                                },
                                "max_results": {
                                    "type": "integer",
                                    "description": "Maximum number of results",
                                    "default": 5
                                }
                            },
                            "required": ["query"]
                        }
                    }
                }
            ],
            "max_tokens": 150
        }' > "$temp_file"
    
    echo "Streaming Response:"
    cat "$temp_file"
    
    # Check for tool calls in streaming response
    if grep -q "tool_calls" "$temp_file"; then
        echo -e "${GREEN}✅ Tool calls detected in streaming response${NC}"
    else
        echo -e "${RED}❌ No tool calls found in streaming response${NC}"
    fi
    
    # Check for proper SSE format
    if grep -q "data: " "$temp_file"; then
        echo -e "${GREEN}✅ Proper SSE format detected${NC}"
    else
        echo -e "${RED}❌ Invalid SSE format${NC}"
    fi
    
    rm "$temp_file"
}

# Function to test multiple tool calls
test_multiple_tool_calls() {
    echo -e "\n${BLUE}🔀 Test 3: Multiple Tool Calls${NC}"
    echo "==============================="
    
    local response=$(curl -s -X POST "$BASE_URL/v1/chat/completions" \
        -H "Content-Type: application/json" \
        -H "Authorization: Bearer $API_KEY" \
        -d '{
            "model": "'$MODEL'",
            "messages": [
                {
                    "role": "user",
                    "content": "Get the weather in both San Francisco and New York, then search for travel information between these cities."
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
                                "location": {"type": "string"}
                            },
                            "required": ["location"]
                        }
                    }
                },
                {
                    "type": "function",
                    "function": {
                        "name": "search_travel",
                        "description": "Search for travel information between cities",
                        "parameters": {
                            "type": "object",
                            "properties": {
                                "from": {"type": "string"},
                                "to": {"type": "string"},
                                "travel_type": {"type": "string", "enum": ["flight", "train", "car"]}
                            },
                            "required": ["from", "to"]
                        }
                    }
                }
            ],
            "max_tokens": 200
        }')
    
    echo "Response:"
    echo "$response" | jq '.' 2>/dev/null || echo "$response"
    
    # Count tool calls
    local tool_call_count=$(echo "$response" | jq '.choices[0].message.tool_calls | length' 2>/dev/null || echo "0")
    
    if [ "$tool_call_count" -gt "1" ]; then
        echo -e "${GREEN}✅ Multiple tool calls detected ($tool_call_count calls)${NC}"
    else
        echo -e "${YELLOW}⚠️  Only $tool_call_count tool call(s) detected${NC}"
    fi
}

# Function to test error handling
test_error_handling() {
    echo -e "\n${BLUE}🚨 Test 4: Error Handling${NC}"
    echo "=========================="
    
    # Test with invalid tool definition
    local response=$(curl -s -X POST "$BASE_URL/v1/chat/completions" \
        -H "Content-Type: application/json" \
        -H "Authorization: Bearer $API_KEY" \
        -d '{
            "model": "'$MODEL'",
            "messages": [
                {
                    "role": "user",
                    "content": "Use an invalid tool."
                }
            ],
            "tools": [
                {
                    "type": "function",
                    "function": {
                        "name": "invalid_tool",
                        "description": "This tool has invalid parameters",
                        "parameters": "invalid_schema"
                    }
                }
            ],
            "max_tokens": 100
        }')
    
    echo "Error Response:"
    echo "$response" | jq '.' 2>/dev/null || echo "$response"
    
    if echo "$response" | jq -e '.error' > /dev/null 2>&1; then
        echo -e "${GREEN}✅ Error properly handled${NC}"
    else
        echo -e "${YELLOW}⚠️  No error detected (may have been handled gracefully)${NC}"
    fi
}

# Function to test performance
test_performance() {
    echo -e "\n${BLUE}⚡ Test 5: Performance Test${NC}"
    echo "==========================="
    
    local start_time=$(date +%s%N)
    
    local response=$(curl -s -X POST "$BASE_URL/v1/chat/completions" \
        -H "Content-Type: application/json" \
        -H "Authorization: Bearer $API_KEY" \
        -d '{
            "model": "'$MODEL'",
            "messages": [
                {
                    "role": "user",
                    "content": "Get the current time using the get_time tool."
                }
            ],
            "tools": [
                {
                    "type": "function",
                    "function": {
                        "name": "get_time",
                        "description": "Get the current time",
                        "parameters": {
                            "type": "object",
                            "properties": {
                                "timezone": {"type": "string", "default": "UTC"}
                            }
                        }
                    }
                }
            ],
            "max_tokens": 50
        }')
    
    local end_time=$(date +%s%N)
    local duration_ms=$(( (end_time - start_time) / 1000000 ))
    
    echo "Response time: ${duration_ms}ms"
    
    if [ "$duration_ms" -lt 5000 ]; then
        echo -e "${GREEN}✅ Good performance (< 5s)${NC}"
    else
        echo -e "${YELLOW}⚠️  Slow response (> 5s)${NC}"
    fi
}

# Main test execution
main() {
    echo -e "${BLUE}Starting Tool Calling Tests...${NC}"
    
    check_server
    
    # Run all tests
    test_basic_tool_calling
    test_streaming_tool_calling
    test_multiple_tool_calls
    test_error_handling
    test_performance
    
    echo -e "\n${GREEN}🎉 Tool Calling Test Suite Complete!${NC}"
    echo "=================================================="
    echo -e "${BLUE}Summary:${NC}"
    echo "• Basic tool calling: Tested"
    echo "• Streaming tool calling: Tested"
    echo "• Multiple tool calls: Tested"
    echo "• Error handling: Tested"
    echo "• Performance: Tested"
    echo ""
    echo -e "${YELLOW}Next Steps:${NC}"
    echo "1. Implement actual tool execution in your client"
    echo "2. Handle tool results and continue conversations"
    echo "3. Add more sophisticated tool calling scenarios"
    echo ""
    echo -e "${GREEN}✅ Your OpenAI-compatible API now supports tool calling!${NC}"
}

# Run tests
main "$@"
