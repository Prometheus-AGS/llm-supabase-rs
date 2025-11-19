#!/bin/bash

# test_streaming.sh - Comprehensive streaming test script for OpenAI-compatible API
# Tests both streaming and non-streaming endpoints with various configurations

set -e  # Exit on any error

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Configuration
BASE_URL="${BASE_URL:-http://localhost:8080}"
API_TOKEN="${API_TOKEN:-test-token}"
MODEL="${MODEL:-claude-4-sonnet-20250514}"
TIMEOUT="${TIMEOUT:-30}"

# Test results
TESTS_PASSED=0
TESTS_FAILED=0
TOTAL_TESTS=0

# Helper functions
log_info() {
    echo -e "${BLUE}[INFO]${NC} $1"
}

log_success() {
    echo -e "${GREEN}[SUCCESS]${NC} $1"
    ((TESTS_PASSED++))
}

log_error() {
    echo -e "${RED}[ERROR]${NC} $1"
    ((TESTS_FAILED++))
}

log_warning() {
    echo -e "${YELLOW}[WARNING]${NC} $1"
}

# Test counter
test_start() {
    ((TOTAL_TESTS++))
    echo -e "\n${BLUE}=== Test $TOTAL_TESTS: $1 ===${NC}"
}

# Check if server is running
check_server() {
    log_info "Checking if server is running at $BASE_URL..."
    
    if curl -s -f "$BASE_URL/health" > /dev/null 2>&1; then
        log_success "Server is running"
        return 0
    else
        log_error "Server is not running at $BASE_URL"
        log_info "Please start the server with: cargo run"
        exit 1
    fi
}

# Test 1: Health Check
test_health() {
    test_start "Health Check"
    
    response=$(curl -s -w "%{http_code}" "$BASE_URL/health")
    http_code="${response: -3}"
    body="${response%???}"
    
    if [ "$http_code" = "200" ]; then
        log_success "Health check passed (HTTP $http_code)"
        log_info "Response: $body"
    else
        log_error "Health check failed (HTTP $http_code)"
        log_info "Response: $body"
    fi
}

# Test 2: Non-streaming Chat Completion
test_non_streaming() {
    test_start "Non-streaming Chat Completion"
    
    local request_body='{
        "model": "'$MODEL'",
        "messages": [
            {
                "role": "user",
                "content": "Say hello in exactly 3 words"
            }
        ],
        "max_tokens": 10,
        "temperature": 0.1,
        "stream": false
    }'
    
    log_info "Making non-streaming request..."
    response=$(curl -s -w "\n%{http_code}" \
        -X POST "$BASE_URL/v1/chat/completions" \
        -H "Content-Type: application/json" \
        -H "Authorization: Bearer $API_TOKEN" \
        -d "$request_body" \
        --max-time $TIMEOUT)
    
    http_code=$(echo "$response" | tail -n1)
    body=$(echo "$response" | head -n -1)
    
    if [ "$http_code" = "200" ]; then
        log_success "Non-streaming request successful (HTTP $http_code)"
        
        # Parse and validate JSON response
        if echo "$body" | jq . > /dev/null 2>&1; then
            log_success "Response is valid JSON"
            
            # Check required fields
            object=$(echo "$body" | jq -r '.object // empty')
            model=$(echo "$body" | jq -r '.model // empty')
            content=$(echo "$body" | jq -r '.choices[0].message.content // empty')
            
            if [ "$object" = "chat.completion" ]; then
                log_success "Response object type is correct: $object"
            else
                log_error "Unexpected object type: $object"
            fi
            
            if [ -n "$content" ]; then
                log_success "Response contains content: '$content'"
            else
                log_error "Response missing content"
            fi
            
            log_info "Full response:"
            echo "$body" | jq .
        else
            log_error "Response is not valid JSON"
            log_info "Response body: $body"
        fi
    else
        log_error "Non-streaming request failed (HTTP $http_code)"
        log_info "Response: $body"
    fi
}

# Test 3: Streaming Chat Completion (Your Original Issue)
test_streaming() {
    test_start "Streaming Chat Completion (Original Issue)"
    
    local request_body='{
        "model": "'$MODEL'",
        "stream": true,
        "messages": [
            {
                "role": "user",
                "content": "tell me something good"
            }
        ]
    }'
    
    log_info "Making streaming request (your original failing request)..."
    log_info "Request body: $request_body"
    
    # Create temporary file for response
    local temp_file=$(mktemp)
    local headers_file=$(mktemp)
    
    # Make streaming request
    curl -s -w "%{http_code}" \
        -X POST "$BASE_URL/v1/chat/completions" \
        -H "Content-Type: application/json" \
        -H "Authorization: Bearer $API_TOKEN" \
        -d "$request_body" \
        --max-time $TIMEOUT \
        -D "$headers_file" \
        -o "$temp_file"
    
    local http_code=$?
    local response_body=$(cat "$temp_file")
    local headers=$(cat "$headers_file")
    
    # Check headers
    log_info "Response headers:"
    echo "$headers"
    
    if echo "$headers" | grep -q "text/event-stream"; then
        log_success "Correct Content-Type: text/event-stream"
    else
        log_error "Missing or incorrect Content-Type header"
    fi
    
    if echo "$headers" | grep -q "no-cache"; then
        log_success "Correct Cache-Control: no-cache"
    else
        log_warning "Missing Cache-Control: no-cache header"
    fi
    
    # Analyze streaming response
    log_info "Streaming response analysis:"
    echo "--- Raw Response ---"
    cat "$temp_file"
    echo "--- End Raw Response ---"
    
    # Count chunks
    local chunk_count=$(grep -c "^data: " "$temp_file" 2>/dev/null || echo "0")
    local done_count=$(grep -c "data: \[DONE\]" "$temp_file" 2>/dev/null || echo "0")
    local content_chunks=$(grep "data: {" "$temp_file" | grep -c "content" 2>/dev/null || echo "0")
    
    log_info "Streaming analysis:"
    log_info "  Total data chunks: $chunk_count"
    log_info "  Content chunks: $content_chunks"
    log_info "  [DONE] chunks: $done_count"
    
    if [ "$chunk_count" -gt 0 ]; then
        log_success "Received streaming chunks"
        
        if [ "$content_chunks" -gt 0 ]; then
            log_success "Received content chunks (streaming is working!)"
        else
            log_warning "Only received [DONE] - no content chunks (this was your original issue)"
            log_info "This suggests the 'message' event type issue we found in logs"
        fi
        
        if [ "$done_count" -gt 0 ]; then
            log_success "Received [DONE] termination"
        else
            log_warning "Missing [DONE] termination"
        fi
    else
        log_error "No streaming chunks received"
    fi
    
    # Clean up
    rm -f "$temp_file" "$headers_file"
}

# Test 4: Streaming with Parameters
test_streaming_with_params() {
    test_start "Streaming with Parameters"
    
    local request_body='{
        "model": "'$MODEL'",
        "stream": true,
        "messages": [
            {
                "role": "system",
                "content": "You are a helpful assistant. Be very brief."
            },
            {
                "role": "user",
                "content": "Count from 1 to 3"
            }
        ],
        "max_tokens": 20,
        "temperature": 0.1,
        "top_p": 0.9
    }'
    
    log_info "Testing streaming with multiple parameters..."
    
    local temp_file=$(mktemp)
    
    curl -s \
        -X POST "$BASE_URL/v1/chat/completions" \
        -H "Content-Type: application/json" \
        -H "Authorization: Bearer $API_TOKEN" \
        -d "$request_body" \
        --max-time $TIMEOUT \
        -o "$temp_file"
    
    local content_chunks=$(grep "data: {" "$temp_file" | grep -c "content" 2>/dev/null || echo "0")
    
    if [ "$content_chunks" -gt 0 ]; then
        log_success "Streaming with parameters works ($content_chunks content chunks)"
    else
        log_error "Streaming with parameters failed"
    fi
    
    rm -f "$temp_file"
}

# Test 5: Complete OpenAI Parameter Support
test_complete_parameters() {
    test_start "Complete OpenAI Parameter Support"
    
    local request_body='{
        "model": "'$MODEL'",
        "messages": [
            {
                "role": "user",
                "content": "Say OK"
            }
        ],
        "max_tokens": 5,
        "temperature": 0.7,
        "top_p": 0.9,
        "n": 1,
        "stream": false,
        "stop": ["END"],
        "presence_penalty": 0.1,
        "frequency_penalty": 0.1,
        "logit_bias": {},
        "logprobs": false,
        "top_logprobs": null,
        "user": "test-user-123",
        "functions": null,
        "function_call": null,
        "tools": null,
        "tool_choice": null,
        "parallel_tool_calls": null,
        "response_format": null,
        "seed": 42,
        "metadata": {
            "test": "complete_params"
        },
        "service_tier": null,
        "store": false,
        "stream_options": null
    }'
    
    log_info "Testing with all OpenAI parameters..."
    
    response=$(curl -s -w "%{http_code}" \
        -X POST "$BASE_URL/v1/chat/completions" \
        -H "Content-Type: application/json" \
        -H "Authorization: Bearer $API_TOKEN" \
        -d "$request_body" \
        --max-time $TIMEOUT)
    
    http_code="${response: -3}"
    
    if [ "$http_code" = "200" ]; then
        log_success "All OpenAI parameters accepted (HTTP $http_code)"
    else
        log_error "Parameter validation failed (HTTP $http_code)"
    fi
}

# Test 6: Error Handling
test_error_handling() {
    test_start "Error Handling"
    
    # Test with invalid model
    local request_body='{
        "model": "invalid-model-name",
        "messages": [
            {
                "role": "user",
                "content": "This should fail"
            }
        ]
    }'
    
    log_info "Testing error handling with invalid model..."
    
    response=$(curl -s -w "%{http_code}" \
        -X POST "$BASE_URL/v1/chat/completions" \
        -H "Content-Type: application/json" \
        -H "Authorization: Bearer $API_TOKEN" \
        -d "$request_body" \
        --max-time $TIMEOUT)
    
    http_code="${response: -3}"
    
    if [ "$http_code" = "400" ] || [ "$http_code" = "422" ]; then
        log_success "Error handling works (HTTP $http_code)"
    else
        log_warning "Unexpected error response (HTTP $http_code)"
    fi
}

# Test 7: Performance Test
test_performance() {
    test_start "Performance Test"
    
    local request_body='{
        "model": "'$MODEL'",
        "messages": [
            {
                "role": "user",
                "content": "Say hello"
            }
        ],
        "max_tokens": 10,
        "stream": false
    }'
    
    log_info "Testing response time..."
    
    local start_time=$(date +%s%N)
    
    response=$(curl -s -w "%{http_code}" \
        -X POST "$BASE_URL/v1/chat/completions" \
        -H "Content-Type: application/json" \
        -H "Authorization: Bearer $API_TOKEN" \
        -d "$request_body" \
        --max-time $TIMEOUT)
    
    local end_time=$(date +%s%N)
    local duration_ms=$(( (end_time - start_time) / 1000000 ))
    
    http_code="${response: -3}"
    
    if [ "$http_code" = "200" ]; then
        log_success "Performance test completed in ${duration_ms}ms"
        
        if [ "$duration_ms" -lt 5000 ]; then
            log_success "Response time is good (< 5s)"
        else
            log_warning "Response time is slow (> 5s)"
        fi
    else
        log_error "Performance test failed (HTTP $http_code)"
    fi
}

# Main test runner
run_all_tests() {
    echo -e "${BLUE}🚀 Starting OpenAI-Compatible API Tests${NC}"
    echo -e "${BLUE}Server: $BASE_URL${NC}"
    echo -e "${BLUE}Model: $MODEL${NC}"
    echo -e "${BLUE}Timeout: ${TIMEOUT}s${NC}"
    echo ""
    
    # Check server availability
    check_server
    
    # Run all tests
    test_health
    test_non_streaming
    test_streaming  # This is your original failing test
    test_streaming_with_params
    test_complete_parameters
    test_error_handling
    test_performance
    
    # Summary
    echo -e "\n${BLUE}=== Test Summary ===${NC}"
    echo -e "Total tests: $TOTAL_TESTS"
    echo -e "${GREEN}Passed: $TESTS_PASSED${NC}"
    echo -e "${RED}Failed: $TESTS_FAILED${NC}"
    
    if [ "$TESTS_FAILED" -eq 0 ]; then
        echo -e "\n${GREEN}🎉 All tests passed! Your OpenAI-compatible API is working correctly.${NC}"
        exit 0
    else
        echo -e "\n${RED}❌ Some tests failed. Check the output above for details.${NC}"
        exit 1
    fi
}

# Command line options
case "${1:-}" in
    "health")
        check_server && test_health
        ;;
    "streaming")
        check_server && test_streaming
        ;;
    "non-streaming")
        check_server && test_non_streaming
        ;;
    "performance")
        check_server && test_performance
        ;;
    "help"|"-h"|"--help")
        echo "Usage: $0 [test_name]"
        echo ""
        echo "Available tests:"
        echo "  health        - Test health endpoint"
        echo "  streaming     - Test streaming (your original issue)"
        echo "  non-streaming - Test non-streaming completion"
        echo "  performance   - Test response time"
        echo "  (no args)     - Run all tests"
        echo ""
        echo "Environment variables:"
        echo "  BASE_URL      - API base URL (default: http://localhost:8080)"
        echo "  API_TOKEN     - Authorization token (default: test-token)"
        echo "  MODEL         - Model name (default: claude-4-sonnet-20250514)"
        echo "  TIMEOUT       - Request timeout in seconds (default: 30)"
        ;;
    "")
        run_all_tests
        ;;
    *)
        echo "Unknown test: $1"
        echo "Run '$0 help' for usage information"
        exit 1
        ;;
esac
