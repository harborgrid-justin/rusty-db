use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use parking_lot::RwLock;
use std::sync::Arc;
use crate::Result;
use crate::catalog::Schema;

/// Materialized view definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MaterializedView {
    pub name: String,
    pub query: String,
    pub schema: Schema,
    pub last_refreshed: std::time::SystemTime,
}

/// View definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct View {
    pub name: String,
    pub query: String,
    pub schema: Schema,
}

/// Window function specification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum WindowFunction {
    RowNumber,
    Rank,
    DenseRank,
    Lead,
    Lag,
    FirstValue,
    LastValue,
}

/// Aggregate function
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AggregateFunction {
    Count,
    Sum,
    Avg,
    Min,
    Max,
    StdDev,
    Variance,
    Median,
}

/// Query result cache
pub struct QueryCache {
    cache: Arc<RwLock<HashMap<String, CachedResult>>>,
    max_size: usize,
}

#[derive(Debug, Clone)]
struct CachedResult {
    query: String,
    result: Vec<Vec<String>>,
    timestamp: std::time::SystemTime,
    ttl_seconds: u64,
}

impl QueryCache {
    pub fn new(max_size: usize) -> Self {
        Self {
            cache: Arc::new(RwLock::new(HashMap::new())),
            max_size,
        }
    }
    
    pub fn get(&self, query: &str) -> Option<Vec<Vec<String>>> {
        let cache = self.cache.read();
        
        if let Some(cached) = cache.get(query) {
            let elapsed = std::time::SystemTime::now()
                .duration_since(cached.timestamp)
                .unwrap()
                .as_secs();
            
            if elapsed < cached.ttl_seconds {
                return Some(cached.result.clone());
            }
        }
        
        None
    }
    
    pub fn put(&self, query: String, result: Vec<Vec<String>>, ttl_seconds: u64) {
        let mut cache = self.cache.write();
        
        if cache.len() >= self.max_size {
            // Remove oldest entry
            if let Some(key) = cache.keys().next().cloned() {
                cache.remove(&key);
            }
        }
        
        cache.insert(query.clone(), CachedResult {
            query,
            result,
            timestamp: std::time::SystemTime::now(),
            ttl_seconds,
        });
    }
    
    pub fn invalidate(&self, query: &str) {
        self.cache.write().remove(query);
    }
    
    pub fn clear(&self) {
        self.cache.write().clear();
    }
}

/// Analytics manager
pub struct AnalyticsManager {
    materialized_views: Arc<RwLock<HashMap<String, MaterializedView>>>,
    views: Arc<RwLock<HashMap<String, View>>>,
    query_cache: QueryCache,
}

impl AnalyticsManager {
    pub fn new() -> Self {
        Self {
            materialized_views: Arc::new(RwLock::new(HashMap::new())),
            views: Arc::new(RwLock::new(HashMap::new())),
            query_cache: QueryCache::new(1000),
        }
    }
    
    pub fn create_view(&self, view: View) -> Result<()> {
        self.views.write().insert(view.name.clone(), view);
        Ok(())
    }
    
    pub fn create_materialized_view(&self, mv: MaterializedView) -> Result<()> {
        self.materialized_views.write().insert(mv.name.clone(), mv);
        Ok(())
    }
    
    pub fn refresh_materialized_view(&self, name: &str) -> Result<()> {
        let mut mvs = self.materialized_views.write();
        
        if let Some(mv) = mvs.get_mut(name) {
            mv.last_refreshed = std::time::SystemTime::now();
            // In production, re-execute query and update data
            Ok(())
        } else {
            Err(crate::error::DbError::Execution(format!("Materialized view {} not found", name)))
        }
    }
    
    pub fn get_cached_query(&self, query: &str) -> Option<Vec<Vec<String>>> {
        self.query_cache.get(query)
    }
    
    pub fn cache_query_result(&self, query: String, result: Vec<Vec<String>>) {
        self.query_cache.put(query, result, 300); // 5 minute TTL
    }
}

impl Default for AnalyticsManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_query_cache() {
        let cache = QueryCache::new(100);
        let query = "SELECT * FROM users";
        let result = vec![vec!["1".to_string(), "Alice".to_string()]];
        
        cache.put(query.to_string(), result.clone(), 60);
        assert!(cache.get(query).is_some());
    }
}
