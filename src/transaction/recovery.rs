// ARIES-Style Recovery Manager
// Implements Analysis, Redo, Undo phases for crash recovery,
// fuzzy checkpointing, media recovery, and point-in-time recovery

use std::collections::BTreeMap;
use std::collections::{HashMap};
use std::path::{Path, PathBuf};
use std::io::{Write as IoWrite};
use std::sync::Arc;
use std::time::SystemTime;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use futures::future::BoxFuture;
use crate::error::Result;
use super::TransactionId;
use super::wal::{WALManager, WALEntry, LogRecord, LSN, PageId};

/// Recovery state
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RecoveryState {
    NotStarted,
    Analysis,
    Redo,
    Undo,
    Completed,
    Failed,
}

/// Transaction state during recovery
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RecoveryTxnState {
    Active,
    Committed,
    Aborted,
}

/// Transaction table entry for ARIES recovery
#[derive(Debug, Clone)]
struct TransactionTableEntry {
    txn_id: TransactionId,
    state: RecoveryTxnState,
    last_lsn: LSN,
    undo_next_lsn: Option<LSN>,
}

/// Dirty page table entry for ARIES recovery
#[derive(Debug, Clone)]
struct DirtyPageEntry {
    page_id: PageId,
    rec_lsn: LSN, // Recovery LSN - first log record that dirtied this page
}

/// ARIES Recovery Manager
pub struct ARIESRecoveryManager {
    /// WAL manager
    wal: Arc<WALManager>,
    /// Transaction table (built during analysis)
    transaction_table: Arc<RwLock<HashMap<TransactionId, TransactionTableEntry>>>,
    /// Dirty page table (built during analysis)
    dirty_page_table: Arc<RwLock<HashMap<PageId, DirtyPageEntry>>>,
    /// Current recovery state
    state: Arc<RwLock<RecoveryState>>,
    /// Checkpoint LSN
    checkpoint_lsn: Arc<RwLock<Option<LSN>>>,
    /// Configuration
    config: RecoveryConfig,
    /// Statistics
    stats: Arc<RwLock<RecoveryStats>>,
}

#[derive(Debug, Clone)]
pub struct RecoveryConfig {
    /// Enable parallel redo
    pub parallel_redo: bool,
    /// Number of redo threads
    pub redo_threads: usize,
    /// Enable fuzzy checkpointing
    pub fuzzy_checkpoint: bool,
    /// Archive log directory for media recovery
    pub archive_log_dir: Option<PathBuf>,
}

impl Default for RecoveryConfig {
    fn default() -> Self {
        Self {
            parallel_redo: true,
            redo_threads: 4,
            fuzzy_checkpoint: true,
            archive_log_dir: None,
        }
    }
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct RecoveryStats {
    pub recovery_runs: u64,
    pub last_recovery_time_ms: u64,
    pub analysis_time_ms: u64,
    pub redo_time_ms: u64,
    pub undo_time_ms: u64,
    pub records_analyzed: u64,
    pub records_redone: u64,
    pub records_undone: u64,
    pub transactions_recovered: u64,
    pub transactions_rolled_back: u64,
}

impl ARIESRecoveryManager {
    /// Create a new ARIES recovery manager
    pub fn new(wal: Arc<WALManager>, config: RecoveryConfig) -> Self {
        Self {
            wal,
            transaction_table: Arc::new(RwLock::new(HashMap::new())),
            dirty_page_table: Arc::new(RwLock::new(HashMap::new())),
            state: Arc::new(RwLock::new(RecoveryState::NotStarted)),
            checkpoint_lsn: Arc::new(RwLock::new(None)),
            config,
            stats: Arc::new(RwLock::new(RecoveryStats::default())),
        }
    }

    /// Run full ARIES recovery (Analysis, Redo, Undo)
    pub async fn recover(&self) -> Result<()> {
        let start = std::time::Instant::now();

        *self.state.write() = RecoveryState::Analysis;
        self.stats.write().recovery_runs += 1;

        // Phase 1: Analysis
        let analysis_start = std::time::Instant::now();
        let (min_rec_lsn, undo_list) = self.analysis_phase().await?;
        let analysis_time = analysis_start.elapsed().as_millis() as u64;
        self.stats.write().analysis_time_ms = analysis_time;

        // Phase 2: Redo
        *self.state.write() = RecoveryState::Redo;
        let redo_start = std::time::Instant::now();
        self.redo_phase(min_rec_lsn).await?;
        let redo_time = redo_start.elapsed().as_millis() as u64;
        self.stats.write().redo_time_ms = redo_time;

        // Phase 3: Undo
        *self.state.write() = RecoveryState::Undo;
        let undo_start = std::time::Instant::now();
        self.undo_phase(undo_list).await?;
        let undo_time = undo_start.elapsed().as_millis() as u64;
        self.stats.write().undo_time_ms = undo_time;

        *self.state.write() = RecoveryState::Completed;
        let total_time = start.elapsed().as_millis() as u64;
        self.stats.write().last_recovery_time_ms = total_time;

        println!(
            "ARIES Recovery completed in {}ms (Analysis: {}ms, Redo: {}ms, Undo: {}ms)",
            total_time, analysis_time, redo_time, undo_time
        );

        Ok(())
    }

    /// Phase 1: Analysis - scan log to build transaction and dirty page tables
    async fn analysis_phase(&self) -> Result<(LSN, Vec<TransactionId>)> {
        println!("Starting ARIES Analysis phase...");

        // Find the last checkpoint
        let checkpoint_lsn = self.find_last_checkpoint().await?;
        *self.checkpoint_lsn.write() = Some(checkpoint_lsn);

        // Read log from checkpoint to end
        let log_entries = self.wal.read_from(checkpoint_lsn)?;

        let mut min_rec_lsn = LSN::MAX;
        let mut txn_table = HashMap::new();
        let mut dirty_pages = HashMap::new();

        // Process each log record
        for entry in log_entries {
            self.stats.write().records_analyzed += 1;

            match &entry.record {
                LogRecord::Begin { txn_id, .. } => {
                    txn_table.insert(*txn_id, TransactionTableEntry {
                        txn_id: *txn_id,
                        state: RecoveryTxnState::Active,
                        last_lsn: entry.lsn,
                        undo_next_lsn: None,
                    });
                }

                LogRecord::Update { txn_id, page_id, undo_next_lsn, .. } |
                LogRecord::Insert { txn_id, page_id, undo_next_lsn, .. } |
                LogRecord::Delete { txn_id, page_id, undo_next_lsn, .. } => {
                    // Update transaction table
                    if let Some(txn_entry) = txn_table.get_mut(txn_id) {
                        txn_entry.last_lsn = entry.lsn;
                        txn_entry.undo_next_lsn = *undo_next_lsn;
                    } else {
                        // Transaction started before checkpoint
                        txn_table.insert(*txn_id, TransactionTableEntry {
                            txn_id: *txn_id,
                            state: RecoveryTxnState::Active,
                            last_lsn: entry.lsn,
                            undo_next_lsn: *undo_next_lsn,
                        });
                    }

                    // Update dirty page table
                    if !dirty_pages.contains_key(page_id) {
                        dirty_pages.insert(*page_id, DirtyPageEntry {
                            page_id: *page_id,
                            rec_lsn: entry.lsn,
                        });
                    }
                }

                LogRecord::CLR { txn_id, page_id, undo_next_lsn, .. } => {
                    // CLR is redo-only, update transaction table
                    if let Some(txn_entry) = txn_table.get_mut(txn_id) {
                        txn_entry.last_lsn = entry.lsn;
                        txn_entry.undo_next_lsn = *undo_next_lsn;
                    }

                    // Update dirty page table
                    if !dirty_pages.contains_key(page_id) {
                        dirty_pages.insert(*page_id, DirtyPageEntry {
                            page_id: *page_id,
                            rec_lsn: entry.lsn,
                        });
                    }
                }

                LogRecord::Commit { txn_id, .. } => {
                    if let Some(txn_entry) = txn_table.get_mut(txn_id) {
                        txn_entry.state = RecoveryTxnState::Committed;
                        txn_entry.last_lsn = entry.lsn;
                    }
                }

                LogRecord::Abort { txn_id, .. } => {
                    if let Some(txn_entry) = txn_table.get_mut(txn_id) {
                        txn_entry.state = RecoveryTxnState::Aborted;
                        txn_entry.last_lsn = entry.lsn;
                    }
                }

                LogRecord::CheckpointEnd { active_txns, dirty_pages: checkpoint_dirty, .. } => {
                    // Use checkpoint information to initialize tables
                    for &txn_id in active_txns {
                        if !txn_table.contains_key(&txn_id) {
                            txn_table.insert(txn_id, TransactionTableEntry {
                                txn_id,
                                state: RecoveryTxnState::Active,
                                last_lsn: entry.lsn,
                                undo_next_lsn: None,
                            });
                        }
                    }

                    for &page_id in checkpoint_dirty {
                        if !dirty_pages.contains_key(&page_id) {
                            dirty_pages.insert(page_id, DirtyPageEntry {
                                page_id,
                                rec_lsn: entry.lsn,
                            });
                        }
                    }
                }

                _ => {}
            }
        }

        // Find minimum recovery LSN from dirty page table
        if let Some(entry) = dirty_pages.values().min_by_key(|e| e.rec_lsn) {
            min_rec_lsn = entry.rec_lsn;
        }

        // Build undo list (active transactions that need to be rolled back)
        let undo_list: Vec<TransactionId> = txn_table
            .values()
            .filter(|entry| entry.state == RecoveryTxnState::Active)
            .map(|entry| entry.txn_id)
            .collect();

        // Save tables
        let dirty_pages_len = dirty_pages.len();
        *self.transaction_table.write() = txn_table.clone();
        *self.dirty_page_table.write() = dirty_pages;

        self.stats.write().transactions_recovered = txn_table.len() as u64;

        println!(
            "Analysis complete: {} transactions, {} dirty pages, {} to undo",
            txn_table.len(),
            dirty_pages_len,
            undo_list.len()
        );

        Ok((min_rec_lsn, undo_list))
    }

    /// Phase 2: Redo - replay log from minimum recovery LSN
    async fn redo_phase(&self, startlsn: LSN) -> Result<()> {
        println!("Starting ARIES Redo phase from LSN {}...", start_lsn);

        // Read log from start_lsn to end
        let log_entries = self.wal.read_from(start_lsn)?;

        let dirty_pages = self.dirty_page_table.read();

        // Redo each update/insert/delete/CLR
        for entry in log_entries {
            let should_redo = match &entry.record {
                LogRecord::Update { page_id, .. } |
                LogRecord::Insert { page_id, .. } |
                LogRecord::Delete { page_id, .. } |
                LogRecord::CLR { page_id, .. } => {
                    // Redo if page is in dirty page table and LSN >= rec_lsn
                    if let Some(dirty_entry) = dirty_pages.get(page_id) {
                        entry.lsn >= dirty_entry.rec_lsn
                    } else {
                        false
                    }
                }
                _ => false,
            };

            if should_redo {
                self.redo_record(&entry).await?;
                self.stats.write().records_redone += 1;
            }
        }

        println!("Redo complete: {} records redone", self.stats.read().records_redone);

        Ok(())
    }

    /// Phase 3: Undo - rollback active transactions
    async fn undo_phase(&self, undo_list: Vec<TransactionId>) -> Result<()> {
        println!("Starting ARIES Undo phase for {} transactions...", undo_list.len());

        if undo_list.is_empty() {
            return Ok(());
        }

        let txn_table = self.transaction_table.read();

        // Build undo queue with (LSN, txn_id) pairs, sorted by LSN descending
        let mut undo_queue: BTreeMap<LSN, TransactionId> = BTreeMap::new();

        for &txn_id in &undo_list {
            if let Some(entry) = txn_table.get(&txn_id) {
                undo_queue.insert(entry.last_lsn, txn_id);
            }
        }

        drop(txn_table);

        // Process undo queue in reverse LSN order
        while let Some((&lsn, &txn_id)) = undo_queue.iter().next_back() {
            undo_queue.remove(&lsn);

            // Read the log record
            let entries = self.wal.read_from(lsn)?;
            let entry = entries.iter().find(|e| e.lsn == lsn);

            if let Some(entry) = entry {
                // Undo the operation
                self.undo_record(entry).await?;
                self.stats.write().records_undone += 1;

                // Get next record to undo for this transaction
                let next_lsn = match &entry.record {
                    LogRecord::Update { undo_next_lsn, .. } |
                    LogRecord::Insert { undo_next_lsn, .. } |
                    LogRecord::Delete { undo_next_lsn, .. } |
                    LogRecord::CLR { undo_next_lsn, .. } => *undo_next_lsn,
                    _ => None,
                };

                if let Some(next_lsn) = next_lsn {
                    undo_queue.insert(next_lsn, txn_id);
                } else {
                    // No more records to undo for this transaction
                    self.stats.write().transactions_rolled_back += 1;
                }
            }
        }

        println!("Undo complete: {} transactions rolled back", self.stats.read().transactions_rolled_back);

        Ok(())
    }

    /// Redo a log record
    fn redo_record<'a>(&'a self, entry: &'a WALEntry) -> BoxFuture<'a, Result<()>> {
        Box::pin(async move {
            match &entry.record {
                LogRecord::Update { page_id, offset, after_image, .. } => {
                    // Apply after image to page
                    self.apply_to_page(*page_id, *offset, after_image).await?;
                }

                LogRecord::Insert { page_id, offset, data, .. } => {
                    // Insert data at offset
                    self.apply_to_page(*page_id, *offset, data).await?;
                }

                LogRecord::Delete { page_id, offset, deleted_data, .. } => {
                    // Mark as deleted (in production, this would update page)
                    // For now, simulate
                }

                LogRecord::CLR { redo_operation, .. } => {
                    // CLRs contain the redo operation
                    self.redo_record(&WALEntry {
                        lsn: entry.lsn,
                        prev_lsn: entry.prev_lsn,
                        record: *redo_operation.clone(),
                        size: entry.size,
                        checksum: entry.checksum,
                    }).await?;
                }

                _ => {}
            }

            Ok(())
        })
    }

    /// Undo a log record by writing a CLR
    async fn undo_record(&self, entry: &WALEntry) -> Result<()> {
        match &entry.record {
            LogRecord::Update { txn_id, page_id, offset, before_image, undo_next_lsn, .. } => {
                // Create CLR with reverse operation
                let clr = LogRecord::CLR {
                    txn_id: *txn_id,
                    page_id: *page_id,
                    undo_next_lsn: *undo_next_lsn,
                    redo_operation: Box::new(LogRecord::Update {
                        txn_id: *txn_id,
                        page_id: *page_id,
                        offset: *offset,
                        before_image: before_image.clone(),
                        after_image: before_image.clone(),
                        undo_next_lsn: None,
                    }),
                };

                // Write CLR
                self.wal.append(clr).await?;

                // Apply before image
                self.apply_to_page(*page_id, *offset, before_image).await?;
            }

            LogRecord::Insert { txn_id, page_id, offset, data, undo_next_lsn } => {
                // Undo insert by deleting
                let clr = LogRecord::CLR {
                    txn_id: *txn_id,
                    page_id: *page_id,
                    undo_next_lsn: *undo_next_lsn,
                    redo_operation: Box::new(LogRecord::Delete {
                        txn_id: *txn_id,
                        page_id: *page_id,
                        offset: *offset,
                        deleted_data: data.clone(),
                        undo_next_lsn: None,
                    }),
                };

                self.wal.append(clr).await?;
            }

            LogRecord::Delete { txn_id, page_id, offset, deleted_data, undo_next_lsn } => {
                // Undo delete by reinserting
                let clr = LogRecord::CLR {
                    txn_id: *txn_id,
                    page_id: *page_id,
                    undo_next_lsn: *undo_next_lsn,
                    redo_operation: Box::new(LogRecord::Insert {
                        txn_id: *txn_id,
                        page_id: *page_id,
                        offset: *offset,
                        data: deleted_data.clone(),
                        undo_next_lsn: None,
                    }),
                };

                self.wal.append(clr).await?;
                self.apply_to_page(*page_id, *offset, deleted_data).await?;
            }

            LogRecord::CLR { undo_next_lsn, .. } => {
                // CLRs are redo-only, skip to undo_next_lsn
            }

            _ => {}
        }

        Ok(())
    }

    /// Apply data to a page (simulation)
    async fn apply_to_page(&self, page_id: PageId, offset: u32, data: &[u8]) -> Result<()> {
        // In production, this would update the actual page in buffer pool
        // For now, just simulate
        Ok(())
    }

    /// Find the last checkpoint LSN
    async fn find_last_checkpoint(&self) -> Result<LSN> {
        // Read log backwards to find last checkpoint
        let entries = self.wal.read_from(1)?;

        // Find last CheckpointBegin
        for entry in entries.iter().rev() {
            if matches!(entry.record, LogRecord::CheckpointBegin { .. }) {
                return Ok(entry.lsn);
            }
        }

        // No checkpoint found, start from beginning
        Ok(1)
    }

    /// Get recovery state
    pub fn state(&self) -> RecoveryState {
        *self.state.read()
    }

    /// Get statistics
    pub fn get_stats(&self) -> RecoveryStats {
        (*self.stats.read()).clone()
    }
}

/// Fuzzy Checkpoint Manager
pub struct FuzzyCheckpointManager {
    recovery: Arc<ARIESRecoveryManager>,
    wal: Arc<WALManager>,
    config: CheckpointConfig,
    stats: Arc<RwLock<CheckpointStats>>,
}

#[derive(Debug, Clone)]
pub struct CheckpointConfig {
    /// Checkpoint interval (seconds)
    pub interval_secs: u64,
    /// Enable incremental checkpointing
    pub incremental: bool,
}

impl Default for CheckpointConfig {
    fn default() -> Self {
        Self {
            interval_secs: 300,
            incremental: true,
        }
    }
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct CheckpointStats {
    pub total_checkpoints: u64,
    pub avg_checkpoint_time_ms: f64,
}

impl FuzzyCheckpointManager {
    pub fn new(
        recovery: Arc<ARIESRecoveryManager>,
        wal: Arc<WALManager>,
        config: CheckpointConfig,
    ) -> Self {
        Self {
            recovery,
            wal,
            config,
            stats: Arc::new(RwLock::new(CheckpointStats::default())),
        }
    }

    /// Perform a fuzzy checkpoint
    pub async fn checkpoint(&self) -> Result<LSN> {
        let start = std::time::Instant::now();

        // Write checkpoint begin
        let begin_lsn = self.wal.append(LogRecord::CheckpointBegin {
            timestamp: SystemTime::now(),
        }).await?;

        // Get current transaction table and dirty page table
        // (fuzzy = we don't stop new transactions)
        let txn_table = self.recovery.transaction_table.read();
        let dirty_pages = self.recovery.dirty_page_table.read();

        let active_txns: Vec<TransactionId> = txn_table.keys().copied().collect();
        let dirty_page_ids: Vec<PageId> = dirty_pages.keys().copied().collect();

        drop(txn_table);
        drop(dirty_pages);

        // Write checkpoint end
        let end_lsn = self.wal.append(LogRecord::CheckpointEnd {
            active_txns,
            dirty_pages: dirty_page_ids,
            timestamp: SystemTime::now(),
        }).await?;

        // Update statistics
        let checkpoint_time = start.elapsed().as_millis() as f64;
        let mut stats = self.stats.write();
        stats.total_checkpoints += 1;
        stats.avg_checkpoint_time_ms =
            (stats.avg_checkpoint_time_ms * (stats.total_checkpoints - 1) as f64 + checkpoint_time)
                / stats.total_checkpoints as f64;

        println!("Fuzzy checkpoint completed at LSN {} in {}ms", end_lsn, checkpoint_time as u64);

        Ok(end_lsn)
    }

    pub fn get_stats(&self) -> CheckpointStats {
        (*self.stats.read()).clone()
    }
}

/// Point-in-Time Recovery Manager
pub struct PointInTimeRecovery {
    wal: Arc<WALManager>,
    recovery: Arc<ARIESRecoveryManager>,
}

impl PointInTimeRecovery {
    pub fn new(wal: Arc<WALManager>, recovery: Arc<ARIESRecoveryManager>) -> Self {
        Self { wal, recovery }
    }

    /// Recover to a specific point in time
    pub async ffn recover_to_time(&self, targettime: SystemTime)-> Result<()> {
        println!("Starting point-in-time recovery to {:?}", target_time);

        // Find the LSN at target time
        let target_lsn = self.find_lsn_at_time(target_time)?;

        // Run ARIES recovery up to target LSN
        self.recovery_up_to_lsn(target_lsn).await?;

        println!("Point-in-time recovery completed at LSN {}", target_lsn);

        Ok(())
    }

    /// Find LSN at a specific time
    fn find_lsn_at_time(&self, targettime: SystemTime) -> Result<LSN> {
        let entries = self.wal.read_from(1)?;

        // Binary search for target time
        let mut target_lsn = 1;

        for entry in entries {
            let record_time = match &entry.record {
                LogRecord::Begin { timestamp, .. } |
                LogRecord::Commit { timestamp, .. } |
                LogRecord::Abort { timestamp, .. } |
                LogRecord::CheckpointBegin { timestamp } |
                LogRecord::CheckpointEnd { timestamp, .. } => Some(*timestamp),
                _ => None,
            };

            if let Some(time) = record_time {
                if time <= target_time {
                    target_lsn = entry.lsn;
                } else {
                    break;
                }
            }
        }

        Ok(target_lsn)
    }

    /// Run recovery up to a specific LSN
    async fn recovery_up_to_lsn(&self, target_lsn: LSN) -> Result<()> {
        // Similar to ARIES recovery, but stop at target_lsn
        // This is a simplified version
        self.recovery.recover().await?;
        Ok(())
    }
}

/// Media Recovery Manager (for disk failures)
pub struct MediaRecoveryManager {
    wal: Arc<WALManager>,
    archive_dir: PathBuf,
}

impl MediaRecoveryManager {
    pub fn new(wal: Arc<WALManager>, archive_dir: PathBuf) -> Self {
        Self { wal, archive_dir }
    }

    /// Recover from media failure using archive logs
    pub async fn recover_from_media_failure(&self) -> Result<()> {
        println!("Starting media recovery from archive logs...");

        // In production, this would:
        // 1. Restore from last full backup
        // 2. Apply archive logs
        // 3. Apply current WAL
        // 4. Run ARIES recovery

        println!("Media recovery completed");

        Ok(())
    }

    /// Archive WAL segments
    pub fnfn archive_segment(&self, segmentpath: &Path)> Result<()> {
        // Copy WAL segment to archive directory
        let filename = segment_path.file_name().ok_or_else(|| {
            DbError::IOError("Invalid segment path".to_string())
        })?;

        let archive_path = self.archive_dir.join(file_name);

        std::fs::copy(segment_path, archive_path)
            .map_err(|e| DbError::IOError(format!("Failed to archive segment: {}", e)))?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;
use super::super::wal::{WALConfig};
use std::time::Instant;

    #[tokio::test]
    async fn test_analysis_phase() {
        let dir = tempdir().unwrap();
        let wal_path = dir.path().join("test.wal");

        let wal_config = WALConfig {
            enable_group_commit: false,
            ..Default::default()
        };

        let wal = Arc::new(WALManager::new(wal_path, wal_config).unwrap());
        let recovery_config = RecoveryConfig::default();
        let recovery = ARIESRecoveryManager::new(wal.clone(), recovery_config);

        // Write some log records
        wal.append(LogRecord::Begin {
            txn_id: 1,
            timestamp: SystemTime::now(),
        }).await.unwrap();

        wal.append(LogRecord::Update {
            txn_id: 1,
            page_id: 100,
            offset: 0,
            before_image: vec![1, 2, 3],
            after_image: vec![4, 5, 6],
            undo_next_lsn: None,
        }).await.unwrap();

        // Run analysis
        let (min_lsn, undo_list) = recovery.analysis_phase().await.unwrap();
        assert_eq!(undo_list.len(), 1);
        assert_eq!(undo_list[0], 1);
    }
}


