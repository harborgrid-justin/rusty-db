// Query Plan Caching Module
//
// Provides intelligent query plan caching with LRU eviction

use crate::{Result, error::DbError};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, VecDeque};
use std::sync::{Arc, RwLock};
use std::time::SystemTime;

// Query plan cache with LRU eviction
pub struct QueryPlanCache {
    cache: Arc<RwLock<HashMap<String, CachedPlan>>>,
    access_order: Arc<RwLock<VecDeque<String>>>,
    max_size: usize,
    hit_count: Arc<RwLock<u64>>,
    miss_count: Arc<RwLock<u64>>,
}

impl QueryPlanCache {
    pub fn new(max_size: usize) -> Self {
        Self {
            cache: Arc::new(RwLock::new(HashMap::new())),
            access_order: Arc::new(RwLock::new(VecDeque::new())),
            max_size,
            hit_count: Arc::new(RwLock::new(0)),
            miss_count: Arc::new(RwLock::new(0)),
        }
    }

    // Get a cached plan
    pub fn get(&self, query_hash: &str) -> Result<Option<QueryPlan>> {
        let cache = self.cache.read()
            .map_err(|_| DbError::LockError("Failed to acquire read lock".to_string()))?;

        if let Some(cached) = cache.get(query_hash) {
            // Update hit count
            let mut hits = self.hit_count.write()
                .map_err(|_| DbError::LockError("Failed to acquire write lock".to_string()))?;
            *hits += 1;

            // Update access order
            let mut order = self.access_order.write()
                .map_err(|_| DbError::LockError("Failed to acquire write lock".to_string()))?;
            order.retain(|k| k != query_hash);
            order.push_back(query_hash.to_string());

            Ok(Some(cached.plan.clone()))
        } else {
            // Update miss count
            let mut misses = self.miss_count.write()
                .map_err(|_| DbError::LockError("Failed to acquire write lock".to_string()))?;
            *misses += 1;

            Ok(None)
        }
    }

    // Put a plan in cache
    pub fn put(&self, query_hash: String, plan: QueryPlan, cost: f64) -> Result<()> {
        let mut cache = self.cache.write()
            .map_err(|_| DbError::LockError("Failed to acquire write lock".to_string()))?;

        // Evict if at capacity
        if cache.len() >= self.max_size && !cache.contains_key(&query_hash) {
            let mut order = self.access_order.write()
                .map_err(|_| DbError::LockError("Failed to acquire write lock".to_string()))?;

            if let Some(evict_key) = order.pop_front() {
                cache.remove(&evict_key);
            }
        }

        let cached = CachedPlan {
            plan: plan.clone(),
            _cost: cost,
            _cached_at: SystemTime::now(),
            _access_count: 0,
        };

        cache.insert(query_hash.clone(), cached);

        let mut order = self.access_order.write()
            .map_err(|_| DbError::LockError("Failed to acquire write lock".to_string()))?;
        order.push_back(query_hash);

        Ok(())
    }

    // Get cache statistics
    pub fn get_statistics(&self) -> Result<CacheStatistics> {
        let hits = *self.hit_count.read()
            .map_err(|_| DbError::LockError("Failed to acquire read lock".to_string()))?;
        let misses = *self.miss_count.read()
            .map_err(|_| DbError::LockError("Failed to acquire read lock".to_string()))?;
        let cache = self.cache.read()
            .map_err(|_| DbError::LockError("Failed to acquire read lock".to_string()))?;

        let total_requests = hits + misses;
        let hit_rate = if total_requests > 0 {
            hits as f64 / total_requests as f64
        } else {
            0.0
        };

        Ok(CacheStatistics {
            hits,
            misses,
            hit_rate,
            total_entries: cache.len(),
            max_size: self.max_size,
        })
    }

    // Clear cache
    pub fn clear(&self) -> Result<()> {
        let mut cache = self.cache.write()
            .map_err(|_| DbError::LockError("Failed to acquire write lock".to_string()))?;
        cache.clear();

        let mut order = self.access_order.write()
            .map_err(|_| DbError::LockError("Failed to acquire write lock".to_string()))?;
        order.clear();

        Ok(())
    }
}

#[derive(Debug, Clone)]
struct CachedPlan {
    plan: QueryPlan,
    _cost: f64,
    _cached_at: SystemTime,
    _access_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryPlan {
    pub query_hash: String,
    pub plan_tree: String,
    pub estimated_cost: f64,
    pub estimated_rows: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheStatistics {
    pub hits: u64,
    pub misses: u64,
    pub hit_rate: f64,
    pub total_entries: usize,
    pub max_size: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_query_plan_cache() {
        let cache = QueryPlanCache::new(100);

        let plan = QueryPlan {
            query_hash: "hash1".to_string(),
            plan_tree: "SELECT * FROM users".to_string(),
            estimated_cost: 10.0,
            estimated_rows: 100,
        };

        assert!(cache.put("hash1".to_string(), plan.clone(), 10.0).is_ok());

        let retrieved = cache.get("hash1").unwrap();
        assert!(retrieved.is_some());

        let stats = cache.get_statistics().unwrap();
        assert_eq!(stats.hits, 1);
    }
}
