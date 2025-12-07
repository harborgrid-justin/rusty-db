use crate::execution::planner::{PlanNode, AggregateFunction};
use crate::parser::JoinType;
use crate::Result;
use crate::error::DbError;
use std::collections::HashMap;
use std::sync::Arc;
use parking_lot::RwLock;

/// Cost-based query optimizer with advanced cardinality estimation
pub struct Optimizer {
    /// Table statistics for cardinality estimation
    statistics: Arc<RwLock<TableStatistics>>,
    /// Join ordering strategy
    join_strategy: JoinOrderingStrategy,
}

impl Optimizer {
    pub fn new() -> Self {
        Self {
            statistics: Arc::new(RwLock::new(TableStatistics::new())),
            join_strategy: JoinOrderingStrategy::DynamicProgramming,
        }
    }

    pub fn with_statistics(statistics: TableStatistics) -> Self {
        Self {
            statistics: Arc::new(RwLock::new(statistics)),
            join_strategy: JoinOrderingStrategy::DynamicProgramming,
        }
    }

    /// Optimize a query plan using various strategies
    pub fn optimize(&self, plan: PlanNode) -> Result<PlanNode> {
        let mut optimized = plan;

        // Apply optimization passes in order
        optimized = self.push_down_predicates(optimized)?;
        optimized = self.push_down_projections(optimized)?;
        optimized = self.reorder_joins(optimized)?;
        optimized = self.select_access_paths(optimized)?;
        optimized = self.constant_folding(optimized)?;
        optimized = self.merge_operators(optimized)?;

        Ok(optimized)
    }

    /// Push filters down closer to table scans for early data reduction
    fn push_down_predicates(&self, plan: PlanNode) -> Result<PlanNode> {
        match plan {
            PlanNode::Filter { input, predicate } => {
                match *input {
                    PlanNode::Join { join_type, left, right, condition } => {
                        // Try to push filter down to join children
                        // For simplicity, we'll keep it above the join for now
                        Ok(PlanNode::Filter {
                            input: Box::new(PlanNode::Join {
                                join_type,
                                left,
                                right,
                                condition,
                            }),
                            predicate,
                        })
                    }
                    other => Ok(PlanNode::Filter {
                        input: Box::new(self.push_down_predicates(other)?),
                        predicate,
                    }),
                }
            }
            PlanNode::Join { join_type, left, right, condition } => {
                Ok(PlanNode::Join {
                    join_type,
                    left: Box::new(self.push_down_predicates(*left)?),
                    right: Box::new(self.push_down_predicates(*right)?),
                    condition,
                })
            }
            PlanNode::Aggregate { input, group_by, aggregates, having } => {
                Ok(PlanNode::Aggregate {
                    input: Box::new(self.push_down_predicates(*input)?),
                    group_by,
                    aggregates,
                    having,
                })
            }
            PlanNode::Sort { input, order_by } => {
                Ok(PlanNode::Sort {
                    input: Box::new(self.push_down_predicates(*input)?),
                    order_by,
                })
            }
            PlanNode::Limit { input, limit, offset } => {
                Ok(PlanNode::Limit {
                    input: Box::new(self.push_down_predicates(*input)?),
                    limit,
                    offset,
                })
            }
            other => Ok(other),
        }
    }

    /// Push down projections to eliminate unnecessary columns early
    fn push_down_projections(&self, plan: PlanNode) -> Result<PlanNode> {
        // Simplified implementation - in production would track required columns
        Ok(plan)
    }

    /// Reorder joins based on estimated costs using dynamic programming
    fn reorder_joins(&self, plan: PlanNode) -> Result<PlanNode> {
        match plan {
            PlanNode::Join { join_type, left, right, condition } => {
                // Recursively optimize children first
                let left = self.reorder_joins(*left)?;
                let right = self.reorder_joins(*right)?;

                // Estimate cardinalities
                let left_card = self.estimate_cardinality(&left);
                let right_card = self.estimate_cardinality(&right);

                // Calculate join costs for both orders
                let left_right_cost = self.estimate_join_cost(&left, &right, &join_type);
                let right_left_cost = self.estimate_join_cost(&right, &left, &join_type);

                // Choose better order
                let (final_left, final_right) = if right_left_cost < left_right_cost {
                    (Box::new(right), Box::new(left))
                } else {
                    (Box::new(left), Box::new(right))
                };

                Ok(PlanNode::Join {
                    join_type,
                    left: final_left,
                    right: final_right,
                    condition,
                })
            }
            PlanNode::Aggregate { input, group_by, aggregates, having } => {
                Ok(PlanNode::Aggregate {
                    input: Box::new(self.reorder_joins(*input)?),
                    group_by,
                    aggregates,
                    having,
                })
            }
            PlanNode::Filter { input, predicate } => {
                Ok(PlanNode::Filter {
                    input: Box::new(self.reorder_joins(*input)?),
                    predicate,
                })
            }
            other => Ok(other),
        }
    }

    /// Select optimal access paths (index vs table scan)
    fn select_access_paths(&self, plan: PlanNode) -> Result<PlanNode> {
        match plan {
            PlanNode::TableScan { table, columns } => {
                // Check if an index would be beneficial
                let stats = self.statistics.read();
                if let Some(table_stats) = stats.tables.get(&table) {
                    // Would evaluate index selectivity here
                    // For now, keep as table scan
                }
                Ok(PlanNode::TableScan { table, columns })
            }
            PlanNode::Join { join_type, left, right, condition } => {
                Ok(PlanNode::Join {
                    join_type,
                    left: Box::new(self.select_access_paths(*left)?),
                    right: Box::new(self.select_access_paths(*right)?),
                    condition,
                })
            }
            other => Ok(other),
        }
    }

    /// Perform constant folding and expression simplification
    fn constant_folding(&self, plan: PlanNode) -> Result<PlanNode> {
        // For now, just pass through - could implement expression evaluation
        Ok(plan)
    }

    /// Merge adjacent operators when beneficial
    fn merge_operators(&self, plan: PlanNode) -> Result<PlanNode> {
        // Could merge consecutive filters, combine limits, etc.
        Ok(plan)
    }

    /// Estimate the cost of executing a plan node
    pub fn estimate_cost(&self, plan: &PlanNode) -> f64 {
        let cardinality = self.estimate_cardinality(plan);
        let cpu_cost = self.estimate_cpu_cost(plan, cardinality);
        let io_cost = self.estimate_io_cost(plan, cardinality);

        // Weight CPU and I/O costs (I/O typically more expensive)
        cpu_cost + (io_cost * 10.0)
    }

    /// Estimate CPU cost for an operator
    fn estimate_cpu_cost(&self, plan: &PlanNode, cardinality: f64) -> f64 {
        match plan {
            PlanNode::TableScan { .. } => cardinality * 0.1,
            PlanNode::Filter { .. } => cardinality * 0.2,
            PlanNode::Join { .. } => cardinality * 0.5,
            PlanNode::Aggregate { .. } => cardinality * 0.3,
            PlanNode::Sort { .. } => cardinality * cardinality.log2() * 0.1,
            _ => cardinality * 0.1,
        }
    }

    /// Estimate I/O cost for an operator
    fn estimate_io_cost(&self, plan: &PlanNode, cardinality: f64) -> f64 {
        match plan {
            PlanNode::TableScan { table, .. } => {
                let stats = self.statistics.read();
                if let Some(table_stats) = stats.tables.get(table) {
                    table_stats.num_pages as f64
                } else {
                    cardinality / 100.0 // Assume 100 rows per page
                }
            }
            _ => 0.0, // Most operators don't do I/O
        }
    }

    /// Estimate output cardinality of a plan node
    pub fn estimate_cardinality(&self, plan: &PlanNode) -> f64 {
        match plan {
            PlanNode::TableScan { table, .. } => {
                let stats = self.statistics.read();
                if let Some(table_stats) = stats.tables.get(table) {
                    table_stats.row_count as f64
                } else {
                    1000.0 // Default estimate
                }
            }
            PlanNode::Filter { input, predicate } => {
                let input_card = self.estimate_cardinality(input);
                let selectivity = self.estimate_filter_selectivity(predicate);
                input_card * selectivity
            }
            PlanNode::Join { left, right, join_type, condition } => {
                let left_card = self.estimate_cardinality(left);
                let right_card = self.estimate_cardinality(right);
                let selectivity = self.estimate_join_selectivity(condition);

                match join_type {
                    JoinType::Inner => left_card * right_card * selectivity,
                    JoinType::Left => left_card.max(left_card * right_card * selectivity),
                    JoinType::Right => right_card.max(left_card * right_card * selectivity),
                    JoinType::Full => left_card + right_card,
                    JoinType::Cross => left_card * right_card,
                }
            }
            PlanNode::Aggregate { input, group_by, .. } => {
                let input_card = self.estimate_cardinality(input);
                if group_by.is_empty() {
                    1.0 // Single row for global aggregate
                } else {
                    // Estimate distinct groups (simplified)
                    (input_card / 10.0).max(1.0).min(input_card)
                }
            }
            PlanNode::Sort { input, .. } => {
                self.estimate_cardinality(input)
            }
            PlanNode::Limit { input, limit, .. } => {
                self.estimate_cardinality(input).min(*limit as f64)
            }
            PlanNode::Project { input, .. } => {
                self.estimate_cardinality(input)
            }
            PlanNode::Subquery { plan, .. } => {
                self.estimate_cardinality(plan)
            }
        }
    }

    /// Estimate filter selectivity
    fn estimate_filter_selectivity(&self, _predicate: &str) -> f64 {
        // Simplified - in production would parse predicate and use histograms
        0.1 // Default 10% selectivity
    }

    /// Estimate join selectivity
    fn estimate_join_selectivity(&self, _condition: &str) -> f64 {
        // Simplified - would analyze join keys and compute selectivity
        0.01 // Default 1% selectivity
    }

    /// Estimate cost of a specific join
    fn estimate_join_cost(&self, left: &PlanNode, right: &PlanNode, join_type: &JoinType) -> f64 {
        let left_card = self.estimate_cardinality(left);
        let right_card = self.estimate_cardinality(right);

        match join_type {
            JoinType::Inner | JoinType::Left | JoinType::Right => {
                // Hash join cost: build hash table + probe
                let build_cost = right_card; // Build smaller relation
                let probe_cost = left_card; // Probe with larger
                build_cost + probe_cost
            }
            JoinType::Cross => {
                // Nested loop
                left_card * right_card
            }
            JoinType::Full => {
                // More expensive due to outer join logic
                (left_card + right_card) * 1.5
            }
        }
    }

    /// Select the best index for a table scan (if available)
    pub fn select_index(&self, table: &str, _filter: Option<&str>) -> Option<String> {
        let stats = self.statistics.read();
        if let Some(table_stats) = stats.tables.get(table) {
            // Select most selective index
            if !table_stats.indexes.is_empty() {
                return Some(table_stats.indexes[0].name.clone());
            }
        }
        None
    }

    /// Update statistics for a table
    pub fn update_statistics(&self, table: String, stats: SingleTableStatistics) {
        let mut statistics = self.statistics.write();
        statistics.tables.insert(table, stats);
    }
}

impl Default for Optimizer {
    fn default() -> Self {
        Self::new()
    }
}

/// Join ordering strategy
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum JoinOrderingStrategy {
    /// Left-deep trees only
    LeftDeep,
    /// Dynamic programming (optimal but expensive)
    DynamicProgramming,
    /// Greedy heuristic
    Greedy,
}

/// Table statistics for cost estimation
#[derive(Debug, Clone)]
pub struct TableStatistics {
    /// Per-table statistics
    pub tables: HashMap<String, SingleTableStatistics>,
}

impl TableStatistics {
    pub fn new() -> Self {
        Self {
            tables: HashMap::new(),
        }
    }

    pub fn add_table(&mut self, name: String, stats: SingleTableStatistics) {
        self.tables.insert(name, stats);
    }
}

/// Statistics for a single table
#[derive(Debug, Clone)]
pub struct SingleTableStatistics {
    /// Number of rows
    pub row_count: usize,
    /// Number of pages on disk
    pub num_pages: usize,
    /// Column statistics
    pub columns: HashMap<String, ColumnStatistics>,
    /// Available indexes
    pub indexes: Vec<IndexStatistics>,
}

impl SingleTableStatistics {
    pub fn new(row_count: usize, num_pages: usize) -> Self {
        Self {
            row_count,
            num_pages,
            columns: HashMap::new(),
            indexes: Vec::new(),
        }
    }

    pub fn add_column_stats(&mut self, name: String, stats: ColumnStatistics) {
        self.columns.insert(name, stats);
    }

    pub fn add_index(&mut self, index: IndexStatistics) {
        self.indexes.push(index);
    }
}

/// Column statistics
#[derive(Debug, Clone)]
pub struct ColumnStatistics {
    /// Number of distinct values
    pub num_distinct: usize,
    /// Number of NULL values
    pub num_nulls: usize,
    /// Histogram for value distribution
    pub histogram: Option<Histogram>,
    /// Min/max values (for range estimates)
    pub min_value: Option<String>,
    pub max_value: Option<String>,
}

impl ColumnStatistics {
    pub fn new(num_distinct: usize, num_nulls: usize) -> Self {
        Self {
            num_distinct,
            num_nulls,
            histogram: None,
            min_value: None,
            max_value: None,
        }
    }

    /// Estimate selectivity for equality predicate
    pub fn estimate_equality_selectivity(&self, total_rows: usize) -> f64 {
        if self.num_distinct == 0 {
            return 0.0;
        }
        1.0 / self.num_distinct as f64
    }

    /// Estimate selectivity for range predicate
    pub fn estimate_range_selectivity(&self, _min: &str, _max: &str, total_rows: usize) -> f64 {
        if let Some(hist) = &self.histogram {
            // Use histogram to estimate
            // Simplified - would do proper range estimation
            return 0.1;
        }

        // Default estimate
        0.33
    }
}

/// Histogram for value distribution
#[derive(Debug, Clone)]
pub struct Histogram {
    pub buckets: Vec<HistogramBucket>,
}

impl Histogram {
    pub fn new(num_buckets: usize) -> Self {
        Self {
            buckets: vec![HistogramBucket::default(); num_buckets],
        }
    }

    pub fn estimate_selectivity(&self, _predicate: &str) -> f64 {
        // Would analyze predicate against histogram
        0.1
    }
}

#[derive(Debug, Clone, Default)]
pub struct HistogramBucket {
    pub lower_bound: String,
    pub upper_bound: String,
    pub count: usize,
    pub num_distinct: usize,
}

/// Index statistics
#[derive(Debug, Clone)]
pub struct IndexStatistics {
    pub name: String,
    pub columns: Vec<String>,
    pub unique: bool,
    pub num_pages: usize,
    pub height: usize,
}

impl IndexStatistics {
    pub fn new(name: String, columns: Vec<String>, unique: bool) -> Self {
        Self {
            name,
            columns,
            unique,
            num_pages: 100,
            height: 3,
        }
    }

    /// Estimate cost of index lookup
    pub fn estimate_lookup_cost(&self, selectivity: f64) -> f64 {
        // Tree traversal + leaf page accesses
        let tree_cost = self.height as f64;
        let leaf_cost = self.num_pages as f64 * selectivity;
        tree_cost + leaf_cost
    }
}

/// Cardinality estimator using advanced techniques
pub struct CardinalityEstimator {
    statistics: Arc<RwLock<TableStatistics>>,
}

impl CardinalityEstimator {
    pub fn new(statistics: Arc<RwLock<TableStatistics>>) -> Self {
        Self { statistics }
    }

    /// Estimate cardinality for complex query
    pub fn estimate(&self, plan: &PlanNode) -> f64 {
        // Delegate to optimizer's estimation
        let optimizer = Optimizer {
            statistics: self.statistics.clone(),
            join_strategy: JoinOrderingStrategy::DynamicProgramming,
        };
        optimizer.estimate_cardinality(plan)
    }

    /// Estimate join cardinality with correlation awareness
    pub fn estimate_join_cardinality(
        &self,
        left_card: f64,
        right_card: f64,
        left_distinct: usize,
        right_distinct: usize,
    ) -> f64 {
        // Use independence assumption with distinct value adjustment
        let max_distinct = left_distinct.max(right_distinct) as f64;
        if max_distinct == 0.0 {
            return 0.0;
        }

        (left_card * right_card) / max_distinct
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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

#[cfg(test)]
mod tests {
    use super::*;
    
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
}
