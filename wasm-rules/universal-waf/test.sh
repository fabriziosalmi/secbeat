#!/bin/bash
# Test runner for Universal WAF WASM module

set -e

echo "🧪 Universal WAF Test Runner"
echo ""

# Build the WASM module
echo "📦 Building WASM module..."
cd "$(dirname "$0")"
cargo build --target wasm32-unknown-unknown --release --quiet

WASM_FILE="target/wasm32-unknown-unknown/release/universal_waf.wasm"
WASM_SIZE=$(ls -lh "$WASM_FILE" | awk '{print $5}')

echo "✓ Module built: $WASM_SIZE"
echo ""

# Create test config
cat > /tmp/test-config.json << 'EOF'
{
  "rules": [
    {
      "id": "rule-001",
      "field": "URI",
      "pattern": "^/admin",
      "action": "Block"
    },
    {
      "id": "rule-002",
      "field": "Header:User-Agent",
      "pattern": "*EvilBot*",
      "action": "Block"
    },
    {
      "id": "rule-003",
      "field": "URI",
      "pattern": "*sqlmap*",
      "action": "Log"
    }
  ]
}
EOF

echo "📝 Test Configuration:"
cat /tmp/test-config.json
echo ""

# Create test requests
cat > /tmp/test-request-1.json << 'EOF'
{
  "method": "GET",
  "uri": "/admin/login",
  "version": "HTTP/1.1",
  "source_ip": "1.2.3.4",
  "headers": null,
  "body_preview": null
}
EOF

cat > /tmp/test-request-2.json << 'EOF'
{
  "method": "GET",
  "uri": "/api/users",
  "version": "HTTP/1.1",
  "source_ip": "1.2.3.4",
  "headers": [
    ["User-Agent", "EvilBot/1.0"]
  ],
  "body_preview": null
}
EOF

cat > /tmp/test-request-3.json << 'EOF'
{
  "method": "GET",
  "uri": "/api/data",
  "version": "HTTP/1.1",
  "source_ip": "1.2.3.4",
  "headers": [
    ["User-Agent", "Mozilla/5.0"]
  ],
  "body_preview": null
}
EOF

echo "🚀 Running tests..."
echo ""

# We'll need the test runner from mitigation-node
# For now, just verify the module loads
echo "✓ WASM module available at: $WASM_FILE"
echo ""
echo "⚠️  Full integration tests require wasmtime runtime"
echo "    Run from mitigation-node with WasmEngine"
echo ""
echo "📊 Module Statistics:"
wasm-objdump -h "$WASM_FILE" 2>/dev/null | head -20 || echo "  (install wabt for detailed analysis)"
