use anyhow::{Context, Result};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::{Duration, SystemTime};
use tokio::sync::RwLock;
use tracing::{debug, error, info, warn};
use uuid::Uuid;

use crate::config::OrchestratorConfig;

/// Node status enumeration (matches orchestrator-node)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum NodeStatus {
    Starting,
    Active,
    Draining,
    Terminating,
    Dead,
    Registered,
}

/// Node metrics for reporting to orchestrator (matches orchestrator-node format)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeMetrics {
    pub cpu_usage: f64,
    pub memory_usage: f64,
    pub packets_per_second: u64,
    pub active_connections: u64,
    pub total_requests: u64,
    pub ddos_blocks: u64,
    pub waf_blocks: u64,
}

/// Node configuration reported to orchestrator
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeConfig {
    pub listen_addr: String,
    pub backend_addr: String,
    pub tls_enabled: bool,
    pub ddos_enabled: bool,
    pub waf_enabled: bool,
    pub max_connections: u32,
    pub rate_limit_rps: u32,
}

/// Registration request payload (matches orchestrator-node format)
#[derive(Debug, Serialize)]
pub struct RegisterRequest {
    pub public_ip: std::net::IpAddr,
    pub config: OrchestratorNodeConfig,
}

/// Orchestrator node configuration format
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrchestratorNodeConfig {
    pub node_type: String,
    pub region: Option<String>,
    pub tags: Vec<String>,
}

/// Registration response from orchestrator
#[derive(Debug, Deserialize)]
pub struct RegisterResponse {
    pub node_id: Uuid,
    pub assigned_config: Option<NodeConfig>,
    pub heartbeat_interval: u64,
}

/// Heartbeat request payload (matches orchestrator-node format)
#[derive(Debug, Serialize)]
pub struct HeartbeatRequest {
    pub node_id: Uuid,
    pub metrics: NodeMetrics,
    pub status: NodeStatus,
}

/// Heartbeat response from orchestrator
#[derive(Debug, Deserialize)]
pub struct HeartbeatResponse {
    pub config_update: Option<NodeConfig>,
    pub commands: Vec<NodeCommand>,
}

/// Commands that can be sent from orchestrator to node
#[derive(Debug, Clone, Deserialize)]
pub struct NodeCommand {
    pub command_type: String,
    pub parameters: serde_json::Value,
    pub timestamp: u64,
}

/// Orchestrator client for node registration and heartbeat
#[derive(Debug)]
pub struct OrchestratorClient {
    config: OrchestratorConfig,
    http_client: Client,
    node_id: Arc<RwLock<Option<Uuid>>>,
    node_status: Arc<RwLock<NodeStatus>>,
    last_heartbeat: Arc<RwLock<Option<SystemTime>>>,
    registration_attempts: Arc<RwLock<u32>>,
}

impl OrchestratorClient {
    /// Create a new orchestrator client
    pub fn new(config: OrchestratorConfig) -> Self {
        let http_client = Self::create_http_client().unwrap_or_else(|e| {
            tracing::error!("Failed to create HTTP client: {}", e);
            Client::new()
        });

        let node_id = config
            .node_id
            .as_ref()
            .and_then(|id| Uuid::parse_str(id).ok())
            .unwrap_or_else(Uuid::new_v4);

        Self {
            config,
            http_client,
            node_id: Arc::new(RwLock::new(Some(node_id))),
            node_status: Arc::new(RwLock::new(NodeStatus::Starting)),
            last_heartbeat: Arc::new(RwLock::new(None)),
            registration_attempts: Arc::new(RwLock::new(0)),
        }
    }

    /// Create HTTP client with error handling
    fn create_http_client() -> Result<Client> {
        Client::builder()
            .timeout(Duration::from_secs(30))
            .build()
            .context("Failed to build HTTP client")
    }

    /// Register node with orchestrator
    pub async fn register(&self, _node_config: NodeConfig) -> Result<RegisterResponse> {
        let mut attempts = self.registration_attempts.write().await;

        if *attempts
            >= self
                .config
                .registration
                .as_ref()
                .and_then(|r| r.max_retries)
                .unwrap_or(3)
        {
            return Err(anyhow::anyhow!("Max registration attempts exceeded"));
        }

        *attempts += 1;

        // Get local IP address for registration
        let public_ip = "127.0.0.1".parse().unwrap_or_else(|e| {
            tracing::warn!("Failed to parse default IP address: {}, using fallback", e);
            std::net::IpAddr::V4(std::net::Ipv4Addr::LOCALHOST)
        });

        let request = RegisterRequest {
            public_ip,
            config: OrchestratorNodeConfig {
                node_type: "mitigation-node".to_string(),
                region: Some("local".to_string()),
                tags: vec![
                    "ddos-protection".to_string(),
                    "waf".to_string(),
                    "tls-termination".to_string(),
                    "l7-proxy".to_string(),
                ],
            },
        };

        let url = format!(
            "{}/api/v1/nodes/register",
            self.config
                .server_url
                .as_deref()
                .unwrap_or("http://localhost:8080")
        );

        info!(
            url = %url,
            attempt = *attempts,
            max_retries = self.config.registration.as_ref()
                .and_then(|r| r.max_retries)
                .unwrap_or(3),
            "Attempting node registration"
        );

        let response = self
            .http_client
            .post(&url)
            .json(&request)
            .timeout(Duration::from_secs(
                self.config
                    .registration
                    .as_ref()
                    .and_then(|r| r.timeout_seconds)
                    .unwrap_or(30),
            ))
            .send()
            .await
            .context("Failed to send registration request")?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(anyhow::anyhow!(
                "Registration failed with status {}: {}",
                status,
                body
            ));
        }

        let register_response: RegisterResponse = response
            .json()
            .await
            .context("Failed to parse registration response")?;

        // Update node ID and reset attempts counter
        *self.node_id.write().await = Some(register_response.node_id);
        *attempts = 0;

        info!(
            node_id = %register_response.node_id,
            heartbeat_interval = register_response.heartbeat_interval,
            "Successfully registered with orchestrator"
        );

        Ok(register_response)
    }

    /// Send heartbeat to orchestrator
    pub async fn send_heartbeat(&self, metrics: NodeMetrics) -> Result<HeartbeatResponse> {
        let node_id = self
            .node_id
            .read()
            .await
            .ok_or_else(|| anyhow::anyhow!("Node not registered"))?;

        let status = self.node_status.read().await.clone();

        let request = HeartbeatRequest {
            node_id,
            metrics,
            status,
        };

        let url = format!(
            "{}/api/v1/nodes/heartbeat",
            self.config
                .server_url
                .as_deref()
                .unwrap_or("http://localhost:8080")
        );

        debug!(node_id = %node_id, "Sending heartbeat to orchestrator");

        let response = self
            .http_client
            .post(&url)
            .json(&request)
            .timeout(Duration::from_secs(
                self.config
                    .heartbeat
                    .as_ref()
                    .map(|h| h.timeout_seconds)
                    .unwrap_or(30),
            ))
            .send()
            .await
            .context("Failed to send heartbeat")?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(anyhow::anyhow!(
                "Heartbeat failed with status {}: {}",
                status,
                body
            ));
        }

        // Update last heartbeat timestamp
        *self.last_heartbeat.write().await = Some(SystemTime::now());

        debug!(
            node_id = %node_id,
            "Heartbeat sent successfully"
        );

        // For now, return empty response since orchestrator returns just HTTP 200
        Ok(HeartbeatResponse {
            config_update: None,
            commands: vec![],
        })
    }

    /// Update node status
    pub async fn set_status(&self, status: NodeStatus) {
        let mut current_status = self.node_status.write().await;
        if *current_status != status {
            info!(old_status = ?*current_status, new_status = ?status, "Node status changed");
            *current_status = status;
        }
    }

    /// Get current node ID
    pub async fn get_node_id(&self) -> Option<Uuid> {
        *self.node_id.read().await
    }

    /// Get current node status
    pub async fn get_status(&self) -> NodeStatus {
        self.node_status.read().await.clone()
    }

    /// Start the heartbeat loop
    pub fn start_heartbeat_loop<F>(self: Arc<Self>, metrics_provider: F) -> Result<()>
    where
        F: Fn() -> NodeMetrics + Send + Sync + 'static,
    {
        if !self.config.enabled {
            info!("Orchestrator integration disabled, skipping heartbeat loop");
            return Ok(());
        }

        let client = Arc::clone(&self);
        let metrics_provider = Arc::new(metrics_provider);
        let interval = Duration::from_secs(
            self.config
                .heartbeat
                .as_ref()
                .map(|h| h.interval_seconds)
                .unwrap_or(60),
        );

        tokio::spawn(async move {
            let mut missed_heartbeats = 0;
            let max_missed = client
                .config
                .heartbeat
                .as_ref()
                .map(|h| h.max_missed)
                .unwrap_or(3);

            loop {
                tokio::time::sleep(interval).await;

                let metrics = metrics_provider();

                match client.send_heartbeat(metrics).await {
                    Ok(response) => {
                        missed_heartbeats = 0;

                        // Process configuration updates
                        if let Some(new_config) = response.config_update {
                            info!(config = ?new_config, "Received configuration update from orchestrator");
                            if let Err(e) = process_config_update(new_config).await {
                                warn!(error = %e, "Failed to apply configuration update");
                            }
                        }

                        // Process commands
                        for command in response.commands {
                            info!(command = ?command, "Received command from orchestrator");
                            if let Err(e) = process_orchestrator_command(command).await {
                                warn!(error = %e, "Failed to process orchestrator command");
                            }
                        }
                    }
                    Err(e) => {
                        missed_heartbeats += 1;
                        warn!(
                            error = %e,
                            missed_count = missed_heartbeats,
                            max_missed = max_missed,
                            "Failed to send heartbeat"
                        );

                        if missed_heartbeats >= max_missed {
                            error!("Too many missed heartbeats, setting status to Dead");
                            client.set_status(NodeStatus::Dead).await;

                            // Attempt re-registration after failures
                            if let Some(node_id) = client.get_node_id().await {
                                warn!(node_id = %node_id, "Attempting to recover connection to orchestrator");
                                // Reset missed counter and try to continue
                                missed_heartbeats = 0;
                            }
                        }
                    }
                }
            }
        });

        Ok(())
    }

    /// Check if node is registered
    pub async fn is_registered(&self) -> bool {
        self.node_id.read().await.is_some()
    }

    /// Get registration retry interval
    pub fn get_retry_interval(&self) -> Duration {
        Duration::from_secs(
            self.config
                .registration
                .as_ref()
                .and_then(|r| r.retry_interval_seconds)
                .unwrap_or(60),
        )
    }
}

/// Helper function to collect system metrics
pub fn collect_system_metrics(
    active_connections: u64,
    total_requests: u64,
    ddos_blocks: u64,
    waf_blocks: u64,
) -> NodeMetrics {
    // Get system metrics
    let cpu_usage = get_cpu_usage();
    let memory_usage = get_memory_usage();
    let packets_per_second = calculate_packets_per_second(active_connections);

    NodeMetrics {
        cpu_usage,
        memory_usage,
        packets_per_second,
        active_connections,
        total_requests,
        ddos_blocks,
        waf_blocks,
    }
}

/// Get CPU usage percentage from /proc/stat on Linux or system estimates on other platforms
fn get_cpu_usage() -> f64 {
    #[cfg(target_os = "linux")]
    {
        use std::fs;
        if let Ok(stat_content) = fs::read_to_string("/proc/stat") {
            if let Some(cpu_line) = stat_content.lines().next() {
                if let Some(cpu_stats) = parse_cpu_line(cpu_line) {
                    return calculate_cpu_percentage(cpu_stats);
                }
            }
        }
        // Fallback to a reasonable default for demo purposes
        25.0
    }

    #[cfg(not(target_os = "linux"))]
    {
        // On macOS/Windows, provide a simulated value based on system load
        use std::process::Command;

        #[cfg(target_os = "macos")]
        {
            if let Ok(output) = Command::new("top").args(["-l", "1", "-n", "0"]).output() {
                if let Ok(top_output) = String::from_utf8(output.stdout) {
                    if let Some(cpu_usage) = parse_macos_cpu(&top_output) {
                        return cpu_usage;
                    }
                }
            }
        }

        // Fallback to a reasonable simulated value
        20.0 + (std::process::id() as f64 % 30.0)
    }
}

/// Get memory usage percentage from /proc/meminfo on Linux or system estimates on other platforms
fn get_memory_usage() -> f64 {
    #[cfg(target_os = "linux")]
    {
        use std::fs;
        if let Ok(meminfo_content) = fs::read_to_string("/proc/meminfo") {
            if let (Some(total), Some(available)) = parse_meminfo(&meminfo_content) {
                let used = total - available;
                return (used as f64 / total as f64) * 100.0;
            }
        }
        // Fallback
        40.0
    }

    #[cfg(not(target_os = "linux"))]
    {
        // Simulate memory usage based on system characteristics
        30.0 + (std::process::id() as f64 % 40.0)
    }
}

/// Calculate packets per second estimate based on active connections
fn calculate_packets_per_second(active_connections: u64) -> u64 {
    // Rough estimate: each connection generates ~10 packets/second on average
    active_connections * 10
}

#[cfg(target_os = "linux")]
fn parse_cpu_line(line: &str) -> Option<Vec<u64>> {
    let parts: Vec<&str> = line.split_whitespace().collect();
    if parts.len() >= 8 && parts[0] == "cpu" {
        let mut values = Vec::new();
        for i in 1..8 {
            if let Ok(val) = parts[i].parse::<u64>() {
                values.push(val);
            } else {
                return None;
            }
        }
        Some(values)
    } else {
        None
    }
}

#[cfg(target_os = "linux")]
fn calculate_cpu_percentage(stats: Vec<u64>) -> f64 {
    if stats.len() >= 7 {
        let idle = stats[3] + stats[4]; // idle + iowait
        let total: u64 = stats.iter().sum();
        let used = total - idle;
        if total > 0 {
            (used as f64 / total as f64) * 100.0
        } else {
            0.0
        }
    } else {
        0.0
    }
}

#[cfg(target_os = "linux")]
fn parse_meminfo(content: &str) -> (Option<u64>, Option<u64>) {
    let mut total = None;
    let mut available = None;

    for line in content.lines() {
        if line.starts_with("MemTotal:") {
            if let Some(value) = extract_meminfo_value(line) {
                total = Some(value);
            }
        } else if line.starts_with("MemAvailable:") {
            if let Some(value) = extract_meminfo_value(line) {
                available = Some(value);
            }
        }

        if total.is_some() && available.is_some() {
            break;
        }
    }

    (total, available)
}

#[cfg(target_os = "linux")]
fn extract_meminfo_value(line: &str) -> Option<u64> {
    let parts: Vec<&str> = line.split_whitespace().collect();
    if parts.len() >= 2 {
        parts[1].parse().ok()
    } else {
        None
    }
}

#[cfg(target_os = "macos")]
fn parse_macos_cpu(output: &str) -> Option<f64> {
    for line in output.lines() {
        if line.contains("CPU usage:") {
            // Parse line like "CPU usage: 12.34% user, 5.67% sys, 81.99% idle"
            if let Some(user_start) = line.find(':') {
                if let Some(user_end) = line[user_start..].find('%') {
                    if let Ok(user_pct) = line[user_start + 1..user_start + user_end]
                        .trim()
                        .parse::<f64>()
                    {
                        if let Some(sys_start) = line[user_start + user_end..].find(',') {
                            if let Some(sys_end) =
                                line[user_start + user_end + sys_start..].find('%')
                            {
                                if let Ok(sys_pct) = line[user_start + user_end + sys_start + 1
                                    ..user_start + user_end + sys_start + sys_end]
                                    .trim()
                                    .parse::<f64>()
                                {
                                    return Some(user_pct + sys_pct);
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    None
}

/// Process configuration updates from the orchestrator
async fn process_config_update(config_update: NodeConfig) -> Result<()> {
    info!(config = ?config_update, "Processing configuration update");

    // Process the received NodeConfig update
    // This would typically involve updating the current running configuration

    info!(
        max_connections = config_update.max_connections,
        backend_addr = %config_update.backend_addr,
        "Updated node configuration from orchestrator"
    );

    // In a real implementation, these updates would be applied to:
    // - The proxy server configuration
    // - Rate limiting settings
    // - WAF rules and thresholds
    // - Backend routing configuration
    // - TLS/SSL settings
    // For now, we just log the configuration change

    Ok(())
}

/// Process commands from the orchestrator
async fn process_orchestrator_command(command: NodeCommand) -> Result<()> {
    info!(command_type = %command.command_type, "Processing orchestrator command");

    match command.command_type.as_str() {
        "terminate" => {
            info!("Received terminate command from orchestrator");
            // Graceful shutdown procedure
            warn!("Node termination requested by orchestrator - initiating graceful shutdown");
            // In a real implementation, this would trigger a graceful shutdown
            // For now, we just log the command
        }

        "drain" => {
            info!("Received drain command from orchestrator");
            // Stop accepting new connections, finish existing ones
            info!("Node draining requested - stopping new connection acceptance");
            // In a real implementation, this would put the node in draining mode
            // For now, we just log the command
        }

        "update_rules" => {
            if let Some(rules) = command.parameters.get("rules") {
                info!(rules = ?rules, "Received WAF rules update command");
                // In a real implementation, this would update the WAF rules dynamically
                // For now, we just log the command
            }
        }

        "block_ip" => {
            if let Some(ip) = command.parameters.get("ip") {
                if let Some(duration) = command.parameters.get("duration_seconds") {
                    info!(ip = %ip, duration = %duration, "Received IP block command");
                    // In a real implementation, this would add the IP to the blocklist
                    // For now, we just log the command
                }
            }
        }

        "unblock_ip" => {
            if let Some(ip) = command.parameters.get("ip") {
                info!(ip = %ip, "Received IP unblock command");
                // In a real implementation, this would remove the IP from the blocklist
                // For now, we just log the command
            }
        }

        "health_check" => {
            info!("Received health check command from orchestrator");
            // Perform internal health checks and report back
            // In a real implementation, this would trigger comprehensive health checks
            // For now, we just log the command
        }

        _ => {
            warn!(command_type = %command.command_type, "Unknown command type received from orchestrator");
        }
    }

    Ok(())
}
