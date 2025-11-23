// Distributed State Synchronization Manager
//
// This module manages distributed state across mitigation nodes using CRDTs.
// It provides:
// - Background sync task to broadcast state updates via NATS
// - Listener task to receive and merge updates from other nodes
// - Global rate limiting based on distributed counters

use anyhow::{Context, Result};
use async_nats::Client;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use tokio::time;
use tokio_stream::StreamExt;
use tracing::{debug, error, info, warn};
use uuid::Uuid;

use super::crdt::{GCounter, NodeId};

/// Configuration for state synchronization
#[derive(Debug, Clone)]
pub struct StateSyncConfig {
    /// How often to broadcast state updates (seconds)
    pub sync_interval_secs: u64,
    
    /// Whether to use delta-based sync (more efficient)
    pub use_delta_sync: bool,
    
    /// Maximum number of counters to track
    pub max_counters: usize,
    
    /// TTL for inactive counters (seconds)
    pub counter_ttl_secs: u64,
}

impl Default for StateSyncConfig {
    fn default() -> Self {
        Self {
            sync_interval_secs: 1,      // Sync every second
            use_delta_sync: true,       // Use efficient delta sync
            max_counters: 100_000,      // Track up to 100K keys
            counter_ttl_secs: 300,      // 5 minute TTL
        }
    }
}

/// State update message sent via NATS
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StateUpdate {
    /// Node that sent this update
    pub node_id: NodeId,
    
    /// Timestamp of the update
    pub timestamp: chrono::DateTime<chrono::Utc>,
    
    /// Map of counter_key → G-Counter
    /// For delta sync, only contains changed counters
    pub counters: HashMap<String, GCounter>,
    
    /// Whether this is a delta update
    pub is_delta: bool,
}

/// Distributed State Manager
///
/// Manages distributed counters (G-Counters) for rate limiting across nodes.
/// Uses NATS for state synchronization.
pub struct StateManager {
    /// This node's unique identifier
    node_id: NodeId,
    
    /// Configuration
    config: StateSyncConfig,
    
    /// Local state: counter_key → G-Counter
    state: Arc<RwLock<HashMap<String, GCounter>>>,
    
    /// Previous state snapshot (for delta sync)
    previous_state: Arc<RwLock<HashMap<String, GCounter>>>,
    
    /// NATS client for broadcasting/receiving updates
    nats_client: Client,
    
    /// Statistics
    stats: Arc<RwLock<StateStats>>,
}

/// Statistics for state synchronization
#[derive(Debug, Clone, Default, Serialize)]
pub struct StateStats {
    pub local_increments: u64,
    pub remote_merges: u64,
    pub broadcasts_sent: u64,
    pub updates_received: u64,
    pub merge_conflicts: u64,
    pub active_counters: usize,
}

impl StateManager {
    /// Create a new State Manager
    pub fn new(config: StateSyncConfig, nats_client: Client) -> Self {
        let node_id = Uuid::new_v4();
        
        info!(
            "🌐 Initializing Distributed State Manager - Node ID: {}",
            node_id
        );

        Self {
            node_id,
            config,
            state: Arc::new(RwLock::new(HashMap::new())),
            previous_state: Arc::new(RwLock::new(HashMap::new())),
            nats_client,
            stats: Arc::new(RwLock::new(StateStats::default())),
        }
    }

    /// Start background synchronization tasks
    pub async fn start(self: Arc<Self>) {
        // Task 1: Periodic state broadcast
        let manager_clone = Arc::clone(&self);
        tokio::spawn(async move {
            manager_clone.sync_broadcast_task().await;
        });

        // Task 2: Listen for remote updates
        let manager_clone = Arc::clone(&self);
        tokio::spawn(async move {
            manager_clone.sync_listener_task().await;
        });

        // Task 3: Cleanup old counters
        let manager_clone = Arc::clone(&self);
        tokio::spawn(async move {
            manager_clone.cleanup_task().await;
        });

        info!("✓ Distributed State Manager tasks started");
    }

    /// Increment a distributed counter
    ///
    /// This is called from the request path (hot path), so it must be fast.
    /// Updates are buffered and flushed by the sync task.
    pub async fn increment(&self, key: impl Into<String>, delta: u64) {
        let key = key.into();
        let mut state = self.state.write().await;
        
        let counter = state.entry(key).or_insert_with(GCounter::new);
        counter.inc(self.node_id, delta);

        let mut stats = self.stats.write().await;
        stats.local_increments += 1;
        stats.active_counters = state.len();
    }

    /// Get the global value for a counter
    ///
    /// This returns the sum across all nodes.
    pub async fn get_global_value(&self, key: &str) -> u64 {
        let state = self.state.read().await;
        state.get(key).map(|c| c.value()).unwrap_or(0)
    }

    /// Check if a key exceeds a global limit
    ///
    /// This is the main API for global rate limiting.
    pub async fn check_global_limit(&self, key: &str, limit: u64) -> bool {
        self.get_global_value(key).await > limit
    }

    /// Get statistics
    pub async fn get_stats(&self) -> StateStats {
        self.stats.read().await.clone()
    }

    /// Background task: Broadcast state updates
    async fn sync_broadcast_task(&self) {
        let mut interval = time::interval(Duration::from_secs(self.config.sync_interval_secs));

        loop {
            interval.tick().await;

            if let Err(e) = self.broadcast_state().await {
                error!("Failed to broadcast state: {}", e);
            }
        }
    }

    /// Broadcast current state to all nodes
    async fn broadcast_state(&self) -> Result<()> {
        let update = if self.config.use_delta_sync {
            self.create_delta_update().await?
        } else {
            self.create_full_update().await?
        };

        // Skip empty updates
        if update.counters.is_empty() {
            return Ok(());
        }

        let json = serde_json::to_string(&update)
            .context("Failed to serialize state update")?;

        self.nats_client
            .publish("secbeat.state.sync", json.into())
            .await
            .context("Failed to publish state update")?;

        let mut stats = self.stats.write().await;
        stats.broadcasts_sent += 1;

        debug!(
            "Broadcasted {} counters (delta: {})",
            update.counters.len(),
            update.is_delta
        );

        Ok(())
    }

    /// Create a delta update (only changed counters)
    async fn create_delta_update(&self) -> Result<StateUpdate> {
        let current_state = self.state.read().await;
        let previous_state = self.previous_state.read().await;

        let mut delta_counters = HashMap::new();

        for (key, current_counter) in current_state.iter() {
            if let Some(previous_counter) = previous_state.get(key) {
                let delta = current_counter.delta(previous_counter);
                if !delta.is_empty() {
                    delta_counters.insert(key.clone(), delta);
                }
            } else {
                // New counter
                delta_counters.insert(key.clone(), current_counter.clone());
            }
        }

        // Update previous state snapshot
        drop(current_state);
        drop(previous_state);
        *self.previous_state.write().await = self.state.read().await.clone();

        Ok(StateUpdate {
            node_id: self.node_id,
            timestamp: chrono::Utc::now(),
            counters: delta_counters,
            is_delta: true,
        })
    }

    /// Create a full state update
    async fn create_full_update(&self) -> Result<StateUpdate> {
        let state = self.state.read().await;

        Ok(StateUpdate {
            node_id: self.node_id,
            timestamp: chrono::Utc::now(),
            counters: state.clone(),
            is_delta: false,
        })
    }

    /// Background task: Listen for remote updates
    async fn sync_listener_task(&self) {
        let mut subscriber = match self.nats_client.subscribe("secbeat.state.sync").await {
            Ok(sub) => sub,
            Err(e) => {
                error!("Failed to subscribe to state sync topic: {}", e);
                return;
            }
        };

        info!("Listening for state updates on secbeat.state.sync");

        while let Some(msg) = subscriber.next().await {
            if let Err(e) = self.handle_remote_update(&msg.payload).await {
                error!("Failed to handle remote update: {}", e);
            }
        }
    }

    /// Handle a remote state update
    async fn handle_remote_update(&self, payload: &[u8]) -> Result<()> {
        let update: StateUpdate = serde_json::from_slice(payload)
            .context("Failed to deserialize state update")?;

        // Ignore updates from ourselves
        if update.node_id == self.node_id {
            return Ok(());
        }

        debug!(
            "Received state update from node {} ({} counters)",
            update.node_id,
            update.counters.len()
        );

        // Merge remote state
        let mut state = self.state.write().await;
        let counters_len = update.counters.len();
        for (key, remote_counter) in update.counters {
            let local_counter = state.entry(key).or_insert_with(GCounter::new);
            local_counter.merge(&remote_counter);
        }

        let mut stats = self.stats.write().await;
        stats.updates_received += 1;
        stats.remote_merges += counters_len as u64;

        Ok(())
    }

    /// Background task: Cleanup old counters
    async fn cleanup_task(&self) {
        let mut interval = time::interval(Duration::from_secs(60)); // Cleanup every minute

        loop {
            interval.tick().await;

            if let Err(e) = self.cleanup_old_counters().await {
                error!("Failed to cleanup old counters: {}", e);
            }
        }
    }

    /// Remove inactive counters (TTL expired)
    async fn cleanup_old_counters(&self) -> Result<()> {
        let mut state = self.state.write().await;
        
        // For now, simple size-based cleanup
        // In production, you'd track last-access timestamps
        if state.len() > self.config.max_counters {
            let to_remove = state.len() - self.config.max_counters;
            let keys_to_remove: Vec<String> = state
                .keys()
                .take(to_remove)
                .cloned()
                .collect();

            for key in keys_to_remove {
                state.remove(&key);
            }

            warn!("Cleaned up {} old counters", to_remove);
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_state_manager_increment() {
        let nats_client = async_nats::connect("nats://localhost:4222")
            .await
            .expect("NATS not available");

        let config = StateSyncConfig::default();
        let manager = StateManager::new(config, nats_client);

        manager.increment("test-key", 10).await;
        assert_eq!(manager.get_global_value("test-key").await, 10);

        manager.increment("test-key", 5).await;
        assert_eq!(manager.get_global_value("test-key").await, 15);
    }

    #[tokio::test]
    async fn test_state_manager_check_limit() {
        let nats_client = async_nats::connect("nats://localhost:4222")
            .await
            .expect("NATS not available");

        let config = StateSyncConfig::default();
        let manager = StateManager::new(config, nats_client);

        manager.increment("test-key", 50).await;

        assert!(!manager.check_global_limit("test-key", 100).await);
        assert!(manager.check_global_limit("test-key", 40).await);
    }
}
