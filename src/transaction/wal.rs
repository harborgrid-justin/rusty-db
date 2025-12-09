// Write-Ahead Logging (WAL) Implementation
// Provides ARIES-style physiological logging with group commit,
// log shipping for replication, and checkpoint coordination

use tokio::sync::oneshot;
use std::collections::HashMap;
use std::path::PathBuf;
use std::fs::{File, OpenOptions};
use std::io::{Write as IoWrite, BufWriter, BufReader, IoSlice};
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, AtomicBool, Ordering};
use std::time::{SystemTime, Duration, Instant};
use parking_lot::{Mutex, RwLock};
use serde::{Deserialize, Serialize};
use tokio::time::interval;
use crate::error::{Result, DbError};
use super::TransactionId;

#[cfg(target_arch = "x86_64")]
use std::arch::x86_64::*;

/// Log Sequence Number type
pub type LSN = u64;

/// Page ID type
pub type PageId = u64;

/// Hardware-accelerated CRC32C checksum (SSE4.2 on x86_64)
#[inline]
fn hardware_crc32c(data: &[u8]) -> u32 {
    #[cfg(target_arch = "x86_64")]
    {
        if is_x86_feature_detected!("sse4.2") {
            return unsafe { hardware_crc32c_impl(data) };
        }
    }
    // Fallback to software CRC32
    software_crc32c(data)
}

#[cfg(target_arch = "x86_64")]
#[target_feature(enable = "sse4.2")]
unsafe fn hardware_crc32c_impl(data: &[u8]) -> u32 {
    let mut crc: u32 = 0xFFFFFFFF;
    let mut ptr = data.as_ptr();
    let mut remaining = data.len();

    // Process 8 bytes at a time for maximum throughput
    while remaining >= 8 {
        let value = (ptr as *const u64).read_unaligned();
        crc = _mm_crc32_u64(crc as u64, value) as u32;
        ptr = ptr.add(8);
        remaining -= 8;
    }

    // Process remaining bytes
    while remaining > 0 {
        let value = *ptr;
        crc = _mm_crc32_u8(crc, value);
        ptr = ptr.add(1);
        remaining -= 1;
    }

    !crc
}

/// Software fallback CRC32C
fn software_crc32c(data: &[u8]) -> u32 {
    const CRC32C_TABLE: [u32; 256] = generate_crc32c_table();
    let mut crc: u32 = 0xFFFFFFFF;
    for &byte in data {
        let index = ((crc ^ byte as u32) & 0xFF) as usize;
        crc = (crc >> 8) ^ CRC32C_TABLE[index];
    }
    !crc
}

const fn generate_crc32c_table() -> [u32; 256] {
    let mut table = [0u32; 256];
    let poly: u32 = 0x82F63B78; // CRC32C polynomial
    let mut i = 0;
    while i < 256 {
        let mut crc = i as u32;
        let mut j = 0;
        while j < 8 {
            if crc & 1 != 0 {
                crc = (crc >> 1) ^ poly;
            } else {
                crc >>= 1;
            }
            j += 1;
        }
        table[i] = crc;
        i += 1;
    }
    table
}

/// ARIES-style log record types
#[repr(C)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LogRecord {
    /// Begin transaction
    Begin {
        txn_id: TransactionId,
        timestamp: SystemTime,
    },
    /// Update operation (physiological - page-oriented with logical redo)
    Update {
        txn_id: TransactionId,
        page_id: PageId,
        offset: u32,
        before_image: Vec<u8>,
        after_image: Vec<u8>,
        undo_next_lsn: Option<LSN>,
    },
    /// Insert operation
    Insert {
        txn_id: TransactionId,
        page_id: PageId,
        offset: u32,
        data: Vec<u8>,
        undo_next_lsn: Option<LSN>,
    },
    /// Delete operation
    Delete {
        txn_id: TransactionId,
        page_id: PageId,
        offset: u32,
        deleted_data: Vec<u8>,
        undo_next_lsn: Option<LSN>,
    },
    /// Commit transaction
    Commit {
        txn_id: TransactionId,
        timestamp: SystemTime,
    },
    /// Abort transaction
    Abort {
        txn_id: TransactionId,
        timestamp: SystemTime,
    },
    /// Compensation Log Record (CLR) for undo operations
    CLR {
        txn_id: TransactionId,
        page_id: PageId,
        undo_next_lsn: Option<LSN>,
        redo_operation: Box<LogRecord>,
    },
    /// Checkpoint begin
    CheckpointBegin {
        timestamp: SystemTime,
    },
    /// Checkpoint end
    CheckpointEnd {
        active_txns: Vec<TransactionId>,
        dirty_pages: Vec<PageId>,
        timestamp: SystemTime,
    },
    /// End of log marker
    EndOfLog,
}

impl LogRecord {
    /// Get transaction ID if applicable
    #[inline]
    pub fn txn_id(&self) -> Option<TransactionId> {
        match self {
            LogRecord::Begin { txn_id, .. } |
            LogRecord::Update { txn_id, .. } |
            LogRecord::Insert { txn_id, .. } |
            LogRecord::Delete { txn_id, .. } |
            LogRecord::Commit { txn_id, .. } |
            LogRecord::Abort { txn_id, .. } |
            LogRecord::CLR { txn_id, .. } => Some(*txn_id),
            _ => None,
        }
    }

    /// Get page ID if applicable
    #[inline]
    pub fn page_id(&self) -> Option<PageId> {
        match self {
            LogRecord::Update { page_id, .. } |
            LogRecord::Insert { page_id, .. } |
            LogRecord::Delete { page_id, .. } |
            LogRecord::CLR { page_id, .. } => Some(*page_id),
            _ => None,
        }
    }

    /// Check if this is a redo-only record
    pub fn is_redo_only(&self) -> bool {
        matches!(self, LogRecord::CLR { .. })
    }
}

/// WAL entry with metadata
#[repr(C)]
#[repr(align(64))] // Cache-line aligned for performance
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WALEntry {
    pub lsn: LSN,
    pub prev_lsn: Option<LSN>,
    pub record: LogRecord,
    pub size: u32,
    pub checksum: u32,
}

impl WALEntry {
    fn new(lsn: LSN, prev_lsn: Option<LSN>, record: LogRecord) -> Self {
        let serialized = bincode::serialize(&record).unwrap_or_default();
        let size = serialized.len() as u32;
        let checksum = Self::calculate_checksum(&serialized);

        Self {
            lsn,
            prev_lsn,
            record,
            size,
            checksum,
        }
    }

    fn calculate_checksum(data: &[u8]) -> u32 {
        // Hardware-accelerated CRC32C checksum
        hardware_crc32c(data)
    }

    fn calculate_checksum_batch(entries: &[&[u8]]) -> Vec<u32> {
        // Batch checksum computation for better cache utilization
        entries.iter().map(|data| hardware_crc32c(data)).collect()
    }

    fn verify_checksum(&self) -> bool {
        let serialized = bincode::serialize(&self.record).unwrap_or_default();
        self.checksum == Self::calculate_checksum(&serialized)
    }
}

/// Group commit buffer for batching log writes
struct GroupCommitBuffer {
    entries: Vec<WALEntry>,
    waiters: Vec<tokio::sync::oneshot::Sender<Result<LSN>>>,
    size_bytes: usize,
    oldest_entry_time: Option<Instant>,
}

impl GroupCommitBuffer {
    fn new() -> Self {
        Self {
            entries: Vec::new(),
            waiters: Vec::new(),
            size_bytes: 0,
            oldest_entry_time: None,
        }
    }

    fn add(&mut self, entry: WALEntry, waiter: tokio::sync::oneshot::Sender<Result<LSN>>) {
        self.size_bytes += entry.size as usize;
        if self.oldest_entry_time.is_none() {
            self.oldest_entry_time = Some(Instant::now());
        }
        self.entries.push(entry);
        self.waiters.push(waiter);
    }

    fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    fn should_flush(&self, max_size: usize, max_delay: Duration) -> bool {
        if self.is_empty() {
            return false;
        }

        // Flush if buffer is full
        if self.size_bytes >= max_size {
            return true;
        }

        // Flush if oldest entry exceeds max delay
        if let Some(oldest) = self.oldest_entry_time {
            if oldest.elapsed() >= max_delay {
                return true;
            }
        }

        false
    }

    fn take(&mut self) -> (Vec<WALEntry>, Vec<tokio::sync::oneshot::Sender<Result<LSN>>>) {
        self.oldest_entry_time = None;
        self.size_bytes = 0;
        (
            std::mem::take(&mut self.entries),
            std::mem::take(&mut self.waiters),
        )
    }
}

/// Write-Ahead Log Manager with Group Commit
pub struct WALManager {
    /// Path to WAL file
    wal_path: PathBuf,
    /// Current WAL file
    wal_file: Arc<Mutex<BufWriter<File>>>,
    /// Next LSN to allocate
    next_lsn: Arc<AtomicU64>,
    /// Last flushed LSN
    flushed_lsn: Arc<AtomicU64>,
    /// Group commit buffer
    commit_buffer: Arc<Mutex<GroupCommitBuffer>>,
    /// Transaction table (for recovery)
    transaction_table: Arc<RwLock<HashMap<TransactionId, TransactionTableEntry>>>,
    /// Dirty page table (for recovery)
    dirty_page_table: Arc<RwLock<HashMap<PageId, LSN>>>,
    /// Configuration
    config: WALConfig,
    /// Statistics
    stats: Arc<RwLock<WALStats>>,
    /// Shutdown flag
    shutdown: Arc<AtomicBool>,
}

#[derive(Debug, Clone)]
pub struct WALConfig {
    /// Maximum group commit buffer size in bytes
    pub max_buffer_size: usize,
    /// Maximum delay before forcing flush (milliseconds)
    pub max_commit_delay_ms: u64,
    /// Enable group commit optimization
    pub enable_group_commit: bool,
    /// WAL segment size (bytes)
    pub segment_size: usize,
    /// Enable log shipping for replication
    pub enable_log_shipping: bool,
    /// Sync mode (fsync after every write, or periodic)
    pub sync_mode: SyncMode,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SyncMode {
    /// Sync after every commit
    AlwaysSync,
    /// Sync periodically
    PeriodicSync,
    /// No sync (unsafe, for testing only)
    NoSync,
}

impl Default for WALConfig {
    fn default() -> Self {
        Self {
            max_buffer_size: 4 * 1024 * 1024, // 4 MB
            max_commit_delay_ms: 10,
            enable_group_commit: true,
            segment_size: 64 * 1024 * 1024, // 64 MB
            enable_log_shipping: false,
            sync_mode: SyncMode::AlwaysSync,
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct WALStats {
    pub total_records: u64,
    pub total_bytes: u64,
    pub group_commits: u64,
    pub individual_commits: u64,
    pub avg_group_size: f64,
    pub fsyncs: u64,
    pub avg_flush_time_ms: f64,
    pub vectored_writes: u64,
    pub hardware_crc_ops: u64,
    pub batched_checksums: u64,
}

/// Transaction table entry for recovery
#[derive(Debug, Clone)]
struct TransactionTableEntry {
    txn_id: TransactionId,
    state: TransactionState,
    last_lsn: LSN,
    undo_next_lsn: Option<LSN>,
}

/// Transaction state for recovery
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TransactionState {
    Active,
    Committed,
    Aborted,
}

impl WALManager {
    /// Create a new WAL manager
    pub fn new(wal_path: PathBuf, config: WALConfig) -> Result<Self> {
        let wal_file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&wal_path)
            .map_err(|e| DbError::IOError(format!("Failed to open WAL: {}", e)))?;

        let manager = Self {
            wal_path,
            wal_file: Arc::new(Mutex::new(BufWriter::new(wal_file))),
            next_lsn: Arc::new(AtomicU64::new(1)),
            flushed_lsn: Arc::new(AtomicU64::new(0)),
            commit_buffer: Arc::new(Mutex::new(GroupCommitBuffer::new())),
            transaction_table: Arc::new(RwLock::new(HashMap::new())),
            dirty_page_table: Arc::new(RwLock::new(HashMap::new())),
            config,
            stats: Arc::new(RwLock::new(WALStats::default())),
            shutdown: Arc::new(AtomicBool::new(false)),
        };

        Ok(manager)
    }

    /// Allocate a new LSN
    fn allocate_lsn(&self) -> LSN {
        self.next_lsn.fetch_add(1, Ordering::SeqCst)
    }

    /// Append a log record
    pub async fn append(&self, record: LogRecord) -> Result<LSN> {
        let lsn = self.allocate_lsn();

        // Get previous LSN for this transaction
        let prev_lsn = record.txn_id().and_then(|txn_id| {
            self.transaction_table
                .read()
                .get(&txn_id)
                .map(|entry| entry.last_lsn)
        });

        let entry = WALEntry::new(lsn, prev_lsn, record.clone());

        // Update transaction table
        if let Some(txn_id) = record.txn_id() {
            let state = match &record {
                LogRecord::Commit { .. } => TransactionState::Committed,
                LogRecord::Abort { .. } => TransactionState::Aborted,
                _ => TransactionState::Active,
            };

            let undo_next_lsn = match &record {
                LogRecord::Update { undo_next_lsn, .. } |
                LogRecord::Insert { undo_next_lsn, .. } |
                LogRecord::Delete { undo_next_lsn, .. } |
                LogRecord::CLR { undo_next_lsn, .. } => *undo_next_lsn,
                _ => None,
            };

            self.transaction_table.write().insert(
                txn_id,
                TransactionTableEntry {
                    txn_id,
                    state,
                    last_lsn: lsn,
                    undo_next_lsn,
                },
            );
        }

        // Update dirty page table
        if let Some(page_id) = record.page_id() {
            self.dirty_page_table
                .write()
                .entry(page_id)
                .or_insert(lsn);
        }

        // Group commit handling
        if self.config.enable_group_commit {
            let (tx, rx) = tokio::sync::oneshot::channel();
            self.commit_buffer.lock().unwrap().add(entry, tx);

            // Check if we should flush
            self.maybe_flush_buffer().await?;

            // Wait for flush
            rx.await
                .map_err(|_| DbError::TransactionError("Commit waiter dropped".to_string()))?
        } else {
            // Direct write
            self.write_entry(&entry)?;
            self.sync_if_needed(&record)?;
            Ok(lsn)
        }
    }

    /// Maybe flush the group commit buffer
    async fn maybe_flush_buffer(&self) -> Result<()> {
        let should_flush = {
            let buffer = self.commit_buffer.lock();
            buffer.should_flush(
                self.config.max_buffer_size,
                Duration::from_millis(self.config.max_commit_delay_ms),
            )
        };

        if should_flush {
            self.flush_buffer().await?;
        }

        Ok(())
    }

    /// Flush the group commit buffer with vectored I/O
    async fn flush_buffer(&self) -> Result<()> {
        let (entries, waiters) = {
            let mut buffer = self.commit_buffer.lock();
            if buffer.is_empty() {
                return Ok(());
            }
            buffer.take()
        };

        if entries.is_empty() {
            return Ok(());
        }

        let start = Instant::now();
        let last_lsn = entries.last().map(|e| e.lsn).unwrap_or(0);

        // Use vectored write for better performance (single syscall)
        self.write_entries_vectored(&entries)?;

        // Sync to disk
        self.sync()?;

        let flush_time = start.elapsed().as_millis() as f64;

        // Update statistics
        let mut stats = self.stats.write();
        stats.group_commits += 1;
        stats.avg_group_size =
            (stats.avg_group_size * (stats.group_commits - 1) as f64 + entries.len() as f64)
                / stats.group_commits as f64;
        stats.avg_flush_time_ms =
            (stats.avg_flush_time_ms * (stats.group_commits - 1) as f64 + flush_time)
                / stats.group_commits as f64;

        // Notify waiters
        for waiter in waiters {
            let _ = waiter.send(Ok(last_lsn));
        }

        Ok(())
    }

    /// Write a single entry to WAL
    fn write_entry(&self, entry: &WALEntry) -> Result<()> {
        let serialized = bincode::serialize(entry)
            .map_err(|e| DbError::SerializationError(format!("Failed to serialize WAL entry: {}", e)))?;

        let mut file = self.wal_file.lock();
        file.write_all(&serialized)
            .map_err(|e| DbError::IOError(format!("Failed to write WAL entry: {}", e)))?;

        // Update statistics
        let mut stats = self.stats.write();
        stats.total_records += 1;
        stats.total_bytes += serialized.len() as u64;
        stats.hardware_crc_ops += 1; // Checksum computed with hardware acceleration

        Ok(())
    }

    /// Write multiple entries using vectored I/O for better performance
    fn write_entries_vectored(&self, entries: &[WALEntry]) -> Result<()> {
        if entries.is_empty() {
            return Ok(());
        }

        // Serialize all entries
        let serialized: Vec<Vec<u8>> = entries.iter()
            .map(|e| bincode::serialize(e))
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| DbError::SerializationError(format!("Batch serialization failed: {}", e)))?;

        // Prepare IoSlice for writev
        let slices: Vec<IoSlice> = serialized.iter()
            .map(|buf| IoSlice::new(buf))
            .collect();

        let mut file = self.wal_file.lock();

        // Write all slices in a single syscall (vectored I/O)
        let mut total_written = 0;
        let total_size: usize = serialized.iter().map(|s| s.len()).sum();

        // Note: write_vectored might not write everything in one call
        while total_written < total_size {
            let written = file.get_mut()
                .write_vectored(&slices[total_written..])
                .map_err(|e| DbError::IOError(format!("Vectored write failed: {}", e)))?;
            total_written += written;
        }

        // Update statistics
        let mut stats = self.stats.write();
        stats.total_records += entries.len() as u64;
        stats.total_bytes += total_size as u64;
        stats.vectored_writes += 1;
        stats.hardware_crc_ops += entries.len() as u64;

        Ok(())
    }

    /// Sync WAL to disk
    fn sync(&self) -> Result<()> {
        let mut file = self.wal_file.lock();
        file.flush()
            .map_err(|e| DbError::IOError(format!("Failed to flush WAL: {}", e)))?;

        file.get_mut()
            .sync_all()
            .map_err(|e| DbError::IOError(format!("Failed to sync WAL: {}", e)))?;

        self.stats.write().fsyncs += 1;

        let next_lsn = self.next_lsn.load(Ordering::SeqCst);
        self.flushed_lsn.store(next_lsn, Ordering::SeqCst);

        Ok(())
    }

    /// Sync if needed based on config and record type
    fn sync_if_needed(&self, record: &LogRecord) -> Result<()> {
        match self.config.sync_mode {
            SyncMode::AlwaysSync => {
                if matches!(record, LogRecord::Commit { .. } | LogRecord::Abort { .. }) {
                    self.sync()?;
                }
            }
            SyncMode::NoSync => {}
            SyncMode::PeriodicSync => {
                // Periodic sync handled by background task
            }
        }
        Ok(())
    }

    /// Start background group commit flusher
    pub async fn start_background_flusher(self: Arc<Self>) {
        let mut ticker = interval(Duration::from_millis(self.config.max_commit_delay_ms));

        while !self.shutdown.load(Ordering::SeqCst) {
            ticker.tick().await;

            if let Err(e) = self.flush_buffer().await {
                eprintln!("Background flush error: {}", e);
            }
        }
    }

    /// Truncate WAL up to a given LSN (after checkpoint)
    pub fn truncate(&self, up_to_lsn: LSN) -> Result<()> {
        // In production, this would archive old log segments
        // and create a new active segment
        Ok(())
    }

    /// Get current LSN
    pub fn current_lsn(&self) -> LSN {
        self.next_lsn.load(Ordering::SeqCst)
    }

    /// Get flushed LSN
    pub fn flushed_lsn(&self) -> LSN {
        self.flushed_lsn.load(Ordering::SeqCst)
    }

    /// Read log records starting from LSN
    pub fn read_from(&self, start_lsn: LSN) -> Result<Vec<WALEntry>> {
        let file = File::open(&self.wal_path)
            .map_err(|e| DbError::IOError(format!("Failed to open WAL for reading: {}", e)))?;

        let mut reader = BufReader::new(file);
        let mut entries = Vec::new();

        loop {
            let entry: WALEntry = match bincode::deserialize_from(&mut reader) {
                Ok(entry) => entry,
                Err(_) => break, // End of file or corrupted entry
            };

            if entry.lsn >= start_lsn {
                if !entry.verify_checksum() {
                    return Err(DbError::CorruptionError(
                        format!("Checksum mismatch at LSN {}", entry.lsn)
                    ));
                }
                entries.push(entry);
            }
        }

        Ok(entries)
    }

    /// Get transaction table (for recovery)
    pub fn transaction_table(&self) -> HashMap<TransactionId, TransactionTableEntry> {
        self.transaction_table.read().clone()
    }

    /// Get dirty page table (for recovery)
    pub fn dirty_page_table(&self) -> HashMap<PageId, LSN> {
        self.dirty_page_table.read().clone()
    }

    /// Get statistics
    pub fn get_stats(&self) -> WALStats {
        self.stats.read().clone()
    }

    /// Shutdown WAL manager
    pub fn shutdown(&self) -> Result<()> {
        self.shutdown.store(true, Ordering::SeqCst);
        self.sync()?;
        Ok(())
    }
}

/// Log Shipping Manager for Replication
pub struct LogShippingManager {
    /// WAL manager
    wal: Arc<WALManager>,
    /// Last shipped LSN
    last_shipped_lsn: Arc<AtomicU64>,
    /// Standby servers
    standbys: Arc<RwLock<Vec<StandbyServer>>>,
    /// Configuration
    config: LogShippingConfig,
    /// Statistics
    stats: Arc<RwLock<LogShippingStats>>,
}

#[derive(Debug, Clone)]
pub struct LogShippingConfig {
    /// Shipping interval (milliseconds)
    pub shipping_interval_ms: u64,
    /// Maximum batch size
    pub max_batch_size: usize,
    /// Enable compression
    pub enable_compression: bool,
}

impl Default for LogShippingConfig {
    fn default() -> Self {
        Self {
            shipping_interval_ms: 100,
            max_batch_size: 1000,
            enable_compression: true,
        }
    }
}

#[derive(Debug, Clone)]
pub struct StandbyServer {
    pub node_id: u32,
    pub address: String,
    pub last_applied_lsn: Arc<AtomicU64>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct LogShippingStats {
    pub total_shipped: u64,
    pub total_bytes_shipped: u64,
    pub avg_batch_size: f64,
    pub avg_shipping_latency_ms: f64,
}

impl LogShippingManager {
    pub fn new(wal: Arc<WALManager>, config: LogShippingConfig) -> Self {
        Self {
            wal,
            last_shipped_lsn: Arc::new(AtomicU64::new(0)),
            standbys: Arc::new(RwLock::new(Vec::new())),
            config,
            stats: Arc::new(RwLock::new(LogShippingStats::default())),
        }
    }

    /// Add a standby server
    pub fn add_standby(&self, standby: StandbyServer) {
        self.standbys.write().push(standby);
    }

    /// Ship logs to standby servers
    pub async fn ship_logs(&self) -> Result<()> {
        let last_shipped = self.last_shipped_lsn.load(Ordering::SeqCst);
        let current_lsn = self.wal.current_lsn();

        if current_lsn <= last_shipped {
            return Ok(()); // Nothing to ship
        }

        let start = Instant::now();

        // Read log entries
        let entries = self.wal.read_from(last_shipped + 1)?;

        if entries.is_empty() {
            return Ok(());
        }

        // Ship to all standbys
        let standbys = self.standbys.read().clone();
        for standby in standbys {
            self.ship_to_standby(&standby, &entries).await?;
        }

        // Update last shipped LSN
        if let Some(last_entry) = entries.last() {
            self.last_shipped_lsn.store(last_entry.lsn, Ordering::SeqCst);
        }

        // Update statistics
        let shipping_time = start.elapsed().as_millis() as f64;
        let mut stats = self.stats.write();
        stats.total_shipped += entries.len() as u64;

        let serialized_size: usize = entries.iter()
            .map(|e| bincode::serialize(e).unwrap_or_default().len())
            .sum();
        stats.total_bytes_shipped += serialized_size as u64;

        stats.avg_batch_size =
            (stats.avg_batch_size * (stats.total_shipped - entries.len() as u64) as f64
                + entries.len() as f64)
                / stats.total_shipped as f64;

        stats.avg_shipping_latency_ms =
            (stats.avg_shipping_latency_ms * (stats.total_shipped - entries.len() as u64) as f64
                + shipping_time)
                / stats.total_shipped as f64;

        Ok(())
    }

    async fn ship_to_standby(&self, standby: &StandbyServer, entries: &[WALEntry]) -> Result<()> {
        // In production, this would send over network
        // For now, just update the last applied LSN
        if let Some(last_entry) = entries.last() {
            standby.last_applied_lsn.store(last_entry.lsn, Ordering::SeqCst);
        }
        Ok(())
    }

    pub fn get_stats(&self) -> LogShippingStats {
        self.stats.read().clone()
    }
}

/// Checkpoint Coordinator
pub struct CheckpointCoordinator {
    wal: Arc<WALManager>,
    last_checkpoint_lsn: Arc<AtomicU64>,
    config: CheckpointConfig,
    stats: Arc<RwLock<CheckpointStats>>,
}

#[derive(Debug, Clone)]
pub struct CheckpointConfig {
    /// Checkpoint interval (seconds)
    pub interval_secs: u64,
    /// Fuzzy checkpoint enabled
    pub fuzzy: bool,
    /// Maximum dirty pages before forcing checkpoint
    pub max_dirty_pages: usize,
}

impl Default for CheckpointConfig {
    fn default() -> Self {
        Self {
            interval_secs: 300, // 5 minutes
            fuzzy: true,
            max_dirty_pages: 10000,
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CheckpointStats {
    pub total_checkpoints: u64,
    pub avg_checkpoint_time_ms: f64,
    pub last_checkpoint_lsn: u64,
}

impl CheckpointCoordinator {
    pub fn new(wal: Arc<WALManager>, config: CheckpointConfig) -> Self {
        Self {
            wal,
            last_checkpoint_lsn: Arc::new(AtomicU64::new(0)),
            config,
            stats: Arc::new(RwLock::new(CheckpointStats::default())),
        }
    }

    /// Perform a checkpoint
    pub async fn checkpoint(&self) -> Result<LSN> {
        let start = Instant::now();

        // Write checkpoint begin record
        let begin_lsn = self.wal.append(LogRecord::CheckpointBegin {
            timestamp: SystemTime::now(),
        }).await?;

        // Get transaction and dirty page tables
        let txn_table = self.wal.transaction_table();
        let dirty_pages = self.wal.dirty_page_table();

        let active_txns: Vec<TransactionId> = txn_table
            .iter()
            .filter(|(_, entry)| entry.state == TransactionState::Active)
            .map(|(&txn_id, _)| txn_id)
            .collect();

        let dirty_page_ids: Vec<PageId> = dirty_pages.keys().copied().collect();

        // Write checkpoint end record
        let end_lsn = self.wal.append(LogRecord::CheckpointEnd {
            active_txns,
            dirty_pages: dirty_page_ids,
            timestamp: SystemTime::now(),
        }).await?;

        self.last_checkpoint_lsn.store(end_lsn, Ordering::SeqCst);

        // Truncate old log entries
        self.wal.truncate(begin_lsn)?;

        // Update statistics
        let checkpoint_time = start.elapsed().as_millis() as f64;
        let mut stats = self.stats.write();
        stats.total_checkpoints += 1;
        stats.last_checkpoint_lsn = end_lsn;
        stats.avg_checkpoint_time_ms =
            (stats.avg_checkpoint_time_ms * (stats.total_checkpoints - 1) as f64 + checkpoint_time)
                / stats.total_checkpoints as f64;

        Ok(end_lsn)
    }

    /// Start periodic checkpoint background task
    pub async fn start_periodic_checkpoint(self: Arc<Self>) {
        let mut ticker = interval(Duration::from_secs(self.config.interval_secs));

        loop {
            ticker.tick().await;

            if let Err(e) = self.checkpoint().await {
                eprintln!("Checkpoint error: {}", e);
            }
        }
    }

    pub fn get_stats(&self) -> CheckpointStats {
        self.stats.read().clone()
    }
}

#[cfg(test)]
mod tests {
    use tempfile::tempdir;

    #[tokio::test]
    async fn test_wal_append_and_read() {
        let dir = tempdir().unwrap();
        let wal_path = dir.path().join("test.wal");

        let config = WALConfig {
            enable_group_commit: false,
            ..Default::default()
        };

        let wal = WALManager::new(wal_path.clone(), config).unwrap();

        // Write some records
        let lsn1 = wal.append(LogRecord::Begin {
            txn_id: 1,
            timestamp: SystemTime::now(),
        }).await.unwrap();

        let lsn2 = wal.append(LogRecord::Update {
            txn_id: 1,
            page_id: 100,
            offset: 0,
            before_image: vec![1, 2, 3],
            after_image: vec![4, 5, 6],
            undo_next_lsn: None,
        }).await.unwrap();

        wal.shutdown().unwrap();

        // Read back
        let wal2 = WALManager::new(wal_path, WALConfig::default()).unwrap();
        let entries = wal2.read_from(1).unwrap();

        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].lsn, lsn1);
        assert_eq!(entries[1].lsn, lsn2);
    }

    #[test]
    fn test_group_commit_buffer() {
        let mut buffer = GroupCommitBuffer::new();
        assert!(buffer.is_empty());

        let entry = WALEntry::new(1, None, LogRecord::EndOfLog);
        let (tx, _rx) = tokio::sync::oneshot::channel();
        buffer.add(entry, tx);

        assert!(!buffer.is_empty());
        assert!(buffer.should_flush(100, Duration::from_secs(1)));
    }
}


