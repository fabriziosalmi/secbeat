# SecBeat

**A Rust-based DDoS mitigation and Web Application Firewall platform**

[![Documentation](https://img.shields.io/badge/📚_Documentation-Visit_Site-blue?style=for-the-badge)](https://fabriziosalmi.github.io/secbeat)
[![Rust](https://img.shields.io/badge/rust-1.78+-93450a.svg?style=flat-square)](https://www.rust-lang.org)
[![License](https://img.shields.io/badge/license-MIT-blue.svg?style=flat-square)](LICENSE)

> 🌐 **[View Full Documentation →](https://fabriziosalmi.github.io/secbeat)**

SecBeat is a distributed security platform built in Rust that provides DDoS mitigation and Web Application Firewall capabilities. The project implements a "smart edge, intelligent orchestrator" architecture where mitigation nodes handle traffic processing while a central orchestrator provides coordination and intelligence.

**Current Status:** Early development (v0.1.0) - Not recommended for production use

## Quick Start

```bash
# Clone the repository
git clone https://github.com/fabriziosalmi/secbeat.git
cd secbeat

# Build (requires Rust 1.78+)
cargo build --release --workspace

# Start services
docker-compose up -d

# Test the deployment
curl -k https://localhost:8443/
```

## What Works Today

### Core Functionality
- **TCP Proxy**: Async reverse proxy with TLS termination (Tokio/Rustls)
- **WAF Engine**: ~100 regex-based attack patterns for SQL injection, XSS, path traversal, and command injection
- **HTTP/HTTPS**: TLS 1.2/1.3 support with certificate management
- **Metrics**: Prometheus-compatible metrics endpoint
- **Management API**: Basic health, status, and configuration endpoints

### Distributed Features
- **NATS Messaging**: Real-time event stream between nodes
- **Fleet Management**: Orchestrator tracks and coordinates mitigation nodes
- **Dynamic Rules**: Hot-reload of WAF rules and IP blacklists
- **Behavioral Analysis**: Sliding window anomaly detection with automated blocking

### ML/AI Capabilities
- **Anomaly Detection**: Random Forest classifier for traffic anomaly detection (smartcore)
- **Behavioral Expert**: Pattern-based analysis with configurable thresholds
- **Resource Manager**: Linear regression for CPU usage prediction

### Experimental Features
- **WASM Runtime**: WebAssembly-based WAF rules (Wasmtime) - functional but basic
- **eBPF/XDP**: Kernel-level packet processing for SYN flood mitigation - experimental, Linux only
- **SYN Proxy**: Basic SYN cookie implementation - prototype with limitations

## Architecture

```
┌─────────────────────────────────────┐
│      Orchestrator Node              │
│  • Fleet Management                 │
│  • ML-based Anomaly Detection       │
│  • Resource Optimization            │
│  • Policy Distribution              │
└─────────────────────────────────────┘
                 │
    ┌────────────┴────────────┐
    │   NATS Message Bus      │
    └────────────┬────────────┘
                 │
    ┌────────────┴────────────┐
    ▼            ▼            ▼
┌─────────┐  ┌─────────┐  ┌─────────┐
│ Mitigation │ Mitigation │ Mitigation │
│  Node 1    │  Node 2    │  Node N    │
│            │            │            │
│ • TCP Proxy│ • TCP Proxy│ • TCP Proxy│
│ • WAF      │ • WAF      │ • WAF      │
│ • TLS      │ • TLS      │ • TLS      │
└─────────┘  └─────────┘  └─────────┘
     │            │            │
     ▼            ▼            ▼
  Backend     Backend     Backend
  Services    Services    Services
```

## Configuration

SecBeat uses TOML configuration files:

```toml
# Basic mitigation node configuration
[network]
listen_address = "0.0.0.0:8443"
upstream_address = "127.0.0.1:8080"

[tls]
enabled = true
cert_path = "certs/cert.pem"
key_path = "certs/key.pem"

[waf]
enabled = true
block_sql_injection = true
block_xss = true
block_path_traversal = true
block_command_injection = true

[metrics]
bind_address = "0.0.0.0:9191"
```

See [Configuration Reference](https://fabriziosalmi.github.io/secbeat/reference/config/) for complete options.

## Development Status

### ✅ Implemented
- [x] TCP reverse proxy (async Tokio)
- [x] TLS 1.2/1.3 termination
- [x] Basic WAF with regex patterns
- [x] NATS-based messaging
- [x] Prometheus metrics
- [x] Management API
- [x] Random Forest anomaly detection
- [x] Behavioral analysis engine
- [x] WASM rule execution (basic)
- [x] Docker deployment

### ⚠️ Experimental
- [ ] eBPF/XDP packet filtering (Linux only, requires CAP_NET_RAW)
- [ ] SYN proxy with cookie validation (prototype, not production-ready)
- [ ] CRDT-based state synchronization (partial implementation)

### 📋 In Development
- [ ] Complete threat intelligence API
- [ ] Enhanced statistics and reporting
- [ ] IP blacklist/whitelist persistence
- [ ] Comprehensive test suite
- [ ] Performance benchmarks
- [ ] Production deployment tooling

### 🔮 Planned
- [ ] HTTP/2 support
- [ ] OWASP ModSecurity CRS integration
- [ ] Advanced ML models (LSTM, Isolation Forest)
- [ ] Dashboard UI
- [ ] Multi-tenant support
- [ ] Cloud provider integrations

## Testing

```bash
# Run unit and integration tests
cargo test --workspace

# Run behavioral analysis test (requires Docker)
./test_behavioral_ban.sh

# Run integration tests
cd mitigation-node && cargo test --test integration_tests
```

**Note:** Many test scripts are present but may require adjustments for different environments.

## Deployment

### Docker (Development)

```bash
docker-compose up -d
```

### Kubernetes (Experimental)

```bash
kubectl apply -f k8s/
```

### Bare Metal

```bash
# Build release binaries
cargo build --release --workspace

# Install binaries
sudo cp target/release/mitigation-node /usr/local/bin/
sudo cp target/release/orchestrator-node /usr/local/bin/

# Set capabilities for SYN proxy (if using)
sudo setcap cap_net_raw,cap_net_admin+ep /usr/local/bin/mitigation-node
```

See [Installation Guide](https://fabriziosalmi.github.io/secbeat/installation/) for detailed instructions.

## Monitoring

SecBeat exposes Prometheus metrics on port 9191:

```bash
curl http://localhost:9191/metrics
```

Key metrics include:
- `secbeat_requests_total` - Total HTTP requests processed
- `secbeat_blocked_total` - Total blocked attacks
- `secbeat_response_time_seconds` - Request latency
- `secbeat_connections_active` - Active connections

## API Reference

### Management API (Mitigation Node)

```bash
# Health check
GET http://localhost:9999/api/v1/status

# Block an IP
POST http://localhost:9999/api/v1/blacklist
Content-Type: application/json
{
  "ip": "192.0.2.100",
  "duration_seconds": 3600
}
```

### Control API (Orchestrator)

```bash
# List fleet nodes
GET http://localhost:3030/api/v1/nodes

# Get node metrics
GET http://localhost:3030/api/v1/nodes/{id}/metrics
```

See [API Reference](https://fabriziosalmi.github.io/secbeat/reference/api/) for complete documentation.

## Known Limitations

1. **SYN Proxy**: Experimental only, not suitable for production
2. **eBPF/XDP**: Linux only, requires kernel 5.15+ and CAP_NET_RAW
3. **WASM Runtime**: Basic implementation, limited rule complexity
4. **Test Coverage**: Integration tests need environment-specific adjustments
5. **Documentation**: Some features documented but implementation incomplete
6. **Performance**: Not yet optimized for high-throughput scenarios
7. **Stability**: Early development, breaking changes expected

## Requirements

- **Rust**: 1.78 or later
- **Operating System**: Linux (recommended) or macOS for development
- **Linux Kernel**: 5.15+ for eBPF/XDP features
- **Memory**: 4GB+ RAM recommended
- **Privileges**: Root/CAP_NET_RAW for SYN proxy mode

## Contributing

This is an early-stage project. Contributions are welcome, but be aware of the current development status. Before contributing:

1. Review the [documentation](https://fabriziosalmi.github.io/secbeat)
2. Check existing issues and pull requests
3. Test your changes thoroughly
4. Follow Rust best practices

## Documentation

- [Quick Start Guide](https://fabriziosalmi.github.io/secbeat/quickstart/)
- [Installation](https://fabriziosalmi.github.io/secbeat/installation/)
- [Architecture Overview](https://fabriziosalmi.github.io/secbeat/core/overview/)
- [API Reference](https://fabriziosalmi.github.io/secbeat/reference/api/)
- [Configuration Reference](https://fabriziosalmi.github.io/secbeat/reference/config/)

## License

MIT License - see [LICENSE](LICENSE) for details.

## Acknowledgments

Built with:
- [Tokio](https://tokio.rs/) - Async runtime
- [Rustls](https://github.com/rustls/rustls) - TLS implementation
- [NATS](https://nats.io/) - Messaging system
- [Wasmtime](https://wasmtime.dev/) - WebAssembly runtime
- [smartcore](https://smartcorelib.org/) - Machine learning library

## Project Status

**Current Version:** 0.1.0 (Early Development)

This project is under active development. Features and APIs are subject to change. Not recommended for production use at this time.

For production DDoS mitigation, consider established solutions like:
- Cloudflare
- AWS Shield
- Fastly
- Akamai

Use SecBeat for:
- Learning Rust systems programming
- Experimenting with DDoS mitigation techniques
- Research and development
- Non-critical environments

---

**⚠️ Important**: This is a development project. Do not deploy to production without thorough testing and understanding of its limitations.
