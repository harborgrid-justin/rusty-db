// Analytics Module - Enterprise Query Analytics and Optimization
//
// This module provides comprehensive analytics capabilities for the database,
// including query caching, cost-based optimization, OLAP operations,
// data profiling, and workload analysis.
//
// # Architecture
//
// The analytics module is organized into focused submodules:
//
// - **Core Components**
//   - [`manager`] - Central coordination of all analytics subsystems
//   - [`query_cache`] - LRU-based query result caching
//   - [`statistics`] - Column statistics and histogram management
//
// - **Query Optimization**
//   - [`cost_model`] - Cost-based query optimization and cardinality estimation
//   - [`query_rewriter`] - Query transformation and optimization rules
//   - [`parallel`] - Parallel query execution support
//
// - **OLAP & Analytics**
//   - [`olap`] - OLAP cube operations (drill-down, roll-up, slice, dice)
//   - [`aggregates`] - Aggregate function definitions
//   - [`window_functions`] - SQL window function support
//   - [`views`] - View and materialized view types
//
// - **Time Series & Trends**
//   - [`timeseries_analyzer`] - Time series analysis and anomaly detection
//   - [`timeseries`] - Time series data structures
//
// - **Data Quality & Profiling**
//   - [`data_profiler`] - Column profiling and type inference
//   - [`quality`] - Data quality metrics and validation
//   - [`sampling`] - Query sampling and approximate query processing
//
// - **Workload Analysis**
//   - [`query_statistics`] - Query execution tracking and workload analysis
//   - [`compression`] - Query result compression
//
// - **Legacy/Existing Modules**
//   - [`caching`] - Additional caching utilities
//   - [`materialized_views`] - Materialized view management
//   - [`approximate`] - Approximate query processing
//   - [`window`] - Window operations
//   - [`cube`] - OLAP cube structures
//   - [`warehouse`] - Data warehouse utilities
//
// # Example
//
// ```rust,ignore
// use crate::analytics::manager::AnalyticsManager;
// use crate::analytics::query_cache::QueryCache;
//
// // Create an analytics manager
// let manager = AnalyticsManager::new();
//
// // Record query execution
// manager.record_query("SELECT * FROM users", 50);
//
// // Analyze workload patterns
// let recommendations = manager.analyze_workload();
// for rec in recommendations.recommendations {
//     println!("Recommendation: {} on {:?}", rec.reason, rec.columns);
// }
// ```
//
// # Design Principles
//
// This module follows enterprise Rust architecture principles:
//
// 1. **Cohesive Modules**: Each submodule is 150-400 lines with single responsibility
// 2. **Strong Typing**: Extensive use of newtypes and enums for type safety
// 3. **Traits for Extensibility**: Core behaviors defined via traits
// 4. **Error Handling**: Consistent use of `Result` with thiserror
// 5. **Documentation**: Comprehensive rustdoc with examples
// 6. **Thread Safety**: Proper use of `Arc`, `RwLock`, and `Send + Sync`

// ============================================================================
// Core Components
// ============================================================================

pub mod manager;
pub mod query_cache;
pub mod statistics;

// ============================================================================
// Query Optimization
// ============================================================================

pub mod cost_model;
pub mod query_rewriter;
pub mod parallel;

// ============================================================================
// OLAP & Analytics
// ============================================================================

pub mod olap;
pub mod aggregates;
pub mod window_functions;
pub mod views;

// ============================================================================
// Time Series & Trends
// ============================================================================

pub mod timeseries_analyzer;

// ============================================================================
// Data Quality & Profiling
// ============================================================================

pub mod data_profiler;
pub mod quality;
pub mod sampling;

// ============================================================================
// Workload Analysis
// ============================================================================

pub mod query_statistics;
pub mod compression;

// ============================================================================
// Legacy/Existing Modules
// ============================================================================

pub mod caching;
pub mod materialized_views;
pub mod approximate;
pub mod window;
pub mod cube;
pub mod timeseries;
pub mod warehouse;
mod query_cache_impl;
mod view_management;
mod mod_new;
// ============================================================================
// Re-exports for Convenience
// ============================================================================

// Manager
pub use manager::{AnalyticsManager, AnalyticsManagerBuilder, AnalyticsConfig, ManagerStats};

// Query Cache
pub use query_cache::{QueryCache, CachedResult, CacheStats};

// Statistics
pub use statistics::{ColumnStatistics, Histogram, HistogramBucket, HistogramType, HistogramManager};

// Cost Model
pub use cost_model::{CostModel, CardinalityEstimator, JoinAlgorithm};

// Query Rewriter
pub use query_rewriter::{
    QueryRewriter, RewriteRule, RewriteResult, RewriteStats,
    DeltaTable, DeltaOperation, DeltaRow,
    IncrementalViewMaintenance, ViewDelta,
};

// Parallel Execution
pub use parallel::ParallelQueryExecutor;

// OLAP
pub use olap::{OlapCube, OlapCubeBuilder, MultidimensionalAggregator, AggregationCube};

// Aggregates
pub use aggregates::AggregateFunction;

// Window Functions
pub use window_functions::{WindowFunction, WindowFrame, FrameType, FrameBound};

// Views
pub use views::{
    MaterializedView, View, ViewStatistics, RefreshSchedule,
    MaterializedViewIndex, CheckOption,
};

// Time Series
pub use timeseries_analyzer::{TimeSeriesAnalyzer, Trend, AnomalyDetector};

// Data Profiler
pub use data_profiler::{
    DataProfiler, ColumnProfile, InferredType, BitmapIndex, IndexSuggestion,
};

// Quality
pub use quality::{
    DataQualityAnalyzer, QualityMetrics, QualityIssue, QualityIssueType,
    QualityDimension, ValidationRule, ValidationRuleType,
    TableQualityReport, QueryPerformanceTracker, PerformanceMetrics,
};

// Sampling
pub use sampling::{
    QueryResultSampler, SamplingMethod, SamplingConfig, SampledResult,
    ApproximateQueryProcessor, ApproximateResult, SampledData,
};

// Query Statistics
pub use query_statistics::{
    QueryStatisticsTracker, QueryStats, QueryExecution,
    WorkloadAnalyzer, WorkloadAnalysisResult, WorkloadQuery,
    IndexRecommendation, PerformanceReport,
};

// Compression
pub use compression::{
    QueryResultCompressor, CompressionAlgorithm, CompressedResult,
    CompressedColumnInfo, CompressionStats,
};
