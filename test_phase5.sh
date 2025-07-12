#!/bin/bash

# Phase 5 Test Script: Centralized Intelligence & Real-time Control
# This script demonstrates the complete SIEM-like functionality

echo "🚀 Phase 5: SecBeat Centralized Intelligence & Real-time Control"
echo "=================================================================="

# Check if NATS server is available
if ! command -v nats-server &> /dev/null; then
    echo "❌ NATS server not found. Please install NATS:"
    echo "   macOS: brew install nats-server"
    echo "   Linux: Download from https://github.com/nats-io/nats-server/releases"
    echo ""
    echo "Continuing without NATS server (some features will be disabled)..."
    sleep 2
else
    echo "✅ NATS server found"
fi

# Function to cleanup background processes
cleanup() {
    echo ""
    echo "🛑 Cleaning up processes..."
    # Kill all background jobs
    jobs -p | xargs -r kill 2>/dev/null
    exit 0
}

# Set trap to cleanup on script exit
trap cleanup EXIT INT TERM

echo ""
echo "📋 Starting Phase 5 Test Sequence:"
echo "1. NATS Server (Message Bus)"
echo "2. Orchestrator (SIEM + Control Plane)"
echo "3. Test Origin Server"
echo "4. Mitigation Node (Event Producer + Command Consumer)"
echo "5. Demonstration of threat intelligence and control"
echo ""

# Start NATS server if available
if command -v nats-server &> /dev/null; then
    echo "🔌 Starting NATS server on port 4222..."
    nats-server --port 4222 --http_port 8222 > /dev/null 2>&1 &
    NATS_PID=$!
    sleep 2
    echo "✅ NATS server running (PID: $NATS_PID)"
else
    echo "⚠️  NATS server not available - events will not be processed"
fi

echo ""
echo "🎭 Starting Orchestrator with Threat Intelligence..."
cd orchestrator-node
cargo run > ../logs/orchestrator.log 2>&1 &
ORCHESTRATOR_PID=$!
cd ..
sleep 3
echo "✅ Orchestrator running (PID: $ORCHESTRATOR_PID)"

echo ""
echo "🎯 Starting Test Origin Server..."
cd mitigation-node
cargo run --bin test-origin > ../logs/origin.log 2>&1 &
ORIGIN_PID=$!
sleep 2
echo "✅ Test Origin Server running (PID: $ORIGIN_PID)"

echo ""
echo "🛡️  Starting Mitigation Node with Event Publishing..."
cargo run --bin mitigation-node > ../logs/mitigation.log 2>&1 &
MITIGATION_PID=$!
cd ..
sleep 5
echo "✅ Mitigation Node running (PID: $MITIGATION_PID)"

echo ""
echo "🎉 All services started! System Status:"
echo "   - NATS Server: localhost:4222 (WebUI: http://localhost:8222)"
echo "   - Orchestrator: http://localhost:3030"
echo "   - Mitigation Node: https://localhost:8443"
echo "   - Origin Server: http://localhost:8080"
echo "   - Metrics: http://localhost:9090/metrics"
echo ""

# Wait for everything to be ready
echo "⏳ Waiting for services to initialize..."
sleep 10

echo ""
echo "🧪 Phase 5 Testing Sequence"
echo "=========================="

echo ""
echo "1️⃣  Testing basic connectivity..."
echo "   → Sending request through mitigation node..."
curl -k -s https://127.0.0.1:8443/ > /dev/null 2>&1
if [ $? -eq 0 ]; then
    echo "   ✅ Basic HTTPS proxy working"
else
    echo "   ❌ HTTPS proxy failed"
fi

echo ""
echo "2️⃣  Testing fleet statistics..."
echo "   → Getting fleet stats from orchestrator..."
FLEET_STATS=$(curl -s http://127.0.0.1:3030/api/v1/fleet/stats 2>/dev/null)
if [ $? -eq 0 ]; then
    echo "   ✅ Fleet statistics: $FLEET_STATS"
else
    echo "   ❌ Fleet statistics failed"
fi

echo ""
echo "3️⃣  Testing WAF event generation..."
echo "   → Sending malicious request (should trigger WAF)..."
curl -k -s "https://127.0.0.1:8443/test?q=<script>alert('xss')</script>" > /dev/null 2>&1
echo "   ✅ Malicious request sent (check logs for WAF events)"

echo ""
echo "4️⃣  Testing threat intelligence manual IP blocking..."
echo "   → Adding 192.168.1.100 to threat intel blocklist..."
BLOCK_RESULT=$(curl -s -X POST http://127.0.0.1:3030/api/v1/rules/block_ip \
  -H "Content-Type: application/json" \
  -d '{"ip": "192.168.1.100", "reason": "Manual test block", "ttl_seconds": 300}' 2>/dev/null)

if [ $? -eq 0 ]; then
    echo "   ✅ IP block command sent: $BLOCK_RESULT"
else
    echo "   ❌ IP block command failed"
fi

echo ""
echo "5️⃣  Testing blocked IPs endpoint..."
echo "   → Getting current blocked IPs list..."
BLOCKED_IPS=$(curl -s http://127.0.0.1:3030/api/v1/rules/blocked_ips 2>/dev/null)
if [ $? -eq 0 ]; then
    echo "   ✅ Blocked IPs retrieved"
    echo "   📊 Blocked IPs data: $BLOCKED_IPS"
else
    echo "   ❌ Blocked IPs endpoint failed"
fi

echo ""
echo "6️⃣  Generating some normal traffic for event stream testing..."
for i in {1..5}; do
    curl -k -s "https://127.0.0.1:8443/api/test$i" > /dev/null 2>&1
    sleep 1
done
echo "   ✅ Generated 5 normal requests"

echo ""
echo "7️⃣  Generating suspicious traffic to test pattern detection..."
curl -k -s "https://127.0.0.1:8443/admin/../../../etc/passwd" > /dev/null 2>&1
curl -k -s "https://127.0.0.1:8443/search?q=1' UNION SELECT * FROM users--" > /dev/null 2>&1
echo "   ✅ Generated suspicious requests (should trigger WAF rules)"

echo ""
echo "📊 Live System Monitoring"
echo "========================"
echo ""
echo "Real-time log monitoring (press Ctrl+C to exit):"
echo "🔍 Watching orchestrator logs for threat intelligence events..."
echo "🔍 Watching mitigation node logs for NATS events..."
echo ""

# Create logs directory if it doesn't exist
mkdir -p logs

# Monitor logs in real-time
echo "📝 Orchestrator Events:"
tail -f logs/orchestrator.log 2>/dev/null | grep -E "(threat|block|NATS|event)" --line-buffered &
LOG_TAIL_1=$!

echo ""
echo "📝 Mitigation Node Events:"
tail -f logs/mitigation.log 2>/dev/null | grep -E "(NATS|event|block|command)" --line-buffered &
LOG_TAIL_2=$!

echo ""
echo "💡 Phase 5 Test Commands You Can Try:"
echo "======================================"
echo ""
echo "📡 Fleet Management:"
echo "   curl http://127.0.0.1:3030/api/v1/fleet/stats"
echo "   curl http://127.0.0.1:3030/api/v1/nodes"
echo ""
echo "🚫 Threat Intelligence:"
echo "   # Block an IP manually:"
echo "   curl -X POST http://127.0.0.1:3030/api/v1/rules/block_ip \\"
echo "     -H 'Content-Type: application/json' \\"
echo "     -d '{\"ip\": \"10.0.0.100\", \"reason\": \"Test block\"}'"
echo ""
echo "   # View blocked IPs:"
echo "   curl http://127.0.0.1:3030/api/v1/rules/blocked_ips"
echo ""
echo "🔍 Generate Events:"
echo "   # Normal request:"
echo "   curl -k https://127.0.0.1:8443/api/test"
echo ""
echo "   # Malicious requests (triggers WAF):"
echo "   curl -k 'https://127.0.0.1:8443/test?q=<script>alert(1)</script>'"
echo "   curl -k 'https://127.0.0.1:8443/admin/../../../etc/passwd'"
echo "   curl -k \"https://127.0.0.1:8443/search?q=1' UNION SELECT password FROM users--\""
echo ""
echo "📊 Metrics:"
echo "   curl http://127.0.0.1:9090/metrics"
echo ""
echo "🌐 NATS Monitoring:"
echo "   # View NATS server stats:"
echo "   curl http://127.0.0.1:8222/varz"
echo ""

# Keep the script running
echo "✨ Phase 5 System Ready! Press Ctrl+C to shutdown..."
echo ""

# Wait for user interrupt
wait
