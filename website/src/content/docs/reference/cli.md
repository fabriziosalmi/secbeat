---
title: CLI & Environment Reference
description: Environment variables and runtime configuration for SecBeat
---

## Overview

SecBeat uses **environment variables** for runtime configuration. There are no command-line flags.

:::note
SecBeat intentionally avoids CLI argument parsing to keep the binary simple and container-friendly. All configuration is done via environment variables and TOML config files.
:::

## Mitigation Node

### Basic Usage

```bash
# Run with default config detection
./mitigation-node

# Run with specific config
SECBEAT_CONFIG=config.prod ./mitigation-node
```

### Environment Variables

| Variable | Description | Default |
|----------|-------------|---------|
| `SECBEAT_CONFIG` | Config file name (without `.toml` extension) | Auto-detect |
| `MITIGATION_CONFIG` | Legacy config name (fallback) | `DEPLOYMENT_ENV` detection |
| `DEPLOYMENT_ENV` | Environment (`production` or `development`) | Auto-detect |
| `RUST_LOG` | Log level filter | `info` |
| `SYN_COOKIE_SECRET` | Secret for SYN cookie generation | **Required in production** |
| `MANAGEMENT_API_KEY` | API authentication key | **Required in production** |
| `SECBEAT_AUTO_GENERATE_CERTS` | Auto-generate TLS certs (dev only) | `false` |
| `SECBEAT_HOSTNAME` | Hostname for generated certs | `localhost` |

### Configuration File Resolution

The mitigation node searches for config files in this order:

1. `{SECBEAT_CONFIG}.toml` (root directory)
2. `mitigation-node/config/{SECBEAT_CONFIG}.toml`
3. `mitigation-node/config/default.toml` (final fallback)

### Operation Modes

Set via `[platform].mode` or `[mitigation].operation_mode` in config:

| Mode | Description | Requirements |
|------|-------------|--------------|
| `tcp` | Basic TCP proxy | None |
| `syn` | SYN flood protection | Linux, `CAP_NET_RAW` |
| `l7` | Full HTTP/TLS/WAF | None |
| `auto` | Auto-detect from features | None |

### Examples

```bash
# Development
SECBEAT_CONFIG=config.dev RUST_LOG=debug ./mitigation-node

# Production
SECBEAT_CONFIG=config.prod \
  SYN_COOKIE_SECRET=$(cat /etc/secbeat/secrets/syn-cookie) \
  MANAGEMENT_API_KEY=$(cat /etc/secbeat/secrets/api-key) \
  RUST_LOG=info \
  ./mitigation-node

# Docker
docker run -d \
  -e SECBEAT_CONFIG=config.prod \
  -e SYN_COOKIE_SECRET=your-secret \
  -e RUST_LOG=info \
  secbeat/mitigation-node:latest
```

## Orchestrator

### Basic Usage

```bash
./orchestrator-node
```

### Configuration

The orchestrator currently uses hardcoded defaults. Configuration via environment variables or files is planned for a future release.

| Setting | Default Value |
|---------|---------------|
| API bind address | `127.0.0.1:3030` |
| Metrics address | `127.0.0.1:9091` |
| NATS URL | `nats://127.0.0.1:4222` |
| Heartbeat timeout | 30 seconds |
| Min fleet size | 1 |
| Scale up CPU threshold | 80% |
| Scale down CPU threshold | 30% |

### Environment Variables

| Variable | Description |
|----------|-------------|
| `RUST_LOG` | Log level filter |

:::caution Work in Progress
The orchestrator is under active development. External configuration support will be added in a future release.
:::

## Logging Configuration

### RUST_LOG Syntax

```bash
# Global level
RUST_LOG=debug

# Module-specific
RUST_LOG=mitigation_node=debug,hyper=warn

# Trace specific components
RUST_LOG=mitigation_node::syn_proxy=trace,mitigation_node::waf=debug
```

### Log Levels

| Level | Description |
|-------|-------------|
| `error` | Critical errors only |
| `warn` | Warnings and errors |
| `info` | Standard operational logs |
| `debug` | Detailed debugging info |
| `trace` | Very verbose tracing |

## Systemd Service

### Service File

```ini
[Unit]
Description=SecBeat Mitigation Node
After=network.target nats.service

[Service]
Type=simple
User=secbeat
Group=secbeat
WorkingDirectory=/opt/secbeat
ExecStart=/usr/local/bin/mitigation-node

# Environment
Environment="SECBEAT_CONFIG=config.prod"
Environment="RUST_LOG=info"
EnvironmentFile=/etc/secbeat/secrets.env

# Security
AmbientCapabilities=CAP_NET_RAW CAP_NET_ADMIN
NoNewPrivileges=true
PrivateTmp=true
ProtectSystem=strict
ReadWritePaths=/var/log/secbeat

Restart=always
RestartSec=10

[Install]
WantedBy=multi-user.target
```

### Secrets File

Create `/etc/secbeat/secrets.env`:

```bash
SYN_COOKIE_SECRET=your-32-byte-hex-secret
MANAGEMENT_API_KEY=your-api-key
ORCHESTRATOR_API_KEY=your-orchestrator-key
```

### Service Management

```bash
# Start service
sudo systemctl start secbeat-mitigation

# Enable on boot
sudo systemctl enable secbeat-mitigation

# Check status
sudo systemctl status secbeat-mitigation

# View logs
sudo journalctl -u secbeat-mitigation -f
```

## Docker Commands

### Build Image

```bash
docker build -t secbeat/mitigation-node:latest .
```

### Run Container

```bash
docker run -d \
  --name secbeat \
  -p 8443:8443 \
  -p 9090:9090 \
  -p 9191:9191 \
  -p 9999:9999 \
  -v /path/to/config.prod.toml:/app/config.prod.toml:ro \
  -v /path/to/certs:/app/certs:ro \
  -e SECBEAT_CONFIG=config.prod \
  -e RUST_LOG=info \
  -e SYN_COOKIE_SECRET=your-secret \
  secbeat/mitigation-node:latest
```

### Docker Compose

```bash
# Start all services
docker-compose up -d

# View logs
docker-compose logs -f mitigation-node

# Stop services
docker-compose down
```

## Troubleshooting

### Check Listening Ports

```bash
sudo lsof -i :8443
sudo netstat -tlnp | grep mitigation-node
```

### Verify Capabilities

```bash
# Check capabilities
getcap /usr/local/bin/mitigation-node

# Set capabilities for SYN proxy
sudo setcap cap_net_raw,cap_net_admin+ep /usr/local/bin/mitigation-node
```

### Test Endpoints

```bash
# Health check
curl http://localhost:9999/api/v1/status

# Metrics
curl http://localhost:9191/metrics

# HTTPS proxy
curl -k https://localhost:8443/
```

### Debug Logging

```bash
# Maximum verbosity
RUST_LOG=trace SECBEAT_CONFIG=config.dev ./mitigation-node 2>&1 | tee debug.log
```

## Next Steps

- [Configuration Reference](/reference/config/) - TOML configuration options
- [API Reference](/reference/api/) - REST API endpoints
- [Quick Start](/quickstart/) - Getting started guide
