# SecBeat Deployment Guide

This document provides deployment instructions for SecBeat across different environments.

## Table of Contents

- [Docker Deployment](#docker-deployment)
- [Docker Compose Deployment](#docker-compose-deployment)
- [Kubernetes Deployment](#kubernetes-deployment)
- [Bare Metal / VM Deployment](#bare-metal--vm-deployment)
- [Production Considerations](#production-considerations)

---

## Docker Deployment

### Single Mitigation Node

```bash
# Build the image (expected: successful build in 2-4 minutes)
docker build -t secbeat/mitigation-node:latest .
# Expected output:
# Successfully built abc123def456
# Successfully tagged secbeat/mitigation-node:latest

# Run with auto-generated certificates (development)
docker run -d \
  --name secbeat-mitigation \
  -p 8443:8443 \
  -p 9090:9090 \
  -e SECBEAT_CONFIG=config.dev \
  -e RUST_LOG=info \
  -e SECBEAT_AUTO_GENERATE_CERTS=true \
  secbeat/mitigation-node:latest

# Run with custom certificates (production)
docker run -d \
  --name secbeat-mitigation \
  -p 8443:8443 \
  -p 9090:9090 \
  -v /path/to/certs:/app/certs:ro \
  -v /path/to/config.prod.toml:/app/config.prod.toml:ro \
  -e SECBEAT_CONFIG=config.prod \
  -e RUST_LOG=info \
  -e SECBEAT_AUTO_GENERATE_CERTS=false \
  -e SYN_COOKIE_SECRET=$(openssl rand -hex 32) \
  secbeat/mitigation-node:latest
```

---

## Docker Compose Deployment

### Quick Start (Development)

```bash
# Start all services (expected: all containers start successfully)
docker-compose up -d
# Expected output:
# Creating network "secbeat_secbeat-network" done
# Creating secbeat-nats ... done
# Creating secbeat-mitigation ... done

# View logs (expected: JSON-formatted log entries)
docker-compose logs -f mitigation-node
# Expected output:
# {"timestamp":"2025-11-24T10:00:00Z","level":"INFO","message":"Starting SecBeat Mitigation Node"}

# Check service status
docker-compose ps

# Stop all services
docker-compose down
```

### Services Included

- **mitigation-node**: Main proxy service (ports 8443, 9090, 9191, 9999)
- **orchestrator**: Fleet management (ports 3030, 9091)
- **nats**: Message broker (ports 4222, 8222)
- **test-origin**: Backend test server (port 8080)
- **prometheus**: Metrics collection (port 9092)

### Environment Variables

Create a `.env` file for customization:

```bash
# SecBeat Configuration
SECBEAT_CONFIG=config.dev
RUST_LOG=info

# Security (generate with: openssl rand -hex 32)
SYN_COOKIE_SECRET=GENERATE_WITH_openssl_rand_hex_32
MANAGEMENT_API_KEY=GENERATE_WITH_openssl_rand_hex_32
ORCHESTRATOR_API_KEY=GENERATE_WITH_openssl_rand_hex_32

# TLS
SECBEAT_AUTO_GENERATE_CERTS=true
SECBEAT_HOSTNAME=localhost
SECBEAT_TLS_ENABLED=false

# NATS
NATS_AUTH_TOKEN=your-nats-token-here
```

---

## Kubernetes Deployment

**Status**: Planned - Not yet implemented

Kubernetes deployment manifests are planned for v0.2.0 release. Use Docker or Docker Compose for v0.1.x deployments.

### Planned Features

- StatefulSet for orchestrator
- DaemonSet for mitigation nodes
- HPA (Horizontal Pod Autoscaler) support
- ConfigMaps for configuration management
- Secrets for sensitive data
- Service mesh integration (Istio/Linkerd)

---

## Bare Metal / VM Deployment

### Prerequisites

- Rust 1.78+
- Linux kernel with raw socket support (for SYN proxy)
- Root/sudo access (for raw socket operations)
- TLS certificates

### Installation Steps

```bash
# 1. Clone repository
git clone https://github.com/fabriziosalmi/secbeat.git
cd secbeat

# 2. Install dependencies (Ubuntu/Debian)
sudo apt update
sudo apt install -y build-essential pkg-config libssl-dev

# 3. Build release binaries
cargo build --release --workspace

# 4. Install binaries
sudo cp target/release/mitigation-node /usr/local/bin/
sudo cp target/release/orchestrator-node /usr/local/bin/

# 5. Set capabilities (for SYN proxy mode)
sudo setcap cap_net_raw,cap_net_admin+ep /usr/local/bin/mitigation-node

# 6. Create configuration directory
sudo mkdir -p /etc/secbeat
sudo cp config.prod.toml /etc/secbeat/config.toml

# 7. Generate TLS certificates
sudo mkdir -p /etc/secbeat/tls
sudo openssl req -x509 -newkey rsa:4096 \
  -keyout /etc/secbeat/tls/key.pem \
  -out /etc/secbeat/tls/cert.pem \
  -days 365 -nodes \
  -subj "/CN=yourdomain.com"

# 8. Install systemd service
sudo cp systemd/secbeat-mitigation.service /etc/systemd/system/
sudo systemctl daemon-reload
sudo systemctl enable secbeat-mitigation
sudo systemctl start secbeat-mitigation

# 9. Check status
sudo systemctl status secbeat-mitigation
```

### Systemd Service Management

```bash
# Start service
sudo systemctl start secbeat-mitigation

# Stop service
sudo systemctl stop secbeat-mitigation

# Restart service
sudo systemctl restart secbeat-mitigation

# View logs
sudo journalctl -u secbeat-mitigation -f

# Check status
sudo systemctl status secbeat-mitigation
```

---

## Production Considerations

### Performance Tuning

```toml
# config.prod.toml optimizations
[network]
max_connections = 100000
buffer_size = 65536
keep_alive_timeout_seconds = 60

[ddos.rate_limiting]
global_requests_per_second = 50000
global_burst_size = 100000
```

### Security Hardening

1. **TLS Configuration**
   - Use valid certificates from trusted CA
   - Enable OCSP stapling
   - Use TLS 1.3 only for maximum security

2. **Firewall Rules**
   ```bash
   # Allow only necessary ports
   sudo ufw allow 8443/tcp  # HTTPS
   sudo ufw allow 9090/tcp  # Metrics (restrict to monitoring network)
   sudo ufw enable
   ```

3. **Secrets Management**
   - Use environment variables or secret management systems
   - Rotate SYN cookie secrets regularly
   - Protect API keys with proper access controls

4. **Monitoring**
   - Set up Prometheus + Grafana
   - Configure alerting for high CPU/memory usage
   - Monitor DDoS event rates
   - Track WAF block rates

### High Availability

Deploy multiple mitigation nodes behind a load balancer:

```
         Load Balancer (HAProxy/NGINX)
                    |
    +---------------+---------------+
    |               |               |
Mitigation-1   Mitigation-2   Mitigation-3
    |               |               |
    +---------------+---------------+
                    |
              Orchestrator
                    |
                  NATS
```

### Scaling

1. **Horizontal Scaling**: Add more mitigation nodes
2. **Vertical Scaling**: Increase CPU/RAM per node
3. **Auto-scaling**: Use orchestrator's predictive scaling

### Logging

For production, configure structured logging:

```toml
[logging]
level = "info"
format = "json"
output = "file"
file_path = "/var/log/secbeat/mitigation-node.log"
max_file_size_mb = 100
max_files = 10
compress_rotated = true
```

---

## Troubleshooting

### Common Issues

**1. Permission Denied (Raw Sockets)**
```bash
# Solution: Set capabilities
sudo setcap cap_net_raw,cap_net_admin+ep /usr/local/bin/mitigation-node
```

**2. TLS Certificate Errors**
```bash
# Check certificate validity
openssl x509 -in /path/to/cert.pem -text -noout

# Verify certificate and key match
openssl x509 -noout -modulus -in cert.pem | openssl md5
openssl rsa -noout -modulus -in key.pem | openssl md5
```

**3. High Memory Usage**
```bash
# Check metrics
curl http://localhost:9191/metrics | grep memory

# Adjust connection limits in config
max_connections = 50000  # Reduce if needed
```

**4. Port Already in Use**
```bash
# Find process using port
sudo lsof -i :8443

# Kill process or change port in configuration
```

---

## Further Reading

- [PLATFORM.md](PLATFORM.md) - Platform architecture and capabilities
- [KERNEL_OPERATIONS.md](KERNEL_OPERATIONS.md) - Kernel-level operations guide
- [README.md](README.md) - Project overview and quick start

---

**Note**: Proxmox-specific deployment automation is planned for future releases. Current recommendation is to use systemd services on Proxmox VMs following the bare metal deployment guide.
