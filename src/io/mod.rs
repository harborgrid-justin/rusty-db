// # High-Performance I/O Layer for RustyDB
//
// This module provides a high-performance, cross-platform I/O abstraction layer
// optimized for database workloads with support for:
//
// - **Windows IOCP**: I/O Completion Ports for Windows
// - **Unix io_uring**: Modern asynchronous I/O for Linux
// - **Direct I/O**: Bypass OS page cache for database-managed caching
// - **Batched Operations**: Submit multiple I/O requests in a single syscall
// - **Zero-Copy**: Minimize data copying where possible
// - **Lock-Free Queues**: High-throughput concurrent data structures
//
// ## Architecture
//
// ```text
// ┌─────────────────────────────────────────────────┐
// │           High-Level API (FileManager)          │
// ├─────────────────────────────────────────────────┤
// │         Async I/O Engine (IoCompletionPort)     │
// ├─────────────────────────────────────────────────┤
// │       Ring Buffer Queue (IoRingBuffer)          │
// ├─────────────────────────────────────────────────┤
// │  Platform-Specific Layer (IOCP / io_uring)      │
// └─────────────────────────────────────────────────┘
// ```
//
// ## Performance Features
//
// - **Sector-Aligned Buffers**: All I/O buffers aligned to 4KB for Direct I/O
// - **Buffer Pool**: Pre-allocated buffer pool to avoid runtime allocations
// - **Batching**: Multiple I/O operations submitted per syscall
// - **Lock-Free**: Submission and completion queues use atomic operations
// - **Thread Pool**: Fixed thread pool for I/O workers to avoid thread creation overhead
//
// ## Example Usage
//
// ```rust,no_run
// use rusty_db::io::{FileManager, IoOptions};
//
// # async fn example() -> rusty_db::Result<()> {
// let mut file_mgr = FileManager::new(IoOptions::default()).await?;
//
// // Open a file for Direct I/O
// let file_id = file_mgr.open("data.db", true).await?;
//
// // Read a 4KB page at offset 8192
// let buffer = file_mgr.read_page(file_id, 8192).await?;
//
// // Write a page with batching
// file_mgr.write_page(file_id, 8192, &buffer).await?;
//
// // Flush all pending writes
// file_mgr.flush(file_id).await?;
// # Ok(())
// # }
// ```

use crate::error::Result;
use std::sync::Arc;

// ============================================================================
// Core I/O Components
// ============================================================================

mod async_io;
mod file_manager;
mod ring_buffer;
mod buffer_pool;
mod metrics;

// Platform-specific implementations
#[cfg(windows)]
mod windows_iocp;

#[cfg(unix)]
mod unix_io_uring;

// Re-exports
pub use async_io::{
    IoRequest, IoCompletion, IoCompletionPort, IoOpType, IoResult, IoStatus,
    AsyncIoEngine, IoHandle, CompletionCallback,
};

pub use file_manager::{
    FileManager, FileHandle, IoOptions, DirectIoFile, FileMetadata,
    ReadOptions, FlushMode, BatchedIoRequest,
};

pub use ring_buffer::{
    IoRingBuffer, RingBufferStats, SubmissionEntry, CompletionEntry,
    RingBufferConfig, RingBufferError,
};

pub use buffer_pool::{
    BufferPool, AlignedBuffer, BufferPoolConfig, BufferPoolStats,
    PooledBuffer, BufferAllocationStrategy,
};

pub use metrics::{
    IoMetrics, IoStats, LatencyHistogram, ThroughputMetrics,
    IoCounters, PerformanceStats,
};

// ============================================================================
// Platform-Specific Exports
// ============================================================================

#[cfg(windows)]
pub use windows_iocp::{
    WindowsIocp, IocpHandle, IocpConfig, OverlappedIo,
    CompletionPacket, IocpStats,
};

#[cfg(unix)]
pub use unix_io_uring::{
    IoUringEngine, IoUringConfig, SqeEntry, CqeEntry,
    IoUringStats, UringProbe,
};

// ============================================================================
// Constants
// ============================================================================

/// Page size for all I/O operations (4KB)
pub const PAGE_SIZE: usize = 4096;

/// Sector size for disk alignment
pub const SECTOR_SIZE: usize = 4096;

/// Default ring buffer size (power of 2)
pub const DEFAULT_RING_SIZE: usize = 4096;

/// Maximum concurrent I/O operations
pub const MAX_CONCURRENT_IO: usize = 65536;

/// I/O worker thread pool size
pub const IO_WORKER_THREADS: usize = 4;

/// Buffer pool size (number of pre-allocated buffers)
pub const BUFFER_POOL_SIZE: usize = 1024;

/// Maximum batch size for I/O operations
pub const MAX_BATCH_SIZE: usize = 256;

// ============================================================================
// Global I/O Engine
// ============================================================================

/// Global I/O engine instance
static IO_ENGINE: once_cell::sync::OnceCell<Arc<AsyncIoEngine>> = once_cell::sync::OnceCell::new();

/// Initialize the global I/O engine
///
/// This should be called once at application startup.
pub fn init_io_engine(config: IoEngineConfig) -> Result<()> {
    let engine = AsyncIoEngine::new(config)?;
    IO_ENGINE
        .set(Arc::new(engine))
        .map_err(|_| DbError::Internal("I/O engine already initialized".to_string()))?;
    Ok(())
}

/// Get the global I/O engine instance
pub fn get_io_engine() -> Result<Arc<AsyncIoEngine>> {
    IO_ENGINE
        .get()
        .cloned()
        .ok_or_else(|| DbError::Internal("I/O engine not initialized".to_string()))
}

// ============================================================================
// Configuration
// ============================================================================

/// I/O engine configuration
#[derive(Debug, Clone)]
pub struct IoEngineConfig {
    /// Number of I/O worker threads
    pub worker_threads: usize,

    /// Ring buffer size (must be power of 2)
    pub ring_size: usize,

    /// Buffer pool size
    pub buffer_pool_size: usize,

    /// Enable Direct I/O
    pub direct_io: bool,

    /// Enable async I/O
    pub async_io: bool,

    /// Maximum batch size
    pub max_batch_size: usize,

    /// Enable I/O metrics collection
    pub enable_metrics: bool,

    /// Platform-specific configuration
    pub platform_config: PlatformConfig,
}

impl Default for IoEngineConfig {
    fn default() -> Self {
        Self {
            worker_threads: IO_WORKER_THREADS,
            ring_size: DEFAULT_RING_SIZE,
            buffer_pool_size: BUFFER_POOL_SIZE,
            direct_io: true,
            async_io: true,
            max_batch_size: MAX_BATCH_SIZE,
            enable_metrics: true,
            platform_config: PlatformConfig::default(),
        }
    }
}

/// Platform-specific configuration
#[derive(Debug, Clone)]
pub enum PlatformConfig {
    #[cfg(windows)]
    Windows {
        /// Number of concurrent threads for IOCP
        concurrent_threads: usize,

        /// Enable file buffering
        enable_buffering: bool,
    },

    #[cfg(unix)]
    Unix {
        /// io_uring queue depth
        queue_depth: u32,

        /// Enable SQPOLL mode
        sqpoll: bool,

        /// SQPOLL idle timeout (ms)
        sqpoll_idle_ms: u32,
    },

    /// Fallback configuration
    Fallback,
}

impl Default for PlatformConfig {
    fn default() -> Self {
        #[cfg(windows)]
        return PlatformConfig::Windows {
            concurrent_threads: 0, // 0 = number of processors
            enable_buffering: false,
        };

        #[cfg(unix)]
        return PlatformConfig::Unix {
            queue_depth: 4096,
            sqpoll: false,
            sqpoll_idle_ms: 2000,
        };

        #[cfg(not(any(windows, unix)))]
        return PlatformConfig::Fallback;
    }
}

// ============================================================================
// Utility Functions
// ============================================================================

/// Check if an offset is sector-aligned
#[inline]
pub fn is_sector_aligned(offset: u64) -> bool {
    offset % SECTOR_SIZE as u64 == 0
}

/// Check if a buffer size is sector-aligned
#[inline]
pub fn is_size_aligned(size: usize) -> bool {
    size % SECTOR_SIZE == 0
}

/// Align an offset up to the next sector boundary
#[inline]
pub fn align_up(offset: u64) -> u64 {
    (offset + SECTOR_SIZE as u64 - 1) & !(SECTOR_SIZE as u64 - 1)
}

/// Align an offset down to the previous sector boundary
#[inline]
pub fn align_down(offset: u64) -> u64 {
    offset & !(SECTOR_SIZE as u64 - 1)
}

/// Align a size up to the next sector boundary
#[inline]
pub fn align_size_up(size: usize) -> usize {
    (size + SECTOR_SIZE - 1) & !(SECTOR_SIZE - 1)
}

/// Align a size down to the previous sector boundary
#[inline]
pub fn align_size_down(size: usize) -> usize {
    size & !(SECTOR_SIZE - 1)
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_alignment_checks() {
        assert!(is_sector_aligned(0));
        assert!(is_sector_aligned(4096));
        assert!(is_sector_aligned(8192));
        assert!(!is_sector_aligned(1));
        assert!(!is_sector_aligned(4097));

        assert!(is_size_aligned(0));
        assert!(is_size_aligned(4096));
        assert!(is_size_aligned(8192));
        assert!(!is_size_aligned(1));
        assert!(!is_size_aligned(4097));
    }

    #[test]
    fn test_align_up() {
        assert_eq!(align_up(0), 0);
        assert_eq!(align_up(1), 4096);
        assert_eq!(align_up(4095), 4096);
        assert_eq!(align_up(4096), 4096);
        assert_eq!(align_up(4097), 8192);
    }

    #[test]
    fn test_align_down() {
        assert_eq!(align_down(0), 0);
        assert_eq!(align_down(1), 0);
        assert_eq!(align_down(4095), 0);
        assert_eq!(align_down(4096), 4096);
        assert_eq!(align_down(4097), 4096);
        assert_eq!(align_down(8191), 4096);
        assert_eq!(align_down(8192), 8192);
    }

    #[test]
    fn test_default_config() {
        let config = IoEngineConfig::default();
        assert_eq!(config.worker_threads, IO_WORKER_THREADS);
        assert_eq!(config.ring_size, DEFAULT_RING_SIZE);
        assert_eq!(config.buffer_pool_size, BUFFER_POOL_SIZE);
        assert!(config.direct_io);
        assert!(config.async_io);
    }
}
