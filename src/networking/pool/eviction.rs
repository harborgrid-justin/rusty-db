// # Connection Eviction
//
// Policies and strategies for evicting idle, expired, or unhealthy connections
// from the pool. Implements LRU, TTL-based, and health-based eviction.

use crate::error::Result;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, VecDeque};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;

/// Connection eviction policy
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum EvictionPolicy {
    /// Least Recently Used - evict connections that haven't been used recently
    LRU,

    /// Time-To-Live - evict connections based on age
    TTL,

    /// Health-based - evict unhealthy connections
    Health,

    /// Idle timeout - evict connections idle for too long
    IdleTimeout,

    /// Composite - combine multiple policies
    Composite,

    /// No eviction
    None,
}

impl Default for EvictionPolicy {
    fn default() -> Self {
        EvictionPolicy::Composite
    }
}

/// Eviction configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvictionConfig {
    /// Eviction policy
    pub policy: EvictionPolicy,

    /// Interval for running eviction checks
    pub eviction_interval: Duration,

    /// Idle timeout before eviction
    pub idle_timeout: Duration,

    /// Maximum connection lifetime
    pub max_lifetime: Duration,

    /// Soft limit - start evicting when pool size exceeds this
    pub soft_limit: usize,

    /// Hard limit - aggressively evict when pool size exceeds this
    pub hard_limit: usize,

    /// Eviction batch size
    pub batch_size: usize,

    /// Enable aggressive eviction when under memory pressure
    pub enable_aggressive_eviction: bool,
}

impl Default for EvictionConfig {
    fn default() -> Self {
        Self {
            policy: EvictionPolicy::Composite,
            eviction_interval: Duration::from_secs(60),
            idle_timeout: Duration::from_secs(300),
            max_lifetime: Duration::from_secs(3600),
            soft_limit: 50,
            hard_limit: 100,
            batch_size: 10,
            enable_aggressive_eviction: true,
        }
    }
}

/// Connection metadata for eviction decisions
#[derive(Debug, Clone)]
pub struct ConnectionMetadata {
    /// Connection identifier
    pub connection_id: u64,

    /// Creation timestamp
    pub created_at: Instant,

    /// Last used timestamp
    pub last_used_at: Instant,

    /// Number of times used
    pub usage_count: u64,

    /// Is connection healthy
    pub is_healthy: bool,

    /// Connection size in bytes (for memory tracking)
    pub size_bytes: usize,
}

impl ConnectionMetadata {
    /// Create new connection metadata
    pub fn new(connection_id: u64) -> Self {
        let now = Instant::now();
        Self {
            connection_id,
            created_at: now,
            last_used_at: now,
            usage_count: 0,
            is_healthy: true,
            size_bytes: 0,
        }
    }

    /// Calculate age of the connection
    pub fn age(&self) -> Duration {
        self.created_at.elapsed()
    }

    /// Calculate idle time
    pub fn idle_time(&self) -> Duration {
        self.last_used_at.elapsed()
    }

    /// Check if connection is expired based on max lifetime
    pub fn is_expired(&self, max_lifetime: Duration) -> bool {
        self.age() > max_lifetime
    }

    /// Check if connection is idle for too long
    pub fn is_idle_expired(&self, idle_timeout: Duration) -> bool {
        self.idle_time() > idle_timeout
    }

    /// Mark connection as used
    pub fn mark_used(&mut self) {
        self.last_used_at = Instant::now();
        self.usage_count += 1;
    }
}

/// Eviction manager
pub struct EvictionManager {
    /// Eviction configuration
    config: EvictionConfig,

    /// Connection metadata tracking
    connections: Arc<RwLock<HashMap<u64, ConnectionMetadata>>>,

    /// LRU access order
    lru_order: Arc<RwLock<VecDeque<u64>>>,

    /// Eviction statistics
    stats: Arc<EvictionStats>,

    /// Background eviction task
    background_task: Option<tokio::task::JoinHandle<()>>,
}

impl EvictionManager {
    /// Create a new eviction manager
    pub fn new(config: EvictionConfig) -> Self {
        Self {
            config,
            connections: Arc::new(RwLock::new(HashMap::new())),
            lru_order: Arc::new(RwLock::new(VecDeque::new())),
            stats: Arc::new(EvictionStats::new()),
            background_task: None,
        }
    }

    /// Register a connection for eviction tracking
    pub async fn register_connection(&self, metadata: ConnectionMetadata) {
        let connection_id = metadata.connection_id;

        let mut connections = self.connections.write().await;
        connections.insert(connection_id, metadata);

        let mut lru = self.lru_order.write().await;
        lru.push_back(connection_id);
    }

    /// Update connection metadata (e.g., after use)
    pub async fn update_connection(&self, connection_id: u64) {
        let mut connections = self.connections.write().await;

        if let Some(metadata) = connections.get_mut(&connection_id) {
            metadata.mark_used();
        }

        // Update LRU order
        let mut lru = self.lru_order.write().await;
        lru.retain(|&id| id != connection_id);
        lru.push_back(connection_id);
    }

    /// Unregister a connection
    pub async fn unregister_connection(&self, connection_id: u64) {
        let mut connections = self.connections.write().await;
        connections.remove(&connection_id);

        let mut lru = self.lru_order.write().await;
        lru.retain(|&id| id != connection_id);
    }

    /// Select connections for eviction
    pub async fn select_evictions(&self) -> Vec<u64> {
        match self.config.policy {
            EvictionPolicy::LRU => self.select_lru_evictions().await,
            EvictionPolicy::TTL => self.select_ttl_evictions().await,
            EvictionPolicy::Health => self.select_health_evictions().await,
            EvictionPolicy::IdleTimeout => self.select_idle_evictions().await,
            EvictionPolicy::Composite => self.select_composite_evictions().await,
            EvictionPolicy::None => Vec::new(),
        }
    }

    /// Select connections for LRU eviction
    async fn select_lru_evictions(&self) -> Vec<u64> {
        let connections = self.connections.read().await;
        let current_size = connections.len();

        if current_size <= self.config.soft_limit {
            return Vec::new();
        }

        let to_evict = if current_size > self.config.hard_limit {
            // Aggressive eviction
            current_size - self.config.soft_limit
        } else {
            // Gentle eviction
            (current_size - self.config.soft_limit).min(self.config.batch_size)
        };

        let lru = self.lru_order.read().await;
        lru.iter().take(to_evict).copied().collect()
    }

    /// Select connections for TTL eviction
    async fn select_ttl_evictions(&self) -> Vec<u64> {
        let connections = self.connections.read().await;
        let mut to_evict = Vec::new();

        for (id, metadata) in connections.iter() {
            if metadata.is_expired(self.config.max_lifetime) {
                to_evict.push(*id);
            }
        }

        to_evict
    }

    /// Select unhealthy connections for eviction
    async fn select_health_evictions(&self) -> Vec<u64> {
        let connections = self.connections.read().await;
        let mut to_evict = Vec::new();

        for (id, metadata) in connections.iter() {
            if !metadata.is_healthy {
                to_evict.push(*id);
            }
        }

        to_evict
    }

    /// Select idle connections for eviction
    async fn select_idle_evictions(&self) -> Vec<u64> {
        let connections = self.connections.read().await;
        let mut to_evict = Vec::new();

        for (id, metadata) in connections.iter() {
            if metadata.is_idle_expired(self.config.idle_timeout) {
                to_evict.push(*id);
            }
        }

        to_evict
    }

    /// Select connections using composite policy
    async fn select_composite_evictions(&self) -> Vec<u64> {
        let mut candidates = Vec::new();

        // First priority: unhealthy connections
        candidates.extend(self.select_health_evictions().await);

        // Second priority: expired connections
        candidates.extend(self.select_ttl_evictions().await);

        // Third priority: idle connections
        candidates.extend(self.select_idle_evictions().await);

        // Fourth priority: LRU if still over limit
        if candidates.is_empty() {
            candidates.extend(self.select_lru_evictions().await);
        }

        // Deduplicate
        candidates.sort_unstable();
        candidates.dedup();

        candidates
    }

    /// Perform eviction
    pub async fn evict(&self, connection_ids: Vec<u64>) -> Result<usize> {
        let mut evicted = 0;

        for connection_id in connection_ids {
            self.unregister_connection(connection_id).await;
            evicted += 1;
        }

        // Update statistics
        self.stats
            .evictions_performed
            .fetch_add(evicted as u64, Ordering::Relaxed);
        self.stats
            .last_eviction_time
            .store(Instant::now().elapsed().as_secs(), Ordering::Relaxed);

        Ok(evicted)
    }

    /// Start background eviction task
    pub fn start_background_eviction(&mut self) {
        let connections = Arc::clone(&self.connections);
        let lru_order = Arc::clone(&self.lru_order);
        let stats = Arc::clone(&self.stats);
        let config = self.config.clone();

        let handle = tokio::spawn(async move {
            let mut ticker = tokio::time::interval(config.eviction_interval);

            loop {
                ticker.tick().await;

                // Check connections for eviction
                let connections_read = connections.read().await;
                let mut to_evict = Vec::new();

                for (id, metadata) in connections_read.iter() {
                    if metadata.is_expired(config.max_lifetime)
                        || metadata.is_idle_expired(config.idle_timeout)
                        || !metadata.is_healthy
                    {
                        to_evict.push(*id);
                    }
                }
                drop(connections_read);

                // Evict connections
                if !to_evict.is_empty() {
                    let mut connections_write = connections.write().await;
                    let mut lru_write = lru_order.write().await;

                    for connection_id in to_evict {
                        connections_write.remove(&connection_id);
                        lru_write.retain(|&id| id != connection_id);

                        stats.evictions_performed.fetch_add(1, Ordering::Relaxed);
                    }
                }
            }
        });

        self.background_task = Some(handle);
    }

    /// Stop background eviction
    pub async fn stop_background_eviction(&mut self) {
        if let Some(handle) = self.background_task.take() {
            handle.abort();
        }
    }

    /// Get eviction statistics
    pub fn statistics(&self) -> EvictionStatistics {
        EvictionStatistics {
            evictions_performed: self.stats.evictions_performed.load(Ordering::Relaxed),
            last_eviction_time: self.stats.last_eviction_time.load(Ordering::Relaxed),
            total_connections_tracked: self.stats.total_connections_tracked.load(Ordering::Relaxed),
        }
    }

    /// Get connection count
    pub async fn connection_count(&self) -> usize {
        let connections = self.connections.read().await;
        connections.len()
    }

    /// Check if eviction is needed
    pub async fn needs_eviction(&self) -> bool {
        let connections = self.connections.read().await;
        connections.len() > self.config.soft_limit
    }
}

impl Drop for EvictionManager {
    fn drop(&mut self) {
        if let Some(handle) = self.background_task.take() {
            handle.abort();
        }
    }
}

/// Eviction statistics (internal)
struct EvictionStats {
    evictions_performed: AtomicU64,
    last_eviction_time: AtomicU64,
    total_connections_tracked: AtomicU64,
}

impl EvictionStats {
    fn new() -> Self {
        Self {
            evictions_performed: AtomicU64::new(0),
            last_eviction_time: AtomicU64::new(0),
            total_connections_tracked: AtomicU64::new(0),
        }
    }
}

/// Eviction statistics (public)
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct EvictionStatistics {
    /// Number of evictions performed
    pub evictions_performed: u64,

    /// Last eviction timestamp (seconds since epoch)
    pub last_eviction_time: u64,

    /// Total connections tracked for eviction
    pub total_connections_tracked: u64,
}

/// Eviction strategy selector
pub struct EvictionStrategySelector {
    /// Available policies and their scores
    policies: HashMap<EvictionPolicy, f64>,

    /// Current policy
    current_policy: EvictionPolicy,

    /// Policy switch threshold
    switch_threshold: f64,
}

impl EvictionStrategySelector {
    /// Create a new strategy selector
    pub fn new() -> Self {
        let mut policies = HashMap::new();
        policies.insert(EvictionPolicy::LRU, 1.0);
        policies.insert(EvictionPolicy::TTL, 1.0);
        policies.insert(EvictionPolicy::Health, 1.0);
        policies.insert(EvictionPolicy::IdleTimeout, 1.0);

        Self {
            policies,
            current_policy: EvictionPolicy::Composite,
            switch_threshold: 0.8,
        }
    }

    /// Update policy score based on performance
    pub fn update_score(&mut self, policy: EvictionPolicy, score: f64) {
        self.policies.insert(policy, score);
    }

    /// Select best policy
    pub fn select_best_policy(&mut self) -> EvictionPolicy {
        let mut best_policy = EvictionPolicy::Composite;
        let mut best_score = 0.0;

        for (policy, score) in self.policies.iter() {
            if *score > best_score {
                best_score = *score;
                best_policy = *policy;
            }
        }

        if best_score > self.switch_threshold {
            self.current_policy = best_policy;
        }

        self.current_policy
    }

    /// Get current policy
    pub fn current_policy(&self) -> EvictionPolicy {
        self.current_policy
    }
}

impl Default for EvictionStrategySelector {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_eviction_policy_default() {
        assert_eq!(EvictionPolicy::default(), EvictionPolicy::Composite);
    }

    #[test]
    fn test_connection_metadata() {
        let mut metadata = ConnectionMetadata::new(1);
        assert_eq!(metadata.connection_id, 1);
        assert_eq!(metadata.usage_count, 0);

        metadata.mark_used();
        assert_eq!(metadata.usage_count, 1);
    }

    #[test]
    fn test_connection_expiration() {
        let metadata = ConnectionMetadata {
            connection_id: 1,
            created_at: Instant::now() - Duration::from_secs(7200),
            last_used_at: Instant::now(),
            usage_count: 0,
            is_healthy: true,
            size_bytes: 0,
        };

        assert!(metadata.is_expired(Duration::from_secs(3600)));
        assert!(!metadata.is_expired(Duration::from_secs(10800)));
    }

    #[test]
    fn test_idle_timeout() {
        let metadata = ConnectionMetadata {
            connection_id: 1,
            created_at: Instant::now(),
            last_used_at: Instant::now() - Duration::from_secs(600),
            usage_count: 0,
            is_healthy: true,
            size_bytes: 0,
        };

        assert!(metadata.is_idle_expired(Duration::from_secs(300)));
        assert!(!metadata.is_idle_expired(Duration::from_secs(1200)));
    }

    #[tokio::test]
    async fn test_eviction_manager_creation() {
        let config = EvictionConfig::default();
        let manager = EvictionManager::new(config);

        assert_eq!(manager.connection_count().await, 0);
    }

    #[tokio::test]
    async fn test_register_connection() {
        let config = EvictionConfig::default();
        let manager = EvictionManager::new(config);

        let metadata = ConnectionMetadata::new(1);
        manager.register_connection(metadata).await;

        assert_eq!(manager.connection_count().await, 1);
    }

    #[tokio::test]
    async fn test_unregister_connection() {
        let config = EvictionConfig::default();
        let manager = EvictionManager::new(config);

        let metadata = ConnectionMetadata::new(1);
        manager.register_connection(metadata).await;
        assert_eq!(manager.connection_count().await, 1);

        manager.unregister_connection(1).await;
        assert_eq!(manager.connection_count().await, 0);
    }

    #[test]
    fn test_strategy_selector() {
        let mut selector = EvictionStrategySelector::new();

        selector.update_score(EvictionPolicy::LRU, 0.9);
        let best = selector.select_best_policy();

        assert_eq!(best, EvictionPolicy::LRU);
    }
}
