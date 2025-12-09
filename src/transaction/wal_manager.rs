// Write-Ahead Log (WAL) management.
//
// This module provides the WAL infrastructure for ensuring durability
// and enabling crash recovery using the ARIES protocol.
//
// # Key Concepts
//
// - **Write-Ahead Logging**: All modifications are logged before being applied.
// - **Log Sequence Number (LSN)**: Monotonically increasing identifier for log entries.
// - **Force-at-Commit**: Log is flushed to disk before commit returns.
//
// # Example
//
// ```rust,ignore
// let wal = WALManager::new("./wal/", 100, true)?;
// let lsn = wal.append(WALEntry::Begin { txn_id: 1, ... })?;
// wal.flush()?;
// ```

use std::fmt;
use std::collections::VecDeque;
use std::fs::{File, OpenOptions};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::SystemTime;

use parking_lot::Mutex;
use serde::{Deserialize, Serialize};

use crate::common::{LogSequenceNumber, TransactionId};

use super::error::{TransactionError, TransactionResult};
use super::types::IsolationLevel;

/// Write-Ahead Log entry types.
///
/// Each entry captures a specific operation for recovery purposes.
/// Entries are self-describing and can be replayed in order.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum WALEntry {
    /// Transaction begin marker.
    Begin {
        txn_id: TransactionId,
        isolation_level: IsolationLevel,
        timestamp: SystemTime,
    },

    /// Transaction commit marker (durable commit point).
    Commit {
        txn_id: TransactionId,
        lsn: LogSequenceNumber,
        timestamp: SystemTime,
    },

    /// Transaction abort marker.
    Abort {
        txn_id: TransactionId,
        lsn: LogSequenceNumber,
        timestamp: SystemTime,
    },

    /// Row insert operation.
    Insert {
        txn_id: TransactionId,
        table: String,
        key: String,
        value: Vec<u8>,
        lsn: LogSequenceNumber,
    },

    /// Row update operation (with before-image for undo).
    Update {
        txn_id: TransactionId,
        table: String,
        key: String,
        old_value: Vec<u8>,
        new_value: Vec<u8>,
        lsn: LogSequenceNumber,
    },

    /// Row delete operation (with before-image for undo).
    Delete {
        txn_id: TransactionId,
        table: String,
        key: String,
        value: Vec<u8>,
        lsn: LogSequenceNumber,
    },

    /// Checkpoint marker for faster recovery.
    Checkpoint {
        lsn: LogSequenceNumber,
        active_txns: Vec<TransactionId>,
        timestamp: SystemTime,
    },

    /// Savepoint creation.
    Savepoint {
        txn_id: TransactionId,
        name: String,
        lsn: LogSequenceNumber,
    },

    /// Rollback to savepoint.
    RollbackToSavepoint {
        txn_id: TransactionId,
        savepoint_name: String,
        lsn: LogSequenceNumber,
    },

    /// Compensation Log Record (CLR) for undo operations.
    Compensation {
        txn_id: TransactionId,
        undo_next_lsn: LogSequenceNumber,
        lsn: LogSequenceNumber,
    },

    /// End of transaction (after commit/abort processing complete).
    End {
        txn_id: TransactionId,
        lsn: LogSequenceNumber,
    },
}

impl WALEntry {
    /// Returns the transaction ID associated with this entry, if any.
    pub fn txn_id(&self) -> Option<TransactionId> {
        match self {
            WALEntry::Begin { txn_id, .. } => Some(*txn_id),
            WALEntry::Commit { txn_id, .. } => Some(*txn_id),
            WALEntry::Abort { txn_id, .. } => Some(*txn_id),
            WALEntry::Insert { txn_id, .. } => Some(*txn_id),
            WALEntry::Update { txn_id, .. } => Some(*txn_id),
            WALEntry::Delete { txn_id, .. } => Some(*txn_id),
            WALEntry::Savepoint { txn_id, .. } => Some(*txn_id),
            WALEntry::RollbackToSavepoint { txn_id, .. } => Some(*txn_id),
            WALEntry::Compensation { txn_id, .. } => Some(*txn_id),
            WALEntry::End { txn_id, .. } => Some(*txn_id),
            WALEntry::Checkpoint { .. } => None,
        }
    }

    /// Returns the LSN of this entry, if present.
    pub fn lsn(&self) -> Option<LogSequenceNumber> {
        match self {
            WALEntry::Commit { lsn, .. } => Some(*lsn),
            WALEntry::Abort { lsn, .. } => Some(*lsn),
            WALEntry::Insert { lsn, .. } => Some(*lsn),
            WALEntry::Update { lsn, .. } => Some(*lsn),
            WALEntry::Delete { lsn, .. } => Some(*lsn),
            WALEntry::Checkpoint { lsn, .. } => Some(*lsn),
            WALEntry::Savepoint { lsn, .. } => Some(*lsn),
            WALEntry::RollbackToSavepoint { lsn, .. } => Some(*lsn),
            WALEntry::Compensation { lsn, .. } => Some(*lsn),
            WALEntry::End { lsn, .. } => Some(*lsn),
            WALEntry::Begin { .. } => None,
        }
    }

    /// Returns true if this is a terminal entry (commit/abort).
    pub fn is_terminal(&self) -> bool {
        matches!(self, WALEntry::Commit { .. } | WALEntry::Abort { .. })
    }

    /// Returns true if this entry is undoable.
    pub fn is_undoable(&self) -> bool {
        matches!(
            self,
            WALEntry::Insert { .. } | WALEntry::Update { .. } | WALEntry::Delete { .. }
        )
    }
}

/// Configuration for WAL manager.
#[derive(Debug, Clone)]
pub struct WALConfig {
    /// Path to the WAL directory.
    pub log_path: PathBuf,
    /// Number of entries to buffer before auto-flush.
    pub buffer_size: usize,
    /// Whether to sync on every commit.
    pub sync_on_commit: bool,
    /// Maximum log file size before rotation.
    pub max_file_size: Option<u64>,
}

impl Default for WALConfig {
    fn default() -> Self {
        Self {
            log_path: PathBuf::from("./wal/transaction.log"),
            buffer_size: 100,
            sync_on_commit: true,
            max_file_size: Some(100 * 1024 * 1024), // 100MB
        }
    }
}

/// Write-Ahead Log Manager.
///
/// Manages the write-ahead log for durability and recovery.
/// Thread-safe and can be shared across multiple transactions.
///
/// # Thread Safety
///
/// All operations are protected by internal locks. Multiple threads
/// can safely append entries concurrently.
///
/// # Durability Guarantees
///
/// When `sync_on_commit` is true, committed transactions are guaranteed
/// to survive crashes. Otherwise, data may be lost in a buffer.
pub struct WALManager {
    /// Path to the log file.
    log_path: PathBuf,
    /// Current log sequence number (monotonically increasing).
    current_lsn: Arc<Mutex<LogSequenceNumber>>,
    /// Buffer of entries awaiting flush.
    log_buffer: Arc<Mutex<VecDeque<WALEntry>>>,
    /// Maximum buffer size before auto-flush.
    buffer_size: usize,
    /// Whether to sync to disk on commit.
    sync_on_commit: bool,
}

impl WALManager {
    /// Creates a new WAL manager.
    ///
    /// # Arguments
    ///
    /// * `log_path` - Path to the WAL file.
    /// * `buffer_size` - Number of entries to buffer before auto-flush.
    /// * `sync_on_commit` - Whether to sync to disk on every commit.
    ///
    /// # Errors
    ///
    /// Returns an error if the log directory cannot be created.
    pub fn new(log_path: PathBuf, buffer_size: usize, sync_on_commit: bool) -> TransactionResult<Self> {
        if let Some(parent) = log_path.parent() {
            std::fs::create_dir_all(parent).map_err(TransactionError::WalWriteError)?;
        }

        Ok(Self {
            log_path,
            current_lsn: Arc::new(Mutex::new(1)),
            log_buffer: Arc::new(Mutex::new(VecDeque::with_capacity(buffer_size))),
            buffer_size,
            sync_on_commit,
        })
    }

    /// Creates a WAL manager from configuration.
    pub fn from_config(config: WALConfig) -> TransactionResult<Self> {
        Self::new(config.log_path, config.buffer_size, config.sync_on_commit)
    }

    /// Appends an entry to the WAL.
    ///
    /// # Arguments
    ///
    /// * `entry` - The WAL entry to append.
    ///
    /// # Returns
    ///
    /// The LSN assigned to this entry.
    ///
    /// # Errors
    ///
    /// Returns an error if the buffer flush fails.
    pub fn append(&self, entry: WALEntry) -> TransactionResult<LogSequenceNumber> {
        let lsn = {
            let mut current_lsn = self.current_lsn.lock();
            let lsn = *current_lsn;
            *current_lsn += 1;
            lsn
        };

        let mut buffer = self.log_buffer.lock();
        buffer.push_back(entry);

        if buffer.len() >= self.buffer_size {
            self.flush_internal(&mut buffer)?;
        }

        Ok(lsn)
    }

    /// Forces all buffered entries to disk.
    ///
    /// # Errors
    ///
    /// Returns an error if the flush fails.
    pub fn flush(&self) -> TransactionResult<()> {
        let mut buffer = self.log_buffer.lock();
        self.flush_internal(&mut buffer)
    }

    /// Internal flush implementation.
    fn flush_internal(&self, buffer: &mut VecDeque<WALEntry>) -> TransactionResult<()> {
        if buffer.is_empty() {
            return Ok(());
        }

        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.log_path)
            .map_err(TransactionError::WalWriteError)?;

        for entry in buffer.drain(..) {
            let serialized = serde_json::to_vec(&entry)?;

            // Write length prefix (4 bytes, little-endian).
            file.write_all(&(serialized.len() as u32).to_le_bytes())
                .map_err(TransactionError::WalWriteError)?;

            // Write entry data.
            file.write_all(&serialized)
                .map_err(TransactionError::WalWriteError)?;
        }

        if self.sync_on_commit {
            file.sync_all().map_err(TransactionError::WalWriteError)?;
        }

        Ok(())
    }

    /// Replays the log from disk.
    ///
    /// # Returns
    ///
    /// A vector of all entries in the log.
    ///
    /// # Errors
    ///
    /// Returns an error if reading the log fails.
    pub fn replay(&self) -> TransactionResult<Vec<WALEntry>> {
        let mut entries = Vec::new();

        if !self.log_path.exists() {
            return Ok(entries);
        }

        let mut file = File::open(&self.log_path).map_err(TransactionError::WalReadError)?;
        let mut length_buf = [0u8; 4];

        loop {
            // Read length prefix.
            match file.read_exact(&mut length_buf) {
                Ok(_) => {}
                Err(ref e) if e.kind() == std::io::ErrorKind::UnexpectedEof => break,
                Err(e) => return Err(TransactionError::WalReadError(e)),
            }

            let length = u32::from_le_bytes(length_buf) as usize;
            let mut entry_buf = vec![0u8; length];

            file.read_exact(&mut entry_buf)
                .map_err(TransactionError::WalReadError)?;

            let entry: WALEntry = serde_json::from_slice(&entry_buf)?;
            entries.push(entry);
        }

        Ok(entries)
    }

    /// Truncates the log before a given LSN.
    ///
    /// Used after checkpointing to reclaim space.
    ///
    /// # Arguments
    ///
    /// * `before_lsn` - Entries with LSN < this value will be removed.
    ///
    /// # Note
    ///
    /// This is a placeholder implementation. In production, this would
    /// create a new log file and copy only entries >= before_lsn.
    pub fn truncate(&self, _before_lsn: LogSequenceNumber) -> TransactionResult<()> {
        // TODO: Implement log truncation/compaction
        Ok(())
    }

    /// Returns the current LSN.
    pub fn get_current_lsn(&self) -> LogSequenceNumber {
        *self.current_lsn.lock()
    }

    /// Returns the path to the log file.
    pub fn log_path(&self) -> &Path {
        &self.log_path
    }

    /// Returns the number of buffered entries.
    pub fn buffered_count(&self) -> usize {
        self.log_buffer.lock().len()
    }

    /// Checks if the log file exists.
    pub fn exists(&self) -> bool {
        self.log_path.exists()
    }

    /// Returns the size of the log file in bytes.
    pub fn file_size(&self) -> TransactionResult<u64> {
        if !self.log_path.exists() {
            return Ok(0);
        }
        std::fs::metadata(&self.log_path)
            .map(|m| m.len())
            .map_err(TransactionError::WalReadError)
    }
}

impl fmt::Debug for WALManager {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("WALManager")
            .field("log_path", &self.log_path)
            .field("current_lsn", &*self.current_lsn.lock())
            .field("buffer_size", &self.buffer_size)
            .field("sync_on_commit", &self.sync_on_commit)
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_log_path() -> PathBuf {
        let id: u64 = rand::random();
        std::env::temp_dir().join(format!("test_wal_{}.log", id))
    }

    #[test]
    fn test_wal_manager_creation() {
        let path = temp_log_path();
        let wal = WALManager::new(path.clone(), 100, true);
        assert!(wal.is_ok());
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn test_wal_entry_txn_id() {
        let entry = WALEntry::Begin {
            txn_id: 42,
            isolation_level: IsolationLevel::ReadCommitted,
            timestamp: SystemTime::now(),
        };
        assert_eq!(entry.txn_id(), Some(42));

        let checkpoint = WALEntry::Checkpoint {
            lsn: 1,
            active_txns: vec![],
            timestamp: SystemTime::now(),
        };
        assert_eq!(checkpoint.txn_id(), None);
    }

    #[test]
    fn test_wal_entry_is_undoable() {
        let insert = WALEntry::Insert {
            txn_id: 1,
            table: "t".to_string(),
            key: "k".to_string(),
            value: vec![],
            lsn: 1,
        };
        assert!(insert.is_undoable());

        let begin = WALEntry::Begin {
            txn_id: 1,
            isolation_level: IsolationLevel::ReadCommitted,
            timestamp: SystemTime::now(),
        };
        assert!(!begin.is_undoable());
    }
}
