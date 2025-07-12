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
use tracing::{debug, error, info, warn};

mod config;
mod ddos;
mod waf;
mod orchestrator;
mod events;
mod management;
mod tcp_proxy;
mod syn_proxy;

use config::{MitigationConfig, ManagementApiConfig};
use ddos::{DdosProtection, DdosCheckResult};
use orchestrator::OrchestratorClient;
use events::{EventSystem, SecurityEvent, WafEventResult};
use tcp_proxy::TcpProxy;
use syn_proxy::SynProxy;

// Helper macro for safely updating metrics
macro_rules! update_metric {
    ($state:expr, $field:ident, $op:ident, $value:expr) => {
        if let Some(ref metrics) = $state.metrics {
            metrics.$field.$op($value, Ordering::Relaxed);
        }
    };
}

// Helper macro for safely reading metrics
macro_rules! read_metric {
    ($state:expr, $field:ident) => {
        $state.metrics.as_ref().map(|m| m.$field.load(Ordering::Relaxed)).unwrap_or(0)
    };
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

/// L7 TLS/HTTP Proxy state management
#[derive(Debug, Clone)]
struct ProxyState {
    /// Configuration
    config: MitigationConfig,
    /// DDoS protection engine (optional based on feature toggles)
    ddos_protection: Option<Arc<DdosProtection>>,
    /// WAF engine (optional based on feature toggles)
    waf_engine: Option<Arc<waf::WafEngine>>,
    /// Global metrics counters (optional based on feature toggles)
    metrics: Option<ProxyMetrics>,
    /// HTTP client for backend connections
    http_client: Client<hyper_rustls::HttpsConnector<hyper::client::HttpConnector>>,
    /// Orchestrator client (optional based on feature toggles)
    #[allow(dead_code)] // Will be used when orchestrator is implemented
    orchestrator_client: Option<Arc<OrchestratorClient>>,
    /// Event system for NATS communication (optional based on feature toggles)
    event_system: Option<Arc<EventSystem>>,
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

    info!("Starting SecBeat Mitigation Node v{} - Production-Grade Security Platform", env!("CARGO_PKG_VERSION"));

    // Determine config file path - support unified config system
    let config_name = std::env::var("SECBEAT_CONFIG").unwrap_or_else(|_| {
        // Fallback to legacy environment variable for backward compatibility
        std::env::var("MITIGATION_CONFIG").unwrap_or_else(|_| {
            // Auto-detect config based on environment
            match std::env::var("DEPLOYMENT_ENV").as_deref() {
                Ok("production") => "config.prod".to_string(),
                Ok("development") => "config.dev".to_string(),
                _ => "config.dev".to_string(), // Default to development
            }
        })
    });
    
    // Try root-level config first (unified), then fallback to mitigation-node config
    let config_paths = vec![
        format!("{}.toml", config_name),
        format!("mitigation-node/config/{}.toml", config_name),
        format!("mitigation-node/config/default.toml"), // Final fallback
    ];
    
    let mut config = None;
    for config_path in config_paths {
        match MitigationConfig::from_file(&config_path.replace(".toml", "")) {
            Ok(loaded_config) => {
                info!("Configuration loaded from {}", config_path);
                config = Some(loaded_config);
                break;
            }
            Err(e) => {
                debug!("Failed to load config from {}: {}", config_path, e);
            }
        }
    }
    
    let config = config.unwrap_or_else(|| {
        warn!("No configuration file found, using defaults");
        MitigationConfig::default()
    });

    // Validate configuration
    if let Err(e) = config.validate() {
        error!("Configuration validation failed: {}", e);
        return Err(anyhow::anyhow!("Invalid configuration: {}", e));
    }

    info!(
        environment = %config.platform.environment,
        deployment_id = %config.platform.deployment_id,
        features = ?config.platform.features,
        "SecBeat platform initialized"
    );

    // Determine operation mode based on feature toggles and configuration
    if config.syn_proxy_enabled() {
        info!("Starting in SYN Proxy mode (Layer 4 Protection)");
        run_syn_proxy_mode(config).await
    } else if config.tls_enabled() {
        info!("Starting in L7 TLS/HTTP Proxy mode (Full Feature Set)");
        run_l7_proxy_mode(config).await
    } else {
        info!("Starting in basic TCP Proxy mode (Minimal Features)");
        run_tcp_proxy_mode(config).await
    }
}

async fn run_l7_proxy_mode(config: MitigationConfig) -> Result<()> {
    info!(
        listen_addr = ?config.listen_addr()?,
        backend_addr = ?config.backend_addr()?,
        tls_enabled = config.tls_enabled(),
        ddos_enabled = config.ddos_enabled(),
        waf_enabled = config.waf_enabled(),
        orchestrator_enabled = config.orchestrator_enabled(),
        nats_enabled = config.nats_enabled(),
        "Initializing L7 TLS/HTTP Proxy with feature toggles"
    );

    // Initialize DDoS protection if enabled
    let ddos_protection = if config.ddos_enabled() {
        Some(Arc::new(DdosProtection::new(config.ddos.clone())
            .context("Failed to initialize DDoS protection")?))
    } else {
        info!("DDoS protection disabled by feature toggle");
        None
    };

    // Initialize WAF engine if enabled
    let waf_engine = if config.waf_enabled() {
        Some(Arc::new(waf::WafEngine::new(config.waf.clone())
            .context("Failed to initialize WAF engine")?))
    } else {
        info!("WAF protection disabled by feature toggle");
        None
    };

    // Initialize metrics if enabled
    let metrics = if config.metrics_enabled() {
        Some(ProxyMetrics::new())
    } else {
        info!("Metrics collection disabled by feature toggle");
        None
    };

    // Setup metrics descriptions if enabled
    if config.metrics_enabled() {
        describe_counter!("https_requests_total", "Total HTTPS requests received");
        describe_counter!("requests_proxied_total", "Total requests proxied to backend");
        describe_counter!("tls_handshakes_completed_total", "Total TLS handshakes completed");
        describe_counter!("tls_handshake_errors_total", "Total TLS handshake errors");
        describe_counter!("http_errors_total", "Total HTTP errors");
        describe_gauge!("active_connections", "Currently active connections");
        describe_counter!("blocked_requests_total", "Total blocked requests");
        describe_counter!("ddos_events_detected_total", "DDoS events detected");
        describe_counter!("waf_events_blocked_total", "WAF events blocked");
    }

    // Initialize HTTP client
    let https_connector = hyper_rustls::HttpsConnectorBuilder::new()
        .with_native_roots()
        .https_only()
        .enable_http1()
        .build();
    let http_client = Client::builder()
        .pool_max_idle_per_host(20)
        .build(https_connector);

    // Create proxy state
    let state = ProxyState {
        config: config.clone(),
        ddos_protection,
        waf_engine,
        metrics: metrics.clone(),
        http_client,
        orchestrator_client: None, // Will be initialized if orchestrator is enabled
        event_system: None, // Will be initialized if NATS is enabled
    };

    // Load TLS configuration if TLS is enabled
    if !config.tls_enabled() {
        return Err(anyhow::anyhow!("L7 proxy mode requires TLS to be enabled"));
    }

    let tls_config = load_tls_config(&config.network.tls).await
        .context("Failed to load TLS configuration")?;
    let tls_acceptor = TlsAcceptor::from(Arc::new(tls_config));

    // Create TCP listener
    let listen_addr = config.listen_addr()?;
    let tcp_listener = TcpListener::bind(&listen_addr).await
        .with_context(|| format!("Failed to bind to {listen_addr}"))?;

    info!(
        listen_addr = %listen_addr,
        "L7 TLS/HTTP proxy server started and listening for connections"
    );

    // Start background tasks if enabled
    let mut background_tasks = Vec::new();
    
    // Start orchestrator client if enabled
    if config.orchestrator_enabled() {
        info!("Starting orchestrator client...");
        // TODO: Implement orchestrator client initialization
        // let orchestrator_task = tokio::spawn(orchestrator_client_task(config.clone()));
        // background_tasks.push(orchestrator_task);
    }
    
    // Start NATS event system if enabled
    if config.nats_enabled() {
        info!("Starting NATS event system...");
        // TODO: Implement NATS event system initialization
        // let nats_task = tokio::spawn(nats_event_task(config.clone()));
        // background_tasks.push(nats_task);
    }
    
    // Start metrics server if enabled
    if config.metrics_enabled() {
        if let Some(metrics_instance) = metrics {
            info!("Starting metrics server on {}", config.metrics.listen_addr);
            let metrics_addr: SocketAddr = config.metrics.listen_addr.parse()
                .expect("Invalid metrics listen address");
            let metrics_task = tokio::spawn(start_metrics_server(metrics_addr, metrics_instance));
            background_tasks.push(metrics_task);
        }
    }
    
    // Start management API if enabled
    if config.management_enabled() {
        info!("Starting management API on {}", config.management.listen_addr);
        let mgmt_task = tokio::spawn(start_management_api(config.management.clone()));
        background_tasks.push(mgmt_task);
    }

    // Accept connections in a loop
    loop {
        match tcp_listener.accept().await {
            Ok((tcp_stream, client_addr)) => {
                let client_ip = client_addr.ip();
                
                // DDoS protection check if enabled
                let ddos_result = if let Some(ref ddos_protection) = state.ddos_protection {
                    ddos_protection.check_connection(client_ip)
                } else {
                    DdosCheckResult::Allow // Always allow if DDoS protection is disabled
                };
                
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
                        update_metric!(state, blocked_requests, fetch_add, 1);
                        update_metric!(state, ddos_events_detected, fetch_add, 1);
                        drop(tcp_stream);
                    }
                    DdosCheckResult::ConnectionLimitExceeded => {
                        warn!(client_addr = %client_addr, "Connection blocked - connection limit exceeded");
                        counter!("l7_proxy_blocked_connection_limit", 1);
                        update_metric!(state, blocked_requests, fetch_add, 1);
                        update_metric!(state, ddos_events_detected, fetch_add, 1);
                        drop(tcp_stream);
                    }
                    DdosCheckResult::Blacklisted => {
                        warn!(client_addr = %client_addr, "Connection blocked - IP blacklisted");
                        counter!("l7_proxy_blocked_blacklist", 1);
                        update_metric!(state, blocked_requests, fetch_add, 1);
                        update_metric!(state, ddos_events_detected, fetch_add, 1);
                        drop(tcp_stream);
                    }
                    DdosCheckResult::GlobalLimitExceeded => {
                        warn!(client_addr = %client_addr, "Connection blocked - global limit exceeded");
                        counter!("l7_proxy_blocked_global_limit", 1);
                        update_metric!(state, blocked_requests, fetch_add, 1);
                        update_metric!(state, ddos_events_detected, fetch_add, 1);
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
            update_metric!(state, blocked_requests, fetch_add, 1);
            counter!("dynamic_blocks_total", 1);
            // Drop connection immediately
            return Ok(());
        }
    }
    
    // Increment active connections counter
    update_metric!(state, active_connections, fetch_add, 1);
    gauge!("active_connections", read_metric!(state, active_connections) as f64);
    
    // Perform TLS handshake
    let tls_stream = match tls_acceptor.accept(tcp_stream).await {
        Ok(stream) => {
            update_metric!(state, tls_handshakes_completed, fetch_add, 1);
            counter!("tls_handshakes_completed", 1);
            stream
        }
        Err(e) => {
            update_metric!(state, tls_handshake_errors, fetch_add, 1);
            counter!("tls_handshake_errors", 1);
            error!(client_addr = %client_addr, error = %e, "TLS handshake failed");
            update_metric!(state, active_connections, fetch_sub, 1);
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
        update_metric!(state, http_errors, fetch_add, 1);
    }

    // Decrement active connections counter
    update_metric!(state, active_connections, fetch_sub, 1);
    gauge!("active_connections", read_metric!(state, active_connections) as f64);

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
    update_metric!(state, https_requests_received, fetch_add, 1);
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
    let waf_result = analyze_request_with_waf(&state, &method, &uri, &user_agent, req.headers());
    let response_result = if waf_result.action == "BLOCK" {
        warn!(
            client_addr = %client_addr,
            uri = %uri,
            matched_rules = ?waf_result.matched_rules,
            "Request blocked by WAF - suspicious content detected"
        );
        counter!("waf_requests_blocked", 1);
        update_metric!(state, blocked_requests, fetch_add, 1);
        update_metric!(state, waf_events_blocked, fetch_add, 1);
        
        // Create blocked response
        let response = Response::builder()
            .status(StatusCode::FORBIDDEN)
            .body(Body::from("Request blocked by Web Application Firewall"))
            .unwrap_or_else(|e| {
                error!("Failed to create WAF blocked response: {}", e);
                Response::new(Body::from("Internal Server Error"))
            });
        
        // Publish security event for blocked request
        let event_data = SecurityEventData {
            source_ip: client_addr.ip(),
            method: method.to_string(),
            uri: uri.to_string(),
            host_header: host_header.clone(),
            user_agent: user_agent.to_string(),
            waf_result: waf_result.clone(),
            response_status: Some(403),
            processing_time: start_time.elapsed(),
        };
        publish_security_event(&state, event_data).await;
        
        return Ok(response);
    } else {
        // Request is allowed, proceed with proxying
        proxy_to_backend(req, &method, &uri, client_addr, &state, &user_agent).await
    };
    
    // Publish security event for all requests (blocked ones already published above)
    let status_code = response_result.as_ref().map(|r| r.status().as_u16()).ok();
    let event_data = SecurityEventData {
        source_ip: client_addr.ip(),
        method: method.to_string(),
        uri: uri.to_string(),
        host_header: host_header.clone(),
        user_agent: user_agent.to_string(),
        waf_result: waf_result.clone(),
        response_status: status_code,
        processing_time: start_time.elapsed(),
    };
    publish_security_event(&state, event_data).await;
    
    response_result
}

/// Analyze request with WAF and return result
fn analyze_request_with_waf(
    state: &ProxyState,
    method: &hyper::Method,
    uri: &hyper::Uri,
    _user_agent: &str,
    headers: &hyper::HeaderMap,
) -> WafEventResult {
    // Convert hyper request to WAF HttpRequest format
    let mut headers_map = std::collections::HashMap::new();
    for (name, value) in headers.iter() {
        if let Ok(value_str) = value.to_str() {
            headers_map.insert(name.as_str().to_lowercase(), value_str.to_string());
        }
    }
    
    let http_request = waf::HttpRequest {
        method: method.as_str().to_string(),
        path: uri.path().to_string(),
        version: "HTTP/1.1".to_string(),
        headers: headers_map,
        body: None, // For GET requests, no body to inspect
        query_string: uri.query().map(|s| s.to_string()),
    };
    
    // Use the WAF engine to inspect the request
    let waf_result = if let Some(ref waf_engine) = state.waf_engine {
        waf_engine.inspect_request(&http_request)
    } else {
        // WAF disabled, allow all requests
        waf::WafResult::Allow
    };
    
    // Convert WAF result to WafEventResult
    match waf_result {
        waf::WafResult::Allow => WafEventResult {
            action: "LOG".to_string(),
            matched_rules: vec![],
            confidence: Some(0.0),
        },
        waf::WafResult::SqlInjection => WafEventResult {
            action: "BLOCK".to_string(),
            matched_rules: vec!["SQL_INJECTION".to_string()],
            confidence: Some(0.95),
        },
        waf::WafResult::XssAttempt => WafEventResult {
            action: "BLOCK".to_string(),
            matched_rules: vec!["XSS_ATTEMPT".to_string()],
            confidence: Some(0.90),
        },
        waf::WafResult::PathTraversal => WafEventResult {
            action: "BLOCK".to_string(),
            matched_rules: vec!["PATH_TRAVERSAL".to_string()],
            confidence: Some(0.85),
        },
        waf::WafResult::CommandInjection => WafEventResult {
            action: "BLOCK".to_string(),
            matched_rules: vec!["COMMAND_INJECTION".to_string()],
            confidence: Some(0.90),
        },
        waf::WafResult::CustomPattern(rule) => WafEventResult {
            action: "BLOCK".to_string(),
            matched_rules: vec![format!("CUSTOM_PATTERN: {}", rule)],
            confidence: Some(0.80),
        },
        waf::WafResult::InvalidHttp => WafEventResult {
            action: "BLOCK".to_string(),
            matched_rules: vec!["INVALID_HTTP".to_string()],
            confidence: Some(1.0),
        },
        waf::WafResult::OversizedRequest => WafEventResult {
            action: "BLOCK".to_string(),
            matched_rules: vec!["OVERSIZED_REQUEST".to_string()],
            confidence: Some(1.0),
        },
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
    let backend_addr = match state.config.backend_addr() {
        Ok(addr) => addr,
        Err(e) => {
            error!("Invalid backend address configuration: {}", e);
            return Ok(Response::builder()
                .status(StatusCode::INTERNAL_SERVER_ERROR)
                .body(Body::from("Configuration error"))
                .unwrap_or_else(|_| Response::new(Body::from("Internal Server Error"))));
        }
    };
    let backend_uri = format!(
        "http://{}{}",
        backend_addr,
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
    backend_req = backend_req.header("Host", backend_addr.to_string());

    let backend_request = backend_req
        .body(req.into_body())
        .unwrap_or_else(|e| {
            error!("Failed to create backend request: {}", e);
            // Return a dummy request that will be handled as an error
            hyper::Request::builder()
                .method("GET")
                .uri("http://invalid")
                .body(Body::empty())
                .expect("Creating fallback request should never fail")
        });

    // Send request to backend
    match state.http_client.request(backend_request).await {
        Ok(response) => {
            update_metric!(state, requests_proxied, fetch_add, 1);
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
            update_metric!(state, http_errors, fetch_add, 1);
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
                .unwrap_or_else(|e| {
                    error!("Failed to create error response: {}", e);
                    Response::new(Body::from("Internal Server Error"))
                });
            Ok(error_response)
        }
    }
}

/// Security event data for NATS publishing
#[derive(Debug)]
struct SecurityEventData {
    source_ip: IpAddr,
    method: String,
    uri: String,
    host_header: Option<String>,
    user_agent: String,
    waf_result: WafEventResult,
    response_status: Option<u16>,
    processing_time: Duration,
}

/// Publish security event to NATS for fleet-wide analysis
async fn publish_security_event(
    state: &ProxyState,
    event_data: SecurityEventData,
) {
    if let Some(ref event_system) = state.event_system {
        let event = SecurityEvent {
            node_id: event_system.node_id,
            timestamp: chrono::Utc::now(),
            source_ip: event_data.source_ip,
            http_method: event_data.method,
            uri: event_data.uri,
            host_header: event_data.host_header,
            user_agent: Some(event_data.user_agent),
            waf_result: event_data.waf_result,
            request_size: None, // Could be calculated from body if needed
            response_status: event_data.response_status,
            processing_time_ms: Some(event_data.processing_time.as_millis() as u64),
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
    builder
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

/// Run in basic TCP proxy mode (Phase 1)
async fn run_tcp_proxy_mode(config: MitigationConfig) -> Result<()> {
    info!(
        listen_addr = %config.listen_addr()?,
        backend_addr = %config.backend_addr()?,
        "Initializing basic TCP proxy"
    );

    let proxy = TcpProxy::new(
        config.listen_addr()?,
        config.backend_addr()?,
        config.network.buffer_size,
    );

    proxy.run().await
}

/// Run in SYN proxy mode (Phase 2)
async fn run_syn_proxy_mode(config: MitigationConfig) -> Result<()> {
    info!(
        listen_port = config.ddos.syn_proxy.listen_port,
        backend_addr = %config.backend_addr()?,
        "Initializing SYN proxy"
    );

    // For now, use a default key since we need hex dependency
    let key_array = [0u8; 32]; // In production, this would be from config

    let mut proxy = SynProxy::new(
        key_array,
        config.ddos.syn_proxy.listen_port,
        config.backend_addr()?,
        std::time::Duration::from_millis(config.ddos.syn_proxy.handshake_timeout_ms),
    );

    proxy.initialize().await?;
    proxy.run().await
}

/// Helper functions for background tasks

async fn start_management_api(config: ManagementApiConfig) -> Result<()> {
    info!("Management API would start on {} (placeholder)", config.listen_addr);
    // TODO: Implement actual management API
    // For now, just keep the task alive
    loop {
        tokio::time::sleep(tokio::time::Duration::from_secs(60)).await;
    }
}
