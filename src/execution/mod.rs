pub mod executor;
pub mod planner;
pub mod optimizer;

pub use executor::Executor;
pub use planner::Planner;
pub use optimizer::Optimizer;

use serde::{Deserialize, Serialize};

/// Query execution result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryResult {
    pub columns: Vec<String>,
    pub rows: Vec<Vec<String>>,
    pub rows_affected: usize,
}

impl QueryResult {
    pub fn new(columns: Vec<String>, rows: Vec<Vec<String>>) -> Self {
        let rows_affected = rows.len();
        Self {
            columns,
            rows,
            rows_affected,
        }
    }
    
    pub fn empty() -> Self {
        Self {
            columns: Vec::new(),
            rows: Vec::new(),
            rows_affected: 0,
        }
    }
    
    pub fn with_affected(rows_affected: usize) -> Self {
        Self {
            columns: Vec::new(),
            rows: Vec::new(),
            rows_affected,
        }
    }
}
