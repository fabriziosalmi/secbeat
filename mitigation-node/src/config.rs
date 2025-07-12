use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use std::time::Duration;

/// Main configuration for the mitigation node
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MitigationConfig {
    /// Network configuration
    pub network: NetworkConfig,
    /// DDoS protection settings
    pub ddos: DdosConfig,
    /// WAF (Web Application Firewall) settings
    pub waf: WafConfig,
    /// SYN Proxy configuration
    pub syn_proxy: SynProxyConfig,
    /// Metrics and monitoring
    pub metrics: MetricsConfig,
    /// Logging configuration
    pub logging: LoggingConfig,
    /// Orchestrator integration
    pub orchestrator: OrchestratorConfig,
    /// NATS messaging configuration
    pub nats: NatsConfig,
    /// Management API configuration
    pub management: ManagementApiConfig,
}

/// Network configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkConfig {
    /// Address to listen on (public-facing, vmbr1 interface)
    pub listen_addr: SocketAddr,
    /// Backend server address (internal, vmbr2 network)
    pub backend_addr: SocketAddr,
    /// Connection timeout in seconds
    pub connection_timeout: u64,
    /// Buffer size for data transfer
    pub buffer_size: usize,
    /// TLS configuration
    pub tls: TlsConfig,
}

/// TLS configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TlsConfig {
    /// Enable TLS termination
    pub enabled: bool,
    /// Path to TLS certificate file
    pub cert_path: String,
    /// Path to TLS private key file
    pub key_path: String,
    /// TLS listen port (usually 443 or 8443)
    pub tls_port: u16,
}

/// DDoS protection configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DdosConfig {
    /// Enable DDoS protection
    pub enabled: bool,
    /// Rate limiting configuration
    pub rate_limiting: RateLimitConfig,
    /// Connection limits
    pub connection_limits: ConnectionLimitsConfig,
    /// Blacklist configuration
    pub blacklist: BlacklistConfig,
}

/// Rate limiting configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RateLimitConfig {
    /// Maximum requests per IP per second
    pub requests_per_second: u32,
    /// Burst allowance (temporary spike tolerance)
    pub burst_size: u32,
    /// Time window for rate limiting in seconds
    pub window_seconds: u64,
    /// Penalty duration for rate limit violations (seconds)
    pub penalty_duration: u64,
}

/// Connection limits configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectionLimitsConfig {
    /// Maximum concurrent connections per IP
    pub max_connections_per_ip: u32,
    /// Maximum total concurrent connections
    pub max_total_connections: u32,
    /// Connection establishment rate limit (new connections per second)
    pub max_new_connections_per_second: u32,
}

/// Blacklist configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlacklistConfig {
    /// Enable automatic blacklisting
    pub enabled: bool,
    /// Threshold for automatic blacklisting (violations per minute)
    pub violation_threshold: u32,
    /// Duration to blacklist IPs (seconds)
    pub blacklist_duration: u64,
    /// Manual blacklist (CIDR ranges)
    pub manual_blacklist: Vec<String>,
    /// Whitelist (CIDR ranges that bypass all protection)
    pub whitelist: Vec<String>,
}

/// WAF configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WafConfig {
    /// Enable WAF
    pub enabled: bool,
    /// HTTP inspection settings
    pub http_inspection: HttpInspectionConfig,
    /// Attack pattern detection
    pub attack_patterns: AttackPatternsConfig,
}

/// HTTP inspection configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HttpInspectionConfig {
    /// Enable HTTP header inspection
    pub inspect_headers: bool,
    /// Enable HTTP body inspection
    pub inspect_body: bool,
    /// Maximum body size to inspect (bytes)
    pub max_body_size: usize,
    /// Enable URL inspection
    pub inspect_url: bool,
    /// Block invalid HTTP requests
    pub block_invalid_http: bool,
}

/// Attack pattern configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AttackPatternsConfig {
    /// Enable SQL injection detection
    pub sql_injection: bool,
    /// Enable XSS detection
    pub xss_detection: bool,
    /// Enable path traversal detection
    pub path_traversal: bool,
    /// Enable command injection detection
    pub command_injection: bool,
    /// Custom attack patterns (regex)
    pub custom_patterns: Vec<String>,
}

/// SYN Proxy configuration for Layer 4 protection
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SynProxyConfig {
    /// Enable SYN proxy mode
    pub enabled: bool,
    /// Secret key for SYN cookie generation
    pub syn_cookie_key: String,
    /// Listening port for raw socket (should match network.listen_addr port)
    pub listen_port: u16,
    /// Maximum time to wait for ACK after SYN-ACK (milliseconds)
    pub handshake_timeout_ms: u64,
    /// Maximum number of pending handshakes to track
    pub max_pending_handshakes: usize,
}

/// Metrics configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricsConfig {
    /// Metrics server address
    pub listen_addr: SocketAddr,
    /// Enable metrics collection
    pub enabled: bool,
    /// Metrics update interval (seconds)
    pub update_interval: u64,
}

/// Logging configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoggingConfig {
    /// Log level (trace, debug, info, warn, error)
    pub level: String,
    /// Enable JSON logging
    pub json_format: bool,
    /// Log file path (optional)
    pub file_path: Option<String>,
}

/// Orchestrator integration configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrchestratorConfig {
    /// Enable orchestrator integration
    pub enabled: bool,
    /// Orchestrator server base URL
    pub server_url: String,
    /// Node identifier (if empty, will be auto-generated)
    pub node_id: Option<String>,
    /// Node registration configuration
    pub registration: RegistrationConfig,
    /// Heartbeat configuration
    pub heartbeat: HeartbeatConfig,
}

/// Node registration configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegistrationConfig {
    /// Registration retry interval (seconds)
    pub retry_interval: u64,
    /// Maximum registration attempts
    pub max_retries: u32,
    /// Registration timeout (seconds)
    pub timeout: u64,
}

/// Heartbeat configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HeartbeatConfig {
    /// Heartbeat interval (seconds)
    pub interval: u64,
    /// Heartbeat timeout (seconds)
    pub timeout: u64,
    /// Maximum missed heartbeats before considering disconnected
    pub max_missed: u32,
}

/// NATS messaging configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NatsConfig {
    /// Enable NATS integration
    pub enabled: bool,
    /// NATS server URL
    pub server_url: String,
    /// Connection timeout (seconds)
    pub connect_timeout: u64,
    /// Maximum reconnection attempts
    pub max_reconnects: u32,
    /// Event publishing enabled
    pub publish_events: bool,
    /// Command consumption enabled
    pub consume_commands: bool,
}

/// Management API configuration for node control
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ManagementApiConfig {
    /// Listen address for management API
    pub listen_addr: String,
    /// Authentication token for secure commands
    pub auth_token: String,
    /// Grace period for shutdown (seconds)
    pub shutdown_grace_period: u64,
    /// Enable management API
    pub enabled: bool,
}

impl Default for MitigationConfig {
    fn default() -> Self {
        Self {
            network: NetworkConfig {
                listen_addr: "127.0.0.1:8443".parse().unwrap(),
                backend_addr: "127.0.0.1:8080".parse().unwrap(),
                connection_timeout: 30,
                buffer_size: 8192,
                tls: TlsConfig {
                    enabled: true,
                    cert_path: "certs/cert.pem".to_string(),
                    key_path: "certs/key.pem".to_string(),
                    tls_port: 8443,
                },
            },
            ddos: DdosConfig {
                enabled: true,
                rate_limiting: RateLimitConfig {
                    requests_per_second: 100,
                    burst_size: 50,
                    window_seconds: 60,
                    penalty_duration: 300, // 5 minutes
                },
                connection_limits: ConnectionLimitsConfig {
                    max_connections_per_ip: 10,
                    max_total_connections: 1000,
                    max_new_connections_per_second: 50,
                },
                blacklist: BlacklistConfig {
                    enabled: true,
                    violation_threshold: 10,
                    blacklist_duration: 3600, // 1 hour
                    manual_blacklist: vec![],
                    whitelist: vec!["127.0.0.0/8".to_string()], // Localhost whitelist
                },
            },
            waf: WafConfig {
                enabled: true,
                http_inspection: HttpInspectionConfig {
                    inspect_headers: true,
                    inspect_body: true,
                    max_body_size: 1_048_576, // 1MB
                    inspect_url: true,
                    block_invalid_http: true,
                },
                attack_patterns: AttackPatternsConfig {
                    sql_injection: true,
                    xss_detection: true,
                    path_traversal: true,
                    command_injection: true,
                    custom_patterns: vec![],
                },
            },
            syn_proxy: SynProxyConfig {
                enabled: true,
                syn_cookie_key: "change-this-to-a-random-64-char-hex-string-in-production".to_string(),
                listen_port: 8443,
                handshake_timeout_ms: 5000, // 5 seconds
                max_pending_handshakes: 10000,
            },
            metrics: MetricsConfig {
                listen_addr: "127.0.0.1:9090".parse().unwrap(),
                enabled: true,
                update_interval: 5,
            },
            logging: LoggingConfig {
                level: "info".to_string(),
                json_format: false,
                file_path: None,
            },
            orchestrator: OrchestratorConfig {
                enabled: true,
                server_url: "http://127.0.0.1:3030".to_string(),
                node_id: None, // Auto-generate UUID
                registration: RegistrationConfig {
                    retry_interval: 10,
                    max_retries: 5,
                    timeout: 30,
                },
                heartbeat: HeartbeatConfig {
                    interval: 30,
                    timeout: 10,
                    max_missed: 3,
                },
            },
            nats: NatsConfig {
                enabled: true,
                server_url: "nats://127.0.0.1:4222".to_string(),
                connect_timeout: 10,
                max_reconnects: 10,
                publish_events: true,
                consume_commands: true,
            },
            management: ManagementApiConfig {
                listen_addr: "0.0.0.0:9999".to_string(),
                auth_token: "secure-management-token-change-in-production".to_string(),
                shutdown_grace_period: 60,
                enabled: true,
            },
        }
    }
}

impl MitigationConfig {
    /// Load configuration from file
    pub fn from_file(path: &str) -> Result<Self, config::ConfigError> {
        let settings = config::Config::builder()
            .add_source(config::File::with_name(path))
            .add_source(config::Environment::with_prefix("MITIGATION"))
            .build()?;
        
        settings.try_deserialize()
    }

    /// Save configuration to file
    pub fn save_to_file(&self, path: &str) -> Result<(), Box<dyn std::error::Error>> {
        let toml_string = toml::to_string_pretty(self)?;
        std::fs::write(path, toml_string)?;
        Ok(())
    }

    /// Get rate limit duration
    pub fn rate_limit_duration(&self) -> Duration {
        Duration::from_secs(self.ddos.rate_limiting.window_seconds)
    }

    /// Get penalty duration
    pub fn penalty_duration(&self) -> Duration {
        Duration::from_secs(self.ddos.rate_limiting.penalty_duration)
    }

    /// Get blacklist duration
    pub fn blacklist_duration(&self) -> Duration {
        Duration::from_secs(self.ddos.blacklist.blacklist_duration)
    }

    /// Get connection timeout
    pub fn connection_timeout(&self) -> Duration {
        Duration::from_secs(self.network.connection_timeout)
    }
}
