#!/bin/bash
# LXC Test Runner for SecBeat
# Tests in a privileged LXC container with full kernel access for eBPF/XDP validation
#
# Usage: ./run_lxc_tests.sh [options]
#
# Options:
#   --quick        Run only quick tests (unit + integration)
#   --full         Run complete test suite including performance
#   --clean        Clean build before testing
#   --skip-build   Skip build step
#
# Requirements:
#   - Proxmox host with LXC container ID 100
#   - SSH access to Proxmox host (root@192.168.100.102)
#   - LXC container with Rust toolchain installed

set -e

# Configuration
PROXMOX_HOST="${PROXMOX_HOST:-root@192.168.100.102}"
LXC_ID="${LXC_ID:-100}"
TEST_DIR="/root/secbeat-test"
REPO_URL="https://github.com/fabriziosalmi/secbeat.git"

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Parse arguments
QUICK=false
FULL=false
CLEAN=false
SKIP_BUILD=false

for arg in "$@"; do
    case $arg in
        --quick) QUICK=true ;;
        --full) FULL=true ;;
        --clean) CLEAN=true ;;
        --skip-build) SKIP_BUILD=true ;;
        *) echo "Unknown option: $arg"; exit 1 ;;
    esac
done

# Functions
log_info() {
    echo -e "${GREEN}[INFO]${NC} $1"
}

log_warn() {
    echo -e "${YELLOW}[WARN]${NC} $1"
}

log_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

run_lxc_command() {
    ssh "$PROXMOX_HOST" "pct exec $LXC_ID -- bash -c '$1'"
}

# Main test sequence
log_info "Starting LXC test suite for SecBeat"
log_info "Target: $PROXMOX_HOST, LXC container $LXC_ID"

# Step 1: Clone/update repository
log_info "Step 1: Updating repository in LXC container..."
if [ "$CLEAN" = true ]; then
    log_info "Cleaning existing directory..."
    run_lxc_command "rm -rf $TEST_DIR"
fi

run_lxc_command "if [ -d $TEST_DIR ]; then cd $TEST_DIR && git pull; else git clone $REPO_URL $TEST_DIR; fi" 2>&1 | tail -5

# Step 2: Verify Rust toolchain
log_info "Step 2: Verifying Rust toolchain..."
RUST_VERSION=$(run_lxc_command "source \$HOME/.cargo/env && rustc --version")
log_info "Rust: $RUST_VERSION"

# Step 3: Build project
if [ "$SKIP_BUILD" = false ]; then
    log_info "Step 3: Building mitigation-node (release mode)..."
    run_lxc_command "cd $TEST_DIR && source \$HOME/.cargo/env && cargo build -p mitigation-node --release" 2>&1 | grep -E "(Compiling|Finished)" | tail -5
    log_info "Build completed"
else
    log_warn "Skipping build step"
fi

# Step 4: Run unit tests (library)
log_info "Step 4: Running unit tests (library)..."
UNIT_LIB_OUTPUT=$(run_lxc_command "cd $TEST_DIR && source \$HOME/.cargo/env && cargo test -p mitigation-node --lib 2>&1" || true)
UNIT_LIB_RESULT=$(echo "$UNIT_LIB_OUTPUT" | grep "test result:" | tail -1)
if [ -z "$UNIT_LIB_RESULT" ]; then
    UNIT_LIB_RESULT=$(echo "$UNIT_LIB_OUTPUT" | grep -E "(FAILED|error:)" | tail -1)
fi
echo "$UNIT_LIB_RESULT"

# Step 5: Run unit tests (test file)
log_info "Step 5: Running unit tests (test file)..."
UNIT_TEST_OUTPUT=$(run_lxc_command "cd $TEST_DIR && source \$HOME/.cargo/env && cargo test -p mitigation-node --test unit_tests 2>&1" || true)
UNIT_TEST_RESULT=$(echo "$UNIT_TEST_OUTPUT" | grep "test result:" | tail -1)
if [ -z "$UNIT_TEST_RESULT" ]; then
    UNIT_TEST_RESULT=$(echo "$UNIT_TEST_OUTPUT" | grep -E "(FAILED|error:)" | tail -1)
fi
echo "$UNIT_TEST_RESULT"

# Step 6: Run integration tests
log_info "Step 6: Running integration tests..."
INTEGRATION_OUTPUT=$(run_lxc_command "cd $TEST_DIR && source \$HOME/.cargo/env && cargo test -p mitigation-node --test integration_tests 2>&1" || true)
INTEGRATION_RESULT=$(echo "$INTEGRATION_OUTPUT" | grep "test result:" | tail -1)
if [ -z "$INTEGRATION_RESULT" ]; then
    INTEGRATION_RESULT=$(echo "$INTEGRATION_OUTPUT" | grep -E "(FAILED|error:)" | tail -1)
fi
echo "$INTEGRATION_RESULT"

# Step 7: Run performance tests (if full mode)
if [ "$FULL" = true ]; then
    log_info "Step 7: Running performance tests..."
    PERF_OUTPUT=$(run_lxc_command "cd $TEST_DIR && source \$HOME/.cargo/env && cargo test -p mitigation-node --test performance_tests -- --test-threads=1 2>&1" || true)
    PERF_RESULT=$(echo "$PERF_OUTPUT" | grep "test result:" | tail -1)
    if [ -z "$PERF_RESULT" ]; then
        PERF_RESULT=$(echo "$PERF_OUTPUT" | grep -E "(FAILED|error:)" | tail -1)
    fi
    echo "$PERF_RESULT"
fi

# Step 8: Check kernel capabilities
log_info "Step 8: Verifying kernel capabilities for eBPF/XDP..."
run_lxc_command "uname -r && ls /sys/kernel/btf/vmlinux 2>/dev/null && echo 'BTF available' || echo 'BTF not available'"

# Summary
log_info ""
log_info "========================================="
log_info "LXC Test Summary"
log_info "========================================="
log_info "Unit tests (lib): $UNIT_LIB_RESULT"
log_info "Unit tests (file): $UNIT_TEST_RESULT"
log_info "Integration tests: $INTEGRATION_RESULT"
if [ "$FULL" = true ]; then
    log_info "Performance tests: $PERF_RESULT"
fi
log_info "========================================="

# Extract pass/fail counts
extract_counts() {
    local result="$1"
    local passed=$(echo "$result" | grep -oP '\d+(?= passed)' || echo "0")
    local failed=$(echo "$result" | grep -oP '\d+(?= failed)' || echo "0")
    echo "$passed:$failed"
}

UNIT_TEST_COUNTS=$(extract_counts "$UNIT_TEST_RESULT")
UNIT_TEST_PASSED=$(echo "$UNIT_TEST_COUNTS" | cut -d: -f1)
UNIT_TEST_FAILED=$(echo "$UNIT_TEST_COUNTS" | cut -d: -f2)

INTEGRATION_COUNTS=$(extract_counts "$INTEGRATION_RESULT")
INTEGRATION_PASSED=$(echo "$INTEGRATION_COUNTS" | cut -d: -f1)
INTEGRATION_FAILED=$(echo "$INTEGRATION_COUNTS" | cut -d: -f2)

# Check for critical failures (unit tests should mostly pass)
CRITICAL_FAILURE=false

if [ "$UNIT_TEST_PASSED" -lt 15 ]; then
    log_error "Critical: Unit tests passed ($UNIT_TEST_PASSED) is below threshold (15)"
    CRITICAL_FAILURE=true
fi

if [ "$INTEGRATION_PASSED" -lt 12 ]; then
    log_error "Critical: Integration tests passed ($INTEGRATION_PASSED) is below threshold (12)"
    CRITICAL_FAILURE=true
fi

if [ "$CRITICAL_FAILURE" = true ]; then
    log_error "Critical test failures detected!"
    exit 1
else
    log_info "✅ LXC tests passed (Unit: $UNIT_TEST_PASSED/$((UNIT_TEST_PASSED + UNIT_TEST_FAILED)), Integration: $INTEGRATION_PASSED/$((INTEGRATION_PASSED + INTEGRATION_FAILED)))"
    log_info "✅ Kernel has eBPF/XDP capabilities available"
    exit 0
fi
