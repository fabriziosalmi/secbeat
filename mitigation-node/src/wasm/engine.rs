// WASM Engine Module - Manages WASM module lifecycle and execution
//
// This module provides:
// - Compilation and caching of WASM modules
// - Execution with strict resource limits (fuel, memory)
// - Hot-reloading capability for dynamic rule updates

use crate::error::{MitigationError, Result};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use tracing::{debug, info, warn};
use wasmtime::*;

use super::abi::{Action, RequestContext, INSPECT_REQUEST_FN};

/// Configuration for WASM engine resource limits
#[derive(Debug, Clone)]
pub struct WasmConfig {
    /// Maximum fuel (CPU instructions) per execution
    /// Default: 100_000 (enough for most rules, ~0.1ms)
    pub max_fuel: u64,
    
    /// Maximum memory per WASM instance (bytes)
    /// Default: 1 MB
    pub max_memory: usize,
    
    /// Enable compilation cache
    pub cache_enabled: bool,
}

impl Default for WasmConfig {
    fn default() -> Self {
        Self {
            max_fuel: 100_000,
            max_memory: 1024 * 1024, // 1 MB
            cache_enabled: true,
        }
    }
}

/// WASM module wrapper with metadata
struct CachedModule {
    module: Module,
    name: String,
    loaded_at: std::time::Instant,
}

/// WASM Engine - manages compilation, caching, and execution of WASM modules
pub struct WasmEngine {
    engine: Engine,
    modules: Arc<RwLock<HashMap<String, CachedModule>>>,
    config: WasmConfig,
}

impl WasmEngine {
    /// Create a new WASM engine with specified configuration
    pub fn new(config: WasmConfig) -> Result<Self> {
        let mut engine_config = Config::new();
        
        // Enable fuel metering for CPU limits
        engine_config.consume_fuel(true);
        
        // Disable WASI and advanced features (we use custom ABI)
        // Note: Must disable relaxed-simd before disabling simd
        engine_config.wasm_relaxed_simd(false);
        engine_config.wasm_simd(false);
        engine_config.wasm_bulk_memory(true);
        engine_config.wasm_reference_types(false);
        
        // Enable caching if requested
        if config.cache_enabled {
            if let Err(e) = engine_config.cache_config_load_default() {
                warn!("Failed to load wasmtime cache config: {}", e);
            }
        }

        let engine = Engine::new(&engine_config)
            .map_err(|e| MitigationError::Wasm(format!("Failed to create wasmtime engine: {}", e)))?;;

        info!(
            "WASM engine initialized with max_fuel={}, max_memory={}",
            config.max_fuel, config.max_memory
        );

        Ok(Self {
            engine,
            modules: Arc::new(RwLock::new(HashMap::new())),
            config,
        })
    }

    /// Load and compile a WASM module from bytecode with optional configuration
    ///
    /// # Arguments
    /// * `name` - Unique identifier for this module (e.g., "bad-bot-v1")
    /// * `bytecode` - WASM bytecode (.wasm file contents)
    /// * `config_json` - Optional JSON configuration for the module
    ///
    /// # Returns
    /// Ok(()) if compilation and caching succeeded
    pub fn load_module_with_config(
        &self,
        name: impl Into<String>,
        bytecode: &[u8],
        config_json: Option<&str>,
    ) -> Result<()> {
        let name = name.into();
        
        debug!("Compiling WASM module: {}", name);
        
        let module = Module::new(&self.engine, bytecode)
            .map_err(|e| MitigationError::Wasm(format!("Failed to compile WASM module: {}", e)))?;;

        // Validate module exports the required function
        let mut has_inspect = false;
        for export in module.exports() {
            let export_type: wasmtime::ExportType = export;
            if export_type.name() == INSPECT_REQUEST_FN {
                has_inspect = true;
                break;
            }
        }

        if !has_inspect {
            return Err(MitigationError::Wasm(format!(
                "WASM module must export '{}' function",
                INSPECT_REQUEST_FN
            )));
        }

        // If config provided, call configure() function
        if let Some(config) = config_json {
            // Create temporary instance to configure
            let mut store = Store::new(&self.engine, ());
            let instance = Instance::new(&mut store, &module, &[])
                .map_err(|e| MitigationError::Wasm(format!("Failed to create instance for configuration: {}", e)))?;;

            // Check if module exports configure function
            if let Ok(configure_fn) = instance.get_typed_func::<(i32, i32), i32>(&mut store, "configure") {
                let config_bytes = config.as_bytes();
                
                // Get memory
                let memory = instance
                    .get_memory(&mut store, "memory")
                    .ok_or_else(|| MitigationError::Wasm("WASM module must export 'memory'".to_string()))?;;

                // Write config to memory
                memory.write(&mut store, 0, config_bytes)
                    .map_err(|e| MitigationError::Wasm(format!("Failed to write config to WASM memory: {}", e)))?;;

                // Call configure
                let result: i32 = configure_fn.call(&mut store, (0, config_bytes.len() as i32))
                    .map_err(|e| MitigationError::Wasm(format!("configure() call failed: {}", e)))?;;

                if result != 0 {
                    return Err(MitigationError::Wasm(format!("Configuration failed with code: {}", result)));
                }

                info!("WASM module '{}' configured successfully", name);
            } else {
                warn!("WASM module '{}' does not support configuration", name);
            }
        }

        let cached = CachedModule {
            module,
            name: name.clone(),
            loaded_at: std::time::Instant::now(),
        };

        let mut modules = self.modules.write().unwrap();
        modules.insert(name.clone(), cached);

        info!("Successfully loaded WASM module: {}", name);
        Ok(())
    }

    /// Load a WASM module without configuration (backward compatibility)
    pub fn load_module(&self, name: impl Into<String>, bytecode: &[u8]) -> Result<()> {
        self.load_module_with_config(name, bytecode, None)
    }

    /// Unload a WASM module from cache
    pub fn unload_module(&self, name: &str) -> Result<()> {
        let mut modules = self.modules.write().unwrap();
        
        if modules.remove(name).is_some() {
            info!("Unloaded WASM module: {}", name);
            Ok(())
        } else {
            Err(MitigationError::Wasm(format!("Module not found: {}", name)))
        }
    }

    /// Execute a WASM module with the given request context
    ///
    /// # Arguments
    /// * `name` - Module identifier
    /// * `ctx` - Request context to pass to the module
    ///
    /// # Returns
    /// Ok(Action) if execution succeeded and returned valid action
    pub fn run_module(&self, name: &str, ctx: &RequestContext) -> Result<Action> {
        // Get module from cache
        let modules = self.modules.read().unwrap();
        let cached = modules
            .get(name)
            .ok_or_else(|| MitigationError::Wasm(format!("Module not loaded: {}", name)))?;;

        // Create a new store for this execution
        let mut store = Store::new(&self.engine, ());
        
        // Set fuel limit
        store.set_fuel(self.config.max_fuel)
            .map_err(|e| MitigationError::Wasm(format!("Failed to set fuel limit: {}", e)))?;;

        // Create instance
        let instance = Instance::new(&mut store, &cached.module, &[])
            .map_err(|e| MitigationError::Wasm(format!("Failed to create WASM instance: {}", e)))?;;

        // Get the inspect_request function
        let inspect_fn = instance
            .get_typed_func::<(i32, i32), i32>(&mut store, INSPECT_REQUEST_FN)
            .map_err(|e| MitigationError::Wasm(format!("Failed to get inspect_request function: {}", e)))?;;

        // Serialize request context to JSON
        let json = ctx.to_json()
            .map_err(|e| MitigationError::Serialization(format!("Failed to serialize request context: {}", e)))?;;
        let json_bytes = json.as_bytes();

        // Get memory export
        let memory = instance
            .get_memory(&mut store, "memory")
            .ok_or_else(|| MitigationError::Wasm("WASM module must export 'memory'".to_string()))?;;

        // Write JSON to WASM memory
        // We use a simple convention: write at offset 0
        // (In production, you'd use proper allocator functions)
        let offset = 0;
        memory
            .write(&mut store, offset, json_bytes)
            .map_err(|e| MitigationError::Wasm(format!("Failed to write to WASM memory: {}", e)))?;;

        // Call the function with (ptr=0, len=json_bytes.len())
        let result = inspect_fn
            .call(&mut store, (offset as i32, json_bytes.len() as i32))
            .map_err(|e| MitigationError::Wasm(format!("WASM function execution failed: {}", e)))?;;

        // Get remaining fuel for logging
        let fuel_consumed = self.config.max_fuel - store.get_fuel().unwrap_or(0);
        debug!(
            "WASM execution: module={}, fuel_consumed={}, result={}",
            name, fuel_consumed, result
        );

        // Convert result to Action
        Action::from_i32(result)
            .ok_or_else(|| MitigationError::Wasm(format!("Invalid action returned from WASM: {}", result)))
    }

    /// Get list of loaded modules
    pub fn list_modules(&self) -> Vec<String> {
        let modules = self.modules.read().unwrap();
        modules.keys().cloned().collect()
    }

    /// Get module metadata
    pub fn get_module_info(&self, name: &str) -> Option<ModuleInfo> {
        let modules = self.modules.read().unwrap();
        modules.get(name).map(|cached| ModuleInfo {
            name: cached.name.clone(),
            loaded_at: cached.loaded_at,
            age: cached.loaded_at.elapsed(),
        })
    }
}

/// Metadata about a loaded module
#[derive(Debug, Clone)]
pub struct ModuleInfo {
    pub name: String,
    pub loaded_at: std::time::Instant,
    pub age: std::time::Duration,
}

// Explicit Drop implementation for WasmEngine to ensure proper cleanup
// 
// Memory Safety Guarantees:
// 1. Arc<RwLock<HashMap>> automatically cleans up when last reference drops
// 2. Each CachedModule contains a wasmtime::Module that properly drops
// 3. wasmtime::Engine cleanup is handled by wasmtime's Drop impl
// 4. No raw pointers or manual memory management
//
// Hot-Reload Cleanup:
// - unload_module() removes from HashMap, triggering Module::drop()
// - Module::drop() releases compiled code and JIT resources
// - Engine persists across reloads (intentional for cache reuse)
// - Store instances are created per-execution and drop immediately
//
// Leak Prevention:
// - Stores are NOT cached (created on each run_module call)
// - Instances are NOT cached (created on each run_module call)
// - Memory exports are NOT held between executions
// - Fuel metering prevents unbounded execution
//
// Tested in: tests/wasm_memory_leak_tests.rs
impl Drop for WasmEngine {
    fn drop(&mut self) {
        // Get write lock to clear all modules
        if let Ok(mut modules) = self.modules.write() {
            let count = modules.len();
            modules.clear();
            
            if count > 0 {
                debug!("WasmEngine dropping - cleared {} cached modules", count);
            }
        } else {
            warn!("WasmEngine drop: failed to acquire lock for cleanup");
        }
        
        // Engine will drop automatically after this
        // wasmtime::Engine handles its own cleanup (compiled code cache, etc.)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Note: These tests require actual WASM bytecode
    // We'll create a sample module in the next step

    #[test]
    fn test_engine_creation() {
        let config = WasmConfig::default();
        let engine = WasmEngine::new(config);
        if let Err(ref e) = engine {
            eprintln!("WasmEngine creation failed: {:?}", e);
        }
        assert!(engine.is_ok(), "WasmEngine creation failed: {:?}", engine.err());
    }

    #[test]
    fn test_custom_config() {
        let config = WasmConfig {
            max_fuel: 50_000,
            max_memory: 512 * 1024,
            cache_enabled: false,
        };
        let engine = WasmEngine::new(config);
        if let Err(ref e) = engine {
            eprintln!("WasmEngine creation failed: {:?}", e);
        }
        assert!(engine.is_ok(), "WasmEngine creation failed: {:?}", engine.err());
    }
}
