use anyhow::{Context, Result};
use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::Json,
    routing::{get, post},
    Router,
};
use chrono::{DateTime, Utc};
use dashmap::DashMap;
use metrics::{counter, describe_counter, describe_gauge, gauge};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use tokio::time;
use tower_http::{cors::CorsLayer, trace::TraceLayer};
use tracing::{debug, error, info, instrument, warn};
use uuid::Uuid;

mod experts;
use experts::{BehavioralConfig, BehavioralExpert, ResourceManager, ThreatIntelExpert};

/// Node information stored in the registry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeInfo {
    /// Unique node identifier
    pub node_id: Uuid,
    /// Public IP address of the node
    pub public_ip: std::net::IpAddr,
    /// Last heartbeat timestamp
    pub last_heartbeat: DateTime<Utc>,
    /// Current node status
    pub status: NodeStatus,
    /// Node metrics from last heartbeat
    pub metrics: Option<NodeMetrics>,
    /// When the node was first registered
    pub registered_at: DateTime<Utc>,
    /// Node configuration
    pub config: NodeConfig,
}

/// Node status enumeration
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum NodeStatus {
    /// Node is active and responding
    Active,
    /// Node is draining connections
    Draining,
    /// Node is being terminated
    Terminating,
    /// Node is not responding (missed heartbeats)
    Dead,
    /// Node just registered but no heartbeat yet
    Registered,
}

/// Node metrics from heartbeat
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeMetrics {
    /// CPU usage percentage
    pub cpu_usage: f64,
    /// Memory usage percentage
    pub memory_usage: f64,
    /// Packets per second being processed
    pub packets_per_second: u64,
    /// Active connections
    pub active_connections: u64,
    /// Total requests processed
    pub total_requests: u64,
    /// Requests blocked by DDoS protection
    pub ddos_blocks: u64,
    /// Requests blocked by WAF
    pub waf_blocks: u64,
}

/// Node configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeConfig {
    /// Node type (e.g., "mitigation_v2", "udp_optimized_v1")
    pub node_type: String,
    /// Region or availability zone
    pub region: Option<String>,
    /// Additional tags
    pub tags: Vec<String>,
}

/// Registration request from a node
#[derive(Debug, Deserialize)]
pub struct RegisterRequest {
    /// Public IP of the node
    pub public_ip: std::net::IpAddr,
    /// Node configuration
    pub config: NodeConfig,
}

/// Registration response to a node
#[derive(Debug, Serialize)]
pub struct RegisterResponse {
    /// Assigned node ID
    pub node_id: Uuid,
    /// Heartbeat interval in seconds
    pub heartbeat_interval: u64,
    /// Orchestrator endpoints
    pub endpoints: OrchestratorEndpoints,
}

/// Orchestrator endpoint information
#[derive(Debug, Serialize)]
pub struct OrchestratorEndpoints {
    /// Heartbeat endpoint
    pub heartbeat_url: String,
    /// Control message endpoint (for receiving commands)
    pub control_url: String,
}

/// Heartbeat request from a node
#[derive(Debug, Deserialize)]
pub struct HeartbeatRequest {
    /// Node ID
    pub node_id: Uuid,
    /// Current node metrics
    pub metrics: NodeMetrics,
    /// Current status
    pub status: NodeStatus,
}

/// Fleet statistics
#[derive(Debug, Serialize)]
pub struct FleetStats {
    /// Total number of registered nodes
    pub total_nodes: usize,
    /// Number of active nodes
    pub active_nodes: usize,
    /// Number of dead nodes
    pub dead_nodes: usize,
    /// Average CPU usage across fleet
    pub avg_cpu_usage: f64,
    /// Average memory usage across fleet
    pub avg_memory_usage: f64,
    /// Total packets per second across fleet
    pub total_pps: u64,
    /// Total active connections across fleet
    pub total_connections: u64,
}

/// Orchestrator application state
#[derive(Debug, Clone)]
pub struct OrchestratorState {
    /// Node registry
    pub nodes: Arc<DashMap<Uuid, NodeInfo>>,
    /// Configuration
    pub config: OrchestratorConfig,
    /// Threat intelligence expert
    pub threat_intel: Arc<ThreatIntelExpert>,
}

/// Orchestrator configuration
#[derive(Debug, Clone)]
pub struct OrchestratorConfig {
    /// Listen address for API
    pub listen_addr: SocketAddr,
    /// Heartbeat timeout in seconds
    pub heartbeat_timeout: u64,
    /// Dead node check interval in seconds
    pub dead_node_check_interval: u64,
    /// Metrics server address
    pub metrics_addr: SocketAddr,
    /// NATS server URL for messaging
    pub nats_url: String,
    /// Webhook URL for provisioning new nodes
    pub provisioning_webhook_url: String,
    /// Minimum fleet size (prevents scaling below this)
    pub min_fleet_size: u32,
    /// CPU threshold for scaling up (0.0-1.0)
    pub scale_up_cpu_threshold: f32,
    /// CPU threshold for scaling down (0.0-1.0)
    pub scale_down_cpu_threshold: f32,
    /// Interval between scaling checks in seconds
    pub scaling_check_interval_seconds: u64,
}

impl Default for OrchestratorConfig {
    fn default() -> Self {
        Self {
            listen_addr: "127.0.0.1:3030".parse().unwrap(),
            heartbeat_timeout: 30,
            dead_node_check_interval: 10,
            metrics_addr: "127.0.0.1:9091".parse().unwrap(),
            nats_url: "nats://127.0.0.1:4222".to_string(),
            provisioning_webhook_url: "http://localhost:8000/provision".to_string(),
            min_fleet_size: 1,
            scale_up_cpu_threshold: 0.80,
            scale_down_cpu_threshold: 0.30,
            scaling_check_interval_seconds: 60,
        }
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize structured logging
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "orchestrator_node=info".into()),
        )
        .with_target(false)
        .with_thread_ids(true)
        .with_line_number(true)
        .init();

    info!(
        "Starting SecBeat Orchestrator v{} - Phase 4: Fleet Management",
        env!("CARGO_PKG_VERSION")
    );

    // Load configuration (using defaults for now)
    let config = OrchestratorConfig::default();

    // Initialize threat intelligence expert
    info!(nats_url = %config.nats_url, "Initializing threat intelligence expert");
    let threat_intel = match ThreatIntelExpert::new(&config.nats_url).await {
        Ok(expert) => {
            info!("Successfully initialized threat intelligence expert");
            Arc::new(expert)
        }
        Err(e) => {
            error!(error = %e, "Failed to initialize threat intelligence expert");
            return Err(e);
        }
    };

    // Start threat intelligence event consumer
    let threat_intel_consumer = Arc::clone(&threat_intel);
    tokio::spawn(async move {
        if let Err(e) = threat_intel_consumer.start_event_consumer().await {
            error!(error = %e, "Threat intelligence event consumer failed");
        }
    });

    // Initialize behavioral analysis expert
    info!(nats_url = %config.nats_url, "Initializing behavioral analysis expert");
    let behavioral_config = BehavioralConfig {
        window_size_seconds: 60,
        error_threshold: 50,           // 50 errors in 60 seconds triggers block
        request_threshold: 1000,       // 1000 requests in 60 seconds triggers block
        block_duration_seconds: 300,   // Block for 5 minutes
        cleanup_interval_seconds: 300, // Cleanup every 5 minutes
    };
    let behavioral_expert = match BehavioralExpert::new(&config.nats_url, behavioral_config).await {
        Ok(expert) => {
            info!("Successfully initialized behavioral analysis expert");
            Arc::new(expert)
        }
        Err(e) => {
            error!(error = %e, "Failed to initialize behavioral analysis expert");
            return Err(e);
        }
    };

    // Start behavioral analysis telemetry consumer
    let behavioral_consumer = Arc::clone(&behavioral_expert);
    tokio::spawn(async move {
        if let Err(e) = behavioral_consumer.start_telemetry_consumer().await {
            error!(error = %e, "Behavioral analysis telemetry consumer failed");
        }
    });

    // Start behavioral analysis cleanup task
    Arc::clone(&behavioral_expert).spawn_cleanup_task();

    // Initialize orchestrator state
    let state = OrchestratorState {
        nodes: Arc::new(DashMap::new()),
        config: config.clone(),
        threat_intel,
    };

    // Initialize resource manager
    info!("Initializing resource manager for intelligent scaling and self-healing");
    let resource_manager = ResourceManager::new(Arc::clone(&state.nodes), config.clone());

    // Get reference to terminating nodes for self-healing
    let terminating_nodes_ref = resource_manager.get_terminating_nodes();

    // Start resource manager
    let _resource_manager_handle = tokio::spawn(async move {
        if let Err(e) = resource_manager.start().await {
            error!(error = %e, "Resource manager failed");
        }
    });

    // Initialize metrics
    initialize_metrics();

    // Start metrics server
    let metrics_config = config.clone();
    let metrics_state = state.clone();
    tokio::spawn(async move {
        if let Err(e) = start_metrics_server(metrics_config.metrics_addr, metrics_state).await {
            error!(error = %e, "Failed to start metrics server");
        }
    });

    // Start dead node detection task with self-healing
    let dead_node_state = state.clone();
    let terminating_nodes_for_monitor = Arc::clone(&terminating_nodes_ref);
    tokio::spawn(async move {
        dead_node_monitor_with_self_healing(dead_node_state, terminating_nodes_for_monitor).await;
    });

    // Create API router
    let app = create_api_router(state);

    info!(listen_addr = %config.listen_addr, "Starting orchestrator API server");

    // Start the server
    let listener = tokio::net::TcpListener::bind(&config.listen_addr)
        .await
        .with_context(|| format!("Failed to bind to {}", config.listen_addr))?;

    axum::serve(listener, app)
        .await
        .context("API server error")?;

    Ok(())
}

/// Create the API router with all endpoints
fn create_api_router(state: OrchestratorState) -> Router {
    Router::new()
        .route("/api/v1/nodes/register", post(register_node))
        .route("/api/v1/nodes/heartbeat", post(node_heartbeat))
        .route("/api/v1/nodes", get(list_nodes))
        .route("/api/v1/nodes/:node_id", get(get_node))
        .route("/api/v1/nodes/:node_id/terminate", post(terminate_node))
        .route("/api/v1/fleet/stats", get(fleet_statistics))
        .route("/api/v1/rules/block_ip", post(block_ip_endpoint))
        .route("/api/v1/rules/blocked_ips", get(get_blocked_ips_endpoint))
        .route("/api/v1/health", get(health_check))
        .layer(CorsLayer::permissive())
        .layer(TraceLayer::new_for_http())
        .with_state(state)
}

/// Register a new node
#[instrument(skip(state))]
async fn register_node(
    State(state): State<OrchestratorState>,
    Json(request): Json<RegisterRequest>,
) -> Result<Json<RegisterResponse>, StatusCode> {
    let node_id = Uuid::new_v4();
    let now = Utc::now();

    let node_info = NodeInfo {
        node_id,
        public_ip: request.public_ip,
        last_heartbeat: now,
        status: NodeStatus::Registered,
        metrics: None,
        registered_at: now,
        config: request.config,
    };

    // Insert into registry
    state.nodes.insert(node_id, node_info);

    // Update metrics
    counter!("orchestrator_nodes_registered_total", 1);
    gauge!("orchestrator_total_nodes", state.nodes.len() as f64);

    info!(
        node_id = %node_id,
        public_ip = %request.public_ip,
        "New node registered successfully"
    );

    let response = RegisterResponse {
        node_id,
        heartbeat_interval: 10, // 10 seconds
        endpoints: OrchestratorEndpoints {
            heartbeat_url: format!("http://{}/api/v1/nodes/heartbeat", state.config.listen_addr),
            control_url: format!(
                "http://{}/api/v1/nodes/{}/commands",
                state.config.listen_addr, node_id
            ),
        },
    };

    Ok(Json(response))
}

/// Process heartbeat from a node
#[instrument(skip(state))]
async fn node_heartbeat(
    State(state): State<OrchestratorState>,
    Json(request): Json<HeartbeatRequest>,
) -> Result<StatusCode, StatusCode> {
    let node_id = request.node_id;

    match state.nodes.get_mut(&node_id) {
        Some(mut node) => {
            // Update heartbeat and metrics
            node.last_heartbeat = Utc::now();
            node.status = request.status;
            node.metrics = Some(request.metrics);

            counter!("orchestrator_heartbeats_received_total", 1);

            debug!(
                node_id = %node_id,
                cpu_usage = node.metrics.as_ref().map(|m| m.cpu_usage).unwrap_or(0.0),
                active_connections = node.metrics.as_ref().map(|m| m.active_connections).unwrap_or(0),
                "Heartbeat received from node"
            );

            Ok(StatusCode::OK)
        }
        None => {
            warn!(node_id = %node_id, "Heartbeat from unknown node");
            counter!("orchestrator_heartbeats_unknown_node", 1);
            Err(StatusCode::NOT_FOUND)
        }
    }
}

/// List all nodes
#[instrument(skip(state))]
async fn list_nodes(State(state): State<OrchestratorState>) -> Json<Vec<NodeInfo>> {
    let nodes: Vec<NodeInfo> = state
        .nodes
        .iter()
        .map(|entry| entry.value().clone())
        .collect();
    Json(nodes)
}

/// Get specific node information
#[instrument(skip(state))]
async fn get_node(
    State(state): State<OrchestratorState>,
    Path(node_id): Path<Uuid>,
) -> Result<Json<NodeInfo>, StatusCode> {
    match state.nodes.get(&node_id) {
        Some(node) => Ok(Json(node.clone())),
        None => Err(StatusCode::NOT_FOUND),
    }
}

/// Terminate a specific node
#[instrument(skip(state))]
async fn terminate_node(
    State(state): State<OrchestratorState>,
    Path(node_id): Path<Uuid>,
) -> Result<StatusCode, StatusCode> {
    match state.nodes.get_mut(&node_id) {
        Some(mut node) => {
            node.status = NodeStatus::Terminating;

            info!(node_id = %node_id, "Node marked for termination");

            // In a real implementation, we would send a termination command to the node
            // For now, we just mark it as terminating
            counter!("orchestrator_nodes_terminated_total", 1);

            Ok(StatusCode::OK)
        }
        None => Err(StatusCode::NOT_FOUND),
    }
}

/// Get fleet statistics
#[instrument(skip(state))]
async fn fleet_statistics(State(state): State<OrchestratorState>) -> Json<FleetStats> {
    let nodes: Vec<NodeInfo> = state
        .nodes
        .iter()
        .map(|entry| entry.value().clone())
        .collect();

    let total_nodes = nodes.len();
    let active_nodes = nodes
        .iter()
        .filter(|n| n.status == NodeStatus::Active)
        .count();
    let dead_nodes = nodes
        .iter()
        .filter(|n| n.status == NodeStatus::Dead)
        .count();

    let (avg_cpu, avg_memory, total_pps, total_connections) = if nodes.is_empty() {
        (0.0, 0.0, 0, 0)
    } else {
        let metrics: Vec<&NodeMetrics> = nodes.iter().filter_map(|n| n.metrics.as_ref()).collect();

        let avg_cpu =
            metrics.iter().map(|m| m.cpu_usage).sum::<f64>() / metrics.len().max(1) as f64;
        let avg_memory =
            metrics.iter().map(|m| m.memory_usage).sum::<f64>() / metrics.len().max(1) as f64;
        let total_pps = metrics.iter().map(|m| m.packets_per_second).sum();
        let total_connections = metrics.iter().map(|m| m.active_connections).sum();

        (avg_cpu, avg_memory, total_pps, total_connections)
    };

    Json(FleetStats {
        total_nodes,
        active_nodes,
        dead_nodes,
        avg_cpu_usage: avg_cpu,
        avg_memory_usage: avg_memory,
        total_pps,
        total_connections,
    })
}

/// Health check endpoint
#[instrument]
async fn health_check() -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "status": "healthy",
        "timestamp": Utc::now(),
        "version": env!("CARGO_PKG_VERSION")
    }))
}

/// Enhanced monitor for dead nodes with self-healing capabilities
#[instrument(skip(state, terminating_nodes))]
async fn dead_node_monitor_with_self_healing(
    state: OrchestratorState,
    terminating_nodes: Arc<RwLock<HashSet<Uuid>>>,
) {
    let mut interval = time::interval(Duration::from_secs(state.config.dead_node_check_interval));

    // HTTP client for self-healing webhooks
    let http_client = match reqwest::Client::builder()
        .timeout(Duration::from_secs(30))
        .build()
    {
        Ok(client) => client,
        Err(e) => {
            error!("Failed to create HTTP client for self-healing: {}", e);
            return;
        }
    };

    loop {
        interval.tick().await;

        let heartbeat_timeout = Duration::from_secs(state.config.heartbeat_timeout);
        let now = Utc::now();

        let mut dead_nodes = Vec::new();
        let mut unexpected_failures = Vec::new();

        // Check for nodes that haven't sent heartbeats
        for mut entry in state.nodes.iter_mut() {
            let node = entry.value_mut();
            let last_heartbeat = node.last_heartbeat;
            let time_since_heartbeat = now - last_heartbeat;

            if let Ok(duration_since_heartbeat) = chrono::Duration::from_std(heartbeat_timeout) {
                if time_since_heartbeat > duration_since_heartbeat
                    && node.status != NodeStatus::Dead
                    && node.status != NodeStatus::Terminating
                {
                    let node_id = node.node_id;
                    let node_ip = node.public_ip;

                    // PHASE 7: Check if this was an expected termination
                    let was_expected = {
                        let terminating = terminating_nodes.read().await;
                        terminating.contains(&node_id)
                    };

                    if was_expected {
                        info!(
                            node_id = %node_id,
                            public_ip = %node_ip,
                            time_since_heartbeat = %time_since_heartbeat,
                            "Node gracefully terminated as commanded - expected shutdown"
                        );

                        // Remove from terminating set since it's now confirmed dead
                        {
                            let mut terminating = terminating_nodes.write().await;
                            terminating.remove(&node_id);
                        }

                        counter!("orchestrator_nodes_gracefully_terminated_total", 1);
                    } else {
                        error!(
                            node_id = %node_id,
                            public_ip = %node_ip,
                            time_since_heartbeat = %time_since_heartbeat,
                            "CRITICAL: UNEXPECTED NODE FAILURE DETECTED - will trigger self-healing"
                        );

                        unexpected_failures.push((node_id, node_ip));
                        counter!("orchestrator_unexpected_node_failures_total", 1);
                    }

                    node.status = NodeStatus::Dead;
                    dead_nodes.push(node_id);

                    counter!("orchestrator_nodes_marked_dead_total", 1);
                }
            } else {
                warn!("Invalid heartbeat timeout duration conversion");
            }
        }

        // PHASE 7: Execute self-healing for unexpected failures
        let unexpected_failure_count = unexpected_failures.len();
        for (failed_node_id, failed_node_ip) in unexpected_failures {
            info!(
                failed_node = %failed_node_id,
                failed_ip = %failed_node_ip,
                "Initiating self-healing for unexpected node failure"
            );

            // Create self-healing webhook payload
            let payload = serde_json::json!({
                "reason": "UNEXPECTED_NODE_FAILURE",
                "timestamp": Utc::now(),
                "failed_node_id": failed_node_id,
                "failed_node_ip": failed_node_ip
            });

            // Send self-healing webhook
            match http_client
                .post(&state.config.provisioning_webhook_url)
                .json(&payload)
                .send()
                .await
            {
                Ok(response) => {
                    if response.status().is_success() {
                        info!(
                            failed_node = %failed_node_id,
                            status = %response.status(),
                            "Self-healing webhook sent successfully"
                        );
                        counter!("orchestrator_self_healing_webhooks_sent_total", 1);
                    } else {
                        let status = response.status();
                        let body = response
                            .text()
                            .await
                            .unwrap_or_else(|_| "unknown".to_string());
                        error!(
                            failed_node = %failed_node_id,
                            status = %status,
                            body = %body,
                            "Self-healing webhook failed"
                        );
                        counter!("orchestrator_self_healing_webhook_errors_total", 1);
                    }
                }
                Err(e) => {
                    error!(
                        failed_node = %failed_node_id,
                        error = %e,
                        "Failed to send self-healing webhook"
                    );
                    counter!("orchestrator_self_healing_webhook_errors_total", 1);
                }
            }
        }

        // Update fleet metrics
        let active_count = state
            .nodes
            .iter()
            .filter(|entry| entry.value().status == NodeStatus::Active)
            .count();

        gauge!("orchestrator_active_nodes", active_count as f64);
        gauge!("orchestrator_total_nodes", state.nodes.len() as f64);

        if !dead_nodes.is_empty() {
            warn!(
                dead_nodes = dead_nodes.len(),
                unexpected_failures = unexpected_failure_count,
                "Detected dead nodes in fleet"
            );
        }
    }
}

/// Initialize metrics descriptions
fn initialize_metrics() {
    describe_counter!(
        "orchestrator_nodes_registered_total",
        "Total number of nodes registered"
    );
    describe_counter!(
        "orchestrator_heartbeats_received_total",
        "Total number of heartbeats received"
    );
    describe_counter!(
        "orchestrator_heartbeats_unknown_node",
        "Total number of heartbeats from unknown nodes"
    );
    describe_counter!(
        "orchestrator_nodes_terminated_total",
        "Total number of nodes terminated"
    );
    describe_counter!(
        "orchestrator_nodes_marked_dead_total",
        "Total number of nodes marked as dead"
    );
    describe_counter!(
        "orchestrator_nodes_gracefully_terminated_total",
        "Total number of nodes gracefully terminated as commanded"
    );
    describe_counter!(
        "orchestrator_unexpected_node_failures_total",
        "Total number of unexpected node failures detected"
    );
    describe_counter!(
        "orchestrator_self_healing_webhooks_sent_total",
        "Total number of self-healing webhooks sent successfully"
    );
    describe_counter!(
        "orchestrator_self_healing_webhook_errors_total",
        "Total number of self-healing webhook errors"
    );
    describe_counter!(
        "orchestrator_nodes_marked_dead_total",
        "Total number of nodes marked as dead"
    );
    describe_gauge!(
        "orchestrator_total_nodes",
        "Current total number of nodes in registry"
    );
    describe_gauge!(
        "orchestrator_active_nodes",
        "Current number of active nodes"
    );
}

/// Start Prometheus metrics server
async fn start_metrics_server(addr: SocketAddr, state: OrchestratorState) -> Result<()> {
    use metrics_exporter_prometheus::PrometheusBuilder;

    info!(metrics_addr = %addr, "Starting Prometheus metrics server");

    let builder = PrometheusBuilder::new();
    builder
        .with_http_listener(addr)
        .install()
        .context("Failed to install Prometheus exporter")?;

    info!(metrics_addr = %addr, "Prometheus metrics server started");

    // Periodically update gauge metrics
    let mut interval = time::interval(Duration::from_secs(5));
    loop {
        interval.tick().await;

        // Update current metrics
        let total_nodes = state.nodes.len();
        let active_nodes = state
            .nodes
            .iter()
            .filter(|entry| entry.value().status == NodeStatus::Active)
            .count();

        gauge!("orchestrator_total_nodes", total_nodes as f64);
        gauge!("orchestrator_active_nodes", active_nodes as f64);
    }
}

/// Manual IP blocking endpoint for threat intelligence
#[instrument(skip(state))]
async fn block_ip_endpoint(
    State(state): State<OrchestratorState>,
    Json(request): Json<experts::threat_intel::BlockIpRequest>,
) -> Result<Json<experts::threat_intel::BlockIpResponse>, StatusCode> {
    match state.threat_intel.block_ip(request).await {
        Ok(response) => Ok(Json(response)),
        Err(e) => {
            error!(error = %e, "Failed to process IP block request");
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

/// Get blocked IPs endpoint
#[instrument(skip(state))]
async fn get_blocked_ips_endpoint(
    State(state): State<OrchestratorState>,
) -> Json<serde_json::Value> {
    let blocked_ips = state.threat_intel.get_blocked_ips();
    let stats = state.threat_intel.get_blocklist_stats();

    Json(serde_json::json!({
        "stats": stats,
        "blocked_ips": blocked_ips
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_node_status_serialization() {
        let status = NodeStatus::Active;
        let json = serde_json::to_string(&status).unwrap();
        assert_eq!(json, "\"Active\"");

        let deserialized: NodeStatus = serde_json::from_str(&json).unwrap();
        assert_eq!(status, deserialized);
    }

    #[tokio::test]
    async fn test_orchestrator_state() {
        // Use a fake NATS URL for testing - test will pass if it gracefully handles connection failure
        let threat_intel = match ThreatIntelExpert::new("nats://127.0.0.1:4222").await {
            Ok(ti) => ti,
            Err(_) => {
                // Skip this test if NATS is not available
                eprintln!("Skipping test - NATS not available");
                return;
            }
        };
        let state = OrchestratorState {
            nodes: Arc::new(DashMap::new()),
            config: OrchestratorConfig::default(),
            threat_intel: Arc::new(threat_intel),
        };

        let node_id = Uuid::new_v4();
        let node_info = NodeInfo {
            node_id,
            public_ip: "192.168.1.100".parse().unwrap(),
            last_heartbeat: Utc::now(),
            status: NodeStatus::Active,
            metrics: None,
            registered_at: Utc::now(),
            config: NodeConfig {
                node_type: "mitigation_v2".to_string(),
                region: Some("us-west-1".to_string()),
                tags: vec!["test".to_string()],
            },
        };

        state.nodes.insert(node_id, node_info);
        assert_eq!(state.nodes.len(), 1);
        assert!(state.nodes.contains_key(&node_id));
    }
}
