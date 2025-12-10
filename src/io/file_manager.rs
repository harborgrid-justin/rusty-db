// # File Manager
//
// High-performance file manager with batched operations, Direct I/O,
// and buffer pooling.

use crate::error::{Result, DbError};
use crate::io::{
    AsyncIoEngine, IoRequest, IoOpType, IoHandle, AlignedBuffer, BufferPool,
    BufferPoolConfig, PAGE_SIZE, SECTOR_SIZE, align_up, align_down,
};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use parking_lot::RwLock;
use std::fs::{File, OpenOptions};

#[cfg(unix)]
use std::os::unix::io::AsRawFd;

#[cfg(unix)]
use std::os::unix::fs::OpenOptionsExt;
#[cfg(windows)]
use std::os::windows::fs::OpenOptionsExt;
#[cfg(windows)]
use std::os::windows::io::AsRawHandle;

// ============================================================================
// File Handle
// ============================================================================

/// File handle for I/O operations
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct FileHandle(pub u64);

impl FileHandle {
    /// Create an invalid handle
    pub const fn invalid() -> Self {
        Self(u64::MAX)
    }

    /// Check if handle is valid
    #[inline]
    pub fn is_valid(&self) -> bool {
        self.0 != u64::MAX
    }
}

// ============================================================================
// File Metadata
// ============================================================================

/// File metadata
#[derive(Debug, Clone)]
pub struct FileMetadata {
    /// File path
    pub path: PathBuf,

    /// File size in bytes
    pub size: u64,

    /// Whether Direct I/O is enabled
    pub direct_io: bool,

    /// Whether file is read-only
    pub read_only: bool,

    /// Creation time
    pub created: std::time::SystemTime,

    /// Last modified time
    pub modified: std::time::SystemTime,
}

// ============================================================================
// I/O Options
// ============================================================================

/// I/O configuration options
#[derive(Debug, Clone)]
pub struct IoOptions {
    /// Enable Direct I/O (bypass OS cache)
    pub direct_io: bool,

    /// Enable asynchronous I/O
    pub async_io: bool,

    /// Pre-allocate file space
    pub preallocate: bool,

    /// Default buffer pool size
    pub buffer_pool_size: usize,

    /// Maximum concurrent I/O operations per file
    pub max_concurrent_io: usize,

    /// Enable write-through (sync writes immediately)
    pub write_through: bool,

    /// Enable read-ahead
    pub read_ahead: bool,

    /// Read-ahead size in bytes
    pub read_ahead_size: usize,
}

impl Default for IoOptions {
    fn default() -> Self {
        Self {
            direct_io: true,
            async_io: true,
            preallocate: true,
            buffer_pool_size: 1024,
            max_concurrent_io: 256,
            write_through: false,
            read_ahead: true,
            read_ahead_size: 128 * 1024, // 128KB
        }
    }
}

// ============================================================================
// Read/Write Options
// ============================================================================

/// Read operation options
#[derive(Debug, Clone)]
pub struct ReadOptions {
    /// Offset in file
    pub offset: u64,

    /// Number of bytes to read
    pub len: usize,

    /// Enable read-ahead
    pub read_ahead: bool,

    /// Hint about sequential access
    pub sequential: bool,
}

impl ReadOptions {
    /// Create default read options
    pub fn new(offset: u64, len: usize) -> Self {
        Self {
            offset,
            len,
            read_ahead: false,
            sequential: false,
        }
    }
}

/// Write operation options
#[derive(Debug, Clone)]
pub struct WriteOptions {
    /// Offset in file
    pub offset: u64,

    /// Sync immediately after write
    pub sync: bool,

    /// Use write-through cache
    pub write_through: bool,
}

impl WriteOptions {
    /// Create default write options
    pub fn new(offset: u64) -> Self {
        Self {
            offset,
            sync: false,
            write_through: false,
        }
    }

    /// Create with immediate sync
    pub fn with_sync(offset: u64) -> Self {
        Self {
            offset,
            sync: true,
            write_through: false,
        }
    }
}

// ============================================================================
// Flush Mode
// ============================================================================

/// Flush mode for sync operations
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FlushMode {
    /// Flush data only (fdatasync)
    Data,

    /// Flush data and metadata (fsync)
    Full,

    /// Async flush (don't wait)
    Async,
}

// ============================================================================
// Batched I/O Request
// ============================================================================

/// Batched I/O request for multiple operations
#[derive(Debug)]
pub struct BatchedIoRequest {
    /// File handle
    pub file: FileHandle,

    /// Operations to perform
    pub operations: Vec<BatchOperation>,
}

/// Single operation in a batch
#[derive(Debug)]
pub struct BatchOperation {
    /// Operation type
    pub op_type: IoOpType,

    /// Offset in file
    pub offset: u64,

    /// Buffer (for reads, will be filled; for writes, contains data)
    pub buffer: AlignedBuffer,

    /// Expected result (for verification)
    pub expected_bytes: usize,
}

// ============================================================================
// Direct I/O File
// ============================================================================

/// File wrapper for Direct I/O operations
pub struct DirectIoFile {
    /// File handle
    file: File,

    /// I/O handle for async operations
    io_handle: IoHandle,

    /// File path (stored for reference but not actively used)
    #[allow(dead_code)]
    path: PathBuf,

    /// File metadata
    metadata: FileMetadata,

    /// Current file size
    size: AtomicU64,
}

use std::sync::atomic::{AtomicU64, Ordering};

impl DirectIoFile {
    /// Open a file for Direct I/O
    pub fn open<P: AsRef<Path>>(path: P, options: &IoOptions) -> Result<Self> {
        let path = path.as_ref();

        #[cfg(unix)]
        let file = {
            let mut opts = OpenOptions::new();
            opts.read(true).write(true).create(true);

            if options.direct_io {
                // O_DIRECT flag for Linux
                opts.custom_flags(libc::O_DIRECT);
            }

            opts.open(path)
                .map_err(|e| DbError::Io(e))?
        };

        #[cfg(windows)]
        let file = {
            let mut opts = OpenOptions::new();
            opts.read(true).write(true).create(true);

            if options.direct_io {
                // FILE_FLAG_NO_BUFFERING for Windows
                opts.custom_flags(0x20000000);
            }

            if options.write_through {
                // FILE_FLAG_WRITE_THROUGH
                opts.custom_flags(0x80000000);
            }

            opts.open(path)
                .map_err(|e| DbError::Io(e))?
        };

        #[cfg(not(any(unix, windows)))]
        let file = {
            OpenOptions::new()
                .read(true)
                .write(true)
                .create(true)
                .open(path)
                .map_err(|e| DbError::Io(e))?
        };

        let metadata = file
            .metadata()
            .map_err(|e| DbError::Io(e))?;

        let size = metadata.len();

        #[cfg(unix)]
        let io_handle = IoHandle::from(file.as_raw_fd());

        #[cfg(windows)]
        let io_handle = {
            use std::os::windows::io::AsRawHandle;
            IoHandle::from(file.as_raw_handle())
        };

        #[cfg(not(any(unix, windows)))]
        let io_handle = IoHandle::invalid();

        let file_metadata = FileMetadata {
            path: path.to_path_buf(),
            size,
            direct_io: options.direct_io,
            read_only: metadata.permissions().readonly(),
            created: metadata
                .created()
                .unwrap_or(std::time::SystemTime::now()),
            modified: metadata
                .modified()
                .unwrap_or(std::time::SystemTime::now()),
        };

        Ok(Self {
            file,
            io_handle,
            path: path.to_path_buf(),
            metadata: file_metadata,
            size: AtomicU64::new(size),
        })
    }

    /// Get file size
    #[inline]
    pub fn size(&self) -> u64 {
        self.size.load(Ordering::Acquire)
    }

    /// Set file size
    pub fn set_size(&self, new_size: u64) -> Result<()> {
        self.file
            .set_len(new_size)
            .map_err(|e| DbError::Io(e))?;
        self.size.store(new_size, Ordering::Release);
        Ok(())
    }

    /// Get I/O handle
    #[inline]
    pub fn io_handle(&self) -> IoHandle {
        self.io_handle
    }

    /// Get metadata
    #[inline]
    pub fn metadata(&self) -> &FileMetadata {
        &self.metadata
    }

    /// Sync file to disk
    pub fn sync(&self, mode: FlushMode) -> Result<()> {
        match mode {
            FlushMode::Data => {
                #[cfg(unix)]
                {
                    unsafe {
                        if libc::fdatasync(self.file.as_raw_fd()) != 0 {
                            return Err(DbError::Internal("fdatasync failed".to_string()));
                        }
                    }
                }

                #[cfg(not(unix))]
                self.file
                    .sync_data()
                    .map_err(|e| DbError::Io(e))?;
            }
            FlushMode::Full => {
                self.file
                    .sync_all()
                    .map_err(|e| DbError::Io(e))?;
            }
            FlushMode::Async => {
                // For async, we don't wait
            }
        }
        Ok(())
    }

    /// Pre-allocate space
    pub fn preallocate(&self, size: u64) -> Result<()> {
        #[cfg(unix)]
        {
            unsafe {
                if libc::posix_fallocate(self.file.as_raw_fd(), 0, size as i64) != 0 {
                    return Err(DbError::Internal("fallocate failed".to_string()));
                }
            }
        }

        #[cfg(not(unix))]
        self.set_size(size)?;

        Ok(())
    }
}

// ============================================================================
// File Manager
// ============================================================================

/// High-performance file manager
pub struct FileManager {
    /// Open files
    files: Arc<RwLock<HashMap<FileHandle, Arc<DirectIoFile>>>>,

    /// Next file handle ID
    next_handle: AtomicU64,

    /// Buffer pool for I/O operations
    buffer_pool: Arc<BufferPool>,

    /// Async I/O engine
    io_engine: Arc<AsyncIoEngine>,

    /// Configuration options
    options: IoOptions,

    /// Statistics
    stats: Arc<parking_lot::Mutex<FileManagerStats>>,
}

impl FileManager {
    /// Create a new file manager
    pub async fn new(options: IoOptions) -> Result<Self> {
        let buffer_pool = Arc::new(BufferPool::new(BufferPoolConfig {
            pool_size: options.buffer_pool_size,
            buffer_size: PAGE_SIZE,
            alignment: SECTOR_SIZE,
            enable_stats: true,
        })?);

        let io_engine = crate::io::get_io_engine()?;

        Ok(Self {
            files: Arc::new(RwLock::new(HashMap::new())),
            next_handle: AtomicU64::new(1),
            buffer_pool,
            io_engine,
            options,
            stats: Arc::new(parking_lot::Mutex::new(FileManagerStats::default())),
        })
    }

    /// Open a file
    pub async fn open<P: AsRef<Path>>(&mut self, path: P, _create: bool) -> Result<FileHandle> {
        let file = DirectIoFile::open(path, &self.options)?;
        let handle = FileHandle(self.next_handle.fetch_add(1, Ordering::Relaxed));

        self.files.write().insert(handle, Arc::new(file));
        self.stats.lock().files_opened += 1;

        Ok(handle)
    }

    /// Close a file
    pub fn close(&mut self, handle: FileHandle) -> Result<()> {
        self.files.write().remove(&handle);
        self.stats.lock().files_closed += 1;
        Ok(())
    }

    /// Get file metadata
    pub fn metadata(&self, handle: FileHandle) -> Result<FileMetadata> {
        let files = self.files.read();
        let file = files
            .get(&handle)
            .ok_or_else(|| DbError::Internal("Invalid file handle".to_string()))?;
        Ok(file.metadata().clone())
    }

    /// Read a page from file
    pub async fn read_page(&self, handle: FileHandle, offset: u64) -> Result<Vec<u8>> {
        self.read(handle, ReadOptions::new(offset, PAGE_SIZE)).await
    }

    /// Write a page to file
    pub async fn write_page(&self, handle: FileHandle, offset: u64, data: &[u8]) -> Result<()> {
        if data.len() != PAGE_SIZE {
            return Err(DbError::Internal(format!(
                "Invalid page size: expected {}, got {}",
                PAGE_SIZE,
                data.len()
            )));
        }
        self.write(handle, WriteOptions::new(offset), data).await
    }

    /// Read data from file
    pub async fn read(&self, handle: FileHandle, options: ReadOptions) -> Result<Vec<u8>> {
        let files = self.files.read();
        let file = files
            .get(&handle)
            .ok_or_else(|| DbError::Internal("Invalid file handle".to_string()))?;

        // Align offset and length for Direct I/O
        let aligned_offset = align_down(options.offset);
        let aligned_len = align_up((options.offset - aligned_offset) + options.len as u64) as usize;

        // Get buffer from pool
        let buffer = self.buffer_pool.allocate(aligned_len)?;

        // Create I/O request
        let request_id = self.io_engine.next_id();
        let mut request = IoRequest::new(
            request_id,
            buffer.as_ptr() as *mut u8,
            aligned_offset,
            aligned_len as u32,
            IoOpType::Read,
            file.io_handle(),
        );

        // Submit request
        self.io_engine.submit(&mut request)?;

        // Wait for completion
        let completion = self.io_engine.wait(request_id).await?;

        if !completion.is_success() {
            self.stats.lock().read_errors += 1;
            return Err(DbError::Internal(format!(
                "Read failed: error code {}",
                completion.error_code
            )));
        }

        // Extract the requested data from the aligned buffer
        let start = (options.offset - aligned_offset) as usize;
        let end = start + options.len;
        let data = buffer.as_slice()[start..end].to_vec();

        self.stats.lock().reads += 1;
        self.stats.lock().bytes_read += options.len as u64;

        Ok(data)
    }

    /// Write data to file
    pub async fn write(&self, handle: FileHandle, options: WriteOptions, data: &[u8]) -> Result<()> {
        let files = self.files.read();
        let file = files
            .get(&handle)
            .ok_or_else(|| DbError::Internal("Invalid file handle".to_string()))?;

        // Align offset and length for Direct I/O
        let aligned_offset = align_down(options.offset);
        let aligned_len = align_up((options.offset - aligned_offset) + data.len() as u64) as usize;

        // Get buffer from pool
        let mut buffer = self.buffer_pool.allocate(aligned_len)?;

        // If we need to do read-modify-write (unaligned writes)
        if aligned_offset != options.offset || data.len() != aligned_len {
            // Read existing data first
            let request_id = self.io_engine.next_id();
            let mut read_request = IoRequest::new(
                request_id,
                buffer.as_ptr() as *mut u8,
                aligned_offset,
                aligned_len as u32,
                IoOpType::Read,
                file.io_handle(),
            );

            self.io_engine.submit(&mut read_request)?;
            let _ = self.io_engine.wait(request_id).await?;
        }

        // Copy user data into buffer
        let start = (options.offset - aligned_offset) as usize;
        buffer.as_mut_slice()[start..start + data.len()].copy_from_slice(data);

        // Create write request
        let request_id = self.io_engine.next_id();
        let mut request = IoRequest::new(
            request_id,
            buffer.as_ptr() as *mut u8,
            aligned_offset,
            aligned_len as u32,
            IoOpType::Write,
            file.io_handle(),
        );

        // Submit request
        self.io_engine.submit(&mut request)?;

        // Wait for completion
        let completion = self.io_engine.wait(request_id).await?;

        if !completion.is_success() {
            self.stats.lock().write_errors += 1;
            return Err(DbError::Internal(format!(
                "Write failed: error code {}",
                completion.error_code
            )));
        }

        // Sync if requested
        if options.sync {
            file.sync(FlushMode::Full)?;
        }

        self.stats.lock().writes += 1;
        self.stats.lock().bytes_written += data.len() as u64;

        Ok(())
    }

    /// Flush file to disk
    pub async fn flush(&self, handle: FileHandle) -> Result<()> {
        let files = self.files.read();
        let file = files
            .get(&handle)
            .ok_or_else(|| DbError::Internal("Invalid file handle".to_string()))?;

        file.sync(FlushMode::Full)?;
        self.stats.lock().flushes += 1;

        Ok(())
    }

    /// Execute batched I/O operations
    pub async fn execute_batch(&self, batch: BatchedIoRequest) -> Result<Vec<usize>> {
        let files = self.files.read();
        let file = files
            .get(&batch.file)
            .ok_or_else(|| DbError::Internal("Invalid file handle".to_string()))?;

        let mut results = Vec::with_capacity(batch.operations.len());
        let mut requests = Vec::with_capacity(batch.operations.len());

        // Create all requests
        for op in &batch.operations {
            let request_id = self.io_engine.next_id();
            let request = IoRequest::new(
                request_id,
                op.buffer.as_ptr() as *mut u8,
                op.offset,
                op.buffer.len() as u32,
                op.op_type,
                file.io_handle(),
            );
            requests.push(request);
        }

        // Submit batch
        let submitted = self.io_engine.submit_batch(&mut requests)?;

        // Wait for all completions
        for request in requests.iter().take(submitted) {
            let completion = self.io_engine.wait(request.id).await?;
            results.push(completion.bytes_transferred);
        }

        self.stats.lock().batch_operations += 1;

        Ok(results)
    }

    /// Get statistics
    pub fn stats(&self) -> FileManagerStats {
        self.stats.lock().clone()
    }

    /// Get buffer pool statistics
    pub fn buffer_pool_stats(&self) -> crate::io::BufferPoolStats {
        self.buffer_pool.stats()
    }
}

// ============================================================================
// Statistics
// ============================================================================

/// File manager statistics
#[derive(Debug, Clone, Default)]
pub struct FileManagerStats {
    /// Number of files opened
    pub files_opened: u64,

    /// Number of files closed
    pub files_closed: u64,

    /// Number of read operations
    pub reads: u64,

    /// Number of write operations
    pub writes: u64,

    /// Number of flush operations
    pub flushes: u64,

    /// Number of batch operations
    pub batch_operations: u64,

    /// Total bytes read
    pub bytes_read: u64,

    /// Total bytes written
    pub bytes_written: u64,

    /// Number of read errors
    pub read_errors: u64,

    /// Number of write errors
    pub write_errors: u64,
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
use std::time::SystemTime;

    #[test]
    fn test_file_handle() {
        let handle = FileHandle(42);
        assert!(handle.is_valid());

        let invalid = FileHandle::invalid();
        assert!(!invalid.is_valid());
    }

    #[test]
    fn test_read_options() {
        let opts = ReadOptions::new(4096, 512);
        assert_eq!(opts.offset, 4096);
        assert_eq!(opts.len, 512);
        assert!(!opts.read_ahead);
    }

    #[test]
    fn test_write_options() {
        let opts = WriteOptions::new(8192);
        assert_eq!(opts.offset, 8192);
        assert!(!opts.sync);

        let sync_opts = WriteOptions::with_sync(8192);
        assert!(sync_opts.sync);
    }
}
