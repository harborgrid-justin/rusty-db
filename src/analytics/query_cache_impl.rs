// Query Result Cache Implementation
//
// Provides intelligent caching of query results with LRU eviction

use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use std::time::SystemTime;

// Query result cache with LRU eviction policy
#[allow(dead_code)]
pub struct QueryCache {
    cache: Arc<RwLock<HashMap<String, CachedResult>>>,
    lru_queue: Arc<RwLock<VecDeque<String>>>,
    max_size: usize,
    max_memory_bytes: usize,
    current_memory_bytes: Arc<RwLock<usize>>,
    hit_count: Arc<RwLock<u64>>,
    miss_count: Arc<RwLock<u64>>,
}

#[allow(dead_code)]
#[derive(Debug, Clone)]
struct CachedResult {
    query: String,
    result: Vec<Vec<String>>,
    timestamp: SystemTime,
    ttl_seconds: u64,
    size_bytes: usize,
    access_count: u64,
    last_access: SystemTime,
}

#[allow(dead_code)]
impl QueryCache {
    pub fn new(max_size: usize) -> Self {
        Self::with_memory_limit(max_size, 100 * 1024 * 1024) // 100MB default
    }

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

    pub fn get(&self, query: &str) -> Option<Vec<Vec<String>>> {
        let mut cache = self.cache.write();

        if let Some(cached) = cache.get_mut(query) {
            let elapsed = SystemTime::now()
                .duration_since(cached.timestamp)
                .unwrap()
                .as_secs();

            if elapsed < cached.ttl_seconds {
                // Update LRU
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
                // Expired entry
                let size = cached.size_bytes;
                cache.remove(query);
                *self.current_memory_bytes.write() -= size;
            }
        }

        *self.miss_count.write() += 1;
        None
    }

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

        cache.insert(query.clone(), CachedResult {
            query: query.clone(),
            result,
            timestamp: SystemTime::now(),
            ttl_seconds,
            size_bytes,
            access_count: 0,
            last_access: SystemTime::now(),
        });

        lru.push_back(query);
        *self.current_memory_bytes.write() += size_bytes;
    }

    fn evict_if_needed(&self, incoming_size: usize) {
        let mut current_memory = *self.current_memory_bytes.read();

        while (current_memory + incoming_size > self.max_memory_bytes ||
               self.cache.read().len() >= self.max_size) &&
              !self.lru_queue.read().is_empty() {

            let query_to_evict = {
                let mut lru = self.lru_queue.write();
                lru.pop_front()
            };

            if let Some(query) = query_to_evict {
                let mut cache = self.cache.write();
                if let Some(removed) = cache.remove(&query) {
                    *self.current_memory_bytes.write() -= removed.size_bytes;
                    current_memory -= removed.size_bytes;
                }
            }
        }
    }

    fn estimate_size(result: &Vec<Vec<String>>) -> usize {
        let mut size = 0;
        for row in result {
            for val in row {
                size += val.len() + size_of::<String>();
            }
            size += size_of::<Vec<String>>();
        }
        size += size_of::<Vec<Vec<String>>>();
        size
    }

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

    pub fn invalidate_pattern(&self, pattern: &str) {
        let keys_to_remove: Vec<String> = self.cache.read()
            .keys()
            .filter(|k| k.contains(pattern))
            .cloned()
            .collect();

        for key in keys_to_remove {
            self.invalidate(&key);
        }
    }

    pub fn clear(&self) {
        self.cache.write().clear();
        self.lru_queue.write().clear();
        *self.current_memory_bytes.write() = 0;
        *self.hit_count.write() = 0;
        *self.miss_count.write() = 0;
    }

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
            hit_rate: if total > 0 { hits as f64 / total as f64 } else { 0.0 },
        }
    }
}

// Cache statistics
#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheStats {
    pub size: usize,
    pub max_size: usize,
    pub memory_bytes: usize,
    pub max_memory_bytes: usize,
    pub hit_count: u64,
    pub miss_count: u64,
    pub hit_rate: f64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_query_cache() {
        let cache = QueryCache::new(10);

        let result = vec![
            vec!["1".to_string(), "Alice".to_string()],
            vec!["2".to_string(), "Bob".to_string()],
        ];

        cache.put("SELECT * FROM users".to_string(), result.clone(), 60);

        let cached = cache.get("SELECT * FROM users");
        assert!(cached.is_some());
        assert_eq!(cached.unwrap(), result);

        let stats = cache.get_stats();
        assert_eq!(stats.hit_count, 1);
    }
}
