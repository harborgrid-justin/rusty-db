// Replication slot core types

use crate::replication::types::{LogSequenceNumber, ReplicaId};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, SystemTime};

use super::errors::SlotError;

/// Unique slot identifier
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SlotId(String);

impl SlotId {
    pub fn new(id: impl Into<String>) -> Result<Self, SlotError> {
        let id = id.into();
        if id.trim().is_empty() {
            return Err(SlotError::InvalidSlotName {
                name: id,
                reason: "Slot ID cannot be empty".to_string(),
            });
        }
        Ok(Self(id))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for SlotId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Slot name (human-readable identifier)
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SlotName(String);

impl SlotName {
    pub fn new(name: impl Into<String>) -> Result<Self, SlotError> {
        let name = name.into();
        if name.trim().is_empty() {
            return Err(SlotError::InvalidSlotName {
                name: name.clone(),
                reason: "Slot name cannot be empty".to_string(),
            });
        }
        if !Self::is_valid_name(&name) {
            return Err(SlotError::InvalidSlotName {
                name: name.clone(),
                reason: "Invalid characters in slot name".to_string(),
            });
        }
        Ok(Self(name))
    }

    fn is_valid_name(name: &str) -> bool {
        name.chars().all(|c| c.is_alphanumeric() || c == '_' || c == '-')
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for SlotName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Replication slot information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SlotInfo {
    /// Unique slot identifier
    pub slot_id: SlotId,
    /// Human-readable slot name
    pub slot_name: SlotName,
    /// Associated replica ID
    pub replica_id: ReplicaId,
    /// Slot type
    pub slot_type: SlotType,
    /// Current LSN position
    pub current_lsn: LogSequenceNumber,
    /// Restart LSN (minimum required)
    pub restart_lsn: LogSequenceNumber,
    /// Confirmed flush LSN
    pub confirmed_flush_lsn: Option<LogSequenceNumber>,
    /// Slot status
    pub status: SlotStatus,
    /// Creation timestamp
    pub created_at: SystemTime,
    /// Last active timestamp
    pub last_active: SystemTime,
    /// Whether the slot is active
    pub active: bool,
    /// Active process PID (if any)
    pub active_pid: Option<u32>,
    /// Slot configuration
    pub config: SlotConfig,
    /// Custom metadata
    pub metadata: HashMap<String, String>,
    /// Slot statistics
    pub statistics: SlotStatistics,
}

/// Types of replication slots
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SlotType {
    /// Physical replication slot (WAL-based)
    Physical,
    /// Logical replication slot (logical decoding)
    Logical,
    /// Custom slot type
    Custom(String),
}

/// Slot status
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SlotStatus {
    /// Slot is active and consuming
    Active,
    /// Slot is inactive but retained
    Inactive,
    /// Slot is being created
    Creating,
    /// Slot is being dropped
    Dropping,
    /// Slot encountered an error
    Error,
    /// Slot has been invalidated
    Invalid,
}

/// Slot configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SlotConfig {
    /// Whether to retain WAL for this slot
    pub retain_wal: bool,
    /// Maximum lag before warning
    pub max_lag_bytes: u64,
    /// Maximum age before warning
    pub max_age: Duration,
    /// Whether to automatically advance on inactivity
    pub auto_advance: bool,
    /// Restart on error
    pub restart_on_error: bool,
    /// Snapshot behavior
    pub snapshot_action: SnapshotAction,
    /// Custom options
    pub custom_options: HashMap<String, String>,
}

impl Default for SlotConfig {
    fn default() -> Self {
        Self {
            retain_wal: true,
            max_lag_bytes: 1024 * 1024 * 1024, // 1GB
            max_age: Duration::from_secs(3600), // 1 hour
            auto_advance: false,
            restart_on_error: true,
            snapshot_action: SnapshotAction::Export,
            custom_options: HashMap::new(),
        }
    }
}

/// Snapshot behavior for slots
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SnapshotAction {
    /// Export snapshot
    Export,
    /// Use existing snapshot
    Use,
    /// No snapshot
    NoSnapshot,
}

/// Slot statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SlotStatistics {
    /// Total bytes consumed
    pub bytes_consumed: u64,
    /// Total records consumed
    pub records_consumed: u64,
    /// Consumption rate (bytes/sec)
    pub consumption_rate: f64,
    /// Current lag in bytes
    pub lag_bytes: u64,
    /// Current lag in time
    pub lag_time: Duration,
    /// Number of restarts
    pub restart_count: u64,
    /// Number of errors
    pub error_count: u64,
    /// Last error message
    pub last_error: Option<String>,
    /// Last error timestamp
    pub last_error_time: Option<SystemTime>,
}

impl Default for SlotStatistics {
    fn default() -> Self {
        Self {
            bytes_consumed: 0,
            records_consumed: 0,
            consumption_rate: 0.0,
            lag_bytes: 0,
            lag_time: Duration::ZERO,
            restart_count: 0,
            error_count: 0,
            last_error: None,
            last_error_time: None,
        }
    }
}

/// Slot advance request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SlotAdvance {
    /// Slot to advance
    pub slot_name: SlotName,
    /// Target LSN
    pub target_lsn: LogSequenceNumber,
    /// Whether to wait for advance
    pub wait: bool,
}

/// Slot health status
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SlotHealthStatus {
    Healthy,
    Degraded,
    Unhealthy,
    Critical,
}

/// Slot health check result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SlotHealth {
    pub slot_name: SlotName,
    pub status: SlotHealthStatus,
    pub lag_bytes: u64,
    pub lag_time: Duration,
    pub last_active: SystemTime,
    pub issues: Vec<String>,
    pub recommendations: Vec<String>,
}

/// Slot consumption record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsumptionRecord {
    /// LSN of this record
    pub lsn: LogSequenceNumber,
    /// Transaction ID
    pub xid: Option<u64>,
    /// Timestamp of the record
    pub timestamp: SystemTime,
    /// Record data
    pub data: Vec<u8>,
    /// Record metadata
    pub metadata: HashMap<String, String>,
}

/// Atomic slot metrics for lock-free updates
#[derive(Debug)]
pub struct AtomicSlotMetrics {
    pub bytes_consumed: Arc<AtomicU64>,
    pub records_consumed: Arc<AtomicU64>,
    pub lag_bytes: Arc<AtomicU64>,
    pub error_count: Arc<AtomicU64>,
}

impl Default for AtomicSlotMetrics {
    fn default() -> Self {
        Self {
            bytes_consumed: Arc::new(AtomicU64::new(0)),
            records_consumed: Arc::new(AtomicU64::new(0)),
            lag_bytes: Arc::new(AtomicU64::new(0)),
            error_count: Arc::new(AtomicU64::new(0)),
        }
    }
}

impl AtomicSlotMetrics {
    pub fn record_consumption(&self, bytes: u64) {
        self.bytes_consumed.fetch_add(bytes, Ordering::Relaxed);
        self.records_consumed.fetch_add(1, Ordering::Relaxed);
    }

    pub fn update_lag(&self, lag: u64) {
        self.lag_bytes.store(lag, Ordering::Relaxed);
    }

    pub fn record_error(&self) {
        self.error_count.fetch_add(1, Ordering::Relaxed);
    }

    pub fn get_statistics(&self) -> (u64, u64, u64, u64) {
        (
            self.bytes_consumed.load(Ordering::Relaxed),
            self.records_consumed.load(Ordering::Relaxed),
            self.lag_bytes.load(Ordering::Relaxed),
            self.error_count.load(Ordering::Relaxed),
        )
    }
}
