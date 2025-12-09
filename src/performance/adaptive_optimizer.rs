/// Adaptive Query Optimizer Module
///
/// Provides adaptive query optimization using execution statistics

use crate::{Result, error::DbError};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

/// Adaptive query optimizer using statistics
pub struct AdaptiveQueryOptimizer {
    statistics: Arc<RwLock<HashMap<String, QueryStatistics>>>,
    learning_rate: f64,
    min_samples: usize,
}

impl AdaptiveQueryOptimizer {
    pub fn new(learning_rate: f64, min_samples: usize) -> Self {
        Self {
            statistics: Arc::new(RwLock::new(HashMap::new())),
            learning_rate,
            min_samples,
        }
    }

    /// Record query execution
    pub fn record_execution(
        &self,
        query_hash: &str,
        actual_cost: f64,
        actual_rows: usize,
        execution_time_ms: u64,
    ) -> Result<()> {
        let mut stats = self.statistics.write()
            .map_err(|_| DbError::LockError("Failed to acquire write lock".to_string()))?;

        let entry = stats.entry(query_hash.to_string())
            .or_insert_with(|| QueryStatistics {
                query_hash: query_hash.to_string(),
                execution_count: 0,
                total_cost: 0.0,
                total_rows: 0,
                total_time_ms: 0,
                avg_cost: 0.0,
                avg_rows: 0.0,
                avg_time_ms: 0.0,
            });

        entry.execution_count += 1;
        entry.total_cost += actual_cost;
        entry.total_rows += actual_rows;
        entry.total_time_ms += execution_time_ms;

        entry.avg_cost = entry.total_cost / entry.execution_count as f64;
        entry.avg_rows = entry.total_rows as f64 / entry.execution_count as f64;
        entry.avg_time_ms = entry.total_time_ms as f64 / entry.execution_count as f64;

        Ok(())
    }

    /// Get optimization suggestions
    pub fn get_suggestions(&self, query_hash: &str) -> Result<OptimizationSuggestions> {
        let stats = self.statistics.read()
            .map_err(|_| DbError::LockError("Failed to acquire read lock".to_string()))?;

        if let Some(stat) = stats.get(query_hash) {
            if stat.execution_count < self.min_samples {
                return Ok(OptimizationSuggestions {
                    query_hash: query_hash.to_string(),
                    suggestions: vec!["Insufficient data for optimization".to_string()],
                    confidence: 0.0,
                });
            }

            let mut suggestions = Vec::new();
            let mut confidence = 1.0;

            // Analyze patterns
            if stat.avg_time_ms > 1000.0 {
                suggestions.push("Query is slow, consider adding indexes".to_string());
                confidence *= 0.9;
            }

            if stat.avg_rows > 10000.0 {
                suggestions.push("Large result set, consider adding LIMIT clause".to_string());
                confidence *= 0.85;
            }

            if stat.avg_cost > 1000.0 {
                suggestions.push("High cost query, review query plan".to_string());
                confidence *= 0.9;
            }

            Ok(OptimizationSuggestions {
                query_hash: query_hash.to_string(),
                suggestions,
                confidence,
            })
        } else {
            Ok(OptimizationSuggestions {
                query_hash: query_hash.to_string(),
                suggestions: vec!["No statistics available".to_string()],
                confidence: 0.0,
            })
        }
    }

    /// Get all query statistics
    pub fn get_all_statistics(&self) -> Result<Vec<QueryStatistics>> {
        let stats = self.statistics.read()
            .map_err(|_| DbError::LockError("Failed to acquire read lock".to_string()))?;
        Ok(stats.values().cloned().collect())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryStatistics {
    pub query_hash: String,
    pub execution_count: usize,
    pub total_cost: f64,
    pub total_rows: usize,
    pub total_time_ms: u64,
    pub avg_cost: f64,
    pub avg_rows: f64,
    pub avg_time_ms: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OptimizationSuggestions {
    pub query_hash: String,
    pub suggestions: Vec<String>,
    pub confidence: f64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_adaptive_optimizer() {
        let optimizer = AdaptiveQueryOptimizer::new(0.1, 5);

        for _ in 0..10 {
            optimizer.record_execution("q1", 100.0, 1000, 1500).unwrap();
        }

        let suggestions = optimizer.get_suggestions("q1").unwrap();
        assert!(!suggestions.suggestions.is_empty());
    }
}
