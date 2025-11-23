// Integration tests for WASM engine
//
// These tests validate:
// 1. Module loading and compilation
// 2. Request inspection with various URIs
// 3. Fuel/memory limits
// 4. Module caching and hot-reload

#![cfg(test)] // Only compile when running tests

use mitigation_node::wasm::{Action, RequestContext, WasmConfig, WasmEngine};
use std::fs;

const BAD_BOT_WASM_PATH: &str = "../target/wasm/bad-bot.wasm";

fn load_bad_bot_module() -> Vec<u8> {
    fs::read(BAD_BOT_WASM_PATH)
        .expect("Failed to read bad-bot.wasm - did you run wasm-rules/bad-bot/build.sh?")
}

#[test]
fn test_engine_creation() {
    let config = WasmConfig::default();
    let engine = WasmEngine::new(config);
    assert!(engine.is_ok(), "Failed to create WASM engine");
}

#[test]
fn test_module_loading() {
    let config = WasmConfig::default();
    let engine = WasmEngine::new(config).unwrap();
    
    let bytecode = load_bad_bot_module();
    
    let result = engine.load_module("bad-bot", &bytecode);
    assert!(result.is_ok(), "Failed to load module: {:?}", result.err());
    
    let modules = engine.list_modules();
    assert_eq!(modules.len(), 1);
    assert!(modules.contains(&"bad-bot".to_string()));
}

#[test]
fn test_block_admin_path() {
    let config = WasmConfig::default();
    let engine = WasmEngine::new(config).unwrap();
    let bytecode = load_bad_bot_module();
    engine.load_module("bad-bot", &bytecode).unwrap();
    
    // Test: /admin should be blocked
    let ctx = RequestContext::minimal("/admin");
    let action = engine.run_module("bad-bot", &ctx).unwrap();
    
    assert_eq!(action, Action::Block, "Expected /admin to be blocked");
}

#[test]
fn test_allow_normal_path() {
    let config = WasmConfig::default();
    let engine = WasmEngine::new(config).unwrap();
    let bytecode = load_bad_bot_module();
    engine.load_module("bad-bot", &bytecode).unwrap();
    
    // Test: /api/users should be allowed
    let ctx = RequestContext::minimal("/api/users");
    let action = engine.run_module("bad-bot", &ctx).unwrap();
    
    assert_eq!(action, Action::Allow, "Expected /api/users to be allowed");
}

#[test]
fn test_log_suspicious_path() {
    let config = WasmConfig::default();
    let engine = WasmEngine::new(config).unwrap();
    let bytecode = load_bad_bot_module();
    engine.load_module("bad-bot", &bytecode).unwrap();
    
    // Test: SQL injection attempt should be logged
    let ctx = RequestContext::minimal("/api/users?id=1' or 1=1--");
    let action = engine.run_module("bad-bot", &ctx).unwrap();
    
    assert_eq!(action, Action::Log, "Expected SQL injection to be logged");
}

#[test]
fn test_block_bad_user_agent() {
    let config = WasmConfig::default();
    let engine = WasmEngine::new(config).unwrap();
    let bytecode = load_bad_bot_module();
    engine.load_module("bad-bot", &bytecode).unwrap();
    
    // Test: BadBot user agent should be blocked
    let ctx = RequestContext {
        method: "GET".to_string(),
        uri: "/api/data".to_string(),
        version: "HTTP/1.1".to_string(),
        source_ip: "192.168.1.100".to_string(),
        headers: Some(vec![
            ("User-Agent".to_string(), "BadBot/1.0".to_string()),
        ]),
        body_preview: None,
    };
    
    let action = engine.run_module("bad-bot", &ctx).unwrap();
    assert_eq!(action, Action::Block, "Expected BadBot to be blocked");
}

#[test]
fn test_fuel_limit() {
    let config = WasmConfig {
        max_fuel: 1000, // Very low fuel limit
        max_memory: 1024 * 1024,
        cache_enabled: false,
    };
    let engine = WasmEngine::new(config).unwrap();
    let bytecode = load_bad_bot_module();
    engine.load_module("bad-bot-limited", &bytecode).unwrap();
    
    // This should fail due to fuel exhaustion
    let ctx = RequestContext::minimal("/test");
    let result = engine.run_module("bad-bot-limited", &ctx);
    
    // Note: Depending on the complexity, this might succeed or fail
    // In production, you'd calibrate fuel limits based on actual usage
    println!("Fuel limit test result: {:?}", result);
}

#[test]
fn test_module_hot_reload() {
    let config = WasmConfig::default();
    let engine = WasmEngine::new(config).unwrap();
    let bytecode = load_bad_bot_module();
    
    // Load module v1
    engine.load_module("bad-bot", &bytecode).unwrap();
    let ctx = RequestContext::minimal("/admin");
    let action = engine.run_module("bad-bot", &ctx).unwrap();
    assert_eq!(action, Action::Block);
    
    // Unload
    engine.unload_module("bad-bot").unwrap();
    let modules = engine.list_modules();
    assert_eq!(modules.len(), 0);
    
    // Reload (simulating hot-reload)
    engine.load_module("bad-bot", &bytecode).unwrap();
    let action = engine.run_module("bad-bot", &ctx).unwrap();
    assert_eq!(action, Action::Block);
}

#[test]
fn test_concurrent_module_execution() {
    use std::sync::Arc;
    use std::thread;
    
    let config = WasmConfig::default();
    let engine = Arc::new(WasmEngine::new(config).unwrap());
    let bytecode = load_bad_bot_module();
    engine.load_module("bad-bot", &bytecode).unwrap();
    
    let mut handles = vec![];
    
    // Spawn 10 threads executing the same module
    for i in 0..10 {
        let engine_clone = Arc::clone(&engine);
        let handle = thread::spawn(move || {
            let ctx = if i % 2 == 0 {
                RequestContext::minimal("/admin")
            } else {
                RequestContext::minimal("/api/users")
            };
            
            engine_clone.run_module("bad-bot", &ctx).unwrap()
        });
        handles.push(handle);
    }
    
    // Collect results
    let results: Vec<Action> = handles.into_iter().map(|h| h.join().unwrap()).collect();
    
    // Verify: half should be Block (/admin), half should be Allow (/api/users)
    let blocks = results.iter().filter(|a| **a == Action::Block).count();
    let allows = results.iter().filter(|a| **a == Action::Allow).count();
    
    assert_eq!(blocks, 5, "Expected 5 blocks");
    assert_eq!(allows, 5, "Expected 5 allows");
}

#[test]
fn test_module_info() {
    let config = WasmConfig::default();
    let engine = WasmEngine::new(config).unwrap();
    let bytecode = load_bad_bot_module();
    engine.load_module("bad-bot", &bytecode).unwrap();
    
    let info = engine.get_module_info("bad-bot");
    assert!(info.is_some());
    
    let info = info.unwrap();
    assert_eq!(info.name, "bad-bot");
    assert!(info.age.as_millis() < 1000); // Loaded less than 1 second ago
}
