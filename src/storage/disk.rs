use crate::common::PageId;
use crate::error::{DbError, Result};
use crate::storage::page::Page;
use parking_lot::RwLock;
use std::collections::{HashMap, VecDeque};
use std::fs::{File, OpenOptions};
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::Instant;

#[cfg(target_arch = "x86_64")]
use std::arch::x86_64::*;

// I/O operation priority levels
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum IoPriority {
    Low = 0,
    Normal = 1,
    High = 2,
    Critical = 3,
}

// I/O operation type
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum IoOpType {
    Read,
    Write,
    Sync,
}

// I/O operation descriptor
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

// I/O scheduler with deadline-aware prioritization
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

// Read-ahead buffer for sequential access patterns
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

// Write-behind buffer for batching writes
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

// Direct I/O configuration
#[repr(C)]
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

// Hardware-accelerated CRC32C checksum (SSE4.2 on x86_64)
#[inline]
pub fn hardware_crc32c(data: &[u8]) -> u32 {
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
        let value= *ptr;
        crc = _mm_crc32_u8(crc, value);
        ptr = ptr.add(1);
        remaining -= 1;
    }

    !crc
}

// Software fallback CRC32C
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

// Vectored I/O batch for efficient multi-page operations
#[derive(Debug, Clone)]
pub struct VectoredIoBatch {
    pub pages: Vec<Page>,
    pub offsets: Vec<u64>,
    pub total_bytes: usize,
}

impl VectoredIoBatch {
    pub fn new() -> Self {
        Self {
            pages: Vec::new(),
            offsets: Vec::new(),
            total_bytes: 0,
        }
    }

    pub fn add_page(&mut self, page: Page, offset: u64, page_size: usize) {
        self.total_bytes += page_size;
        self.offsets.push(offset);
        self.pages.push(page);
    }

    pub fn len(&self) -> usize {
        self.pages.len()
    }

    pub fn is_empty(&self) -> bool {
        self.pages.is_empty()
    }

    pub fn clear(&mut self) {
        self.pages.clear();
        self.offsets.clear();
        self.total_bytes = 0;
    }
}

// Write coalescing engine for merging adjacent writes
struct WriteCoalescer {
    pending_writes: HashMap<PageId, (Page, u64)>,
    coalesce_window_us: u64,
    last_flush: Instant,
    max_batch_size: usize,
}

impl WriteCoalescer {
    fn new(coalesce_window_us: u64, max_batch_size: usize) -> Self {
        Self {
            pending_writes: HashMap::new(),
            coalesce_window_us,
            last_flush: Instant::now(),
            max_batch_size,
        }
    }

    fn add_write(&mut self, page: Page, offset: u64) -> bool {
        self.pending_writes.insert(page.id, (page, offset));
        self.should_flush()
    }

    fn should_flush(&self) -> bool {
        if self.pending_writes.len() >= self.max_batch_size {
            return true;
        }
        if self.last_flush.elapsed().as_micros() as u64 >= self.coalesce_window_us {
            return true;
        }
        false
    }

    fn get_coalesced_batch(&mut self) -> VectoredIoBatch {
        let mut batch = VectoredIoBatch::new();

        // Sort by offset for sequential writes
        let mut writes: Vec<_> = self.pending_writes.drain().collect();
        writes.sort_by_key(|(_, (_, offset))| *offset);

        for (_, (page, offset)) in writes {
            batch.add_page(page.clone(), offset, page.data.len());
        }

        self.last_flush = Instant::now();
        batch
    }

    fn pending_count(&self) -> usize {
        self.pending_writes.len()
    }
}

// io_uring operation descriptor (placeholder for future implementation)
#[derive(Debug, Clone)]
pub struct IoUringOp {
    pub op_type: IoOpType,
    pub page_id: PageId,
    pub offset: u64,
    pub data: Vec<u8>,
    pub user_data: u64,
}

impl IoUringOp {
    pub fn read(page_id: PageId, offset: u64, size: usize) -> Self {
        Self {
            op_type: IoOpType::Read,
            page_id,
            offset,
            data: vec![0u8; size],
            user_data: page_id as u64,
        }
    }

    pub fn write(page_id: PageId, offset: u64, data: Vec<u8>) -> Self {
        Self {
            op_type: IoOpType::Write,
            page_id,
            offset,
            data,
            user_data: page_id as u64,
        }
    }
}

// io_uring interface (simulated for now, real implementation would use io-uring crate)
pub struct IoUring {
    submission_queue: VecDeque<IoUringOp>,
    completion_queue: VecDeque<(u64, Result<usize>)>,
    max_queue_depth: usize,
    polling_mode: bool,
}

impl IoUring {
    pub fn new(queue_depth: usize, polling_mode: bool) -> Self {
        Self {
            submission_queue: VecDeque::with_capacity(queue_depth),
            completion_queue: VecDeque::with_capacity(queue_depth),
            max_queue_depth: queue_depth,
            polling_mode,
        }
    }

    pub fn submit_op(&mut self, op: IoUringOp) -> Result<()> {
        if self.submission_queue.len() >= self.max_queue_depth {
            return Err(DbError::Storage("io_uring queue full".to_string()));
        }
        self.submission_queue.push_back(op);
        Ok(())
    }

    pub fn submit_batch(&mut self) -> Result<usize> {
        // In real implementation, this would submit to kernel io_uring
        let count = self.submission_queue.len();
        Ok(count)
    }

    pub fn wait_completions(&mut self, min_complete: usize) -> Result<usize> {
        // Simulated - real implementation would wait on io_uring
        Ok(min_complete)
    }

    pub fn get_completion(&mut self) -> Option<(u64, Result<usize>)> {
        self.completion_queue.pop_front()
    }

    pub fn pending_submissions(&self) -> usize {
        self.submission_queue.len()
    }
}

// Advanced disk manager with I/O optimizations
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

    // Write coalescing engine
    write_coalescer: Arc<Mutex<WriteCoalescer>>,

    // io_uring interface
    io_uring: Arc<Mutex<IoUring>>,

    // Direct I/O configuration
    direct_io_config: DirectIoConfig,

    // Adaptive page sizing
    adaptive_page_size: bool,
    min_page_size: usize,
    max_page_size: usize,

    // Statistics
    stats: Arc<RwLock<DiskStats>>,
}

#[repr(C)]
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
    vectored_reads: u64,
    vectored_writes: u64,
    coalesced_writes: u64,
    io_uring_ops: u64,
    hardware_crc_ops: u64,
    total_iops: u64,
    peak_iops: u64,
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
            write_coalescer: Arc::new(Mutex::new(WriteCoalescer::new(5000, 64))), // 5ms window, 64 pages
            io_uring: Arc::new(Mutex::new(IoUring::new(256, false))), // 256 queue depth
            direct_io_config,
            adaptive_page_size: false,
            min_page_size: 4096,
            max_page_size: 2 * 1024 * 1024, // 2MB
            stats: Arc::new(RwLock::new(DiskStats::default())),
        })
    }

    #[inline]
    pub fn read_page(&self, page_id: PageId) -> Result<Page> {
        let start = Instant::now();

        // Check read-ahead buffer first
        {
            let mut read_ahead = self.read_ahead.lock()
                .map_err(|e| DbError::Storage(format!("Mutex poisoned: {}", e)))?;
            if let Some(data) = read_ahead.get(page_id) {
                let mut stats = self.stats.write();
                stats.read_ahead_hits += 1;
                stats.reads += 1;
                return Ok(Page::from_bytes(page_id, data));
            }
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
        let mut read_ahead = self.read_ahead.lock()
            .map_err(|e| DbError::Storage(format!("Mutex poisoned: {}", e)))?;
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

    #[inline]
    pub fn write_page(&self, page: &Page) -> Result<()> {
        let start = Instant::now();

        // Try write-behind buffer first
        let mut write_behind = self.write_behind.lock()
            .map_err(|e| DbError::Storage(format!("Mutex poisoned: {}", e)))?;
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
        let mut write_behind = self.write_behind.lock()
            .map_err(|e| DbError::Storage(format!("Mutex poisoned: {}", e)))?;

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
        let mut write_behind = self.write_behind.lock()
            .map_err(|e| DbError::Storage(format!("Mutex poisoned: {}", e)))?;
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
        let page = Page::new(page_id as PageId as PageId, self.page_size);
        self.write_page(&page)?;

        Ok(page_id as PageId as PageId)
    }

    pub fn get_num_pages(&self) -> u32 {
        *self.num_pages.lock().unwrap_or_else(|e| e.into_inner())
    }

    // Async read operation (simulated - would use io_uring in production)
    pub fn read_page_async(&self, page_id: PageId, priority: IoPriority) -> Result<()> {
        let op = IoOperation::new(IoOpType::Read, page_id, priority);
        self.scheduler.lock().unwrap().schedule(op);
        Ok(())
    }

    // Async write operation (simulated - would use io_uring in production)
    pub fn write_page_async(&self, page: &Page, priority: IoPriority) -> Result<()> {
        let op = IoOperation::new(IoOpType::Write, page.id, priority);
        self.scheduler.lock().unwrap().schedule(op);

        // Buffer the write
        self.write_behind.lock()
            .map_err(|e| DbError::Storage(format!("Mutex poisoned: {}", e)))?
            .add(page.id, page.data.clone());

        Ok(())
    }

    // Process pending async operations
    pub fn process_async_ops(&self, max_ops: usize) -> Result<usize> {
        let mut processed = 0;

        for _ in 0..max_ops {
            let op = {
                let mut scheduler = self.scheduler.lock().unwrap();
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

    // Vectored read - read multiple pages in a single syscall
    pub fn read_pages_vectored(&self, page_ids: &[PageId]) -> Result<Vec<Page>> {
        let start = Instant::now();

        if page_ids.is_empty() {
            return Ok(Vec::new());
        }

        let mut file = self.data_file.lock()
            .map_err(|e| DbError::Storage(format!("Mutex poisoned: {}", e)))?;

        let mut pages = Vec::with_capacity(page_ids.len());
        let mut bufs: Vec<Vec<u8>> = page_ids.iter()
            .map(|_| vec![0u8; self.page_size])
            .collect();

        // Read pages sequentially (in real impl with preadv, would be single syscall)
        for (idx, &page_id) in page_ids.iter().enumerate() {
            let offset = page_id as u64 * self.page_size as u64;
            file.seek(SeekFrom::Start(offset))?;
            file.read_exact(&mut bufs[idx])?;

            pages.push(Page::from_bytes(page_id, bufs[idx].clone()));
        }

        // Update statistics
        let latency = start.elapsed().as_micros() as u64;
        let mut stats = self.stats.write();
        stats.reads += page_ids.len() as u64;
        stats.vectored_reads += 1;
        stats.read_bytes += (page_ids.len() * self.page_size) as u64;
        stats.avg_read_latency_us = (stats.avg_read_latency_us + latency) / 2;
        stats.total_iops += page_ids.len() as u64;

        Ok(pages)
    }

    // Vectored write - write multiple pages in a single syscall
    pub fn write_pages_vectored(&self, pages: &[Page]) -> Result<()> {
        let start = Instant::now();

        if pages.is_empty() {
            return Ok(());
        }

        let mut file = self.data_file.lock()
            .map_err(|e| DbError::Storage(format!("Mutex poisoned: {}", e)))?;

        // Sort pages by ID for sequential writes
        let mut sorted_pages: Vec<_> = pages.iter().collect();
        sorted_pages.sort_by_key(|p| p.id);

        // Write pages (in real impl with pwritev, would be single syscall)
        for page in sorted_pages {
            let offset = page.id as u64 * self.page_size as u64;
            file.seek(SeekFrom::Start(offset))?;
            file.write_all(&page.data)?;
        }

        // Sync if needed
        if self.direct_io_config.enabled {
            file.sync_data()?;
        }

        // Update statistics
        let latency = start.elapsed().as_micros() as u64;
        let mut stats = self.stats.write();
        stats.writes += pages.len() as u64;
        stats.vectored_writes += 1;
        stats.write_bytes += (pages.len() * self.page_size) as u64;
        stats.avg_write_latency_us = (stats.avg_write_latency_us + latency) / 2;
        stats.total_iops += pages.len() as u64;

        Ok(())
    }

    // Write with coalescing - batches writes automatically
    pub fn write_page_coalesced(&self, page: &Page) -> Result<()> {
        let offset = page.id as u64 * self.page_size as u64;

        let should_flush = {
            let mut coalescer = self.write_coalescer.lock()
                .map_err(|e| DbError::Storage(format!("Mutex poisoned: {}", e)))?;
            coalescer.add_write(page.clone(), offset)
        };

        if should_flush {
            self.flush_coalesced_writes()?;
        }

        let mut stats = self.stats.write();
        stats.coalesced_writes += 1;

        Ok(())
    }

    // Flush coalesced writes
    pub fn flush_coalesced_writes(&self) -> Result<()> {
        let batch = {
            let mut coalescer = self.write_coalescer.lock()
                .map_err(|e| DbError::Storage(format!("Mutex poisoned: {}", e)))?;
            coalescer.get_coalesced_batch()
        };

        if !batch.is_empty() {
            self.write_pages_vectored(&batch.pages)?;
        }

        Ok(())
    }

    // Submit async read via io_uring
    pub fn read_page_io_uring(&self, page_id: PageId) -> Result<()> {
        let offset = page_id as u64 * self.page_size as u64;
        let op = IoUringOp::read(page_id, offset, self.page_size);

        let mut io_uring = self.io_uring.lock()
            .map_err(|e| DbError::Storage(format!("Mutex poisoned: {}", e)))?;
        io_uring.submit_op(op)?;

        self.stats.write().io_uring_ops += 1;

        Ok(())
    }

    // Submit async write via io_uring
    pub fn write_page_io_uring(&self, page: &Page) -> Result<()> {
        let offset = page.id as u64 * self.page_size as u64;
        let op = IoUringOp::write(page.id, offset, page.data.clone());

        let mut io_uring = self.io_uring.lock()
            .map_err(|e| DbError::Storage(format!("Mutex poisoned: {}", e)))?;
        io_uring.submit_op(op)?;

        self.stats.write().io_uring_ops += 1;

        Ok(())
    }

    // Submit pending io_uring operations
    pub fn submit_io_uring_batch(&self) -> Result<usize> {
        let mut io_uring = self.io_uring.lock()
            .map_err(|e| DbError::Storage(format!("Mutex poisoned: {}", e)))?;
        io_uring.submit_batch()
    }

    // Wait for io_uring completions
    pub fn wait_io_uring_completions(&self, min_complete: usize) -> Result<Vec<(u64, Result<usize>)>> {
        let mut io_uring = self.io_uring.lock()
            .map_err(|e| DbError::Storage(format!("Mutex poisoned: {}", e)))?;
        io_uring.wait_completions(min_complete)?;

        let mut completions = Vec::new();
        while let Some(completion) = io_uring.get_completion() {
            completions.push(completion);
        }

        Ok(completions)
    }

    // Compute hardware-accelerated checksum for a page
    pub fn compute_hardware_checksum(&self, data: &[u8]) -> u32 {
        self.stats.write().hardware_crc_ops += 1;
        hardware_crc32c(data)
    }

    // Select adaptive page size based on workload
    pub fn select_adaptive_page_size(&self, data_size: usize, access_pattern: &str) -> usize {
        if !self.adaptive_page_size {
            return self.page_size;
        }

        match access_pattern {
            "sequential" | "scan" => {
                // Large pages for sequential access
                std::cmp::min(data_size.next_power_of_two(), self.max_page_size)
            }
            "random" | "point" => {
                // Small pages for random access
                std::cmp::max(self.min_page_size, self.page_size)
            }
            _ => self.page_size,
        }
    }

    // Get enhanced statistics
    pub fn get_enhanced_stats(&self) -> DiskStats {
        self.stats.read().clone()
    }

    // Calculate current IOPS
    pub fn calculate_iops(&self, duration_secs: f64) -> f64 {
        let stats = self.stats.read();
        stats.total_iops as f64 / duration_secs
    }
}

#[cfg(test)]
mod tests {
    use tempfile::tempdir;
    use super::IoScheduler;
    use crate::DbError;
    use crate::storage::disk::{IoOpType, IoOperation};
    use crate::storage::{DiskManager, IoPriority};

    #[test]
    fn test_disk_manager() -> Result<(), DbError> {
        let dir = tempdir()?;
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
    fn test_read_ahead() -> Result<(), DbError> {
        let dir = tempdir()?;
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
    fn test_write_behind() -> Result<(), DbError> {
        let dir = tempdir()?;
        let dm = DiskManager::new(dir.path().to_str().unwrap(), 4096)?;

        let page_id = dm.allocate_page()?;
        let mut page = dm.read_page(page_id)?;
        page.data[0] = 99;

        dm.write_page(&page)?;

        let stats = dm.get_stats();
        assert!(stats.write_behind_hits > 0);

        Ok(())
    }

    struct ScheduleInfo {
    // Placeholder for future scheduling metadata
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
