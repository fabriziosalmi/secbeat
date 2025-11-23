---
title: Installation & Deployment
description: Complete guide for deploying SecBeat across different environments
---

## Docker Deployment

Deploy SecBeat using Docker for quick testing and development environments.

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
```

### Production Mode

```bash
# Run with custom certificates
docker run -d \
  --name secbeat-mitigation \
  -p 8443:8443 \
  -p 9090:9090 \
  -v /path/to/certs:/app/certs:ro \
  -v /path/to/config.prod.toml:/app/config.prod.toml:ro \
  -e SECBEAT_CONFIG=config.prod \
  -e RUST_LOG=info \
  -e SYN_COOKIE_SECRET=$(openssl rand -hex 32) \
  secbeat/mitigation-node:latest
```

## Docker Compose Deployment

Full-stack deployment with all services orchestrated together.

### Quick Start

```bash
# Start all services (expected: all containers start successfully)
docker-compose up -d
# Expected output:
# Creating network "secbeat_secbeat-network" done
# Creating secbeat-nats ... done
# Creating secbeat-mitigation ... done

# View logs
docker-compose logs -f mitigation-node

# Check status
docker-compose ps

# Stop services
docker-compose down
```

### Services Included

| Service | Description | Ports |
|---------|-------------|-------|
| Mitigation Node | Main proxy service with DDoS protection | 8443, 9090, 9191, 9999 |
| Orchestrator | Fleet management and control plane | 3030, 9091 |
| NATS | High-performance message broker | 4222, 8222 |
| Prometheus | Metrics collection and monitoring | 9092 |

### Environment Configuration

Create a `.env` file:

```bash
# Configuration
SECBEAT_CONFIG=config.dev
RUST_LOG=info

# Security
SYN_COOKIE_SECRET=your-secret-here
MANAGEMENT_API_KEY=your-api-key-here
ORCHESTRATOR_API_KEY=your-orchestrator-key-here

# TLS
SECBEAT_AUTO_GENERATE_CERTS=true
SECBEAT_HOSTNAME=localhost
```

## Kubernetes Deployment

Production-grade deployment with high availability and auto-scaling.

### Prerequisites

- Kubernetes cluster 1.20+
- kubectl configured
- Helm 3.x (optional)
- Ingress controller

### Deploy Mitigation Nodes

```yaml
apiVersion: apps/v1
kind: Deployment
metadata:
  name: secbeat-mitigation
  namespace: secbeat
spec:
  replicas: 3
  selector:
    matchLabels:
      app: secbeat-mitigation
  template:
    metadata:
      labels:
        app: secbeat-mitigation
    spec:
      containers:
      - name: mitigation-node
        image: secbeat/mitigation-node:latest
        ports:
        - containerPort: 8443
        - containerPort: 9090
        env:
        - name: SECBEAT_CONFIG
          value: "config.prod"
        - name: RUST_LOG
          value: "info"
        resources:
          requests:
            memory: "256Mi"
            cpu: "500m"
          limits:
            memory: "1Gi"
            cpu: "2000m"
```

### Horizontal Pod Autoscaler

```yaml
apiVersion: autoscaling/v2
kind: HorizontalPodAutoscaler
metadata:
  name: secbeat-mitigation-hpa
spec:
  scaleTargetRef:
    apiVersion: apps/v1
    kind: Deployment
    name: secbeat-mitigation
  minReplicas: 3
  maxReplicas: 10
  metrics:
  - type: Resource
    resource:
      name: cpu
      target:
        type: Utilization
        averageUtilization: 70
```

## Bare Metal / VM Deployment

Direct installation on Linux servers for maximum performance.

### System Requirements

#### Hardware
- CPU: 4+ cores
- RAM: 4GB minimum, 8GB recommended
- Storage: 20GB SSD
- Network: 1Gbps+ NIC

#### Software
- OS: Ubuntu 22.04+ / RHEL 8+
- Rust: 1.78+
- OpenSSL: 1.1.1+
- systemd

### Installation Steps

```bash
# 1. Build release binary
cargo build --release --workspace

# 2. Install binary
sudo cp target/release/mitigation-node /usr/local/bin/
sudo chmod +x /usr/local/bin/mitigation-node

# 3. Set capabilities (for SYN proxy)
sudo setcap cap_net_raw,cap_net_admin+ep /usr/local/bin/mitigation-node

# 4. Create configuration directory
sudo mkdir -p /etc/secbeat
sudo cp config.prod.toml /etc/secbeat/

# 5. Install systemd service
sudo cp systemd/secbeat-mitigation.service /etc/systemd/system/
sudo systemctl daemon-reload
sudo systemctl enable secbeat-mitigation
sudo systemctl start secbeat-mitigation
```

## Production Considerations

:::danger Important Production Requirements
Before deploying to production, ensure all security measures are in place.
:::

### Security Hardening

- ✓ Generate strong SYN cookie secrets
- ✓ Use valid TLS certificates (Let's Encrypt or commercial)
- ✓ Change all default API keys
- ✓ Configure firewall rules
- ✓ Enable rate limiting
- ✓ Set up log aggregation
- ✓ Configure backup and disaster recovery
- ✓ Implement secret rotation policy

### Performance Tuning

```toml
# High traffic configuration
[network]
max_connections = 100000
buffer_size = 65536
tcp_nodelay = true

[ddos.rate_limiting]
global_requests_per_second = 100000
per_ip_requests_per_second = 1000

[performance]
worker_threads = 8
io_uring_enabled = true
```

## Security Best Practices

### TLS Configuration

```toml
[tls]
enabled = true
cert_path = "/etc/secbeat/certs/cert.pem"
key_path = "/etc/secbeat/certs/key.pem"

# Strong cipher suites only
cipher_suites = [
    "TLS_AES_256_GCM_SHA384",
    "TLS_AES_128_GCM_SHA256",
    "TLS_CHACHA20_POLY1305_SHA256"
]

# TLS 1.3 only
min_tls_version = "1.3"
```

### API Security

```toml
[management_api]
enabled = true
bind_address = "127.0.0.1:9999"
api_key_header = "X-SecBeat-API-Key"
require_https = true
rate_limit_per_minute = 100
```

## Monitoring Setup

### Prometheus Configuration

```yaml
# prometheus.yml
scrape_configs:
  - job_name: 'secbeat-mitigation'
    static_configs:
      - targets: ['localhost:9090']
    scrape_interval: 15s

  - job_name: 'secbeat-orchestrator'
    static_configs:
      - targets: ['localhost:9091']
    scrape_interval: 30s
```

### Key Metrics to Monitor

| Category | Metrics |
|----------|---------|
| **Traffic** | `secbeat_packets_processed_total`<br>`secbeat_requests_per_second`<br>`secbeat_bandwidth_bytes` |
| **Security** | `secbeat_attacks_blocked_total`<br>`secbeat_waf_rules_triggered`<br>`secbeat_rate_limit_exceeded` |
| **Performance** | `secbeat_latency_seconds`<br>`secbeat_cpu_usage_percent`<br>`secbeat_memory_usage_bytes` |

## Troubleshooting

### Common Issues

#### Permission denied for raw sockets

```bash
sudo setcap cap_net_raw,cap_net_admin+ep /usr/local/bin/mitigation-node
```

#### TLS handshake failures

Check certificate validity and permissions:

```bash
openssl x509 -in cert.pem -text -noout
ls -l /path/to/certs/
```

#### High memory usage

Adjust connection limits:

```toml
[network]
max_connections = 10000  # Reduce if needed
buffer_size = 32768      # Smaller buffers
```

### Debug Mode

```bash
# Enable debug logging
RUST_LOG=debug cargo run --release

# Trace specific modules
RUST_LOG=mitigation_node::syn_proxy=trace cargo run
```

## Next Steps

- [Quick Start](/quickstart/) - Get started quickly
- [Configuration Reference](/reference/config/) - Configuration options
- [API Reference](/reference/api/) - API documentation
