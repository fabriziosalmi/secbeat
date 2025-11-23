#!/usr/bin/env bash
# Unified Test Suite for SecBeat
#
# Runs complete test pipeline:
# 1. Unit tests (cargo test)
# 2. Build Docker images
# 3. Start test environment
# 4. Integration tests
# 5. Cleanup
#
# Usage:
#   ./test_unified.sh            # Run full suite
#   ./test_unified.sh --skip-unit  # Skip unit tests
#   ./test_unified.sh --no-cleanup # Leave environment running

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$SCRIPT_DIR"

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
CYAN='\033[0;36m'
BOLD='\033[1m'
NC='\033[0m' # No Color

# Flags
SKIP_UNIT=false
NO_CLEANUP=false

# Parse arguments
for arg in "$@"; do
    case $arg in
        --skip-unit)
            SKIP_UNIT=true
            ;;
        --no-cleanup)
            NO_CLEANUP=true
            ;;
        --help|-h)
            echo "Usage: $0 [OPTIONS]"
            echo ""
            echo "Options:"
            echo "  --skip-unit    Skip unit tests (cargo test)"
            echo "  --no-cleanup   Leave test environment running after tests"
            echo "  --help         Show this help message"
            exit 0
            ;;
    esac
done

# Statistics
declare -A TEST_RESULTS
declare -A TEST_TIMES

log_header() {
    echo -e "\n${BOLD}${CYAN}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
    echo -e "${BOLD}${CYAN}  $1${NC}"
    echo -e "${BOLD}${CYAN}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}\n"
}

log_step() {
    echo -e "${BLUE}▶${NC} $1"
}

log_success() {
    echo -e "${GREEN}✓${NC} $1"
}

log_error() {
    echo -e "${RED}✗${NC} $1"
}

log_warn() {
    echo -e "${YELLOW}⚠${NC} $1"
}

# Record test result
record_test() {
    local name=$1
    local status=$2
    local duration=$3
    
    TEST_RESULTS["$name"]=$status
    TEST_TIMES["$name"]=$duration
}

# Print summary table
print_summary() {
    log_header "TEST SUMMARY"
    
    echo -e "${BOLD}| Component          | Status | Time    |${NC}"
    echo -e "${BOLD}|--------------------|--------|---------|${NC}"
    
    for component in Unit XDP WASM ML E2E; do
        if [[ -v TEST_RESULTS["$component"] ]]; then
            local status="${TEST_RESULTS[$component]}"
            local time="${TEST_TIMES[$component]}"
            
            if [[ "$status" == "PASS" ]]; then
                printf "| %-18s | ${GREEN}%-6s${NC} | %-7s |\n" "$component" "$status" "$time"
            elif [[ "$status" == "SKIP" ]]; then
                printf "| %-18s | ${YELLOW}%-6s${NC} | %-7s |\n" "$component" "$status" "$time"
            else
                printf "| %-18s | ${RED}%-6s${NC} | %-7s |\n" "$component" "$status" "$time"
            fi
        fi
    done
    
    echo ""
}

# Cleanup on exit
cleanup() {
    if [[ "$NO_CLEANUP" == "true" ]]; then
        log_warn "Skipping cleanup (--no-cleanup flag set)"
    else
        log_step "Cleaning up test environment..."
        "$PROJECT_ROOT/tests/setup_env.sh" down >/dev/null 2>&1 || true
    fi
}

trap cleanup EXIT

# ==============================================================================
# STAGE 1: UNIT TESTS
# ==============================================================================

run_unit_tests() {
    log_header "STAGE 1: UNIT TESTS"
    
    if [[ "$SKIP_UNIT" == "true" ]]; then
        log_warn "Skipping unit tests (--skip-unit flag set)"
        record_test "Unit" "SKIP" "0s"
        return 0
    fi
    
    log_step "Running cargo test (orchestrator-node)..."
    
    local start_time=$(date +%s)
    
    if cargo test --package orchestrator-node --lib >/dev/null 2>&1; then
        local end_time=$(date +%s)
        local duration=$((end_time - start_time))
        log_success "Unit tests passed in ${duration}s"
        record_test "Unit" "PASS" "${duration}s"
        return 0
    else
        local end_time=$(date +%s)
        local duration=$((end_time - start_time))
        log_error "Unit tests failed"
        record_test "Unit" "FAIL" "${duration}s"
        return 1
    fi
}

# ==============================================================================
# STAGE 2: BUILD & START ENVIRONMENT
# ==============================================================================

build_and_start() {
    log_header "STAGE 2: TEST ENVIRONMENT"
    
    log_step "Building images and starting test environment..."
    local start_time=$(date +%s)
    
    if "$PROJECT_ROOT/tests/setup_env.sh" up; then
        local end_time=$(date +%s)
        local duration=$((end_time - start_time))
        log_success "Environment ready in ${duration}s"
        return 0
    else
        local end_time=$(date +%s)
        local duration=$((end_time - start_time))
        log_error "Environment setup failed"
        return 1
    fi
}

# ==============================================================================
# STAGE 3: INTEGRATION TESTS
# ==============================================================================

run_integration_tests() {
    log_header "STAGE 3: INTEGRATION TESTS"
    
    # Test 1: E2E Flow (Basic connectivity)
    log_step "Running E2E request flow test..."
    local start_time=$(date +%s)
    if timeout 30 cargo test --test e2e_scenarios test_e2e_request_flow -- --ignored --nocapture >/dev/null 2>&1; then
        local end_time=$(date +%s)
        record_test "E2E" "PASS" "$((end_time - start_time))s"
        log_success "E2E test passed"
    else
        local end_time=$(date +%s)
        record_test "E2E" "FAIL" "$((end_time - start_time))s"
        log_error "E2E test failed"
    fi
    
    # Test 2: XDP Blocking
    log_step "Running XDP blocking test..."
    start_time=$(date +%s)
    if timeout 30 cargo test --test e2e_scenarios test_xdp_ip_block -- --ignored --nocapture >/dev/null 2>&1; then
        local end_time=$(date +%s)
        record_test "XDP" "PASS" "$((end_time - start_time))s"
        log_success "XDP test passed"
    else
        local end_time=$(date +%s)
        record_test "XDP" "FAIL" "$((end_time - start_time))s"
        log_error "XDP test failed"
    fi
    
    # Test 3: WASM WAF
    log_step "Running WASM WAF test..."
    start_time=$(date +%s)
    if timeout 60 cargo test --test e2e_scenarios test_wasm_waf_block_admin -- --ignored --nocapture >/dev/null 2>&1; then
        local end_time=$(date +%s)
        record_test "WASM" "PASS" "$((end_time - start_time))s"
        log_success "WASM test passed"
    else
        local end_time=$(date +%s)
        record_test "WASM" "FAIL" "$((end_time - start_time))s"
        log_error "WASM test failed"
    fi
    
    # Test 4: ML Anomaly Detection (may be slow)
    log_step "Running ML anomaly detection test (may take 2+ minutes)..."
    start_time=$(date +%s)
    if timeout 150 cargo test --test e2e_scenarios test_ml_anomaly_detection -- --ignored --nocapture >/dev/null 2>&1; then
        local end_time=$(date +%s)
        record_test "ML" "PASS" "$((end_time - start_time))s"
        log_success "ML test passed"
    else
        local end_time=$(date +%s)
        record_test "ML" "FAIL" "$((end_time - start_time))s"
        log_warn "ML test failed (training may need more time)"
    fi
}

# ==============================================================================
# MAIN EXECUTION
# ==============================================================================

main() {
    log_header "SecBeat Unified Test Suite"
    
    # Stage 1: Unit Tests
    if ! run_unit_tests && [[ "$SKIP_UNIT" != "true" ]]; then
        log_error "Unit tests failed - aborting"
        print_summary
        exit 1
    fi
    
    # Stage 2: Build & Start
    if ! build_and_start; then
        log_error "Environment setup failed - aborting"
        print_summary
        exit 1
    fi
    
    # Stage 3: Integration Tests
    run_integration_tests
    
    # Summary
    print_summary
    
    echo -e "${GREEN}${BOLD}✅ Test suite completed!${NC}\n"
}

main
