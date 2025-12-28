// # Query Caching System
//
// Enterprise-grade query result caching with intelligent invalidation and monitoring.
//
// ## Overview
//
// The cache module provides a high-performance caching layer for query results,
// reducing query execution time and database load. It includes:
//
// - **Query Cache**: LRU-based caching with TTL and memory limits
// - **Cache Invalidation**: Fine-grained invalidation based on table dependencies
// - **Statistics**: Comprehensive metrics for cache hit/miss ratios and performance
//
// ## Features
//
// - **LRU Eviction**: Least Recently Used policy for memory-bounded cache
// - **TTL Expiration**: Time-based expiration for cache entries
// - **Table Tracking**: Automatic invalidation when tables are modified
// - **Row-level Granularity**: Fine-grained invalidation for specific rows
// - **Thread-safe**: Concurrent access with minimal lock contention
// - **Memory Bounded**: Configurable memory limits with automatic eviction
// - **Monitoring**: Real-time statistics and performance metrics
//
// ## Usage
//
// ```rust
// use rusty_db::cache::{QueryCache, CacheConfig};
// use rusty_db::Result;
//
// # fn example() -> Result<()> {
// // Create cache with configuration
// let config = CacheConfig {
//     max_entries: 10000,
//     max_memory_bytes: 1024 * 1024 * 1024, // 1 GB
//     default_ttl_secs: 300, // 5 minutes
// };
//
// let mut cache = QueryCache::new(config);
//
// // Cache a query result
// let query_sql = "SELECT * FROM users WHERE age > 18";
// let result = vec![/* query result data */];
// cache.put(query_sql, result.clone(), vec![1], None)?;
//
// // Retrieve from cache
// if let Some(cached) = cache.get(query_sql) {
//     println!("Cache hit!");
// }
//
// // Invalidate when table changes
// cache.invalidate_table(1)?;
// # Ok(())
// # }
// ```
//
// ## Architecture
//
// ```text
// ┌─────────────────────────────────────────────────────────┐
// │                    Query Cache                          │
// │  ┌────────────────┐  ┌──────────────┐  ┌─────────────┐ │
// │  │  Query Hash    │  │   LRU List   │  │   Memory    │ │
// │  │  (Blake3)      │→ │   Eviction   │→ │   Tracking  │ │
// │  └────────────────┘  └──────────────┘  └─────────────┘ │
// └─────────────────────────────────────────────────────────┘
//                              ↓
// ┌─────────────────────────────────────────────────────────┐
// │              Cache Invalidation Engine                  │
// │  ┌────────────────┐  ┌──────────────┐  ┌─────────────┐ │
// │  │  Table → Query │  │  Row → Query │  │  Dependency │ │
// │  │  Mapping       │  │  Mapping     │  │  Graph      │ │
// │  └────────────────┘  └──────────────┘  └─────────────┘ │
// └─────────────────────────────────────────────────────────┘
//                              ↓
// ┌─────────────────────────────────────────────────────────┐
// │                 Cache Statistics                        │
// │  ┌────────────────┐  ┌──────────────┐  ┌─────────────┐ │
// │  │   Hit/Miss     │  │   Latency    │  │   Memory    │ │
// │  │   Counters     │  │   Tracking   │  │   Usage     │ │
// │  └────────────────┘  └──────────────┘  └─────────────┘ │
// └─────────────────────────────────────────────────────────┘
// ```
//
// ## Performance Characteristics
//
// - **Get**: O(1) average case hash lookup + O(1) LRU update
// - **Put**: O(1) average case insertion + O(1) dependency tracking
// - **Invalidation**: O(k) where k is number of affected cache entries
// - **Memory**: Bounded by configuration, automatic eviction
// - **Concurrency**: Read-write locks minimize contention
//
// ## Cache Key Generation
//
// Cache keys are generated using BLAKE3 cryptographic hash of:
// - Normalized SQL query text
// - Query parameters (if any)
// - Transaction isolation level
// - Database schema version
//
// This ensures queries with different semantics are cached separately.

pub mod query_cache;
pub mod cache_invalidation;
pub mod cache_statistics;

// Re-export public types
pub use query_cache::{QueryCache, CacheConfig, CacheEntry};
pub use cache_invalidation::{CacheInvalidator, InvalidationStrategy, TableDependency};
pub use cache_statistics::{CacheStatistics, CacheMetrics, EvictionReason};

use crate::error::Result;

/// Default maximum number of cache entries
pub const DEFAULT_MAX_ENTRIES: usize = 10_000;

/// Default maximum memory in bytes (1 GB)
pub const DEFAULT_MAX_MEMORY_BYTES: usize = 1024 * 1024 * 1024;

/// Default TTL in seconds (5 minutes)
pub const DEFAULT_TTL_SECONDS: u64 = 300;

/// Minimum TTL in seconds (1 second)
pub const MIN_TTL_SECONDS: u64 = 1;

/// Maximum TTL in seconds (24 hours)
pub const MAX_TTL_SECONDS: u64 = 86400;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cache_constants() {
        assert!(DEFAULT_MAX_ENTRIES > 0);
        assert!(DEFAULT_MAX_MEMORY_BYTES > 0);
        assert!(DEFAULT_TTL_SECONDS >= MIN_TTL_SECONDS);
        assert!(DEFAULT_TTL_SECONDS <= MAX_TTL_SECONDS);
    }
}
