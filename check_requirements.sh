#!/bin/bash
# SecBeat eBPF Requirements Checker
# Verifies system requirements for eBPF/XDP development

set -e

GREEN='\033[0;32m'
RED='\033[0;31m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

echo -e "${BLUE}╔════════════════════════════════════════════════════════════╗${NC}"
echo -e "${BLUE}║  SecBeat eBPF/XDP Requirements Check                      ║${NC}"
echo -e "${BLUE}╚════════════════════════════════════════════════════════════╝${NC}"
echo ""

ALL_PASSED=true

# 1. Check kernel version
echo -e "${YELLOW}[1/6] Checking Linux Kernel Version...${NC}"
if [[ "$OSTYPE" != "linux-gnu"* ]]; then
    echo -e "${YELLOW}⚠️  Not running on Linux (detected: $OSTYPE)${NC}"
    echo "   eBPF/XDP requires Linux kernel. Development on macOS is limited to compilation only."
    echo "   You can build eBPF programs but cannot load/test them."
    ALL_PASSED=false
else
    KERNEL_VERSION=$(uname -r | cut -d'.' -f1,2)
    KERNEL_MAJOR=$(echo $KERNEL_VERSION | cut -d'.' -f1)
    KERNEL_MINOR=$(echo $KERNEL_VERSION | cut -d'.' -f2)
    
    echo "   Current kernel: $(uname -r)"
    
    if [ "$KERNEL_MAJOR" -ge 5 ] && [ "$KERNEL_MINOR" -ge 10 ]; then
        echo -e "   ${GREEN}✅ Kernel 5.10+ detected (good for Aya)${NC}"
    else
        echo -e "   ${YELLOW}⚠️  Kernel older than 5.10, some features may not work${NC}"
        echo "   Recommended: Upgrade to kernel 5.10 or newer"
        ALL_PASSED=false
    fi
fi
echo ""

# 2. Check Rust installation
echo -e "${YELLOW}[2/6] Checking Rust Installation...${NC}"
if ! command -v rustc &> /dev/null; then
    echo -e "${RED}❌ Rust not installed!${NC}"
    echo "   Install from: https://rustup.rs/"
    ALL_PASSED=false
else
    RUST_VERSION=$(rustc --version)
    echo "   $RUST_VERSION"
    echo -e "   ${GREEN}✅ Rust installed${NC}"
fi
echo ""

# 3. Check for nightly toolchain
echo -e "${YELLOW}[3/6] Checking Rust Nightly Toolchain...${NC}"
if rustup toolchain list | grep -q nightly; then
    NIGHTLY_VERSION=$(rustup run nightly rustc --version)
    echo "   $NIGHTLY_VERSION"
    echo -e "   ${GREEN}✅ Nightly toolchain available${NC}"
else
    echo -e "${RED}❌ Nightly toolchain not installed!${NC}"
    echo "   Installing nightly toolchain..."
    rustup install nightly
    echo -e "   ${GREEN}✅ Nightly toolchain installed${NC}"
fi
echo ""

# 4. Check for rust-src component
echo -e "${YELLOW}[4/6] Checking rust-src Component...${NC}"
if rustup component list --toolchain nightly | grep -q "rust-src (installed)"; then
    echo -e "   ${GREEN}✅ rust-src component installed${NC}"
else
    echo -e "${YELLOW}⚠️  rust-src not installed${NC}"
    echo "   Installing rust-src component..."
    rustup component add rust-src --toolchain nightly
    echo -e "   ${GREEN}✅ rust-src component installed${NC}"
fi
echo ""

# 5. Check for bpf-linker
echo -e "${YELLOW}[5/6] Checking bpf-linker...${NC}"
if command -v bpf-linker &> /dev/null; then
    BPF_LINKER_VERSION=$(bpf-linker --version)
    echo "   $BPF_LINKER_VERSION"
    echo -e "   ${GREEN}✅ bpf-linker installed${NC}"
else
    echo -e "${RED}❌ bpf-linker not installed!${NC}"
    echo "   Install with: cargo install bpf-linker"
    echo "   Note: This may take several minutes"
    ALL_PASSED=false
fi
echo ""

# 6. Check for LLVM (optional but recommended)
echo -e "${YELLOW}[6/6] Checking LLVM Installation...${NC}"
if command -v llvm-objdump &> /dev/null; then
    LLVM_VERSION=$(llvm-objdump --version | head -n1)
    echo "   $LLVM_VERSION"
    echo -e "   ${GREEN}✅ LLVM installed (good for debugging)${NC}"
else
    echo -e "${YELLOW}⚠️  LLVM not found (optional)${NC}"
    echo "   Install for better debugging: apt install llvm (Ubuntu) or brew install llvm (macOS)"
fi
echo ""

# Summary
echo -e "${BLUE}════════════════════════════════════════════════════════════${NC}"
if [ "$ALL_PASSED" = true ]; then
    echo -e "${GREEN}✅ All critical requirements met!${NC}"
    echo ""
    echo "You can now build eBPF programs with:"
    echo "  ./build_ebpf.sh"
    echo ""
    if [[ "$OSTYPE" != "linux-gnu"* ]]; then
        echo -e "${YELLOW}Note: To load and test XDP programs, you need a Linux system.${NC}"
        echo "      You can still compile on macOS and deploy to Linux."
    fi
elif [[ "$OSTYPE" != "linux-gnu"* ]] && command -v bpf-linker &> /dev/null; then
    echo -e "${GREEN}✅ Development environment ready (compilation only)!${NC}"
    echo ""
    echo "You can build eBPF programs with:"
    echo "  ./build_ebpf.sh"
    echo ""
    echo -e "${YELLOW}Note: Running on macOS - compilation supported but not runtime loading.${NC}"
    echo "      Deploy to Linux for actual XDP testing."
    exit 0
else
    echo -e "${RED}❌ Some requirements are missing.${NC}"
    echo "Please install missing components before proceeding."
    exit 1
fi
echo -e "${BLUE}════════════════════════════════════════════════════════════${NC}"
