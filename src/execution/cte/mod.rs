//! # Common Table Expressions (CTE) Module
//! 
//! This module provides comprehensive support for Common Table Expressions including:
//! - Non-recursive CTEs (WITH clause)
//! - Recursive CTEs (WITH RECURSIVE)  
//! - Multiple CTEs in a single query
//! - CTE materialization and optimization
//! - Advanced query transformation and rewriting
//! - Performance monitoring and profiling
//! 
//! ## Architecture
//! 
//! The CTE module is organized into several specialized submodules:
//! - `types`: Core types and data structures
//! - `context`: CTE execution context and materialization
//! - `evaluator`: CTE evaluation engines (recursive and non-recursive)
//! - `optimizer`: Query optimization and cost estimation
//! - `cache`: Result caching and memory management
//! - `monitor`: Performance monitoring and statistics
//! - `transform`: Query transformation and rewriting
//! - `integration`: SQL generation, validation, and serialization
//! 
//! ## Examples
//! 
//! ### Simple CTE
//! 
//! ```sql
//! WITH regional_sales AS (
//!     SELECT region, SUM(amount) AS total_sales
//!     FROM orders
//!     GROUP BY region
//! )
//! SELECT region, total_sales
//! FROM regional_sales
//! WHERE total_sales > 1000000;
//! ```
//! 
//! ### Recursive CTE
//! 
//! ```sql
//! WITH RECURSIVE employee_hierarchy AS (
//!     SELECT id, name, manager_id, 1 AS level
//!     FROM employees
//!     WHERE manager_id IS NULL
//!     
//!     UNION ALL
//!     
//!     SELECT e.id, e.name, e.manager_id, eh.level + 1
//!     FROM employees e
//!     JOIN employee_hierarchy eh ON e.manager_id = eh.id
//! )
//! SELECT * FROM employee_hierarchy;
//! ```

pub mod types;
pub mod context;
pub mod evaluator;
pub mod optimizer; 
pub mod cache;
pub mod monitor;
pub mod transform;
pub mod integration;

// Re-export commonly used types and traits
pub use types::*;
pub use context::CteContext;
pub use evaluator::{CteEvaluator, RecursiveCteEvaluator};
pub use optimizer::{CteOptimizer, CteExecutionEngine};
pub use cache::{CteResultCache, CteMemoryManager};
pub use monitor::{CteProfiler, CteStatistics};
pub use transform::{CteQueryRewriter, CteSubqueryFlattener};
pub use integration::{CteSqlGenerator, CteValidator};

use crate::error::DbError;
use crate::execution::{PlanNode, QueryResult};

/// CTE module result type for consistent error handling
pub type CteResult<T> = std::result::Result<T, DbError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_module_integration() {
        // Test that all modules are properly integrated
        let context = CteContext::new();
        let optimizer = CteOptimizer::new();
        let cache = CteResultCache::new(100);
        
        assert!(context.is_empty());
        assert_eq!(cache.size(), 0);
        assert!(optimizer.get_statistics().is_empty());
    }
}