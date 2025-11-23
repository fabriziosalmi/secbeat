#!/bin/bash
# Build script for WASM rules

set -e

echo "Building bad-bot WASM rule..."

cd "$(dirname "$0")"

# Build for wasm32-unknown-unknown target
cargo build --target wasm32-unknown-unknown --release

# Copy to output directory
mkdir -p ../../target/wasm
cp target/wasm32-unknown-unknown/release/bad_bot_rule.wasm ../../target/wasm/bad-bot.wasm

echo "✓ Built: target/wasm/bad-bot.wasm"
ls -lh ../../target/wasm/bad-bot.wasm
