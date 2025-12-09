// Query Cache with LRU Eviction
//
// This module provides a high-performance query result cache with:
//
// - **LRU Eviction**: Least recently used entries are evicted first
// - **Memory Limits**: Configurable memory and entry count limits
// - **TTL Support**: Time-to-live for cached entries
// - **Statistics**: Hit/miss tracking for monitoring

use std::time::Instant;
use std::collections::VecDeque;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap};
use std::sync::Arc;
use std::time::SystemTime;

// =============================================================================
// Query Cache
// =============================================================================

/// Query result cache with LRU eviction policy.
///
/// Caches query results to avoid re-execution of expensive queries.
/// Uses a combination of entry count and memory limits for eviction.
///
/// # Example
///
/// ```rust,ignore
/// let cache = QueryCache::new(1000);
///
/// // Cache a query result
/// cache.put("SELECT * FROM users".to_string(), result, 300);
///
/// // Retrieve cached result
/// if let Some(result) = cache.get("SELECT * FROM users") {
///     println!("Cache hit!");
/// }
/// ```
pub struct QueryCache {
    /// Cached entries
    cache: Arc<RwLock<HashMap<String, CachedResult>>>,

    /// LRU queue for eviction ordering
    lru_queue: Arc<RwLock<VecDeque<String>>>,

    /// Maximum number of entries
    max_size: usize,

    /// Maximum memory usage in bytes
    max_memory_bytes: usize,

    /// Current memory usage
    current_memory_bytes: Arc<RwLock<usize>>,

    /// Cache hit count
    hit_count: Arc<RwLock<u64>>,

    /// Cache miss count
    miss_count: Arc<RwLock<u64>>,
}

/// Cached query result.
#[derive(Debug, Clone)]
pub struct CachedResult {
    /// Original query
    pub query: String,

    /// Query result data
    pub result: Vec<Vec<String>>,

    /// When the entry was cached
    pub timestamp: SystemTime,

    /// Time-to-live in seconds
    pub ttl_seconds: u64,

    /// Estimated size in bytes
    pub size_bytes: usize,

    /// Number of times accessed
    pub access_count: u64,

    /// Last access time
    pub last_access: SystemTime,
    pub query_hash: i32,
    pub created_at: Instant,
}

impl QueryCache {
    /// Create a new cache with the given maximum entry count.
    ///
    /// Uses a default memory limit of 100MB.
    pub fn new(max_size: usize) -> Self {
        Self::with_memory_limit(max_size, 100 * 1024 * 1024)
    }

    /// Create a new cache with entry count and memory limits.
    pub fn with_memory_limit(max_size: usize, max_memory_bytes: usize) -> Self {
        Self {
            cache: Arc::new(RwLock::new(HashMap::new())),
            lru_queue: Arc::new(RwLock::new(VecDeque::new())),
            max_size,
            max_memory_bytes,
            current_memory_bytes: Arc::new(RwLock::new(0)),
            hit_count: Arc::new(RwLock::new(0)),
            miss_count: Arc::new(RwLock::new(0)),
        }
    }

    /// Get a cached query result.
    ///
    /// Returns `None` if the query is not cached or has expired.
    pub fn get(&self, query: &str) -> Option<Vec<Vec<String>>> {
        let mut cache = self.cache.write();

        if let Some(cached) = cache.get_mut(query) {
            let elapsed = SystemTime::now()
                .duration_since(cached.timestamp)
                .unwrap_or_default()
                .as_secs();

            if elapsed < cached.ttl_seconds {
                // Update access statistics
                cached.access_count += 1;
                cached.last_access = SystemTime::now();
                *self.hit_count.write() += 1;

                // Move to back of LRU queue
                let mut lru = self.lru_queue.write();
                if let Some(pos) = lru.iter().position(|q| q == query) {
                    lru.remove(pos);
                }
                lru.push_back(query.to_string());

                return Some(cached.result.clone());
            } else {
                // Expired entry - remove it
                let size = cached.size_bytes;
                cache.remove(query);
                *self.current_memory_bytes.write() -= size;

                let mut lru = self.lru_queue.write();
                if let Some(pos) = lru.iter().position(|q| q == query) {
                    lru.remove(pos);
                }
            }
        }

        *self.miss_count.write() += 1;
        None
    }

    /// Cache a query result with the given TTL.
    pub fn put(&self, query: String, result: Vec<Vec<String>>, ttl_seconds: u64) {
        let size_bytes = Self::estimate_size(&result);

        // Evict if necessary
        self.evict_if_needed(size_bytes);

        let mut cache = self.cache.write();
        let mut lru = self.lru_queue.write();

        // Remove old entry if exists
        if let Some(old) = cache.remove(&query) {
            *self.current_memory_bytes.write() -= old.size_bytes;
            if let Some(pos) = lru.iter().position(|q| q == &query) {
                lru.remove(pos);
            }
        }

        let now = SystemTime::now();
        cache.insert(
            query.clone(),
            CachedResult {
                query: query.clone(),
                result,
                timestamp: now,
                ttl_seconds,
                size_bytes,
                access_count: 0,
                last_access: now,
                query_hash: 0,
                created_at: Instant::now(),
            },
        );

        lru.push_back(query);
        *self.current_memory_bytes.write() += size_bytes;
    }

    /// Evict entries if needed to make room for new entry.
    fn evict_if_needed(&self, incoming_size: usize) {
        loop {
            let current_memory = *self.current_memory_bytes.read();
            let cache_len = self.cache.read().len();
            let lru_empty = self.lru_queue.read().is_empty();

            if (current_memory + incoming_size <= self.max_memory_bytes
                && cache_len < self.max_size)
                || lru_empty
            {
                break;
            }

            // Evict the least recently used entry
            let query_to_evict = { self.lru_queue.write().pop_front() };

            if let Some(query) = query_to_evict {
                let mut cache = self.cache.write();
                if let Some(removed) = cache.remove(&query) {
                    *self.current_memory_bytes.write() -= removed.size_bytes;
                }
            }
        }
    }

    /// Estimate the size of a result set in bytes.
    fn estimate_size(result: &Vec<Vec<String>>) -> usize {
        let mut size = size_of::<Vec<Vec<String>>>();
        for row in result {
            size += size_of::<Vec<String>>();
            for val in row {
                size += val.len() + size_of::<String>();
            }
        }
        size
    }

    /// Invalidate a specific query.
    pub fn invalidate(&self, query: &str) {
        let mut cache = self.cache.write();
        if let Some(removed) = cache.remove(query) {
            *self.current_memory_bytes.write() -= removed.size_bytes;

            let mut lru = self.lru_queue.write();
            if let Some(pos) = lru.iter().position(|q| q == query) {
                lru.remove(pos);
            }
        }
    }

    /// Invalidate all queries matching a pattern.
    pub fn invalidate_pattern(&self, pattern: &str) {
        let keys_to_remove: Vec<String> = self
            .cache
            .read()
            .keys()
            .filter(|k| k.contains(pattern))
            .cloned()
            .collect();

        for key in keys_to_remove {
            self.invalidate(&key);
        }
    }

    /// Invalidate queries referencing a specific table.
    pub fn invalidate_table(&self, table_name: &str) {
        let pattern = format!("FROM {}", table_name);
        self.invalidate_pattern(&pattern);

        // Also check for JOIN references
        let join_pattern = format!("JOIN {}", table_name);
        self.invalidate_pattern(&join_pattern);
    }

    /// Clear all cached entries.
    pub fn clear(&self) {
        self.cache.write().clear();
        self.lru_queue.write().clear();
        *self.current_memory_bytes.write() = 0;
        *self.hit_count.write() = 0;
        *self.miss_count.write() = 0;
    }

    /// Get cache statistics.
    pub fn get_stats(&self) -> CacheStats {
        let hits = *self.hit_count.read();
        let misses = *self.miss_count.read();
        let total = hits + misses;

        CacheStats {
            size: self.cache.read().len(),
            max_size: self.max_size,
            memory_bytes: *self.current_memory_bytes.read(),
            max_memory_bytes: self.max_memory_bytes,
            hit_count: hits,
            miss_count: misses,
            hit_rate: if total > 0 {
                hits as f64 / total as f64
            } else {
                0.0
            },
        }
    }

    /// Get the most accessed queries.
    pub fn get_hot_queries(&self, limit: usize) -> Vec<(String, u64)> {
        let cache = self.cache.read();
        let mut entries: Vec<_> = cache
            .values()
            .map(|c| (c.query.clone(), c.access_count))
            .collect();

        entries.sort_by(|a, b| b.1.cmp(&a.1));
        entries.truncate(limit);
        entries
    }

    /// Get entries that will expire soon.
    pub fn get_expiring_soon(&self, within_seconds: u64) -> Vec<String> {
        let cache = self.cache.read();
        let now = SystemTime::now();

        cache
            .values()
            .filter(|c| {
                let elapsed = now
                    .duration_since(c.timestamp)
                    .unwrap_or_default()
                    .as_secs();
                let remaining = c.ttl_seconds.saturating_sub(elapsed);
                remaining <= within_seconds
            })
            .map(|c| c.query.clone())
            .collect()
    }
}

// =============================================================================
// Cache Statistics
// =============================================================================

/// Cache statistics for monitoring.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheStats {
    /// Current number of cached entries
    pub size: usize,

    /// Maximum entry count
    pub max_size: usize,

    /// Current memory usage in bytes
    pub memory_bytes: usize,

    /// Maximum memory limit in bytes
    pub max_memory_bytes: usize,

    /// Number of cache hits
    pub hit_count: u64,

    /// Number of cache misses
    pub miss_count: u64,

    /// Hit rate (0.0 to 1.0)
    pub hit_rate: f64,
}

impl CacheStats {
    /// Get the memory utilization percentage.
    pub fn memory_utilization(&self) -> f64 {
        if self.max_memory_bytes == 0 {
            0.0
        } else {
            self.memory_bytes as f64 / self.max_memory_bytes as f64 * 100.0
        }
    }

    /// Get the entry utilization percentage.
    pub fn entry_utilization(&self) -> f64 {
        if self.max_size == 0 {
            0.0
        } else {
            self.size as f64 / self.max_size as f64 * 100.0
        }
    }

    /// Check if the cache is performing well.
    pub fn is_healthy(&self) -> bool {
        self.hit_rate >= 0.7 // At least 70% hit rate
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cache_put_get() {
        let cache = QueryCache::new(100);
        let query = "SELECT * FROM users";
        let result = vec![vec!["1".to_string(), "Alice".to_string()]];

        cache.put(query.to_string(), result.clone(), 60);

        let cached = cache.get(query);
        assert!(cached.is_some());
        assert_eq!(cached.unwrap(), result);
    }

    #[test]
    fn test_cache_miss() {
        let cache = QueryCache::new(100);
        assert!(cache.get("SELECT * FROM nonexistent").is_none());
    }

    #[test]
    fn test_cache_eviction() {
        let cache = QueryCache::new(2);

        cache.put("query1".to_string(), vec![vec!["1".to_string()]], 60);
        cache.put("query2".to_string(), vec![vec!["2".to_string()]], 60);
        cache.put("query3".to_string(), vec![vec!["3".to_string()]], 60);

        // First query should be evicted
        assert!(cache.get("query1").is_none());
        assert!(cache.get("query2").is_some());
        assert!(cache.get("query3").is_some());
    }

    #[test]
    fn test_cache_lru_update() {
        let cache = QueryCache::new(2);

        cache.put("query1".to_string(), vec![vec!["1".to_string()]], 60);
        cache.put("query2".to_string(), vec![vec!["2".to_string()]], 60);

        // Access query1 to make it more recent
        cache.get("query1");

        // Add new query - query2 should be evicted (LRU)
        cache.put("query3".to_string(), vec![vec!["3".to_string()]], 60);

        assert!(cache.get("query1").is_some());
        assert!(cache.get("query2").is_none());
        assert!(cache.get("query3").is_some());
    }

    #[test]
    fn test_cache_invalidate() {
        let cache = QueryCache::new(100);
        cache.put("query1".to_string(), vec![vec!["1".to_string()]], 60);

        cache.invalidate("query1");
        assert!(cache.get("query1").is_none());
    }

    #[test]
    fn test_cache_invalidate_pattern() {
        let cache = QueryCache::new(100);
        cache.put("SELECT * FROM users".to_string(), vec![], 60);
        cache.put("SELECT * FROM orders".to_string(), vec![], 60);
        cache.put("INSERT INTO logs".to_string(), vec![], 60);

        cache.invalidate_pattern("SELECT");

        assert!(cache.get("SELECT * FROM users").is_none());
        assert!(cache.get("SELECT * FROM orders").is_none());
        assert!(cache.get("INSERT INTO logs").is_some());
    }

    #[test]
    fn test_cache_stats() {
        let cache = QueryCache::new(100);
        cache.put("query1".to_string(), vec![], 60);

        cache.get("query1"); // Hit
        cache.get("query2"); // Miss

        let stats = cache.get_stats();
        assert_eq!(stats.size, 1);
        assert_eq!(stats.hit_count, 1);
        assert_eq!(stats.miss_count, 1);
        assert!((stats.hit_rate - 0.5).abs() < 0.001);
    }

    #[test]
    fn test_cache_clear() {
        let cache = QueryCache::new(100);
        cache.put("query1".to_string(), vec![], 60);
        cache.put("query2".to_string(), vec![], 60);

        cache.clear();

        let stats = cache.get_stats();
        assert_eq!(stats.size, 0);
        assert_eq!(stats.memory_bytes, 0);
    }
}
