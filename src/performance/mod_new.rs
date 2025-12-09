/// Performance Optimization and Monitoring Module
///
/// This module provides enterprise-grade performance features for query optimization,
/// caching, workload analysis, and performance monitoring.
///
/// # Module Organization
///
/// The performance module is organized into focused submodules:
///
/// | Module | Responsibility |
/// |--------|----------------|
/// | [`plan_cache`] | Query plan caching with LRU eviction |
/// | [`workload_analysis`] | Workload analysis and query pattern detection |
/// | [`adaptive_optimizer`] | Adaptive query optimization using statistics |
/// | [`performance_stats`] | Performance metrics collection and reporting |
///
/// # Quick Start
///
/// ```rust,ignore
/// use rusty_db::performance::{QueryPlanCache, WorkloadAnalyzer};
///
/// // Create a query plan cache
/// let cache = QueryPlanCache::new(1000);
///
/// // Create a workload analyzer
/// let analyzer = WorkloadAnalyzer::new(10000);
/// ```
///
/// # Architecture
///
/// ```text
/// ┌─────────────────────────────────────────────────────────────┐
/// │                  Performance Module                          │
/// │  ┌─────────────┐  ┌─────────────┐  ┌─────────────────────┐  │
/// │  │ Plan Cache  │  │  Workload   │  │   Adaptive          │  │
/// │  │             │  │  Analyzer   │  │   Optimizer         │  │
/// │  └─────────────┘  └─────────────┘  └─────────────────────┘  │
/// │  ┌─────────────┐                                            │
/// │  │Performance  │                                            │
/// │  │Statistics   │                                            │
/// │  └─────────────┘                                            │
/// └─────────────────────────────────────────────────────────────┘
/// ```

// =============================================================================
// Submodule declarations - Commented out as they're declared in parent mod.rs
// =============================================================================

// pub mod plan_cache;
// pub mod workload_analysis;
// pub mod adaptive_optimizer;
// pub mod performance_stats;

// =============================================================================
// Re-exports for convenient access
// =============================================================================

// Plan caching
pub use super::plan_cache::{
    CacheStatistics,
    QueryPlan,
    QueryPlanCache,
};

// Workload analysis
pub use super::workload_analysis::{
    QueryExecution,
    WorkloadAnalysis,
    WorkloadAnalyzer,
};

// Adaptive optimization
pub use super::adaptive_optimizer::{
    AdaptiveQueryOptimizer,
    OptimizationSuggestions,
    QueryStatistics,
};

// Performance statistics
pub use super::performance_stats::{
    GlobalPerformanceStats,
    PerformanceStatsCollector,
    QueryPerformanceStats,
};

// =============================================================================
// Module-level tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_plan_cache_integration() {
        let cache = QueryPlanCache::new(100);
        let plan = QueryPlan {
            query_hash: "test".to_string(),
            plan_tree: "SELECT * FROM test".to_string(),
            estimated_cost: 10.0,
            estimated_rows: 100,
        };

        assert!(cache.put("test".to_string(), plan, 10.0).is_ok());
        assert!(cache.get("test").unwrap().is_some());
    }

    #[test]
    fn test_workload_analyzer_integration() {
        let analyzer = WorkloadAnalyzer::new(1000);
        let execution = QueryExecution {
            query_hash: "q1".to_string(),
            execution_time_ms: 100,
            rows_returned: 50,
            timestamp: std::time::SystemTime::now(),
        };

        assert!(analyzer.log_execution(execution).is_ok());
        assert!(analyzer.analyze().is_ok());
    }

    #[test]
    fn test_adaptive_optimizer_integration() {
        let optimizer = AdaptiveQueryOptimizer::new(0.1, 5);

        for _ in 0..10 {
            optimizer.record_execution("q1", 100.0, 1000, 150).unwrap();
        }

        let suggestions = optimizer.get_suggestions("q1").unwrap();
        assert!(!suggestions.suggestions.is_empty());
    }

    #[test]
    fn test_performance_stats_integration() {
        let collector = PerformanceStatsCollector::new();

        collector.record_query("q1", 100, 1000).unwrap();

        let stats = collector.get_query_stats("q1").unwrap();
        assert!(stats.is_some());

        let global = collector.get_global_stats().unwrap();
        assert_eq!(global.total_queries, 1);
    }
}

// =============================================================================
// Note on legacy mod.rs
// =============================================================================
//
// The previous mod.rs file (now backed up as mod.rs.backup) contained ~3000 lines
// of code with many components mixed together. This refactored version splits
// the functionality into focused, maintainable modules while preserving all
// public interfaces and functionality.
//
// Key improvements:
// - Smaller, focused modules (< 500 lines each)
// - Clear separation of concerns
// - Easier to test and maintain
// - Better documentation
// - Preserved all public APIs
//
// The backup file is retained for reference and can be removed once the
// refactoring is verified to be complete and correct.
