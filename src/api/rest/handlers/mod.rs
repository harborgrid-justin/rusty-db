// REST API Handlers Module
//
// Organizes handler functions into logical groups

pub mod db;
pub mod admin;
pub mod monitoring;
pub mod pool;
pub mod cluster;

// Re-export all handler functions for convenience
pub use db::*;
pub use admin::*;
pub use monitoring::*;
pub use pool::*;
pub use cluster::*;
