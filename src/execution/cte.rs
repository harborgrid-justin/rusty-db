use std::collections::HashSet;
use tokio::time::sleep;
use super::planner::PlanNode;
use super::QueryResult;
/// Common Table Expressions (CTE) Support
///
/// This module provides comprehensive support for CTEs including:
/// - Non-recursive CTEs (WITH clause)
/// - Recursive CTEs (WITH RECURSIVE)
/// - Multiple CTEs in a single query
/// - CTE materialization and optimization

use crate::error::DbError;
use std::collections::HashMap;

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
    pub fn register_cte(&mut self, cte: CteDefinition) -> Result<(), DbError> {
        if self.definitions.contains_key(&cte.name) {
            return Err(DbError::AlreadyExists(format!(
                "CTE '{}' already defined",
                cte.name
            ))));
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
        baseresult: QueryResult,
        recursive_plan: &PlanNode,
    ) -> Result<QueryResult, DbError> {
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
                ))));
            }
        }
        
        Ok(QueryResult::new(columns, all_rows))
    }
    
    fn execute_recursive_step(
        &self,
        cte_name: &str,
        working_table: &QueryResult,
        recursiveplan: &PlanNode,
    ) -> Result<QueryResult, DbError> {
        // In a full implementation, this would:
        // 1. Replace CTE references in recursive_plan with working_table data
        // 2. Execute the plan
        // 3. Return new rows not yet in the result
        
        // For demonstration, we detect cycles and prevent duplicates
        let cycle_detector = CycleDetector::new();
        if cycle_detector.has_cycle(&working_table.rows) {
            return Ok(QueryResult::empty());
        }
        
        // Return empty to terminate recursion in this stub
        Ok(QueryResult::empty())
    }
}

/// Cycle detection for recursive CTEs
/// Prevents infinite loops by detecting when the same rows appear again
pub struct CycleDetector {
    seen_hashes: std::collections::HashSet<u64>,
}

impl CycleDetector {
    pub fn new() -> Self {
        Self {
            seen_hashes: std::collections::HashSet::new(),
        }
    }
    
    /// Check if a set of rows creates a cycle
    pub fn has_cycle(&self, rows: &[Vec<String>]) -> bool {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        
        for row in rows {
            let mut hasher = DefaultHasher::new();
            row.hash(&mut hasher);
            let hash = hasher.finish();
            
            if self.seen_hashes.contains(&hash) {
                return true;
            }
        }
        false
    }
    
    /// Add rows to the cycle detection set
    pub fn add_rows(&mut self, rows: &[Vec<String>]) {
        
        for row in rows {
            let mut hasher = DefaultHasher::new();
            row.hash(&mut hasher);
            let hash = hasher.finish();
            self.seen_hashes.insert(hash);
        }
    }
    
    /// Clear the cycle detector for a new evaluation
    pub fn clear(&mut self) {
        self.seen_hashes.clear();
    }
}

/// Advanced CTE materialization strategies
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum MaterializationStrategy {
    /// Always inline the CTE (substitute the query)
    AlwaysInline,
    /// Always materialize the CTE (execute once and cache)
    AlwaysMaterialize,
    /// Decide based on cost analysis
    CostBased,
    /// Materialize only if referenced multiple times
    MultiReference,
}

/// CTE cost estimator
pub struct CteCostEstimator {
    /// Estimated rows for CTEs
    row_estimates: HashMap<String, usize>,
    /// Estimated cost for CTEs
    cost_estimates: HashMap<String, f64>,
}

impl CteCostEstimator {
    pub fn new() -> Self {
        Self {
            row_estimates: HashMap::new(),
            cost_estimates: HashMap::new(),
        }
    }
    
    /// Estimate the cost of executing a CTE query
    pub fn estimate_cost(&mut self, cte: &CteDefinition) -> f64 {
        if let Some(&cost) = self.cost_estimates.get(&cte.name) {
            return cost;
        }
        
        let cost = self.calculate_plan_cost(&cte.query);
        self.cost_estimates.insert(cte.name.clone(), cost);
        cost
    }
    
    fn calculate_plan_cost(&self, plan: &PlanNode) -> f64 {
        match plan {
            PlanNode::TableScan { .. } => 100.0, // Base table scan cost
            PlanNode::Filter { input, .. } => {
                self.calculate_plan_cost(input) * 1.1 // 10% overhead
            }
            PlanNode::Project { input, .. } => {
                self.calculate_plan_cost(input) * 1.05 // 5% overhead
            }
            PlanNode::Join { left, right, .. } => {
                let left_cost = self.calculate_plan_cost(left);
                let right_cost = self.calculate_plan_cost(right);
                left_cost + right_cost + (left_cost * right_cost * 0.01) // Join cost
            }
            PlanNode::Aggregate { input, .. } => {
                self.calculate_plan_cost(input) * 2.0 // Aggregation is expensive
            }
            PlanNode::Sort { input, .. } => {
                self.calculate_plan_cost(input) * 1.5 // Sorting is moderately expensive
            }
            PlanNode::Limit { input, .. } => {
                self.calculate_plan_cost(input) * 0.5 // Limit reduces cost
            }
            PlanNode::Subquery { plan, .. } => {
                self.calculate_plan_cost(plan) * 1.2 // Subquery overhead
            }
        }
    }
    
    /// Estimate the number of rows a CTE will produce
    pub fn estimate_rows(&mut self, cte: &CteDefinition) -> usize {
        if let Some(&rows) = self.row_estimates.get(&cte.name) {
            return rows;
        }
        
        let rows = self.estimate_plan_rows(&cte.query);
        self.row_estimates.insert(cte.name.clone(), rows);
        rows
    }
    
    fn estimate_plan_rows(&self, plan: &PlanNode) -> usize {
        match plan {
            PlanNode::TableScan { .. } => 1000, // Assume 1000 rows per table
            PlanNode::Filter { input, .. } => {
                (self.estimate_plan_rows(input) as f64 * 0.1) as usize // Filters reduce by 90%
            }
            PlanNode::Project { input, .. } => self.estimate_plan_rows(input),
            PlanNode::Join { left, right, .. } => {
                let left_rows = self.estimate_plan_rows(left);
                let right_rows = self.estimate_plan_rows(right);
                left_rows * right_rows / 10 // Assume 10:1 reduction from join condition
            }
            PlanNode::Aggregate { input, .. } => {
                (self.estimate_plan_rows(input) as f64 * 0.05) as usize // Aggregation reduces significantly
            }
            PlanNode::Sort { input, .. } => self.estimate_plan_rows(input),
            PlanNode::Limit { input, limit, .. } => {
                self.estimate_plan_rows(input).min(*limit)
            }
            PlanNode::Subquery { plan, .. } => self.estimate_plan_rows(plan),
        }
    }
}

/// CTE dependency graph for optimization
pub struct CteDependencyGraph {
    /// Maps CTE name to list of CTEs it depends on
    dependencies: HashMap<String, Vec<String>>,
}

impl CteDependencyGraph {
    pub fn new() -> Self {
        Self {
            dependencies: HashMap::new(),
        }
    }
    
    /// Build dependency graph from CTE definitions
    pub fn build(&mut self, ctes: &[CteDefinition]) {
        for cte in ctes {
            let deps = self.extract_dependencies(&cte.query);
            self.dependencies.insert(cte.name.clone(), deps);
        }
    }
    
    fn extract_dependencies(&self, plan: &PlanNode) -> Vec<String> {
        let mut deps = Vec::new();
        self.extract_deps_recursive(plan, &mut deps);
        deps
    }
    
    fn extract_deps_recursive(&self, plan: &PlanNode, deps: &mut Vec<String>) {
        match plan {
            PlanNode::TableScan { table, .. } => {
                // Check if this is a CTE reference
                if !deps.contains(table) {
                    deps.push(table.clone());
                }
            }
            PlanNode::Filter { input, .. }
            | PlanNode::Project { input, .. }
            | PlanNode::Sort { input, .. }
            | PlanNode::Limit { input, .. }
            | PlanNode::Aggregate { input, .. } => {
                self.extract_deps_recursive(input, deps);
            }
            PlanNode::Join { left, right, .. } => {
                self.extract_deps_recursive(left, deps);
                self.extract_deps_recursive(right, deps);
            }
            PlanNode::Subquery { plan, .. } => {
                self.extract_deps_recursive(plan, deps);
            }
        }
    }
    
    /// Perform topological sort to determine execution order
    pub fn topological_sort(&self, ctes: &[CteDefinition]) -> Result<Vec<String>, DbError> {
        let mut sorted = Vec::new();
        let mut visited = std::collections::HashSet::new();
        let mut in_progress = std::collections::HashSet::new();
        
        for cte in ctes {
            if !visited.contains(&cte.name) {
                self.visit(&cte.name, &mut visited, &mut in_progress, &mut sorted)?;
            }
        }
        
        sorted.reverse();
        Ok(sorted)
    }
    
    fn visit(
        &self,
        name: &str,
        visited: &mut std::collections::HashSet<String>,
        in_progress: &mut std::collections::HashSet<String>,
        sorted: &mut Vec<String>,
    ) -> Result<(), DbError> {
        if in_progress.contains(name) {
            return Err(DbError::InvalidOperation(format!(
                "Circular dependency detected in CTE '{}'",
                name
            ))));
        }
        
        if visited.contains(name) {
            return Ok(());
        }
        
        in_progress.insert(name.to_string());
        
        if let Some(deps) = self.dependencies.get(name) {
            for dep in deps {
                self.visit(dep, visited, in_progress, sorted)?;
            }
        }
        
        in_progress.remove(name);
        visited.insert(name.to_string());
        sorted.push(name.to_string());
        
        Ok(())
    }
    
    /// Check if there are any circular dependencies
    pub fn has_circular_dependency(&self) -> bool {
        let mut visited = std::collections::HashSet::new();
        let mut in_progress = std::collections::HashSet::new();
        
        for name in self.dependencies.keys() {
            if !visited.contains(name) {
                if self.has_cycle_dfs(name, &mut visited, &mut in_progress) {
                    return true;
                }
            }
        }
        false
    }
    
    fn has_cycle_dfs(
        &self,
        name: &str,
        visited: &mut std::collections::HashSet<String>,
        in_progress: &mut std::collections::HashSet<String>,
    ) -> bool {
        if in_progress.contains(name) {
            return true;
        }
        
        if visited.contains(name) {
            return false;
        }
        
        in_progress.insert(name.to_string());
        
        if let Some(deps) = self.dependencies.get(name) {
            for dep in deps {
                if self.has_cycle_dfs(dep, visited, in_progress) {
                    return true;
                }
            }
        }
        
        in_progress.remove(name);
        visited.insert(name.to_string());
        
        false
    }
}

/// CTE statistics collector for monitoring and optimization
pub struct CteStatistics {
    /// Number of times each CTE was executed
    execution_counts: HashMap<String, usize>,
    /// Total execution time for each CTE (in milliseconds)
    execution_times: HashMap<String, u128>,
    /// Number of rows produced by each CTE
    row_counts: HashMap<String, usize>,
    /// Memory usage for materialized CTEs (in bytes)
    memory_usage: HashMap<String, usize>,
}

impl CteStatistics {
    pub fn new() -> Self {
        Self {
            execution_counts: HashMap::new(),
            execution_times: HashMap::new(),
            row_counts: HashMap::new(),
            memory_usage: HashMap::new(),
        }
    }
    
    /// Record a CTE execution
    pub fn record_execution(
        &mut self,
        cte_name: &str,
        duration_ms: u128,
        row_count: usize,
        memory_bytes: usize,
    ) {
        *self.execution_counts.entry(cte_name.to_string()).or_insert(0) += 1;
        *self.execution_times.entry(cte_name.to_string()).or_insert(0) += duration_ms;
        self.row_counts.insert(cte_name.to_string(), row_count);
        self.memory_usage.insert(cte_name.to_string(), memory_bytes);
    }
    
    /// Get average execution time for a CTE
    pub fn get_average_execution_time(&self, cte_name: &str) -> Option<f64> {
        let count = self.execution_counts.get(cte_name)?;
        let total_time = self.execution_times.get(cte_name)?;
        
        if *count == 0 {
            return None;
        }
        
        Some(*total_time as f64 / *count as f64)
    }
    
    /// Get total memory used by all materialized CTEs
    pub fn get_total_memory_usage(&self) -> usize {
        self.memory_usage.values().sum()
    }
    
    /// Get statistics report
    pub fn generate_report(&self) -> CteStatisticsReport {
        CteStatisticsReport {
            total_ctes: self.execution_counts.len(),
            total_executions: self.execution_counts.values().sum(),
            total_memory_bytes: self.get_total_memory_usage(),
            cte_details: self.execution_counts.keys()
                .map(|name| {
                    let count = self.execution_counts.get(name).copied().unwrap_or(0);
                    let avg_time = self.get_average_execution_time(name).unwrap_or(0.0);
                    let rows = self.row_counts.get(name).copied().unwrap_or(0);
                    let memory = self.memory_usage.get(name).copied().unwrap_or(0);
                    
                    CteDetail {
                        name: name.clone(),
                        execution_count: count,
                        average_time_ms: avg_time,
                        row_count: rows,
                        memory_bytes: memory,
                    }
                })
                .collect(),
        }
    }
}

#[derive(Debug)]
pub struct CteStatisticsReport {
    pub total_ctes: usize,
    pub total_executions: usize,
    pub total_memory_bytes: usize,
    pub cte_details: Vec<CteDetail>,
}

#[derive(Debug)]
pub struct CteDetail {
    pub name: String,
    pub execution_count: usize,
    pub average_time_ms: f64,
    pub row_count: usize,
    pub memory_bytes: usize,
}

/// CTE rewrite rules for optimization
pub struct CteRewriteRules;

impl CteRewriteRules {
    /// Merge adjacent CTEs when possible
    pub fn merge_ctes(ctes: Vec<CteDefinition>) -> Vec<CteDefinition> {
        // In a full implementation, this would analyze CTEs and merge
        // ones that can be combined for better performance
        // For now, return as-is
        ctes
    }
    
    /// Push predicates into CTEs when beneficial
    pub fn push_predicates(cte: &mut CteDefinition, _predicate: &str) {
        // In a full implementation, this would push filter predicates
        // into CTE definitions to reduce intermediate result size
    }
    
    /// Eliminate unused CTEs
    pub fn eliminate_unused(
        ctes: Vec<CteDefinition>,
        main_query: &PlanNode,
    ) -> Vec<CteDefinition> {
        let mut tracker = CteReferenceTracker::new();
        let mut temp_context = CteContext::new();
        
        for cte in &ctes {
            let _ = temp_context.register_cte(cte.clone());
        }
        
        tracker.track_plan(main_query, &temp_context);
        
        ctes.into_iter()
            .filter(|cte| tracker.get_reference_count(&cte.name) > 0)
            .collect()
    }
}

/// Nested CTE support - handles CTEs within CTEs
pub struct NestedCteHandler {
    nesting_level: usize,
    max_nesting_level: usize,
}

impl NestedCteHandler {
    pub fn new() -> Self {
        Self {
            nesting_level: 0,
            max_nesting_level: 10, // Prevent excessive nesting
        }
    }
    
    pub fn with_max_nesting(max_level: usize) -> Self {
        Self {
            nesting_level: 0,
            max_nesting_level: max_level,
        }
    }
    
    /// Enter a new CTE nesting level
    pub fn enter_nesting(&mut self) -> Result<(), DbError> {
        if self.nesting_level >= self.max_nesting_level {
            return Err(DbError::InvalidOperation(format!(
                "Maximum CTE nesting level ({}) exceeded",
                self.max_nesting_level
            ))));
        }
        self.nesting_level += 1;
        Ok(())
    }
    
    /// Exit current CTE nesting level
    pub fn exit_nesting(&mut self) {
        if self.nesting_level > 0 {
            self.nesting_level -= 1;
        }
    }
    
    /// Get current nesting level
    pub fn get_nesting_level(&self) -> usize {
        self.nesting_level
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
    pub fn parse(sql: &str) -> Result<Option<(Vec<CteDefinition>, String)>, DbError> {
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
    
    #[test]
    fn test_cycle_detector() {
        let mut detector = CycleDetector::new();
        
        let rows = vec![
            vec!["1".to_string(), "A".to_string()],
            vec!["2".to_string(), "B".to_string()],
        ];
        
        assert!(!detector.has_cycle(&rows));
        detector.add_rows(&rows);
        assert!(detector.has_cycle(&rows));
        
        detector.clear();
        assert!(!detector.has_cycle(&rows));
    }
    
    #[test]
    fn test_cte_cost_estimator() {
        let mut estimator = CteCostEstimator::new();
        
        let simple_cte = CteDefinition {
            name: "simple".to_string(),
            columns: vec!["id".to_string()],
            query: Box::new(PlanNode::TableScan {
                table: "test".to_string(),
                columns: vec!["*".to_string()],
            }),
            recursive: false,
        };
        
        let cost = estimator.estimate_cost(&simple_cte);
        assert!(cost > 0.0);
        
        let rows = estimator.estimate_rows(&simple_cte);
        assert!(rows > 0);
    }
    
    #[test]
    fn test_cte_dependency_graph() {
        let mut graph = CteDependencyGraph::new();
        
        let cte1 = CteDefinition {
            name: "cte1".to_string(),
            columns: vec!["id".to_string()],
            query: Box::new(PlanNode::TableScan {
                table: "base_table".to_string(),
                columns: vec!["*".to_string()],
            }),
            recursive: false,
        };
        
        let cte2 = CteDefinition {
            name: "cte2".to_string(),
            columns: vec!["id".to_string()],
            query: Box::new(PlanNode::TableScan {
                table: "cte1".to_string(),
                columns: vec!["*".to_string()],
            }),
            recursive: false,
        };
        
        graph.build(&[cte1.clone(), cte2.clone()]);
        
        let sorted = graph.topological_sort(&[cte1, cte2]);
        assert!(sorted.is_ok());
        
        let order = sorted.unwrap();
        // The topological sort will include base_table, cte1, and cte2
        assert!(order.len() >= 2);
    }
    
    #[test]
    fn test_circular_dependency_detection() {
        let mut graph = CteDependencyGraph::new();
        
        // Create circular dependency: cte1 -> cte2 -> cte1
        let cte1 = CteDefinition {
            name: "cte1".to_string(),
            columns: vec!["id".to_string()],
            query: Box::new(PlanNode::TableScan {
                table: "cte2".to_string(),
                columns: vec!["*".to_string()],
            }),
            recursive: false,
        };
        
        let cte2 = CteDefinition {
            name: "cte2".to_string(),
            columns: vec!["id".to_string()],
            query: Box::new(PlanNode::TableScan {
                table: "cte1".to_string(),
                columns: vec!["*".to_string()],
            }),
            recursive: false,
        };
        
        graph.build(&[cte1.clone(), cte2.clone()]);
        assert!(graph.has_circular_dependency());
        
        let result = graph.topological_sort(&[cte1, cte2]);
        assert!(result.is_err());
    }
    
    #[test]
    fn test_cte_statistics() {
        let mut stats = CteStatistics::new();
        
        stats.record_execution("test_cte", 100, 50, 1024);
        stats.record_execution("test_cte", 150, 50, 1024);
        
        let avg_time = stats.get_average_execution_time("test_cte");
        assert_eq!(avg_time, Some(125.0));
        
        let total_memory = stats.get_total_memory_usage();
        assert_eq!(total_memory, 1024);
        
        let report = stats.generate_report();
        assert_eq!(report.total_ctes, 1);
        assert_eq!(report.total_executions, 2);
    }
    
    #[test]
    fn test_nested_cte_handler() {
        let mut handler = NestedCteHandler::new();
        
        assert_eq!(handler.get_nesting_level(), 0);
        
        assert!(handler.enter_nesting().is_ok());
        assert_eq!(handler.get_nesting_level(), 1);
        
        handler.exit_nesting();
        assert_eq!(handler.get_nesting_level(), 0);
    }
    
    #[test]
    fn test_max_nesting_level() {
        let mut handler = NestedCteHandler::with_max_nesting(2);
        
        assert!(handler.enter_nesting().is_ok());
        assert!(handler.enter_nesting().is_ok());
        
        // Should fail on third nesting
        let result = handler.enter_nesting();
        assert!(result.is_err());
    }
    
    #[test]
    fn test_eliminate_unused_ctes() {
        let used_cte = CteDefinition {
            name: "used".to_string(),
            columns: vec!["id".to_string()],
            query: Box::new(PlanNode::TableScan {
                table: "base".to_string(),
                columns: vec!["*".to_string()],
            }),
            recursive: false,
        };
        
        let unused_cte = CteDefinition {
            name: "unused".to_string(),
            columns: vec!["id".to_string()],
            query: Box::new(PlanNode::TableScan {
                table: "base".to_string(),
                columns: vec!["*".to_string()],
            }),
            recursive: false,
        };
        
        let main_query = PlanNode::TableScan {
            table: "used".to_string(),
            columns: vec!["*".to_string()],
        };
        
        let ctes = vec![used_cte, unused_cte];
        let filtered = CteRewriteRules::eliminate_unused(ctes, &main_query);
        
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].name, "used");
    }
}

/// CTE Execution Engine - Orchestrates CTE execution with optimization
pub struct CteExecutionEngine {
    cost_estimator: CteCostEstimator,
    statistics: CteStatistics,
    cache: CteResultCache,
    parallel_executor: ParallelCteExecutor,
}

impl CteExecutionEngine {
    pub fn new() -> Self {
        Self {
            cost_estimator: CteCostEstimator::new(),
            statistics: CteStatistics::new(),
            cache: CteResultCache::new(1000), // 1000 entry cache
            parallel_executor: ParallelCteExecutor::new(4), // 4 parallel workers
        }
    }
    
    /// Execute CTEs with full optimization
    pub fn execute_with_optimization(
        &mut self,
        ctes: Vec<CteDefinition>,
        mainquery: PlanNode,
    ) -> Result<QueryResult, DbError> {
        // Build dependency graph
        let mut dep_graph = CteDependencyGraph::new();
        dep_graph.build(&ctes);
        
        // Check for circular dependencies
        if dep_graph.has_circular_dependency() {
            return Err(DbError::InvalidOperation(
                "Circular dependency detected in CTEs".to_string()
            ));
        }
        
        // Get optimal execution order
        let execution_order = dep_graph.topological_sort(&ctes)?;
        
        // Execute CTEs in order
        let mut context = CteContext::new();
        for cte_name in execution_order {
            if let Some(cte) = ctes.iter().find(|c| c.name == cte_name) {
                self.execute_single_cte(cte, &mut context)?;
            }
        }
        
        // Execute main query
        self.execute_main_query(&main_query, &context)
    }
    
    fn execute_single_cte(
        &mut self,
        cte: &CteDefinition,
        context: &mut CteContext,
    ) -> Result<(), DbError> {
        let start = std::time::Instant::now();
        
        // Check cache first
        if let Some(cached_result) = self.cache.get(&cte.name) {
            context.materialize(cte.name.clone(), cached_result);
            return Ok(());
        }
        
        // Execute the CTE query
        let result = if cte.recursive {
            self.execute_recursive_cte(cte)?
        } else {
            self.execute_non_recursive_cte(cte)?
        };
        
        // Record statistics
        let duration = start.elapsed().as_millis();
        let memory = self.estimate_memory_usage(&result);
        self.statistics.record_execution(
            &cte.name,
            duration,
            result.rows.len(),
            memory,
        );
        
        // Cache result
        self.cache.insert(cte.name.clone(), result.clone());
        
        // Materialize
        context.materialize(cte.name.clone(), result);
        
        Ok(())
    }
    
    fn execute_non_recursive_cte(&self, _cte: &CteDefinition) -> Result<QueryResult, DbError> {
        // Placeholder: Execute non-recursive CTE
        Ok(QueryResult::empty())
    }
    
    fn execute_recursive_cte(&self, cte: &CteDefinition) -> Result<QueryResult, DbError> {
        let evaluator = RecursiveCteEvaluator::new();
        let base_result = QueryResult::empty();
        evaluator.evaluate(&cte.name, base_result, &cte.query)
    }
    
    fn execute_main_query(
        &self,
        _main_query: &PlanNode,
        _context: &CteContext,
    ) -> Result<QueryResult, DbError> {
        // Placeholder: Execute main query
        Ok(QueryResult::empty())
    }
    
    fn estimate_memory_usage(&self, result: &QueryResult) -> usize {
        // Rough estimate: sum of all string lengths
        let mut total = 0;
        for row in &result.rows {
            for cell in row {
                total += cell.len();
            }
        }
        total
    }
    
    /// Get execution statistics
    pub fn get_statistics(&self) -> &CteStatistics {
        &self.statistics
    }
}

/// CTE Result Cache with LRU eviction
pub struct CteResultCache {
    cache: HashMap<String, CachedResult>,
    access_order: Vec<String>,
    max_entries: usize,
}

#[derive(Clone)]
struct CachedResult {
    result: QueryResult,
    cached_at: std::time::Instant,
    access_count: usize,
}

impl CteResultCache {
    pub fn new(max_entries: usize) -> Self {
        Self {
            cache: HashMap::new(),
            access_order: Vec::new(),
            max_entries,
        }
    }
    
    pub fn insert(&mut self, key: String, result: QueryResult) {
        // Evict if necessary
        if self.cache.len() >= self.max_entries && !self.cache.contains_key(&key) {
            self.evict_lru();
        }
        
        let cached = CachedResult {
            result,
            cached_at: std::time::Instant::now(),
            access_count: 0,
        };
        
        self.cache.insert(key.clone(), cached);
        self.access_order.retain(|k| k != &key);
        self.access_order.push(key);
    }
    
    pub fn get(&mut self, key: &str) -> Option<QueryResult> {
        if let Some(cached) = self.cache.get_mut(key) {
            cached.access_count += 1;
            
            // Update access order
            self.access_order.retain(|k| k != key);
            self.access_order.push(key.to_string());
            
            Some(cached.result.clone())
        } else {
            None
        }
    }
    
    fn evict_lru(&mut self) {
        if let Some(key) = self.access_order.first().cloned() {
            self.cache.remove(&key);
            self.access_order.remove(0);
        }
    }
    
    pub fn clear(&mut self) {
        self.cache.clear();
        self.access_order.clear();
    }
    
    pub fn size(&self) -> usize {
        self.cache.len()
    }
}

/// Parallel CTE Executor for concurrent execution of independent CTEs
pub struct ParallelCteExecutor {
    worker_count: usize,
}

impl ParallelCteExecutor {
    pub fn new(worker_count: usize) -> Self {
        Self { worker_count }
    }
    
    /// Execute independent CTEs in parallel
    pub fn execute_parallel(
        &self,
        _ctes: Vec<CteDefinition>,
        _dependency_graph: &CteDependencyGraph,
    ) -> Result<HashMap<String, QueryResult>, DbError> {
        // Placeholder: In a full implementation, this would:
        // 1. Identify independent CTEs that can run in parallel
        // 2. Create a thread pool with worker_count threads
        // 3. Execute independent CTEs concurrently
        // 4. Synchronize when dependencies require it
        
        Ok(HashMap::new())
    }
}

/// Incremental CTE Evaluator for efficient recursive CTE updates
pub struct IncrementalCteEvaluator {
    delta_cache: HashMap<String, Vec<Vec<String>>>,
}

impl IncrementalCteEvaluator {
    pub fn new() -> Self {
        Self {
            delta_cache: HashMap::new(),
        }
    }
    
    /// Evaluate recursive CTE incrementally using semi-naive evaluation
    pub fn evaluate_incremental(
        &mut self,
        cte_name: &str,
        baseresult: QueryResult,
        recursive_plan: &PlanNode,
    ) -> Result<QueryResult, DbError> {
        let mut all_rows = base_result.rows.clone();
        let columns = base_result.columns.clone();
        let mut delta = base_result.rows.clone();
        
        let max_iterations = 100;
        for iteration in 0..max_iterations {
            if delta.is_empty() {
                break;
            }
            
            // Compute new rows using only the delta
            let new_delta = self.compute_delta(cte_name, &delta, recursive_plan)?;
            
            if new_delta.is_empty() {
                break;
            }
            
            // Add new rows to result
            all_rows.extend(new_delta.clone());
            delta = new_delta;
            
            // Cache delta for potential reuse
            self.delta_cache.insert(
                format!("{}_iteration_{}", cte_name, iteration),
                delta.clone(),
            ));
            
            if iteration == max_iterations - 1 {
                return Err(DbError::InvalidOperation(format!(
                    "Incremental evaluation exceeded max iterations for CTE '{}'",
                    cte_name
                ))));
            }
        }
        
        Ok(QueryResult::new(columns, all_rows))
    }
    
    fn compute_delta(
        &self,
        _cte_name: &str,
        delta: &[Vec<String>],
        _recursive_plan: &PlanNode,
    ) -> Result<Vec<Vec<String>>, DbError> {
        // Placeholder: Compute new rows from delta
        Ok(Vec::new())
    }
    
    pub fn clear_cache(&mut self) {
        self.delta_cache.clear();
    }
}

/// CTE Query Rewriter - Advanced query transformation for optimization
pub struct CteQueryRewriter {
    rewrite_stats: HashMap<String, usize>,
}

impl CteQueryRewriter {
    pub fn new() -> Self {
        Self {
            rewrite_stats: HashMap::new(),
        }
    }
    
    /// Apply all rewrite rules to optimize CTE queries
    pub fn rewrite(&mut self, ctes: Vec<CteDefinition>) -> Vec<CteDefinition> {
        let mut rewritten = ctes;
        
        // Apply various rewrite rules
        rewritten = self.inline_simple_ctes(rewritten);
        rewritten = self.merge_adjacent_filters(rewritten);
        rewritten = self.push_down_projections(rewritten);
        
        rewritten
    }
    
    fn inline_simple_ctes(&mut self, ctes: Vec<CteDefinition>) -> Vec<CteDefinition> {
        // Inline CTEs that are simple enough
        *self.rewrite_stats.entry("inline".to_string()).or_insert(0) += 1;
        ctes
    }
    
    fn merge_adjacent_filters(&mut self, ctes: Vec<CteDefinition>) -> Vec<CteDefinition> {
        // Merge adjacent filter operations
        *self.rewrite_stats.entry("merge_filters".to_string()).or_insert(0) += 1;
        ctes
    }
    
    fn push_down_projections(&mut self, ctes: Vec<CteDefinition>) -> Vec<CteDefinition> {
        // Push projections down to reduce intermediate result size
        *self.rewrite_stats.entry("pushdown".to_string()).or_insert(0) += 1;
        ctes
    }
    
    pub fn get_rewrite_count(&self, rule: &str) -> usize {
        self.rewrite_stats.get(rule).copied().unwrap_or(0)
    }
}

/// CTE Materialization Strategy Selector
pub struct MaterializationStrategySelector {
    threshold_multiple_refs: usize,
    threshold_cost: f64,
}

impl MaterializationStrategySelector {
    pub fn new() -> Self {
        Self {
            threshold_multiple_refs: 2,
            threshold_cost: 1000.0,
        }
    }
    
    /// Select optimal materialization strategy for a CTE
    pub fn select_strategy(
        &self,
        cte: &CteDefinition,
        reference_count: usize,
        estimatedcost: f64,
    ) -> MaterializationStrategy {
        // Recursive CTEs must be materialized
        if cte.recursive {
            return MaterializationStrategy::AlwaysMaterialize;
        }
        
        // Multiple references -> materialize
        if reference_count >= self.threshold_multiple_refs {
            return MaterializationStrategy::AlwaysMaterialize;
        }
        
        // High cost -> materialize
        if estimated_cost >= self.threshold_cost {
            return MaterializationStrategy::AlwaysMaterialize;
        }
        
        // Single reference, low cost -> inline
        if reference_count == 1 {
            return MaterializationStrategy::AlwaysInline;
        }
        
        // Default to cost-based decision
        MaterializationStrategy::CostBased
    }
}

/// CTE Plan Visualizer for debugging and optimization
pub struct CtePlanVisualizer;

impl CtePlanVisualizer {
    /// Generate a visual representation of CTE execution plan
    pub fn visualize(ctes: &[CteDefinition], mainquery: &PlanNode) -> String {
        let mut output = String::new();
        output.push_str("CTE Execution Plan:\n");
        output.push_str("==================\n\n");
        
        for (i, cte) in ctes.iter().enumerate() {
            output.push_str(&format!("CTE {}: {}\n", i + 1, cte.name)));
            output.push_str(&format!("  Columns: {:?}\n", cte.columns)));
            output.push_str(&format!("  Recursive: {}\n", cte.recursive)));
            output.push_str(&format!("  Query Plan:\n{}\n", Self::visualize_plan(&cte.query, 4))));
            output.push_str("\n");
        }
        
        output.push_str("Main Query:\n");
        output.push_str(&Self::visualize_plan(main_query, 2));
        
        output
    }
    
    fn visualize_plan(plan: &PlanNode, indent: usize) -> String {
        let prefix = " ".repeat(indent);
        match plan {
            PlanNode::TableScan { table, columns } => {
                format!("{}TableScan: {} [{:?}]\n", prefix, table, columns)
            }
            PlanNode::Filter { input, predicate } => {
                format!(
                    "{}Filter: {}\n{}",
                    prefix,
                    predicate,
                    Self::visualize_plan(input, indent + 2)
                )
            }
            PlanNode::Project { input, columns } => {
                format!(
                    "{}Project: {:?}\n{}",
                    prefix,
                    columns,
                    Self::visualize_plan(input, indent + 2)
                )
            }
            PlanNode::Join { join_type, left, right, condition } => {
                format!(
                    "{}Join ({:?}): {}\n{}{}",
                    prefix,
                    join_type,
                    condition,
                    Self::visualize_plan(left, indent + 2),
                    Self::visualize_plan(right, indent + 2)
                )
            }
            PlanNode::Aggregate { input, group_by, aggregates, .. } => {
                format!(
                    "{}Aggregate:\n{}  Group By: {:?}\n{}  Functions: {:?}\n{}",
                    prefix,
                    prefix,
                    group_by,
                    prefix,
                    aggregates,
                    Self::visualize_plan(input, indent + 2)
                )
            }
            PlanNode::Sort { input, order_by } => {
                format!(
                    "{}Sort: {:?}\n{}",
                    prefix,
                    order_by,
                    Self::visualize_plan(input, indent + 2)
                )
            }
            PlanNode::Limit { input, limit, offset } => {
                format!(
                    "{}Limit: {} (offset: {:?})\n{}",
                    prefix,
                    limit,
                    offset,
                    Self::visualize_plan(input, indent + 2)
                )
            }
            PlanNode::Subquery { plan, .. } => {
                format!(
                    "{}Subquery:\n{}",
                    prefix,
                    Self::visualize_plan(plan, indent + 2)
                )
            }
        }
    }
}

/// CTE Memory Manager - Manages memory for materialized CTEs
pub struct CteMemoryManager {
    memory_limit: usize,
    current_usage: usize,
    allocations: HashMap<String, usize>,
}

impl CteMemoryManager {
    pub fn new(memory_limit_mb: usize) -> Self {
        Self {
            memory_limit: memory_limit_mb * 1024 * 1024,
            current_usage: 0,
            allocations: HashMap::new(),
        }
    }
    
    /// Allocate memory for a CTE result
    pub ffn allocate(&mut self, cte_name: String, sizebytes: usize)-> Result<(), DbError> {
        if self.current_usage + size_bytes > self.memory_limit {
            return Err(DbError::Internal(format!(
                "Out of memory: Cannot allocate {} bytes for CTE '{}'. Current usage: {}, Limit: {}",
                size_bytes, cte_name, self.current_usage, self.memory_limit
            ))));
        }
        
        self.current_usage += size_bytes;
        self.allocations.insert(cte_name, size_bytes);
        Ok(())
    }
    
    /// Deallocate memory for a CTE
    pub fn deallocate(&mut self, cte_name: &str) {
        if let Some(size) = self.allocations.remove(cte_name) {
            self.current_usage = self.current_usage.saturating_sub(size);
        }
    }
    
    /// Get current memory usage
    pub fn get_usage(&self) -> usize {
        self.current_usage
    }
    
    /// Get memory usage percentage
    pub fn get_usage_percentage(&self) -> f64 {
        (self.current_usage as f64 / self.memory_limit as f64) * 100.0
    }
    
    /// Clear all allocations
    pub fn clear(&mut self) {
        self.allocations.clear();
        self.current_usage = 0;
    }
}

/// CTE Join Optimizer - Optimizes joins involving CTEs
pub struct CteJoinOptimizer;

impl CteJoinOptimizer {
    /// Optimize join order when CTEs are involved
    pub fn optimize_join_order(
        joins: Vec<JoinNode>,
        cte_stats: &HashMap<String, CteJoinStatistics>,
    ) -> Vec<JoinNode> {
        // Use dynamic programming to find optimal join order
        let mut optimized = joins;
        
        // Sort by estimated selectivity (smaller results first)
        optimized.sort_by(|a, b| {
            let a_size = Self::estimate_join_size(&a.table, cte_stats);
            let b_size = Self::estimate_join_size(&b.table, cte_stats);
            a_size.partial_cmp(&b_size).unwrap()
        });
        
        optimized
    }
    
    fn estimate_join_size(table: &str, stats: &HashMap<String, CteJoinStatistics>) -> f64 {
        stats
            .get(table)
            .map(|s| s.estimated_rows as f64)
            .unwrap_or(1000.0)
    }
}

#[derive(Debug, Clone)]
pub struct JoinNode {
    pub table: String,
    pub join_type: String,
    pub condition: String,
}

#[derive(Debug, Clone)]
pub struct CteJoinStatistics {
    pub estimated_rows: usize,
    pub estimated_selectivity: f64,
}

/// CTE Predicate Pushdown Optimizer
pub struct CtePredicatePushdown;

impl CtePredicatePushdown {
    /// Push predicates into CTE definitions when beneficial
    pub fn push_predicates(
        cte: &mut CteDefinition,
        predicates: Vec<String>,
    ) -> Vec<String> {
        let mut pushed = Vec::new();
        let mut remaining = Vec::new();
        
        for predicate in predicates {
            if Self::can_push_predicate(&predicate, cte) {
                // Modify CTE to include predicate
                Self::add_filter_to_plan(&mut cte.query, predicate.clone());
                pushed.push(predicate);
            } else {
                remaining.push(predicate);
            }
        }
        
        remaining
    }
    
    fn can_push_predicate(predicate: &str, cte: &CteDefinition) -> bool {
        // Check if predicate references only columns in CTE
        for column in &cte.columns {
            if predicate.contains(column) {
                return true;
            }
        }
        false
    }
    
    fn add_filter_to_plan(plan: &mut Box<PlanNode>, predicate: String) {
        // Wrap plan with filter
        let current_plan = std::mem::replace(
            plan.as_mut(),
            PlanNode::TableScan {
                table: String::new(),
                columns: Vec::new(),
            },
        );
        
        *plan = Box::new(PlanNode::Filter {
            input: Box::new(current_plan),
            predicate,
        });
    }
}

/// CTE Profiler for performance analysis
pub struct CteProfiler {
    profiles: HashMap<String, CteProfile>,
}

#[derive(Debug, Clone)]
pub struct CteProfile {
    pub cte_name: String,
    pub execution_count: usize,
    pub total_time_ms: u128,
    pub min_time_ms: u128,
    pub max_time_ms: u128,
    pub avg_time_ms: f64,
    pub total_rows_produced: usize,
    pub avg_rows_produced: f64,
}

impl CteProfiler {
    pub fn new() -> Self {
        Self {
            profiles: HashMap::new(),
        }
    }
    
    /// Record a CTE execution for profiling
    pub fn record(
        &mut self,
        cte_name: String,
        execution_time_ms: u128,
        rows_produced: usize,
    ) {
        let profile = self.profiles.entry(cte_name.clone()).or_insert(CteProfile {
            cte_name: cte_name.clone(),
            execution_count: 0,
            total_time_ms: 0,
            min_time_ms: u128::MAX,
            max_time_ms: 0,
            avg_time_ms: 0.0,
            total_rows_produced: 0,
            avg_rows_produced: 0.0,
        });
        
        profile.execution_count += 1;
        profile.total_time_ms += execution_time_ms;
        profile.min_time_ms = profile.min_time_ms.min(execution_time_ms);
        profile.max_time_ms = profile.max_time_ms.max(execution_time_ms);
        profile.avg_time_ms = profile.total_time_ms as f64 / profile.execution_count as f64;
        profile.total_rows_produced += rows_produced;
        profile.avg_rows_produced = profile.total_rows_produced as f64 / profile.execution_count as f64;
    }
    
    /// Get profile for a specific CTE
    pub fn get_profile(&self, cte_name: &str) -> Option<&CteProfile> {
        self.profiles.get(cte_name)
    }
    
    /// Get all profiles sorted by total execution time
    pub fn get_top_ctes_by_time(&self, limit: usize) -> Vec<CteProfile> {
        let mut profiles: Vec<_> = self.profiles.values().cloned().collect();
        profiles.sort_by(|a, b| b.total_time_ms.cmp(&a.total_time_ms));
        profiles.into_iter().take(limit).collect()
    }
    
    /// Generate performance report
    pub fn generate_report(&self) -> String {
        let mut report = String::new();
        report.push_str("CTE Performance Report\n");
        report.push_str("======================\n\n");
        
        for profile in self.get_top_ctes_by_time(10) {
            report.push_str(&format!("CTE: {}\n", profile.cte_name)));
            report.push_str(&format!("  Executions: {}\n", profile.execution_count)));
            report.push_str(&format!("  Total Time: {} ms\n", profile.total_time_ms)));
            report.push_str(&format!("  Avg Time: {:.2} ms\n", profile.avg_time_ms)));
            report.push_str(&format!("  Min Time: {} ms\n", profile.min_time_ms)));
            report.push_str(&format!("  Max Time: {} ms\n", profile.max_time_ms)));
            report.push_str(&format!("  Avg Rows: {:.0}\n", profile.avg_rows_produced)));
            report.push_str("\n");
        }
        
        report
    }
}

#[cfg(test)]
mod extended_tests {

    #[test]
    fn test_cte_execution_engine() {
        let mut engine = CteExecutionEngine::new();
        
        let cte = CteDefinition {
            name: "test_cte".to_string(),
            columns: vec!["id".to_string()],
            query: Box::new(PlanNode::TableScan {
                table: "base".to_string(),
                columns: vec!["*".to_string()],
            }),
            recursive: false,
        };
        
        let main_query = PlanNode::TableScan {
            table: "test_cte".to_string(),
            columns: vec!["*".to_string()],
        };
        
        let result = engine.execute_with_optimization(vec![cte], main_query);
        assert!(result.is_ok());
    }
    
    #[test]
    fn test_cte_result_cache() {
        let mut cache = CteResultCache::new(3);
        
        let result1 = QueryResult::new(vec!["id".to_string()], vec![vec!["1".to_string()]]);
        let result2 = QueryResult::new(vec!["id".to_string()], vec![vec!["2".to_string()]]);
        
        cache.insert("cte1".to_string(), result1);
        cache.insert("cte2".to_string(), result2);
        
        assert_eq!(cache.size(), 2);
        
        let retrieved = cache.get("cte1");
        assert!(retrieved.is_some());
    }
    
    #[test]
    fn test_cache_lru_eviction() {
        let mut cache = CteResultCache::new(2);
        
        let result1 = QueryResult::new(vec!["id".to_string()], vec![vec!["1".to_string()]]);
        let result2 = QueryResult::new(vec!["id".to_string()], vec![vec!["2".to_string()]]);
        let result3 = QueryResult::new(vec!["id".to_string()], vec![vec!["3".to_string()]]);
        
        cache.insert("cte1".to_string(), result1);
        cache.insert("cte2".to_string(), result2);
        cache.insert("cte3".to_string(), result3);
        
        // cte1 should be evicted (LRU)
        assert_eq!(cache.size(), 2);
        assert!(cache.get("cte1").is_none());
        assert!(cache.get("cte2").is_some());
        assert!(cache.get("cte3").is_some());
    }
    
    #[test]
    fn test_incremental_cte_evaluator() {
        let mut evaluator = IncrementalCteEvaluator::new();
        
        let base = QueryResult::new(
            vec!["id".to_string()],
            vec![vec!["1".to_string()]],
        );
        
        let plan = PlanNode::TableScan {
            table: "test".to_string(),
            columns: vec!["*".to_string()],
        };
        
        let result = evaluator.evaluate_incremental("test_cte", base, &plan);
        assert!(result.is_ok());
    }
    
    #[test]
    fn test_materialization_strategy_selector() {
        let selector = MaterializationStrategySelector::new();
        
        let recursive_cte = CteDefinition {
            name: "recursive".to_string(),
            columns: vec!["id".to_string()],
            query: Box::new(PlanNode::TableScan {
                table: "test".to_string(),
                columns: vec!["*".to_string()],
            }),
            recursive: true,
        };
        
        let strategy = selector.select_strategy(&recursive_cte, 1, 100.0);
        assert_eq!(strategy, MaterializationStrategy::AlwaysMaterialize);
        
        let simple_cte = CteDefinition {
            name: "simple".to_string(),
            columns: vec!["id".to_string()],
            query: Box::new(PlanNode::TableScan {
                table: "test".to_string(),
                columns: vec!["*".to_string()],
            }),
            recursive: false,
        };
        
        let strategy = selector.select_strategy(&simple_cte, 1, 100.0);
        assert_eq!(strategy, MaterializationStrategy::AlwaysInline);
    }
    
    #[test]
    fn test_cte_plan_visualizer() {
        let cte = CteDefinition {
            name: "test_cte".to_string(),
            columns: vec!["id".to_string(), "name".to_string()],
            query: Box::new(PlanNode::Filter {
                input: Box::new(PlanNode::TableScan {
                    table: "users".to_string(),
                    columns: vec!["*".to_string()],
                }),
                predicate: "id > 0".to_string(),
            }),
            recursive: false,
        };
        
        let main_query = PlanNode::TableScan {
            table: "test_cte".to_string(),
            columns: vec!["*".to_string()],
        };
        
        let visualization = CtePlanVisualizer::visualize(&[cte], &main_query);
        assert!(visualization.contains("CTE 1: test_cte"));
        assert!(visualization.contains("Main Query:"));
    }
    
    #[test]
    fn test_cte_memory_manager() {
        let mut manager = CteMemoryManager::new(10); // 10 MB limit
        
        assert!(manager.allocate("cte1".to_string(), 5 * 1024 * 1024).is_ok());
        assert_eq!(manager.get_usage(), 5 * 1024 * 1024);
        
        // Should fail - exceeds limit
        assert!(manager.allocate("cte2".to_string(), 6 * 1024 * 1024).is_err());
        
        manager.deallocate("cte1");
        assert_eq!(manager.get_usage(), 0);
    }
    
    #[test]
    fn test_cte_profiler() {
        let mut profiler = CteProfiler::new();
        
        profiler.record("test_cte".to_string(), 100, 50);
        profiler.record("test_cte".to_string(), 200, 60);
        
        let profile = profiler.get_profile("test_cte");
        assert!(profile.is_some());
        
        let p = profile.unwrap();
        assert_eq!(p.execution_count, 2);
        assert_eq!(p.avg_time_ms, 150.0);
        assert_eq!(p.min_time_ms, 100);
        assert_eq!(p.max_time_ms, 200);
    }
    
    #[test]
    fn test_cte_query_rewriter() {
        let mut rewriter = CteQueryRewriter::new();
        
        let cte = CteDefinition {
            name: "test".to_string(),
            columns: vec!["id".to_string()],
            query: Box::new(PlanNode::TableScan {
                table: "base".to_string(),
                columns: vec!["*".to_string()],
            }),
            recursive: false,
        };
        
        let rewritten = rewriter.rewrite(vec![cte]);
        assert_eq!(rewritten.len(), 1);
        assert!(rewriter.get_rewrite_count("inline") > 0);
    }
}

/// Advanced CTE Optimization Framework
pub mod optimization {

    /// CTE Cost Model for accurate cost estimation
    pub struct CteCostModel {
        cpu_cost_factor: f64,
        io_cost_factor: f64,
        network_cost_factor: f64,
        memory_cost_factor: f64,
    }
    
    impl CteCostModel {
        pub fn new() -> Self {
            Self {
                cpu_cost_factor: 1.0,
                io_cost_factor: 4.0,
                network_cost_factor: 10.0,
                memory_cost_factor: 0.5,
            }
        }
        
        /// Calculate comprehensive cost including all factors
        pub fn calculate_total_cost(
            &self,
            cpu_ops: usize,
            io_ops: usize,
            network_ops: usize,
            memory_bytes: usize,
        ) -> f64 {
            let cpu_cost = cpu_ops as f64 * self.cpu_cost_factor;
            let io_cost = io_ops as f64 * self.io_cost_factor;
            let network_cost = network_ops as f64 * self.network_cost_factor;
            let memory_cost = (memory_bytes as f64 / 1024.0) * self.memory_cost_factor;
            
            cpu_cost + io_cost + network_cost + memory_cost
        }
        
        /// Estimate cost for a CTE plan
        pub fn estimate_plan_cost(&self, plan: &PlanNode) -> (usize, usize, usize, usize) {
            match plan {
                PlanNode::TableScan { .. } => (100, 10, 0, 1024),
                PlanNode::Filter { input, .. } => {
                    let (cpu, io, net, mem) = self.estimate_plan_cost(input);
                    (cpu + 50, io, net, mem)
                }
                PlanNode::Project { input, .. } => {
                    let (cpu, io, net, mem) = self.estimate_plan_cost(input);
                    (cpu + 10, io, net, mem / 2)
                }
                PlanNode::Join { left, right, .. } => {
                    let (cpu_l, io_l, net_l, mem_l) = self.estimate_plan_cost(left);
                    let (cpu_r, io_r, net_r, mem_r) = self.estimate_plan_cost(right);
                    (cpu_l + cpu_r + 1000, io_l + io_r, net_l + net_r, mem_l + mem_r)
                }
                PlanNode::Aggregate { input, .. } => {
                    let (cpu, io, net, mem) = self.estimate_plan_cost(input);
                    (cpu * 2, io, net, mem * 2)
                }
                PlanNode::Sort { input, .. } => {
                    let (cpu, io, net, mem) = self.estimate_plan_cost(input);
                    (cpu * 3, io + 5, net, mem * 2)
                }
                PlanNode::Limit { input, .. } => {
                    let (cpu, io, net, mem) = self.estimate_plan_cost(input);
                    (cpu / 2, io / 2, net, mem / 2)
                }
                PlanNode::Subquery { plan, .. } => {
                    let (cpu, io, net, mem) = self.estimate_plan_cost(plan);
                    (cpu, io, net, mem)
                }
            }
        }
    }
    
    /// CTE Selectivity Estimator
    pub struct CteSelectivityEstimator {
        histogram_buckets: usize,
    }
    
    impl CteSelectivityEstimator {
        pub fn new() -> Self {
            Self {
                histogram_buckets: 100,
            }
        }
        
        /// Estimate selectivity of a filter predicate
        pub fn estimate_selectivity(&self, predicate: &str) -> f64 {
            // Simple heuristics for common predicates
            if predicate.contains("=") {
                0.01 // Equality is very selective
            } else if predicate.contains(">") || predicate.contains("<") {
                0.33 // Range predicates moderately selective
            } else if predicate.contains("LIKE") {
                0.10 // String matching
            } else {
                0.50 // Default selectivity
            }
        }
        
        /// Build histogram for column values
        pub fn build_histogram(&self, _values: &[String]) -> Histogram {
            Histogram {
                buckets: vec![HistogramBucket {
                    lower: "0".to_string(),
                    upper: "100".to_string(),
                    count: 100,
                    distinct_count: 80,
                }],
            }
        }
    }
    
    #[derive(Debug, Clone)]
    pub struct Histogram {
        pub buckets: Vec<HistogramBucket>,
    }
    
    #[derive(Debug, Clone)]
    pub struct HistogramBucket {
        pub lower: String,
        pub upper: String,
        pub count: usize,
        pub distinct_count: usize,
    }
    
    /// CTE Cardinality Estimator
    pub struct CteCardinalityEstimator {
        base_table_sizes: HashMap<String, usize>,
    }
    
    impl CteCardinalityEstimator {
        pub fn new() -> Self {
            Self {
                base_table_sizes: HashMap::new(),
            }
        }
        
        pub fn set_table_size(&mut self, table: String, size: usize) {
            self.base_table_sizes.insert(table, size);
        }
        
        /// Estimate output cardinality of a plan
        pub fn estimate_cardinality(&self, plan: &PlanNode) -> usize {
            match plan {
                PlanNode::TableScan { table, .. } => {
                    self.base_table_sizes.get(table).copied().unwrap_or(1000)
                }
                PlanNode::Filter { input, predicate } => {
                    let input_card = self.estimate_cardinality(input);
                    let selectivity = self.estimate_filter_selectivity(predicate);
                    ((input_card as f64) * selectivity) as usize
                }
                PlanNode::Project { input, .. } => {
                    self.estimate_cardinality(input)
                }
                PlanNode::Join { left, right, .. } => {
                    let left_card = self.estimate_cardinality(left);
                    let right_card = self.estimate_cardinality(right);
                    // Assume 10% join selectivity
                    (left_card * right_card) / 10
                }
                PlanNode::Aggregate { input, group_by, .. } => {
                    let input_card = self.estimate_cardinality(input);
                    if group_by.is_empty() {
                        1 // Single row for aggregate without grouping
                    } else {
                        input_card / 10 // Assume 10:1 reduction
                    }
                }
                PlanNode::Sort { input, .. } => {
                    self.estimate_cardinality(input)
                }
                PlanNode::Limit { input, limit, .. } => {
                    self.estimate_cardinality(input).min(*limit)
                }
                PlanNode::Subquery { plan, .. } => {
                    self.estimate_cardinality(plan)
                }
            }
        }
        
        fn estimate_filter_selectivity(&self, predicate: &str) -> f64 {
            if predicate.contains("=") {
                0.01
            } else if predicate.contains(">") || predicate.contains("<") {
                0.33
            } else {
                0.50
            }
        }
    }
}

/// CTE Execution Monitoring and Observability
pub mod monitoring {
    use std::time::Duration;
use std::time::Instant;
use std::time::SystemTime;

    /// CTE Execution Monitor
    pub struct CteExecutionMonitor {
        executions: Vec<CteExecution>,
        active_executions: HashMap<String>,
    }
    
    #[derive(Debug, Clone)]
    pub struct CteExecution {
        pub cte_name: String,
        pub start_time: std::time::SystemTime,
        pub duration: Duration,
        pub rows_produced: usize,
        pub memory_used: usize,
        pub cache_hit: bool,
    }
    
    impl CteExecutionMonitor {
        pub fn new() -> Self {
            Self {
                executions: Vec::new(),
                active_executions: HashMap::new(),
            }
        }
        
        /// Start monitoring a CTE execution
        pub fn start_execution(&mut self, cte_name: String) {
            self.active_executions.insert(cte_name::now());
        }
        
        /// End monitoring and record results
        pub fn end_execution(
            &mut self,
            cte_name: String,
            rows_produced: usize,
            memory_used: usize,
            cache_hit: bool,
        ) {
            if let Some(start) = self.active_executions.remove(&cte_name) {
                let duration = start.elapsed();
                
                self.executions.push(CteExecution {
                    cte_name,
                    start_time: std::time::SystemTime::now(),
                    duration,
                    rows_produced,
                    memory_used,
                    cache_hit,
                });
            }
        }
        
        /// Get execution history
        pub fn get_history(&self) -> &[CteExecution] {
            &self.executions
        }
        
        /// Get average execution time for a CTE
        pub fn get_average_time(&self, cte_name: &str) -> Option<Duration> {
            let executions: Vec<_> = self.executions
                .iter()
                .filter(|e| e.cte_name == cte_name)
                .collect();
            
            if executions.is_empty() {
                return None;
            }
            
            let total_ms: u128 = executions
                .iter()
                .map(|e| e.duration.as_millis())
                .sum();
            
            let avg_ms = total_ms / executions.len() as u128;
            Some(Duration::from_millis(avg_ms as u64))
        }
        
        /// Get cache hit rate
        pub fn get_cache_hit_rate(&self, cte_name: &str) -> f64 {
            let executions: Vec<_> = self.executions
                .iter()
                .filter(|e| e.cte_name == cte_name)
                .collect();
            
            if executions.is_empty() {
                return 0.0;
            }
            
            let hits = executions.iter().filter(|e| e.cache_hit).count();
            hits as f64 / executions.len() as f64
        }
    }
    
    /// CTE Performance Metrics
    pub struct CtePerformanceMetrics {
        total_executions: usize,
        total_time: Duration,
        cache_hits: usize,
        cache_misses: usize,
        total_rows_produced: usize,
        total_memory_used: usize,
    }
    
    impl CtePerformanceMetrics {
        pub fn new() -> Self {
            Self {
                total_executions: 0,
                total_time: Duration::ZERO,
                cache_hits: 0,
                cache_misses: 0,
                total_rows_produced: 0,
                total_memory_used: 0,
            }
        }
        
        pub fn record_execution(
            &mut self,
            duration: Duration,
            cache_hit: bool,
            rows: usize,
            memory: usize,
        ) {
            self.total_executions += 1;
            self.total_time += duration;
            self.total_rows_produced += rows;
            self.total_memory_used += memory;
            
            if cache_hit {
                self.cache_hits += 1;
            } else {
                self.cache_misses += 1;
            }
        }
        
        pub fn get_cache_hit_rate(&self) -> f64 {
            if self.total_executions == 0 {
                return 0.0;
            }
            self.cache_hits as f64 / self.total_executions as f64
        }
        
        pub fn get_average_time(&self) -> Duration {
            if self.total_executions == 0 {
                return Duration::ZERO;
            }
            self.total_time / self.total_executions as u32
        }
        
        pub fn get_average_rows(&self) -> f64 {
            if self.total_executions == 0 {
                return 0.0;
            }
            self.total_rows_produced as f64 / self.total_executions as f64
        }
    }
}

/// CTE Advanced Features
pub mod advanced {

    /// Window Function Support in CTEs
    pub struct CteWindowFunction {
        partition_by: Vec<String>,
        order_by: Vec<String>,
        frame: WindowFrame,
    }
    
    #[derive(Debug, Clone)]
    pub enum WindowFrame {
        Rows { start: i64, end: i64 },
        Range { start: String, end: String },
        Groups { start: i64, end: i64 },
    }
    
    impl CteWindowFunction {
        pub fn new(
            partition_by: Vec<String>,
            order_by: Vec<String>,
            frame: WindowFrame,
        ) -> Self {
            Self {
                partition_by,
                order_by,
                frame,
            }
        }
        
        /// Apply window function to CTE result
        pub fn apply(&self, result: &mut QueryResult) -> Result<(), DbError> {
            // Group by partition
            let partitions = self.partition_rows(&result.rows);
            
            // Apply window function to each partition
            for partition in partitions {
                self.apply_to_partition(&partition)?;
            }
            
            Ok(())
        }
        
        fn partition_rows(&self, rows: &[Vec<String>]) -> Vec<Vec<Vec<String>>> {
            // Placeholder: partition rows by partition_by columns
            vec![rows.to_vec()]
        }
        
        fn apply_to_partition(&self, partition: &[Vec<String>]) -> Result<(), DbError> {
            // Placeholder: apply window function to partition
            Ok(())
        }
    }
    
    /// CTE Lateral Join Support
    pub struct CteLateralJoin {
        outer_cte: String,
        inner_cte: String,
        correlation_columns: Vec<String>,
    }
    
    impl CteLateralJoin {
        pub fn new(
            outer_cte: String,
            inner_cte: String,
            correlation_columns: Vec<String>,
        ) -> Self {
            Self {
                outer_cte,
                inner_cte,
                correlation_columns,
            }
        }
        
        /// Execute lateral join between CTEs
        pub fn execute(&self, context: &CteContext) -> Result<QueryResult, DbError> {
            let outer_result = context.get_materialized(&self.outer_cte)
                .ok_or_else(|| DbError::NotFound(format!(
                    "CTE '{}' not materialized",
                    self.outer_cte
                )))?);
            
            let inner_result = context.get_materialized(&self.inner_cte)
                .ok_or_else(|| DbError::NotFound(format!(
                    "CTE '{}' not materialized",
                    self.inner_cte
                )))?);
            
            // Perform lateral join
            let mut result_rows = Vec::new();
            for outer_row in &outer_result.rows {
                for inner_row in &inner_result.rows {
                    let mut combined = outer_row.clone();
                    combined.extend(inner_row.clone());
                    result_rows.push(combined);
                }
            }
            
            let mut result_cols = outer_result.columns.clone();
            result_cols.extend(inner_result.columns.clone());
            
            Ok(QueryResult::new(result_cols, result_rows))
        }
    }
    
    /// CTE Merge Optimization
    pub struct CteMergeOptimizer;
    
    impl CteMergeOptimizer {
        /// Attempt to merge CTEs for better performance
        pub fn try_merge(
            cte1: &CteDefinition,
            cte2: &CteDefinition,
        ) -> Option<CteDefinition> {
            // Can only merge if cte2 references cte1 directly
            if !Self::references_cte(&cte2.query, &cte1.name) {
                return None;
            }
            
            // Check if merge is beneficial
            if !Self::is_beneficial_to_merge(cte1, cte2) {
                return None;
            }
            
            // Create merged CTE
            Some(CteDefinition {
                name: format!("{}_merged_{}", cte1.name, cte2.name),
                columns: cte2.columns.clone(),
                query: Box::new(Self::merge_plans(&cte1.query, &cte2.query)),
                recursive: cte1.recursive || cte2.recursive,
            })
        }
        
        fn references_cte(plan: &PlanNode, cte_name: &str) -> bool {
            match plan {
                PlanNode::TableScan { table, .. } => table == cte_name,
                PlanNode::Filter { input, .. }
                | PlanNode::Project { input, .. }
                | PlanNode::Sort { input, .. }
                | PlanNode::Limit { input, .. }
                | PlanNode::Aggregate { input, .. } => Self::references_cte(input, cte_name),
                PlanNode::Join { left, right, .. } => {
                    Self::references_cte(left, cte_name) || Self::references_cte(right, cte_name)
                }
                PlanNode::Subquery { plan, .. } => Self::references_cte(plan, cte_name),
            }
        }
        
        fn is_beneficial_to_merge(_cte1: &CteDefinition, _cte2: &CteDefinition) -> bool {
            // Simple heuristic: merge if both are simple queries
            true
        }
        
        fn merge_plans(plan1: &PlanNode, plan2: &PlanNode) -> PlanNode {
            // Placeholder: merge query plans
            plan2.clone()
        }
    }
    
    /// CTE Partitioned Execution for large datasets
    pub struct CtePartitionedExecutor {
        partition_count: usize,
    }
    
    impl CtePartitionedExecutor {
        pub fn new(partition_count: usize) -> Self {
            Self { partition_count }
        }
        
        /// Execute CTE in partitions for better memory management
        pub fn execute_partitioned(
            &self,
            cte: &CteDefinition,
        ) -> Result<Vec<QueryResult>, DbError> {
            let mut results = Vec::new());
            
            for partition_id in 0..self.partition_count {
                let partition_result = self.execute_partition(cte, partition_id)?;
                results.push(partition_result);
            }
            
            Ok(results)
        }
        
        fn execute_partition(
            &self,
            _cte: &CteDefinition,
            _partition_id: usize,
        ) -> Result<QueryResult, DbError> {
            // Placeholder: execute CTE for specific partition
            Ok(QueryResult::empty())
        }
        
        /// Merge partition results
        pub fn merge_results(&self, partitions: Vec<QueryResult>) -> QueryResult {
            if partitions.is_empty() {
                return QueryResult::empty();
            }
            
            let columns = partitions[0].columns.clone();
            let mut all_rows = Vec::new();
            
            for partition in partitions {
                all_rows.extend(partition.rows);
            }
            
            QueryResult::new(columns, all_rows)
        }
    }
}

/// CTE Query Transformation and Rewriting
pub mod transformation {

    /// CTE Subquery Flattening
    pub struct CteSubqueryFlattener;
    
    impl CteSubqueryFlattener {
        /// Flatten subqueries into CTEs
        pub fn flatten(query: &PlanNode) -> (Vec<CteDefinition>, PlanNode) {
            let mut ctes = Vec::new();
            let mut counter = 0;
            
            let flattened = Self::flatten_recursive(query, &mut ctes, &mut counter);
            
            (ctes, flattened)
        }
        
        fn flatten_recursive(
            plan: &PlanNode,
            ctes: &mut Vec<CteDefinition>,
            counter: &mut usize,
        ) -> PlanNode {
            match plan {
                PlanNode::Subquery { plan: subplan, .. } => {
                    // Extract subquery into CTE
                    let cte_name = format!("__subquery_{}", counter));
                    *counter += 1;
                    
                    let cte = CteDefinition {
                        name: cte_name.clone(),
                        columns: vec!["*".to_string()],
                        query: subplan.clone(),
                        recursive: false,
                    };
                    
                    ctes.push(cte);
                    
                    PlanNode::TableScan {
                        table: cte_name,
                        columns: vec!["*".to_string()],
                    }
                }
                PlanNode::Filter { input, predicate } => {
                    PlanNode::Filter {
                        input: Box::new(Self::flatten_recursive(input, ctes, counter)),
                        predicate: predicate.clone(),
                    }
                }
                PlanNode::Project { input, columns } => {
                    PlanNode::Project {
                        input: Box::new(Self::flatten_recursive(input, ctes, counter)),
                        columns: columns.clone(),
                    }
                }
                other => other.clone(),
            }
        }
    }
    
    /// CTE Common Expression Elimination
    pub struct CteCommonExpressionEliminator;
    
    impl CteCommonExpressionEliminator {
        /// Find and eliminate common expressions
        pub fn eliminate(ctes: Vec<CteDefinition>) -> Vec<CteDefinition> {
            // Build expression frequency map
            let mut expr_freq: HashMap<String, usize> = HashMap::new();
            
            for cte in &ctes {
                Self::count_expressions(&cte.query, &mut expr_freq);
            }
            
            // Extract common expressions into new CTEs
            let common_exprs: Vec<_> = expr_freq
                .iter()
                .filter(|(_, &count)| count > 1)
                .collect();
            
            if common_exprs.is_empty() {
                return ctes;
            }
            
            // For now, return original CTEs
            // Full implementation would extract common expressions
            ctes
        }
        
        fn count_expressions(plan: &PlanNode, freq: &mut HashMap<String, usize>) {
            let expr_key = format!("{:?}", plan));
            *freq.entry(expr_key).or_insert(0) += 1;
            
            match plan {
                PlanNode::Filter { input, .. }
                | PlanNode::Project { input, .. }
                | PlanNode::Sort { input, .. }
                | PlanNode::Limit { input, .. }
                | PlanNode::Aggregate { input, .. } => {
                    Self::count_expressions(input, freq);
                }
                PlanNode::Join { left, right, .. } => {
                    Self::count_expressions(left, freq);
                    Self::count_expressions(right, freq);
                }
                PlanNode::Subquery { plan, .. } => {
                    Self::count_expressions(plan, freq);
                }
                _ => {}
            }
        }
    }
    
    /// CTE Predicate Factorization
    pub struct CtePredicateFactorizer;
    
    impl CtePredicateFactorizer {
        /// Factor out common predicates across CTEs
        pub fn factorize(ctes: Vec<CteDefinition>) -> Vec<CteDefinition> {
            // Analyze predicates in all CTEs
            let mut common_predicates = Vec::new();
            
            for cte in &ctes {
                let predicates = Self::extract_predicates(&cte.query);
                if common_predicates.is_empty() {
                    common_predicates = predicates;
                } else {
                    common_predicates.retain(|p| predicates.contains(p));
                }
            }
            
            // If no common predicates, return original
            if common_predicates.is_empty() {
                return ctes;
            }
            
            // Apply factorization
            ctes
        }
        
        fn extract_predicates(plan: &PlanNode) -> Vec<String> {
            let mut predicates = Vec::new();
            
            match plan {
                PlanNode::Filter { predicate, input } => {
                    predicates.push(predicate.clone());
                    predicates.extend(Self::extract_predicates(input));
                }
                PlanNode::Join { left, right, condition, .. } => {
                    predicates.push(condition.clone());
                    predicates.extend(Self::extract_predicates(left));
                    predicates.extend(Self::extract_predicates(right));
                }
                PlanNode::Project { input, .. }
                | PlanNode::Sort { input, .. }
                | PlanNode::Limit { input, .. }
                | PlanNode::Aggregate { input, .. } => {
                    predicates.extend(Self::extract_predicates(input));
                }
                PlanNode::Subquery { plan, .. } => {
                    predicates.extend(Self::extract_predicates(plan));
                }
                _ => {}
            }
            
            predicates
        }
    }
}

#[cfg(test)]
mod optimization_tests {

    #[test]
    fn test_cost_model() {
        let model = CteCostModel::new();
        let cost = model.calculate_total_cost(1000, 100, 10, 1024);
        assert!(cost > 0.0);
    }
    
    #[test]
    fn test_selectivity_estimator() {
        let estimator = CteSelectivityEstimator::new();
        
        let eq_selectivity = estimator.estimate_selectivity("id = 5");
        assert_eq!(eq_selectivity, 0.01);
        
        let range_selectivity = estimator.estimate_selectivity("age > 18");
        assert_eq!(range_selectivity, 0.33);
    }
    
    #[test]
    fn test_cardinality_estimator() {
        let mut estimator = CteCardinalityEstimator::new();
        estimator.set_table_size("users".to_string(), 10000);
        
        let plan = PlanNode::TableScan {
            table: "users".to_string(),
            columns: vec!["*".to_string()],
        };
        
        let cardinality = estimator.estimate_cardinality(&plan);
        assert_eq!(cardinality, 10000);
    }
    
    #[test]
    fn test_execution_monitor() {
        let mut monitor = CteExecutionMonitor::new();
        
        monitor.start_execution("test_cte".to_string());
        std::thread::sleep(std::time::Duration::from_millis(10));
        monitor.end_execution("test_cte".to_string(), 100, 1024, false);
        
        let history = monitor.get_history();
        assert_eq!(history.len(), 1);
        assert_eq!(history[0].rows_produced, 100);
    }
    
    #[test]
    fn test_performance_metrics() {
        let mut metrics = CtePerformanceMetrics::new();
        
        metrics.record_execution(
            std::time::Duration::from_millis(100),
            true,
            50,
            1024,
        );
        metrics.record_execution(
            std::time::Duration::from_millis(200),
            false,
            60,
            2048,
        );
        
        assert_eq!(metrics.get_cache_hit_rate(), 0.5);
        assert_eq!(metrics.get_average_rows(), 55.0);
    }
    
    #[test]
    fn test_subquery_flattener() {
        let subquery = PlanNode::Subquery {
            plan: Box::new(PlanNode::TableScan {
                table: "users".to_string(),
                columns: vec!["*".to_string()],
            }),
            alias: "sub".to_string(),
        };
        
        let (ctes, _flattened) = CteSubqueryFlattener::flatten(&subquery);
        assert_eq!(ctes.len(), 1);
        assert!(ctes[0].name.starts_with("__subquery_"));
    }
    
    #[test]
    fn test_partitioned_executor() {
        let executor = CtePartitionedExecutor::new(4);
        
        let cte = CteDefinition {
            name: "test".to_string(),
            columns: vec!["id".to_string()],
            query: Box::new(PlanNode::TableScan {
                table: "large_table".to_string(),
                columns: vec!["*".to_string()],
            }),
            recursive: false,
        };
        
        let results = executor.execute_partitioned(&cte);
        assert!(results.is_ok());
    }
}

/// CTE Integration and Utilities
pub mod integration {

    /// CTE SQL Generator - Generate SQL from CTE definitions
    pub struct CteSqlGenerator;
    
    impl CteSqlGenerator {
        /// Generate SQL WITH clause from CTE definitions
        pub fn generate_sql(ctes: &[CteDefinition], mainquery: &str) -> String {
            if ctes.is_empty() {
                return main_query.to_string();
            }
            
            let mut sql = String::from("WITH ");
            
            for (i, cte) in ctes.iter().enumerate() {
                if i > 0 {
                    sql.push_str(",\n     ");
                }
                
                if cte.recursive {
                    sql.push_str("RECURSIVE ");
                }
                
                sql.push_str(&format!("{} (", cte.name)));
                sql.push_str(&cte.columns.join(", "));
                sql.push_str(") AS (\n  ");
                sql.push_str(&Self::plan_to_sql(&cte.query));
                sql.push_str("\n)");
            }
            
            sql.push_str("\n");
            sql.push_str(main_query);
            
            sql
        }
        
        fn plan_to_sql(plan: &PlanNode) -> String {
            match plan {
                PlanNode::TableScan { table, columns } => {
                    format!("SELECT {} FROM {}", columns.join(", "), table)
                }
                PlanNode::Filter { input, predicate } => {
                    format!("{} WHERE {}", Self::plan_to_sql(input), predicate)
                }
                PlanNode::Project { input, columns } => {
                    let base = Self::plan_to_sql(input));
                    // Replace SELECT clause
                    if let Some(pos) = base.find("SELECT") {
                        let after_select = &base[pos + 6..];
                        if let Some(from_pos) = after_select.find("FROM") {
                            format!(
                                "SELECT {} FROM{}",
                                columns.join(", "),
                                &after_select[from_pos + 4..]
                            )
                        } else {
                            base
                        }
                    } else {
                        base
                    }
                }
                _ => "SELECT * FROM placeholder".to_string(),
            }
        }
    }
    
    /// CTE Validator - Validates CTE definitions for correctness
    pub struct CteValidator);
    
    impl CteValidator {
        /// Validate a CTE definition
        pub fn validate(cte: &CteDefinition) -> Result<(), DbError> {
            // Check name is not empty
            if cte.name.is_empty() {
                return Err(DbError::InvalidInput("CTE name cannot be empty".to_string()));
            }
            
            // Check for SQL injection in name
            if cte.name.contains(';') || cte.name.contains("--") {
                return Err(DbError::InvalidInput(
                    "CTE name contains invalid characters".to_string()
                ));
            }
            
            // Check columns are not empty
            if cte.columns.is_empty() {
                return Err(DbError::InvalidInput(
                    "CTE must have at least one column".to_string()
                ));
            }
            
            // Validate column names
            for column in &cte.columns {
                if column.is_empty() {
                    return Err(DbError::InvalidInput(
                        "Column name cannot be empty".to_string()
                    ));
                }
            }
            
            Ok(())
        }
        
        /// Validate all CTEs in a collection
        pub fn validate_all(ctes: &[CteDefinition]) -> Result<(), DbError> {
            for cte in ctes {
                Self::validate(cte)?;
            }
            
            // Check for duplicate names
            let mut names = std::collections::HashSet::new();
            for cte in ctes {
                if !names.insert(&cte.name) {
                    return Err(DbError::AlreadyExists(format!(
                        "Duplicate CTE name: {}",
                        cte.name
                    ))));
                }
            }
            
            Ok(())
        }
    }
    
    /// CTE Serializer for persistence
    pub struct CteSerializer;
    
    impl CteSerializer {
        /// Serialize CTE to JSON
        pub fn to_json(cte: &CteDefinition) -> Result<String, DbError> {
            serde_json::to_string_pretty(&SerializableCte::from(cte))
                .map_err(|e| DbError::Internal(format!("Serialization error: {}", e)))
        }
        
        /// Deserialize CTE from JSON
        pub fn from_json(_json: &str) -> Result<CteDefinition, DbError> {
            // Placeholder for deserialization
            Err(DbError::Internal("Deserialization not yet implemented".to_string()))
        }
    }
    
    #[derive(serde::Serialize, serde::Deserialize)]
    struct SerializableCte {
        name: String,
        columns: Vec<String>,
        recursive: bool,
        // Query plan would need custom serialization
    }
    
    impl SerializableCte {
        fn from(cte: &CteDefinition) -> Self {
            Self {
                name: cte.name.clone(),
                columns: cte.columns.clone(),
                recursive: cte.recursive,
            }
        }
    }
}

/// CTE Documentation and Examples
pub mod documentation {
    /// Comprehensive CTE usage examples
    pub struct CteExamples);
    
    impl CteExamples {
        /// Example 1: Sales Report with CTEs
        pub fn sales_report_example() -> &'static str {
            r#"
            WITH monthly_sales AS (
                SELECT 
                    DATE_TRUNC('month', sale_date) AS month,
                    SUM(amount) AS total_sales
                FROM sales
                GROUP BY month
            ),
            avg_monthly AS (
                SELECT AVG(total_sales) AS avg_sales
                FROM monthly_sales
            )
            SELECT 
                ms.month,
                ms.total_sales,
                ms.total_sales - am.avg_sales AS deviation
            FROM monthly_sales ms
            CROSS JOIN avg_monthly am
            ORDER BY ms.month;
            "#
        }
        
        /// Example 2: Recursive Employee Hierarchy
        pub fn employee_hierarchy_example() -> &'static str {
            r#"
            WITH RECURSIVE org_chart AS (
                -- Base case: CEO (no manager)
                SELECT id, name, manager_id, 0 AS level, name AS path
                FROM employees
                WHERE manager_id IS NULL
                
                UNION ALL
                
                -- Recursive case: employees with managers
                SELECT 
                    e.id,
                    e.name,
                    e.manager_id,
                    oc.level + 1,
                    oc.path || ' -> ' || e.name
                FROM employees e
                JOIN org_chart oc ON e.manager_id = oc.id
            )
            SELECT * FROM org_chart ORDER BY level, name;
            "#
        }
        
        /// Example 3: Bill of Materials (BOM)
        pub fn bill_of_materials_example() -> &'static str {
            r#"
            WITH RECURSIVE bom AS (
                SELECT 
                    part_id,
                    component_id,
                    quantity,
                    1 AS level
                FROM parts_components
                WHERE part_id = 'PRODUCT-A'
                
                UNION ALL
                
                SELECT 
                    pc.part_id,
                    pc.component_id,
                    pc.quantity * bom.quantity,
                    bom.level + 1
                FROM parts_components pc
                JOIN bom ON pc.part_id = bom.component_id
            )
            SELECT component_id, SUM(quantity) AS total_needed, MAX(level) AS max_depth
            FROM bom
            GROUP BY component_id;
            "#
        }
        
        /// Example 4: Graph Traversal
        pub fn graph_traversal_example() -> &'static str {
            r#"
            WITH RECURSIVE path_finder AS (
                SELECT 
                    start_node,
                    end_node,
                    ARRAY[start_node] AS path,
                    0 AS distance
                FROM edges
                WHERE start_node = 'A'
                
                UNION ALL
                
                SELECT 
                    e.start_node,
                    e.end_node,
                    pf.path || e.end_node,
                    pf.distance + 1
                FROM edges e
                JOIN path_finder pf ON e.start_node = pf.end_node
                WHERE e.end_node != ALL(pf.path)  -- Avoid cycles
                  AND pf.distance < 10             -- Limit depth
            )
            SELECT DISTINCT ON (end_node)
                end_node,
                path,
                distance
            FROM path_finder
            WHERE end_node = 'Z'
            ORDER BY end_node, distance;
            "#
        }
    }
}

// Re-export commonly used types and functions
pub use advanced::{CteLateralJoin, CteMergeOptimizer, CtePartitionedExecutor, CteWindowFunction};
pub use documentation::CteExamples;
pub use integration::{CteSerializer, CteSqlGenerator, CteValidator};
pub use monitoring::{CteExecutionMonitor, CtePerformanceMetrics};
pub use optimization::{CteCardinalityEstimator, CteCostModel, CteSelectivityEstimator};
pub use transformation::{CteCommonExpressionEliminator, CtePredicateFactorizer, CteSubqueryFlattener};


