use anyhow::{Context, Result};
use async_nats::{Client, ConnectOptions};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::net::IpAddr;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use tokio_stream::StreamExt;
use tracing::{debug, error, info, instrument, warn};
use uuid::Uuid;

/// WAF analysis result for event reporting
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WafEventResult {
    /// Action taken (LOG, BLOCK, etc.)
    pub action: String,
    /// Rules that matched (if any)
    pub matched_rules: Vec<String>,
    /// Confidence score (0.0 to 1.0)
    pub confidence: Option<f64>,
}

/// Security event published to NATS
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityEvent {
    /// Node ID that generated this event
    pub node_id: Uuid,
    /// Event timestamp (ISO 8601)
    pub timestamp: DateTime<Utc>,
    /// Source IP address
    pub source_ip: IpAddr,
    /// HTTP method
    pub http_method: String,
    /// Request URI
    pub uri: String,
    /// Host header value
    pub host_header: Option<String>,
    /// User agent header
    pub user_agent: Option<String>,
    /// WAF analysis result
    pub waf_result: WafEventResult,
    /// Request size in bytes
    pub request_size: Option<usize>,
    /// Response status code
    pub response_status: Option<u16>,
    /// Processing time in milliseconds
    pub processing_time_ms: Option<u64>,
}

/// Control command received from orchestrator
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ControlCommand {
    /// Unique command identifier
    pub command_id: Uuid,
    /// Action to perform
    pub action: String,
    /// Rule type for the action
    pub rule_type: String,
    /// Target of the rule (IP address, etc.)
    pub target: String,
    /// Time-to-live for the rule in seconds
    pub ttl_seconds: Option<u64>,
    /// Additional parameters
    pub parameters: Option<serde_json::Value>,
}

/// Dynamic rule state management
#[derive(Debug, Clone)]
pub struct DynamicRuleState {
    /// Blocked IP addresses with expiration times
    pub blocked_ips: Arc<RwLock<HashSet<IpAddr>>>,
    /// Rule metadata for tracking
    pub rule_metadata: Arc<RwLock<std::collections::HashMap<Uuid, ControlCommand>>>,
}

impl DynamicRuleState {
    pub fn new() -> Self {
        Self {
            blocked_ips: Arc::new(RwLock::new(HashSet::new())),
            rule_metadata: Arc::new(RwLock::new(std::collections::HashMap::new())),
        }
    }

    /// Check if an IP is blocked
    pub async fn is_ip_blocked(&self, ip: &IpAddr) -> bool {
        self.blocked_ips.read().await.contains(ip)
    }

    /// Add an IP to the blocklist
    pub async fn add_blocked_ip(&self, ip: IpAddr, command: ControlCommand) {
        {
            let mut blocked = self.blocked_ips.write().await;
            blocked.insert(ip);
        }
        
        {
            let mut metadata = self.rule_metadata.write().await;
            metadata.insert(command.command_id, command);
        }
        
        info!(ip = %ip, "Added IP to dynamic blocklist");
    }

    /// Remove an IP from the blocklist
    pub async fn remove_blocked_ip(&self, ip: &IpAddr, command_id: Uuid) {
        {
            let mut blocked = self.blocked_ips.write().await;
            blocked.remove(ip);
        }
        
        {
            let mut metadata = self.rule_metadata.write().await;
            metadata.remove(&command_id);
        }
        
        info!(ip = %ip, "Removed IP from dynamic blocklist");
    }

    /// Get current blocklist size
    pub async fn get_blocked_count(&self) -> usize {
        self.blocked_ips.read().await.len()
    }
}

/// NATS event publisher and command consumer
#[derive(Debug, Clone)]
pub struct EventSystem {
    /// NATS client
    client: Client,
    /// Node ID for event attribution
    pub node_id: Uuid,
    /// Dynamic rule state
    rule_state: DynamicRuleState,
}

impl EventSystem {
    /// Create new event system
    pub async fn new(nats_url: &str, node_id: Uuid) -> Result<Self> {
        info!(nats_url = %nats_url, node_id = %node_id, "Connecting to NATS server");

        let options = ConnectOptions::new()
            .retry_on_initial_connect()
            .reconnect_delay_callback(|attempts| {
                if attempts < 10 {
                    Duration::from_millis(200 * attempts as u64)
                } else {
                    Duration::from_secs(10)
                }
            });

        let client = async_nats::connect_with_options(nats_url, options)
            .await
            .context("Failed to connect to NATS server")?;

        info!("Successfully connected to NATS server");

        Ok(Self {
            client,
            node_id,
            rule_state: DynamicRuleState::new(),
        })
    }

    /// Publish a security event
    #[instrument(skip(self, event))]
    pub async fn publish_security_event(&self, event: SecurityEvent) -> Result<()> {
        let payload = serde_json::to_vec(&event)
            .context("Failed to serialize security event")?;

        self.client
            .publish("secbeat.events.waf", payload.into())
            .await
            .context("Failed to publish security event")?;

        debug!(
            node_id = %event.node_id,
            source_ip = %event.source_ip,
            method = %event.http_method,
            uri = %event.uri,
            "Published security event"
        );

        Ok(())
    }

    /// Start consuming control commands
    pub async fn start_command_consumer(self: Arc<Self>) -> Result<()> {
        let client = self.client.clone();
        let rule_state = self.rule_state.clone();

        let mut subscriber = client
            .subscribe("secbeat.control.commands")
            .await
            .context("Failed to subscribe to control commands")?;

        info!("Started consuming control commands from secbeat.control.commands");

        tokio::spawn(async move {
            while let Some(message) = subscriber.next().await {
                match serde_json::from_slice::<ControlCommand>(&message.payload) {
                    Ok(command) => {
                        if let Err(e) = Self::process_control_command(&rule_state, command).await {
                            error!(error = %e, "Failed to process control command");
                        }
                    }
                    Err(e) => {
                        warn!(error = %e, "Failed to parse control command");
                    }
                }
            }
        });

        Ok(())
    }

    /// Process a control command
    async fn process_control_command(
        rule_state: &DynamicRuleState,
        command: ControlCommand,
    ) -> Result<()> {
        info!(
            command_id = %command.command_id,
            action = %command.action,
            rule_type = %command.rule_type,
            target = %command.target,
            "Processing control command"
        );

        match command.action.as_str() {
            "ADD_DYNAMIC_RULE" => {
                match command.rule_type.as_str() {
                    "IP_BLOCK" => {
                        let ip: IpAddr = command.target.parse()
                            .context("Invalid IP address in command")?;
                        
                        rule_state.add_blocked_ip(ip, command.clone()).await;
                        
                        // Schedule removal if TTL is specified
                        if let Some(ttl) = command.ttl_seconds {
                            let rule_state = rule_state.clone();
                            let command_id = command.command_id;
                            tokio::spawn(async move {
                                tokio::time::sleep(Duration::from_secs(ttl)).await;
                                rule_state.remove_blocked_ip(&ip, command_id).await;
                            });
                        }
                    }
                    _ => {
                        warn!(rule_type = %command.rule_type, "Unknown rule type");
                    }
                }
            }
            "REMOVE_DYNAMIC_RULE" => {
                match command.rule_type.as_str() {
                    "IP_BLOCK" => {
                        let ip: IpAddr = command.target.parse()
                            .context("Invalid IP address in command")?;
                        
                        rule_state.remove_blocked_ip(&ip, command.command_id).await;
                    }
                    _ => {
                        warn!(rule_type = %command.rule_type, "Unknown rule type");
                    }
                }
            }
            _ => {
                warn!(action = %command.action, "Unknown command action");
            }
        }

        Ok(())
    }

    /// Get dynamic rule state
    pub fn get_rule_state(&self) -> &DynamicRuleState {
        &self.rule_state
    }

    /// Get current blocked IP count
    pub async fn get_blocked_ip_count(&self) -> usize {
        self.rule_state.get_blocked_count().await
    }

    /// Check if an IP address is in the dynamic blocklist
    pub async fn is_ip_blocked(&self, ip: IpAddr) -> bool {
        let blocked_ips = self.rule_state.blocked_ips.read().await;
        blocked_ips.contains(&ip)
    }
}
