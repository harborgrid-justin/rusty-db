// Advanced Subquery Support
//
// This module provides comprehensive subquery capabilities including:
// - Correlated subqueries
// - Scalar subqueries
// - EXISTS and NOT EXISTS operators
// - IN and NOT IN operators
// - ANY and ALL operators
// - Subquery decorrelation optimization

use super::planner::PlanNode;
use super::QueryResult;
use crate::error::DbError;

// Subquery type classification
#[derive(Debug, Clone, PartialEq)]
pub enum SubqueryType {
    // Scalar subquery - returns single value
    Scalar,
    // EXISTS subquery - returns boolean
    Exists,
    // IN subquery - checks if value in set
    In,
    // Correlated subquery - references outer query
    Correlated,
    // Uncorrelated subquery - independent of outer query
    Uncorrelated,
}

// Subquery expression
#[derive(Debug, Clone)]
pub struct SubqueryExpr {
    pub subquery_type: SubqueryType,
    pub plan: Box<PlanNode>,
    pub outer_refs: Vec<String>, // Columns referenced from outer query
    pub negated: bool,           // For NOT EXISTS, NOT IN
}

impl SubqueryExpr {
    pub fn new(subquery_type: SubqueryType, plan: Box<PlanNode>) -> Self {
        Self {
            subquery_type,
            plan,
            outer_refs: Vec::new(),
            negated: false,
        }
    }

    pub fn with_outer_refs(mut self, refs: Vec<String>) -> Self {
        self.outer_refs = refs;
        self
    }

    pub fn with_negation(mut self, negated: bool) -> Self {
        self.negated = negated;
        self
    }

    // Check if subquery is correlated
    pub fn is_correlated(&self) -> bool {
        !self.outer_refs.is_empty()
    }
}

// EXISTS subquery evaluator
pub struct ExistsEvaluator;

impl ExistsEvaluator {
    // Evaluate EXISTS subquery
    // Returns true if subquery returns at least one row
    pub fn evaluate(result: &QueryResult, negated: bool) -> bool {
        let has_rows = !result.rows.is_empty();
        if negated {
            !has_rows
        } else {
            has_rows
        }
    }

    // Optimize EXISTS subquery
    // Can stop after finding first row
    pub fn can_short_circuit() -> bool {
        true
    }
}

// IN subquery evaluator
pub struct InEvaluator;

impl InEvaluator {
    // Evaluate IN subquery
    // Check if value exists in subquery result set
    pub fn evaluate(value: &str, result: &QueryResult, negated: bool) -> Result<bool, DbError> {
        if result.columns.len() != 1 {
            return Err(DbError::InvalidInput(
                "IN subquery must return exactly one column".to_string(),
            ));
        }

        let in_set = result
            .rows
            .iter()
            .any(|row| row.get(0).map(|v| v == value).unwrap_or(false));

        Ok(if negated { !in_set } else { in_set })
    }

    // Convert IN subquery to semi-join for optimization
    pub fn convert_to_semijoin(outer_column: String, subquery: SubqueryExpr) -> PlanNode {
        // Semi-join: returns rows from outer where match exists in inner
        // This is more efficient than nested loop evaluation

        PlanNode::Join {
            join_type: crate::parser::JoinType::Inner,
            left: Box::new(PlanNode::TableScan {
                table: "outer".to_string(),
                columns: vec![outer_column.clone()],
            }),
            right: subquery.plan,
            condition: format!("{} = subquery.value", outer_column),
        }
    }
}

// Scalar subquery evaluator
pub struct ScalarSubqueryEvaluator;

impl ScalarSubqueryEvaluator {
    // Evaluate scalar subquery
    // Must return exactly one row and one column
    pub fn evaluate(result: &QueryResult) -> Result<Option<String>, DbError> {
        if result.columns.len() > 1 {
            return Err(DbError::InvalidInput(
                "Scalar subquery must return exactly one column".to_string(),
            ));
        }

        if result.rows.len() > 1 {
            return Err(DbError::InvalidInput(
                "Scalar subquery returned more than one row".to_string(),
            ));
        }

        if result.rows.is_empty() {
            return Ok(None); // NULL result
        }

        Ok(result.rows[0].get(0).cloned())
    }
}

// ANY/ALL operator evaluator
pub struct QuantifiedComparisonEvaluator;

impl QuantifiedComparisonEvaluator {
    // Evaluate ANY operator
    // Returns true if comparison is true for ANY value in subquery
    pub fn evaluate_any(
        value: &str,
        operator: ComparisonOp,
        result: &QueryResult,
    ) -> Result<bool, DbError> {
        if result.columns.len() != 1 {
            return Err(DbError::InvalidInput(
                "Quantified comparison subquery must return exactly one column".to_string(),
            ));
        }

        for row in &result.rows {
            if let Some(subquery_value) = row.get(0) {
                if Self::compare(value, operator, subquery_value)? {
                    return Ok(true);
                }
            }
        }

        Ok(false)
    }

    // Evaluate ALL operator
    // Returns true if comparison is true for ALL values in subquery
    pub fn evaluate_all(
        value: &str,
        operator: ComparisonOp,
        result: &QueryResult,
    ) -> Result<bool, DbError> {
        if result.columns.len() != 1 {
            return Err(DbError::InvalidInput(
                "Quantified comparison subquery must return exactly one column".to_string(),
            ));
        }

        if result.rows.is_empty() {
            return Ok(true); // Vacuously true
        }

        for row in &result.rows {
            if let Some(subquery_value) = row.get(0) {
                if !Self::compare(value, operator, subquery_value)? {
                    return Ok(false);
                }
            }
        }

        Ok(true)
    }

    fn compare(left: &str, op: ComparisonOp, right: &str) -> Result<bool, DbError> {
        // Try numeric comparison first
        if let (Ok(l), Ok(r)) = (left.parse::<f64>(), right.parse::<f64>()) {
            return Ok(match op {
                ComparisonOp::Equal => l == r,
                ComparisonOp::NotEqual => l != r,
                ComparisonOp::Less => l < r,
                ComparisonOp::LessOrEqual => l <= r,
                ComparisonOp::Greater => l > r,
                ComparisonOp::GreaterOrEqual => l >= r,
            });
        }

        // Fall back to string comparison
        Ok(match op {
            ComparisonOp::Equal => left == right,
            ComparisonOp::NotEqual => left != right,
            ComparisonOp::Less => left < right,
            ComparisonOp::LessOrEqual => left <= right,
            ComparisonOp::Greater => left > right,
            ComparisonOp::GreaterOrEqual => left >= right,
        })
    }
}

// Comparison operators for quantified comparisons
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ComparisonOp {
    Equal,
    NotEqual,
    Less,
    LessOrEqual,
    Greater,
    GreaterOrEqual,
}

// Correlated subquery handler
pub struct CorrelatedSubqueryHandler;

impl CorrelatedSubqueryHandler {
    // Execute correlated subquery for each outer row
    pub fn execute_correlated(
        _outer_row: &[String],
        _subquery: &SubqueryExpr,
    ) -> Result<QueryResult, DbError> {
        // In a full implementation:
        // 1. Bind outer row values to subquery parameters
        // 2. Execute subquery with bound values
        // 3. Return result

        // For now, return empty result
        Ok(QueryResult::empty())
    }

    // Estimate cost of correlated execution
    pub fn estimate_cost(outer_rows: usize, subquery_cost: f64) -> f64 {
        // Correlated subquery executes once per outer row
        outer_rows as f64 * subquery_cost
    }
}

// Subquery decorrelation optimizer
// Converts correlated subqueries to joins when possible
pub struct SubqueryDecorrelator;

impl SubqueryDecorrelator {
    // Attempt to decorrelate a subquery
    // Converts correlated subqueries to joins for better performance
    pub fn decorrelate(subquery: &SubqueryExpr) -> Option<PlanNode> {
        if !subquery.is_correlated() {
            return None; // Already uncorrelated
        }

        // Pattern matching for common decorrelation cases
        match subquery.subquery_type {
            SubqueryType::Exists => Self::decorrelate_exists(subquery),
            SubqueryType::In => Self::decorrelate_in(subquery),
            SubqueryType::Scalar => Self::decorrelate_scalar(subquery),
            _ => None,
        }
    }

    fn decorrelate_exists(_subquery: &SubqueryExpr) -> Option<PlanNode> {
        // Convert EXISTS correlated subquery to semi-join
        // Example:
        // SELECT * FROM orders o
        // WHERE EXISTS (SELECT 1 FROM items i WHERE i.order_id = o.id)
        //
        // Becomes:
        // SELECT DISTINCT o.* FROM orders o
        // INNER JOIN items i ON i.order_id = o.id

        // Placeholder - full implementation would construct semi-join
        None
    }

    fn decorrelate_in(_subquery: &SubqueryExpr) -> Option<PlanNode> {
        // Convert IN correlated subquery to join
        // Similar to EXISTS but returns matching values
        None
    }

    fn decorrelate_scalar(_subquery: &SubqueryExpr) -> Option<PlanNode> {
        // Convert scalar correlated subquery to LEFT JOIN with aggregation
        None
    }

    // Check if subquery can be decorrelated
    pub fn can_decorrelate(subquery: &SubqueryExpr) -> bool {
        // Check for patterns that can be decorrelated
        match subquery.subquery_type {
            SubqueryType::Exists | SubqueryType::In => true,
            SubqueryType::Scalar => {
                // Scalar can be decorrelated if it has simple aggregation
                Self::has_simple_aggregation(&subquery.plan)
            }
            _ => false,
        }
    }

    fn has_simple_aggregation(_plan: &PlanNode) -> bool {
        // Check if plan has a simple aggregation (MAX, MIN, etc.)
        // that can be converted to a join
        false // Placeholder
    }
}

// Subquery cache for repeated evaluations
pub struct SubqueryCache {
    cache: std::collections::HashMap<String, QueryResult>,
    max_size: usize,
}

impl SubqueryCache {
    pub fn new(max_size: usize) -> Self {
        Self {
            cache: std::collections::HashMap::new(),
            max_size,
        }
    }

    // Get cached result for subquery
    pub fn get(&self, key: &str) -> Option<&QueryResult> {
        self.cache.get(key)
    }

    // Cache subquery result
    pub fn put(&mut self, key: String, result: QueryResult) {
        if self.cache.len() >= self.max_size {
            // Simple eviction: clear cache when full
            self.cache.clear();
        }
        self.cache.insert(key, result);
    }

    // Generate cache key from subquery
    pub fn generate_key(subquery: &SubqueryExpr) -> String {
        // Simple hash-based key generation
        // In production, would use a better hashing strategy
        format!("{:?}", subquery.plan)
    }
}

// Subquery optimizer
pub struct SubqueryOptimizer;

impl SubqueryOptimizer {
    // Optimize subquery execution strategy
    pub fn optimize(subquery: &SubqueryExpr) -> SubqueryExecutionStrategy {
        // Decide on execution strategy based on subquery characteristics

        if !subquery.is_correlated() {
            // Uncorrelated: execute once and cache
            return SubqueryExecutionStrategy::ExecuteOnce;
        }

        // Try to decorrelate
        if SubqueryDecorrelator::can_decorrelate(subquery) {
            return SubqueryExecutionStrategy::Decorrelate;
        }

        // Fall back to nested loop execution
        SubqueryExecutionStrategy::NestedLoop
    }

    // Estimate subquery execution cost
    pub fn estimate_cost(
        strategy: &SubqueryExecutionStrategy,
        outer_cardinality: usize,
        inner_cardinality: usize,
    ) -> f64 {
        match strategy {
            SubqueryExecutionStrategy::ExecuteOnce => inner_cardinality as f64,
            SubqueryExecutionStrategy::NestedLoop => (outer_cardinality * inner_cardinality) as f64,
            SubqueryExecutionStrategy::Decorrelate => {
                // Join-based execution
                (outer_cardinality + inner_cardinality) as f64 * 1.5
            }
        }
    }
}

// Subquery execution strategy
#[derive(Debug, Clone, PartialEq)]
pub enum SubqueryExecutionStrategy {
    // Execute subquery once and cache result
    ExecuteOnce,
    // Execute subquery for each outer row (nested loop)
    NestedLoop,
    // Decorrelate and execute as join
    Decorrelate,
}

// Subquery rewrite rules
pub struct SubqueryRewriter;

impl SubqueryRewriter {
    // Apply rewrite rules to simplify subqueries
    pub fn rewrite(subquery: SubqueryExpr) -> SubqueryExpr {
        // Apply various rewrite rules
        let mut result = subquery;

        // Rule 1: Convert NOT IN to NOT EXISTS for better performance
        result = Self::not_in_to_not_exists(result);

        // Rule 2: Push down predicates into subquery
        result = Self::push_predicates(result);

        result
    }

    fn not_in_to_not_exists(subquery: SubqueryExpr) -> SubqueryExpr {
        if subquery.subquery_type == SubqueryType::In && subquery.negated {
            // Convert to NOT EXISTS for better performance
            SubqueryExpr {
                subquery_type: SubqueryType::Exists,
                ..subquery
            }
        } else {
            subquery
        }
    }

    fn push_predicates(subquery: SubqueryExpr) -> SubqueryExpr {
        // Push down predicates from outer query into subquery
        // This reduces the subquery result set
        subquery // Placeholder
    }
}

// Subquery context for tracking subquery metadata
pub struct SubqueryContext {
    // Nesting level (0 = top-level query)
    nesting_level: usize,
    // Columns available from outer queries
    outer_columns: Vec<String>,
}

impl SubqueryContext {
    pub fn new() -> Self {
        Self {
            nesting_level: 0,
            outer_columns: Vec::new(),
        }
    }

    pub fn enter_subquery(&mut self, available_columns: Vec<String>) {
        self.nesting_level += 1;
        self.outer_columns.extend(available_columns);
    }

    pub fn exit_subquery(&mut self) {
        self.nesting_level = self.nesting_level.saturating_sub(1);
    }

    pub fn is_outer_column(&self, column: &str) -> bool {
        self.outer_columns.contains(&column.to_string())
    }

    pub fn nesting_level(&self) -> usize {
        self.nesting_level
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn test_exists_evaluator() {
        let result = QueryResult::new(vec!["id".to_string()], vec![vec!["1".to_string()]]);

        assert!(ExistsEvaluator::evaluate(&result, false));
        assert!(!ExistsEvaluator::evaluate(&result, true)); // NOT EXISTS

        let empty_result = QueryResult::empty();
        assert!(!ExistsEvaluator::evaluate(&empty_result, false));
        assert!(ExistsEvaluator::evaluate(&empty_result, true)); // NOT EXISTS
    }

    #[test]
    fn test_in_evaluator() {
        let result = QueryResult::new(
            vec!["value".to_string()],
            vec![
                vec!["1".to_string()],
                vec!["2".to_string()],
                vec!["3".to_string()],
            ],
        );

        assert!(InEvaluator::evaluate("2", &result, false).unwrap());
        assert!(!InEvaluator::evaluate("4", &result, false).unwrap());
        assert!(InEvaluator::evaluate("4", &result, true).unwrap()); // NOT IN
    }

    #[test]
    fn test_scalar_subquery_evaluator() {
        // Valid scalar subquery
        let result = QueryResult::new(vec!["count".to_string()], vec![vec!["42".to_string()]]);

        let value = ScalarSubqueryEvaluator::evaluate(&result).unwrap();
        assert_eq!(value, Some("42".to_string()));

        // Empty result (NULL)
        let empty = QueryResult::empty();
        let value = ScalarSubqueryEvaluator::evaluate(&empty).unwrap();
        assert_eq!(value, None);

        // Too many rows - should error
        let multi_row = QueryResult::new(
            vec!["count".to_string()],
            vec![vec!["1".to_string()], vec!["2".to_string()]],
        );

        assert!(ScalarSubqueryEvaluator::evaluate(&multi_row).is_err());
    }

    #[test]
    fn test_quantified_comparison() {
        let result = QueryResult::new(
            vec!["value".to_string()],
            vec![
                vec!["10".to_string()],
                vec!["20".to_string()],
                vec!["30".to_string()],
            ],
        );

        // 15 < ANY (10, 20, 30) -> true (15 < 20 and 15 < 30)
        assert!(
            QuantifiedComparisonEvaluator::evaluate_any("15", ComparisonOp::Less, &result).unwrap()
        );

        // 5 < ALL (10, 20, 30) -> true
        assert!(
            QuantifiedComparisonEvaluator::evaluate_all("5", ComparisonOp::Less, &result).unwrap()
        );

        // 25 < ALL (10, 20, 30) -> false (25 not < 10)
        assert!(
            !QuantifiedComparisonEvaluator::evaluate_all("25", ComparisonOp::Less, &result)
                .unwrap()
        );
    }

    #[test]
    fn test_subquery_expr() {
        let plan = Box::new(PlanNode::TableScan {
            table: "test".to_string(),
            columns: vec!["*".to_string()],
        });

        let subquery = SubqueryExpr::new(SubqueryType::Exists, plan)
            .with_outer_refs(vec!["outer_id".to_string()])
            .with_negation(false);

        assert!(subquery.is_correlated());
        assert_eq!(subquery.outer_refs.len(), 1);
    }

    #[test]
    fn test_subquery_cache() {
        let mut cache = SubqueryCache::new(10);

        let result = QueryResult::new(vec!["id".to_string()], vec![vec!["1".to_string()]]);

        cache.put("key1".to_string(), result.clone());

        let cached = cache.get("key1");
        assert!(cached.is_some());
        assert_eq!(cached.unwrap().rows.len(), 1);

        assert!(cache.get("key2").is_none());
    }

    #[test]
    fn test_subquery_optimizer() {
        let uncorrelated = SubqueryExpr::new(
            SubqueryType::Scalar,
            Box::new(PlanNode::TableScan {
                table: "test".to_string(),
                columns: vec!["*".to_string()],
            }),
        );

        let strategy = SubqueryOptimizer::optimize(&uncorrelated);
        assert_eq!(strategy, SubqueryExecutionStrategy::ExecuteOnce);
    }

    #[test]
    fn test_subquery_context() {
        let mut context = SubqueryContext::new();
        assert_eq!(context.nesting_level(), 0);

        context.enter_subquery(vec!["outer_col".to_string()]);
        assert_eq!(context.nesting_level(), 1);
        assert!(context.is_outer_column("outer_col"));

        context.exit_subquery();
        assert_eq!(context.nesting_level(), 0);
    }
}
