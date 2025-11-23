# Contributing to SecBeat

Thank you for your interest in contributing to SecBeat! This document provides guidelines and instructions for contributing to the project.

## Development Environment Setup

### Prerequisites

- **Rust**: 1.78 or later (install via [rustup](https://rustup.rs/))
- **Docker**: 20.10 or later
- **Docker Compose**: 1.29 or later
- **Git**: 2.30 or later
- **Linux**: Required for eBPF/XDP features (kernel 5.15+)

### Initial Setup

```bash
# Clone the repository
git clone https://github.com/fabriziosalmi/secbeat.git
cd secbeat

# Install Rust toolchain
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source $HOME/.cargo/env

# Build the project
cargo build --workspace

# Run tests
cargo test --workspace

# Start development environment
docker-compose up -d
```

### Development Workflow

1. **Create a feature branch**
   ```bash
   git checkout -b feature/your-feature-name
   ```

2. **Make your changes**
   - Follow Rust naming conventions
   - Add tests for new functionality
   - Update documentation for API changes

3. **Run quality checks**
   ```bash
   # Format code
   cargo fmt --all

   # Run linter
   cargo clippy --all-targets --all-features

   # Run tests
   cargo test --workspace

   # Check documentation
   cargo doc --no-deps --open
   ```

4. **Commit your changes**
   ```bash
   git add .
   git commit -m "feat: add feature description"
   ```

   Use conventional commit format:
   - `feat:` for new features
   - `fix:` for bug fixes
   - `docs:` for documentation changes
   - `test:` for test additions/changes
   - `refactor:` for code refactoring
   - `perf:` for performance improvements

5. **Push and create Pull Request**
   ```bash
   git push origin feature/your-feature-name
   ```

## Code Standards

### Rust Style Guide

- Follow the [Rust API Guidelines](https://rust-lang.github.io/api-guidelines/)
- Use `cargo fmt` for consistent formatting
- Address all `cargo clippy` warnings
- Maximum line length: 100 characters
- Use meaningful variable names (no single-letter variables except iterators)

### Documentation

- Add doc comments (`///`) for all public APIs
- Include examples in doc comments where helpful
- Update README.md for user-facing changes
- Keep CHANGELOG.md current

### Testing

- Write unit tests for all new functionality
- Add integration tests for complex features
- Maintain or improve test coverage
- Ensure all tests pass before submitting PR

## Pull Request Process

1. **Before Submitting**
   - Ensure all tests pass locally
   - Run `cargo fmt` and `cargo clippy`
   - Update documentation
   - Add entry to CHANGELOG.md

2. **PR Description Should Include**
   - Summary of changes
   - Motivation and context
   - Testing performed
   - Breaking changes (if any)
   - Related issues (use "Closes #123" to auto-close)

3. **Review Process**
   - At least one maintainer approval required
   - CI checks must pass
   - Address all review comments
   - Keep PR scope focused (split large changes)

4. **After Approval**
   - Maintainers will merge when ready
   - Delete feature branch after merge

## Architecture Guidelines

### Mitigation Node

Located in `mitigation-node/`:
- Handles traffic processing
- Implements WAF rules
- Manages TLS termination
- Reports metrics to orchestrator

### Orchestrator Node

Located in `orchestrator-node/`:
- Coordinates mitigation nodes
- Runs ML models for threat detection
- Distributes policies
- Aggregates metrics

### Common Libraries

Located in `secbeat-common/`:
- Shared data structures
- Protocol definitions
- Utility functions

## Testing Strategy

### Unit Tests

```bash
# Run all unit tests
cargo test --workspace --lib

# Run specific module tests
cargo test --package mitigation-node --lib waf
```

### Integration Tests

```bash
# Run integration tests
cargo test --workspace --test '*'

# Run behavioral analysis test
./test_behavioral_ban.sh
```

### Performance Tests

```bash
# Build with optimizations
cargo build --release

# Run benchmarks (requires nightly Rust)
cargo +nightly bench
```

## Debugging

### Enable Debug Logging

```bash
RUST_LOG=debug cargo run --bin mitigation-node
```

### Linux Capabilities (SYN Proxy)

```bash
# Grant capabilities to binary
sudo setcap cap_net_raw,cap_net_admin+ep target/debug/mitigation-node

# Verify capabilities
getcap target/debug/mitigation-node
```

### Docker Debugging

```bash
# View logs
docker-compose logs -f mitigation-node

# Enter container
docker-compose exec mitigation-node bash

# Rebuild without cache
docker-compose build --no-cache
```

## Common Issues

### Build Errors

**Problem**: Linker errors on macOS
**Solution**: Install Xcode Command Line Tools
```bash
xcode-select --install
```

**Problem**: Missing dependencies
**Solution**: Ensure all workspace members are built
```bash
cargo build --workspace
```

### Runtime Errors

**Problem**: "Operation not permitted" (SYN proxy)
**Solution**: Grant capabilities or run with sudo
```bash
sudo ./target/debug/mitigation-node
```

**Problem**: Port already in use
**Solution**: Kill process on port or change config
```bash
lsof -ti:8443 | xargs kill -9
```

## Getting Help

- **Questions**: Open a [GitHub Discussion](https://github.com/fabriziosalmi/secbeat/discussions)
- **Bugs**: File an [Issue](https://github.com/fabriziosalmi/secbeat/issues)
- **Security**: See [SECURITY.md](SECURITY.md) for responsible disclosure

## License

By contributing, you agree that your contributions will be licensed under the MIT License.
