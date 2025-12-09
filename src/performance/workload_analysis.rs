// Workload Analysis Module
//
// Analyzes query patterns and provides insights into database workload

use crate::{Result, error::DbError};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, VecDeque};
use std::sync::{Arc, RwLock};
use std::time::SystemTime;

// Workload analyzer
pub struct WorkloadAnalyzer {
    query_log: Arc<RwLock<Vec<QueryExecution>>>,
    max_log_size: usize,
    analysis_cache: Arc<RwLock<Option<WorkloadAnalysis>>>,
}

impl WorkloadAnalyzer {
    pub fn new(max_log_size: usize) -> Self {
        Self {
            query_log: Arc::new(RwLock::new(Vec::new())),
            max_log_size,
            analysis_cache: Arc::new(RwLock::new(None)),
        }
    }

    // Log query execution
    pub fn log_execution(&self, execution: QueryExecution) -> Result<()> {
        let mut log = self.query_log.write()
            .map_err(|_| DbError::LockError("Failed to acquire write lock".to_string()))?;

        log.push(execution);

        // Trim if too large
        if log.len() > self.max_log_size {
            log.remove(0);
        }

        // Invalidate analysis cache
        let mut cache = self.analysis_cache.write()
            .map_err(|_| DbError::LockError("Failed to acquire write lock".to_string()))?;
        *cache = None;

        Ok(())
    }

    // Analyze workload
    pub fn analyze(&self) -> Result<WorkloadAnalysis> {
        // Check cache
        let cache = self.analysis_cache.read()
            .map_err(|_| DbError::LockError("Failed to acquire read lock".to_string()))?;

        if let Some(analysis) = cache.as_ref() {
            return Ok(analysis.clone());
        }
        drop(cache);

        let log = self.query_log.read()
            .map_err(|_| DbError::LockError("Failed to acquire read lock".to_string()))?;

        if log.is_empty() {
            return Ok(WorkloadAnalysis::default());
        }

        // Analyze query patterns
        let total_queries = log.len();
        let total_time_ms: u64 = log.iter().map(|q| q.execution_time_ms).sum();
        let avg_time_ms = total_time_ms as f64 / total_queries as f64;

        let slow_queries = log.iter()
            .filter(|q| q.execution_time_ms > 1000)
            .count();

        // Identify most frequent queries
        let mut query_counts: HashMap<String, usize> = HashMap::new();
        for execution in log.iter() {
            *query_counts.entry(execution.query_hash.clone()).or_insert(0) += 1;
        }

        let mut most_frequent: Vec<_> = query_counts.into_iter().collect();
        most_frequent.sort_by_key(|(_, count)| std::cmp::Reverse(*count));
        let top_queries: Vec<_> = most_frequent.into_iter().take(10).collect();

        let analysis = WorkloadAnalysis {
            total_queries,
            avg_execution_time_ms: avg_time_ms,
            slow_query_count: slow_queries,
            slow_query_percentage: (slow_queries as f64 / total_queries as f64) * 100.0,
            top_queries,
            analysis_time: SystemTime::now(),
        };

        // Update cache
        let mut cache = self.analysis_cache.write()
            .map_err(|_| DbError::LockError("Failed to acquire write lock".to_string()))?;
        *cache = Some(analysis.clone());

        Ok(analysis)
    }

    // Get query execution log
    pub fn get_log(&self) -> Result<Vec<QueryExecution>> {
        let log = self.query_log.read()
            .map_err(|_| DbError::LockError("Failed to acquire read lock".to_string()))?;
        Ok(log.clone())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryExecution {
    pub query_hash: String,
    pub execution_time_ms: u64,
    pub rows_returned: usize,
    pub timestamp: SystemTime,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkloadAnalysis {
    pub total_queries: usize,
    pub avg_execution_time_ms: f64,
    pub slow_query_count: usize,
    pub slow_query_percentage: f64,
    pub top_queries: Vec<(String, usize)>,
    pub analysis_time: SystemTime,
}

impl Default for WorkloadAnalysis {
    fn default() -> Self {
        Self {
            total_queries: 0,
            avg_execution_time_ms: 0.0,
            slow_query_count: 0,
            slow_query_percentage: 0.0,
            top_queries: Vec::new(),
            analysis_time: SystemTime::now(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_workload_analyzer() {
        let analyzer = WorkloadAnalyzer::new(1000);

        for i in 0..100 {
            let execution = QueryExecution {
                query_hash: format!("q{}", i % 10),
                execution_time_ms: 100 + i,
                rows_returned: 1000,
                timestamp: SystemTime::now(),
            };
            analyzer.log_execution(execution).unwrap();
        }

        let analysis = analyzer.analyze().unwrap();
        assert_eq!(analysis.total_queries, 100);
        assert!(!analysis.top_queries.is_empty());
    }
}
