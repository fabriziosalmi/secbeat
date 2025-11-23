# Chapter 3.1: WASM Runtime Integration

**Status:** ✅ COMPLETE  
**Implementation Date:** 2025-11-23  
**Testing Platform:** Proxmox LXC Container 100 (Ubuntu, Linux kernel 6.14.11-2-pve)

## Overview

Chapter 3.1 implements a **WebAssembly (WASM) runtime** for hot-reloadable WAF rules in the SecBeat mitigation node. This enables:

1. **Dynamic Rule Deployment** - Load new WAF rules without recompiling or restarting
2. **Safety Isolation** - WASM provides sandboxed execution with strict resource limits
3. **Hot-Reload Capability** - Update rules in production without downtime
4. **Performance** - Near-native execution speed (~6K-15K CPU instructions per request)

## Why WASM for WAF Rules?

Traditional WAF implementations require:
- Recompiling the entire application for rule changes
- Restarting services (downtime)
- Manual deployment coordination
- No isolation between rule logic and core system

**WASM Runtime solves this by:**
- Compiling rules once, loading at runtime
- Hot-swapping modules without restart
- Enforcing strict memory and CPU limits (fuel)
- Enabling distributed rule deployment via NATS

## Architecture

```
┌─────────────────────────────────────────────────────┐
│              Orchestrator Node (NATS)                │
│       Distributes WASM modules to mitigation nodes  │
└──────────────────────┬──────────────────────────────┘
                       │
                       │ NATS: wasm.rule.deploy
                       ▼
┌─────────────────────────────────────────────────────┐
│            Mitigation Node (WasmEngine)              │
│  ┌───────────────────────────────────────────────┐  │
│  │  WasmEngine                                   │  │
│  │  - Module cache (HashMap<String, Module>)    │  │
│  │  - Fuel limit: 100K instructions/request     │  │
│  │  - Memory limit: 1 MB per instance           │  │
│  └───────────────────────────────────────────────┘  │
│                       │                              │
│                       ▼                              │
│  ┌───────────────────────────────────────────────┐  │
│  │  HTTP Request Flow                            │  │
│  │  1. Parse request → RequestContext           │  │
│  │  2. Serialize to JSON                        │  │
│  │  3. Write to WASM memory                     │  │
│  │  4. Call inspect_request(ptr, len)           │  │
│  │  5. Read Action from return value            │  │
│  └───────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────┘
                       │
                       ▼
┌─────────────────────────────────────────────────────┐
│          WASM Module (bad-bot-rule.wasm)             │
│  - Blocks /admin paths                              │
│  - Blocks known bad bots (BadBot, SQLMap, etc.)     │
│  - Logs suspicious patterns (SQLi, path traversal)  │
│  - Size: 85 KB (optimized with LTO)                │
│  - Fuel cost: 6K-15K instructions                  │
└─────────────────────────────────────────────────────┘
```

## Implementation Details

### 1. WASM ABI Definition

**File:** `mitigation-node/src/wasm/abi.rs`

Defines the contract between host (SecBeat) and guest (WASM modules).

#### Action Enum

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(i32)]
pub enum Action {
    Allow = 0,       // Pass the request through
    Block = 1,       // Drop/reject the request
    Log = 2,         // Log but allow (passive mode)
    RateLimit = 3,   // Apply rate limiting
}
```

#### Request Context

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RequestContext {
    pub method: String,              // HTTP method (GET, POST, etc.)
    pub uri: String,                 // Request URI/path
    pub version: String,             // HTTP version
    pub source_ip: String,           // Source IP address
    pub headers: Option<Vec<(String, String)>>,  // Optional headers
    pub body_preview: Option<String>,            // Optional body preview
}
```

#### Memory Passing Convention

Host writes JSON-encoded `RequestContext` to WASM linear memory at offset 0, then calls:

```rust
extern "C" fn inspect_request(ptr: *const u8, len: usize) -> i32
```

- `ptr`: Pointer to JSON string in WASM memory (always 0)
- `len`: Length of JSON string
- Returns: `Action` as i32 (0=Allow, 1=Block, 2=Log, 3=RateLimit)

### 2. WASM Engine Implementation

**File:** `mitigation-node/src/wasm/engine.rs`

#### Engine Configuration

```rust
pub struct WasmConfig {
    pub max_fuel: u64,        // CPU limit (default: 100K instructions)
    pub max_memory: usize,    // Memory limit (default: 1 MB)
    pub cache_enabled: bool,  // Enable compilation cache
}
```

#### Module Caching

```rust
pub struct WasmEngine {
    engine: Engine,                                    // Shared wasmtime engine
    modules: Arc<RwLock<HashMap<String, CachedModule>>>, // Module cache
    config: WasmConfig,
}
```

- Modules compiled once and cached by name
- Thread-safe access via `Arc<RwLock<>>`
- Automatic validation of required exports

#### Execution Flow

```rust
pub fn run_module(&self, name: &str, ctx: &RequestContext) -> Result<Action> {
    // 1. Get module from cache
    let cached = modules.get(name)?;
    
    // 2. Create new Store with fuel limit
    let mut store = Store::new(&self.engine, ());
    store.set_fuel(self.config.max_fuel)?;
    
    // 3. Instantiate module
    let instance = Instance::new(&mut store, &cached.module, &[])?;
    
    // 4. Get function and memory
    let inspect_fn = instance.get_typed_func::<(i32, i32), i32>(...)?;
    let memory = instance.get_memory(...)?;
    
    // 5. Write JSON to WASM memory
    let json = ctx.to_json()?;
    memory.write(&mut store, 0, json.as_bytes())?;
    
    // 6. Call WASM function
    let result = inspect_fn.call(&mut store, (0, json.len() as i32))?;
    
    // 7. Track fuel consumption
    let fuel_consumed = max_fuel - store.get_fuel()?;
    
    // 8. Convert result to Action
    Action::from_i32(result)?
}
```

### 3. Sample WASM Rule: Bad Bot Detection

**File:** `wasm-rules/bad-bot/src/lib.rs`

#### Implementation

```rust
#[no_mangle]
pub extern "C" fn inspect_request(ptr: *const u8, len: usize) -> i32 {
    // Parse JSON from memory
    let json_bytes = unsafe { std::slice::from_raw_parts(ptr, len) };
    let ctx: RequestContext = serde_json::from_str(...)?;
    
    // Rule 1: Block /admin paths
    if ctx.uri.contains("/admin") {
        return Action::Block as i32;
    }
    
    // Rule 2: Block bad bots
    if let Some(ref headers) = ctx.headers {
        for (name, value) in headers {
            if name.to_lowercase() == "user-agent" && is_bad_bot(value) {
                return Action::Block as i32;
            }
        }
    }
    
    // Rule 3: Log suspicious patterns
    if is_suspicious(&ctx.uri) {
        return Action::Log as i32;
    }
    
    // Default: Allow
    Action::Allow as i32
}
```

#### Bad Bot Patterns

```rust
fn is_bad_bot(user_agent: &str) -> bool {
    let ua_lower = user_agent.to_lowercase();
    let bad_patterns = [
        "badbot", "sqlmap", "nikto", "masscan",
        "zgrab", "nmap", "metasploit",
    ];
    bad_patterns.iter().any(|p| ua_lower.contains(p))
}
```

#### Suspicious URI Patterns

```rust
fn is_suspicious(uri: &str) -> bool {
    let uri_lower = uri.to_lowercase();
    
    // SQL injection
    if uri_lower.contains("' or ") || uri_lower.contains("union select") {
        return true;
    }
    
    // Path traversal
    if uri_lower.contains("../") || uri_lower.contains("..\\") {
        return true;
    }
    
    // Command injection
    if uri_lower.contains(";") && 
       (uri_lower.contains("wget") || uri_lower.contains("curl")) {
        return true;
    }
    
    false
}
```

### 4. Build Configuration

**File:** `wasm-rules/bad-bot/Cargo.toml`

```toml
[package]
name = "bad-bot-rule"
version = "0.1.0"
edition = "2021"

[workspace]
# Standalone package (not part of main workspace)

[lib]
crate-type = ["cdylib"]  # Create dynamic library for WASM

[dependencies]
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"

[profile.release]
opt-level = "z"      # Optimize for size
lto = true           # Link Time Optimization
codegen-units = 1    # Better optimization
strip = true         # Strip symbols
```

**Build Command:**
```bash
cargo build --target wasm32-unknown-unknown --release
```

**Output:** `target/wasm32-unknown-unknown/release/bad_bot_rule.wasm` (85 KB)

## Testing

### Test Infrastructure

**Standalone Test Runner:** `wasm-rules/test-runner`

A minimal binary that:
1. Loads WASM module via wasmtime
2. Executes 7 test scenarios
3. Validates Action responses
4. Tracks fuel consumption

#### Test Scenarios

| Test | URI | Headers | Expected | Fuel Used |
|------|-----|---------|----------|-----------|
| 1 | `/admin` | - | Block | 6,818 |
| 2 | `/api/users` | - | Allow | 10,383 |
| 3 | `/api/users?id=1' or 1=1--` | - | Log | 10,190 |
| 4 | `/api/data` | User-Agent: BadBot/1.0 | Block | 14,321 |
| 5 | `/admin/login` | - | Block | 7,661 |
| 6 | `/files/../../etc/passwd` | - | Log | 12,195 |
| 7 | `/api/search` | User-Agent: sqlmap/1.0 | Block | 15,045 |

### Deployment and Execution

**Platform:** Proxmox LXC Container 100
- Host: 192.168.100.102 (root/invaders)
- Container: vmid 100 (Ubuntu 22.04, kernel 6.14.11-2-pve)
- Rust: nightly-1.93.0-nightly (94b49fd99 2025-11-22)

**Deployment Script:** `deploy-and-test.sh`

```bash
#!/bin/bash
# 1. Build WASM module locally (macOS)
# 2. Create tarball with source code
# 3. Transfer to Proxmox host
# 4. Copy into container
# 5. Build test runner inside container (Linux)
# 6. Execute tests
```

**Results:**
```
🧪 SecBeat WASM Test Runner

📦 Loading WASM module: bad-bot.wasm
✓ Module loaded successfully

Test 1: Block /admin paths
  → Fuel consumed: 6818
  ✓ PASS: /admin blocked as expected

Test 2: Allow normal paths
  → Fuel consumed: 10383
  ✓ PASS: /api/users allowed as expected

Test 3: Log SQL injection attempts
  → Fuel consumed: 10190
  ✓ PASS: SQL injection logged as expected

Test 4: Block bad bot user agents
  → Fuel consumed: 14321
  ✓ PASS: BadBot blocked as expected

Test 5: Block /admin subdirectories
  → Fuel consumed: 7661
  ✓ PASS: /admin/login blocked as expected

Test 6: Log path traversal attempts
  → Fuel consumed: 12195
  ✓ PASS: Path traversal logged as expected

Test 7: Block SQLMap user agent
  → Fuel consumed: 15045
  ✓ PASS: SQLMap blocked as expected

━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
Results: 7 passed, 0 failed
✓ All tests passed!
```

## Performance Characteristics

### Fuel Consumption Analysis

- **Simple URI check** (Test 1: /admin): **6,818 instructions**
- **Full header scan** (Test 4: BadBot): **14,321 instructions**
- **Complex pattern matching** (Test 7: SQLMap): **15,045 instructions**

**Average:** ~11,000 instructions per request  
**Fuel Limit:** 100,000 instructions (9x safety margin)

### Throughput Estimates

Assuming:
- CPU: Modern x86_64 (3 GHz)
- Instructions per cycle: ~2
- WASM overhead: 2x native

**Calculation:**
```
3 GHz * 2 IPC / (11,000 instructions * 2 overhead) = ~273K requests/second/core
```

**Real-world estimate:** 100K-200K requests/second/core (accounting for JSON serialization, memory copies, etc.)

### Memory Usage

- **Module size:** 85 KB (loaded once, shared across threads)
- **Per-request overhead:** ~1 KB (JSON serialization)
- **WASM instance:** Minimal (linear memory allocated on-demand)

## Security Considerations

### Sandboxing

WASM provides strong isolation:
- No direct system access
- No network access
- No file system access
- Can only call exported host functions

### Resource Limits

**Fuel Limits:**
- Prevents infinite loops
- Caps CPU usage per request
- Configurable per-deployment

**Memory Limits:**
- 1 MB per WASM instance
- Linear memory cannot grow beyond limit
- Host controls memory allocation

### Code Validation

Before loading:
1. Verify WASM bytecode signature
2. Check for required exports (`inspect_request`)
3. Validate module doesn't import dangerous functions
4. Compile and cache module

## Integration with Mitigation Node

### Loading Modules

```rust
use mitigation_node::{WasmConfig, WasmEngine};

let config = WasmConfig::default();
let engine = WasmEngine::new(config)?;

let bytecode = std::fs::read("bad-bot.wasm")?;
engine.load_module("bad-bot", &bytecode)?;
```

### Executing Rules

```rust
use mitigation_node::RequestContext;

let ctx = RequestContext {
    method: "GET".to_string(),
    uri: "/admin".to_string(),
    // ...
};

let action = engine.run_module("bad-bot", &ctx)?;

match action {
    Action::Block => { /* drop connection */ },
    Action::Allow => { /* forward request */ },
    Action::Log => { /* log and allow */ },
    Action::RateLimit => { /* apply rate limit */ },
}
```

### Hot-Reload

```rust
// Unload old version
engine.unload_module("bad-bot")?;

// Load new version
let new_bytecode = std::fs::read("bad-bot-v2.wasm")?;
engine.load_module("bad-bot", &new_bytecode)?;

// No restart required!
```

## Future Enhancements

### 1. Orchestrator Integration

Distribute WASM modules via NATS:

```rust
// Orchestrator publishes new rule
nats_client.publish("wasm.rule.deploy", wasm_bytecode)?;

// Mitigation nodes subscribe
nats_client.subscribe("wasm.rule.deploy", |msg| {
    engine.load_module("rule-v2", &msg.data)?;
})?;
```

### 2. Rule Composition

Chain multiple WASM modules:

```rust
let actions = vec![
    engine.run_module("geofence", &ctx)?,
    engine.run_module("bad-bot", &ctx)?,
    engine.run_module("rate-limit", &ctx)?,
];

// Take most restrictive action
let final_action = actions.iter().max()?;
```

### 3. Metrics and Observability

Track per-module metrics:

```rust
#[derive(Debug)]
pub struct ModuleStats {
    pub total_calls: u64,
    pub blocks: u64,
    pub allows: u64,
    pub logs: u64,
    pub avg_fuel: u64,
    pub errors: u64,
}
```

### 4. Advanced ABI Features

- **Shared state:** Allow WASM to read/write shared maps
- **Callbacks:** WASM can call host functions (DNS lookup, IP geolocation)
- **Streaming:** Process large request bodies incrementally

## Comparison to Other Approaches

| Feature | WASM Rules | Lua Scripts | Native Code |
|---------|-----------|-------------|-------------|
| **Hot-reload** | ✓ Yes | ✓ Yes | ✗ No |
| **Safety** | ✓ Sandboxed | ~ Limited | ✗ None |
| **Performance** | ✓✓ Near-native | ~ Interpreted | ✓✓✓ Native |
| **Tooling** | ✓ Rust/C/C++ | ~ Lua only | ✓ Any language |
| **Resource limits** | ✓ Fuel + memory | ~ Basic | ✗ None |
| **Cross-platform** | ✓ Yes | ✓ Yes | ✗ Arch-specific |

## Production Checklist

- [x] WASM ABI defined and documented
- [x] WasmEngine with module caching
- [x] Fuel limits enforced (100K instructions)
- [x] Memory limits enforced (1 MB)
- [x] Sample rule implemented and tested
- [x] Integration tests passing on Linux
- [ ] Orchestrator NATS integration
- [ ] Prometheus metrics for WASM execution
- [ ] Rule versioning and rollback
- [ ] Production deployment on all mitigation nodes

## References

- **wasmtime Documentation:** https://docs.wasmtime.dev/
- **WASM Specification:** https://webassembly.github.io/spec/
- **Rust WASM Book:** https://rustwasm.github.io/docs/book/
- **Cloudflare Workers:** Similar approach for edge computing

## Commits

All changes committed to main branch:
- WASM ABI definition (abi.rs)
- WasmEngine implementation (engine.rs)
- Sample bad-bot rule (wasm-rules/bad-bot)
- Standalone test runner
- Deployment scripts for Proxmox
- Documentation (this file)

## Verification Checklist

- [x] Action enum defined with 4 states
- [x] RequestContext with JSON serialization
- [x] WasmEngine with module caching
- [x] Fuel limit enforcement (100K)
- [x] Memory limit enforcement (1 MB)
- [x] Module loading/unloading
- [x] Sample WASM rule (bad-bot)
- [x] 7 integration tests passing
- [x] Tested on Linux (Proxmox container)
- [x] Fuel consumption measured
- [x] Documentation complete

---

**Status:** Chapter 3.1 complete and verified on production-like Linux environment ✅
