// REST API Handlers Module
//
// Organizes handler functions into logical groups

pub mod db;
pub mod admin;
pub mod monitoring;
pub mod pool;
pub mod cluster;
pub mod sql;
pub mod string_functions;

use std::sync::Arc;
use crate::catalog::Catalog;
use crate::transaction::TransactionManager;
use crate::parser::SqlParser;
use parking_lot::RwLock;

lazy_static::lazy_static! {
    pub static ref CATALOG: Arc<RwLock<Catalog>> = Arc::new(RwLock::new(Catalog::new()));
    pub static ref TXN_MANAGER: Arc<TransactionManager> = Arc::new(TransactionManager::new());
    pub static ref SQL_PARSER: SqlParser = SqlParser::new();
}

// Re-export all handler functions for convenience
pub use db::*;
pub use admin::*;
pub use monitoring::*;
pub use pool::*;
pub use cluster::*;
pub use sql::*;
