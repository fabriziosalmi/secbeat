# Changelog

All notable changes to SecBeat will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.9.6] - 2025-12-03

### Changed
- **CLI Reference**: Completely rewritten - SecBeat uses environment variables only, no CLI flags
- **Documentation**: Comprehensive sync with codebase - removed all inaccurate references
- **Index page**: Accurate feature status, removed inflated performance claims

### Fixed
- Fixed incorrect CLI flag references (--config, --mode, --interface do not exist)
- Fixed config file references (removed non-existent config.l7-notls.toml)
- Fixed debug mode commands in installation docs
- Fixed all version references to v0.9.6

## [0.9.5] - 2025-12-03

### Security
- Updated wasmtime from v26 to v29 (fixes CVE-2025-64345, CVE-2025-53901)
- Merged mdast-util-to-hast security update (PR #6)

### Changed
- Corrected README feature status: eBPF/XDP (232 lines), SYN Proxy (729 lines), WASM (533 lines) now shown as implemented
- Updated documentation to reflect actual implementation state
- Standardized version numbers across all documentation files

### Fixed
- Fixed broken GitHub links in website (replaced placeholder with actual username)
- Fixed magic number documentation (added 86400=24h, 3600=1h explanations)
- Removed condescending language ("just", "simply") from documentation
- Standardized date formats to ISO 8601

## [0.9.2] - 2025-11-24

### Added
- MIT License with 2025 copyright
- Core documentation files for website (overview, observability, syn-flood)
- Explicit slugs in Starlight content collection
- GitHub Pages deployment configuration (site and base URL)
- Comprehensive website README with build and deployment instructions

### Changed
- Expanded all acronyms on first use per documentation standards (Rule #7)
  - DDoS, WAF, TLS, XSS, SQL in README.md
  - eBPF, XDP, NATS, WASM in README.md
  - CVSS, PGP in SECURITY.md
  - TCP, ML, CPU in CONTRIBUTING.md
- Updated .gitignore to allow website/src/content/docs/core/ folder
- Fixed Astro config for proper GitHub Pages deployment

### Fixed
- Starlight build error: slug 'core/overview' not found
- GitHub Pages deployment workflow
- Sitemap integration warning (missing site URL)

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
