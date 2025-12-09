// Query Optimizer Module
//
// This module provides cost-based query optimization with advanced techniques
// including memoization, CSE, view matching, and dynamic programming join enumeration.
//
// Structure:
// - cost_model: Statistics and cardinality estimation
// - plan_transformation: Advanced optimization techniques (CSE, memoization, DPccp)
// - rules: Core optimizer and basic transformation rules

pub mod cost_model;
pub mod plan_transformation;
pub mod rules;

// Re-export main types
pub use cost_model::{
    CardinalityEstimator, ColumnStatistics, Histogram, HistogramBucket, HistogramType,
    IndexStatistics, SingleTableStatistics, TableStatistics,
};

pub use plan_transformation::{
    AdaptiveStatistics, CardinalityError, EquivalenceClass, ExpressionHash, ExecutionFeedback,
    MaterializedView, MemoTable, BitSet,
};

pub use rules::{JoinOrderingStrategy, Optimizer};

#[cfg(test)]
mod tests {
    use super::*;
    use crate::execution::planner::PlanNode;
    use crate::parser::JoinType;

    #[test]
    fn test_optimizer() {
        let optimizer = Optimizer::new();
        let plan = PlanNode::TableScan {
            table: "users".to_string(),
            columns: vec!["id".to_string()],
        };

        let optimized = optimizer.optimize(plan).unwrap();
        matches!(optimized, PlanNode::TableScan { .. });
    }

    #[test]
    fn test_cardinality_estimation() {
        let optimizer = Optimizer::new();

        // Add some statistics
        let mut table_stats = SingleTableStatistics::new(10000, 100);
        table_stats.add_column_stats(
            "id".to_string(),
            ColumnStatistics::new(10000, 0),
        );

        optimizer.update_statistics("users".to_string(), table_stats);

        let plan = PlanNode::TableScan {
            table: "users".to_string(),
            columns: vec!["id".to_string()],
        };

        let card = optimizer.estimate_cardinality(&plan);
        assert_eq!(card, 10000.0);
    }

    #[test]
    fn test_join_cost_estimation() {
        let optimizer = Optimizer::new();

        let left = PlanNode::TableScan {
            table: "users".to_string(),
            columns: vec!["id".to_string()],
        };

        let right = PlanNode::TableScan {
            table: "orders".to_string(),
            columns: vec!["user_id".to_string()],
        };

        let cost = optimizer.estimate_join_cost(&left, &right, &JoinType::Inner);
        assert!(cost > 0.0);
    }
}
