#!/bin/bash
set -e

# Colors for output
GREEN='\033[0;32m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

echo -e "${BLUE}=== XML/HTML to SQLite WASM Builder ===${NC}"

# 1. Build WASM Module
echo -e "\n${GREEN}Building WASM module...${NC}"
# Determine if we should build for release or debug (default debug for faster dev builds)
MODE=${1:-debug}

# Fix for getrandom 0.3 on wasm32-unknown-unknown
export RUSTFLAGS='--cfg getrandom_backend="wasm_js"'

# Use Homebrew LLVM for WASM compilation support
export CC="/opt/homebrew/opt/llvm/bin/clang"
export AR="/opt/homebrew/opt/llvm/bin/llvm-ar"

if [ "$MODE" == "release" ]; then
    wasm-pack build --target web --out-dir web/public/wasm --release
else
    wasm-pack build --target web --out-dir web/public/wasm --dev
fi

# 2. Check and Install Frontend Dependencies
echo -e "\n${GREEN}Checking frontend dependencies...${NC}"
cd web
if [ ! -d "node_modules" ]; then
    echo "Installing dependencies..."
    npm install
else
    echo "node_modules exists, skipping install (run 'npm install' manually if needed)"
fi

# 3. Start Frontend Dev Server
echo -e "\n${GREEN}Starting frontend development server...${NC}"
npm run dev
