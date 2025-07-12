use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use dashmap::DashMap;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::collections::{HashSet, VecDeque};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use tokio::time;
use tracing::{debug, error, info, instrument, warn};
use uuid::Uuid;

// Machine Learning imports for predictive scaling
use linfa::prelude::*;
use linfa_linear::LinearRegression;
use ndarray::{Array1, Array2};

use crate::{NodeInfo, NodeStatus, OrchestratorConfig};

/// Time-series data point for CPU usage history
#[derive(Debug, Clone)]
pub struct CpuDataPoint {
    pub timestamp: Instant,
    pub cpu_usage: f32,
}

/// Enhanced resource management expert for intelligent scaling with ML prediction
#[derive(Debug)]
pub struct ResourceManager {
    /// Reference to the node registry
    node_registry: Arc<DashMap<Uuid, NodeInfo>>,
    /// Configuration
    config: OrchestratorConfig,
    /// HTTP client for webhook calls
    http_client: Client,
    /// Consecutive scale-up checks that met threshold
    scale_up_checks: u32,
    /// Consecutive scale-down checks that met threshold
    scale_down_checks: u32,
    /// Last scaling action timestamp
    last_scaling_action: Option<DateTime<Utc>>,
    /// Time-series buffer for CPU usage history (for ML prediction)
    cpu_history: VecDeque<CpuDataPoint>,
    /// Set of nodes that have been commanded to terminate (for self-healing)
    terminating_nodes: Arc<RwLock<HashSet<Uuid>>>,
}

/// Scaling action types
#[derive(Debug, Clone, Serialize)]
pub enum ScalingAction {
    ScaleUp,
    ScaleDown { target_node_id: Uuid },
}

/// Fleet metrics calculated from all active nodes
#[derive(Debug, Clone)]
pub struct FleetMetrics {
    pub active_node_count: usize,
    pub avg_cpu_usage: f32,
    pub avg_memory_usage: f32,
    pub total_connections: u64,
    pub lowest_connection_node: Option<(Uuid, u64)>,
}

/// Webhook payload for scale-up actions
#[derive(Debug, Serialize)]
pub struct ScaleUpWebhookPayload {
    pub reason: String,
    pub timestamp: DateTime<Utc>,
    pub fleet_metrics: FleetMetricsForWebhook,
    pub prediction_info: Option<PredictionInfo>,
}

/// Self-healing webhook payload for unexpected node failures
#[derive(Debug, Serialize)]
pub struct SelfHealingWebhookPayload {
    pub reason: String,
    pub timestamp: DateTime<Utc>,
    pub failed_node_id: Uuid,
    pub failed_node_ip: std::net::IpAddr,
    pub fleet_metrics: FleetMetricsForWebhook,
}

/// Prediction information for webhook payload
#[derive(Debug, Serialize)]
pub struct PredictionInfo {
    pub predicted_cpu_usage: f32,
    pub prediction_horizon_minutes: u32,
    pub confidence: f32,
}

/// Fleet metrics for webhook payload
#[derive(Debug, Serialize)]
pub struct FleetMetricsForWebhook {
    pub active_nodes: usize,
    pub avg_cpu_usage: f32,
    pub avg_memory_usage: f32,
    pub total_connections: u64,
}

/// Node termination command payload
#[derive(Debug, Serialize)]
pub struct TerminationCommand {
    pub reason: String,
    pub timestamp: DateTime<Utc>,
    pub grace_period_seconds: u64,
}

impl ResourceManager {
    /// Create new resource manager with predictive capabilities
    pub fn new(
        node_registry: Arc<DashMap<Uuid, NodeInfo>>,
        config: OrchestratorConfig,
    ) -> Self {
        let http_client = Client::builder()
            .timeout(Duration::from_secs(30))
            .build()
            .expect("Failed to create HTTP client");

        Self {
            node_registry,
            config,
            http_client,
            scale_up_checks: 0,
            scale_down_checks: 0,
            last_scaling_action: None,
            cpu_history: VecDeque::new(),
            terminating_nodes: Arc::new(RwLock::new(HashSet::new())),
        }
    }

    /// Get reference to terminating nodes set for external access
    pub fn get_terminating_nodes(&self) -> Arc<RwLock<HashSet<Uuid>>> {
        Arc::clone(&self.terminating_nodes)
    }

    /// Start the resource manager loop
    #[instrument(skip(self))]
    pub async fn start(mut self) -> Result<()> {
        info!(
            interval_seconds = self.config.scaling_check_interval_seconds,
            min_fleet_size = self.config.min_fleet_size,
            scale_up_threshold = self.config.scale_up_cpu_threshold,
            scale_down_threshold = self.config.scale_down_cpu_threshold,
            "Starting resource manager"
        );

        let mut interval = time::interval(Duration::from_secs(
            self.config.scaling_check_interval_seconds,
        ));

        loop {
            interval.tick().await;
            
            if let Err(e) = self.perform_scaling_check().await {
                error!(error = %e, "Failed to perform scaling check");
            }
        }
    }

    /// Perform a scaling check and take action if needed (now with ML prediction)
    #[instrument(skip(self))]
    async fn perform_scaling_check(&mut self) -> Result<()> {
        let metrics = self.calculate_fleet_metrics();
        
        // Add current CPU usage to history for ML prediction
        self.add_cpu_data_point(metrics.avg_cpu_usage);
        
        debug!(
            active_nodes = metrics.active_node_count,
            avg_cpu = metrics.avg_cpu_usage,
            avg_memory = metrics.avg_memory_usage,
            total_connections = metrics.total_connections,
            history_points = self.cpu_history.len(),
            "Fleet metrics calculated"
        );

        // Try to predict future CPU usage
        let predicted_cpu = self.predict_future_cpu().unwrap_or(metrics.avg_cpu_usage);
        
        info!(
            current_cpu = metrics.avg_cpu_usage,
            predicted_cpu = predicted_cpu,
            "CPU prediction completed"
        );

        // Check if we should scale up (using prediction instead of current CPU)
        if self.should_scale_up_predictive(predicted_cpu, &metrics) {
            self.scale_up_checks += 1;
            self.scale_down_checks = 0;

            // Require 2 consecutive checks for scale-up to prevent flapping
            if self.scale_up_checks >= 2 {
                info!(
                    checks = self.scale_up_checks,
                    predicted_cpu = predicted_cpu,
                    current_cpu = metrics.avg_cpu_usage,
                    threshold = self.config.scale_up_cpu_threshold,
                    "Triggering predictive scale-up action"
                );
                
                self.execute_predictive_scale_up(&metrics, predicted_cpu).await?;
                self.reset_scaling_counters();
            } else {
                debug!(
                    checks = self.scale_up_checks,
                    "Predictive scale-up condition met, waiting for confirmation"
                );
            }
        }
        // Check if we should scale down (using current CPU for safety)
        else if self.should_scale_down(&metrics) {
            self.scale_down_checks += 1;
            self.scale_up_checks = 0;

            // Require 5 consecutive checks for scale-down to be more conservative
            if self.scale_down_checks >= 5 {
                if let Some((target_node_id, connections)) = metrics.lowest_connection_node {
                    info!(
                        checks = self.scale_down_checks,
                        target_node = %target_node_id,
                        connections = connections,
                        avg_cpu = metrics.avg_cpu_usage,
                        threshold = self.config.scale_down_cpu_threshold,
                        "Triggering scale-down action"
                    );
                    
                    self.execute_scale_down(target_node_id).await?;
                    self.reset_scaling_counters();
                } else {
                    warn!("Scale-down condition met but no suitable target node found");
                }
            } else {
                debug!(
                    checks = self.scale_down_checks,
                    "Scale-down condition met, waiting for confirmation"
                );
            }
        }
        // Conditions not met, reset counters
        else {
            if self.scale_up_checks > 0 || self.scale_down_checks > 0 {
                debug!("Scaling conditions no longer met, resetting counters");
            }
            self.reset_scaling_counters();
        }

        Ok(())
    }

    /// Calculate fleet-wide metrics from all active nodes
    fn calculate_fleet_metrics(&self) -> FleetMetrics {
        let active_nodes: Vec<_> = self
            .node_registry
            .iter()
            .filter(|entry| entry.value().status == NodeStatus::Active)
            .map(|entry| entry.value().clone())
            .collect();

        if active_nodes.is_empty() {
            return FleetMetrics {
                active_node_count: 0,
                avg_cpu_usage: 0.0,
                avg_memory_usage: 0.0,
                total_connections: 0,
                lowest_connection_node: None,
            };
        }

        let mut total_cpu = 0.0f64;
        let mut total_memory = 0.0f64;
        let mut total_connections = 0u64;
        let mut lowest_connections = u64::MAX;
        let mut lowest_node_id = None;

        for node in &active_nodes {
            if let Some(metrics) = &node.metrics {
                total_cpu += metrics.cpu_usage as f64;
                total_memory += metrics.memory_usage as f64;
                total_connections += metrics.active_connections;

                if metrics.active_connections < lowest_connections {
                    lowest_connections = metrics.active_connections;
                    lowest_node_id = Some(node.node_id);
                }
            }
        }

        let node_count = active_nodes.len();
        
        FleetMetrics {
            active_node_count: node_count,
            avg_cpu_usage: (total_cpu / node_count as f64) as f32,
            avg_memory_usage: (total_memory / node_count as f64) as f32,
            total_connections,
            lowest_connection_node: lowest_node_id.map(|id| (id, lowest_connections)),
        }
    }

    /// Add CPU data point to history buffer
    fn add_cpu_data_point(&mut self, cpu_usage: f32) {
        let now = Instant::now();
        self.cpu_history.push_back(CpuDataPoint {
            timestamp: now,
            cpu_usage,
        });

        // Keep only last 60 minutes of data (assuming 1-minute intervals)
        let cutoff_time = now - Duration::from_secs(60 * 60);
        while let Some(front) = self.cpu_history.front() {
            if front.timestamp < cutoff_time {
                self.cpu_history.pop_front();
            } else {
                break;
            }
        }

        debug!(
            data_points = self.cpu_history.len(),
            current_cpu = cpu_usage,
            "Added CPU data point to history"
        );
    }

    /// Predict future CPU usage using linear regression
    fn predict_future_cpu(&self) -> Option<f32> {
        // Need at least 10 data points for meaningful prediction
        if self.cpu_history.len() < 10 {
            debug!(
                data_points = self.cpu_history.len(),
                "Insufficient data for CPU prediction"
            );
            return None;
        }

        // Convert timestamps to minutes since start
        let start_time = self.cpu_history.front()?.timestamp;
        let features: Vec<f64> = self.cpu_history
            .iter()
            .map(|point| {
                let elapsed = point.timestamp.duration_since(start_time).as_secs() as f64 / 60.0;
                elapsed
            })
            .collect();

        let targets: Vec<f64> = self.cpu_history
            .iter()
            .map(|point| point.cpu_usage as f64)
            .collect();

        // Store the last time value before moving features into matrix
        let last_time = *features.last()?;

        // Create ndarray matrices
        let feature_matrix = Array2::from_shape_vec((features.len(), 1), features).ok()?;
        let target_array = Array1::from_vec(targets);

        // Create dataset
        let dataset = Dataset::new(feature_matrix, target_array);

        // Train linear regression model
        let model = match LinearRegression::default().fit(&dataset) {
            Ok(model) => model,
            Err(e) => {
                warn!(error = %e, "Failed to train prediction model");
                return None;
            }
        };

        // Predict CPU usage 10 minutes into the future
        let future_time_minutes = last_time + 10.0;
        let future_features = Array2::from_shape_vec((1, 1), vec![future_time_minutes]).ok()?;
        
        let prediction = model.predict(&future_features);
        let predicted_cpu = prediction[0] as f32;
        
        // Clamp prediction to reasonable bounds (0-100%)
        let clamped_prediction = predicted_cpu.max(0.0).min(1.0);
        
        info!(
            raw_prediction = predicted_cpu,
            clamped_prediction = clamped_prediction,
            data_points = self.cpu_history.len(),
            "CPU usage predicted for +10 minutes"
        );
        
        Some(clamped_prediction)
    }

    /// Check if the fleet should scale up based on predicted CPU
    fn should_scale_up_predictive(&self, predicted_cpu: f32, metrics: &FleetMetrics) -> bool {
        metrics.active_node_count > 0 
            && predicted_cpu > self.config.scale_up_cpu_threshold
    }

    /// Check if the fleet should scale down
    fn should_scale_down(&self, metrics: &FleetMetrics) -> bool {
        metrics.active_node_count > self.config.min_fleet_size as usize
            && metrics.avg_cpu_usage < self.config.scale_down_cpu_threshold
            && metrics.lowest_connection_node.is_some()
    }

    /// Execute predictive scale-up action by calling provisioning webhook
    #[instrument(skip(self))]
    async fn execute_predictive_scale_up(&mut self, metrics: &FleetMetrics, predicted_cpu: f32) -> Result<()> {
        let prediction_info = PredictionInfo {
            predicted_cpu_usage: predicted_cpu,
            prediction_horizon_minutes: 10,
            confidence: if self.cpu_history.len() >= 20 { 0.8 } else { 0.6 },
        };

        let payload = ScaleUpWebhookPayload {
            reason: "PREDICTED_HIGH_FLEET_CPU_LOAD".to_string(),
            timestamp: Utc::now(),
            fleet_metrics: FleetMetricsForWebhook {
                active_nodes: metrics.active_node_count,
                avg_cpu_usage: metrics.avg_cpu_usage,
                avg_memory_usage: metrics.avg_memory_usage,
                total_connections: metrics.total_connections,
            },
            prediction_info: Some(prediction_info),
        };

        info!(
            webhook_url = %self.config.provisioning_webhook_url,
            predicted_cpu = predicted_cpu,
            current_cpu = metrics.avg_cpu_usage,
            "Calling provisioning webhook for predictive scale-up"
        );

        let response = self
            .http_client
            .post(&self.config.provisioning_webhook_url)
            .json(&payload)
            .send()
            .await
            .context("Failed to send predictive scale-up webhook")?;

        if response.status().is_success() {
            info!(
                status = %response.status(),
                "Predictive scale-up webhook called successfully"
            );
            self.last_scaling_action = Some(Utc::now());
        } else {
            let status = response.status();
            let body = response.text().await.unwrap_or_else(|_| "unknown".to_string());
            warn!(
                status = %status,
                body = %body,
                "Predictive scale-up webhook returned error"
            );
        }

        Ok(())
    }

    /// Execute self-healing action for unexpected node failure
    #[instrument(skip(self))]
    pub async fn execute_self_healing(&mut self, failed_node_id: Uuid, failed_node_ip: std::net::IpAddr) -> Result<()> {
        let metrics = self.calculate_fleet_metrics();
        
        let payload = SelfHealingWebhookPayload {
            reason: "UNEXPECTED_NODE_FAILURE".to_string(),
            timestamp: Utc::now(),
            failed_node_id,
            failed_node_ip,
            fleet_metrics: FleetMetricsForWebhook {
                active_nodes: metrics.active_node_count,
                avg_cpu_usage: metrics.avg_cpu_usage,
                avg_memory_usage: metrics.avg_memory_usage,
                total_connections: metrics.total_connections,
            },
        };

        error!(
            failed_node = %failed_node_id,
            failed_ip = %failed_node_ip,
            webhook_url = %self.config.provisioning_webhook_url,
            "CRITICAL: UNEXPECTED NODE FAILURE DETECTED. Initiating self-healing."
        );

        let response = self
            .http_client
            .post(&self.config.provisioning_webhook_url)
            .json(&payload)
            .send()
            .await
            .context("Failed to send self-healing webhook")?;

        if response.status().is_success() {
            info!(
                status = %response.status(),
                failed_node = %failed_node_id,
                "Self-healing webhook called successfully"
            );
        } else {
            let status = response.status();
            let body = response.text().await.unwrap_or_else(|_| "unknown".to_string());
            error!(
                status = %status,
                body = %body,
                failed_node = %failed_node_id,
                "Self-healing webhook returned error"
            );
        }

        Ok(())
    }

    /// Execute scale-down action by commanding target node to terminate
    #[instrument(skip(self))]
    async fn execute_scale_down(&mut self, target_node_id: Uuid) -> Result<()> {
        // First, mark the node as draining in our registry
        if let Some(mut node_entry) = self.node_registry.get_mut(&target_node_id) {
            node_entry.status = NodeStatus::Draining;
            info!(
                target_node = %target_node_id,
                "Marked node as draining"
            );
        } else {
            warn!(
                target_node = %target_node_id,
                "Target node not found in registry"
            );
            return Ok(());
        }

        // Add to terminating nodes set for self-healing tracking
        {
            let mut terminating = self.terminating_nodes.write().await;
            terminating.insert(target_node_id);
            info!(
                target_node = %target_node_id,
                "Added node to terminating set for self-healing tracking"
            );
        }

        // Get node information for termination call
        let node_info = self
            .node_registry
            .get(&target_node_id)
            .map(|entry| entry.value().clone());

        if let Some(node) = node_info {
            let termination_url = format!("http://{}:9999/control/terminate", node.public_ip);
            
            let payload = TerminationCommand {
                reason: "LOW_FLEET_CPU_LOAD".to_string(),
                timestamp: Utc::now(),
                grace_period_seconds: 60,
            };

            info!(
                target_node = %target_node_id,
                termination_url = %termination_url,
                "Sending termination command to node"
            );

            // Note: In production, this should use the management API auth token
            let response = self
                .http_client
                .post(&termination_url)
                .header("Authorization", "Bearer secure-management-token-change-in-production")
                .json(&payload)
                .send()
                .await
                .context("Failed to send termination command")?;

            if response.status().is_success() {
                info!(
                    target_node = %target_node_id,
                    status = %response.status(),
                    "Termination command sent successfully"
                );
                self.last_scaling_action = Some(Utc::now());
            } else {
                let status = response.status();
                let body = response.text().await.unwrap_or_else(|_| "unknown".to_string());
                warn!(
                    target_node = %target_node_id,
                    status = %status,
                    body = %body,
                    "Termination command failed - removing from terminating set"
                );
                
                // Remove from terminating set if command failed
                let mut terminating = self.terminating_nodes.write().await;
                terminating.remove(&target_node_id);
            }
        }

        Ok(())
    }

    /// Reset scaling check counters
    fn reset_scaling_counters(&mut self) {
        self.scale_up_checks = 0;
        self.scale_down_checks = 0;
    }
}
