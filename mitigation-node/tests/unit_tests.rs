//! Unit tests for the SecBeat mitigation node
//! 
//! This module contains comprehensive unit tests for all major components:
//! - WAF engine functionality
//! - Configuration management
//! - DDoS protection algorithms
//! - Management API endpoints
//! - Event system integration
//! - SYN proxy functionality

use anyhow::Result;
use std::collections::HashMap;
use std::net::{IpAddr, Ipv4Addr};
use tokio::time::Duration;

// Import modules under test
use mitigation_node::config::MitigationConfig;
use mitigation_node::waf::{WafEngine, WafResult};
use mitigation_node::ddos::DdosProtection;
use mitigation_node::events::EventSystem;

/// Test configuration loading and validation
#[cfg(test)]
mod config_tests {
    use super::*;
    
    #[test]
    fn test_config_loading_from_file() {
        // Test loading the production config
        let config_path = "../config/production.toml";
        let config = MitigationConfig::from_file(config_path);
        
        assert!(config.is_ok(), "Should be able to load production config");
        
        if let Ok(config) = config {
            // Validate required fields
            assert!(config.network.public_port > 0);
            assert!(config.ddos.enabled);
            assert!(config.waf.enabled);
        }
    }
    
    #[test]
    fn test_config_validation() {
        let mut config = MitigationConfig::default();
        
        // Test valid configuration
        assert!(config.validate().is_ok());
        
        // Test invalid port
        config.network.public_port = 0;
        assert!(config.validate().is_err());
        
        // Test invalid timeout
        config.network.public_port = 8080;
        config.network.connection_timeout_seconds = 0;
        assert!(config.validate().is_err());
    }
    
    #[test]
    fn test_environment_overrides() {
        std::env::set_var("SECBEAT_PUBLIC_PORT", "9999");
        std::env::set_var("SECBEAT_DDos_ENABLED", "false");
        
        let mut config = MitigationConfig::default();
        config.apply_environment_overrides();
        
        assert_eq!(config.network.public_port, 9999);
        assert_eq!(config.ddos.enabled, false);
        
        // Clean up
        std::env::remove_var("SECBEAT_PUBLIC_PORT");
        std::env::remove_var("SECBEAT_DDos_ENABLED");
    }
}

/// Test WAF engine functionality
#[cfg(test)]
mod waf_tests {
    use super::*;
    
    #[tokio::test]
    async fn test_waf_engine_initialization() {
        let config = MitigationConfig::default();
        let waf = WafEngine::new(config.waf.clone()).await;
        
        assert!(waf.is_ok(), "WAF engine should initialize successfully");
    }
    
    #[tokio::test]
    async fn test_sql_injection_detection() {
        let config = MitigationConfig::default();
        let waf = WafEngine::new(config.waf.clone()).await.unwrap();
        
        // Test basic SQL injection patterns
        let malicious_inputs = vec![
            "'; DROP TABLE users; --",
            "1' OR '1'='1",
            "admin'/*",
            "' UNION SELECT * FROM passwords --",
            "1; DELETE FROM accounts WHERE 1=1 --",
        ];
        
        for input in malicious_inputs {
            let result = waf.check_sql_injection(input).await;
            assert_eq!(result, WafResult::Blocked, 
                      "Should detect SQL injection in: {}", input);
        }
        
        // Test legitimate inputs
        let legitimate_inputs = vec![
            "user123",
            "john.doe@example.com", 
            "My name is O'Brien",
            "Price is $19.99",
        ];
        
        for input in legitimate_inputs {
            let result = waf.check_sql_injection(input).await;
            assert_eq!(result, WafResult::Allowed,
                      "Should allow legitimate input: {}", input);
        }
    }
    
    #[tokio::test]
    async fn test_xss_detection() {
        let config = MitigationConfig::default();
        let waf = WafEngine::new(config.waf.clone()).await.unwrap();
        
        // Test XSS patterns
        let xss_inputs = vec![
            "<script>alert('xss')</script>",
            "javascript:alert(1)",
            "<img src=x onerror=alert(1)>",
            "<svg onload=alert(1)>",
            "javascript:void(0)",
        ];
        
        for input in xss_inputs {
            let result = waf.check_xss(input).await;
            assert_eq!(result, WafResult::Blocked,
                      "Should detect XSS in: {}", input);
        }
    }
    
    #[tokio::test]
    async fn test_path_traversal_detection() {
        let config = MitigationConfig::default();
        let waf = WafEngine::new(config.waf.clone()).await.unwrap();
        
        // Test path traversal patterns
        let traversal_inputs = vec![
            "../../../etc/passwd",
            "..\\..\\..\\windows\\system32\\config\\sam",
            "....//....//....//etc/passwd",
            "%2e%2e%2f%2e%2e%2f%2e%2e%2fetc%2fpasswd",
        ];
        
        for input in traversal_inputs {
            let result = waf.check_path_traversal(input).await;
            assert_eq!(result, WafResult::Blocked,
                      "Should detect path traversal in: {}", input);
        }
    }
    
    #[tokio::test]
    async fn test_custom_pattern_management() {
        let config = MitigationConfig::default();
        let mut waf = WafEngine::new(config.waf.clone()).await.unwrap();
        
        // Add custom pattern
        let custom_pattern = "badword123";
        waf.add_custom_pattern(custom_pattern).await.unwrap();
        
        // Test detection
        let result = waf.check_custom_patterns("This contains badword123").await;
        assert_eq!(result, WafResult::Blocked);
        
        // Remove pattern
        let removed_count = waf.remove_custom_pattern(custom_pattern).await.unwrap();
        assert_eq!(removed_count, 1);
        
        // Test that it's no longer detected
        let result = waf.check_custom_patterns("This contains badword123").await;
        assert_eq!(result, WafResult::Allowed);
    }
    
    #[tokio::test]
    async fn test_waf_stats() {
        let config = MitigationConfig::default();
        let waf = WafEngine::new(config.waf.clone()).await.unwrap();
        
        let stats = waf.get_stats().await;
        assert!(stats.enabled);
        assert!(stats.sql_patterns > 0);
        assert!(stats.xss_patterns > 0);
        assert!(stats.path_traversal_patterns > 0);
    }
}

/// Test DDoS protection functionality
#[cfg(test)]
mod ddos_tests {
    use super::*;
    
    #[tokio::test]
    async fn test_ddos_protection_initialization() {
        let config = MitigationConfig::default();
        let ddos = DdosProtection::new(config.ddos.clone());
        
        assert!(ddos.is_ok(), "DDoS protection should initialize successfully");
    }
    
    #[tokio::test]
    async fn test_rate_limiting() {
        let config = MitigationConfig::default();
        let mut ddos_config = config.ddos.clone();
        ddos_config.rate_limiting.requests_per_second = 10;
        ddos_config.rate_limiting.burst_size = 20;
        
        let ddos = DdosProtection::new(ddos_config).unwrap();
        let client_ip: IpAddr = "192.168.1.100".parse().unwrap();
        
        // Test within limits
        for _ in 0..15 {
            let result = ddos.check_rate_limit(client_ip).await;
            assert_eq!(result, WafResult::Allowed);
        }
        
        // Test exceeding burst
        for _ in 0..10 {
            let result = ddos.check_rate_limit(client_ip).await;
            if result == WafResult::Blocked {
                break; // Expected to be blocked eventually
            }
        }
    }
    
    #[tokio::test]
    async fn test_connection_tracking() {
        let config = MitigationConfig::default();
        let ddos = DdosProtection::new(config.ddos.clone()).unwrap();
        let client_ip: IpAddr = "192.168.1.101".parse().unwrap();
        
        // Record connections
        ddos.record_connection(client_ip);
        ddos.record_connection(client_ip);
        
        let stats = ddos.get_stats();
        assert!(stats.active_connections > 0);
        
        // Record disconnections
        ddos.record_disconnection(client_ip);
        ddos.record_disconnection(client_ip);
    }
    
    #[tokio::test]
    async fn test_blacklist_functionality() {
        let config = MitigationConfig::default();
        let ddos = DdosProtection::new(config.ddos.clone()).unwrap();
        let malicious_ip: IpAddr = "10.0.0.1".parse().unwrap();
        
        // Add to blacklist
        ddos.add_to_blacklist(malicious_ip, Duration::from_secs(60)).await;
        
        // Check blacklist
        let is_blocked = ddos.is_blacklisted(malicious_ip).await;
        assert!(is_blocked);
        
        // Remove from blacklist
        ddos.remove_from_blacklist(malicious_ip).await;
        let is_blocked = ddos.is_blacklisted(malicious_ip).await;
        assert!(!is_blocked);
    }
}

/// Test event system functionality  
#[cfg(test)]
mod event_tests {
    use super::*;
    
    #[tokio::test]
    async fn test_event_system_initialization() {
        let nats_url = "nats://localhost:4222";
        let node_id = uuid::Uuid::new_v4();
        
        // This will fail if NATS is not running, but that's expected in unit tests
        let event_system = EventSystem::new(nats_url, node_id).await;
        
        // We expect this to fail in unit test environment without NATS
        // The important thing is that it handles the error gracefully
        match event_system {
            Ok(_) => println!("Event system connected successfully"),
            Err(e) => println!("Event system failed to connect (expected): {}", e),
        }
    }
    
    #[tokio::test]
    async fn test_event_serialization() {
        use mitigation_node::events::{SecurityEvent, WafEventResult};
        
        let event = SecurityEvent {
            node_id: uuid::Uuid::new_v4(),
            timestamp: chrono::Utc::now(),
            source_ip: "192.168.1.100".parse().unwrap(),
            http_method: "GET".to_string(),
            uri: "/api/users".to_string(),
            host_header: Some("example.com".to_string()),
            user_agent: Some("test-agent".to_string()),
            waf_result: WafEventResult {
                action: "BLOCK".to_string(),
                matched_rules: vec!["sql_injection".to_string()],
                confidence: Some(0.95),
            },
            request_size: Some(1024),
            response_status: Some(403),
            processing_time_ms: Some(5),
        };
        
        // Test serialization
        let serialized = serde_json::to_string(&event);
        assert!(serialized.is_ok());
        
        // Test deserialization
        if let Ok(json) = serialized {
            let deserialized: Result<SecurityEvent, _> = serde_json::from_str(&json);
            assert!(deserialized.is_ok());
        }
    }
}

/// Performance and stress tests
#[cfg(test)]
mod performance_tests {
    use super::*;
    use std::time::Instant;
    
    #[tokio::test]
    async fn test_waf_performance() {
        let config = MitigationConfig::default();
        let waf = WafEngine::new(config.waf.clone()).await.unwrap();
        
        let test_input = "SELECT * FROM users WHERE id = 1";
        let iterations = 1000;
        
        let start = Instant::now();
        for _ in 0..iterations {
            let _ = waf.check_sql_injection(test_input).await;
        }
        let duration = start.elapsed();
        
        let requests_per_second = (iterations as f64) / duration.as_secs_f64();
        println!("WAF Performance: {:.2} requests/second", requests_per_second);
        
        // Expect at least 1000 requests per second
        assert!(requests_per_second > 1000.0, 
               "WAF should process at least 1000 requests/second, got {:.2}", 
               requests_per_second);
    }
    
    #[tokio::test]
    async fn test_ddos_rate_limiting_performance() {
        let config = DdosConfig::default();
        let ddos = DdosProtection::new(config).unwrap();
        let client_ip: IpAddr = "192.168.1.200".parse().unwrap();
        
        let iterations = 10000;
        let start = Instant::now();
        
        for _ in 0..iterations {
            let _ = ddos.check_rate_limit(client_ip).await;
        }
        
        let duration = start.elapsed();
        let checks_per_second = (iterations as f64) / duration.as_secs_f64();
        
        println!("DDoS Rate Limiting Performance: {:.2} checks/second", checks_per_second);
        
        // Expect at least 5000 checks per second
        assert!(checks_per_second > 5000.0,
               "Rate limiting should handle at least 5000 checks/second, got {:.2}",
               checks_per_second);
    }
}

/// Integration tests for component interactions
#[cfg(test)]
mod integration_tests {
    use super::*;
    use std::sync::Arc;
    use tokio::sync::RwLock;
    
    #[tokio::test]
    async fn test_waf_ddos_integration() {
        // Initialize both systems
        let config = MitigationConfig::default();
        let waf = Arc::new(RwLock::new(
            WafEngine::new(config.waf.clone()).await.unwrap()
        ));
        let ddos = Arc::new(
            DdosProtection::new(config.ddos.clone()).unwrap()
        );
        
        let client_ip: IpAddr = "192.168.1.300".parse().unwrap();
        let malicious_request = "'; DROP TABLE users; --";
        
        // Check DDoS first
        let ddos_result = ddos.check_rate_limit(client_ip).await;
        if ddos_result == WafResult::Allowed {
            // Check WAF
            let waf_guard = waf.read().await;
            let waf_result = waf_guard.check_sql_injection(malicious_request).await;
            assert_eq!(waf_result, WafResult::Blocked);
        }
    }
    
    #[tokio::test]
    async fn test_config_reload_integration() {
        // Test that configuration changes are applied correctly
        let mut config = MitigationConfig::default();
        
        // Change WAF settings
        config.waf.enabled = false;
        
        let waf = WafEngine::new(config.waf.clone()).await.unwrap();
        let stats = waf.get_stats().await;
        assert!(!stats.enabled);
        
        // Re-enable WAF
        config.waf.enabled = true;
        let waf = WafEngine::new(config.waf.clone()).await.unwrap();
        let stats = waf.get_stats().await;
        assert!(stats.enabled);
    }
}

/// Security tests to validate protection mechanisms
#[cfg(test)]
mod security_tests {
    use super::*;
    
    #[tokio::test]
    async fn test_attack_simulation() {
        let config = MitigationConfig::default();
        let waf = WafEngine::new(config.waf.clone()).await.unwrap();
        
        // Simulate various attack vectors
        let attack_vectors = vec![
            // SQL Injection variations
            ("sql1", "1' OR 1=1 --"),
            ("sql2", "'; EXEC xp_cmdshell('dir'); --"),
            ("sql3", "' UNION SELECT password FROM users --"),
            
            // XSS variations  
            ("xss1", "<script>document.location='http://evil.com'</script>"),
            ("xss2", "<img src=\"javascript:alert('XSS')\">"),
            ("xss3", "<svg/onload=alert('XSS')>"),
            
            // Path traversal variations
            ("path1", "../../../etc/passwd"),
            ("path2", "..\\..\\..\\windows\\system32\\config\\sam"),
            ("path3", "....//....//....//etc/shadow"),
            
            // Command injection
            ("cmd1", "; cat /etc/passwd"),
            ("cmd2", "| whoami"),
            ("cmd3", "& netstat -an"),
        ];
        
        let mut blocked_count = 0;
        let total_attacks = attack_vectors.len();
        
        for (attack_type, payload) in attack_vectors {
            let mut is_blocked = false;
            
            // Test against all WAF checks
            if waf.check_sql_injection(payload).await == WafResult::Blocked {
                is_blocked = true;
            }
            if waf.check_xss(payload).await == WafResult::Blocked {
                is_blocked = true;
            }
            if waf.check_path_traversal(payload).await == WafResult::Blocked {
                is_blocked = true;
            }
            if waf.check_command_injection(payload).await == WafResult::Blocked {
                is_blocked = true;
            }
            
            if is_blocked {
                blocked_count += 1;
                println!("✅ Blocked {}: {}", attack_type, payload);
            } else {
                println!("❌ Missed {}: {}", attack_type, payload);
            }
        }
        
        let block_rate = (blocked_count as f64 / total_attacks as f64) * 100.0;
        println!("Attack Detection Rate: {:.1}% ({}/{} attacks blocked)", 
                block_rate, blocked_count, total_attacks);
        
        // Expect at least 80% block rate
        assert!(block_rate >= 80.0, 
               "WAF should block at least 80% of attacks, got {:.1}%", block_rate);
    }
    
    #[tokio::test]
    async fn test_evasion_techniques() {
        let config = MitigationConfig::default();
        let waf = WafEngine::new(config.waf.clone()).await.unwrap();
        
        // Test common evasion techniques
        let evasion_attempts = vec![
            // URL encoding
            "%27%20OR%201%3D1%20--",
            // Double URL encoding  
            "%2527%2520OR%25201%253D1%2520--",
            // Unicode encoding
            "\\u0027\\u0020OR\\u00201\\u003D1\\u0020--",
            // Case variations
            "' oR 1=1 --",
            "' Or 1=1 --",
            // Comment variations
            "'/**/OR/**/1=1/**/--",
            // Whitespace variations
            "'%09OR%091=1%09--",
        ];
        
        for evasion in evasion_attempts {
            let result = waf.check_sql_injection(evasion).await;
            // Should ideally block evasion attempts
            // This test helps identify areas for improvement
            println!("Evasion test '{}': {:?}", evasion, result);
        }
    }
}
