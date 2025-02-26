#!/bin/bash

# 鍚敤閿欒妫€鏌?
set -e

echo "Starting build process..."

# 妫€鏌?wasm-pack 鏄惁瀹夎
if ! command -v wasm-pack &> /dev/null; then
    echo "wasm-pack not found. Please install it with: cargo install wasm-pack"
    exit 1
fi

echo "Building WebAssembly with wasm-pack..."
# 娣诲姞 --verbose 鍙傛暟鏉ヨ幏鍙栨洿澶氳緭鍑轰俊鎭?
wasm-pack build --target web --verbose

# echo "Creating dist directory..."
# mkdir -p dist

echo "delete files"
rm -rf dist/atomicals*
rm -rf dist/mining_worker.js
rm -rf dist/worker_entry.js
rm -rf dist/index.html
echo "Copying files to dist directory..."
cp -r pkg/* dist/
cp src/worker_entry.js dist/
cp examples/index.html dist/

echo "Build completed successfully!"
