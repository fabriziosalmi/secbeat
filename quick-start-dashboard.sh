#!/bin/bash
# Quick Start: Launch SecBeat Orchestrator + Dashboard
# Run this from the secbeat root directory

set -e

echo "🚀 SecBeat Quick Start - Chapter 4.2 Dashboard"
echo "=============================================="
echo ""

# Colors
GREEN='\033[0;32m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Check if we're in the right directory
if [ ! -f "Cargo.toml" ] || [ ! -d "dashboard" ]; then
    echo "❌ Error: Must run from secbeat root directory"
    exit 1
fi

# Check if dashboard dependencies are installed
if [ ! -d "dashboard/node_modules" ]; then
    echo "📦 Installing dashboard dependencies..."
    cd dashboard
    npm install
    cd ..
    echo ""
fi

# Function to cleanup background processes
cleanup() {
    echo ""
    echo "🛑 Shutting down..."
    kill $ORCHESTRATOR_PID 2>/dev/null || true
    kill $DASHBOARD_PID 2>/dev/null || true
    exit 0
}

trap cleanup INT TERM

echo "${BLUE}Step 1:${NC} Starting Orchestrator (Rust)..."
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
cargo run --quiet --bin orchestrator-node &
ORCHESTRATOR_PID=$!
echo "✅ Orchestrator started (PID: $ORCHESTRATOR_PID)"
echo "   API: http://localhost:3030"
echo ""

# Wait for orchestrator to be ready
echo "⏳ Waiting for orchestrator to be ready..."
for i in {1..30}; do
    if curl -s http://localhost:3030/api/v1/health > /dev/null 2>&1; then
        echo "${GREEN}✅ Orchestrator is ready!${NC}"
        break
    fi
    sleep 1
    if [ $i -eq 30 ]; then
        echo "❌ Orchestrator failed to start in 30 seconds"
        cleanup
    fi
done
echo ""

echo "${BLUE}Step 2:${NC} Starting Dashboard (React)..."
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
cd dashboard
npm run dev &
DASHBOARD_PID=$!
cd ..
echo "✅ Dashboard started (PID: $DASHBOARD_PID)"
echo "   URL: http://localhost:5173"
echo ""

echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "${GREEN}✨ SecBeat Dashboard is running!${NC}"
echo ""
echo "📊 Dashboard:    http://localhost:5173"
echo "🔧 Orchestrator: http://localhost:3030"
echo ""
echo "📖 Available pages:"
echo "   • Overview:  http://localhost:5173"
echo "   • Nodes:     http://localhost:5173/nodes"
echo "   • Attacks:   http://localhost:5173/attacks"
echo ""
echo "🧪 Test API endpoints:"
echo "   curl http://localhost:3030/api/v1/dashboard/summary"
echo "   curl http://localhost:3030/api/v1/dashboard/attacks"
echo "   curl http://localhost:3030/api/v1/nodes"
echo ""
echo "Press Ctrl+C to stop all services"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"

# Keep script running and wait for Ctrl+C
wait
