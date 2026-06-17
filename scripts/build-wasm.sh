#!/usr/bin/env bash
# Build the Wasm Playground and copy artifacts to docs
set -e

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
ROOT_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"

echo "Building Wasm module..."
cd "$ROOT_DIR"
wasm-pack build --target web --no-default-features --example yaoxiang_wasm -- --out-name yaoxiang

echo "Copying artifacts to docs..."
mkdir -p docs/src/.vitepress/public/wasm
cp pkg/yaoxiang.js pkg/yaoxiang_bg.wasm docs/src/.vitepress/public/wasm/

echo "Done! Wasm Playground is ready at docs/src/.vitepress/public/wasm/"
echo "  - yaoxiang.js ($(du -h docs/src/.vitepress/public/wasm/yaoxiang.js | cut -f1))"
echo "  - yaoxiang_bg.wasm ($(du -h docs/src/.vitepress/public/wasm/yaoxiang_bg.wasm | cut -f1))"
