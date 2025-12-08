//! # SIMD-Optimized Scan Engine
//!
//! High-performance vectorized query execution engine using AVX2 SIMD instructions
//! for processing 8-16 elements per instruction with zero-allocation scan loops.
//!
//! ## Architecture
//!
//! The SIMD engine provides vectorized operations for:
//! - **Filter Operations**: Predicate evaluation with SIMD comparisons
//! - **Columnar Scanning**: Sequential and random access with late materialization
//! - **Aggregate Operations**: Vectorized SUM, COUNT, MIN, MAX, AVG
//! - **String Operations**: Vectorized string comparison and pattern matching
//!
//! ## Performance Characteristics
//!
//! - Processes 8-16 elements per SIMD instruction
//! - Zero allocations in the scan loop
//! - Automatic fallback for non-AVX2 CPUs
//! - Cache-oblivious algorithms
//! - Explicit prefetch instructions for sequential access
//!
//! ## Example Usage
//!
//! ```rust,no_run
//! use rusty_db::simd::{ColumnScan, FilterOp, SimdAggregator};
//!
//! # fn example() -> rusty_db::Result<()> {
//! let data: Vec<i32> = vec![1, 2, 3, 4, 5, 6, 7, 8];
//! let mut result = vec![0u8; (data.len() + 7) / 8];
//!
//! // SIMD-accelerated filtering
//! unsafe {
//!     rusty_db::simd::filter::filter_i32_eq_avx2(&data, 5, &mut result);
//! }
//!
//! // Vectorized aggregation
//! let f64_data: Vec<f64> = vec![1.0, 2.0, 3.0, 4.0];
//! let sum = unsafe {
//!     rusty_db::simd::aggregate::sum_f64_avx2(&f64_data)
//! };
//!
//! # Ok(())
//! # }
//! ```
//!
//! ## Safety
//!
//! SIMD operations are inherently `unsafe` as they require:
//! - CPU feature detection (AVX2 support)
//! - Proper memory alignment
//! - Valid pointer arithmetic
//!
//! The engine automatically detects CPU capabilities and falls back to scalar
//! implementations when SIMD is not available.

use crate::error::{DbError, Result};
use crate::common::{Value, Tuple};

/// SIMD filter operations
pub mod filter;

/// Columnar scan operations
pub mod scan;

/// Aggregate operations
pub mod aggregate;

/// String operations
pub mod string;

/// SIMD-accelerated hash functions (xxHash3, wyhash)
pub mod hash;

// Re-export commonly used types
pub use filter::{FilterOp, PredicateType};
pub use scan::{ColumnScan, ScanStrategy, BatchProcessor};
pub use aggregate::{SimdAggregator, AggregateOp};
pub use string::{StringMatcher, PatternType};
pub use hash::{xxhash3_avx2, wyhash, hash_str, hash_str_batch, HashBuilder};

/// CPU feature detection for SIMD capabilities
#[derive(Debug, Clone, Copy)]
pub struct CpuFeatures {
    /// AVX2 support (256-bit SIMD)
    pub avx2: bool,
    /// AVX512 support (512-bit SIMD)
    pub avx512: bool,
    /// SSE4.2 support (128-bit SIMD)
    pub sse42: bool,
}

impl CpuFeatures {
    /// Detect CPU features at runtime
    #[cfg(target_arch = "x86_64")]
    pub fn detect() -> Self {
        Self {
            avx2: is_x86_feature_detected!("avx2"),
            avx512: is_x86_feature_detected!("avx512f"),
            sse42: is_x86_feature_detected!("sse4.2"),
        }
    }

    #[cfg(not(target_arch = "x86_64"))]
    pub fn detect() -> Self {
        Self {
            avx2: false,
            avx512: false,
            sse42: false,
        }
    }

    /// Check if any SIMD support is available
    pub fn has_simd(&self) -> bool {
        self.avx2 || self.avx512 || self.sse42
    }

    /// Get optimal vector width in bytes
    pub fn vector_width(&self) -> usize {
        if self.avx512 {
            64 // 512 bits
        } else if self.avx2 {
            32 // 256 bits
        } else if self.sse42 {
            16 // 128 bits
        } else {
            8 // Scalar fallback
        }
    }

    /// Get optimal number of elements to process per iteration
    pub fn elements_per_iteration<T>(&self) -> usize {
        self.vector_width() / std::mem::size_of::<T>()
    }
}

/// Global CPU feature detection (cached)
static CPU_FEATURES: std::sync::OnceLock<CpuFeatures> = std::sync::OnceLock::new();

/// Get CPU features (cached)
pub fn cpu_features() -> &'static CpuFeatures {
    CPU_FEATURES.get_or_init(CpuFeatures::detect)
}

/// Prefetch hint for sequential access
#[inline(always)]
pub fn prefetch_read<T>(ptr: *const T, ahead: usize) {
    #[cfg(target_arch = "x86_64")]
    unsafe {
        use std::arch::x86_64::_mm_prefetch;
        let prefetch_ptr = ptr.add(ahead) as *const i8;
        _mm_prefetch(prefetch_ptr, std::arch::x86_64::_MM_HINT_T0);
    }

    #[cfg(not(target_arch = "x86_64"))]
    {
        let _ = (ptr, ahead); // Suppress unused warnings
    }
}

/// Prefetch hint for non-temporal access (streaming)
#[inline(always)]
pub fn prefetch_nta<T>(ptr: *const T, ahead: usize) {
    #[cfg(target_arch = "x86_64")]
    unsafe {
        use std::arch::x86_64::_mm_prefetch;
        let prefetch_ptr = ptr.add(ahead) as *const i8;
        _mm_prefetch(prefetch_ptr, std::arch::x86_64::_MM_HINT_NTA);
    }

    #[cfg(not(target_arch = "x86_64"))]
    {
        let _ = (ptr, ahead); // Suppress unused warnings
    }
}

/// Memory alignment utilities
pub mod align {
    /// Check if pointer is aligned to N bytes
    #[inline(always)]
    pub fn is_aligned<T>(ptr: *const T, alignment: usize) -> bool {
        (ptr as usize) % alignment == 0
    }

    /// Align value up to next multiple of alignment
    #[inline(always)]
    pub fn align_up(value: usize, alignment: usize) -> usize {
        (value + alignment - 1) & !(alignment - 1)
    }

    /// Align value down to previous multiple of alignment
    #[inline(always)]
    pub fn align_down(value: usize, alignment: usize) -> usize {
        value & !(alignment - 1)
    }

    /// Get number of elements until next alignment boundary
    #[inline(always)]
    pub fn elements_to_alignment<T>(ptr: *const T, alignment: usize) -> usize {
        let addr = ptr as usize;
        let next_aligned = align_up(addr, alignment);
        (next_aligned - addr) / std::mem::size_of::<T>()
    }
}

/// Batch size for vectorized operations
pub const BATCH_SIZE: usize = 1024;

/// AVX2 vector width for i32 elements
pub const AVX2_I32_WIDTH: usize = 8;

/// AVX2 vector width for i64 elements
pub const AVX2_I64_WIDTH: usize = 4;

/// AVX2 vector width for f32 elements
pub const AVX2_F32_WIDTH: usize = 8;

/// AVX2 vector width for f64 elements
pub const AVX2_F64_WIDTH: usize = 4;

/// Selection vector for late materialization
///
/// Stores row indices that pass filter predicates, enabling late materialization
/// where only matching rows are fully reconstructed.
#[derive(Debug, Clone)]
pub struct SelectionVector {
    /// Row indices that passed filters
    indices: Vec<usize>,
    /// Current position in the vector
    position: usize,
}

impl SelectionVector {
    /// Create new selection vector with capacity
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            indices: Vec::with_capacity(capacity),
            position: 0,
        }
    }

    /// Add an index to the selection vector
    #[inline(always)]
    pub fn add(&mut self, index: usize) {
        self.indices.push(index);
    }

    /// Add multiple indices
    #[inline(always)]
    pub fn add_range(&mut self, start: usize, count: usize) {
        for i in 0..count {
            self.indices.push(start + i);
        }
    }

    /// Clear the selection vector
    pub fn clear(&mut self) {
        self.indices.clear();
        self.position = 0;
    }

    /// Get selected indices
    pub fn indices(&self) -> &[usize] {
        &self.indices
    }

    /// Get number of selected rows
    pub fn len(&self) -> usize {
        self.indices.len()
    }

    /// Check if empty
    pub fn is_empty(&self) -> bool {
        self.indices.is_empty()
    }

    /// Get selectivity ratio
    pub fn selectivity(&self, total_rows: usize) -> f64 {
        if total_rows == 0 {
            0.0
        } else {
            self.len() as f64 / total_rows as f64
        }
    }

    /// Reset position for iteration
    pub fn reset(&mut self) {
        self.position = 0;
    }

    /// Get next batch of indices
    pub fn next_batch(&mut self, batch_size: usize) -> Option<&[usize]> {
        if self.position >= self.indices.len() {
            return None;
        }

        let end = std::cmp::min(self.position + batch_size, self.indices.len());
        let batch = &self.indices[self.position..end];
        self.position = end;
        Some(batch)
    }
}

/// Vectorized comparison result
///
/// Represents the result of a SIMD comparison operation as a bitmask.
#[derive(Debug, Clone, Copy)]
pub struct ComparisonMask {
    /// Bitmask where each bit represents a comparison result
    pub mask: u32,
}

impl ComparisonMask {
    /// Create from raw mask
    #[inline(always)]
    pub fn from_mask(mask: u32) -> Self {
        Self { mask }
    }

    /// Check if all comparisons were true
    #[inline(always)]
    pub fn all(&self) -> bool {
        self.mask == 0xFFFFFFFF
    }

    /// Check if any comparison was true
    #[inline(always)]
    pub fn any(&self) -> bool {
        self.mask != 0
    }

    /// Check if all comparisons were false
    #[inline(always)]
    pub fn none(&self) -> bool {
        self.mask == 0
    }

    /// Count number of true comparisons
    #[inline(always)]
    pub fn count(&self) -> u32 {
        self.mask.count_ones()
    }

    /// Get bit at position
    #[inline(always)]
    pub fn get(&self, index: usize) -> bool {
        (self.mask & (1 << index)) != 0
    }

    /// Combine with AND operation
    #[inline(always)]
    pub fn and(&self, other: ComparisonMask) -> ComparisonMask {
        ComparisonMask {
            mask: self.mask & other.mask,
        }
    }

    /// Combine with OR operation
    #[inline(always)]
    pub fn or(&self, other: ComparisonMask) -> ComparisonMask {
        ComparisonMask {
            mask: self.mask | other.mask,
        }
    }

    /// Negate the mask
    #[inline(always)]
    pub fn not(&self) -> ComparisonMask {
        ComparisonMask {
            mask: !self.mask,
        }
    }
}

/// Statistics for SIMD operations
#[derive(Debug, Clone, Default)]
pub struct SimdStats {
    /// Total rows processed
    pub rows_processed: u64,
    /// Rows that passed filters
    pub rows_selected: u64,
    /// Number of SIMD operations
    pub simd_ops: u64,
    /// Number of scalar operations (fallback)
    pub scalar_ops: u64,
    /// Bytes processed
    pub bytes_processed: u64,
    /// Cache misses (estimated)
    pub cache_misses: u64,
}

impl SimdStats {
    /// Create new statistics
    pub fn new() -> Self {
        Self::default()
    }

    /// Get selectivity ratio
    pub fn selectivity(&self) -> f64 {
        if self.rows_processed == 0 {
            0.0
        } else {
            self.rows_selected as f64 / self.rows_processed as f64
        }
    }

    /// Get SIMD usage ratio
    pub fn simd_ratio(&self) -> f64 {
        let total = self.simd_ops + self.scalar_ops;
        if total == 0 {
            0.0
        } else {
            self.simd_ops as f64 / total as f64
        }
    }

    /// Merge with other statistics
    pub fn merge(&mut self, other: &SimdStats) {
        self.rows_processed += other.rows_processed;
        self.rows_selected += other.rows_selected;
        self.simd_ops += other.simd_ops;
        self.scalar_ops += other.scalar_ops;
        self.bytes_processed += other.bytes_processed;
        self.cache_misses += other.cache_misses;
    }

    /// Reset statistics
    pub fn reset(&mut self) {
        *self = Self::default();
    }
}

/// SIMD execution context
///
/// Provides context and configuration for SIMD operations
pub struct SimdContext {
    /// CPU features
    pub features: CpuFeatures,
    /// Statistics
    pub stats: SimdStats,
    /// Enable prefetching
    pub enable_prefetch: bool,
    /// Prefetch distance (elements ahead)
    pub prefetch_distance: usize,
    /// Batch size for processing
    pub batch_size: usize,
}

impl SimdContext {
    /// Create new SIMD context
    pub fn new() -> Self {
        Self {
            features: CpuFeatures::detect(),
            stats: SimdStats::new(),
            enable_prefetch: true,
            prefetch_distance: 64, // Cache line size
            batch_size: BATCH_SIZE,
        }
    }

    /// Check if AVX2 is available
    pub fn has_avx2(&self) -> bool {
        self.features.avx2
    }

    /// Check if AVX512 is available
    pub fn has_avx512(&self) -> bool {
        self.features.avx512
    }

    /// Get optimal vector width
    pub fn vector_width(&self) -> usize {
        self.features.vector_width()
    }

    /// Update statistics
    pub fn record_simd_op(&mut self, rows: u64) {
        self.stats.simd_ops += 1;
        self.stats.rows_processed += rows;
    }

    /// Update scalar operation count
    pub fn record_scalar_op(&mut self, rows: u64) {
        self.stats.scalar_ops += 1;
        self.stats.rows_processed += rows;
    }

    /// Record selected rows
    pub fn record_selection(&mut self, rows: u64) {
        self.stats.rows_selected += rows;
    }
}

impl Default for SimdContext {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cpu_features() {
        let features = CpuFeatures::detect();
        println!("AVX2: {}", features.avx2);
        println!("AVX512: {}", features.avx512);
        println!("SSE4.2: {}", features.sse42);
        println!("Vector width: {} bytes", features.vector_width());
    }

    #[test]
    fn test_selection_vector() {
        let mut sv = SelectionVector::with_capacity(10);
        sv.add(0);
        sv.add(5);
        sv.add(7);

        assert_eq!(sv.len(), 3);
        assert_eq!(sv.indices(), &[0, 5, 7]);
        assert_eq!(sv.selectivity(10), 0.3);

        sv.clear();
        assert!(sv.is_empty());
    }

    #[test]
    fn test_comparison_mask() {
        let mask1 = ComparisonMask::from_mask(0b11110000);
        let mask2 = ComparisonMask::from_mask(0b11001100);

        assert_eq!(mask1.count(), 4);
        assert!(mask1.any());
        assert!(!mask1.all());

        let and_mask = mask1.and(mask2);
        assert_eq!(and_mask.mask, 0b11000000);

        let or_mask = mask1.or(mask2);
        assert_eq!(or_mask.mask, 0b11111100);
    }

    #[test]
    fn test_alignment() {
        use align::*;

        assert_eq!(align_up(10, 16), 16);
        assert_eq!(align_up(16, 16), 16);
        assert_eq!(align_up(17, 16), 32);

        assert_eq!(align_down(10, 16), 0);
        assert_eq!(align_down(16, 16), 16);
        assert_eq!(align_down(17, 16), 16);
    }

    #[test]
    fn test_simd_stats() {
        let mut stats = SimdStats::new();
        stats.rows_processed = 1000;
        stats.rows_selected = 250;
        stats.simd_ops = 80;
        stats.scalar_ops = 20;

        assert_eq!(stats.selectivity(), 0.25);
        assert_eq!(stats.simd_ratio(), 0.8);
    }

    #[test]
    fn test_simd_context() {
        let ctx = SimdContext::new();
        println!("Has AVX2: {}", ctx.has_avx2());
        println!("Has AVX512: {}", ctx.has_avx512());
        println!("Vector width: {}", ctx.vector_width());
    }
}


