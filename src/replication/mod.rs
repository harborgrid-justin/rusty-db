#![allow(hidden_glob_reexports)]

// # Replication System
//
// This module provides a comprehensive enterprise-grade replication system
// for the RustyDB database. It supports both synchronous and asynchronous
// replication with strong consistency guarantees, automatic failover,
// conflict resolution, health monitoring, snapshot management, and replication slots.
//
// ## Key Features
//
// - **Multi-Master Replication**: Support for multiple write nodes with automatic conflict resolution
// - **Automatic Failover**: Fast detection and recovery from node failures with minimal downtime
// - **Advanced Conflict Resolution**: Multiple strategies including Last Writer Wins, CRDT, and custom resolvers
// - **WAL-Based Replication**: Write-ahead logging for consistent and reliable data replication
// - **Real-time Health Monitoring**: Comprehensive health monitoring, alerting, and performance analytics
// - **Snapshot Management**: Incremental and full backups with compression and encryption support
// - **Replication Slots**: Logical and physical slot management with WAL retention policies
// - **Performance Optimization**: Advanced buffer management, connection pooling, and throughput optimization
// - **Enterprise Security**: End-to-end encryption, authentication, and authorization controls
// - **Operational Excellence**: Comprehensive metrics, logging, tracing, and diagnostic capabilities
//
// ## Module Organization
//
// The replication system has been refactored into smaller, focused modules:
//
// - `core::types` - Fundamental types and data structures
// - `core::manager` - Central replication manager
// - `core::wal` - Write-Ahead Log management
// - `core::conflicts` - Conflict detection and resolution
// - `core::health` - Health monitoring
// - `core::snapshots` - Snapshot management
// - `core::slots` - Replication slot management

// Re-export everything from the core module
#[allow(hidden_glob_reexports)]
pub use core::*;

// Re-export types from the old types module (temporary until full refactoring)
mod types;
pub use types::ReplicaId;

mod core;
mod monitor;
mod legacy_types;
mod wal;
mod snapshots;
mod slots;
mod manager;
mod conflicts;
