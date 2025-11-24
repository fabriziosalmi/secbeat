---
title: Dynamic Rules
description: Machine Learning-generated WASM rules for adaptive security
---

## Overview

Dynamic Rules transform Machine Learning (ML) anomaly detection into executable WebAssembly (WASM) security policies, enabling **behavior-based blocking** instead of just Internet Protocol (IP)-based blocking.

## The Problem with Static Rules

**Traditional Web Application Firewall (WAF) Limitations:**
- Attackers rotate IPs (botnets, Virtual Private Networks (VPNs), proxies)
- Blocks expire (e.g., 1-hour ban)
- New IPs bypass previous defenses
- Manual rule updates lag behind threats

**Example Attack:**
```
203.0.113.10 → 100 SQL injection attempts → BLOCKED for 1 hour
203.0.113.11 → 100 SQL injection attempts → ALLOWED (new IP)
203.0.113.12 → 100 SQL injection attempts → ALLOWED (new IP)
...
```

The attacker cycles through IPs faster than blocks expire.

## Behavior-Based Dynamic Rules

**Our Solution**: Detect attack **patterns**, generate **WASM rules** that match behavior, not IPs.

**Example Generated Rule:**
```rust
// Auto-generated from ML anomaly detection
if request.uri.contains("/wp-admin") 
   && request.headers.get("user-agent").map_or(false, |ua| ua.len() < 20)
   && request.method == "POST" {
    return Action::Block;  // WordPress scanner pattern
}
```

This blocks the **attack pattern** regardless of source IP.

## Architecture

```mermaid
graph LR
    A[Traffic Patterns] --> B[ML Anomaly Detection]
    B --> C[Pattern Extraction]
    C --> D[Rule Generator]
    D --> E[WASM Module]
    E --> F[NATS Distribution]
    F --> G[Mitigation Fleet]
```

1. **ML Expert** detects anomalies (e.g., "High POST rate to /wp-admin with short User-Agent")
2. **Rule Generator** converts to JSON configuration
3. **WASM Module** (universal-waf) applies rules dynamically
4. **Orchestrator** deploys fleet-wide via NATS
5. **Mitigation Nodes** execute rule on all traffic

## Universal WAF Module

The `universal-waf` WASM module reads **data-driven** JSON rules:

```json
{
  "rules": [
    {
      "id": "block-sqli-pattern",
      "field": "URI",
      "pattern": "*' OR *--*",
      "action": "Block"
    },
    {
      "id": "block-short-ua-admin",
      "field": "Header:User-Agent",
      "pattern": ".",
      "max_length": 20,
      "requires": {"URI": "/admin/*"},
      "action": "RateLimit"
    }
  ]
}
```

## Dynamic Rule Lifecycle

### 1. Detection Phase

```bash
# ML model detects anomaly
[2025-11-24T01:00:00Z] ANOMALY DETECTED
  Type: path_traversal
  Source IPs: 203.0.113.{10-50} (41 unique)
  Pattern: /../../etc/passwd
  Confidence: 0.97
```

### 2. Generation Phase

Orchestrator generates JSON rule:

```json
{
  "id": "block_path_traversal_2025_11_24",
  "field": "URI",
  "pattern": "*../*",
  "action": "Block",
  "ttl_seconds": 3600
}
```

### 3. Deployment Phase

```bash
# Deploy to fleet via NATS
curl -X POST http://orchestrator:8080/api/v1/rules/deploy \
  -d '{"rule_id": "block_path_traversal_2025_11_24", "ttl_seconds": 3600}'

# Expected output:
# {"deployed_to": 10, "failed": 0, "deployment_time_ms": 342}
```

### 4. Execution Phase

```bash
# Rule blocks matching requests
[2025-11-24T01:00:15Z] BLOCKED by dynamic rule
  Rule: block_path_traversal_2025_11_24
  Source: 203.0.113.99 (NEW IP)
  URI: /admin/../../etc/passwd
  Action: Block
```

Notice: **New IP blocked** because pattern matched, not IP.

### 5. Expiration Phase

```bash
# Rule expires after Time To Live (TTL)
[2025-11-24T02:00:05Z] RULE EXPIRED
  Name: block_path_traversal_2025_11_24
  Lifetime: 3600s
  Requests Blocked: 1,247
  False Positives: 0
```

Rules auto-expire to prevent stale blocks.

## Configuration

```toml
# config.prod.toml
[ml.dynamic_rules]
enabled = true
min_confidence = 0.85  # Only generate rules for high-confidence anomalies
max_active_rules = 100  # Prevent rule explosion
default_ttl_seconds = 3600  # 1 hour auto-expiration
auto_deploy = true  # Deploy without manual approval

[waf.wasm]
module = "universal-waf.wasm"  # Data-driven WAF module
fuel_limit = 50000  # Execution limit
```

## Monitoring

### View Active Dynamic Rules

```bash
curl http://localhost:9090/api/v1/rules/active

# Expected output:
# {
#   "rules": [
#     {
#       "id": "block_sqli_2025_11_24_001",
#       "pattern": "SQL injection signature",
#       "created_at": "2025-11-24T00:30:00Z",
#       "expires_at": "2025-11-24T01:30:00Z",
#       "blocks": 342
#     }
#   ]
# }
```

### Rule Effectiveness

```bash
curl http://localhost:9090/api/v1/rules/stats

# Expected output:
# {
#   "total_dynamic_rules": 15,
#   "active_rules": 8,
#   "total_blocks": 12547,
#   "false_positive_rate": 0.02
# }
```

## Performance Impact

| Metric | Without Dynamic Rules | With Dynamic Rules |
|--------|----------------------|--------------------|
| Latency | 0.5ms | 0.7ms (+40%) |
| Throughput | 50K req/s | 48K req/s (-4%) |
| Attack Block Rate | 60% | 95% (+58%) |

## Learn More

- [WASM Runtime](/intelligence/wasm-runtime)
- [Hot Reload Guide](/intelligence/hot-reload)
- [ML Overview](/core/overview#ml-capabilities)
