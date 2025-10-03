#!/bin/bash

echo "🚀 Testing Streaming Chat Completions with claude-sonnet-4@20250514"
echo "=================================================================="

# Start the server in the background if not already running
if ! pgrep -f "llm-supabase-rs" > /dev/null; then
    echo "🔧 Starting server..."
    cargo build --release
    ./target/release/llm-supabase-rs &
    SERVER_PID=$!
    echo "⏳ Waiting for server to start..."
    sleep 5
else
    echo "✅ Server already running"
    SERVER_PID=""
fi

echo ""
echo "📤 Sending streaming request..."
echo ""

# Test streaming endpoint
curl -X POST http://localhost:8080/v1/chat/completions \
    -H "Content-Type: application/json" \
    -H "Authorization: Bearer test-key" \
    -N \
    -d '{
        "model": "claude-sonnet-4@20250514",
        "messages": [
            {
                "role": "user", 
                "content": "Count from 1 to 5, one number at a time. Be very brief."
            }
        ],
        "max_tokens": 50,
        "temperature": 0.1,
        "stream": true
    }' | tee streaming_response.log

echo ""
echo ""
echo "📋 Response saved to streaming_response.log"

# Analyze the response
echo ""
echo "🔍 Analyzing response..."
echo "======================="

if [ -f streaming_response.log ]; then
    echo "📄 Response content:"
    cat streaming_response.log
    echo ""
    
    # Count data chunks
    DATA_CHUNKS=$(grep -c "data:" streaming_response.log)
    DONE_CHUNKS=$(grep -c "data: \[DONE\]" streaming_response.log)
    CONTENT_CHUNKS=$(grep -o '"content":"[^"]*"' streaming_response.log | wc -l)
    
    echo "📊 Analysis:"
    echo "  Total data chunks: $DATA_CHUNKS"
    echo "  DONE chunks: $DONE_CHUNKS"
    echo "  Content chunks: $CONTENT_CHUNKS"
    
    if [ "$CONTENT_CHUNKS" -gt 0 ]; then
        echo ""
        echo "🎉 SUCCESS: Found content chunks!"
        echo "📝 Content found:"
        grep -o '"content":"[^"]*"' streaming_response.log
    else
        echo ""
        echo "⚠️  WARNING: No content chunks found!"
        echo "🔍 This confirms the issue - only DONE chunks are being sent."
    fi
else
    echo "❌ No response file found"
fi

# Clean up
if [ ! -z "$SERVER_PID" ]; then
    echo ""
    echo "🛑 Stopping server..."
    kill $SERVER_PID
fi

echo ""
echo "✅ Test complete!"