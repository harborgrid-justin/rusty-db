// # Async I/O Engine
//
// Core asynchronous I/O engine providing completion-based I/O abstraction
// across Windows IOCP and Unix io_uring.

use tokio::sync::oneshot;
use std::time::Instant;
use crate::error::{Result, DbError};
use std::sync::atomic::{AtomicU8, AtomicU64, AtomicUsize, Ordering};
use std::sync::Arc;
use parking_lot::{Mutex, RwLock};
use std::collections::HashMap;
use std::time::{Duration};
use tokio::sync::{Notify};

// ============================================================================
// I/O Operation Types
// ============================================================================

/// Type of I/O operation
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum IoOpType {
    /// Read operation
    Read = 0,
    /// Write operation
    Write = 1,
    /// Sync/flush operation
    Sync = 2,
    /// Vectored read (readv)
    ReadV = 3,
    /// Vectored write (writev)
    WriteV = 4,
    /// Fallocate (pre-allocate space)
    Fallocate = 5,
    /// Fsync (sync file data and metadata)
    Fsync = 6,
    /// Fdatasync (sync file data only)
    Fdatasync = 7,
}

impl IoOpType {
    /// Check if operation is a read
    #[inline]
    pub fn is_read(&self) -> bool {
        matches!(self, IoOpType::Read | IoOpType::ReadV)
    }

    /// Check if operation is a write
    #[inline]
    pub fn is_write(&self) -> bool {
        matches!(self, IoOpType::Write | IoOpType::WriteV)
    }

    /// Check if operation is a sync
    #[inline]
    pub fn is_sync(&self) -> bool {
        matches!(self, IoOpType::Sync | IoOpType::Fsync | IoOpType::Fdatasync)
    }
}

// ============================================================================
// I/O Status
// ============================================================================

/// Status of an I/O operation
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum IoStatus {
    /// Operation is pending
    Pending = 0,
    /// Operation completed successfully
    Completed = 1,
    /// Operation failed
    Failed = 2,
    /// Operation was cancelled
    Cancelled = 3,
}

impl From<u8> for IoStatus {
    fn from(v: u8) -> Self {
        match v {
            0 => IoStatus::Pending,
            1 => IoStatus::Completed,
            2 => IoStatus::Failed,
            3 => IoStatus::Cancelled,
            _ => IoStatus::Failed,
        }
    }
}

// ============================================================================
// I/O Request Structure
// ============================================================================

/// I/O request aligned for direct I/O
///
/// This structure is aligned to 4096 bytes to ensure proper alignment
/// for Direct I/O operations that bypass the OS page cache.
#[repr(C, align(4096))]
pub struct IoRequest {
    /// Unique request ID
    pub id: u64,

    /// Buffer pointer (must be page-aligned for Direct I/O)
    pub buffer: *mut u8,

    /// File offset (must be sector-aligned)
    pub offset: u64,

    /// Length in bytes (must be sector-aligned)
    pub len: u32,

    /// Type of I/O operation
    pub op_type: IoOpType,

    /// Completion status (atomic for lock-free updates)
    completion: AtomicU8,

    /// File handle/descriptor
    pub file_handle: IoHandle,

    /// Number of bytes actually transferred
    pub bytes_transferred: AtomicUsize,

    /// Error code (0 = success)
    pub error_code: AtomicUsize,

    /// Timestamp when request was submitted
    pub submit_time: Instant,

    /// User data (opaque pointer for callbacks)
    pub user_data: usize,
}

impl IoRequest {
    /// Create a new I/O request
    pub fn new(
        id: u64,
        buffer: *mut u8,
        offset: u64,
        len: u32,
        op_type: IoOpType,
        file_handle: IoHandle,
    ) -> Self {
        Self {
            id,
            buffer,
            offset,
            len,
            op_type,
            completion: AtomicU8::new(IoStatus::Pending as u8),
            file_handle,
            bytes_transferred: AtomicUsize::new(0),
            error_code: AtomicUsize::new(0),
            submit_time: Instant::now(),
            user_data: 0,
        }
    }

    /// Get completion status
    #[inline]
    pub fn status(&self) -> IoStatus {
        IoStatus::from(self.completion.load(Ordering::Acquire))
    }

    /// Set completion status
    #[inline]
    pub fn set_status(&self, status: IoStatus) {
        self.completion.store(status as u8, Ordering::Release);
    }

    /// Check if request is complete
    #[inline]
    pub fn is_complete(&self) -> bool {
        matches!(
            self.status(),
            IoStatus::Completed | IoStatus::Failed | IoStatus::Cancelled
        )
    }

    /// Check if request succeeded
    #[inline]
    pub fn is_success(&self) -> bool {
        self.status() == IoStatus::Completed && self.error_code.load(Ordering::Acquire) == 0
    }

    /// Get bytes transferred
    #[inline]
    pub fn get_bytes_transferred(&self) -> usize {
        self.bytes_transferred.load(Ordering::Acquire)
    }

    /// Set completion result
    #[inline]
    pub fn set_result(&self, bytes: usize, error: usize) {
        self.bytes_transferred.store(bytes, Ordering::Release);
        self.error_code.store(error, Ordering::Release);
        if error == 0 && bytes > 0 {
            self.set_status(IoStatus::Completed);
        } else {
            self.set_status(IoStatus::Failed);
        }
    }

    /// Get elapsed time since submission
    #[inline]
    pub fn elapsed(&self) -> Duration {
        self.submit_time.elapsed()
    }
}

// Safety: IoRequest can be sent between threads
unsafe impl Send for IoRequest {}
unsafe impl Sync for IoRequest {}

// ============================================================================
// I/O Handle
// ============================================================================

/// Platform-independent I/O handle
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct IoHandle(pub usize);

impl IoHandle {
    /// Create an invalid handle
    pub const fn invalid() -> Self {
        Self(usize::MAX)
    }

    /// Check if handle is valid
    #[inline]
    pub fn is_valid(&self) -> bool {
        self.0 != usize::MAX
    }
}

#[cfg(unix)]
impl From<std::os::unix::io::RawFd> for IoHandle {
    fn from(fd: std::os::unix::io::RawFd) -> Self {
        Self(fd as usize)
    }
}

#[cfg(windows)]
impl From<std::os::windows::io::RawHandle> for IoHandle {
    fn from(handle: std::os::windows::io::RawHandle) -> Self {
        Self(handle as usize)
    }
}

// ============================================================================
// I/O Completion
// ============================================================================

/// I/O completion result
#[derive(Debug)]
pub struct IoCompletion {
    /// Request ID
    pub id: u64,

    /// Status of the operation
    pub status: IoStatus,

    /// Number of bytes transferred
    pub bytes_transferred: usize,

    /// Error code (0 = success)
    pub error_code: usize,

    /// Duration from submission to completion
    pub duration: Duration,

    /// Operation type
    pub op_type: IoOpType,
}

impl IoCompletion {
    /// Create from an IoRequest
    pub fn from_request(request: &IoRequest) -> Self {
        Self {
            id: request.id,
            status: request.status(),
            bytes_transferred: request.get_bytes_transferred(),
            error_code: request.error_code.load(Ordering::Acquire),
            duration: request.elapsed(),
            op_type: request.op_type,
        }
    }

    /// Check if operation was successful
    #[inline]
    pub fn is_success(&self) -> bool {
        self.status == IoStatus::Completed && self.error_code == 0
    }
}

// ============================================================================
// Completion Callback
// ============================================================================

/// Callback for I/O completion
pub type CompletionCallback = Box<dyn FnOnce(IoCompletion) + Send + 'static>;

// ============================================================================
// I/O Result
// ============================================================================

/// I/O operation result
#[derive(Debug)]
pub struct IoResult {
    /// Number of bytes transferred
    pub bytes_transferred: usize,

    /// Operation type
    pub op_type: IoOpType,

    /// Duration
    pub duration: Duration,
}

// ============================================================================
// I/O Completion Port
// ============================================================================

/// High-performance completion port abstraction
///
/// This provides a unified interface over Windows IOCP and Unix io_uring
/// for asynchronous I/O operations.
pub struct IoCompletionPort {
    /// Number of pending operations
    pending: AtomicU64,

    /// Number of completed operations
    completed: AtomicU64,

    /// Lock-free completion queue
    completions: crossbeam::queue::ArrayQueue<IoCompletion>,

    /// Map of request ID to completion channel
    waiters: Arc<RwLock<HashMap<u64, oneshot::Sender<IoCompletion>>>>,

    /// Notification for new completions
    notify: Arc<Notify>,

    /// Platform-specific backend
    #[cfg(windows)]
    backend: Arc<crate::io::windows_iocp::WindowsIocp>,

    #[cfg(unix)]
    backend: Arc<crate::io::unix_io_uring::IoUringEngine>,

    /// Statistics
    stats: Arc<Mutex<CompletionPortStats>>,
}

impl IoCompletionPort {
    /// Create a new completion port
    pub fn new(queue_size: usize) -> Result<Self> {
        #[cfg(windows)]
        let backend = Arc::new(crate::io::windows_iocp::WindowsIocp::new(
            crate::io::windows_iocp::IocpConfig::default(),
        )?);

        #[cfg(unix)]
        let backend = Arc::new(crate::io::unix_io_uring::IoUringEngine::new(
            crate::io::unix_io_uring::IoUringConfig::default(),
        )?);

        Ok(Self {
            pending: AtomicU64::new(0),
            completed: AtomicU64::new(0),
            completions: crossbeam::queue::ArrayQueue::new(queue_size),
            waiters: Arc::new(RwLock::new(HashMap::new())),
            notify: Arc::new(Notify::new()),
            #[cfg(any(windows, unix))]
            backend,
            stats: Arc::new(Mutex::new(CompletionPortStats::default())),
        })
    }

    /// Submit an I/O request
    #[inline]
    pub fn submit(&self, request: &mut IoRequest) -> Result<()> {
        self.pending.fetch_add(1, Ordering::Relaxed);

        #[cfg(windows)]
        unsafe { self.backend.submit(request)?; }

        #[cfg(unix)]
        self.backend.submit(request)?;

        #[cfg(not(any(windows, unix)))]
        return Err(DbError::Internal("I/O not supported on this platform".to_string()));

        Ok(())
    }

    /// Submit multiple I/O requests in a batch
    pub fn submit_batch(&self, requests: &mut [IoRequest]) -> Result<usize> {
        let count = requests.len();
        self.pending.fetch_add(count as u64, Ordering::Relaxed);

        #[cfg(windows)]
        let submitted = self.backend.submit_batch(requests)?;

        #[cfg(unix)]
        let submitted = self.backend.submit_batch(requests)?;

        #[cfg(not(any(windows, unix)))]
        let submitted = 0;

        if submitted < count {
            self.pending.fetch_sub((count - submitted) as u64, Ordering::Relaxed);
        }

        Ok(submitted)
    }

    /// Poll for completed I/O operations
    ///
    /// Returns the number of completions retrieved.
    pub fn poll(&self, max_completions: usize) -> Result<usize> {
        #[cfg(windows)]
        let completions = self.backend.poll(max_completions)?;

        #[cfg(unix)]
        let completions = self.backend.poll(max_completions)?;

        #[cfg(not(any(windows, unix)))]
        let completions = Vec::new();

        let count = completions.len();

        // Update counters
        self.pending.fetch_sub(count as u64, Ordering::Relaxed);
        self.completed.fetch_add(count as u64, Ordering::Relaxed);

        // Process completions
        for completion in completions {
            // Check if someone is waiting for this specific completion
            let mut waiters = self.waiters.write();
            if let Some(sender) = waiters.remove(&completion.id) {
                let _ = sender.send(completion);
            } else {
                // Add to completion queue
                if self.completions.push(completion).is_err() {
                    // Queue is full, record error
                    self.stats.lock().unwrap().queue_overflows += 1;
                }
            }
        }

        if count > 0 {
            self.notify.notify_waiters();
        }

        Ok(count)
    }

    /// Get the next completion (non-blocking)
    #[inline]
    pub fn try_get_completion(&self) -> Option<IoCompletion> {
        self.completions.pop()
    }

    /// Wait for a completion (blocking)
    pub async fn get_completion(&self) -> Option<IoCompletion> {
        loop {
            // Try to get a completion
            if let Some(completion) = self.try_get_completion() {
                return Some(completion);
            }

            // No completions available, wait for notification
            self.notify.notified().await;
        }
    }

    /// Wait for a specific I/O request to complete
    pub async fn wait_for(&self, request_id: u64) -> Result<IoCompletion> {
        let (tx, rx) = oneshot::channel();

        // Register waiter
        self.waiters.write().insert(request_id, tx);

        // Wait for completion
        rx.await.map_err(|_| DbError::Internal("I/O completion channel closed".to_string()))
    }

    /// Get number of pending operations
    #[inline]
    pub fn pending_count(&self) -> u64 {
        self.pending.load(Ordering::Relaxed)
    }

    /// Get number of completed operations
    #[inline]
    pub fn completed_count(&self) -> u64 {
        self.completed.load(Ordering::Relaxed)
    }

    /// Get statistics
    pub fn stats(&self) -> CompletionPortStats {
        self.stats.lock().unwrap().clone()
    }
}

// ============================================================================
// Statistics
// ============================================================================

/// Completion port statistics
#[derive(Debug, Clone, Default)]
pub struct CompletionPortStats {
    /// Number of submissions
    pub submissions: u64,

    /// Number of completions
    pub completions: u64,

    /// Number of errors
    pub errors: u64,

    /// Number of queue overflows
    pub queue_overflows: u64,

    /// Total bytes read
    pub bytes_read: u64,

    /// Total bytes written
    pub bytes_written: u64,
}

// ============================================================================
// Async I/O Engine
// ============================================================================

/// Main async I/O engine
pub struct AsyncIoEngine {
    /// Completion port
    completion_port: Arc<IoCompletionPort>,

    /// Next request ID
    next_id: AtomicU64,

    /// Worker threads
    workers: Vec<std::thread::JoinHandle<()>>,

    /// Shutdown flag
    shutdown: Arc<AtomicU8>,

    /// Configuration
    config: crate::io::IoEngineConfig,
}

impl AsyncIoEngine {
    /// Create a new async I/O engine
    pub fn new(config: crate::io::IoEngineConfig) -> Result<Self> {
        let completion_port = Arc::new(IoCompletionPort::new(config.ring_size)?);
        let shutdown = Arc::new(AtomicU8::new(0));

        // Spawn worker threads
        let mut workers = Vec::new();
        for i in 0..config.worker_threads {
            let cp = completion_port.clone();
            let sd = shutdown.clone();

            let handle = std::thread::Builder::new()
                .name(format!("io-worker-{}", i))
                .spawn(move || {
                    Self::worker_loop(cp, sd)
                })
                .map_err(|e| DbError::Internal(format!("Failed to spawn I/O worker: {}", e)))?;

            workers.push(handle);
        }

        Ok(Self {
            completion_port,
            next_id: AtomicU64::new(1),
            workers,
            shutdown,
            config,
        })
    }

    /// Worker thread main loop
    fn worker_loop(completion_port: Arc<IoCompletionPort>, shutdown: Arc<AtomicU8>) {
        while shutdown.load(Ordering::Relaxed) == 0 {
            // Poll for completions
            match completion_port.poll(256) {
                Ok(count) => {
                    if count == 0 {
                        // No completions briefly
                        std::thread::sleep(Duration::from_micros(100));
                    }
                }
                Err(e) => {
                    tracing::error!("I/O poll error: {}", e);
                    std::thread::sleep(Duration::from_millis(1));
                }
            }
        }
    }

    /// Get next request ID
    #[inline]
    pub fn next_id(&self) -> u64 {
        self.next_id.fetch_add(1, Ordering::Relaxed)
    }

    /// Submit an I/O request
    #[inline]
    pub fn submit(&self, request: &mut IoRequest) -> Result<()> {
        self.completion_port.submit(request)
    }

    /// Submit multiple requests
    #[inline]
    pub fn submit_batch(&self, requests: &mut [IoRequest]) -> Result<usize> {
        self.completion_port.submit_batch(requests)
    }

    /// Wait for a request to complete
    pub async fn wait(&self, request_id: u64) -> Result<IoCompletion> {
        self.completion_port.wait_for(request_id).await
    }

    /// Get completion port
    #[inline]
    pub fn completion_port(&self) -> &Arc<IoCompletionPort> {
        &self.completion_port
    }

    /// Get statistics
    pub fn stats(&self) -> CompletionPortStats {
        self.completion_port.stats()
    }

    /// Shutdown the engine
    pub fn shutdown(&mut self) {
        self.shutdown.store(1, Ordering::Relaxed);

        // Wait for workers to finish
        while let Some(worker) = self.workers.pop() {
            let _ = worker.join();
        }
    }
}

impl Drop for AsyncIoEngine {
    fn drop(&mut self) {
        self.shutdown();
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::io::PAGE_SIZE;

    #[test]
    fn test_io_request_status() {
        let buffer = vec![0u8; PAGE_SIZE];
        let request = IoRequest::new(
            1,
            buffer.as_ptr() as *mut u8,
            0,
            PAGE_SIZE as u32,
            IoOpType::Read,
            IoHandle::invalid(),
        );

        assert_eq!(request.status(), IoStatus::Pending);
        assert!(!request.is_complete());

        request.set_result(PAGE_SIZE, 0);
        assert_eq!(request.status(), IoStatus::Completed);
        assert!(request.is_complete());
        assert!(request.is_success());
        assert_eq!(request.get_bytes_transferred(), PAGE_SIZE);
    }

    #[test]
    fn test_io_op_type_checks() {
        assert!(IoOpType::Read.is_read());
        assert!(!IoOpType::Read.is_write());
        assert!(!IoOpType::Read.is_sync());

        assert!(IoOpType::Write.is_write());
        assert!(!IoOpType::Write.is_read());

        assert!(IoOpType::Fsync.is_sync());
        assert!(!IoOpType::Fsync.is_read());
        assert!(!IoOpType::Fsync.is_write());
    }

    #[test]
    fn test_io_handle() {
        let invalid = IoHandle::invalid();
        assert!(!invalid.is_valid());

        let valid = IoHandle(42);
        assert!(valid.is_valid());
    }
}
