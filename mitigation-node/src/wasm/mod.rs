// WASM Module - WebAssembly runtime for hot-reloadable WAF rules
//
// This module provides:
// - Custom ABI for host-guest communication (abi.rs)
// - WASM engine with fuel limits and module caching (engine.rs)
// - Hot-reload capability for dynamic rule deployment

pub mod abi;
pub mod engine;

pub use abi::{Action, RequestContext, INSPECT_REQUEST_FN};
pub use engine::{WasmConfig, WasmEngine, ModuleInfo};
