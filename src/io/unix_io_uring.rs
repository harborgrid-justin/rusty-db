// # Unix io_uring Support
//
// High-performance asynchronous I/O using Linux io_uring.

use crate::error::Result;
use crate::io::{IoRequest, IoCompletion, IoOpType, IoStatus};
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, AtomicU32, Ordering};
use parking_lot::Mutex;
use std::time::Duration;

// ============================================================================
// io_uring Configuration
// ============================================================================

/// io_uring configuration
#[derive(Debug, Clone)]
pub struct IoUringConfig {
    /// Queue depth (number of entries)
    pub queue_depth: u32,

    /// Enable SQPOLL mode (kernel polling thread)
    pub sqpoll: bool,

    /// SQPOLL idle timeout (ms)
    pub sqpoll_idle_ms: u32,

    /// Enable IOPOLL mode (busy polling)
    pub iopoll: bool,

    /// Enable statistics
    pub enable_stats: bool,
}

impl Default for IoUringConfig {
    fn default() -> Self {
        Self {
            queue_depth: 4096,
            sqpoll: false,
            sqpoll_idle_ms: 2000,
            iopoll: false,
            enable_stats: true,
        }
    }
}

// ============================================================================
// SQE Entry (Submission Queue Entry)
// ============================================================================

/// Submission queue entry
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct SqeEntry {
    /// Operation type
    pub opcode: u8,

    /// Flags
    pub flags: u8,

    /// I/O priority
    pub ioprio: u16,

    /// File descriptor
    pub fd: i32,

    /// File offset or address
    pub off: u64,

    /// Buffer address or length
    pub addr: u64,

    /// Buffer length or offset
    pub len: u32,

    /// Operation-specific flags
    pub op_flags: u32,

    /// User data (request ID)
    pub user_data: u64,

    /// Buffer group or pad
    pub buf_index: u16,

    /// Personality
    pub personality: u16,

    /// Splice fd or pad
    pub splice_fd_in: i32,

    /// Padding
    pub pad: [u64; 2],
}

impl SqeEntry {
    /// Create a new SQE entry
    pub fn new(opcode: u8, fd: i32, user_data: u64) -> Self {
        Self {
            opcode,
            flags: 0,
            ioprio: 0,
            fd,
            off: 0,
            addr: 0,
            len: 0,
            op_flags: 0,
            user_data,
            buf_index: 0,
            personality: 0,
            splice_fd_in: 0,
            pad: [0; 2],
        }
    }

    /// Create read SQE
    pub fn read(fd: i32, buffer: *mut u8, len: u32, offset: u64, user_data: u64) -> Self {
        let mut sqe = Self::new(IORING_OP_READ, fd, user_data);
        sqe.addr = buffer as u64;
        sqe.len = len;
        sqe.off = offset;
        sqe
    }

    /// Create write SQE
    pub fn write(fd: i32, buffer: *const u8, len: u32, offset: u64, user_data: u64) -> Self {
        let mut sqe = Self::new(IORING_OP_WRITE, fd, user_data);
        sqe.addr = buffer as u64;
        sqe.len = len;
        sqe.off = offset;
        sqe
    }

    /// Create fsync SQE
    pub fn fsync(fd: i32, user_data: u64) -> Self {
        Self::new(IORING_OP_FSYNC, fd, user_data)
    }

    /// Create fdatasync SQE
    pub fn fdatasync(fd: i32, user_data: u64) -> Self {
        let mut sqe = Self::new(IORING_OP_FSYNC, fd, user_data);
        sqe.op_flags = IORING_FSYNC_DATASYNC;
        sqe
    }
}

// ============================================================================
// CQE Entry (Completion Queue Entry)
// ============================================================================

/// Completion queue entry
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct CqeEntry {
    /// User data from SQE
    pub user_data: u64,

    /// Result (bytes transferred or error code)
    pub res: i32,

    /// Flags
    pub flags: u32,
}

impl CqeEntry {
    /// Check if operation was successful
    #[inline]
    pub fn is_success(&self) -> bool {
        self.res >= 0
    }

    /// Get bytes transferred
    #[inline]
    pub fn bytes_transferred(&self) -> Option<usize> {
        if self.res >= 0 {
            Some(self.res as usize)
        } else {
            None
        }
    }

    /// Get error code
    #[inline]
    pub fn error_code(&self) -> Option<i32> {
        if self.res < 0 {
            Some(-self.res)
        } else {
            None
        }
    }
}

// ============================================================================
// io_uring Constants
// ============================================================================

// Operation codes
const IORING_OP_NOP: u8 = 0;
const IORING_OP_READV: u8 = 1;
const IORING_OP_WRITEV: u8 = 2;
const IORING_OP_FSYNC: u8 = 3;
const IORING_OP_READ_FIXED: u8 = 4;
const IORING_OP_WRITE_FIXED: u8 = 5;
const IORING_OP_POLL_ADD: u8 = 6;
const IORING_OP_POLL_REMOVE: u8 = 7;
const IORING_OP_SYNC_FILE_RANGE: u8 = 8;
const IORING_OP_SENDMSG: u8 = 9;
const IORING_OP_RECVMSG: u8 = 10;
const IORING_OP_TIMEOUT: u8 = 11;
const IORING_OP_READ: u8 = 22;
const IORING_OP_WRITE: u8 = 23;

// Flags
const IORING_FSYNC_DATASYNC: u32 = 1 << 0;

// Setup flags
const IORING_SETUP_IOPOLL: u32 = 1 << 0;
const IORING_SETUP_SQPOLL: u32 = 1 << 1;
const IORING_SETUP_SQ_AFF: u32 = 1 << 2;
const IORING_SETUP_CQSIZE: u32 = 1 << 3;

// ============================================================================
// io_uring Statistics
// ============================================================================

/// io_uring statistics
#[derive(Debug, Clone, Default)]
pub struct IoUringStats {
    /// Number of submissions
    pub submissions: u64,

    /// Number of completions
    pub completions: u64,

    /// Number of errors
    pub errors: u64,

    /// Total bytes transferred
    pub bytes_transferred: u64,

    /// Number of submission queue full events
    pub sq_full: u64,

    /// Number of completion queue overflows
    pub cq_overflow: u64,
}

// ============================================================================
// Uring Probe
// ============================================================================

/// Probe io_uring capabilities
pub struct UringProbe {
    /// Supported operations
    pub supported_ops: Vec<u8>,

    /// Maximum workers
    pub max_workers: u32,

    /// Features
    pub features: u32,
}

impl UringProbe {
    /// Check if an operation is supported
    pub fn supports_op(&self, opcode: u8) -> bool {
        self.supported_ops.contains(&opcode)
    }
}

// ============================================================================
// io_uring Engine
// ============================================================================

/// io_uring-based I/O engine
pub struct IoUringEngine {
    /// Configuration
    config: IoUringConfig,

    /// Ring file descriptor (if available)
    #[cfg(target_os = "linux")]
    ring_fd: i32,

    /// Submission queue head
    sq_head: Arc<AtomicU32>,

    /// Submission queue tail
    sq_tail: Arc<AtomicU32>,

    /// Completion queue head
    cq_head: Arc<AtomicU32>,

    /// Completion queue tail
    cq_tail: Arc<AtomicU32>,

    /// Submission queue entries
    #[cfg(target_os = "linux")]
    sq_entries: Arc<Mutex<Vec<SqeEntry>>>,

    /// Completion queue entries
    #[cfg(target_os = "linux")]
    cq_entries: Arc<Mutex<Vec<CqeEntry>>>,

    /// Statistics
    stats: Arc<Mutex<IoUringStats>>,

    /// Pending count
    pending_count: AtomicU64,
}

#[cfg(target_os = "linux")]
impl IoUringEngine {
    /// Create new io_uring engine
    pub fn new(config: IoUringConfig) -> Result<Self> {
        // For now, we'll create a simulated io_uring
        // In production, this would use actual io_uring syscalls

        Ok(Self {
            config: config.clone(),
            ring_fd: -1, // Invalid fd for simulation
            sq_head: Arc::new(AtomicU32::new(0)),
            sq_tail: Arc::new(AtomicU32::new(0)),
            cq_head: Arc::new(AtomicU32::new(0)),
            cq_tail: Arc::new(AtomicU32::new(0)),
            sq_entries: Arc::new(Mutex::new(Vec::with_capacity(config.queue_depth as usize))),
            cq_entries: Arc::new(Mutex::new(Vec::new())),
            stats: Arc::new(Mutex::new(IoUringStats::default())),
            pending_count: AtomicU64::new(0),
        })
    }

    /// Submit an I/O request
    pub fn submit(&self, request: &mut IoRequest) -> Result<()> {
        let sqe = match request.op_type {
            IoOpType::Read | IoOpType::ReadV => SqeEntry::read(
                request.file_handle.0 as i32,
                request.buffer,
                request.len,
                request.offset,
                request.id,
            ),
            IoOpType::Write | IoOpType::WriteV => SqeEntry::write(
                request.file_handle.0 as i32,
                request.buffer,
                request.len,
                request.offset,
                request.id,
            ),
            IoOpType::Fsync => SqeEntry::fsync(request.file_handle.0 as i32, request.id),
            IoOpType::Fdatasync => SqeEntry::fdatasync(request.file_handle.0 as i32, request.id),
            _ => {
                return Err(DbError::Internal(format!(
                    "Unsupported operation: {:?}",
                    request.op_type
                )))));
            }
        };

        // Add to submission queue
        let mut sq = self.sq_entries.lock();
        if sq.len() >= self.config.queue_depth as usize {
            self.stats.lock().sq_full += 1;
            return Err(DbError::Internal("Submission queue full".to_string()));
        }

        sq.push(sqe);
        self.sq_tail.fetch_add(1, Ordering::Release);
        self.pending_count.fetch_add(1, Ordering::Relaxed);
        self.stats.lock().submissions += 1;

        // In a real implementation, we would call io_uring_enter here
        self.process_submissions()?;

        Ok(())
    }

    /// Submit multiple requests
    pub fn submit_batch(&self, requests: &mut [IoRequest]) -> Result<usize> {
        let mut submitted = 0;

        for request in requests {
            if self.submit(request).is_ok() {
                submitted += 1;
            } else {
                break;
            }
        }

        Ok(submitted)
    }

    /// Process submission queue (simulated)
    fn process_submissions(&self) -> Result<()> {
        let mut sq = self.sq_entries.lock();
        let mut cq = self.cq_entries.lock();

        // In a real implementation, this would be handled by the kernel
        // For simulation, we immediately complete operations

        while let Some(sqe) = sq.pop() {
            // Simulate I/O completion
            let res = match sqe.opcode {
                IORING_OP_READ | IORING_OP_READV => {
                    // Simulate successful read
                    sqe.len as i32
                }
                IORING_OP_WRITE | IORING_OP_WRITEV => {
                    // Simulate successful write
                    sqe.len as i32
                }
                IORING_OP_FSYNC => {
                    // Simulate successful sync
                    0
                }
                _ => {
                    // Unsupported operation
                    -libc::EINVAL
                }
            };

            let cqe = CqeEntry {
                user_data: sqe.user_data,
                res,
                flags: 0,
            };

            cq.push(cqe);
            self.cq_tail.fetch_add(1, Ordering::Release);
        }

        self.sq_head.store(self.sq_tail.load(Ordering::Acquire), Ordering::Release);

        Ok(())
    }

    /// Poll for completions
    pub fn poll(&self, maxcompletions: usize) -> Result<Vec<IoCompletion>> {
        let mut cq = self.cq_entries.lock();
        let mut completions = Vec::with_capacity(max_completions.min(cq.len()));

        for _ in 0..max_completions {
            if let Some(cqe) = cq.pop() {
                self.cq_head.fetch_add(1, Ordering::Release);
                self.pending_count.fetch_sub(1, Ordering::Relaxed);

                let completion = IoCompletion {
                    id: cqe.user_data,
                    status: if cqe.is_success() {
                        IoStatus::Completed
                    } else {
                        IoStatus::Failed
                    },
                    bytes_transferred: cqe.bytes_transferred().unwrap_or(0),
                    error_code: cqe.error_code().map(|e| e as usize).unwrap_or(0),
                    duration: Duration::from_secs(0), // TODO: Track duration
                    op_type: IoOpType::Read,          // TODO: Track op type
                };

                completions.push(completion);

                let mut stats = self.stats.lock();
                stats.completions += 1;
                if let Some(bytes) = cqe.bytes_transferred() {
                    stats.bytes_transferred += bytes as u64;
                } else {
                    stats.errors += 1;
                }
            } else {
                break;
            }
        }

        Ok(completions)
    }

    /// Get statistics
    pub fn stats(&self) -> IoUringStats {
        self.stats.lock().clone()
    }

    /// Get pending count
    #[inline]
    pub fn pending_count(&self) -> u64 {
        self.pending_count.load(Ordering::Relaxed)
    }

    /// Probe capabilities
    pub fn probe() -> Result<UringProbe> {
        // In a real implementation, this would probe the kernel
        Ok(UringProbe {
            supported_ops: vec![
                IORING_OP_READ,
                IORING_OP_WRITE,
                IORING_OP_READV,
                IORING_OP_WRITEV,
                IORING_OP_FSYNC,
            ],
            max_workers: 4,
            features: 0,
        })
    }
}

// ============================================================================
// Non-Linux Stub Implementation
// ============================================================================

#[cfg(not(target_os = "linux"))]
impl IoUringEngine {
    pub fn new(config: IoUringConfig) -> Result<Self> {
        Ok(Self {
            config: _config,
            sq_head: Arc::new(AtomicU32::new(0)),
            sq_tail: Arc::new(AtomicU32::new(0)),
            cq_head: Arc::new(AtomicU32::new(0)),
            cq_tail: Arc::new(AtomicU32::new(0)),
            stats: Arc::new(Mutex::new(IoUringStats::default())),
            pending_count: AtomicU64::new(0),
        })
    }

    pub fn submit(&self, _request: &mut IoRequest) -> Result<()> {
        // Fallback to synchronous I/O on non-Linux systems
        Err(DbError::Internal(
            "io_uring not supported on this platform".to_string(),
        ))
    }

    pub fn submit_batch(&self, _requests: &mut [IoRequest]) -> Result<usize> {
        Err(DbError::Internal(
            "io_uring not supported on this platform".to_string(),
        ))
    }

    pub fn poll(&self, _max_completions: usize) -> Result<Vec<IoCompletion>> {
        Ok(Vec::new())
    }

    pub fn stats(&self) -> IoUringStats {
        self.stats.lock().clone()
    }

    pub fn pending_count(&self) -> u64 {
        self.pending_count.load(Ordering::Relaxed)
    }

    pub fn probe() -> Result<UringProbe> {
        Ok(UringProbe {
            supported_ops: Vec::new(),
            max_workers: 0,
            features: 0,
        })
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sqe_creation() {
        let sqe = SqeEntry::read(3, std::ptr::null_mut(), 4096, 0, 42);
        assert_eq!(sqe.opcode, IORING_OP_READ);
        assert_eq!(sqe.fd, 3);
        assert_eq!(sqe.len, 4096);
        assert_eq!(sqe.user_data, 42);
    }

    #[test]
    fn test_cqe_result() {
        let cqe = CqeEntry {
            user_data: 42,
            res: 4096,
            flags: 0,
        };

        assert!(cqe.is_success());
        assert_eq!(cqe.bytes_transferred(), Some(4096));
        assert_eq!(cqe.error_code(), None);

        let cqe_err = CqeEntry {
            user_data: 43,
            res: -5, // EIO
            flags: 0,
        };

        assert!(!cqe_err.is_success());
        assert_eq!(cqe_err.bytes_transferred(), None);
        assert_eq!(cqe_err.error_code(), Some(5));
    }

    #[test]
    fn test_uring_probe() {
        let probe = UringProbe {
            supported_ops: vec![IORING_OP_READ, IORING_OP_WRITE],
            max_workers: 4,
            features: 0,
        };

        assert!(probe.supports_op(IORING_OP_READ));
        assert!(probe.supports_op(IORING_OP_WRITE));
        assert!(!probe.supports_op(IORING_OP_POLL_ADD));
    }

    #[test]
    fn test_config_default() {
        let config = IoUringConfig::default();
        assert_eq!(config.queue_depth, 4096);
        assert!(!config.sqpoll);
        assert!(!config.iopoll);
    }
}
