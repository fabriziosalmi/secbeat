---
title: Observability
description: Real-time monitoring, metrics, and dynamic control for SecBeat
slug: core/observability
---

## Overview

SecBeat provides comprehensive observability through Extended Berkeley Packet Filter (eBPF) statistics, Prometheus metrics, and a REST Application Programming Interface (API) for dynamic control. The observability layer enables:

- Real-time packet statistics (PASS/DROP counters)
- Dynamic Internet Protocol (IP) blocklist management via REST API
- Prometheus-compatible metrics exposure
- Integration with management API for fleet-wide visibility

## eBPF Statistics

### Kernel-Side Counters

SecBeat uses eBPF `PerCpuArray` maps for high-performance statistics tracking without atomic operations:

```rust
#[map]
static STATS: PerCpuArray<u64> = PerCpuArray::with_max_entries(
    secbeat_common::STATS_ARRAY_SIZE as u32, 
    0
);
```

**Counter Types:**
- `STATS[0]` → Packets allowed (XDP_PASS)
- `STATS[1]` → Packets blocked (XDP_DROP)

**Performance Benefits:**
- Each Central Processing Unit (CPU) maintains its own counter (no contention)
- Userspace aggregates counts across all CPUs
- Minimal overhead in the packet processing fast path

### Reading Statistics

Access statistics through the management API:

```bash
# Get current packet statistics
curl http://localhost:9090/api/v1/stats
```

Expected output:
```json
{
  "packets_processed": 12543,
  "packets_passed": 12500,
  "packets_dropped": 43,
  "attacks_blocked": 43,
  "requests_per_second": 0,
  "latency_ms": 0.0,
  "cpu_percent": 0,
  "memory_mb": 0
}
```

## Dynamic Blocklist Management

### Unblock IP Address

Remove an IP from the kernel-level blocklist without restarting:

```bash
# Remove IP from blocklist
curl -X DELETE http://localhost:9090/api/v1/blocklist/192.168.100.12
```

Expected output:
```json
{
  "success": true,
  "message": "IP 192.168.100.12 removed from blocklist"
}
```

### API Endpoints

| Method | Endpoint | Description |
|--------|----------|-------------|
| `GET` | `/api/v1/stats` | Retrieve packet statistics |
| `DELETE` | `/api/v1/blocklist/:ip` | Remove IP from blocklist |
| `POST` | `/api/v1/blocklist` | Add IP to blocklist |
| `GET` | `/api/v1/health` | Health check endpoint |

## Prometheus Metrics

SecBeat exposes metrics in Prometheus format at `/metrics`:

```bash
# Scrape Prometheus metrics
curl http://localhost:9090/metrics
```

Expected output:
```text
# HELP secbeat_packets_total Total packets processed
# TYPE secbeat_packets_total counter
secbeat_packets_total{action="pass"} 12500
secbeat_packets_total{action="drop"} 43

# HELP secbeat_attacks_blocked Total attacks blocked
# TYPE secbeat_attacks_blocked counter
secbeat_attacks_blocked 43
```

### Grafana Integration

Import the SecBeat dashboard for visualization:

1. Add Prometheus datasource to Grafana
2. Import dashboard from `dashboard/` directory
3. Configure scrape interval (recommended: 15s)

## Distributed Observability

### NATS Event Stream

Mitigation nodes publish events to NATS for centralized monitoring:

```rust
// Example event published to NATS
{
  "node_id": "mitigation-1",
  "timestamp": "2025-11-24T00:50:00Z",
  "event_type": "attack_blocked",
  "source_ip": "203.0.113.42",
  "attack_pattern": "sql_injection"
}
```

### Fleet-Wide Statistics

The orchestrator node aggregates statistics from all mitigation nodes:

```bash
# Query orchestrator for fleet statistics
curl http://orchestrator:8080/api/v1/fleet/stats
```

## Performance Considerations

**eBPF Overhead:**
- PerCpuArray: ~10ns per counter increment
- HashMap lookup: ~50-100ns per packet
- Total XDP processing: <1µs per packet

**API Latency:**
- Local stats query: <1ms
- Blocklist modification: <5ms
- Prometheus scrape: <10ms for 10k metrics

## Troubleshooting

### Stats Show Zero

**Cause:** eBPF program not loaded or detached

**Solution:**
```bash
# Verify eBPF program is loaded
sudo bpftool prog list | grep secbeat

# Check kernel logs
sudo dmesg | tail -20
```

### Blocklist API Returns 500

**Cause:** Insufficient capabilities for eBPF map operations

**Solution:**
```bash
# Grant required capabilities
sudo setcap cap_net_admin,cap_bpf=eip ./mitigation-node
```

### High Memory Usage

**Cause:** PerCpuArray allocates memory per CPU core

**Expected Memory:**
- STATS map: 16 bytes × 2 entries × num_cpus
- Blocklist map: 1MB (default 10,000 entries)

## Next Steps

- [Configure Prometheus scraping](/reference/config#prometheus)
- [Set up Grafana dashboards](/enterprise/dashboard)
- [Enable distributed state sync](/enterprise/distributed-state)
