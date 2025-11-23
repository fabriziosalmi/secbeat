// Standalone WASM Test Runner
// Designed to run on Linux (Proxmox container) without Aya dependencies

use anyhow::{anyhow, Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use wasmtime::*;

/// Action enum matching the WASM ABI
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(i32)]
pub enum Action {
    Allow = 0,
    Block = 1,
    Log = 2,
    RateLimit = 3,
}

impl Action {
    fn from_i32(value: i32) -> Option<Self> {
        match value {
            0 => Some(Action::Allow),
            1 => Some(Action::Block),
            2 => Some(Action::Log),
            3 => Some(Action::RateLimit),
            _ => None,
        }
    }
}

/// Request context matching the WASM ABI
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RequestContext {
    pub method: String,
    pub uri: String,
    pub version: String,
    pub source_ip: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub headers: Option<Vec<(String, String)>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub body_preview: Option<String>,
}

impl RequestContext {
    fn minimal(uri: impl Into<String>) -> Self {
        Self {
            method: "GET".to_string(),
            uri: uri.into(),
            version: "HTTP/1.1".to_string(),
            source_ip: "0.0.0.0".to_string(),
            headers: None,
            body_preview: None,
        }
    }

    fn with_headers(uri: impl Into<String>, headers: Vec<(String, String)>) -> Self {
        Self {
            method: "GET".to_string(),
            uri: uri.into(),
            version: "HTTP/1.1".to_string(),
            source_ip: "192.168.1.100".to_string(),
            headers: Some(headers),
            body_preview: None,
        }
    }
}

/// Simple WASM engine for testing
struct WasmEngine {
    engine: Engine,
    module: Module,
}

impl WasmEngine {
    fn new(wasm_path: &str) -> Result<Self> {
        let mut config = Config::new();
        config.consume_fuel(true);

        let engine = Engine::new(&config)?;
        let bytecode = fs::read(wasm_path)
            .context(format!("Failed to read WASM file: {}", wasm_path))?;
        let module = Module::new(&engine, bytecode)?;

        Ok(Self { engine, module })
    }

    fn run(&self, ctx: &RequestContext) -> Result<Action> {
        let mut store = Store::new(&self.engine, ());
        store.set_fuel(100_000)?;

        let instance = Instance::new(&mut store, &self.module, &[])?;

        let inspect_fn = instance
            .get_typed_func::<(i32, i32), i32>(&mut store, "inspect_request")?;

        let json = serde_json::to_string(ctx)?;
        let json_bytes = json.as_bytes();

        let memory = instance
            .get_memory(&mut store, "memory")
            .ok_or_else(|| anyhow!("WASM module must export 'memory'"))?;

        memory.write(&mut store, 0, json_bytes)?;

        let result = inspect_fn.call(&mut store, (0, json_bytes.len() as i32))?;

        let fuel_consumed = 100_000 - store.get_fuel().unwrap_or(0);
        println!("  → Fuel consumed: {}", fuel_consumed);

        Action::from_i32(result)
            .ok_or_else(|| anyhow!("Invalid action: {}", result))
    }
}

fn main() -> Result<()> {
    println!("🧪 SecBeat WASM Test Runner\n");

    // Find WASM file
    let wasm_path = if std::path::Path::new("../target/wasm/bad-bot.wasm").exists() {
        "../target/wasm/bad-bot.wasm"
    } else if std::path::Path::new("bad-bot.wasm").exists() {
        "bad-bot.wasm"
    } else {
        return Err(anyhow!("bad-bot.wasm not found! Run: cd ../bad-bot && ./build.sh"));
    };

    println!("📦 Loading WASM module: {}", wasm_path);
    let engine = WasmEngine::new(wasm_path)?;
    println!("✓ Module loaded successfully\n");

    let mut passed = 0;
    let mut failed = 0;

    // Test 1: Block /admin
    println!("Test 1: Block /admin paths");
    let ctx = RequestContext::minimal("/admin");
    match engine.run(&ctx) {
        Ok(Action::Block) => {
            println!("  ✓ PASS: /admin blocked as expected\n");
            passed += 1;
        }
        Ok(action) => {
            println!("  ✗ FAIL: Expected Block, got {:?}\n", action);
            failed += 1;
        }
        Err(e) => {
            println!("  ✗ FAIL: Execution error: {}\n", e);
            failed += 1;
        }
    }

    // Test 2: Allow normal path
    println!("Test 2: Allow normal paths");
    let ctx = RequestContext::minimal("/api/users");
    match engine.run(&ctx) {
        Ok(Action::Allow) => {
            println!("  ✓ PASS: /api/users allowed as expected\n");
            passed += 1;
        }
        Ok(action) => {
            println!("  ✗ FAIL: Expected Allow, got {:?}\n", action);
            failed += 1;
        }
        Err(e) => {
            println!("  ✗ FAIL: Execution error: {}\n", e);
            failed += 1;
        }
    }

    // Test 3: Log SQL injection
    println!("Test 3: Log SQL injection attempts");
    let ctx = RequestContext::minimal("/api/users?id=1' or 1=1--");
    match engine.run(&ctx) {
        Ok(Action::Log) => {
            println!("  ✓ PASS: SQL injection logged as expected\n");
            passed += 1;
        }
        Ok(action) => {
            println!("  ✗ FAIL: Expected Log, got {:?}\n", action);
            failed += 1;
        }
        Err(e) => {
            println!("  ✗ FAIL: Execution error: {}\n", e);
            failed += 1;
        }
    }

    // Test 4: Block bad bot
    println!("Test 4: Block bad bot user agents");
    let ctx = RequestContext::with_headers(
        "/api/data",
        vec![("User-Agent".to_string(), "BadBot/1.0".to_string())],
    );
    match engine.run(&ctx) {
        Ok(Action::Block) => {
            println!("  ✓ PASS: BadBot blocked as expected\n");
            passed += 1;
        }
        Ok(action) => {
            println!("  ✗ FAIL: Expected Block, got {:?}\n", action);
            failed += 1;
        }
        Err(e) => {
            println!("  ✗ FAIL: Execution error: {}\n", e);
            failed += 1;
        }
    }

    // Test 5: Block /admin/login
    println!("Test 5: Block /admin subdirectories");
    let ctx = RequestContext::minimal("/admin/login");
    match engine.run(&ctx) {
        Ok(Action::Block) => {
            println!("  ✓ PASS: /admin/login blocked as expected\n");
            passed += 1;
        }
        Ok(action) => {
            println!("  ✗ FAIL: Expected Block, got {:?}\n", action);
            failed += 1;
        }
        Err(e) => {
            println!("  ✗ FAIL: Execution error: {}\n", e);
            failed += 1;
        }
    }

    // Test 6: Log path traversal
    println!("Test 6: Log path traversal attempts");
    let ctx = RequestContext::minimal("/files/../../etc/passwd");
    match engine.run(&ctx) {
        Ok(Action::Log) => {
            println!("  ✓ PASS: Path traversal logged as expected\n");
            passed += 1;
        }
        Ok(action) => {
            println!("  ✗ FAIL: Expected Log, got {:?}\n", action);
            failed += 1;
        }
        Err(e) => {
            println!("  ✗ FAIL: Execution error: {}\n", e);
            failed += 1;
        }
    }

    // Test 7: Block SQLMap user agent
    println!("Test 7: Block SQLMap user agent");
    let ctx = RequestContext::with_headers(
        "/api/search",
        vec![("User-Agent".to_string(), "sqlmap/1.0".to_string())],
    );
    match engine.run(&ctx) {
        Ok(Action::Block) => {
            println!("  ✓ PASS: SQLMap blocked as expected\n");
            passed += 1;
        }
        Ok(action) => {
            println!("  ✗ FAIL: Expected Block, got {:?}\n", action);
            failed += 1;
        }
        Err(e) => {
            println!("  ✗ FAIL: Execution error: {}\n", e);
            failed += 1;
        }
    }

    // Summary
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("Results: {} passed, {} failed", passed, failed);
    
    if failed == 0 {
        println!("✓ All tests passed!");
        Ok(())
    } else {
        Err(anyhow!("{} test(s) failed", failed))
    }
}
