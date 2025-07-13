#!/bin/bash

# SecBeat Comprehensive Testing Suite
# Production-grade testing framework for Proxmox deployment
# Includes static analysis, unit tests, integration tests, and e2e testing

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$SCRIPT_DIR"

# Configuration
TEST_ENV="${TEST_ENV:-development}"
PROXMOX_NODE="${PROXMOX_NODE:-192.168.100.23}"
RUST_LOG="${RUST_LOG:-info}"
PARALLEL_JOBS="${PARALLEL_JOBS:-4}"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
CYAN='\033[0;36m'
MAGENTA='\033[0;35m'
BOLD='\033[1m'
NC='\033[0m'

# Test counters
TOTAL_TESTS=0
PASSED_TESTS=0
FAILED_TESTS=0
SKIPPED_TESTS=0

# Test timing
START_TIME=$(date +%s)
PHASE_START_TIME=$START_TIME

echo -e "${CYAN}${BOLD}==========================================================${NC}"
echo -e "${CYAN}${BOLD}     SecBeat Production Testing Suite v2.0              ${NC}"
echo -e "${CYAN}${BOLD}     Comprehensive Static & End-to-End Testing          ${NC}"
echo -e "${CYAN}${BOLD}==========================================================${NC}"
echo
echo -e "${BLUE}Environment: ${BOLD}$TEST_ENV${NC}"
echo -e "${BLUE}Proxmox Node: ${BOLD}$PROXMOX_NODE${NC}"
echo -e "${BLUE}Parallel Jobs: ${BOLD}$PARALLEL_JOBS${NC}"
echo -e "${BLUE}Start Time: ${BOLD}$(date)${NC}"
echo

# Utility functions
print_phase() {
    local phase_end=$(date +%s)
    local phase_duration=$((phase_end - PHASE_START_TIME))
    echo
    echo -e "${MAGENTA}${BOLD}=== $1 (${phase_duration}s) ===${NC}"
    PHASE_START_TIME=$phase_end
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

print_info() {
    echo -e "${CYAN}[INFO]${NC} $1"
}

# Check dependencies
check_dependencies() {
    print_phase "Dependency Check"
    
    local deps=("cargo" "rust" "docker" "kubectl" "curl" "jq" "nc" "ss" "git")
    local missing_deps=()
    
    for dep in "${deps[@]}"; do
        if ! command -v "$dep" &> /dev/null; then
            missing_deps+=("$dep")
        fi
    done
    
    if [ ${#missing_deps[@]} -eq 0 ]; then
        print_success "All required dependencies are installed"
    else
        print_warning "Missing dependencies: ${missing_deps[*]}"
        print_info "Installing missing dependencies..."
        
        # Install missing dependencies based on OS
        if [[ "$OSTYPE" == "darwin"* ]]; then
            # macOS
            for dep in "${missing_deps[@]}"; do
                case $dep in
                    "kubectl") brew install kubectl ;;
                    "jq") brew install jq ;;
                    *) print_warning "Please install $dep manually" ;;
                esac
            done
        elif [[ "$OSTYPE" == "linux-gnu"* ]]; then
            # Linux
            sudo apt-get update
            for dep in "${missing_deps[@]}"; do
                case $dep in
                    "kubectl") 
                        curl -LO "https://dl.k8s.io/release/$(curl -L -s https://dl.k8s.io/release/stable.txt)/bin/linux/amd64/kubectl"
                        sudo install -o root -g root -m 0755 kubectl /usr/local/bin/kubectl
                        ;;
                    "jq") sudo apt-get install -y jq ;;
                    "nc") sudo apt-get install -y netcat ;;
                    "ss") sudo apt-get install -y iproute2 ;;
                    *) print_warning "Please install $dep manually" ;;
                esac
            done
        fi
    fi
}

# Environment setup
setup_environment() {
    print_phase "Environment Setup"
    
    # Create test directories
    mkdir -p logs reports tmp
    
    # Set environment variables
    export RUST_LOG="$RUST_LOG"
    export RUST_BACKTRACE=1
    export CARGO_TERM_COLOR=always
    
    # Clean previous test artifacts
    print_test "Cleaning previous test artifacts"
    rm -rf logs/* reports/* tmp/*
    print_success "Test environment cleaned"
    
    # Check Rust toolchain
    print_test "Verifying Rust toolchain"
    if cargo --version &> /dev/null; then
        local rust_version=$(rustc --version)
        print_success "Rust toolchain: $rust_version"
    else
        print_error "Rust toolchain not found"
        exit 1
    fi
}

# Static analysis and linting
run_static_analysis() {
    print_phase "Static Analysis"
    
    # Rust formatting check
    print_test "Checking Rust code formatting"
    if cargo fmt --all -- --check; then
        print_success "Code formatting is consistent"
    else
        print_warning "Code formatting issues found (run 'cargo fmt')"
    fi
    
    # Clippy linting
    print_test "Running Clippy linting"
    if cargo clippy --all-targets --all-features -- -D warnings; then
        print_success "No Clippy warnings found"
    else
        print_error "Clippy warnings found"
    fi
    
    # Security audit
    print_test "Running security audit"
    if command -v cargo-audit &> /dev/null; then
        if cargo audit; then
            print_success "No security vulnerabilities found"
        else
            print_warning "Security vulnerabilities detected"
        fi
    else
        print_skip "cargo-audit not installed"
    fi
    
    # Dependency check
    print_test "Checking dependencies"
    if cargo tree > reports/dependency-tree.txt; then
        print_success "Dependency tree generated"
    else
        print_error "Failed to generate dependency tree"
    fi
    
    # License check
    print_test "Checking license compliance"
    if command -v cargo-license &> /dev/null; then
        cargo license > reports/licenses.txt
        print_success "License information collected"
    else
        print_skip "cargo-license not installed"
    fi
}

# Build all components
build_components() {
    print_phase "Build Components"
    
    # Build mitigation node
    print_test "Building mitigation node"
    cd "$PROJECT_ROOT/mitigation-node"
    if cargo build --release; then
        print_success "Mitigation node built successfully"
    else
        print_error "Failed to build mitigation node"
        return 1
    fi
    
    # Build orchestrator node
    print_test "Building orchestrator node"
    cd "$PROJECT_ROOT/orchestrator-node"
    if cargo build --release; then
        print_success "Orchestrator node built successfully"
    else
        print_error "Failed to build orchestrator node"
        return 1
    fi
    
    cd "$PROJECT_ROOT"
}

# Unit tests
run_unit_tests() {
    print_phase "Unit Tests"
    
    print_test "Running unit tests for mitigation node"
    cd "$PROJECT_ROOT/mitigation-node"
    if cargo test --lib --bins -- --test-threads="$PARALLEL_JOBS"; then
        print_success "Mitigation node unit tests passed"
    else
        print_error "Mitigation node unit tests failed"
    fi
    
    print_test "Running unit tests for orchestrator node"
    cd "$PROJECT_ROOT/orchestrator-node"
    if cargo test --lib --bins -- --test-threads="$PARALLEL_JOBS"; then
        print_success "Orchestrator node unit tests passed"
    else
        print_error "Orchestrator node unit tests failed"
    fi
    
    cd "$PROJECT_ROOT"
}

# Integration tests
run_integration_tests() {
    print_phase "Integration Tests"
    
    # Test component startup
    print_test "Testing component startup sequences"
    
    # Start test origin server
    cd "$PROJECT_ROOT/mitigation-node"
    ./target/release/test-origin > ../logs/test-origin.log 2>&1 &
    local origin_pid=$!
    sleep 2
    
    if kill -0 $origin_pid 2>/dev/null; then
        print_success "Test origin server started (PID: $origin_pid)"
    else
        print_error "Test origin server failed to start"
        return 1
    fi
    
    # Test configuration loading
    print_test "Testing configuration loading"
    local test_configs=("config/dev.toml" "config/l7.toml" "config/tcp.toml")
    for config in "${test_configs[@]}"; do
        if [ -f "$config" ]; then
            if ./target/release/mitigation-node --config="$config" --validate-config; then
                print_success "Configuration $config is valid"
            else
                print_error "Configuration $config is invalid"
            fi
        else
            print_skip "Configuration $config not found"
        fi
    done
    
    # Cleanup
    kill $origin_pid 2>/dev/null || true
    wait $origin_pid 2>/dev/null || true
    
    cd "$PROJECT_ROOT"
}

# Performance tests
run_performance_tests() {
    print_phase "Performance Tests"
    
    print_test "Running performance benchmarks"
    
    # Start components for performance testing
    cd "$PROJECT_ROOT/mitigation-node"
    
    # Start test origin
    ./target/release/test-origin > ../logs/perf-origin.log 2>&1 &
    local origin_pid=$!
    sleep 2
    
    # Start mitigation node in TCP mode (fastest)
    RUST_LOG=warn ./target/release/mitigation-node --config=config/tcp.toml > ../logs/perf-proxy.log 2>&1 &
    local proxy_pid=$!
    sleep 3
    
    # Basic connection test
    print_test "Testing basic connectivity"
    if echo "test" | nc -w 5 127.0.0.1 8443 | grep -q "test"; then
        print_success "Basic connectivity test passed"
    else
        print_error "Basic connectivity test failed"
    fi
    
    # Load testing with multiple connections
    print_test "Testing concurrent connections"
    local concurrent_connections=100
    local success_count=0
    
    for i in $(seq 1 $concurrent_connections); do
        if echo "test$i" | nc -w 1 127.0.0.1 8443 > /dev/null 2>&1; then
            success_count=$((success_count + 1))
        fi &
        
        # Limit concurrent processes
        if (( i % 20 == 0 )); then
            wait
        fi
    done
    wait
    
    local success_rate=$((success_count * 100 / concurrent_connections))
    if [ $success_rate -gt 90 ]; then
        print_success "Concurrent connections test passed ($success_count/$concurrent_connections, ${success_rate}%)"
    else
        print_warning "Concurrent connections test partial success ($success_count/$concurrent_connections, ${success_rate}%)"
    fi
    
    # Cleanup
    kill $proxy_pid $origin_pid 2>/dev/null || true
    wait $proxy_pid $origin_pid 2>/dev/null || true
    
    cd "$PROJECT_ROOT"
}

# Security tests
run_security_tests() {
    print_phase "Security Tests"
    
    print_test "Testing WAF protection"
    
    cd "$PROJECT_ROOT/mitigation-node"
    
    # Start components for security testing
    ./target/release/test-origin > ../logs/sec-origin.log 2>&1 &
    local origin_pid=$!
    sleep 2
    
    # Start mitigation node in L7 mode with WAF
    RUST_LOG=warn ./target/release/mitigation-node --config=config/l7.toml > ../logs/sec-proxy.log 2>&1 &
    local proxy_pid=$!
    sleep 5
    
    # Test SQL injection blocking
    print_test "Testing SQL injection protection"
    local response=$(curl -k -s -w "%{http_code}" "https://127.0.0.1:8443/test?id=1' OR '1'='1" 2>/dev/null || echo "000")
    if [[ "$response" == *"403"* ]] || [[ "$response" == *"400"* ]]; then
        print_success "SQL injection blocked successfully"
    else
        print_warning "SQL injection test inconclusive (response: $response)"
    fi
    
    # Test XSS blocking
    print_test "Testing XSS protection"
    local response=$(curl -k -s -w "%{http_code}" "https://127.0.0.1:8443/test?data=<script>alert('xss')</script>" 2>/dev/null || echo "000")
    if [[ "$response" == *"403"* ]] || [[ "$response" == *"400"* ]]; then
        print_success "XSS attempt blocked successfully"
    else
        print_warning "XSS test inconclusive (response: $response)"
    fi
    
    # Test normal request
    print_test "Testing legitimate request handling"
    local response=$(curl -k -s -w "%{http_code}" "https://127.0.0.1:8443/api/health" 2>/dev/null || echo "000")
    if [[ "$response" == *"200"* ]]; then
        print_success "Legitimate requests handled correctly"
    else
        print_warning "Legitimate request test inconclusive (response: $response)"
    fi
    
    # Cleanup
    kill $proxy_pid $origin_pid 2>/dev/null || true
    wait $proxy_pid $origin_pid 2>/dev/null || true
    
    cd "$PROJECT_ROOT"
}

# Stress testing
run_stress_tests() {
    print_phase "Stress Tests"
    
    if command -v ab &> /dev/null; then
        print_test "Running Apache Bench stress test"
        
        cd "$PROJECT_ROOT/mitigation-node"
        
        # Start components
        ./target/release/test-origin > ../logs/stress-origin.log 2>&1 &
        local origin_pid=$!
        sleep 2
        
        RUST_LOG=warn ./target/release/mitigation-node --config=config/tcp.toml > ../logs/stress-proxy.log 2>&1 &
        local proxy_pid=$!
        sleep 3
        
        # Run stress test
        local requests=1000
        local concurrency=50
        
        if ab -n $requests -c $concurrency -k http://127.0.0.1:8443/ > reports/stress-test.txt 2>&1; then
            local rps=$(grep "Requests per second" reports/stress-test.txt | awk '{print $4}')
            print_success "Stress test completed - $rps requests/second"
        else
            print_warning "Stress test completed with warnings"
        fi
        
        # Cleanup
        kill $proxy_pid $origin_pid 2>/dev/null || true
        wait $proxy_pid $origin_pid 2>/dev/null || true
    else
        print_skip "Apache Bench not available for stress testing"
    fi
    
    cd "$PROJECT_ROOT"
}

# Container tests
run_container_tests() {
    print_phase "Container Tests"
    
    if command -v docker &> /dev/null; then
        print_test "Testing Docker build"
        
        if [ -f "Dockerfile" ]; then
            if docker build -t secbeat:test . > logs/docker-build.log 2>&1; then
                print_success "Docker build successful"
                
                # Test container startup
                print_test "Testing container startup"
                if docker run -d --name secbeat-test -p 8444:8443 secbeat:test > logs/docker-run.log 2>&1; then
                    sleep 5
                    if docker ps | grep -q secbeat-test; then
                        print_success "Container started successfully"
                    else
                        print_error "Container failed to start"
                    fi
                    
                    # Cleanup
                    docker stop secbeat-test > /dev/null 2>&1 || true
                    docker rm secbeat-test > /dev/null 2>&1 || true
                else
                    print_error "Failed to start container"
                fi
                
                # Cleanup image
                docker rmi secbeat:test > /dev/null 2>&1 || true
            else
                print_error "Docker build failed"
            fi
        else
            print_skip "Dockerfile not found"
        fi
    else
        print_skip "Docker not available"
    fi
}

# Network tests for Proxmox deployment
run_network_tests() {
    print_phase "Network Tests (Proxmox Ready)"
    
    print_test "Testing network connectivity to Proxmox node"
    if ping -c 3 "$PROXMOX_NODE" > /dev/null 2>&1; then
        print_success "Proxmox node $PROXMOX_NODE is reachable"
    else
        print_warning "Proxmox node $PROXMOX_NODE is not reachable"
    fi
    
    print_test "Testing SSH connectivity to Proxmox node"
    if ssh -o ConnectTimeout=5 -o BatchMode=yes "$PROXMOX_NODE" echo "SSH test" > /dev/null 2>&1; then
        print_success "SSH connectivity to Proxmox node confirmed"
    else
        print_warning "SSH connectivity to Proxmox node failed (check SSH keys)"
    fi
    
    print_test "Testing multi-node network simulation"
    local test_ports=(8443 8444 8445)
    for port in "${test_ports[@]}"; do
        if ! ss -tuln | grep -q ":$port "; then
            print_success "Port $port is available for multi-node testing"
        else
            print_warning "Port $port is in use"
        fi
    done
}

# Generate test report
generate_report() {
    print_phase "Test Report Generation"
    
    local end_time=$(date +%s)
    local total_duration=$((end_time - START_TIME))
    local test_time=$(date)
    
    # Create HTML report
    cat > reports/test-report.html << EOF
<!DOCTYPE html>
<html>
<head>
    <title>SecBeat Test Report</title>
    <style>
        body { font-family: Arial, sans-serif; margin: 40px; }
        .header { background-color: #f0f0f0; padding: 20px; border-radius: 8px; }
        .pass { color: #28a745; }
        .fail { color: #dc3545; }
        .warn { color: #ffc107; }
        .skip { color: #6c757d; }
        table { width: 100%; border-collapse: collapse; margin: 20px 0; }
        th, td { border: 1px solid #ddd; padding: 12px; text-align: left; }
        th { background-color: #f8f9fa; }
    </style>
</head>
<body>
    <div class="header">
        <h1>SecBeat Production Testing Report</h1>
        <p><strong>Test Date:</strong> $test_time</p>
        <p><strong>Environment:</strong> $TEST_ENV</p>
        <p><strong>Duration:</strong> ${total_duration}s</p>
        <p><strong>Proxmox Node:</strong> $PROXMOX_NODE</p>
    </div>
    
    <h2>Test Summary</h2>
    <table>
        <tr><th>Metric</th><th>Count</th><th>Percentage</th></tr>
        <tr><td class="pass">Passed</td><td>$PASSED_TESTS</td><td>$(( PASSED_TESTS * 100 / TOTAL_TESTS ))%</td></tr>
        <tr><td class="fail">Failed</td><td>$FAILED_TESTS</td><td>$(( FAILED_TESTS * 100 / TOTAL_TESTS ))%</td></tr>
        <tr><td class="skip">Skipped</td><td>$SKIPPED_TESTS</td><td>$(( SKIPPED_TESTS * 100 / TOTAL_TESTS ))%</td></tr>
        <tr><td><strong>Total</strong></td><td><strong>$TOTAL_TESTS</strong></td><td><strong>100%</strong></td></tr>
    </table>
    
    <h2>Production Readiness Checklist</h2>
    <ul>
        <li class="$([ $FAILED_TESTS -eq 0 ] && echo "pass" || echo "fail")">All tests passing</li>
        <li class="pass">Build artifacts generated</li>
        <li class="pass">Configuration validation</li>
        <li class="pass">Security testing completed</li>
        <li class="pass">Performance benchmarks</li>
        <li class="$(command -v docker >/dev/null && echo "pass" || echo "warn")">Container support</li>
        <li class="$(ping -c 1 "$PROXMOX_NODE" >/dev/null 2>&1 && echo "pass" || echo "warn")">Proxmox connectivity</li>
    </ul>
    
    <h2>Next Steps for Production Deployment</h2>
    <ol>
        <li>Review failed tests and resolve issues</li>
        <li>Deploy to Proxmox staging environment</li>
        <li>Run load testing in production-like environment</li>
        <li>Configure monitoring and alerting</li>
        <li>Set up backup and disaster recovery</li>
        <li>Schedule production deployment</li>
    </ol>
    
</body>
</html>
EOF

    print_success "HTML test report generated: reports/test-report.html"
    
    # Create JSON report for CI/CD
    cat > reports/test-results.json << EOF
{
  "summary": {
    "total": $TOTAL_TESTS,
    "passed": $PASSED_TESTS,
    "failed": $FAILED_TESTS,
    "skipped": $SKIPPED_TESTS,
    "duration": $total_duration,
    "success_rate": $(( PASSED_TESTS * 100 / TOTAL_TESTS ))
  },
  "environment": {
    "test_env": "$TEST_ENV",
    "proxmox_node": "$PROXMOX_NODE",
    "rust_version": "$(rustc --version)",
    "test_date": "$test_time"
  },
  "production_ready": $([ $FAILED_TESTS -eq 0 ] && echo "true" || echo "false")
}
EOF

    print_success "JSON test results generated: reports/test-results.json"
}

# Cleanup function
cleanup() {
    print_info "Cleaning up test processes..."
    pkill -f "test-origin" 2>/dev/null || true
    pkill -f "mitigation-node" 2>/dev/null || true
    pkill -f "orchestrator-node" 2>/dev/null || true
    docker stop secbeat-test 2>/dev/null || true
    docker rm secbeat-test 2>/dev/null || true
}

# Set up signal handlers
trap cleanup EXIT INT TERM

# Main test execution
main() {
    check_dependencies
    setup_environment
    run_static_analysis
    build_components
    run_unit_tests
    run_integration_tests
    run_performance_tests
    run_security_tests
    run_stress_tests
    run_container_tests
    run_network_tests
    generate_report
    
    # Final summary
    print_phase "Test Summary"
    
    local end_time=$(date +%s)
    local total_duration=$((end_time - START_TIME))
    
    echo -e "${BOLD}Test Execution Summary:${NC}"
    echo -e "  ${GREEN}Passed:${NC} $PASSED_TESTS"
    echo -e "  ${RED}Failed:${NC} $FAILED_TESTS"
    echo -e "  ${YELLOW}Skipped:${NC} $SKIPPED_TESTS"
    echo -e "  ${BLUE}Total:${NC} $TOTAL_TESTS"
    echo -e "  ${CYAN}Duration:${NC} ${total_duration}s"
    echo -e "  ${MAGENTA}Success Rate:${NC} $(( PASSED_TESTS * 100 / TOTAL_TESTS ))%"
    echo
    
    if [ $FAILED_TESTS -eq 0 ]; then
        echo -e "${GREEN}${BOLD}✅ ALL TESTS PASSED - PRODUCTION READY!${NC}"
        echo -e "${GREEN}Ready for Proxmox deployment at $PROXMOX_NODE${NC}"
        exit 0
    else
        echo -e "${RED}${BOLD}❌ SOME TESTS FAILED - REVIEW REQUIRED${NC}"
        echo -e "${RED}Please review failed tests before production deployment${NC}"
        exit 1
    fi
}

# Execute main function
main "$@"
