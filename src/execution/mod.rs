pub mod executor;
pub mod planner;
pub mod optimizer;
pub mod cte;
pub mod subquery;
pub mod optimization;
pub mod parallel;
pub mod vectorized;
pub mod adaptive;
pub mod hash_join;
pub mod hash_join_simd;
pub mod sort_merge;
pub mod expressions;

pub use executor::Executor;
pub use planner::{Planner, PlanNode};
pub use optimizer::{Optimizer, TableStatistics, SingleTableStatistics, ColumnStatistics, IndexStatistics};
pub use cte::{CteContext, CteDefinition, RecursiveCteEvaluator, CteOptimizer};
pub use subquery::{SubqueryExpr, SubqueryType, ExistsEvaluator, InEvaluator, ScalarSubqueryEvaluator};
pub use optimization::{PlanCache, StatisticsCollector, AdaptiveOptimizer, MaterializedViewRewriter};
pub use parallel::{ParallelExecutor, ParallelizationOptimizer};
pub use vectorized::{VectorizedExecutor, ColumnBatch, ColumnValue, AggregationType};
pub use adaptive::{AdaptiveExecutor, AdaptiveContext, RuntimeStatistics};
pub use hash_join::{HashJoinExecutor, HashJoinConfig, BloomFilterHashJoin};
pub use hash_join_simd::{SimdHashJoin, SimdHashJoinConfig};
pub use sort_merge::{ExternalMergeSorter, SortMergeJoin, TopKSelector};
pub use expressions::{ExpressionEvaluator, Expr, ExprValue, BinaryOperator, UnaryOperator};

use serde::{Deserialize, Serialize};

/// Query execution result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryResult {
    pub columns: Vec<String>,
    pub rows: Vec<Vec<String>>,
    pub rows_affected: usize,
}

impl QueryResult {
    pub fn new(columns: Vec<String>, rows: Vec<Vec<String>>) -> Self {
        let rows_affected = rows.len();
        Self {
            columns,
            rows,
            rows_affected,
        }
    }
    
    pub fn empty() -> Self {
        Self {
            columns: Vec::new(),
            rows: Vec::new(),
            rows_affected: 0,
        }
    }
    
    pub fn with_affected(rows_affected: usize) -> Self {
        Self {
            columns: Vec::new(),
            rows: Vec::new(),
            rows_affected,
        }
    }
}


