#!/bin/bash
# WASM Memory Leak Audit Script
#
# This script performs comprehensive memory leak analysis on the WASM engine
# using multiple tools and techniques:
#
# 1. valgrind/memcheck - detect memory leaks and invalid accesses
# 2. heaptrack - profile heap allocations over time
# 3. massif - heap profiler for detailed allocation patterns
# 4. Custom stress testing - many hot-reload cycles
#
# Requirements:
# - valgrind (Linux/macOS): brew install valgrind or apt install valgrind
# - heaptrack (Linux): apt install heaptrack
# - massif (part of valgrind)
#
# Usage:
#   ./audit_wasm_memory.sh [--valgrind|--heaptrack|--massif|--all]

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Default mode
MODE="${1:---all}"

# Create reports directory
REPORT_DIR="../reports/wasm-memory-audit"
mkdir -p "$REPORT_DIR"
TIMESTAMP=$(date +"%Y%m%d-%H%M%S")

echo -e "${BLUE}========================================${NC}"
echo -e "${BLUE}WASM Memory Leak Audit${NC}"
echo -e "${BLUE}========================================${NC}"
echo ""
echo "Report directory: $REPORT_DIR"
echo "Timestamp: $TIMESTAMP"
echo ""

# Function to check if a command exists
command_exists() {
    command -v "$1" >/dev/null 2>&1
}

# Function to run valgrind memcheck
run_valgrind() {
    echo -e "${YELLOW}[1/4] Running Valgrind Memcheck...${NC}"
    
    if ! command_exists valgrind; then
        echo -e "${RED}❌ valgrind not found. Install with: brew install valgrind${NC}"
        return 1
    fi

    # Build test binary in debug mode (with symbols)
    echo "Building test binary with debug symbols..."
    cargo test --no-run --test wasm_memory_leak_tests 2>&1 | grep -E "Executable.*wasm_memory_leak" | tail -1 > /tmp/test_binary.txt || true
    
    # Extract binary path
    TEST_BINARY=$(cat /tmp/test_binary.txt | grep -oE '\(/[^)]+\)' | tr -d '()' || echo "")
    
    if [ -z "$TEST_BINARY" ]; then
        # Fallback: find the binary manually
        TEST_BINARY=$(find ../target/debug/deps -name 'wasm_memory_leak_tests-*' -type f -executable | head -1)
    fi
    
    if [ -z "$TEST_BINARY" ] || [ ! -f "$TEST_BINARY" ]; then
        echo -e "${RED}❌ Could not find test binary${NC}"
        return 1
    fi
    
    echo "Test binary: $TEST_BINARY"
    
    # Run valgrind
    VALGRIND_LOG="$REPORT_DIR/valgrind-${TIMESTAMP}.log"
    echo "Running valgrind (this may take a while)..."
    
    valgrind \
        --leak-check=full \
        --show-leak-kinds=all \
        --track-origins=yes \
        --verbose \
        --log-file="$VALGRIND_LOG" \
        "$TEST_BINARY" \
        --test test_hot_reload_no_memory_leak \
        --nocapture \
        2>&1 || true
    
    echo ""
    echo -e "${GREEN}✓ Valgrind report saved to: $VALGRIND_LOG${NC}"
    
    # Parse results
    if grep -q "ERROR SUMMARY: 0 errors" "$VALGRIND_LOG"; then
        echo -e "${GREEN}✓ No memory errors detected${NC}"
    else
        echo -e "${RED}⚠️  Memory errors detected - check report${NC}"
    fi
    
    if grep -q "definitely lost: 0 bytes" "$VALGRIND_LOG"; then
        echo -e "${GREEN}✓ No definite memory leaks${NC}"
    else
        echo -e "${RED}⚠️  Memory leaks detected - check report${NC}"
    fi
    
    echo ""
}

# Function to run heaptrack (Linux only)
run_heaptrack() {
    echo -e "${YELLOW}[2/4] Running Heaptrack...${NC}"
    
    if ! command_exists heaptrack; then
        echo -e "${YELLOW}⚠️  heaptrack not found (Linux only). Skipping...${NC}"
        echo "Install with: sudo apt install heaptrack"
        return 0
    fi
    
    # Build test binary
    cargo test --no-run --test wasm_memory_leak_tests
    TEST_BINARY=$(find ../target/debug/deps -name 'wasm_memory_leak_tests-*' -type f -executable | head -1)
    
    if [ -z "$TEST_BINARY" ]; then
        echo -e "${RED}❌ Could not find test binary${NC}"
        return 1
    fi
    
    HEAPTRACK_DATA="$REPORT_DIR/heaptrack-${TIMESTAMP}.gz"
    
    echo "Running heaptrack..."
    heaptrack -o "$HEAPTRACK_DATA" "$TEST_BINARY" --test test_hot_reload_no_memory_leak --nocapture 2>&1 || true
    
    echo -e "${GREEN}✓ Heaptrack data saved to: $HEAPTRACK_DATA${NC}"
    echo "Analyze with: heaptrack --analyze $HEAPTRACK_DATA"
    echo ""
}

# Function to run massif
run_massif() {
    echo -e "${YELLOW}[3/4] Running Massif (heap profiler)...${NC}"
    
    if ! command_exists valgrind; then
        echo -e "${RED}❌ valgrind not found. Install with: brew install valgrind${NC}"
        return 1
    fi
    
    # Build test binary
    cargo test --no-run --test wasm_memory_leak_tests
    TEST_BINARY=$(find ../target/debug/deps -name 'wasm_memory_leak_tests-*' -type f -executable | head -1)
    
    if [ -z "$TEST_BINARY" ]; then
        echo -e "${RED}❌ Could not find test binary${NC}"
        return 1
    fi
    
    MASSIF_OUT="$REPORT_DIR/massif-${TIMESTAMP}.out"
    
    echo "Running massif (this may take a while)..."
    valgrind \
        --tool=massif \
        --massif-out-file="$MASSIF_OUT" \
        --time-unit=B \
        --detailed-freq=1 \
        "$TEST_BINARY" \
        --test test_hot_reload_no_memory_leak \
        --nocapture \
        2>&1 || true
    
    echo -e "${GREEN}✓ Massif output saved to: $MASSIF_OUT${NC}"
    echo "Visualize with: ms_print $MASSIF_OUT"
    echo ""
}

# Function to run custom stress test
run_stress_test() {
    echo -e "${YELLOW}[4/4] Running Custom Stress Test...${NC}"
    
    STRESS_LOG="$REPORT_DIR/stress-test-${TIMESTAMP}.log"
    
    echo "Building and running stress test..."
    cargo test --test wasm_memory_leak_tests -- --nocapture > "$STRESS_LOG" 2>&1 || true
    
    echo -e "${GREEN}✓ Stress test log saved to: $STRESS_LOG${NC}"
    
    # Parse results
    if grep -q "Memory growth factor:" "$STRESS_LOG"; then
        GROWTH=$(grep "Memory growth factor:" "$STRESS_LOG" | tail -1 | grep -oE '[0-9]+\.[0-9]+')
        if (( $(echo "$GROWTH < 2.0" | bc -l) )); then
            echo -e "${GREEN}✓ Memory growth acceptable: ${GROWTH}x${NC}"
        else
            echo -e "${RED}⚠️  High memory growth: ${GROWTH}x - possible leak!${NC}"
        fi
    else
        echo -e "${YELLOW}⚠️  Could not parse memory growth${NC}"
    fi
    
    # Check test results
    if grep -q "test result: ok" "$STRESS_LOG"; then
        echo -e "${GREEN}✓ All tests passed${NC}"
    else
        echo -e "${RED}❌ Some tests failed - check log${NC}"
    fi
    
    echo ""
}

# Generate summary report
generate_summary() {
    echo -e "${BLUE}========================================${NC}"
    echo -e "${BLUE}Audit Summary${NC}"
    echo -e "${BLUE}========================================${NC}"
    
    SUMMARY_FILE="$REPORT_DIR/SUMMARY-${TIMESTAMP}.md"
    
    cat > "$SUMMARY_FILE" <<EOF
# WASM Memory Leak Audit Summary

**Date**: $(date)
**Test Suite**: wasm_memory_leak_tests.rs
**Report Directory**: $REPORT_DIR

## Test Methodology

1. **Valgrind Memcheck**: Detects memory leaks and invalid memory accesses
2. **Heaptrack**: Profiles heap allocations over time (Linux only)
3. **Massif**: Heap profiler for detailed allocation patterns
4. **Stress Test**: 100+ hot-reload cycles with memory tracking

## Files Generated

EOF
    
    ls -lh "$REPORT_DIR" | tail -n +2 >> "$SUMMARY_FILE"
    
    cat >> "$SUMMARY_FILE" <<EOF

## Key Metrics to Check

- **Definite Leaks**: Should be 0 bytes
- **Possible Leaks**: Review wasmtime internal allocations
- **Memory Growth Factor**: Should be < 2.0x after 100 reload cycles
- **Test Pass Rate**: All tests should pass

## Analysis Commands

\`\`\`bash
# View valgrind results
cat $REPORT_DIR/valgrind-${TIMESTAMP}.log | grep -A 10 "LEAK SUMMARY"

# Stress test results
cat $REPORT_DIR/stress-test-${TIMESTAMP}.log | grep "Memory growth"

# Massif visualization
ms_print $REPORT_DIR/massif-${TIMESTAMP}.out | head -100
\`\`\`

## Recommendations

1. Monitor wasmtime Store cleanup in Drop implementations
2. Verify Module instances don't reference parent Engine
3. Check RwLock<HashMap> cleanup in unload_module()
4. Review wasmtime fuel metering for resource cleanup
5. Consider adding explicit memory::drop() calls

## Next Steps

- [ ] Review all reports for anomalies
- [ ] Fix any detected leaks
- [ ] Add continuous memory leak testing to CI/CD
- [ ] Document wasmtime cleanup guarantees
- [ ] Consider periodic garbage collection hints

EOF
    
    echo -e "${GREEN}✓ Summary saved to: $SUMMARY_FILE${NC}"
    cat "$SUMMARY_FILE"
}

# Main execution
main() {
    case "$MODE" in
        --valgrind)
            run_valgrind
            ;;
        --heaptrack)
            run_heaptrack
            ;;
        --massif)
            run_massif
            ;;
        --stress)
            run_stress_test
            ;;
        --all)
            run_valgrind
            run_heaptrack
            run_massif
            run_stress_test
            generate_summary
            ;;
        *)
            echo "Usage: $0 [--valgrind|--heaptrack|--massif|--stress|--all]"
            exit 1
            ;;
    esac
    
    echo ""
    echo -e "${GREEN}========================================${NC}"
    echo -e "${GREEN}Audit Complete${NC}"
    echo -e "${GREEN}========================================${NC}"
    echo ""
    echo "Reports available in: $REPORT_DIR"
}

# Check if we're in the right directory
if [ ! -f "Cargo.toml" ]; then
    echo -e "${RED}Error: Run this script from mitigation-node directory${NC}"
    exit 1
fi

main
