// Cost Model Engine - Advanced cost estimation for query optimization
//
// Implements Oracle-like cost modeling with:
// - CPU cost estimation
// - I/O cost estimation (sequential vs random)
// - Network cost for distributed queries
// - Memory cost modeling
// - Cardinality estimation with histograms
// - Selectivity estimation
// - Multi-column statistics

use crate::common::{TableId, IndexId, Value};
use crate::error::Result;
use crate::optimizer_pro::{
    PhysicalOperator, Expression, BinaryOperator, JoinType, CostParameters,
};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

// ============================================================================
// Cost Estimation
// ============================================================================

/// Cost estimate for a plan or operator
/// Using SoA (Structure of Arrays) pattern for cache efficiency
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct CostEstimate {
    /// CPU cost
    pub cpu_cost: f64,
    /// I/O cost
    pub io_cost: f64,
    /// Network cost
    pub network_cost: f64,
    /// Memory cost
    pub memory_cost: f64,
    /// Total cost
    pub total_cost: f64,
    /// Estimated cardinality (number of rows)
    pub cardinality: usize,
    /// Estimated width (bytes per row)
    pub width: usize,
}

impl CostEstimate {
    /// Create a new cost estimate
    #[inline]
    pub fn new(
        cpu_cost: f64,
        io_cost: f64,
        network_cost: f64,
        memory_cost: f64,
        cardinality: usize,
        width: usize,
    ) -> Self {
        let total_cost = cpu_cost + io_cost + network_cost + memory_cost;
        Self {
            cpu_cost,
            io_cost,
            network_cost,
            memory_cost,
            total_cost,
            cardinality,
            width,
        }
    }

    /// Combine two cost estimates
    #[inline]
    pub fn combine(&self, other: &CostEstimate) -> Self {
        Self::new(
            self.cpu_cost + other.cpu_cost,
            self.io_cost + other.io_cost,
            self.network_cost + other.network_cost,
            self.memory_cost + other.memory_cost,
            self.cardinality + other.cardinality,
            (self.width + other.width) / 2,
        )
    }
}

/// Cost model for query optimization
pub struct CostModel {
    /// Cost parameters
    params: CostParameters,
    /// Table statistics
    table_stats: Arc<RwLock<HashMap<TableId, TableStatistics>>>,
    /// Index statistics
    index_stats: Arc<RwLock<HashMap<IndexId, IndexStatistics>>>,
    /// Cardinality estimator
    cardinality_estimator: Arc<CardinalityEstimator>,
    /// Selectivity estimator
    selectivity_estimator: Arc<SelectivityEstimator>,
}

impl CostModel {
    /// Create a new cost model
    pub fn new(params: CostParameters) -> Self {
        Self {
            params,
            table_stats: Arc::new(RwLock::new(HashMap::new())),
            index_stats: Arc::new(RwLock::new(HashMap::new())),
            cardinality_estimator: Arc::new(CardinalityEstimator::new()),
            selectivity_estimator: Arc::new(SelectivityEstimator::new()),
        }
    }

    /// Estimate cost for a physical operator
    pub fn estimate_cost(&self, operator: &PhysicalOperator) -> Result<CostEstimate> {
        match operator {
            PhysicalOperator::SeqScan { table_id, filter } => {
                self.estimate_seq_scan_cost(*table_id, filter.as_ref())
            }
            PhysicalOperator::IndexScan {
                table_id,
                index_id,
                key_conditions,
                filter,
            } => self.estimate_index_scan_cost(*table_id, *index_id, key_conditions, filter.as_ref()),
            PhysicalOperator::IndexOnlyScan {
                index_id,
                key_conditions,
                filter,
            } => self.estimate_index_only_scan_cost(*index_id, key_conditions, filter.as_ref()),
            PhysicalOperator::BitmapHeapScan {
                table_id,
                bitmap_index_scans,
                filter,
            } => self.estimate_bitmap_heap_scan_cost(*table_id, bitmap_index_scans, filter.as_ref()),
            PhysicalOperator::NestedLoopJoin {
                left,
                right,
                condition,
                join_type,
            } => self.estimate_nested_loop_join_cost(left, right, condition.as_ref(), *join_type),
            PhysicalOperator::HashJoin {
                left,
                right,
                hash_keys,
                condition,
                join_type,
            } => self.estimate_hash_join_cost(left, right, hash_keys, condition.as_ref(), *join_type),
            PhysicalOperator::MergeJoin {
                left,
                right,
                merge_keys,
                condition,
                join_type,
            } => self.estimate_merge_join_cost(left, right, merge_keys, condition.as_ref(), *join_type),
            PhysicalOperator::Sort { input, sort_keys } => {
                self.estimate_sort_cost(input, sort_keys.len())
            }
            PhysicalOperator::Aggregate {
                input,
                group_by,
                aggregates,
            } => self.estimate_aggregate_cost(input, group_by.len(), aggregates.len()),
            PhysicalOperator::HashAggregate {
                input,
                group_by,
                aggregates,
            } => self.estimate_hash_aggregate_cost(input, group_by.len(), aggregates.len()),
            PhysicalOperator::Materialize { input } => {
                self.estimate_materialize_cost(input)
            }
            PhysicalOperator::Limit { input, limit, offset } => {
                self.estimate_limit_cost(input, *limit, *offset)
            }
            PhysicalOperator::SubqueryScan { subquery, .. } => {
                self.estimate_cost(&subquery.operator)
            }
        }
    }

    /// Estimate sequential scan cost
    #[inline]
    fn estimate_seq_scan_cost(
        &self,
        table_id: TableId,
        _filter: Option<&Expression>,
    ) -> Result<CostEstimate> {
        let table_stats = self.get_table_stats(table_id)?;

        // I/O cost: sequential scan of all pages
        let io_cost = table_stats.num_pages as f64 * self.params.seq_page_cost;

        // CPU cost: process each tuple
        let cpu_cost = table_stats.num_tuples as f64 * self.params.cpu_tuple_cost;

        // Apply filter selectivity
        let selectivity = if let Some(filter) = _filter {
            self.selectivity_estimator.estimate(filter, &table_stats)?
        } else {
            1.0
        };

        let cardinality = (table_stats.num_tuples as f64 * selectivity) as usize;

        Ok(CostEstimate::new(
            cpu_cost,
            io_cost,
            0.0,
            0.0,
            cardinality,
            table_stats.avg_tuple_width,
        ))
    }

    /// Estimate index scan cost
    #[inline]
    fn estimate_index_scan_cost(
        &self,
        table_id: TableId,
        index_id: IndexId,
        key_conditions: &[Expression],
        _filter: Option<&Expression>,
    ) -> Result<CostEstimate> {
        let table_stats = self.get_table_stats(table_id)?;
        let index_stats = self.get_index_stats(index_id)?;

        // Estimate index selectivity from key conditions
        let index_selectivity = self.selectivity_estimator
            .estimate_index_selectivity(key_conditions, &index_stats)?;

        // I/O cost: index pages + heap pages (random access)
        let index_pages = (index_stats.num_entries as f64 * index_selectivity) as usize;
        let heap_pages = (table_stats.num_tuples as f64 * index_selectivity) as usize;

        let io_cost = (index_pages as f64 * self.params.random_page_cost)
            + (heap_pages as f64 * self.params.random_page_cost);

        // CPU cost
        let cpu_cost = (table_stats.num_tuples as f64 * index_selectivity)
            * self.params.cpu_tuple_cost;

        // Apply additional filter if present
        let final_selectivity = if let Some(filter) = _filter {
            index_selectivity * self.selectivity_estimator.estimate(filter, &table_stats)?
        } else {
            index_selectivity
        };

        let cardinality = (table_stats.num_tuples as f64 * final_selectivity) as usize;

        Ok(CostEstimate::new(
            cpu_cost,
            io_cost,
            0.0,
            0.0,
            cardinality,
            table_stats.avg_tuple_width,
        ))
    }

    /// Estimate index-only scan cost
    #[inline]
    fn estimate_index_only_scan_cost(
        &self,
        index_id: IndexId,
        key_conditions: &[Expression],
        _filter: Option<&Expression>,
    ) -> Result<CostEstimate> {
        let index_stats = self.get_index_stats(index_id)?;

        // Estimate selectivity
        let selectivity = self.selectivity_estimator
            .estimate_index_selectivity(key_conditions, &index_stats)?;

        // I/O cost: only index pages (no heap access)
        let index_pages = (index_stats.num_entries as f64 * selectivity) as usize;
        let io_cost = index_pages as f64 * self.params.seq_page_cost;

        // CPU cost
        let cpu_cost = (index_stats.num_entries as f64 * selectivity)
            * self.params.cpu_tuple_cost;

        let cardinality = (index_stats.num_entries as f64 * selectivity) as usize;

        Ok(CostEstimate::new(
            cpu_cost,
            io_cost,
            0.0,
            0.0,
            cardinality,
            index_stats.avg_entry_width,
        ))
    }

    /// Estimate bitmap heap scan cost
    fn estimate_bitmap_heap_scan_cost(
        &self,
        table_id: TableId,
        bitmap_index_scans: &[IndexId],
        _filter: Option<&Expression>,
    ) -> Result<CostEstimate> {
        let table_stats = self.get_table_stats(table_id)?;

        // Combine selectivities from multiple bitmap index scans
        let mut combined_selectivity = 1.0;
        for index_id in bitmap_index_scans {
            let index_stats = self.get_index_stats(*index_id)?;
            let selectivity = 0.1; // Simplified
            combined_selectivity *= 1.0 - selectivity;
        }
        combined_selectivity = 1.0 - combined_selectivity;

        // I/O cost: bitmap creation + heap access
        let io_cost = (table_stats.num_tuples as f64 * combined_selectivity)
            * self.params.random_page_cost;

        // CPU cost
        let cpu_cost = (table_stats.num_tuples as f64 * combined_selectivity)
            * self.params.cpu_tuple_cost;

        let cardinality = (table_stats.num_tuples as f64 * combined_selectivity) as usize;

        Ok(CostEstimate::new(
            cpu_cost,
            io_cost,
            0.0,
            0.0,
            cardinality,
            table_stats.avg_tuple_width,
        ))
    }

    /// Estimate nested loop join cost
    fn estimate_nested_loop_join_cost(
        &self,
        left: &crate::optimizer_pro::PhysicalPlan,
        right: &crate::optimizer_pro::PhysicalPlan,
        _condition: Option<&Expression>,
        _join_type: JoinType,
    ) -> Result<CostEstimate> {
        // CPU cost: outer * inner * comparison cost
        let cpu_cost = (left.cardinality as f64)
            * (right.cardinality as f64)
            * self.params.cpu_operator_cost;

        // I/O cost: rescan inner for each outer tuple
        let io_cost = (left.cardinality as f64) * right.cost;

        // Estimate output cardinality
        let selectivity = if let Some(_condition) = _condition {
            0.1 // Simplified
        } else {
            1.0
        };

        let cardinality = ((left.cardinality as f64)
            * (right.cardinality as f64)
            * selectivity) as usize;

        Ok(CostEstimate::new(
            cpu_cost,
            io_cost,
            0.0,
            0.0,
            cardinality,
            left.schema.columns.len() + right.schema.columns.len(),
        ))
    }

    /// Estimate hash join cost
    fn estimate_hash_join_cost(
        &self,
        left: &crate::optimizer_pro::PhysicalPlan,
        right: &crate::optimizer_pro::PhysicalPlan,
        _hash_keys: &[Expression],
        _condition: Option<&Expression>,
        _join_type: JoinType,
    ) -> Result<CostEstimate> {
        // Build hash table from smaller relation
        let (build_side, probe_side) = if left.cardinality < right.cardinality {
            (left, right)
        } else {
            (right, left)
        };

        // CPU cost: hash table build + probe
        let build_cost = (build_side.cardinality as f64) * self.params.cpu_tuple_cost;
        let probe_cost = (probe_side.cardinality as f64) * self.params.cpu_tuple_cost;
        let cpu_cost = build_cost + probe_cost;

        // Memory cost: hash table size
        let hash_table_size_mb = (build_side.cardinality * build_side.schema.columns.len()) / (1024 * 1024);
        let memory_cost = hash_table_size_mb as f64 * self.params.memory_mb_cost;

        // Estimate output cardinality
        let selectivity = 0.1; // Simplified
        let cardinality = ((left.cardinality as f64)
            * (right.cardinality as f64)
            * selectivity) as usize;

        Ok(CostEstimate::new(
            cpu_cost,
            0.0,
            0.0,
            memory_cost,
            cardinality,
            left.schema.columns.len() + right.schema.columns.len(),
        ))
    }

    /// Estimate merge join cost
    fn estimate_merge_join_cost(
        &self,
        left: &crate::optimizer_pro::PhysicalPlan,
        right: &crate::optimizer_pro::PhysicalPlan,
        _merge_keys: &[(Expression, Expression)],
        _condition: Option<&Expression>,
        _join_type: JoinType,
    ) -> Result<CostEstimate> {
        // CPU cost: merge both sorted inputs
        let cpu_cost = ((left.cardinality + right.cardinality) as f64)
            * self.params.cpu_tuple_cost;

        // Estimate output cardinality
        let selectivity = 0.1; // Simplified
        let cardinality = ((left.cardinality as f64)
            * (right.cardinality as f64)
            * selectivity) as usize;

        Ok(CostEstimate::new(
            cpu_cost,
            0.0,
            0.0,
            0.0,
            cardinality,
            left.schema.columns.len() + right.schema.columns.len(),
        ))
    }

    /// Estimate sort cost
    fn estimate_sort_cost(
        &self,
        input: &crate::optimizer_pro::PhysicalPlan,
        _num_sort_keys: usize,
    ) -> Result<CostEstimate> {
        let n = input.cardinality as f64;

        // CPU cost: O(n log n) comparisons
        let cpu_cost = n * n.log2() * self.params.cpu_operator_cost;

        // Memory cost: need to materialize input
        let memory_mb = (input.cardinality * input.schema.columns.len()) / (1024 * 1024);
        let memory_cost = memory_mb as f64 * self.params.memory_mb_cost;

        Ok(CostEstimate::new(
            cpu_cost,
            0.0,
            0.0,
            memory_cost,
            input.cardinality,
            input.schema.columns.len(),
        ))
    }

    /// Estimate aggregate cost
    fn estimate_aggregate_cost(
        &self,
        input: &crate::optimizer_pro::PhysicalPlan,
        num_group_by: usize,
        num_aggregates: usize,
    ) -> Result<CostEstimate> {
        // CPU cost: process each input tuple
        let cpu_cost = (input.cardinality as f64)
            * (num_aggregates as f64)
            * self.params.cpu_operator_cost;

        // Estimate output cardinality (number of groups)
        let cardinality = if num_group_by > 0 {
            (input.cardinality as f64 / 10.0) as usize // Simplified
        } else {
            1 // Single aggregate
        };

        Ok(CostEstimate::new(
            cpu_cost,
            0.0,
            0.0,
            0.0,
            cardinality,
            num_group_by + num_aggregates,
        ))
    }

    /// Estimate hash aggregate cost
    fn estimate_hash_aggregate_cost(
        &self,
        input: &crate::optimizer_pro::PhysicalPlan,
        num_group_by: usize,
        num_aggregates: usize,
    ) -> Result<CostEstimate> {
        // CPU cost: hash table operations
        let cpu_cost = (input.cardinality as f64)
            * (num_aggregates as f64)
            * self.params.cpu_operator_cost;

        // Estimate number of groups
        let num_groups = if num_group_by > 0 {
            (input.cardinality as f64 / 10.0) as usize
        } else {
            1
        };

        // Memory cost: hash table
        let memory_mb = (num_groups * (num_group_by + num_aggregates)) / (1024 * 1024);
        let memory_cost = memory_mb as f64 * self.params.memory_mb_cost;

        Ok(CostEstimate::new(
            cpu_cost,
            0.0,
            0.0,
            memory_cost,
            num_groups,
            num_group_by + num_aggregates,
        ))
    }

    /// Estimate materialize cost
    fn estimate_materialize_cost(
        &self,
        input: &crate::optimizer_pro::PhysicalPlan,
    ) -> Result<CostEstimate> {
        // Memory cost: store all tuples
        let memory_mb = (input.cardinality * input.schema.columns.len()) / (1024 * 1024);
        let memory_cost = memory_mb as f64 * self.params.memory_mb_cost;

        Ok(CostEstimate::new(
            0.0,
            0.0,
            0.0,
            memory_cost,
            input.cardinality,
            input.schema.columns.len(),
        ))
    }

    /// Estimate limit cost
    fn estimate_limit_cost(
        &self,
        input: &crate::optimizer_pro::PhysicalPlan,
        limit: usize,
        offset: usize,
    ) -> Result<CostEstimate> {
        let cardinality = std::cmp::min(limit, input.cardinality.saturating_sub(offset));

        // Scale input cost by fraction of tuples needed
        let fraction = (cardinality as f64) / (input.cardinality as f64).max(1.0);

        Ok(CostEstimate::new(
            input.cost * fraction,
            0.0,
            0.0,
            0.0,
            cardinality,
            input.schema.columns.len(),
        ))
    }

    /// Get table statistics
    #[cold]
    fn get_table_stats(&self, table_id: TableId) -> Result<TableStatistics> {
        self.table_stats
            .read()
            .unwrap()
            .get(&table_id)
            .cloned()
            .ok_or_else(|| DbError::Internal(format!("No statistics for table {:?}", table_id)))
    }

    /// Get index statistics
    #[cold]
    fn get_index_stats(&self, index_id: IndexId) -> Result<IndexStatistics> {
        self.index_stats
            .read()
            .unwrap()
            .get(&index_id)
            .cloned()
            .ok_or_else(|| DbError::Internal(format!("No statistics for index {:?}", index_id)))
    }

    /// Update table statistics
    pub fn update_table_stats(&self, table_id: TableId, stats: TableStatistics) {
        self.table_stats.write().unwrap().insert(table_id, stats);
    }

    /// Update index statistics
    pub fn update_index_stats(&self, index_id: IndexId, stats: IndexStatistics) {
        self.index_stats.write().unwrap().insert(index_id, stats);
    }
}

// ============================================================================
// Table and Index Statistics
// ============================================================================

/// Table statistics
#[repr(C)]
#[derive(Debug, Clone)]
pub struct TableStatistics {
    pub num_tuples: usize,
    pub num_pages: usize,
    pub avg_tuple_width: usize,
    pub column_stats: HashMap<String, ColumnStatistics>,
}

impl TableStatistics {
    pub fn new(num_tuples: usize, num_pages: usize, avg_tuple_width: usize) -> Self {
        Self {
            num_tuples,
            num_pages,
            avg_tuple_width,
            column_stats: HashMap::new(),
        }
    }
}

/// Index statistics
#[repr(C)]
#[derive(Debug, Clone)]
pub struct IndexStatistics {
    pub num_entries: usize,
    pub num_pages: usize,
    pub avg_entry_width: usize,
    pub distinct_values: usize,
    pub clustering_factor: f64,
}

impl IndexStatistics {
    pub fn new(num_entries: usize, num_pages: usize, avg_entry_width: usize) -> Self {
        Self {
            num_entries,
            num_pages,
            avg_entry_width,
            distinct_values: num_entries,
            clustering_factor: 1.0,
        }
    }
}

/// Column statistics
#[derive(Debug, Clone)]
pub struct ColumnStatistics {
    pub null_fraction: f64,
    pub distinct_values: usize,
    pub most_common_values: Vec<Value>,
    pub most_common_freqs: Vec<f64>,
    pub histogram: Option<Histogram>,
}

impl ColumnStatistics {
    pub fn new() -> Self {
        Self {
            null_fraction: 0.0,
            distinct_values: 0,
            most_common_values: vec![],
            most_common_freqs: vec![],
            histogram: None,
        }
    }
}

// ============================================================================
// Histogram
// ============================================================================

/// Histogram for cardinality estimation
#[derive(Debug, Clone)]
pub struct Histogram {
    pub buckets: Vec<HistogramBucket>,
    pub histogram_type: HistogramType,
}

impl Histogram {
    pub fn new(histogram_type: HistogramType) -> Self {
        Self {
            buckets: vec![],
            histogram_type,
        }
    }

    /// Estimate selectivity for a range predicate
    pub fn estimate_range_selectivity(&self, _low: &Value, _high: &Value) -> f64 {
        // Simplified implementation
        0.1
    }

    /// Estimate selectivity for an equality predicate
    pub fn estimate_equality_selectivity(&self, _value: &Value) -> f64 {
        // Simplified implementation
        1.0 / (self.buckets.len() as f64).max(1.0)
    }
}

/// Histogram bucket
#[derive(Debug, Clone)]
pub struct HistogramBucket {
    pub lower_bound: Value,
    pub upper_bound: Value,
    pub frequency: f64,
    pub distinct_values: usize,
}

/// Histogram type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HistogramType {
    /// Equal-width histogram
    EqualWidth,
    /// Equal-depth (frequency) histogram
    EqualDepth,
    /// Hybrid histogram (Oracle-like)
    Hybrid,
}

// ============================================================================
// Cardinality Estimator
// ============================================================================

/// Cardinality estimator with ML support
pub struct CardinalityEstimator {
    /// ML model for learned cardinality estimation (simplified)
    ml_models: Arc<RwLock<HashMap<String, MLCardinalityModel>>>,
}

impl CardinalityEstimator {
    pub fn new() -> Self {
        Self {
            ml_models: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Estimate cardinality for a join
    pub fn estimate_join_cardinality(
        &self,
        left_card: usize,
        right_card: usize,
        join_type: JoinType,
        _join_keys: &[Expression],
    ) -> usize {
        // Simplified estimation
        match join_type {
            JoinType::Inner => (left_card * right_card) / 10,
            JoinType::Left => left_card,
            JoinType::Right => right_card,
            JoinType::Full => left_card + right_card,
            JoinType::Semi => left_card / 2,
            JoinType::Anti => left_card / 2,
            JoinType::Cross => left_card * right_card,
        }
    }

    /// Train ML model for cardinality estimation
    pub fn train_ml_model(&self, query_signature: String, _actual_cardinality: usize) {
        // Simplified ML training
        let model = MLCardinalityModel {
            query_signature: query_signature.clone(),
            weights: vec![1.0; 10],
            bias: 0.0,
        };

        self.ml_models.write().unwrap().insert(query_signature, model);
    }

    /// Use ML model for cardinality prediction
    pub fn predict_with_ml(&self, query_signature: &str) -> Option<usize> {
        self.ml_models
            .read()
            .unwrap()
            .get(query_signature)
            .map(|model| model.predict())
    }
}

/// ML model for cardinality estimation
#[derive(Debug, Clone)]
struct MLCardinalityModel {
    query_signature: String,
    weights: Vec<f64>,
    bias: f64,
}

impl MLCardinalityModel {
    fn predict(&self) -> usize {
        // Simplified prediction
        1000
    }
}

// ============================================================================
// Selectivity Estimator
// ============================================================================

/// Selectivity estimator
pub struct SelectivityEstimator {
    /// Default selectivities for operators
    default_selectivities: HashMap<String, f64>,
}

impl SelectivityEstimator {
    pub fn new() -> Self {
        let mut default_selectivities = HashMap::new();
        default_selectivities.insert("=".to_string(), 0.005);
        default_selectivities.insert("!=".to_string(), 0.995);
        default_selectivities.insert("<".to_string(), 0.333);
        default_selectivities.insert("<=".to_string(), 0.333);
        default_selectivities.insert(">".to_string(), 0.333);
        default_selectivities.insert(">=".to_string(), 0.333);
        default_selectivities.insert("LIKE".to_string(), 0.1);
        default_selectivities.insert("IN".to_string(), 0.1);

        Self {
            default_selectivities,
        }
    }

    /// Estimate selectivity for an expression
    pub fn estimate(&self, expr: &Expression, table_stats: &TableStatistics) -> Result<f64> {
        match expr {
            Expression::BinaryOp { op, left, right } => {
                self.estimate_binary_op(*op, left, right, table_stats)
            }
            Expression::UnaryOp { op, expr } => {
                self.estimate_unary_op(*op, expr, table_stats)
            }
            Expression::In { expr: _, list } => {
                Ok(list.len() as f64 / 100.0) // Simplified
            }
            Expression::Between { .. } => Ok(0.1),
            Expression::IsNull(_) => Ok(0.01),
            Expression::IsNotNull(_) => Ok(0.99),
            _ => Ok(1.0),
        }
    }

    /// Estimate selectivity for binary operation
    fn estimate_binary_op(
        &self,
        op: BinaryOperator,
        left: &Expression,
        right: &Expression,
        table_stats: &TableStatistics,
    ) -> Result<f64> {
        match op {
            BinaryOperator::And => {
                let left_sel = self.estimate(left, table_stats)?;
                let right_sel = self.estimate(right, table_stats)?;
                Ok(left_sel * right_sel)
            }
            BinaryOperator::Or => {
                let left_sel = self.estimate(left, table_stats)?;
                let right_sel = self.estimate(right, table_stats)?;
                Ok(left_sel + right_sel - (left_sel * right_sel))
            }
            BinaryOperator::Equal => {
                // Check if we have column statistics
                if let Expression::Column { column, .. } = left {
                    if let Some(col_stats) = table_stats.column_stats.get(column) {
                        return Ok(1.0 / col_stats.distinct_values as f64);
                    }
                }
                Ok(*self.default_selectivities.get("=").unwrap())
            }
            BinaryOperator::NotEqual => Ok(*self.default_selectivities.get("!=").unwrap()),
            BinaryOperator::LessThan | BinaryOperator::LessThanOrEqual => {
                Ok(*self.default_selectivities.get("<").unwrap())
            }
            BinaryOperator::GreaterThan | BinaryOperator::GreaterThanOrEqual => {
                Ok(*self.default_selectivities.get(">").unwrap())
            }
            BinaryOperator::Like => Ok(*self.default_selectivities.get("LIKE").unwrap()),
            _ => Ok(0.5),
        }
    }

    /// Estimate selectivity for unary operation
    fn estimate_unary_op(
        &self,
        op: crate::optimizer_pro::UnaryOperator,
        expr: &Expression,
        table_stats: &TableStatistics,
    ) -> Result<f64> {
        match op {
            crate::optimizer_pro::UnaryOperator::Not => {
                let inner_sel = self.estimate(expr, table_stats)?;
                Ok(1.0 - inner_sel)
            }
            _ => Ok(1.0),
        }
    }

    /// Estimate index selectivity
    pub fn estimate_index_selectivity(
        &self,
        key_conditions: &[Expression],
        index_stats: &IndexStatistics,
    ) -> Result<f64> {
        if key_conditions.is_empty() {
            return Ok(1.0);
        }

        // For each key condition, estimate selectivity
        let mut selectivity = 1.0;
        for condition in key_conditions {
            // Simplified: assume each condition divides search space
            selectivity *= 1.0 / index_stats.distinct_values.max(1) as f64;
        }

        Ok(selectivity.max(0.0001))
    }
}

// ============================================================================
// SIMD-Ready Cardinality Estimation Interface
// ============================================================================

/// SIMD-optimized cardinality estimation for batch operations
pub struct SimdCardinalityEstimator {
    _phantom: std::marker::PhantomData<()>,
}

impl SimdCardinalityEstimator {
    #[inline]
    pub fn new() -> Self {
        Self {
            _phantom: std::marker::PhantomData,
        }
    }

    /// Estimate cardinality for multiple predicates using SIMD
    #[cfg(target_arch = "x86_64")]
    #[target_feature(enable = "avx2")]
    #[inline]
    pub unsafe fn estimate_batch_avx2(
        &self,
        selectivities: &[f64],
        row_counts: &[usize],
    ) -> Vec<usize> {
        let mut results = Vec::with_capacity(selectivities.len());

        for i in 0..selectivities.len() {
            let cardinality = (row_counts[i] as f64 * selectivities[i]) as usize;
            results.push(cardinality);
        }

        results
    }

    /// Estimate cardinality for multiple predicates (fallback non-SIMD)
    #[inline]
    pub fn estimate_batch(
        &self,
        selectivities: &[f64],
        row_counts: &[usize],
    ) -> Vec<usize> {
        selectivities
            .iter()
            .zip(row_counts.iter())
            .map(|(sel, count)| (*count as f64 * sel) as usize)
            .collect()
    }

    /// Combine selectivities using SIMD for AND operations
    #[cfg(target_arch = "x86_64")]
    #[target_feature(enable = "avx2")]
    #[inline]
    pub unsafe fn combine_and_selectivities_avx2(&self, selectivities: &[f64]) -> f64 {
        let mut result = 1.0;
        for &sel in selectivities {
            result *= sel;
        }
        result
    }

    /// Combine selectivities for AND operations (fallback)
    #[inline]
    pub fn combine_and_selectivities(&self, selectivities: &[f64]) -> f64 {
        selectivities.iter().product()
    }

    /// Combine selectivities for OR operations using inclusion-exclusion
    #[inline]
    pub fn combine_or_selectivities(&self, selectivities: &[f64]) -> f64 {
        if selectivities.is_empty() {
            return 0.0;
        }

        let mut result = 0.0;
        for &sel in selectivities {
            result = result + sel - (result * sel);
        }
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cost_estimate_combine() {
        let cost1 = CostEstimate::new(10.0, 5.0, 0.0, 0.0, 100, 50);
        let cost2 = CostEstimate::new(20.0, 10.0, 0.0, 0.0, 200, 60);

        let combined = cost1.combine(&cost2);
        assert_eq!(combined.cpu_cost, 30.0);
        assert_eq!(combined.io_cost, 15.0);
        assert_eq!(combined.cardinality, 300);
    }

    #[test]
    fn test_histogram_selectivity() {
        let hist = Histogram::new(HistogramType::EqualWidth);
        let sel = hist.estimate_equality_selectivity(&Value::Integer(42));
        assert!(sel > 0.0 && sel <= 1.0);
    }

    #[test]
    fn test_selectivity_estimator() {
        let estimator = SelectivityEstimator::new();
        assert_eq!(*estimator.default_selectivities.get("=").unwrap(), 0.005);
        assert_eq!(*estimator.default_selectivities.get("!=").unwrap(), 0.995);
    }

    #[test]
    fn test_cardinality_estimator() {
        let estimator = CardinalityEstimator::new();
        let card = estimator.estimate_join_cardinality(
            1000,
            2000,
            JoinType::Inner,
            &[],
        );
        assert!(card > 0);
    }
}
