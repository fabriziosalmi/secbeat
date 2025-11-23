#!/usr/bin/env bash
# Setup Test Environment for SecBeat Integration Tests
#
# This script:
# 1. Builds all Docker images required for testing
# 2. Starts the test cluster with docker-compose
# 3. Waits for all services to become healthy
# 4. Prints service URLs for manual testing
#
# Usage:
#   ./tests/setup_env.sh         # Start test environment
#   ./tests/setup_env.sh down    # Stop and cleanup

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
COMPOSE_FILE="$PROJECT_ROOT/docker-compose.test.yml"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

log_info() {
    echo -e "${BLUE}[INFO]${NC} $1"
}

log_success() {
    echo -e "${GREEN}[SUCCESS]${NC} $1"
}

log_warn() {
    echo -e "${YELLOW}[WARN]${NC} $1"
}

log_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

# Cleanup function
cleanup() {
    log_info "Stopping test environment..."
    docker-compose -f "$COMPOSE_FILE" down -v --remove-orphans
    log_success "Test environment stopped"
}

# Build Docker images
build_images() {
    log_info "Building Docker images for test environment..."
    
    # Build orchestrator
    log_info "Building orchestrator-node image..."
    docker build -f "$PROJECT_ROOT/orchestrator.Dockerfile" -t secbeat-orchestrator:test "$PROJECT_ROOT"
    
    # Build mitigation-node
    log_info "Building mitigation-node image..."
    docker build -f "$PROJECT_ROOT/Dockerfile" -t secbeat-mitigation:test "$PROJECT_ROOT"
    
    # Build mock-origin
    log_info "Building mock-origin image..."
    docker build -f "$PROJECT_ROOT/tests/mock-origin/Dockerfile" -t secbeat-mock-origin:test "$PROJECT_ROOT/tests/mock-origin"
    
    log_success "All images built successfully"
}

# Wait for service to be healthy
wait_for_health() {
    local service=$1
    local max_attempts=30
    local attempt=1
    
    log_info "Waiting for $service to be healthy..."
    
    while [ $attempt -le $max_attempts ]; do
        if docker-compose -f "$COMPOSE_FILE" ps | grep "$service" | grep -q "healthy"; then
            log_success "$service is healthy"
            return 0
        fi
        
        if [ $attempt -eq $max_attempts ]; then
            log_error "$service failed to become healthy after $max_attempts attempts"
            docker-compose -f "$COMPOSE_FILE" logs "$service" | tail -50
            return 1
        fi
        
        echo -n "."
        sleep 2
        ((attempt++))
    done
}

# Start test environment
start_env() {
    log_info "Starting test environment..."
    
    # Start services
    docker-compose -f "$COMPOSE_FILE" up -d
    
    # Wait for each service to be healthy
    wait_for_health "secbeat-test-nats" || exit 1
    wait_for_health "secbeat-test-orchestrator" || exit 1
    wait_for_health "secbeat-test-mitigation" || exit 1
    wait_for_health "secbeat-test-origin" || exit 1
    
    log_success "All services are healthy!"
    echo ""
    log_info "Test environment is ready:"
    echo "  - NATS:         http://localhost:18222"
    echo "  - Orchestrator: http://localhost:13030"
    echo "  - Mitigation:   http://localhost:18080"
    echo "  - Mock Origin:  http://localhost:18888"
    echo ""
    log_info "Run tests with: cargo test --test e2e_scenarios"
}

# Show logs
show_logs() {
    log_info "Showing test environment logs..."
    docker-compose -f "$COMPOSE_FILE" logs -f
}

# Main
case "${1:-up}" in
    up|start)
        build_images
        start_env
        ;;
    down|stop|cleanup)
        cleanup
        ;;
    logs)
        show_logs
        ;;
    rebuild)
        cleanup
        build_images
        start_env
        ;;
    *)
        echo "Usage: $0 {up|down|logs|rebuild}"
        echo "  up/start    - Build and start test environment"
        echo "  down/stop   - Stop and cleanup"
        echo "  logs        - Show logs"
        echo "  rebuild     - Full rebuild and restart"
        exit 1
        ;;
esac
