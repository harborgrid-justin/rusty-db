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

pub mod types;
pub mod manager;
pub mod wal;
pub mod conflicts;
pub mod health;
pub mod snapshots;
pub mod slots;

// Re-export core types
pub use types::*;
pub use manager::ReplicationManager;
