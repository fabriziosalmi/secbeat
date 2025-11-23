# Changelog

All notable changes to SecBeat will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
- L7 plain HTTP mode support (TLS-optional configuration)
- Comprehensive deployment documentation for Linux environments
- Docker-in-LXC deployment guidelines and limitations
- Integration test suite for behavioral analysis
- Native LXC execution support for eBPF/XDP features

### Changed
- Fixed Aya 0.13 API migration (MapData ownership model)
- Resolved module import architecture (library namespace usage)
- Improved Docker build caching strategy
- Updated configuration file structure for multiple deployment scenarios

### Fixed
- Debug trait cascade removal for eBPF-incompatible types
- Borrow-after-move errors in distributed state synchronization
- GPG signature errors in Debian package installation
- Docker layer caching issues during rapid iteration

### Removed
- Hardcoded TLS requirement in L7 proxy mode

## [0.1.0] - 2025-11-23

### Added
- Initial project structure
- TCP reverse proxy with async Tokio runtime
- TLS 1.2/1.3 termination using Rustls
- WAF engine with regex-based attack detection
  - SQL injection patterns (~30 rules)
  - XSS patterns (~35 rules)
  - Path traversal patterns (~21 rules)
  - Command injection patterns (~20 rules)
- NATS-based messaging between nodes
- Prometheus metrics endpoint
- Management API for health and configuration
- Orchestrator node for fleet coordination
- Random Forest anomaly detection (smartcore)
- Behavioral analysis with sliding window
- WASM runtime for custom WAF rules (Wasmtime)
- Experimental eBPF/XDP packet filtering (Linux only)
- Experimental SYN proxy with cookie validation
- Docker Compose deployment
- Basic documentation and examples

### Known Issues
- SYN proxy is prototype-quality, not production-ready
- eBPF/XDP requires Linux kernel 5.15+ and CAP_NET_RAW
- Test coverage incomplete
- WASM runtime has limited rule complexity support
- Performance not optimized for high-throughput scenarios
- Documentation ahead of implementation in some areas

## Release Dates

- **Unreleased**: In development
- **0.1.0**: 2025-11-23

## Version Support

| Version | Status      | Support End |
|---------|-------------|-------------|
| 0.1.x   | Development | TBD         |

---

**Note**: This project is in early development. Breaking changes may occur between minor versions until 1.0.0 release.

For detailed information about each release, see the [GitHub Releases](https://github.com/fabriziosalmi/secbeat/releases) page.
