# Behavioral Analysis Testing Guide

## Overview

This guide covers testing the Real-Time Behavioral Analysis Expert, a distributed anomaly detection system that automatically blocks malicious IPs based on traffic patterns.

## Architecture

```
┌─────────────────────┐
│  Mitigation Node    │ ← HTTP requests from clients
│  (Edge Security)    │
└──────────┬──────────┘
           │ Publishes telemetry events
           │ Topic: secbeat.telemetry.{node_id}
           ↓
    ┌──────────────┐
    │     NATS     │ ← Message bus
    └──────────────┘
           ↓
┌──────────────────────┐
│   Orchestrator       │ ← BehavioralExpert analyzes
│  (Control Plane)     │   sliding window
└──────────┬───────────┘
           │ Publishes block commands
           │ Topic: secbeat.commands.block
           ↓
    ┌──────────────┐
    │     NATS     │
    └──────────────┘
           ↓
┌─────────────────────┐
│  Mitigation Node    │ ← Receives ban, updates
│  (Edge Security)    │   DynamicRuleState
└─────────────────────┘
           ↓
    Future requests from IP → HTTP 403 Forbidden
```

## Detection Algorithms

### Sliding Window Algorithm

The behavioral expert tracks IP behavior over a 60-second sliding window:

- **Window Size**: 60 seconds (configurable)
- **Error Threshold**: 50 errors (4xx/5xx status codes)
- **Request Threshold**: 1000 requests (high-frequency spike)

### Anomaly Detection

1. **Error Rate Anomaly**: If an IP generates 50+ errors in 60 seconds → block for 5 minutes
2. **High-Frequency Spike**: If an IP makes 1000+ requests in 60 seconds → block for 5 minutes

### Memory Management

- **Cleanup Interval**: Every 5 minutes
- **Timestamp Pruning**: Events older than window_size are removed
- **Empty IP Removal**: IPs with no recent activity are purged
- **Duplicate Prevention**: Recently blocked IPs are tracked to avoid redundant commands

## Testing Methods

### Method 1: Full End-to-End Test (Recommended)

```bash
./test_behavioral_ban.sh
```

**What it tests:**
- ✅ Service availability and baseline health
- ✅ Error flood attack simulation (60 sequential 404s)
- ✅ Telemetry event publishing (Mitigation Node → NATS)
- ✅ Behavioral analysis (Orchestrator sliding window algorithm)
- ✅ Block command publishing (Orchestrator → NATS)
- ✅ Dynamic IP blocking (Mitigation Node enforcement)
- ✅ Ban verification (HTTP 403 or connection refused)

**Expected Output:**
```
╔════════════════════════════════════════════════════════════╗
║                  🎉 TEST PASSED! 🎉                        ║
╚════════════════════════════════════════════════════════════╝
✅ Request was blocked (HTTP 403 Forbidden)
✅ Behavioral Analysis Expert successfully detected anomaly
✅ NATS message propagation working
✅ Dynamic IP blocking enforced
```

### Method 2: Quick Test (Development)

```bash
./test_behavioral_quick.sh
```

Simplified version for rapid testing during development.

### Method 3: Makefile Targets

```bash
# Full behavioral test
make test-behavioral

# Quick test
make test-behavioral-quick
```

### Method 4: Manual Testing

```bash
# 1. Generate errors
for i in {1..60}; do
    curl -s http://localhost:8443/attack-$i &
done
wait

# 2. Wait for analysis
sleep 5

# 3. Verify ban
curl -v http://localhost:8443/health
# Should return: HTTP 403 Forbidden
```

## Unit Tests

Run the behavioral expert unit tests:

```bash
cd orchestrator-node
cargo test --bin orchestrator-node experts::behavioral::tests
```

**Test Coverage:**
- ✅ `test_error_flood_triggers_block` - Error rate anomaly detection
- ✅ `test_request_flood_triggers_block` - High-frequency spike detection
- ✅ `test_sliding_window_pruning` - Old event cleanup
- ✅ `test_cleanup_removes_inactive_ips` - Memory management

## Configuration

### Behavioral Expert Config (Orchestrator)

File: `orchestrator-node/src/main.rs`

```rust
let behavioral_config = BehavioralConfig {
    window_size_seconds: 60,      // Sliding window duration
    error_threshold: 50,           // Errors to trigger ban
    request_threshold: 1000,       // Requests to trigger ban
    block_duration_seconds: 300,   // Ban duration (5 minutes)
    cleanup_interval_seconds: 300, // Cleanup frequency
};
```

### NATS Topics

- **Telemetry**: `secbeat.telemetry.{node_id}` (Mitigation → Orchestrator)
- **Block Commands**: `secbeat.commands.block` (Orchestrator → Mitigation)
- **WAF Events**: `secbeat.events.waf` (Mitigation → Orchestrator)

## Troubleshooting

### Test Fails: Request Not Blocked

**Possible causes:**

1. **NATS not running**
   ```bash
   docker-compose ps | grep nats
   docker-compose logs nats
   ```

2. **Orchestrator not receiving telemetry**
   ```bash
   docker-compose logs orchestrator | grep -i telemetry
   docker-compose logs orchestrator | grep -i behavioral
   ```

3. **Threshold not reached**
   - Check: Did you send enough errors? (default: 50)
   - Check: Were they within the 60-second window?

4. **Mitigation node not receiving block commands**
   ```bash
   docker-compose logs mitigation-node | grep -i block
   docker-compose logs mitigation-node | grep -i command
   ```

### Monitoring NATS Topics

Subscribe to NATS topics to debug message flow:

```bash
# Watch telemetry events
docker-compose exec nats nats sub 'secbeat.telemetry.>'

# Watch block commands
docker-compose exec nats nats sub 'secbeat.commands.block'

# Watch WAF events
docker-compose exec nats nats sub 'secbeat.events.waf'
```

### Check Blocked IP Count

```bash
# Mitigation node API
curl http://localhost:8080/api/v1/stats | jq '.blocked_ips'

# Check orchestrator logs
docker-compose logs orchestrator | grep "Block command generated"
```

### Verify Sliding Window

The sliding window should prune old events automatically:

```bash
# Send 10 errors
for i in {1..10}; do curl -s http://localhost:8443/err-$i; done

# Wait 61 seconds (window_size + 1)
sleep 61

# Send 50 more errors - should trigger ban
for i in {1..50}; do curl -s http://localhost:8443/err2-$i; done
```

## Performance Considerations

### Non-Blocking Telemetry

Telemetry events are published asynchronously using `tokio::spawn` to avoid blocking request processing:

```rust
pub fn publish_telemetry_event(&self, event: TelemetryEvent) {
    let client = self.nats_client.clone();
    tokio::spawn(async move {
        // Publish without blocking
    });
}
```

### Memory Safety

The cleanup task runs every 5 minutes to prevent unbounded memory growth:

```rust
pub fn spawn_cleanup_task(self: Arc<Self>) {
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_secs(300));
        loop {
            interval.tick().await;
            self.cleanup().await;
        }
    });
}
```

### Error-Only Publishing

To reduce NATS load, telemetry is only published for error responses (4xx/5xx):

```rust
if let Some(status) = status_code {
    if status >= 400 {
        publish_telemetry_event(&state, ...);
    }
}
```

## Advanced Testing

### Simulating Different Attack Patterns

**Slow Attack (Below Threshold)**
```bash
# 40 errors over 60 seconds (below threshold of 50)
for i in {1..40}; do
    curl -s http://localhost:8443/slow-$i
    sleep 1.5
done
# Should NOT trigger ban
```

**Burst Attack (Above Threshold)**
```bash
# 60 errors in rapid succession (exceeds threshold)
for i in {1..60}; do curl -s http://localhost:8443/burst-$i & done
wait
# Should trigger ban
```

**Distributed Attack (Multiple IPs)**
```bash
# Use different source IPs (requires routing configuration)
# Each IP tracked independently in sliding window
```

### Testing TTL Expiration

```bash
# 1. Trigger ban
./test_behavioral_quick.sh

# 2. Wait for TTL expiration (5 minutes)
sleep 300

# 3. Verify ban lifted
curl http://localhost:8443/health
# Should return: HTTP 200 OK
```

## Integration with CI/CD

Add to your CI pipeline:

```yaml
# .github/workflows/test.yml
- name: Test Behavioral Analysis
  run: |
    docker-compose up -d
    sleep 10
    make test-behavioral-quick
```

## Production Recommendations

1. **Tune Thresholds**: Adjust based on your traffic patterns
2. **Monitor False Positives**: Use Grafana dashboards to track blocked IPs
3. **Allowlist Critical IPs**: Implement IP allowlist for internal services
4. **Alert on High Ban Rate**: Set up alerts if ban rate exceeds normal
5. **Review Ban Logs**: Regularly audit behavioral ban decisions

## Next Steps

- Review [NATS Documentation](https://docs.nats.io/) for advanced messaging patterns
- Explore [Orchestrator API](docs/api.md) for manual IP management
- Implement custom detection algorithms in `behavioral.rs`
- Add machine learning models for advanced threat detection (Q3 2025 roadmap)
