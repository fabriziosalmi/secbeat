# SecBeat Orchestrator Node - Production Docker Image
FROM rust:1.88-slim AS builder

# Install system dependencies for building
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

# Set working directory
WORKDIR /app

# Copy workspace configuration
COPY Cargo.toml Cargo.lock ./

# Copy source code
COPY orchestrator-node/ ./orchestrator-node/
COPY mitigation-node/ ./mitigation-node/
COPY secbeat-common/ ./secbeat-common/
COPY secbeat-ebpf/ ./secbeat-ebpf/

# Build the orchestrator node in release mode
RUN cargo build --release --bin orchestrator-node

# Runtime stage
FROM debian:bookworm-slim

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    curl \
    && rm -rf /var/lib/apt/lists/*

# Create user for security
RUN useradd --create-home --shell /bin/bash --user-group secbeat

# Set working directory
WORKDIR /app

# Copy binary from builder
COPY --from=builder /app/target/release/orchestrator-node /usr/local/bin/

# Create logs directory
RUN mkdir -p logs && chown secbeat:secbeat logs

# Switch to non-root user
USER secbeat

# Health check
HEALTHCHECK --interval=30s --timeout=10s --start-period=30s --retries=3 \
    CMD curl -f http://localhost:9090/health || exit 1

# Default configuration
ENV RUST_LOG=info
ENV RUST_BACKTRACE=1

# Expose ports
EXPOSE 9090

# Default command
CMD ["orchestrator-node"]
