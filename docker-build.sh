#!/bin/bash

# SecBeat Docker Build Script
set -e

echo "🔨 Building SecBeat Docker Images..."

# Build mitigation node image
echo "📦 Building mitigation-node image..."
docker build -t secbeat/mitigation-node:latest .

# Tag for version
VERSION=$(grep '^version' mitigation-node/Cargo.toml | head -1 | cut -d'"' -f2)
docker tag secbeat/mitigation-node:latest secbeat/mitigation-node:${VERSION}

echo "✅ Built secbeat/mitigation-node:latest and secbeat/mitigation-node:${VERSION}"

# Build orchestrator node (if Dockerfile exists)
if [ -f "orchestrator.Dockerfile" ]; then
    echo "📦 Building orchestrator-node image..."
    docker build -f orchestrator.Dockerfile -t secbeat/orchestrator-node:latest .
    docker tag secbeat/orchestrator-node:latest secbeat/orchestrator-node:${VERSION}
    echo "✅ Built secbeat/orchestrator-node:latest and secbeat/orchestrator-node:${VERSION}"
fi

echo "🎉 Docker build complete!"
echo ""
echo "🚀 To start the development stack:"
echo "   docker-compose up -d"
echo ""
echo "🏭 To start the production stack:"
echo "   docker-compose -f docker-compose.prod.yml up -d"
echo ""
echo "📊 Services:"
echo "   - SecBeat Proxy:  https://localhost:8443"
echo "   - Metrics:        http://localhost:9191/metrics"
echo "   - Management:     http://localhost:9999"
echo "   - Grafana:        http://localhost:3000 (admin/secbeat123)"
echo "   - Prometheus:     http://localhost:9091"
