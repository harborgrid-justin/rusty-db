pub mod adaptive;
pub mod cte;
pub mod executor;
pub mod expressions;
pub mod hash_join;
pub mod hash_join_simd;
pub mod optimization;
pub mod optimizer;
pub mod parallel;
pub mod planner;
pub mod sort_merge;
pub mod string_functions;
pub mod subquery;
pub mod vectorized;

pub use adaptive::{AdaptiveContext, AdaptiveExecutor, RuntimeStatistics};
pub use cte::{CteContext, CteDefinition, CteOptimizer, RecursiveCteEvaluator};
pub use executor::Executor;
pub use expressions::{BinaryOperator, Expr, ExprValue, ExpressionEvaluator, UnaryOperator};
pub use hash_join::{BloomFilterHashJoin, HashJoinConfig, HashJoinExecutor};
pub use hash_join_simd::{SimdHashJoin, SimdHashJoinConfig};
pub use optimization::{
    AdaptiveOptimizer, MaterializedViewRewriter, PlanCache, StatisticsCollector,
};
pub use optimizer::{
    ColumnStatistics, IndexStatistics, Optimizer, SingleTableStatistics, TableStatistics,
};
pub use parallel::{ParallelExecutor, ParallelizationOptimizer};
pub use planner::{PlanNode, Planner};
pub use sort_merge::{ExternalMergeSorter, SortMergeJoin, TopKSelector};
pub use string_functions::{StringFunctionExecutor, StringFunctionValidator};
pub use subquery::{
    ExistsEvaluator, InEvaluator, ScalarSubqueryEvaluator, SubqueryExpr, SubqueryType,
};
pub use vectorized::{AggregationType, ColumnBatch, ColumnValue, VectorizedExecutor};

use serde::{Deserialize, Serialize};

// Query execution result
#[derive(Debug, Clone, Serialize, Deserialize, bincode::Encode, bincode::Decode)]
pub struct QueryResult {
    pub columns: Vec<String>,
    pub rows: Vec<Vec<String>>,
    pub rows_affected: usize,
    pub affected_rows: ()
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
