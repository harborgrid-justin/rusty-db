// # Change Data Capture (CDC) Engine
//
// Provides enterprise-grade change data capture from the Write-Ahead Log.
// Captures INSERT, UPDATE, and DELETE operations with before/after images,
// column-level tracking, and low-latency event delivery.

use std::collections::VecDeque;
use std::sync::Mutex;
use std::time::Duration;
use std::collections::{HashMap};
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, AtomicBool, Ordering};
use std::time::{SystemTime};
use parking_lot::{RwLock};
use serde::{Deserialize, Serialize};
use tokio::sync::{mpsc, broadcast};
use tokio::time::interval;
use crate::{Result, DbError};
use crate::common::{TransactionId, TableId, RowId, Value, LogSequenceNumber};
use crate::transaction::wal::{LogRecord, WALEntry, LSN};

/// Change event type
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ChangeType {
    Insert,
    Update,
    Delete,
    Truncate,
}

/// Column-level change information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ColumnChange {
    /// Column name
    pub column_name: String,
    /// Column index in schema
    pub column_index: usize,
    /// Old value (for UPDATE and DELETE)
    pub old_value: Option<Value>,
    /// New value (for INSERT and UPDATE)
    pub new_value: Option<Value>,
    /// Whether this column was modified
    pub modified: bool,
}

impl ColumnChange {
    pub fn new(column_name: String, column_index: usize) -> Self {
        Self {
            column_name,
            column_index,
            old_value: None,
            new_value: None,
            modified: false,
        }
    }

    pub fn with_old_value(mut self, value: Value) -> Self {
        self.old_value = Some(value);
        self
    }

    pub fn with_new_value(mut self, value: Value) -> Self {
        self.new_value = Some(value);
        self.modified = true;
        self
    }

    pub fn is_modified(&self) -> bool {
        self.modified
    }
}

/// Change data capture event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChangeEvent {
    /// Unique event identifier
    pub event_id: u64,
    /// Log Sequence Number from WAL
    pub lsn: LogSequenceNumber,
    /// Transaction ID that made this change
    pub txn_id: TransactionId,
    /// Table that was modified
    pub table_id: TableId,
    /// Table name
    pub table_name: String,
    /// Type of change
    pub change_type: ChangeType,
    /// Row identifier
    pub row_id: RowId,
    /// Before image (full row before change)
    pub before_image: Option<HashMap<String, Value>>,
    /// After image (full row after change)
    pub after_image: Option<HashMap<String, Value>>,
    /// Column-level changes
    pub column_changes: Vec<ColumnChange>,
    /// Timestamp when change occurred
    pub timestamp: SystemTime,
    /// Commit timestamp
    pub commit_timestamp: Option<SystemTime>,
    /// Additional metadata
    pub metadata: HashMap<String, String>,
}

impl ChangeEvent {
    pub fn new(
        event_id: u64,
        lsn: LogSequenceNumber,
        txn_id: TransactionId,
        table_id: TableId,
        table_name: String,
        change_type: ChangeType,
    ) -> Self {
        Self {
            event_id,
            lsn,
            txn_id,
            table_id,
            table_name,
            change_type,
            row_id: 0,
            before_image: None,
            after_image: None,
            column_changes: Vec::new(),
            timestamp: SystemTime::now(),
            commit_timestamp: None,
            metadata: HashMap::new(),
        }
    }

    /// Check if a specific column was modified
    pub fn column_modified(&self, column_name: &str) -> bool {
        self.column_changes
            .iter()
            .any(|c| c.column_name == column_name && c.modified)
    }

    /// Get list of modified column names
    pub fn modified_columns(&self) -> Vec<&str> {
        self.column_changes
            .iter()
            .filter(|c| c.modified)
            .map(|c| c.column_name.as_str())
            .collect()
    }

    /// Get change for a specific column
    pub fn get_column_change(&self, column_name: &str) -> Option<&ColumnChange> {
        self.column_changes.iter().find(|c| c.column_name == column_name)
    }
}

/// Change event batch for efficient processing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChangeEventBatch {
    /// Batch identifier
    pub batch_id: u64,
    /// Events in this batch
    pub events: Vec<ChangeEvent>,
    /// First LSN in batch
    pub start_lsn: LogSequenceNumber,
    /// Last LSN in batch
    pub end_lsn: LogSequenceNumber,
    /// Batch creation time
    pub created_at: SystemTime,
    /// Number of transactions in batch
    pub txn_count: usize,
}

impl ChangeEventBatch {
    pub fn new(batch_id: u64) -> Self {
        Self {
            batch_id,
            events: Vec::new(),
            start_lsn: 0,
            end_lsn: 0,
            created_at: SystemTime::now(),
            txn_count: 0,
        }
    }

    pub fn add_event(&mut self, event: ChangeEvent) {
        if self.events.is_empty() {
            self.start_lsn = event.lsn;
        }
        self.end_lsn = event.lsn;
        self.events.push(event);
    }

    pub fn is_empty(&self) -> bool {
        self.events.is_empty()
    }

    pub fn len(&self) -> usize {
        self.events.len()
    }

    pub fn total_size_bytes(&self) -> usize {
        // Estimate size
        self.events.len() * 1024 // rough estimate
    }
}

/// CDC capture filter for selective change capture
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CaptureFilter {
    /// Tables to capture changes from
    pub included_tables: Option<Vec<TableId>>,
    /// Tables to exclude from capture
    pub excluded_tables: Vec<TableId>,
    /// Capture only specific change types
    pub change_types: Vec<ChangeType>,
    /// Minimum transaction ID to capture
    pub min_txn_id: Option<TransactionId>,
    /// Capture column-level changes
    pub capture_column_changes: bool,
    /// Capture before images
    pub capture_before_image: bool,
    /// Capture after images
    pub capture_after_image: bool,
}

impl Default for CaptureFilter {
    fn default() -> Self {
        Self {
            included_tables: None, // All tables
            excluded_tables: Vec::new(),
            change_types: vec![
                ChangeType::Insert,
                ChangeType::Update,
                ChangeType::Delete,
            ],
            min_txn_id: None,
            capture_column_changes: true,
            capture_before_image: true,
            capture_after_image: true,
        }
    }
}

impl CaptureFilter {
    pub fn should_capture(&self, table_id: TableId, change_type: &ChangeType) -> bool {
        // Check excluded tables
        if self.excluded_tables.contains(&table_id) {
            return false;
        }

        // Check included tables
        if let Some(ref included) = self.included_tables {
            if !included.contains(&table_id) {
                return false;
            }
        }

        // Check change type
        if !self.change_types.contains(change_type) {
            return false;
        }

        true
    }
}

/// CDC capture process state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CaptureState {
    Stopped,
    Starting,
    Running,
    Paused,
    Stopping,
    Failed,
}

/// CDC capture statistics
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CaptureStatistics {
    /// Total events captured
    pub total_events: u64,
    /// Events captured per change type
    pub events_by_type: HashMap<String, u64>,
    /// Total batches created
    pub total_batches: u64,
    /// Average batch size
    pub avg_batch_size: f64,
    /// Total bytes processed
    pub total_bytes: u64,
    /// Capture latency (time from WAL write to capture)
    pub avg_capture_latency_ms: f64,
    pub p50_capture_latency_ms: f64,
    pub p95_capture_latency_ms: f64,
    pub p99_capture_latency_ms: f64,
    /// Processing rate (events/second)
    pub events_per_second: f64,
    /// Last processed LSN
    pub last_lsn: LogSequenceNumber,
    /// Lag behind current WAL (in LSNs)
    pub lag_lsn: u64,
    /// Number of filtered events
    pub filtered_events: u64,
    /// Number of errors
    pub error_count: u64,
}

impl CaptureStatistics {
    pub fn record_event(&mut self, change_type: &ChangeType, latency_ms: f64) {
        self.total_events += 1;
        let key = format!("{:?}", change_type);
        *self.events_by_type.entry(key).or_insert(0) += 1;

        // Simple moving average for latency
        self.avg_capture_latency_ms =
            (self.avg_capture_latency_ms * 0.9) + (latency_ms * 0.1);
    }

    pub fn record_batch(&mut self, batch_size: usize) {
        self.total_batches += 1;
        self.avg_batch_size =
            ((self.avg_batch_size * (self.total_batches - 1) as f64) + batch_size as f64)
            / self.total_batches as f64;
    }
}

/// CDC Engine Configuration
#[derive(Debug, Clone)]
pub struct CDCConfig {
    /// Batch size for change events
    pub batch_size: usize,
    /// Maximum batch wait time
    pub batch_timeout: Duration,
    /// Buffer size for pending events
    pub event_buffer_size: usize,
    /// Enable compression for event storage
    pub enable_compression: bool,
    /// Capture filter
    pub filter: CaptureFilter,
    /// Enable metrics collection
    pub enable_metrics: bool,
    /// Checkpoint interval (persist capture position)
    pub checkpoint_interval: Duration,
}

impl Default for CDCConfig {
    fn default() -> Self {
        Self {
            batch_size: 1000,
            batch_timeout: Duration::from_millis(100),
            event_buffer_size: 10000,
            enable_compression: true,
            filter: CaptureFilter::default(),
            enable_metrics: true,
            checkpoint_interval: Duration::from_secs(10),
        }
    }
}

/// Capture position checkpoint for recovery
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CaptureCheckpoint {
    /// Last processed LSN
    pub lsn: LogSequenceNumber,
    /// Last event ID
    pub event_id: u64,
    /// Checkpoint timestamp
    pub timestamp: SystemTime,
    /// Transaction table state
    pub active_transactions: Vec<TransactionId>,
}

/// CDC Engine for capturing database changes
pub struct CDCEngine {
    /// Engine configuration
    config: CDCConfig,
    /// Current capture state
    state: Arc<RwLock<CaptureState>>,
    /// Next event ID
    next_event_id: Arc<AtomicU64>,
    /// Next batch ID
    next_batch_id: Arc<AtomicU64>,
    /// Current LSN position
    current_lsn: Arc<AtomicU64>,
    /// Pending change events
    event_buffer: Arc<Mutex<VecDeque<ChangeEvent>>>,
    /// Current batch being built
    current_batch: Arc<Mutex<ChangeEventBatch>>,
    /// Event broadcast channel
    event_tx: broadcast::Sender<ChangeEvent>,
    /// Batch broadcast channel
    batch_tx: broadcast::Sender<ChangeEventBatch>,
    /// Statistics
    stats: Arc<RwLock<CaptureStatistics>>,
    /// Table metadata cache
    table_metadata: Arc<RwLock<HashMap<TableId, TableMetadata>>>,
    /// Active transactions being tracked
    active_txns: Arc<RwLock<HashMap<TransactionId, TransactionContext>>>,
    /// Last checkpoint
    last_checkpoint: Arc<RwLock<Option<CaptureCheckpoint>>>,
    /// Shutdown flag
    shutdown: Arc<AtomicBool>,
}

/// Table metadata for CDC
#[derive(Debug, Clone)]
struct TableMetadata {
    table_id: TableId,
    table_name: String,
    column_names: Vec<String>,
    column_types: Vec<String>,
    primary_key_columns: Vec<usize>,
}

/// Transaction context for CDC
#[derive(Debug, Clone)]
struct TransactionContext {
    txn_id: TransactionId,
    start_lsn: LogSequenceNumber,
    start_time: SystemTime,
    events: Vec<ChangeEvent>,
}

impl CDCEngine {
    /// Create a new CDC engine
    pub fn new(config: CDCConfig) -> Self {
        let (event_tx, _) = broadcast::channel(config.event_buffer_size);
        let (batch_tx, _) = broadcast::channel(100);

        Self {
            config,
            state: Arc::new(RwLock::new(CaptureState::Stopped)),
            next_event_id: Arc::new(AtomicU64::new(1)),
            next_batch_id: Arc::new(AtomicU64::new(1)),
            current_lsn: Arc::new(AtomicU64::new(0)),
            event_buffer: Arc::new(Mutex::new(VecDeque::new())),
            current_batch: Arc::new(Mutex::new(ChangeEventBatch::new(1))),
            event_tx,
            batch_tx,
            stats: Arc::new(RwLock::new(CaptureStatistics::default())),
            table_metadata: Arc::new(RwLock::new(HashMap::new())),
            active_txns: Arc::new(RwLock::new(HashMap::new())),
            last_checkpoint: Arc::new(RwLock::new(None)),
            shutdown: Arc::new(AtomicBool::new(false)),
        }
    }

    /// Start the CDC capture process
    pub async fn start(&self) -> Result<()> {
        let mut state = self.state.write();
        if *state != CaptureState::Stopped {
            return Err(DbError::InvalidOperation(
                "CDC engine is already running".to_string()
            ));
        }
        *state = CaptureState::Starting;
        drop(state);

        // Start background tasks
        self.spawn_batch_processor();
        self.spawn_checkpoint_task();
        self.spawn_metrics_collector();

        *self.state.write() = CaptureState::Running;
        Ok(())
    }

    /// Stop the CDC capture process
    pub async fn stop(&self) -> Result<()> {
        *self.state.write() = CaptureState::Stopping;
        self.shutdown.store(true, Ordering::SeqCst);

        // Wait for pending events to be processed
        tokio::time::sleep(Duration::from_millis(100)).await;

        *self.state.write() = CaptureState::Stopped;
        Ok(())
    }

    /// Pause the CDC capture
    pub fn pause(&self) -> Result<()> {
        *self.state.write() = CaptureState::Paused;
        Ok(())
    }

    /// Resume the CDC capture
    pub fn resume(&self) -> Result<()> {
        *self.state.write() = CaptureState::Running;
        Ok(())
    }

    /// Process a WAL entry and generate change events
    pub async fn process_wal_entry(&self, entry: &WALEntry) -> Result<Vec<ChangeEvent>> {
        let _state = *self.state.read();
        if state != CaptureState::Running {
            return Ok(Vec::new());
        }

        let mut events = Vec::new();

        match &entry.record {
            LogRecord::Insert { txn_id, page_id, data, .. } => {
                if let Some(event) = self.process_insert(*txn_id, entry.lsn, *page_id, data).await? {
                    events.push(event);
                }
            }
            LogRecord::Update { txn_id, page_id, before_image, after_image, .. } => {
                if let Some(event) = self.process_update(
                    *txn_id,
                    entry.lsn,
                    *page_id,
                    before_image,
                    after_image
                ).await? {
                    events.push(event);
                }
            }
            LogRecord::Delete { txn_id, page_id, deleted_data, .. } => {
                if let Some(event) = self.process_delete(
                    *txn_id,
                    entry.lsn,
                    *page_id,
                    deleted_data
                ).await? {
                    events.push(event);
                }
            }
            LogRecord::Commit { txn_id, timestamp } => {
                self.process_commit(*txn_id, *timestamp).await?;
            }
            LogRecord::Abort { txn_id, .. } => {
                self.process_abort(*txn_id).await?;
            }
            _ => {}
        }

        // Add events to buffer
        for event in &events {
            self.add_event(event.clone()).await?;
        }

        Ok(events)
    }

    /// Process an INSERT operation
    async fn process_insert(
        &self,
        txn_id: TransactionId,
        lsn: LogSequenceNumber,
        page_id: u64,
        data: &[u8],
    ) -> Result<Option<ChangeEvent>> {
        // Extract table ID from page ID (simplified)
        let table_id = self.extract_table_id(page_id);

        // Check filter
        if !self.config.filter.should_capture(table_id, &ChangeType::Insert) {
            self.stats.write().filtered_events += 1;
            return Ok(None);
        }

        let event_id = self.next_event_id.fetch_add(1, Ordering::SeqCst);
        let table_metadata = self.get_table_metadata(table_id).await?;

        let mut event = ChangeEvent::new(
            event_id,
            lsn,
            txn_id,
            table_id,
            table_metadata.table_name.clone(),
            ChangeType::Insert,
        );

        // Parse after image from data
        if self.config.filter.capture_after_image {
            event.after_image = Some(self.parse_row_data(data, &table_metadata)?);
        }

        // Generate column changes
        if self.config.filter.capture_column_changes {
            event.column_changes = self.generate_column_changes(
                None,
                event.after_image.as_ref(),
                &table_metadata,
            );
        }

        Ok(Some(event))
    }

    /// Process an UPDATE operation
    async fn process_update(
        &self,
        txn_id: TransactionId,
        lsn: LogSequenceNumber,
        page_id: u64,
        before_image: &[u8],
        after_image: &[u8],
    ) -> Result<Option<ChangeEvent>> {
        let table_id = self.extract_table_id(page_id);

        if !self.config.filter.should_capture(table_id, &ChangeType::Update) {
            self.stats.write().filtered_events += 1;
            return Ok(None);
        }

        let event_id = self.next_event_id.fetch_add(1, Ordering::SeqCst);
        let table_metadata = self.get_table_metadata(table_id).await?;

        let mut event = ChangeEvent::new(
            event_id,
            lsn,
            txn_id,
            table_id,
            table_metadata.table_name.clone(),
            ChangeType::Update,
        );

        // Parse before and after images
        let before = if self.config.filter.capture_before_image {
            Some(self.parse_row_data(before_image, &table_metadata)?)
        } else {
            None
        };

        let after = if self.config.filter.capture_after_image {
            Some(self.parse_row_data(after_image, &table_metadata)?)
        } else {
            None
        };

        event.before_image = before.clone();
        event.after_image = after.clone();

        // Generate column-level changes
        if self.config.filter.capture_column_changes {
            event.column_changes = self.generate_column_changes(
                before.as_ref(),
                after.as_ref(),
                &table_metadata,
            );
        }

        Ok(Some(event))
    }

    /// Process a DELETE operation
    async fn process_delete(
        &self,
        txn_id: TransactionId,
        lsn: LogSequenceNumber,
        page_id: u64,
        deleted_data: &[u8],
    ) -> Result<Option<ChangeEvent>> {
        let table_id = self.extract_table_id(page_id);

        if !self.config.filter.should_capture(table_id, &ChangeType::Delete) {
            self.stats.write().filtered_events += 1;
            return Ok(None);
        }

        let event_id = self.next_event_id.fetch_add(1, Ordering::SeqCst);
        let table_metadata = self.get_table_metadata(table_id).await?;

        let mut event = ChangeEvent::new(
            event_id,
            lsn,
            txn_id,
            table_id,
            table_metadata.table_name.clone(),
            ChangeType::Delete,
        );

        // Parse before image
        if self.config.filter.capture_before_image {
            event.before_image = Some(self.parse_row_data(deleted_data, &table_metadata)?);
        }

        // Generate column changes
        if self.config.filter.capture_column_changes {
            event.column_changes = self.generate_column_changes(
                event.before_image.as_ref(),
                None,
                &table_metadata,
            );
        }

        Ok(Some(event))
    }

    /// Process transaction commit
    async fn process_commit(&self, txn_id: TransactionId, timestamp: SystemTime) -> Result<()> {
        let mut active_txns = self.active_txns.write();
        if let Some(mut txn_ctx) = active_txns.remove(&txn_id) {
            // Update commit timestamp for all events in this transaction
            for event in &mut txn_ctx.events {
                event.commit_timestamp = Some(timestamp);
            }
        }
        Ok(())
    }

    /// Process transaction abort
    async fn process_abort(&self, txn_id: TransactionId) -> Result<()> {
        let mut active_txns = self.active_txns.write();
        active_txns.remove(&txn_id);
        Ok(())
    }

    /// Add event to buffer
    async fn add_event(&self, event: ChangeEvent) -> Result<()> {
        let start_time = Instant::now();

        // Update statistics
        {
            let mut stats = self.stats.write();
            let latency_ms = start_time.elapsed().as_millis() as f64;
            stats.record_event(&event.change_type, latency_ms);
            stats.last_lsn = event.lsn;
        }

        // Broadcast event
        let _ = self.event_tx.send(event.clone());

        // Add to batch
        let mut batch = self.current_batch.lock();
        batch.add_event(event);

        // Check if batch should be flushed
        if batch.len() >= self.config.batch_size {
            let completed_batch = std::mem::replace(
                &mut *batch,
                ChangeEventBatch::new(self.next_batch_id.fetch_add(1, Ordering::SeqCst)),
            );
            drop(batch);

            self.flush_batch(completed_batch).await?;
        }

        Ok(())
    }

    /// Flush a completed batch
    async fn flush_batch(&self, batch: ChangeEventBatch) -> Result<()> {
        if batch.is_empty() {
            return Ok(());
        }

        self.stats.write().record_batch(batch.len());

        // Broadcast batch
        let _ = self.batch_tx.send(batch);

        Ok(())
    }

    /// Generate column-level changes
    fn generate_column_changes(
        &self,
        before: Option<&HashMap<String, Value>>,
        after: Option<&HashMap<String, Value>>,
        metadata: &TableMetadata,
    ) -> Vec<ColumnChange> {
        let mut changes = Vec::new();

        for (idx, col_name) in metadata.column_names.iter().enumerate() {
            let old_val = before.and_then(|b| b.get(col_name).cloned());
            let new_val = after.and_then(|a| a.get(col_name).cloned());

            let modified = old_val != new_val;

            let mut change = ColumnChange::new(col_name.clone(), idx);
            if let Some(old) = old_val {
                change = change.with_old_value(old);
            }
            if let Some(new) = new_val {
                change = change.with_new_value(new);
            }
            change.modified = modified;

            changes.push(change);
        }

        changes
    }

    /// Subscribe to change events
    pub fn subscribe_events(&self) -> broadcast::Receiver<ChangeEvent> {
        self.event_tx.subscribe()
    }

    /// Subscribe to batches
    pub fn subscribe_batches(&self) -> broadcast::Receiver<ChangeEventBatch> {
        self.batch_tx.subscribe()
    }

    /// Get current statistics
    pub fn get_statistics(&self) -> CaptureStatistics {
        self.stats.read().clone()
    }

    /// Get current state
    pub fn get_state(&self) -> CaptureState {
        *self.state.read()
    }

    /// Create a checkpoint
    pub async fn checkpoint(&self) -> Result<CaptureCheckpoint> {
        let checkpoint = CaptureCheckpoint {
            lsn: self.current_lsn.load(Ordering::SeqCst),
            event_id: self.next_event_id.load(Ordering::SeqCst),
            timestamp: SystemTime::now(),
            active_transactions: self.active_txns.read().keys().copied().collect(),
        };

        *self.last_checkpoint.write() = Some(checkpoint.clone());
        Ok(checkpoint)
    }

    /// Restore from checkpoint
    pub async fn restore_from_checkpoint(&self, checkpoint: &CaptureCheckpoint) -> Result<()> {
        self.current_lsn.store(checkpoint.lsn, Ordering::SeqCst);
        self.next_event_id.store(checkpoint.event_id, Ordering::SeqCst);
        Ok(())
    }

    // Helper methods

    fn extract_table_id(&self, page_id: u64) -> TableId {
        // Extract table ID from page ID (simplified - high bits)
        (page_id >> 32) as TableId
    }

    async fn get_table_metadata(&self, table_id: TableId) -> Result<TableMetadata> {
        let cache = self.table_metadata.read();
        if let Some(metadata) = cache.get(&table_id) {
            return Ok(metadata.clone());
        }
        drop(cache);

        // Create dummy metadata (in production, fetch from catalog)
        let metadata = TableMetadata {
            table_id,
            table_name: format!("table_{}", table_id),
            column_names: vec!["col1".to_string(), "col2".to_string()],
            column_types: vec!["INTEGER".to_string(), "VARCHAR".to_string()],
            primary_key_columns: vec![0],
        };

        self.table_metadata.write().insert(table_id, metadata.clone());
        Ok(metadata)
    }

    fn parse_row_data(
        &self,
        data: &[u8],
        metadata: &TableMetadata,
    ) -> Result<HashMap<String, Value>> {
        // Simplified parsing (in production, use proper deserialization)
        let mut row = HashMap::new();
        for (idx, col_name) in metadata.column_names.iter().enumerate() {
            // Dummy value
            row.insert(col_name.clone(), Value::Integer(idx as i64));
        }
        Ok(row)
    }

    fn spawn_batch_processor(&self) {
        let current_batch = self.current_batch.clone();
        let next_batch_id = self.next_batch_id.clone();
        let config = self.config.clone();
        let shutdown = self.shutdown.clone();
        let _state = self.state.clone();
        let flush_fn = {
            let engine = self.clone_for_task();
            move |batch: ChangeEventBatch| {
                let engine = engine.clone();
                async move {
                    engine.flush_batch(batch).await
                }
            }
        };

        tokio::spawn(async move {
            let mut interval = interval(config.batch_timeout);

            while !shutdown.load(Ordering::SeqCst) {
                interval.tick().await;

                if *state.read() != CaptureState::Running {
                    continue;
                }

                let completed = {
                    let mut batch = current_batch.lock();
                    if !batch.is_empty() {
                        Some(std::mem::replace(
                            &mut *batch,
                            ChangeEventBatch::new(next_batch_id.fetch_add(1, Ordering::SeqCst)),
                        ))
                    } else {
                        None
                    }
                };

                if let Some(completed) = completed {
                    let _ = flush_fn(completed).await;
                }
            }
        });
    }

    fn spawn_checkpoint_task(&self) {
        let checkpoint_interval = self.config.checkpoint_interval;
        let shutdown = self.shutdown.clone();
        let engine = self.clone_for_task();

        tokio::spawn(async move {
            let mut interval = interval(checkpoint_interval);

            while !shutdown.load(Ordering::SeqCst) {
                interval.tick().await;
                let _ = engine.checkpoint().await;
            }
        });
    }

    fn spawn_metrics_collector(&self) {
        if !self.config.enable_metrics {
            return;
        }

        let _stats = self.stats.clone();
        let shutdown = self.shutdown.clone();

        tokio::spawn(async move {
            let mut interval = interval(Duration::from_secs(1));
            let mut last_events = 0u64;

            while !shutdown.load(Ordering::SeqCst) {
                interval.tick().await;

                let mut stats = stats.write();
                let current_events = stats.total_events;
                stats.events_per_second = (current_events - last_events) as f64;
                last_events = current_events;
            }
        });
    }

    fn clone_for_task(&self) -> Arc<Self> {
        Arc::new(Self {
            config: self.config.clone(),
            state: self.state.clone(),
            next_event_id: self.next_event_id.clone(),
            next_batch_id: self.next_batch_id.clone(),
            current_lsn: self.current_lsn.clone(),
            event_buffer: self.event_buffer.clone(),
            current_batch: self.current_batch.clone(),
            event_tx: self.event_tx.clone(),
            batch_tx: self.batch_tx.clone(),
            stats: self.stats.clone(),
            table_metadata: self.table_metadata.clone(),
            active_txns: self.active_txns.clone(),
            last_checkpoint: self.last_checkpoint.clone(),
            shutdown: self.shutdown.clone(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_change_event_creation() {
        let event = ChangeEvent::new(1, 100, 1, 1, "users".to_string(), ChangeType::Insert);
        assert_eq!(event.event_id, 1);
        assert_eq!(event.lsn, 100);
        assert_eq!(event.change_type, ChangeType::Insert);
    }

    #[test]
    fn test_column_change() {
        let mut change = ColumnChange::new("name".to_string(), 0);
        change = change.with_new_value(Value::String("Alice".to_string()));
        assert!(change.is_modified());
        assert_eq!(change.column_name, "name");
    }

    #[test]
    fn test_capture_filter() {
        let filter = CaptureFilter::default();
        assert!(filter.should_capture(1, &ChangeType::Insert));
        assert!(filter.should_capture(2, &ChangeType::Update));
    }

    #[test]
    fn test_batch_operations() {
        let mut batch = ChangeEventBatch::new(1);
        assert!(batch.is_empty());

        let event = ChangeEvent::new(1, 100, 1, 1, "test".to_string(), ChangeType::Insert);
        batch.add_event(event);

        assert_eq!(batch.len(), 1);
        assert_eq!(batch.start_lsn, 100);
    }

    #[tokio::test]
    async fn test_cdc_engine_lifecycle() {
        let config = CDCConfig::default();
        let engine = CDCEngine::new(config);

        assert_eq!(engine.get_state(), CaptureState::Stopped);

        engine.start().await.unwrap();
        assert_eq!(engine.get_state(), CaptureState::Running);

        engine.pause().unwrap();
        assert_eq!(engine.get_state(), CaptureState::Paused);

        engine.resume().unwrap();
        assert_eq!(engine.get_state(), CaptureState::Running);

        engine.stop().await.unwrap();
        assert_eq!(engine.get_state(), CaptureState::Stopped);
    }
}
