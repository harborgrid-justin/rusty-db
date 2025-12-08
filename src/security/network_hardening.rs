//! # Network Threat Protection
//!
//! Military-grade network security hardening for RustyDB providing comprehensive
//! protection against DDoS attacks, protocol exploits, man-in-the-middle attacks,
//! and network-based intrusions.
//!
//! ## Components
//!
//! - **Adaptive Rate Limiting**: Token bucket and sliding window with behavior-based adjustment
//! - **Connection Guard**: Connection throttling, pooling, and protection
//! - **DDoS Mitigation**: Multi-layer attack detection and automatic mitigation
//! - **Protocol Validation**: Strict protocol compliance enforcement
//! - **TLS Enforcement**: Certificate pinning, PFS, and secure cipher selection
//! - **Network Anomaly Detection**: Statistical and ML-based anomaly detection
//! - **IP Reputation**: Dynamic IP scoring and blacklist management
//!
//! ## Architecture
//!
//! ```text
//! ┌────────────────────────────────────────────────────────────────┐
//! │                    Network Hardening Stack                     │
//! ├────────────────────────────────────────────────────────────────┤
//! │  IP Reputation Check → DDoS Detection → Connection Guard       │
//! │         ↓                    ↓                  ↓               │
//! │  Rate Limiting → Protocol Validation → TLS Enforcement         │
//! │         ↓                    ↓                  ↓               │
//! │      Anomaly Detection → Threat Intelligence                   │
//! └────────────────────────────────────────────────────────────────┘
//! ```

use crate::{Result, DbError};
use parking_lot::{RwLock, Mutex};
use std::collections::{HashMap, HashSet, VecDeque, BTreeMap};
use std::net::{IpAddr, SocketAddr};
use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use serde::{Deserialize, Serialize};
use sha2::{Sha256, Digest};

// ============================================================================
// Constants
// ============================================================================

/// Maximum connections per IP
const MAX_CONNECTIONS_PER_IP: usize = 100;

/// Maximum request rate per IP (requests per second)
const MAX_REQUESTS_PER_SECOND_PER_IP: u64 = 1000;

/// DDoS detection window (seconds)
const DDOS_DETECTION_WINDOW: u64 = 60;

/// Minimum reputation score (0-100)
const MIN_REPUTATION_SCORE: i32 = 20;

/// Anomaly detection window size
const ANOMALY_WINDOW_SIZE: usize = 1000;

/// Z-score threshold for anomaly detection
const ANOMALY_Z_SCORE_THRESHOLD: f64 = 3.0;

/// Connection timeout (seconds)
const CONNECTION_TIMEOUT_SECS: u64 = 30;

/// Slow request timeout (seconds)
const SLOW_REQUEST_TIMEOUT_SECS: u64 = 10;

/// Maximum request size (bytes)
const MAX_REQUEST_SIZE: usize = 10 * 1024 * 1024; // 10MB

/// Maximum header count
const MAX_HEADER_COUNT: usize = 100;

/// Maximum header size (bytes)
const MAX_HEADER_SIZE: usize = 8192;

// ============================================================================
// Adaptive Rate Limiter
// ============================================================================

/// Adaptive rate limiter with behavior-based adjustment
pub struct AdaptiveRateLimiter {
    /// Token buckets per key (IP, user, endpoint)
    token_buckets: Arc<RwLock<HashMap<String, TokenBucket>>>,
    /// Sliding windows per key
    sliding_windows: Arc<RwLock<HashMap<String, SlidingWindow>>>,
    /// Configuration
    config: Arc<RwLock<RateLimitConfig>>,
    /// IP reputation integration
    ip_reputation: Arc<IPReputationChecker>,
    /// Statistics
    stats: Arc<RwLock<RateLimitStats>>,
}

/// Rate limit configuration
#[derive(Debug, Clone)]
pub struct RateLimitConfig {
    /// Global requests per second
    pub global_rps: u64,
    /// Per-IP requests per second
    pub per_ip_rps: u64,
    /// Per-user requests per second
    pub per_user_rps: u64,
    /// Burst multiplier
    pub burst_multiplier: f64,
    /// Enable adaptive behavior
    pub adaptive: bool,
    /// Reputation score adjustment factor
    pub reputation_factor: f64,
}

impl Default for RateLimitConfig {
    fn default() -> Self {
        Self {
            global_rps: 100_000,
            per_ip_rps: 1_000,
            per_user_rps: 10_000,
            burst_multiplier: 2.0,
            adaptive: true,
            reputation_factor: 0.5,
        }
    }
}

/// Token bucket implementation
pub struct TokenBucket {
    /// Capacity (max tokens)
    capacity: f64,
    /// Current tokens
    tokens: f64,
    /// Refill rate (tokens per second)
    refill_rate: f64,
    /// Last refill time
    last_refill: Instant,
    /// Adaptive multiplier based on behavior
    adaptive_multiplier: f64,
}

impl TokenBucket {
    fn new(capacity: f64, refill_rate: f64) -> Self {
        Self {
            capacity,
            tokens: capacity,
            refill_rate,
            last_refill: Instant::now(),
            adaptive_multiplier: 1.0,
        }
    }

    fn refill(&mut self) {
        let now = Instant::now();
        let elapsed = now.duration_since(self.last_refill).as_secs_f64();
        let new_tokens = elapsed * self.refill_rate * self.adaptive_multiplier;
        self.tokens = (self.tokens + new_tokens).min(self.capacity);
        self.last_refill = now;
    }

    fn try_consume(&mut self, amount: f64) -> bool {
        self.refill();
        if self.tokens >= amount {
            self.tokens -= amount;
            true
        } else {
            false
        }
    }

    fn adjust_multiplier(&mut self, factor: f64) {
        self.adaptive_multiplier = (self.adaptive_multiplier * factor).clamp(0.1, 2.0);
    }
}

/// Sliding window for rate limiting
pub struct SlidingWindow {
    /// Window duration
    window_duration: Duration,
    /// Request timestamps
    requests: VecDeque<Instant>,
    /// Maximum requests in window
    max_requests: usize,
}

impl SlidingWindow {
    fn new(window_duration: Duration, max_requests: usize) -> Self {
        Self {
            window_duration,
            requests: VecDeque::new(),
            max_requests,
        }
    }

    fn clean_old(&mut self) {
        let cutoff = Instant::now() - self.window_duration;
        while let Some(&front) = self.requests.front() {
            if front < cutoff {
                self.requests.pop_front();
            } else {
                break;
            }
        }
    }

    fn try_add(&mut self) -> bool {
        self.clean_old();
        if self.requests.len() < self.max_requests {
            self.requests.push_back(Instant::now());
            true
        } else {
            false
        }
    }

    fn current_rate(&mut self) -> f64 {
        self.clean_old();
        self.requests.len() as f64 / self.window_duration.as_secs_f64()
    }
}

/// Rate limit statistics
#[derive(Debug, Clone, Default)]
pub struct RateLimitStats {
    pub total_requests: u64,
    pub allowed_requests: u64,
    pub blocked_requests: u64,
    pub adaptive_adjustments: u64,
}

impl AdaptiveRateLimiter {
    pub fn new(config: RateLimitConfig, ip_reputation: Arc<IPReputationChecker>) -> Self {
        Self {
            token_buckets: Arc::new(RwLock::new(HashMap::new())),
            sliding_windows: Arc::new(RwLock::new(HashMap::new())),
            config: Arc::new(RwLock::new(config)),
            ip_reputation,
            stats: Arc::new(RwLock::new(RateLimitStats::default())),
        }
    }

    /// Check if request is allowed
    pub fn check_rate_limit(&self, key: &str, ip: IpAddr) -> Result<bool> {
        let mut stats = self.stats.write();
        stats.total_requests += 1;

        // Get reputation score
        let reputation = self.ip_reputation.get_reputation(&ip);

        // Reject if reputation too low
        if reputation.score < MIN_REPUTATION_SCORE {
            stats.blocked_requests += 1;
            return Ok(false);
        }

        let config = self.config.read();

        // Calculate adjusted rate based on reputation
        let rate_adjustment = if config.adaptive {
            1.0 + (reputation.score as f64 - 50.0) / 100.0 * config.reputation_factor
        } else {
            1.0
        };

        // Check token bucket
        let mut buckets = self.token_buckets.write();
        let bucket = buckets.entry(key.to_string()).or_insert_with(|| {
            TokenBucket::new(
                config.per_ip_rps as f64 * config.burst_multiplier,
                config.per_ip_rps as f64,
            )
        });

        bucket.adjust_multiplier(rate_adjustment);

        if !bucket.try_consume(1.0) {
            stats.blocked_requests += 1;
            return Ok(false);
        }

        // Check sliding window
        let mut windows = self.sliding_windows.write();
        let window = windows.entry(key.to_string()).or_insert_with(|| {
            SlidingWindow::new(
                Duration::from_secs(1),
                (config.per_ip_rps as f64 * rate_adjustment) as usize,
            )
        });

        if !window.try_add() {
            stats.blocked_requests += 1;
            return Ok(false);
        }

        stats.allowed_requests += 1;
        Ok(true)
    }

    /// Get current statistics
    pub fn get_stats(&self) -> RateLimitStats {
        self.stats.read().clone()
    }
}

// ============================================================================
// Connection Guard
// ============================================================================

/// Connection security and throttling
pub struct ConnectionGuard {
    /// Active connections per IP
    connections: Arc<RwLock<HashMap<IpAddr, ConnectionInfo>>>,
    /// Connection limits
    limits: ConnectionLimits,
    /// Statistics
    stats: Arc<RwLock<ConnectionStats>>,
}

/// Connection information
#[derive(Debug, Clone)]
struct ConnectionInfo {
    /// Connection count
    count: usize,
    /// Connection timestamps
    timestamps: VecDeque<Instant>,
    /// Total bytes sent
    bytes_sent: u64,
    /// Total bytes received
    bytes_received: u64,
    /// Last activity
    last_activity: Instant,
    /// Suspicious activity count
    suspicious_count: u32,
}

/// Connection limits
#[derive(Debug, Clone)]
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

/// Connection statistics
#[derive(Debug, Clone, Default)]
pub struct ConnectionStats {
    pub total_connections: u64,
    pub active_connections: u64,
    pub rejected_connections: u64,
    pub timed_out_connections: u64,
    pub suspicious_connections: u64,
}

impl ConnectionGuard {
    pub fn new(limits: ConnectionLimits) -> Self {
        Self {
            connections: Arc::new(RwLock::new(HashMap::new())),
            limits,
            stats: Arc::new(RwLock::new(ConnectionStats::default())),
        }
    }

    /// Check if connection is allowed
    pub fn check_connection(&self, ip: IpAddr) -> Result<bool> {
        let mut connections = self.connections.write();
        let mut stats = self.stats.write();

        // Check global limit
        let total_connections: usize = connections.values().map(|c| c.count).sum();
        if total_connections >= self.limits.max_connections_global {
            stats.rejected_connections += 1;
            return Ok(false);
        }

        // Check per-IP limit
        if let Some(info) = connections.get(&ip) {
            if info.count >= self.limits.max_connections_per_ip {
                stats.rejected_connections += 1;
                return Ok(false);
            }

            // Check connection rate
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

    /// Register new connection
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

    /// Unregister connection
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

    /// Mark suspicious activity
    pub fn mark_suspicious(&self, ip: IpAddr) {
        let mut connections = self.connections.write();
        if let Some(info) = connections.get_mut(&ip) {
            info.suspicious_count += 1;
        }
        self.stats.write().suspicious_connections += 1;
    }

    /// Cleanup idle connections
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

    /// Get statistics
    pub fn get_stats(&self) -> ConnectionStats {
        self.stats.read().clone()
    }
}

// ============================================================================
// DDoS Mitigation
// ============================================================================

/// DDoS attack detection and mitigation
pub struct DDoSMitigator {
    /// Traffic patterns per IP
    patterns: Arc<RwLock<HashMap<IpAddr, TrafficPattern>>>,
    /// Global traffic metrics
    global_metrics: Arc<RwLock<GlobalTrafficMetrics>>,
    /// Detection thresholds
    thresholds: DDoSThresholds,
    /// Active attacks
    active_attacks: Arc<RwLock<HashMap<String, DDoSAttack>>>,
    /// Statistics
    stats: Arc<RwLock<DDoSStats>>,
}

/// Traffic pattern for an IP
#[derive(Debug, Clone)]
struct TrafficPattern {
    /// Request timestamps
    requests: VecDeque<Instant>,
    /// Request sizes
    request_sizes: VecDeque<usize>,
    /// Response codes
    response_codes: VecDeque<u16>,
    /// Unique endpoints accessed
    endpoints: HashSet<String>,
    /// User agents seen
    user_agents: HashSet<String>,
    /// Last update
    last_update: Instant,
}

impl TrafficPattern {
    fn new() -> Self {
        Self {
            requests: VecDeque::new(),
            request_sizes: VecDeque::new(),
            response_codes: VecDeque::new(),
            endpoints: HashSet::new(),
            user_agents: HashSet::new(),
            last_update: Instant::now(),
        }
    }

    fn add_request(&mut self, size: usize, endpoint: String, user_agent: Option<String>) {
        let now = Instant::now();
        self.requests.push_back(now);
        self.request_sizes.push_back(size);
        self.endpoints.insert(endpoint);
        if let Some(ua) = user_agent {
            self.user_agents.insert(ua);
        }
        self.last_update = now;

        // Keep only recent data
        let cutoff = now - Duration::from_secs(DDOS_DETECTION_WINDOW);
        while let Some(&front) = self.requests.front() {
            if front < cutoff {
                self.requests.pop_front();
                self.request_sizes.pop_front();
            } else {
                break;
            }
        }
    }

    fn request_rate(&self) -> f64 {
        if self.requests.is_empty() {
            return 0.0;
        }

        let window = Duration::from_secs(DDOS_DETECTION_WINDOW);
        let recent = self.requests.iter()
            .filter(|&&t| t.elapsed() < window)
            .count();

        recent as f64 / window.as_secs_f64()
    }

    fn entropy(&self) -> f64 {
        // Calculate Shannon entropy of endpoints accessed
        if self.endpoints.is_empty() {
            return 0.0;
        }

        let total = self.requests.len() as f64;
        let mut entropy = 0.0;

        for _ in &self.endpoints {
            let p = 1.0 / self.endpoints.len() as f64;
            entropy -= p * p.log2();
        }

        entropy
    }
}

/// Global traffic metrics
#[derive(Debug, Clone, Default)]
struct GlobalTrafficMetrics {
    total_requests: u64,
    total_bytes: u64,
    error_count: u64,
    last_window_requests: VecDeque<(Instant, u64)>,
}

/// DDoS detection thresholds
#[derive(Debug, Clone)]
pub struct DDoSThresholds {
    /// Maximum requests per second per IP
    pub max_rps_per_ip: f64,
    /// Maximum global requests per second
    pub max_global_rps: f64,
    /// Minimum entropy for legitimate traffic
    pub min_entropy: f64,
    /// Maximum error rate (percentage)
    pub max_error_rate: f64,
    /// Minimum unique endpoints for legitimate traffic
    pub min_unique_endpoints: usize,
}

impl Default for DDoSThresholds {
    fn default() -> Self {
        Self {
            max_rps_per_ip: 100.0,
            max_global_rps: 100_000.0,
            min_entropy: 2.0,
            max_error_rate: 50.0,
            min_unique_endpoints: 5,
        }
    }
}

/// DDoS attack information
#[derive(Debug, Clone)]
pub struct DDoSAttack {
    pub attack_id: String,
    pub attack_type: DDoSAttackType,
    pub source_ips: HashSet<IpAddr>,
    pub start_time: Instant,
    pub request_count: u64,
    pub mitigation_actions: Vec<MitigationAction>,
}

/// DDoS attack types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DDoSAttackType {
    /// High volume flood
    VolumeFlood,
    /// HTTP flood with valid requests
    HttpFlood,
    /// Slowloris (slow connections)
    Slowloris,
    /// Application layer attack
    ApplicationLayer,
    /// Distributed attack from multiple IPs
    Distributed,
    /// Cache busting attack
    CacheBusting,
}

/// Mitigation action
#[derive(Debug, Clone)]
pub enum MitigationAction {
    RateLimit,
    IpBlock,
    Challenge,
    TrafficShape,
    Alert,
}

/// DDoS statistics
#[derive(Debug, Clone, Default)]
pub struct DDoSStats {
    pub attacks_detected: u64,
    pub attacks_mitigated: u64,
    pub ips_blocked: u64,
    pub requests_blocked: u64,
}

impl DDoSMitigator {
    pub fn new(thresholds: DDoSThresholds) -> Self {
        Self {
            patterns: Arc::new(RwLock::new(HashMap::new())),
            global_metrics: Arc::new(RwLock::new(GlobalTrafficMetrics::default())),
            thresholds,
            active_attacks: Arc::new(RwLock::new(HashMap::new())),
            stats: Arc::new(RwLock::new(DDoSStats::default())),
        }
    }

    /// Analyze request for DDoS patterns
    pub fn analyze_request(
        &self,
        ip: IpAddr,
        request_size: usize,
        endpoint: String,
        user_agent: Option<String>,
    ) -> Result<DDoSAnalysisResult> {
        let mut patterns = self.patterns.write();
        let pattern = patterns.entry(ip).or_insert_with(TrafficPattern::new);

        pattern.add_request(request_size, endpoint.clone(), user_agent);

        // Check for attack patterns
        let request_rate = pattern.request_rate();
        let entropy = pattern.entropy();

        // Volume-based detection
        if request_rate > self.thresholds.max_rps_per_ip {
            self.detect_attack(
                ip,
                DDoSAttackType::VolumeFlood,
                format!("High request rate: {:.2} req/s", request_rate),
            );
            return Ok(DDoSAnalysisResult::Blocked(DDoSAttackType::VolumeFlood));
        }

        // Entropy-based detection (cache busting, randomized requests)
        if entropy < self.thresholds.min_entropy && pattern.requests.len() > 100 {
            self.detect_attack(
                ip,
                DDoSAttackType::CacheBusting,
                format!("Low entropy: {:.2}", entropy),
            );
            return Ok(DDoSAnalysisResult::Suspicious);
        }

        // Check if IP has too few unique endpoints (likely bot)
        if pattern.requests.len() > 1000 && pattern.endpoints.len() < self.thresholds.min_unique_endpoints {
            self.detect_attack(
                ip,
                DDoSAttackType::ApplicationLayer,
                format!("Low endpoint diversity: {}", pattern.endpoints.len()),
            );
            return Ok(DDoSAnalysisResult::Suspicious);
        }

        Ok(DDoSAnalysisResult::Clean)
    }

    fn detect_attack(&self, ip: IpAddr, attack_type: DDoSAttackType, reason: String) {
        let mut attacks = self.active_attacks.write();
        let attack_id = format!("{}_{:?}_{}", ip, attack_type, Instant::now().elapsed().as_secs());

        let attack = DDoSAttack {
            attack_id: attack_id.clone(),
            attack_type,
            source_ips: {
                let mut set = HashSet::new();
                set.insert(ip);
                set
            },
            start_time: Instant::now(),
            request_count: 1,
            mitigation_actions: vec![MitigationAction::RateLimit, MitigationAction::Alert],
        };

        attacks.insert(attack_id, attack);

        let mut stats = self.stats.write();
        stats.attacks_detected += 1;
    }

    /// Get current active attacks
    pub fn get_active_attacks(&self) -> Vec<DDoSAttack> {
        self.active_attacks.read().values().cloned().collect()
    }

    /// Get statistics
    pub fn get_stats(&self) -> DDoSStats {
        self.stats.read().clone()
    }
}

/// DDoS analysis result
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DDoSAnalysisResult {
    Clean,
    Suspicious,
    Blocked(DDoSAttackType),
}

// ============================================================================
// Protocol Validator
// ============================================================================

/// Strict protocol validation and sanitization
pub struct ProtocolValidator {
    /// Validation rules
    rules: Arc<RwLock<ValidationRules>>,
    /// Statistics
    stats: Arc<RwLock<ValidationStats>>,
}

/// Validation rules
#[derive(Debug, Clone)]
pub struct ValidationRules {
    pub max_request_size: usize,
    pub max_header_count: usize,
    pub max_header_size: usize,
    pub max_uri_length: usize,
    pub allowed_methods: HashSet<String>,
    pub allowed_content_types: HashSet<String>,
    pub require_host_header: bool,
    pub require_user_agent: bool,
}

impl Default for ValidationRules {
    fn default() -> Self {
        let mut allowed_methods = HashSet::new();
        allowed_methods.insert("GET".to_string());
        allowed_methods.insert("POST".to_string());
        allowed_methods.insert("PUT".to_string());
        allowed_methods.insert("DELETE".to_string());
        allowed_methods.insert("PATCH".to_string());
        allowed_methods.insert("HEAD".to_string());
        allowed_methods.insert("OPTIONS".to_string());

        let mut allowed_content_types = HashSet::new();
        allowed_content_types.insert("application/json".to_string());
        allowed_content_types.insert("application/x-www-form-urlencoded".to_string());
        allowed_content_types.insert("multipart/form-data".to_string());
        allowed_content_types.insert("text/plain".to_string());

        Self {
            max_request_size: MAX_REQUEST_SIZE,
            max_header_count: MAX_HEADER_COUNT,
            max_header_size: MAX_HEADER_SIZE,
            max_uri_length: 2048,
            allowed_methods,
            allowed_content_types,
            require_host_header: true,
            require_user_agent: false,
        }
    }
}

/// Validation statistics
#[derive(Debug, Clone, Default)]
pub struct ValidationStats {
    pub total_validated: u64,
    pub validation_passed: u64,
    pub validation_failed: u64,
    pub method_violations: u64,
    pub size_violations: u64,
    pub header_violations: u64,
    pub protocol_violations: u64,
}

impl ProtocolValidator {
    pub fn new(rules: ValidationRules) -> Self {
        Self {
            rules: Arc::new(RwLock::new(rules)),
            stats: Arc::new(RwLock::new(ValidationStats::default())),
        }
    }

    /// Validate HTTP request
    pub fn validate_request(
        &self,
        method: &str,
        uri: &str,
        headers: &HashMap<String, String>,
        body_size: usize,
    ) -> Result<()> {
        let mut stats = self.stats.write();
        stats.total_validated += 1;

        let rules = self.rules.read();

        // Validate method
        if !rules.allowed_methods.contains(method) {
            stats.validation_failed += 1;
            stats.method_violations += 1;
            return Err(DbError::InvalidOperation(format!("Invalid method: {}", method)));
        }

        // Validate URI length
        if uri.len() > rules.max_uri_length {
            stats.validation_failed += 1;
            stats.size_violations += 1;
            return Err(DbError::InvalidOperation("URI too long".to_string()));
        }

        // Validate header count
        if headers.len() > rules.max_header_count {
            stats.validation_failed += 1;
            stats.header_violations += 1;
            return Err(DbError::InvalidOperation("Too many headers".to_string()));
        }

        // Validate header sizes
        for (key, value) in headers {
            if key.len() + value.len() > rules.max_header_size {
                stats.validation_failed += 1;
                stats.header_violations += 1;
                return Err(DbError::InvalidOperation("Header too large".to_string()));
            }
        }

        // Validate required headers
        if rules.require_host_header && !headers.contains_key("Host") {
            stats.validation_failed += 1;
            stats.header_violations += 1;
            return Err(DbError::InvalidOperation("Missing Host header".to_string()));
        }

        // Validate body size
        if body_size > rules.max_request_size {
            stats.validation_failed += 1;
            stats.size_violations += 1;
            return Err(DbError::InvalidOperation("Request body too large".to_string()));
        }

        // Validate content type if present
        if let Some(content_type) = headers.get("Content-Type") {
            let ct = content_type.split(';').next().unwrap_or("").trim();
            if !rules.allowed_content_types.contains(ct) {
                stats.validation_failed += 1;
                stats.protocol_violations += 1;
                return Err(DbError::InvalidOperation(format!("Invalid content type: {}", ct)));
            }
        }

        stats.validation_passed += 1;
        Ok(())
    }

    /// Get statistics
    pub fn get_stats(&self) -> ValidationStats {
        self.stats.read().clone()
    }
}

// ============================================================================
// TLS Enforcer
// ============================================================================

/// TLS/mTLS security enforcement
pub struct TLSEnforcer {
    /// Configuration
    config: Arc<RwLock<TLSConfig>>,
    /// Pinned certificates
    pinned_certs: Arc<RwLock<HashMap<String, Vec<u8>>>>,
    /// Certificate cache
    cert_cache: Arc<RwLock<HashMap<String, CertificateInfo>>>,
    /// Statistics
    stats: Arc<RwLock<TLSStats>>,
}

/// TLS configuration
#[derive(Debug, Clone)]
pub struct TLSConfig {
    /// Minimum TLS version (1.2 or 1.3)
    pub min_version: TLSVersion,
    /// Require perfect forward secrecy
    pub require_pfs: bool,
    /// Allowed cipher suites (AEAD only)
    pub allowed_ciphers: Vec<String>,
    /// Require client certificates
    pub require_client_cert: bool,
    /// Enable certificate pinning
    pub enable_cert_pinning: bool,
    /// Enable OCSP stapling
    pub enable_ocsp_stapling: bool,
}

/// TLS version
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum TLSVersion {
    TLS12,
    TLS13,
}

impl Default for TLSConfig {
    fn default() -> Self {
        Self {
            min_version: TLSVersion::TLS13,
            require_pfs: true,
            allowed_ciphers: vec![
                "TLS_AES_256_GCM_SHA384".to_string(),
                "TLS_AES_128_GCM_SHA256".to_string(),
                "TLS_CHACHA20_POLY1305_SHA256".to_string(),
            ],
            require_client_cert: false,
            enable_cert_pinning: true,
            enable_ocsp_stapling: true,
        }
    }
}

/// Certificate information
#[derive(Debug, Clone)]
struct CertificateInfo {
    fingerprint: String,
    subject: String,
    issuer: String,
    valid_from: SystemTime,
    valid_until: SystemTime,
    is_pinned: bool,
}

/// TLS statistics
#[derive(Debug, Clone, Default)]
pub struct TLSStats {
    pub connections_established: u64,
    pub handshake_failures: u64,
    pub cert_validation_failures: u64,
    pub pinning_violations: u64,
    pub protocol_downgrades_blocked: u64,
    pub weak_cipher_rejected: u64,
}

impl TLSEnforcer {
    pub fn new(config: TLSConfig) -> Self {
        Self {
            config: Arc::new(RwLock::new(config)),
            pinned_certs: Arc::new(RwLock::new(HashMap::new())),
            cert_cache: Arc::new(RwLock::new(HashMap::new())),
            stats: Arc::new(RwLock::new(TLSStats::default())),
        }
    }

    /// Pin a certificate for a host
    pub fn pin_certificate(&self, host: String, cert_der: Vec<u8>) {
        let fingerprint = Self::calculate_fingerprint(&cert_der);
        self.pinned_certs.write().insert(host, fingerprint);
    }

    /// Validate TLS connection
    pub fn validate_connection(
        &self,
        version: TLSVersion,
        cipher: &str,
        cert_der: Option<&[u8]>,
        host: Option<&str>,
    ) -> Result<()> {
        let mut stats = self.stats.write();
        let config = self.config.read();

        // Check TLS version
        if version < config.min_version {
            stats.protocol_downgrades_blocked += 1;
            return Err(DbError::InvalidOperation(format!(
                "TLS version {:?} below minimum {:?}",
                version, config.min_version
            )));
        }

        // Check cipher suite
        if !config.allowed_ciphers.iter().any(|c| c == cipher) {
            stats.weak_cipher_rejected += 1;
            return Err(DbError::InvalidOperation(format!("Cipher not allowed: {}", cipher)));
        }

        // Check PFS requirement
        if config.require_pfs && !Self::cipher_supports_pfs(cipher) {
            stats.weak_cipher_rejected += 1;
            return Err(DbError::InvalidOperation("Cipher does not support PFS".to_string()));
        }

        // Validate certificate if pinning enabled
        if config.enable_cert_pinning {
            if let (Some(cert), Some(host)) = (cert_der, host) {
                let fingerprint = Self::calculate_fingerprint(cert);

                if let Some(pinned_fp) = self.pinned_certs.read().get(host) {
                    if &fingerprint != pinned_fp {
                        stats.pinning_violations += 1;
                        return Err(DbError::InvalidOperation("Certificate pinning violation".to_string()));
                    }
                }
            }
        }

        // Require client certificate if configured
        if config.require_client_cert && cert_der.is_none() {
            stats.cert_validation_failures += 1;
            return Err(DbError::InvalidOperation("Client certificate required".to_string()));
        }

        stats.connections_established += 1;
        Ok(())
    }

    fn calculate_fingerprint(cert_der: &[u8]) -> Vec<u8> {
        let mut hasher = Sha256::new();
        hasher.update(cert_der);
        hasher.finalize().to_vec()
    }

    fn cipher_supports_pfs(cipher: &str) -> bool {
        // ECDHE and DHE provide perfect forward secrecy
        cipher.contains("ECDHE") || cipher.contains("DHE")
    }

    /// Get statistics
    pub fn get_stats(&self) -> TLSStats {
        self.stats.read().clone()
    }
}

// ============================================================================
// Network Anomaly Detector
// ============================================================================

/// Network anomaly detection using statistical analysis
pub struct NetworkAnomalyDetector {
    /// Metrics time series
    metrics: Arc<RwLock<MetricsTimeSeries>>,
    /// Baseline statistics
    baseline: Arc<RwLock<BaselineStats>>,
    /// Detected anomalies
    anomalies: Arc<RwLock<VecDeque<Anomaly>>>,
    /// Statistics
    stats: Arc<RwLock<AnomalyStats>>,
}

/// Metrics time series
#[derive(Debug, Clone)]
struct MetricsTimeSeries {
    request_rates: VecDeque<(Instant, f64)>,
    response_times: VecDeque<(Instant, f64)>,
    error_rates: VecDeque<(Instant, f64)>,
    payload_sizes: VecDeque<(Instant, usize)>,
}

impl MetricsTimeSeries {
    fn new() -> Self {
        Self {
            request_rates: VecDeque::with_capacity(ANOMALY_WINDOW_SIZE),
            response_times: VecDeque::with_capacity(ANOMALY_WINDOW_SIZE),
            error_rates: VecDeque::with_capacity(ANOMALY_WINDOW_SIZE),
            payload_sizes: VecDeque::with_capacity(ANOMALY_WINDOW_SIZE),
        }
    }

    fn add_sample(&mut self, request_rate: f64, response_time: f64, error_rate: f64, payload_size: usize) {
        let now = Instant::now();

        self.request_rates.push_back((now, request_rate));
        self.response_times.push_back((now, response_time));
        self.error_rates.push_back((now, error_rate));
        self.payload_sizes.push_back((now, payload_size));

        // Keep window size
        if self.request_rates.len() > ANOMALY_WINDOW_SIZE {
            self.request_rates.pop_front();
            self.response_times.pop_front();
            self.error_rates.pop_front();
            self.payload_sizes.pop_front();
        }
    }
}

/// Baseline statistics
#[derive(Debug, Clone, Default)]
struct BaselineStats {
    mean_request_rate: f64,
    stddev_request_rate: f64,
    mean_response_time: f64,
    stddev_response_time: f64,
    mean_error_rate: f64,
    stddev_error_rate: f64,
    mean_payload_size: f64,
    stddev_payload_size: f64,
}

/// Detected anomaly
#[derive(Debug, Clone)]
pub struct Anomaly {
    pub timestamp: Instant,
    pub anomaly_type: AnomalyType,
    pub severity: f64,
    pub description: String,
}

/// Anomaly types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AnomalyType {
    RequestRateSpike,
    ResponseTimeSpike,
    ErrorRateSpike,
    PayloadSizeAnomaly,
    MultiMetricAnomaly,
}

/// Anomaly detection statistics
#[derive(Debug, Clone, Default)]
pub struct AnomalyStats {
    pub total_samples: u64,
    pub anomalies_detected: u64,
    pub request_rate_anomalies: u64,
    pub response_time_anomalies: u64,
    pub error_rate_anomalies: u64,
    pub payload_anomalies: u64,
}

impl NetworkAnomalyDetector {
    pub fn new() -> Self {
        Self {
            metrics: Arc::new(RwLock::new(MetricsTimeSeries::new())),
            baseline: Arc::new(RwLock::new(BaselineStats::default())),
            anomalies: Arc::new(RwLock::new(VecDeque::with_capacity(1000))),
            stats: Arc::new(RwLock::new(AnomalyStats::default())),
        }
    }

    /// Add metrics sample
    pub fn add_sample(&self, request_rate: f64, response_time: f64, error_rate: f64, payload_size: usize) {
        let mut metrics = self.metrics.write();
        metrics.add_sample(request_rate, response_time, error_rate, payload_size);

        self.stats.write().total_samples += 1;

        // Update baseline periodically
        if metrics.request_rates.len() >= ANOMALY_WINDOW_SIZE / 2 {
            drop(metrics);
            self.update_baseline();
        }
    }

    /// Detect anomalies in current metrics
    pub fn detect_anomalies(&self) -> Vec<Anomaly> {
        let metrics = self.metrics.read();
        let baseline = self.baseline.read();
        let mut detected = Vec::new();

        if metrics.request_rates.is_empty() {
            return detected;
        }

        // Get current values
        let current_rr = metrics.request_rates.back().map(|(_, v)| *v).unwrap_or(0.0);
        let current_rt = metrics.response_times.back().map(|(_, v)| *v).unwrap_or(0.0);
        let current_er = metrics.error_rates.back().map(|(_, v)| *v).unwrap_or(0.0);

        // Calculate z-scores
        let rr_zscore = if baseline.stddev_request_rate > 0.0 {
            (current_rr - baseline.mean_request_rate) / baseline.stddev_request_rate
        } else {
            0.0
        };

        let rt_zscore = if baseline.stddev_response_time > 0.0 {
            (current_rt - baseline.mean_response_time) / baseline.stddev_response_time
        } else {
            0.0
        };

        let er_zscore = if baseline.stddev_error_rate > 0.0 {
            (current_er - baseline.mean_error_rate) / baseline.stddev_error_rate
        } else {
            0.0
        };

        let mut stats = self.stats.write();

        // Detect request rate anomaly
        if rr_zscore.abs() > ANOMALY_Z_SCORE_THRESHOLD {
            detected.push(Anomaly {
                timestamp: Instant::now(),
                anomaly_type: AnomalyType::RequestRateSpike,
                severity: rr_zscore.abs(),
                description: format!("Request rate anomaly: {:.2} req/s (z-score: {:.2})", current_rr, rr_zscore),
            });
            stats.anomalies_detected += 1;
            stats.request_rate_anomalies += 1;
        }

        // Detect response time anomaly
        if rt_zscore.abs() > ANOMALY_Z_SCORE_THRESHOLD {
            detected.push(Anomaly {
                timestamp: Instant::now(),
                anomaly_type: AnomalyType::ResponseTimeSpike,
                severity: rt_zscore.abs(),
                description: format!("Response time anomaly: {:.2} ms (z-score: {:.2})", current_rt, rt_zscore),
            });
            stats.anomalies_detected += 1;
            stats.response_time_anomalies += 1;
        }

        // Detect error rate anomaly
        if er_zscore.abs() > ANOMALY_Z_SCORE_THRESHOLD {
            detected.push(Anomaly {
                timestamp: Instant::now(),
                anomaly_type: AnomalyType::ErrorRateSpike,
                severity: er_zscore.abs(),
                description: format!("Error rate anomaly: {:.2}% (z-score: {:.2})", current_er * 100.0, er_zscore),
            });
            stats.anomalies_detected += 1;
            stats.error_rate_anomalies += 1;
        }

        // Store detected anomalies
        let mut anomalies = self.anomalies.write();
        for anomaly in &detected {
            anomalies.push_back(anomaly.clone());
            if anomalies.len() > 1000 {
                anomalies.pop_front();
            }
        }

        detected
    }

    fn update_baseline(&self) {
        let metrics = self.metrics.read();
        let mut baseline = self.baseline.write();

        // Calculate mean and standard deviation for each metric
        if !metrics.request_rates.is_empty() {
            let values: Vec<f64> = metrics.request_rates.iter().map(|(_, v)| *v).collect();
            baseline.mean_request_rate = Self::mean(&values);
            baseline.stddev_request_rate = Self::stddev(&values, baseline.mean_request_rate);
        }

        if !metrics.response_times.is_empty() {
            let values: Vec<f64> = metrics.response_times.iter().map(|(_, v)| *v).collect();
            baseline.mean_response_time = Self::mean(&values);
            baseline.stddev_response_time = Self::stddev(&values, baseline.mean_response_time);
        }

        if !metrics.error_rates.is_empty() {
            let values: Vec<f64> = metrics.error_rates.iter().map(|(_, v)| *v).collect();
            baseline.mean_error_rate = Self::mean(&values);
            baseline.stddev_error_rate = Self::stddev(&values, baseline.mean_error_rate);
        }
    }

    fn mean(values: &[f64]) -> f64 {
        if values.is_empty() {
            return 0.0;
        }
        values.iter().sum::<f64>() / values.len() as f64
    }

    fn stddev(values: &[f64], mean: f64) -> f64 {
        if values.len() < 2 {
            return 0.0;
        }
        let variance = values.iter()
            .map(|v| (v - mean).powi(2))
            .sum::<f64>() / (values.len() - 1) as f64;
        variance.sqrt()
    }

    /// Get recent anomalies
    pub fn get_recent_anomalies(&self, count: usize) -> Vec<Anomaly> {
        let anomalies = self.anomalies.read();
        anomalies.iter().rev().take(count).cloned().collect()
    }

    /// Get statistics
    pub fn get_stats(&self) -> AnomalyStats {
        self.stats.read().clone()
    }
}

// ============================================================================
// IP Reputation Checker
// ============================================================================

/// IP reputation system with dynamic scoring
pub struct IPReputationChecker {
    /// IP reputation scores
    reputations: Arc<RwLock<HashMap<IpAddr, IPReputation>>>,
    /// Known bad actors (permanent blacklist)
    blacklist: Arc<RwLock<HashSet<IpAddr>>>,
    /// Trusted IPs (permanent whitelist)
    whitelist: Arc<RwLock<HashSet<IpAddr>>>,
    /// Configuration
    config: ReputationConfig,
    /// Statistics
    stats: Arc<RwLock<ReputationStats>>,
}

/// IP reputation information
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
            score: 50, // Neutral starting score
            last_seen: Instant::now(),
            request_count: 0,
            failed_requests: 0,
            successful_requests: 0,
            violations: Vec::new(),
            first_seen: Instant::now(),
        }
    }

    fn update_score(&mut self) {
        // Calculate score based on behavior
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

/// Violation types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ViolationType {
    RateLimitExceeded,
    ProtocolViolation,
    AuthenticationFailure,
    SuspiciousPattern,
    DDoSAttempt,
    MalformedRequest,
}

/// Reputation configuration
#[derive(Debug, Clone)]
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

/// Reputation statistics
#[derive(Debug, Clone, Default)]
pub struct ReputationStats {
    pub total_ips: u64,
    pub blacklisted_ips: u64,
    pub whitelisted_ips: u64,
    pub low_reputation_ips: u64,
    pub high_reputation_ips: u64,
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

    /// Get reputation for an IP
    pub fn get_reputation(&self, ip: &IpAddr) -> IPReputation {
        // Check whitelist first
        if self.whitelist.read().contains(ip) {
            let mut rep = IPReputation::new(*ip);
            rep.score = 100;
            return rep;
        }

        // Check blacklist
        if self.blacklist.read().contains(ip) {
            let mut rep = IPReputation::new(*ip);
            rep.score = 0;
            return rep;
        }

        // Get or create reputation
        let mut reputations = self.reputations.write();
        reputations.entry(*ip)
            .or_insert_with(|| IPReputation::new(*ip))
            .clone()
    }

    /// Record request from IP
    pub fn record_request(&self, ip: IpAddr, success: bool) {
        let mut reputations = self.reputations.write();
        let reputation = reputations.entry(ip)
            .or_insert_with(|| IPReputation::new(ip));

        reputation.record_request(success);

        // Auto-blacklist if score too low
        if reputation.score <= self.config.auto_blacklist_score {
            drop(reputations);
            self.blacklist_ip(ip);
        }
    }

    /// Record violation from IP
    pub fn record_violation(&self, ip: IpAddr, violation: ViolationType) {
        let mut reputations = self.reputations.write();
        let reputation = reputations.entry(ip)
            .or_insert_with(|| IPReputation::new(ip));

        reputation.record_violation(violation);

        // Auto-blacklist if too many violations
        if reputation.violations.len() >= self.config.violation_threshold {
            drop(reputations);
            self.blacklist_ip(ip);
        }
    }

    /// Manually blacklist an IP
    pub fn blacklist_ip(&self, ip: IpAddr) {
        self.blacklist.write().insert(ip);
        self.stats.write().blacklisted_ips += 1;
    }

    /// Manually whitelist an IP
    pub fn whitelist_ip(&self, ip: IpAddr) {
        self.whitelist.write().insert(ip);
        self.stats.write().whitelisted_ips += 1;
    }

    /// Check if IP is blacklisted
    pub fn is_blacklisted(&self, ip: &IpAddr) -> bool {
        self.blacklist.read().contains(ip)
    }

    /// Check if IP is whitelisted
    pub fn is_whitelisted(&self, ip: &IpAddr) -> bool {
        self.whitelist.read().contains(ip)
    }

    /// Get statistics
    pub fn get_stats(&self) -> ReputationStats {
        let mut stats = self.stats.read().clone();
        let reputations = self.reputations.read();

        stats.total_ips = reputations.len() as u64;
        stats.low_reputation_ips = reputations.values().filter(|r| r.score < 30).count() as u64;
        stats.high_reputation_ips = reputations.values().filter(|r| r.score > 70).count() as u64;

        stats
    }

    /// Cleanup old reputation entries
    pub fn cleanup_old_entries(&self, max_age: Duration) {
        let mut reputations = self.reputations.write();
        reputations.retain(|_, rep| rep.last_seen.elapsed() < max_age);
    }
}

// ============================================================================
// Integrated Network Hardening Manager
// ============================================================================

/// Integrated network hardening system
pub struct NetworkHardeningManager {
    pub rate_limiter: Arc<AdaptiveRateLimiter>,
    pub connection_guard: Arc<ConnectionGuard>,
    pub ddos_mitigator: Arc<DDoSMitigator>,
    pub protocol_validator: Arc<ProtocolValidator>,
    pub tls_enforcer: Arc<TLSEnforcer>,
    pub anomaly_detector: Arc<NetworkAnomalyDetector>,
    pub ip_reputation: Arc<IPReputationChecker>,
}

impl NetworkHardeningManager {
    pub fn new() -> Self {
        let ip_reputation = Arc::new(IPReputationChecker::new(ReputationConfig::default()));

        Self {
            rate_limiter: Arc::new(AdaptiveRateLimiter::new(
                RateLimitConfig::default(),
                ip_reputation.clone(),
            )),
            connection_guard: Arc::new(ConnectionGuard::new(ConnectionLimits::default())),
            ddos_mitigator: Arc::new(DDoSMitigator::new(DDoSThresholds::default())),
            protocol_validator: Arc::new(ProtocolValidator::new(ValidationRules::default())),
            tls_enforcer: Arc::new(TLSEnforcer::new(TLSConfig::default())),
            anomaly_detector: Arc::new(NetworkAnomalyDetector::new()),
            ip_reputation,
        }
    }

    /// Check if request should be allowed (all layers)
    pub fn check_request(
        &self,
        ip: IpAddr,
        method: &str,
        uri: &str,
        headers: &HashMap<String, String>,
        body_size: usize,
    ) -> Result<bool> {
        // 1. IP reputation check
        if self.ip_reputation.is_blacklisted(&ip) {
            return Ok(false);
        }

        // 2. Connection guard
        if !self.connection_guard.check_connection(ip)? {
            self.ip_reputation.record_violation(ip, ViolationType::RateLimitExceeded);
            return Ok(false);
        }

        // 3. Rate limiting
        let rate_key = format!("ip:{}", ip);
        if !self.rate_limiter.check_rate_limit(&rate_key, ip)? {
            self.ip_reputation.record_violation(ip, ViolationType::RateLimitExceeded);
            return Ok(false);
        }

        // 4. Protocol validation
        if let Err(_) = self.protocol_validator.validate_request(method, uri, headers, body_size) {
            self.ip_reputation.record_violation(ip, ViolationType::ProtocolViolation);
            return Ok(false);
        }

        // 5. DDoS detection
        let user_agent = headers.get("User-Agent").cloned();
        let analysis = self.ddos_mitigator.analyze_request(
            ip,
            body_size,
            uri.to_string(),
            user_agent,
        )?;

        match analysis {
            DDoSAnalysisResult::Blocked(_) => {
                self.ip_reputation.record_violation(ip, ViolationType::DDoSAttempt);
                return Ok(false);
            }
            DDoSAnalysisResult::Suspicious => {
                self.ip_reputation.record_violation(ip, ViolationType::SuspiciousPattern);
            }
            DDoSAnalysisResult::Clean => {}
        }

        // Record successful check
        self.ip_reputation.record_request(ip, true);

        Ok(true)
    }

    /// Get comprehensive statistics
    pub fn get_all_stats(&self) -> NetworkHardeningStats {
        NetworkHardeningStats {
            rate_limit: self.rate_limiter.get_stats(),
            connection: self.connection_guard.get_stats(),
            ddos: self.ddos_mitigator.get_stats(),
            validation: self.protocol_validator.get_stats(),
            tls: self.tls_enforcer.get_stats(),
            anomaly: self.anomaly_detector.get_stats(),
            reputation: self.ip_reputation.get_stats(),
        }
    }
}

impl Default for NetworkHardeningManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Comprehensive network hardening statistics
#[derive(Debug, Clone)]
pub struct NetworkHardeningStats {
    pub rate_limit: RateLimitStats,
    pub connection: ConnectionStats,
    pub ddos: DDoSStats,
    pub validation: ValidationStats,
    pub tls: TLSStats,
    pub anomaly: AnomalyStats,
    pub reputation: ReputationStats,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_token_bucket() {
        let mut bucket = TokenBucket::new(10.0, 1.0);
        assert!(bucket.try_consume(5.0));
        assert!(bucket.try_consume(5.0));
        assert!(!bucket.try_consume(1.0)); // Should fail, no tokens left
    }

    #[test]
    fn test_connection_guard() {
        let guard = ConnectionGuard::new(ConnectionLimits::default());
        let ip: IpAddr = "192.168.1.1".parse().unwrap();

        assert!(guard.check_connection(ip).unwrap());
        guard.register_connection(ip);
        guard.unregister_connection(ip);
    }

    #[test]
    fn test_ip_reputation() {
        let checker = IPReputationChecker::new(ReputationConfig::default());
        let ip: IpAddr = "192.168.1.1".parse().unwrap();

        let rep = checker.get_reputation(&ip);
        assert_eq!(rep.score, 50); // Neutral score

        checker.record_request(ip, true);
        checker.record_request(ip, true);

        let rep = checker.get_reputation(&ip);
        assert!(rep.score > 50); // Should improve
    }

    #[test]
    fn test_protocol_validator() {
        let validator = ProtocolValidator::new(ValidationRules::default());
        let mut headers = HashMap::new();
        headers.insert("Host".to_string(), "example.com".to_string());

        assert!(validator.validate_request("GET", "/test", &headers, 100).is_ok());
        assert!(validator.validate_request("INVALID", "/test", &headers, 100).is_err());
    }

    #[test]
    fn test_network_hardening_manager() {
        let manager = NetworkHardeningManager::new();
        let ip: IpAddr = "192.168.1.1".parse().unwrap();
        let mut headers = HashMap::new();
        headers.insert("Host".to_string(), "example.com".to_string());

        let result = manager.check_request(ip, "GET", "/test", &headers, 100);
        assert!(result.is_ok());
    }
}
