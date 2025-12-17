// Analytics Module - Enterprise Query Analytics and Optimization
//
// ⚠️ **WARNING: MODULE BLOAT - 27 FILES WITH OVERLAP** ⚠️
//
// **Issue**: Analytics module has grown to 27 files with significant overlap
//
// **Duplication Identified**:
// 1. **3 Cache Implementations**: query_cache.rs, query_cache_impl.rs, caching.rs
// 2. **3 View Files**: views.rs, materialized_views.rs, view_management.rs
// 3. **2 Window Files**: window.rs, window_functions.rs
// 4. **2 Time Series**: timeseries.rs, timeseries_analyzer.rs (+ ml_engine/timeseries.rs)
// 5. **2 Compression**: compression.rs (+ inmemory/compression.rs)
// 6. **2 OLAP Files**: olap.rs, cube.rs
//
// **TODO - MEDIUM PRIORITY - Module Consolidation**:
// 1. Merge cache implementations into single query_cache.rs
// 2. Merge view files into single views/ submodule
// 3. Merge window files into single window_functions.rs
// 4. Merge time series into single timeseries/ submodule (with ml_engine)
// 5. Remove compression.rs (use src/compression/ instead)
// 6. Merge OLAP files (olap.rs + cube.rs)
//
// **Recommended Target Structure** (12 files instead of 27):
// ```
// src/analytics/
//   ├── mod.rs              (this file)
//   ├── manager.rs          (central coordinator)
//   ├── query_cache.rs      (merged: query_cache + query_cache_impl + caching)
//   ├── statistics.rs       (column stats, histograms)
//   ├── cost_model.rs       (cost estimation)
//   ├── query_rewriter.rs   (transformations)
//   ├── olap.rs             (merged: olap + cube)
//   ├── window_functions.rs (merged: window + window_functions)
//   ├── views/              (merged: views + materialized_views + view_management)
//   │   ├── mod.rs
//   │   ├── materialized.rs
//   │   └── refresh.rs
//   ├── timeseries/         (merged: timeseries + timeseries_analyzer + ml_engine/timeseries)
//   │   ├── mod.rs
//   │   ├── analysis.rs
//   │   └── forecasting.rs
//   ├── data_profiler.rs    (profiling + quality)
//   └── workload.rs         (query_statistics + sampling)
// ```
//
// **Unbounded Data Structures to Fix**:
// - Query cache: Unbounded LRU (query_cache.rs) - use BoundedHashMap
// - Statistics: Unbounded column stats (statistics.rs) - add limits
// - OLAP cube: Unbounded dimensions (olap.rs) - add capacity limits
// - Workload history: Unbounded query history (query_statistics.rs) - add retention policy
//
// **Impact**: 27 files → 12 files, ~2000 lines of duplication eliminated
// **Priority**: MEDIUM - consolidate in next major refactoring
// **Cross-Reference**: See diagrams/08_specialized_engines_flow.md section 10
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
pub mod parallel;
pub mod query_rewriter;

// ============================================================================
// OLAP & Analytics
// ============================================================================

pub mod aggregates;
pub mod olap;
pub mod views;
pub mod window_functions;

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

pub mod compression;
pub mod query_statistics;

// ============================================================================
// Legacy/Existing Modules
// ============================================================================

pub mod approximate;
pub mod caching;
pub mod cube;
pub mod materialized_views;
mod query_cache_impl;
pub mod timeseries;
mod view_management;
pub mod warehouse;
pub mod window;
// ============================================================================
// Re-exports for Convenience
// ============================================================================

// Manager
pub use manager::{AnalyticsConfig, AnalyticsManager, AnalyticsManagerBuilder, ManagerStats};

// Query Cache
pub use query_cache::{CacheStats, CachedResult, QueryCache};

// Statistics
pub use statistics::{
    ColumnStatistics, Histogram, HistogramBucket, HistogramManager, HistogramType,
};

// Cost Model
pub use cost_model::{CardinalityEstimator, CostModel, JoinAlgorithm};

// Query Rewriter
pub use query_rewriter::{
    DeltaOperation, DeltaRow, DeltaTable, IncrementalViewMaintenance, QueryRewriter, RewriteResult,
    RewriteRule, RewriteStats, ViewDelta,
};

// Parallel Execution
pub use parallel::ParallelQueryExecutor;

// OLAP
pub use olap::{AggregationCube, MultidimensionalAggregator, OlapCube, OlapCubeBuilder};

// Aggregates
pub use aggregates::AggregateFunction;

// Window Functions
pub use window_functions::{FrameBound, FrameType, WindowFrame, WindowFunction};

// Views
pub use views::{
    CheckOption, MaterializedView, MaterializedViewIndex, RefreshSchedule, View, ViewStatistics,
};

// Time Series
pub use timeseries_analyzer::{AnomalyDetector, TimeSeriesAnalyzer, Trend};

// Data Profiler
pub use data_profiler::{BitmapIndex, ColumnProfile, DataProfiler, IndexSuggestion, InferredType};

// Quality
pub use quality::{
    DataQualityAnalyzer, PerformanceMetrics, QualityDimension, QualityIssue, QualityIssueType,
    QualityMetrics, QueryPerformanceTracker, TableQualityReport, ValidationRule,
    ValidationRuleType,
};

// Sampling
pub use sampling::{
    ApproximateQueryProcessor, ApproximateResult, QueryResultSampler, SampledData, SampledResult,
    SamplingConfig, SamplingMethod,
};

// Query Statistics
pub use query_statistics::{
    IndexRecommendation, PerformanceReport, QueryExecution, QueryStatisticsTracker, QueryStats,
    WorkloadAnalysisResult, WorkloadAnalyzer, WorkloadQuery,
};

// Compression
pub use compression::{
    CompressedColumnInfo, CompressedResult, CompressionAlgorithm, CompressionStats,
    QueryResultCompressor,
};
