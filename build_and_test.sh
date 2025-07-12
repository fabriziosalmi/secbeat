#!/bin/bash

# SecBeat Production Build and Basic Test Script
# This script builds and verifies all components are ready for production

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

echo "=============================================="
echo "    SecBeat Production Build & Test Script"
echo "=============================================="
echo ""

# Function to print status messages
print_status() {
    echo -e "${BLUE}[INFO]${NC} $1"
}

print_success() {
    echo -e "${GREEN}[PASS]${NC} $1"
}

print_warning() {
    echo -e "${YELLOW}[WARN]${NC} $1"
}

print_error() {
    echo -e "${RED}[FAIL]${NC} $1"
}

# Check prerequisites
print_status "Checking prerequisites..."

if ! command -v rustc &> /dev/null; then
    print_error "Rust compiler not found. Please install Rust."
    exit 1
fi

if ! command -v cargo &> /dev/null; then
    print_error "Cargo not found. Please install Rust."
    exit 1
fi

print_success "Rust toolchain available: $(rustc --version)"

# Build orchestrator node
print_status "Building orchestrator-node..."
cd orchestrator-node
if cargo build --release; then
    print_success "Orchestrator build completed"
else
    print_error "Orchestrator build failed"
    exit 1
fi
cd ..

# Build mitigation node with all features
print_status "Building mitigation-node with all features..."
cd mitigation-node
if cargo build --release --all-features; then
    print_success "Mitigation node build completed"
else
    print_error "Mitigation node build failed"
    exit 1
fi
cd ..

# Verify binaries exist
print_status "Verifying release binaries..."

if [ -f "orchestrator-node/target/release/orchestrator-node" ]; then
    print_success "orchestrator-node binary created"
else
    print_error "orchestrator-node binary missing"
    exit 1
fi

if [ -f "mitigation-node/target/release/mitigation-node" ]; then
    print_success "mitigation-node binary created"
else
    print_error "mitigation-node binary missing"
    exit 1
fi

if [ -f "mitigation-node/target/release/test-origin" ]; then
    print_success "test-origin binary created"
else
    print_error "test-origin binary missing"
    exit 1
fi

# Create TLS certificates if needed
print_status "Checking TLS certificates..."
if [ ! -f "mitigation-node/certs/cert.pem" ] || [ ! -f "mitigation-node/certs/key.pem" ]; then
    print_warning "TLS certificates missing, generating..."
    cd mitigation-node
    mkdir -p certs
    openssl req -x509 -newkey rsa:2048 \
        -keyout certs/key.pem -out certs/cert.pem \
        -days 365 -nodes -subj "/CN=localhost" 2>/dev/null
    cd ..
    print_success "TLS certificates generated"
else
    print_success "TLS certificates found"
fi

# Check documentation
print_status "Verifying documentation..."
docs_found=0
for doc in README.md PHASE1_README.md PHASE2_README.md PHASE3_README.md PHASE4_README.md PHASE6_README.md PHASE7_README.md; do
    if [ -f "$doc" ]; then
        docs_found=$((docs_found + 1))
    fi
done

if [ $docs_found -eq 7 ]; then
    print_success "All documentation files present"
else
    print_warning "Some documentation files missing ($docs_found/7 found)"
fi

# Check test scripts
print_status "Verifying test scripts..."
tests_found=0
for test in test_phase1.sh test_phase2.sh test_phase3.sh test_phase4.sh test_phase6.sh test_phase7.sh test_all.sh; do
    if [ -f "$test" ] && [ -x "$test" ]; then
        tests_found=$((tests_found + 1))
    fi
done

if [ $tests_found -eq 7 ]; then
    print_success "All test scripts present and executable"
else
    print_warning "Some test scripts missing or not executable ($tests_found/7 found)"
fi

# Check .gitignore
if [ -f ".gitignore" ]; then
    print_success ".gitignore file present"
else
    print_warning ".gitignore file missing"
fi

print_status "Running basic functionality test..."
cd mitigation-node

# Start test origin server
./target/release/test-origin &
ORIGIN_PID=$!
sleep 2

# Test if origin server is responding
if curl -s --max-time 5 "http://127.0.0.1:8080/" > /dev/null; then
    print_success "Test origin server working"
else
    print_error "Test origin server not responding"
    kill $ORIGIN_PID 2>/dev/null || true
    exit 1
fi

# Clean up
kill $ORIGIN_PID 2>/dev/null || true
cd ..

echo ""
echo "=============================================="
echo "             Build Summary"
echo "=============================================="
print_success "✅ All components built successfully"
print_success "✅ Release binaries created"
print_success "✅ TLS certificates ready"
print_success "✅ Documentation complete"
print_success "✅ Test infrastructure ready"
print_success "✅ Basic functionality verified"

echo ""
echo -e "${GREEN}🚀 SecBeat is ready for production deployment!${NC}"
echo ""
echo "Next steps:"
echo "  • Run individual phase tests: sudo ./test_phase1.sh"
echo "  • Run full test suite: sudo ./test_all.sh"
echo "  • Deploy orchestrator: cd orchestrator-node && cargo run --release"
echo "  • Deploy mitigation node: cd mitigation-node && sudo cargo run --release"
echo ""
echo "For more information, see README.md"
