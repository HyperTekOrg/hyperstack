#!/bin/bash
set -e

echo "Building local Hyperstack packages for auth testing..."

# Build core SDK
echo "Building hyperstack-typescript (core SDK)..."
cd /Users/adrian/code/defi/hypertek/hyperstack-oss/typescript/core
npm install
npm run build

# Build React SDK
echo "Building hyperstack-react..."
cd /Users/adrian/code/defi/hypertek/hyperstack-oss/typescript/react
npm install
npm run build

# Build stacks SDK
echo "Building hyperstack-stacks..."
cd /Users/adrian/code/defi/hypertek/hyperstack-oss/stacks/sdk/typescript
npm install
npm run build

echo "All packages built successfully!"
echo ""
echo "Next steps:"
echo "1. cd /Users/adrian/code/defi/hypertek/hyperstack-oss/examples/ore-react"
echo "2. rm -rf node_modules package-lock.json"
echo "3. npm install"
echo "4. npm run dev"
