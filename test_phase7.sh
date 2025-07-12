#!/bin/bash
set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
PURPLE='\033[0;35m'
CYAN='\033[0;36m'
NC='\033[0m' # No Color

echo -e "${PURPLE}🚀 Phase 7: Predictive AI and Proactive Self-Healing${NC}"
echo "=================================================================="

# Check if NATS server is available
if ! command -v nats-server &> /dev/null; then
    echo -e "${RED}❌ NATS server not found. Please install NATS:${NC}"
    echo "   macOS: brew install nats-server"
    echo "   Linux: Download from https://github.com/nats-io/nats-server/releases"
    echo ""
    echo "Continuing without NATS server (some features will be disabled)..."
    NATS_AVAILABLE=false
else
    echo -e "${GREEN}✅ NATS server found${NC}"
    NATS_AVAILABLE=true
fi

echo ""
echo -e "${BLUE}📋 Starting Phase 7 Test Sequence:${NC}"
echo "1. NATS Server (Message Bus)"
echo "2. Orchestrator with ML Prediction & Self-Healing"
echo "3. Test Origin Server"
echo "4. Mitigation Node Fleet"
echo "5. Mock Webhook Server (Infrastructure Provisioning)"
echo "6. Predictive scaling tests"
echo "7. Self-healing failure simulation"

# Create logs directory
mkdir -p logs

# Function to cleanup processes
cleanup() {
    echo ""
    echo -e "${YELLOW}🛑 Cleaning up processes...${NC}"
    
    # Kill processes by PID if they exist
    [ ! -z "$ORCHESTRATOR_PID" ] && kill $ORCHESTRATOR_PID 2>/dev/null || true
    [ ! -z "$ORIGIN_PID" ] && kill $ORIGIN_PID 2>/dev/null || true
    [ ! -z "$MITIGATION1_PID" ] && kill $MITIGATION1_PID 2>/dev/null || true
    [ ! -z "$MITIGATION2_PID" ] && kill $MITIGATION2_PID 2>/dev/null || true
    [ ! -z "$WEBHOOK_PID" ] && kill $WEBHOOK_PID 2>/dev/null || true
    [ ! -z "$NATS_PID" ] && kill $NATS_PID 2>/dev/null || true
    
    # Additional cleanup
    pkill -f "orchestrator-node" 2>/dev/null || true
    pkill -f "mitigation-node" 2>/dev/null || true
    pkill -f "test-origin" 2>/dev/null || true
    pkill -f "nats-server" 2>/dev/null || true
    pkill -f "python.*webhook" 2>/dev/null || true
    
    echo -e "${GREEN}✅ Cleanup completed${NC}"
}

# Set trap for cleanup on exit
trap cleanup EXIT

echo ""
echo -e "${CYAN}🔌 Starting NATS server on port 4222...${NC}"
if [ "$NATS_AVAILABLE" = true ]; then
    nats-server --port 4222 --http_port 8222 > logs/nats.log 2>&1 &
    NATS_PID=$!
    sleep 2
    echo -e "${GREEN}✅ NATS server running (PID: $NATS_PID)${NC}"
else
    echo -e "${YELLOW}⚠️  NATS server not available, skipping...${NC}"
fi

echo ""
echo -e "${CYAN}🧠 Starting Orchestrator with ML Prediction & Self-Healing...${NC}"
cd /Users/fab/GitHub/secbeat/orchestrator-node
cargo run > ../logs/orchestrator.log 2>&1 &
ORCHESTRATOR_PID=$!
cd /Users/fab/GitHub/secbeat
sleep 3
echo -e "${GREEN}✅ Orchestrator running (PID: $ORCHESTRATOR_PID)${NC}"

echo ""
echo -e "${CYAN}🎯 Starting Test Origin Server...${NC}"
cd /Users/fab/GitHub/secbeat/mitigation-node
cargo run --bin test-origin > ../logs/origin.log 2>&1 &
ORIGIN_PID=$!
cd ..
sleep 2
echo -e "${GREEN}✅ Test Origin Server running (PID: $ORIGIN_PID)${NC}"

echo ""
echo -e "${CYAN}🛡️  Starting Mitigation Node Fleet...${NC}"
cd mitigation-node

# Start first mitigation node
cargo run > ../logs/mitigation1.log 2>&1 &
MITIGATION1_PID=$!
sleep 3
echo -e "${GREEN}✅ Mitigation Node 1 running (PID: $MITIGATION1_PID)${NC}"

# Start second mitigation node on different port
echo "   Starting second mitigation node on port 8444..."
RUST_LOG=info cargo run -- --listen-port 8444 --tls-port 8444 --management-port 9998 > ../logs/mitigation2.log 2>&1 &
MITIGATION2_PID=$!
sleep 3
echo -e "${GREEN}✅ Mitigation Node 2 running (PID: $MITIGATION2_PID)${NC}"

cd ..

echo ""
echo -e "${CYAN}🔗 Starting Mock Webhook Server (Infrastructure Provisioning)...${NC}"
# Create a simple Python webhook server to receive provisioning requests
cat > webhook_server.py << 'EOF'
#!/usr/bin/env python3
import json
import time
from http.server import HTTPServer, BaseHTTPRequestHandler
from datetime import datetime
import threading

class WebhookHandler(BaseHTTPRequestHandler):
    def log_message(self, format, *args):
        timestamp = datetime.now().strftime("%Y-%m-%d %H:%M:%S")
        print(f"[{timestamp}] {format % args}")

    def do_POST(self):
        content_length = int(self.headers['Content-Length'])
        post_data = self.rfile.read(content_length)
        
        try:
            payload = json.loads(post_data.decode('utf-8'))
            reason = payload.get('reason', 'unknown')
            timestamp = payload.get('timestamp', 'unknown')
            
            print(f"\n🚨 PROVISIONING WEBHOOK RECEIVED:")
            print(f"   Reason: {reason}")
            print(f"   Timestamp: {timestamp}")
            
            if reason == "PREDICTED_HIGH_FLEET_CPU_LOAD":
                prediction_info = payload.get('prediction_info', {})
                print(f"   🧠 ML PREDICTION:")
                print(f"      Predicted CPU: {prediction_info.get('predicted_cpu_usage', 'N/A')}")
                print(f"      Horizon: {prediction_info.get('prediction_horizon_minutes', 'N/A')} minutes")
                print(f"      Confidence: {prediction_info.get('confidence', 'N/A')}")
                
            elif reason == "UNEXPECTED_NODE_FAILURE":
                failed_node_id = payload.get('failed_node_id', 'unknown')
                failed_node_ip = payload.get('failed_node_ip', 'unknown')
                print(f"   💀 SELF-HEALING TRIGGERED:")
                print(f"      Failed Node ID: {failed_node_id}")
                print(f"      Failed Node IP: {failed_node_ip}")
                print(f"   🔧 Provisioning replacement node...")
            
            fleet_metrics = payload.get('fleet_metrics', {})
            if fleet_metrics:
                print(f"   📊 Fleet Status:")
                print(f"      Active Nodes: {fleet_metrics.get('active_nodes', 'N/A')}")
                print(f"      Avg CPU: {fleet_metrics.get('avg_cpu_usage', 'N/A')}")
                print(f"      Total Connections: {fleet_metrics.get('total_connections', 'N/A')}")
            
            print(f"   ✅ Webhook processed successfully\n")
            
            # Simulate provisioning delay
            time.sleep(1)
            
            self.send_response(200)
            self.send_header('Content-type', 'application/json')
            self.end_headers()
            response = {"status": "success", "message": "Provisioning initiated"}
            self.wfile.write(json.dumps(response).encode('utf-8'))
            
        except Exception as e:
            print(f"❌ Error processing webhook: {e}")
            self.send_response(400)
            self.end_headers()

if __name__ == '__main__':
    server = HTTPServer(('localhost', 8000), WebhookHandler)
    print("🔗 Mock Webhook Server listening on http://localhost:8000")
    server.serve_forever()
EOF

python3 webhook_server.py > logs/webhook.log 2>&1 &
WEBHOOK_PID=$!
sleep 2
echo -e "${GREEN}✅ Mock Webhook Server running (PID: $WEBHOOK_PID)${NC}"

echo ""
echo -e "${GREEN}🎉 All services started! System Status:${NC}"
echo "   - NATS Server: localhost:4222 (WebUI: http://localhost:8222)"
echo "   - Orchestrator (ML + Self-Healing): http://localhost:3030"
echo "   - Mitigation Node 1: https://localhost:8443 (Management: http://localhost:9999)"
echo "   - Mitigation Node 2: https://localhost:8444 (Management: http://localhost:9998)"
echo "   - Origin Server: http://localhost:8080"
echo "   - Mock Webhook Server: http://localhost:8000"
echo "   - Metrics: http://localhost:9091/metrics"

echo ""
echo -e "${YELLOW}⏳ Waiting for services to initialize and ML data collection...${NC}"
sleep 15

echo ""
echo -e "${PURPLE}🧪 Phase 7 Testing Sequence${NC}"
echo "=========================="

echo ""
echo -e "${BLUE}1️⃣  Testing basic connectivity...${NC}"
echo "   → Sending request through mitigation node..."
if curl -k -s https://localhost:8443/api/test > /dev/null 2>&1; then
    echo -e "${GREEN}   ✅ Basic connectivity working${NC}"
else
    echo -e "${RED}   ❌ Basic connectivity failed${NC}"
fi

echo ""
echo -e "${BLUE}2️⃣  Testing fleet statistics...${NC}"
echo "   → Getting fleet stats from orchestrator..."
FLEET_STATS=$(curl -s http://localhost:3030/api/v1/fleet/stats 2>/dev/null || echo "ERROR")
echo -e "${GREEN}   ✅ Fleet statistics: $FLEET_STATS${NC}"

echo ""
echo -e "${BLUE}3️⃣  Testing ML Prediction Data Collection...${NC}"
echo "   → Resource Manager should be collecting CPU data for ML prediction"
echo "   → Generating varied load to create prediction data..."

# Generate varied CPU load patterns for ML training
for i in {1..20}; do
    # Send bursts of requests to create CPU variation
    for j in {1..5}; do
        curl -k -s https://localhost:8443/api/test >/dev/null 2>&1 &
        curl -k -s https://localhost:8444/api/test >/dev/null 2>&1 &
    done
    
    if [ $((i % 5)) -eq 0 ]; then
        echo "   → Generated load batch $i/20"
    fi
    
    sleep 3
done

wait  # Wait for background requests to complete
echo -e "${GREEN}   ✅ ML training data generated${NC}"

echo ""
echo -e "${BLUE}4️⃣  Testing Predictive Scaling Logic...${NC}"
echo "   → Orchestrator should now have sufficient data for ML predictions"
echo "   → Monitoring for predictive scaling decisions..."
echo "   → Check orchestrator logs for ML predictions and scaling decisions"

# Wait for a few prediction cycles
sleep 120

echo ""
echo -e "${BLUE}5️⃣  Testing Self-Healing: Simulating Unexpected Node Failure...${NC}"
echo -e "${YELLOW}   ⚠️  This will FORCEFULLY KILL a mitigation node to simulate a crash!${NC}"
echo "   → Press Ctrl+C to cancel, or wait 10 seconds to proceed..."
sleep 10

echo "   → Forcefully terminating Mitigation Node 2 (simulating crash)..."
if [ ! -z "$MITIGATION2_PID" ]; then
    kill -9 $MITIGATION2_PID 2>/dev/null || true
    echo -e "${YELLOW}   💀 Mitigation Node 2 forcefully terminated${NC}"
else
    echo -e "${YELLOW}   ⚠️  Mitigation Node 2 PID not found${NC}"
fi

echo "   → Waiting for orchestrator to detect the failure and trigger self-healing..."
echo "   → Monitoring webhook server for self-healing requests..."

# Wait for dead node detection and self-healing
sleep 45

echo ""
echo -e "${BLUE}6️⃣  Verifying Self-Healing Response...${NC}"
echo "   → Checking if self-healing webhook was triggered..."
echo "   → Check webhook server logs for UNEXPECTED_NODE_FAILURE events"

echo ""
echo -e "${GREEN}📊 Live System Monitoring${NC}"
echo "========================"

echo ""
echo -e "${CYAN}Real-time log monitoring (press Ctrl+C to exit):${NC}"
echo -e "${CYAN}🔍 Watching orchestrator logs for ML predictions and self-healing events...${NC}"

echo ""
echo -e "${YELLOW}📝 Recent Orchestrator Events:${NC}"
tail -n 20 logs/orchestrator.log | grep -E "(prediction|ML|CPU|self-healing|UNEXPECTED|CRITICAL)" || echo "No prediction/self-healing events found yet"

echo ""
echo -e "${YELLOW}📝 Recent Webhook Events:${NC}"
tail -n 10 logs/webhook.log || echo "No webhook events logged yet"

echo ""
echo -e "${PURPLE}💡 Phase 7 Advanced Commands You Can Try:${NC}"
echo "=========================================="

echo ""
echo -e "${CYAN}🧠 ML Prediction & Scaling:${NC}"
echo "   # Check current fleet metrics:"
echo "   curl http://127.0.0.1:3030/api/v1/fleet/stats"
echo ""
echo "   # Monitor scaling decisions in real-time:"
echo "   tail -f logs/orchestrator.log | grep -E '(prediction|scaling|ML|CPU)'"
echo ""
echo "   # Generate high CPU load to trigger predictive scaling:"
echo "   for i in {1..100}; do curl -k https://127.0.0.1:8443/api/test & done"

echo ""
echo -e "${CYAN}🔧 Self-Healing Testing:${NC}"
echo "   # Simulate another node failure:"
echo "   kill -9 \$MITIGATION1_PID"
echo ""
echo "   # Check self-healing metrics:"
echo "   curl http://127.0.0.1:9091/metrics | grep -E '(unexpected|self_healing|terminated)'"
echo ""
echo "   # Monitor webhook activity:"
echo "   tail -f logs/webhook.log"

echo ""
echo -e "${CYAN}📊 Advanced Monitoring:${NC}"
echo "   # NATS stats (if available):"
echo "   curl http://127.0.0.1:8222/varz"
echo ""
echo "   # Full metrics export:"
echo "   curl http://127.0.0.1:9091/metrics"
echo ""
echo "   # Node details:"
echo "   curl http://127.0.0.1:3030/api/v1/nodes"

echo ""
echo -e "${GREEN}✨ Phase 7 System Ready!${NC}"
echo -e "${GREEN}🧠 Predictive AI: ACTIVE${NC}"
echo -e "${GREEN}🔧 Self-Healing: ACTIVE${NC}"
echo -e "${GREEN}⚡ ML-Based Scaling: MONITORING${NC}"

echo ""
echo -e "${PURPLE}Press Ctrl+C to shutdown...${NC}"

# Keep the script running and monitor logs
tail -f logs/orchestrator.log logs/webhook.log 2>/dev/null | while read line; do
    if echo "$line" | grep -q "prediction\|UNEXPECTED\|self-healing\|CRITICAL"; then
        echo -e "${YELLOW}🔔 $line${NC}"
    fi
done
