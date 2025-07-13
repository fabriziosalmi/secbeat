//! SecBeat Mitigation Node Library
//! 
//! This library provides core functionality for DDoS mitigation, WAF protection,
//! and traffic filtering.

pub mod config;
pub mod ddos;
pub mod events;
pub mod management;
pub mod orchestrator;
pub mod syn_proxy;
pub mod tcp_proxy;
pub mod waf;

// Re-export commonly used types
pub use config::{MitigationConfig, DdosConfig};
pub use ddos::DdosProtection;
pub use events::EventSystem;
pub use management::{ShutdownSignal, ManagementState};
pub use waf::{WafEngine, WafResult};

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
