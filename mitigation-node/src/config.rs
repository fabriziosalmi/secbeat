use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use tracing::info;

/// Main configuration for the mitigation node - unified platform config
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MitigationConfig {
    /// Platform-wide configuration
    pub platform: PlatformConfig,
    /// Network configuration
    pub network: NetworkConfig,
    /// DDoS protection settings
    pub ddos: DdosConfig,
    /// WAF (Web Application Firewall) settings
    pub waf: WafConfig,
    /// Orchestrator integration
    pub orchestrator: OrchestratorConfig,
    /// NATS messaging configuration
    pub nats: NatsConfig,
    /// Metrics and monitoring
    pub metrics: MetricsConfig,
    /// Logging configuration
    pub logging: LoggingConfig,
    /// Management API configuration
    pub management: ManagementApiConfig,
    /// Health check configuration
    pub health: HealthConfig,
    /// Debug configuration
    pub debug: DebugConfig,
}

/// Platform-wide configuration and feature toggles
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlatformConfig {
    /// Deployment environment (development, staging, production)
    pub environment: String,
    /// Unique deployment identifier
    pub deployment_id: String,
    /// Regional identifier
    pub region: String,
    /// Enabled platform features
    pub features: Vec<String>,
    /// Operation mode: tcp, syn, l7, auto
    pub mode: Option<String>,
}

/// Backend server configuration for load balancing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackendServer {
    pub address: String,
    pub weight: u32,
    pub health_check: bool,
}

/// Network configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkConfig {
    /// Public interface address (0.0.0.0 for all interfaces)
    pub public_interface: String,
    /// Public port
    pub public_port: u16,
    /// Backend interface address
    pub backend_interface: String,
    /// Backend port
    pub backend_port: u16,
    /// Maximum concurrent connections
    pub max_connections: u32,
    /// Connection timeout in seconds
    pub connection_timeout_seconds: u64,
    /// Keep-alive timeout in seconds
    pub keep_alive_timeout_seconds: u64,
    /// Buffer size for data transfer
    pub buffer_size: usize,
    /// Load balancing mode
    pub load_balance_mode: Option<String>,
    /// Backend servers for load balancing
    pub backend_servers: Vec<BackendServer>,
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
    /// Path to CA certificate file (optional)
    pub ca_path: Option<String>,
    /// Minimum TLS version
    pub min_version: String,
    /// Maximum TLS version
    pub max_version: String,
    /// Supported cipher suites
    pub cipher_suites: Option<Vec<String>>,
    /// ALPN protocols
    pub alpn_protocols: Vec<String>,
    /// Require client certificates
    pub client_cert_required: bool,
    /// Client CA certificate path
    pub client_cert_ca_path: Option<String>,
    /// Enable OCSP stapling
    pub ocsp_stapling: bool,
}

/// DDoS protection configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DdosConfig {
    /// Enable DDoS protection
    pub enabled: bool,
    /// Detection threshold multiplier
    pub detection_threshold_multiplier: Option<f64>,
    /// Mitigation duration in seconds
    pub mitigation_duration_seconds: Option<u64>,
    /// Learning period in seconds
    pub learning_period_seconds: Option<u64>,
    /// Rate limiting configuration
    pub rate_limiting: RateLimitConfig,
    /// Connection limits
    pub connection_limits: ConnectionLimitsConfig,
    /// SYN proxy configuration
    pub syn_proxy: SynProxyConfig,
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
    pub penalty_duration_seconds: Option<u64>,
    /// Global rate limiting
    pub global_requests_per_second: Option<u32>,
    /// Global burst size
    pub global_burst_size: Option<u32>,
    /// Path-specific rate limits
    pub path_limits: Option<Vec<PathRateLimit>>,
}

/// Path-specific rate limit configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PathRateLimit {
    pub pattern: String,
    pub requests_per_second: u32,
    pub burst_size: u32,
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
    /// Maximum concurrent handshakes
    pub max_concurrent_handshakes: Option<u32>,
    /// Track connection state
    pub track_connection_state: Option<bool>,
    /// Connection timeout in seconds
    pub connection_timeout_seconds: Option<u64>,
    /// Idle timeout in seconds
    pub idle_timeout_seconds: Option<u64>,
}

/// SYN Proxy configuration for Layer 4 protection
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SynProxyConfig {
    /// Enable SYN proxy mode
    pub enabled: bool,
    /// Secret key for SYN cookie generation
    pub cookie_secret: String,
    /// Listening port for raw socket
    pub listen_port: u16,
    /// Maximum time to wait for ACK after SYN-ACK (milliseconds)
    pub handshake_timeout_ms: u64,
    /// Maximum number of pending handshakes to track
    pub max_pending_handshakes: usize,
    /// SYN flood detection threshold
    pub syn_flood_threshold: Option<u32>,
}

/// Blacklist configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlacklistConfig {
    /// Enable blacklisting
    pub enabled: bool,
    /// Enable automatic blacklisting
    pub auto_blacklist_enabled: Option<bool>,
    /// Threshold for automatic blacklisting (violations per minute)
    pub violation_threshold: u32,
    /// Duration to blacklist IPs (seconds)
    pub blacklist_duration_seconds: Option<u64>,
    /// Escalation multiplier for repeat offenders
    pub escalation_multiplier: Option<f64>,
    /// Maximum blacklist duration (seconds)
    pub max_blacklist_duration_seconds: Option<u64>,
    /// Manual blacklist (CIDR ranges)
    pub static_blacklist: Option<Vec<String>>,
    /// Whitelist (CIDR ranges that bypass all protection)
    pub static_whitelist: Option<Vec<String>>,
    /// Enable geographic blocking
    pub geo_blocking_enabled: Option<bool>,
    /// Blocked countries (ISO 3166-1 alpha-2)
    pub blocked_countries: Option<Vec<String>>,
    /// Allowed countries (empty = allow all except blocked)
    pub allowed_countries: Option<Vec<String>>,
}

/// WAF configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WafConfig {
    /// Enable WAF
    pub enabled: bool,
    /// WAF operation mode: monitor, block, challenge
    pub mode: Option<String>,
    /// Maximum request size in bytes
    pub max_request_size_bytes: Option<usize>,
    /// Maximum header size in bytes
    pub max_header_size_bytes: Option<usize>,
    /// Maximum URL length
    pub max_url_length: Option<usize>,
    /// Maximum query parameters
    pub max_query_params: Option<u32>,
    /// HTTP inspection settings
    pub http_inspection: HttpInspectionConfig,
    /// Attack pattern detection
    pub attack_patterns: AttackPatternsConfig,
    /// AI-powered detection
    pub ai_detection: Option<AiDetectionConfig>,
}

/// HTTP inspection configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HttpInspectionConfig {
    /// Enable HTTP header inspection
    pub inspect_headers: bool,
    /// Enable HTTP body inspection
    pub inspect_body: bool,
    /// Enable URL inspection
    pub inspect_url: bool,
    /// Enable cookie inspection
    pub inspect_cookies: Option<bool>,
    /// Enable User-Agent inspection
    pub inspect_user_agent: Option<bool>,
    /// Maximum body size to inspect (bytes)
    pub max_body_size_bytes: Option<usize>,
    /// Body inspection timeout (milliseconds)
    pub body_inspection_timeout_ms: Option<u64>,
    /// Allowed content types
    pub allowed_content_types: Option<Vec<String>>,
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
    /// Enable LDAP injection detection
    pub ldap_injection: Option<bool>,
    /// Enable XML injection detection
    pub xml_injection: Option<bool>,
    /// Enable XXE protection
    pub xxe_protection: Option<bool>,
    /// Enable CSRF protection
    pub csrf_protection: Option<bool>,
    /// Enable session fixation protection
    pub session_fixation_protection: Option<bool>,
    /// Enable clickjacking protection
    pub clickjacking_protection: Option<bool>,
    /// Enable custom rules
    pub custom_rules_enabled: Option<bool>,
    /// Custom rules file path
    pub custom_rules_path: Option<String>,
    /// Enable anomaly scoring
    pub anomaly_scoring_enabled: Option<bool>,
    /// Anomaly threshold
    pub anomaly_threshold: Option<u32>,
    /// Inbound anomaly threshold
    pub inbound_anomaly_threshold: Option<u32>,
    /// Outbound anomaly threshold
    pub outbound_anomaly_threshold: Option<u32>,
}

/// AI-powered detection configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiDetectionConfig {
    /// Enable AI detection
    pub enabled: bool,
    /// Path to ML model
    pub model_path: Option<String>,
    /// Confidence threshold
    pub confidence_threshold: Option<f64>,
    /// Batch size for inference
    pub batch_size: Option<u32>,
    /// Inference timeout (milliseconds)
    pub inference_timeout_ms: Option<u64>,
    /// Feature extraction methods
    pub feature_extraction: Option<Vec<String>>,
}

/// Orchestrator integration configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrchestratorConfig {
    /// Enable orchestrator integration
    pub enabled: bool,
    /// Orchestrator server base URL
    pub server_url: Option<String>,
    /// Node type identifier
    pub node_type: Option<String>,
    /// Node identifier (if empty, will be auto-generated)
    pub node_id: Option<String>,
    /// Authentication method
    pub auth_method: Option<String>,
    /// Authentication token
    pub auth_token: Option<String>,
    /// Client certificate path
    pub client_cert_path: Option<String>,
    /// Client key path
    pub client_key_path: Option<String>,
    /// Node registration configuration
    pub registration: Option<RegistrationConfig>,
    /// Heartbeat configuration
    pub heartbeat: Option<HeartbeatConfig>,
    /// Command handling configuration
    pub commands: Option<CommandConfig>,
    /// Circuit breaker configuration
    pub circuit_breaker: Option<CircuitBreakerConfig>,
}

/// Node registration configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegistrationConfig {
    /// Enable auto-registration
    pub auto_register: Option<bool>,
    /// Retry interval in seconds
    pub retry_interval_seconds: Option<u64>,
    /// Maximum retry attempts
    pub max_retries: Option<u32>,
    /// Registration timeout in seconds
    pub timeout_seconds: Option<u64>,
    /// Additional registration data
    pub registration_data: Option<HashMap<String, serde_json::Value>>,
}

/// Heartbeat configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HeartbeatConfig {
    /// Heartbeat interval in seconds
    pub interval_seconds: u64,
    /// Heartbeat timeout in seconds
    pub timeout_seconds: u64,
    /// Maximum missed heartbeats before considering node offline
    pub max_missed: u32,
    /// Include metrics in heartbeat
    pub include_metrics: Option<bool>,
}

/// Command handling configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandConfig {
    /// Enable command handling
    pub enabled: bool,
    /// Command timeout in seconds
    pub command_timeout_seconds: u64,
    /// Maximum concurrent commands
    pub max_concurrent_commands: u32,
}

/// Circuit breaker configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CircuitBreakerConfig {
    /// Enable circuit breaker
    pub enabled: bool,
    /// Failure threshold
    pub failure_threshold: u32,
    /// Recovery timeout in seconds
    pub recovery_timeout_seconds: u64,
    /// Half-open max requests
    pub half_open_max_requests: u32,
}

/// NATS messaging configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NatsConfig {
    /// Enable NATS messaging
    pub enabled: bool,
    /// NATS server URLs
    pub servers: Option<Vec<String>>,
    /// Connection name
    pub name: Option<String>,
    /// Connection timeout in seconds
    pub connect_timeout_seconds: Option<u64>,
    /// Reconnect wait time in seconds
    pub reconnect_wait_seconds: Option<u64>,
    /// Maximum reconnection attempts
    pub max_reconnects: Option<u32>,
    /// Ping interval in seconds
    pub ping_interval_seconds: Option<u64>,
    /// Maximum outstanding pings
    pub max_outstanding_pings: Option<u32>,
    /// Authentication method
    pub auth_method: Option<String>,
    /// Authentication token
    pub auth_token: Option<String>,
    /// NKey seed
    pub nkey_seed: Option<String>,
    /// Credentials file path
    pub credentials_file: Option<String>,
    /// TLS settings for NATS
    pub tls_enabled: Option<bool>,
    /// TLS certificate path for NATS
    pub tls_cert_path: Option<String>,
    /// TLS key path for NATS
    pub tls_key_path: Option<String>,
    /// TLS CA path for NATS
    pub tls_ca_path: Option<String>,
    /// Subject configuration
    pub subjects: Option<NatsSubjectsConfig>,
}

/// NATS subjects configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NatsSubjectsConfig {
    /// Events publishing subject
    pub events_subject: String,
    /// Metrics publishing subject
    pub metrics_subject: String,
    /// Alerts publishing subject
    pub alerts_subject: String,
    /// Commands subscription subject
    pub commands_subject: String,
    /// Config updates subject
    pub config_updates_subject: String,
    /// JetStream enabled
    pub jetstream_enabled: Option<bool>,
    /// JetStream stream name
    pub stream_name: Option<String>,
    /// JetStream max age in seconds
    pub max_age_seconds: Option<u64>,
    /// JetStream max messages
    pub max_msgs: Option<u64>,
}

/// Metrics configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricsConfig {
    /// Enable metrics collection
    pub enabled: bool,
    /// Metrics server address
    pub listen_addr: String,
    /// Metrics collection interval (seconds)
    pub collection_interval_seconds: Option<u64>,
    /// Aggregation interval (seconds)
    pub aggregation_interval_seconds: Option<u64>,
    /// Retention hours
    pub retention_hours: Option<u64>,
    /// Enable Prometheus
    pub prometheus_enabled: Option<bool>,
    /// Prometheus path
    pub prometheus_path: Option<String>,
    /// Prometheus namespace
    pub prometheus_namespace: Option<String>,
    /// Enable custom metrics
    pub custom_metrics_enabled: Option<bool>,
    /// High cardinality metrics
    pub high_cardinality_metrics: Option<bool>,
    /// Metrics exporters
    pub exporters: Option<HashMap<String, serde_json::Value>>,
}

/// Logging configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoggingConfig {
    /// Log level (trace, debug, info, warn, error)
    pub level: String,
    /// Log format (json, text)
    pub format: Option<String>,
    /// Output destinations
    pub outputs: Option<Vec<String>>,
    /// Log file path (optional)
    pub file_path: Option<String>,
    /// Maximum file size in MB
    pub max_file_size_mb: Option<u64>,
    /// Maximum number of files
    pub max_files: Option<u32>,
    /// Include timestamp
    pub include_timestamp: Option<bool>,
    /// Include log level
    pub include_level: Option<bool>,
    /// Include target
    pub include_target: Option<bool>,
    /// Include thread ID
    pub include_thread_id: Option<bool>,
    /// Include line number
    pub include_line_number: Option<bool>,
    /// Enable sampling
    pub sampling_enabled: Option<bool>,
    /// Sampling rate
    pub sampling_rate: Option<f64>,
    /// Audit logging
    pub audit: Option<AuditLoggingConfig>,
}

/// Audit logging configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditLoggingConfig {
    /// Enable audit logging
    pub enabled: bool,
    /// Audit log file path
    pub file_path: String,
    /// Events to include
    pub include_events: Vec<String>,
}

/// Management API configuration for node control
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ManagementApiConfig {
    /// Enable management API
    pub enabled: bool,
    /// Listen address for management API
    pub listen_addr: String,
    /// Enable TLS for management API
    pub tls_enabled: Option<bool>,
    /// Authentication method
    pub auth_method: Option<String>,
    /// Authentication token for secure commands
    pub auth_token: Option<String>,
    /// Basic auth username
    pub basic_auth_user: Option<String>,
    /// Basic auth password
    pub basic_auth_password: Option<String>,
    /// Enable status endpoint
    pub enable_status: Option<bool>,
    /// Enable metrics endpoint
    pub enable_metrics: Option<bool>,
    /// Enable config endpoint
    pub enable_config: Option<bool>,
    /// Enable health endpoint
    pub enable_health: Option<bool>,
    /// Enable debug endpoints
    pub enable_debug: Option<bool>,
    /// Enable CORS
    pub cors_enabled: Option<bool>,
    /// CORS origins
    pub cors_origins: Option<Vec<String>>,
    /// Enable rate limiting
    pub rate_limiting_enabled: Option<bool>,
    /// Rate limit requests per minute
    pub rate_limit_requests_per_minute: Option<u32>,
    /// Grace period for shutdown (seconds)
    pub shutdown_grace_period_seconds: Option<u64>,
    /// Force shutdown timeout (seconds)
    pub force_shutdown_timeout_seconds: Option<u64>,
}

/// Health check configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthConfig {
    /// Enable health checks
    pub enabled: bool,
    /// Health check listen address
    pub listen_addr: String,
    /// Health check path
    pub path: String,
    /// Check interval in seconds
    pub check_interval_seconds: Option<u64>,
    /// Timeout in seconds
    pub timeout_seconds: Option<u64>,
    /// Check backend connectivity
    pub check_backend: Option<bool>,
    /// Check NATS connectivity
    pub check_nats: Option<bool>,
    /// Check orchestrator connectivity
    pub check_orchestrator: Option<bool>,
    /// Readiness path
    pub readiness_path: Option<String>,
    /// Liveness path
    pub liveness_path: Option<String>,
}

/// Debug configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DebugConfig {
    /// Enable debug features
    pub enabled: bool,
    /// Enable pprof
    pub pprof_enabled: Option<bool>,
    /// pprof address
    pub pprof_addr: Option<String>,
    /// CPU profiling
    pub cpu_profiling: Option<bool>,
    /// Memory profiling
    pub memory_profiling: Option<bool>,
    /// Block profiling
    pub block_profiling: Option<bool>,
    /// Mutex profiling
    pub mutex_profiling: Option<bool>,
    /// Request tracing enabled
    pub request_tracing_enabled: Option<bool>,
    /// Trace sampling rate
    pub trace_sampling_rate: Option<f64>,
    /// Debug endpoints enabled
    pub debug_endpoints_enabled: Option<bool>,
    /// Debug auth required
    pub debug_auth_required: Option<bool>,
}

impl MitigationConfig {
    /// Check if a feature is enabled
    pub fn has_feature(&self, feature: &str) -> bool {
        self.platform.features.contains(&feature.to_string())
    }

    /// Get listen address from network config
    pub fn listen_addr(&self) -> Result<SocketAddr, std::net::AddrParseError> {
        format!(
            "{}:{}",
            self.network.public_interface, self.network.public_port
        )
        .parse()
    }

    /// Get backend address from network config
    pub fn backend_addr(&self) -> Result<SocketAddr, std::net::AddrParseError> {
        format!(
            "{}:{}",
            self.network.backend_interface, self.network.backend_port
        )
        .parse()
    }

    /// Get connection timeout as Duration
    pub fn connection_timeout(&self) -> Duration {
        Duration::from_secs(self.network.connection_timeout_seconds)
    }

    /// Get penalty duration as Duration
    pub fn penalty_duration(&self) -> Duration {
        Duration::from_secs(
            self.ddos
                .rate_limiting
                .penalty_duration_seconds
                .unwrap_or(300),
        )
    }

    /// Get blacklist duration as Duration
    pub fn blacklist_duration(&self) -> Duration {
        Duration::from_secs(
            self.ddos
                .blacklist
                .blacklist_duration_seconds
                .unwrap_or(3600),
        )
    }

    /// Check if SYN proxy is enabled
    pub fn syn_proxy_enabled(&self) -> bool {
        self.has_feature("syn_proxy") && self.ddos.syn_proxy.enabled
    }

    /// Check if WAF is enabled
    pub fn waf_enabled(&self) -> bool {
        self.has_feature("waf_protection") && self.waf.enabled
    }

    /// Check if DDoS protection is enabled
    pub fn ddos_enabled(&self) -> bool {
        self.has_feature("ddos_protection") && self.ddos.enabled
    }

    /// Check if TLS is enabled
    pub fn tls_enabled(&self) -> bool {
        self.has_feature("tls_termination") && self.network.tls.enabled
    }

    /// Check if orchestrator integration is enabled
    pub fn orchestrator_enabled(&self) -> bool {
        self.has_feature("orchestrator") && self.orchestrator.enabled
    }

    /// Check if NATS messaging is enabled
    pub fn nats_enabled(&self) -> bool {
        self.has_feature("nats_messaging") && self.nats.enabled
    }

    /// Check if metrics are enabled
    pub fn metrics_enabled(&self) -> bool {
        self.has_feature("metrics") && self.metrics.enabled
    }

    /// Check if management API is enabled
    pub fn management_enabled(&self) -> bool {
        self.has_feature("management_api") && self.management.enabled
    }
}

impl Default for MitigationConfig {
    fn default() -> Self {
        Self {
            platform: PlatformConfig {
                environment: "development".to_string(),
                deployment_id: "secbeat-dev-local".to_string(),
                region: "local".to_string(),
                features: vec![
                    "ddos_protection".to_string(),
                    "waf_protection".to_string(),
                    "tls_termination".to_string(),
                    "metrics".to_string(),
                    "management_api".to_string(),
                ],
                mode: Some("auto".to_string()),
            },
            network: NetworkConfig {
                public_interface: "127.0.0.1".to_string(),
                public_port: 8443,
                backend_interface: "127.0.0.1".to_string(),
                backend_port: 8080,
                max_connections: 1000,
                connection_timeout_seconds: 30,
                keep_alive_timeout_seconds: 60,
                buffer_size: 8192,
                load_balance_mode: None,
                backend_servers: vec![BackendServer {
                    address: "127.0.0.1:8080".to_string(),
                    weight: 100,
                    health_check: true,
                }],
                tls: TlsConfig {
                    enabled: true,
                    cert_path: "mitigation-node/certs/cert.pem".to_string(),
                    key_path: "mitigation-node/certs/key.pem".to_string(),
                    ca_path: None,
                    min_version: "1.2".to_string(),
                    max_version: "1.3".to_string(),
                    cipher_suites: None,
                    alpn_protocols: vec!["h2".to_string(), "http/1.1".to_string()],
                    client_cert_required: false,
                    client_cert_ca_path: None,
                    ocsp_stapling: false,
                },
            },
            ddos: DdosConfig {
                enabled: true,
                detection_threshold_multiplier: Some(2.0),
                mitigation_duration_seconds: Some(60),
                learning_period_seconds: Some(300),
                rate_limiting: RateLimitConfig {
                    requests_per_second: 100,
                    burst_size: 200,
                    window_seconds: 60,
                    penalty_duration_seconds: Some(60),
                    global_requests_per_second: Some(5000),
                    global_burst_size: Some(10000),
                    path_limits: None,
                },
                connection_limits: ConnectionLimitsConfig {
                    max_connections_per_ip: 10,
                    max_total_connections: 1000,
                    max_new_connections_per_second: 100,
                    max_concurrent_handshakes: Some(1000),
                    track_connection_state: None,
                    connection_timeout_seconds: None,
                    idle_timeout_seconds: None,
                },
                syn_proxy: SynProxyConfig {
                    enabled: true,
                    cookie_secret: "dev-cookie-secret-not-for-production-use-only".to_string(),
                    listen_port: 8443,
                    handshake_timeout_ms: 5000,
                    max_pending_handshakes: 5000,
                    syn_flood_threshold: Some(100),
                },
                blacklist: BlacklistConfig {
                    enabled: true,
                    auto_blacklist_enabled: Some(true),
                    violation_threshold: 5,
                    blacklist_duration_seconds: Some(300),
                    escalation_multiplier: Some(1.5),
                    max_blacklist_duration_seconds: Some(3600),
                    static_blacklist: None,
                    static_whitelist: Some(vec![
                        "127.0.0.0/8".to_string(),
                        "10.0.0.0/8".to_string(),
                        "172.16.0.0/12".to_string(),
                        "192.168.0.0/16".to_string(),
                    ]),
                    geo_blocking_enabled: Some(false),
                    blocked_countries: None,
                    allowed_countries: None,
                },
            },
            waf: WafConfig {
                enabled: true,
                mode: Some("monitor".to_string()),
                max_request_size_bytes: Some(1048576),
                max_header_size_bytes: Some(8192),
                max_url_length: Some(2048),
                max_query_params: Some(50),
                http_inspection: HttpInspectionConfig {
                    inspect_headers: true,
                    inspect_body: true,
                    inspect_url: true,
                    inspect_cookies: Some(true),
                    inspect_user_agent: Some(true),
                    max_body_size_bytes: Some(524288),
                    body_inspection_timeout_ms: Some(500),
                    allowed_content_types: Some(vec![
                        "application/json".to_string(),
                        "application/x-www-form-urlencoded".to_string(),
                        "multipart/form-data".to_string(),
                        "text/plain".to_string(),
                    ]),
                },
                attack_patterns: AttackPatternsConfig {
                    sql_injection: true,
                    xss_detection: true,
                    path_traversal: true,
                    command_injection: true,
                    ldap_injection: Some(false),
                    xml_injection: Some(false),
                    xxe_protection: Some(false),
                    csrf_protection: Some(false),
                    session_fixation_protection: Some(false),
                    clickjacking_protection: Some(false),
                    custom_rules_enabled: Some(false),
                    custom_rules_path: None,
                    anomaly_scoring_enabled: Some(false),
                    anomaly_threshold: None,
                    inbound_anomaly_threshold: None,
                    outbound_anomaly_threshold: None,
                },
                ai_detection: None,
            },
            orchestrator: OrchestratorConfig {
                enabled: false,
                server_url: None,
                node_type: None,
                node_id: None,
                auth_method: None,
                auth_token: None,
                client_cert_path: None,
                client_key_path: None,
                registration: None,
                heartbeat: None,
                commands: None,
                circuit_breaker: None,
            },
            nats: NatsConfig {
                enabled: false,
                servers: None,
                name: None,
                connect_timeout_seconds: None,
                reconnect_wait_seconds: None,
                max_reconnects: None,
                ping_interval_seconds: None,
                max_outstanding_pings: None,
                auth_method: None,
                auth_token: None,
                nkey_seed: None,
                credentials_file: None,
                tls_enabled: None,
                tls_cert_path: None,
                tls_key_path: None,
                tls_ca_path: None,
                subjects: None,
            },
            metrics: MetricsConfig {
                enabled: true,
                listen_addr: "127.0.0.1:9090".to_string(),
                collection_interval_seconds: Some(10),
                aggregation_interval_seconds: Some(60),
                retention_hours: Some(2),
                prometheus_enabled: Some(true),
                prometheus_path: Some("/metrics".to_string()),
                prometheus_namespace: Some("secbeat_dev".to_string()),
                custom_metrics_enabled: Some(true),
                high_cardinality_metrics: Some(true),
                exporters: None,
            },
            logging: LoggingConfig {
                level: "debug".to_string(),
                format: Some("text".to_string()),
                outputs: Some(vec!["stdout".to_string()]),
                file_path: None,
                max_file_size_mb: None,
                max_files: None,
                include_timestamp: Some(true),
                include_level: Some(true),
                include_target: Some(true),
                include_thread_id: Some(false),
                include_line_number: Some(true),
                sampling_enabled: Some(false),
                sampling_rate: None,
                audit: None,
            },
            management: ManagementApiConfig {
                enabled: true,
                listen_addr: "127.0.0.1:9999".to_string(),
                tls_enabled: Some(false),
                auth_method: Some("none".to_string()),
                auth_token: None,
                basic_auth_user: None,
                basic_auth_password: None,
                enable_status: Some(true),
                enable_metrics: Some(true),
                enable_config: Some(true),
                enable_health: Some(true),
                enable_debug: Some(true),
                cors_enabled: Some(true),
                cors_origins: Some(vec!["*".to_string()]),
                rate_limiting_enabled: Some(false),
                rate_limit_requests_per_minute: None,
                shutdown_grace_period_seconds: Some(10),
                force_shutdown_timeout_seconds: Some(5),
            },
            health: HealthConfig {
                enabled: true,
                listen_addr: "127.0.0.1:8081".to_string(),
                path: "/health".to_string(),
                check_interval_seconds: Some(30),
                timeout_seconds: Some(5),
                check_backend: Some(true),
                check_nats: Some(false),
                check_orchestrator: Some(false),
                readiness_path: Some("/ready".to_string()),
                liveness_path: Some("/live".to_string()),
            },
            debug: DebugConfig {
                enabled: true,
                pprof_enabled: Some(true),
                pprof_addr: Some("127.0.0.1:6060".to_string()),
                cpu_profiling: Some(false),
                memory_profiling: Some(false),
                block_profiling: None,
                mutex_profiling: None,
                request_tracing_enabled: Some(true),
                trace_sampling_rate: Some(1.0),
                debug_endpoints_enabled: Some(true),
                debug_auth_required: Some(false),
            },
        }
    }
}

impl MitigationConfig {
    /// Load configuration from file
    pub fn from_file(path: &str) -> Result<Self, config::ConfigError> {
        let settings = config::Config::builder()
            .add_source(config::File::with_name(path))
            .add_source(config::Environment::with_prefix("SECBEAT"))
            .build()?;

        settings.try_deserialize()
    }

    /// Save configuration to file
    #[allow(dead_code)]
    pub fn save_to_file(&self, path: &str) -> Result<(), Box<dyn std::error::Error>> {
        let toml_string = toml::to_string_pretty(self)?;
        std::fs::write(path, toml_string)?;
        Ok(())
    }

    /// Validate configuration
    pub fn validate(&self) -> Result<(), String> {
        // Validate network configuration
        if self.network.public_port == 0 {
            return Err("Public port cannot be 0".to_string());
        }

        if self.network.backend_port == 0 {
            return Err("Backend port cannot be 0".to_string());
        }

        // Validate TLS configuration if enabled
        if self.tls_enabled() {
            if self.network.tls.cert_path.is_empty() {
                return Err("TLS certificate path cannot be empty when TLS is enabled".to_string());
            }
            if self.network.tls.key_path.is_empty() {
                return Err("TLS key path cannot be empty when TLS is enabled".to_string());
            }
        }

        // Validate feature consistency
        if self.syn_proxy_enabled() && !self.ddos_enabled() {
            return Err("SYN proxy requires DDoS protection to be enabled".to_string());
        }

        Ok(())
    }
}

/// Runtime configuration manager for hot-reloading
pub struct ConfigManager {
    current_config: Arc<RwLock<MitigationConfig>>,
    config_path: String,
    watchers: Vec<tokio::sync::broadcast::Sender<MitigationConfig>>,
}

impl ConfigManager {
    /// Create new configuration manager
    pub fn new(config: MitigationConfig, config_path: String) -> Self {
        Self {
            current_config: Arc::new(RwLock::new(config)),
            config_path,
            watchers: Vec::new(),
        }
    }

    /// Get current configuration
    pub async fn get_config(&self) -> MitigationConfig {
        self.current_config.read().await.clone()
    }

    /// Reload configuration from file
    pub async fn reload_config(&mut self) -> Result<(), String> {
        info!("Reloading configuration from {}", self.config_path);

        let new_config = MitigationConfig::from_file(&self.config_path)
            .map_err(|e| format!("Failed to load config: {}", e))?;

        // Validate new configuration
        new_config
            .validate()
            .map_err(|e| format!("Invalid config: {}", e))?;

        // Update current configuration
        {
            let mut current = self.current_config.write().await;
            *current = new_config.clone();
        }

        // Notify watchers
        for sender in &self.watchers {
            let _ = sender.send(new_config.clone());
        }

        info!("Configuration reloaded successfully");
        Ok(())
    }

    /// Subscribe to configuration changes
    pub fn subscribe(&mut self) -> tokio::sync::broadcast::Receiver<MitigationConfig> {
        let (sender, receiver) = tokio::sync::broadcast::channel(10);
        self.watchers.push(sender);
        receiver
    }

    /// Update configuration with environment variables
    pub async fn apply_env_overrides(&mut self) -> Result<(), String> {
        let mut config = self.current_config.write().await;

        // Apply environment variable overrides
        if let Ok(log_level) = std::env::var("RUST_LOG") {
            config.logging.level = log_level;
        }

        if let Ok(max_connections) = std::env::var("MAX_CONNECTIONS") {
            config.ddos.connection_limits.max_connections_per_ip = max_connections
                .parse()
                .map_err(|e| format!("Invalid MAX_CONNECTIONS: {}", e))?;
        }

        if let Ok(backend_addr) = std::env::var("BACKEND_ADDRESS") {
            config.network.backend_interface = backend_addr;
        }

        if let Ok(public_port) = std::env::var("PUBLIC_PORT") {
            config.network.public_port = public_port
                .parse()
                .map_err(|e| format!("Invalid PUBLIC_PORT: {}", e))?;
        }

        // TLS certificate overrides
        if let Ok(cert_path) = std::env::var("TLS_CERT_PATH") {
            config.network.tls.cert_path = cert_path;
        }

        if let Ok(key_path) = std::env::var("TLS_KEY_PATH") {
            config.network.tls.key_path = key_path;
        }

        // Security overrides
        if let Ok(syn_cookie_secret) = std::env::var("SYN_COOKIE_SECRET") {
            config.ddos.syn_proxy.cookie_secret = syn_cookie_secret;
        }

        info!("Applied environment variable overrides");
        Ok(())
    }
}
