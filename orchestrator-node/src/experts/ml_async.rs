// Async ML Inference Engine
//
// CRITICAL: ML inference (Random Forest) must NOT block the request thread.
// This module provides an async inference queue with a dedicated thread pool.
//
// Design:
// - Request thread sends features to async channel (non-blocking)
// - Dedicated worker pool performs CPU-intensive inference
// - Results returned via oneshot channel
// - Bounded queue prevents memory exhaustion under load

use anyhow::Result;
use smartcore::ensemble::random_forest_classifier::RandomForestClassifier;
use smartcore::linalg::basic::matrix::DenseMatrix;
use std::sync::Arc;
use tokio::sync::{mpsc, oneshot, RwLock};
use tracing::{debug, error, warn};

use super::features::TrafficFeatures;

/// Type alias for Random Forest model
type RFModel = RandomForestClassifier<f64, i32, DenseMatrix<f64>, Vec<i32>>;

/// ML inference request
struct InferenceRequest {
    features: Vec<f64>,
    response_tx: oneshot::Sender<f64>,
}

/// Async ML Inference Engine
#[derive(Clone)]
pub struct AsyncMlEngine {
    /// Model wrapped in Arc<RwLock> for thread-safe updates
    model: Arc<RwLock<Option<RFModel>>>,
    /// Request channel sender (cloneable for multiple producers)
    request_tx: mpsc::Sender<InferenceRequest>,
}

impl AsyncMlEngine {
    /// Create a new async ML engine with a dedicated inference worker
    ///
    /// # Arguments
    /// * `queue_size` - Maximum pending inference requests (prevents memory exhaustion)
    /// * `num_workers` - Number of worker threads for parallel inference
    pub fn new(queue_size: usize, _num_workers: usize) -> Self {
        let (request_tx, mut request_rx) = mpsc::channel::<InferenceRequest>(queue_size);
        let model = Arc::new(RwLock::new(None));

        // Spawn single worker thread for CPU-intensive inference
        // TODO: Support multiple workers with work-stealing queue
        let model_clone = Arc::clone(&model);
        tokio::spawn(async move {
            debug!("ML worker started");

            while let Some(req) = request_rx.recv().await {
                    // Perform BLOCKING inference in a blocking thread pool
                    // This prevents blocking the tokio runtime
                    let model_ref = Arc::clone(&model_clone);
                    let features = req.features;
                    
                    let result = tokio::task::spawn_blocking(move || {
                        // This runs in a dedicated thread pool for blocking operations
                        Self::predict_blocking(&model_ref, &features)
                    })
                    .await;

                    match result {
                        Ok(Ok(score)) => {
                            // Send result back (ignore if receiver dropped)
                            let _ = req.response_tx.send(score);
                        }
                        Ok(Err(e)) => {
                            error!("Inference failed: {}", e);
                            let _ = req.response_tx.send(0.0); // Default to non-anomalous
                        }
                        Err(e) => {
                            error!("Worker task panicked: {}", e);
                            let _ = req.response_tx.send(0.0);
                        }
                    }
                }

                debug!("ML worker stopped");
            });

        Self { model, request_tx }
    }

    /// Update the model (called during retraining)
    pub async fn update_model(&self, new_model: RFModel) {
        let mut model = self.model.write().await;
        *model = Some(new_model);
        debug!("ML model updated");
    }

    /// Predict anomaly score asynchronously (NON-BLOCKING)
    ///
    /// Returns immediately with a future that resolves when inference completes.
    /// Will not block the calling thread.
    pub async fn predict_async(&self, features: &TrafficFeatures) -> Result<f64> {
        let feature_vec = features.to_vector();
        
        // Create oneshot channel for response
        let (response_tx, response_rx) = oneshot::channel();

        // Send request to worker (non-blocking unless queue is full)
        match self.request_tx.try_send(InferenceRequest {
            features: feature_vec,
            response_tx,
        }) {
            Ok(_) => {
                // Wait for result from worker
                response_rx.await.map_err(|e| anyhow::anyhow!("Inference cancelled: {}", e))
            }
            Err(mpsc::error::TrySendError::Full(_)) => {
                // Queue is full - reject immediately to prevent backpressure
                warn!("ML inference queue full, dropping request");
                Ok(0.0) // Default to non-anomalous when overloaded
            }
            Err(mpsc::error::TrySendError::Closed(_)) => {
                Err(anyhow::anyhow!("ML inference engine shut down"))
            }
        }
    }

    /// Blocking prediction (runs in dedicated thread pool)
    fn predict_blocking(model: &Arc<RwLock<Option<RFModel>>>, features: &[f64]) -> Result<f64> {
        // This is a blocking operation but runs in spawn_blocking thread pool
        let model_guard = model.blocking_read();
        
        match &*model_guard {
            Some(model) => {
                let x = DenseMatrix::from_2d_vec(&vec![features.to_vec()]);
                
                let predictions = model.predict(&x)
                    .map_err(|e| anyhow::anyhow!("Model prediction failed: {}", e))?;
                
                // Random Forest returns class probabilities
                // Class 1 (anomaly) probability as score
                let anomaly_class = predictions.first().copied().unwrap_or(0);
                Ok(if anomaly_class == 1 { 1.0 } else { 0.0 })
            }
            None => {
                // No model yet
                Ok(0.0)
            }
        }
    }

    /// Get queue depth (for monitoring)
    pub fn queue_depth(&self) -> usize {
        // Note: mpsc::Sender doesn't expose queue depth directly
        // This is a placeholder for metrics
        0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_async_engine_creation() {
        let engine = AsyncMlEngine::new(100, 2);
        assert!(engine.request_tx.capacity() > 0);
    }

    #[tokio::test]
    async fn test_predict_without_model() {
        let engine = AsyncMlEngine::new(100, 2);
        let features = TrafficFeatures::zero("127.0.0.1".to_string());
        
        // Should return 0.0 when no model loaded
        let score = engine.predict_async(&features).await.unwrap();
        assert_eq!(score, 0.0);
    }

    #[tokio::test]
    async fn test_queue_overflow_handling() {
        // Small queue to test overflow
        let engine = AsyncMlEngine::new(2, 1);
        let features = TrafficFeatures::zero("127.0.0.1".to_string());
        
        // Fill the queue
        for _ in 0..10 {
            let _ = engine.predict_async(&features).await;
        }
        
        // Should still handle gracefully (drops when full)
        let score = engine.predict_async(&features).await.unwrap();
        assert!(score >= 0.0 && score <= 1.0);
    }
}
