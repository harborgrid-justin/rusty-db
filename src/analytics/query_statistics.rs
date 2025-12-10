// Query Statistics and Workload Analysis
//
// This module provides comprehensive query statistics tracking, workload
// analysis, and performance monitoring for analytical query processing.
//
// # Architecture
//
// The statistics system operates at multiple levels:
// - Individual query execution tracking
// - Aggregate workload pattern analysis
// - Performance trend detection
// - Index recommendation based on access patterns
//
// # Example
//
// ```rust,ignore
// use crate::analytics::query_statistics::{QueryStatisticsTracker, WorkloadAnalyzer};
//
// let mut tracker = QueryStatisticsTracker::new();
// tracker.record_query("SELECT * FROM users WHERE id = 1", 50);
//
// let analyzer = WorkloadAnalyzer::new();
// let recommendations = analyzer.analyze(&tracker);
// ```

use std::collections::VecDeque;
use parking_lot::RwLock;
use std::collections::HashMap;
use std::sync::Arc;

// A single query execution record.
#[derive(Debug, Clone)]
pub struct QueryExecution {
    // Query identifier (hash of normalized query)
    pub query_id: u64,
    // Original SQL text
    pub sql: String,
    // Normalized SQL (parameters replaced)
    pub normalized_sql: String,
    // Execution time in milliseconds
    pub execution_time_ms: u64,
    // Number of rows examined
    pub rows_examined: u64,
    // Number of rows returned
    pub rows_returned: u64,
    // Bytes read
    pub bytes_read: u64,
    // Whether query used an index
    pub used_index: bool,
    // Index names used
    pub indexes_used: Vec<String>,
    // Tables accessed
    pub tables_accessed: Vec<String>,
    // Execution timestamp
    pub executed_at: std::time::Instant,
    // User/connection identifier
    pub user_id: Option<String>,
}

impl QueryExecution {
    // Creates a new query execution record.
    pub fn new(sql: impl Into<String>, execution_time_ms: u64) -> Self {
        let sql_str = sql.into();
        let normalized = Self::normalize_sql(&sql_str);
        let query_id = Self::hash_query(&normalized);

        Self {
            query_id,
            sql: sql_str,
            normalized_sql: normalized,
            execution_time_ms,
            rows_examined: 0,
            rows_returned: 0,
            bytes_read: 0,
            used_index: false,
            indexes_used: Vec::new(),
            tables_accessed: Vec::new(),
            executed_at: std::time::Instant::now(),
            user_id: None,
        }
    }

    // Normalizes SQL by replacing literal values with placeholders.
    fn normalize_sql(sql: &str) -> String {
        // Simple normalization: replace numbers and quoted strings
        let mut result = sql.to_string();

        // Replace quoted strings
        while let Some(start) = result.find('\'') {
            if let Some(end) = result[start + 1..].find('\'') {
                result = format!("{}?{}", &result[..start], &result[start + end + 2..]);
            } else {
                break;
            }
        }

        // Replace numbers (simple approach)
        let words: Vec<&str> = result.split_whitespace().collect();
        let normalized: Vec<String> = words
            .iter()
            .map(|w| {
                if w.parse::<f64>().is_ok() {
                    "?".to_string()
                } else {
                    w.to_string()
                }
            })
            .collect();

        normalized.join(" ")
    }

    // Generates a hash for the normalized query.
    fn hash_query(sql: &str) -> u64 {
        let mut hash: u64 = 0;
        for byte in sql.bytes() {
            hash = hash.wrapping_mul(31).wrapping_add(byte as u64);
        }
        hash
    }

    // Sets the row statistics.
    pub fn with_rows(mut self, examined: u64, returned: u64) -> Self {
        self.rows_examined = examined;
        self.rows_returned = returned;
        self
    }

    // Sets the index usage.
    pub fn with_index(mut self, index_name: impl Into<String>) -> Self {
        self.used_index = true;
        self.indexes_used.push(index_name.into());
        self
    }

    // Sets the tables accessed.
    pub fn with_tables(mut self, tables: Vec<String>) -> Self {
        self.tables_accessed = tables;
        self
    }
}

// Aggregated statistics for a query pattern.
#[derive(Debug, Clone)]
pub struct QueryStats {
    // Query identifier
    pub query_id: u64,
    // Normalized SQL
    pub normalized_sql: String,
    // Total execution count
    pub execution_count: u64,
    // Total execution time
    pub total_time_ms: u64,
    // Minimum execution time
    pub min_time_ms: u64,
    // Maximum execution time
    pub max_time_ms: u64,
    // Average execution time
    pub avg_time_ms: f64,
    // Standard deviation of execution time
    pub stddev_time_ms: f64,
    // Total rows examined
    pub total_rows_examined: u64,
    // Total rows returned
    pub total_rows_returned: u64,
    // Percentage of executions using index
    pub index_usage_percent: f64,
    // First seen timestamp
    pub first_seen: std::time::Instant,
    // Last seen timestamp
    pub last_seen: std::time::Instant,
    // Tables accessed
    pub tables: Vec<String>,
    // Execution times for percentile calculation
    execution_times: Vec<u64>,
}

impl QueryStats {
    // Creates new statistics from an execution.
    fn new(execution: &QueryExecution) -> Self {
        Self {
            query_id: execution.query_id,
            normalized_sql: execution.normalized_sql.clone(),
            execution_count: 1,
            total_time_ms: execution.execution_time_ms,
            min_time_ms: execution.execution_time_ms,
            max_time_ms: execution.execution_time_ms,
            avg_time_ms: execution.execution_time_ms as f64,
            stddev_time_ms: 0.0,
            total_rows_examined: execution.rows_examined,
            total_rows_returned: execution.rows_returned,
            index_usage_percent: if execution.used_index { 100.0 } else { 0.0 },
            first_seen: execution.executed_at,
            last_seen: execution.executed_at,
            tables: execution.tables_accessed.clone(),
            execution_times: vec![execution.execution_time_ms],
        }
    }

    // Updates statistics with a new execution.
    fn update(&mut self, execution: &QueryExecution) {
        self.execution_count += 1;
        self.total_time_ms += execution.execution_time_ms;
        self.min_time_ms = self.min_time_ms.min(execution.execution_time_ms);
        self.max_time_ms = self.max_time_ms.max(execution.execution_time_ms);
        self.avg_time_ms = self.total_time_ms as f64 / self.execution_count as f64;
        self.total_rows_examined += execution.rows_examined;
        self.total_rows_returned += execution.rows_returned;
        self.last_seen = execution.executed_at;

        // Update index usage percentage
        let index_count = if execution.used_index { 1.0 } else { 0.0 };
        self.index_usage_percent = ((self.index_usage_percent * (self.execution_count - 1) as f64)
            + index_count * 100.0)
            / self.execution_count as f64;

        // Keep execution times for percentile (limit to last 1000)
        self.execution_times.push(execution.execution_time_ms);
        if self.execution_times.len() > 1000 {
            self.execution_times.remove(0);
        }

        // Update stddev
        self.update_stddev();
    }

    // Updates standard deviation calculation.
    fn update_stddev(&mut self) {
        if self.execution_times.len() < 2 {
            self.stddev_time_ms = 0.0;
            return;
        }

        let mean = self.avg_time_ms;
        let variance: f64 = self
            .execution_times
            .iter()
            .map(|&x| (x as f64 - mean).powi(2))
            .sum::<f64>()
            / (self.execution_times.len() - 1) as f64;

        self.stddev_time_ms = variance.sqrt();
    }

    // Returns the p95 execution time.
    pub fn p95_time_ms(&self) -> u64 {
        self.percentile(95)
    }

    // Returns the p99 execution time.
    pub fn p99_time_ms(&self) -> u64 {
        self.percentile(99)
    }

    // Calculates a percentile of execution times.
    fn percentile(&self, p: usize) -> u64 {
        if self.execution_times.is_empty() {
            return 0;
        }

        let mut sorted = self.execution_times.clone();
        sorted.sort_unstable();

        let idx = ((p as f64 / 100.0) * sorted.len() as f64) as usize;
        sorted[idx.min(sorted.len() - 1)]
    }

    // Returns selectivity (rows returned / rows examined).
    pub fn selectivity(&self) -> f64 {
        if self.total_rows_examined == 0 {
            return 1.0;
        }
        self.total_rows_returned as f64 / self.total_rows_examined as f64
    }
}

// Tracks query execution statistics.
#[derive(Debug)]
pub struct QueryStatisticsTracker {
    // Statistics by query ID
    stats: Arc<RwLock<HashMap<u64, QueryStats>>>,
    // Recent executions for time-based analysis
    recent_executions: Arc<RwLock<VecDeque<QueryExecution>>>,
    // Maximum recent executions to keep
    max_recent: usize,
    // Total queries tracked
    total_queries: Arc<RwLock<u64>>,
}

impl Default for QueryStatisticsTracker {
    fn default() -> Self {
        Self::new()
    }
}

impl QueryStatisticsTracker {
    // Creates a new query statistics tracker.
    pub fn new() -> Self {
        Self {
            stats: Arc::new(RwLock::new(HashMap::new())),
            recent_executions: Arc::new(RwLock::new(VecDeque::new())),
            max_recent: 10000,
            total_queries: Arc::new(RwLock::new(0)),
        }
    }

    // Records a query execution.
    pub fn record_execution(&self, execution: QueryExecution) {
        let query_id = execution.query_id;

        // Update aggregated stats
        let mut stats = self.stats.write();
        stats
            .entry(query_id)
            .and_modify(|s| s.update(&execution))
            .or_insert_with(|| QueryStats::new(&execution));

        // Add to recent executions
        let mut recent = self.recent_executions.write();
        recent.push_back(execution);
        while recent.len() > self.max_recent {
            recent.pop_front();
        }

        *self.total_queries.write() += 1;
    }

    // Records a simple query with just SQL and time.
    pub fn record_query(&self, sql: &str, execution_time_ms: u64) {
        self.record_execution(QueryExecution::new(sql, execution_time_ms));
    }

    // Returns statistics for a specific query.
    pub fn get_stats(&self, query_id: u64) -> Option<QueryStats> {
        self.stats.read().get(&query_id).cloned()
    }

    // Returns all query statistics.
    pub fn get_all_stats(&self) -> Vec<QueryStats> {
        self.stats.read().values().cloned().collect()
    }

    // Returns the top N slowest queries by average time.
    pub fn top_slow_queries(&self, n: usize) -> Vec<QueryStats> {
        let mut stats: Vec<QueryStats> = self.get_all_stats();
        stats.sort_by(|a, b| b.avg_time_ms.partial_cmp(&a.avg_time_ms).unwrap());
        stats.into_iter().take(n).collect()
    }

    // Returns the top N most frequent queries.
    pub fn top_frequent_queries(&self, n: usize) -> Vec<QueryStats> {
        let mut stats: Vec<QueryStats> = self.get_all_stats();
        stats.sort_by(|a, b| b.execution_count.cmp(&a.execution_count));
        stats.into_iter().take(n).collect()
    }

    // Returns queries with low index usage.
    pub fn queries_needing_indexes(&self, threshold: f64) -> Vec<QueryStats> {
        self.get_all_stats()
            .into_iter()
            .filter(|s| s.index_usage_percent < threshold)
            .collect()
    }

    // Returns the total number of unique queries tracked.
    pub fn unique_query_count(&self) -> usize {
        self.stats.read().len()
    }

    // Returns the total execution count.
    pub fn total_execution_count(&self) -> u64 {
        *self.total_queries.read()
    }

    // Clears all statistics.
    pub fn clear(&self) {
        self.stats.write().clear();
        self.recent_executions.write().clear();
        *self.total_queries.write() = 0;
    }
}

// Query workload pattern.
#[derive(Debug, Clone)]
pub struct WorkloadQuery {
    // Query pattern
    pub pattern: String,
    // Columns used in WHERE clauses
    pub filter_columns: Vec<String>,
    // Columns used in ORDER BY
    pub order_columns: Vec<String>,
    // Columns used in GROUP BY
    pub group_columns: Vec<String>,
    // Columns used in JOIN conditions
    pub join_columns: Vec<String>,
    // Frequency weight
    pub frequency: f64,
    // Average execution time
    pub avg_time_ms: f64,
}

// Index recommendation from workload analysis.
#[derive(Debug, Clone)]
pub struct IndexRecommendation {
    // Table name
    pub table: String,
    // Columns for the index
    pub columns: Vec<String>,
    // Recommended index type
    pub index_type: String,
    // Estimated improvement
    pub estimated_improvement: f64,
    // Reasoning for the recommendation
    pub reason: String,
    // Priority (1-10, higher = more important)
    pub priority: u8,
}

// Workload analyzer for query pattern analysis.
#[derive(Debug)]
pub struct WorkloadAnalyzer {
    // Minimum frequency for pattern consideration
    min_frequency: u64,
    // Time window for analysis (seconds)
    _time_window_secs: u64,
}

impl Default for WorkloadAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

impl WorkloadAnalyzer {
    // Creates a new workload analyzer.
    pub fn new() -> Self {
        Self {
            min_frequency: 10,
            _time_window_secs: 3600,
        }
    }

    // Analyzes the workload and generates recommendations.
    pub fn analyze(&self, tracker: &QueryStatisticsTracker) -> WorkloadAnalysisResult {
        let stats = tracker.get_all_stats();

        let mut table_access: HashMap<String, u64> = HashMap::new();
        let mut slow_queries: Vec<QueryStats> = Vec::new();
        let mut recommendations: Vec<IndexRecommendation> = Vec::new();

        // Analyze query patterns
        for stat in &stats {
            // Track table access frequency
            for table in &stat.tables {
                *table_access.entry(table.clone()).or_insert(0) += stat.execution_count;
            }

            // Identify slow queries
            if stat.avg_time_ms > 1000.0 && stat.execution_count >= self.min_frequency {
                slow_queries.push(stat.clone());

                // Generate index recommendations for slow queries without index
                if stat.index_usage_percent < 50.0 && !stat.tables.is_empty() {
                    recommendations.push(IndexRecommendation {
                        table: stat.tables[0].clone(),
                        columns: self.extract_filter_columns(&stat.normalized_sql),
                        index_type: "btree".to_string(),
                        estimated_improvement: (stat.avg_time_ms * 0.8).min(stat.avg_time_ms - 10.0),
                        reason: format!(
                            "Query executed {} times with avg {}ms, no index used",
                            stat.execution_count, stat.avg_time_ms
                        ),
                        priority: self.calculate_priority(stat),
                    });
                }
            }
        }

        // Sort recommendations by priority
        recommendations.sort_by(|a, b| b.priority.cmp(&a.priority));

        WorkloadAnalysisResult {
            total_queries: tracker.total_execution_count(),
            unique_patterns: stats.len(),
            slow_queries,
            recommendations,
            table_access_frequency: table_access,
            avg_query_time_ms: self.calculate_avg_time(&stats),
        }
    }

    // Extracts filter columns from a SQL query (simplified).
    fn extract_filter_columns(&self, sql: &str) -> Vec<String> {
        let mut columns = Vec::new();
        let upper = sql.to_uppercase();

        if let Some(where_pos) = upper.find("WHERE") {
            let after_where = &sql[where_pos + 5..];
            // Simple extraction: look for column = value patterns
            for part in after_where.split(|c| c == '=' || c == '>' || c == '<') {
                let trimmed = part.trim();
                if !trimmed.is_empty()
                    && !trimmed.starts_with('?')
                    && !trimmed.chars().next().map(|c| c.is_ascii_digit()).unwrap_or(false)
                {
                    if let Some(col) = trimmed.split_whitespace().last() {
                        if !col.eq_ignore_ascii_case("AND")
                            && !col.eq_ignore_ascii_case("OR")
                            && col.len() > 1
                        {
                            columns.push(col.to_string());
                        }
                    }
                }
            }
        }

        columns.into_iter().take(3).collect()
    }

    // Calculates recommendation priority.
    fn calculate_priority(&self, stats: &QueryStats) -> u8 {
        let freq_score = (stats.execution_count as f64).log10() * 2.0;
        let time_score = (stats.avg_time_ms / 100.0).min(5.0);
        let score = freq_score + time_score;
        score.min(10.0).max(1.0) as u8
    }

    // Calculates average query time.
    fn calculate_avg_time(&self, stats: &[QueryStats]) -> f64 {
        if stats.is_empty() {
            return 0.0;
        }

        let total_time: f64 = stats.iter().map(|s| s.avg_time_ms * s.execution_count as f64).sum();
        let total_count: u64 = stats.iter().map(|s| s.execution_count).sum();

        if total_count == 0 {
            0.0
        } else {
            total_time / total_count as f64
        }
    }
}

// Result of workload analysis.
#[derive(Debug)]
pub struct WorkloadAnalysisResult {
    // Total queries analyzed
    pub total_queries: u64,
    // Number of unique query patterns
    pub unique_patterns: usize,
    // Slow queries identified
    pub slow_queries: Vec<QueryStats>,
    // Index recommendations
    pub recommendations: Vec<IndexRecommendation>,
    // Table access frequency
    pub table_access_frequency: HashMap<String, u64>,
    // Average query time across all queries
    pub avg_query_time_ms: f64,
}

impl WorkloadAnalysisResult {
    // Returns the hottest tables by access frequency.
    pub fn hot_tables(&self, n: usize) -> Vec<(String, u64)> {
        let mut tables: Vec<(String, u64)> = self.table_access_frequency.clone().into_iter().collect();
        tables.sort_by(|a, b| b.1.cmp(&a.1));
        tables.into_iter().take(n).collect()
    }
}

// Performance report for a time period.
#[derive(Debug, Clone)]
pub struct PerformanceReport {
    // Report period start
    pub period_start: std::time::Instant,
    // Report period end
    pub period_end: std::time::Instant,
    // Total queries executed
    pub total_queries: u64,
    // Queries per second
    pub qps: f64,
    // Average latency
    pub avg_latency_ms: f64,
    // P50 latency
    pub p50_latency_ms: u64,
    // P95 latency
    pub p95_latency_ms: u64,
    // P99 latency
    pub p99_latency_ms: u64,
    // Error rate percentage
    pub error_rate: f64,
    // Slow query count
    pub slow_query_count: u64,
    // Slow query threshold used
    pub slow_threshold_ms: u64,
}

#[cfg(test)]
mod tests {
    use super::*;
use std::time::Instant;

    #[test]
    fn test_query_execution() {
        let exec = QueryExecution::new("SELECT * FROM users WHERE id = 123", 50);

        assert!(exec.query_id > 0);
        assert!(exec.normalized_sql.contains("?"));
    }

    #[test]
    fn test_statistics_tracking() {
        let tracker = QueryStatisticsTracker::new();

        tracker.record_query("SELECT * FROM users WHERE id = 1", 50);
        tracker.record_query("SELECT * FROM users WHERE id = 2", 60);
        tracker.record_query("SELECT * FROM orders WHERE id = 1", 100);

        assert_eq!(tracker.total_execution_count(), 3);
        // First two should normalize to same pattern
        assert!(tracker.unique_query_count() <= 2);
    }

    #[test]
    fn test_query_stats_percentiles() {
        let exec1 = QueryExecution::new("SELECT 1", 10);
        let mut stats = QueryStats::new(&exec1);

        for i in 1..100 {
            let exec = QueryExecution::new("SELECT 1", i);
            stats.update(&exec);
        }

        assert!(stats.p95_time_ms() >= 90);
        assert!(stats.p99_time_ms() >= 95);
    }

    #[test]
    fn test_workload_analyzer() {
        let tracker = QueryStatisticsTracker::new();

        for _ in 0..20 {
            let mut exec = QueryExecution::new("SELECT * FROM users WHERE email = 'test'", 1500);
            exec.tables_accessed = vec!["users".to_string()];
            tracker.record_execution(exec);
        }

        let analyzer = WorkloadAnalyzer::new();
        let result = analyzer.analyze(&tracker);

        assert!(!result.slow_queries.is_empty());
    }
}
