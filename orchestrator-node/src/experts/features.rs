// Feature Extraction Module for ML-based Anomaly Detection
//
// This module extracts meaningful features from raw telemetry data
// for use with the Isolation Forest model.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Traffic features extracted from telemetry events for a single IP
///
/// These features are designed to capture behavioral patterns that
/// distinguish normal traffic from anomalies:
/// - Volume patterns (request rate)
/// - Error patterns (error ratio)
/// - Diversity patterns (URI entropy)
/// - Latency patterns (response time)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrafficFeatures {
    /// IP address these features belong to
    pub ip: String,

    /// Total number of requests in the observation window
    pub request_count: u64,

    /// Ratio of error responses (4xx + 5xx) to total requests
    /// Range: [0.0, 1.0]
    pub error_ratio: f64,

    /// Number of distinct URIs accessed
    /// High value = scanning/probing behavior
    pub distinct_uris: usize,

    /// Shannon entropy of URI distribution
    /// High entropy = diverse access patterns (normal user)
    /// Low entropy = repetitive access (bot/scanner)
    /// Range: [0.0, log2(distinct_uris)]
    pub uri_entropy: f64,

    /// Average latency in milliseconds
    /// Sudden spikes might indicate attacks
    pub avg_latency_ms: f64,

    /// Standard deviation of latency
    /// High variance might indicate probing/timing attacks
    pub latency_stddev_ms: f64,

    /// Request rate (requests per second)
    pub request_rate: f64,

    /// Ratio of unique user agents seen
    /// Range: [0.0, 1.0]
    /// Low value = same UA repeated (bot)
    pub user_agent_diversity: f64,

    /// Timestamp of feature extraction
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

impl TrafficFeatures {
    /// Create features from raw telemetry data
    #[allow(dead_code)]
    pub fn from_telemetry(
        ip: String,
        requests: &[RequestMetadata],
        window_duration_secs: f64,
    ) -> Self {
        let request_count = requests.len() as u64;

        // Calculate error ratio
        let errors = requests
            .iter()
            .filter(|r| r.status_code >= 400)
            .count();
        let error_ratio = if request_count > 0 {
            errors as f64 / request_count as f64
        } else {
            0.0
        };

        // Calculate URI diversity
        let mut uri_counts: HashMap<String, usize> = HashMap::new();
        for req in requests {
            *uri_counts.entry(req.uri.clone()).or_insert(0) += 1;
        }
        let distinct_uris = uri_counts.len();

        // Calculate URI entropy
        let uri_entropy = calculate_entropy(&uri_counts, request_count as usize);

        // Calculate latency statistics
        let latencies: Vec<f64> = requests.iter().map(|r| r.latency_ms).collect();
        let (avg_latency_ms, latency_stddev_ms) = if !latencies.is_empty() {
            let avg = latencies.iter().sum::<f64>() / latencies.len() as f64;
            let variance = latencies
                .iter()
                .map(|l| (l - avg).powi(2))
                .sum::<f64>()
                / latencies.len() as f64;
            (avg, variance.sqrt())
        } else {
            (0.0, 0.0)
        };

        // Calculate request rate
        let request_rate = if window_duration_secs > 0.0 {
            request_count as f64 / window_duration_secs
        } else {
            0.0
        };

        // Calculate user agent diversity
        let mut user_agents: HashMap<String, usize> = HashMap::new();
        for req in requests {
            *user_agents
                .entry(req.user_agent.clone())
                .or_insert(0) += 1;
        }
        let user_agent_diversity = if request_count > 0 {
            user_agents.len() as f64 / request_count as f64
        } else {
            0.0
        };

        Self {
            ip,
            request_count,
            error_ratio,
            distinct_uris,
            uri_entropy,
            avg_latency_ms,
            latency_stddev_ms,
            request_rate,
            user_agent_diversity,
            timestamp: chrono::Utc::now(),
        }
    }

    /// Convert features to a vector for ML model input
    ///
    /// Order matters! Must be consistent across training and inference.
    /// Returns: [request_count, error_ratio, distinct_uris, uri_entropy,
    ///           avg_latency_ms, latency_stddev_ms, request_rate, user_agent_diversity]
    pub fn to_vector(&self) -> Vec<f64> {
        vec![
            self.request_count as f64,
            self.error_ratio,
            self.distinct_uris as f64,
            self.uri_entropy,
            self.avg_latency_ms,
            self.latency_stddev_ms,
            self.request_rate,
            self.user_agent_diversity,
        ]
    }

    /// Get feature names (for debugging/logging)
    #[allow(dead_code)]
    pub fn feature_names() -> Vec<&'static str> {
        vec![
            "request_count",
            "error_ratio",
            "distinct_uris",
            "uri_entropy",
            "avg_latency_ms",
            "latency_stddev_ms",
            "request_rate",
            "user_agent_diversity",
        ]
    }

    /// Number of features in the vector
    #[allow(dead_code)]
    pub const FEATURE_COUNT: usize = 8;

    /// Create a "zero" feature vector (for initialization)
    #[allow(dead_code)]
    pub fn zero(ip: String) -> Self {
        Self {
            ip,
            request_count: 0,
            error_ratio: 0.0,
            distinct_uris: 0,
            uri_entropy: 0.0,
            avg_latency_ms: 0.0,
            latency_stddev_ms: 0.0,
            request_rate: 0.0,
            user_agent_diversity: 0.0,
            timestamp: chrono::Utc::now(),
        }
    }
}

/// Metadata for a single request (extracted from telemetry)
#[derive(Debug, Clone)]
pub struct RequestMetadata {
    #[allow(dead_code)]
    pub uri: String,
    #[allow(dead_code)]
    pub status_code: u16,
    #[allow(dead_code)]
    pub latency_ms: f64,
    #[allow(dead_code)]
    pub user_agent: String,
}

/// Calculate Shannon entropy of a distribution
///
/// H = -Σ(p_i * log2(p_i))
///
/// where p_i is the probability of element i
fn calculate_entropy(counts: &HashMap<String, usize>, total: usize) -> f64 {
    if total == 0 {
        return 0.0;
    }

    let mut entropy = 0.0;
    for &count in counts.values() {
        if count > 0 {
            let probability = count as f64 / total as f64;
            entropy -= probability * probability.log2();
        }
    }
    entropy
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_feature_extraction_normal() {
        let requests = vec![
            RequestMetadata {
                uri: "/api/users".to_string(),
                status_code: 200,
                latency_ms: 50.0,
                user_agent: "Mozilla/5.0".to_string(),
            },
            RequestMetadata {
                uri: "/api/posts".to_string(),
                status_code: 200,
                latency_ms: 45.0,
                user_agent: "Mozilla/5.0".to_string(),
            },
            RequestMetadata {
                uri: "/api/comments".to_string(),
                status_code: 200,
                latency_ms: 55.0,
                user_agent: "Chrome/90.0".to_string(),
            },
        ];

        let features = TrafficFeatures::from_telemetry(
            "192.168.1.100".to_string(),
            &requests,
            1.0, // 1 second window
        );

        assert_eq!(features.request_count, 3);
        assert_eq!(features.error_ratio, 0.0); // No errors
        assert_eq!(features.distinct_uris, 3); // 3 different URIs
        assert!(features.uri_entropy > 0.0); // Should have entropy
        assert!(features.request_rate > 0.0);
    }

    #[test]
    fn test_feature_extraction_attack() {
        // Scanner hitting same endpoint repeatedly with errors
        let requests = vec![
            RequestMetadata {
                uri: "/admin".to_string(),
                status_code: 404,
                latency_ms: 10.0,
                user_agent: "BadBot/1.0".to_string(),
            },
            RequestMetadata {
                uri: "/admin".to_string(),
                status_code: 404,
                latency_ms: 12.0,
                user_agent: "BadBot/1.0".to_string(),
            },
            RequestMetadata {
                uri: "/admin".to_string(),
                status_code: 404,
                latency_ms: 11.0,
                user_agent: "BadBot/1.0".to_string(),
            },
        ];

        let features = TrafficFeatures::from_telemetry(
            "1.2.3.4".to_string(),
            &requests,
            1.0,
        );

        assert_eq!(features.request_count, 3);
        assert_eq!(features.error_ratio, 1.0); // All errors!
        assert_eq!(features.distinct_uris, 1); // Same URI repeated
        assert_eq!(features.uri_entropy, 0.0); // No entropy (all same)
        assert!(features.user_agent_diversity < 0.5); // Same UA
    }

    #[test]
    fn test_entropy_calculation() {
        let mut counts = HashMap::new();
        counts.insert("a".to_string(), 1);
        counts.insert("b".to_string(), 1);
        counts.insert("c".to_string(), 1);
        counts.insert("d".to_string(), 1);

        // Uniform distribution should have max entropy
        let entropy = calculate_entropy(&counts, 4);
        assert!((entropy - 2.0).abs() < 0.01); // log2(4) = 2.0

        // Single item should have zero entropy
        let mut single = HashMap::new();
        single.insert("only".to_string(), 10);
        let entropy = calculate_entropy(&single, 10);
        assert_eq!(entropy, 0.0);
    }

    #[test]
    fn test_to_vector() {
        let features = TrafficFeatures {
            ip: "test".to_string(),
            request_count: 100,
            error_ratio: 0.1,
            distinct_uris: 10,
            uri_entropy: 2.5,
            avg_latency_ms: 50.0,
            latency_stddev_ms: 5.0,
            request_rate: 10.0,
            user_agent_diversity: 0.8,
            timestamp: chrono::Utc::now(),
        };

        let vector = features.to_vector();
        assert_eq!(vector.len(), TrafficFeatures::FEATURE_COUNT);
        assert_eq!(vector[0], 100.0); // request_count
        assert_eq!(vector[1], 0.1); // error_ratio
        assert_eq!(vector[7], 0.8); // user_agent_diversity
    }

    #[test]
    fn test_feature_names() {
        let names = TrafficFeatures::feature_names();
        assert_eq!(names.len(), TrafficFeatures::FEATURE_COUNT);
        assert_eq!(names[0], "request_count");
        assert_eq!(names[7], "user_agent_diversity");
    }
}
