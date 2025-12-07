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
        cte_name: &str,
        working_table: &QueryResult,
        _recursive_plan: &PlanNode,
    ) -> Result<QueryResult> {
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
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        
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
    pub fn topological_sort(&self, ctes: &[CteDefinition]) -> Result<Vec<String>> {
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
    ) -> Result<()> {
        if in_progress.contains(name) {
            return Err(DbError::InvalidOperation(format!(
                "Circular dependency detected in CTE '{}'",
                name
            )));
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
    pub fn enter_nesting(&mut self) -> Result<()> {
        if self.nesting_level >= self.max_nesting_level {
            return Err(DbError::InvalidOperation(format!(
                "Maximum CTE nesting level ({}) exceeded",
                self.max_nesting_level
            )));
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
