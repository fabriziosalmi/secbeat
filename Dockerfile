# SecBeat Mitigation Node - Production Docker Image
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
COPY mitigation-node/ ./mitigation-node/
COPY orchestrator-node/ ./orchestrator-node/

# Build the mitigation node in release mode
RUN cargo build --release --bin mitigation-node --bin test-origin

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

# Copy binaries from builder
COPY --from=builder /app/target/release/mitigation-node /usr/local/bin/
COPY --from=builder /app/target/release/test-origin /usr/local/bin/

# Copy configuration files
COPY --chown=secbeat:secbeat config.prod.toml ./config.prod.toml
COPY --chown=secbeat:secbeat config.dev.toml ./config.dev.toml
COPY --chown=secbeat:secbeat mitigation-node/config/ ./config/

# Copy certificates (if they exist)
COPY --chown=secbeat:secbeat mitigation-node/certs/ ./certs/

# Create logs directory
RUN mkdir -p logs && chown secbeat:secbeat logs

# Switch to non-root user
USER secbeat

# Health check
HEALTHCHECK --interval=30s --timeout=10s --start-period=30s --retries=3 \
    CMD curl -f http://localhost:9191/metrics || exit 1

# Default configuration
ENV SECBEAT_CONFIG=config.prod
ENV RUST_LOG=info
ENV RUST_BACKTRACE=1

# Expose ports
EXPOSE 8443 9090 9191 9999

# Default command
CMD ["mitigation-node", "--config", "config.prod.toml"]
