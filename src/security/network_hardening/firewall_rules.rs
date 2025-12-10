// # Firewall Rules and IP Management
//
// IP reputation checking and connection guard for network security.

use std::time::Instant;
use std::collections::{HashSet, VecDeque};
use crate::Result;
use parking_lot::RwLock;
use std::collections::HashMap;
use std::net::IpAddr;
use std::sync::Arc;
use std::time::Duration;
use serde::{Serialize, Deserialize};

// Constants
const MAX_CONNECTIONS_PER_IP: usize = 100;
const CONNECTION_TIMEOUT_SECS: u64 = 30;

// ============================================================================
// IP Reputation Checker
// ============================================================================

#[derive(Debug, Clone)]
pub struct IPReputation {
    pub ip: IpAddr,
    pub score: i32, // 0-100
    pub last_seen: Instant,
    pub request_count: u64,
    pub failed_requests: u64,
    pub successful_requests: u64,
    pub violations: Vec<ViolationType>,
    pub first_seen: Instant,
}

impl IPReputation {
    fn new(ip: IpAddr) -> Self {
        Self {
            ip,
            score: 50,
            last_seen: Instant::now(),
            request_count: 0,
            failed_requests: 0,
            successful_requests: 0,
            violations: Vec::new(),
            first_seen: Instant::now(),
        }
    }

    fn update_score(&mut self) {
        let success_rate = if self.request_count > 0 {
            self.successful_requests as f64 / self.request_count as f64
        } else {
            0.5
        };

        let violation_penalty = self.violations.len() as i32 * 5;
        let base_score = (success_rate * 100.0) as i32;
        self.score = (base_score - violation_penalty).clamp(0, 100);
    }

    fn record_request(&mut self, success: bool) {
        self.request_count += 1;
        if success {
            self.successful_requests += 1;
        } else {
            self.failed_requests += 1;
        }
        self.last_seen = Instant::now();
        self.update_score();
    }

    fn record_violation(&mut self, violation: ViolationType) {
        self.violations.push(violation);
        if self.violations.len() > 100 {
            self.violations.remove(0);
        }
        self.update_score();
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ViolationType {
    RateLimitExceeded,
    ProtocolViolation,
    AuthenticationFailure,
    SuspiciousPattern,
    DDoSAttempt,
    MalformedRequest,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReputationConfig {
    pub decay_rate: f64,
    pub violation_threshold: usize,
    pub auto_blacklist_score: i32,
    pub auto_whitelist_score: i32,
}

impl Default for ReputationConfig {
    fn default() -> Self {
        Self {
            decay_rate: 0.1,
            violation_threshold: 10,
            auto_blacklist_score: 10,
            auto_whitelist_score: 90,
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ReputationStats {
    pub total_ips: u64,
    pub blacklisted_ips: u64,
    pub whitelisted_ips: u64,
    pub low_reputation_ips: u64,
    pub high_reputation_ips: u64,
}

pub struct IPReputationChecker {
    reputations: Arc<RwLock<HashMap<IpAddr, IPReputation>>>,
    blacklist: Arc<RwLock<HashSet<IpAddr>>>,
    whitelist: Arc<RwLock<HashSet<IpAddr>>>,
    config: ReputationConfig,
    stats: Arc<RwLock<ReputationStats>>,
}

impl IPReputationChecker {
    pub fn new(config: ReputationConfig) -> Self {
        Self {
            reputations: Arc::new(RwLock::new(HashMap::new())),
            blacklist: Arc::new(RwLock::new(HashSet::new())),
            whitelist: Arc::new(RwLock::new(HashSet::new())),
            config,
            stats: Arc::new(RwLock::new(ReputationStats::default())),
        }
    }

    pub fn get_reputation(&self, ip: &IpAddr) -> IPReputation {
        if self.whitelist.read().contains(ip) {
            let mut rep = IPReputation::new(*ip);
            rep.score = 100;
            return rep;
        }

        if self.blacklist.read().contains(ip) {
            let mut rep = IPReputation::new(*ip);
            rep.score = 0;
            return rep;
        }

        let mut reputations = self.reputations.write();
        reputations.entry(*ip)
            .or_insert_with(|| IPReputation::new(*ip))
            .clone()
    }

    pub fn record_request(&self, ip: IpAddr, success: bool) {
        let mut reputations = self.reputations.write();
        let reputation = reputations.entry(ip)
            .or_insert_with(|| IPReputation::new(ip));

        reputation.record_request(success);

        if reputation.score <= self.config.auto_blacklist_score {
            drop(reputations);
            self.blacklist_ip(ip);
        }
    }

    pub fn record_violation(&self, ip: IpAddr, violation: ViolationType) {
        let mut reputations = self.reputations.write();
        let reputation = reputations.entry(ip)
            .or_insert_with(|| IPReputation::new(ip));

        reputation.record_violation(violation);

        if reputation.violations.len() >= self.config.violation_threshold {
            drop(reputations);
            self.blacklist_ip(ip);
        }
    }

    pub fn blacklist_ip(&self, ip: IpAddr) {
        self.blacklist.write().insert(ip);
        self.stats.write().blacklisted_ips += 1;
    }

    pub fn whitelist_ip(&self, ip: IpAddr) {
        self.whitelist.write().insert(ip);
        self.stats.write().whitelisted_ips += 1;
    }

    pub fn is_blacklisted(&self, ip: &IpAddr) -> bool {
        self.blacklist.read().contains(ip)
    }

    pub fn is_whitelisted(&self, ip: &IpAddr) -> bool {
        self.whitelist.read().contains(ip)
    }

    pub fn get_stats(&self) -> ReputationStats {
        let mut stats = self.stats.read().clone();
        let reputations = self.reputations.read();

        stats.total_ips = reputations.len() as u64;
        stats.low_reputation_ips = reputations.values().filter(|r| r.score < 30).count() as u64;
        stats.high_reputation_ips = reputations.values().filter(|r| r.score > 70).count() as u64;

        stats
    }

    pub fn cleanup_old_entries(&self, max_age: Duration) {
        let mut reputations = self.reputations.write();
        reputations.retain(|_, rep| rep.last_seen.elapsed() < max_age);
    }
}

// ============================================================================
// Connection Guard
// ============================================================================

#[derive(Debug, Clone)]
struct ConnectionInfo {
    count: usize,
    timestamps: VecDeque<Instant>,
    bytes_sent: u64,
    bytes_received: u64,
    last_activity: Instant,
    suspicious_count: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectionLimits {
    pub max_connections_per_ip: usize,
    pub max_connections_global: usize,
    pub connection_timeout: Duration,
    pub idle_timeout: Duration,
    pub max_connection_rate: usize,
    pub rate_window: Duration,
}

impl Default for ConnectionLimits {
    fn default() -> Self {
        Self {
            max_connections_per_ip: MAX_CONNECTIONS_PER_IP,
            max_connections_global: 10_000,
            connection_timeout: Duration::from_secs(CONNECTION_TIMEOUT_SECS),
            idle_timeout: Duration::from_secs(60),
            max_connection_rate: 10,
            rate_window: Duration::from_secs(1),
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ConnectionStats {
    pub total_connections: u64,
    pub active_connections: u64,
    pub rejected_connections: u64,
    pub timed_out_connections: u64,
    pub suspicious_connections: u64,
}

pub struct ConnectionGuard {
    connections: Arc<RwLock<HashMap<IpAddr, ConnectionInfo>>>,
    limits: ConnectionLimits,
    stats: Arc<RwLock<ConnectionStats>>,
}

impl ConnectionGuard {
    pub fn new(limits: ConnectionLimits) -> Self {
        Self {
            connections: Arc::new(RwLock::new(HashMap::new())),
            limits,
            stats: Arc::new(RwLock::new(ConnectionStats::default())),
        }
    }

    pub fn check_connection(&self, ip: IpAddr) -> Result<bool> {
        let connections = self.connections.write();
        let mut stats = self.stats.write();

        let total_connections: usize = connections.values().map(|c| c.count).sum();
        if total_connections >= self.limits.max_connections_global {
            stats.rejected_connections += 1;
            return Ok(false);
        }

        if let Some(info) = connections.get(&ip) {
            if info.count >= self.limits.max_connections_per_ip {
                stats.rejected_connections += 1;
                return Ok(false);
            }

            let recent_count = info.timestamps.iter()
                .filter(|&&t| t.elapsed() < self.limits.rate_window)
                .count();

            if recent_count >= self.limits.max_connection_rate {
                stats.rejected_connections += 1;
                return Ok(false);
            }
        }

        stats.total_connections += 1;
        stats.active_connections += 1;

        Ok(true)
    }

    pub fn register_connection(&self, ip: IpAddr) {
        let mut connections = self.connections.write();
        let info = connections.entry(ip).or_insert_with(|| ConnectionInfo {
            count: 0,
            timestamps: VecDeque::new(),
            bytes_sent: 0,
            bytes_received: 0,
            last_activity: Instant::now(),
            suspicious_count: 0,
        });

        info.count += 1;
        info.timestamps.push_back(Instant::now());
        info.last_activity = Instant::now();
    }

    pub fn unregister_connection(&self, ip: IpAddr) {
        let mut connections = self.connections.write();
        if let Some(info) = connections.get_mut(&ip) {
            info.count = info.count.saturating_sub(1);
            if info.count == 0 {
                connections.remove(&ip);
            }
        }

        let mut stats = self.stats.write();
        stats.active_connections = stats.active_connections.saturating_sub(1);
    }

    pub fn mark_suspicious(&self, ip: IpAddr) {
        let mut connections = self.connections.write();
        if let Some(info) = connections.get_mut(&ip) {
            info.suspicious_count += 1;
        }
        self.stats.write().suspicious_connections += 1;
    }

    pub fn cleanup_idle(&self) {
        let mut connections = self.connections.write();
        let mut stats = self.stats.write();

        connections.retain(|_, info| {
            if info.last_activity.elapsed() > self.limits.idle_timeout {
                stats.timed_out_connections += 1;
                stats.active_connections = stats.active_connections.saturating_sub(info.count as u64);
                false
            } else {
                true
            }
        });
    }

    pub fn get_stats(&self) -> ConnectionStats {
        self.stats.read().clone()
    }
}
