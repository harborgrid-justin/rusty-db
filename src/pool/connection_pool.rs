//! # Enterprise Connection Pooling Engine
//!
//! This module provides a comprehensive, Oracle-inspired connection pooling system
//! with advanced features including elastic sizing, sophisticated wait queue management,
//! pool partitioning, and extensive monitoring capabilities.
//!
//! ## Key Features
//!
//! - **Elastic Pool Sizing**: Dynamic adjustment between min/max connections
//! - **Connection Lifecycle Management**: Factory pattern, state reset, caching
//! - **Advanced Wait Queue**: Fair/priority queuing, deadlock detection
//! - **Pool Partitioning**: User/application/service-based isolation
//! - **Comprehensive Monitoring**: Real-time metrics and leak detection
//!
//! ## Architecture
//!
//! The connection pool is designed for high concurrency with minimal contention:
//! - Lock-free operations where possible
//! - Fine-grained locking for critical sections
//! - Background maintenance thread for housekeeping
//! - Per-partition statistics for reduced contention
//!
//! ## Module Organization
//!
//! The connection pool has been refactored into smaller, focused modules:
//!
//! - `connection::core` - Pool core engine and configuration
//! - `connection::lifecycle` - Connection lifecycle management
//! - `connection::wait_queue` - Wait queue management
//! - `connection::partitioning` - Pool partitioning
//! - `connection::statistics` - Statistics and monitoring

// Re-export everything from the connection module
pub use crate::pool::connection::*;
