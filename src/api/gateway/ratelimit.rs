// Gateway Module
//
// Part of the API Gateway and Security system for RustyDB

use std::collections::{HashMap, HashSet, VecDeque};
use std::net::IpAddr;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use sha2::{Sha256, Digest};
use hmac::Hmac;
use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};
use uuid::Uuid;

use crate::error::DbError;
use super::types::*;

// ============================================================================
// Rate Limiting & Throttling - Token Bucket, Sliding Window
// ============================================================================

// Rate limiter
pub struct RateLimiter {
    // Rate limit configurations
    configs: Arc<RwLock<HashMap<String, RateLimitConfig>>>,
    // Token buckets
    buckets: Arc<RwLock<HashMap<String, TokenBucket>>>,
    // Sliding windows
    windows: Arc<RwLock<HashMap<String, SlidingWindow>>>,
    // Quota manager
    quota_manager: Arc<QuotaManager>,
}

// Rate limit configuration
#[derive(Debug, Clone)]
pub struct RateLimitConfig {
    // Limit type
    pub limit_type: RateLimitType,
    // Requests per window
    pub requests: u64,
    // Window duration (seconds)
    pub window: u64,
    // Burst size
    pub burst: Option<u64>,
}

// Rate limit type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RateLimitType {
    // Token bucket algorithm
    TokenBucket,
    // Sliding window
    SlidingWindow,
    // Fixed window
    FixedWindow,
}

// Token bucket
pub struct TokenBucket {
    // Capacity
    capacity: u64,
    // Current tokens
    tokens: f64,
    // Refill rate (tokens per second)
    refill_rate: f64,
    // Last refill time
    last_refill: Instant,
}

// Sliding window
pub struct SlidingWindow {
    // Window size (seconds)
    window_size: u64,
    // Request timestamps
    requests: VecDeque<Instant>,
    // Max requests
    max_requests: u64,
}

// Quota manager
pub struct QuotaManager {
    // User quotas
    quotas: Arc<RwLock<HashMap<String, UserQuota>>>,
}

// User quota
#[derive(Debug, Clone)]
pub struct UserQuota {
    // User ID
    pub user_id: String,
    // Daily limit
    pub daily_limit: u64,
    // Monthly limit
    pub monthly_limit: u64,
    // Current daily usage
    pub daily_usage: u64,
    // Current monthly usage
    pub monthly_usage: u64,
    // Reset timestamp
    pub daily_reset: SystemTime,
    // Monthly reset timestamp
    pub monthly_reset: SystemTime,
}

impl RateLimiter {
    // Create new rate limiter
    pub fn new() -> Self {
        Self {
            configs: Arc::new(RwLock::new(HashMap::new())),
            buckets: Arc::new(RwLock::new(HashMap::new())),
            windows: Arc::new(RwLock::new(HashMap::new())),
            quota_manager: Arc::new(QuotaManager::new()),
        }
    }

    // Configure rate limit
    pub fn configure(&self, key: String, config: RateLimitConfig) {
        let mut configs = self.configs.write();
        configs.insert(key, config);
    }

    // Check rate limit
    pub fn check_rate_limit(&self, key: &str, override_config: Option<&RateLimitConfig>) -> Result<(), DbError> {
        let configs = self.configs.read();
        let config = override_config.or_else(|| configs.get(key));

        let config = match config {
            Some(c) => c,
            None => return Ok(()), // No limit configured
        };

        match config.limit_type {
            RateLimitType::TokenBucket => {
                self.check_token_bucket(key, config)
            },
            RateLimitType::SlidingWindow => {
                self.check_sliding_window(key, config)
            },
            RateLimitType::FixedWindow => {
                self.check_fixed_window(key, config)
            },
        }
    }

    // Check token bucket
    fn check_token_bucket(&self, key: &str, config: &RateLimitConfig) -> Result<(), DbError> {
        let mut buckets = self.buckets.write();

        let bucket = buckets.entry(key.to_string()).or_insert_with(|| {
            TokenBucket::new(
                config.burst.unwrap_or(config.requests),
                config.requests as f64 / config.window as f64,
            )
        });

        bucket.consume(1)
    }

    // Check sliding window
    fn check_sliding_window(&self, key: &str, config: &RateLimitConfig) -> Result<(), DbError> {
        let mut windows = self.windows.write();

        let window = windows.entry(key.to_string()).or_insert_with(|| {
            SlidingWindow::new(config.window, config.requests)
        });

        window.allow_request()
    }

    // Check fixed window
    fn check_fixed_window(&self, key: &str, config: &RateLimitConfig) -> Result<(), DbError> {
        // Simplified implementation using sliding window
        self.check_sliding_window(key, config)
    }

    // Get rate limit status
    pub fn get_status(&self, key: &str) -> Option<RateLimitStatus> {
        let buckets = self.buckets.read();
        if let Some(bucket) = buckets.get(key) {
            return Some(RateLimitStatus {
                remaining: bucket.tokens as u64,
                reset_at: Instant::now() + Duration::from_secs(1),
            });
        }

        let windows = self.windows.read();
        if let Some(window) = windows.get(key) {
            return Some(RateLimitStatus {
                remaining: window.max_requests.saturating_sub(window.requests.len() as u64),
                reset_at: Instant::now() + Duration::from_secs(window.window_size),
            });
        }

        None
    }
}

// Rate limit status
#[derive(Debug, Clone)]
pub struct RateLimitStatus {
    // Remaining requests
    pub remaining: u64,
    // Reset time
    pub reset_at: Instant,
}

impl TokenBucket {
    // Create new token bucket
    fn new(capacity: u64, refill_rate: f64) -> Self {
        Self {
            capacity,
            tokens: capacity as f64,
            refill_rate,
            last_refill: Instant::now(),
        }
    }

    // Refill tokens
    fn refill(&mut self) {
        let now = Instant::now();
        let elapsed = now.duration_since(self.last_refill).as_secs_f64();

        let new_tokens = elapsed * self.refill_rate;
        self.tokens = (self.tokens + new_tokens).min(self.capacity as f64);
        self.last_refill = now;
    }

    // Consume tokens
    pub(crate) fn consume(&mut self, amount: u64) -> Result<(), DbError> {
        self.refill();

        if self.tokens >= amount as f64 {
            self.tokens -= amount as f64;
            Ok(())
        } else {
            Err(DbError::InvalidOperation("Rate limit exceeded".to_string()))
        }
    }
}

impl SlidingWindow {
    // Create new sliding window
    fn new(window_size: u64, max_requests: u64) -> Self {
        Self {
            window_size,
            requests: VecDeque::new(),
            max_requests,
        }
    }

    // Clean old requests
    fn clean_old_requests(&mut self) {
        let cutoff = Instant::now() - Duration::from_secs(self.window_size);

        while let Some(&oldest) = self.requests.front() {
            if oldest < cutoff {
                self.requests.pop_front();
            } else {
                break;
            }
        }
    }

    // Allow request
    pub(crate) fn allow_request(&mut self) -> Result<(), DbError> {
        self.clean_old_requests();

        if self.requests.len() < self.max_requests as usize {
            self.requests.push_back(Instant::now());
            Ok(())
        } else {
            Err(DbError::InvalidOperation("Rate limit exceeded".to_string()))
        }
    }
}

impl QuotaManager {
    fn new() -> Self {
        Self {
            quotas: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    // Set user quota
    pub fn set_quota(&self, user_id: String, daily_limit: u64, monthly_limit: u64) {
        let mut quotas = self.quotas.write();

        let now = SystemTime::now();
        quotas.insert(user_id.clone(), UserQuota {
            user_id,
            daily_limit,
            monthly_limit,
            daily_usage: 0,
            monthly_usage: 0,
            daily_reset: now + Duration::from_secs(86400),
            monthly_reset: now + Duration::from_secs(30 * 86400),
        });
    }

    // Check and update quota
    pub fn check_quota(&self, user_id: &str) -> Result<(), DbError> {
        let mut quotas = self.quotas.write();

        let quota = match quotas.get_mut(user_id) {
            Some(q) => q,
            None => return Ok(()), // No quota set
        };

        let now = SystemTime::now();

        // Reset daily quota if needed
        if now >= quota.daily_reset {
            quota.daily_usage = 0;
            quota.daily_reset = now + Duration::from_secs(86400);
        }

        // Reset monthly quota if needed
        if now >= quota.monthly_reset {
            quota.monthly_usage = 0;
            quota.monthly_reset = now + Duration::from_secs(30 * 86400);
        }

        // Check limits
        if quota.daily_usage >= quota.daily_limit {
            return Err(DbError::InvalidOperation("Daily quota exceeded".to_string()));
        }
        if quota.monthly_usage >= quota.monthly_limit {
            return Err(DbError::InvalidOperation("Monthly quota exceeded".to_string()));
        }

        // Update usage
        quota.daily_usage += 1;
        quota.monthly_usage += 1;

        Ok(())
    }

    // Get quota status
    pub fn get_quota_status(&self, user_id: &str) -> Option<UserQuota> {
        let quotas = self.quotas.read();
        quotas.get(user_id).cloned()
    }
}
