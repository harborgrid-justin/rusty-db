// Raft Log Management
//
// This module manages the replicated log in Raft consensus.
// Features:
// - Append-only log with term and index
// - Log compaction via snapshots
// - Persistent storage support
// - Efficient log matching

#![allow(dead_code)]

use crate::error::{DbError, Result};
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::time::SystemTime;

/// Log index type
pub type LogIndex = u64;

/// Log entry in the Raft log
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogEntry {
    /// Term when entry was received by leader
    pub term: u64,

    /// Index of this entry in the log
    pub index: LogIndex,

    /// Command data (serialized MembershipCommand)
    pub data: Vec<u8>,

    /// Timestamp when entry was created
    pub timestamp: SystemTime,
}

impl LogEntry {
    /// Create a new log entry
    pub fn new(term: u64, index: LogIndex, data: Vec<u8>) -> Self {
        Self {
            term,
            index,
            data,
            timestamp: SystemTime::now(),
        }
    }
}

/// Raft log structure
pub struct RaftLog {
    /// Log entries (index 0 is reserved)
    entries: VecDeque<LogEntry>,

    /// Snapshot metadata
    snapshot: Option<Snapshot>,

    /// First log index (after snapshot)
    first_index: LogIndex,
}

impl RaftLog {
    /// Create a new empty log
    pub fn new() -> Self {
        Self {
            entries: VecDeque::new(),
            snapshot: None,
            first_index: 1,
        }
    }

    /// Get the index of the last log entry
    pub fn last_index(&self) -> LogIndex {
        if let Some(last) = self.entries.back() {
            last.index
        } else if let Some(snapshot) = &self.snapshot {
            snapshot.last_included_index
        } else {
            0
        }
    }

    /// Get the term of the last log entry
    pub fn last_term(&self) -> u64 {
        if let Some(last) = self.entries.back() {
            last.term
        } else if let Some(snapshot) = &self.snapshot {
            snapshot.last_included_term
        } else {
            0
        }
    }

    /// Get a log entry by index
    pub fn get(&self, index: LogIndex) -> Option<&LogEntry> {
        if index < self.first_index {
            return None;
        }

        let offset = (index - self.first_index) as usize;
        self.entries.get(offset)
    }

    /// Get the term of a log entry at given index
    pub fn term_at(&self, index: LogIndex) -> Option<u64> {
        if index == 0 {
            return Some(0);
        }

        if let Some(snapshot) = &self.snapshot {
            if index == snapshot.last_included_index {
                return Some(snapshot.last_included_term);
            }
        }

        self.get(index).map(|entry| entry.term)
    }

    /// Append a new log entry
    pub fn append(&mut self, entry: LogEntry) -> Result<()> {
        if entry.index != self.last_index() + 1 {
            return Err(DbError::InvalidState(format!(
                "Log entry index mismatch: expected {}, got {}",
                self.last_index() + 1,
                entry.index
            )));
        }

        self.entries.push_back(entry);
        Ok(())
    }

    /// Append multiple log entries
    pub fn append_entries(&mut self, entries: Vec<LogEntry>) -> Result<()> {
        for entry in entries {
            self.append(entry)?;
        }
        Ok(())
    }

    /// Truncate log from given index (used when log conflicts)
    pub fn truncate_from(&mut self, index: LogIndex) -> Result<()> {
        if index < self.first_index {
            return Err(DbError::InvalidOperation(
                "Cannot truncate before first index".to_string(),
            ));
        }

        let offset = (index - self.first_index) as usize;
        self.entries.truncate(offset);
        Ok(())
    }

    /// Get log entries in range [from, to]
    pub fn get_range(&self, from: LogIndex, to: LogIndex) -> Vec<LogEntry> {
        if from < self.first_index || from > to {
            return Vec::new();
        }

        let start_offset = (from - self.first_index) as usize;
        let end_offset = (to - self.first_index + 1) as usize;

        self.entries
            .iter()
            .skip(start_offset)
            .take(end_offset - start_offset)
            .cloned()
            .collect()
    }

    /// Create a snapshot of the log up to given index
    pub fn create_snapshot(
        &mut self,
        last_index: LogIndex,
        last_term: u64,
        data: Vec<u8>,
    ) -> Result<()> {
        if last_index > self.last_index() {
            return Err(DbError::InvalidOperation(
                "Cannot snapshot beyond last index".to_string(),
            ));
        }

        let snapshot = Snapshot {
            last_included_index: last_index,
            last_included_term: last_term,
            data,
            created_at: SystemTime::now(),
        };

        // Remove log entries that are now in the snapshot
        if last_index >= self.first_index {
            let entries_to_remove = (last_index - self.first_index + 1) as usize;
            self.entries.drain(..entries_to_remove);
            self.first_index = last_index + 1;
        }

        self.snapshot = Some(snapshot);
        Ok(())
    }

    /// Install a snapshot (used when receiving snapshot from leader)
    pub fn install_snapshot(&mut self, snapshot: Snapshot) -> Result<()> {
        // Discard all existing log entries
        self.entries.clear();
        self.first_index = snapshot.last_included_index + 1;
        self.snapshot = Some(snapshot);
        Ok(())
    }

    /// Get current snapshot
    pub fn snapshot(&self) -> Option<&Snapshot> {
        self.snapshot.as_ref()
    }

    /// Get total number of entries (excluding snapshot)
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Check if log is empty
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty() && self.snapshot.is_none()
    }

    /// Get total size of log entries in bytes
    pub fn size_bytes(&self) -> usize {
        self.entries.iter().map(|e| e.data.len()).sum()
    }

    /// Check if log needs compaction
    pub fn needs_compaction(&self, threshold: u64) -> bool {
        self.entries.len() as u64 > threshold
    }

    /// Compact log by creating snapshot if needed
    pub async fn compact_if_needed(&mut self, threshold: u64, state_data: Vec<u8>) -> Result<()> {
        if self.needs_compaction(threshold) {
            let last_index = self.last_index();
            let last_term = self.last_term();
            self.create_snapshot(last_index, last_term, state_data)?;
        }
        Ok(())
    }
}

/// Snapshot metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Snapshot {
    /// Index of last log entry included in snapshot
    pub last_included_index: LogIndex,

    /// Term of last log entry included in snapshot
    pub last_included_term: u64,

    /// Snapshot data (serialized state machine state)
    pub data: Vec<u8>,

    /// When snapshot was created
    pub created_at: SystemTime,
}

impl Snapshot {
    /// Get snapshot size in bytes
    pub fn size_bytes(&self) -> usize {
        self.data.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_log() {
        let log = RaftLog::new();
        assert_eq!(log.last_index(), 0);
        assert_eq!(log.last_term(), 0);
        assert!(log.is_empty());
    }

    #[test]
    fn test_append_entries() {
        let mut log = RaftLog::new();

        let entry1 = LogEntry::new(1, 1, vec![1, 2, 3]);
        assert!(log.append(entry1).is_ok());
        assert_eq!(log.last_index(), 1);

        let entry2 = LogEntry::new(1, 2, vec![4, 5, 6]);
        assert!(log.append(entry2).is_ok());
        assert_eq!(log.last_index(), 2);
        assert_eq!(log.len(), 2);
    }

    #[test]
    fn test_append_with_gap_fails() {
        let mut log = RaftLog::new();
        let entry = LogEntry::new(1, 5, vec![1, 2, 3]);
        assert!(log.append(entry).is_err());
    }

    #[test]
    fn test_get_entry() {
        let mut log = RaftLog::new();
        let entry1 = LogEntry::new(1, 1, vec![1, 2, 3]);
        log.append(entry1.clone()).unwrap();

        let retrieved = log.get(1).unwrap();
        assert_eq!(retrieved.index, 1);
        assert_eq!(retrieved.term, 1);
        assert_eq!(retrieved.data, vec![1, 2, 3]);
    }

    #[test]
    fn test_truncate() {
        let mut log = RaftLog::new();
        log.append(LogEntry::new(1, 1, vec![1])).unwrap();
        log.append(LogEntry::new(1, 2, vec![2])).unwrap();
        log.append(LogEntry::new(2, 3, vec![3])).unwrap();

        log.truncate_from(2).unwrap();
        assert_eq!(log.last_index(), 1);
        assert_eq!(log.len(), 1);
    }

    #[test]
    fn test_get_range() {
        let mut log = RaftLog::new();
        log.append(LogEntry::new(1, 1, vec![1])).unwrap();
        log.append(LogEntry::new(1, 2, vec![2])).unwrap();
        log.append(LogEntry::new(2, 3, vec![3])).unwrap();

        let range = log.get_range(1, 2);
        assert_eq!(range.len(), 2);
        assert_eq!(range[0].index, 1);
        assert_eq!(range[1].index, 2);
    }

    #[test]
    fn test_snapshot_creation() {
        let mut log = RaftLog::new();
        log.append(LogEntry::new(1, 1, vec![1])).unwrap();
        log.append(LogEntry::new(1, 2, vec![2])).unwrap();
        log.append(LogEntry::new(2, 3, vec![3])).unwrap();

        log.create_snapshot(2, 1, vec![1, 2]).unwrap();

        assert_eq!(log.first_index, 3);
        assert_eq!(log.len(), 1); // Only entry 3 remains
        assert!(log.snapshot().is_some());
    }

    #[test]
    fn test_needs_compaction() {
        let mut log = RaftLog::new();
        assert!(!log.needs_compaction(10));

        for i in 1..=15 {
            log.append(LogEntry::new(1, i, vec![i as u8])).unwrap();
        }

        assert!(log.needs_compaction(10));
        assert!(!log.needs_compaction(20));
    }
}
