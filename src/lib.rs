// RustyDB - Enterprise-grade Rust Database Management System
// Core library module

pub mod storage;
pub mod catalog;
pub mod execution;
pub mod parser;
pub mod transaction;
pub mod index;
pub mod network;
pub mod error;

pub use error::{Result, DbError};

/// Database configuration
#[derive(Debug, Clone)]
pub struct Config {
    pub data_dir: String,
    pub page_size: usize,
    pub buffer_pool_size: usize,
    pub port: u16,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            data_dir: "./data".to_string(),
            page_size: 4096,
            buffer_pool_size: 1000,
            port: 5432,
        }
    }
}
