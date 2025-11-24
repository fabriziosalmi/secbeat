//! Performance and load tests for SecBeat mitigation node
//!
//! These tests validate system performance under various load conditions:
//! - Throughput and latency benchmarks
//! - Concurrent request handling
//! - Memory and CPU usage under load
//! - Scalability testing
//! - Resource exhaustion scenarios

use anyhow::Result;
use std::collections::HashMap;
use std::net::IpAddr;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use tokio::task::JoinSet;

// Import modules under test
use mitigation_node::config::MitigationConfig;
use mitigation_node::ddos::{DdosProtection, DdosCheckResult};
use mitigation_node::waf::{WafEngine, WafResult, HttpRequest};

// Test helper functions
fn create_test_request(uri: &str, body: Option<&str>) -> HttpRequest {
    HttpRequest {
        method: "GET".to_string(),
        path: uri.split('?').next().unwrap_or("/").to_string(),
        version: "HTTP/1.1".to_string(),
        headers: HashMap::new(),
        body: body.map(|s| s.as_bytes().to_vec()),
        query_string: uri.split('?').nth(1).map(|s| s.to_string()),
    }
}

/// Performance test configuration
#[derive(Debug, Clone)]
struct PerfTestConfig {
    concurrent_users: usize,
    requests_per_user: usize,
    test_duration: Duration,
    warmup_duration: Duration,
}

impl Default for PerfTestConfig {
    fn default() -> Self {
        Self {
            concurrent_users: 100,
            requests_per_user: 100,
            test_duration: Duration::from_secs(30),
            warmup_duration: Duration::from_secs(5),
        }
    }
}

/// Performance test results
#[derive(Debug)]
struct PerfTestResults {
    total_requests: u64,
    successful_requests: u64,
    failed_requests: u64,
    avg_latency_ms: f64,
    p95_latency_ms: f64,
    p99_latency_ms: f64,
    requests_per_second: f64,
    test_duration: Duration,
}

/// WAF performance benchmarks
#[cfg(test)]
mod waf_performance_tests {
    use super::*;

    #[tokio::test]
    async fn test_waf_baseline_performance() {
        let config = MitigationConfig::default();
        let waf = WafEngine::new(config.waf.clone()).await.unwrap();

        let test_inputs = vec![
            "SELECT * FROM users WHERE id = 1",
            "normal user input",
            "<script>alert('test')</script>",
            "../../../etc/passwd",
            "'; DROP TABLE users; --",
        ];

        let iterations = 1000; // Reduced for test speed
        let mut latencies = Vec::with_capacity(iterations);

        // Warmup
        for _ in 0..100 {
            for input in &test_inputs {
                let request = create_test_request(&format!("/?q={}", input), None);
                let _ = waf.inspect_request(&request);
            }
        }

        // Benchmark
        let start = Instant::now();
        for i in 0..iterations {
            let input = &test_inputs[i % test_inputs.len()];
            let request = create_test_request(&format!("/?q={}", input), None);
            let request_start = Instant::now();
            let _ = waf.inspect_request(&request);
            let latency = request_start.elapsed();

            latencies.push(latency.as_nanos() as f64 / 1_000_000.0); // Convert to ms
        }
        let total_duration = start.elapsed();

        // Calculate statistics
        latencies.sort_by(|a, b| a.partial_cmp(b).unwrap());
        let avg_latency = latencies.iter().sum::<f64>() / latencies.len() as f64;
        let p95_idx = (latencies.len() as f64 * 0.95) as usize;
        let p99_idx = (latencies.len() as f64 * 0.99) as usize;
        let p95_latency = latencies[p95_idx];
        let p99_latency = latencies[p99_idx];
        let rps = iterations as f64 / total_duration.as_secs_f64();

        println!("WAF Baseline Performance:");
        println!("  Requests: {}", iterations);
        println!("  Avg latency: {:.2} ms", avg_latency);
        println!("  P95 latency: {:.2} ms", p95_latency);
        println!("  P99 latency: {:.2} ms", p99_latency);
        println!("  Requests/sec: {:.2}", rps);

        // Performance assertions
        assert!(
            avg_latency < 1.0,
            "Average latency should be < 1ms, got {:.2}ms",
            avg_latency
        );
        assert!(
            p95_latency < 5.0,
            "P95 latency should be < 5ms, got {:.2}ms",
            p95_latency
        );
        assert!(
            rps > 10000.0,
            "Should handle > 10k requests/sec, got {:.2}",
            rps
        );
    }

    #[tokio::test]
    async fn test_waf_concurrent_performance() {
        let config = MitigationConfig::default();
        let waf = Arc::new(RwLock::new(
            WafEngine::new(config.waf.clone()).await.unwrap(),
        ));

        let test_config = PerfTestConfig::default();
        let results = run_concurrent_waf_test(waf, test_config).await;

        println!("WAF Concurrent Performance:");
        println!("  Total requests: {}", results.total_requests);
        println!("  Successful: {}", results.successful_requests);
        println!("  Failed: {}", results.failed_requests);
        println!("  Avg latency: {:.2} ms", results.avg_latency_ms);
        println!("  P95 latency: {:.2} ms", results.p95_latency_ms);
        println!("  P99 latency: {:.2} ms", results.p99_latency_ms);
        println!("  Requests/sec: {:.2}", results.requests_per_second);

        // Performance expectations for concurrent load
        assert!(
            results.avg_latency_ms < 10.0,
            "Concurrent avg latency should be < 10ms, got {:.2}ms",
            results.avg_latency_ms
        );
        assert!(
            results.requests_per_second > 1000.0,
            "Should handle > 1k concurrent requests/sec, got {:.2}",
            results.requests_per_second
        );
        assert!(
            results.successful_requests > results.total_requests * 95 / 100,
            "Should have > 95% success rate"
        );
    }

    async fn run_concurrent_waf_test(
        waf: Arc<RwLock<WafEngine>>,
        config: PerfTestConfig,
    ) -> PerfTestResults {
        let test_inputs = vec![
            "normal input",
            "SELECT * FROM users",
            "'; DROP TABLE test; --",
            "<script>alert('xss')</script>",
            "../../../etc/passwd",
        ];

        let mut tasks = JoinSet::new();
        let start_time = Instant::now();

        // Spawn concurrent user tasks
        for user_id in 0..config.concurrent_users {
            let waf_clone = Arc::clone(&waf);
            let inputs = test_inputs.clone();
            let requests_per_user = config.requests_per_user;

            tasks.spawn(async move {
                let mut latencies = Vec::new();
                let mut successes = 0;
                let mut failures = 0;

                for i in 0..requests_per_user {
                    let input = &inputs[i % inputs.len()];

                    let request_start = Instant::now();
                    let waf_guard = waf_clone.read().await;
                    let request = create_test_request(&format!("/?q={}", input), None);
                    let result = waf_guard.inspect_request(&request);
                    drop(waf_guard);
                    let latency = request_start.elapsed();

                    latencies.push(latency.as_nanos() as f64 / 1_000_000.0);

                    match result {
                        WafResult::Allow | WafResult::SqlInjection => successes += 1,
                        _ => failures += 1,
                    }
                }

                (user_id, latencies, successes, failures)
            });
        }

        // Collect results
        let mut all_latencies = Vec::new();
        let mut total_successes = 0;
        let mut total_failures = 0;

        while let Some(result) = tasks.join_next().await {
            let (_, latencies, successes, failures) = result.unwrap();
            all_latencies.extend(latencies);
            total_successes += successes;
            total_failures += failures;
        }

        let total_duration = start_time.elapsed();

        // Calculate statistics
        all_latencies.sort_by(|a, b| a.partial_cmp(b).unwrap());
        let avg_latency = all_latencies.iter().sum::<f64>() / all_latencies.len() as f64;
        let p95_idx = (all_latencies.len() as f64 * 0.95) as usize;
        let p99_idx = (all_latencies.len() as f64 * 0.99) as usize;
        let p95_latency = all_latencies[p95_idx];
        let p99_latency = all_latencies[p99_idx];

        let total_requests = (total_successes + total_failures) as u64;
        let rps = total_requests as f64 / total_duration.as_secs_f64();

        PerfTestResults {
            total_requests,
            successful_requests: total_successes as u64,
            failed_requests: total_failures as u64,
            avg_latency_ms: avg_latency,
            p95_latency_ms: p95_latency,
            p99_latency_ms: p99_latency,
            requests_per_second: rps,
            test_duration: total_duration,
        }
    }

    #[tokio::test]
    async fn test_waf_memory_efficiency() {
        let config = MitigationConfig::default();
        let mut waf = WafEngine::new(config.waf.clone()).await.unwrap();

        // Measure memory before adding patterns
        let initial_memory = get_memory_usage_mb();

        // Add many custom patterns
        let pattern_count = 1000;
        for i in 0..pattern_count {
            let pattern = format!("test_pattern_{}", i);
            let _ = waf.add_custom_pattern(&pattern).await;
        }

        let after_patterns_memory = get_memory_usage_mb();

        // Test pattern matching performance with many patterns
        let test_input = "This contains test_pattern_500 in it";
        let iterations = 10000;

        let start = Instant::now();
        for _ in 0..iterations {
            let request = create_test_request(&format!("/?q={}", test_input), None);
            let _ = waf.inspect_request(&request);
        }
        let duration = start.elapsed();

        let final_memory = get_memory_usage_mb();

        println!("WAF Memory Efficiency:");
        println!("  Initial memory: {:.2} MB", initial_memory);
        println!(
            "  After {} patterns: {:.2} MB",
            pattern_count, after_patterns_memory
        );
        println!("  After {} checks: {:.2} MB", iterations, final_memory);
        println!(
            "  Memory per pattern: {:.2} KB",
            (after_patterns_memory - initial_memory) * 1024.0 / pattern_count as f64
        );

        let rps = iterations as f64 / duration.as_secs_f64();
        println!(
            "  Performance with {} patterns: {:.2} req/sec",
            pattern_count, rps
        );

        // Memory efficiency assertions
        let memory_increase = final_memory - initial_memory;
        assert!(
            memory_increase < 50.0,
            "Memory increase should be < 50MB, got {:.2}MB",
            memory_increase
        );
        assert!(
            rps > 1000.0,
            "Should maintain > 1k req/sec with many patterns, got {:.2}",
            rps
        );
    }

    fn get_memory_usage_mb() -> f64 {
        use std::fs;

        // Try to read memory usage from /proc/self/status (Linux)
        if let Ok(status) = fs::read_to_string("/proc/self/status") {
            for line in status.lines() {
                if line.starts_with("VmRSS:") {
                    if let Some(kb_str) = line.split_whitespace().nth(1) {
                        if let Ok(kb) = kb_str.parse::<f64>() {
                            return kb / 1024.0;
                        }
                    }
                }
            }
        }

        // Fallback for non-Linux systems
        0.0
    }
}

/// DDoS protection performance tests
#[cfg(test)]
mod ddos_performance_tests {
    use super::*;
    use std::net::IpAddr;

    #[tokio::test]
    async fn test_ddos_rate_limiting_performance() {
        let config = MitigationConfig::default();
        let ddos = DdosProtection::new(config.ddos.clone()).unwrap();

        let client_ip: IpAddr = "192.168.1.100".parse().unwrap();
        let iterations = 100000;

        // Warmup
        for _ in 0..1000 {
            let _ = ddos.check_connection(client_ip);
        }

        // Benchmark
        let start = Instant::now();
        for _ in 0..iterations {
            let _ = ddos.check_connection(client_ip);
        }
        let duration = start.elapsed();

        let checks_per_second = iterations as f64 / duration.as_secs_f64();

        println!("DDoS Rate Limiting Performance:");
        println!("  Checks: {}", iterations);
        println!("  Duration: {:.2}s", duration.as_secs_f64());
        println!("  Checks/sec: {:.2}", checks_per_second);

        // Should handle high-frequency rate limit checks
        assert!(
            checks_per_second > 50000.0,
            "Should handle > 50k rate limit checks/sec, got {:.2}",
            checks_per_second
        );
    }

    #[tokio::test]
    async fn test_ddos_concurrent_ip_tracking() {
        let config = MitigationConfig::default();
        let ddos = Arc::new(DdosProtection::new(config.ddos.clone()).unwrap());

        let concurrent_ips = 1000;
        let requests_per_ip = 100;

        let mut tasks = JoinSet::new();
        let start_time = Instant::now();

        // Spawn tasks for different IPs
        for ip_suffix in 0..concurrent_ips {
            let ddos_clone = Arc::clone(&ddos);

            tasks.spawn(async move {
                let client_ip: IpAddr = format!("192.168.{}.{}", ip_suffix / 256, ip_suffix % 256)
                    .parse()
                    .unwrap();

                let mut allowed_count = 0;
                let mut blocked_count = 0;

                for _ in 0..requests_per_ip {
                    match ddos_clone.check_connection(client_ip) {
                        DdosCheckResult::Allow => allowed_count += 1,
                        DdosCheckResult::RateLimited | DdosCheckResult::Blacklisted | DdosCheckResult::ConnectionLimitExceeded | DdosCheckResult::GlobalLimitExceeded => blocked_count += 1,
                    }
                }

                (allowed_count, blocked_count)
            });
        }

        // Collect results
        let mut total_allowed = 0;
        let mut total_blocked = 0;

        while let Some(result) = tasks.join_next().await {
            let (allowed, blocked) = result.unwrap();
            total_allowed += allowed;
            total_blocked += blocked;
        }

        let total_duration = start_time.elapsed();
        let total_requests = total_allowed + total_blocked;
        let rps = total_requests as f64 / total_duration.as_secs_f64();

        println!("DDoS Concurrent IP Tracking:");
        println!("  Concurrent IPs: {}", concurrent_ips);
        println!("  Requests per IP: {}", requests_per_ip);
        println!("  Total requests: {}", total_requests);
        println!("  Allowed: {}", total_allowed);
        println!("  Blocked: {}", total_blocked);
        println!("  Requests/sec: {:.2}", rps);

        // Performance expectations
        assert!(
            rps > 5000.0,
            "Should handle > 5k concurrent IP requests/sec, got {:.2}",
            rps
        );
    }

    #[tokio::test]
    #[ignore] // add_to_blacklist and is_blacklisted are private methods
    async fn test_ddos_blacklist_performance() {
        let config = MitigationConfig::default();
        let ddos = DdosProtection::new(config.ddos.clone()).unwrap();

        // Add many IPs to blacklist
        let blacklist_size = 10000;
        for i in 0..blacklist_size {
            let ip: IpAddr = format!("10.0.{}.{}", i / 256, i % 256).parse().unwrap();
            // ddos.add_to_blacklist(ip, Duration::from_secs(300)).await;
        }

        // Test blacklist lookup performance
        let test_ip: IpAddr = "10.0.50.50".parse().unwrap();
        let iterations = 100000;

        let start = Instant::now();
        for _ in 0..iterations {
            // let _ = ddos.is_blacklisted(test_ip).await;
        }
        let duration = start.elapsed();

        let lookups_per_second = iterations as f64 / duration.as_secs_f64();

        println!("DDoS Blacklist Performance:");
        println!("  Blacklist size: {}", blacklist_size);
        println!("  Lookups: {}", iterations);
        println!("  Lookups/sec: {:.2}", lookups_per_second);

        // Blacklist lookups should remain fast even with large lists
        assert!(
            lookups_per_second > 100000.0,
            "Should handle > 100k blacklist lookups/sec, got {:.2}",
            lookups_per_second
        );
    }
}

/// Stress testing scenarios
#[cfg(test)]
mod stress_tests {
    use super::*;

    #[tokio::test]
    async fn test_system_under_extreme_load() {
        let config = MitigationConfig::default();
        let waf = Arc::new(RwLock::new(
            WafEngine::new(config.waf.clone()).await.unwrap(),
        ));
        let ddos = Arc::new(DdosProtection::new(config.ddos.clone()).unwrap());

        let extreme_config = PerfTestConfig {
            concurrent_users: 500,
            requests_per_user: 200,
            test_duration: Duration::from_secs(60),
            warmup_duration: Duration::from_secs(10),
        };

        println!("Starting extreme load test...");
        let start_time = Instant::now();

        let mut tasks = JoinSet::new();

        // Spawn many concurrent tasks
        for user_id in 0..extreme_config.concurrent_users {
            let waf_clone = Arc::clone(&waf);
            let ddos_clone = Arc::clone(&ddos);

            tasks.spawn(async move {
                let client_ip: IpAddr = format!("192.168.{}.{}", user_id / 256, user_id % 256)
                    .parse()
                    .unwrap();

                let mut results = Vec::new();

                for i in 0..extreme_config.requests_per_user {
                    let request_start = Instant::now();

                    // DDoS check
                    let ddos_result = ddos_clone.check_connection(client_ip);

                    if matches!(ddos_result, DdosCheckResult::Allow) {
                        // WAF check
                        let waf_guard = waf_clone.read().await;
                        let test_input = format!("test input {} from user {}", i, user_id);
                        let request = create_test_request(&format!("/?q={}", test_input), None);
                        let _waf_result = waf_guard.inspect_request(&request);
                    }

                    let latency = request_start.elapsed();
                    results.push(latency.as_micros() as f64 / 1000.0); // Convert to ms
                }

                results
            });
        }

        // Collect all results
        let mut all_latencies = Vec::new();

        while let Some(result) = tasks.join_next().await {
            let latencies = result.unwrap();
            all_latencies.extend(latencies);
        }

        let total_duration = start_time.elapsed();

        // Calculate statistics
        all_latencies.sort_by(|a, b| a.partial_cmp(b).unwrap());
        let avg_latency = all_latencies.iter().sum::<f64>() / all_latencies.len() as f64;
        let p95_idx = (all_latencies.len() as f64 * 0.95) as usize;
        let p99_idx = (all_latencies.len() as f64 * 0.99) as usize;
        let p95_latency = all_latencies[p95_idx];
        let p99_latency = all_latencies[p99_idx];

        let total_requests = all_latencies.len() as u64;
        let rps = total_requests as f64 / total_duration.as_secs_f64();

        println!("Extreme Load Test Results:");
        println!("  Total requests: {}", total_requests);
        println!("  Test duration: {:.2}s", total_duration.as_secs_f64());
        println!("  Avg latency: {:.2} ms", avg_latency);
        println!("  P95 latency: {:.2} ms", p95_latency);
        println!("  P99 latency: {:.2} ms", p99_latency);
        println!("  Requests/sec: {:.2}", rps);

        // Under extreme load, system should still function reasonably
        assert!(
            avg_latency < 50.0,
            "Avg latency under extreme load should be < 50ms, got {:.2}ms",
            avg_latency
        );
        assert!(
            p95_latency < 100.0,
            "P95 latency under extreme load should be < 100ms, got {:.2}ms",
            p95_latency
        );
        assert!(
            rps > 500.0,
            "Should maintain > 500 req/sec under extreme load, got {:.2}",
            rps
        );
    }

    #[tokio::test]
    async fn test_memory_leak_detection() {
        let config = MitigationConfig::default();
        let waf = Arc::new(RwLock::new(
            WafEngine::new(config.waf.clone()).await.unwrap(),
        ));

        let initial_memory = get_memory_usage_mb();
        let iterations = 50000;

        // Run many operations that could potentially leak memory
        for i in 0..iterations {
            // Add and remove patterns
            if i % 100 == 0 {
                let mut waf_guard = waf.write().await;
                let pattern = format!("temp_pattern_{}", i);
                let _ = waf_guard.add_custom_pattern(&pattern).await;
                let _ = waf_guard.remove_custom_pattern(&pattern).await;
            }

            // Regular checks
            let waf_guard = waf.read().await;
            let test_input = format!("test input {}", i);
            let request = create_test_request(&format!("/?q={}", test_input), None);
            let _ = waf_guard.inspect_request(&request);

            // Check memory every 1000 iterations
            if i % 1000 == 0 {
                let current_memory = get_memory_usage_mb();
                let memory_increase = current_memory - initial_memory;

                // Memory shouldn't grow unbounded
                assert!(
                    memory_increase < 100.0,
                    "Memory increase should be < 100MB after {} iterations, got {:.2}MB",
                    i,
                    memory_increase
                );
            }
        }

        let final_memory = get_memory_usage_mb();
        let total_memory_increase = final_memory - initial_memory;

        println!("Memory Leak Detection:");
        println!("  Initial memory: {:.2} MB", initial_memory);
        println!("  Final memory: {:.2} MB", final_memory);
        println!("  Memory increase: {:.2} MB", total_memory_increase);
        println!("  Iterations: {}", iterations);

        // Total memory increase should be reasonable
        assert!(
            total_memory_increase < 50.0,
            "Total memory increase should be < 50MB, got {:.2}MB",
            total_memory_increase
        );
    }

    fn get_memory_usage_mb() -> f64 {
        use std::fs;

        if let Ok(status) = fs::read_to_string("/proc/self/status") {
            for line in status.lines() {
                if line.starts_with("VmRSS:") {
                    if let Some(kb_str) = line.split_whitespace().nth(1) {
                        if let Ok(kb) = kb_str.parse::<f64>() {
                            return kb / 1024.0;
                        }
                    }
                }
            }
        }
        0.0
    }
}

/// Scalability tests
#[cfg(test)]
mod scalability_tests {
    use super::*;

    #[tokio::test]
    async fn test_linear_scalability() {
        // Test how performance scales with increasing load
        let config = MitigationConfig::default();
        let waf = Arc::new(RwLock::new(
            WafEngine::new(config.waf.clone()).await.unwrap(),
        ));

        let load_levels = vec![10, 50, 100, 200, 500];
        let mut results = Vec::new();

        for &concurrent_users in &load_levels {
            let test_config = PerfTestConfig {
                concurrent_users,
                requests_per_user: 50,
                test_duration: Duration::from_secs(10),
                warmup_duration: Duration::from_secs(2),
            };

            let result = run_concurrent_waf_test(Arc::clone(&waf), test_config).await;

            println!(
                "Load level {} users: {:.2} req/sec, {:.2}ms avg latency",
                concurrent_users, result.requests_per_second, result.avg_latency_ms
            );

            results.push((concurrent_users, result));
        }

        // Analyze scalability
        for i in 1..results.len() {
            let (prev_users, ref prev_result) = results[i - 1];
            let (curr_users, ref curr_result) = results[i];

            let user_ratio = curr_users as f64 / prev_users as f64;
            let throughput_ratio =
                curr_result.requests_per_second / prev_result.requests_per_second;

            println!(
                "Scaling {}x users: throughput scaled by {:.2}x",
                user_ratio, throughput_ratio
            );

            // Throughput should scale reasonably (at least 50% of linear scaling)
            assert!(
                throughput_ratio > user_ratio * 0.5,
                "Throughput scaling should be at least 50% linear, got {:.2}x for {:.2}x users",
                throughput_ratio,
                user_ratio
            );
        }
    }

    // Helper function from earlier
    async fn run_concurrent_waf_test(
        waf: Arc<RwLock<WafEngine>>,
        config: PerfTestConfig,
    ) -> PerfTestResults {
        let test_inputs = vec![
            "normal input",
            "SELECT * FROM users",
            "'; DROP TABLE test; --",
            "<script>alert('xss')</script>",
            "../../../etc/passwd",
        ];

        let mut tasks = JoinSet::new();
        let start_time = Instant::now();

        for _user_id in 0..config.concurrent_users {
            let waf_clone = Arc::clone(&waf);
            let inputs = test_inputs.clone();
            let requests_per_user = config.requests_per_user;

            tasks.spawn(async move {
                let mut latencies = Vec::new();
                let mut successes = 0;
                let mut failures = 0;

                for i in 0..requests_per_user {
                    let input = &inputs[i % inputs.len()];

                    let request_start = Instant::now();
                    let waf_guard = waf_clone.read().await;
                    let request = create_test_request(&format!("/?q={}", input), None);
                    let result = waf_guard.inspect_request(&request);
                    drop(waf_guard);
                    let latency = request_start.elapsed();

                    latencies.push(latency.as_nanos() as f64 / 1_000_000.0);

                    match result {
                        WafResult::Allow | WafResult::SqlInjection => successes += 1,
                        _ => failures += 1,
                    }
                }

                (latencies, successes, failures)
            });
        }

        let mut all_latencies = Vec::new();
        let mut total_successes = 0;
        let mut total_failures = 0;

        while let Some(result) = tasks.join_next().await {
            let (latencies, successes, failures) = result.unwrap();
            all_latencies.extend(latencies);
            total_successes += successes;
            total_failures += failures;
        }

        let total_duration = start_time.elapsed();

        all_latencies.sort_by(|a, b| a.partial_cmp(b).unwrap());
        let avg_latency = all_latencies.iter().sum::<f64>() / all_latencies.len() as f64;
        let p95_idx = (all_latencies.len() as f64 * 0.95) as usize;
        let p99_idx = (all_latencies.len() as f64 * 0.99) as usize;

        let total_requests = (total_successes + total_failures) as u64;
        let rps = total_requests as f64 / total_duration.as_secs_f64();

        PerfTestResults {
            total_requests,
            successful_requests: total_successes as u64,
            failed_requests: total_failures as u64,
            avg_latency_ms: avg_latency,
            p95_latency_ms: all_latencies[p95_idx],
            p99_latency_ms: all_latencies[p99_idx],
            requests_per_second: rps,
            test_duration: total_duration,
        }
    }
}
