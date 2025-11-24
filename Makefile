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
	@echo "Note: Use 'make test-docker' for containerized tests with Linux capabilities"
	./test_comprehensive.sh

# Run tests in Docker container (with CAP_NET_ADMIN for XDP/eBPF)
test-docker:
	@echo "Building test container..."
	docker build -t secbeat-test -f Dockerfile.test .
	@echo "Running tests in container with CAP_NET_ADMIN..."
	docker run --rm \
		--cap-add=NET_ADMIN \
		--cap-add=NET_RAW \
		-v $(PWD):/workspace \
		-e RUST_LOG=$(RUST_LOG) \
		secbeat-test

# Run tests in Docker with interactive shell (for debugging)
test-docker-shell:
	@echo "Starting interactive test container..."
	docker run -it --rm \
		--cap-add=NET_ADMIN \
		--cap-add=NET_RAW \
		-v $(PWD):/workspace \
		-e RUST_LOG=$(RUST_LOG) \
		secbeat-test bash

# Build test container only
test-docker-build:
	@echo "Building test container..."
	docker build -t secbeat-test -f Dockerfile.test .

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

# Behavioral analysis tests
test-behavioral:
	@echo "Running behavioral analysis end-to-end test..."
	./test_behavioral_ban.sh

test-behavioral-quick:
	@echo "Running quick behavioral ban test..."
	./test_behavioral_quick.sh

# Help target
help:
	@echo "Available targets:"
	@echo "  build              - Build all components (debug)"
	@echo "  release            - Build all components (release)"
	@echo "  test               - Run all tests (requires sudo for XDP tests)"
	@echo "  test-docker        - Run tests in Docker with CAP_NET_ADMIN (no sudo)"
	@echo "  test-docker-shell  - Interactive test container for debugging"
	@echo "  test-docker-build  - Build test container only"
	@echo "  clean              - Clean build artifacts"
	@echo "  check              - Check code without building"
	@echo "  fmt                - Format code"
	@echo "  clippy             - Run clippy linter"
	@echo "  doc                - Generate documentation"
	@echo "  setup-certs        - Setup TLS certificates for testing"
	@echo "  deps-ubuntu        - Install dependencies (Ubuntu/Debian)"
	@echo "  deps-macos         - Install dependencies (macOS)"
	@echo "  test-phase*        - Run individual phase tests"
	@echo "  test-behavioral    - Run behavioral analysis E2E test"
	@echo "  test-behavioral-quick - Run quick behavioral ban test"
	@echo "  help               - Show this help"
