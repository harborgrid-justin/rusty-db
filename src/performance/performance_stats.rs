// Performance Statistics Collector Module
//
// Collects and maintains performance statistics for query execution

use crate::{error::DbError, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use std::time::SystemTime;

// Statistics collector for query performance
#[allow(dead_code)]
pub struct PerformanceStatsCollector {
    query_stats: Arc<RwLock<HashMap<String, QueryPerformanceStats>>>,
    global_stats: Arc<RwLock<GlobalPerformanceStats>>,
}

#[allow(dead_code)]
impl PerformanceStatsCollector {
    pub fn new() -> Self {
        Self {
            query_stats: Arc::new(RwLock::new(HashMap::new())),
            global_stats: Arc::new(RwLock::new(GlobalPerformanceStats::default())),
        }
    }

    // Record query execution
    pub fn record_query(
        &self,
        query_hash: &str,
        execution_time_ms: u64,
        rows: usize,
    ) -> Result<()> {
        // Update query-specific stats
        let mut query_stats = self
            .query_stats
            .write()
            .map_err(|_| DbError::LockError("Failed to acquire write lock".to_string()))?;

        let stats = query_stats
            .entry(query_hash.to_string())
            .or_insert_with(|| QueryPerformanceStats {
                query_hash: query_hash.to_string(),
                execution_count: 0,
                total_time_ms: 0,
                total_rows: 0,
                min_time_ms: u64::MAX,
                max_time_ms: 0,
                last_execution: SystemTime::now(),
            });

        stats.execution_count += 1;
        stats.total_time_ms += execution_time_ms;
        stats.total_rows += rows;
        stats.min_time_ms = stats.min_time_ms.min(execution_time_ms);
        stats.max_time_ms = stats.max_time_ms.max(execution_time_ms);
        stats.last_execution = SystemTime::now();

        // Update global stats
        let mut global_stats = self
            .global_stats
            .write()
            .map_err(|_| DbError::LockError("Failed to acquire write lock".to_string()))?;
        global_stats.total_queries += 1;
        global_stats.total_execution_time_ms += execution_time_ms;
        global_stats.total_rows_processed += rows;

        Ok(())
    }

    // Get stats for a specific query
    pub fn get_query_stats(&self, query_hash: &str) -> Result<Option<QueryPerformanceStats>> {
        let query_stats = self
            .query_stats
            .read()
            .map_err(|_| DbError::LockError("Failed to acquire read lock".to_string()))?;
        Ok(query_stats.get(query_hash).cloned())
    }

    // Get global performance statistics
    pub fn get_global_stats(&self) -> Result<GlobalPerformanceStats> {
        let global_stats = self
            .global_stats
            .read()
            .map_err(|_| DbError::LockError("Failed to acquire read lock".to_string()))?;
        Ok(global_stats.clone())
    }

    // Get top N slowest queries
    pub fn get_slowest_queries(&self, n: usize) -> Result<Vec<QueryPerformanceStats>> {
        let query_stats = self
            .query_stats
            .read()
            .map_err(|_| DbError::LockError("Failed to acquire read lock".to_string()))?;

        let mut stats: Vec<_> = query_stats.values().cloned().collect();
        stats.sort_by(|a, b| {
            let a_avg = a.total_time_ms / a.execution_count.max(1) as u64;
            let b_avg = b.total_time_ms / b.execution_count.max(1) as u64;
            b_avg.cmp(&a_avg)
        });

        Ok(stats.into_iter().take(n).collect())
    }
}

impl Default for PerformanceStatsCollector {
    fn default() -> Self {
        Self::new()
    }
}

#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryPerformanceStats {
    pub query_hash: String,
    pub execution_count: usize,
    pub total_time_ms: u64,
    pub total_rows: usize,
    pub min_time_ms: u64,
    pub max_time_ms: u64,
    pub last_execution: SystemTime,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GlobalPerformanceStats {
    pub total_queries: u64,
    pub total_execution_time_ms: u64,
    pub total_rows_processed: usize,
}

impl Default for GlobalPerformanceStats {
    fn default() -> Self {
        Self {
            total_queries: 0,
            total_execution_time_ms: 0,
            total_rows_processed: 0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_performance_stats_collector() {
        let collector = PerformanceStatsCollector::new();

        for i in 0..10 {
            collector.record_query("q1", 100 + i * 10, 1000).unwrap();
        }

        let stats = collector.get_query_stats("q1").unwrap();
        assert!(stats.is_some());
        assert_eq!(stats.unwrap().execution_count, 10);

        let global_stats = collector.get_global_stats().unwrap();
        assert_eq!(global_stats.total_queries, 10);
    }
}
