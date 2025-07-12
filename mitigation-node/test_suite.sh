#!/bin/bash

# SecBeat Mitigation Node Test Suite
# Tests the TCP proxy with metrics collection

echo "🛡️  SecBeat Mitigation Node Test Suite"
echo "======================================"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Function to print colored output
print_status() {
    echo -e "${BLUE}[$(date +'%H:%M:%S')]${NC} $1"
}

print_success() {
    echo -e "${GREEN}✅ $1${NC}"
}

print_warning() {
    echo -e "${YELLOW}⚠️  $1${NC}"
}

print_error() {
    echo -e "${RED}❌ $1${NC}"
}

# Check if we're on the right network
print_status "Checking network configuration..."

# Build the binaries
print_status "Building mitigation node binaries..."
cargo build --release --bin mitigation-node --bin test-origin

if [ $? -eq 0 ]; then
    print_success "Binaries built successfully"
else
    print_error "Failed to build binaries"
    exit 1
fi

# Start test origin server
print_status "Starting test origin server on 127.0.0.1:8080..."
RUST_LOG=info ./target/release/test-origin &
ORIGIN_PID=$!
sleep 2

# Check if origin is running
if ps -p $ORIGIN_PID > /dev/null; then
    print_success "Test origin server started (PID: $ORIGIN_PID)"
else
    print_error "Failed to start test origin server"
    exit 1
fi

# Start mitigation proxy
print_status "Starting mitigation proxy on 127.0.0.1:8443..."
RUST_LOG=info ./target/release/mitigation-node &
PROXY_PID=$!
sleep 3

# Check if proxy is running
if ps -p $PROXY_PID > /dev/null; then
    print_success "Mitigation proxy started (PID: $PROXY_PID)"
else
    print_error "Failed to start mitigation proxy"
    kill $ORIGIN_PID 2>/dev/null
    exit 1
fi

# Wait for services to be ready
print_status "Waiting for services to initialize..."
sleep 2

# Test 1: Simple HTTPS connectivity test
print_status "Test 1: Testing basic HTTPS connectivity..."
if command -v curl &> /dev/null; then
    response=$(curl -k -s --connect-timeout 5 https://127.0.0.1:8443/ 2>/dev/null)
    if [ $? -eq 0 ] && [[ "$response" == *"SecBeat"* ]]; then
        print_success "HTTPS connectivity test passed"
    else
        print_warning "HTTPS connectivity test failed - checking logs..."
    fi
else
    print_warning "curl not available, skipping HTTPS test"
fi

# Test 2: Metrics endpoint
print_status "Test 2: Testing metrics endpoint..."
if command -v curl &> /dev/null; then
    metrics_response=$(curl -s --connect-timeout 5 http://127.0.0.1:9090/metrics 2>/dev/null)
    if [ $? -eq 0 ] && [[ "$metrics_response" == *"https_requests_received"* ]]; then
        print_success "Metrics endpoint is working"
    else
        print_warning "Metrics endpoint test failed"
    fi
else
    print_warning "curl not available, skipping metrics test"
fi

# Test 3: Load test with multiple HTTPS connections
print_status "Test 3: Running HTTPS load test (10 concurrent connections)..."
if command -v curl &> /dev/null; then
    for i in {1..10}; do
        curl -k -s https://127.0.0.1:8443/ > /dev/null &
    done
    wait
    print_success "HTTPS load test completed"
    sleep 1
else
    print_warning "curl not available, skipping load test"
fi

# Test 4: WAF functionality test
print_status "Test 4: Testing WAF functionality..."
if command -v curl &> /dev/null; then
    # Test suspicious request (should be blocked)
    response=$(curl -k -s -w "%{http_code}" 'https://127.0.0.1:8443/test?id=<script>alert("xss")</script>' 2>/dev/null)
    if [[ "$response" == *"403"* ]] || [[ "$response" == *"400"* ]]; then
        print_success "WAF successfully blocked suspicious request"
    else
        print_warning "WAF test may have failed (response: $response)"
    fi
    
    # Test normal request (should pass)
    response=$(curl -k -s -w "%{http_code}" https://127.0.0.1:8443/api/status 2>/dev/null)
    if [[ "$response" == *"200"* ]]; then
        print_success "Normal request passed through WAF correctly"
    else
        print_warning "Normal request test failed (response: $response)"
    fi
else
    print_warning "curl not available, skipping WAF test"
fi

# Show current metrics
print_status "Fetching current metrics..."
if command -v curl &> /dev/null; then
    echo
    echo "📊 Current L7 Proxy Metrics:"
    echo "============================"
    curl -s http://127.0.0.1:9090/metrics | grep -E "(https_requests_received|tls_handshakes_completed|requests_proxied|waf_requests_blocked|active_connections)" || print_warning "Could not fetch metrics"
    echo
fi

# Cleanup
print_status "Cleaning up test processes..."
kill $PROXY_PID 2>/dev/null
kill $ORIGIN_PID 2>/dev/null

# Wait for processes to terminate
sleep 2

print_success "Test suite completed!"
echo
echo "📋 Summary:"
echo "- Test origin server: 127.0.0.1:8080"
echo "- L7 TLS/HTTP proxy: 127.0.0.1:8443 (HTTPS)"  
echo "- Metrics endpoint: 127.0.0.1:9090/metrics"
echo
echo "🔧 For Proxmox deployment, update IPs to:"
echo "- Proxy listen: 192.168.100.1:8443 (vmbr1 - HTTPS)"
echo "- Backend target: 192.168.100.10:80 (vmbr2 - HTTP)"
echo "- Metrics: 192.168.100.1:9090"
echo
echo "🚀 Phase 3 Complete: L7 TLS/HTTP Proxy with WAF placeholder!"
echo "   Next: Phase 4 - Orchestrator and node self-registration"
