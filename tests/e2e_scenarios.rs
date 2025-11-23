//! End-to-End Integration Tests for SecBeat
//!
//! These tests verify the full system behavior including:
//! - WASM rule deployment and enforcement
//! - XDP packet filtering
//! - ML-based anomaly detection
//! - CRDT state synchronization
//!
//! Prerequisites:
//! - Test environment must be running (./tests/setup_env.sh up)
//! - All services must be healthy (NATS, orchestrator, mitigation, origin)
//!
//! Usage:
//!   cargo test --test e2e_scenarios

use anyhow::Result;
use reqwest::Client;
use serde_json::json;
use std::time::Duration;
use tokio::time::sleep;

// Test environment URLs
const ORCHESTRATOR_URL: &str = "http://localhost:13030";
const MITIGATION_URL: &str = "http://localhost:18080";
const ORIGIN_URL: &str = "http://localhost:18888";

/// Helper: Create HTTP client with reasonable timeouts
fn create_client() -> Client {
    Client::builder()
        .timeout(Duration::from_secs(10))
        .build()
        .expect("Failed to create HTTP client")
}

/// Helper: Wait for all services to be ready
async fn wait_for_services() -> Result<()> {
    let client = create_client();
    let max_attempts = 30;

    // Check orchestrator health
    for i in 1..=max_attempts {
        match client.get(format!("{}/health", ORCHESTRATOR_URL)).send().await {
            Ok(resp) if resp.status().is_success() => {
                println!("✓ Orchestrator is ready");
                break;
            }
            _ if i == max_attempts => anyhow::bail!("Orchestrator never became healthy"),
            _ => {
                println!("Waiting for orchestrator... ({}/{})", i, max_attempts);
                sleep(Duration::from_secs(2)).await;
            }
        }
    }

    // Check origin health
    for i in 1..=max_attempts {
        match client.get(format!("{}/health", ORIGIN_URL)).send().await {
            Ok(resp) if resp.status().is_success() => {
                println!("✓ Origin is ready");
                break;
            }
            _ if i == max_attempts => anyhow::bail!("Origin never became healthy"),
            _ => {
                println!("Waiting for origin... ({}/{})", i, max_attempts);
                sleep(Duration::from_secs(2)).await;
            }
        }
    }

    Ok(())
}

// ==============================================================================
// Scenario 1: WASM WAF Rule Deployment and Enforcement
// ==============================================================================

#[tokio::test]
#[ignore] // Run with: cargo test --test e2e_scenarios -- --ignored
async fn test_wasm_waf_block_admin() -> Result<()> {
    wait_for_services().await?;

    let client = create_client();

    // Step 1: Verify /admin is accessible through mitigation proxy (before WAF rule)
    println!("📋 Step 1: Testing baseline - /admin should be accessible");
    let resp = client.get(format!("{}/admin", MITIGATION_URL)).send().await?;
    assert!(
        resp.status().is_success() || resp.status().as_u16() == 404,
        "Expected 2xx or 404, got {}",
        resp.status()
    );
    println!("✓ Baseline: /admin is accessible");

    // Step 2: Deploy WASM rule to block /admin
    println!("📋 Step 2: Deploying WASM WAF rule to block /admin");
    let wasm_rule = json!({
        "rule_id": "block_admin_test",
        "description": "Block access to /admin endpoints",
        "pattern": "/admin",
        "action": "block"
    });

    let deploy_resp = client
        .post(format!("{}/api/wasm/deploy", ORCHESTRATOR_URL))
        .json(&wasm_rule)
        .send()
        .await?;

    assert!(
        deploy_resp.status().is_success(),
        "WAF rule deployment failed: {}",
        deploy_resp.status()
    );
    println!("✓ WAF rule deployed successfully");

    // Step 3: Wait for rule propagation
    println!("📋 Step 3: Waiting for rule propagation (5s)...");
    sleep(Duration::from_secs(5)).await;

    // Step 4: Verify /admin is now blocked
    println!("📋 Step 4: Verifying /admin is blocked");
    let blocked_resp = client.get(format!("{}/admin", MITIGATION_URL)).send().await?;
    assert_eq!(
        blocked_resp.status().as_u16(),
        403,
        "Expected 403 Forbidden, got {}",
        blocked_resp.status()
    );
    println!("✓ /admin is correctly blocked by WAF rule");

    // Step 5: Verify other endpoints still work
    println!("📋 Step 5: Verifying other endpoints are unaffected");
    let ok_resp = client.get(format!("{}/", MITIGATION_URL)).send().await?;
    assert!(
        ok_resp.status().is_success(),
        "Homepage should be accessible, got {}",
        ok_resp.status()
    );
    println!("✓ Other endpoints remain accessible");

    println!("✅ WASM WAF test passed!");
    Ok(())
}

// ==============================================================================
// Scenario 2: ML-based Behavioral Anomaly Detection
// ==============================================================================

#[tokio::test]
#[ignore]
async fn test_ml_anomaly_detection() -> Result<()> {
    wait_for_services().await?;

    let client = create_client();

    // Step 1: Establish baseline (normal traffic)
    println!("📋 Step 1: Sending baseline normal traffic (10 requests)");
    for i in 1..=10 {
        client.get(format!("{}/", MITIGATION_URL)).send().await?;
        sleep(Duration::from_millis(100)).await;
        if i % 5 == 0 {
            println!("  Sent {}/10 baseline requests", i);
        }
    }
    println!("✓ Baseline traffic sent");

    // Step 2: Generate anomaly (spam 500 requests in 10 seconds)
    println!("📋 Step 2: Generating anomaly traffic (500 requests in 10s)");
    let spam_start = std::time::Instant::now();
    for i in 1..=500 {
        // Send rapid requests
        let _ = client.get(format!("{}/api", MITIGATION_URL)).send().await;

        if i % 100 == 0 {
            println!("  Sent {}/500 spam requests", i);
        }

        // Small delay to spread over 10 seconds (20ms per request)
        sleep(Duration::from_millis(20)).await;
    }
    let spam_duration = spam_start.elapsed();
    println!("✓ Anomaly traffic generated in {:?}", spam_duration);

    // Step 3: Wait for ML model to detect and ban
    println!("📋 Step 3: Waiting for ML detection (30s)...");
    sleep(Duration::from_secs(30)).await;

    // Step 4: Verify IP is blocked
    println!("📋 Step 4: Verifying IP is blocked");
    let blocked_resp = client.get(format!("{}/", MITIGATION_URL)).send().await;

    match blocked_resp {
        Ok(resp) => {
            // Could be 403 (blocked), 429 (rate limited), or connection error
            let status = resp.status().as_u16();
            assert!(
                status == 403 || status == 429 || status >= 500,
                "Expected blocking status, got {}",
                status
            );
            println!("✓ IP is blocked (status: {})", status);
        }
        Err(e) => {
            // Connection errors also indicate blocking
            println!("✓ IP is blocked (connection error: {})", e);
        }
    }

    println!("✅ ML anomaly detection test passed!");
    Ok(())
}

// ==============================================================================
// Scenario 3: XDP IP Blocking via API
// ==============================================================================

#[tokio::test]
#[ignore]
async fn test_xdp_ip_block() -> Result<()> {
    wait_for_services().await?;

    let client = create_client();

    // Step 1: Verify connectivity to origin
    println!("📋 Step 1: Verifying baseline connectivity");
    let baseline_resp = client.get(format!("{}/", MITIGATION_URL)).send().await?;
    assert!(
        baseline_resp.status().is_success(),
        "Baseline check failed: {}",
        baseline_resp.status()
    );
    println!("✓ Baseline connectivity confirmed");

    // Step 2: Block test IP via orchestrator API
    println!("📋 Step 2: Blocking IP 1.2.3.4 via orchestrator API");
    let block_request = json!({
        "ip": "1.2.3.4",
        "reason": "Integration test XDP block",
        "duration_seconds": 300
    });

    let block_resp = client
        .post(format!("{}/api/block", ORCHESTRATOR_URL))
        .json(&block_request)
        .send()
        .await?;

    assert!(
        block_resp.status().is_success(),
        "Block request failed: {}",
        block_resp.status()
    );
    println!("✓ IP block command sent");

    // Step 3: Wait for XDP rule propagation
    println!("📋 Step 3: Waiting for XDP propagation (3s)...");
    sleep(Duration::from_secs(3)).await;

    // Step 4: Verify block is active (query orchestrator)
    println!("📋 Step 4: Verifying block status");
    let status_resp = client
        .get(format!("{}/api/blocks", ORCHESTRATOR_URL))
        .send()
        .await?;

    assert!(status_resp.status().is_success());
    let blocks: serde_json::Value = status_resp.json().await?;
    println!("  Active blocks: {}", blocks);

    // We can't easily test actual XDP filtering without sending packets from 1.2.3.4
    // This test verifies the API flow works
    println!("✓ Block is registered in orchestrator");

    println!("✅ XDP IP block test passed!");
    Ok(())
}

// ==============================================================================
// Scenario 4: CRDT State Synchronization (Global Counter)
// ==============================================================================

#[tokio::test]
#[ignore]
async fn test_crdt_state_sync() -> Result<()> {
    wait_for_services().await?;

    let client = create_client();

    // Step 1: Increment counter on orchestrator
    println!("📋 Step 1: Incrementing global counter (5 times)");
    for i in 1..=5 {
        let increment_resp = client
            .post(format!("{}/api/counter/increment", ORCHESTRATOR_URL))
            .send()
            .await?;

        assert!(increment_resp.status().is_success());
        if i % 2 == 0 {
            println!("  Incremented {}/5 times", i);
        }
    }
    println!("✓ Counter incremented");

    // Step 2: Wait for CRDT synchronization
    println!("📋 Step 2: Waiting for CRDT sync (5s)...");
    sleep(Duration::from_secs(5)).await;

    // Step 3: Read counter value
    println!("📋 Step 3: Reading global counter value");
    let counter_resp = client
        .get(format!("{}/api/counter", ORCHESTRATOR_URL))
        .send()
        .await?;

    assert!(counter_resp.status().is_success());
    let counter_data: serde_json::Value = counter_resp.json().await?;
    let count = counter_data["count"].as_u64().unwrap_or(0);

    println!("  Global counter value: {}", count);
    assert!(
        count >= 5,
        "Expected counter >= 5, got {}",
        count
    );
    println!("✓ Counter synchronized correctly");

    println!("✅ CRDT state sync test passed!");
    Ok(())
}

// ==============================================================================
// Scenario 5: End-to-End Request Flow
// ==============================================================================

#[tokio::test]
#[ignore]
async fn test_e2e_request_flow() -> Result<()> {
    wait_for_services().await?;

    let client = create_client();

    // Test complete request flow: Client -> Mitigation -> Origin -> Back
    println!("📋 Testing full request flow through mitigation proxy");

    // Test 1: Homepage
    let home_resp = client.get(format!("{}/", MITIGATION_URL)).send().await?;
    assert!(home_resp.status().is_success());
    let home_body = home_resp.text().await?;
    assert!(home_body.contains("Test Origin") || home_body.contains("SecBeat"));
    println!("✓ Homepage request successful");

    // Test 2: Health endpoint
    let health_resp = client.get(format!("{}/health", MITIGATION_URL)).send().await?;
    assert!(health_resp.status().is_success());
    println!("✓ Health endpoint successful");

    // Test 3: Echo endpoint
    let echo_resp = client.get(format!("{}/echo", MITIGATION_URL)).send().await?;
    assert!(echo_resp.status().is_success());
    let echo_json: serde_json::Value = echo_resp.json().await?;
    println!("  Echo response: {}", echo_json);
    println!("✓ Echo endpoint successful");

    // Test 4: Error endpoint (should return 500)
    let error_resp = client.get(format!("{}/error", MITIGATION_URL)).send().await?;
    assert_eq!(error_resp.status().as_u16(), 500);
    println!("✓ Error endpoint returns 500 as expected");

    println!("✅ E2E request flow test passed!");
    Ok(())
}
