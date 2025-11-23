---
title: API Reference
description: RESTful APIs for managing and controlling SecBeat
---

## API Overview

SecBeat provides comprehensive RESTful APIs for management, monitoring, and control operations.

### Available APIs

| API | Port | Purpose |
|-----|------|---------|
| Management API | 9999 | Mitigation Node control |
| Orchestrator API | 3030 | Control Plane operations |
| Metrics API | 9090, 9091, 9191 | Prometheus metrics |

## Authentication

All API requests require authentication via API keys in headers.

### API Key Header

```
X-SecBeat-API-Key: your-api-key-here
```

### Example Request

```bash
curl -H "X-SecBeat-API-Key: your-key" \
  http://localhost:9999/api/v1/status
```

:::danger Security Warning
Always change default API keys in production! Set via `MANAGEMENT_API_KEY` environment variable.
:::

## Management API

Control mitigation node operations, rules, and configuration.

### GET /api/v1/status

Get current node status and health information.

```bash
curl http://localhost:9999/api/v1/status
```

**Response:**
```json
{
  "status": "running",
  "mode": "l7",
  "uptime_seconds": 86400,
  "connections": {
    "active": 1247,
    "total": 524288
  },
  "health": "healthy"
}
```

### POST /api/v1/rules

Add a new WAF rule dynamically.

```bash
curl -X POST http://localhost:9999/api/v1/rules \
  -H "Content-Type: application/json" \
  -d '{
    "pattern": "(?i)(union.*select|select.*from)",
    "action": "block",
    "severity": "high"
  }'
```

**Response:**
```json
{
  "id": "rule_12345",
  "status": "active",
  "created_at": "2025-11-08T10:30:00Z"
}
```

### GET /api/v1/rules

List all active WAF rules.

```bash
curl http://localhost:9999/api/v1/rules
```

**Response:**
```json
{
  "rules": [
    {
      "id": "rule_12345",
      "pattern": "(?i)(union.*select)",
      "action": "block",
      "hits": 42
    }
  ],
  "total": 50000
}
```

### DELETE /api/v1/rules/:id

Remove a WAF rule.

```bash
curl -X DELETE http://localhost:9999/api/v1/rules/rule_12345
```

### POST /api/v1/blacklist

Add an IP to the blacklist.

```bash
curl -X POST http://localhost:9999/api/v1/blacklist \
  -H "Content-Type: application/json" \
  -d '{
    "ip": "192.0.2.100",
    "reason": "repeated attacks",
    "duration_seconds": 3600
  }'
```

### GET /api/v1/stats

Get detailed statistics.

```bash
curl http://localhost:9999/api/v1/stats
```

**Response:**
```json
{
  "packets_processed": 2500000,
  "attacks_blocked": 1247,
  "requests_per_second": 50000,
  "latency_ms": 0.3,
  "cpu_percent": 12,
  "memory_mb": 256
}
```

## Orchestrator API

Fleet management and control plane operations.

### GET /api/v1/nodes

List all registered mitigation nodes.

```bash
curl http://localhost:3030/api/v1/nodes
```

**Response:**
```json
{
  "nodes": [
    {
      "id": "node-1",
      "address": "10.0.1.10:9090",
      "status": "healthy",
      "mode": "l7",
      "load": 0.12
    },
    {
      "id": "node-2",
      "address": "10.0.1.11:9090",
      "status": "healthy",
      "mode": "syn",
      "load": 0.08
    }
  ]
}
```

### POST /api/v1/policy

Deploy a security policy to all nodes.

```bash
curl -X POST http://localhost:3030/api/v1/policy \
  -H "Content-Type: application/json" \
  -d '{
    "name": "strict-mode",
    "rules": [
      {"type": "rate_limit", "value": 1000},
      {"type": "geo_block", "countries": ["CN", "RU"]}
    ]
  }'
```

### POST /api/v1/scale

Trigger manual scaling operation.

```bash
curl -X POST http://localhost:3030/api/v1/scale \
  -d '{"action": "scale_up", "count": 2}'
```

### GET /api/v1/threats

Get threat intelligence summary.

```bash
curl http://localhost:3030/api/v1/threats
```

**Response:**
```json
{
  "active_threats": 15,
  "top_attackers": [
    {"ip": "192.0.2.50", "attacks": 542},
    {"ip": "203.0.113.100", "attacks": 387}
  ],
  "attack_types": {
    "syn_flood": 8,
    "http_flood": 5,
    "slowloris": 2
  }
}
```

## Metrics Endpoints

Prometheus-compatible metrics for monitoring.

### Mitigation Node Metrics

```bash
# Port 9090 - Public metrics
curl http://localhost:9090/metrics

# Port 9191 - Internal metrics
curl http://localhost:9191/metrics
```

### Key Metrics

| Metric | Type | Description |
|--------|------|-------------|
| `secbeat_packets_processed_total` | Counter | Total packets processed |
| `secbeat_attacks_blocked_total` | Counter | Total attacks blocked |
| `secbeat_latency_seconds` | Histogram | Request latency distribution |
| `secbeat_connections_active` | Gauge | Current active connections |
| `secbeat_cpu_usage_percent` | Gauge | CPU usage percentage |
| `secbeat_memory_usage_bytes` | Gauge | Memory usage in bytes |

## Webhooks

Configure webhooks to receive real-time event notifications.

### Configuration

```toml
[webhooks]
enabled = true
endpoints = [
  "https://your-app.com/webhooks/secbeat"
]
events = ["attack_detected", "node_health", "rule_triggered"]
```

### Event Payload Example

```json
{
  "event": "attack_detected",
  "timestamp": "2025-11-08T10:45:30Z",
  "node_id": "node-1",
  "data": {
    "attack_type": "syn_flood",
    "source_ip": "192.0.2.100",
    "packets_per_second": 100000,
    "action": "blocked"
  }
}
```

## Usage Examples

### Python Example

```python
import requests

API_KEY = "your-api-key"
BASE_URL = "http://localhost:9999/api/v1"

headers = {"X-SecBeat-API-Key": API_KEY}

# Get status
response = requests.get(f"{BASE_URL}/status", headers=headers)
status = response.json()
print(f"Status: {status['status']}")

# Add rule
rule = {
    "pattern": "(?i)script.*alert",
    "action": "block",
    "severity": "high"
}
response = requests.post(f"{BASE_URL}/rules", json=rule, headers=headers)
print(f"Rule created: {response.json()['id']}")
```

### Bash Script Example

```bash
#!/bin/bash
API_KEY="your-api-key"
BASE="http://localhost:9999/api/v1"

# Monitor stats every 5 seconds
while true; do
  curl -s -H "X-SecBeat-API-Key: $API_KEY" \
    "$BASE/stats" | jq '.requests_per_second'
  sleep 5
done
```

### JavaScript Example

```javascript
const API_KEY = 'your-api-key';
const BASE_URL = 'http://localhost:9999/api/v1';

async function getStatus() {
  const response = await fetch(`${BASE_URL}/status`, {
    headers: {'X-SecBeat-API-Key': API_KEY}
  });
  const data = await response.json();
  console.log('Status:', data);
}

getStatus();
```

## Next Steps

- [Configuration Reference](/reference/config/) - Configuration options
- [CLI Reference](/reference/cli/) - Command-line tools
- [Quick Start](/quickstart/) - Get started quickly
