//! Application firewall with rate limiting and DDoS protection
//!
//! This module provides rate limiting, DDoS protection, connection limits,
//! and suspicious activity detection.

use crate::error::{DbError, Result};
use std::collections::HashMap;
use std::net::IpAddr;
use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime};
use tokio::sync::RwLock;

/// Rate limit configuration
#[derive(Debug, Clone)]
pub struct RateLimitConfig {
    /// Maximum requests per window
    pub max_requests: u64,

    /// Time window
    pub window: Duration,

    /// Burst size (allow this many requests immediately)
    pub burst_size: u64,
}

impl Default for RateLimitConfig {
    fn default() -> Self {
        Self {
            max_requests: 100,
            window: Duration::from_secs(60),
            burst_size: 10,
        }
    }
}

impl RateLimitConfig {
    /// Create a new rate limit configuration
    pub fn new(max_requests: u64, window: Duration) -> Self {
        Self {
            max_requests,
            window,
            burst_size: max_requests / 10,
        }
    }

    /// Set burst size
    pub fn with_burst_size(mut self, burst_size: u64) -> Self {
        self.burst_size = burst_size;
        self
    }
}

/// Rate limiter using token bucket algorithm
#[derive(Debug, Clone)]
pub struct RateLimiter {
    /// Configuration
    config: RateLimitConfig,

    /// Available tokens
    tokens: f64,

    /// Last refill time
    last_refill: Instant,
}

impl RateLimiter {
    /// Create a new rate limiter
    pub fn new(config: RateLimitConfig) -> Self {
        Self {
            tokens: config.burst_size as f64,
            config,
            last_refill: Instant::now(),
        }
    }

    /// Check if request is allowed
    pub fn allow(&mut self) -> bool {
        self.refill();

        if self.tokens >= 1.0 {
            self.tokens -= 1.0;
            true
        } else {
            false
        }
    }

    /// Refill tokens
    fn refill(&mut self) {
        let now = Instant::now();
        let elapsed = now.duration_since(self.last_refill);

        // Calculate tokens to add based on elapsed time
        let refill_rate = self.config.max_requests as f64 / self.config.window.as_secs_f64();
        let tokens_to_add = elapsed.as_secs_f64() * refill_rate;

        self.tokens = (self.tokens + tokens_to_add).min(self.config.burst_size as f64);
        self.last_refill = now;
    }

    /// Get available tokens
    pub fn available_tokens(&self) -> f64 {
        self.tokens
    }

    /// Reset limiter
    pub fn reset(&mut self) {
        self.tokens = self.config.burst_size as f64;
        self.last_refill = Instant::now();
    }
}

/// Connection tracking information
#[derive(Debug, Clone)]
pub struct ConnectionInfo {
    /// IP address
    pub ip: IpAddr,

    /// Connection count
    pub connection_count: u64,

    /// First connection time
    pub first_seen: Instant,

    /// Last connection time
    pub last_seen: Instant,

    /// Total bytes sent
    pub bytes_sent: u64,

    /// Total bytes received
    pub bytes_received: u64,

    /// Failed connection attempts
    pub failed_attempts: u64,

    /// Suspicious activity score
    pub suspicion_score: u32,
}

impl ConnectionInfo {
    /// Create new connection info
    pub fn new(ip: IpAddr) -> Self {
        let now = Instant::now();
        Self {
            ip,
            connection_count: 1,
            first_seen: now,
            last_seen: now,
            bytes_sent: 0,
            bytes_received: 0,
            failed_attempts: 0,
            suspicion_score: 0,
        }
    }

    /// Record connection
    pub fn record_connection(&mut self) {
        self.connection_count += 1;
        self.last_seen = Instant::now();
    }

    /// Record failed attempt
    pub fn record_failure(&mut self) {
        self.failed_attempts += 1;
        self.suspicion_score += 10;
        self.last_seen = Instant::now();
    }

    /// Record traffic
    pub fn record_traffic(&mut self, bytes_sent: u64, bytes_received: u64) {
        self.bytes_sent += bytes_sent;
        self.bytes_received += bytes_received;
    }

    /// Check if suspicious
    pub fn is_suspicious(&self) -> bool {
        self.suspicion_score > 100
            || self.failed_attempts > 10
            || self.connection_count > 1000
    }

    /// Decay suspicion score
    pub fn decay_suspicion(&mut self, amount: u32) {
        self.suspicion_score = self.suspicion_score.saturating_sub(amount);
    }
}

/// DDoS protection configuration
#[derive(Debug, Clone)]
pub struct DdosProtectionConfig {
    /// Enable SYN flood protection
    pub syn_flood_protection: bool,

    /// Maximum SYN packets per second
    pub max_syn_per_second: u64,

    /// Enable connection limit per IP
    pub connection_limit_enabled: bool,

    /// Maximum connections per IP
    pub max_connections_per_ip: u64,

    /// Enable bandwidth limiting
    pub bandwidth_limit_enabled: bool,

    /// Maximum bandwidth per IP (bytes/sec)
    pub max_bandwidth_per_ip: u64,

    /// Blacklist duration for suspicious IPs
    pub blacklist_duration: Duration,
}

impl Default for DdosProtectionConfig {
    fn default() -> Self {
        Self {
            syn_flood_protection: true,
            max_syn_per_second: 100,
            connection_limit_enabled: true,
            max_connections_per_ip: 100,
            bandwidth_limit_enabled: true,
            max_bandwidth_per_ip: 10_000_000, // 10 MB/s
            blacklist_duration: Duration::from_secs(3600), // 1 hour
        }
    }
}

/// Firewall configuration
#[derive(Debug, Clone)]
pub struct FirewallConfig {
    /// Enable firewall
    pub enabled: bool,

    /// Rate limit configuration
    pub rate_limit: RateLimitConfig,

    /// DDoS protection configuration
    pub ddos_protection: DdosProtectionConfig,

    /// Maximum total connections
    pub max_total_connections: u64,

    /// Connection timeout
    pub connection_timeout: Duration,

    /// Enable geo-blocking
    pub geo_blocking_enabled: bool,

    /// Blocked countries (ISO 3166-1 alpha-2)
    pub blocked_countries: Vec<String>,
}

impl Default for FirewallConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            rate_limit: RateLimitConfig::default(),
            ddos_protection: DdosProtectionConfig::default(),
            max_total_connections: 10000,
            connection_timeout: Duration::from_secs(300), // 5 minutes
            geo_blocking_enabled: false,
            blocked_countries: Vec::new(),
        }
    }
}

impl FirewallConfig {
    /// Create a new firewall configuration
    pub fn new() -> Self {
        Self::default()
    }

    /// Set rate limit
    pub fn with_rate_limit(mut self, config: RateLimitConfig) -> Self {
        self.rate_limit = config;
        self
    }

    /// Set DDoS protection
    pub fn with_ddos_protection(mut self, config: DdosProtectionConfig) -> Self {
        self.ddos_protection = config;
        self
    }

    /// Set max connections
    pub fn with_max_connections(mut self, max: u64) -> Self {
        self.max_total_connections = max;
        self
    }
}

/// Blacklist entry
#[derive(Debug, Clone)]
pub struct BlacklistEntry {
    /// IP address
    pub ip: IpAddr,

    /// Blacklisted at
    pub blacklisted_at: Instant,

    /// Expires at
    pub expires_at: Instant,

    /// Reason
    pub reason: String,
}

impl BlacklistEntry {
    /// Create new blacklist entry
    pub fn new(ip: IpAddr, duration: Duration, reason: String) -> Self {
        let now = Instant::now();
        Self {
            ip,
            blacklisted_at: now,
            expires_at: now + duration,
            reason,
        }
    }

    /// Check if expired
    pub fn is_expired(&self) -> bool {
        Instant::now() > self.expires_at
    }
}

/// Application firewall
pub struct ApplicationFirewall {
    /// Configuration
    config: FirewallConfig,

    /// Rate limiters per IP
    rate_limiters: Arc<RwLock<HashMap<IpAddr, RateLimiter>>>,

    /// Connection tracking
    connections: Arc<RwLock<HashMap<IpAddr, ConnectionInfo>>>,

    /// Blacklist
    blacklist: Arc<RwLock<HashMap<IpAddr, BlacklistEntry>>>,

    /// Total active connections
    total_connections: Arc<RwLock<u64>>,

    /// Monitoring task handle
    monitoring_task: Option<tokio::task::JoinHandle<()>>,
}

impl ApplicationFirewall {
    /// Create a new application firewall
    pub fn new(config: FirewallConfig) -> Result<Self> {
        Ok(Self {
            config,
            rate_limiters: Arc::new(RwLock::new(HashMap::new())),
            connections: Arc::new(RwLock::new(HashMap::new())),
            blacklist: Arc::new(RwLock::new(HashMap::new())),
            total_connections: Arc::new(RwLock::new(0)),
            monitoring_task: None,
        })
    }

    /// Start monitoring and cleanup tasks
    pub async fn start_monitoring(&mut self) -> Result<()> {
        let blacklist = self.blacklist.clone();
        let connections = self.connections.clone();

        let task = tokio::spawn(async move {
            let mut ticker = tokio::time::interval(Duration::from_secs(60));
            loop {
                ticker.tick().await;

                // Clean up expired blacklist entries
                let mut bl = blacklist.write().await;
                bl.retain(|_, entry| !entry.is_expired());
                drop(bl);

                // Decay suspicion scores
                let mut conns = connections.write().await;
                for conn in conns.values_mut() {
                    conn.decay_suspicion(5);
                }
                drop(conns);
            }
        });

        self.monitoring_task = Some(task);
        Ok(())
    }

    /// Stop monitoring
    pub async fn stop_monitoring(&mut self) {
        if let Some(task) = self.monitoring_task.take() {
            task.abort();
        }
    }

    /// Check if connection is allowed
    pub async fn allow_connection(&mut self, ip: IpAddr) -> Result<bool> {
        if !self.config.enabled {
            return Ok(true);
        }

        // Check blacklist
        {
            let blacklist = self.blacklist.read().await;
            if let Some(entry) = blacklist.get(&ip) {
                if !entry.is_expired() {
                    return Ok(false);
                }
            }
        }

        // Check total connections
        {
            let total = *self.total_connections.read().await;
            if total >= self.config.max_total_connections {
                return Ok(false);
            }
        }

        // Check rate limit
        {
            let mut limiters = self.rate_limiters.write().await;
            let limiter = limiters
                .entry(ip)
                .or_insert_with(|| RateLimiter::new(self.config.rate_limit.clone()));

            if !limiter.allow() {
                return Ok(false);
            }
        }

        // Check connection limit per IP
        if self.config.ddos_protection.connection_limit_enabled {
            let connections = self.connections.read().await;
            if let Some(info) = connections.get(&ip) {
                if info.connection_count >= self.config.ddos_protection.max_connections_per_ip {
                    return Ok(false);
                }
            }
        }

        // Record connection
        {
            let mut connections = self.connections.write().await;
            let info = connections.entry(ip).or_insert_with(|| ConnectionInfo::new(ip));
            info.record_connection();

            // Check if suspicious
            if info.is_suspicious() {
                drop(connections);
                self.blacklist_ip(
                    ip,
                    self.config.ddos_protection.blacklist_duration,
                    "Suspicious activity detected".to_string(),
                )
                .await?;
                return Ok(false);
            }
        }

        // Increment total connections
        {
            let mut total = self.total_connections.write().await;
            *total += 1;
        }

        Ok(true)
    }

    /// Record connection close
    pub async fn record_close(&self, ip: IpAddr) -> Result<()> {
        let mut total = self.total_connections.write().await;
        *total = total.saturating_sub(1);
        Ok(())
    }

    /// Record failed connection attempt
    pub async fn record_failure(&self, ip: IpAddr) -> Result<()> {
        let mut connections = self.connections.write().await;
        let info = connections.entry(ip).or_insert_with(|| ConnectionInfo::new(ip));
        info.record_failure();
        Ok(())
    }

    /// Record traffic
    pub async fn record_traffic(
        &self,
        ip: IpAddr,
        bytes_sent: u64,
        bytes_received: u64,
    ) -> Result<()> {
        let mut connections = self.connections.write().await;
        if let Some(info) = connections.get_mut(&ip) {
            info.record_traffic(bytes_sent, bytes_received);

            // Check bandwidth limit
            if self.config.ddos_protection.bandwidth_limit_enabled {
                let elapsed = info.last_seen.duration_since(info.first_seen).as_secs();
                if elapsed > 0 {
                    let bandwidth = (info.bytes_sent + info.bytes_received) / elapsed;
                    if bandwidth > self.config.ddos_protection.max_bandwidth_per_ip {
                        info.suspicion_score += 50;
                    }
                }
            }
        }
        Ok(())
    }

    /// Blacklist IP address
    pub async fn blacklist_ip(&self, ip: IpAddr, duration: Duration, reason: String) -> Result<()> {
        let mut blacklist = self.blacklist.write().await;
        blacklist.insert(ip, BlacklistEntry::new(ip, duration, reason));
        Ok(())
    }

    /// Remove IP from blacklist
    pub async fn unblacklist_ip(&self, ip: IpAddr) -> Result<()> {
        let mut blacklist = self.blacklist.write().await;
        blacklist.remove(&ip);
        Ok(())
    }

    /// Check if IP is blacklisted
    pub async fn is_blacklisted(&self, ip: IpAddr) -> bool {
        let blacklist = self.blacklist.read().await;
        if let Some(entry) = blacklist.get(&ip) {
            !entry.is_expired()
        } else {
            false
        }
    }

    /// Get connection info
    pub async fn get_connection_info(&self, ip: IpAddr) -> Option<ConnectionInfo> {
        let connections = self.connections.read().await;
        connections.get(&ip).cloned()
    }

    /// Get total connections
    pub async fn get_total_connections(&self) -> u64 {
        *self.total_connections.read().await
    }

    /// Get statistics
    pub async fn get_statistics(&self) -> FirewallStatistics {
        let total_connections = *self.total_connections.read().await;
        let connections = self.connections.read().await;
        let blacklist = self.blacklist.read().await;

        let suspicious_ips = connections.values().filter(|c| c.is_suspicious()).count();

        FirewallStatistics {
            total_connections,
            tracked_ips: connections.len() as u64,
            blacklisted_ips: blacklist.len() as u64,
            suspicious_ips: suspicious_ips as u64,
        }
    }

    /// Clear all data
    pub async fn clear(&self) -> Result<()> {
        self.rate_limiters.write().await.clear();
        self.connections.write().await.clear();
        self.blacklist.write().await.clear();
        *self.total_connections.write().await = 0;
        Ok(())
    }
}

impl Drop for ApplicationFirewall {
    fn drop(&mut self) {
        if let Some(task) = self.monitoring_task.take() {
            task.abort();
        }
    }
}

/// Firewall statistics
#[derive(Debug, Clone)]
pub struct FirewallStatistics {
    /// Total active connections
    pub total_connections: u64,

    /// Number of tracked IPs
    pub tracked_ips: u64,

    /// Number of blacklisted IPs
    pub blacklisted_ips: u64,

    /// Number of suspicious IPs
    pub suspicious_ips: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rate_limiter() {
        let config = RateLimitConfig::new(10, Duration::from_secs(1));
        let mut limiter = RateLimiter::new(config);

        // Should allow burst_size requests immediately
        for _ in 0..10 {
            assert!(limiter.allow());
        }

        // Should deny next request
        assert!(!limiter.allow());
    }

    #[test]
    fn test_connection_info() {
        let ip: IpAddr = "192.168.1.1".parse().unwrap();
        let mut info = ConnectionInfo::new(ip);

        assert_eq!(info.connection_count, 1);
        assert_eq!(info.failed_attempts, 0);
        assert!(!info.is_suspicious());

        for _ in 0..15 {
            info.record_failure();
        }

        assert!(info.is_suspicious());
    }

    #[tokio::test]
    async fn test_application_firewall() {
        let config = FirewallConfig::default();
        let mut firewall = ApplicationFirewall::new(config).unwrap();

        let ip: IpAddr = "192.168.1.1".parse().unwrap();

        assert!(firewall.allow_connection(ip).await.unwrap());
        assert_eq!(firewall.get_total_connections().await, 1);

        firewall.record_close(ip).await.unwrap();
        assert_eq!(firewall.get_total_connections().await, 0);
    }

    #[tokio::test]
    async fn test_blacklist() {
        let config = FirewallConfig::default();
        let mut firewall = ApplicationFirewall::new(config).unwrap();

        let ip: IpAddr = "192.168.1.1".parse().unwrap();

        firewall
            .blacklist_ip(ip, Duration::from_secs(60), "Test".to_string())
            .await
            .unwrap();

        assert!(firewall.is_blacklisted(ip).await);
        assert!(!firewall.allow_connection(ip).await.unwrap());

        firewall.unblacklist_ip(ip).await.unwrap();
        assert!(!firewall.is_blacklisted(ip).await);
    }
}
