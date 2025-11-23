//! Orchestrator Node Library
//!
//! This library provides the core functionality for the SecBeat orchestrator,
//! including experts for behavioral analysis, threat intelligence, and ML-based
//! anomaly detection.

pub mod experts;
pub mod types;

// Re-export commonly used types
pub use experts::{
    BehavioralConfig, BehavioralExpert, BlockCommand, RequestMetadata, ResourceManager,
    TelemetryEvent, ThreatIntelExpert, TrafficFeatures,
};
pub use types::{NodeInfo, NodeStatus, OrchestratorConfig, XdpStats};
