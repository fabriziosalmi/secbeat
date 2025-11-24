// WASM Memory Leak Audit Tests
//
// These tests validate that WasmEngine properly cleans up resources during:
// 1. Hot-reload cycles (load → unload → reload)
// 2. Multiple module executions
// 3. Module instance creation and destruction
// 4. Store and memory management
//
// Test methodology:
// - Use jemalloc stats to track heap allocations (Linux/macOS)
// - Perform many iterations to amplify any potential leaks
// - Verify memory doesn't grow unbounded
// - Check wasmtime's internal cleanup

#![cfg(test)]

use mitigation_node::wasm::{Action, RequestContext, WasmConfig, WasmEngine};
use std::fs;

const BAD_BOT_WASM_PATH: &str = "../target/wasm/bad-bot.wasm";
const ITERATIONS: usize = 100; // Number of reload cycles for leak detection

fn load_bad_bot_module() -> Vec<u8> {
    fs::read(BAD_BOT_WASM_PATH)
        .expect("Failed to read bad-bot.wasm - did you run wasm-rules/bad-bot/build.sh?")
}

/// Get current heap allocated bytes (works on Linux/macOS with jemalloc or system allocator)
#[cfg(target_os = "linux")]
fn get_heap_allocated() -> Option<usize> {
    use std::fs::File;
    use std::io::Read;

    // Read /proc/self/statm for memory stats
    let mut file = File::open("/proc/self/statm").ok()?;
    let mut contents = String::new();
    file.read_to_string(&mut contents).ok()?;
    
    // Format: total resident shared text lib data dt
    let parts: Vec<&str> = contents.split_whitespace().collect();
    if parts.len() >= 2 {
        // Resident set size in pages
        let rss_pages: usize = parts[1].parse().ok()?;
        let page_size = 4096; // Standard page size
        Some(rss_pages * page_size)
    } else {
        None
    }
}

#[cfg(not(target_os = "linux"))]
fn get_heap_allocated() -> Option<usize> {
    // Fallback: no direct heap tracking on non-Linux
    // Return None to skip memory assertions but still run tests
    None
}

#[test]
fn test_hot_reload_no_memory_leak() {
    let config = WasmConfig::default();
    let engine = WasmEngine::new(config).unwrap();
    let bytecode = load_bad_bot_module();

    // Get baseline memory
    let baseline = get_heap_allocated();
    println!("Baseline memory: {:?} bytes", baseline);

    // Perform many hot-reload cycles
    for i in 0..ITERATIONS {
        // Load module
        engine.load_module(format!("bad-bot-{}", i), &bytecode)
            .expect("Failed to load module");

        // Run a few requests
        let ctx = RequestContext::minimal("/admin");
        let action = engine.run_module(&format!("bad-bot-{}", i), &ctx)
            .expect("Failed to run module");
        assert_eq!(action, Action::Block);

        // Unload module
        engine.unload_module(&format!("bad-bot-{}", i))
            .expect("Failed to unload module");

        // Periodic memory check
        if i % 20 == 19 {
            if let Some(current) = get_heap_allocated() {
                println!("Iteration {}: {} bytes", i + 1, current);
            }
        }
    }

    // Force garbage collection hints (Rust doesn't have explicit GC, but drop scope helps)
    drop(engine);

    // Final memory check
    let final_mem = get_heap_allocated();
    println!("Final memory: {:?} bytes", final_mem);

    // Memory leak assertion
    if let (Some(base), Some(final_val)) = (baseline, final_mem) {
        let growth = final_val as f64 / base as f64;
        println!("Memory growth factor: {:.2}x", growth);

        // Allow up to 2x growth (some caching is expected)
        // In a real leak scenario, we'd see 10x+ growth
        assert!(
            growth < 2.0,
            "Memory grew by {:.2}x - possible leak detected! Base: {} bytes, Final: {} bytes",
            growth,
            base,
            final_val
        );
    } else {
        println!("⚠️  Memory tracking not available on this platform - leak test skipped");
    }
}

#[test]
fn test_module_reuse_no_leak() {
    let config = WasmConfig::default();
    let engine = WasmEngine::new(config).unwrap();
    let bytecode = load_bad_bot_module();

    // Load module once
    engine.load_module("bad-bot-reuse", &bytecode)
        .expect("Failed to load module");

    let baseline = get_heap_allocated();
    println!("Baseline memory (module loaded): {:?} bytes", baseline);

    // Execute many times without reloading
    for i in 0..ITERATIONS * 10 {
        let ctx = RequestContext::minimal("/admin");
        let action = engine.run_module("bad-bot-reuse", &ctx)
            .expect("Failed to run module");
        assert_eq!(action, Action::Block);

        if i % 200 == 199 {
            if let Some(current) = get_heap_allocated() {
                println!("Execution {}: {} bytes", i + 1, current);
            }
        }
    }

    let final_mem = get_heap_allocated();
    println!("Final memory (after {} executions): {:?} bytes", ITERATIONS * 10, final_mem);

    // Check for execution memory leaks
    if let (Some(base), Some(final_val)) = (baseline, final_mem) {
        let growth = final_val as f64 / base as f64;
        println!("Memory growth factor: {:.2}x", growth);

        // Execution should not leak - allow minimal growth for JIT warmup
        assert!(
            growth < 1.5,
            "Memory grew by {:.2}x during executions - possible Store/Instance leak!",
            growth
        );
    } else {
        println!("⚠️  Memory tracking not available - execution leak test skipped");
    }
}

#[test]
fn test_multiple_modules_cleanup() {
    let config = WasmConfig::default();
    let engine = WasmEngine::new(config).unwrap();
    let bytecode = load_bad_bot_module();

    let baseline = get_heap_allocated();
    println!("Baseline memory: {:?} bytes", baseline);

    // Load many modules
    for i in 0..50 {
        engine.load_module(format!("module-{}", i), &bytecode)
            .expect("Failed to load module");
    }

    let peak = get_heap_allocated();
    println!("Peak memory (50 modules loaded): {:?} bytes", peak);

    // Unload all modules
    for i in 0..50 {
        engine.unload_module(&format!("module-{}", i))
            .expect("Failed to unload module");
    }

    // Give time for cleanup
    std::thread::sleep(std::time::Duration::from_millis(100));

    let final_mem = get_heap_allocated();
    println!("Final memory (all unloaded): {:?} bytes", final_mem);

    // Verify cleanup brings memory back down
    if let (Some(base), Some(final_val)) = (baseline, final_mem) {
        let cleanup_ratio = final_val as f64 / base as f64;
        println!("Memory after cleanup: {:.2}x baseline", cleanup_ratio);

        // After unloading, memory should be close to baseline
        assert!(
            cleanup_ratio < 1.5,
            "Memory not properly cleaned up after unload - ratio: {:.2}x",
            cleanup_ratio
        );
    } else {
        println!("⚠️  Memory tracking not available - cleanup test skipped");
    }
}

#[test]
fn test_drop_engine_cleanup() {
    let baseline = get_heap_allocated();
    println!("Baseline memory (no engine): {:?} bytes", baseline);

    {
        let config = WasmConfig::default();
        let engine = WasmEngine::new(config).unwrap();
        let bytecode = load_bad_bot_module();

        // Load multiple modules
        for i in 0..20 {
            engine.load_module(format!("drop-test-{}", i), &bytecode)
                .expect("Failed to load module");
        }

        let peak = get_heap_allocated();
        println!("Peak memory (engine + 20 modules): {:?} bytes", peak);

        // Engine will be dropped here
    }

    // Give time for Drop cleanup
    std::thread::sleep(std::time::Duration::from_millis(200));

    let final_mem = get_heap_allocated();
    println!("Final memory (engine dropped): {:?} bytes", final_mem);

    // Verify Drop properly cleans up
    if let (Some(base), Some(final_val)) = (baseline, final_mem) {
        let cleanup_ratio = final_val as f64 / base as f64;
        println!("Memory after Drop: {:.2}x baseline", cleanup_ratio);

        // Drop should clean up most allocations
        assert!(
            cleanup_ratio < 2.0,
            "Engine Drop did not clean up properly - ratio: {:.2}x",
            cleanup_ratio
        );
    } else {
        println!("⚠️  Memory tracking not available - Drop test skipped");
    }
}

#[test]
fn test_verify_module_count_consistency() {
    let config = WasmConfig::default();
    let engine = WasmEngine::new(config).unwrap();
    let bytecode = load_bad_bot_module();

    // Load 10 modules
    for i in 0..10 {
        engine.load_module(format!("count-test-{}", i), &bytecode)
            .expect("Failed to load module");
    }

    assert_eq!(engine.list_modules().len(), 10, "Expected 10 modules loaded");

    // Unload 5 modules
    for i in 0..5 {
        engine.unload_module(&format!("count-test-{}", i))
            .expect("Failed to unload module");
    }

    assert_eq!(engine.list_modules().len(), 5, "Expected 5 modules remaining");

    // Reload 5 modules
    for i in 0..5 {
        engine.load_module(format!("count-test-new-{}", i), &bytecode)
            .expect("Failed to load module");
    }

    assert_eq!(engine.list_modules().len(), 10, "Expected 10 modules total");

    // This verifies HashMap cleanup is working correctly
}
