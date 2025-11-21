#!/bin/bash
# SecBeat eBPF Build Script
# Compiles the eBPF kernel program using bpf-linker

set -e

GREEN='\033[0;32m'
RED='\033[0;31m'
YELLOW='\033[1;33m'
NC='\033[0m'

echo -e "${YELLOW}🔨 Building SecBeat eBPF Program${NC}"

# Configure LLVM on macOS
if [[ "$OSTYPE" == "darwin"* ]]; then
    if [[ -d "/opt/homebrew/opt/llvm" ]]; then
        export LLVM_SYS_210_PREFIX="/opt/homebrew/opt/llvm"
        export PATH="/opt/homebrew/opt/llvm/bin:$PATH"
        echo -e "${GREEN}✅ LLVM configured: $LLVM_SYS_210_PREFIX${NC}"
    elif [[ -d "/usr/local/opt/llvm" ]]; then
        export LLVM_SYS_210_PREFIX="/usr/local/opt/llvm"
        export PATH="/usr/local/opt/llvm/bin:$PATH"
        echo -e "${GREEN}✅ LLVM configured: $LLVM_SYS_210_PREFIX${NC}"
    else
        echo -e "${RED}⚠️  LLVM not found at expected locations${NC}"
        echo "Install with: brew install llvm"
    fi
fi

# Check if bpf-linker is installed
if ! command -v bpf-linker &> /dev/null; then
    echo -e "${RED}❌ bpf-linker not found!${NC}"
    echo "Installing bpf-linker..."
    cargo install bpf-linker
fi

# Check if we're using nightly toolchain
RUSTUP_TOOLCHAIN=$(rustup show active-toolchain | cut -d' ' -f1)
if [[ ! "$RUSTUP_TOOLCHAIN" =~ "nightly" ]]; then
    echo -e "${YELLOW}⚠️  Nightly toolchain not active, switching...${NC}"
    rustup default nightly
fi

# Determine target architecture
ARCH=$(uname -m)
if [ "$ARCH" == "x86_64" ]; then
    TARGET="bpfel-unknown-none"
elif [ "$ARCH" == "aarch64" ]; then
    TARGET="bpfel-unknown-none"  # ARM is also little-endian
else
    echo -e "${YELLOW}⚠️  Unknown architecture: $ARCH, defaulting to bpfel-unknown-none${NC}"
    TARGET="bpfel-unknown-none"
fi

echo "Architecture: $ARCH"
echo "Target: $TARGET"
echo ""

# Add rust-src component if not present
rustup component add rust-src

# Build the eBPF program
echo -e "${YELLOW}Building secbeat-ebpf for $TARGET...${NC}"
cargo build \
    --package secbeat-ebpf \
    --release \
    --target $TARGET \
    -Z build-std=core

# Copy the built binary to a known location
mkdir -p target/bpf
cp target/$TARGET/release/secbeat-ebpf target/bpf/secbeat-ebpf

echo ""
echo -e "${GREEN}✅ eBPF program built successfully!${NC}"
echo "Output: target/bpf/secbeat-ebpf"
echo ""
echo "File info:"
file target/bpf/secbeat-ebpf
ls -lh target/bpf/secbeat-ebpf
