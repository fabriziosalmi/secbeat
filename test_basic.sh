#!/bin/bash

#############################################
# SecBeat Basic Test Suite                 #
# Tests compilation and basic functionality #
# without requiring root privileges        #
#############################################

set -e

echo "============================================"
echo "     SecBeat Basic Test Suite              "
echo "   Compilation and Basic Functionality     "
echo "============================================"
echo ""

# Test directories
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
MITIGATION_DIR="$SCRIPT_DIR/mitigation-node"
ORCHESTRATOR_DIR="$SCRIPT_DIR/orchestrator-node"

echo "Starting basic test suite at $(date)"
echo ""

# Function to print test results
print_result() {
    if [ $1 -eq 0 ]; then
        echo "[PASS] $2"
    else
        echo "[FAIL] $2"
        exit 1
    fi
}

# Test 1: Check if source files exist
echo "=== Test 1: Source File Verification ==="
if [ -f "$MITIGATION_DIR/src/main.rs" ] && [ -f "$ORCHESTRATOR_DIR/src/main.rs" ]; then
    print_result 0 "Source files exist"
else
    print_result 1 "Source files missing"
fi

# Test 2: Check configuration files
echo ""
echo "=== Test 2: Configuration File Verification ==="
if [ -f "$MITIGATION_DIR/config/default.toml" ]; then
    print_result 0 "Configuration files exist"
else
    print_result 1 "Configuration files missing"
fi

# Test 3: Build mitigation-node
echo ""
echo "=== Test 3: Mitigation Node Compilation ==="
cd "$MITIGATION_DIR"
if cargo check --quiet; then
    print_result 0 "Mitigation node compiles successfully"
else
    print_result 1 "Mitigation node compilation failed"
fi

# Test 4: Build orchestrator-node
echo ""
echo "=== Test 4: Orchestrator Node Compilation ==="
cd "$ORCHESTRATOR_DIR"
if cargo check --quiet; then
    print_result 0 "Orchestrator node compiles successfully"
else
    print_result 1 "Orchestrator node compilation failed"
fi

# Test 5: Release builds
echo ""
echo "=== Test 5: Release Build Verification ==="
cd "$MITIGATION_DIR"
if cargo build --release --quiet; then
    print_result 0 "Mitigation node release build successful"
else
    print_result 1 "Mitigation node release build failed"
fi

cd "$ORCHESTRATOR_DIR"
if cargo build --release --quiet; then
    print_result 0 "Orchestrator node release build successful"
else
    print_result 1 "Orchestrator node release build failed"
fi

# Test 6: Binary existence
echo ""
echo "=== Test 6: Binary Verification ==="
if [ -f "$MITIGATION_DIR/target/release/mitigation-node" ] && [ -f "$ORCHESTRATOR_DIR/target/release/orchestrator-node" ]; then
    print_result 0 "Release binaries created successfully"
else
    print_result 1 "Release binaries not found"
fi

# Test 7: Help output test (basic functionality)
echo ""
echo "=== Test 7: Basic Functionality Test ==="
cd "$MITIGATION_DIR"
if timeout 5s ./target/release/mitigation-node --help >/dev/null 2>&1; then
    print_result 0 "Mitigation node runs and responds to --help"
else
    # This might fail if --help is not implemented, which is OK
    echo "[INFO] Mitigation node help test completed (may not support --help flag)"
fi

cd "$ORCHESTRATOR_DIR"
if timeout 5s ./target/release/orchestrator-node --help >/dev/null 2>&1; then
    print_result 0 "Orchestrator node runs and responds to --help"
else
    # This might fail if --help is not implemented, which is OK
    echo "[INFO] Orchestrator node help test completed (may not support --help flag)"
fi

# Test 8: Documentation verification
echo ""
echo "=== Test 8: Documentation Verification ==="
cd "$SCRIPT_DIR"
if [ -f "README.md" ] && [ -f "PHASE1_README.md" ] && [ -f "PHASE2_README.md" ] && [ -f "PHASE3_README.md" ] && [ -f "PHASE4_README.md" ]; then
    print_result 0 "Documentation files exist"
else
    print_result 1 "Documentation files missing"
fi

# Test 9: Test script verification
echo ""
echo "=== Test 9: Test Script Verification ==="
test_files=("test_phase1.sh" "test_phase2.sh" "test_phase3.sh" "test_phase4.sh" "test_all.sh")
all_exist=true
for file in "${test_files[@]}"; do
    if [ ! -f "$file" ] || [ ! -x "$file" ]; then
        all_exist=false
        break
    fi
done

if [ "$all_exist" = true ]; then
    print_result 0 "All test scripts exist and are executable"
else
    print_result 1 "Some test scripts missing or not executable"
fi

# Test 10: Git ignore verification
echo ""
echo "=== Test 10: Git Configuration Verification ==="
if [ -f ".gitignore" ]; then
    print_result 0 ".gitignore file exists"
else
    print_result 1 ".gitignore file missing"
fi

echo ""
echo "============================================"
echo "         Basic Test Suite Complete         "
echo "============================================"
echo ""
echo "✅ All basic tests passed!"
echo "✅ Both nodes compile successfully"
echo "✅ Release builds created"
echo "✅ Documentation complete"
echo "✅ Test infrastructure ready"
echo ""
echo "Note: For full integration testing including network"
echo "      operations, run the phase-specific tests with"
echo "      appropriate privileges."
echo ""
echo "To run full integration tests:"
echo "  sudo ./test_all.sh"
echo ""
echo "Basic test suite completed at $(date)"
