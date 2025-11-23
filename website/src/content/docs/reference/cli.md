---
title: CLI Reference
description: Command-line interface reference for SecBeat
---

## Mitigation Node

### Basic Usage

```bash
mitigation-node [OPTIONS]
```

### Options

#### --config, -c
Specify configuration file path.

```bash
mitigation-node --config /etc/secbeat/config.prod.toml
```

#### --mode, -m
Override operation mode from config.

```bash
mitigation-node --mode l7
```

Valid modes: `tcp`, `syn`, `l7`, `auto`

#### --listen, -l
Override listen address.

```bash
mitigation-node --listen 0.0.0.0:8443
```

#### --upstream, -u
Override upstream backend address.

```bash
mitigation-node --upstream 127.0.0.1:8080
```

#### --log-level
Set logging verbosity.

```bash
mitigation-node --log-level debug
```

Valid levels: `error`, `warn`, `info`, `debug`, `trace`

#### --dry-run
Validate configuration without starting.

```bash
mitigation-node --dry-run --config config.prod.toml
```

#### --version, -v
Display version information.

```bash
mitigation-node --version
```

#### --help, -h
Display help message.

```bash
mitigation-node --help
```

## Orchestrator

### Basic Usage

```bash
orchestrator [OPTIONS]
```

### Options

#### --config, -c
Specify configuration file.

```bash
orchestrator --config /etc/secbeat/orchestrator.toml
```

#### --bind, -b
API server bind address.

```bash
orchestrator --bind 0.0.0.0:3030
```

#### --nats-url
NATS server URL.

```bash
orchestrator --nats-url nats://nats-server:4222
```

## Environment Variables

### RUST_LOG
Control logging via env var (overrides --log-level).

```bash
# Basic logging
RUST_LOG=info mitigation-node

# Module-specific logging
RUST_LOG=mitigation_node=debug,orchestrator=info mitigation-node

# Trace specific components
RUST_LOG=mitigation_node::syn_proxy=trace mitigation-node
```

### SECBEAT_CONFIG
Default configuration file.

```bash
export SECBEAT_CONFIG=/etc/secbeat/config.prod.toml
mitigation-node
```

### SECBEAT_AUTO_GENERATE_CERTS
Auto-generate self-signed certificates (development only).

```bash
SECBEAT_AUTO_GENERATE_CERTS=true mitigation-node
```

## Common Tasks

### Start with Custom Config

```bash
mitigation-node --config config.prod.toml --log-level info
```

### Test Configuration

```bash
mitigation-node --dry-run --config config.prod.toml
```

### Debug Mode

```bash
RUST_LOG=debug mitigation-node --config config.dev.toml
```

### Production Start

```bash
SECBEAT_CONFIG=config.prod \
  SYN_COOKIE_SECRET=$(cat /etc/secbeat/secrets/syn-cookie) \
  MANAGEMENT_API_KEY=$(cat /etc/secbeat/secrets/api-key) \
  mitigation-node
```

## Systemd Service

### Service File

```ini
[Unit]
Description=SecBeat Mitigation Node
After=network.target

[Service]
Type=simple
User=secbeat
Group=secbeat
ExecStart=/usr/local/bin/mitigation-node --config /etc/secbeat/config.prod.toml
Restart=always
RestartSec=10

# Environment
Environment="RUST_LOG=info"
EnvironmentFile=/etc/secbeat/secrets.env

# Security
AmbientCapabilities=CAP_NET_RAW CAP_NET_ADMIN
NoNewPrivileges=true
PrivateTmp=true

[Install]
WantedBy=multi-user.target
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
  -v /etc/secbeat:/etc/secbeat:ro \
  -e SECBEAT_CONFIG=config.prod \
  secbeat/mitigation-node:latest
```

### View Logs

```bash
docker logs -f secbeat
```

### Execute Commands

```bash
docker exec secbeat mitigation-node --version
```

## Troubleshooting Commands

### Check Listening Ports

```bash
sudo lsof -i :8443
sudo netstat -tlnp | grep mitigation-node
```

### Verify Capabilities

```bash
getcap /usr/local/bin/mitigation-node
```

### Test Backend Connection

```bash
curl -v http://localhost:8080
```

### Monitor Metrics

```bash
# Prometheus metrics
curl http://localhost:9090/metrics

# Internal metrics
curl http://localhost:9191/metrics

# Pretty print
curl -s http://localhost:9090/metrics | grep secbeat_
```

## Next Steps

- [Configuration Reference](/reference/config/) - Configuration options
- [API Reference](/reference/api/) - API endpoints
- [Quick Start](/quickstart/) - Getting started guide
