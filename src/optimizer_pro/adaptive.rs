// Adaptive Query Execution - Runtime plan adaptation and correction
//
// Implements Oracle-like adaptive query execution with:
// - Runtime statistics feedback
// - Adaptive join methods
// - Automatic plan correction
// - Cardinality feedback loop
// - SQL Plan Directives

use std::collections::VecDeque;
use std::time::SystemTime;
use std::time::Instant;
use crate::common::{TableId, IndexId, Value};
use crate::error::Result;
use crate::optimizer_pro::{
    PhysicalPlan, PhysicalOperator, Expression, JoinType, PlanId, ExecutionResult,
};
use std::collections::{HashMap};
use std::sync::{Arc, RwLock};
use std::time::{Duration};

// ============================================================================
// Adaptive Executor
// ============================================================================

/// Adaptive query executor with runtime plan correction
pub struct AdaptiveExecutor {
    /// Runtime statistics collector
    stats_collector: Arc<RuntimeStatsCollector>,
    /// Plan corrector
    plan_corrector: Arc<PlanCorrector>,
    /// SQL Plan Directives
    plan_directives: Arc<RwLock<PlanDirectives>>,
    /// Adaptive join selector
    adaptive_join_selector: Arc<AdaptiveJoinSelector>,
    /// Cardinality feedback loop
    feedback_loop: Arc<CardinalityFeedbackLoop>,
}

impl AdaptiveExecutor {
    /// Create a new adaptive executor
    pub fn new() -> Self {
        let stats_collector = Arc::new(RuntimeStatsCollector::new());
        let plan_directives = Arc::new(RwLock::new(PlanDirectives::new()));
        let feedback_loop = Arc::new(CardinalityFeedbackLoop::new());

        Self {
            stats_collector: Arc::clone(&stats_collector),
            plan_corrector: Arc::new(PlanCorrector::new(Arc::clone(&stats_collector))),
            plan_directives,
            adaptive_join_selector: Arc::new(AdaptiveJoinSelector::new()),
            feedback_loop,
        }
    }

    /// Execute a plan with adaptive optimization
    pub fn execute(&self, plan: &PhysicalPlan) -> Result<ExecutionResult> {
        let start = Instant::now();
        let mut corrections = Vec::new();

        // Start collecting runtime statistics
        let execution_id = self.stats_collector.start_execution(plan.plan_id);

        // Execute the plan with monitoring
        let rows = self.execute_with_monitoring(plan, &execution_id, &mut corrections)?;

        // Stop collecting statistics
        let runtime_stats = self.stats_collector.stop_execution(execution_id);

        // Update cardinality feedback loop
        self.feedback_loop.record_actual_cardinality(
            plan.plan_id,
            plan.cardinality,
            rows.len(),
        );

        // Check if plan needs correction
        if self.should_correct_plan(&runtime_stats, plan)? {
            let corrected_plan = self.plan_corrector.correct_plan(plan, &runtime_stats)?;
            corrections.push(format!(
                "Plan corrected: switched from {:?} to {:?}",
                plan.operator,
                corrected_plan.operator
            ));

            // Re-execute with corrected plan
            let corrected_rows = self.execute_with_monitoring(
                &corrected_plan,
                &execution_id,
                &mut corrections,
            )?;

            return Ok(ExecutionResult {
                rows: corrected_rows,
                execution_time: start.elapsed(),
                adaptive_corrections: corrections,
            });
        }

        // Generate SQL Plan Directives if needed
        if self.should_create_directive(&runtime_stats, plan)? {
            self.create_plan_directive(plan, &runtime_stats)?;
        }

        Ok(ExecutionResult {
            rows,
            execution_time: start.elapsed(),
            adaptive_corrections: corrections,
        })
    }

    /// Execute with runtime monitoring
    #[inline]
    fn execute_with_monitoring(
        &self,
        plan: &PhysicalPlan,
        execution_id: &ExecutionId,
        corrections: &mut Vec<String>,
    ) -> Result<Vec<Vec<Value>>> {
        match &plan.operator {
            PhysicalOperator::SeqScan { table_id, filter } => {
                self.execute_seq_scan(*table_id, filter.as_ref(), execution_id)
            }
            PhysicalOperator::IndexScan {
                table_id,
                index_id,
                key_conditions,
                filter,
            } => self.execute_index_scan(
                *table_id,
                *index_id,
                key_conditions,
                filter.as_ref(),
                execution_id,
            ),
            PhysicalOperator::NestedLoopJoin {
                left,
                right,
                condition,
                join_type,
            } => {
                // Check if we should switch join method adaptively
                if let Some(better_method) = self.adaptive_join_selector.select_join_method(
                    left.cardinality,
                    right.cardinality,
                    execution_id,
                )? {
                    corrections.push(format!("Adaptive join switch: {:?}", better_method));
                }

                self.execute_nested_loop_join(left, right, condition.as_ref(), *join_type, execution_id)
            }
            PhysicalOperator::HashJoin {
                left,
                right,
                hash_keys,
                condition,
                join_type,
            } => self.execute_hash_join(left, right, hash_keys, condition.as_ref(), *join_type, execution_id),
            PhysicalOperator::MergeJoin {
                left,
                right,
                merge_keys,
                condition,
                join_type,
            } => self.execute_merge_join(left, right, merge_keys, condition.as_ref(), *join_type, execution_id),
            _ => Ok(vec![]),
        }
    }

    /// Execute sequential scan
    fn execute_seq_scan(
        &self,
        _table_id: TableId,
        _filter: Option<&Expression>,
        execution_id: &ExecutionId,
    ) -> Result<Vec<Vec<Value>>> {
        // Record operator start
        self.stats_collector.record_operator_start(execution_id, "SeqScan");

        // Simulate execution
        let rows = vec![];

        // Record operator end
        self.stats_collector.record_operator_end(execution_id, "SeqScan", rows.len());

        Ok(rows)
    }

    /// Execute index scan
    fn execute_index_scan(
        &self,
        _table_id: TableId,
        _index_id: IndexId,
        _key_conditions: &[Expression],
        _filter: Option<&Expression>,
        execution_id: &ExecutionId,
    ) -> Result<Vec<Vec<Value>>> {
        self.stats_collector.record_operator_start(execution_id, "IndexScan");

        // Simulate execution
        let rows = vec![];

        self.stats_collector.record_operator_end(execution_id, "IndexScan", rows.len());

        Ok(rows)
    }

    /// Execute nested loop join
    fn execute_nested_loop_join(
        &self,
        left: &PhysicalPlan,
        right: &PhysicalPlan,
        _condition: Option<&Expression>,
        _join_type: JoinType,
        execution_id: &ExecutionId,
    ) -> Result<Vec<Vec<Value>>> {
        self.stats_collector.record_operator_start(execution_id, "NestedLoopJoin");

        let left_rows = self.execute_with_monitoring(left, execution_id, &mut vec![])?;
        let _right_rows = self.execute_with_monitoring(right, execution_id, &mut vec![])?;

        // Record actual vs estimated cardinality
        self.stats_collector.record_cardinality_mismatch(
            execution_id,
            left.cardinality,
            left_rows.len(),
        );

        // Simulate join execution
        let rows = vec![];

        self.stats_collector.record_operator_end(execution_id, "NestedLoopJoin", rows.len());

        Ok(rows)
    }

    /// Execute hash join
    fn execute_hash_join(
        &self,
        left: &PhysicalPlan,
        right: &PhysicalPlan,
        _hash_keys: &[Expression],
        _condition: Option<&Expression>,
        _join_type: JoinType,
        execution_id: &ExecutionId,
    ) -> Result<Vec<Vec<Value>>> {
        self.stats_collector.record_operator_start(execution_id, "HashJoin");

        let left_rows = self.execute_with_monitoring(left, execution_id, &mut vec![])?;
        let _right_rows = self.execute_with_monitoring(right, execution_id, &mut vec![])?;

        // Simulate join execution
        let rows = vec![];

        self.stats_collector.record_operator_end(execution_id, "HashJoin", rows.len());

        Ok(rows)
    }

    /// Execute merge join
    fn execute_merge_join(
        &self,
        left: &PhysicalPlan,
        right: &PhysicalPlan,
        _merge_keys: &[(Expression, Expression)],
        _condition: Option<&Expression>,
        _join_type: JoinType,
        execution_id: &ExecutionId,
    ) -> Result<Vec<Vec<Value>>> {
        self.stats_collector.record_operator_start(execution_id, "MergeJoin");

        let left_rows = self.execute_with_monitoring(left, execution_id, &mut vec![])?;
        let _right_rows = self.execute_with_monitoring(right, execution_id, &mut vec![])?;

        // Simulate join execution
        let rows = vec![];

        self.stats_collector.record_operator_end(execution_id, "MergeJoin", rows.len());

        Ok(rows)
    }

    /// Check if plan should be corrected
    fn should_correct_plan(&self, runtime_stats: &RuntimeStatistics, plan: &PhysicalPlan) -> Result<bool> {
        // Check for significant cardinality misestimation
        if let Some(actual_rows) = runtime_stats.actual_rows {
            let estimated_rows = plan.cardinality;
            let error_ratio = (actual_rows as f64) / (estimated_rows as f64).max(1.0);

            // Correct if error is > 10x
            if error_ratio > 10.0 || error_ratio < 0.1 {
                return Ok(true);
            }
        }

        // Check for slow operators
        if runtime_stats.execution_time > Duration::from_secs(10) {
            return Ok(true);
        }

        Ok(false)
    }

    /// Check if should create plan directive
    fn should_create_directive(&self, runtime_stats: &RuntimeStatistics, plan: &PhysicalPlan) -> Result<bool> {
        // Create directive if cardinality misestimation is significant
        if let Some(actual_rows) = runtime_stats.actual_rows {
            let estimated_rows = plan.cardinality;
            let error_ratio = (actual_rows as f64) / (estimated_rows as f64).max(1.0);

            return Ok(error_ratio > 5.0 || error_ratio < 0.2);
        }

        Ok(false)
    }

    /// Create a plan directive
    fn create_plan_directive(&self, plan: &PhysicalPlan, runtime_stats: &RuntimeStatistics) -> Result<()> {
        let directive = PlanDirective {
            directive_id: DirectiveId(rand::random()),
            query_signature: format!("{:?}", plan.operator),
            cardinality_adjustment: if let Some(actual) = runtime_stats.actual_rows {
                (actual as f64) / (plan.cardinality as f64).max(1.0)
            } else {
                1.0
            },
            created_at: SystemTime::now(),
            usage_count: 0,
            last_used: SystemTime::now(),
        };

        self.plan_directives.write().unwrap().add_directive(directive);

        Ok(())
    }
}

// ============================================================================
// Runtime Statistics Collector
// ============================================================================

/// Runtime statistics collector
pub struct RuntimeStatsCollector {
    /// Active executions
    executions: Arc<RwLock<HashMap<ExecutionId, ExecutionStats>>>,
    /// Completed executions
    completed: Arc<RwLock<VecDeque<RuntimeStatistics>>>,
}

impl RuntimeStatsCollector {
    pub fn new() -> Self {
        Self {
            executions: Arc::new(RwLock::new(HashMap::new())),
            completed: Arc::new(RwLock::new(VecDeque::new())),
        }
    }

    /// Start execution tracking
    pub fn start_execution(&self, plan_id: PlanId) -> ExecutionId {
        let execution_id = ExecutionId(rand::random());

        let stats = ExecutionStats {
            execution_id,
            plan_id,
            start_time: Instant::now(),
            operator_stats: HashMap::new(),
            cardinality_mismatches: vec![],
        };

        self.executions.write().unwrap().insert(execution_id, stats);

        execution_id
    }

    /// Stop execution tracking
    pub fn stop_execution(&self, execution_id: ExecutionId) -> RuntimeStatistics {
        let mut executions = self.executions.write().unwrap();

        if let Some(stats) = executions.remove(&execution_id) {
            let runtime_stats = RuntimeStatistics {
                execution_id: stats.execution_id,
                plan_id: stats.plan_id,
                execution_time: stats.start_time.elapsed(),
                actual_rows: stats.operator_stats.values().map(|s| s.rows_produced).sum::<usize>().into(),
                operator_stats: stats.operator_stats,
                cardinality_mismatches: stats.cardinality_mismatches,
            };

            // Store in completed executions
            let mut completed = self.completed.write().unwrap();
            completed.push_back(runtime_stats.clone());

            // Keep only last 1000 executions
            if completed.len() > 1000 {
                completed.pop_front();
            }

            runtime_stats
        } else {
            RuntimeStatistics::default()
        }
    }

    /// Record operator start
    pub fn record_operator_start(&self, execution_id: &ExecutionId, operator: &str) {
        let mut executions = self.executions.write().unwrap();

        if let Some(stats) = executions.get_mut(execution_id) {
            stats.operator_stats.insert(
                operator.to_string(),
                OperatorStats {
                    operator_name: operator.to_string(),
                    start_time: Instant::now(),
                    end_time: None,
                    rows_produced: 0,
                },
            );
        }
    }

    /// Record operator end
    pub fn record_operator_end(&self, execution_id: &ExecutionId, operator: &str, rows_produced: usize) {
        let mut executions = self.executions.write().unwrap();

        if let Some(stats) = executions.get_mut(execution_id) {
            if let Some(op_stats) = stats.operator_stats.get_mut(operator) {
                op_stats.end_time = Some(Instant::now());
                op_stats.rows_produced = rows_produced;
            }
        }
    }

    /// Record cardinality mismatch
    pub fn record_cardinality_mismatch(
        &self,
        execution_id: &ExecutionId,
        estimated: usize,
        actual: usize,
    ) {
        let mut executions = self.executions.write().unwrap();

        if let Some(stats) = executions.get_mut(execution_id) {
            stats.cardinality_mismatches.push(CardinalityMismatch {
                estimated,
                actual,
                error_ratio: (actual as f64) / (estimated as f64).max(1.0),
            });
        }
    }

    /// Get completed executions
    pub fn get_completed_executions(&self) -> Vec<RuntimeStatistics> {
        self.completed.read().unwrap().iter().cloned().collect()
    }
}

/// Execution ID
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ExecutionId(u64);

/// Execution statistics (during execution)
#[derive(Debug, Clone)]
struct ExecutionStats {
    execution_id: ExecutionId,
    plan_id: PlanId,
    start_time: Instant,
    operator_stats: HashMap<String, OperatorStats>,
    cardinality_mismatches: Vec<CardinalityMismatch>,
}

/// Runtime statistics (after execution)
#[repr(C)]
#[derive(Debug, Clone, Default)]
pub struct RuntimeStatistics {
    pub execution_id: ExecutionId,
    pub plan_id: PlanId,
    pub execution_time: Duration,
    pub actual_rows: Option<usize>,
    pub operator_stats: HashMap<String, OperatorStats>,
    pub cardinality_mismatches: Vec<CardinalityMismatch>,
}

/// Operator statistics
#[derive(Debug, Clone)]
pub struct OperatorStats {
    pub operator_name: String,
    pub start_time: Instant,
    pub end_time: Option<Instant>,
    pub rows_produced: usize,
}

/// Cardinality mismatch record
#[derive(Debug, Clone)]
pub struct CardinalityMismatch {
    pub estimated: usize,
    pub actual: usize,
    pub error_ratio: f64,
}

impl Default for ExecutionId {
    fn default() -> Self {
        ExecutionId(0)
    }
}

impl Default for PlanId {
    fn default() -> Self {
        PlanId(0)
    }
}

// ============================================================================
// Plan Corrector
// ============================================================================

/// Plan corrector for runtime plan adaptation
pub struct PlanCorrector {
    stats_collector: Arc<RuntimeStatsCollector>,
}

impl PlanCorrector {
    pub fn new(stats_collector: Arc<RuntimeStatsCollector>) -> Self {
        Self { stats_collector }
    }

    /// Correct a plan based on runtime statistics
    pub fn correct_plan(
        &self,
        plan: &PhysicalPlan,
        runtime_stats: &RuntimeStatistics,
    ) -> Result<PhysicalPlan> {
        // Analyze cardinality mismatches
        let avg_error = self.analyze_cardinality_errors(runtime_stats);

        // Decide on correction strategy
        if avg_error > 10.0 {
            return self.switch_join_method(plan);
        }

        // Return original plan if no correction needed
        Ok(plan.clone())
    }

    /// Analyze cardinality errors
    fn analyze_cardinality_errors(&self, runtime_stats: &RuntimeStatistics) -> f64 {
        if runtime_stats.cardinality_mismatches.is_empty() {
            return 1.0;
        }

        let sum: f64 = runtime_stats
            .cardinality_mismatches
            .iter()
            .map(|m| m.error_ratio)
            .sum();

        sum / runtime_stats.cardinality_mismatches.len() as f64
    }

    /// Switch join method
    fn switch_join_method(&self, plan: &PhysicalPlan) -> Result<PhysicalPlan> {
        // Create a corrected plan with different join method
        let mut corrected = plan.clone();

        // Switch nested loop to hash join (simplified)
        if let PhysicalOperator::NestedLoopJoin {
            left,
            right,
            condition,
            join_type,
        } = &plan.operator
        {
            corrected.operator = PhysicalOperator::HashJoin {
                left: left.clone(),
                right: right.clone(),
                hash_keys: vec![],
                condition: condition.clone(),
                join_type: *join_type,
            };
        }

        Ok(corrected)
    }
}

// ============================================================================
// Adaptive Join Selector
// ============================================================================

/// Adaptive join method selector
pub struct AdaptiveJoinSelector {
    /// Join method performance history
    performance_history: Arc<RwLock<HashMap<String, JoinMethodPerformance>>>,
}

impl AdaptiveJoinSelector {
    pub fn new() -> Self {
        Self {
            performance_history: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Select best join method based on runtime information
    pub fn select_join_method(
        &self,
        left_cardinality: usize,
        right_cardinality: usize,
        _execution_id: &ExecutionId,
    ) -> Result<Option<JoinMethod>> {
        // Heuristics for join method selection
        if left_cardinality < 1000 || right_cardinality < 1000 {
            return Ok(Some(JoinMethod::NestedLoop));
        }

        if left_cardinality > 100000 && right_cardinality > 100000 {
            return Ok(Some(JoinMethod::Hash));
        }

        // Check performance history
        let key = format!("{}_{}", left_cardinality / 1000, right_cardinality / 1000);
        if let Some(perf) = self.performance_history.read().unwrap().get(&key) {
            return Ok(Some(perf.best_method));
        }

        Ok(None)
    }

    /// Record join performance
    pub fn record_performance(
        &self,
        left_cardinality: usize,
        right_cardinality: usize,
        method: JoinMethod,
        execution_time: Duration,
    ) {
        let key = format!("{}_{}", left_cardinality / 1000, right_cardinality / 1000);

        let mut history = self.performance_history.write().unwrap();

        let perf = history.entry(key).or_insert(JoinMethodPerformance {
            nested_loop_time: None,
            hash_join_time: None,
            merge_join_time: None,
            best_method: method,
        });

        match method {
            JoinMethod::NestedLoop => perf.nested_loop_time = Some(execution_time),
            JoinMethod::Hash => perf.hash_join_time = Some(execution_time),
            JoinMethod::Merge => perf.merge_join_time = Some(execution_time),
        }

        // Update best method
        perf.best_method = perf.find_best_method();
    }
}

/// Join method
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JoinMethod {
    NestedLoop,
    Hash,
    Merge,
}

/// Join method performance
#[derive(Debug, Clone)]
struct JoinMethodPerformance {
    nested_loop_time: Option<Duration>,
    hash_join_time: Option<Duration>,
    merge_join_time: Option<Duration>,
    best_method: JoinMethod,
}

impl JoinMethodPerformance {
    fn find_best_method(&self) -> JoinMethod {
        let mut best = JoinMethod::Hash;
        let mut best_time = Duration::from_secs(u64::MAX);

        if let Some(time) = self.nested_loop_time {
            if time < best_time {
                best = JoinMethod::NestedLoop;
                best_time = time;
            }
        }

        if let Some(time) = self.hash_join_time {
            if time < best_time {
                best = JoinMethod::Hash;
                best_time = time;
            }
        }

        if let Some(time) = self.merge_join_time {
            if time < best_time {
                best = JoinMethod::Merge;
            }
        }

        best
    }
}

// ============================================================================
// SQL Plan Directives
// ============================================================================

/// SQL Plan Directives manager
#[derive(Debug)]
pub struct PlanDirectives {
    directives: HashMap<DirectiveId, PlanDirective>,
}

impl PlanDirectives {
    pub fn new() -> Self {
        Self {
            directives: HashMap::new(),
        }
    }

    /// Add a plan directive
    pub fn add_directive(&mut self, directive: PlanDirective) {
        self.directives.insert(directive.directive_id, directive);
    }

    /// Get directive for query
    pub fn get_directive(&self, query_signature: &str) -> Option<&PlanDirective> {
        self.directives
            .values()
            .find(|d| d.query_signature == query_signature)
    }

    /// Prune old directives
    pub fn prune_old_directives(&mut self, max_age: Duration) {
        let now = SystemTime::now();
        self.directives.retain(|_, directive| {
            if let Ok(age) = now.duration_since(directive.created_at) {
                age < max_age
            } else {
                true
            }
        });
    }
}

/// Directive ID
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct DirectiveId(u64);

/// Plan directive
#[derive(Debug, Clone)]
pub struct PlanDirective {
    pub directive_id: DirectiveId,
    pub query_signature: String,
    pub cardinality_adjustment: f64,
    pub created_at: SystemTime,
    pub usage_count: u64,
    pub last_used: SystemTime,
}

// ============================================================================
// Cardinality Feedback Loop
// ============================================================================

/// Cardinality feedback loop
pub struct CardinalityFeedbackLoop {
    /// Cardinality predictions
    predictions: Arc<RwLock<HashMap<PlanId, CardinalityPrediction>>>,
}

impl CardinalityFeedbackLoop {
    pub fn new() -> Self {
        Self {
            predictions: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Record actual cardinality
    pub fn record_actual_cardinality(
        &self,
        plan_id: PlanId,
        estimated: usize,
        actual: usize,
    ) {
        let mut predictions = self.predictions.write().unwrap();

        let prediction = predictions.entry(plan_id).or_insert(CardinalityPrediction {
            plan_id,
            total_estimated: 0,
            total_actual: 0,
            executions: 0,
        });

        prediction.total_estimated += estimated;
        prediction.total_actual += actual;
        prediction.executions += 1;
    }

    /// Get cardinality adjustment factor
    pub fn get_adjustment_factor(&self, plan_id: PlanId) -> f64 {
        let predictions = self.predictions.read().unwrap();

        if let Some(prediction) = predictions.get(&plan_id) {
            if prediction.executions > 0 && prediction.total_estimated > 0 {
                return (prediction.total_actual as f64) / (prediction.total_estimated as f64);
            }
        }

        1.0
    }
}

/// Cardinality prediction
#[derive(Debug, Clone)]
struct CardinalityPrediction {
    plan_id: PlanId,
    total_estimated: usize,
    total_actual: usize,
    executions: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_runtime_stats_collector() {
        let collector = RuntimeStatsCollector::new();
        let plan_id = PlanId(1);

        let execution_id = collector.start_execution(plan_id);
        collector.record_operator_start(&execution_id, "SeqScan");
        collector.record_operator_end(&execution_id, "SeqScan", 100);

        let stats = collector.stop_execution(execution_id);
        assert_eq!(stats.plan_id, plan_id);
    }

    #[test]
    fn test_cardinality_feedback_loop() {
        let feedback = CardinalityFeedbackLoop::new();
        let plan_id = PlanId(1);

        feedback.record_actual_cardinality(plan_id, 100, 200);
        feedback.record_actual_cardinality(plan_id, 100, 200);

        let adjustment = feedback.get_adjustment_factor(plan_id);
        assert!((adjustment - 2.0).abs() < 0.01);
    }

    #[test]
    fn test_adaptive_join_selector() {
        let selector = AdaptiveJoinSelector::new();

        let method = selector.select_join_method(500, 500, &ExecutionId(1));
        assert!(method.is_ok());

        selector.record_performance(
            1000,
            1000,
            JoinMethod::Hash,
            Duration::from_millis(100),
        );
    }
}
