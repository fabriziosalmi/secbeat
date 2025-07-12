# SecBeat Phase 1: Basic TCP Proxy

## Overview

Phase 1 establishes the foundational infrastructure for the SecBeat mitigation node. This phase implements a basic, high-performance TCP proxy that forwards traffic between clients and backend servers using Rust's async/await capabilities with Tokio.

## Objectives

- ✅ Create a multi-threaded, asynchronous TCP proxy
- ✅ Establish basic logging and observability
- ✅ Set up the project structure and build system
- ✅ Implement bidirectional traffic forwarding
- ✅ Add basic error handling and connection management

## Architecture

```
[Client] ──────► [Mitigation Node:8443] ──────► [Backend:8080]
                      TCP Proxy
```

The proxy listens on port 8443 and forwards all traffic to a backend service on port 8080. This creates the basic data plane that will be enhanced with security features in subsequent phases.

## Key Components

### 1. Async TCP Listener
- Binds to `0.0.0.0:8443` for incoming connections
- Uses Tokio's `TcpListener` for high-performance async I/O
- Spawns a new task for each connection to handle concurrent clients

### 2. Connection Forwarding
- Establishes a new connection to the backend (`127.0.0.1:8080`) for each client
- Uses `tokio::io::copy_bidirectional` for efficient data copying
- Maintains connection state and handles cleanup on disconnect

### 3. Logging and Observability
- Structured logging with `tracing` and `tracing-subscriber`
- Logs connection establishment, data transfer metrics, and errors
- Provides visibility into proxy performance and behavior

## Configuration

The proxy uses hardcoded configuration in Phase 1:
- **Listen Address**: `0.0.0.0:8443`
- **Backend Address**: `127.0.0.1:8080`
- **Log Level**: Configurable via `RUST_LOG` environment variable

## Building and Running

### Build
```bash
cd mitigation-node
cargo build --release
```

### Run
```bash
cd mitigation-node
RUST_LOG=info cargo run
```

### Test
```bash
cd mitigation-node
./test_suite.sh
```

## Testing

The Phase 1 test suite includes:

1. **Build Verification**: Ensures the project compiles successfully
2. **Basic Connectivity**: Validates TCP proxy functionality
3. **Data Transfer**: Tests bidirectional data flow
4. **Error Handling**: Verifies graceful handling of connection failures

### Test Backend Server

The test suite includes a simple HTTP server that responds to requests:
```bash
# Start test origin server
cargo run --bin test-origin
```

### Manual Testing
```bash
# Test with curl (HTTP over TCP)
curl -v http://127.0.0.1:8443/

# Test with netcat for raw TCP
echo "Hello World" | nc 127.0.0.1 8443
```

## Performance Characteristics

- **Concurrency**: Handles thousands of concurrent connections
- **Memory Usage**: Minimal per-connection overhead
- **Latency**: Sub-millisecond proxy overhead
- **Throughput**: Limited primarily by network bandwidth

## Dependencies

### Core Dependencies
- `tokio`: Async runtime and networking
- `tracing`: Structured logging framework
- `tracing-subscriber`: Log output formatting
- `anyhow`: Error handling

### Development Dependencies
- Standard Rust toolchain (1.78+)
- Basic shell tools for testing

## Metrics and Monitoring

Phase 1 includes basic logging for:
- Server startup and shutdown
- New connection acceptance
- Connection termination with byte counts
- Error conditions and failures

Future phases will add Prometheus metrics and more sophisticated monitoring.

## Known Limitations

- No security features (added in later phases)
- Hardcoded configuration (improved in Phase 2+)
- Basic error handling (enhanced progressively)
- No load balancing or failover (future enhancement)

## Next Steps

Phase 1 provides the foundation for:
- **Phase 2**: Adding SYN Proxy for DDoS mitigation
- **Phase 3**: TLS termination and HTTP parsing
- **Phase 4**: Orchestrator integration and self-registration
- **Future Phases**: Advanced security, scaling, and AI features

## Troubleshooting

### Common Issues

1. **Port Already in Use**
   ```
   Error: Address already in use (os error 48)
   ```
   Solution: Ensure no other service is using port 8443

2. **Backend Connection Refused**
   ```
   Error: Connection refused (os error 61)
   ```
   Solution: Start the test origin server on port 8080

3. **Permission Denied**
   ```
   Error: Permission denied (os error 13)
   ```
   Solution: Run with appropriate permissions or use unprivileged ports

### Debug Mode
```bash
RUST_LOG=debug cargo run
```

This provides detailed tracing for troubleshooting connection issues and performance analysis.
