// Rule Generator - Converts Anomaly Detection results into WASM rules
//
// This module closes the loop between ML detection and WASM execution:
// 1. ML Expert detects anomaly pattern (e.g., User-Agent + URI combo)
// 2. Rule Generator creates WASM config to block that pattern
// 3. WASM rule is deployed fleet-wide via NATS
//
// Strategy: Instead of compiling WASM on-the-fly, we use a pre-compiled
// "Universal WAF" module with data-driven rules via JSON configuration.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use tracing::{debug, info};

use crate::experts::anomaly_ml::AnomalyScore;

// ============================================================================
// WASM Configuration Schema (matches universal-waf module)
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WafConfig {
    pub rules: Vec<WafRule>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WafRule {
    pub id: String,
    pub field: String,      // "URI", "Method", "Header:X", "SourceIP"
    pub pattern: String,    // Pattern to match (glob or regex)
    pub action: String,     // "Block", "Allow", "Log", "RateLimit"
}

// ============================================================================
// Deployment Package
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WasmDeployment {
    /// Unique deployment identifier
    pub deployment_id: String,
    
    /// Module name (e.g., "universal-waf")
    pub module_name: String,
    
    /// WASM bytecode (base64 encoded for transmission)
    pub bytecode_base64: String,
    
    /// JSON configuration for the module
    pub config_json: String,
    
    /// Human-readable description
    pub description: String,
    
    /// Timestamp
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

// ============================================================================
// Rule Generator
// ============================================================================

pub struct RuleGenerator {
    /// Pre-loaded Universal WAF bytecode
    universal_waf_bytecode: Vec<u8>,
    
    /// Rule ID counter
    rule_counter: std::sync::atomic::AtomicU64,
}

impl RuleGenerator {
    /// Create a new Rule Generator
    ///
    /// # Arguments
    /// * `universal_waf_path` - Path to universal-waf.wasm file
    pub fn new(universal_waf_path: &str) -> Result<Self> {
        let bytecode = std::fs::read(universal_waf_path)
            .context("Failed to load universal-waf.wasm")?;
        
        info!(
            "Rule Generator initialized with universal-waf ({} bytes)",
            bytecode.len()
        );

        Ok(Self {
            universal_waf_bytecode: bytecode,
            rule_counter: std::sync::atomic::AtomicU64::new(0),
        })
    }

    /// Generate a WASM deployment from an anomaly detection result
    ///
    /// # Arguments
    /// * `anomaly` - Anomaly detection result with features
    ///
    /// # Returns
    /// WasmDeployment ready to be distributed via NATS
    pub fn generate_from_anomaly(&self, anomaly: &AnomalyScore) -> Result<WasmDeployment> {
        let rules = self.analyze_anomaly_pattern(anomaly);
        
        let config = WafConfig { rules };
        let config_json = serde_json::to_string(&config)
            .context("Failed to serialize WAF config")?;

        let deployment_id = format!(
            "anomaly-{}-{}",
            anomaly.ip.replace('.', "-"),
            chrono::Utc::now().timestamp()
        );

        let deployment = WasmDeployment {
            deployment_id,
            module_name: "universal-waf".to_string(),
            bytecode_base64: base64::encode(&self.universal_waf_bytecode),
            config_json,
            description: format!(
                "Auto-generated rule for anomaly from {}: score={:.2}",
                anomaly.ip, anomaly.score
            ),
            timestamp: chrono::Utc::now(),
        };

        info!(
            "Generated WASM deployment: {} rules for IP {}",
            config.rules.len(),
            anomaly.ip
        );

        Ok(deployment)
    }

    /// Analyze anomaly pattern and generate WAF rules
    fn analyze_anomaly_pattern(&self, anomaly: &AnomalyScore) -> Vec<WafRule> {
        let features = &anomaly.features;
        let mut rules = Vec::new();

        // Rule 1: High error ratio = Block IP temporarily
        if features.error_ratio > 0.7 {
            rules.push(WafRule {
                id: self.next_rule_id(),
                field: "SourceIP".to_string(),
                pattern: format!("^{}$", anomaly.ip),
                action: "Block".to_string(),
            });
        }

        // Rule 2: Low URI entropy = Scanning behavior
        // Block repetitive URI patterns
        if features.uri_entropy < 1.0 && features.distinct_uris < 3 {
            // We don't know the exact URI from features, so block suspicious patterns
            rules.push(WafRule {
                id: self.next_rule_id(),
                field: "URI".to_string(),
                pattern: "^/admin".to_string(),
                action: "Block".to_string(),
            });

            rules.push(WafRule {
                id: self.next_rule_id(),
                field: "URI".to_string(),
                pattern: ".env".to_string(),
                action: "Block".to_string(),
            });

            rules.push(WafRule {
                id: self.next_rule_id(),
                field: "URI".to_string(),
                pattern: "wp-admin".to_string(),
                action: "Block".to_string(),
            });
        }

        // Rule 3: Low user agent diversity = Bot behavior
        // Block known scanner user agents
        if features.user_agent_diversity < 0.3 {
            rules.push(WafRule {
                id: self.next_rule_id(),
                field: "Header:User-Agent".to_string(),
                pattern: "*bot*".to_string(),
                action: "Block".to_string(),
            });

            rules.push(WafRule {
                id: self.next_rule_id(),
                field: "Header:User-Agent".to_string(),
                pattern: "*scanner*".to_string(),
                action: "Block".to_string(),
            });

            rules.push(WafRule {
                id: self.next_rule_id(),
                field: "Header:User-Agent".to_string(),
                pattern: "*sqlmap*".to_string(),
                action: "Block".to_string(),
            });
        }

        // Rule 4: Very high request rate = DDoS
        if features.request_rate > 100.0 {
            rules.push(WafRule {
                id: self.next_rule_id(),
                field: "SourceIP".to_string(),
                pattern: format!("^{}$", anomaly.ip),
                action: "RateLimit".to_string(),
            });
        }

        // If no specific rules generated, create a generic block rule
        if rules.is_empty() {
            rules.push(WafRule {
                id: self.next_rule_id(),
                field: "SourceIP".to_string(),
                pattern: format!("^{}$", anomaly.ip),
                action: "Log".to_string(), // Log for further analysis
            });
        }

        debug!(
            "Generated {} rules from anomaly: error_ratio={:.2}, uri_entropy={:.2}, ua_diversity={:.2}",
            rules.len(),
            features.error_ratio,
            features.uri_entropy,
            features.user_agent_diversity
        );

        rules
    }

    /// Generate rule ID
    fn next_rule_id(&self) -> String {
        let id = self.rule_counter.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        format!("rule-{:06}", id)
    }

    /// Generate a manual rule (for testing or manual intervention)
    pub fn generate_manual_rule(
        &self,
        field: String,
        pattern: String,
        action: String,
        description: String,
    ) -> Result<WasmDeployment> {
        let rule = WafRule {
            id: self.next_rule_id(),
            field,
            pattern,
            action,
        };

        let config = WafConfig { rules: vec![rule] };
        let config_json = serde_json::to_string(&config)
            .context("Failed to serialize WAF config")?;

        let deployment = WasmDeployment {
            deployment_id: format!("manual-{}", chrono::Utc::now().timestamp()),
            module_name: "universal-waf".to_string(),
            bytecode_base64: base64::encode(&self.universal_waf_bytecode),
            config_json,
            description,
            timestamp: chrono::Utc::now(),
        };

        Ok(deployment)
    }

    /// Get statistics
    pub fn get_stats(&self) -> GeneratorStats {
        GeneratorStats {
            total_rules_generated: self.rule_counter.load(std::sync::atomic::Ordering::SeqCst),
            wasm_module_size: self.universal_waf_bytecode.len(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeneratorStats {
    pub total_rules_generated: u64,
    pub wasm_module_size: usize,
}

// ============================================================================
// Base64 Encoding Helper (simple implementation)
// ============================================================================

mod base64 {
    const CHARSET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

    pub fn encode(input: &[u8]) -> String {
        let mut output = String::new();
        let mut i = 0;

        while i < input.len() {
            let b1 = input[i];
            let b2 = input.get(i + 1).copied().unwrap_or(0);
            let b3 = input.get(i + 2).copied().unwrap_or(0);

            let c1 = (b1 >> 2) & 0x3F;
            let c2 = ((b1 & 0x03) << 4) | ((b2 >> 4) & 0x0F);
            let c3 = ((b2 & 0x0F) << 2) | ((b3 >> 6) & 0x03);
            let c4 = b3 & 0x3F;

            output.push(CHARSET[c1 as usize] as char);
            output.push(CHARSET[c2 as usize] as char);
            output.push(if i + 1 < input.len() { CHARSET[c3 as usize] as char } else { '=' });
            output.push(if i + 2 < input.len() { CHARSET[c4 as usize] as char } else { '=' });

            i += 3;
        }

        output
    }

    #[allow(dead_code)]
    pub fn decode(input: &str) -> Result<Vec<u8>, &'static str> {
        let mut output = Vec::new();
        let chars: Vec<char> = input.chars().filter(|&c| c != '\n' && c != '\r').collect();
        let mut i = 0;

        while i < chars.len() {
            let c1 = decode_char(chars[i])?;
            let c2 = decode_char(chars.get(i + 1).copied().unwrap_or('='))?;
            let c3 = decode_char(chars.get(i + 2).copied().unwrap_or('='))?;
            let c4 = decode_char(chars.get(i + 3).copied().unwrap_or('='))?;

            output.push(((c1 << 2) | (c2 >> 4)) as u8);
            if chars.get(i + 2) != Some(&'=') {
                output.push(((c2 << 4) | (c3 >> 2)) as u8);
            }
            if chars.get(i + 3) != Some(&'=') {
                output.push(((c3 << 6) | c4) as u8);
            }

            i += 4;
        }

        Ok(output)
    }

    fn decode_char(c: char) -> Result<u8, &'static str> {
        match c {
            'A'..='Z' => Ok((c as u8) - b'A'),
            'a'..='z' => Ok((c as u8) - b'a' + 26),
            '0'..='9' => Ok((c as u8) - b'0' + 52),
            '+' => Ok(62),
            '/' => Ok(63),
            '=' => Ok(0),
            _ => Err("Invalid base64 character"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::experts::features::TrafficFeatures;

    #[test]
    fn test_base64_encoding() {
        let input = b"Hello, World!";
        let encoded = base64::encode(input);
        assert_eq!(encoded, "SGVsbG8sIFdvcmxkIQ==");
    }

    #[test]
    fn test_rule_generation_logic() {
        // Simulate high error ratio anomaly
        let anomaly = AnomalyScore {
            ip: "1.2.3.4".to_string(),
            score: 0.9,
            is_anomaly: true,
            features: TrafficFeatures {
                ip: "1.2.3.4".to_string(),
                request_count: 100,
                error_ratio: 0.95, // Very high errors
                distinct_uris: 1,
                uri_entropy: 0.0,
                avg_latency_ms: 10.0,
                latency_stddev_ms: 1.0,
                request_rate: 50.0,
                user_agent_diversity: 0.1,
                timestamp: chrono::Utc::now(),
            },
            timestamp: chrono::Utc::now(),
        };

        // Create generator with dummy bytecode
        let generator = RuleGenerator {
            universal_waf_bytecode: vec![0u8; 100],
            rule_counter: std::sync::atomic::AtomicU64::new(0),
        };

        let rules = generator.analyze_anomaly_pattern(&anomaly);
        
        // Should generate multiple rules
        assert!(!rules.is_empty());
        
        // Should block the IP
        assert!(rules.iter().any(|r| r.field == "SourceIP"));
    }
}
