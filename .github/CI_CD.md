# CI/CD Configuration

## Overview

SecBeat uses GitHub Actions for continuous integration and testing. The CI pipeline ensures code quality, security, and functionality before merging changes.

## Workflows

### Test Suite (`.github/workflows/test.yml`)

Runs on every push and pull request to `main` and `develop` branches.

**Jobs:**

1. **Quick Checks** (runs first, ~2-3 minutes)
   - Code formatting (`cargo fmt`)
   - Linting (`cargo clippy`)
   - Build verification

2. **Unit Tests** (~3-5 minutes)
   - Fast unit tests without Docker
   - Library and binary tests
   - Runs in parallel with integration tests

3. **Integration Tests** (~10-15 minutes)
   - Runs in Docker with Linux capabilities
   - Requires `CAP_NET_ADMIN` and `CAP_NET_RAW`
   - Tests real networking functionality
   - Generates test reports

4. **Security Audit**
   - Checks for known security vulnerabilities
   - Runs `cargo audit`
   - Fails on high-severity issues

5. **Coverage** (main branch only)
   - Code coverage analysis
   - Uploads to Codecov
   - Optional (doesn't fail CI)

## Running Tests Locally

### Quick Tests (No Docker)
```bash
# Format check
cargo fmt --all -- --check

# Linting
cargo clippy --workspace --all-targets --all-features -- -D warnings

# Unit tests
cargo test --workspace --lib
```

### Full Test Suite (Docker)
```bash
# Build test container
make test-docker-build

# Run all tests in Docker
make test-docker

# Interactive debugging
make test-docker-shell
```

### LXC Testing (Recommended for eBPF/XDP)
**Why LXC?** Docker containers don't have full kernel access, so eBPF/XDP features can't be properly tested. LXC containers with privileged mode provide real kernel access.

```bash
# Quick test run (unit + integration)
./run_lxc_tests.sh --quick

# Full test suite including performance
./run_lxc_tests.sh --full

# Clean build before testing
./run_lxc_tests.sh --clean --full
```

**Requirements:**
- Proxmox host with LXC container (ID 100)
- SSH access to Proxmox host
- LXC container with Rust toolchain
- Privileged container mode for kernel access

**Configuration:**
```bash
export PROXMOX_HOST="root@your-proxmox-host"
export LXC_ID="100"
```

### Individual Test Suites
```bash
# Unit tests only
cargo test --workspace --lib

# Integration tests
cargo test --workspace --test integration_tests

# Performance tests
cargo test --workspace --test performance_tests -- --test-threads=1
```

## Docker Test Environment

The `Dockerfile.test` provides:
- Rust toolchain (1.80+)
- Linux networking capabilities
- eBPF/XDP dependencies
- Isolated test environment
- No sudo required

**Capabilities Required:**
- `CAP_NET_ADMIN` - Network configuration
- `CAP_NET_RAW` - Raw socket access

## Test Reports

Test artifacts are saved on every run:
- Location: GitHub Actions artifacts
- Retention: 7 days
- Includes: Test output, debug builds, coverage reports

## Environment Variables

```bash
RUST_BACKTRACE=1      # Enable backtraces
RUST_LOG=debug        # Logging level
CARGO_TERM_COLOR=always  # Colored output
```

## Troubleshooting

### Tests Fail in CI but Pass Locally
1. Check if you're using Docker locally
2. Verify Rust version matches CI (1.80+)
3. Check for timing-sensitive tests
4. Review GitHub Actions logs

### Docker Build Fails
```bash
# Clean Docker cache
docker system prune -a

# Rebuild from scratch
docker build --no-cache -t secbeat-test -f Dockerfile.test .
```

### Permission Issues
```bash
# Ensure Docker has NET_ADMIN capability
docker run --rm --cap-add=NET_ADMIN --cap-add=NET_RAW secbeat-test bash
```

## Adding New Tests

1. **Unit Tests**: Add to `src/` modules with `#[cfg(test)]`
2. **Integration Tests**: Add to `tests/integration_tests.rs`
3. **Performance Tests**: Add to `tests/performance_tests.rs`
4. **Update CI**: Modify `.github/workflows/test.yml` if needed

## Performance Benchmarks

Performance tests track:
- WAF inspection latency (P50/P95/P99)
- DDoS check throughput
- Memory efficiency
- Concurrent request handling

Thresholds are configured in test files.

## Security Scanning

- **cargo audit**: Checks dependencies for known vulnerabilities
- **clippy**: Catches common security issues
- **Manual review**: Required for all PRs

## Badge Status

Add to your README:
```markdown
![Tests](https://github.com/fabriziosalmi/secbeat/workflows/Test%20Suite/badge.svg)
```
