# 🚀 Phase 6: Intelligent Scaling & Node Self-Termination - Implementation Guide

## Overview

Phase 6 transforms the SecBeat system into a fully autonomous fleet that can intelligently scale itself based on real-time metrics and safely terminate nodes when needed. The system maintains a strict separation between decision-making (orchestrator) and infrastructure provisioning (external webhooks).

### New Capabilities

1. **Intelligent Scaling Decisions**: Resource Manager analyzes fleet-wide CPU metrics to determine when to scale up/down
2. **Infrastructure-Agnostic Provisioning**: Scale-up actions trigger generic webhooks (Ansible, Terraform, etc.)
3. **Node Self-Termination**: Mitigation nodes can gracefully shut themselves down via secure management API
4. **Fleet Stability**: Anti-flapping mechanisms prevent rapid scaling oscillations
5. **Graceful Shutdown**: Nodes drain connections before terminating

## Architecture Overview

```
┌─────────────────┐    Scaling Decision    ┌─────────────────┐
│                 │◄──────────────────────►│                 │
│ Resource Manager│                        │ Action Executor │
│   (Orchestrator)│                        │                 │
└─────────────────┘                        └─────────────────┘
         │                                           │
         │ Fleet Metrics                             │
         ▼                                           ▼
┌─────────────────┐                        ┌─────────────────┐
│   Node Registry │                        │ Webhook/Direct  │
│   (Heartbeats)  │                        │   Commands      │
└─────────────────┘                        └─────────────────┘
         ▲                                           │
         │                                           ▼
┌─────────────────┐    Management API     ┌─────────────────┐
│ Mitigation Node │◄──────────────────────►│  Infrastructure │
│ (Self-terminate)│                        │ (Ansible/etc.)  │
└─────────────────┘                        └─────────────────┘
```

## Configuration

### Orchestrator Configuration

The orchestrator now includes scaling parameters:

```toml
# config/orchestrator.toml (if using file-based config)
provisioning_webhook_url = "http://localhost:8000/provision"
min_fleet_size = 1
scale_up_cpu_threshold = 0.80     # 80%
scale_down_cpu_threshold = 0.30   # 30%
scaling_check_interval_seconds = 60
```

**Default values in code:**
- `provisioning_webhook_url`: "http://localhost:8000/provision"
- `min_fleet_size`: 1
- `scale_up_cpu_threshold`: 0.80 (80%)
- `scale_down_cpu_threshold`: 0.30 (30%)
- `scaling_check_interval_seconds`: 60

### Mitigation Node Configuration

Each mitigation node includes management API settings:

```toml
# config/default.toml
[management]
enabled = true
listen_addr = "0.0.0.0:9999"
auth_token = "secure-management-token-change-in-production"
shutdown_grace_period = 60
```

## Quick Start

1. **Start the complete system:**
   ```bash
   cd /Users/fab/GitHub/secbeat
   chmod +x test_phase6.sh
   ./test_phase6.sh
   ```

2. **Manual startup:**
   ```bash
   # Terminal 1: NATS Server
   nats-server --port 4222 --http_port 8222
   
   # Terminal 2: Orchestrator with Resource Manager
   cd orchestrator-node && cargo run
   
   # Terminal 3: Test Origin Server
   cd mitigation-node && cargo run --bin test-origin
   
   # Terminal 4: Mitigation Node with Management API
   cd mitigation-node && cargo run --bin mitigation-node
   ```

## Key Features

### 1. Resource Manager Expert

The Resource Manager runs in the orchestrator and:
- **Monitors**: Analyzes fleet-wide CPU utilization every 60 seconds
- **Decides**: Triggers scaling actions based on configurable thresholds
- **Prevents Flapping**: Requires 2 consecutive checks for scale-up, 5 for scale-down
- **Selects Targets**: Chooses nodes with lowest connection counts for termination

#### Scale-Up Logic
```rust
// Trigger when average CPU > 80% for 2+ consecutive checks
if fleet_avg_cpu > 0.80 && consecutive_checks >= 2 {
    call_provisioning_webhook("HIGH_FLEET_CPU_LOAD").await
}
```

#### Scale-Down Logic
```rust
// Trigger when average CPU < 30% for 5+ consecutive checks
// AND fleet size > minimum
if fleet_avg_cpu < 0.30 && consecutive_checks >= 5 && nodes > min_fleet_size {
    terminate_node_with_lowest_connections().await
}
```

### 2. Infrastructure-Agnostic Scaling

#### Scale-Up Webhook
When scaling up is needed, the orchestrator POSTs to the configured webhook:

```json
{
    "reason": "HIGH_FLEET_CPU_LOAD",
    "timestamp": "2025-07-12T10:30:45Z",
    "fleet_metrics": {
        "active_nodes": 2,
        "avg_cpu_usage": 0.85,
        "avg_memory_usage": 0.65,
        "total_connections": 150
    }
}
```

**Integration Examples:**
- **Ansible Tower**: Job template trigger URL
- **Terraform Cloud**: Run trigger API
- **Proxmox**: Custom provisioning script
- **AWS/GCP**: Lambda/Cloud Function

### 3. Node Self-Termination API

Each mitigation node exposes a secure management API on port 9999:

#### Authentication
```bash
Authorization: Bearer secure-management-token-change-in-production
```

#### Termination Endpoint
```http
POST /control/terminate
Content-Type: application/json
Authorization: Bearer <token>

{
    "reason": "LOW_FLEET_CPU_LOAD",
    "timestamp": "2025-07-12T10:30:45Z",
    "grace_period_seconds": 60
}
```

#### Response
```json
{
    "success": true,
    "message": "Graceful shutdown initiated with 60 second grace period",
    "grace_period_seconds": 60
}
```

### 4. Graceful Shutdown Process

When a termination command is received:

1. **Authentication**: Verify bearer token
2. **Accept Command**: Return 202 Accepted immediately  
3. **Signal Shutdown**: Set atomic shutdown flag
4. **Stop New Connections**: TCP listener stops accepting
5. **Grace Period**: Wait for in-flight requests (60s default)
6. **Final Heartbeat**: Send "Terminating" status to orchestrator
7. **Exit**: Process terminates with exit code 0

## API Endpoints

### Orchestrator (Port 3030)

| Endpoint | Method | Description |
|----------|--------|-------------|
| `/api/v1/fleet/stats` | GET | Fleet statistics with scaling metrics |
| `/api/v1/nodes` | GET | List all nodes with detailed status |
| `/api/v1/rules/block_ip` | POST | Threat intelligence (from Phase 5) |
| `/api/v1/rules/blocked_ips` | GET | View blocked IPs (from Phase 5) |

### Mitigation Node Management API (Port 9999)

| Endpoint | Method | Description |
|----------|--------|-------------|
| `/control/terminate` | POST | Initiate graceful node shutdown |

## Testing Phase 6 Features

### 1. Test Resource Manager
```bash
# View current fleet metrics
curl http://127.0.0.1:3030/api/v1/fleet/stats

# Resource Manager logs scaling decisions every 60 seconds
tail -f logs/orchestrator.log | grep -i "resource\|scaling"
```

### 2. Test Scale-Up Simulation
```bash
# Generate high CPU load to trigger scaling
for i in {1..100}; do 
    curl -k https://127.0.0.1:8443/api/test
done

# Watch for webhook calls in orchestrator logs
# Note: Webhook will fail unless you have a receiver on port 8000
```

### 3. Test Node Termination
```bash
# Unauthorized access (should return 401)
curl -X POST http://127.0.0.1:9999/control/terminate

# Authorized termination
curl -X POST http://127.0.0.1:9999/control/terminate \
  -H "Authorization: Bearer secure-management-token-change-in-production" \
  -H "Content-Type: application/json" \
  -d '{
    "reason": "Scale-down test",
    "timestamp": "'$(date -u +%Y-%m-%dT%H:%M:%SZ)'",
    "grace_period_seconds": 30
  }'
```

### 4. Test Webhook Receiver
```bash
# Simple webhook receiver for testing
python3 -c "
import http.server
import socketserver
import json

class WebhookHandler(http.server.BaseHTTPRequestHandler):
    def do_POST(self):
        content_length = int(self.headers['Content-Length'])
        body = self.rfile.read(content_length)
        print(f'Received webhook: {body.decode()}')
        self.send_response(200)
        self.end_headers()
        self.wfile.write(b'OK')

with socketserver.TCPServer(('', 8000), WebhookHandler) as httpd:
    print('Webhook receiver listening on port 8000...')
    httpd.serve_forever()
"
```

## Production Deployment

### 1. Security Configuration
```toml
[management]
auth_token = "$(openssl rand -hex 32)"  # Generate secure token
listen_addr = "127.0.0.1:9999"         # Bind to localhost only
```

### 2. Infrastructure Integration

#### Ansible Tower Integration
```bash
# Set provisioning webhook to Ansible Tower job template
provisioning_webhook_url = "https://tower.example.com/api/v2/job_templates/123/launch/"

# Include authentication headers in ResourceManager
Authorization: Bearer <tower-token>
```

#### Terraform Cloud Integration
```bash
# Use Terraform Cloud run triggers
provisioning_webhook_url = "https://app.terraform.io/api/v2/runs"

# Payload includes workspace and configuration
```

### 3. Monitoring & Alerting

Monitor these key metrics:
- `secbeat_scaling_decisions_total{action="scale_up"}`
- `secbeat_scaling_decisions_total{action="scale_down"}`
- `secbeat_node_terminations_total`
- `secbeat_fleet_size_current`
- `secbeat_webhook_calls_total{status="success|failure"}`

## Troubleshooting

### Resource Manager Issues
```bash
# Check scaling configuration
curl http://127.0.0.1:3030/api/v1/fleet/stats

# Monitor scaling decisions
tail -f logs/orchestrator.log | grep "ResourceManager"
```

### Webhook Failures
- Verify webhook URL is accessible
- Check authentication/authorization
- Monitor webhook receiver logs
- Test with curl/Postman first

### Node Termination Issues
- Verify management API port (9999) is accessible
- Check authentication token
- Monitor grace period in logs
- Ensure no firewall blocking

### Fleet Instability
- Check for flapping (rapid scale up/down)
- Adjust thresholds if needed
- Increase consecutive check requirements
- Monitor node heartbeat patterns

## Security Considerations

1. **Management API Security**: Use strong authentication tokens, restrict network access
2. **Webhook Security**: Validate webhook endpoints, use HTTPS, implement rate limiting
3. **Graceful Shutdown**: Ensure proper connection draining to avoid service disruption
4. **Fleet Stability**: Monitor for scaling storms and adjust thresholds appropriately

## What's Next

Phase 7 will add:
- **Predictive AI**: Machine learning models for proactive scaling
- **Self-Healing**: Automatic recovery from node failures
- **Advanced Analytics**: Behavioral pattern recognition for threat detection
- **Multi-Region Support**: Cross-region fleet management and scaling
