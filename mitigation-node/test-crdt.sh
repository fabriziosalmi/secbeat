#!/bin/bash
# Standalone test for CRDT implementation

echo "🧪 Testing CRDT Implementation"
echo ""

cd "$(dirname "$0")"

# Run unit tests
echo "Running unit tests..."
cargo test --package mitigation-node --lib distributed::crdt::tests -- --nocapture

echo ""
echo "✓ CRDT tests complete"
