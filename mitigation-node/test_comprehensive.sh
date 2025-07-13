#!/bin/bash

# SecBeat Comprehensive Test Suite
# Runs all tests and generates detailed reports

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
PURPLE='\033[0;35m'
CYAN='\033[0;36m'
NC='\033[0m' # No Color

# Function to print colored output
print_header() {
    echo -e "${PURPLE}========================================${NC}"
    echo -e "${PURPLE}$1${NC}"
    echo -e "${PURPLE}========================================${NC}"
}

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

print_info() {
    echo -e "${CYAN}ℹ️  $1${NC}"
}

# Configuration
WORKSPACE_ROOT="/Users/fab/GitHub/secbeat"
MITIGATION_NODE_DIR="$WORKSPACE_ROOT/mitigation-node"
REPORTS_DIR="$MITIGATION_NODE_DIR/reports"
TEST_RESULTS_DIR="$REPORTS_DIR/tests"
TIMESTAMP=$(date +'%Y%m%d_%H%M%S')

# Create directories
mkdir -p "$REPORTS_DIR"
mkdir -p "$TEST_RESULTS_DIR"

print_header "SecBeat Comprehensive Test Suite - $TIMESTAMP"

cd "$MITIGATION_NODE_DIR"

# Test configuration
CARGO_FLAGS="--color=always"
RUST_LOG="${RUST_LOG:-info}"
RUST_BACKTRACE="${RUST_BACKTRACE:-1}"

export RUST_LOG
export RUST_BACKTRACE

print_status "Test environment:"
print_info "  Workspace: $WORKSPACE_ROOT"
print_info "  Reports: $REPORTS_DIR"
print_info "  Rust log level: $RUST_LOG"
print_info "  Backtrace: $RUST_BACKTRACE"

# Function to run a test suite and capture results
run_test_suite() {
    local test_name="$1"
    local test_args="$2"
    local output_file="$TEST_RESULTS_DIR/${test_name}_${TIMESTAMP}.txt"
    
    print_status "Running $test_name tests..."
    
    if cargo test $CARGO_FLAGS $test_args 2>&1 | tee "$output_file"; then
        print_success "$test_name tests completed"
        return 0
    else
        print_error "$test_name tests failed"
        return 1
    fi
}

# Function to generate test summary
generate_test_summary() {
    local summary_file="$REPORTS_DIR/test_summary_${TIMESTAMP}.md"
    
    print_status "Generating test summary..."
    
    cat > "$summary_file" << EOF
# SecBeat Test Suite Summary

**Execution Date:** $(date)
**Environment:** $(uname -a)
**Rust Version:** $(rustc --version)
**Cargo Version:** $(cargo --version)

## Test Results

EOF

    local total_tests=0
    local passed_tests=0
    local failed_tests=0
    
    for result_file in "$TEST_RESULTS_DIR"/*_${TIMESTAMP}.txt; do
        if [ -f "$result_file" ]; then
            local test_name=$(basename "$result_file" "_${TIMESTAMP}.txt")
            echo "### $test_name" >> "$summary_file"
            echo "" >> "$summary_file"
            
            # Extract test results
            local test_count=$(grep -E "test result:" "$result_file" | tail -1 || echo "")
            if [ -n "$test_count" ]; then
                echo "**Result:** \`$test_count\`" >> "$summary_file"
                
                # Parse numbers
                local passed=$(echo "$test_count" | grep -oE "[0-9]+ passed" | grep -oE "[0-9]+" || echo "0")
                local failed=$(echo "$test_count" | grep -oE "[0-9]+ failed" | grep -oE "[0-9]+" || echo "0")
                
                total_tests=$((total_tests + passed + failed))
                passed_tests=$((passed_tests + passed))
                failed_tests=$((failed_tests + failed))
            else
                echo "**Result:** No test results found" >> "$summary_file"
            fi
            
            echo "" >> "$summary_file"
            
            # Include any performance metrics
            if grep -q "Performance:" "$result_file"; then
                echo "**Performance Metrics:**" >> "$summary_file"
                echo "\`\`\`" >> "$summary_file"
                grep -A 5 "Performance:" "$result_file" | head -10 >> "$summary_file"
                echo "\`\`\`" >> "$summary_file"
                echo "" >> "$summary_file"
            fi
            
            # Include any failures
            if grep -q "FAILED" "$result_file"; then
                echo "**Failures:**" >> "$summary_file"
                echo "\`\`\`" >> "$summary_file"
                grep -B 2 -A 2 "FAILED" "$result_file" >> "$summary_file"
                echo "\`\`\`" >> "$summary_file"
                echo "" >> "$summary_file"
            fi
        fi
    done
    
    # Add overall summary
    cat >> "$summary_file" << EOF

## Overall Summary

- **Total Tests:** $total_tests
- **Passed:** $passed_tests
- **Failed:** $failed_tests
- **Success Rate:** $(( total_tests > 0 ? (passed_tests * 100 / total_tests) : 0 ))%

## Test Files

EOF

    for result_file in "$TEST_RESULTS_DIR"/*_${TIMESTAMP}.txt; do
        if [ -f "$result_file" ]; then
            echo "- [$(basename "$result_file")]($result_file)" >> "$summary_file"
        fi
    done
    
    print_success "Test summary generated: $summary_file"
    
    # Show summary statistics
    print_info "Overall Test Results:"
    print_info "  Total: $total_tests"
    print_info "  Passed: $passed_tests"
    print_info "  Failed: $failed_tests"
    if [ $total_tests -gt 0 ]; then
        local success_rate=$(( passed_tests * 100 / total_tests ))
        print_info "  Success Rate: ${success_rate}%"
    fi
}

# Build the project first
print_status "Building mitigation-node..."
if cargo build $CARGO_FLAGS; then
    print_success "Build completed successfully"
else
    print_error "Build failed"
    exit 1
fi

# Check for compilation warnings/errors
print_status "Running cargo check..."
if cargo check $CARGO_FLAGS > "$TEST_RESULTS_DIR/cargo_check_${TIMESTAMP}.txt" 2>&1; then
    print_success "Cargo check passed"
else
    print_warning "Cargo check found issues (check detailed output)"
fi

# Run test suites
test_suites=(
    "unit_tests:--test unit_tests"
    "integration_tests:--test integration_tests"
    "performance_tests:--test performance_tests"
    "doc_tests:--doc"
    "all_tests:--lib --bins"
)

test_results=()

for suite in "${test_suites[@]}"; do
    IFS=':' read -r test_name test_args <<< "$suite"
    
    if run_test_suite "$test_name" "$test_args"; then
        test_results+=("$test_name:PASS")
    else
        test_results+=("$test_name:FAIL")
    fi
    
    echo # Add spacing between test suites
done

# Generate comprehensive report
generate_test_summary

# Display final results
print_header "Test Suite Execution Summary"

for result in "${test_results[@]}"; do
    IFS=':' read -r test_name status <<< "$result"
    if [ "$status" = "PASS" ]; then
        print_success "$test_name: PASSED"
    else
        print_error "$test_name: FAILED"
    fi
done

# Check if any tests failed
failed_count=0
for result in "${test_results[@]}"; do
    if [[ "$result" == *":FAIL" ]]; then
        failed_count=$((failed_count + 1))
    fi
done

if [ $failed_count -eq 0 ]; then
    print_success "All test suites completed successfully!"
    exit 0
else
    print_error "$failed_count test suite(s) failed"
    exit 1
fi
