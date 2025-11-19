#!/bin/bash

# Setup script for LLM Supabase Tool Calling Tests
# This script installs dependencies and prepares the test environment

set -e

echo "🚀 Setting up LLM Supabase Tool Calling Test Suite"
echo "=================================================="

# Check Node.js version
echo "📋 Checking Node.js version..."
if ! command -v node &> /dev/null; then
    echo "❌ Node.js is not installed. Please install Node.js 18 or higher."
    exit 1
fi

NODE_VERSION=$(node -v | cut -d'v' -f2 | cut -d'.' -f1)
if [ "$NODE_VERSION" -lt 18 ]; then
    echo "❌ Node.js version $NODE_VERSION is too old. Please install Node.js 18 or higher."
    exit 1
fi

echo "✅ Node.js $(node -v) detected"

# Check if npm is available
if ! command -v npm &> /dev/null; then
    echo "❌ npm is not available. Please install npm."
    exit 1
fi

echo "✅ npm $(npm -v) detected"

# Install dependencies
echo "📦 Installing dependencies..."
npm install

# Make test script executable
echo "🔧 Making test script executable..."
chmod +x test-tool-calling.ts

echo ""
echo "✅ Setup complete!"
echo ""
echo "📖 Usage:"
echo "  npm run test          - Run all tests"
echo "  npm run test:watch    - Run tests in watch mode"
echo "  ./test-tool-calling.ts - Run tests directly"
echo ""
echo "🔍 Before running tests, make sure your service is running on http://localhost:8080"
echo ""
echo "📚 See README.md for detailed documentation"