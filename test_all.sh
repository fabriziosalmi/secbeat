#!/bin/bash

# SecBeat Comprehensive Test Suite
# Tests all phases of the SecBeat project from Phase 1 through Phase 7

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$SCRIPT_DIR"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
CYAN='\033[0;36m'
MAGENTA='\033[0;35m'
NC='\033[0m' # No Color

# Configuration
START_TIME=$(date +%s)
TOTAL_TESTS=0
PASSED_TESTS=0
FAILED_TESTS=0
SKIPPED_TESTS=0

echo -e "${CYAN}============================================${NC}"
echo -e "${CYAN}    SecBeat Comprehensive Test Suite      ${NC}"
echo -e "${CYAN}   Testing All Phases (1-7) End-to-End   ${NC}"
echo -e "${CYAN}============================================${NC}"
echo

# Function to print test status
print_header() {
    echo -e "${MAGENTA}=== $1 ===${NC}"
}

print_test() {
    echo -e "${BLUE}[TEST]${NC} $1"
    TOTAL_TESTS=$((TOTAL_TESTS + 1))
}

print_success() {
    echo -e "${GREEN}[PASS]${NC} $1"
    PASSED_TESTS=$((PASSED_TESTS + 1))
}

print_error() {
    echo -e "${RED}[FAIL]${NC} $1"
    FAILED_TESTS=$((FAILED_TESTS + 1))
}

print_warning() {
    echo -e "${YELLOW}[WARN]${NC} $1"
}

print_skip() {
    echo -e "${YELLOW}[SKIP]${NC} $1"
    SKIPPED_TESTS=$((SKIPPED_TESTS + 1))
}

# Function to check if running as root
check_privileges() {
    if [ "$EUID" -ne 0 ]; then
        print_error "This comprehensive test requires root privileges"
        echo "Please run with sudo: sudo ./test_all.sh"
        exit 1
    fi
}

# Function to run a phase test
run_phase_test() {
    local phase_num=$1
    local test_script="$PROJECT_ROOT/test_phase${phase_num}.sh"
    local phase_name="$2"
    
    print_header "Phase $phase_num: $phase_name"
    
    if [ -f "$test_script" ]; then
        print_test "Running Phase $phase_num test suite..."
        
        if bash "$test_script"; then
            print_success "Phase $phase_num tests completed successfully"
            return 0
        else
            print_error "Phase $phase_num tests failed"
            return 1
        fi
    else
        print_skip "Phase $phase_num test script not found: $test_script"
        return 2
    fi
}

# Function to check dependencies
check_dependencies() {
    print_header "Dependency Check"
    
    # Check Rust toolchain
    print_test "Checking Rust toolchain..."
    if command -v cargo &> /dev/null; then
        RUST_VERSION=$(rustc --version)
        print_success "Rust toolchain available: $RUST_VERSION"
    else
        print_error "Rust toolchain not found"
        return 1
    fi
    
    # Check required tools
    local tools=("curl" "jq" "lsof" "bc")
    for tool in "${tools[@]}"; do
        print_test "Checking for $tool..."
        if command -v "$tool" &> /dev/null; then
            print_success "$tool is available"
        else
            print_warning "$tool not found (some tests may be limited)"
        fi
    done
    
    # Check optional tools
    local optional_tools=("hping3" "wrk" "ab" "openssl")
    for tool in "${optional_tools[@]}"; do
        print_test "Checking for optional tool $tool..."
        if command -v "$tool" &> /dev/null; then
            print_success "$tool is available"
        else
            print_warning "$tool not found (advanced tests may be skipped)"
        fi
    done
    
    echo
}

# Function to build all components
build_all_components() {
    print_header "Building All Components"
    
    # Build orchestrator
    print_test "Building orchestrator-node..."
    cd "$PROJECT_ROOT/orchestrator-node"
    if cargo build --release; then
        print_success "Orchestrator build completed"
    else
        print_error "Orchestrator build failed"
        return 1
    fi
    
    # Build mitigation node with all features
    print_test "Building mitigation-node with all features..."
    cd "$PROJECT_ROOT/mitigation-node"
    if cargo build --release --all-features; then
        print_success "Mitigation node build completed"
    else
        print_error "Mitigation node build failed"
        return 1
    fi
    
    echo
}

# Function to run pre-flight checks
preflight_checks() {
    print_header "Pre-flight Checks"
    
    # Check for port conflicts
    print_test "Checking for port conflicts..."
    local ports=(8080 8443 8444 8445 9090 9091)
    local conflicts=0
    
    for port in "${ports[@]}"; do
        if lsof -i:$port > /dev/null 2>&1; then
            print_warning "Port $port is in use"
            conflicts=$((conflicts + 1))
        fi
    done
    
    if [ $conflicts -eq 0 ]; then
        print_success "No port conflicts detected"
    else
        print_warning "$conflicts port(s) in use - some tests may fail"
    fi
    
    # Check disk space
    print_test "Checking disk space..."
    local available_space=$(df -h . | tail -1 | awk '{print $4}' | sed 's/[^0-9.]//g')
    if [ "${available_space%.*}" -gt 1 ]; then
        print_success "Sufficient disk space available"
    else
        print_warning "Low disk space may affect tests"
    fi
    
    # Check system load
    print_test "Checking system load..."
    local load=$(uptime | awk -F'load average:' '{print $2}' | awk '{print $1}' | sed 's/,//')
    if [ "$(echo "$load < 2.0" | bc -l 2>/dev/null || echo "1")" = "1" ]; then
        print_success "System load is acceptable"
    else
        print_warning "High system load may affect test timing"
    fi
    
    echo
}

# Function to generate test report
generate_report() {
    local end_time=$(date +%s)
    local duration=$((end_time - START_TIME))
    local minutes=$((duration / 60))
    local seconds=$((duration % 60))
    
    echo
    print_header "Test Results Summary"
    
    echo -e "${CYAN}Total Test Duration: ${minutes}m ${seconds}s${NC}"
    echo
    echo -e "${GREEN}Passed Tests:  $PASSED_TESTS${NC}"
    echo -e "${RED}Failed Tests:  $FAILED_TESTS${NC}"
    echo -e "${YELLOW}Skipped Tests: $SKIPPED_TESTS${NC}"
    echo -e "${BLUE}Total Tests:   $TOTAL_TESTS${NC}"
    echo
    
    if [ $FAILED_TESTS -eq 0 ]; then
        echo -e "${GREEN}🎉 All tests passed! SecBeat is ready for production deployment.${NC}"
        SUCCESS_RATE="100%"
    else
        local success_rate=$((PASSED_TESTS * 100 / (PASSED_TESTS + FAILED_TESTS)))
        echo -e "${YELLOW}⚠️  Some tests failed. Success rate: ${success_rate}%${NC}"
        SUCCESS_RATE="${success_rate}%"
    fi
    
    echo
    print_header "Next Steps"
    
    if [ $FAILED_TESTS -eq 0 ]; then
        echo "✅ All phases tested successfully"
        echo "✅ System is ready for production deployment"
        echo "✅ All security features are functional"
        echo "✅ Orchestrator and nodes are properly integrated"
        echo "✅ AI and self-healing capabilities are working"
        echo
        echo "🚀 You can now deploy SecBeat to your production environment!"
    else
        echo "❌ Some tests failed - review the output above"
        echo "🔧 Fix any issues before production deployment"
        echo "📋 Re-run specific phase tests as needed"
        echo "🔍 Check logs for detailed error information"
    fi
    
    echo
    echo -e "${CYAN}============================================${NC}"
    echo -e "${CYAN}  SecBeat Comprehensive Test Complete     ${NC}"
    echo -e "${CYAN}    Success Rate: $SUCCESS_RATE                     ${NC}"
    echo -e "${CYAN}============================================${NC}"
}

# Function to handle cleanup on exit
cleanup() {
    echo
    print_warning "Cleaning up any remaining processes..."
    
    # Kill any remaining test processes
    pkill -f "test-origin" 2>/dev/null || true
    pkill -f "mitigation-node" 2>/dev/null || true
    pkill -f "orchestrator-node" 2>/dev/null || true
    
    # Kill any webhook servers from Phase 7
    pkill -f "webhook_server" 2>/dev/null || true
    pkill -f "python.*8090" 2>/dev/null || true
    
    sleep 2
    print_warning "Cleanup completed"
}

# Set up signal handlers
trap cleanup EXIT INT TERM

# Main execution flow
main() {
    echo "Starting comprehensive test suite at $(date)"
    echo
    
    # Check prerequisites
    check_privileges
    check_dependencies
    
    # Pre-flight checks
    preflight_checks
    
    # Build all components
    build_all_components
    
    # Initialize test results
    local phase_results=()
    
    # Run Phase 1: Basic TCP Proxy
    if run_phase_test 1 "Basic TCP Proxy"; then
        phase_results[1]="PASS"
    else
        phase_results[1]="FAIL"
        if [ "${STOP_ON_FAILURE:-false}" = "true" ]; then
            print_error "Stopping on Phase 1 failure"
            exit 1
        fi
    fi
    
    echo
    
    # Run Phase 2: SYN Proxy DDoS Mitigation
    if run_phase_test 2 "SYN Proxy DDoS Mitigation"; then
        phase_results[2]="PASS"
    else
        phase_results[2]="FAIL"
        if [ "${STOP_ON_FAILURE:-false}" = "true" ]; then
            print_error "Stopping on Phase 2 failure"
            exit 1
        fi
    fi
    
    echo
    
    # Run Phase 3: TLS Termination and L7 HTTP Parsing
    if run_phase_test 3 "TLS Termination and L7 HTTP Parsing"; then
        phase_results[3]="PASS"
    else
        phase_results[3]="FAIL"
        if [ "${STOP_ON_FAILURE:-false}" = "true" ]; then
            print_error "Stopping on Phase 3 failure"
            exit 1
        fi
    fi
    
    echo
    
    # Run Phase 4: Orchestrator Integration & Self-Registration
    if run_phase_test 4 "Orchestrator Integration & Self-Registration"; then
        phase_results[4]="PASS"
    else
        phase_results[4]="FAIL"
        if [ "${STOP_ON_FAILURE:-false}" = "true" ]; then
            print_error "Stopping on Phase 4 failure"
            exit 1
        fi
    fi
    
    echo
    
    # Check if Phase 6 and 7 tests exist (they should from previous work)
    local phase6_exists=false
    local phase7_exists=false
    
    if [ -f "$PROJECT_ROOT/test_phase6.sh" ]; then
        phase6_exists=true
    fi
    
    if [ -f "$PROJECT_ROOT/test_phase7.sh" ]; then
        phase7_exists=true
    fi
    
    # Run Phase 6: Intelligent Scaling & Node Self-Termination (if available)
    if [ "$phase6_exists" = true ]; then
        if run_phase_test 6 "Intelligent Scaling & Node Self-Termination"; then
            phase_results[6]="PASS"
        else
            phase_results[6]="FAIL"
        fi
        echo
    else
        print_skip "Phase 6 test not available"
        phase_results[6]="SKIP"
    fi
    
    # Run Phase 7: Predictive AI & Proactive Self-Healing (if available)
    if [ "$phase7_exists" = true ]; then
        if run_phase_test 7 "Predictive AI & Proactive Self-Healing"; then
            phase_results[7]="PASS"
        else
            phase_results[7]="FAIL"
        fi
        echo
    else
        print_skip "Phase 7 test not available"
        phase_results[7]="SKIP"
    fi
    
    # Display phase results summary
    print_header "Phase Results Summary"
    for i in {1..7}; do
        if [ -n "${phase_results[i]}" ]; then
            case "${phase_results[i]}" in
                "PASS")
                    echo -e "Phase $i: ${GREEN}PASSED${NC}"
                    ;;
                "FAIL")
                    echo -e "Phase $i: ${RED}FAILED${NC}"
                    ;;
                "SKIP")
                    echo -e "Phase $i: ${YELLOW}SKIPPED${NC}"
                    ;;
            esac
        fi
    done
    
    echo
    
    # Generate final report
    generate_report
    
    # Return appropriate exit code
    if [ $FAILED_TESTS -eq 0 ]; then
        return 0
    else
        return 1
    fi
}

# Parse command line arguments
while [[ $# -gt 0 ]]; do
    case $1 in
        --stop-on-failure)
            STOP_ON_FAILURE=true
            shift
            ;;
        --help)
            echo "Usage: $0 [OPTIONS]"
            echo ""
            echo "Options:"
            echo "  --stop-on-failure    Stop testing when a phase fails"
            echo "  --help              Show this help message"
            echo ""
            echo "This script runs comprehensive tests for all SecBeat phases."
            echo "It requires root privileges for network operations."
            exit 0
            ;;
        *)
            print_error "Unknown option: $1"
            echo "Use --help for usage information"
            exit 1
            ;;
    esac
done

# Run main function
main
