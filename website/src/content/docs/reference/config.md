---
title: Configuration Reference
description: Complete configuration options for SecBeat
---

## Configuration File Format

SecBeat uses TOML format for configuration. Configuration is specified via environment variables.

```bash
# Specify config file (without .toml extension)
SECBEAT_CONFIG=config.prod ./mitigation-node

# Or use absolute path to directory containing config
SECBEAT_CONFIG=/etc/secbeat/config.prod ./mitigation-node
```

## Platform Settings

```toml
[platform]
# Operation mode: tcp, syn, l7, or auto
mode = "l7"

# Node identifier (auto-generated if not set)
node_id = "node-1"

# Environment: dev, staging, prod
environment = "prod"
```

## Network Configuration

```toml
[network]
# Listen address for incoming traffic
listen_address = "0.0.0.0:8443"

# Backend upstream servers
upstream_address = "127.0.0.1:8080"

# Connection limits
max_connections = 100000
connection_timeout_seconds = 60

# Buffer sizes (bytes)
buffer_size = 65536

# TCP options
tcp_nodelay = true
tcp_keepalive = true
```

## TLS Configuration

```toml
[tls]
enabled = true
cert_path = "/etc/secbeat/certs/cert.pem"
key_path = "/etc/secbeat/certs/key.pem"

# Minimum TLS version: 1.2 or 1.3
min_tls_version = "1.3"

# Cipher suites (leave empty for defaults)
cipher_suites = [
    "TLS_AES_256_GCM_SHA384",
    "TLS_AES_128_GCM_SHA256",
    "TLS_CHACHA20_POLY1305_SHA256"
]
```

## DDoS Protection

```toml
[ddos.rate_limiting]
# Global rate limits
global_requests_per_second = 100000

# Per-IP rate limits
per_ip_requests_per_second = 1000
per_ip_connections = 100

# Burst allowance
burst_size = 5000

[ddos.blacklist]
# IP addresses to block
ips = ["192.0.2.100", "203.0.113.50"]

# CIDR ranges to block
cidrs = ["10.0.0.0/8"]

# Auto-blacklist threshold
auto_blacklist_threshold = 10000
auto_blacklist_duration_seconds = 3600  # 3600 seconds = 1 hour
```

## WAF Configuration

```toml
[waf]
enabled = true

# Block common attack patterns
block_sql_injection = true
block_xss = true
block_path_traversal = true
block_command_injection = true

# Custom rules file
rules_file = "/etc/secbeat/waf-rules.json"

# Action: block, log, or challenge
default_action = "block"
```

## SYN Proxy Settings

```toml
[syn_proxy]
enabled = true

# Cookie secret (change in production!)
cookie_secret = "${SYN_COOKIE_SECRET}"

# Timeout for handshake completion
timeout_seconds = 30

# Maximum SYN rate per IP
max_syn_per_second = 1000
```

## Metrics & Telemetry

```toml
[telemetry]
# Prometheus metrics port
metrics_port = 9090

# Internal metrics port
internal_metrics_port = 9191

# Enable detailed metrics
detailed_metrics = true

# Metrics update interval
update_interval_seconds = 10
```

## Management API

```toml
[management_api]
enabled = true
bind_address = "127.0.0.1:9999"

# API key for authentication
api_key = "${MANAGEMENT_API_KEY}"
api_key_header = "X-SecBeat-API-Key"

# Security
require_https = false  # Set true in production
rate_limit_per_minute = 100
```

## Orchestrator Integration

```toml
[orchestrator]
enabled = true
endpoint = "http://localhost:3030"

# Authentication
api_key = "${ORCHESTRATOR_API_KEY}"

# Registration
auto_register = true
heartbeat_interval_seconds = 30

# NATS connection
nats_url = "nats://localhost:4222"
```

## Logging

```toml
[logging]
# Log level: error, warn, info, debug, trace
level = "info"

# Log format: json or text
format = "json"

# Output: stdout, stderr, or file path
output = "stdout"

# File rotation (if using file output)
max_size_mb = 100
max_files = 10
```

## Performance Tuning

```toml
[performance]
# Worker threads (default: number of CPU cores)
worker_threads = 8

# Enable io_uring (Linux 5.1+)
io_uring_enabled = true

# Memory pool size
memory_pool_size_mb = 512
```

## Webhooks

```toml
[webhooks]
enabled = true

# Webhook endpoints
endpoints = [
    "https://your-app.com/webhooks/secbeat"
]

# Events to send
events = [
    "attack_detected",
    "node_health",
    "rule_triggered"
]

# Retry configuration
max_retries = 3
retry_delay_seconds = 5
```

## Environment Variables

SecBeat supports environment variable substitution in config files:

```toml
# Use ${VAR_NAME} syntax
cookie_secret = "${SYN_COOKIE_SECRET}"
api_key = "${MANAGEMENT_API_KEY}"
```

Set environment variables:

```bash
export SYN_COOKIE_SECRET=$(openssl rand -hex 32)
export MANAGEMENT_API_KEY=$(openssl rand -hex 32)
export ORCHESTRATOR_API_KEY=$(openssl rand -hex 32)
```

## Configuration Examples

### Development

```toml
[platform]
mode = "tcp"
environment = "dev"

[network]
listen_address = "0.0.0.0:8080"
max_connections = 1000

[tls]
enabled = false

[logging]
level = "debug"
```

### Production

```toml
[platform]
mode = "l7"
environment = "prod"

[network]
listen_address = "0.0.0.0:443"
max_connections = 100000
tcp_nodelay = true

[tls]
enabled = true
cert_path = "/etc/secbeat/certs/cert.pem"
key_path = "/etc/secbeat/certs/key.pem"
min_tls_version = "1.3"

[ddos.rate_limiting]
global_requests_per_second = 100000
per_ip_requests_per_second = 1000

[logging]
level = "info"
format = "json"
```

## Next Steps

- [API Reference](/reference/api/) - API endpoints
- [CLI Reference](/reference/cli/) - Command-line options
- [Installation](/installation/) - Deployment guides
