use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use parking_lot::RwLock;
use std::sync::Arc;
use std::time::{Duration, SystemTime};

/// Query statistics and performance metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryStats {
    pub query_id: u64,
    pub sql: String,
    pub execution_time_ms: u64,
    pub rows_affected: usize,
    pub bytes_read: usize,
    pub bytes_written: usize,
    pub cache_hits: usize,
    pub cache_misses: usize,
    pub timestamp: SystemTime,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemMetrics {
    pub active_connections: usize,
    pub total_queries: u64,
    pub queries_per_second: f64,
    pub buffer_pool_hit_rate: f64,
    pub active_transactions: usize,
    pub locks_held: usize,
    pub disk_reads: u64,
    pub disk_writes: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SlowQuery {
    pub query: String,
    pub execution_time_ms: u64,
    pub timestamp: SystemTime,
}

/// Monitoring and diagnostics system
pub struct MonitoringSystem {
    query_stats: Arc<RwLock<Vec<QueryStats>>>,
    slow_queries: Arc<RwLock<Vec<SlowQuery>>>,
    slow_query_threshold_ms: u64,
    metrics: Arc<RwLock<SystemMetrics>>,
}

impl MonitoringSystem {
    pub fn new() -> Self {
        Self {
            query_stats: Arc::new(RwLock::new(Vec::new())),
            slow_queries: Arc::new(RwLock::new(Vec::new())),
            slow_query_threshold_ms: 1000, // 1 second
            metrics: Arc::new(RwLock::new(SystemMetrics {
                active_connections: 0,
                total_queries: 0,
                queries_per_second: 0.0,
                buffer_pool_hit_rate: 0.0,
                active_transactions: 0,
                locks_held: 0,
                disk_reads: 0,
                disk_writes: 0,
            })),
        }
    }
    
    pub fn record_query(&self, stats: QueryStats) {
        if stats.execution_time_ms >= self.slow_query_threshold_ms {
            self.slow_queries.write().push(SlowQuery {
                query: stats.sql.clone(),
                execution_time_ms: stats.execution_time_ms,
                timestamp: stats.timestamp,
            });
        }
        
        self.query_stats.write().push(stats);
        self.metrics.write().total_queries += 1;
    }
    
    pub fn get_slow_queries(&self) -> Vec<SlowQuery> {
        self.slow_queries.read().clone()
    }
    
    pub fn get_metrics(&self) -> SystemMetrics {
        self.metrics.read().clone()
    }
    
    pub fn update_metrics<F>(&self, updater: F) 
    where F: FnOnce(&mut SystemMetrics) {
        let mut metrics = self.metrics.write();
        updater(&mut *metrics);
    }
    
    pub fn get_query_stats(&self, limit: usize) -> Vec<QueryStats> {
        let stats = self.query_stats.read();
        stats.iter().rev().take(limit).cloned().collect()
    }
}

impl Default for MonitoringSystem {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_query_recording() {
        let monitor = MonitoringSystem::new();
        let stats = QueryStats {
            query_id: 1,
            sql: "SELECT * FROM users".to_string(),
            execution_time_ms: 50,
            rows_affected: 10,
            bytes_read: 1024,
            bytes_written: 0,
            cache_hits: 5,
            cache_misses: 2,
            timestamp: SystemTime::now(),
        };
        
        monitor.record_query(stats);
        assert_eq!(monitor.get_metrics().total_queries, 1);
    }
}
