// Replication core module
//
// This module provides the core replication functionality organized into:
// - Types and data structures
// - Manager and orchestration
// - WAL (Write-Ahead Log) management
// - Conflict resolution
// - Health monitoring
// - Snapshot management
// - Slot management

pub mod conflicts;
pub mod health;
pub mod manager;
pub mod slots;
pub mod snapshots;
pub mod types;
pub mod wal;

// Re-export core types
pub use manager::ReplicationManager;
pub use types::*;
