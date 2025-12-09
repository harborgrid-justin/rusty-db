// Analytics Manager - Core Coordination
//
// This module provides the central `AnalyticsManager` that coordinates
// all analytics subsystems including caching, statistics, OLAP operations,
// and query optimization.
//
// # Architecture
//
// The AnalyticsManager follows a facade pattern, providing a unified
// interface to:
// - Query caching and result reuse
// - Statistics collection and histogram management
// - Cost-based query optimization
// - Parallel query execution
// - Data profiling and quality analysis
//
// # Example
//
// ```rust,ignore
// use crate::analytics::manager::AnalyticsManager;
//
// let manager = AnalyticsManager::new();
//
// // Execute with caching
// let _result = manager.execute_cached("SELECT * FROM users WHERE active = true");
//
// // Analyze workload patterns
// let recommendations = manager.analyze_workload();
// ```

use parking_lot::RwLock;
use std::collections::HashMap;
use std::sync::Arc;

use super::compression::{CompressionAlgorithm, QueryResultCompressor};
use super::cost_model::{CardinalityEstimator, CostModel};
use super::data_profiler::DataProfiler;
use super::parallel::ParallelQueryExecutor;
use super::quality::{DataQualityAnalyzer, QueryPerformanceTracker};
use super::query_cache::{CacheStats, CachedResult, QueryCache};
use super::query_rewriter::{QueryRewriter, RewriteResult};
use super::query_statistics::{QueryStatisticsTracker, WorkloadAnalysisResult, WorkloadAnalyzer};
use super::statistics::{ColumnStatistics, HistogramManager};

/// Configuration for the analytics manager.
#[derive(Debug, Clone)]
pub struct AnalyticsConfig {
    /// Enable query caching
    pub cache_enabled: bool,
    /// Maximum cache size in bytes
    pub max_cache_size: usize,
    /// Cache TTL in seconds
    pub cache_ttl_secs: u64,
    /// Enable query rewriting
    pub rewrite_enabled: bool,
    /// Enable parallel execution
    pub parallel_enabled: bool,
    /// Number of parallel workers
    pub parallel_workers: usize,
    /// Enable result compression
    pub compression_enabled: bool,
    /// Compression algorithm
    pub compression_algorithm: CompressionAlgorithm,
    /// Enable statistics collection
    pub statistics_enabled: bool,
    /// Slow query threshold in milliseconds
    pub slow_query_threshold_ms: u64,
}

impl Default for AnalyticsConfig {
    fn default() -> Self {
        Self {
            cache_enabled: true,
            max_cache_size: 256 * 1024 * 1024, // 256MB
            cache_ttl_secs: 3600,
            rewrite_enabled: true,
            parallel_enabled: true,
            parallel_workers: num_cpus(),
            compression_enabled: false,
            compression_algorithm: CompressionAlgorithm::Adaptive,
            statistics_enabled: true,
            slow_query_threshold_ms: 1000,
        }
    }
}

/// Returns the number of available CPUs.
fn num_cpus() -> usize {
    std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(4)
}

/// Central analytics manager coordinating all subsystems.
#[derive(Debug)]
pub struct AnalyticsManager {
    /// Configuration
    config: AnalyticsConfig,
    /// Query cache
    cache: Arc<QueryCache>,
    /// Query statistics tracker
    statistics_tracker: Arc<QueryStatisticsTracker>,
    /// Query rewriter
    rewriter: Arc<RwLock<QueryRewriter>>,
    /// Cost model for optimization
    cost_model: Arc<CostModel>,
    /// Cardinality estimator
    cardinality_estimator: Arc<CardinalityEstimator>,
    /// Histogram manager
    histogram_manager: Arc<RwLock<HistogramManager>>,
    /// Parallel executor
    parallel_executor: Arc<ParallelQueryExecutor>,
    /// Result compressor
    compressor: Arc<QueryResultCompressor>,
    /// Data profiler
    profiler: Arc<RwLock<DataProfiler>>,
    /// Quality analyzer
    quality_analyzer: Arc<RwLock<DataQualityAnalyzer>>,
    /// Performance tracker
    performance_tracker: Arc<QueryPerformanceTracker>,
    /// Workload analyzer
    workload_analyzer: Arc<WorkloadAnalyzer>,
    /// Column statistics cache
    column_stats: Arc<RwLock<HashMap<String, ColumnStatistics>>>,
    /// Manager start time
    started_at: std::time::Instant,
}

impl Default for AnalyticsManager {
    fn default() -> Self {
        Self::new()
    }
}

impl AnalyticsManager {
    /// Creates a new analytics manager with default configuration.
    pub fn new() -> Self {
        Self::with_config(AnalyticsConfig::default())
    }

    /// Creates an analytics manager with custom configuration.
    pub fn with_config(config: AnalyticsConfig) -> Self {
        let cache = Arc::new(QueryCache::new(config.max_cache_size));
        let rewriter = if config.rewrite_enabled {
            Arc::new(RwLock::new(QueryRewriter::with_standard_rules()))
        } else {
            Arc::new(RwLock::new(QueryRewriter::new()))
        };

        Self {
            cache,
            statistics_tracker: Arc::new(QueryStatisticsTracker::new()),
            rewriter,
            cost_model: Arc::new(CostModel::new()),
            cardinality_estimator: Arc::new(CardinalityEstimator::new()),
            histogram_manager: Arc::new(RwLock::new(HistogramManager::new())),
            parallel_executor: Arc::new(ParallelQueryExecutor::new(config.parallel_workers)),
            compressor: Arc::new(QueryResultCompressor::new(config.compression_algorithm)),
            profiler: Arc::new(RwLock::new(DataProfiler::new())),
            quality_analyzer: Arc::new(RwLock::new(DataQualityAnalyzer::new())),
            performance_tracker: Arc::new(
                QueryPerformanceTracker::new()
                    .with_slow_threshold(config.slow_query_threshold_ms),
            ),
            workload_analyzer: Arc::new(WorkloadAnalyzer::new()),
            column_stats: Arc::new(RwLock::new(HashMap::new())),
            started_at: std::time::Instant::now(),
            config,
        }
    }

    /// Returns the current configuration.
    pub fn config(&self) -> &AnalyticsConfig {
        &self.config
    }

    // ========================================================================
    // Cache Operations
    // ========================================================================

    /// Checks the cache for a query result.
    pub fn get_cached(&self, query: &str) -> Option<CachedResult> {
        if !self.config.cache_enabled {
            return None;
        }
        self.cache.get(query)
    }

    /// Stores a result in the cache.
    pub fn cache_result(&self, query: &str, result: CachedResult) {
        if self.config.cache_enabled {
            self.cache.put(query, result);
        }
    }

    /// Invalidates cache entries for a table.
    pub fn invalidate_table(&self, table: &str) {
        self.cache.invalidate_table(table);
    }

    /// Returns cache statistics.
    pub fn cache_stats(&self) -> CacheStats {
        self.cache.stats()
    }

    /// Clears the entire cache.
    pub fn clear_cache(&self) {
        self.cache.clear();
    }

    // ========================================================================
    // Query Rewriting
    // ========================================================================

    /// Rewrites a query for optimization.
    pub fn rewrite_query(&self, query: &str) -> RewriteResult {
        if !self.config.rewrite_enabled {
            return RewriteResult {
                original: query.to_string(),
                rewritten: query.to_string(),
                rules_applied: Vec::new(),
                iterations: 0,
            };
        }
        self.rewriter.write().rewrite(query)
    }

    // ========================================================================
    // Statistics & Cost Model
    // ========================================================================

    /// Records query execution statistics.
    pub fn record_query(&self, query: &str, execution_time_ms: u64) {
        if self.config.statistics_enabled {
            self.statistics_tracker.record_query(query, execution_time_ms);

            let query_hash = Self::hash_query(query);
            self.performance_tracker.record(query_hash, execution_time_ms);
        }
    }

    /// Estimates the cost of a query.
    pub fn estimate_cost(&self, query: &str) -> f64 {
        self.cost_model.estimate(query, &self.column_stats.read())
    }

    /// Estimates the cardinality for a table.
    pub fn estimate_cardinality(&self, table: &str) -> usize {
        self.cardinality_estimator.estimate(table, &self.column_stats.read())
    }

    /// Updates column statistics.
    pub fn update_column_stats(&self, column: &str, stats: ColumnStatistics) {
        self.column_stats.write().insert(column.to_string(), stats);
    }

    /// Gets column statistics.
    pub fn get_column_stats(&self, column: &str) -> Option<ColumnStatistics> {
        self.column_stats.read().get(column).cloned()
    }

    // ========================================================================
    // Workload Analysis
    // ========================================================================

    /// Analyzes the current workload and returns recommendations.
    pub fn analyze_workload(&self) -> WorkloadAnalysisResult {
        self.workload_analyzer.analyze(&self.statistics_tracker)
    }

    /// Returns the top N slowest queries.
    pub fn top_slow_queries(&self, n: usize) -> Vec<super::query_statistics::QueryStats> {
        self.statistics_tracker.top_slow_queries(n)
    }

    /// Returns the top N most frequent queries.
    pub fn top_frequent_queries(&self, n: usize) -> Vec<super::query_statistics::QueryStats> {
        self.statistics_tracker.top_frequent_queries(n)
    }

    // ========================================================================
    // Data Quality
    // ========================================================================

    /// Analyzes data quality for a column.
    pub fn analyze_quality(
        &self,
        column: &str,
        values: &[Option<String>],
    ) -> super::quality::QualityMetrics {
        self.quality_analyzer.write().analyze_column(column, values)
    }

    /// Profiles a column.
    pub fn profile_column(
        &self,
        column: &str,
        values: &[Option<String>],
    ) -> super::data_profiler::ColumnProfile {
        self.profiler.write().profile_column(column, values)
    }

    // ========================================================================
    // Histograms
    // ========================================================================

    /// Gets the histogram manager for advanced operations.
    pub fn histogram_manager(&self) -> &Arc<RwLock<HistogramManager>> {
        &self.histogram_manager
    }

    // ========================================================================
    // Parallel Execution
    // ========================================================================

    /// Gets the parallel executor.
    pub fn parallel_executor(&self) -> &Arc<ParallelQueryExecutor> {
        &self.parallel_executor
    }

    /// Checks if parallel execution is enabled.
    pub fn is_parallel_enabled(&self) -> bool {
        self.config.parallel_enabled
    }

    // ========================================================================
    // Compression
    // ========================================================================

    /// Gets the result compressor.
    pub fn compressor(&self) -> &Arc<QueryResultCompressor> {
        &self.compressor
    }

    /// Checks if compression is enabled.
    pub fn is_compression_enabled(&self) -> bool {
        self.config.compression_enabled
    }

    // ========================================================================
    // Manager Status
    // ========================================================================

    /// Returns the manager uptime.
    pub fn uptime(&self) -> std::time::Duration {
        self.started_at.elapsed()
    }

    /// Returns comprehensive manager statistics.
    pub fn stats(&self) -> ManagerStats {
        ManagerStats {
            uptime_secs: self.started_at.elapsed().as_secs(),
            cache_stats: self.cache.stats(),
            total_queries_tracked: self.statistics_tracker.total_execution_count(),
            unique_query_patterns: self.statistics_tracker.unique_query_count(),
            column_stats_count: self.column_stats.read().len(),
            compression_stats: self.compressor.stats(),
        }
    }

    /// Resets all statistics.
    pub fn reset_stats(&self) {
        self.statistics_tracker.clear();
        self.performance_tracker.clear();
        self.compressor.reset_stats();
        self.rewriter.write().reset_stats();
    }

    /// Simple query hash function.
    fn hash_query(query: &str) -> u64 {
        let mut hash: u64 = 0;
        for byte in query.bytes() {
            hash = hash.wrapping_mul(31).wrapping_add(byte as u64);
        }
        hash
    }
}

/// Comprehensive statistics for the analytics manager.
#[derive(Debug, Clone)]
pub struct ManagerStats {
    /// Manager uptime in seconds
    pub uptime_secs: u64,
    /// Cache statistics
    pub cache_stats: CacheStats,
    /// Total queries tracked
    pub total_queries_tracked: u64,
    /// Unique query patterns
    pub unique_query_patterns: usize,
    /// Column statistics entries
    pub column_stats_count: usize,
    /// Compression statistics
    pub compression_stats: super::compression::CompressionStats,
}

/// Builder for creating an AnalyticsManager with custom settings.
#[derive(Debug, Default)]
pub struct AnalyticsManagerBuilder {
    config: AnalyticsConfig,
}

impl AnalyticsManagerBuilder {
    /// Creates a new builder with default configuration.
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets whether caching is enabled.
    pub fn cache_enabled(mut self, enabled: bool) -> Self {
        self.config.cache_enabled = enabled;
        self
    }

    /// Sets the maximum cache size.
    pub fn max_cache_size(mut self, size: usize) -> Self {
        self.config.max_cache_size = size;
        self
    }

    /// Sets the cache TTL.
    pub fn cache_ttl(mut self, secs: u64) -> Self {
        self.config.cache_ttl_secs = secs;
        self
    }

    /// Sets whether query rewriting is enabled.
    pub fn rewrite_enabled(mut self, enabled: bool) -> Self {
        self.config.rewrite_enabled = enabled;
        self
    }

    /// Sets whether parallel execution is enabled.
    pub fn parallel_enabled(mut self, enabled: bool) -> Self {
        self.config.parallel_enabled = enabled;
        self
    }

    /// Sets the number of parallel workers.
    pub fn parallel_workers(mut self, workers: usize) -> Self {
        self.config.parallel_workers = workers;
        self
    }

    /// Sets whether compression is enabled.
    pub fn compression_enabled(mut self, enabled: bool) -> Self {
        self.config.compression_enabled = enabled;
        self
    }

    /// Sets the compression algorithm.
    pub fn compression_algorithm(mut self, algorithm: CompressionAlgorithm) -> Self {
        self.config.compression_algorithm = algorithm;
        self
    }

    /// Sets the slow query threshold.
    pub fn slow_query_threshold(mut self, threshold_ms: u64) -> Self {
        self.config.slow_query_threshold_ms = threshold_ms;
        self
    }

    /// Builds the AnalyticsManager.
    pub fn build(self) -> AnalyticsManager {
        AnalyticsManager::with_config(self.config)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_manager_creation() {
        let manager = AnalyticsManager::new();
        assert!(manager.config.cache_enabled);
        assert!(manager.config.rewrite_enabled);
    }

    #[test]
    fn test_builder() {
        let manager = AnalyticsManagerBuilder::new()
            .cache_enabled(false)
            .parallel_workers(8)
            .build();

        assert!(!manager.config.cache_enabled);
        assert_eq!(manager.config.parallel_workers, 8);
    }

    #[test]
    fn test_query_recording() {
        let manager = AnalyticsManager::new();

        manager.record_query("SELECT * FROM users", 50);
        manager.record_query("SELECT * FROM users", 60);

        let _stats = manager.stats();
        assert_eq!(stats.total_queries_tracked, 2);
    }

    #[test]
    fn test_cache_operations() {
        let manager = AnalyticsManager::new();

        let _result = CachedResult {
            query: "".to_string(),
            result: vec![],
            timestamp: (),
            data: vec![],
            created_at: std::time::Instant::now(),
            size_bytes: 100,
            access_count: 0,
            query_hash: 123,
            ttl_seconds: 0,
            last_access: (),
        };

        manager.cache_result("SELECT 1", result);

        assert!(manager.get_cached("SELECT 1").is_some());
        assert!(manager.get_cached("SELECT 2").is_none());
    }

    #[test]
    fn test_uptime() {
        let manager = AnalyticsManager::new();
        std::thread::sleep(std::time::Duration::from_millis(10));

        assert!(manager.uptime().as_millis() >= 10);
    }
}
