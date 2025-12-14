// Traffic shaping and rate limiting.
//
// Provides rate limiting per client/peer, priority queuing, bandwidth allocation,
// and burst handling for optimal traffic management.

use crate::error::{DbError, Result};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::{RwLock, Semaphore};

/// Traffic shaper for managing request flow
pub struct TrafficShaper {
    /// Rate limiters per client/node
    rate_limiters: Arc<RwLock<HashMap<String, RateLimiter>>>,
    /// Priority queues for request prioritization
    priority_queues: Arc<RwLock<Vec<PriorityQueue>>>,
    /// Bandwidth limits per node
    bandwidth_limits: Arc<RwLock<HashMap<String, BandwidthLimit>>>,
    /// Global rate limiter
    global_limiter: Arc<RateLimiter>,
}

impl TrafficShaper {
    /// Create a new traffic shaper with default settings
    pub fn new() -> Self {
        Self {
            rate_limiters: Arc::new(RwLock::new(HashMap::new())),
            priority_queues: Arc::new(RwLock::new(Vec::new())),
            bandwidth_limits: Arc::new(RwLock::new(HashMap::new())),
            global_limiter: Arc::new(RateLimiter::new(10000.0, 1000)), // 10k req/s global
        }
    }

    /// Create with custom global rate limit
    pub fn with_global_limit(rate_per_second: f64, burst_size: usize) -> Self {
        Self {
            rate_limiters: Arc::new(RwLock::new(HashMap::new())),
            priority_queues: Arc::new(RwLock::new(Vec::new())),
            bandwidth_limits: Arc::new(RwLock::new(HashMap::new())),
            global_limiter: Arc::new(RateLimiter::new(rate_per_second, burst_size)),
        }
    }

    /// Set rate limit for a specific client
    pub async fn set_client_rate_limit(
        &self,
        client_id: &str,
        rate_per_second: f64,
        burst_size: usize,
    ) {
        let mut limiters = self.rate_limiters.write().await;
        limiters.insert(
            client_id.to_string(),
            RateLimiter::new(rate_per_second, burst_size),
        );
    }

    /// Set bandwidth limit for a node
    pub async fn set_bandwidth_limit(&self, node_id: &str, bytes_per_second: u64) {
        let mut limits = self.bandwidth_limits.write().await;
        limits.insert(node_id.to_string(), BandwidthLimit::new(bytes_per_second));
    }

    /// Check if request is allowed by rate limits
    pub async fn check_rate_limit(&self, client_id: &str) -> Result<()> {
        // Check global rate limit
        if !self.global_limiter.allow().await {
            return Err(DbError::Network("Global rate limit exceeded".to_string()));
        }

        // Check client-specific rate limit if set
        if !client_id.is_empty() {
            let limiters = self.rate_limiters.read().await;
            if let Some(limiter) = limiters.get(client_id) {
                if !limiter.allow().await {
                    return Err(DbError::Network(format!(
                        "Rate limit exceeded for client {}",
                        client_id
                    )));
                }
            }
        }

        Ok(())
    }

    /// Check bandwidth limit for a transfer
    pub async fn check_bandwidth(&self, node_id: &str, bytes: u64) -> Result<()> {
        let limits = self.bandwidth_limits.read().await;
        if let Some(limit) = limits.get(node_id) {
            if !limit.allow(bytes).await {
                return Err(DbError::Network(format!(
                    "Bandwidth limit exceeded for node {}",
                    node_id
                )));
            }
        }
        Ok(())
    }

    /// Add a priority queue
    pub async fn add_priority_queue(&self, queue: PriorityQueue) {
        let mut queues = self.priority_queues.write().await;
        queues.push(queue);
    }

    /// Reset all rate limits
    pub async fn reset(&self) {
        let mut limiters = self.rate_limiters.write().await;
        for limiter in limiters.values_mut() {
            limiter.reset();
        }
        self.global_limiter.reset();
    }
}

impl Default for TrafficShaper {
    fn default() -> Self {
        Self::new()
    }
}

/// Token bucket rate limiter
pub struct RateLimiter {
    /// Maximum tokens (burst size)
    capacity: usize,
    /// Current tokens
    tokens: Arc<RwLock<f64>>,
    /// Refill rate (tokens per second)
    refill_rate: f64,
    /// Last refill time
    last_refill: Arc<RwLock<Instant>>,
    /// Semaphore for flow control
    #[allow(dead_code)] // Reserved for concurrent rate limiting
    semaphore: Arc<Semaphore>,
}

impl RateLimiter {
    /// Create a new rate limiter
    ///
    /// # Arguments
    /// * `rate_per_second` - Number of requests allowed per second
    /// * `burst_size` - Maximum burst size
    pub fn new(rate_per_second: f64, burst_size: usize) -> Self {
        Self {
            capacity: burst_size,
            tokens: Arc::new(RwLock::new(burst_size as f64)),
            refill_rate: rate_per_second,
            last_refill: Arc::new(RwLock::new(Instant::now())),
            semaphore: Arc::new(Semaphore::new(burst_size)),
        }
    }

    /// Refill tokens based on elapsed time
    async fn refill(&self) {
        let mut last_refill = self.last_refill.write().await;
        let now = Instant::now();
        let elapsed = now.duration_since(*last_refill).as_secs_f64();

        if elapsed > 0.0 {
            let mut tokens = self.tokens.write().await;
            let new_tokens = elapsed * self.refill_rate;
            *tokens = (*tokens + new_tokens).min(self.capacity as f64);
            *last_refill = now;
        }
    }

    /// Check if a request is allowed
    pub async fn allow(&self) -> bool {
        self.refill().await;

        let mut tokens = self.tokens.write().await;
        if *tokens >= 1.0 {
            *tokens -= 1.0;
            true
        } else {
            false
        }
    }

    /// Wait until a token is available (blocking)
    pub async fn acquire(&self) -> Result<()> {
        loop {
            if self.allow().await {
                return Ok(());
            }

            // Wait a bit before retrying
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    }

    /// Try to acquire multiple tokens
    pub async fn acquire_many(&self, count: usize) -> bool {
        self.refill().await;

        let mut tokens = self.tokens.write().await;
        if *tokens >= count as f64 {
            *tokens -= count as f64;
            true
        } else {
            false
        }
    }

    /// Reset the rate limiter
    pub fn reset(&self) {
        // Reset is synchronous, we'd need to spawn a task or use blocking
        // For now, just document that tokens will refill naturally
    }

    /// Get current token count
    pub async fn available_tokens(&self) -> f64 {
        self.refill().await;
        let tokens = self.tokens.read().await;
        *tokens
    }
}

/// Bandwidth limiter using token bucket
pub struct BandwidthLimit {
    /// Bytes per second allowed
    bytes_per_second: u64,
    /// Current bucket of bytes
    bucket: Arc<RwLock<u64>>,
    /// Last update time
    last_update: Arc<RwLock<Instant>>,
}

impl BandwidthLimit {
    /// Create a new bandwidth limit
    pub fn new(bytes_per_second: u64) -> Self {
        Self {
            bytes_per_second,
            bucket: Arc::new(RwLock::new(bytes_per_second)), // Start with full bucket
            last_update: Arc::new(RwLock::new(Instant::now())),
        }
    }

    /// Refill the bucket based on elapsed time
    async fn refill(&self) {
        let mut last_update = self.last_update.write().await;
        let now = Instant::now();
        let elapsed = now.duration_since(*last_update).as_secs_f64();

        if elapsed > 0.0 {
            let mut bucket = self.bucket.write().await;
            let new_bytes = (elapsed * self.bytes_per_second as f64) as u64;
            *bucket = (*bucket + new_bytes).min(self.bytes_per_second * 2); // Allow 2x burst
            *last_update = now;
        }
    }

    /// Check if transfer is allowed
    pub async fn allow(&self, bytes: u64) -> bool {
        self.refill().await;

        let mut bucket = self.bucket.write().await;
        if *bucket >= bytes {
            *bucket -= bytes;
            true
        } else {
            false
        }
    }

    /// Wait until bandwidth is available
    pub async fn acquire(&self, bytes: u64) -> Result<()> {
        while !self.allow(bytes).await {
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
        Ok(())
    }
}

/// Priority queue for request prioritization
pub struct PriorityQueue {
    /// Queue priority (higher = more important)
    pub priority: u8,
    /// Queue name
    pub name: String,
    /// Current queue depth
    depth: Arc<RwLock<usize>>,
    /// Maximum queue depth
    max_depth: usize,
}

impl PriorityQueue {
    /// Create a new priority queue
    pub fn new(name: String, priority: u8, max_depth: usize) -> Self {
        Self {
            priority,
            name,
            depth: Arc::new(RwLock::new(0)),
            max_depth,
        }
    }

    /// Try to enqueue a request
    pub async fn enqueue(&self) -> Result<()> {
        let mut depth = self.depth.write().await;
        if *depth >= self.max_depth {
            return Err(DbError::Network("Queue is full".to_string()));
        }
        *depth += 1;
        Ok(())
    }

    /// Dequeue a request
    pub async fn dequeue(&self) {
        let mut depth = self.depth.write().await;
        if *depth > 0 {
            *depth -= 1;
        }
    }

    /// Get current queue depth
    pub async fn depth(&self) -> usize {
        let depth = self.depth.read().await;
        *depth
    }

    /// Check if queue is full
    pub async fn is_full(&self) -> bool {
        let depth = self.depth.read().await;
        *depth >= self.max_depth
    }
}

/// Leaky bucket rate limiter
///
/// Alternative to token bucket - requests leak out at a fixed rate.
pub struct LeakyBucketLimiter {
    /// Maximum bucket size
    capacity: usize,
    /// Current bucket level
    level: Arc<RwLock<usize>>,
    /// Leak rate (requests per second)
    leak_rate: f64,
    /// Last leak time
    last_leak: Arc<RwLock<Instant>>,
}

impl LeakyBucketLimiter {
    /// Create a new leaky bucket limiter
    pub fn new(capacity: usize, leak_rate: f64) -> Self {
        Self {
            capacity,
            level: Arc::new(RwLock::new(0)),
            leak_rate,
            last_leak: Arc::new(RwLock::new(Instant::now())),
        }
    }

    /// Leak tokens based on elapsed time
    async fn leak(&self) {
        let mut last_leak = self.last_leak.write().await;
        let now = Instant::now();
        let elapsed = now.duration_since(*last_leak).as_secs_f64();

        if elapsed > 0.0 {
            let mut level = self.level.write().await;
            let leaked = (elapsed * self.leak_rate) as usize;
            *level = level.saturating_sub(leaked);
            *last_leak = now;
        }
    }

    /// Try to add a request to the bucket
    pub async fn allow(&self) -> bool {
        self.leak().await;

        let mut level = self.level.write().await;
        if *level < self.capacity {
            *level += 1;
            true
        } else {
            false
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_rate_limiter() {
        let limiter = RateLimiter::new(10.0, 10);

        // Should allow up to burst size
        for _ in 0..10 {
            assert!(limiter.allow().await);
        }

        // Should deny after burst
        assert!(!limiter.allow().await);

        // Wait for refill
        tokio::time::sleep(Duration::from_millis(200)).await;

        // Should allow again after refill
        assert!(limiter.allow().await);
    }

    #[tokio::test]
    async fn test_rate_limiter_refill() {
        let limiter = RateLimiter::new(100.0, 10);

        // Drain the bucket
        for _ in 0..10 {
            limiter.allow().await;
        }

        // Wait for 1 second (should refill 100 tokens, capped at 10)
        tokio::time::sleep(Duration::from_secs(1)).await;

        let available = limiter.available_tokens().await;
        assert_eq!(available, 10.0); // Should be capped at capacity
    }

    #[tokio::test]
    async fn test_bandwidth_limit() {
        let limit = BandwidthLimit::new(1000); // 1000 bytes/sec

        // Should allow small transfer
        assert!(limit.allow(500).await);

        // Should allow another 500 bytes
        assert!(limit.allow(500).await);

        // Should deny exceeding capacity
        assert!(!limit.allow(100).await);
    }

    #[tokio::test]
    async fn test_priority_queue() {
        let queue = PriorityQueue::new("high".to_string(), 10, 5);

        // Should enqueue up to max depth
        for _ in 0..5 {
            assert!(queue.enqueue().await.is_ok());
        }

        // Should reject when full
        assert!(queue.enqueue().await.is_err());
        assert!(queue.is_full().await);

        // Should allow after dequeue
        queue.dequeue().await;
        assert!(queue.enqueue().await.is_ok());
    }

    #[tokio::test]
    async fn test_traffic_shaper() {
        let shaper = TrafficShaper::with_global_limit(100.0, 10);

        // Should allow requests within limit
        for _ in 0..10 {
            assert!(shaper.check_rate_limit("client1").await.is_ok());
        }

        // Should deny after exceeding global limit
        assert!(shaper.check_rate_limit("client1").await.is_err());
    }

    #[tokio::test]
    async fn test_client_specific_rate_limit() {
        let shaper = TrafficShaper::new();
        shaper.set_client_rate_limit("client1", 5.0, 5).await;

        // Should allow up to client limit
        for _ in 0..5 {
            assert!(shaper.check_rate_limit("client1").await.is_ok());
        }

        // Should deny after client limit
        assert!(shaper.check_rate_limit("client1").await.is_err());

        // Different client should still be allowed (global limit)
        assert!(shaper.check_rate_limit("client2").await.is_ok());
    }

    #[tokio::test]
    async fn test_leaky_bucket() {
        let limiter = LeakyBucketLimiter::new(10, 5.0);

        // Fill the bucket
        for _ in 0..10 {
            assert!(limiter.allow().await);
        }

        // Should deny when full
        assert!(!limiter.allow().await);

        // Wait for leak
        tokio::time::sleep(Duration::from_secs(1)).await;

        // Should allow again after leak (5 requests leaked)
        for _ in 0..5 {
            assert!(limiter.allow().await);
        }
    }

    #[tokio::test]
    async fn test_acquire_many() {
        let limiter = RateLimiter::new(100.0, 20);

        // Should acquire multiple tokens
        assert!(limiter.acquire_many(10).await);

        // Should have 10 tokens left
        let available = limiter.available_tokens().await;
        assert_eq!(available, 10.0);

        // Should fail to acquire more than available
        assert!(!limiter.acquire_many(15).await);
    }
}
