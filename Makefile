# SecBeat Makefile
# Build and management tasks for the SecBeat project

.PHONY: all build clean test help install deps check fmt clippy doc release

# Default target
all: build

# Build all components
build:
	@echo "Building all SecBeat components..."
	cargo build --workspace

# Build release version
release:
	@echo "Building release version..."
	cargo build --workspace --release

# Run all tests
test:
	@echo "Running all tests..."
	cargo test --workspace
	@echo "Running comprehensive integration tests..."
	sudo ./test_comprehensive.sh

# Clean build artifacts
clean:
	@echo "Cleaning build artifacts..."
	cargo clean

# Check code without building
check:
	@echo "Checking code..."
	cargo check --workspace

# Format code
fmt:
	@echo "Formatting code..."
	cargo fmt --all

# Run clippy linter
clippy:
	@echo "Running clippy..."
	cargo clippy --workspace --all-targets --all-features -- -D warnings

# Generate documentation
doc:
	@echo "Generating documentation..."
	cargo doc --workspace --no-deps --open

# Install dependencies (Ubuntu/Debian)
deps-ubuntu:
	@echo "Installing dependencies for Ubuntu/Debian..."
	sudo apt update
	sudo apt install -y build-essential pkg-config libssl-dev curl jq

# Install dependencies (macOS)
deps-macos:
	@echo "Installing dependencies for macOS..."
	brew install openssl curl jq

# Setup TLS certificates for testing
setup-certs:
	@echo "Setting up TLS certificates..."
	mkdir -p mitigation-node/certs
	openssl req -x509 -newkey rsa:4096 \
		-keyout mitigation-node/certs/key.pem \
		-out mitigation-node/certs/cert.pem \
		-days 365 -nodes \
		-subj "/CN=localhost"
	@echo "Certificates created in mitigation-node/certs/"

# Run individual phase tests
test-phase1:
	sudo ./test_phase1.sh

test-phase2:
	sudo ./test_phase2.sh

test-phase3:
	sudo ./test_phase3.sh

test-phase4:
	sudo ./test_phase4.sh

test-phase5:
	sudo ./test_phase5.sh

test-phase6:
	sudo ./test_phase6.sh

test-phase7:
	sudo ./test_phase7.sh

# Help target
help:
	@echo "Available targets:"
	@echo "  build        - Build all components (debug)"
	@echo "  release      - Build all components (release)"
	@echo "  test         - Run all tests"
	@echo "  clean        - Clean build artifacts"
	@echo "  check        - Check code without building"
	@echo "  fmt          - Format code"
	@echo "  clippy       - Run clippy linter"
	@echo "  doc          - Generate documentation"
	@echo "  setup-certs  - Setup TLS certificates for testing"
	@echo "  deps-ubuntu  - Install dependencies (Ubuntu/Debian)"
	@echo "  deps-macos   - Install dependencies (macOS)"
	@echo "  test-phase*  - Run individual phase tests"
	@echo "  help         - Show this help"
