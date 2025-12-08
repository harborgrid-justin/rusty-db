//! # Query Optimizer Pro - Advanced Cost-Based Query Optimization
//!
//! Oracle-like query optimizer with advanced cost-based optimization, adaptive execution,
//! SQL plan management, and machine learning-based cardinality estimation.
//!
//! ## Architecture Overview
//!
//! The optimizer follows a multi-phase approach:
//!
//! 1. **Query Transformation**: Rewrites queries for better performance
//! 2. **Plan Generation**: Generates multiple candidate plans using dynamic programming
//! 3. **Cost Estimation**: Estimates costs using sophisticated cost models
//! 4. **Plan Selection**: Selects the best plan based on cost estimates
//! 5. **Adaptive Execution**: Monitors and adapts plans at runtime
//! 6. **Plan Management**: Manages plan baselines for stability
//!
//! ## Key Features
//!
//! - **Cost-Based Optimization**: CPU, I/O, network, and memory cost modeling
//! - **Cardinality Estimation**: Histogram-based with ML enhancement
//! - **Join Enumeration**: Bushy, left-deep, and right-deep tree generation
//! - **Adaptive Execution**: Runtime plan correction and statistics feedback
//! - **Plan Baselines**: Stable plan guarantees with evolution
//! - **Query Transformations**: Predicate pushdown, view merging, subquery unnesting
//! - **Hints System**: Oracle-compatible optimizer hints
//!
//! ## Example Usage
//!
//! ```rust,no_run
//! use rusty_db::optimizer_pro::{QueryOptimizer, OptimizerConfig};
//! use rusty_db::parser::Query;
//!
//! # fn example() -> rusty_db::Result<()> {
//! let config = OptimizerConfig::default();
//! let optimizer = QueryOptimizer::new(config);
//!
//! // Parse and optimize a query
//! let query = Query::parse("SELECT * FROM users WHERE age > 25")?;
//! let plan = optimizer.optimize(&query)?;
//!
//! // Execute with adaptive monitoring
//! let result = optimizer.execute_adaptive(&plan)?;
//! # Ok(())
//! # }
//! ```

use crate::common::{TableId, IndexId, Value, Schema};
use crate::error::{DbError, Result};
use std::collections::{HashMap, BTreeMap, HashSet, VecDeque};
use std::sync::{Arc, RwLock, Mutex};
use std::time::{Duration, Instant, SystemTime};

// Submodules
pub mod cost_model;
pub mod plan_generator;
pub mod adaptive;
pub mod plan_baselines;
pub mod transformations;
pub mod hints;

// Re-exports
pub use cost_model::{CostModel, CostEstimate, CardinalityEstimator, Histogram};
pub use plan_generator::{PlanGenerator, JoinEnumerator, AccessPathSelector};
pub use adaptive::{AdaptiveExecutor, RuntimeStatistics, PlanCorrector};
pub use plan_baselines::{PlanBaselineManager, SqlPlanBaseline, PlanHistory};
pub use transformations::{QueryTransformer, TransformationRule};
pub use hints::{HintParser, OptimizerHint, HintValidator};

// ============================================================================
// Core Types
// ============================================================================

/// Unique identifier for a query
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct QueryId(pub u64);

/// Unique identifier for a plan
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PlanId(pub u64);

/// Query fingerprint for plan caching
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct QueryFingerprint {
    /// Normalized query text
    pub normalized_text: String,
    /// Parameter types
    pub param_types: Vec<String>,
    /// Schema version
    pub schema_version: u64,
}

impl QueryFingerprint {
    /// Create a new query fingerprint
    pub fn new(query_text: &str, param_types: Vec<String>, schema_version: u64) -> Self {
        Self {
            normalized_text: Self::normalize_query(query_text),
            param_types,
            schema_version,
        }
    }

    /// Normalize query text by removing whitespace and literals
    fn normalize_query(text: &str) -> String {
        // Simple normalization - in production this would be more sophisticated
        text.to_lowercase()
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" ")
    }
}

/// Physical operator types
#[derive(Debug, Clone, PartialEq)]
pub enum PhysicalOperator {
    /// Sequential scan
    SeqScan {
        table_id: TableId,
        filter: Option<Expression>,
    },
    /// Index scan
    IndexScan {
        table_id: TableId,
        index_id: IndexId,
        key_conditions: Vec<Expression>,
        filter: Option<Expression>,
    },
    /// Index-only scan
    IndexOnlyScan {
        index_id: IndexId,
        key_conditions: Vec<Expression>,
        filter: Option<Expression>,
    },
    /// Bitmap heap scan
    BitmapHeapScan {
        table_id: TableId,
        bitmap_index_scans: Vec<IndexId>,
        filter: Option<Expression>,
    },
    /// Nested loop join
    NestedLoopJoin {
        left: Box<PhysicalPlan>,
        right: Box<PhysicalPlan>,
        condition: Option<Expression>,
        join_type: JoinType,
    },
    /// Hash join
    HashJoin {
        left: Box<PhysicalPlan>,
        right: Box<PhysicalPlan>,
        hash_keys: Vec<Expression>,
        condition: Option<Expression>,
        join_type: JoinType,
    },
    /// Merge join
    MergeJoin {
        left: Box<PhysicalPlan>,
        right: Box<PhysicalPlan>,
        merge_keys: Vec<(Expression, Expression)>,
        condition: Option<Expression>,
        join_type: JoinType,
    },
    /// Sort
    Sort {
        input: Box<PhysicalPlan>,
        sort_keys: Vec<SortKey>,
    },
    /// Aggregation
    Aggregate {
        input: Box<PhysicalPlan>,
        group_by: Vec<Expression>,
        aggregates: Vec<AggregateFunction>,
    },
    /// Hash aggregation
    HashAggregate {
        input: Box<PhysicalPlan>,
        group_by: Vec<Expression>,
        aggregates: Vec<AggregateFunction>,
    },
    /// Materialize
    Materialize {
        input: Box<PhysicalPlan>,
    },
    /// Limit
    Limit {
        input: Box<PhysicalPlan>,
        limit: usize,
        offset: usize,
    },
    /// Subquery scan
    SubqueryScan {
        subquery: Box<PhysicalPlan>,
        alias: String,
    },
}

/// Physical execution plan
#[repr(C)]
#[derive(Debug, Clone, PartialEq)]
pub struct PhysicalPlan {
    /// Plan identifier
    pub plan_id: PlanId,
    /// Root operator
    pub operator: PhysicalOperator,
    /// Estimated cost
    pub cost: f64,
    /// Estimated cardinality
    pub cardinality: usize,
    /// Output schema
    pub schema: Schema,
    /// Plan metadata
    pub metadata: PlanMetadata,
}

/// Plan metadata
#[repr(C)]
#[derive(Debug, Clone, PartialEq)]
pub struct PlanMetadata {
    /// Creation timestamp
    pub created_at: SystemTime,
    /// Optimizer version
    pub optimizer_version: String,
    /// Applied hints
    pub hints: Vec<OptimizerHint>,
    /// Applied transformations
    pub transformations: Vec<String>,
    /// Is plan from baseline
    pub from_baseline: bool,
}

/// Join type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JoinType {
    Inner,
    Left,
    Right,
    Full,
    Semi,
    Anti,
    Cross,
}

/// Sort key
#[derive(Debug, Clone, PartialEq)]
pub struct SortKey {
    pub expression: Expression,
    pub ascending: bool,
    pub nulls_first: bool,
}

/// Aggregate function
#[derive(Debug, Clone, PartialEq)]
pub struct AggregateFunction {
    pub function: AggregateFunctionType,
    pub arguments: Vec<Expression>,
    pub distinct: bool,
    pub filter: Option<Expression>,
}

/// Aggregate function type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AggregateFunctionType {
    Count,
    Sum,
    Avg,
    Min,
    Max,
    StdDev,
    Variance,
    First,
    Last,
}

/// Expression representation
#[derive(Debug, Clone, PartialEq)]
pub enum Expression {
    Column { table: String, column: String },
    Literal(Value),
    BinaryOp { op: BinaryOperator, left: Box<Expression>, right: Box<Expression> },
    UnaryOp { op: UnaryOperator, expr: Box<Expression> },
    Function { name: String, args: Vec<Expression> },
    Cast { expr: Box<Expression>, target_type: String },
    Case { conditions: Vec<(Expression, Expression)>, else_expr: Option<Box<Expression>> },
    In { expr: Box<Expression>, list: Vec<Expression> },
    Between { expr: Box<Expression>, low: Box<Expression>, high: Box<Expression> },
    IsNull(Box<Expression>),
    IsNotNull(Box<Expression>),
}

/// Binary operators
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinaryOperator {
    Add,
    Subtract,
    Multiply,
    Divide,
    Modulo,
    Equal,
    NotEqual,
    LessThan,
    LessThanOrEqual,
    GreaterThan,
    GreaterThanOrEqual,
    And,
    Or,
    Like,
    NotLike,
}

/// Unary operators
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnaryOperator {
    Not,
    Negate,
    Abs,
    Upper,
    Lower,
}

// ============================================================================
// Optimizer Configuration
// ============================================================================

/// Optimizer configuration
#[derive(Debug, Clone)]
pub struct OptimizerConfig {
    /// Enable cost-based optimization
    pub enable_cost_based: bool,
    /// Enable adaptive query execution
    pub enable_adaptive: bool,
    /// Enable plan baselines
    pub enable_plan_baselines: bool,
    /// Enable query transformations
    pub enable_transformations: bool,
    /// Maximum join order combinations to consider
    pub max_join_combinations: usize,
    /// Timeout for optimization
    pub optimization_timeout: Duration,
    /// Enable parallel plan search
    pub enable_parallel_search: bool,
    /// Enable ML-based cardinality estimation
    pub enable_ml_cardinality: bool,
    /// Cost model parameters
    pub cost_params: CostParameters,
    /// Transformation rules to apply
    pub transformation_rules: Vec<String>,
}

impl Default for OptimizerConfig {
    fn default() -> Self {
        Self {
            enable_cost_based: true,
            enable_adaptive: true,
            enable_plan_baselines: true,
            enable_transformations: true,
            max_join_combinations: 10000,
            optimization_timeout: Duration::from_secs(30),
            enable_parallel_search: true,
            enable_ml_cardinality: true,
            cost_params: CostParameters::default(),
            transformation_rules: vec![
                "predicate_pushdown".to_string(),
                "join_predicate_pushdown".to_string(),
                "subquery_unnesting".to_string(),
                "view_merging".to_string(),
                "common_subexpression_elimination".to_string(),
            ],
        }
    }
}

/// Cost model parameters
#[derive(Debug, Clone)]
pub struct CostParameters {
    /// CPU cost per tuple processed
    pub cpu_tuple_cost: f64,
    /// CPU cost per operator call
    pub cpu_operator_cost: f64,
    /// Sequential I/O cost per page
    pub seq_page_cost: f64,
    /// Random I/O cost per page
    pub random_page_cost: f64,
    /// Network cost per tuple
    pub network_tuple_cost: f64,
    /// Memory cost per MB
    pub memory_mb_cost: f64,
    /// Parallel tuple cost factor
    pub parallel_tuple_cost: f64,
    /// Parallel setup cost
    pub parallel_setup_cost: f64,
}

impl Default for CostParameters {
    fn default() -> Self {
        Self {
            cpu_tuple_cost: 0.01,
            cpu_operator_cost: 0.0025,
            seq_page_cost: 1.0,
            random_page_cost: 4.0,
            network_tuple_cost: 0.1,
            memory_mb_cost: 0.001,
            parallel_tuple_cost: 0.1,
            parallel_setup_cost: 1000.0,
        }
    }
}

// ============================================================================
// Query Optimizer
// ============================================================================

/// Main query optimizer
pub struct QueryOptimizer {
    /// Optimizer configuration
    config: OptimizerConfig,
    /// Cost model
    cost_model: Arc<CostModel>,
    /// Plan generator
    plan_generator: Arc<PlanGenerator>,
    /// Query transformer
    transformer: Arc<QueryTransformer>,
    /// Adaptive executor
    adaptive_executor: Arc<AdaptiveExecutor>,
    /// Plan baseline manager
    baseline_manager: Arc<PlanBaselineManager>,
    /// Hint parser
    hint_parser: Arc<HintParser>,
    /// Plan cache
    plan_cache: Arc<RwLock<PlanCache>>,
    /// Query statistics
    stats: Arc<RwLock<OptimizerStatistics>>,
}

impl QueryOptimizer {
    /// Create a new query optimizer
    pub fn new(config: OptimizerConfig) -> Self {
        let cost_model = Arc::new(CostModel::new(config.cost_params.clone()));
        let plan_generator = Arc::new(PlanGenerator::new(
            config.max_join_combinations,
            Arc::clone(&cost_model),
        ));
        let transformer = Arc::new(QueryTransformer::new(config.transformation_rules.clone()));
        let adaptive_executor = Arc::new(AdaptiveExecutor::new());
        let baseline_manager = Arc::new(PlanBaselineManager::new());
        let hint_parser = Arc::new(HintParser::new());

        Self {
            config,
            cost_model,
            plan_generator,
            transformer,
            adaptive_executor,
            baseline_manager,
            hint_parser,
            plan_cache: Arc::new(RwLock::new(PlanCache::new(1000))),
            stats: Arc::new(RwLock::new(OptimizerStatistics::default())),
        }
    }

    /// Optimize a query
    pub fn optimize(&self, query: &Query) -> Result<PhysicalPlan> {
        let start = Instant::now();
        let mut stats = self.stats.write().unwrap();
        stats.queries_optimized += 1;

        drop(stats);

        // Extract hints
        let hints = self.hint_parser.parse_hints(&query.text)?;

        // Generate query fingerprint
        let fingerprint = QueryFingerprint::new(
            &query.text,
            query.param_types.clone(),
            query.schema_version,
        );

        // Check plan cache
        if let Some(cached_plan) = self.plan_cache.read().unwrap().get(&fingerprint) {
            let mut stats = self.stats.write().unwrap();
            stats.cache_hits += 1;
            return Ok(cached_plan.clone());
        }

        // Check plan baselines if enabled
        if self.config.enable_plan_baselines {
            if let Some(baseline) = self.baseline_manager.get_baseline(&fingerprint)? {
                if let Some(plan) = baseline.get_best_plan() {
                    return Ok(plan.clone());
                }
            }
        }

        // Apply query transformations if enabled
        let transformed_query = if self.config.enable_transformations {
            self.transformer.transform(query)?
        } else {
            query.clone()
        };

        // Generate candidate plans
        let candidate_plans = self.plan_generator.generate_plans(&transformed_query, &hints)?;

        // Select best plan based on cost
        let best_plan = self.select_best_plan(candidate_plans)?;

        // Cache the plan
        self.plan_cache.write().unwrap().insert(fingerprint, best_plan.clone());

        // Update statistics
        let mut stats = self.stats.write().unwrap();
        stats.total_optimization_time += start.elapsed();

        Ok(best_plan)
    }

    /// Execute a plan with adaptive optimization
    pub fn execute_adaptive(&self, plan: &PhysicalPlan) -> Result<ExecutionResult> {
        if self.config.enable_adaptive {
            self.adaptive_executor.execute(plan)
        } else {
            // Non-adaptive execution
            Ok(ExecutionResult {
                rows: vec![],
                execution_time: Duration::from_secs(0),
                adaptive_corrections: vec![],
            })
        }
    }

    /// Select the best plan from candidates
    /// Optimized to eliminate heap allocations in comparison loops
    #[inline]
    fn select_best_plan(&self, mut plans: Vec<PhysicalPlan>) -> Result<PhysicalPlan> {
        if plans.is_empty() {
            return Err(DbError::Internal("No candidate plans generated".to_string()));
        }

        // Sort by cost in-place, unstable for better performance
        plans.sort_unstable_by(|a, b| a.cost.partial_cmp(&b.cost).unwrap());

        Ok(plans.into_iter().next().unwrap())
    }

    /// Get optimizer statistics
    pub fn get_statistics(&self) -> OptimizerStatistics {
        self.stats.read().unwrap().clone()
    }

    /// Clear plan cache
    pub fn clear_cache(&self) {
        self.plan_cache.write().unwrap().clear();
    }

    /// Capture a plan baseline
    pub fn capture_baseline(&self, fingerprint: QueryFingerprint, plan: PhysicalPlan) -> Result<()> {
        self.baseline_manager.capture_baseline(fingerprint, plan)
    }

    /// Evolve plan baselines
    pub fn evolve_baselines(&self) -> Result<usize> {
        self.baseline_manager.evolve_baselines()
    }
}

// ============================================================================
// Supporting Types
// ============================================================================

/// Query representation
#[derive(Debug, Clone)]
pub struct Query {
    pub text: String,
    pub param_types: Vec<String>,
    pub schema_version: u64,
}

impl Query {
    /// Parse a query from text
    pub fn parse(text: &str) -> Result<Self> {
        Ok(Self {
            text: text.to_string(),
            param_types: vec![],
            schema_version: 1,
        })
    }
}

/// Execution result
#[derive(Debug)]
pub struct ExecutionResult {
    pub rows: Vec<Vec<Value>>,
    pub execution_time: Duration,
    pub adaptive_corrections: Vec<String>,
}

/// Plan cache
struct PlanCache {
    cache: HashMap<QueryFingerprint, PhysicalPlan>,
    max_size: usize,
    access_order: VecDeque<QueryFingerprint>,
}

impl PlanCache {
    fn new(max_size: usize) -> Self {
        Self {
            cache: HashMap::new(),
            max_size,
            access_order: VecDeque::new(),
        }
    }

    fn get(&self, fingerprint: &QueryFingerprint) -> Option<PhysicalPlan> {
        self.cache.get(fingerprint).cloned()
    }

    fn insert(&mut self, fingerprint: QueryFingerprint, plan: PhysicalPlan) {
        if self.cache.len() >= self.max_size {
            if let Some(oldest) = self.access_order.pop_front() {
                self.cache.remove(&oldest);
            }
        }

        self.cache.insert(fingerprint.clone(), plan);
        self.access_order.push_back(fingerprint);
    }

    fn clear(&mut self) {
        self.cache.clear();
        self.access_order.clear();
    }
}

/// Optimizer statistics
#[derive(Debug, Clone, Default)]
pub struct OptimizerStatistics {
    pub queries_optimized: u64,
    pub cache_hits: u64,
    pub total_optimization_time: Duration,
    pub avg_plans_generated: f64,
    pub transformations_applied: u64,
    pub adaptive_corrections: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_query_fingerprint() {
        let fp1 = QueryFingerprint::new(
            "SELECT * FROM users WHERE id = ?",
            vec!["int".to_string()],
            1,
        );
        let fp2 = QueryFingerprint::new(
            "SELECT  *  FROM  users  WHERE  id  =  ?",
            vec!["int".to_string()],
            1,
        );

        assert_eq!(fp1.normalized_text, fp2.normalized_text);
    }

    #[test]
    fn test_optimizer_creation() {
        let config = OptimizerConfig::default();
        let optimizer = QueryOptimizer::new(config);

        let stats = optimizer.get_statistics();
        assert_eq!(stats.queries_optimized, 0);
    }

    #[test]
    fn test_plan_cache() {
        let mut cache = PlanCache::new(2);

        let fp1 = QueryFingerprint::new("SELECT 1", vec![], 1);
        let fp2 = QueryFingerprint::new("SELECT 2", vec![], 1);
        let fp3 = QueryFingerprint::new("SELECT 3", vec![], 1);

        let plan = PhysicalPlan {
            plan_id: PlanId(1),
            operator: PhysicalOperator::SeqScan {
                table_id: TableId(1),
                filter: None,
            },
            cost: 100.0,
            cardinality: 1000,
            schema: Schema { columns: vec![] },
            metadata: PlanMetadata {
                created_at: SystemTime::now(),
                optimizer_version: "1.0".to_string(),
                hints: vec![],
                transformations: vec![],
                from_baseline: false,
            },
        };

        cache.insert(fp1.clone(), plan.clone());
        cache.insert(fp2.clone(), plan.clone());

        assert!(cache.get(&fp1).is_some());

        // This should evict fp1
        cache.insert(fp3.clone(), plan.clone());

        assert!(cache.get(&fp1).is_none());
        assert!(cache.get(&fp2).is_some());
        assert!(cache.get(&fp3).is_some());
    }
}


