use anyhow::{Context, Result};
use async_nats::Client;
use chrono::{DateTime, Utc};
use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use std::net::IpAddr;
use std::sync::Arc;
use tokio_stream::StreamExt;
use tracing::{debug, error, info, instrument, warn};
use uuid::Uuid;

/// Information about a blocked IP
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlockInfo {
    /// Reason for blocking
    pub reason: String,
    /// When the IP was blocked
    pub blocked_at: DateTime<Utc>,
    /// Who/what initiated the block
    pub blocked_by: String,
    /// TTL for the block (if any)
    pub ttl_seconds: Option<u64>,
    /// Additional metadata
    pub metadata: Option<serde_json::Value>,
}

/// Security event from NATS stream
#[derive(Debug, Clone, Deserialize)]
#[allow(dead_code)]
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

/// WAF analysis result
#[derive(Debug, Clone, Deserialize)]
#[allow(dead_code)]
pub struct WafEventResult {
    /// Action taken (LOG, BLOCK, etc.)
    pub action: String,
    /// Rules that matched (if any)
    pub matched_rules: Vec<String>,
    /// Confidence score (0.0 to 1.0)
    pub confidence: Option<f64>,
}

/// Control command to send to nodes
#[derive(Debug, Clone, Serialize)]
pub struct ControlCommand {
    /// Unique command ID
    pub command_id: Uuid,
    /// Action to take
    pub action: String,
    /// Rule type
    pub rule_type: String,
    /// Target (e.g., IP address)
    pub target: String,
    /// TTL in seconds
    pub ttl_seconds: u64,
    /// Command timestamp
    pub timestamp: DateTime<Utc>,
    /// Metadata
    pub metadata: Option<serde_json::Value>,
}

/// Request to manually block an IP
#[derive(Debug, Deserialize)]
pub struct BlockIpRequest {
    /// IP address to block
    pub ip: IpAddr,
    /// Reason for blocking
    pub reason: String,
    /// TTL in seconds (optional, defaults to 3600)
    pub ttl_seconds: Option<u64>,
    /// Additional metadata
    pub metadata: Option<serde_json::Value>,
}

/// Response for block IP request
#[derive(Debug, Serialize)]
pub struct BlockIpResponse {
    /// Success status
    pub success: bool,
    /// Message
    pub message: String,
    /// Command ID generated
    pub command_id: Option<Uuid>,
}

/// Threat Intelligence Expert for analyzing security events and managing IP blocklists
#[derive(Debug)]
pub struct ThreatIntelExpert {
    /// NATS client for messaging
    nats_client: Client,
    /// In-memory IP blocklist
    blocked_ips: Arc<DashMap<IpAddr, BlockInfo>>,
    /// Expert configuration
    config: ThreatIntelConfig,
}

/// Configuration for the threat intelligence expert
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct ThreatIntelConfig {
    /// Default TTL for manual blocks (seconds)
    pub default_ttl_seconds: u64,
    /// Whether to auto-publish commands for detected threats
    pub auto_publish_commands: bool,
    /// Minimum confidence threshold for auto-actions
    pub min_confidence_threshold: f64,
}

impl Default for ThreatIntelConfig {
    fn default() -> Self {
        Self {
            default_ttl_seconds: 3600, // 1 hour
            auto_publish_commands: true,
            min_confidence_threshold: 0.8,
        }
    }
}

impl ThreatIntelExpert {
    /// Create a new threat intelligence expert
    pub async fn new(nats_url: &str) -> Result<Self> {
        info!(nats_url = %nats_url, "Initializing Threat Intelligence Expert");
        
        let nats_client = async_nats::connect(nats_url)
            .await
            .context("Failed to connect to NATS server")?;
            
        info!("Successfully connected to NATS server for threat intelligence");
        
        Ok(Self {
            nats_client,
            blocked_ips: Arc::new(DashMap::new()),
            config: ThreatIntelConfig::default(),
        })
    }
    
    /// Start the event consumer that processes security events from the fleet
    #[instrument(skip(self))]
    pub async fn start_event_consumer(self: Arc<Self>) -> Result<()> {
        info!("Starting security event consumer");
        
        let mut subscriber = self.nats_client
            .subscribe("secbeat.events.waf")
            .await
            .context("Failed to subscribe to security events topic")?;
            
        info!("Successfully subscribed to secbeat.events.waf topic");
        
        // Process events in a loop
        while let Some(message) = subscriber.next().await {
            match serde_json::from_slice::<SecurityEvent>(&message.payload) {
                Ok(event) => {
                    debug!(
                        node_id = %event.node_id,
                        source_ip = %event.source_ip,
                        method = %event.http_method,
                        uri = %event.uri,
                        "Received security event from fleet"
                    );
                    
                    // Analyze the event
                    if let Err(e) = self.analyze_security_event(&event).await {
                        error!(error = %e, "Failed to analyze security event");
                    }
                }
                Err(e) => {
                    warn!(error = %e, "Failed to deserialize security event");
                }
            }
        }
        
        warn!("Security event consumer loop ended");
        Ok(())
    }
    
    /// Analyze a security event and take action if needed
    #[instrument(skip(self, event))]
    async fn analyze_security_event(&self, event: &SecurityEvent) -> Result<()> {
        // Check if the source IP is in our blocklist
        if let Some(block_info) = self.blocked_ips.get(&event.source_ip) {
            // High-threat event detected!
            warn!(
                source_ip = %event.source_ip,
                node_id = %event.node_id,
                reason = %block_info.reason,
                blocked_at = %block_info.blocked_at,
                "High-threat event detected! Blocked IP attempted access"
            );
            
            // Publish a control command to ensure all nodes have this IP blocked
            let command = ControlCommand {
                command_id: Uuid::new_v4(),
                action: "ADD_DYNAMIC_RULE".to_string(),
                rule_type: "IP_BLOCK".to_string(),
                target: event.source_ip.to_string(),
                ttl_seconds: block_info.ttl_seconds.unwrap_or(self.config.default_ttl_seconds),
                timestamp: Utc::now(),
                metadata: Some(serde_json::json!({
                    "triggered_by_event": {
                        "node_id": event.node_id,
                        "uri": event.uri,
                        "method": event.http_method
                    },
                    "original_block_reason": block_info.reason
                })),
            };
            
            self.publish_control_command(&command).await?;
        }
        
        // Check for suspicious patterns in the event itself
        if event.waf_result.action == "BLOCK" {
            info!(
                source_ip = %event.source_ip,
                node_id = %event.node_id,
                matched_rules = ?event.waf_result.matched_rules,
                "WAF blocked suspicious request"
            );
            
            // Could implement additional logic here for auto-blocking repeat offenders
        }
        
        Ok(())
    }
    
    /// Manually add an IP to the blocklist
    #[instrument(skip(self))]
    pub async fn block_ip(&self, request: BlockIpRequest) -> Result<BlockIpResponse> {
        info!(
            ip = %request.ip,
            reason = %request.reason,
            ttl_seconds = ?request.ttl_seconds,
            "Manually blocking IP address"
        );
        
        let ttl_seconds = request.ttl_seconds.unwrap_or(self.config.default_ttl_seconds);
        
        let block_info = BlockInfo {
            reason: request.reason.clone(),
            blocked_at: Utc::now(),
            blocked_by: "manual_operator".to_string(),
            ttl_seconds: Some(ttl_seconds),
            metadata: request.metadata.clone(),
        };
        
        // Add to our in-memory blocklist
        self.blocked_ips.insert(request.ip, block_info);
        
        // Create and publish control command
        let command = ControlCommand {
            command_id: Uuid::new_v4(),
            action: "ADD_DYNAMIC_RULE".to_string(),
            rule_type: "IP_BLOCK".to_string(),
            target: request.ip.to_string(),
            ttl_seconds,
            timestamp: Utc::now(),
            metadata: Some(serde_json::json!({
                "block_reason": request.reason,
                "blocked_by": "manual_operator",
                "metadata": request.metadata
            })),
        };
        
        match self.publish_control_command(&command).await {
            Ok(()) => {
                info!(
                    ip = %request.ip,
                    command_id = %command.command_id,
                    "Successfully published IP block command to fleet"
                );
                
                Ok(BlockIpResponse {
                    success: true,
                    message: format!("IP {} blocked successfully", request.ip),
                    command_id: Some(command.command_id),
                })
            }
            Err(e) => {
                error!(
                    ip = %request.ip,
                    error = %e,
                    "Failed to publish IP block command"
                );
                
                // Remove from our blocklist since the command failed
                self.blocked_ips.remove(&request.ip);
                
                Ok(BlockIpResponse {
                    success: false,
                    message: format!("Failed to block IP {}: {}", request.ip, e),
                    command_id: None,
                })
            }
        }
    }
    
    /// Publish a control command to the fleet
    #[instrument(skip(self, command))]
    async fn publish_control_command(&self, command: &ControlCommand) -> Result<()> {
        let payload = serde_json::to_vec(command)
            .context("Failed to serialize control command")?;
            
        self.nats_client
            .publish("secbeat.control.commands", payload.into())
            .await
            .context("Failed to publish control command")?;
            
        debug!(
            command_id = %command.command_id,
            action = %command.action,
            target = %command.target,
            "Published control command to fleet"
        );
        
        Ok(())
    }
    
    /// Get current blocklist statistics
    pub fn get_blocklist_stats(&self) -> BlocklistStats {
        let total_blocked = self.blocked_ips.len();
        let manual_blocks = self.blocked_ips
            .iter()
            .filter(|entry| entry.value().blocked_by == "manual_operator")
            .count();
        let auto_blocks = total_blocked - manual_blocks;
        
        BlocklistStats {
            total_blocked_ips: total_blocked,
            manual_blocks,
            auto_blocks,
        }
    }
    
    /// Get list of blocked IPs
    pub fn get_blocked_ips(&self) -> Vec<(IpAddr, BlockInfo)> {
        self.blocked_ips
            .iter()
            .map(|entry| (*entry.key(), entry.value().clone()))
            .collect()
    }
    
    /// Remove an IP from the blocklist
    #[instrument(skip(self))]
    pub async fn unblock_ip(&self, ip: IpAddr) -> Result<()> {
        if self.blocked_ips.remove(&ip).is_some() {
            info!(ip = %ip, "Removed IP from threat intelligence blocklist");
            
            // Publish command to remove from all nodes
            let command = ControlCommand {
                command_id: Uuid::new_v4(),
                action: "REMOVE_DYNAMIC_RULE".to_string(),
                rule_type: "IP_BLOCK".to_string(),
                target: ip.to_string(),
                ttl_seconds: 0,
                timestamp: Utc::now(),
                metadata: Some(serde_json::json!({
                    "unblocked_by": "manual_operator"
                })),
            };
            
            self.publish_control_command(&command).await?;
        }
        
        Ok(())
    }
}

/// Statistics about the blocklist
#[derive(Debug, Serialize)]
pub struct BlocklistStats {
    pub total_blocked_ips: usize,
    pub manual_blocks: usize,
    pub auto_blocks: usize,
}
