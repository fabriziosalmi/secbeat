use anyhow::{Context, Result};
use axum::{
    extract::{Request, State},
    http::{header::AUTHORIZATION, StatusCode},
    middleware::{self, Next},
    response::{Json, Response},
    routing::post,
    Router,
};
use serde::{Deserialize, Serialize};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};
use tokio::{
    net::TcpListener,
    sync::oneshot,
    time::{sleep, Duration},
};
use tracing::{error, info, instrument, warn};

use crate::config::ManagementApiConfig;

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
            if token == state.config.auth_token {
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
        state.config.shutdown_grace_period,
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
        message: format!("Graceful shutdown initiated with {} second grace period", grace_period),
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
    info!(grace_period = grace_period_seconds, "Waiting for grace period");
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
        .route("/control/terminate", post(handle_terminate))
        .layer(middleware::from_fn_with_state(state.clone(), auth_middleware))
        .with_state(state)
}

/// Start the management API server
#[instrument(skip(config, shutdown_signal))]
pub async fn start_management_api(
    config: ManagementApiConfig,
    shutdown_signal: ShutdownSignal,
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
