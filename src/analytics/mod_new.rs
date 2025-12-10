// Analytics and Advanced Query Processing Module
//
// This module provides comprehensive analytics capabilities including views,
// materialized views, aggregation functions, window functions, and query caching.
//
// # Module Organization
//
// The analytics module combines existing submodules with newly refactored components:
//
// | Module | Responsibility |
// |--------|----------------|
// | [`view_management`] | View and materialized view management |
// | [`query_cache_impl`] | Query result caching |
// | [`caching`] | Legacy caching implementation |
// | [`materialized_views`] | Materialized view operations |
// | [`approximate`] | Approximate query processing |
// | [`window`] | Window function execution |
// | [`cube`] | OLAP cube operations |
// | [`timeseries`] | Time series analytics |
// | [`warehouse`] | Data warehouse features |
//
// # Quick Start
//
// ```rust,ignore
// use rusty_db::analytics::{ViewManager, QueryCache};
//
// // Create a view manager
// let manager = ViewManager::new();
//
// // Create a query cache
// let cache = QueryCache::new(1000);
// ```

// =============================================================================
// Submodule declarations - Commented out as they're declared in parent mod.rs
// =============================================================================

// New refactored modules - declared in parent mod.rs
// pub mod view_management;
// pub mod query_cache_impl;

// Existing legacy submodules - declared in parent mod.rs
// pub mod caching;
// pub mod materialized_views;
// pub mod approximate;
// pub mod window;
// pub mod cube;
// pub mod timeseries;
// pub mod warehouse;

// =============================================================================
// Re-exports from existing code (from mod_old.rs)
// =============================================================================

use serde::{Deserialize, Serialize};
use crate::error::{Result, DbError};
use std::time::SystemTime;

// Re-export new modules
pub use super::view_management::{
    CheckOption,
    MaterializedView,
    MaterializedViewIndex,
    RefreshSchedule,
    View,
    ViewManager,
    ViewStatistics,
};

pub use super::query_cache_impl::{
    CacheStats,
    QueryCache,
};

// =============================================================================
// Core analytics types (from mod_old.rs)
// =============================================================================

// Window function specification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum WindowFunction {
    RowNumber,
    Rank,
    DenseRank,
    Lead { offset: usize, default: Option<String> },
    Lag { offset: usize, default: Option<String> },
    FirstValue,
    LastValue,
    NthValue { n: usize },
    NTile { buckets: usize },
    PercentRank,
    CumeDist,
}

// Window frame specification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WindowFrame {
    pub frame_type: FrameType,
    pub start_bound: FrameBound,
    pub end_bound: FrameBound,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FrameType {
    Rows,
    Range,
    Groups,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FrameBound {
    UnboundedPreceding,
    Preceding(usize),
    CurrentRow,
    Following(usize),
    UnboundedFollowing,
}

// Aggregate function
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AggregateFunction {
    Count,
    CountDistinct,
    Sum,
    Avg,
    Min,
    Max,
    StdDev,
    StdDevPop,
    Variance,
    VarPop,
    Median,
    Mode,
    Percentile { percentile: f64 },
    FirstValue,
    LastValue,
    StringAgg { separator: String },
    ArrayAgg,
    JsonAgg,
    JsonObjectAgg,
    BitAnd,
    BitOr,
    BitXor,
    BoolAnd,
    BoolOr,
    Every,
    Corr,
    CovarPop,
    CovarSamp,
    RegrSlope,
    RegrIntercept,
    RegrR2,
}

// Query execution statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryExecution {
    pub query: String,
    pub start_time: SystemTime,
    pub duration_ms: f64,
    pub rows_scanned: u64,
    pub rows_returned: u64,
    pub cache_hit: bool,
    pub execution_plan: String,
    pub success: bool,
}

// Column statistics for query optimization
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ColumnStatistics {
    pub table_name: String,
    pub column_name: String,
    pub distinct_count: u64,
    pub null_count: u64,
    pub total_count: u64,
    pub min_value: Option<String>,
    pub max_value: Option<String>,
    pub avg_length: f64,
    pub most_common_values: Vec<(String, u64)>,
    pub last_updated: SystemTime,
}

impl ColumnStatistics {
    pub fn new(table: String, column: String) -> Self {
        Self {
            table_name: table,
            column_name: column,
            distinct_count: 0,
            null_count: 0,
            total_count: 0,
            min_value: None,
            max_value: None,
            avg_length: 0.0,
            most_common_values: Vec::new(),
            last_updated: SystemTime::now(),
        }
    }

    pub fn selectivity(&self) -> f64 {
        if self.total_count == 0 {
            return 1.0;
        }
        self.distinct_count as f64 / self.total_count as f64
    }

    pub fn null_fraction(&self) -> f64 {
        if self.total_count == 0 {
            return 0.0;
        }
        self.null_count as f64 / self.total_count as f64
    }
}

// =============================================================================
// Module-level tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_view_manager_integration() {
        let manager = ViewManager::new();
        assert_eq!(manager.list_views().len(), 0);
        assert_eq!(manager.list_materialized_views().len(), 0);
    }

    #[test]
    fn test_query_cache_integration() {
        let cache = QueryCache::new(100);
        let result = vec![vec!["test".to_string()]];

        cache.put("SELECT 1".to_string(), result.clone(), 60);
        let cached = cache.get("SELECT 1");

        assert!(cached.is_some());
        assert_eq!(cached.unwrap(), result);
    }

    #[test]
    fn test_column_statistics() {
        let mut stats = ColumnStatistics::new("users".to_string(), "id".to_string());
        stats.total_count = 100;
        stats.distinct_count = 100;
        stats.null_count = 0;

        assert_eq!(stats.selectivity(), 1.0);
        assert_eq!(stats.null_fraction(), 0.0);
    }
}

// =============================================================================
// Note on refactoring
// =============================================================================
//
// The previous mod_old.rs file contained ~3000 lines with mixed concerns.
// This refactored version:
// - Extracts view management into view_management.rs
// - Extracts query caching into query_cache_impl.rs
// - Preserves all existing submodules (caching, materialized_views, etc.)
// - Maintains all public APIs and functionality
// - Provides clearer module organization
//
// The mod_old.rs.backup file is retained for reference.
