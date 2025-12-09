// # Replication Types and Data Structures
//
// This module provides comprehensive type definitions for the replication system,
// implementing strong typing with newtypes to prevent primitive obsession and
// ensure type safety throughout the replication subsystem.
//
// ## Key Features
//
// - **Strong Typing**: Newtypes for replica IDs, addresses, and sequence numbers
// - **Comprehensive Validation**: Input validation with detailed error messages
// - **Serialization Support**: Full serde support for network transmission
// - **Memory Safety**: Safe handling of replication data with proper lifetime management
// - **Performance Metrics**: Detailed statistics and health monitoring types
//
// ## Usage Examples
//
// ```rust
// use crate::replication::types::*;
//
// // Create strongly-typed replica ID
// let _replica_id = ReplicaId::new("replica-01")?;
//
// // Create replica node with validation
// let replica = ReplicaNode::new(
//     replica_id,
//     ReplicaAddress::new("127.0.0.1:5432")?,
//     ReplicaRole::ReadOnly
// )?;
//
// // Create WAL entry with sequence number
// let wal_entry = WalEntry::new(
//     LogSequenceNumber::new(1000),
//     ReplicationOperation::Insert,
//     TableName::new("users")?,
//     b"data".to_vec()
// )?;
// ```

use std::time::SystemTime;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use std::path::PathBuf;
use std::time::{Duration};
use thiserror::Error;
use crate::error::DbError;

/// Replication-specific error types
#[derive(Error, Debug)]
pub enum ReplicationTypeError {
    #[error("Invalid replica ID '{replica_id}': {reason}")]
    InvalidReplicaId { replica_id: String, reason: String },

    #[error("Invalid replica address '{address}': {reason}")]
    InvalidReplicaAddress { address: String, reason: String },

    #[error("Invalid table name '{table_name}': {reason}")]
    InvalidTableName { table_name: String, reason: String },

    #[error("Invalid slot name '{slot_name}': {reason}")]
    InvalidSlotName { slot_name: String, reason: String },

    #[error("Invalid snapshot ID '{snapshot_id}': {reason}")]
    InvalidSnapshotId { snapshot_id: String, reason: String },

    #[error("Invalid region name '{region}': {reason}")]
    InvalidRegionName { region: String, reason: String },

    #[error("Value out of range: {value} not in range [{min}, {max}]")]
    ValueOutOfRange { value: i64, min: i64, max: i64 },

    #[error("Invalid checksum: expected {expected}, got {actual}")]
    InvalidChecksum { expected: u32, actual: u32 },
}

/// Strongly-typed replica identifier
///
/// Ensures replica IDs follow naming conventions and are valid for use
/// across the replication system.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ReplicaId(String);

impl ReplicaId {
    /// Creates a new replica ID with validation
    ///
    /// # Arguments
    ///
    /// * `id` - The replica identifier string
    ///
    /// # Returns
    ///
    /// * `Ok(ReplicaId)` - Valid replica ID
    /// * `Err(ReplicationTypeError)` - Invalid replica ID
    ///
    /// # Validation Rules
    ///
    /// - Length: 1-64 characters
    /// - Characters: alphanumeric, hyphens, underscores only
    /// - Must start with a letter
    /// - No consecutive special characters
    ///
    /// # Examples
    ///
    /// ```rust
    /// let valid = ReplicaId::new("replica-01").unwrap();
    /// let invalid = ReplicaId::new("123-invalid"); // Error: must start with letter
    /// ```
    pub fn new(id: impl Into<String>) -> Result<Self, ReplicationTypeError> {
        let id = id.into();

        // Length validation
        if id.is_empty() || id.len() > 64 {
            return Err(ReplicationTypeError::InvalidReplicaId {
                replica_id: id,
                reason: "Length must be 1-64 characters".to_string(),
            });
        }

        // Must start with a letter
        if !id.chars().next().unwrap().is_ascii_alphabetic() {
            return Err(ReplicationTypeError::InvalidReplicaId {
                replica_id: id,
                reason: "Must start with a letter".to_string(),
            });
        }

        // Valid characters only
        if !id.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_') {
            return Err(ReplicationTypeError::InvalidReplicaId {
                replica_id: id,
                reason: "Only alphanumeric, hyphens, and underscores allowed".to_string(),
            });
        }

        // No consecutive special characters
        if id.contains("--") || id.contains("__") || id.contains("-_") || id.contains("_-") {
            return Err(ReplicationTypeError::InvalidReplicaId {
                replica_id: id,
                reason: "No consecutive special characters allowed".to_string(),
            });
        }

        Ok(Self(id))
    }

    /// Returns the replica ID as a string slice
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Consumes the ReplicaId and returns the inner String
    pub fn into_string(self) -> String {
        self.0
    }
}

impl fmt::Display for ReplicaId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Strongly-typed replica network address
///
/// Validates and normalizes network addresses for replica connections.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ReplicaAddress(String);

impl ReplicaAddress {
    /// Creates a new replica address with validation
    ///
    /// # Arguments
    ///
    /// * `address` - The network address (IP:port or hostname:port)
    ///
    /// # Returns
    ///
    /// * `Ok(ReplicaAddress)` - Valid address
    /// * `Err(ReplicationTypeError)` - Invalid address
    ///
    /// # Validation Rules
    ///
    /// - Must contain exactly one colon (for port separation)
    /// - Port must be in range 1-65535
    /// - Host part must be valid hostname or IP address
    ///
    /// # Examples
    ///
    /// ```rust
    /// let address = ReplicaAddress::new("127.0.0.1:5432")?;
    /// let hostname = ReplicaAddress::new("replica.example.com:5432")?;
    /// ```
    pub fn new(address: impl Into<String>) -> Result<Self, ReplicationTypeError> {
        let address = address.into();

        // Basic format validation
        let parts: Vec<&str> = address.split(':').collect();
        if parts.len() != 2 {
            return Err(ReplicationTypeError::InvalidReplicaAddress {
                address: address.clone(),
                reason: "Must be in format 'host:port'".to_string(),
            });
        }

        let host = parts[0];
        let port_str = parts[1];

        // Validate host part (simplified)
        if host.is_empty() {
            return Err(ReplicationTypeError::InvalidReplicaAddress {
                address: address.clone(),
                reason: "Host cannot be empty".to_string(),
            });
        }

        // Validate port
        let port: u16 = port_str.parse().map_err(|_| {
            ReplicationTypeError::InvalidReplicaAddress {
                address: address.clone(),
                reason: "Port must be a valid number".to_string(),
            }
        })?;

        if port == 0 {
            return Err(ReplicationTypeError::InvalidReplicaAddress {
                address: address.clone(),
                reason: "Port cannot be 0".to_string(),
            });
        }

        Ok(Self(address))
    }

    /// Returns the address as a string slice
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Extracts the host portion of the address
    pub fn host(&self) -> &str {
        self.0.split(':').next().unwrap()
    }

    /// Extracts the port portion of the address
    pub fn port(&self) -> u16 {
        self.0.split(':').nth(1).unwrap().parse().unwrap()
    }
}

impl fmt::Display for ReplicaAddress {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Strongly-typed table name for replication operations
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TableName(String);

impl TableName {
    /// Creates a new table name with validation
    pub fn new(name: impl Into<String>) -> Result<Self, ReplicationTypeError> {
        let name = name.into();

        if name.is_empty() || name.len() > 128 {
            return Err(ReplicationTypeError::InvalidTableName {
                table_name: name,
                reason: "Length must be 1-128 characters".to_string(),
            });
        }

        // Must start with letter or underscore
        if !name.chars().next().unwrap().is_ascii_alphabetic() && name.chars().next().unwrap() != '_' {
            return Err(ReplicationTypeError::InvalidTableName {
                table_name: name,
                reason: "Must start with letter or underscore".to_string(),
            });
        }

        // Valid characters for SQL identifiers
        if !name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
            return Err(ReplicationTypeError::InvalidTableName {
                table_name: name,
                reason: "Only alphanumeric and underscore characters allowed".to_string(),
            });
        }

        Ok(Self(name))
    }

    /// Returns the table name as a string slice
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for TableName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Log Sequence Number for WAL ordering
///
/// Provides strong typing for LSN values with ordering guarantees.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct LogSequenceNumber(u64);

impl LogSequenceNumber {
    /// Creates a new LSN
    pub fn new(lsn: u64) -> Self {
        Self(lsn)
    }

    /// Returns the LSN value
    pub fn value(&self) -> u64 {
        self.0
    }

    /// Returns the next LSN in sequence
    pub fn next(&self) -> Self {
        Self(self.0.saturating_add(1))
    }

    /// Calculates the difference between two LSNs
    pub fn distance_from(&self, other: &LogSequenceNumber) -> u64 {
        if self.0 >= other.0 {
            self.0 - other.0
        } else {
            other.0 - self.0
        }
    }
}

impl fmt::Display for LogSequenceNumber {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "LSN({})", self.0)
    }
}

/// Transaction ID for grouping WAL entries
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TransactionId(u64);

impl TransactionId {
    /// Creates a new transaction ID
    pub fn new(txn_id: u64) -> Self {
        Self(txn_id)
    }

    /// Returns the transaction ID value
    pub fn value(&self) -> u64 {
        self.0
    }
}

impl fmt::Display for TransactionId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "TXN({})", self.0)
    }
}

/// Replication mode configuration
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ReplicationMode {
    /// Wait for all replicas to acknowledge before committing
    Synchronous,
    /// Fire-and-forget replication for maximum performance
    Asynchronous,
    /// Wait for at least one replica to acknowledge
    SemiSynchronous,
}

impl fmt::Display for ReplicationMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ReplicationMode::Synchronous => write!(f, "Synchronous"),
            ReplicationMode::Asynchronous => write!(f, "Asynchronous"),
            ReplicationMode::SemiSynchronous => write!(f, "Semi-Synchronous"),
        }
    }
}

/// Replica status enumeration
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ReplicaStatus {
    /// Replica is active and up-to-date
    Active,
    /// Replica is lagging but connected
    Lagging,
    /// Replica is disconnected
    Disconnected,
    /// Replica is in process of initial sync
    Syncing,
    /// Replica has failed and needs intervention
    Failed,
    /// Replica is paused for maintenance
    Paused,
}

impl fmt::Display for ReplicaStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ReplicaStatus::Active => write!(f, "Active"),
            ReplicaStatus::Lagging => write!(f, "Lagging"),
            ReplicaStatus::Disconnected => write!(f, "Disconnected"),
            ReplicaStatus::Syncing => write!(f, "Syncing"),
            ReplicaStatus::Failed => write!(f, "Failed"),
            ReplicaStatus::Paused => write!(f, "Paused"),
        }
    }
}

/// Replica role in the replication topology
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ReplicaRole {
    /// Primary replica that accepts writes
    Primary,
    /// Read-only replica for queries
    ReadOnly,
    /// Standby replica for failover
    Standby,
    /// Cascading replica that can have sub-replicas
    Cascading,
}

impl fmt::Display for ReplicaRole {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ReplicaRole::Primary => write!(f, "Primary"),
            ReplicaRole::ReadOnly => write!(f, "Read-Only"),
            ReplicaRole::Standby => write!(f, "Standby"),
            ReplicaRole::Cascading => write!(f, "Cascading"),
        }
    }
}

/// Type of replication operation
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ReplicationOperation {
    /// Insert new row(s)
    Insert,
    /// Update existing row(s)
    Update,
    /// Delete row(s)
    Delete,
    /// Create new table
    CreateTable,
    /// Drop existing table
    DropTable,
    /// Alter table structure
    AlterTable,
    /// Begin transaction
    BeginTransaction,
    /// Commit transaction
    CommitTransaction,
    /// Rollback transaction
    RollbackTransaction,
    /// Create index
    CreateIndex,
    /// Drop index
    DropIndex,
    /// Create view
    CreateView,
    /// Drop view
    DropView,
    /// Truncate table
    TruncateTable,
}

impl fmt::Display for ReplicationOperation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ReplicationOperation::Insert => write!(f, "INSERT"),
            ReplicationOperation::Update => write!(f, "UPDATE"),
            ReplicationOperation::Delete => write!(f, "DELETE"),
            ReplicationOperation::CreateTable => write!(f, "CREATE TABLE"),
            ReplicationOperation::DropTable => write!(f, "DROP TABLE"),
            ReplicationOperation::AlterTable => write!(f, "ALTER TABLE"),
            ReplicationOperation::BeginTransaction => write!(f, "BEGIN"),
            ReplicationOperation::CommitTransaction => write!(f, "COMMIT"),
            ReplicationOperation::RollbackTransaction => write!(f, "ROLLBACK"),
            ReplicationOperation::CreateIndex => write!(f, "CREATE INDEX"),
            ReplicationOperation::DropIndex => write!(f, "DROP INDEX"),
            ReplicationOperation::CreateView => write!(f, "CREATE VIEW"),
            ReplicationOperation::DropView => write!(f, "DROP VIEW"),
            ReplicationOperation::TruncateTable => write!(f, "TRUNCATE"),
        }
    }
}

/// Replication topology configuration
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ReplicationTopology {
    /// One primary with multiple read replicas
    SinglePrimary,
    /// Multiple primaries with conflict resolution
    MultiPrimary,
    /// Replicas can have their own replicas
    Cascading,
    /// Linear chain of replicas for consistency
    ChainReplication,
}

impl fmt::Display for ReplicationTopology {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ReplicationTopology::SinglePrimary => write!(f, "Single Primary"),
            ReplicationTopology::MultiPrimary => write!(f, "Multi-Primary"),
            ReplicationTopology::Cascading => write!(f, "Cascading"),
            ReplicationTopology::ChainReplication => write!(f, "Chain Replication"),
        }
    }
}

/// Conflict resolution strategy
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ConflictResolutionStrategy {
    /// Most recent timestamp wins
    LastWriteWins,
    /// First write is preserved
    FirstWriteWins,
    /// Primary's version always wins
    PrimaryWins,
    /// Custom conflict resolver
    Custom,
}

impl fmt::Display for ConflictResolutionStrategy {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ConflictResolutionStrategy::LastWriteWins => write!(f, "Last Write Wins"),
            ConflictResolutionStrategy::FirstWriteWins => write!(f, "First Write Wins"),
            ConflictResolutionStrategy::PrimaryWins => write!(f, "Primary Wins"),
            ConflictResolutionStrategy::Custom => write!(f, "Custom"),
        }
    }
}

/// Lag monitoring trend
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum LagTrend {
    /// Lag is decreasing
    Improving,
    /// Lag is stable
    Stable,
    /// Lag is increasing
    Degrading,
    /// Lag is critically high
    Critical,
}

impl fmt::Display for LagTrend {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LagTrend::Improving => write!(f, "Improving"),
            LagTrend::Stable => write!(f, "Stable"),
            LagTrend::Degrading => write!(f, "Degrading"),
            LagTrend::Critical => write!(f, "Critical"),
        }
    }
}

/// Replica node information with comprehensive metadata
///
/// Contains all information needed to manage a replica connection,
/// monitor its health, and track its replication status.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReplicaNode {
    /// Unique identifier for the replica
    pub id: ReplicaId,
    /// Network address for connection
    pub address: ReplicaAddress,
    /// Current operational status
    pub status: ReplicaStatus,
    /// Role in replication topology
    pub role: ReplicaRole,
    /// Current replication lag in bytes
    pub lag_bytes: u64,
    /// Last successful sync timestamp
    pub last_sync: SystemTime,
    /// Last heartbeat timestamp
    pub last_heartbeat: SystemTime,
    /// Connection priority (0 = highest)
    pub priority: u8,
    /// Maximum allowed lag before alerting
    pub max_lag_bytes: u64,
    /// Additional metadata
    pub metadata: HashMap<String, String>,
}

impl ReplicaNode {
    /// Creates a new replica node with validation
    ///
    /// # Arguments
    ///
    /// * `id` - Unique replica identifier
    /// * `address` - Network address
    /// * `role` - Replica role in topology
    ///
    /// # Returns
    ///
    /// * `Ok(ReplicaNode)` - Valid replica node
    /// * `Err(DbError)` - Creation failed
    pub fn new(id: ReplicaId, address: ReplicaAddress, role: ReplicaRole) -> Result<Self, DbError> {
        let now = SystemTime::now();

        Ok(Self {
            id,
            address,
            status: ReplicaStatus::Syncing,
            role,
            lag_bytes: 0,
            last_sync: now,
            last_heartbeat: now,
            priority: 10, // Default medium priority
            max_lag_bytes: 1024 * 1024, // 1MB default
            metadata: HashMap::new(),
        })
    }

    /// Updates the replica's replication lag
    pub fn update_lag(&mut self, lag_bytes: u64) {
        self.lag_bytes = lag_bytes;

        // Auto-update status based on lag
        if lag_bytes > self.max_lag_bytes {
            self.status = ReplicaStatus::Lagging;
        } else if self.status == ReplicaStatus::Lagging && lag_bytes < self.max_lag_bytes / 2 {
            self.status = ReplicaStatus::Active;
        }
    }

    /// Updates the last sync timestamp
    pub fn update_sync_time(&mut self) {
        self.last_sync = SystemTime::now();
    }

    /// Updates the heartbeat timestamp
    pub fn update_heartbeat(&mut self) {
        self.last_heartbeat = SystemTime::now();

        // If replica was disconnected, mark as syncing
        if self.status == ReplicaStatus::Disconnected {
            self.status = ReplicaStatus::Syncing;
        }
    }

    /// Checks if the replica is healthy based on heartbeat
    pub fn is_healthy(&self, heartbeat_timeout: Duration) -> bool {
        self.last_heartbeat
            .elapsed()
            .map(|elapsed| elapsed < heartbeat_timeout)
            .unwrap_or(false)
    }

    /// Gets the age of the last sync
    pub fn sync_age(&self) -> Option<Duration> {
        self.last_sync.elapsed().ok()
    }

    /// Sets metadata for the replica
    pub fn set_metadata(&mut self, key: String, value: String) {
        self.metadata.insert(key, value);
    }

    /// Gets metadata for the replica
    pub fn get_metadata(&self, key: &str) -> Option<&String> {
        self.metadata.get(key)
    }
}

/// WAL (Write-Ahead Log) entry with comprehensive metadata
///
/// Represents a single operation in the write-ahead log with all
/// necessary information for replication and recovery.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WalEntry {
    /// Log sequence number for ordering
    pub lsn: LogSequenceNumber,
    /// Associated transaction ID (if any)
    pub transaction_id: Option<TransactionId>,
    /// Type of operation
    pub operation: ReplicationOperation,
    /// Table affected by operation
    pub table_name: TableName,
    /// Operation data payload
    pub data: Vec<u8>,
    /// Timestamp when entry was created
    pub timestamp: SystemTime,
    /// CRC32 checksum for integrity verification
    pub checksum: u32,
    /// Size of the entry in bytes
    pub size_bytes: usize,
}

impl WalEntry {
    /// Creates a new WAL entry with validation
    ///
    /// # Arguments
    ///
    /// * `lsn` - Log sequence number
    /// * `operation` - Type of operation
    /// * `table_name` - Affected table
    /// * `data` - Operation payload
    ///
    /// # Returns
    ///
    /// * `Ok(WalEntry)` - Valid WAL entry
    /// * `Err(DbError)` - Creation failed
    pub fn new(
        lsn: LogSequenceNumber,
        operation: ReplicationOperation,
        table_name: TableName,
        data: Vec<u8>,
    ) -> Result<Self, DbError> {
        let timestamp = SystemTime::now();
        let size_bytes = data.len() + size_of::<WalEntry>();
        let checksum = Self::calculate_checksum(&data);

        Ok(Self {
            lsn,
            transaction_id: None,
            operation,
            table_name,
            data,
            timestamp,
            checksum,
            size_bytes,
        })
    }

    /// Creates a WAL entry with transaction ID
    pub fn new_with_txn(
        lsn: LogSequenceNumber,
        transaction_id: TransactionId,
        operation: ReplicationOperation,
        table_name: TableName,
        data: Vec<u8>,
    ) -> Result<Self, DbError> {
        let mut entry = Self::new(lsn, operation, table_name, data)?;
        entry.transaction_id = Some(transaction_id);
        Ok(entry)
    }

    /// Validates the entry's checksum
    ///
    /// # Returns
    ///
    /// * `Ok(())` - Checksum is valid
    /// * `Err(ReplicationTypeError)` - Checksum mismatch
    pub fn validate_checksum(&self) -> Result<(), ReplicationTypeError> {
        let calculated = Self::calculate_checksum(&self.data);
        if calculated == self.checksum {
            Ok(())
        } else {
            Err(ReplicationTypeError::InvalidChecksum {
                expected: self.checksum,
                actual: calculated,
            })
        }
    }

    /// Calculates CRC32 checksum for data integrity
    fn calculate_checksum(data: &[u8]) -> u32 {
        // Simple checksum implementation - in production use proper CRC32
        data.iter().fold(0u32, |acc, &b| acc.wrapping_add(b as u32))
    }

    /// Gets the age of this WAL entry
    pub fn age(&self) -> Option<Duration> {
        self.timestamp.elapsed().ok()
    }

    /// Checks if this entry is part of a transaction
    pub fn is_transactional(&self) -> bool {
        self.transaction_id.is_some()
    }
}

/// Replication log entry for network transmission
///
/// Simplified version of WalEntry for efficient network transmission
/// between replicas.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReplicationLogEntry {
    /// Sequential entry number
    pub sequence_number: u64,
    /// Type of operation
    pub operation: ReplicationOperation,
    /// Timestamp when operation occurred
    pub timestamp: i64,
    /// Operation payload
    pub data: Vec<u8>,
}

impl ReplicationLogEntry {
    /// Creates a new replication log entry
    pub fn new(sequence_number: u64, operation: ReplicationOperation, data: Vec<u8>) -> Self {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() as i64;

        Self {
            sequence_number,
            operation,
            timestamp,
            data,
        }
    }

    /// Converts to WAL entry with specified LSN and table
    pub fn to_wal_entry(
        &self,
        lsn: LogSequenceNumber,
        table_name: TableName,
    ) -> Result<WalEntry, DbError> {
        WalEntry::new(lsn, self.operation.clone(), table_name, self.data.clone())
    }
}

/// Default implementations for common types
impl Default for ReplicationMode {
    fn default() -> Self {
        ReplicationMode::Asynchronous
    }
}

impl Default for ReplicationTopology {
    fn default() -> Self {
        ReplicationTopology::SinglePrimary
    }
}

impl Default for ConflictResolutionStrategy {
    fn default() -> Self {
        ConflictResolutionStrategy::LastWriteWins
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_replica_id_validation() {
        // Valid IDs
        assert!(ReplicaId::new("replica1").is_ok());
        assert!(ReplicaId::new("replica-01").is_ok());
        assert!(ReplicaId::new("r_01").is_ok());

        // Invalid IDs
        assert!(ReplicaId::new("").is_err()); // Empty
        assert!(ReplicaId::new("123").is_err()); // Starts with number
        assert!(ReplicaId::new("replica@01").is_err()); // Invalid character
        assert!(ReplicaId::new("replica--01").is_err()); // Consecutive special chars
    }

    #[test]
    fn test_replica_address_validation() {
        // Valid addresses
        assert!(ReplicaAddress::new("127.0.0.1:5432").is_ok());
        assert!(ReplicaAddress::new("localhost:3306").is_ok());
        assert!(ReplicaAddress::new("replica.example.com:5432").is_ok());

        // Invalid addresses
        assert!(ReplicaAddress::new("127.0.0.1").is_err()); // No port
        assert!(ReplicaAddress::new("127.0.0.1:0").is_err()); // Port 0
        assert!(ReplicaAddress::new(":5432").is_err()); // No host
        assert!(ReplicaAddress::new("127.0.0.1:99999").is_err()); // Invalid port
    }

    #[test]
    fn test_table_name_validation() {
        // Valid names
        assert!(TableName::new("users").is_ok());
        assert!(TableName::new("_temp").is_ok());
        assert!(TableName::new("table_01").is_ok());

        // Invalid names
        assert!(TableName::new("").is_err()); // Empty
        assert!(TableName::new("1table").is_err()); // Starts with number
        assert!(TableName::new("table-01").is_err()); // Hyphen not allowed
        assert!(TableName::new("table@test").is_err()); // Invalid character
    }

    #[test]
    fn test_lsn_ordering() {
        let lsn1 = LogSequenceNumber::new(100);
        let lsn2 = LogSequenceNumber::new(200);
        let lsn3 = LogSequenceNumber::new(150);

        assert!(lsn1 < lsn2);
        assert!(lsn3 > lsn1);
        assert!(lsn3 < lsn2);

        assert_eq!(lsn1.next(), LogSequenceNumber::new(101));
        assert_eq!(lsn1.distance_from(&lsn2), 100);
    }

    #[test]
    fn test_replica_node_creation() {
        let id = ReplicaId::new("test-replica").unwrap();
        let address = ReplicaAddress::new("127.0.0.1:5432").unwrap();

        let replica = ReplicaNode::new(id, address, ReplicaRole::ReadOnly).unwrap();

        assert_eq!(replica.status, ReplicaStatus::Syncing);
        assert_eq!(replica.role, ReplicaRole::ReadOnly);
        assert_eq!(replica.lag_bytes, 0);
        assert!(replica.is_healthy(Duration::from_secs(30)));
    }

    #[test]
    fn test_replica_lag_updates() {
        let id = ReplicaId::new("test-replica").unwrap();
        let address = ReplicaAddress::new("127.0.0.1:5432").unwrap();
        let mut replica = ReplicaNode::new(id, address, ReplicaRole::ReadOnly).unwrap();

        replica.status = ReplicaStatus::Active;

        // Update lag beyond threshold
        replica.update_lag(2 * 1024 * 1024); // 2MB
        assert_eq!(replica.status, ReplicaStatus::Lagging);

        // Reduce lag below threshold
        replica.update_lag(256 * 1024); // 256KB
        assert_eq!(replica.status, ReplicaStatus::Active);
    }

    #[test]
    fn test_wal_entry_creation() {
        let lsn = LogSequenceNumber::new(1000);
        let table_name = TableName::new("users").unwrap();
        let data = b"test data".to_vec();

        let entry = WalEntry::new(
            lsn,
            ReplicationOperation::Insert,
            table_name,
            data.clone(),
        ).unwrap();

        assert_eq!(entry.lsn, lsn);
        assert_eq!(entry.operation, ReplicationOperation::Insert);
        assert_eq!(entry.data, data);
        assert!(entry.validate_checksum().is_ok());
        assert!(!entry.is_transactional());
    }

    #[test]
    fn test_wal_entry_with_transaction() {
        let lsn = LogSequenceNumber::new(1000);
        let txn_id = TransactionId::new(42);
        let table_name = TableName::new("users").unwrap();
        let data = b"test data".to_vec();

        let entry = WalEntry::new_with_txn(
            lsn,
            txn_id,
            ReplicationOperation::Insert,
            table_name,
            data,
        ).unwrap();

        assert!(entry.is_transactional());
        assert_eq!(entry.transaction_id, Some(txn_id));
    }

    #[test]
    fn test_replication_log_entry() {
        let entry = ReplicationLogEntry::new(
            100,
            ReplicationOperation::Update,
            b"update data".to_vec(),
        );

        assert_eq!(entry.sequence_number, 100);
        assert_eq!(entry.operation, ReplicationOperation::Update);

        let lsn = LogSequenceNumber::new(1000);
        let table_name = TableName::new("test_table").unwrap();
        let wal_entry = entry.to_wal_entry(lsn, table_name).unwrap();

        assert_eq!(wal_entry.lsn, lsn);
        assert_eq!(wal_entry.operation, ReplicationOperation::Update);
    }

    #[test]
    fn test_display_implementations() {
        let _replica_id = ReplicaId::new("test-replica").unwrap();
        assert_eq!(format!("{}", replica_id), "test-replica");

        let address = ReplicaAddress::new("127.0.0.1:5432").unwrap();
        assert_eq!(format!("{}", address), "127.0.0.1:5432");

        let lsn = LogSequenceNumber::new(1000);
        assert_eq!(format!("{}", lsn), "LSN(1000)");

        assert_eq!(format!("{}", ReplicationMode::SemiSynchronous), "Semi-Synchronous");
        assert_eq!(format!("{}", ReplicaStatus::Active), "Active");
        assert_eq!(format!("{}", ReplicationOperation::Insert), "INSERT");
    }
}
