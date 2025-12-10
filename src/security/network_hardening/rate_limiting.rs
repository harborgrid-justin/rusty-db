// # Rate Limiting and DDoS Mitigation
//
// Adaptive rate limiting and DDoS attack detection and mitigation.

use std::time::Instant;
use std::collections::{HashSet, VecDeque, HashMap};
use crate::Result;
use parking_lot::RwLock;
use std::net::IpAddr;
use std::sync::Arc;
use std::time::Duration;
use serde::{Serialize, Deserialize};

use super::firewall_rules::IPReputationChecker;

// Constants
const MAX_REQUESTS_PER_SECOND_PER_IP: u64 = 1000;
const DDOS_DETECTION_WINDOW: u64 = 60;
const MIN_REPUTATION_SCORE: i32 = 20;

// ============================================================================
// Adaptive Rate Limiter
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RateLimitConfig {
    pub global_rps: u64,
    pub per_ip_rps: u64,
    pub per_user_rps: u64,
    pub burst_multiplier: f64,
    pub adaptive: bool,
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

pub struct TokenBucket {
    capacity: f64,
    tokens: f64,
    refill_rate: f64,
    last_refill: Instant,
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

pub struct SlidingWindow {
    window_duration: Duration,
    requests: VecDeque<Instant>,
    max_requests: usize,
}

impl SlidingWindow {
    fn new(window_duration: Duration, maxrequests: usize) -> Self {
        Self {
            window_duration,
            requests: VecDeque::new(),
            max_requests: maxrequests,
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

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RateLimitStats {
    pub total_requests: u64,
    pub allowed_requests: u64,
    pub blocked_requests: u64,
    pub adaptive_adjustments: u64,
}

pub struct AdaptiveRateLimiter {
    token_buckets: Arc<RwLock<HashMap<String, TokenBucket>>>,
    sliding_windows: Arc<RwLock<HashMap<String, SlidingWindow>>>,
    config: Arc<RwLock<RateLimitConfig>>,
    ip_reputation: Arc<IPReputationChecker>,
    stats: Arc<RwLock<RateLimitStats>>,
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

    pub fn check_rate_limit(&self, key: &str, ip: IpAddr) -> Result<bool> {
        let mut stats = self.stats.write();
        stats.total_requests += 1;

        let reputation = self.ip_reputation.get_reputation(&ip);

        if reputation.score < MIN_REPUTATION_SCORE {
            stats.blocked_requests += 1;
            return Ok(false);
        }

        let config = self.config.read();

        let rate_adjustment = if config.adaptive {
            1.0 + (reputation.score as f64 - 50.0) / 100.0 * config.reputation_factor
        } else {
            1.0
        };

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

    pub fn get_stats(&self) -> RateLimitStats {
        self.stats.read().clone()
    }
}

// ============================================================================
// DDoS Mitigator
// ============================================================================

#[derive(Debug, Clone)]
struct TrafficPattern {
    requests: VecDeque<Instant>,
    request_sizes: VecDeque<usize>,
    response_codes: VecDeque<u16>,
    endpoints: HashSet<String>,
    user_agents: HashSet<String>,
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DDoSThresholds {
    pub max_rps_per_ip: f64,
    pub max_global_rps: f64,
    pub min_entropy: f64,
    pub max_error_rate: f64,
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DDoSAttack {
    pub attack_id: String,
    pub attack_type: DDoSAttackType,
    pub source_ips: HashSet<IpAddr>,
    #[serde(skip, default = "Instant::now")]
    pub start_time: Instant,
    pub request_count: u64,
    pub mitigation_actions: Vec<MitigationAction>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DDoSAttackType {
    VolumeFlood,
    HttpFlood,
    Slowloris,
    ApplicationLayer,
    Distributed,
    CacheBusting,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MitigationAction {
    RateLimit,
    IpBlock,
    Challenge,
    TrafficShape,
    Alert,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DDoSStats {
    pub attacks_detected: u64,
    pub attacks_mitigated: u64,
    pub ips_blocked: u64,
    pub requests_blocked: u64,
}

pub struct DDoSMitigator {
    patterns: Arc<RwLock<HashMap<IpAddr, TrafficPattern>>>,
    thresholds: DDoSThresholds,
    active_attacks: Arc<RwLock<HashMap<String, DDoSAttack>>>,
    stats: Arc<RwLock<DDoSStats>>,
}

impl DDoSMitigator {
    pub fn new(thresholds: DDoSThresholds) -> Self {
        Self {
            patterns: Arc::new(RwLock::new(HashMap::new())),
            thresholds,
            active_attacks: Arc::new(RwLock::new(HashMap::new())),
            stats: Arc::new(RwLock::new(DDoSStats::default())),
        }
    }

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

        let request_rate = pattern.request_rate();
        let entropy = pattern.entropy();

        if request_rate > self.thresholds.max_rps_per_ip {
            self.detect_attack(
                ip,
                DDoSAttackType::VolumeFlood,
                format!("High request rate: {:.2} req/s", request_rate),
            );
            return Ok(DDoSAnalysisResult::Blocked(DDoSAttackType::VolumeFlood));
        }

        if entropy < self.thresholds.min_entropy && pattern.requests.len() > 100 {
            self.detect_attack(
                ip,
                DDoSAttackType::CacheBusting,
                format!("Low entropy: {:.2}", entropy),
            );
            return Ok(DDoSAnalysisResult::Suspicious);
        }

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
        let attack_id = format!("{}_{}_{}", ip, format!("{:?}", attack_type), Instant::now().elapsed().as_secs());

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

    pub fn get_active_attacks(&self) -> Vec<DDoSAttack> {
        self.active_attacks.read().values().cloned().collect()
    }

    pub fn get_stats(&self) -> DDoSStats {
        self.stats.read().clone()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum DDoSAnalysisResult {
    Clean,
    Suspicious,
    Blocked(DDoSAttackType),
}
