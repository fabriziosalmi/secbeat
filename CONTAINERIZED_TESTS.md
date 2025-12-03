# Containerized Test Suite Documentation

## Overview

SecBeat's test suite has been containerized to eliminate sudo requirements and provide consistent test environments across different platforms. Tests run in Docker containers with Linux capabilities (CAP_NET_ADMIN, CAP_NET_RAW) instead of full root/sudo access.

## Benefits

### Security
- ✅ **No sudo required**: Tests run with minimal Linux capabilities
- ✅ **Isolated network**: Tests don't affect host networking
- ✅ **Non-root execution**: Tests run as `secbeat` user inside container
- ✅ **Minimal privileges**: Only CAP_NET_ADMIN/CAP_NET_RAW granted

### Portability
- ✅ **Consistent environment**: Same Rust version, dependencies across all platforms
- ✅ **Reproducible builds**: Docker image ensures deterministic test environment
- ✅ **CI/CD ready**: GitHub Actions, GitLab CI, Jenkins compatible
- ✅ **Multi-platform**: Works on Linux, macOS, Windows (with Docker Desktop)

### Development
- ✅ **Fast iteration**: Volume mounts for live code updates
- ✅ **Cached builds**: Persistent cargo cache between test runs
- ✅ **Interactive debugging**: Shell access with `make test-docker-shell`
- ✅ **Parallel execution**: PARALLEL_JOBS environment variable

## Quick Start

### Basic Test Execution

```bash
# Run all tests in Docker (no sudo)
make test-docker

# Run tests with custom log level
RUST_LOG=debug make test-docker

# Interactive shell for debugging
make test-docker-shell
```

### Using Docker Compose

```bash
# Run test runner service
docker-compose -f docker-compose.test.yml --profile test-runner up test-runner

# Interactive test environment
docker-compose -f docker-compose.test.yml run --rm test-runner bash

# Run specific test
docker-compose -f docker-compose.test.yml run --rm test-runner cargo test wasm_memory_leak
```

## Architecture

### Dockerfile.test

**Base Image**: `rust:1.80-bookworm`

**Installed Components**:
- Rust toolchain (rustfmt, clippy, cargo-fuzz)
- Build essentials (gcc, pkg-config, libssl-dev)
- Network tools (iproute2, iptables, tcpdump)
- eBPF/XDP dependencies (linux-headers, llvm, clang, libbpf-dev)
- Debugging tools (strace, gdb, valgrind, heaptrack)
- Performance profiling (linux-perf)

**User Configuration**:
- Non-root user: `secbeat` (UID 1000)
- Sudoers entry for capability management
- Cargo/Rustup in user home directory

**Linux Capabilities**:
```dockerfile
docker run --cap-add=NET_ADMIN --cap-add=NET_RAW
```

### Network Isolation

**Test Network**: `172.25.0.0/16` (Docker bridge)

**Isolation Benefits**:
- XDP/eBPF programs don't affect host interfaces
- Firewall rules contained to test network
- Multiple test instances can run in parallel
- Cleanup automatic on container stop

## Makefile Targets

### Primary Targets

```bash
# Legacy: Run tests with sudo (for systems without Docker)
make test

# Recommended: Run tests in Docker (no sudo)
make test-docker

# Build test container only
make test-docker-build

# Interactive test shell
make test-docker-shell
```

### Target Details

#### `make test-docker`
- Builds `secbeat-test` image
- Runs container with CAP_NET_ADMIN + CAP_NET_RAW
- Mounts workspace as volume for live updates
- Executes: `cargo test --workspace && ./test_comprehensive.sh`
- Removes container after completion (`--rm`)

#### `make test-docker-shell`
- Same capabilities as `test-docker`
- Starts interactive bash shell
- Useful for debugging failed tests
- Access to all test tools (valgrind, strace, gdb)

## Environment Variables

### Build-Time Variables

```bash
# Rust toolchain version (default: 1.80)
RUST_VERSION=1.80

# Parallel build jobs
PARALLEL_JOBS=4
```

### Runtime Variables

```bash
# Rust logging level
RUST_LOG=info              # options: error, warn, info, debug, trace

# Test environment identifier
TEST_ENV=docker            # automatic in containers

# Enable backtrace on panic
RUST_BACKTRACE=1           # default in containers
```

## Volume Mounts

### Workspace Mount

```bash
-v $(PWD):/workspace
```

**Purpose**: Live code updates without rebuild  
**Permissions**: Read/write as `secbeat` user  
**Files Affected**: Source code, tests, scripts, configs

### Target Cache

```bash
-v ./target:/workspace/target
```

**Purpose**: Persist compiled artifacts between runs  
**Benefit**: 10-100x faster subsequent test runs  
**Size**: ~500MB-2GB depending on build type

### Cargo Cache

```bash
-v cargo-cache:/home/secbeat/.cargo
```

**Purpose**: Cache downloaded crates and build cache  
**Benefit**: Avoid re-downloading dependencies  
**Size**: ~1-2GB for full dependency tree

### Reports Directory

```bash
-v ./reports:/workspace/reports
```

**Purpose**: Persist test reports, coverage data, profiling results  
**Files**: valgrind logs, heaptrack data, performance reports

## Linux Capabilities Explained

### CAP_NET_ADMIN

**Required For**:
- XDP program attachment to network interfaces
- eBPF map creation and manipulation
- Network namespace management
- iptables/nftables rule modification
- Traffic control (tc) commands

**Security Note**: Scoped to container network namespace

### CAP_NET_RAW

**Required For**:
- Raw socket creation (for SYN proxy)
- Packet injection in tests
- Custom protocol implementations
- ICMP packet generation

**Security Note**: Container isolated from host network

### CAP_SYS_ADMIN

**Required For**:
- bpf() syscall (eBPF program loading)
- perf_event_open() for profiling
- mount operations (if needed)

**Security Note**: Only needed for eBPF tests, can be omitted for unit tests

## CI/CD Integration

### GitHub Actions

```yaml
name: Containerized Tests

on: [push, pull_request]

jobs:
  test:
    runs-on: ubuntu-latest
    
    steps:
      - uses: actions/checkout@v3
      
      - name: Build test container
        run: docker build -t secbeat-test -f Dockerfile.test .
      
      - name: Run tests
        run: |
          docker run --rm \
            --cap-add=NET_ADMIN \
            --cap-add=NET_RAW \
            -v ${{ github.workspace }}:/workspace \
            -e RUST_LOG=info \
            secbeat-test
      
      - name: Upload test reports
        if: always()
        uses: actions/upload-artifact@v3
        with:
          name: test-reports
          path: reports/
```

### GitLab CI

```yaml
test:
  image: docker:latest
  services:
    - docker:dind
  script:
    - docker build -t secbeat-test -f Dockerfile.test .
    - docker run --rm --cap-add=NET_ADMIN --cap-add=NET_RAW -v $CI_PROJECT_DIR:/workspace secbeat-test
  artifacts:
    paths:
      - reports/
    when: always
```

## Troubleshooting

### Permission Denied Errors

**Symptom**: `permission denied` when running tests

**Solution**:
```bash
# Ensure volume mounts have correct permissions
chmod -R 755 $(pwd)

# Or run with user mapping
docker run --user $(id -u):$(id -g) ...
```

### eBPF Program Load Failures

**Symptom**: `bpf() syscall failed`

**Solution**:
```bash
# Add SYS_ADMIN capability
docker run --cap-add=SYS_ADMIN ...

# Or run specific eBPF tests separately
docker-compose -f docker-compose.test.yml run --rm mitigation-node
```

### Cargo Cache Corruption

**Symptom**: Build errors after container updates

**Solution**:
```bash
# Clear cargo cache volume
docker volume rm secbeat_cargo-test-cache

# Rebuild from scratch
docker-compose -f docker-compose.test.yml build --no-cache test-runner
```

### Network Connectivity Issues

**Symptom**: Tests timeout connecting to services

**Solution**:
```bash
# Check network creation
docker network ls | grep secbeat

# Inspect network
docker network inspect secbeat_secbeat-test-net

# Restart compose stack
docker-compose -f docker-compose.test.yml down
docker-compose -f docker-compose.test.yml up
```

## Performance Tuning

### Build Cache Optimization

```bash
# Use BuildKit for faster builds
export DOCKER_BUILDKIT=1

# Parallel layer builds
docker build --build-arg BUILDKIT_INLINE_CACHE=1 -t secbeat-test -f Dockerfile.test .
```

### Test Parallelization

```bash
# Set parallel jobs
docker run -e PARALLEL_JOBS=8 ...

# Or in docker-compose.test.yml
environment:
  - PARALLEL_JOBS=8
```

### Resource Limits

```bash
# Limit container resources
docker run \
  --memory=4g \
  --cpus=4 \
  --cap-add=NET_ADMIN \
  secbeat-test
```

## Migration from Sudo-Based Tests

### Before (Sudo Required)

```bash
# Old approach
sudo ./test_comprehensive.sh

# Issues:
# - Requires root password
# - Affects host system
# - Not portable
# - Security risk
```

### After (Containerized)

```bash
# New approach
make test-docker

# Benefits:
# - No sudo needed
# - Isolated environment
# - Portable across platforms
# - Secure by design
```

### Gradual Migration Steps

1. **Phase 1**: Add Docker test support (DONE)
2. **Phase 2**: Update CI/CD to use containers
3. **Phase 3**: Update documentation to recommend Docker
4. **Phase 4**: Deprecate sudo-based tests
5. **Phase 5**: Remove sudo requirements from scripts

## Best Practices

### For Developers

✅ **DO**: Use `make test-docker` for local testing  
✅ **DO**: Mount workspace for live code updates  
✅ **DO**: Use cargo cache volume for faster builds  
✅ **DO**: Check test reports in `reports/` directory  

❌ **DON'T**: Run tests with `--privileged` flag  
❌ **DON'T**: Modify host network from containers  
❌ **DON'T**: Cache Store/Instance objects (memory leaks)  
❌ **DON'T**: Run production workloads in test containers  

### For CI/CD

✅ **DO**: Build test image in CI pipeline  
✅ **DO**: Cache Docker layers between runs  
✅ **DO**: Upload test reports as artifacts  
✅ **DO**: Run tests on every PR  

❌ **DON'T**: Use `latest` tag for test images  
❌ **DON'T**: Run tests without capabilities  
❌ **DON'T**: Ignore test failures in CI  
❌ **DON'T**: Skip integration tests  

## Future Enhancements

### Planned Features

1. **Multi-Architecture Support**
   - ARM64 builds for Apple Silicon
   - Cross-compilation for embedded targets

2. **Test Sharding**
   - Parallel test execution across containers
   - Dynamic work distribution

3. **Coverage Reporting**
   - Integrated tarpaulin for code coverage
   - Automatic coverage upload to Codecov

4. **Performance Benchmarking**
   - Criterion.rs integration
   - Historical performance tracking

5. **Security Scanning**
   - cargo-audit in test pipeline
   - SAST (Static Application Security Testing)
   - Container vulnerability scanning

## References

- Docker Capabilities: https://docs.docker.com/engine/reference/run/#runtime-privilege-and-linux-capabilities
- eBPF in Containers: https://ebpf.io/what-is-ebpf/
- Rust Testing: https://doc.rust-lang.org/book/ch11-00-testing.html
- Cargo Cache: https://doc.rust-lang.org/cargo/guide/cargo-home.html

## Support

For issues with containerized tests:
1. Check this documentation
2. Review troubleshooting section
3. Open GitHub issue with test logs
4. Include Docker version and platform info

---

**Last Updated**: 2025-11-24  
**Version**: 1.0  
**Status**: Production Ready
