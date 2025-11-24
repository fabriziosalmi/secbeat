---
title: Hot Reload
description: Update WASM rules without downtime
---

## Overview

Hot Reload enables **zero-downtime updates** to Web Application Firewall (WAF) rules, allowing you to deploy new security policies without restarting mitigation nodes.

## Why Hot Reload?

**Traditional Deployment:**
1. Update WAF rule code
2. Recompile binary
3. Stop mitigation service
4. Deploy new binary
5. Restart service

**Downtime**: 10-60 seconds during restart  
**Risk**: Unprotected traffic during update  
**Coordination**: Requires orchestration across fleet

**With Hot Reload:**
1. Update WebAssembly (WASM) module
2. Send reload command via Application Programming Interface (API)
3. Module swaps atomically

**Downtime**: 0 milliseconds  
**Risk**: None - old module serves until new ready  
**Coordination**: NATS handles distribution

## How It Works

### Atomic Module Swap

```rust
pub struct WasmEngine {
    current: Arc<RwLock<Module>>,  // Active module serving requests
    staging: Arc<RwLock<Option<Module>>>,  // New module being loaded
}

impl WasmEngine {
    pub async fn reload(&self, new_wasm: &[u8]) -> Result<()> {
        // 1. Load new module in staging (doesn't affect traffic)
        let new_module = Module::from_binary(new_wasm)?;
        *self.staging.write().await = Some(new_module);
        
        // 2. Atomic swap (single instruction)
        let mut current = self.current.write().await;
        if let Some(staged) = self.staging.write().await.take() {
            *current = staged;  // Instant swap
        }
        
        Ok(())
    }
}
```

Requests use `current` module - swap is **atomic** (no torn reads).

## Reload Methods

### 1. Manual Reload via API

```bash
# Upload new WASM module
curl -X POST http://localhost:9090/api/v1/wasm/reload \
  -F "name=bad-bot-blocker" \
  -F "file=@updated.wasm"

# Expected output:
# {
#   "success": true,
#   "message": "Module reloaded",
#   "version": "v2",
#   "downtime_ms": 0,
#   "requests_during_reload": 1247
# }
```

All 1,247 requests served **without interruption**.

### 2. Orchestrated Fleet Reload

```bash
# Reload across entire fleet via orchestrator
curl -X POST http://orchestrator:8080/api/v1/wasm/reload-fleet \
  -F "file=@universal-waf-v2.wasm" \
  -F "target=all"

# Expected output:
# {
#   "success": true,
#   "deployed_to": ["node-1", "node-2", "node-3"],
#   "failed": [],
#   "total_downtime_ms": 0
# }
```

### 3. Automatic Reload on Rule Update

Dynamic rules trigger automatic reload:

```bash
# ML generates new rule
[2025-11-24T01:00:00Z] New dynamic rule: block_sqli_v2

# Orchestrator updates universal-waf config
[2025-11-24T01:00:01Z] Updated waf-config.json with new rule

# NATS publishes reload event
[2025-11-24T01:00:02Z] NATS: secbeat.wasm.reload

# All nodes reload automatically
[2025-11-24T01:00:03Z] Node-1: Reloaded universal-waf (0ms downtime)
[2025-11-24T01:00:03Z] Node-2: Reloaded universal-waf (0ms downtime)
[2025-11-24T01:00:03Z] Node-3: Reloaded universal-waf (0ms downtime)
```

## Versioning and Rollback

### Version Tracking

```bash
# View current module version
curl http://localhost:9090/api/v1/wasm/version

# Expected output:
# {
#   "name": "universal-waf",
#   "version": "v2",
#   "loaded_at": "2025-11-24T01:00:03Z",
#   "previous_version": "v1",
#   "size_bytes": 87040
# }
```

### Rollback to Previous Version

```bash
# If new version has issues, rollback
curl -X POST http://localhost:9090/api/v1/wasm/rollback

# Expected output:
# {
#   "success": true,
#   "rolled_back_to": "v1",
#   "reason": "Manual rollback",
#   "downtime_ms": 0
# }
```

Previous module kept in memory for instant rollback.

## Safety Guarantees

### Pre-Reload Validation

```rust
// Validate WASM module before loading
pub async fn reload_safe(&self, new_wasm: &[u8]) -> Result<()> {
    // 1. Verify WASM format
    let module = Module::from_binary(new_wasm)
        .map_err(|e| Error::InvalidWasm(e))?;
    
    // 2. Check required exports
    if !module.has_export("inspect_request") {
        return Err(Error::MissingExport);
    }
    
    // 3. Test with sample request
    let test_req = create_test_request();
    let result = module.call("inspect_request", test_req)?;
    if result < 0 || result > 3 {
        return Err(Error::InvalidAction);
    }
    
    // 4. All checks passed - reload
    self.reload_atomic(module).await
}
```

### Graceful Degradation

If reload fails:

```bash
# Reload with invalid WASM
curl -X POST http://localhost:9090/api/v1/wasm/reload \
  -F "file=@corrupted.wasm"

# Expected output:
# {
#   "success": false,
#   "error": "Invalid WASM module: missing inspect_request export",
#   "current_version": "v1",
#   "status": "still_serving_v1"
# }
```

Old module **continues serving** - no impact to traffic.

## Configuration

```toml
# config.prod.toml
[waf.wasm]
module = "universal-waf.wasm"
hot_reload_enabled = true
keep_previous_versions = 2  # For rollback
validate_before_reload = true

[waf.wasm.reload]
auto_reload_on_file_change = false  # Use API/NATS instead
max_reload_frequency_seconds = 10  # Rate limit reloads
```

## Monitoring Hot Reloads

### Prometheus Metrics

```bash
# Check reload metrics
curl http://localhost:9090/metrics | grep wasm_reload

# Expected output:
# wasm_reload_total{module="universal-waf"} 15
# wasm_reload_failures_total{module="universal-waf"} 0
# wasm_reload_duration_seconds{module="universal-waf"} 0.008
```

### Reload History

```bash
curl http://localhost:9090/api/v1/wasm/history

# Expected output:
# {
#   "reloads": [
#     {
#       "timestamp": "2025-11-24T01:00:03Z",
#       "version": "v2",
#       "duration_ms": 8,
#       "requests_during_reload": 1247,
#       "success": true
#     },
#     {
#       "timestamp": "2025-11-24T00:30:00Z",
#       "version": "v1",
#       "duration_ms": 12,
#       "requests_during_reload": 891,
#       "success": true
#     }
#   ]
# }
```

## Best Practices

**Test Before Fleet Reload:**
```bash
# 1. Deploy to canary node first
curl -X POST http://node-1:9090/api/v1/wasm/reload -F "file=@new.wasm"

# 2. Monitor for errors (5 minutes)
curl http://node-1:9090/api/v1/wasm/errors

# 3. If clean, deploy to fleet
curl -X POST http://orchestrator:8080/api/v1/wasm/reload-fleet -F "file=@new.wasm"
```

**Version All Modules:**
- Embed version in WASM metadata
- Use semantic versioning (v1.0.0)
- Tag Git commits with WASM versions

**Keep Rollback Window:**
- Retain previous 2 versions in memory
- Archive all versions to object storage
- Document rollback procedure

## Troubleshooting

### Reload Timeout

**Error**: `Reload timed out after 30s`

**Cause**: New WASM module too large or slow to compile

**Solution**:
```bash
# Optimize WASM module
wasm-opt -Oz --strip-debug -o optimized.wasm original.wasm

# Check size
ls -lh optimized.wasm
# Should be <500 KB
```

### Version Mismatch

**Error**: `Version conflict: expected v2, got v1`

**Cause**: Partial fleet reload - some nodes still on old version

**Solution**:
```bash
# Force fleet sync
curl -X POST http://orchestrator:8080/api/v1/wasm/sync-versions
```

## Learn More

- [WASM Runtime](/intelligence/wasm-runtime)
- [Dynamic Rules](/intelligence/dynamic-rules)
- [Fleet Management](/enterprise/distributed-state)
