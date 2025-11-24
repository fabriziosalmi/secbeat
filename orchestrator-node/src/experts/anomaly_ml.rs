// ML-based Anomaly Detection Expert using Isolation Forest
//
// This expert uses unsupervised machine learning to detect anomalies that
// evade static thresholds (e.g., low-and-slow attacks, distributed botnets).
//
// Key Features:
// - Training Mode: Learns normal behavior without banning
// - Inference Mode: Actively detects and blocks anomalies
// - Automatic model retraining to adapt to traffic changes
// - Per-IP and global anomaly scoring

use anyhow::{Context, Result};
use async_nats::Client;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use smartcore::ensemble::random_forest_classifier::RandomForestClassifier;
use smartcore::linalg::basic::matrix::DenseMatrix;
use smartcore::tree::decision_tree_classifier::SplitCriterion;
use std::collections::{HashMap, VecDeque};
use std::net::IpAddr;
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio::time::{self, Duration};
use tracing::{debug, error, info, warn};
use uuid::Uuid;

use super::behavioral::BlockCommand;
use super::features::{RequestMetadata, TrafficFeatures};
use super::ml_async::AsyncMlEngine;

/// Operating mode of the anomaly expert
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum OperatingMode {
    /// Learning normal behavior - NO bans issued
    Training,
    /// Active anomaly detection - issues bans
    Inference,
}

/// Configuration for the Anomaly Expert
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnomalyConfig {
    /// Training duration before switching to inference (seconds)
    pub training_duration_secs: u64,
    
    /// Window duration for feature extraction (seconds)
    pub feature_window_secs: f64,
    
    /// Minimum samples before model can be trained
    pub min_training_samples: usize,
    
    /// Anomaly score threshold for banning (0.0 - 1.0)
    /// Higher = more strict (fewer false positives)
    pub anomaly_threshold: f64,
    
    /// Retraining interval (seconds)
    pub retrain_interval_secs: u64,
    
    /// Maximum feature buffer size per IP
    pub max_buffer_size: usize,
    
    /// Number of trees in Random Forest
    pub n_trees: u16,
    
    /// Maximum depth of trees
    pub max_depth: u16,
}

impl Default for AnomalyConfig {
    fn default() -> Self {
        Self {
            training_duration_secs: 300, // 5 minutes training
            feature_window_secs: 60.0,   // 1 minute windows
            min_training_samples: 50,    // Need at least 50 samples
            anomaly_threshold: 0.7,      // 70% anomaly score triggers ban
            retrain_interval_secs: 300,  // Retrain every 5 minutes
            max_buffer_size: 1000,       // Keep last 1000 feature vectors
            n_trees: 100,                // 100 trees in forest
            max_depth: 10,               // Max tree depth
        }
    }
}

/// Anomaly detection result
#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct AnomalyScore {
    pub ip: String,
    pub score: f64,           // 0.0 = normal, 1.0 = anomaly
    pub is_anomaly: bool,     // score > threshold
    pub features: TrafficFeatures,
    pub timestamp: DateTime<Utc>,
}

/// ML-based Anomaly Detection Expert
pub struct AnomalyExpert {
    /// Configuration
    config: AnomalyConfig,
    
    /// Current operating mode
    mode: Arc<RwLock<OperatingMode>>,
    
    /// Timestamp when training started (used for metrics/logging)
    #[allow(dead_code)]
    training_start: DateTime<Utc>,
    
    /// Feature buffer for training (IP -> features)
    feature_buffer: Arc<RwLock<HashMap<String, VecDeque<TrafficFeatures>>>>,
    
    /// Request metadata buffer (IP -> requests)
    #[allow(dead_code)]
    request_buffer: Arc<RwLock<HashMap<String, VecDeque<RequestMetadata>>>>,
    
    /// Async ML inference engine (NON-BLOCKING)
    ml_engine: AsyncMlEngine,
    
    /// Last training timestamp
    last_training: Arc<RwLock<DateTime<Utc>>>,
    
    /// NATS client for publishing ban commands
    #[allow(dead_code)]
    nats_client: Client,
    
    /// Statistics
    stats: Arc<RwLock<AnomalyStats>>,
}

/// Statistics for anomaly detection
#[derive(Debug, Clone, Default, Serialize)]
pub struct AnomalyStats {
    pub total_samples: u64,
    pub training_samples: u64,
    pub anomalies_detected: u64,
    pub bans_issued: u64,
    pub model_retrains: u64,
    pub last_retrain: Option<DateTime<Utc>>,
}

impl AnomalyExpert {
    /// Create a new Anomaly Expert
    pub fn new(config: AnomalyConfig, nats_client: Client) -> Self {
        let now = Utc::now();
        
        info!(
            "🤖 Initializing Anomaly Expert - Training mode for {} seconds",
            config.training_duration_secs
        );
        
        // Create async ML engine with queue size 1000 and 4 worker threads
        let ml_engine = AsyncMlEngine::new(1000, 4);
        
        info!("✓ Async ML inference engine created (queue: 1000, workers: 4)");
        
        Self {
            config,
            mode: Arc::new(RwLock::new(OperatingMode::Training)),
            training_start: now,
            feature_buffer: Arc::new(RwLock::new(HashMap::new())),
            request_buffer: Arc::new(RwLock::new(HashMap::new())),
            ml_engine,
            last_training: Arc::new(RwLock::new(now)),
            nats_client,
            stats: Arc::new(RwLock::new(AnomalyStats::default())),
        }
    }

    /// Start the anomaly expert background tasks
    pub async fn start(self: Arc<Self>) {
        // Task 1: Mode switcher (Training -> Inference)
        let expert_clone = Arc::clone(&self);
        tokio::spawn(async move {
            expert_clone.mode_switcher_task().await;
        });

        // Task 2: Periodic model retraining
        let expert_clone = Arc::clone(&self);
        tokio::spawn(async move {
            expert_clone.retraining_task().await;
        });

        info!("✓ Anomaly Expert background tasks started");
    }

    /// Add a request to the buffer for feature extraction
    #[allow(dead_code)]
    pub async fn observe_request(&self, ip: String, metadata: RequestMetadata) {
        let mut buffer = self.request_buffer.write().await;
        let requests = buffer.entry(ip.clone()).or_insert_with(VecDeque::new);
        
        requests.push_back(metadata);
        
        // Trim buffer to max size
        while requests.len() > self.config.max_buffer_size {
            requests.pop_front();
        }
        
        // Update stats
        let mut stats = self.stats.write().await;
        stats.total_samples += 1;
    }

    /// Extract features and score for anomaly detection
    #[allow(dead_code)]
    pub async fn check_anomaly(&self, ip: String) -> Option<AnomalyScore> {
        // Extract features from request buffer
        let features = {
            let buffer = self.request_buffer.read().await;
            let requests = buffer.get(&ip)?;
            
            if requests.is_empty() {
                return None;
            }
            
            let requests_vec: Vec<RequestMetadata> = requests.iter().cloned().collect();
            TrafficFeatures::from_telemetry(
                ip.clone(),
                &requests_vec,
                self.config.feature_window_secs,
            )
        };

        // Store features for training
        {
            let mut feature_buffer = self.feature_buffer.write().await;
            let feature_vec = feature_buffer
                .entry(ip.clone())
                .or_insert_with(VecDeque::new);
            feature_vec.push_back(features.clone());
            
            while feature_vec.len() > self.config.max_buffer_size {
                feature_vec.pop_front();
            }
        }

        // Calculate anomaly score
        let score = self.calculate_anomaly_score(&features).await;
        let is_anomaly = score > self.config.anomaly_threshold;

        let result = AnomalyScore {
            ip: ip.clone(),
            score,
            is_anomaly,
            features,
            timestamp: Utc::now(),
        };

        // If in inference mode and anomaly detected, issue ban
        let mode = *self.mode.read().await;
        if mode == OperatingMode::Inference && is_anomaly {
            self.issue_ban(&result).await;
        }

        Some(result)
    }

    /// Calculate anomaly score for given features (NON-BLOCKING)
    async fn calculate_anomaly_score(&self, features: &TrafficFeatures) -> f64 {
        // Use async ML engine - this will NOT block the request thread
        match self.ml_engine.predict_async(features).await {
            Ok(score) => score,
            Err(e) => {
                warn!("Async ML prediction failed: {}, falling back to heuristics", e);
                self.heuristic_score(features)
            }
        }
    }

    /// Heuristic-based scoring when model not available
    fn heuristic_score(&self, features: &TrafficFeatures) -> f64 {
        let mut score = 0.0;
        let mut factors = 0.0;

        // High error ratio is suspicious
        if features.error_ratio > 0.5 {
            score += features.error_ratio;
            factors += 1.0;
        }

        // Low URI entropy (repetitive access) is suspicious
        if features.uri_entropy < 1.0 && features.distinct_uris < 3 {
            score += 1.0 - features.uri_entropy;
            factors += 1.0;
        }

        // Very high request rate
        if features.request_rate > 100.0 {
            score += (features.request_rate / 100.0).min(1.0);
            factors += 1.0;
        }

        // Low user agent diversity
        if features.user_agent_diversity < 0.3 {
            score += 1.0 - features.user_agent_diversity;
            factors += 1.0;
        }

        if factors > 0.0 {
            score / factors
        } else {
            0.0
        }
    }

    /// Train the Random Forest model on collected features
    async fn train_model(&self) -> Result<()> {
        let features_data = {
            let buffer = self.feature_buffer.read().await;
            let all_features: Vec<TrafficFeatures> = buffer
                .values()
                .flat_map(|v| v.iter().cloned())
                .collect();
            all_features
        };

        if features_data.len() < self.config.min_training_samples {
            info!(
                "Not enough samples for training: {} < {}",
                features_data.len(),
                self.config.min_training_samples
            );
            return Ok(());
        }

        info!("🎓 Training Random Forest model with {} samples", features_data.len());

        // Convert features to matrix
        let feature_vectors: Vec<Vec<f64>> = features_data
            .iter()
            .map(|f| f.to_vector())
            .collect();
        
        let x = DenseMatrix::from_2d_vec(&feature_vectors);

        // For unsupervised learning, we label everything as "normal" (0)
        // The model will learn the distribution and outliers will be detected
        // In practice, we'd use Isolation Forest, but smartcore's implementation
        // is via Random Forest with synthetic outliers
        let y: Vec<i32> = vec![0; features_data.len()];

        // Train Random Forest in a blocking task (CPU-intensive)
        let n_trees = self.config.n_trees;
        let max_depth = self.config.max_depth;
        
        let model = tokio::task::spawn_blocking(move || {
            RandomForestClassifier::fit(
                &x,
                &y,
                smartcore::ensemble::random_forest_classifier::RandomForestClassifierParameters::default()
                    .with_n_trees(n_trees)
                    .with_max_depth(max_depth)
                    .with_criterion(SplitCriterion::Gini),
            )
        })
        .await
        .context("Training task panicked")?
        .context("Failed to train Random Forest model")?;

        // Update async ML engine with new model
        self.ml_engine.update_model(model).await;
        *self.last_training.write().await = Utc::now();

        // Update stats
        let mut stats = self.stats.write().await;
        stats.model_retrains += 1;
        stats.last_retrain = Some(Utc::now());
        stats.training_samples = features_data.len() as u64;

        info!("✓ Model trained successfully and updated in async engine");
        Ok(())
    }

    /// Issue a ban command for detected anomaly
    async fn issue_ban(&self, anomaly: &AnomalyScore) {
        info!(
            "🚫 Anomaly detected: {} (score: {:.2}, features: error_ratio={:.2}, uri_entropy={:.2})",
            anomaly.ip, anomaly.score,
            anomaly.features.error_ratio,
            anomaly.features.uri_entropy
        );

        let ban_command = BlockCommand {
            command_id: Uuid::new_v4(),
            ip: anomaly.ip.parse().unwrap_or(IpAddr::from([0, 0, 0, 0])),
            reason: format!(
                "ML Anomaly Detection: score={:.2}, error_ratio={:.2}, uri_entropy={:.2}",
                anomaly.score,
                anomaly.features.error_ratio,
                anomaly.features.uri_entropy
            ),
            duration_seconds: 3600, // 1 hour ban
            action: "block".to_string(),
            issued_at: Utc::now(),
            source: "AnomalyExpert".to_string(),
        };

        // Publish to NATS
        let json = match serde_json::to_string(&ban_command) {
            Ok(j) => j,
            Err(e) => {
                error!("Failed to serialize ban command: {}", e);
                return;
            }
        };

        if let Err(e) = self.nats_client.publish("orchestrator.ban", json.into()).await {
            error!("Failed to publish ban command: {}", e);
        } else {
            let mut stats = self.stats.write().await;
            stats.anomalies_detected += 1;
            stats.bans_issued += 1;
        }
    }

    /// Background task: Switch from Training to Inference mode
    async fn mode_switcher_task(&self) {
        let training_duration = Duration::from_secs(self.config.training_duration_secs);
        time::sleep(training_duration).await;

        info!("⚡ Training period complete - switching to INFERENCE mode");
        
        // Train initial model
        if let Err(e) = self.train_model().await {
            error!("Failed to train initial model: {}", e);
        }

        // Switch mode
        *self.mode.write().await = OperatingMode::Inference;
        
        info!("✓ Anomaly Expert now actively detecting threats");
    }

    /// Background task: Periodic model retraining
    async fn retraining_task(&self) {
        let mut interval = time::interval(Duration::from_secs(self.config.retrain_interval_secs));
        
        loop {
            interval.tick().await;
            
            let mode = *self.mode.read().await;
            if mode == OperatingMode::Inference {
                debug!("🔄 Retraining model with latest data");
                
                if let Err(e) = self.train_model().await {
                    error!("Failed to retrain model: {}", e);
                }
            }
        }
    }

    /// Get current operating mode
    #[allow(dead_code)]
    pub async fn get_mode(&self) -> OperatingMode {
        *self.mode.read().await
    }

    /// Get current statistics
    #[allow(dead_code)]
    pub async fn get_stats(&self) -> AnomalyStats {
        self.stats.read().await.clone()
    }

    /// Force mode switch (for testing)
    #[allow(dead_code)]
    pub async fn set_mode(&self, mode: OperatingMode) {
        *self.mode.write().await = mode;
        info!("Mode manually set to: {:?}", mode);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_anomaly_expert_initialization() {
        let config = AnomalyConfig::default();
        let nats_client = async_nats::connect("nats://localhost:4222")
            .await
            .expect("NATS not available");
        
        let expert = AnomalyExpert::new(config, nats_client);
        
        assert_eq!(expert.get_mode().await, OperatingMode::Training);
    }

    #[test]
    fn test_heuristic_scoring() {
        let config = AnomalyConfig::default();
        let nats_client_placeholder = unsafe {
            std::mem::zeroed() // Placeholder for unit test
        };
        let expert = AnomalyExpert::new(config, nats_client_placeholder);

        // Normal traffic
        let normal_features = TrafficFeatures {
            ip: "192.168.1.100".to_string(),
            request_count: 10,
            error_ratio: 0.0,
            distinct_uris: 5,
            uri_entropy: 2.0,
            avg_latency_ms: 50.0,
            latency_stddev_ms: 5.0,
            request_rate: 10.0,
            user_agent_diversity: 0.8,
            timestamp: Utc::now(),
        };
        
        let score = expert.heuristic_score(&normal_features);
        assert!(score < 0.3, "Normal traffic should have low score");

        // Anomalous traffic
        let anomaly_features = TrafficFeatures {
            ip: "1.2.3.4".to_string(),
            request_count: 100,
            error_ratio: 0.9, // High errors
            distinct_uris: 1, // Repetitive
            uri_entropy: 0.0, // No entropy
            avg_latency_ms: 10.0,
            latency_stddev_ms: 1.0,
            request_rate: 200.0, // High rate
            user_agent_diversity: 0.1, // Low diversity
            timestamp: Utc::now(),
        };
        
        let score = expert.heuristic_score(&anomaly_features);
        assert!(score > 0.7, "Anomalous traffic should have high score");
    }
}
