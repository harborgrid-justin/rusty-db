#![allow(dead_code)]
// Transaction WebSocket Event Types
//
// Real-time event types for transaction layer WebSocket streaming

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use utoipa::ToSchema;

use super::super::types::TransactionId;

// ============================================================================
// Transaction Event Types
// ============================================================================

/// Transaction lifecycle event
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct TransactionEvent {
    pub event_type: TransactionEventType,
    pub transaction_id: TransactionId,
    pub timestamp: i64,
    pub isolation_level: Option<String>,
    pub metadata: HashMap<String, serde_json::Value>,
}

/// Type of transaction event
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum TransactionEventType {
    Begin,
    Commit,
    Rollback,
    Savepoint,
    Timeout,
}

/// Lock event for real-time lock monitoring
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct LockEvent {
    pub event_type: LockEventType,
    pub transaction_id: TransactionId,
    pub resource_id: String,
    pub lock_mode: String,
    pub timestamp: i64,
    pub wait_time_ms: Option<u64>,
}

/// Type of lock event
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum LockEventType {
    Acquired,
    Released,
    WaitStart,
    WaitEnd,
    Upgraded,
    Escalated,
}

/// Deadlock detection event
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct DeadlockEvent {
    pub deadlock_id: String,
    pub detected_at: i64,
    pub cycle: Vec<TransactionId>,
    pub victim: TransactionId,
    pub resolution: DeadlockResolution,
}

/// Deadlock resolution strategy
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum DeadlockResolution {
    AbortYoungest,
    AbortOldest,
    AbortLeastWork,
    Manual,
}

/// MVCC version visibility event
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct MvccEvent {
    pub event_type: MvccEventType,
    pub transaction_id: TransactionId,
    pub table: String,
    pub key: String,
    pub version_count: usize,
    pub timestamp: i64,
}

/// Type of MVCC event
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum MvccEventType {
    VersionCreated,
    VersionDeleted,
    GarbageCollected,
    SnapshotTaken,
}

/// Write-Ahead Log event
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct WalEvent {
    pub event_type: WalEventType,
    pub lsn: String,
    pub transaction_id: Option<TransactionId>,
    pub size_bytes: u64,
    pub timestamp: i64,
}

/// Type of WAL event
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum WalEventType {
    Write,
    Flush,
    Checkpoint,
    Truncate,
}

/// Two-Phase Commit event
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct TwoPhaseCommitEvent {
    pub event_type: TpcEventType,
    pub transaction_id: TransactionId,
    pub coordinator_id: String,
    pub participant_id: String,
    pub state: String,
    pub timestamp: i64,
}

/// Type of 2PC event
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum TpcEventType {
    Prepare,
    PrepareOk,
    PrepareAbort,
    Commit,
    Abort,
    Timeout,
}

/// Snapshot isolation event
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct SnapshotEvent {
    pub snapshot_id: u64,
    pub transaction_id: TransactionId,
    pub active_txn_count: usize,
    pub min_txn_id: TransactionId,
    pub max_txn_id: TransactionId,
    pub timestamp: i64,
}

/// Transaction statistics update event
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct TransactionStatsEvent {
    pub total_commits: u64,
    pub total_aborts: u64,
    pub total_deadlocks: u64,
    pub active_transactions: u64,
    pub avg_commit_latency_ms: u64,
    pub p99_latency_ms: u64,
    pub abort_rate: f64,
    pub timestamp: i64,
}

// ============================================================================
// WebSocket Message Wrapper
// ============================================================================

/// WebSocket message wrapper for transaction events
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct TransactionWsMessage {
    pub channel: TransactionChannel,
    pub data: serde_json::Value,
    pub timestamp: i64,
}

/// Transaction event channels
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum TransactionChannel {
    /// Transaction lifecycle events (begin, commit, rollback)
    Lifecycle,
    /// Lock acquisition and release events
    Locks,
    /// Deadlock detection alerts
    Deadlocks,
    /// MVCC version visibility changes
    Mvcc,
    /// Write-ahead log events
    Wal,
    /// Two-phase commit protocol events
    TwoPhaseCommit,
    /// Snapshot isolation events
    Snapshots,
    /// Transaction statistics updates
    Statistics,
}

// ============================================================================
// Subscription Configuration
// ============================================================================

/// Configuration for transaction event subscription
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct TransactionSubscriptionConfig {
    /// Channels to subscribe to
    pub channels: Vec<TransactionChannel>,
    /// Filter by specific transaction IDs (empty = all)
    pub transaction_ids: Option<Vec<TransactionId>>,
    /// Filter by isolation level
    pub isolation_level: Option<String>,
    /// Minimum event priority (0-10, higher = more important)
    pub min_priority: Option<u8>,
    /// Update interval in milliseconds for statistics
    pub stats_interval_ms: Option<u64>,
}

impl Default for TransactionSubscriptionConfig {
    fn default() -> Self {
        Self {
            channels: vec![TransactionChannel::Lifecycle],
            transaction_ids: None,
            isolation_level: None,
            min_priority: None,
            stats_interval_ms: Some(1000),
        }
    }
}
