// # Query Cache Implementation
//
// LRU-based query result caching with TTL expiration and memory management.

use crate::common::{TableId, Value};
use crate::error::{DbError, Result};
use sha2::{Sha256, Digest};
use std::collections::{HashMap, VecDeque};
use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant};

/// Configuration for the query cache
#[derive(Debug, Clone)]
pub struct CacheConfig {
    /// Maximum number of cache entries
    pub max_entries: usize,

    /// Maximum total memory in bytes
    pub max_memory_bytes: usize,

    /// Default TTL for cache entries in seconds
    pub default_ttl_secs: u64,
}

impl Default for CacheConfig {
    fn default() -> Self {
        Self {
            max_entries: super::DEFAULT_MAX_ENTRIES,
            max_memory_bytes: super::DEFAULT_MAX_MEMORY_BYTES,
            default_ttl_secs: super::DEFAULT_TTL_SECONDS,
        }
    }
}

impl CacheConfig {
    /// Validate configuration parameters
    pub fn validate(&self) -> Result<()> {
        if self.max_entries == 0 {
            return Err(DbError::Configuration(
                "max_entries must be greater than 0".to_string(),
            ));
        }
        if self.max_memory_bytes == 0 {
            return Err(DbError::Configuration(
                "max_memory_bytes must be greater than 0".to_string(),
            ));
        }
        if self.default_ttl_secs < super::MIN_TTL_SECONDS {
            return Err(DbError::Configuration(format!(
                "default_ttl_secs must be at least {}",
                super::MIN_TTL_SECONDS
            )));
        }
        if self.default_ttl_secs > super::MAX_TTL_SECONDS {
            return Err(DbError::Configuration(format!(
                "default_ttl_secs must be at most {}",
                super::MAX_TTL_SECONDS
            )));
        }
        Ok(())
    }
}

/// A cached query result entry
#[derive(Debug, Clone)]
pub struct CacheEntry {
    /// The query result data
    pub result: Vec<Vec<Value>>,

    /// Tables this query depends on
    pub dependent_tables: Vec<TableId>,

    /// When this entry was created
    pub created_at: Instant,

    /// Time-to-live duration
    pub ttl: Duration,

    /// Estimated memory size in bytes
    pub memory_size: usize,

    /// Last access time (for LRU)
    pub last_accessed: Instant,

    /// Access count for statistics
    pub access_count: u64,
}

impl CacheEntry {
    /// Create a new cache entry
    pub fn new(
        result: Vec<Vec<Value>>,
        dependent_tables: Vec<TableId>,
        ttl_secs: u64,
    ) -> Self {
        let memory_size = Self::estimate_memory_size(&result);
        let now = Instant::now();

        Self {
            result,
            dependent_tables,
            created_at: now,
            ttl: Duration::from_secs(ttl_secs),
            memory_size,
            last_accessed: now,
            access_count: 0,
        }
    }

    /// Check if entry has expired
    pub fn is_expired(&self) -> bool {
        self.created_at.elapsed() > self.ttl
    }

    /// Update access time and increment counter
    pub fn mark_accessed(&mut self) {
        self.last_accessed = Instant::now();
        self.access_count += 1;
    }

    /// Estimate memory size of result data
    fn estimate_memory_size(result: &[Vec<Value>]) -> usize {
        let mut size = std::mem::size_of::<Vec<Vec<Value>>>();

        for row in result {
            size += std::mem::size_of::<Vec<Value>>();
            for value in row {
                size += match value {
                    Value::Null => 1,
                    Value::Boolean(_) => 1,
                    Value::Integer(_) => 8,
                    Value::Float(_) => 8,
                    Value::String(s) => s.len() + 24, // String overhead
                    Value::Bytes(b) => b.len() + 24,  // Vec overhead
                    Value::Date(_) => 8,
                    Value::Timestamp(_) => 8,
                    Value::Json(j) => j.to_string().len() + 32,
                    Value::Array(a) => a.len() * 64, // Rough estimate
                    Value::Text => 4,
                };
            }
        }

        size
    }
}

/// Cache key for query lookups
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct CacheKey(String);

impl CacheKey {
    /// Generate cache key from query SQL
    fn from_sql(sql: &str) -> Self {
        // Normalize SQL: trim, lowercase, remove extra whitespace
        let normalized = sql
            .trim()
            .to_lowercase()
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" ");

        // Use SHA-256 hash for consistent key generation
        let mut hasher = Sha256::new();
        hasher.update(normalized.as_bytes());
        let hash = hasher.finalize();
        CacheKey(format!("{:x}", hash))
    }
}

/// LRU queue entry for eviction
#[derive(Debug, Clone)]
struct LruEntry {
    key: CacheKey,
    last_accessed: Instant,
}

/// Query cache with LRU eviction and TTL expiration
pub struct QueryCache {
    /// Configuration
    config: CacheConfig,

    /// Cache storage (key -> entry)
    cache: Arc<RwLock<HashMap<CacheKey, CacheEntry>>>,

    /// LRU queue for eviction (oldest first)
    lru_queue: Arc<RwLock<VecDeque<LruEntry>>>,

    /// Table dependencies (table_id -> set of cache keys)
    table_dependencies: Arc<RwLock<HashMap<TableId, Vec<CacheKey>>>>,

    /// Current total memory usage
    memory_usage: Arc<RwLock<usize>>,

    /// Statistics tracking
    statistics: Arc<RwLock<super::CacheStatistics>>,
}

impl QueryCache {
    /// Create a new query cache with configuration
    pub fn new(config: CacheConfig) -> Self {
        config.validate().expect("Invalid cache configuration");

        Self {
            config,
            cache: Arc::new(RwLock::new(HashMap::new())),
            lru_queue: Arc::new(RwLock::new(VecDeque::new())),
            table_dependencies: Arc::new(RwLock::new(HashMap::new())),
            memory_usage: Arc::new(RwLock::new(0)),
            statistics: Arc::new(RwLock::new(super::CacheStatistics::new())),
        }
    }

    /// Get cached query result
    pub fn get(&self, sql: &str) -> Option<Vec<Vec<Value>>> {
        let key = CacheKey::from_sql(sql);

        let mut cache = self.cache.write().unwrap();
        let mut stats = self.statistics.write().unwrap();

        if let Some(entry) = cache.get_mut(&key) {
            // Check if expired
            if entry.is_expired() {
                stats.record_miss();
                stats.record_eviction(super::EvictionReason::Expired);
                self.remove_entry_internal(&key, &mut cache);
                return None;
            }

            // Update access tracking
            entry.mark_accessed();
            stats.record_hit();

            // Update LRU queue
            self.update_lru(&key);

            Some(entry.result.clone())
        } else {
            stats.record_miss();
            None
        }
    }

    /// Put query result in cache
    pub fn put(
        &mut self,
        sql: &str,
        result: Vec<Vec<Value>>,
        dependent_tables: Vec<TableId>,
        custom_ttl_secs: Option<u64>,
    ) -> Result<()> {
        let key = CacheKey::from_sql(sql);
        let ttl_secs = custom_ttl_secs.unwrap_or(self.config.default_ttl_secs);

        // Validate TTL
        if ttl_secs < super::MIN_TTL_SECONDS || ttl_secs > super::MAX_TTL_SECONDS {
            return Err(DbError::InvalidInput(format!(
                "TTL must be between {} and {} seconds",
                super::MIN_TTL_SECONDS,
                super::MAX_TTL_SECONDS
            )));
        }

        let entry = CacheEntry::new(result, dependent_tables.clone(), ttl_secs);
        let entry_size = entry.memory_size;

        // Check if single entry exceeds max memory
        if entry_size > self.config.max_memory_bytes {
            return Err(DbError::LimitExceeded(
                "Single cache entry exceeds maximum memory limit".to_string(),
            ));
        }

        let mut cache = self.cache.write().unwrap();
        let mut memory = self.memory_usage.write().unwrap();

        // Evict entries if necessary
        while *memory + entry_size > self.config.max_memory_bytes
            || cache.len() >= self.config.max_entries
        {
            if !self.evict_lru_entry(&mut cache, &mut memory) {
                break; // No more entries to evict
            }
        }

        // Insert entry
        if let Some(old_entry) = cache.insert(key.clone(), entry) {
            *memory = memory.saturating_sub(old_entry.memory_size);
        }
        *memory += entry_size;

        // Update LRU queue
        drop(memory);
        drop(cache);
        self.add_to_lru(&key);

        // Track table dependencies
        self.add_table_dependencies(&key, &dependent_tables)?;

        Ok(())
    }

    /// Invalidate all cache entries for a table
    pub fn invalidate_table(&mut self, table_id: TableId) -> Result<()> {
        let deps = self.table_dependencies.read().unwrap();
        let keys_to_remove = deps.get(&table_id).cloned().unwrap_or_default();
        drop(deps);

        let mut cache = self.cache.write().unwrap();
        let mut memory = self.memory_usage.write().unwrap();
        let mut stats = self.statistics.write().unwrap();

        for key in keys_to_remove {
            if let Some(entry) = cache.remove(&key) {
                *memory = memory.saturating_sub(entry.memory_size);
                stats.record_eviction(super::EvictionReason::TableInvalidation);
                self.remove_from_lru(&key);
            }
        }

        // Remove table from dependencies
        drop(cache);
        drop(memory);
        drop(stats);
        self.table_dependencies.write().unwrap().remove(&table_id);

        Ok(())
    }

    /// Clear all cache entries
    pub fn clear(&mut self) -> Result<()> {
        let mut cache = self.cache.write().unwrap();
        let mut memory = self.memory_usage.write().unwrap();

        cache.clear();
        *memory = 0;

        drop(cache);
        drop(memory);

        self.lru_queue.write().unwrap().clear();
        self.table_dependencies.write().unwrap().clear();

        Ok(())
    }

    /// Get current memory usage in bytes
    pub fn memory_usage(&self) -> usize {
        *self.memory_usage.read().unwrap()
    }

    /// Get current number of cached entries
    pub fn entry_count(&self) -> usize {
        self.cache.read().unwrap().len()
    }

    /// Get cache statistics
    pub fn statistics(&self) -> super::CacheStatistics {
        self.statistics.read().unwrap().clone()
    }

    /// Update LRU queue when entry is accessed
    fn update_lru(&self, key: &CacheKey) {
        let mut queue = self.lru_queue.write().unwrap();

        // Remove old entry if exists
        queue.retain(|e| &e.key != key);

        // Add to back (most recently used)
        queue.push_back(LruEntry {
            key: key.clone(),
            last_accessed: Instant::now(),
        });
    }

    /// Add entry to LRU queue
    fn add_to_lru(&self, key: &CacheKey) {
        let mut queue = self.lru_queue.write().unwrap();
        queue.push_back(LruEntry {
            key: key.clone(),
            last_accessed: Instant::now(),
        });
    }

    /// Remove entry from LRU queue
    fn remove_from_lru(&self, key: &CacheKey) {
        let mut queue = self.lru_queue.write().unwrap();
        queue.retain(|e| &e.key != key);
    }

    /// Evict least recently used entry
    fn evict_lru_entry(
        &self,
        cache: &mut HashMap<CacheKey, CacheEntry>,
        memory: &mut usize,
    ) -> bool {
        let mut queue = self.lru_queue.write().unwrap();

        if let Some(lru_entry) = queue.pop_front() {
            if let Some(entry) = cache.remove(&lru_entry.key) {
                *memory = memory.saturating_sub(entry.memory_size);
                let mut stats = self.statistics.write().unwrap();
                stats.record_eviction(super::EvictionReason::LruEviction);
                true
            } else {
                false
            }
        } else {
            false
        }
    }

    /// Remove entry and update memory tracking (internal)
    fn remove_entry_internal(
        &self,
        key: &CacheKey,
        cache: &mut HashMap<CacheKey, CacheEntry>,
    ) {
        if let Some(entry) = cache.remove(key) {
            let mut memory = self.memory_usage.write().unwrap();
            *memory = memory.saturating_sub(entry.memory_size);
            self.remove_from_lru(key);
        }
    }

    /// Add table dependencies for cache entry
    fn add_table_dependencies(
        &self,
        key: &CacheKey,
        tables: &[TableId],
    ) -> Result<()> {
        let mut deps = self.table_dependencies.write().unwrap();

        for &table_id in tables {
            deps.entry(table_id)
                .or_insert_with(Vec::new)
                .push(key.clone());
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cache_config_default() {
        let config = CacheConfig::default();
        assert!(config.validate().is_ok());
        assert_eq!(config.max_entries, super::super::DEFAULT_MAX_ENTRIES);
    }

    #[test]
    fn test_cache_config_validation() {
        let mut config = CacheConfig::default();
        config.max_entries = 0;
        assert!(config.validate().is_err());

        config.max_entries = 100;
        config.max_memory_bytes = 0;
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_cache_key_generation() {
        let key1 = CacheKey::from_sql("SELECT * FROM users");
        let key2 = CacheKey::from_sql("SELECT  *  FROM  users"); // Extra spaces
        let key3 = CacheKey::from_sql("SELECT * FROM USERS"); // Different case

        // All should generate same key due to normalization
        assert_eq!(key1, key2);
        assert_eq!(key2, key3);
    }

    #[test]
    fn test_cache_entry_creation() {
        let result = vec![vec![Value::Integer(1), Value::String("test".to_string())]];
        let entry = CacheEntry::new(result, vec![1], 60);

        assert!(!entry.is_expired());
        assert_eq!(entry.access_count, 0);
        assert!(entry.memory_size > 0);
    }

    #[test]
    fn test_cache_basic_operations() {
        let config = CacheConfig {
            max_entries: 100,
            max_memory_bytes: 1024 * 1024,
            default_ttl_secs: 60,
        };

        let mut cache = QueryCache::new(config);

        // Put entry
        let result = vec![vec![Value::Integer(42)]];
        assert!(cache
            .put("SELECT 42", result.clone(), vec![1], None)
            .is_ok());

        // Get entry
        let cached = cache.get("SELECT 42");
        assert!(cached.is_some());
        assert_eq!(cached.unwrap(), result);

        // Statistics
        let stats = cache.statistics();
        assert_eq!(stats.metrics.hits, 1);
        assert_eq!(stats.metrics.misses, 0);
    }

    #[test]
    fn test_cache_miss() {
        let cache = QueryCache::new(CacheConfig::default());
        let result = cache.get("SELECT * FROM nonexistent");
        assert!(result.is_none());

        let stats = cache.statistics();
        assert_eq!(stats.metrics.misses, 1);
    }

    #[test]
    fn test_table_invalidation() {
        let mut cache = QueryCache::new(CacheConfig::default());

        let result = vec![vec![Value::Integer(1)]];
        cache
            .put("SELECT * FROM users", result, vec![1], None)
            .unwrap();

        assert!(cache.get("SELECT * FROM users").is_some());

        // Invalidate table 1
        cache.invalidate_table(1).unwrap();
        assert!(cache.get("SELECT * FROM users").is_none());
    }

    #[test]
    fn test_memory_limit() {
        let config = CacheConfig {
            max_entries: 1000,
            max_memory_bytes: 1024, // Very small limit
            default_ttl_secs: 60,
        };

        let mut cache = QueryCache::new(config);

        // Add entries until memory limit
        for i in 0..10 {
            let sql = format!("SELECT {}", i);
            let result = vec![vec![Value::String("x".repeat(100))]];
            let _ = cache.put(&sql, result, vec![1], None);
        }

        // Should have evicted some entries
        assert!(cache.entry_count() < 10);
    }

    #[test]
    fn test_clear_cache() {
        let mut cache = QueryCache::new(CacheConfig::default());

        let result = vec![vec![Value::Integer(1)]];
        cache.put("SELECT 1", result, vec![1], None).unwrap();

        assert_eq!(cache.entry_count(), 1);
        cache.clear().unwrap();
        assert_eq!(cache.entry_count(), 0);
        assert_eq!(cache.memory_usage(), 0);
    }
}
