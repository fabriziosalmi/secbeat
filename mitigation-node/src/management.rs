use anyhow::{Context, Result};
use axum::{
    extract::{Request, State},
    http::{header::AUTHORIZATION, StatusCode},
    middleware::{self, Next},
    response::{Json, Response},
    routing::{delete, get, post},
    Router,
};
use serde::{Deserialize, Serialize};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};
use tokio::{
    net::TcpListener,
    sync::{oneshot, RwLock},
    time::{sleep, Duration},
};
use tracing::{error, info, instrument, warn};

use crate::config::ManagementApiConfig;
use crate::events::EventSystem;
use crate::waf::WafEngine;

/// Shared shutdown signal for graceful termination
#[derive(Debug)]
pub struct ShutdownSignal {
    /// Atomic flag indicating shutdown has been initiated
    pub should_shutdown: Arc<AtomicBool>,
}

impl Clone for ShutdownSignal {
    fn clone(&self) -> Self {
        Self {
            should_shutdown: Arc::clone(&self.should_shutdown),
        }
    }
}

impl ShutdownSignal {
    pub fn new() -> (Self, oneshot::Receiver<()>) {
        let (sender, receiver) = oneshot::channel();
        let signal = Self {
            should_shutdown: Arc::new(AtomicBool::new(false)),
        };

        // Spawn a task to trigger the receiver when shutdown is initiated
        let should_shutdown = Arc::clone(&signal.should_shutdown);
        tokio::spawn(async move {
            while !should_shutdown.load(Ordering::Relaxed) {
                tokio::time::sleep(Duration::from_millis(100)).await;
            }
            let _ = sender.send(());
        });

        (signal, receiver)
    }

    pub fn is_shutdown_requested(&self) -> bool {
        self.should_shutdown.load(Ordering::Relaxed)
    }

    pub fn initiate_shutdown(&self) {
        self.should_shutdown.store(true, Ordering::Relaxed);
    }
}

/// Management API state
#[derive(Debug, Clone)]
pub struct ManagementState {
    /// Configuration
    pub config: ManagementApiConfig,
    /// Shutdown signal
    pub shutdown_signal: ShutdownSignal,
    /// WAF engine for runtime updates
    pub waf_engine: Option<Arc<RwLock<WafEngine>>>,
    /// Event system for publishing control commands
    pub event_system: Option<Arc<EventSystem>>,
    /// Configuration file path for reloading
    pub config_file_path: Option<String>,
}

/// Termination command payload
#[derive(Debug, Deserialize)]
pub struct TerminationCommand {
    /// Reason for termination
    pub reason: String,
    /// Timestamp of the command
    pub timestamp: String,
    /// Grace period in seconds
    pub grace_period_seconds: u64,
}

/// Termination response
#[derive(Debug, Serialize)]
pub struct TerminationResponse {
    /// Success status
    pub success: bool,
    /// Response message
    pub message: String,
    /// Actual grace period that will be used
    pub grace_period_seconds: u64,
}

/// WAF rule management request
#[derive(Debug, Deserialize)]
pub struct WafRuleRequest {
    /// Pattern to add/remove
    pub pattern: String,
    /// Rule type (optional, defaults to "custom")
    pub rule_type: Option<String>,
}

/// WAF rule response
#[derive(Debug, Serialize)]
pub struct WafRuleResponse {
    /// Success status
    pub success: bool,
    /// Response message
    pub message: String,
    /// Number of patterns affected (for remove operations)
    pub patterns_affected: Option<usize>,
}

/// Configuration reload request
#[derive(Debug, Deserialize)]
pub struct ConfigReloadRequest {
    /// Force reload even if no changes detected
    pub force: Option<bool>,
}

/// Configuration reload response
#[derive(Debug, Serialize)]
pub struct ConfigReloadResponse {
    /// Success status
    pub success: bool,
    /// Response message
    pub message: String,
    /// Timestamp of reload
    pub timestamp: String,
}

/// Health check response
#[derive(Debug, Serialize)]
pub struct HealthResponse {
    /// Service status
    pub status: String,
    /// WAF status
    pub waf_enabled: bool,
    /// Number of active connections
    pub active_connections: u64,
    /// Uptime in seconds
    pub uptime_seconds: u64,
}

/// WAF statistics response
#[derive(Debug, Serialize)]
pub struct WafStatsResponse {
    /// WAF enabled status
    pub enabled: bool,
    /// SQL injection patterns count
    pub sql_patterns: u32,
    /// XSS patterns count
    pub xss_patterns: u32,
    /// Path traversal patterns count
    pub path_traversal_patterns: u32,
    /// Command injection patterns count
    pub command_injection_patterns: u32,
    /// Custom patterns count
    pub custom_patterns: u32,
}

/// Authentication middleware
async fn auth_middleware(
    State(state): State<ManagementState>,
    request: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    // Extract Authorization header
    let auth_header = request
        .headers()
        .get(AUTHORIZATION)
        .and_then(|h| h.to_str().ok());

    match auth_header {
        Some(auth) if auth.starts_with("Bearer ") => {
            let token = &auth[7..]; // Remove "Bearer " prefix
            if token == state.config.auth_token.as_deref().unwrap_or("") {
                Ok(next.run(request).await)
            } else {
                warn!("Invalid management API token provided");
                Err(StatusCode::UNAUTHORIZED)
            }
        }
        _ => {
            warn!("Missing or invalid Authorization header");
            Err(StatusCode::UNAUTHORIZED)
        }
    }
}

/// Handle termination command
#[instrument(skip(state))]
async fn handle_terminate(
    State(state): State<ManagementState>,
    Json(command): Json<TerminationCommand>,
) -> Result<Json<TerminationResponse>, StatusCode> {
    info!(
        reason = %command.reason,
        grace_period = command.grace_period_seconds,
        "Received termination command"
    );

    // Use the configured grace period, or the requested one if smaller
    let grace_period = std::cmp::min(
        command.grace_period_seconds,
        state.config.shutdown_grace_period_seconds.unwrap_or(30),
    );

    // Signal shutdown initiation
    state.shutdown_signal.initiate_shutdown();

    // Spawn graceful shutdown task
    let shutdown_signal = state.shutdown_signal.clone();
    let termination_reason = command.reason.clone();
    tokio::spawn(async move {
        perform_graceful_shutdown(shutdown_signal, termination_reason, grace_period).await;
    });

    let response = TerminationResponse {
        success: true,
        message: format!("Graceful shutdown initiated with {grace_period} second grace period"),
        grace_period_seconds: grace_period,
    };

    Ok(Json(response))
}

/// Perform graceful shutdown process
#[instrument(skip(_shutdown_signal))]
async fn perform_graceful_shutdown(
    _shutdown_signal: ShutdownSignal,
    reason: String,
    grace_period_seconds: u64,
) {
    info!(
        reason = %reason,
        grace_period = grace_period_seconds,
        "Starting graceful shutdown process"
    );

    // Wait for the grace period to allow in-flight requests to complete
    info!(
        grace_period = grace_period_seconds,
        "Waiting for grace period"
    );
    sleep(Duration::from_secs(grace_period_seconds)).await;

    // Log final shutdown message
    info!(reason = %reason, "Grace period completed, terminating process");

    // Send final heartbeat with terminating status if possible
    // (This would integrate with the heartbeat system)

    // Exit the process
    std::process::exit(0);
}

/// Create management API router
fn create_management_router(state: ManagementState) -> Router {
    Router::new()
        // Control endpoints
        .route("/control/terminate", post(handle_terminate))
        // Health and status endpoints
        .route("/health", get(health_check))
        .route("/status/waf", get(handle_waf_stats))
        // WAF management endpoints
        .route("/waf/patterns", post(handle_add_waf_pattern))
        .route("/waf/patterns", delete(handle_remove_waf_pattern))
        .route("/waf/reload", post(handle_reload_waf_patterns))
        // Configuration management endpoints
        .route("/config/reload", post(handle_config_reload))
        .layer(middleware::from_fn_with_state(
            state.clone(),
            auth_middleware,
        ))
        .with_state(state)
}

/// Start the management API server
#[instrument(skip(config, shutdown_signal, waf_engine, event_system))]
pub async fn start_management_api(
    config: ManagementApiConfig,
    shutdown_signal: ShutdownSignal,
    waf_engine: Option<Arc<RwLock<WafEngine>>>,
    event_system: Option<Arc<EventSystem>>,
    config_file_path: Option<String>,
) -> Result<()> {
    if !config.enabled {
        info!("Management API is disabled");
        return Ok(());
    }

    info!(
        listen_addr = %config.listen_addr,
        "Starting management API server"
    );

    let state = ManagementState {
        config: config.clone(),
        shutdown_signal,
        waf_engine,
        event_system,
        config_file_path,
    };

    let app = create_management_router(state);

    let listener = TcpListener::bind(&config.listen_addr)
        .await
        .with_context(|| format!("Failed to bind to {}", config.listen_addr))?;

    info!(
        addr = %config.listen_addr,
        "Management API server listening"
    );

    // Start the server
    if let Err(e) = axum::serve(listener, app).await {
        error!(error = %e, "Management API server failed");
        return Err(e.into());
    }

    Ok(())
}

/// Health check endpoint for management API
async fn health_check() -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "status": "healthy",
        "service": "mitigation-node-management",
        "timestamp": chrono::Utc::now().to_rfc3339()
    }))
}

/// Handle WAF pattern addition
#[instrument(skip(state))]
async fn handle_add_waf_pattern(
    State(state): State<ManagementState>,
    Json(request): Json<WafRuleRequest>,
) -> Result<Json<WafRuleResponse>, StatusCode> {
    info!(pattern = %request.pattern, "Adding WAF pattern");

    if let Some(ref waf_engine) = state.waf_engine {
        let mut waf = waf_engine.write().await;
        match waf.add_custom_pattern(&request.pattern).await {
            Ok(_) => {
                info!(pattern = %request.pattern, "Successfully added WAF pattern");
                Ok(Json(WafRuleResponse {
                    success: true,
                    message: "Pattern added successfully".to_string(),
                    patterns_affected: Some(1),
                }))
            }
            Err(e) => {
                warn!(pattern = %request.pattern, error = %e, "Failed to add WAF pattern");
                Ok(Json(WafRuleResponse {
                    success: false,
                    message: format!("Failed to add pattern: {}", e),
                    patterns_affected: Some(0),
                }))
            }
        }
    } else {
        warn!("WAF engine not available");
        Ok(Json(WafRuleResponse {
            success: false,
            message: "WAF engine not available".to_string(),
            patterns_affected: Some(0),
        }))
    }
}

/// Handle WAF pattern removal
#[instrument(skip(state))]
async fn handle_remove_waf_pattern(
    State(state): State<ManagementState>,
    Json(request): Json<WafRuleRequest>,
) -> Result<Json<WafRuleResponse>, StatusCode> {
    info!(pattern = %request.pattern, "Removing WAF pattern");

    if let Some(ref waf_engine) = state.waf_engine {
        let mut waf = waf_engine.write().await;
        match waf.remove_custom_pattern(&request.pattern).await {
            Ok(removed_count) => {
                info!(pattern = %request.pattern, count = removed_count, "Successfully removed WAF patterns");
                Ok(Json(WafRuleResponse {
                    success: true,
                    message: format!("Removed {} matching patterns", removed_count),
                    patterns_affected: Some(removed_count),
                }))
            }
            Err(e) => {
                warn!(pattern = %request.pattern, error = %e, "Failed to remove WAF pattern");
                Ok(Json(WafRuleResponse {
                    success: false,
                    message: format!("Failed to remove pattern: {}", e),
                    patterns_affected: Some(0),
                }))
            }
        }
    } else {
        warn!("WAF engine not available");
        Ok(Json(WafRuleResponse {
            success: false,
            message: "WAF engine not available".to_string(),
            patterns_affected: Some(0),
        }))
    }
}

/// Handle WAF pattern reload
#[instrument(skip(state))]
async fn handle_reload_waf_patterns(
    State(state): State<ManagementState>,
) -> Result<Json<ConfigReloadResponse>, StatusCode> {
    info!("Reloading WAF patterns from configuration");

    if let Some(ref waf_engine) = state.waf_engine {
        let mut waf = waf_engine.write().await;
        match waf.reload_patterns().await {
            Ok(_) => {
                info!("Successfully reloaded WAF patterns");
                Ok(Json(ConfigReloadResponse {
                    success: true,
                    message: "WAF patterns reloaded successfully".to_string(),
                    timestamp: chrono::Utc::now().to_rfc3339(),
                }))
            }
            Err(e) => {
                warn!(error = %e, "Failed to reload WAF patterns");
                Ok(Json(ConfigReloadResponse {
                    success: false,
                    message: format!("Failed to reload patterns: {}", e),
                    timestamp: chrono::Utc::now().to_rfc3339(),
                }))
            }
        }
    } else {
        warn!("WAF engine not available");
        Ok(Json(ConfigReloadResponse {
            success: false,
            message: "WAF engine not available".to_string(),
            timestamp: chrono::Utc::now().to_rfc3339(),
        }))
    }
}

/// Handle configuration reload
#[instrument(skip(state))]
async fn handle_config_reload(
    State(state): State<ManagementState>,
    Json(request): Json<ConfigReloadRequest>,
) -> Result<Json<ConfigReloadResponse>, StatusCode> {
    info!(force = ?request.force, "Reloading configuration");

    // Try to reload configuration from file if available
    let config_reloaded = if let Some(ref config_path) = state.config_file_path {
        match crate::config::MitigationConfig::from_file(config_path) {
            Ok(new_config) => {
                info!(path = %config_path, "Configuration reloaded from file");

                // Validate the new configuration
                if let Err(e) = new_config.validate() {
                    warn!(error = %e, "New configuration validation failed");
                    return Ok(Json(ConfigReloadResponse {
                        success: false,
                        message: format!("Configuration validation failed: {}", e),
                        timestamp: chrono::Utc::now().to_rfc3339(),
                    }));
                }

                // TODO: In a full implementation, we would:
                // 1. Apply the new configuration to running services
                // 2. Update DDoS protection settings
                // 3. Update TLS configuration if needed
                // 4. Update backend server configuration
                // 5. Restart services that require it

                info!("Configuration validated and would be applied");
                true
            }
            Err(e) => {
                warn!(error = %e, path = %config_path, "Failed to reload configuration from file");
                false
            }
        }
    } else {
        warn!("No configuration file path available for reload");
        false
    };

    // Reload WAF patterns if WAF engine is available
    let waf_reloaded = if let Some(ref waf_engine) = state.waf_engine {
        let mut waf = waf_engine.write().await;
        match waf.reload_patterns().await {
            Ok(_) => {
                info!("WAF patterns reloaded successfully");
                true
            }
            Err(e) => {
                warn!(error = %e, "Failed to reload WAF patterns");
                false
            }
        }
    } else {
        info!("WAF engine not available for reload");
        true // Not an error if WAF is disabled
    };

    if config_reloaded && waf_reloaded {
        Ok(Json(ConfigReloadResponse {
            success: true,
            message: "Configuration and WAF patterns reloaded successfully".to_string(),
            timestamp: chrono::Utc::now().to_rfc3339(),
        }))
    } else if waf_reloaded {
        Ok(Json(ConfigReloadResponse {
            success: true,
            message: "WAF patterns reloaded successfully (configuration file not available)"
                .to_string(),
            timestamp: chrono::Utc::now().to_rfc3339(),
        }))
    } else {
        Ok(Json(ConfigReloadResponse {
            success: false,
            message: "Failed to reload configuration and/or WAF patterns".to_string(),
            timestamp: chrono::Utc::now().to_rfc3339(),
        }))
    }
}

/// Handle WAF statistics request
#[instrument(skip(state))]
async fn handle_waf_stats(
    State(state): State<ManagementState>,
) -> Result<Json<WafStatsResponse>, StatusCode> {
    if let Some(ref waf_engine) = state.waf_engine {
        let waf = waf_engine.read().await;
        let stats = waf.get_stats().await;
        Ok(Json(WafStatsResponse {
            enabled: stats.enabled,
            sql_patterns: stats.sql_patterns,
            xss_patterns: stats.xss_patterns,
            path_traversal_patterns: stats.path_traversal_patterns,
            command_injection_patterns: stats.command_injection_patterns,
            custom_patterns: stats.custom_patterns,
        }))
    } else {
        Ok(Json(WafStatsResponse {
            enabled: false,
            sql_patterns: 0,
            xss_patterns: 0,
            path_traversal_patterns: 0,
            command_injection_patterns: 0,
            custom_patterns: 0,
        }))
    }
}
