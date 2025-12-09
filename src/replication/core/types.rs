// Replication types and data structures
use serde::{Serialize, Deserialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ReplicationMode {
    Synchronous,
    Asynchronous,
    SemiSync,
    MultiMaster,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ReplicaStatus {
    Active,
    Lagging,
    Disconnected,
    Syncing,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReplicaNode {
    pub id: String,
    pub address: String,
    pub status: ReplicaStatus,
    pub lag_bytes: u64,
    pub last_sync: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReplicationLogEntry {
    pub sequence_number: u64,
    pub operation: ReplicationOperation,
    pub timestamp: i64,
    pub data: Vec<u8>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ReplicationOperation {
    Insert,
    Update,
    Delete,
    CreateTable,
    DropTable,
    AlterTable,
    BeginTransaction,
    CommitTransaction,
    RollbackTransaction,
    CreateIndex,
    DropIndex,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ReplicationTopology {
    SingleMaster,
    MultiMaster,
    Cascading,
    ChainReplication,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ConflictResolutionStrategy {
    LastWriteWins,
    FirstWriteWins,
    Primary,
    Custom,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReplicationConflict {
    pub conflict_id: u64,
    pub sequence_number: u64,
    pub table_name: String,
    pub primary_key: String,
    pub local_version: Vec<u8>,
    pub remote_version: Vec<u8>,
    pub local_timestamp: i64,
    pub remote_timestamp: i64,
    pub resolved: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WALEntry {
    pub lsn: u64,
    pub transaction_id: Option<u64>,
    pub operation: ReplicationOperation,
    pub table_name: String,
    pub data: Vec<u8>,
    pub timestamp: i64,
    pub checksum: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReplicationSnapshot {
    pub snapshot_id: String,
    pub lsn: u64,
    pub timestamp: i64,
    pub tables: Vec<String>,
    pub data_files: Vec<PathBuf>,
    pub size_bytes: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReplicationHealth {
    pub replica_id: String,
    pub is_healthy: bool,
    pub last_heartbeat: i64,
    pub replication_delay_ms: u64,
    pub pending_transactions: usize,
    pub error_count: u32,
    pub last_error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReplicationStats {
    pub total_replicas: usize,
    pub healthy_replicas: usize,
    pub lagging_replicas: usize,
    pub average_lag_ms: u64,
    pub total_conflicts: usize,
    pub unresolved_conflicts: usize,
    pub wal_size: usize,
    pub latest_lsn: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ReplicationEvent {
    ReplicaAdded { replica_id: String, address: String },
    ReplicaRemoved { replica_id: String },
    ReplicaStatusChanged { replica_id: String, status: ReplicaStatus },
    ConflictDetected { conflict_id: u64, table: String },
    ConflictResolved { conflict_id: u64 },
    SnapshotCreated { snapshot_id: String },
    ReplicationLagWarning { replica_id: String, lag_bytes: u64 },
    FailoverInitiated { old_primary: String, new_primary: String },
    SyncCompleted { replica_id: String },
}

pub trait ReplicationEventListener: Send + Sync {
    fn on_event(&self, event: ReplicationEvent);
}
