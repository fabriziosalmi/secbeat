use anyhow::{Context, Result};
use hyper::service::service_fn;
use hyper::{Body, Client, Request, Response, StatusCode};
use metrics::{counter, gauge, describe_counter, describe_gauge};
use rustls::{Certificate, PrivateKey, ServerConfig};
use rustls_pemfile::{certs, pkcs8_private_keys};
use std::convert::Infallible;
use std::fs::File;
use std::io::BufReader;
use std::net::{IpAddr, SocketAddr};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tokio::net::{TcpListener, TcpStream};
use tokio_rustls::TlsAcceptor;
use tracing::{debug, error, info, instrument, warn};

mod config;
mod ddos;
mod waf;
mod orchestrator;
mod events;
mod management;

use config::MitigationConfig;
use ddos::{DdosProtection, DdosCheckResult};
use orchestrator::{OrchestratorClient, NodeConfig, NodeStatus, collect_system_metrics};
use events::{EventSystem, SecurityEvent, WafEventResult};
use management::{start_management_api, ShutdownSignal};

/// L7 TLS/HTTP Proxy state management
#[derive(Debug, Clone)]
struct ProxyState {
    /// Configuration
    config: MitigationConfig,
    /// DDoS protection engine
    ddos_protection: Arc<DdosProtection>,
    /// Global metrics counters
    metrics: ProxyMetrics,
    /// HTTP client for backend connections
    http_client: Client<hyper_rustls::HttpsConnector<hyper::client::HttpConnector>>,
    /// Orchestrator client
    orchestrator_client: Option<Arc<OrchestratorClient>>,
    /// Event system for NATS communication
    event_system: Option<Arc<EventSystem>>,
}

/// Global metrics counters
#[derive(Debug, Clone)]
struct ProxyMetrics {
    /// Total HTTPS requests received
    https_requests_received: Arc<AtomicU64>,
    /// Total requests proxied to backend
    requests_proxied: Arc<AtomicU64>,
    /// Total TLS handshakes completed
    tls_handshakes_completed: Arc<AtomicU64>,
    /// Total TLS handshake errors
    tls_handshake_errors: Arc<AtomicU64>,
    /// Total HTTP errors
    http_errors: Arc<AtomicU64>,
    /// Currently active connections
    active_connections: Arc<AtomicU64>,
    /// Total blocked requests (DDoS + WAF)
    blocked_requests: Arc<AtomicU64>,
    /// DDoS events detected
    ddos_events_detected: Arc<AtomicU64>,
    /// WAF events blocked
    waf_events_blocked: Arc<AtomicU64>,
}

impl ProxyMetrics {
    fn new() -> Self {
        Self {
            https_requests_received: Arc::new(AtomicU64::new(0)),
            requests_proxied: Arc::new(AtomicU64::new(0)),
            tls_handshakes_completed: Arc::new(AtomicU64::new(0)),
            tls_handshake_errors: Arc::new(AtomicU64::new(0)),
            http_errors: Arc::new(AtomicU64::new(0)),
            active_connections: Arc::new(AtomicU64::new(0)),
            blocked_requests: Arc::new(AtomicU64::new(0)),
            ddos_events_detected: Arc::new(AtomicU64::new(0)),
            waf_events_blocked: Arc::new(AtomicU64::new(0)),
        }
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize structured logging
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "mitigation_node=info".into()),
        )
        .with_target(false)
        .with_thread_ids(true)
        .with_line_number(true)
        .init();

    info!("Starting Mitigation Node v{} - Phase 5: Centralized Intelligence & Real-time Control", env!("CARGO_PKG_VERSION"));

    // Load configuration from file if available, otherwise use defaults
    let config = match MitigationConfig::from_file("config/default") {
        Ok(config) => {
            info!("Configuration loaded from config/default.toml");
            config
        }
        Err(e) => {
            warn!("Failed to load config file: {}, using defaults", e);
            MitigationConfig::default()
        }
    };

    // Validate TLS configuration
    if !config.network.tls.enabled {
        error!("TLS is disabled in configuration. This is required for Phase 3.");
        return Err(anyhow::anyhow!("TLS must be enabled for L7 proxy"));
    }

    info!("L7 TLS/HTTP Proxy enabled on port: {}", config.network.tls.tls_port);
    
    // Initialize DDoS protection
    let ddos_protection = Arc::new(DdosProtection::new(config.ddos.clone())?);
    
    // Initialize metrics
    let metrics = ProxyMetrics::new();
    initialize_metrics();

    // Create HTTPS client for backend connections
    let https_connector = hyper_rustls::HttpsConnectorBuilder::new()
        .with_native_roots()
        .https_or_http()
        .enable_http1()
        .build();
    let http_client = Client::builder().build::<_, hyper::Body>(https_connector);

    // Initialize orchestrator client if enabled
    let orchestrator_client = if config.orchestrator.enabled {
        info!("Initializing orchestrator integration");
        let client = Arc::new(OrchestratorClient::new(config.orchestrator.clone()));
        
        // Create node configuration for registration
        let node_config = NodeConfig {
            listen_addr: config.network.listen_addr.to_string(),
            backend_addr: config.network.backend_addr.to_string(),
            tls_enabled: config.network.tls.enabled,
            ddos_enabled: config.ddos.enabled,
            waf_enabled: config.waf.enabled,
            max_connections: config.ddos.connection_limits.max_total_connections,
            rate_limit_rps: config.ddos.rate_limiting.requests_per_second,
        };

        // Attempt initial registration
        match client.register(node_config).await {
            Ok(response) => {
                info!(
                    node_id = %response.node_id,
                    "Successfully registered with orchestrator"
                );
                client.set_status(NodeStatus::Active).await;
            }
            Err(e) => {
                warn!(error = %e, "Failed to register with orchestrator, will retry later");
                client.set_status(NodeStatus::Draining).await;
            }
        }

        Some(client)
    } else {
        info!("Orchestrator integration disabled");
        None
    };

    // Initialize event system if enabled
    let event_system = if config.nats.enabled {
        info!("Initializing NATS event system");
        
        // Get node ID from orchestrator client or generate one
        let node_id = if let Some(ref client) = orchestrator_client {
            client.get_node_id().await.unwrap_or_else(|| uuid::Uuid::new_v4())
        } else {
            uuid::Uuid::new_v4()
        };

        match EventSystem::new(&config.nats.server_url, node_id).await {
            Ok(event_system) => {
                let event_system = Arc::new(event_system);
                
                // Start command consumer if enabled
                if config.nats.consume_commands {
                    let event_system_clone = Arc::clone(&event_system);
                    if let Err(e) = event_system_clone.start_command_consumer().await {
                        warn!(error = %e, "Failed to start command consumer");
                    } else {
                        info!("Started NATS command consumer");
                    }
                }
                
                Some(event_system)
            }
            Err(e) => {
                warn!(error = %e, "Failed to initialize NATS event system");
                None
            }
        }
    } else {
        info!("NATS integration disabled");
        None
    };

    // Create proxy state
    let proxy_state = ProxyState {
        config: config.clone(),
        ddos_protection: Arc::clone(&ddos_protection),
        metrics: metrics.clone(),
        http_client,
        orchestrator_client: orchestrator_client.clone(),
        event_system,
    };

    // Start metrics server
    let metrics_clone = metrics.clone();
    let metrics_config = config.metrics.clone();
    tokio::spawn(async move {
        if let Err(e) = start_metrics_server(metrics_config.listen_addr, metrics_clone).await {
            error!(error = %e, "Failed to start metrics server");
        }
    });

    // Start orchestrator heartbeat loop if enabled
    if let Some(orchestrator_client) = orchestrator_client.clone() {
        let metrics_for_heartbeat = metrics.clone();
        
        tokio::spawn(async move {
            let metrics_provider = move || {
                collect_system_metrics(
                    metrics_for_heartbeat.active_connections.load(Ordering::Relaxed),
                    metrics_for_heartbeat.https_requests_received.load(Ordering::Relaxed),
                    metrics_for_heartbeat.blocked_requests.load(Ordering::Relaxed),
                    metrics_for_heartbeat.waf_events_blocked.load(Ordering::Relaxed),
                )
            };

            if let Err(e) = orchestrator_client.start_heartbeat_loop(metrics_provider) {
                error!(error = %e, "Failed to start orchestrator heartbeat loop");
            }
        });
        
        info!("Started orchestrator heartbeat loop");
    }

    // Start DDoS cleanup task
    ddos_protection.start_cleanup_task();

    // Initialize and start management API
    let (shutdown_signal, shutdown_receiver) = ShutdownSignal::new();
    
    if config.management.enabled {
        info!("Starting management API server");
        let management_config = config.management.clone();
        let shutdown_signal_clone = shutdown_signal.clone();
        
        tokio::spawn(async move {
            if let Err(e) = start_management_api(management_config, shutdown_signal_clone).await {
                error!(error = %e, "Management API server failed");
            }
        });
    } else {
        info!("Management API is disabled");
    }

    // Start the L7 TLS/HTTP proxy server with shutdown signal monitoring
    tokio::select! {
        result = run_l7_proxy(proxy_state) => result,
        _ = shutdown_receiver => {
            info!("Shutdown signal received, terminating proxy");
            Ok(())
        }
    }
}

/// Main L7 TLS/HTTP proxy server
#[instrument(name = "l7_proxy_server", skip(state))]
async fn run_l7_proxy(state: ProxyState) -> Result<()> {
    info!(
        listen_addr = %state.config.network.listen_addr,
        backend_addr = %state.config.network.backend_addr,
        tls_port = state.config.network.tls.tls_port,
        cert_path = %state.config.network.tls.cert_path,
        ddos_enabled = state.config.ddos.enabled,
        "Initializing L7 TLS/HTTP Proxy"
    );

    // Load TLS configuration
    let tls_config = load_tls_config(&state.config.network.tls).await
        .context("Failed to load TLS configuration")?;
    let tls_acceptor = TlsAcceptor::from(Arc::new(tls_config));

    // Create TCP listener
    let listen_addr = SocketAddr::new(
        state.config.network.listen_addr.ip(),
        state.config.network.tls.tls_port,
    );
    let tcp_listener = TcpListener::bind(&listen_addr).await
        .with_context(|| format!("Failed to bind to {}", listen_addr))?;

    info!(
        listen_addr = %listen_addr,
        "L7 TLS/HTTP proxy server started and listening for connections"
    );

    // Accept connections in a loop
    loop {
        match tcp_listener.accept().await {
            Ok((tcp_stream, client_addr)) => {
                let client_ip = client_addr.ip();
                
                // DDoS protection check
                let ddos_result = state.ddos_protection.check_connection(client_ip);
                
                match ddos_result {
                    DdosCheckResult::Allow => {
                        // Connection allowed, proceed with TLS handshake
                        let tls_acceptor = tls_acceptor.clone();
                        let state_clone = state.clone();
                        
                        tokio::spawn(async move {
                            if let Err(e) = handle_tls_connection(tcp_stream, client_addr, tls_acceptor, state_clone).await {
                                error!(
                                    client_addr = %client_addr,
                                    error = %e,
                                    "Failed to handle TLS connection"
                                );
                            }
                        });
                    }
                    DdosCheckResult::RateLimited => {
                        warn!(client_addr = %client_addr, "Connection blocked - rate limited");
                        counter!("l7_proxy_blocked_rate_limit", 1);
                        state.metrics.blocked_requests.fetch_add(1, Ordering::Relaxed);
                        state.metrics.ddos_events_detected.fetch_add(1, Ordering::Relaxed);
                        drop(tcp_stream);
                    }
                    DdosCheckResult::ConnectionLimitExceeded => {
                        warn!(client_addr = %client_addr, "Connection blocked - connection limit exceeded");
                        counter!("l7_proxy_blocked_connection_limit", 1);
                        state.metrics.blocked_requests.fetch_add(1, Ordering::Relaxed);
                        state.metrics.ddos_events_detected.fetch_add(1, Ordering::Relaxed);
                        drop(tcp_stream);
                    }
                    DdosCheckResult::Blacklisted => {
                        warn!(client_addr = %client_addr, "Connection blocked - IP blacklisted");
                        counter!("l7_proxy_blocked_blacklist", 1);
                        state.metrics.blocked_requests.fetch_add(1, Ordering::Relaxed);
                        state.metrics.ddos_events_detected.fetch_add(1, Ordering::Relaxed);
                        drop(tcp_stream);
                    }
                    DdosCheckResult::GlobalLimitExceeded => {
                        warn!(client_addr = %client_addr, "Connection blocked - global limit exceeded");
                        counter!("l7_proxy_blocked_global_limit", 1);
                        state.metrics.blocked_requests.fetch_add(1, Ordering::Relaxed);
                        state.metrics.ddos_events_detected.fetch_add(1, Ordering::Relaxed);
                        drop(tcp_stream);
                    }
                }
            }
            Err(e) => {
                error!(error = %e, "Failed to accept connection");
                tokio::time::sleep(Duration::from_millis(100)).await;
            }
        }
    }
}

/// Handle TLS connection and HTTP processing
async fn handle_tls_connection(
    tcp_stream: TcpStream,
    client_addr: SocketAddr,
    tls_acceptor: TlsAcceptor,
    state: ProxyState,
) -> Result<()> {
    // PHASE 5: Check dynamic IP blocklist BEFORE any processing
    if let Some(ref event_system) = state.event_system {
        if event_system.is_ip_blocked(client_addr.ip()).await {
            warn!(
                client_ip = %client_addr.ip(),
                "Connection dropped - IP is in dynamic blocklist"
            );
            state.metrics.blocked_requests.fetch_add(1, Ordering::Relaxed);
            counter!("dynamic_blocks_total", 1);
            // Drop connection immediately
            return Ok(());
        }
    }
    
    // Increment active connections counter
    state.metrics.active_connections.fetch_add(1, Ordering::Relaxed);
    gauge!("active_connections", state.metrics.active_connections.load(Ordering::Relaxed) as f64);
    
    // Perform TLS handshake
    let tls_stream = match tls_acceptor.accept(tcp_stream).await {
        Ok(stream) => {
            state.metrics.tls_handshakes_completed.fetch_add(1, Ordering::Relaxed);
            counter!("tls_handshakes_completed", 1);
            stream
        }
        Err(e) => {
            state.metrics.tls_handshake_errors.fetch_add(1, Ordering::Relaxed);
            counter!("tls_handshake_errors", 1);
            error!(client_addr = %client_addr, error = %e, "TLS handshake failed");
            state.metrics.active_connections.fetch_sub(1, Ordering::Relaxed);
            return Err(e.into());
        }
    };

    debug!(client_addr = %client_addr, "TLS handshake completed successfully");

    // Clone state for the service closure
    let state_for_service = state.clone();
    
    // Create HTTP service for this connection
    let service = service_fn(move |req: Request<Body>| {
        let state = state_for_service.clone();
        let client_addr = client_addr;
        
        async move {
            handle_http_request(req, client_addr, state).await
        }
    });

    // Serve HTTP requests over TLS
    if let Err(e) = hyper::server::conn::Http::new()
        .serve_connection(tls_stream, service)
        .await
    {
        warn!(client_addr = %client_addr, error = %e, "HTTP connection error");
        state.metrics.http_errors.fetch_add(1, Ordering::Relaxed);
    }

    // Decrement active connections counter
    state.metrics.active_connections.fetch_sub(1, Ordering::Relaxed);
    gauge!("active_connections", state.metrics.active_connections.load(Ordering::Relaxed) as f64);

    debug!(client_addr = %client_addr, "Connection closed");
    Ok(())
}

/// Handle individual HTTP request
async fn handle_http_request(
    req: Request<Body>,
    client_addr: SocketAddr,
    state: ProxyState,
) -> Result<Response<Body>, Infallible> {
    let start_time = std::time::Instant::now();
    
    // Increment HTTPS requests counter
    state.metrics.https_requests_received.fetch_add(1, Ordering::Relaxed);
    counter!("https_requests_received", 1);

    let method = req.method().clone();
    let uri = req.uri().clone();
    let _uri_path = uri.path();
    
    // Extract headers before moving req
    let user_agent = req.headers()
        .get("user-agent")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("unknown")
        .to_string();
        
    let host_header = req.headers()
        .get("host")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string());
    
    debug!(
        client_addr = %client_addr,
        method = %method,
        uri = %uri,
        "Processing HTTPS request"
    );

    // PHASE 5: WAF analysis with event publishing
    let waf_result = analyze_request_with_waf(&method, &uri, &user_agent);
    let response_result = if waf_result.action == "BLOCK" {
        warn!(
            client_addr = %client_addr,
            uri = %uri,
            matched_rules = ?waf_result.matched_rules,
            "Request blocked by WAF - suspicious content detected"
        );
        counter!("waf_requests_blocked", 1);
        state.metrics.blocked_requests.fetch_add(1, Ordering::Relaxed);
        state.metrics.waf_events_blocked.fetch_add(1, Ordering::Relaxed);
        
        // Create blocked response
        let response = Response::builder()
            .status(StatusCode::FORBIDDEN)
            .body(Body::from("Request blocked by Web Application Firewall"))
            .unwrap();
        
        // Publish security event for blocked request
        publish_security_event(&state, client_addr.ip(), &method, &uri, &host_header, &user_agent, &waf_result, Some(403), start_time.elapsed()).await;
        
        return Ok(response);
    } else {
        // Request is allowed, proceed with proxying
        proxy_to_backend(req, &method, &uri, client_addr, &state, &user_agent).await
    };
    
    // Publish security event for all requests (blocked ones already published above)
    let status_code = response_result.as_ref().map(|r| r.status().as_u16()).ok();
    publish_security_event(&state, client_addr.ip(), &method, &uri, &host_header, &user_agent, &waf_result, status_code, start_time.elapsed()).await;
    
    response_result
}

/// Analyze request with WAF and return result
fn analyze_request_with_waf(_method: &hyper::Method, uri: &hyper::Uri, _user_agent: &str) -> WafEventResult {
    let uri_path = uri.path();
    let mut matched_rules = Vec::new();
    
    // Simple WAF rules (can be enhanced later)
    if uri_path.contains("..") {
        matched_rules.push("PATH_TRAVERSAL".to_string());
    }
    if uri_path.contains("<script>") {
        matched_rules.push("XSS_SCRIPT_TAG".to_string());
    }
    if uri_path.contains("UNION") || uri_path.contains("SELECT") {
        matched_rules.push("SQL_INJECTION".to_string());
    }
    
    let action = if matched_rules.is_empty() {
        "LOG".to_string()
    } else {
        "BLOCK".to_string()
    };
    
    let confidence = Some(if matched_rules.is_empty() { 0.0 } else { 0.9 });
    
    WafEventResult {
        action,
        matched_rules,
        confidence,
    }
}

/// Proxy request to backend server
async fn proxy_to_backend(
    req: Request<Body>,
    method: &hyper::Method,
    uri: &hyper::Uri,
    client_addr: SocketAddr,
    state: &ProxyState,
    user_agent: &str,
) -> Result<Response<Body>, Infallible> {

    // Create backend request URI
    let backend_uri = format!(
        "http://{}{}",
        state.config.network.backend_addr,
        uri.path_and_query().map(|pq| pq.as_str()).unwrap_or("/")
    );

    debug!(
        client_addr = %client_addr,
        backend_uri = %backend_uri,
        "Proxying request to backend"
    );

    // Create new request for backend
    let mut backend_req = Request::builder()
        .method(method.clone())
        .uri(backend_uri.clone());

    // Copy headers from original request
    for (key, value) in req.headers() {
        if key != "host" {
            backend_req = backend_req.header(key, value);
        }
    }

    // Add X-Forwarded-For header
    backend_req = backend_req.header("X-Forwarded-For", client_addr.ip().to_string());
    backend_req = backend_req.header("Host", state.config.network.backend_addr.to_string());

    let backend_request = backend_req
        .body(req.into_body())
        .unwrap();

    // Send request to backend
    match state.http_client.request(backend_request).await {
        Ok(response) => {
            state.metrics.requests_proxied.fetch_add(1, Ordering::Relaxed);
            counter!("requests_proxied", 1);
            
            info!(
                client_addr = %client_addr,
                backend_uri = %backend_uri,
                status = %response.status(),
                user_agent = %user_agent,
                "Request proxied successfully"
            );
            
            Ok(response)
        }
        Err(e) => {
            state.metrics.http_errors.fetch_add(1, Ordering::Relaxed);
            counter!("http_errors", 1);
            
            error!(
                client_addr = %client_addr,
                backend_uri = %backend_uri,
                error = %e,
                "Failed to proxy request to backend"
            );
            
            let error_response = Response::builder()
                .status(StatusCode::BAD_GATEWAY)
                .body(Body::from("Backend server unavailable"))
                .unwrap();
            Ok(error_response)
        }
    }
}

/// Publish security event to NATS for fleet-wide analysis
async fn publish_security_event(
    state: &ProxyState,
    source_ip: IpAddr,
    method: &hyper::Method,
    uri: &hyper::Uri,
    host_header: &Option<String>,
    user_agent: &str,
    waf_result: &WafEventResult,
    response_status: Option<u16>,
    processing_time: Duration,
) {
    if let Some(ref event_system) = state.event_system {
        let event = SecurityEvent {
            node_id: event_system.node_id,
            timestamp: chrono::Utc::now(),
            source_ip,
            http_method: method.to_string(),
            uri: uri.to_string(),
            host_header: host_header.clone(),
            user_agent: Some(user_agent.to_string()),
            waf_result: waf_result.clone(),
            request_size: None, // Could be calculated from body if needed
            response_status,
            processing_time_ms: Some(processing_time.as_millis() as u64),
        };
        
        if let Err(e) = event_system.publish_security_event(event).await {
            warn!(error = %e, "Failed to publish security event to NATS");
        }
    }
}

/// Load TLS configuration from certificate and key files
async fn load_tls_config(tls_config: &config::TlsConfig) -> Result<ServerConfig> {
    info!(
        cert_path = %tls_config.cert_path,
        key_path = %tls_config.key_path,
        "Loading TLS configuration"
    );

    // Load certificate
    let cert_file = File::open(&tls_config.cert_path)
        .with_context(|| format!("Failed to open certificate file: {}", tls_config.cert_path))?;
    let mut cert_reader = BufReader::new(cert_file);
    let certs = certs(&mut cert_reader)
        .with_context(|| "Failed to parse certificate file")?
        .into_iter()
        .map(Certificate)
        .collect();

    // Load private key
    let key_file = File::open(&tls_config.key_path)
        .with_context(|| format!("Failed to open private key file: {}", tls_config.key_path))?;
    let mut key_reader = BufReader::new(key_file);
    let keys = pkcs8_private_keys(&mut key_reader)
        .with_context(|| "Failed to parse private key file")?;
    
    if keys.is_empty() {
        return Err(anyhow::anyhow!("No private keys found in key file"));
    }
    
    let private_key = PrivateKey(keys[0].clone());

    // Build TLS configuration
    let config = ServerConfig::builder()
        .with_safe_defaults()
        .with_no_client_auth()
        .with_single_cert(certs, private_key)
        .context("Failed to build TLS configuration")?;

    info!(
        cert_path = %tls_config.cert_path,
        key_path = %tls_config.key_path,
        "TLS configuration loaded successfully"
    );

    Ok(config)
}

/// Initialize metrics descriptions
fn initialize_metrics() {
    describe_counter!(
        "https_requests_received",
        "Total number of HTTPS requests received"
    );
    describe_counter!(
        "requests_proxied",
        "Total number of requests proxied to backend"
    );
    describe_counter!(
        "tls_handshakes_completed",
        "Total number of TLS handshakes completed"
    );
    describe_counter!(
        "tls_handshake_errors",
        "Total number of TLS handshake errors"
    );
    describe_counter!(
        "http_errors",
        "Total number of HTTP processing errors"
    );
    describe_counter!(
        "waf_requests_blocked",
        "Total number of requests blocked by WAF"
    );
    describe_counter!(
        "l7_proxy_blocked_rate_limit",
        "Connections blocked by rate limiting at L7"
    );
    describe_counter!(
        "l7_proxy_blocked_connection_limit",
        "Connections blocked by connection limits at L7"
    );
    describe_counter!(
        "l7_proxy_blocked_blacklist",
        "Connections blocked by IP blacklist at L7"
    );
    describe_counter!(
        "l7_proxy_blocked_global_limit",
        "Connections blocked by global limits at L7"
    );
    describe_gauge!(
        "active_connections",
        "Current number of active connections"
    );
}

/// Start Prometheus metrics server
async fn start_metrics_server(listen_addr: SocketAddr, metrics: ProxyMetrics) -> Result<()> {
    info!(
        metrics_addr = %listen_addr,
        "Starting Prometheus metrics server"
    );

    // Install Prometheus exporter
    let builder = metrics_exporter_prometheus::PrometheusBuilder::new();
    let _handle = builder
        .with_http_listener(listen_addr)
        .install()
        .context("Failed to install Prometheus exporter")?;

    info!(
        metrics_addr = %listen_addr,
        "Prometheus metrics server started"
    );

    // Update metrics periodically
    let mut interval = tokio::time::interval(Duration::from_secs(5));
    loop {
        interval.tick().await;
        
        // Update gauge metrics
        gauge!("active_connections", metrics.active_connections.load(Ordering::Relaxed) as f64);
        gauge!("https_requests_received", metrics.https_requests_received.load(Ordering::Relaxed) as f64);
        gauge!("tls_handshakes_completed", metrics.tls_handshakes_completed.load(Ordering::Relaxed) as f64);
    }
}
