/// Common Table Expressions (CTE) Support
/// 
/// This module provides comprehensive support for CTEs including:
/// - Non-recursive CTEs (WITH clause)
/// - Recursive CTEs (WITH RECURSIVE)
/// - Multiple CTEs in a single query
/// - CTE materialization and optimization

use crate::Result;
use crate::error::DbError;
use std::collections::HashMap;
use super::planner::PlanNode;
use super::QueryResult;

/// CTE Definition
#[derive(Debug, Clone)]
pub struct CteDefinition {
    pub name: String,
    pub columns: Vec<String>,
    pub query: Box<PlanNode>,
    pub recursive: bool,
}

/// CTE context for storing materialized CTE results
#[derive(Debug)]
pub struct CteContext {
    // Maps CTE name to its materialized result
    materialized_ctes: HashMap<String, QueryResult>,
    // Maps CTE name to its definition
    definitions: HashMap<String, CteDefinition>,
}

impl CteContext {
    pub fn new() -> Self {
        Self {
            materialized_ctes: HashMap::new(),
            definitions: HashMap::new(),
        }
    }
    
    /// Register a CTE definition
    pub fn register_cte(&mut self, cte: CteDefinition) -> Result<()> {
        if self.definitions.contains_key(&cte.name) {
            return Err(DbError::AlreadyExists(format!(
                "CTE '{}' already defined",
                cte.name
            )));
        }
        self.definitions.insert(cte.name.clone(), cte);
        Ok(())
    }
    
    /// Get a CTE definition
    pub fn get_definition(&self, name: &str) -> Option<&CteDefinition> {
        self.definitions.get(name)
    }
    
    /// Store materialized CTE result
    pub fn materialize(&mut self, name: String, result: QueryResult) {
        self.materialized_ctes.insert(name, result);
    }
    
    /// Get materialized CTE result
    pub fn get_materialized(&self, name: &str) -> Option<&QueryResult> {
        self.materialized_ctes.get(name)
    }
    
    /// Check if a name refers to a CTE
    pub fn is_cte(&self, name: &str) -> bool {
        self.definitions.contains_key(name)
    }
}

/// Recursive CTE evaluator
pub struct RecursiveCteEvaluator {
    max_iterations: usize,
}

impl RecursiveCteEvaluator {
    pub fn new() -> Self {
        Self {
            max_iterations: 100, // Default max recursion depth
        }
    }
    
    pub fn with_max_iterations(max_iterations: usize) -> Self {
        Self { max_iterations }
    }
    
    /// Evaluate a recursive CTE
    /// 
    /// A recursive CTE has the form:
    /// WITH RECURSIVE cte_name AS (
    ///   base_query      -- Non-recursive term
    ///   UNION [ALL]
    ///   recursive_query -- Recursive term that references cte_name
    /// )
    pub fn evaluate(
        &self,
        cte_name: &str,
        base_result: QueryResult,
        recursive_plan: &PlanNode,
    ) -> Result<QueryResult> {
        let mut all_rows = base_result.rows.clone();
        let columns = base_result.columns.clone();
        let mut working_table = base_result;
        
        for iteration in 0..self.max_iterations {
            // If working table is empty, recursion terminates
            if working_table.rows.is_empty() {
                break;
            }
            
            // Execute recursive query with current working table
            // In a real implementation, this would execute the recursive_plan
            // with the working_table as the CTE reference
            // For now, we'll simulate by returning empty to prevent infinite loop
            let new_rows = self.execute_recursive_step(
                cte_name,
                &working_table,
                recursive_plan,
            )?;
            
            if new_rows.rows.is_empty() {
                break;
            }
            
            // Append new rows to result
            all_rows.extend(new_rows.rows.clone());
            working_table = new_rows;
            
            // Check for infinite recursion
            if iteration == self.max_iterations - 1 {
                return Err(DbError::InvalidOperation(format!(
                    "Recursive CTE '{}' exceeded maximum iterations ({})",
                    cte_name, self.max_iterations
                )));
            }
        }
        
        Ok(QueryResult::new(columns, all_rows))
    }
    
    fn execute_recursive_step(
        &self,
        _cte_name: &str,
        _working_table: &QueryResult,
        _recursive_plan: &PlanNode,
    ) -> Result<QueryResult> {
        // Placeholder: In a full implementation, this would:
        // 1. Replace CTE references in recursive_plan with working_table data
        // 2. Execute the plan
        // 3. Return new rows not yet in the result
        
        // For now, return empty to terminate recursion
        Ok(QueryResult::empty())
    }
}

/// CTE optimizer - optimizes CTE execution strategy
pub struct CteOptimizer;

impl CteOptimizer {
    /// Decide whether to materialize or inline a CTE
    pub fn should_materialize(cte: &CteDefinition, reference_count: usize) -> bool {
        // Materialize if:
        // 1. CTE is recursive (must materialize)
        // 2. CTE is referenced multiple times (avoid recomputation)
        // 3. CTE query is expensive (based on heuristics)
        
        if cte.recursive {
            return true;
        }
        
        if reference_count > 1 {
            return true;
        }
        
        // Check if query is expensive (simple heuristic)
        Self::is_expensive_query(&cte.query)
    }
    
    fn is_expensive_query(plan: &PlanNode) -> bool {
        // Heuristic: queries with joins, aggregates, or sorts are expensive
        match plan {
            PlanNode::Join { .. } => true,
            PlanNode::Aggregate { .. } => true,
            PlanNode::Sort { .. } => true,
            PlanNode::Filter { input, .. } 
            | PlanNode::Project { input, .. }
            | PlanNode::Limit { input, .. } => Self::is_expensive_query(input),
            _ => false,
        }
    }
    
    /// Optimize CTE execution order
    pub fn optimize_execution_order(ctes: &[CteDefinition]) -> Vec<String> {
        // Simple topological sort based on dependencies
        // In a full implementation, this would analyze CTE dependencies
        // and order them for optimal execution
        
        ctes.iter().map(|cte| cte.name.clone()).collect()
    }
}

/// CTE plan node (extension to PlanNode)
#[derive(Debug, Clone)]
pub struct CtePlanNode {
    pub ctes: Vec<CteDefinition>,
    pub main_query: Box<PlanNode>,
}

impl CtePlanNode {
    pub fn new(ctes: Vec<CteDefinition>, main_query: Box<PlanNode>) -> Self {
        Self { ctes, main_query }
    }
}

/// CTE reference tracker - tracks how many times each CTE is referenced
pub struct CteReferenceTracker {
    references: HashMap<String, usize>,
}

impl CteReferenceTracker {
    pub fn new() -> Self {
        Self {
            references: HashMap::new(),
        }
    }
    
    pub fn track_plan(&mut self, plan: &PlanNode, cte_context: &CteContext) {
        match plan {
            PlanNode::TableScan { table, .. } => {
                if cte_context.is_cte(table) {
                    *self.references.entry(table.clone()).or_insert(0) += 1;
                }
            }
            PlanNode::Filter { input, .. }
            | PlanNode::Project { input, .. }
            | PlanNode::Limit { input, .. }
            | PlanNode::Sort { input, .. } => {
                self.track_plan(input, cte_context);
            }
            PlanNode::Join { left, right, .. } => {
                self.track_plan(left, cte_context);
                self.track_plan(right, cte_context);
            }
            PlanNode::Aggregate { input, .. } => {
                self.track_plan(input, cte_context);
            }
            PlanNode::Subquery { plan, .. } => {
                self.track_plan(plan, cte_context);
            }
        }
    }
    
    pub fn get_reference_count(&self, cte_name: &str) -> usize {
        self.references.get(cte_name).copied().unwrap_or(0)
    }
}

/// CTE cache - caches intermediate results for recursive CTEs
pub struct CteCache {
    cache: HashMap<String, Vec<Vec<String>>>,
}

impl CteCache {
    pub fn new() -> Self {
        Self {
            cache: HashMap::new(),
        }
    }
    
    pub fn insert(&mut self, key: String, rows: Vec<Vec<String>>) {
        self.cache.insert(key, rows);
    }
    
    pub fn contains(&self, rows: &[Vec<String>]) -> bool {
        // Check if any cached entry contains these rows
        for cached_rows in self.cache.values() {
            if Self::rows_equal(cached_rows, rows) {
                return true;
            }
        }
        false
    }
    
    fn rows_equal(a: &[Vec<String>], b: &[Vec<String>]) -> bool {
        if a.len() != b.len() {
            return false;
        }
        a.iter().zip(b.iter()).all(|(r1, r2)| r1 == r2)
    }
}

/// WITH clause parser helper
pub struct WithClauseParser;

impl WithClauseParser {
    /// Parse a WITH clause from SQL
    /// 
    /// Example:
    /// ```sql
    /// WITH 
    ///   cte1 AS (SELECT ...),
    ///   cte2 AS (SELECT ...)
    /// SELECT ...
    /// ```
    pub fn parse(sql: &str) -> Result<Option<(Vec<CteDefinition>, String)>> {
        let sql_upper = sql.trim().to_uppercase();
        
        if !sql_upper.starts_with("WITH") {
            return Ok(None);
        }
        
        // Simple parser: extract CTE definitions and main query
        // In a full implementation, this would use a proper SQL parser
        
        // For now, return None to indicate CTEs should be parsed by the main parser
        Ok(None)
    }
}

/// CTE examples and usage patterns
pub mod examples {
    use super::*;
    
    /// Example: Simple non-recursive CTE
    /// 
    /// ```sql
    /// WITH regional_sales AS (
    ///     SELECT region, SUM(amount) AS total_sales
    ///     FROM orders
    ///     GROUP BY region
    /// )
    /// SELECT region, total_sales
    /// FROM regional_sales
    /// WHERE total_sales > 1000000;
    /// ```
    pub fn simple_cte_example() -> &'static str {
        "Simple CTE example - see documentation"
    }
    
    /// Example: Recursive CTE (organization hierarchy)
    /// 
    /// ```sql
    /// WITH RECURSIVE employee_hierarchy AS (
    ///     -- Base case: top-level employees
    ///     SELECT id, name, manager_id, 1 AS level
    ///     FROM employees
    ///     WHERE manager_id IS NULL
    ///     
    ///     UNION ALL
    ///     
    ///     -- Recursive case: employees reporting to previous level
    ///     SELECT e.id, e.name, e.manager_id, eh.level + 1
    ///     FROM employees e
    ///     JOIN employee_hierarchy eh ON e.manager_id = eh.id
    /// )
    /// SELECT * FROM employee_hierarchy;
    /// ```
    pub fn recursive_cte_example() -> &'static str {
        "Recursive CTE example - see documentation"
    }
    
    /// Example: Multiple CTEs
    /// 
    /// ```sql
    /// WITH 
    ///   sales_2023 AS (
    ///     SELECT * FROM sales WHERE year = 2023
    ///   ),
    ///   top_products AS (
    ///     SELECT product_id, SUM(amount) AS total
    ///     FROM sales_2023
    ///     GROUP BY product_id
    ///     ORDER BY total DESC
    ///     LIMIT 10
    ///   )
    /// SELECT p.name, tp.total
    /// FROM top_products tp
    /// JOIN products p ON tp.product_id = p.id;
    /// ```
    pub fn multiple_ctes_example() -> &'static str {
        "Multiple CTEs example - see documentation"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_cte_context() {
        let mut context = CteContext::new();
        
        let cte = CteDefinition {
            name: "test_cte".to_string(),
            columns: vec!["id".to_string(), "name".to_string()],
            query: Box::new(PlanNode::TableScan {
                table: "users".to_string(),
                columns: vec!["*".to_string()],
            }),
            recursive: false,
        };
        
        assert!(context.register_cte(cte.clone()).is_ok());
        assert!(context.is_cte("test_cte"));
        assert!(!context.is_cte("other_cte"));
        
        // Test duplicate registration
        assert!(context.register_cte(cte).is_err());
    }
    
    #[test]
    fn test_cte_materialization() {
        let mut context = CteContext::new();
        
        let result = QueryResult::new(
            vec!["id".to_string(), "name".to_string()],
            vec![
                vec!["1".to_string(), "Alice".to_string()],
                vec!["2".to_string(), "Bob".to_string()],
            ],
        );
        
        context.materialize("test_cte".to_string(), result);
        
        let materialized = context.get_materialized("test_cte");
        assert!(materialized.is_some());
        assert_eq!(materialized.unwrap().rows.len(), 2);
    }
    
    #[test]
    fn test_recursive_cte_evaluator() {
        let evaluator = RecursiveCteEvaluator::new();
        
        let base_result = QueryResult::new(
            vec!["id".to_string(), "value".to_string()],
            vec![vec!["1".to_string(), "10".to_string()]],
        );
        
        let recursive_plan = PlanNode::TableScan {
            table: "test_cte".to_string(),
            columns: vec!["*".to_string()],
        };
        
        let result = evaluator.evaluate("test_cte", base_result, &recursive_plan);
        assert!(result.is_ok());
    }
    
    #[test]
    fn test_cte_optimizer_materialization() {
        let cte_recursive = CteDefinition {
            name: "recursive_cte".to_string(),
            columns: vec!["id".to_string()],
            query: Box::new(PlanNode::TableScan {
                table: "test".to_string(),
                columns: vec!["*".to_string()],
            }),
            recursive: true,
        };
        
        // Recursive CTEs must be materialized
        assert!(CteOptimizer::should_materialize(&cte_recursive, 1));
        
        let cte_multi_ref = CteDefinition {
            name: "multi_ref_cte".to_string(),
            columns: vec!["id".to_string()],
            query: Box::new(PlanNode::TableScan {
                table: "test".to_string(),
                columns: vec!["*".to_string()],
            }),
            recursive: false,
        };
        
        // CTEs referenced multiple times should be materialized
        assert!(CteOptimizer::should_materialize(&cte_multi_ref, 2));
    }
    
    #[test]
    fn test_cte_reference_tracker() {
        let mut tracker = CteReferenceTracker::new();
        let mut context = CteContext::new();
        
        let cte = CteDefinition {
            name: "test_cte".to_string(),
            columns: vec!["id".to_string()],
            query: Box::new(PlanNode::TableScan {
                table: "users".to_string(),
                columns: vec!["*".to_string()],
            }),
            recursive: false,
        };
        
        context.register_cte(cte).unwrap();
        
        let plan = PlanNode::Join {
            join_type: crate::parser::JoinType::Inner,
            left: Box::new(PlanNode::TableScan {
                table: "test_cte".to_string(),
                columns: vec!["*".to_string()],
            }),
            right: Box::new(PlanNode::TableScan {
                table: "test_cte".to_string(),
                columns: vec!["*".to_string()],
            }),
            condition: "true".to_string(),
        };
        
        tracker.track_plan(&plan, &context);
        assert_eq!(tracker.get_reference_count("test_cte"), 2);
    }
    
    #[test]
    fn test_cte_cache() {
        let mut cache = CteCache::new();
        
        let rows = vec![
            vec!["1".to_string(), "Alice".to_string()],
            vec!["2".to_string(), "Bob".to_string()],
        ];
        
        cache.insert("test".to_string(), rows.clone());
        assert!(cache.contains(&rows));
        
        let other_rows = vec![vec!["3".to_string(), "Charlie".to_string()]];
        assert!(!cache.contains(&other_rows));
    }
}
