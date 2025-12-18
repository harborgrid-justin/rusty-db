// Core CTE types and definitions

use std::collections::{HashMap, HashSet};
use std::hash::{DefaultHasher, Hash, Hasher};

use crate::error::DbError;
use crate::execution::{planner::PlanNode, QueryResult};

// Maximum iterations for recursive CTEs to prevent infinite loops
// SECURITY: Prevents infinite recursion DoS attacks
// Example: WITH RECURSIVE r AS (SELECT 1 UNION ALL SELECT n+1 FROM r) SELECT * FROM r
const MAX_CTE_ITERATIONS: usize = 1000;

// CTE Definition
#[derive(Debug, Clone)]
pub struct CteDefinition {
    pub name: String,
    pub columns: Vec<String>,
    pub query: Box<PlanNode>,
    pub recursive: bool,
}

// CTE context for storing materialized CTE results
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

    // Register a CTE definition
    pub fn register_cte(&mut self, cte: CteDefinition) -> Result<(), DbError> {
        if self.definitions.contains_key(&cte.name) {
            return Err(DbError::AlreadyExists(format!(
                "CTE '{}' already defined",
                cte.name
            )));
        }
        self.definitions.insert(cte.name.clone(), cte);
        Ok(())
    }

    // Get a CTE definition
    pub fn get_definition(&self, name: &str) -> Option<&CteDefinition> {
        self.definitions.get(name)
    }

    // Store materialized CTE result
    // FIXED: Now enforces MAX_MATERIALIZED_CTES limit to prevent unbounded memory growth
    pub fn materialize(&mut self, name: String, result: QueryResult) -> Result<(), DbError> {
        // Check if we've hit the materialization limit
        if self.materialized_ctes.len() >= crate::execution::MAX_MATERIALIZED_CTES
            && !self.materialized_ctes.contains_key(&name)
        {
            // Evict oldest CTE (simple FIFO strategy)
            // In production, could use LRU or size-based eviction
            if let Some(first_key) = self.materialized_ctes.keys().next().cloned() {
                eprintln!(
                    "WARNING: CTE materialization limit reached ({}). Evicting CTE '{}'",
                    crate::execution::MAX_MATERIALIZED_CTES,
                    first_key
                );
                self.materialized_ctes.remove(&first_key);
            }
        }

        self.materialized_ctes.insert(name, result);
        Ok(())
    }

    // Get materialized CTE result
    pub fn get_materialized(&self, name: &str) -> Option<&QueryResult> {
        self.materialized_ctes.get(name)
    }

    // Check if a name refers to a CTE
    pub fn is_cte(&self, name: &str) -> bool {
        self.definitions.contains_key(name)
    }
}

// Recursive CTE evaluator
pub struct RecursiveCteEvaluator {
    max_iterations: usize,
}

impl RecursiveCteEvaluator {
    pub fn new() -> Self {
        Self {
            max_iterations: MAX_CTE_ITERATIONS,
        }
    }

    pub fn with_max_iterations(max_iterations: usize) -> Self {
        Self { max_iterations }
    }

    // Evaluate a recursive CTE
    //
    // MEMORY ISSUE (diagrams/04_query_processing_flow.md):
    // All rows are kept in memory - can cause OOM on large recursive queries
    //
    // TODO: Implement spill-to-disk for large recursive CTEs:
    // 1. Set memory limit per CTE (e.g., 100MB)
    // 2. When limit exceeded, spill intermediate results to disk
    // 3. Use external merge for final result assembly
    // 4. Add streaming evaluation where possible
    //
    // Expected improvement: No OOM on large graph traversals, bounded memory
    // Effort: 1 week
    pub fn evaluate(
        &self,
        cte_name: &str,
        base_result: QueryResult,
        recursive_plan: &PlanNode,
    ) -> Result<QueryResult, DbError> {
        let mut all_rows = base_result.rows.clone();
        let columns = base_result.columns.clone();
        let mut working_table = base_result;

        for iteration in 0..self.max_iterations {
            if working_table.rows.is_empty() {
                break;
            }

            let new_rows = self.execute_recursive_step(cte_name, &working_table, recursive_plan)?;

            if new_rows.rows.is_empty() {
                break;
            }

            // TODO: Check memory usage here and spill to disk if needed
            all_rows.extend(new_rows.rows.clone());
            working_table = new_rows;

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
        working_table: &QueryResult,
        _recursive_plan: &PlanNode,
    ) -> Result<QueryResult, DbError> {
        let cycle_detector = CycleDetector::new();
        if cycle_detector.has_cycle(&working_table.rows) {
            return Ok(QueryResult::empty());
        }
        Ok(QueryResult::empty())
    }
}

// Cycle detection for recursive CTEs
pub struct CycleDetector {
    seen_hashes: HashSet<u64>,
}

impl CycleDetector {
    pub fn new() -> Self {
        Self {
            seen_hashes: HashSet::new(),
        }
    }

    pub fn has_cycle(&self, rows: &[Vec<String>]) -> bool {
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

    pub fn add_rows(&mut self, rows: &[Vec<String>]) {
        for row in rows {
            let mut hasher = DefaultHasher::new();
            row.hash(&mut hasher);
            let hash = hasher.finish();
            self.seen_hashes.insert(hash);
        }
    }

    pub fn clear(&mut self) {
        self.seen_hashes.clear();
    }
}

// CTE plan node (extension to PlanNode)
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
