//! SecBeat Mitigation Node Library
//!
//! This library provides core functionality for DDoS mitigation, WAF protection,
//! and traffic filtering.

pub mod config;
pub mod ddos;
pub mod distributed;
pub mod error;
pub mod events;
pub mod management;
pub mod orchestrator;
pub mod secret;
pub mod syn_proxy;
pub mod tcp_proxy;
pub mod waf;
pub mod wasm;

#[cfg(target_os = "linux")]
pub mod bpf_loader;

// Re-export commonly used types
pub use config::{DdosConfig, MitigationConfig};
pub use ddos::DdosProtection;
pub use distributed::{GCounter, NodeId, PNCounter, StateManager, StateSyncConfig};
pub use error::{MitigationError, Result};
pub use events::EventSystem;
pub use management::{ManagementState, ShutdownSignal};
pub use secret::Secret;
pub use waf::{WafEngine, WafResult};
pub use wasm::{Action as WasmAction, RequestContext, WasmConfig, WasmEngine};

#[cfg(target_os = "linux")]
pub use bpf_loader::BpfHandle;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_library_imports() {
        // Simple test to ensure all modules can be imported
        let _ = std::any::type_name::<MitigationConfig>();
        let _ = std::any::type_name::<DdosConfig>();
        let _ = std::any::type_name::<WafEngine>();
        let _ = std::any::type_name::<WafResult>();
    }
}
