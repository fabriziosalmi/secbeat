# 🚀 Phase 5: Centralized Intelligence & Real-time Control - Setup Guide

## Overview

Phase 5 transforms the SecBeat system from a simple fleet manager into a Security Information and Event Management (SIEM) system with real-time control capabilities.

### New Capabilities

1. **Real-time Event Streaming**: All HTTP requests generate structured security events
2. **Threat Intelligence Engine**: Maintains IP blocklists and analyzes event patterns
3. **Command & Control**: Push defensive rules to all nodes simultaneously
4. **WAF Event Analysis**: Enhanced WAF with detailed event reporting
5. **Dynamic IP Blocking**: Real-time IP blocking based on threat intelligence

## Prerequisites

### NATS Server Installation

NATS is our real-time message bus for event streaming and control commands.

**macOS:**
```bash
brew install nats-server
```

**Linux (Ubuntu/Debian):**
```bash
wget https://github.com/nats-io/nats-server/releases/download/v2.10.4/nats-server-v2.10.4-linux-amd64.tar.gz
tar -xzf nats-server-v2.10.4-linux-amd64.tar.gz
sudo mv nats-server-v2.10.4-linux-amd64/nats-server /usr/local/bin/
```

**Docker:**
```bash
docker run -p 4222:4222 -p 8222:8222 nats:latest -js -m 8222
```

### Verify Installation
```bash
nats-server --version
```

## Architecture Overview

```
┌─────────────────┐    NATS Topics:           ┌─────────────────┐
│                 │    • secbeat.events.waf   │                 │
│ Mitigation Node │◄──► • secbeat.control     │   Orchestrator  │
│   (Producer)    │      commands             │     (SIEM)      │
│                 │                           │                 │
└─────────────────┘                           └─────────────────┘
        │                                             │
        │ HTTP Proxy                                  │ Fleet API
        ▼                                             ▼
┌─────────────────┐                           ┌─────────────────┐
│  Client Traffic │                           │ Threat Intel    │
│                 │                           │ & Control Plane │
└─────────────────┘                           └─────────────────┘
```

## Quick Start

1. **Start the complete system:**
   ```bash
   cd /Users/fab/GitHub/secbeat
   ./test_phase5.sh
   ```

2. **Manual startup (if preferred):**
   ```bash
   # Terminal 1: NATS Server
   nats-server --port 4222 --http_port 8222
   
   # Terminal 2: Orchestrator with Threat Intelligence
   cd orchestrator-node && cargo run
   
   # Terminal 3: Test Origin Server
   cd mitigation-node && cargo run --bin test-origin
   
   # Terminal 4: Mitigation Node with Event Publishing
   cd mitigation-node && cargo run --bin mitigation-node
   ```

## Testing Phase 5 Features

### 1. Verify Event Streaming
```bash
# Generate a normal request (creates LOG event)
curl -k https://127.0.0.1:8443/api/test

# Generate a malicious request (creates BLOCK event)
curl -k "https://127.0.0.1:8443/test?q=<script>alert('xss')</script>"
```

### 2. Test Threat Intelligence
```bash
# Manually block an IP
curl -X POST http://127.0.0.1:3030/api/v1/rules/block_ip \
  -H "Content-Type: application/json" \
  -d '{"ip": "192.168.1.100", "reason": "Suspicious activity", "ttl_seconds": 3600}'

# View blocked IPs
curl http://127.0.0.1:3030/api/v1/rules/blocked_ips
```

### 3. Test Fleet-wide Control
```bash
# Block your own IP to test dynamic blocking
curl -X POST http://127.0.0.1:3030/api/v1/rules/block_ip \
  -H "Content-Type: application/json" \
  -d '{"ip": "127.0.0.1", "reason": "Testing dynamic block"}'

# Try to make a request (should be blocked)
curl -k https://127.0.0.1:8443/test
```

### 4. Monitor Real-time Events
```bash
# Monitor NATS subjects
nats sub "secbeat.events.waf"
nats sub "secbeat.control.commands"

# Or view NATS web UI
open http://127.0.0.1:8222
```

## API Endpoints

### Orchestrator (Port 3030)

| Endpoint | Method | Description |
|----------|--------|-------------|
| `/api/v1/fleet/stats` | GET | Fleet statistics |
| `/api/v1/nodes` | GET | List all nodes |
| `/api/v1/rules/block_ip` | POST | Block IP address |
| `/api/v1/rules/blocked_ips` | GET | List blocked IPs |

### Example: Block IP Request
```json
{
  "ip": "192.168.1.100",
  "reason": "Multiple failed login attempts",
  "ttl_seconds": 3600,
  "metadata": {
    "detection_method": "manual",
    "severity": "high"
  }
}
```

### Example: Block IP Response
```json
{
  "success": true,
  "message": "IP 192.168.1.100 blocked successfully",
  "command_id": "123e4567-e89b-12d3-a456-426614174000"
}
```

## Event Stream Format

### Security Event (Published to `secbeat.events.waf`)
```json
{
  "node_id": "c2d77c15-093e-489b-b2f7-7e62e5db2630",
  "timestamp": "2025-07-12T10:30:45.123Z",
  "source_ip": "192.168.1.100",
  "http_method": "GET",
  "uri": "/api/test?q=<script>alert('xss')</script>",
  "host_header": "example.com",
  "user_agent": "Mozilla/5.0 ...",
  "waf_result": {
    "action": "BLOCK",
    "matched_rules": ["XSS_SCRIPT_TAG"],
    "confidence": 0.9
  },
  "response_status": 403,
  "processing_time_ms": 25
}
```

### Control Command (Published to `secbeat.control.commands`)
```json
{
  "command_id": "123e4567-e89b-12d3-a456-426614174000",
  "action": "ADD_DYNAMIC_RULE",
  "rule_type": "IP_BLOCK",
  "target": "192.168.1.100",
  "ttl_seconds": 3600,
  "timestamp": "2025-07-12T10:30:45.123Z",
  "metadata": {
    "block_reason": "Suspicious activity",
    "blocked_by": "manual_operator"
  }
}
```

## System Monitoring

### NATS Monitoring
- **Web UI**: http://127.0.0.1:8222
- **Stats API**: `curl http://127.0.0.1:8222/varz`

### Application Metrics
- **Mitigation Node**: http://127.0.0.1:9090/metrics
- **Orchestrator**: http://127.0.0.1:9091/metrics

### Key Metrics to Watch
- `secbeat_security_events_published_total`
- `secbeat_control_commands_processed_total`
- `secbeat_dynamic_blocks_total`
- `waf_requests_blocked`

## Troubleshooting

### NATS Connection Issues
```bash
# Check if NATS is running
lsof -i :4222

# Test NATS connectivity
nats pub test "hello world"
nats sub test
```

### Event Publishing Problems
Check logs for:
- "Failed to connect to NATS server"
- "Failed to publish security event"
- "Failed to serialize security event"

### Control Command Issues
Check logs for:
- "Failed to subscribe to control commands"
- "Unknown command action"
- "Invalid IP address in command"

## Configuration

### Mitigation Node (`config/default.toml`)
```toml
[nats]
enabled = true
server_url = "nats://127.0.0.1:4222"
publish_events = true
consume_commands = true
reconnect_attempts = 10
command_timeout = 30
```

### Orchestrator
NATS URL is configured in code defaults:
- `nats_url: "nats://127.0.0.1:4222"`

## Security Considerations

1. **NATS Authentication**: In production, enable NATS authentication
2. **TLS for NATS**: Use TLS-encrypted NATS connections
3. **Command Authorization**: Validate control commands before execution
4. **Rate Limiting**: Implement rate limiting for manual block API
5. **Audit Logging**: Log all threat intelligence actions

## What's Next

Phase 6 will add:
- **Intelligent Auto-scaling**: Automatic fleet scaling based on CPU/traffic
- **Node Self-termination**: Graceful node shutdown commands
- **Resource Management**: Advanced fleet resource optimization
- **Infrastructure Integration**: Webhooks for external automation

Phase 7 will add:
- **Predictive AI**: Machine learning for threat prediction
- **Self-healing**: Automatic recovery from node failures
- **Advanced Analytics**: Behavioral pattern recognition
