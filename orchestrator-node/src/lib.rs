//! Orchestrator Node Library
//!
//! This library provides the core functionality for the SecBeat orchestrator,
//! including experts for behavioral analysis, threat intelligence, and ML-based
//! anomaly detection.

pub mod experts;
pub mod rule_gen;
pub mod types;

// Re-export commonly used types
pub use experts::{
    AnomalyConfig, AnomalyExpert, AnomalyScore, BehavioralConfig, BehavioralExpert, BlockCommand,
    OperatingMode, RequestMetadata, ResourceManager, TelemetryEvent, ThreatIntelExpert,
    TrafficFeatures,
};
pub use rule_gen::{GeneratorStats, RuleGenerator, WafConfig, WafRule, WasmDeployment};
pub use types::{NodeInfo, NodeStatus, OrchestratorConfig, XdpStats};
