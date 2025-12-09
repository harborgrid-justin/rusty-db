// Cost Model for Query Optimization
//
// This module provides cost estimation for query plan operators:
//
// - **Cardinality Estimation**: Predict output row counts
// - **Cost Functions**: Estimate CPU and I/O costs
// - **Join Algorithm Selection**: Choose optimal join strategies
// - **Optimizer Hints**: User-provided optimization directives

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// =============================================================================
// Cardinality Estimator
// =============================================================================

// Estimates the cardinality (row count) of query plan operators.
pub struct CardinalityEstimator {
    // Known table cardinalities
    table_cardinalities: HashMap<String, u64>,

    // Cached selectivity estimates
    selectivity_cache: HashMap<String, f64>,
}

impl CardinalityEstimator {
    // Create a new cardinality estimator.
    pub fn new() -> Self {
        Self {
            table_cardinalities: HashMap::new(),
            selectivity_cache: HashMap::new(),
        }
    }

    // Set the cardinality for a table.
    pub fn set_table_cardinality(&mut self, table: String, cardinality: u64) {
        self.table_cardinalities.insert(table, cardinality);
    }

    // Estimate cardinality for a table scan.
    pub fn estimate_scan(&self, table: &str) -> u64 {
        self.table_cardinalities.get(table).copied().unwrap_or(1000)
    }

    // Estimate cardinality after a filter operation.
    //
    // Uses default selectivity of 0.1 (10%) if not cached.
    pub fn estimate_filter(&self, input_card: u64, predicate: &str) -> u64 {
        let selectivity = self
            .selectivity_cache
            .get(predicate)
            .copied()
            .unwrap_or(0.1);

        (input_card as f64 * selectivity).max(1.0) as u64
    }

    // Estimate cardinality for a join operation.
    pub fn estimate_join(&self, left_card: u64, right_card: u64, join_type: JoinType) -> u64 {
        match join_type {
            JoinType::Inner => {
                // Assume foreign key join selectivity
                left_card.max(right_card)
            }
            JoinType::Left => left_card,
            JoinType::Right => right_card,
            JoinType::Full => left_card + right_card,
            JoinType::Cross => left_card.saturating_mul(right_card),
        }
    }

    // Estimate cardinality for an aggregate operation.
    pub fn estimate_aggregate(&self, input_card: u64, group_by_cols: usize) -> u64 {
        if group_by_cols == 0 {
            return 1; // Single row result for no GROUP BY
        }

        // Estimate distinct values using sqrt heuristic
        let distinct_per_col = (input_card as f64).sqrt();
        distinct_per_col.powi(group_by_cols as i32).min(input_card as f64) as u64
    }

    // Estimate cardinality for a DISTINCT operation.
    pub fn estimate_distinct(&self, input_card: u64, _columns: &[String]) -> u64 {
        // Assume 50% distinct values by default
        (input_card / 2).max(1)
    }

    // Cache a selectivity estimate.
    pub fn cache_selectivity(&mut self, predicate: String, selectivity: f64) {
        self.selectivity_cache.insert(predicate, selectivity.clamp(0.0, 1.0));
    }
}

impl Default for CardinalityEstimator {
    fn default() -> Self {
        Self::new()
    }
}

// =============================================================================
// Cost Model
// =============================================================================

// Cost model for query optimization.
//
// Provides cost estimates for various query plan operators based on
// configurable cost factors.
pub struct CostModel {
    // Cost factor for sequential scan per row
    pub seq_scan_cost_factor: f64,

    // Cost factor for index scan per row
    pub index_scan_cost_factor: f64,

    // Cost factor for hash join per row
    pub hash_join_cost_factor: f64,

    // Cost factor for merge join per row
    pub merge_join_cost_factor: f64,

    // Cost factor for nested loop join per row
    pub nested_loop_cost_factor: f64,

    // Cost factor for sorting per row
    pub sort_cost_factor: f64,

    // Cost factor for hash aggregate per row
    pub hash_aggregate_cost_factor: f64,
}

impl CostModel {
    // Create a new cost model with default factors.
    pub fn new() -> Self {
        Self {
            seq_scan_cost_factor: 1.0,
            index_scan_cost_factor: 0.01,
            hash_join_cost_factor: 0.1,
            merge_join_cost_factor: 0.15,
            nested_loop_cost_factor: 0.5,
            sort_cost_factor: 0.2,
            hash_aggregate_cost_factor: 0.1,
        }
    }

    // Create a cost model optimized for in-memory operations.
    pub fn in_memory() -> Self {
        Self {
            seq_scan_cost_factor: 0.1,
            index_scan_cost_factor: 0.005,
            hash_join_cost_factor: 0.05,
            merge_join_cost_factor: 0.08,
            nested_loop_cost_factor: 0.3,
            sort_cost_factor: 0.1,
            hash_aggregate_cost_factor: 0.05,
        }
    }

    // Cost of a sequential scan.
    pub fn cost_seq_scan(&self, cardinality: u64) -> f64 {
        cardinality as f64 * self.seq_scan_cost_factor
    }

    // Cost of an index scan.
    pub fn cost_index_scan(&self, cardinality: u64) -> f64 {
        cardinality as f64 * self.index_scan_cost_factor
    }

    // Cost of a hash join.
    pub fn cost_hash_join(&self, left_card: u64, right_card: u64) -> f64 {
        // Build cost + probe cost
        let build_cost = right_card as f64 * self.hash_join_cost_factor;
        let probe_cost = left_card as f64 * self.hash_join_cost_factor;
        build_cost + probe_cost
    }

    // Cost of a merge join.
    pub fn cost_merge_join(&self, left_card: u64, right_card: u64) -> f64 {
        (left_card + right_card) as f64 * self.merge_join_cost_factor
    }

    // Cost of a nested loop join.
    pub fn cost_nested_loop(&self, left_card: u64, right_card: u64) -> f64 {
        (left_card * right_card) as f64 * self.nested_loop_cost_factor
    }

    // Cost of a sort operation.
    pub fn cost_sort(&self, cardinality: u64) -> f64 {
        let n = cardinality as f64;
        n * n.log2().max(1.0) * self.sort_cost_factor
    }

    // Cost of a hash aggregate.
    pub fn cost_hash_aggregate(&self, cardinality: u64) -> f64 {
        cardinality as f64 * self.hash_aggregate_cost_factor
    }

    // Choose the best join algorithm based on cost.
    pub fn choose_join_algorithm(&self, left_card: u64, right_card: u64) -> JoinAlgorithm {
        let hash_cost = self.cost_hash_join(left_card, right_card);
        let merge_cost = self.cost_merge_join(left_card, right_card);
        let nested_cost = self.cost_nested_loop(left_card, right_card);

        if hash_cost <= merge_cost && hash_cost <= nested_cost {
            JoinAlgorithm::Hash
        } else if merge_cost <= nested_cost {
            JoinAlgorithm::Merge
        } else {
            JoinAlgorithm::NestedLoop
        }
    }

    // Estimate total plan cost.
    pub fn estimate_plan_cost(&self, operations: &[PlanOperation]) -> f64 {
        operations.iter().map(|op| self.cost_operation(op)).sum()
    }

    // Cost a single operation.
    fn cost_operation(&self, op: &PlanOperation) -> f64 {
        match op {
            PlanOperation::SeqScan { cardinality } => self.cost_seq_scan(*cardinality),
            PlanOperation::IndexScan { cardinality } => self.cost_index_scan(*cardinality),
            PlanOperation::HashJoin {
                left_card,
                right_card,
            } => self.cost_hash_join(*left_card, *right_card),
            PlanOperation::MergeJoin {
                left_card,
                right_card,
            } => self.cost_merge_join(*left_card, *right_card),
            PlanOperation::NestedLoop {
                left_card,
                right_card,
            } => self.cost_nested_loop(*left_card, *right_card),
            PlanOperation::Sort { cardinality } => self.cost_sort(*cardinality),
            PlanOperation::HashAggregate { cardinality } => self.cost_hash_aggregate(*cardinality),
        }
    }
}

impl Default for CostModel {
    fn default() -> Self {
        Self::new()
    }
}

// =============================================================================
// Supporting Types
// =============================================================================

// Join algorithm types.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum JoinAlgorithm {
    // Hash join (good for large unsorted inputs)
    Hash,
    // Merge join (good for sorted inputs)
    Merge,
    // Nested loop join (good for small inputs or index lookups)
    NestedLoop,
}

// Join types.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum JoinType {
    // Inner join
    Inner,
    // Left outer join
    Left,
    // Right outer join
    Right,
    // Full outer join
    Full,
    // Cross join (Cartesian product)
    Cross,
}

// Query plan operations for cost estimation.
#[derive(Debug, Clone)]
pub enum PlanOperation {
    SeqScan { cardinality: u64 },
    IndexScan { cardinality: u64 },
    HashJoin { left_card: u64, right_card: u64 },
    MergeJoin { left_card: u64, right_card: u64 },
    NestedLoop { left_card: u64, right_card: u64 },
    Sort { cardinality: u64 },
    HashAggregate { cardinality: u64 },
}

// Optimizer hints for query execution.
//
// Allows users to influence the query optimizer's decisions.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct OptimizerHints {
    // Force use of specific indexes
    pub use_index: Option<Vec<String>>,

    // Force a specific join order
    pub join_order: Option<Vec<String>>,

    // Set degree of parallelism
    pub parallelism: Option<usize>,

    // Force use of a materialized view
    pub materialized_view: Option<String>,

    // Disable result caching
    pub no_cache: bool,

    // Force a specific join algorithm
    pub join_algorithm: Option<JoinAlgorithm>,

    // Force sequential scan (disable index usage)
    pub seq_scan: bool,
}

impl OptimizerHints {
    // Create empty hints.
    pub fn none() -> Self {
        Self::default()
    }

    // Create hints to use a specific index.
    pub fn use_index(index_name: &str) -> Self {
        Self {
            use_index: Some(vec![index_name.to_string()]),
            ..Default::default()
        }
    }

    // Create hints for parallel execution.
    pub fn parallel(degree: usize) -> Self {
        Self {
            parallelism: Some(degree),
            ..Default::default()
        }
    }

    // Check if any hints are specified.
    pub fn has_hints(&self) -> bool {
        self.use_index.is_some()
            || self.join_order.is_some()
            || self.parallelism.is_some()
            || self.materialized_view.is_some()
            || self.no_cache
            || self.join_algorithm.is_some()
            || self.seq_scan
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cardinality_estimator() {
        let mut estimator = CardinalityEstimator::new();
        estimator.set_table_cardinality("users".to_string(), 1000);

        assert_eq!(estimator.estimate_scan("users"), 1000);
        assert_eq!(estimator.estimate_scan("unknown"), 1000); // Default
    }

    #[test]
    fn test_filter_estimation() {
        let estimator = CardinalityEstimator::new();
        let filtered = estimator.estimate_filter(1000, "age > 18");

        // Default selectivity is 0.1
        assert_eq!(filtered, 100);
    }

    #[test]
    fn test_join_estimation() {
        let estimator = CardinalityEstimator::new();

        let inner = estimator.estimate_join(100, 50, JoinType::Inner);
        assert_eq!(inner, 100);

        let cross = estimator.estimate_join(100, 50, JoinType::Cross);
        assert_eq!(cross, 5000);
    }

    #[test]
    fn test_aggregate_estimation() {
        let estimator = CardinalityEstimator::new();

        let no_group = estimator.estimate_aggregate(1000, 0);
        assert_eq!(no_group, 1);

        let with_group = estimator.estimate_aggregate(1000, 1);
        assert!(with_group > 1 && with_group < 1000);
    }

    #[test]
    fn test_cost_model() {
        let model = CostModel::new();

        let seq_cost = model.cost_seq_scan(1000);
        let index_cost = model.cost_index_scan(1000);

        // Index scan should be cheaper
        assert!(index_cost < seq_cost);
    }

    #[test]
    fn test_join_algorithm_selection() {
        let model = CostModel::new();

        // For small inputs, nested loop might be best
        // For larger inputs, hash join is usually best
        let algo = model.choose_join_algorithm(10000, 10000);
        assert_eq!(algo, JoinAlgorithm::Hash);
    }

    #[test]
    fn test_plan_cost() {
        let model = CostModel::new();
        let operations = vec![
            PlanOperation::SeqScan { cardinality: 1000 },
            PlanOperation::Sort { cardinality: 1000 },
        ];

        let cost = model.estimate_plan_cost(&operations);
        assert!(cost > 0.0);
    }

    #[test]
    fn test_optimizer_hints() {
        let hints = OptimizerHints::use_index("idx_users_email");
        assert!(hints.has_hints());
        assert!(hints.use_index.is_some());

        let empty = OptimizerHints::none();
        assert!(!empty.has_hints());
    }
}
