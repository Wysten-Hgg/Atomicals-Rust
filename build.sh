#!/bin/bash

# 启用错误检查
set -e

echo "Starting build process..."

# 检查 wasm-pack 是否安装
if ! command -v wasm-pack &> /dev/null; then
    echo "wasm-pack not found. Please install it with: cargo install wasm-pack"
    exit 1
fi

echo "Building WebAssembly with wasm-pack..."
# 添加 --verbose 参数来获取更多输出信息
wasm-pack build --target web --verbose

# echo "Creating dist directory..."
# mkdir -p dist

echo "delete files"
rm -rf dist/atomicals*
rm -rf dist/mining_worker.js
rm -rf dist/worker_entry.js

echo "Copying files to dist directory..."
cp -r pkg/* dist/
cp src/worker_entry.js dist/
# cp examples/index.html dist/

echo "Build completed successfully!"