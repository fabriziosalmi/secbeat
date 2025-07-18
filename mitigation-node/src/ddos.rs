use crate::config::DdosConfig;
use anyhow::Result;
use dashmap::DashMap;
use governor::{clock::DefaultClock, state::InMemoryState, Quota, RateLimiter};
use ipnet::IpNet;
use metrics::{counter, gauge};
use std::net::IpAddr;
use std::sync::atomic::{AtomicU32, AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::time::interval;
use tracing::{debug, info, warn};

type IpRateLimiter = RateLimiter<governor::state::direct::NotKeyed, InMemoryState, DefaultClock>;

/// DDoS protection engine
#[derive(Debug, Clone)]
pub struct DdosProtection {
    config: DdosConfig,
    /// Rate limiters per IP
    rate_limiters: Arc<DashMap<IpAddr, IpRateLimiter>>,
    /// Connection counters per IP
    connection_counters: Arc<DashMap<IpAddr, AtomicU32>>,
    /// Blacklisted IPs with expiration time
    blacklist: Arc<DashMap<IpAddr, Instant>>,
    /// Violation counters per IP
    violation_counters: Arc<DashMap<IpAddr, AtomicU64>>,
    /// Whitelisted CIDR ranges
    whitelist: Vec<IpNet>,
    /// Manual blacklisted CIDR ranges
    manual_blacklist: Vec<IpNet>,
    /// Global connection counter
    total_connections: Arc<AtomicU32>,
    /// Last cleanup time
    last_cleanup: Arc<std::sync::Mutex<Instant>>,
}

/// Result of DDoS check
#[derive(Debug, Clone, PartialEq)]
pub enum DdosCheckResult {
    /// Allow the connection
    Allow,
    /// Block due to rate limiting
    RateLimited,
    /// Block due to connection limit
    ConnectionLimitExceeded,
    /// Block due to blacklist
    Blacklisted,
    /// Block due to global connection limit
    GlobalLimitExceeded,
}

impl DdosProtection {
    /// Create new DDoS protection instance
    pub fn new(config: DdosConfig) -> Result<Self> {
        info!("Initializing DDoS protection");

        // Parse whitelist CIDR ranges
        let whitelist: Vec<IpNet> = config
            .blacklist
            .static_whitelist
            .as_ref()
            .unwrap_or(&Vec::new())
            .iter()
            .filter_map(|cidr| match cidr.parse::<IpNet>() {
                Ok(net) => Some(net),
                Err(e) => {
                    warn!(cidr = %cidr, error = %e, "Failed to parse whitelist CIDR");
                    None
                }
            })
            .collect();

        // Parse manual blacklist CIDR ranges
        let manual_blacklist: Vec<IpNet> = config
            .blacklist
            .static_blacklist
            .as_ref()
            .unwrap_or(&Vec::new())
            .iter()
            .filter_map(|cidr| match cidr.parse::<IpNet>() {
                Ok(net) => Some(net),
                Err(e) => {
                    warn!(cidr = %cidr, error = %e, "Failed to parse blacklist CIDR");
                    None
                }
            })
            .collect();

        info!(
            rate_limit_rps = config.rate_limiting.requests_per_second,
            max_connections_per_ip = config.connection_limits.max_connections_per_ip,
            max_total_connections = config.connection_limits.max_total_connections,
            whitelist_count = whitelist.len(),
            blacklist_count = manual_blacklist.len(),
            "DDoS protection configured"
        );

        Ok(Self {
            config,
            rate_limiters: Arc::new(DashMap::new()),
            connection_counters: Arc::new(DashMap::new()),
            blacklist: Arc::new(DashMap::new()),
            violation_counters: Arc::new(DashMap::new()),
            whitelist,
            manual_blacklist,
            total_connections: Arc::new(AtomicU32::new(0)),
            last_cleanup: Arc::new(std::sync::Mutex::new(Instant::now())),
        })
    }

    /// Check if a connection should be allowed
    pub fn check_connection(&self, ip: IpAddr) -> DdosCheckResult {
        if !self.config.enabled {
            return DdosCheckResult::Allow;
        }

        // Check whitelist first
        if self.is_whitelisted(ip) {
            debug!(ip = %ip, "IP whitelisted, allowing connection");
            return DdosCheckResult::Allow;
        }

        // Check manual blacklist
        if self.is_manually_blacklisted(ip) {
            debug!(ip = %ip, "IP manually blacklisted");
            counter!("ddos_blocked_manual_blacklist", 1);
            return DdosCheckResult::Blacklisted;
        }

        // Check dynamic blacklist
        if self.is_blacklisted(ip) {
            debug!(ip = %ip, "IP dynamically blacklisted");
            counter!("ddos_blocked_blacklist", 1);
            return DdosCheckResult::Blacklisted;
        }

        // Check global connection limit
        let total_connections = self.total_connections.load(Ordering::Relaxed);
        if total_connections >= self.config.connection_limits.max_total_connections {
            debug!(
                ip = %ip,
                total_connections = total_connections,
                max_total = self.config.connection_limits.max_total_connections,
                "Global connection limit exceeded"
            );
            counter!("ddos_blocked_global_limit", 1);
            return DdosCheckResult::GlobalLimitExceeded;
        }

        // Check per-IP connection limit
        let connection_count = self
            .connection_counters
            .entry(ip)
            .or_insert_with(|| AtomicU32::new(0))
            .load(Ordering::Relaxed);

        if connection_count >= self.config.connection_limits.max_connections_per_ip {
            debug!(
                ip = %ip,
                connections = connection_count,
                max_per_ip = self.config.connection_limits.max_connections_per_ip,
                "Per-IP connection limit exceeded"
            );
            self.record_violation(ip);
            counter!("ddos_blocked_connection_limit", 1);
            return DdosCheckResult::ConnectionLimitExceeded;
        }

        // Check rate limit
        if self.is_rate_limited(ip) {
            debug!(ip = %ip, "IP rate limited");
            self.record_violation(ip);
            counter!("ddos_blocked_rate_limit", 1);
            return DdosCheckResult::RateLimited;
        }

        DdosCheckResult::Allow
    }

    /// Record a new connection
    pub fn record_connection(&self, ip: IpAddr) {
        if !self.config.enabled {
            return;
        }

        // Increment per-IP counter
        self.connection_counters
            .entry(ip)
            .or_insert_with(|| AtomicU32::new(0))
            .fetch_add(1, Ordering::Relaxed);

        // Increment global counter
        self.total_connections.fetch_add(1, Ordering::Relaxed);

        debug!(
            ip = %ip,
            per_ip_connections = self.connection_counters.get(&ip).map(|c| c.load(Ordering::Relaxed)).unwrap_or(0),
            total_connections = self.total_connections.load(Ordering::Relaxed),
            "Connection recorded"
        );
    }

    /// Record connection closure
    pub fn record_disconnection(&self, ip: IpAddr) {
        if !self.config.enabled {
            return;
        }

        // Decrement per-IP counter
        if let Some(counter) = self.connection_counters.get(&ip) {
            let prev = counter.fetch_sub(1, Ordering::Relaxed);
            if prev > 0 {
                debug!(
                    ip = %ip,
                    remaining_connections = prev - 1,
                    "Connection closed"
                );
            }
        }

        // Decrement global counter
        let prev_total = self.total_connections.fetch_sub(1, Ordering::Relaxed);
        if prev_total > 0 {
            gauge!("ddos_total_connections", (prev_total - 1) as f64);
        }
    }

    /// Check if IP is rate limited
    fn is_rate_limited(&self, ip: IpAddr) -> bool {
        let rate_limiter = self.rate_limiters.entry(ip).or_insert_with(|| {
            let requests_per_second = std::num::NonZeroU32::new(self.config.rate_limiting.requests_per_second)
                .unwrap_or_else(|| {
                    tracing::warn!("Invalid requests_per_second value {}, using default of 10", 
                        self.config.rate_limiting.requests_per_second);
                    std::num::NonZeroU32::new(10).expect("10 is non-zero")
                });
            RateLimiter::direct(Quota::per_second(requests_per_second))
        });

        rate_limiter.check().is_err()
    }

    /// Check if IP is whitelisted
    fn is_whitelisted(&self, ip: IpAddr) -> bool {
        self.whitelist.iter().any(|net| net.contains(&ip))
    }

    /// Check if IP is manually blacklisted
    fn is_manually_blacklisted(&self, ip: IpAddr) -> bool {
        self.manual_blacklist.iter().any(|net| net.contains(&ip))
    }

    /// Check if IP is dynamically blacklisted
    fn is_blacklisted(&self, ip: IpAddr) -> bool {
        if let Some(expiry) = self.blacklist.get(&ip) {
            if Instant::now() < *expiry {
                return true;
            } else {
                // Expired, remove from blacklist
                self.blacklist.remove(&ip);
            }
        }
        false
    }

    /// Record a violation for automatic blacklisting
    fn record_violation(&self, ip: IpAddr) {
        if !self.config.blacklist.enabled {
            return;
        }

        let violations = self
            .violation_counters
            .entry(ip)
            .or_insert_with(|| AtomicU64::new(0))
            .fetch_add(1, Ordering::Relaxed)
            + 1;

        debug!(ip = %ip, violations = violations, "Violation recorded");

        // Check if we should blacklist this IP
        if violations >= self.config.blacklist.violation_threshold as u64 {
            self.add_to_blacklist(ip);
        }
    }

    /// Add IP to dynamic blacklist
    fn add_to_blacklist(&self, ip: IpAddr) {
        let expiry = Instant::now()
            + Duration::from_secs(
                self.config
                    .blacklist
                    .blacklist_duration_seconds
                    .unwrap_or(300),
            );
        self.blacklist.insert(ip, expiry);

        // Reset violation counter
        self.violation_counters.remove(&ip);

        warn!(
            ip = %ip,
            duration_seconds = self.config.blacklist.blacklist_duration_seconds.unwrap_or(300),
            "IP added to blacklist"
        );

        counter!("ddos_blacklist_additions", 1);
    }

    /// Start background cleanup task
    pub fn start_cleanup_task(self: Arc<Self>) {
        let protection = Arc::clone(&self);
        tokio::spawn(async move {
            let mut interval = interval(Duration::from_secs(60)); // Cleanup every minute

            loop {
                interval.tick().await;
                protection.cleanup().await;
            }
        });
    }

    /// Cleanup expired entries
    async fn cleanup(&self) {
        let now = Instant::now();
        let mut last_cleanup = match self.last_cleanup.lock() {
            Ok(guard) => guard,
            Err(poisoned) => {
                warn!("Cleanup mutex was poisoned, recovering");
                poisoned.into_inner()
            }
        };

        // Only cleanup if enough time has passed
        if now.duration_since(*last_cleanup) < Duration::from_secs(60) {
            return;
        }
        *last_cleanup = now;
        drop(last_cleanup);

        debug!("Starting DDoS protection cleanup");

        // Cleanup expired blacklist entries
        let mut expired_blacklist = Vec::new();
        for entry in self.blacklist.iter() {
            if now >= *entry.value() {
                expired_blacklist.push(*entry.key());
            }
        }

        for ip in expired_blacklist {
            self.blacklist.remove(&ip);
            debug!(ip = %ip, "Removed expired blacklist entry");
        }

        // Cleanup zero connection counters
        let mut zero_connections = Vec::new();
        for entry in self.connection_counters.iter() {
            if entry.value().load(Ordering::Relaxed) == 0 {
                zero_connections.push(*entry.key());
            }
        }

        for ip in zero_connections {
            self.connection_counters.remove(&ip);
        }

        // Reset violation counters periodically (every hour)
        if now.duration_since(Instant::now()) > Duration::from_secs(3600) {
            self.violation_counters.clear();
            debug!("Reset violation counters");
        }

        // Update metrics
        gauge!("ddos_blacklist_size", self.blacklist.len() as f64);
        gauge!("ddos_active_ips", self.connection_counters.len() as f64);
        gauge!(
            "ddos_total_connections",
            self.total_connections.load(Ordering::Relaxed) as f64
        );

        debug!(
            blacklist_size = self.blacklist.len(),
            active_ips = self.connection_counters.len(),
            total_connections = self.total_connections.load(Ordering::Relaxed),
            "DDoS protection cleanup completed"
        );
    }

    /// Get protection statistics
    pub fn get_stats(&self) -> DdosStats {
        DdosStats {
            total_connections: self.total_connections.load(Ordering::Relaxed),
            blacklisted_ips: self.blacklist.len() as u32,
            active_ips: self.connection_counters.len() as u32,
            rate_limited_ips: self.rate_limiters.len() as u32,
        }
    }
}

/// DDoS protection statistics
#[derive(Debug, Clone)]
pub struct DdosStats {
    pub total_connections: u32,
    pub blacklisted_ips: u32,
    pub active_ips: u32,
    pub rate_limited_ips: u32,
}
