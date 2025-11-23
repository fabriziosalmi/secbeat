---
title: Quick Start
description: Get SecBeat up and running in minutes
---

## Quick Commands

### Development

```bash
# Start all services
docker-compose up -d

# View logs
docker-compose logs -f mitigation-node

# Test the proxy
curl -k https://localhost:8443/

# Check metrics
curl http://localhost:9191/metrics

# Stop all services
docker-compose down
```

### Production Build

```bash
# Build release binaries
cargo build --release --workspace

# Run tests
cargo test --workspace

# Install
sudo cp target/release/mitigation-node /usr/local/bin/
sudo setcap cap_net_raw,cap_net_admin+ep /usr/local/bin/mitigation-node
```

## Monitoring Endpoints

| Service | Endpoint | Description |
|---------|----------|-------------|
| Mitigation Node | `http://localhost:9090/metrics` | Prometheus metrics |
| Mitigation Node | `http://localhost:9191/metrics` | Internal metrics |
| Mitigation Node | `http://localhost:9999/` | Management API |
| Orchestrator | `http://localhost:3030/` | Control API |
| Orchestrator | `http://localhost:9091/metrics` | Orchestrator metrics |
| Prometheus | `http://localhost:9092/` | Metrics dashboard |
| NATS | `http://localhost:8222/` | NATS monitoring |

## Configuration Files

| File | Purpose | Environment |
|------|---------|-------------|
| `config.dev.toml` | Development | Local testing |
| `config.prod.toml` | Production | Live deployment |
| `config.l7.toml` | L7 mode with TLS | Full features |
| `config.l7-notls.toml` | L7 mode without TLS | Testing |

## Operation Modes

### TCP Mode
- Basic proxy with minimal overhead
- High performance
- No root privileges required

### SYN Mode (Beta)
- SYN flood protection prototype
- Requires root or CAP_NET_RAW capability
- **Experimental** - use TCP mode for production

### L7 Mode
- Full WAF + DDoS protection
- TLS termination
- Complete security features

### Select Mode in Config

```toml
[platform]
mode = "l7"  # tcp, syn, l7, or auto
```

## Security Checklist

Before deploying to production, ensure you:

- ✓ Change `SYN_COOKIE_SECRET` in production
- ✓ Replace default API keys
- ✓ Use valid TLS certificates
- ✓ Configure firewall rules
- ✓ Enable rate limiting
- ✓ Set up monitoring alerts
- ✓ Rotate secrets regularly
- ✓ Review blacklist/whitelist IPs

## Performance Tuning

### High Traffic (100K+ connections)

```toml
[network]
max_connections = 100000
buffer_size = 65536

[ddos.rate_limiting]
global_requests_per_second = 100000
```

### Low Latency

```toml
[network]
buffer_size = 16384
connection_timeout_seconds = 10
```

### Memory Constrained

```toml
[network]
max_connections = 10000
buffer_size = 8192
```

## Troubleshooting

### Service won't start

```bash
# Check logs
docker-compose logs mitigation-node

# Verify ports aren't in use
sudo lsof -i :8443
```

### High CPU usage

```bash
# Check metrics
curl http://localhost:9191/metrics | grep cpu

# Reduce connection limits
# Edit config: max_connections = 5000
```

### TLS errors

```bash
# Verify certificates
openssl x509 -in certs/cert.pem -text -noout

# Regenerate if needed
make setup-certs
```

### Permission denied (SYN mode)

```bash
# Set capabilities
sudo setcap cap_net_raw,cap_net_admin+ep /usr/local/bin/mitigation-node

# Or run with sudo
sudo ./target/release/mitigation-node
```

## Next Steps

- [Core Architecture](/core/overview/) - Understand how SecBeat works
- [Installation](/installation/) - Detailed installation guide
- [API Reference](/reference/api/) - API documentation
