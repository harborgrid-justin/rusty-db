// # Windows I/O Completion Ports (IOCP)
//
// High-performance asynchronous I/O using Windows IOCP.

use crate::error::Result;
use crate::io::{IoRequest, IoCompletion, IoOpType, IoStatus, IoHandle};
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use parking_lot::Mutex;
use std::time::Duration;
use std::ptr;

// ============================================================================
// Send-safe pointer wrapper
// ============================================================================

/// A Send-safe wrapper for raw pointers
/// SAFETY: The caller must ensure the pointer is valid for the lifetime of the wrapper
/// and that concurrent access is properly synchronized.
#[derive(Clone, Copy)]
struct SendPtr(*mut std::ffi::c_void);

unsafe impl Send for SendPtr {}
unsafe impl Sync for SendPtr {}

impl SendPtr {
    #[inline]
    fn new(ptr: *mut std::ffi::c_void) -> Self {
        Self(ptr)
    }

    #[inline]
    fn as_ptr(self) -> *mut std::ffi::c_void {
        self.0
    }
}

// ============================================================================
// IOCP Configuration
// ============================================================================

/// IOCP configuration
#[derive(Debug, Clone)]
pub struct IocpConfig {
    /// Number of concurrent threads (0 = number of processors)
    pub concurrent_threads: usize,

    /// Completion queue size
    pub queue_size: usize,

    /// Enable statistics
    pub enable_stats: bool,

    /// Timeout for GetQueuedCompletionStatus (ms)
    pub timeout_ms: u32,
}

impl Default for IocpConfig {
    fn default() -> Self {
        Self {
            concurrent_threads: 0,
            queue_size: 4096,
            enable_stats: true,
            timeout_ms: 1000,
        }
    }
}

// ============================================================================
// IOCP Handle
// ============================================================================

/// Windows IOCP handle wrapper
#[derive(Debug, Clone, Copy)]
pub struct IocpHandle(pub usize);

impl IocpHandle {
    /// Create invalid handle
    pub const fn invalid() -> Self {
        Self(0)
    }

    /// Check if valid
    #[inline]
    pub fn is_valid(&self) -> bool {
        self.0 != 0
    }
}

#[cfg(windows)]
impl From<windows_sys::Win32::Foundation::HANDLE> for IocpHandle {
    fn from(handle: windows_sys::Win32::Foundation::HANDLE) -> Self {
        Self(handle as usize)
    }
}

#[cfg(windows)]
impl From<IocpHandle> for windows_sys::Win32::Foundation::HANDLE {
    fn from(handle: IocpHandle) -> Self {
        handle.0 as windows_sys::Win32::Foundation::HANDLE
    }
}

// ============================================================================
// Overlapped I/O
// ============================================================================

/// Overlapped I/O structure for Windows
#[cfg(windows)]
#[repr(C)]
pub struct OverlappedIo {
    /// Windows OVERLAPPED structure
    overlapped: windows_sys::Win32::System::IO::OVERLAPPED,

    /// Associated request ID
    pub request_id: u64,

    /// User data
    pub user_data: usize,
}

// Safety: OverlappedIo is safe to send between threads as it's used for async I/O completion
#[cfg(windows)]
unsafe impl Send for OverlappedIo {}
#[cfg(windows)]
unsafe impl Sync for OverlappedIo {}

#[cfg(windows)]
impl OverlappedIo {
    /// Create new overlapped I/O
    pub fn new(request_id: u64, offset: u64) -> Self {
        let mut overlapped: windows_sys::Win32::System::IO::OVERLAPPED = unsafe { std::mem::zeroed() };

        // Set offset (split into high and low parts)
        overlapped.Anonymous.Anonymous.Offset = (offset & 0xFFFFFFFF) as u32;
        overlapped.Anonymous.Anonymous.OffsetHigh = (offset >> 32) as u32;

        Self {
            overlapped,
            request_id,
            user_data: 0,
        }
    }

    /// Get pointer to OVERLAPPED
    pub fn as_ptr(&mut self) -> *mut windows_sys::Win32::System::IO::OVERLAPPED {
        &mut self.overlapped as *mut _
    }
}

#[cfg(not(windows))]
pub struct OverlappedIo {
    pub request_id: u64,
    pub user_data: usize,
}

#[cfg(not(windows))]
impl OverlappedIo {
    pub fn new(request_id: u64, _offset: u64) -> Self {
        Self {
            request_id,
            user_data: 0,
        }
    }
}

// ============================================================================
// Completion Packet
// ============================================================================

/// I/O completion packet
#[derive(Debug)]
pub struct CompletionPacket {
    /// Number of bytes transferred
    pub bytes_transferred: u32,

    /// Completion key
    pub completion_key: usize,

    /// Request ID
    pub request_id: u64,

    /// Error code
    pub error_code: u32,
}

// ============================================================================
// IOCP Statistics
// ============================================================================

/// IOCP statistics
#[derive(Debug, Clone, Default)]
pub struct IocpStats {
    /// Number of submissions
    pub submissions: u64,

    /// Number of completions
    pub completions: u64,

    /// Number of errors
    pub errors: u64,

    /// Number of timeouts
    pub timeouts: u64,

    /// Total bytes transferred
    pub bytes_transferred: u64,
}

// ============================================================================
// Windows IOCP Engine
// ============================================================================

/// Windows I/O Completion Port engine
pub struct WindowsIocp {
    /// IOCP handle
    #[cfg(windows)]
    iocp_handle: IocpHandle,

    /// Configuration
    config: IocpConfig,

    /// Pending overlapped operations
    #[cfg(windows)]
    pending_ops: Arc<Mutex<std::collections::HashMap<u64, Box<OverlappedIo>>>>,

    /// Statistics
    stats: Arc<Mutex<IocpStats>>,

    /// Number of pending operations
    pending_count: AtomicU64,
}

#[cfg(windows)]
impl WindowsIocp {
    /// Create new IOCP engine
    pub fn new(config: IocpConfig) -> Result<Self> {
        use windows_sys::Win32::System::IO::CreateIoCompletionPort;
        use windows_sys::Win32::Foundation::INVALID_HANDLE_VALUE;

        // Create IOCP
        let iocp_handle = unsafe {
            CreateIoCompletionPort(
                INVALID_HANDLE_VALUE,
                0,
                0,
                config.concurrent_threads as u32,
            )
        };

        if iocp_handle == 0 {
            return Err(DbError::Internal("Failed to create IOCP".to_string()));
        }

        Ok(Self {
            iocp_handle: IocpHandle::from(iocp_handle),
            config,
            pending_ops: Arc::new(Mutex::new(std::collections::HashMap::new())),
            stats: Arc::new(Mutex::new(IocpStats::default())),
            pending_count: AtomicU64::new(0),
        })
    }

    /// Associate a file handle with the IOCP
    pub fn associate_file(&self, file_handle: IoHandle, completion_key: usize) -> Result<()> {
        use windows_sys::Win32::System::IO::CreateIoCompletionPort;

        let _result = unsafe {
            CreateIoCompletionPort(
                file_handle.0 as isize,
                self.iocp_handle.into(),
                completion_key,
                0,
            )
        };

        if result == 0 {
            return Err(DbError::Internal("Failed to associate file with IOCP".to_string()));
        }

        Ok(())
    }

    /// Submit an I/O request
    pub fn submit(&self, request: &mut IoRequest) -> Result<()> {
        // Create overlapped structure
        let mut overlapped = Box::new(OverlappedIo::new(request.id, request.offset));

        // Perform the I/O operation based on type
        match request.op_type {
            IoOpType::Read | IoOpType::ReadV => self.submit_read(request, &mut overlapped)?,
            IoOpType::Write | IoOpType::WriteV => self.submit_write(request, &mut overlapped)?,
            IoOpType::Sync | IoOpType::Fsync | IoOpType::Fdatasync => {
                self.submit_sync(request)?;
                return Ok(());
            }
            _ => {
                return Err(DbError::Internal(format!(
                    "Unsupported operation type: {:?}",
                    request.op_type
                )));
            }
        };

        // Store overlapped for later completion
        self.pending_ops.lock().insert(request.id, overlapped);
        self.pending_count.fetch_add(1, Ordering::Relaxed);
        self.stats.lock().submissions += 1;

        Ok(())
    }

    /// Submit a read operation
    fn submit_read(&self, request: &IoRequest, overlapped: &mut Box<OverlappedIo>) -> Result<bool> {
        use windows_sys::Win32::Storage::FileSystem::ReadFile;

        let mut bytes_read = 0u32;
        let buffer_ptr = SendPtr::new(request.buffer as *mut std::ffi::c_void);

        let _result = unsafe {
            ReadFile(
                request.file_handle.0 as isize,
                buffer_ptr.as_ptr() as *mut u8,
                request.len,
                &mut bytes_read,
                overlapped.as_ptr(),
            )
        };

        if result == 0 {
            let error = unsafe { windows_sys::Win32::Foundation::GetLastError() };
            if error != windows_sys::Win32::Foundation::ERROR_IO_PENDING {
                return Err(DbError::IoError(format!("ReadFile failed: {}", error)));
            }
        }

        Ok(true)
    }

    /// Submit a write operation
    fn submit_write(&self, request: &IoRequest, overlapped: &mut Box<OverlappedIo>) -> Result<bool> {
        use windows_sys::Win32::Storage::FileSystem::WriteFile;

        let mut bytes_written = 0u32;
        let buffer_ptr = SendPtr::new(request.buffer as *mut std::ffi::c_void);

        let _result = unsafe {
            WriteFile(
                request.file_handle.0 as isize,
                buffer_ptr.as_ptr() as *const u8,
                request.len,
                &mut bytes_written,
                overlapped.as_ptr(),
            )
        };

        if result == 0 {
            let error = unsafe { windows_sys::Win32::Foundation::GetLastError() };
            if error != windows_sys::Win32::Foundation::ERROR_IO_PENDING {
                return Err(DbError::IoError(format!("WriteFile failed: {}", error)));
            }
        }

        Ok(true)
    }

    /// Submit a sync operation
    fn submit_sync(&self, request: &IoRequest) -> Result<()> {
        use windows_sys::Win32::Storage::FileSystem::FlushFileBuffers;

        let _result = unsafe { FlushFileBuffers(request.file_handle.0 as isize) };

        if result == 0 {
            let error = unsafe { windows_sys::Win32::Foundation::GetLastError() };
            return Err(DbError::IoError(format!("FlushFileBuffers failed: {}", error)));
        }

        // Mark as completed immediately
        request.set_result(0, 0);

        Ok(())
    }

    /// Submit multiple I/O requests
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

    /// Poll for completions
    pub fn poll(&self, max_completions: usize) -> Result<Vec<IoCompletion>> {
        use windows_sys::Win32::System::IO::GetQueuedCompletionStatus;

        let mut completions = Vec::with_capacity(max_completions);

        for _ in 0..max_completions {
            let mut bytes_transferred = 0u32;
            let mut completion_key = 0usize;
            let mut overlapped_ptr: *mut windows_sys::Win32::System::IO::OVERLAPPED = ptr::null_mut();

            let _result = unsafe {
                GetQueuedCompletionStatus(
                    self.iocp_handle.into(),
                    &mut bytes_transferred,
                    &mut completion_key,
                    &mut overlapped_ptr,
                    self.config.timeout_ms,
                )
            };

            if overlapped_ptr.is_null() {
                // Timeout or no more completions
                if result == 0 {
                    self.stats.lock().timeouts += 1;
                }
                break;
            }

            // Get the overlapped structure
            let overlapped = unsafe {
                let _offset = overlapped_ptr as usize - overlapped_ptr as usize;
                &*(overlapped_ptr as *const OverlappedIo)
            };

            let request_id = overlapped.request_id;

            // Remove from pending
            self.pending_ops.lock().remove(&request_id);
            self.pending_count.fetch_sub(1, Ordering::Relaxed);

            let error_code = if result == 0 {
                unsafe { windows_sys::Win32::Foundation::GetLastError() }
            } else {
                0
            };

            let completion = IoCompletion {
                id: request_id,
                status: if error_code == 0 {
                    IoStatus::Completed
                } else {
                    IoStatus::Failed
                },
                bytes_transferred: bytes_transferred as usize,
                error_code: error_code as usize,
                duration: Duration::from_secs(0), // TODO: Track actual duration
                op_type: IoOpType::Read,          // TODO: Store op type in overlapped
            };

            completions.push(completion);

            let mut stats = self.stats.lock();
            stats.completions += 1;
            stats.bytes_transferred += bytes_transferred as u64;
            if error_code != 0 {
                stats.errors += 1;
            }
        }

        Ok(completions)
    }

    /// Get statistics
    pub fn stats(&self) -> IocpStats {
        self.stats.lock().clone()
    }

    /// Get pending count
    #[inline]
    pub fn pending_count(&self) -> u64 {
        self.pending_count.load(Ordering::Relaxed)
    }
}

#[cfg(windows)]
impl Drop for WindowsIocp {
    fn drop(&mut self) {
        use windows_sys::Win32::Foundation::CloseHandle;

        if self.iocp_handle.is_valid() {
            unsafe {
                CloseHandle(self.iocp_handle.into());
            }
        }
    }
}

// ============================================================================
// Non-Windows Stub Implementation
// ============================================================================

#[cfg(not(windows))]
impl WindowsIocp {
    pub fn new(_config: IocpConfig) -> Result<Self> {
        Err(DbError::Internal(
            "IOCP is only supported on Windows".to_string(),
        ))
    }

    pub fn submit(&self, _request: &mut IoRequest) -> Result<()> {
        Err(DbError::Internal(
            "IOCP is only supported on Windows".to_string(),
        ))
    }

    pub fn submit_batch(&self, _requests: &mut [IoRequest]) -> Result<usize> {
        Err(DbError::Internal(
            "IOCP is only supported on Windows".to_string(),
        ))
    }

    pub fn poll(&self, _max_completions: usize) -> Result<Vec<IoCompletion>> {
        Err(DbError::Internal(
            "IOCP is only supported on Windows".to_string(),
        ))
    }

    pub fn stats(&self) -> IocpStats {
        IocpStats::default()
    }

    pub fn pending_count(&self) -> u64 {
        0
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
#[cfg(windows)]
mod tests {
    use super::*;

    #[test]
    fn test_iocp_handle() {
        let handle = IocpHandle::invalid();
        assert!(!handle.is_valid());

        let handle = IocpHandle(123);
        assert!(handle.is_valid());
    }

    #[test]
    fn test_overlapped_io() {
        let overlapped = OverlappedIo::new(42, 4096);
        assert_eq!(overlapped.request_id, 42);
    }

    #[test]
    fn test_iocp_creation() {
        let config = IocpConfig::default();
        let _result = WindowsIocp::new(config);
        assert!(result.is_ok());
    }
}
