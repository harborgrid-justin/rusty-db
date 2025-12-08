// Example: Integrating Memory Hardening with Buffer Pool
//
// This example demonstrates how to integrate the military-grade memory hardening
// features with rusty-db's buffer pool and allocators.

use rusty_db::security::memory_hardening::*;
use rusty_db::buffer::page_cache::{PageBuffer, BufferFrame};
use std::sync::Arc;

// ============================================================================
// Example 1: Secure Buffer Pool Page with Overflow Protection
// ============================================================================

/// Wrapper around PageBuffer that adds overflow protection
pub struct SecurePageBuffer {
    /// Underlying secure buffer
    buffer: SecureBuffer<u8>,
    /// Page ID for tracking
    page_id: u64,
}

impl SecurePageBuffer {
    /// Create a new secure page buffer
    pub fn new(page_id: u64) -> rusty_db::Result<Self> {
        Ok(Self {
            buffer: SecureBuffer::new(4096)?,
            page_id,
        })
    }

    /// Write data with bounds checking and canary protection
    pub fn write_data(&mut self, offset: usize, data: &[u8]) -> rusty_db::Result<()> {
        // Automatic bounds checking
        self.buffer.write(offset, data)?;

        // Canary values are automatically checked
        self.buffer.verify_canaries()?;

        Ok(())
    }

    /// Read data with security verification
    pub fn read_data(&self, offset: usize, len: usize) -> rusty_db::Result<Vec<u8>> {
        // Verify canaries before read
        self.buffer.verify_canaries()?;

        // Read with bounds checking
        self.buffer.read(offset, len)
    }

    /// Get page ID
    pub fn page_id(&self) -> u64 {
        self.page_id
    }
}

// When dropped, SecurePageBuffer automatically:
// 1. Zeroes all memory (prevents data leakage)
// 2. Verifies canaries one final time
// 3. Releases guard pages

// ============================================================================
// Example 2: Secure Allocator for Buffer Pool
// ============================================================================

/// Secure allocator wrapper for buffer pool frames
pub struct SecureBufferPoolAllocator {
    /// Underlying secure zeroing allocator
    allocator: SecureZeroingAllocator,
    /// Configuration
    config: MemoryHardeningConfig,
}

impl SecureBufferPoolAllocator {
    /// Create new secure allocator
    pub fn new() -> Self {
        let config = MemoryHardeningConfig {
            enable_guard_pages: true,
            enable_canaries: true,
            enable_zeroing: true,
            enable_double_free_detection: true,
            enable_encryption: false, // Optional for performance
            ..Default::default()
        };

        Self {
            allocator: SecureZeroingAllocator::with_config(config.clone()),
            config,
        }
    }

    /// Allocate a frame with security features
    pub fn allocate_frame(&self, size: usize) -> rusty_db::Result<std::ptr::NonNull<u8>> {
        // Allocate with automatic:
        // - Random noise fill (prevent info leakage)
        // - Metadata tracking (double-free detection)
        // - Statistics collection
        self.allocator.allocate(size)
    }

    /// Deallocate frame with secure zeroing
    pub fn deallocate_frame(&self, ptr: std::ptr::NonNull<u8>, size: usize) -> rusty_db::Result<()> {
        // Automatic:
        // - Volatile zero writes (3 passes)
        // - Double-free detection
        // - Statistics update
        self.allocator.deallocate(ptr, size)
    }

    /// Get security statistics
    pub fn get_stats(&self) -> AllocatorStatsSnapshot {
        self.allocator.stats()
    }
}

// ============================================================================
// Example 3: Isolated Heap for Sensitive Data
// ============================================================================

/// Example of using isolated heap for encryption keys and passwords
pub fn secure_key_storage_example() -> rusty_db::Result<()> {
    // Create isolated heap (2MB for sensitive data)
    let mut sensitive_heap = IsolatedHeap::new(2 * 1024 * 1024)?;

    // Allocate space for encryption key (32 bytes)
    let key_ptr = sensitive_heap.allocate(32)?;

    // Write key data (in production, this would be actual key material)
    let key_data = vec![0xAB; 32];
    unsafe {
        std::ptr::copy_nonoverlapping(
            key_data.as_ptr(),
            key_ptr.as_ptr(),
            32
        );
    }

    // Encrypt the key in memory
    sensitive_heap.encrypt_region(0, 32)?;

    // Later, decrypt to use
    sensitive_heap.decrypt_region(0, 32)?;

    // When heap is dropped:
    // - Entire heap is zeroed
    // - Memory is securely deallocated

    Ok(())
}

// ============================================================================
// Example 4: Integration with Existing Buffer Frame
// ============================================================================

/// Enhanced buffer frame with memory hardening
pub struct HardenedBufferFrame {
    /// Frame ID
    frame_id: u32,
    /// Secure page buffer
    secure_buffer: SecurePageBuffer,
    /// Memory canary for frame metadata
    metadata_canary: MemoryCanary,
}

impl HardenedBufferFrame {
    /// Create new hardened buffer frame
    pub fn new(frame_id: u32) -> rusty_db::Result<Self> {
        let address = frame_id as usize;
        Ok(Self {
            frame_id,
            secure_buffer: SecurePageBuffer::new(frame_id as u64)?,
            metadata_canary: MemoryCanary::new(address),
        })
    }

    /// Verify frame integrity
    pub fn verify_integrity(&self) -> rusty_db::Result<()> {
        // Check buffer canaries
        self.secure_buffer.buffer.verify_canaries()?;

        // Check metadata canary
        let stored_canary = self.metadata_canary.encode();
        if self.metadata_canary.is_corrupted(stored_canary) {
            return Err(rusty_db::error::DbError::Other(
                "Frame metadata corrupted!".into()
            ));
        }

        Ok(())
    }

    /// Write to frame with full security checks
    pub fn write(&mut self, offset: usize, data: &[u8]) -> rusty_db::Result<()> {
        // Verify integrity before write
        self.verify_integrity()?;

        // Write with bounds checking
        self.secure_buffer.write_data(offset, data)?;

        // Verify integrity after write
        self.verify_integrity()?;

        Ok(())
    }

    /// Read from frame with security verification
    pub fn read(&self, offset: usize, len: usize) -> rusty_db::Result<Vec<u8>> {
        // Verify integrity before read
        self.verify_integrity()?;

        // Read with bounds checking
        self.secure_buffer.read_data(offset, len)
    }
}

// ============================================================================
// Example 5: Buffer Pool with Memory Hardening
// ============================================================================

pub struct HardenedBufferPool {
    /// Secure allocator
    allocator: Arc<SecureBufferPoolAllocator>,
    /// Isolated heap for sensitive metadata
    sensitive_heap: IsolatedHeap,
    /// Frames
    frames: Vec<HardenedBufferFrame>,
    /// Security metrics
    metrics: SecurityMetrics,
}

impl HardenedBufferPool {
    /// Create new hardened buffer pool
    pub fn new(num_frames: usize) -> rusty_db::Result<Self> {
        let allocator = Arc::new(SecureBufferPoolAllocator::new());
        let sensitive_heap = IsolatedHeap::new(1024 * 1024)?; // 1MB for metadata

        // Create frames with security features
        let mut frames = Vec::with_capacity(num_frames);
        for i in 0..num_frames {
            frames.push(HardenedBufferFrame::new(i as u32)?);
        }

        Ok(Self {
            allocator,
            sensitive_heap,
            frames,
            metrics: SecurityMetrics::default(),
        })
    }

    /// Get frame with security verification
    pub fn get_frame(&mut self, frame_id: usize) -> rusty_db::Result<&mut HardenedBufferFrame> {
        if frame_id >= self.frames.len() {
            return Err(rusty_db::error::DbError::Other(
                "Frame ID out of bounds".into()
            ));
        }

        let frame = &mut self.frames[frame_id];

        // Verify frame integrity
        frame.verify_integrity()?;

        Ok(frame)
    }

    /// Get security metrics
    pub fn security_metrics(&self) -> &SecurityMetrics {
        &self.metrics
    }

    /// Verify integrity of entire pool
    pub fn verify_pool_integrity(&self) -> rusty_db::Result<()> {
        for frame in &self.frames {
            frame.verify_integrity()?;
        }
        Ok(())
    }
}

// ============================================================================
// Example 6: Configuration Profiles
// ============================================================================

/// Development configuration (balanced security/performance)
pub fn dev_config() -> MemoryHardeningConfig {
    MemoryHardeningConfig {
        enable_guard_pages: true,
        enable_canaries: true,
        enable_zeroing: true,
        enable_double_free_detection: true,
        enable_encryption: false,
        enable_isolated_heap: false,
        canary_check_frequency: CanaryCheckFrequency::Periodic,
        guard_page_size: PAGE_SIZE,
        ..Default::default()
    }
}

/// Production configuration (maximum security)
pub fn production_config() -> MemoryHardeningConfig {
    MemoryHardeningConfig {
        enable_guard_pages: true,
        enable_canaries: true,
        enable_zeroing: true,
        enable_double_free_detection: true,
        enable_encryption: true,
        enable_isolated_heap: true,
        enable_quarantine: true,
        canary_check_frequency: CanaryCheckFrequency::Always,
        guard_page_size: PAGE_SIZE * 2,
        quarantine_duration: std::time::Duration::from_secs(3600),
        enable_bounds_checking: true,
        enable_access_logging: false,
    }
}

/// Test/Debugging configuration (all features enabled)
pub fn debug_config() -> MemoryHardeningConfig {
    MemoryHardeningConfig {
        enable_guard_pages: true,
        enable_canaries: true,
        enable_zeroing: true,
        enable_double_free_detection: true,
        enable_encryption: true,
        enable_isolated_heap: true,
        enable_quarantine: true,
        canary_check_frequency: CanaryCheckFrequency::Always,
        guard_page_size: PAGE_SIZE * 4,
        quarantine_duration: std::time::Duration::from_secs(86400),
        enable_bounds_checking: true,
        enable_access_logging: true,
    }
}

// ============================================================================
// Usage Example
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_secure_page_buffer() {
        let mut buffer = SecurePageBuffer::new(1).unwrap();

        // Write data
        let data = vec![1, 2, 3, 4];
        buffer.write_data(0, &data).unwrap();

        // Read data
        let read_data = buffer.read_data(0, 4).unwrap();
        assert_eq!(data, read_data);

        // Verify canaries are intact
        buffer.buffer.verify_canaries().unwrap();
    }

    #[test]
    fn test_overflow_detection() {
        let mut buffer = SecurePageBuffer::new(1).unwrap();

        // Attempt to write beyond bounds
        let too_much_data = vec![0u8; 5000]; // Page is only 4096 bytes
        let result = buffer.write_data(0, &too_much_data);

        // Should fail with overflow error
        assert!(result.is_err());
    }

    #[test]
    fn test_secure_allocator() {
        let allocator = SecureBufferPoolAllocator::new();

        // Allocate memory
        let ptr = allocator.allocate_frame(1024).unwrap();

        // Check stats
        let stats = allocator.get_stats();
        assert_eq!(stats.active_allocations, 1);

        // Deallocate
        allocator.deallocate_frame(ptr, 1024).unwrap();

        // Verify deallocation
        let stats = allocator.get_stats();
        assert_eq!(stats.active_allocations, 0);
    }

    #[test]
    fn test_double_free_detection() {
        let allocator = SecureBufferPoolAllocator::new();
        let ptr = allocator.allocate_frame(256).unwrap();

        // First free should succeed
        allocator.deallocate_frame(ptr, 256).unwrap();

        // Second free should fail
        let result = allocator.deallocate_frame(ptr, 256);
        assert!(result.is_err());

        // Check that double-free was detected
        let stats = allocator.get_stats();
        assert_eq!(stats.double_free_detected, 1);
    }

    #[test]
    fn test_hardened_buffer_pool() {
        let mut pool = HardenedBufferPool::new(10).unwrap();

        // Get frame
        let frame = pool.get_frame(0).unwrap();

        // Write data
        let data = vec![42u8; 100];
        frame.write(0, &data).unwrap();

        // Read data
        let read_data = frame.read(0, 100).unwrap();
        assert_eq!(data, read_data);

        // Verify pool integrity
        pool.verify_pool_integrity().unwrap();
    }
}

// ============================================================================
// Performance Benchmarks
// ============================================================================

#[cfg(test)]
mod benchmarks {
    use super::*;
    use std::time::Instant;

    #[test]
    #[ignore] // Run with: cargo test --release -- --ignored
    fn benchmark_secure_buffer_write() {
        let mut buffer = SecurePageBuffer::new(1).unwrap();
        let data = vec![42u8; 1024];

        let start = Instant::now();
        for _ in 0..10000 {
            buffer.write_data(0, &data).unwrap();
        }
        let elapsed = start.elapsed();

        println!("Secure buffer write (10k iterations): {:?}", elapsed);
        println!("Average per write: {:?}", elapsed / 10000);
    }

    #[test]
    #[ignore]
    fn benchmark_allocator_throughput() {
        let allocator = SecureBufferPoolAllocator::new();

        let start = Instant::now();
        let mut ptrs = Vec::new();

        // Allocate 1000 times
        for _ in 0..1000 {
            ptrs.push(allocator.allocate_frame(4096).unwrap());
        }

        // Deallocate all
        for ptr in ptrs {
            allocator.deallocate_frame(ptr, 4096).unwrap();
        }

        let elapsed = start.elapsed();
        println!("Allocator throughput (1000 alloc/dealloc): {:?}", elapsed);
    }
}
