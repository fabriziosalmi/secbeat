use anyhow::{Context, Result};
use async_nats::Client;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::net::IpAddr;
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio_stream::StreamExt;
use tracing::{debug, error, info, warn};
use uuid::Uuid;

// ============================================================================
// Step 1: Data Contract - Shared Message Structures
// ============================================================================

/// Telemetry event published by Mitigation Nodes via NATS
/// Topic: secbeat.telemetry.{node_id}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TelemetryEvent {
    /// Node that sent this event
    pub node_id: Uuid,
    /// Source IP address making the request
    pub source_ip: IpAddr,
    /// Request URI path
    pub request_uri: String,
    /// HTTP status code returned
    pub status_code: u16,
    /// Timestamp of the event (ISO 8601)
    pub timestamp: DateTime<Utc>,
    /// HTTP method (GET, POST, etc.)
    #[serde(default)]
    pub method: Option<String>,
    /// User agent string
    #[serde(default)]
    pub user_agent: Option<String>,
}

/// Block command sent by Orchestrator to Mitigation Nodes via NATS
/// Topic: secbeat.commands.block
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlockCommand {
    /// Unique command identifier for tracking
    pub command_id: Uuid,
    /// IP address to block
    pub ip: IpAddr,
    /// Human-readable reason for blocking
    pub reason: String,
    /// Duration to block in seconds
    pub duration_seconds: u64,
    /// Action type (always "block" for now)
    pub action: String,
    /// Timestamp when command was issued
    pub issued_at: DateTime<Utc>,
    /// Which expert generated this command
    #[serde(default)]
    pub source: String,
}

// ============================================================================
// Step 2: Behavioral Expert Implementation
// ============================================================================

/// Configuration for the behavioral analysis expert
#[derive(Debug, Clone)]
pub struct BehavioralConfig {
    /// Sliding window duration in seconds
    pub window_size_seconds: u64,
    /// Number of errors (4xx/5xx) to trigger block
    pub error_threshold: usize,
    /// Number of total requests to trigger block (rate limiting)
    pub request_threshold: usize,
    /// How long to block an IP in seconds
    pub block_duration_seconds: u64,
    /// How often to prune old events (seconds)
    pub cleanup_interval_seconds: u64,
}

impl Default for BehavioralConfig {
    fn default() -> Self {
        Self {
            window_size_seconds: 60,        // 1 minute sliding window
            error_threshold: 50,             // 50 errors in 60s
            request_threshold: 1000,         // 1000 requests in 60s
            block_duration_seconds: 300,     // Block for 5 minutes
            cleanup_interval_seconds: 10,    // Cleanup every 10s
        }
    }
}

/// IP tracking data for behavioral analysis
#[derive(Debug, Clone)]
struct IpBehavior {
    /// All request timestamps (for rate limiting)
    request_timestamps: Vec<i64>,
    /// Error request timestamps (4xx/5xx status codes)
    error_timestamps: Vec<i64>,
    /// When this IP was last seen
    last_seen: DateTime<Utc>,
}

impl IpBehavior {
    fn new() -> Self {
        Self {
            request_timestamps: Vec::new(),
            error_timestamps: Vec::new(),
            last_seen: Utc::now(),
        }
    }

    /// Add a new event to this IP's behavior tracking
    fn add_event(&mut self, timestamp: i64, is_error: bool) {
        self.request_timestamps.push(timestamp);
        if is_error {
            self.error_timestamps.push(timestamp);
        }
        self.last_seen = Utc::now();
    }

    /// Prune timestamps older than the window
    fn prune(&mut self, cutoff_timestamp: i64) {
        self.request_timestamps.retain(|&ts| ts >= cutoff_timestamp);
        self.error_timestamps.retain(|&ts| ts >= cutoff_timestamp);
    }

    /// Get count of requests in the current window
    fn request_count(&self) -> usize {
        self.request_timestamps.len()
    }

    /// Get count of errors in the current window
    fn error_count(&self) -> usize {
        self.error_timestamps.len()
    }

    /// Check if this IP should be pruned (no activity and no data)
    fn is_empty(&self) -> bool {
        self.request_timestamps.is_empty() && self.error_timestamps.is_empty()
    }
}

/// Real-time behavioral analysis expert
/// Analyzes traffic patterns using sliding window algorithm
/// Detects error rate anomalies and high-frequency request spikes
pub struct BehavioralExpert {
    /// NATS client for messaging (None in test mode)
    nats_client: Option<Client>,
    /// Configuration parameters
    config: BehavioralConfig,
    /// IP behavior tracking (IP -> behavior data)
    ip_behaviors: Arc<RwLock<HashMap<IpAddr, IpBehavior>>>,
    /// Recently blocked IPs (to avoid duplicate blocks)
    recently_blocked: Arc<RwLock<HashMap<IpAddr, DateTime<Utc>>>>,
}

impl BehavioralExpert {
    /// Create a new behavioral analysis expert
    pub async fn new(nats_url: &str, config: BehavioralConfig) -> Result<Self> {
        info!(
            nats_url = %nats_url,
            window_size = config.window_size_seconds,
            error_threshold = config.error_threshold,
            request_threshold = config.request_threshold,
            "Initializing Behavioral Analysis Expert"
        );

        let nats_client = async_nats::connect(nats_url)
            .await
            .context("Failed to connect to NATS server for behavioral analysis")?;

        info!("Successfully connected to NATS server for behavioral analysis");

        Ok(Self {
            nats_client: Some(nats_client),
            config,
            ip_behaviors: Arc::new(RwLock::new(HashMap::new())),
            recently_blocked: Arc::new(RwLock::new(HashMap::new())),
        })
    }

    /// Create a new behavioral analysis expert for testing (without NATS)
    #[cfg(test)]
    fn new_test(config: BehavioralConfig) -> Self {
        Self {
            nats_client: None,
            config,
            ip_behaviors: Arc::new(RwLock::new(HashMap::new())),
            recently_blocked: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Start the telemetry consumer that processes events from mitigation nodes
    #[tracing::instrument(skip(self))]
    pub async fn start_telemetry_consumer(self: Arc<Self>) -> Result<()> {
        info!("Starting behavioral analysis telemetry consumer");

        let nats_client = self.nats_client.as_ref()
            .context("NATS client not initialized")?;

        // Subscribe to all telemetry topics (wildcard for all nodes)
        let mut subscriber = nats_client
            .subscribe("secbeat.telemetry.>")
            .await
            .context("Failed to subscribe to telemetry topic")?;

        info!("Successfully subscribed to secbeat.telemetry.> topic");

        // Process events in a loop
        while let Some(message) = subscriber.next().await {
            match serde_json::from_slice::<TelemetryEvent>(&message.payload) {
                Ok(event) => {
                    debug!(
                        node_id = %event.node_id,
                        source_ip = %event.source_ip,
                        status = event.status_code,
                        uri = %event.request_uri,
                        "Received telemetry event"
                    );

                    // Analyze the event and potentially generate block command
                    if let Some(block_cmd) = self.process_event(event).await {
                        info!(
                            command_id = %block_cmd.command_id,
                            ip = %block_cmd.ip,
                            reason = %block_cmd.reason,
                            "Generated block command - publishing to fleet"
                        );

                        // Publish block command to the fleet
                        match serde_json::to_vec(&block_cmd) {
                            Ok(payload) => {
                                if let Err(e) = nats_client
                                    .publish("secbeat.commands.block", payload.into())
                                    .await
                                {
                                    error!(error = %e, "Failed to publish block command");
                                }
                            }
                            Err(e) => {
                                error!(error = %e, "Failed to serialize block command");
                            }
                        }
                    }
                }
                Err(e) => {
                    warn!(error = %e, "Failed to deserialize telemetry event");
                }
            }
        }

        warn!("Behavioral analysis telemetry consumer loop ended");
        Ok(())
    }

    /// Process a telemetry event and potentially generate a block command
    /// Returns Some(BlockCommand) if a threat is detected, None otherwise
    #[tracing::instrument(skip(self), fields(ip = %event.source_ip, status = event.status_code))]
    pub async fn process_event(&self, event: TelemetryEvent) -> Option<BlockCommand> {
        let timestamp = event.timestamp.timestamp();
        let is_error = event.status_code >= 400;
        let ip = event.source_ip;

        // Check if this IP was recently blocked
        {
            let blocked = self.recently_blocked.read().await;
            if let Some(blocked_at) = blocked.get(&ip) {
                let elapsed = Utc::now().signed_duration_since(*blocked_at);
                if elapsed.num_seconds() < self.config.block_duration_seconds as i64 {
                    debug!(ip = %ip, "IP already blocked, skipping analysis");
                    return None;
                }
            }
        }

        // Update behavior tracking
        let (request_count, error_count) = {
            let mut behaviors = self.ip_behaviors.write().await;
            let behavior = behaviors.entry(ip).or_insert_with(IpBehavior::new);

            // Add this event
            behavior.add_event(timestamp, is_error);

            // Prune old events (sliding window)
            let cutoff = timestamp - self.config.window_size_seconds as i64;
            behavior.prune(cutoff);

            (behavior.request_count(), behavior.error_count())
        };

        debug!(
            ip = %ip,
            requests = request_count,
            errors = error_count,
            "Updated behavior tracking"
        );

        // Analyze and decide if we should block
        let should_block = error_count >= self.config.error_threshold
            || request_count >= self.config.request_threshold;

        if should_block {
            let reason = if error_count >= self.config.error_threshold {
                format!(
                    "Error rate anomaly: {} errors in {}s (threshold: {})",
                    error_count, self.config.window_size_seconds, self.config.error_threshold
                )
            } else {
                format!(
                    "High-frequency spike: {} requests in {}s (threshold: {})",
                    request_count, self.config.window_size_seconds, self.config.request_threshold
                )
            };

            warn!(
                ip = %ip,
                errors = error_count,
                requests = request_count,
                reason = %reason,
                "Threat detected - issuing block command"
            );

            // Mark as recently blocked
            {
                let mut blocked = self.recently_blocked.write().await;
                blocked.insert(ip, Utc::now());
            }

            Some(BlockCommand {
                command_id: Uuid::new_v4(),
                ip,
                reason,
                duration_seconds: self.config.block_duration_seconds,
                action: "block".to_string(),
                issued_at: Utc::now(),
                source: "behavioral_expert".to_string(),
            })
        } else {
            None
        }
    }

    /// Cleanup old data from memory (called periodically)
    /// Removes IPs with no recent activity and expired blocks
    pub async fn cleanup(&self) {
        let cutoff_timestamp = Utc::now().timestamp() - self.config.window_size_seconds as i64;

        // Clean up IP behaviors - first prune old events, then remove empty IPs
        let mut removed_ips = 0;
        {
            let mut behaviors = self.ip_behaviors.write().await;
            
            // First prune all timestamps
            for behavior in behaviors.values_mut() {
                behavior.prune(cutoff_timestamp);
            }
            
            // Then remove empty IP entries
            behaviors.retain(|ip, behavior| {
                if behavior.is_empty() {
                    debug!(ip = %ip, "Removing inactive IP from tracking");
                    removed_ips += 1;
                    false
                } else {
                    true
                }
            });
        }

        // Clean up expired blocks
        let mut removed_blocks = 0;
        {
            let mut blocked = self.recently_blocked.write().await;
            blocked.retain(|ip, blocked_at| {
                let elapsed = Utc::now().signed_duration_since(*blocked_at);
                if elapsed.num_seconds() >= self.config.block_duration_seconds as i64 {
                    debug!(ip = %ip, "Removing expired block");
                    removed_blocks += 1;
                    false
                } else {
                    true
                }
            });
        }

        if removed_ips > 0 || removed_blocks > 0 {
            info!(
                removed_ips = removed_ips,
                removed_blocks = removed_blocks,
                "Cleanup completed"
            );
        }
    }

    /// Get current statistics for monitoring
    #[allow(dead_code)]
    pub async fn get_stats(&self) -> BehavioralStats {
        let behaviors = self.ip_behaviors.read().await;
        let blocked = self.recently_blocked.read().await;

        BehavioralStats {
            tracked_ips: behaviors.len(),
            blocked_ips: blocked.len(),
            total_events_in_window: behaviors
                .values()
                .map(|b| b.request_count())
                .sum(),
            total_errors_in_window: behaviors
                .values()
                .map(|b| b.error_count())
                .sum(),
        }
    }

    /// Start the cleanup task (runs periodically in the background)
    pub fn spawn_cleanup_task(self: Arc<Self>) {
        let interval_secs = self.config.cleanup_interval_seconds;
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(
                std::time::Duration::from_secs(interval_secs)
            );

            loop {
                interval.tick().await;
                self.cleanup().await;
            }
        });
    }
}

/// Statistics snapshot from the behavioral expert
#[allow(dead_code)]
#[derive(Debug, Clone, Serialize)]
pub struct BehavioralStats {
    /// Number of IPs currently being tracked
    pub tracked_ips: usize,
    /// Number of currently blocked IPs
    pub blocked_ips: usize,
    /// Total events in current sliding windows
    pub total_events_in_window: usize,
    /// Total errors in current sliding windows
    pub total_errors_in_window: usize,
}

// ============================================================================
// Unit Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    /// Test: Simulate a flood of 404 errors from a single IP
    /// Expected: BlockCommand should be generated after ERROR_THRESHOLD is exceeded
    #[tokio::test]
    async fn test_error_flood_triggers_block() {
        // Create expert with low thresholds for testing
        let config = BehavioralConfig {
            window_size_seconds: 60,
            error_threshold: 10,  // Low threshold for testing
            request_threshold: 100,
            block_duration_seconds: 300,
            cleanup_interval_seconds: 10,
        };

        let expert = BehavioralExpert::new_test(config);
        let attacker_ip: IpAddr = "192.0.2.100".parse().unwrap();
        let node_id = Uuid::new_v4();

        // Send 15 requests with 404 errors (exceeds threshold of 10)
        for i in 0..15 {
            let event = TelemetryEvent {
                node_id,
                source_ip: attacker_ip,
                request_uri: format!("/nonexistent/{}", i),
                status_code: 404,
                timestamp: Utc::now(),
                method: Some("GET".to_string()),
                user_agent: Some("AttackBot/1.0".to_string()),
            };

            let result = expert.process_event(event).await;

            if i < 9 {
                // First 9 should not trigger block (threshold is 10)
                assert!(result.is_none(), "Should not block before threshold at event {}", i);
            } else {
                // 10th error should trigger block (index 9, count 10)
                if i == 9 {
                    assert!(result.is_some(), "Should block after reaching threshold");
                    let block_cmd = result.unwrap();
                    assert_eq!(block_cmd.ip, attacker_ip);
                    assert_eq!(block_cmd.action, "block");
                    assert!(block_cmd.reason.contains("Error rate anomaly"));
                    println!("✅ Block command generated: {:?}", block_cmd);
                } else {
                    // Subsequent requests should not generate duplicate blocks
                    assert!(result.is_none(), "Should not generate duplicate blocks");
                }
            }
        }
    }

    /// Test: Simulate high-frequency request spike
    /// Expected: BlockCommand should be generated after REQUEST_THRESHOLD is exceeded
    #[tokio::test]
    async fn test_request_flood_triggers_block() {
        let config = BehavioralConfig {
            window_size_seconds: 60,
            error_threshold: 50,
            request_threshold: 20,  // Low threshold for testing
            block_duration_seconds: 300,
            cleanup_interval_seconds: 10,
        };

        let expert = BehavioralExpert::new_test(config);
        let attacker_ip: IpAddr = "203.0.113.50".parse().unwrap();
        let node_id = Uuid::new_v4();

        // Send 25 successful requests (exceeds threshold of 20)
        for i in 0..25 {
            let event = TelemetryEvent {
                node_id,
                source_ip: attacker_ip,
                request_uri: "/api/data".to_string(),
                status_code: 200,  // Success, not an error
                timestamp: Utc::now(),
                method: Some("GET".to_string()),
                user_agent: Some("SpeedBot/2.0".to_string()),
            };

            let result = expert.process_event(event).await;

            if i < 19 {
                assert!(result.is_none(), "Should not block before threshold at event {}", i);
            } else {
                if i == 19 {
                    assert!(result.is_some(), "Should block after reaching request threshold");
                    let block_cmd = result.unwrap();
                    assert_eq!(block_cmd.ip, attacker_ip);
                    assert!(block_cmd.reason.contains("High-frequency spike"));
                    println!("✅ Block command generated: {:?}", block_cmd);
                } else {
                    assert!(result.is_none(), "Should not generate duplicate blocks");
                }
            }
        }
    }

    /// Test: Verify sliding window prunes old events
    #[tokio::test]
    async fn test_sliding_window_pruning() {
        let config = BehavioralConfig {
            window_size_seconds: 2,  // Very short window for testing
            error_threshold: 5,
            request_threshold: 100,
            block_duration_seconds: 300,
            cleanup_interval_seconds: 10,
        };

        let expert = BehavioralExpert::new_test(config);
        let ip: IpAddr = "198.51.100.1".parse().unwrap();
        let node_id = Uuid::new_v4();

        // Send 4 errors
        for i in 0..4 {
            let event = TelemetryEvent {
                node_id,
                source_ip: ip,
                request_uri: "/test".to_string(),
                status_code: 500,
                timestamp: Utc::now(),
                method: Some("GET".to_string()),
                user_agent: None,
            };
            expert.process_event(event).await;
        }

        // Wait for window to expire
        tokio::time::sleep(tokio::time::Duration::from_secs(3)).await;

        // Send 1 more error - should not trigger block because old events are pruned
        let event = TelemetryEvent {
            node_id,
            source_ip: ip,
            request_uri: "/test".to_string(),
            status_code: 500,
            timestamp: Utc::now(),
            method: Some("GET".to_string()),
            user_agent: None,
        };

        let result = expert.process_event(event).await;
        assert!(result.is_none(), "Should not block after window expired");
        println!("✅ Sliding window correctly pruned old events");
    }

    /// Test: Cleanup removes inactive IPs
    #[tokio::test]
    async fn test_cleanup_removes_inactive_ips() {
        let config = BehavioralConfig {
            window_size_seconds: 5,  // Short window
            error_threshold: 10,
            request_threshold: 100,
            block_duration_seconds: 300,
            cleanup_interval_seconds: 10,
        };
        
        let expert = BehavioralExpert::new_test(config);
        let ip: IpAddr = "192.0.2.200".parse().unwrap();
        let node_id = Uuid::new_v4();

        // Add events with old timestamps (outside the window)
        for _ in 0..3 {
            let event = TelemetryEvent {
                node_id,
                source_ip: ip,
                request_uri: "/test".to_string(),
                status_code: 200,
                timestamp: Utc::now() - chrono::Duration::seconds(10), // Outside 5s window
                method: Some("GET".to_string()),
                user_agent: None,
            };
            expert.process_event(event).await;
        }

        let stats_before = expert.get_stats().await;
        assert_eq!(stats_before.tracked_ips, 1, "Should have 1 tracked IP");

        // Events should already be pruned in the sliding window during process_event
        // but IP entry still exists. Cleanup will remove empty IP entries
        expert.cleanup().await;

        let stats_after = expert.get_stats().await;
        assert_eq!(stats_after.tracked_ips, 0, "Should have 0 tracked IPs after cleanup");
        println!("✅ Cleanup correctly removed inactive IP");
    }
}
