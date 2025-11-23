//! Common types used across the orchestrator

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::net::IpAddr;
use uuid::Uuid;

/// Node information stored in the registry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeInfo {
    /// Unique node identifier
    pub node_id: Uuid,
    /// Node IP address
    pub ip_address: IpAddr,
    /// Public IP address (for external access)
    pub public_ip: IpAddr,
    /// Node status
    pub status: NodeStatus,
    /// Last heartbeat timestamp
    pub last_heartbeat: DateTime<Utc>,
    /// CPU usage percentage (0.0 - 100.0)
    pub cpu_usage: f32,
    /// Memory usage percentage (0.0 - 100.0)
    pub memory_usage: f32,
    /// Active connections count
    pub active_connections: u64,
    /// XDP stats (if available)
    pub xdp_stats: Option<XdpStats>,
}

/// Node status enumeration
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum NodeStatus {
    /// Node is actively processing traffic
    Active,
    /// Node is draining connections (graceful shutdown)
    Draining,
    /// Node is offline/unreachable
    Offline,
    /// Node has failed health checks
    Unhealthy,
}

/// XDP statistics from a mitigation node
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct XdpStats {
    /// Total packets processed by XDP
    pub packets_processed: u64,
    /// Packets dropped by XDP
    pub packets_dropped: u64,
    /// Packets passed to kernel
    pub packets_passed: u64,
}

/// Orchestrator configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrchestratorConfig {
    /// Minimum number of nodes to maintain
    pub min_nodes: usize,
    /// Maximum number of nodes allowed
    pub max_nodes: usize,
    /// Minimum fleet size (alias for min_nodes)
    pub min_fleet_size: usize,
    /// CPU threshold for scaling up (percentage)
    pub cpu_scale_up_threshold: f32,
    /// CPU threshold for scaling down (percentage)
    pub cpu_scale_down_threshold: f32,
    /// Legacy field name for scale down threshold
    pub scale_down_cpu_threshold: f32,
    /// Cooldown period between scaling actions (seconds)
    pub scaling_cooldown_secs: u64,
    /// Scaling check interval (seconds)
    pub scaling_check_interval_seconds: u64,
    /// Webhook URL for scale-up actions
    pub scale_up_webhook_url: Option<String>,
    /// Webhook URL for provisioning
    pub provisioning_webhook_url: Option<String>,
    /// Webhook URL for self-healing actions
    pub self_healing_webhook_url: Option<String>,
}

impl Default for OrchestratorConfig {
    fn default() -> Self {
        Self {
            min_nodes: 2,
            max_nodes: 10,
            min_fleet_size: 2,
            cpu_scale_up_threshold: 70.0,
            cpu_scale_down_threshold: 30.0,
            scale_down_cpu_threshold: 30.0,
            scaling_cooldown_secs: 300, // 5 minutes
            scaling_check_interval_seconds: 60, // 1 minute
            scale_up_webhook_url: None,
            provisioning_webhook_url: None,
            self_healing_webhook_url: None,
        }
    }
}
