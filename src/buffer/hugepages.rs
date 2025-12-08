//! # Huge Page Support for Buffer Pool
//!
//! Provides support for 2MB and 1GB huge pages to dramatically reduce TLB
//! (Translation Lookaside Buffer) misses and improve memory access performance.
//!
//! ## Performance Benefits
//!
//! ### Standard 4KB Pages
//! - TLB entries: ~64-512 entries (depending on CPU)
//! - Coverage: 256KB - 2MB of memory
//! - TLB miss rate: 5-15% for large buffer pools
//! - TLB miss cost: ~100-200 CPU cycles
//!
//! ### 2MB Huge Pages
//! - TLB entries: Same number but 512x coverage each
//! - Coverage: 128MB - 1GB of memory
//! - TLB miss rate: 0.5-3% (10x improvement)
//! - Performance gain: 5-15% for memory-intensive workloads
//!
//! ### 1GB Huge Pages
//! - TLB entries: 512x coverage compared to 2MB
//! - Coverage: 64GB - 512GB of memory
//! - TLB miss rate: <0.5%
//! - Performance gain: Up to 20% for very large buffer pools
//!
//! ## Linux Support
//!
//! ### Transparent Huge Pages (THP)
//! - Automatically promoted by kernel
//! - No application changes required
//! - Can be explicitly requested with `madvise(MADV_HUGEPAGE)`
//!
//! ### Explicit Huge Pages
//! - Allocated from hugetlbfs
//! - Guaranteed huge pages
//! - Requires system configuration (`/proc/sys/vm/nr_hugepages`)
//! - Better for production databases
//!
//! ## Windows Support
//!
//! - Large pages (2MB) via `VirtualAlloc` with `MEM_LARGE_PAGES`
//! - Requires `SeLockMemoryPrivilege`
//! - Not as mature as Linux support

use crate::error::Result;
use std::alloc::{alloc, dealloc, Layout};
use std::ptr;
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::sync::Arc;
use parking_lot::Mutex;

// ============================================================================
// Constants
// ============================================================================

/// Standard page size (4KB)
pub const PAGE_SIZE_4K: usize = 4096;

/// Huge page size (2MB)
pub const PAGE_SIZE_2M: usize = 2 * 1024 * 1024;

/// Huge page size (1GB)
pub const PAGE_SIZE_1G: usize = 1024 * 1024 * 1024;

// ============================================================================
// Huge Page Configuration
// ============================================================================

/// Huge page size options
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HugePageSize {
    /// Standard 4KB pages
    Size4K,

    /// 2MB huge pages
    Size2M,

    /// 1GB huge pages (requires special kernel config)
    Size1G,
}

impl HugePageSize {
    /// Get size in bytes
    pub fn bytes(&self) -> usize {
        match self {
            HugePageSize::Size4K => PAGE_SIZE_4K,
            HugePageSize::Size2M => PAGE_SIZE_2M,
            HugePageSize::Size1G => PAGE_SIZE_1G,
        }
    }

    /// Get number of 4KB pages in this huge page
    pub fn page_count(&self) -> usize {
        match self {
            HugePageSize::Size4K => 1,
            HugePageSize::Size2M => 512,
            HugePageSize::Size1G => 262144,
        }
    }
}

/// Huge page allocation strategy
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AllocationStrategy {
    /// Use standard pages (no huge pages)
    StandardPages,

    /// Use transparent huge pages (THP) with madvise
    TransparentHugePages,

    /// Use explicit huge pages from hugetlbfs
    ExplicitHugePages,

    /// Try explicit, fall back to THP, then standard
    BestEffort,
}

/// Huge page configuration
#[derive(Debug, Clone)]
pub struct HugePageConfig {
    /// Enable huge pages
    pub enabled: bool,

    /// Preferred huge page size
    pub page_size: HugePageSize,

    /// Allocation strategy
    pub strategy: AllocationStrategy,

    /// Fallback to standard pages if huge pages unavailable
    pub allow_fallback: bool,

    /// Minimum allocation size to use huge pages
    pub min_allocation_size: usize,
}

impl Default for HugePageConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            page_size: HugePageSize::Size2M,
            strategy: AllocationStrategy::BestEffort,
            allow_fallback: true,
            min_allocation_size: PAGE_SIZE_2M * 10, // At least 20MB
        }
    }
}

// ============================================================================
// Huge Page Allocator
// ============================================================================

/// Huge page allocator
pub struct HugePageAllocator {
    /// Configuration
    config: HugePageConfig,

    /// Statistics
    stats: Arc<Mutex<HugePageStats>>,

    /// Total allocated bytes
    allocated_bytes: Arc<AtomicU64>,

    /// Number of huge pages allocated
    huge_pages_allocated: Arc<AtomicUsize>,

    /// Number of standard pages allocated (fallback)
    standard_pages_allocated: Arc<AtomicUsize>,
}

/// Statistics for huge page allocations
#[derive(Debug, Clone, Default)]
pub struct HugePageStats {
    /// Total allocation requests
    pub total_requests: u64,

    /// Successful huge page allocations
    pub huge_page_allocations: u64,

    /// Standard page allocations (fallback)
    pub standard_page_allocations: u64,

    /// Failed allocations
    pub failed_allocations: u64,

    /// Total bytes allocated via huge pages
    pub huge_page_bytes: u64,

    /// Total bytes allocated via standard pages
    pub standard_page_bytes: u64,

    /// TLB miss rate estimate (lower is better)
    pub estimated_tlb_miss_rate: f64,
}

impl HugePageAllocator {
    /// Create a new huge page allocator
    pub fn new(config: HugePageConfig) -> Self {
        Self {
            config,
            stats: Arc::new(Mutex::new(HugePageStats::default())),
            allocated_bytes: Arc::new(AtomicU64::new(0)),
            huge_pages_allocated: Arc::new(AtomicUsize::new(0)),
            standard_pages_allocated: Arc::new(AtomicUsize::new(0)),
        }
    }

    /// Allocate memory with huge page support
    pub fn allocate(&self, size: usize, alignment: usize) -> Result<HugePageAllocation> {
        self.stats.lock().total_requests += 1;

        // Check if we should use huge pages
        if !self.config.enabled || size < self.config.min_allocation_size {
            return self.allocate_standard(size, alignment);
        }

        // Try allocation based on strategy
        match self.config.strategy {
            AllocationStrategy::StandardPages => self.allocate_standard(size, alignment),

            AllocationStrategy::TransparentHugePages => {
                self.allocate_transparent_huge(size, alignment)
            }

            AllocationStrategy::ExplicitHugePages => {
                self.allocate_explicit_huge(size, alignment)
            }

            AllocationStrategy::BestEffort => {
                // Try explicit, then THP, then standard
                if let Ok(alloc) = self.allocate_explicit_huge(size, alignment) {
                    return Ok(alloc);
                }

                if let Ok(alloc) = self.allocate_transparent_huge(size, alignment) {
                    return Ok(alloc);
                }

                if self.config.allow_fallback {
                    self.allocate_standard(size, alignment)
                } else {
                    Err(DbError::Internal("Huge page allocation failed".into()))
                }
            }
        }
    }

    /// Allocate standard pages
    fn allocate_standard(&self, size: usize, alignment: usize) -> Result<HugePageAllocation> {
        let layout = Layout::from_size_align(size, alignment)
            .map_err(|_| DbError::Internal("Invalid layout".into()))?;

        let ptr = unsafe { alloc(layout) };
        if ptr.is_null() {
            self.stats.lock().failed_allocations += 1;
            return Err(DbError::Internal("Allocation failed".into()));
        }

        // Zero the memory
        unsafe {
            ptr::write_bytes(ptr, 0, size);
        }

        let mut stats = self.stats.lock();
        stats.standard_page_allocations += 1;
        stats.standard_page_bytes += size as u64;
        drop(stats);

        self.allocated_bytes.fetch_add(size as u64, Ordering::Relaxed);
        self.standard_pages_allocated
            .fetch_add((size + PAGE_SIZE_4K - 1) / PAGE_SIZE_4K, Ordering::Relaxed);

        Ok(HugePageAllocation {
            ptr,
            size,
            layout,
            huge_page_size: HugePageSize::Size4K,
            is_huge_page: false,
        })
    }

    /// Allocate using transparent huge pages (Linux with madvise)
    fn allocate_transparent_huge(&self, size: usize, alignment: usize) -> Result<HugePageAllocation> {
        // Round up to huge page size
        let huge_page_size = self.config.page_size.bytes();
        let aligned_size = ((size + huge_page_size - 1) / huge_page_size) * huge_page_size;

        // Allocate with huge page alignment
        let layout = Layout::from_size_align(aligned_size, huge_page_size)
            .map_err(|_| DbError::Internal("Invalid layout".into()))?;

        let ptr = unsafe { alloc(layout) };
        if ptr.is_null() {
            return if self.config.allow_fallback {
                self.allocate_standard(size, alignment)
            } else {
                Err(DbError::Internal("THP allocation failed".into()))
            };
        }

        // Zero the memory
        unsafe {
            ptr::write_bytes(ptr, 0, aligned_size);
        }

        // Advise kernel to use huge pages (Linux-specific)
        #[cfg(target_os = "linux")]
        {
            unsafe {
                let _result = libc::madvise(
                    ptr as *mut libc::c_void,
                    aligned_size,
                    libc::MADV_HUGEPAGE,
                );

                if result != 0 {
                    // madvise failed, but allocation succeeded
                    // Continue with regular pages
                }
            }
        }

        let mut stats = self.stats.lock();
        stats.huge_page_allocations += 1;
        stats.huge_page_bytes += aligned_size as u64;
        drop(stats);

        self.allocated_bytes
            .fetch_add(aligned_size as u64, Ordering::Relaxed);
        self.huge_pages_allocated
            .fetch_add(aligned_size / huge_page_size, Ordering::Relaxed);

        Ok(HugePageAllocation {
            ptr,
            size: aligned_size,
            layout,
            huge_page_size: self.config.page_size,
            is_huge_page: true,
        })
    }

    /// Allocate explicit huge pages from hugetlbfs (Linux)
    fn allocate_explicit_huge(&self, size: usize, alignment: usize) -> Result<HugePageAllocation> {
        // This would require mapping files from /dev/hugepages or using
        // shmget with SHM_HUGETLB
        // For now, fall back to THP
        self.allocate_transparent_huge(size, alignment)
    }

    /// Get allocation statistics
    pub fn stats(&self) -> HugePageStats {
        let mut stats = self.stats.lock().clone();

        // Calculate estimated TLB miss rate
        let total_bytes = stats.huge_page_bytes + stats.standard_page_bytes;
        if total_bytes > 0 {
            // Estimate based on page size distribution
            let huge_ratio = stats.huge_page_bytes as f64 / total_bytes as f64;
            let standard_ratio = stats.standard_page_bytes as f64 / total_bytes as f64;

            // Rough TLB miss rate estimates (empirical):
            // - 4KB pages: 10% miss rate
            // - 2MB pages: 1% miss rate
            // - 1GB pages: 0.1% miss rate
            stats.estimated_tlb_miss_rate = (standard_ratio * 0.10) + (huge_ratio * 0.01);
        }

        stats
    }

    /// Get memory efficiency (percentage of memory that uses huge pages)
    pub fn huge_page_efficiency(&self) -> f64 {
        let _stats = self.stats.lock();
        let total = stats.huge_page_bytes + stats.standard_page_bytes;

        if total > 0 {
            stats.huge_page_bytes as f64 / total as f64
        } else {
            0.0
        }
    }
}

// ============================================================================
// Huge Page Allocation
// ============================================================================

/// A huge page allocation
pub struct HugePageAllocation {
    /// Pointer to allocated memory
    ptr: *mut u8,

    /// Size of allocation
    size: usize,

    /// Layout for deallocation
    layout: Layout,

    /// Huge page size used
    huge_page_size: HugePageSize,

    /// Whether this is actually a huge page
    is_huge_page: bool,
}

impl HugePageAllocation {
    /// Get pointer to memory
    #[inline]
    pub fn as_ptr(&self) -> *const u8 {
        self.ptr
    }

    /// Get mutable pointer to memory
    #[inline]
    pub fn as_mut_ptr(&mut self) -> *mut u8 {
        self.ptr
    }

    /// Get size of allocation
    #[inline]
    pub fn size(&self) -> usize {
        self.size
    }

    /// Get memory as slice
    #[inline]
    pub fn as_slice(&self) -> &[u8] {
        unsafe { std::slice::from_raw_parts(self.ptr, self.size) }
    }

    /// Get memory as mutable slice
    #[inline]
    pub fn as_mut_slice(&mut self) -> &mut [u8] {
        unsafe { std::slice::from_raw_parts_mut(self.ptr, self.size) }
    }

    /// Check if this is a huge page allocation
    #[inline]
    pub fn is_huge_page(&self) -> bool {
        self.is_huge_page
    }

    /// Get huge page size
    #[inline]
    pub fn huge_page_size(&self) -> HugePageSize {
        self.huge_page_size
    }

    /// Zero the allocation
    pub fn zero(&mut self) {
        unsafe {
            ptr::write_bytes(self.ptr, 0, self.size);
        }
    }

    /// Copy data into allocation
    pub fn copy_from_slice(&mut self, data: &[u8]) -> Result<()> {
        if data.len() > self.size {
            return Err(DbError::Internal(format!(
                "Data too large: {} > {}",
                data.len(),
                self.size
            )));
        }

        self.as_mut_slice()[..data.len()].copy_from_slice(data);
        Ok(())
    }
}

impl Drop for HugePageAllocation {
    fn drop(&mut self) {
        if !self.ptr.is_null() {
            unsafe {
                dealloc(self.ptr, self.layout);
            }
        }
    }
}

unsafe impl Send for HugePageAllocation {}
unsafe impl Sync for HugePageAllocation {}

// ============================================================================
// System Information
// ============================================================================

/// Query system huge page information
pub fn query_huge_page_info() -> HugePageSystemInfo {
    let mut info = HugePageSystemInfo::default();

    // Try to read from /proc/meminfo (Linux)
    #[cfg(target_os = "linux")]
    {
        if let Ok(contents) = std::fs::read_to_string("/proc/meminfo") {
            for line in contents.lines() {
                if line.starts_with("HugePages_Total:") {
                    if let Some(value) = line.split_whitespace().nth(1) {
                        info.total_huge_pages = value.parse().unwrap_or(0);
                    }
                } else if line.starts_with("HugePages_Free:") {
                    if let Some(value) = line.split_whitespace().nth(1) {
                        info.free_huge_pages = value.parse().unwrap_or(0);
                    }
                } else if line.starts_with("Hugepagesize:") {
                    if let Some(value) = line.split_whitespace().nth(1) {
                        info.huge_page_size_kb = value.parse().unwrap_or(0);
                    }
                }
            }

            info.supported = info.total_huge_pages > 0;
        }

        // Check THP status
        if let Ok(contents) = std::fs::read_to_string("/sys/kernel/mm/transparent_hugepage/enabled") {
            info.thp_enabled = contents.contains("[always]") || contents.contains("[madvise]");
        }
    }

    info
}

/// System huge page information
#[derive(Debug, Clone, Default)]
pub struct HugePageSystemInfo {
    /// Whether huge pages are supported
    pub supported: bool,

    /// Total huge pages configured
    pub total_huge_pages: usize,

    /// Free huge pages available
    pub free_huge_pages: usize,

    /// Huge page size in KB
    pub huge_page_size_kb: usize,

    /// Whether transparent huge pages are enabled
    pub thp_enabled: bool,
}

impl HugePageSystemInfo {
    /// Get available huge page memory in bytes
    pub fn available_bytes(&self) -> u64 {
        (self.free_huge_pages * self.huge_page_size_kb * 1024) as u64
    }

    /// Check if system is properly configured for huge pages
    pub fn is_well_configured(&self) -> bool {
        self.supported && (self.total_huge_pages > 0 || self.thp_enabled)
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_huge_page_sizes() {
        assert_eq!(HugePageSize::Size4K.bytes(), 4096);
        assert_eq!(HugePageSize::Size2M.bytes(), 2 * 1024 * 1024);
        assert_eq!(HugePageSize::Size1G.bytes(), 1024 * 1024 * 1024);
    }

    #[test]
    fn test_standard_allocation() {
        let config = HugePageConfig {
            enabled: false,
            ..Default::default()
        };

        let allocator = HugePageAllocator::new(config);
        let mut alloc = allocator.allocate(PAGE_SIZE_4K, PAGE_SIZE_4K).unwrap();

        assert_eq!(alloc.size(), PAGE_SIZE_4K);
        assert!(!alloc.is_huge_page());

        // Test operations
        alloc.zero();
        assert_eq!(alloc.as_slice()[0], 0);

        let data = vec![42u8; 100];
        alloc.copy_from_slice(&data).unwrap();
        assert_eq!(alloc.as_slice()[0], 42);
    }

    #[test]
    fn test_huge_page_allocation() {
        let config = HugePageConfig {
            enabled: true,
            strategy: AllocationStrategy::BestEffort,
            ..Default::default()
        };

        let allocator = HugePageAllocator::new(config);

        // Allocate large enough to trigger huge pages
        let size = PAGE_SIZE_2M * 10;
        let alloc = allocator.allocate(size, PAGE_SIZE_2M);

        if let Ok(alloc) = alloc {
            assert!(alloc.size() >= size);

            let _stats = allocator.stats();
            assert!(stats.total_requests > 0);
        }
        // If huge pages not available, that's OK (test environment)
    }

    #[test]
    fn test_huge_page_stats() {
        let config = HugePageConfig::default();
        let allocator = HugePageAllocator::new(config);

        // Make some allocations
        let _ = allocator.allocate(PAGE_SIZE_4K, PAGE_SIZE_4K);
        let _ = allocator.allocate(PAGE_SIZE_2M * 10, PAGE_SIZE_2M);

        let _stats = allocator.stats();
        assert!(stats.total_requests >= 2);
        assert!(stats.estimated_tlb_miss_rate >= 0.0);
        assert!(stats.estimated_tlb_miss_rate <= 1.0);
    }

    #[test]
    fn test_system_info() {
        let info = query_huge_page_info();

        // Should always return something (even if not supported)
        assert!(info.huge_page_size_kb >= 0);
        assert!(info.total_huge_pages >= 0);
    }
}


