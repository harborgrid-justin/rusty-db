use std::fs::{File, OpenOptions};
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::collections::{HashMap, VecDeque};
use std::time::{Duration, Instant};
use parking_lot::RwLock;
use crate::Result;
use crate::storage::page::{Page, PageId};
use crate::error::DbError;

/// I/O operation priority levels
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum IoPriority {
    Low = 0,
    Normal = 1,
    High = 2,
    Critical = 3,
}

/// I/O operation type
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum IoOpType {
    Read,
    Write,
    Sync,
}

/// I/O operation descriptor
#[derive(Debug, Clone)]
struct IoOperation {
    op_type: IoOpType,
    page_id: PageId,
    priority: IoPriority,
    deadline: Option<Instant>,
    submitted_at: Instant,
}

impl IoOperation {
    fn new(op_type: IoOpType, page_id: PageId, priority: IoPriority) -> Self {
        Self {
            op_type,
            page_id,
            priority,
            deadline: None,
            submitted_at: Instant::now(),
        }
    }

    fn with_deadline(mut self, deadline: Instant) -> Self {
        self.deadline = Some(deadline);
        self
    }

    fn is_overdue(&self) -> bool {
        if let Some(deadline) = self.deadline {
            Instant::now() > deadline
        } else {
            false
        }
    }
}

/// I/O scheduler with deadline-aware prioritization
struct IoScheduler {
    read_queue: VecDeque<IoOperation>,
    write_queue: VecDeque<IoOperation>,
    sync_queue: VecDeque<IoOperation>,
    pending_ops: HashMap<PageId, IoOperation>,
}

impl IoScheduler {
    fn new() -> Self {
        Self {
            read_queue: VecDeque::new(),
            write_queue: VecDeque::new(),
            sync_queue: VecDeque::new(),
            pending_ops: HashMap::new(),
        }
    }

    fn schedule(&mut self, op: IoOperation) {
        let page_id = op.page_id;

        // Coalesce duplicate operations
        if let Some(existing) = self.pending_ops.get(&page_id) {
            if existing.priority < op.priority {
                // Replace with higher priority operation
                self.pending_ops.insert(page_id, op.clone());
            }
            return;
        }

        self.pending_ops.insert(page_id, op.clone());

        match op.op_type {
            IoOpType::Read => self.read_queue.push_back(op),
            IoOpType::Write => self.write_queue.push_back(op),
            IoOpType::Sync => self.sync_queue.push_back(op),
        }
    }

    fn next_operation(&mut self) -> Option<IoOperation> {
        // Always process sync operations first
        if let Some(op) = self.sync_queue.pop_front() {
            self.pending_ops.remove(&op.page_id);
            return Some(op);
        }

        // Check for overdue operations
        for queue in [&mut self.read_queue, &mut self.write_queue] {
            if let Some(idx) = queue.iter().position(|op| op.is_overdue()) {
                let op = queue.remove(idx).unwrap();
                self.pending_ops.remove(&op.page_id);
                return Some(op);
            }
        }

        // Prioritize reads over writes (read-preferring)
        if let Some(op) = self.read_queue.pop_front() {
            self.pending_ops.remove(&op.page_id);
            return Some(op);
        }

        if let Some(op) = self.write_queue.pop_front() {
            self.pending_ops.remove(&op.page_id);
            return Some(op);
        }

        None
    }

    fn pending_count(&self) -> usize {
        self.read_queue.len() + self.write_queue.len() + self.sync_queue.len()
    }
}

/// Read-ahead buffer for sequential access patterns
struct ReadAheadBuffer {
    buffer: HashMap<PageId, Vec<u8>>,
    max_pages: usize,
    window_size: usize,
    last_access: PageId,
    access_pattern: Vec<PageId>,
}

impl ReadAheadBuffer {
    fn new(max_pages: usize, window_size: usize) -> Self {
        Self {
            buffer: HashMap::new(),
            max_pages,
            window_size,
            last_access: 0,
            access_pattern: Vec::new(),
        }
    }

    fn get(&mut self, page_id: PageId) -> Option<Vec<u8>> {
        self.record_access(page_id);
        self.buffer.remove(&page_id)
    }

    fn prefetch(&mut self, page_id: PageId, data: Vec<u8>) {
        if self.buffer.len() >= self.max_pages {
            // Evict oldest entry
            if let Some(&oldest) = self.buffer.keys().next() {
                self.buffer.remove(&oldest);
            }
        }

        self.buffer.insert(page_id, data);
    }

    fn record_access(&mut self, page_id: PageId) {
        self.access_pattern.push(page_id);
        if self.access_pattern.len() > self.window_size {
            self.access_pattern.remove(0);
        }
        self.last_access = page_id;
    }

    fn predict_next_pages(&self) -> Vec<PageId> {
        if self.access_pattern.len() < 2 {
            return vec![self.last_access + 1];
        }

        // Detect sequential access pattern
        let is_sequential = self.access_pattern.windows(2)
            .all(|w| w[1] == w[0] + 1);

        if is_sequential {
            // Prefetch next N pages
            (1..=4).map(|offset| self.last_access + offset).collect()
        } else {
            // Random access - prefetch just the next page
            vec![self.last_access + 1]
        }
    }

    fn clear(&mut self) {
        self.buffer.clear();
    }
}

/// Write-behind buffer for batching writes
struct WriteBehindBuffer {
    buffer: HashMap<PageId, Vec<u8>>,
    max_pages: usize,
    dirty_pages: Vec<PageId>,
    batch_size: usize,
}

impl WriteBehindBuffer {
    fn new(max_pages: usize, batch_size: usize) -> Self {
        Self {
            buffer: HashMap::new(),
            max_pages,
            dirty_pages: Vec::new(),
            batch_size,
        }
    }

    fn add(&mut self, page_id: PageId, data: Vec<u8>) -> bool {
        if self.buffer.len() >= self.max_pages {
            return false;
        }

        self.buffer.insert(page_id, data);
        if !self.dirty_pages.contains(&page_id) {
            self.dirty_pages.push(page_id);
        }

        true
    }

    fn should_flush(&self) -> bool {
        self.dirty_pages.len() >= self.batch_size
    }

    fn get_flush_batch(&mut self) -> Vec<(PageId, Vec<u8>)> {
        let batch_size = self.batch_size.min(self.dirty_pages.len());
        if batch_size == 0 {
            return Vec::new();
        }

        // Sort for sequential writes
        self.dirty_pages.sort_unstable();

        let page_ids: Vec<PageId> = self.dirty_pages.drain(..batch_size).collect();

        page_ids.into_iter()
            .filter_map(|page_id| {
                self.buffer.remove(&page_id).map(|data| (page_id, data))
            })
            .collect()
    }

    fn flush_all(&mut self) -> Vec<(PageId, Vec<u8>)> {
        self.dirty_pages.sort_unstable();

        let page_ids: Vec<PageId> = self.dirty_pages.drain(..).collect();

        page_ids.into_iter()
            .filter_map(|page_id| {
                self.buffer.remove(&page_id).map(|data| (page_id, data))
            })
            .collect()
    }

    fn contains(&self, page_id: PageId) -> bool {
        self.buffer.contains_key(&page_id)
    }
}

/// Direct I/O configuration
#[derive(Debug, Clone, Copy)]
pub struct DirectIoConfig {
    pub enabled: bool,
    pub alignment: usize,
    pub min_size: usize,
}

impl Default for DirectIoConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            alignment: 4096,  // 4KB alignment for most systems
            min_size: 4096,
        }
    }
}

/// Advanced disk manager with I/O optimizations
#[derive(Clone)]
pub struct DiskManager {
    data_file: Arc<Mutex<File>>,
    pub page_size: usize,
    num_pages: Arc<Mutex<u32>>,

    // I/O scheduling
    scheduler: Arc<Mutex<IoScheduler>>,

    // Read-ahead and write-behind buffers
    read_ahead: Arc<Mutex<ReadAheadBuffer>>,
    write_behind: Arc<Mutex<WriteBehindBuffer>>,

    // Direct I/O configuration
    direct_io_config: DirectIoConfig,

    // Statistics
    stats: Arc<RwLock<DiskStats>>,
}

#[derive(Debug, Clone, Default)]
struct DiskStats {
    reads: u64,
    writes: u64,
    read_bytes: u64,
    write_bytes: u64,
    read_ahead_hits: u64,
    write_behind_hits: u64,
    avg_read_latency_us: u64,
    avg_write_latency_us: u64,
}

impl DiskManager {
    pub fn new(data_dir: &str, page_size: usize) -> Result<Self> {
        Self::with_config(data_dir, page_size, DirectIoConfig::default())
    }

    pub fn with_config(
        data_dir: &str,
        page_size: usize,
        direct_io_config: DirectIoConfig,
    ) -> Result<Self> {
        std::fs::create_dir_all(data_dir)?;

        let mut path = PathBuf::from(data_dir);
        path.push("data.db");

        let mut options = OpenOptions::new();
        options.read(true).write(true).create(true);

        // Note: Direct I/O would require platform-specific flags (O_DIRECT on Linux)
        // For portability, we simulate the behavior

        let file = options.open(&path)?;
        let metadata = file.metadata()?;
        let num_pages = (metadata.len() / page_size as u64) as u32;

        Ok(Self {
            data_file: Arc::new(Mutex::new(file)),
            page_size,
            num_pages: Arc::new(Mutex::new(num_pages)),
            scheduler: Arc::new(Mutex::new(IoScheduler::new())),
            read_ahead: Arc::new(Mutex::new(ReadAheadBuffer::new(64, 10))),
            write_behind: Arc::new(Mutex::new(WriteBehindBuffer::new(128, 32))),
            direct_io_config,
            stats: Arc::new(RwLock::new(DiskStats::default())),
        })
    }

    pub fn read_page(&self, page_id: PageId) -> Result<Page> {
        let start = Instant::now();

        // Check read-ahead buffer first
        if let Some(data) = self.read_ahead.lock().get(page_id) {
            let mut stats = self.stats.write();
            stats.read_ahead_hits += 1;
            stats.reads += 1;
            return Ok(Page::from_bytes(page_id, data));
        }

        // Perform actual read
        let page = self.read_from_disk(page_id)?;

        // Update statistics
        let latency = start.elapsed().as_micros() as u64;
        let mut stats = self.stats.write();
        stats.reads += 1;
        stats.read_bytes += self.page_size as u64;
        stats.avg_read_latency_us = (stats.avg_read_latency_us + latency) / 2;

        // Trigger read-ahead
        self.trigger_read_ahead(page_id)?;

        Ok(page)
    }

    fn read_from_disk(&self, page_id: PageId) -> Result<Page> {
        let mut file = self.data_file.lock()
            .map_err(|e| DbError::Storage(format!("Mutex poisoned: {}", e)))?;
        let offset = page_id as u64 * self.page_size as u64;

        file.seek(SeekFrom::Start(offset))?;

        let mut data = vec![0u8; self.page_size];
        file.read_exact(&mut data)?;

        Ok(Page::from_bytes(page_id, data))
    }

    fn trigger_read_ahead(&self, page_id: PageId) -> Result<()> {
        let mut read_ahead = self.read_ahead.lock();
        let next_pages = read_ahead.predict_next_pages();

        // Prefetch predicted pages
        for next_page_id in next_pages {
            if !read_ahead.buffer.contains_key(&next_page_id) {
                if let Ok(page) = self.read_from_disk(next_page_id) {
                    read_ahead.prefetch(next_page_id, page.data);
                }
            }
        }

        Ok(())
    }

    pub fn write_page(&self, page: &Page) -> Result<()> {
        let start = Instant::now();

        // Try write-behind buffer first
        let mut write_behind = self.write_behind.lock();
        if write_behind.add(page.id, page.data.clone()) {
            drop(write_behind);

            // Update statistics
            let mut stats = self.stats.write();
            stats.write_behind_hits += 1;

            // Flush if needed
            self.flush_write_behind_if_needed()?;

            return Ok(());
        }
        drop(write_behind);

        // Write directly if buffer is full
        self.write_to_disk(page)?;

        // Update statistics
        let latency = start.elapsed().as_micros() as u64;
        let mut stats = self.stats.write();
        stats.writes += 1;
        stats.write_bytes += self.page_size as u64;
        stats.avg_write_latency_us = (stats.avg_write_latency_us + latency) / 2;

        Ok(())
    }

    fn write_to_disk(&self, page: &Page) -> Result<()> {
        let mut file = self.data_file.lock()
            .map_err(|e| DbError::Storage(format!("Mutex poisoned: {}", e)))?;
        let offset = page.id as u64 * self.page_size as u64;

        file.seek(SeekFrom::Start(offset))?;
        file.write_all(&page.data)?;

        // Sync based on configuration
        if self.direct_io_config.enabled {
            file.sync_data()?;
        }

        Ok(())
    }

    fn flush_write_behind_if_needed(&self) -> Result<()> {
        let mut write_behind = self.write_behind.lock();

        if write_behind.should_flush() {
            let batch = write_behind.get_flush_batch();
            drop(write_behind);

            for (page_id, data) in batch {
                let page = Page::from_bytes(page_id, data);
                self.write_to_disk(&page)?;
            }
        }

        Ok(())
    }

    pub fn flush_all_writes(&self) -> Result<()> {
        let mut write_behind = self.write_behind.lock();
        let batch = write_behind.flush_all();
        drop(write_behind);

        for (page_id, data) in batch {
            let page = Page::from_bytes(page_id, data);
            self.write_to_disk(&page)?;
        }

        // Force sync
        let file = self.data_file.lock()
            .map_err(|e| DbError::Storage(format!("Mutex poisoned: {}", e)))?;
        file.sync_all()?;

        Ok(())
    }

    pub fn allocate_page(&self) -> Result<PageId> {
        let mut num_pages = self.num_pages.lock()
            .map_err(|e| DbError::Storage(format!("Mutex poisoned: {}", e)))?;
        let page_id = *num_pages;
        *num_pages += 1;

        // Write empty page
        let page = Page::new(page_id, self.page_size);
        self.write_page(&page)?;

        Ok(page_id)
    }

    pub fn get_num_pages(&self) -> u32 {
        *self.num_pages.lock().unwrap_or_else(|e| e.into_inner())
    }

    /// Async read operation (simulated - would use io_uring in production)
    pub fn read_page_async(&self, page_id: PageId, priority: IoPriority) -> Result<()> {
        let op = IoOperation::new(IoOpType::Read, page_id, priority);
        self.scheduler.lock().schedule(op);
        Ok(())
    }

    /// Async write operation (simulated - would use io_uring in production)
    pub fn write_page_async(&self, page: &Page, priority: IoPriority) -> Result<()> {
        let op = IoOperation::new(IoOpType::Write, page.id, priority);
        self.scheduler.lock().schedule(op);

        // Buffer the write
        self.write_behind.lock().add(page.id, page.data.clone());

        Ok(())
    }

    /// Process pending async operations
    pub fn process_async_ops(&self, max_ops: usize) -> Result<usize> {
        let mut processed = 0;

        for _ in 0..max_ops {
            let op = {
                let mut scheduler = self.scheduler.lock();
                scheduler.next_operation()
            };

            if let Some(op) = op {
                match op.op_type {
                    IoOpType::Read => {
                        self.read_page(op.page_id)?;
                    }
                    IoOpType::Write => {
                        self.flush_write_behind_if_needed()?;
                    }
                    IoOpType::Sync => {
                        self.flush_all_writes()?;
                    }
                }
                processed += 1;
            } else {
                break;
            }
        }

        Ok(processed)
    }

    pub fn get_stats(&self) -> DiskStats {
        self.stats.read().clone()
    }

    pub fn reset_stats(&self) {
        *self.stats.write() = DiskStats::default();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_disk_manager() -> Result<()> {
        let dir = tempdir().unwrap();
        let dm = DiskManager::new(dir.path().to_str().unwrap(), 4096)?;

        let page_id = dm.allocate_page()?;
        assert_eq!(page_id, 0);

        let mut page = dm.read_page(page_id)?;
        page.data[0] = 42;
        dm.write_page(&page)?;

        let loaded = dm.read_page(page_id)?;
        assert_eq!(loaded.data[0], 42);

        Ok(())
    }

    #[test]
    fn test_read_ahead() -> Result<()> {
        let dir = tempdir().unwrap();
        let dm = DiskManager::new(dir.path().to_str().unwrap(), 4096)?;

        // Allocate sequential pages
        for _ in 0..10 {
            dm.allocate_page()?;
        }

        // Read sequentially to trigger read-ahead
        for i in 0..5 {
            dm.read_page(i)?;
        }

        let stats = dm.get_stats();
        assert!(stats.read_ahead_hits > 0);

        Ok(())
    }

    #[test]
    fn test_write_behind() -> Result<()> {
        let dir = tempdir().unwrap();
        let dm = DiskManager::new(dir.path().to_str().unwrap(), 4096)?;

        let page_id = dm.allocate_page()?;
        let mut page = dm.read_page(page_id)?;
        page.data[0] = 99;

        dm.write_page(&page)?;

        let stats = dm.get_stats();
        assert!(stats.write_behind_hits > 0);

        Ok(())
    }

    #[test]
    fn test_io_scheduler() {
        let mut scheduler = IoScheduler::new();

        let op1 = IoOperation::new(IoOpType::Read, 1, IoPriority::Normal);
        let op2 = IoOperation::new(IoOpType::Write, 2, IoPriority::High);
        let op3 = IoOperation::new(IoOpType::Sync, 3, IoPriority::Critical);

        scheduler.schedule(op1);
        scheduler.schedule(op2);
        scheduler.schedule(op3);

        assert_eq!(scheduler.pending_count(), 3);

        // Sync should be processed first
        let next = scheduler.next_operation().unwrap();
        assert_eq!(next.op_type, IoOpType::Sync);
    }
}
