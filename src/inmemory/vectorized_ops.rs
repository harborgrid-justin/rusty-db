// SIMD-Accelerated Vectorized Operations
//
// Implements high-performance SIMD operations for columnar data processing:
// - Vectorized filters and predicates
// - Batch aggregations (SUM, AVG, MIN, MAX, COUNT)
// - Cache-conscious algorithms
// - Branch-free conditional operations

use std::arch::x86_64::*;
use crate::inmemory::column_store::ColumnDataType;

/// Cache line size for alignment
pub const CACHE_LINE_SIZE: usize = 64;

/// Vector width for SIMD operations (number of elements)
pub const VECTOR_WIDTH_I64: usize = 4; // 256-bit SIMD / 64-bit elements
pub const VECTOR_WIDTH_I32: usize = 8; // 256-bit SIMD / 32-bit elements

/// Cache-line aligned data structure
#[repr(align(64))]
pub struct CacheLine<T> {
    pub data: T,
}

impl<T> CacheLine<T> {
    pub fn new(data: T) -> Self {
        Self { data }
    }
}

/// Comparison operators for vectorized filters
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ComparisonOp {
    Equal,
    NotEqual,
    LessThan,
    LessThanOrEqual,
    GreaterThan,
    GreaterThanOrEqual,
}

/// Vector mask for SIMD operations
pub struct VectorMask {
    pub mask: Vec<bool>,
    pub count: usize,
}

impl VectorMask {
    pub fn new(size: usize) -> Self {
        Self {
            mask: vec![false; size],
            count: 0,
        }
    }

    pub fn from_vec(mask: Vec<bool>) -> Self {
        let count = mask.iter().filter(|&&x| x).count();
        Self { mask, count }
    }

    pub fn len(&self) -> usize {
        self.mask.len()
    }

    pub fn is_empty(&self) -> bool {
        self.mask.is_empty()
    }

    pub fn get(&self, index: usize) -> bool {
        self.mask.get(index).copied().unwrap_or(false)
    }

    pub fn set(&mut self, index: usize, value: bool) {
        if index < self.mask.len() {
            let old = self.mask[index];
            self.mask[index] = value;
            if value && !old {
                self.count += 1;
            } else if !value && old {
                self.count -= 1;
            }
        }
    }

    pub fn selectivity(&self) -> f64 {
        if self.mask.is_empty() {
            return 0.0;
        }
        self.count as f64 / self.mask.len() as f64
    }
}

/// Batch of vector data for SIMD processing
pub struct VectorBatch {
    pub data: Vec<u8>,
    pub count: usize,
    pub data_type: ColumnDataType,
}

impl VectorBatch {
    pub fn new(capacity: usize, data_type: ColumnDataType) -> Self {
        Self {
            data: Vec::with_capacity(capacity),
            count: 0,
            data_type,
        }
    }

    pub fn from_slice(data: &[u8], count: usize, data_type: ColumnDataType) -> Self {
        Self {
            data: data.to_vec(),
            count,
            data_type,
        }
    }

    pub fn as_i64_slice(&self) -> &[i64] {
        assert_eq!(self.data_type, ColumnDataType::Int64);
        unsafe {
            std::slice::from_raw_parts(
                self.data.as_ptr() as *const i64,
                self.count,
            )
        }
    }

    pub fn as_i32_slice(&self) -> &[i32] {
        assert_eq!(self.data_type, ColumnDataType::Int32);
        unsafe {
            std::slice::from_raw_parts(
                self.data.as_ptr() as *const i32,
                self.count,
            )
        }
    }

    pub fn as_f64_slice(&self) -> &[f64] {
        assert_eq!(self.data_type, ColumnDataType::Float64);
        unsafe {
            std::slice::from_raw_parts(
                self.data.as_ptr() as *const f64,
                self.count,
            )
        }
    }
}

/// SIMD operator trait
pub trait SimdOperator: Send + Sync {
    fn vector_width(&self) -> usize;
    fn supports_type(&self, data_type: ColumnDataType) -> bool;
}

/// Vectorized filter operations
pub struct VectorizedFilter {
    vector_width: usize,
}

impl VectorizedFilter {
    pub fn new(vector_width: usize) -> Self {
        Self { vector_width }
    }

    /// Filter Int64 values using SIMD
    pub fn filter_int64<F>(&self, batch: &VectorBatch, predicate: F) -> Vec<bool>
    where
        F: Fn(i64) -> bool,
    {
        let values = batch.as_i64_slice();
        let mut results = vec![false; values.len()];

        // Process in SIMD-friendly chunks
        let chunks = values.len() / VECTOR_WIDTH_I64;
        let _remainder = values.len() % VECTOR_WIDTH_I64;

        // Vectorized path (simulated - in production would use actual SIMD intrinsics)
        for i in 0..chunks {
            let start = i * VECTOR_WIDTH_I64;
            let end = start + VECTOR_WIDTH_I64;

            for (j, &val) in values[start..end].iter().enumerate() {
                results[start + j] = predicate(val);
            }
        }

        // Handle remainder
        let remainder_start = chunks * VECTOR_WIDTH_I64;
        for (i, &val) in values[remainder_start..].iter().enumerate() {
            results[remainder_start + i] = predicate(val);
        }

        results
    }

    /// Compare Int64 values with SIMD
    pub fn compare_int64(&self, batch: &VectorBatch, op: ComparisonOp, value: i64) -> VectorMask {
        let values = batch.as_i64_slice();
        let mut mask = vec![false; values.len()];

        #[cfg(target_arch = "x86_64")]
        {
            if is_x86_feature_detected!("avx2") {
                return unsafe { self.compare_int64_avx2(values, op, value) };
            }
        }

        // Scalar fallback
        for (i, &v) in values.iter().enumerate() {
            mask[i] = match op {
                ComparisonOp::Equal => v == value,
                ComparisonOp::NotEqual => v != value,
                ComparisonOp::LessThan => v < value,
                ComparisonOp::LessThanOrEqual => v <= value,
                ComparisonOp::GreaterThan => v > value,
                ComparisonOp::GreaterThanOrEqual => v >= value,
            };
        }

        VectorMask::from_vec(mask)
    }

    #[cfg(target_arch = "x86_64")]
    #[target_feature(enable = "avx2")]
    unsafe fn compare_int64_avx2(&self, values: &[i64], op: ComparisonOp, compare_value: i64) -> VectorMask {
        let mut mask = vec![false; values.len()];
        let compare_vec = _mm256_set1_epi64x(compare_value);

        let chunks = values.len() / 4;
        for i in 0..chunks {
            let offset = i * 4;
            let vals = _mm256_loadu_si256(values.as_ptr().add(offset) as *const __m256i);

            let cmp_result = match op {
                ComparisonOp::Equal => _mm256_cmpeq_epi64(vals, compare_vec),
                ComparisonOp::GreaterThan => _mm256_cmpgt_epi64(vals, compare_vec),
                _ => {
                    // Fallback for unsupported ops
                    for j in 0..4 {
                        let v = values[offset + j];
                        mask[offset + j] = match op {
                            ComparisonOp::NotEqual => v != compare_value,
                            ComparisonOp::LessThan => v < compare_value,
                            ComparisonOp::LessThanOrEqual => v <= compare_value,
                            ComparisonOp::GreaterThanOrEqual => v >= compare_value,
                            _ => false,
                        };
                    }
                    continue;
                }
            };

            // Extract mask
            let mask_bits = _mm256_movemask_pd(_mm256_castsi256_pd(cmp_result));
            for j in 0..4 {
                mask[offset + j] = (mask_bits & (1 << j)) != 0;
            }
        }

        // Handle remainder
        for i in (chunks * 4)..values.len() {
            let v = values[i];
            mask[i] = match op {
                ComparisonOp::Equal => v == compare_value,
                ComparisonOp::NotEqual => v != compare_value,
                ComparisonOp::LessThan => v < compare_value,
                ComparisonOp::LessThanOrEqual => v <= compare_value,
                ComparisonOp::GreaterThan => v > compare_value,
                ComparisonOp::GreaterThanOrEqual => v >= compare_value,
            };
        }

        VectorMask::from_vec(mask)
    }

    /// Range filter (between min and max)
    pub fn range_filter_int64(&self, batch: &VectorBatch, min: i64, max: i64) -> VectorMask {
        let values = batch.as_i64_slice();
        let mut mask = vec![false; values.len()];

        for (i, &v) in values.iter().enumerate() {
            mask[i] = v >= min && v <= max;
        }

        VectorMask::from_vec(mask)
    }

    /// IN predicate (value in set)
    pub fn in_filter_int64(&self, batch: &VectorBatch, set: &[i64]) -> VectorMask {
        let values = batch.as_i64_slice();
        let mut mask = vec![false; values.len()];

        // For small sets, use linear search
        // For large sets, would use hash set
        for (i, &v) in values.iter().enumerate() {
            mask[i] = set.contains(&v);
        }

        VectorMask::from_vec(mask)
    }

    /// Combine two masks with AND
    pub fn and_masks(&self, mask1: &VectorMask, mask2: &VectorMask) -> VectorMask {
        assert_eq!(mask1.len(), mask2.len());

        let mut result = vec![false; mask1.len()];
        for i in 0..mask1.len() {
            result[i] = mask1.mask[i] && mask2.mask[i];
        }

        VectorMask::from_vec(result)
    }

    /// Combine two masks with OR
    pub fn or_masks(&self, mask1: &VectorMask, mask2: &VectorMask) -> VectorMask {
        assert_eq!(mask1.len(), mask2.len());

        let mut result = vec![false; mask1.len()];
        for i in 0..mask1.len() {
            result[i] = mask1.mask[i] || mask2.mask[i];
        }

        VectorMask::from_vec(result)
    }

    /// Negate mask
    pub fn not_mask(&self, mask: &VectorMask) -> VectorMask {
        let result: Vec<bool> = mask.mask.iter().map(|&x| !x).collect();
        VectorMask::from_vec(result)
    }
}

impl SimdOperator for VectorizedFilter {
    fn vector_width(&self) -> usize {
        self.vector_width
    }

    fn supports_type(&self, data_type: ColumnDataType) -> bool {
        matches!(
            data_type,
            ColumnDataType::Int8
                | ColumnDataType::Int16
                | ColumnDataType::Int32
                | ColumnDataType::Int64
                | ColumnDataType::Float32
                | ColumnDataType::Float64
        )
    }
}

/// Vectorized aggregation operations
pub struct VectorizedAggregator {
    vector_width: usize,
}

impl VectorizedAggregator {
    pub fn new(vector_width: usize) -> Self {
        Self { vector_width }
    }

    /// Sum Int64 values using SIMD
    pub fn sum_int64(&self, batch: &VectorBatch) -> i64 {
        let values = batch.as_i64_slice();

        #[cfg(target_arch = "x86_64")]
        {
            if is_x86_feature_detected!("avx2") {
                return unsafe { self.sum_int64_avx2(values) };
            }
        }

        // Scalar fallback
        values.iter().sum()
    }

    #[cfg(target_arch = "x86_64")]
    #[target_feature(enable = "avx2")]
    unsafe fn sum_int64_avx2(&self, values: &[i64]) -> i64 {
        let mut sum_vec = _mm256_setzero_si256();

        let chunks = values.len() / 4;
        for i in 0..chunks {
            let vals = _mm256_loadu_si256(values.as_ptr().add(i * 4) as *const __m256i);
            sum_vec = _mm256_add_epi64(sum_vec, vals);
        }

        // Horizontal sum
        let mut sum_array = [0i64; 4];
        _mm256_storeu_si256(sum_array.as_mut_ptr() as *mut __m256i, sum_vec);
        let mut total: i64 = sum_array.iter().sum();

        // Add remainder
        for i in (chunks * 4)..values.len() {
            total += values[i];
        }

        total
    }

    /// Count non-null values
    pub fn count(&self, batch: &VectorBatch, null_mask: Option<&[bool]>) -> usize {
        if let Some(mask) = null_mask {
            mask.iter().filter(|&&is_null| !is_null).count()
        } else {
            batch.count
        }
    }

    /// Min Int64 value
    pub fn min_int64(&self, batch: &VectorBatch) -> Option<i64> {
        let values = batch.as_i64_slice();
        values.iter().copied().min()
    }

    /// Max Int64 value
    pub fn max_int64(&self, batch: &VectorBatch) -> Option<i64> {
        let values = batch.as_i64_slice();
        values.iter().copied().max()
    }

    /// Average of Int64 values
    pub fn avg_int64(&self, batch: &VectorBatch) -> Option<f64> {
        if batch.count == 0 {
            return None;
        }

        let sum = self.sum_int64(batch);
        Some(sum as f64 / batch.count as f64)
    }

    /// Sum Float64 values using SIMD
    pub fn sum_float64(&self, batch: &VectorBatch) -> f64 {
        let values = batch.as_f64_slice();

        #[cfg(target_arch = "x86_64")]
        {
            if is_x86_feature_detected!("avx") {
                return unsafe { self.sum_float64_avx(values) };
            }
        }

        // Scalar fallback
        values.iter().sum()
    }

    #[cfg(target_arch = "x86_64")]
    #[target_feature(enable = "avx")]
    unsafe fn sum_float64_avx(&self, values: &[f64]) -> f64 {
        let mut sum_vec = _mm256_setzero_pd();

        let chunks = values.len() / 4;
        for i in 0..chunks {
            let vals = _mm256_loadu_pd(values.as_ptr().add(i * 4));
            sum_vec = _mm256_add_pd(sum_vec, vals);
        }

        // Horizontal sum
        let mut sum_array = [0.0f64; 4];
        _mm256_storeu_pd(sum_array.as_mut_ptr(), sum_vec);
        let mut total: f64 = sum_array.iter().sum();

        // Add remainder
        for i in (chunks * 4)..values.len() {
            total += values[i];
        }

        total
    }

    /// Conditional sum (SUM with WHERE clause)
    pub fn conditional_sum_int64(&self, batch: &VectorBatch, mask: &VectorMask) -> i64 {
        let values = batch.as_i64_slice();
        let mut sum = 0i64;

        for (i, &val) in values.iter().enumerate() {
            if mask.get(i) {
                sum += val;
            }
        }

        sum
    }

    /// Conditional count
    pub fn conditional_count(&self, mask: &VectorMask) -> usize {
        mask.count
    }

    /// Variance calculation
    pub fn variance_int64(&self, batch: &VectorBatch) -> Option<f64> {
        if batch.count < 2 {
            return None;
        }

        let values = batch.as_i64_slice();
        let mean = self.avg_int64(batch)?;

        let sum_sq_diff: f64 = values
            .iter()
            .map(|&v| {
                let diff = v as f64 - mean;
                diff * diff
            })
            .sum();

        Some(sum_sq_diff / batch.count as f64)
    }

    /// Standard deviation
    pub fn stddev_int64(&self, batch: &VectorBatch) -> Option<f64> {
        self.variance_int64(batch).map(|v| v.sqrt())
    }
}

impl SimdOperator for VectorizedAggregator {
    fn vector_width(&self) -> usize {
        self.vector_width
    }

    fn supports_type(&self, data_type: ColumnDataType) -> bool {
        matches!(
            data_type,
            ColumnDataType::Int8
                | ColumnDataType::Int16
                | ColumnDataType::Int32
                | ColumnDataType::Int64
                | ColumnDataType::Float32
                | ColumnDataType::Float64
        )
    }
}

/// Vectorized gather/scatter operations
pub struct VectorGatherScatter {
    vector_width: usize,
}

impl VectorGatherScatter {
    pub fn new(vector_width: usize) -> Self {
        Self { vector_width }
    }

    /// Gather values at specified indices
    pub fn gather_int64(&self, values: &[i64], indices: &[usize]) -> Vec<i64> {
        let mut result = Vec::with_capacity(indices.len());

        for &idx in indices {
            if idx < values.len() {
                result.push(values[idx]);
            }
        }

        result
    }

    /// Scatter values to specified indices
    pub fn scatter_int64(&self, values: &[i64], indices: &[usize], dest: &mut [i64]) {
        for (i, &idx) in indices.iter().enumerate() {
            if idx < dest.len() && i < values.len() {
                dest[idx] = values[i];
            }
        }
    }

    /// Compress (remove non-selected values)
    pub fn compress_int64(&self, values: &[i64], mask: &VectorMask) -> Vec<i64> {
        let mut result = Vec::with_capacity(mask.count);

        for (i, &val) in values.iter().enumerate() {
            if mask.get(i) {
                result.push(val);
            }
        }

        result
    }

    /// Expand (insert default values for non-selected)
    pub fn expand_int64(&self, values: &[i64], mask: &VectorMask, default: i64) -> Vec<i64> {
        let mut result = vec![default; mask.len()];
        let mut value_idx = 0;

        for i in 0..mask.len() {
            if mask.get(i) && value_idx < values.len() {
                result[i] = values[value_idx];
                value_idx += 1;
            }
        }

        result
    }
}

/// Cache-conscious batch processor
pub struct BatchProcessor {
    batch_size: usize,
    cache_line_size: usize,
}

impl BatchProcessor {
    pub fn new(batch_size: usize, cache_line_size: usize) -> Self {
        Self {
            batch_size,
            cache_line_size,
        }
    }

    /// Process data in cache-friendly batches
    pub fn process_batches<F>(&self, data: &[i64], mut processor: F)
    where
        F: FnMut(&[i64]),
    {
        for chunk in data.chunks(self.batch_size) {
            processor(chunk);
        }
    }

    /// Calculate optimal batch size for cache
    pub fn optimal_batch_size(&self, element_size: usize) -> usize {
        // Try to fit batch in L1 cache (typically 32KB)
        let l1_cache_size = 32 * 1024;
        l1_cache_size / element_size
    }

    /// Prefetch data for next batch
    pub fn prefetch(&self, _data: &[i64], _offset: usize) {
        // In production, would use prefetch intrinsics
        // _mm_prefetch / __builtin_prefetch
    }
}

/// Branch-free selection using masks
pub fn branchfree_select_i64(condition: bool, true_val: i64, false_val: i64) -> i64 {
    // Branch-free conditional selection
    let mask = -(condition as i64);
    (true_val & mask) | (false_val & !mask)
}

/// Parallel column scan
pub fn parallel_scan_int64<F>(values: &[i64], predicate: F, num_threads: usize) -> Vec<bool>
where
    F: Fn(i64) -> bool + Send + Sync + 'static,
{
    use std::sync::Arc;
    use std::thread;

    let predicate = Arc::new(predicate);
    let chunk_size = (values.len() + num_threads - 1) / num_threads;

    let handles: Vec<_> = (0..num_threads)
        .map(|i| {
            let start = i * chunk_size;
            let end = ((i + 1) * chunk_size).min(values.len());
            let chunk: Vec<i64> = values[start..end].to_vec();
            let pred = Arc::clone(&predicate);

            thread::spawn(move || {
                chunk.iter().map(|&v| pred(v)).collect::<Vec<bool>>()
            })
        })
        .collect();

    let mut results = Vec::with_capacity(values.len());
    for handle in handles {
        if let Ok(chunk_results) = handle.join() {
            results.extend(chunk_results);
        }
    }

    results
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vector_mask() {
        let mut mask = VectorMask::new(10);
        assert_eq!(mask.count, 0);

        mask.set(0, true);
        mask.set(5, true);
        assert_eq!(mask.count, 2);
        assert_eq!(mask.selectivity(), 0.2);
    }

    #[test]
    fn test_filter_int64() {
        let filter = VectorizedFilter::new(8);

        let mut data = Vec::new();
        for i in 0..100 {
            data.extend_from_slice(&(i as i64).to_le_bytes());
        }

        let batch = VectorBatch::from_slice(&data, 100, ColumnDataType::Int64);
        let results = filter.filter_int64(&batch, |x| x > 50);

        assert_eq!(results.iter().filter(|&&x| x).count(), 49);
    }

    #[test]
    fn test_compare_int64() {
        let filter = VectorizedFilter::new(8);

        let mut data = Vec::new();
        for i in 0..20 {
            data.extend_from_slice(&(i as i64).to_le_bytes());
        }

        let batch = VectorBatch::from_slice(&data, 20, ColumnDataType::Int64);
        let mask = filter.compare_int64(&batch, ComparisonOp::GreaterThan, 10);

        assert_eq!(mask.count, 9); // 11-19 are greater than 10
    }

    #[test]
    fn test_sum_int64() {
        let agg = VectorizedAggregator::new(8);

        let mut data = Vec::new();
        for i in 1..=10 {
            data.extend_from_slice(&(i as i64).to_le_bytes());
        }

        let batch = VectorBatch::from_slice(&data, 10, ColumnDataType::Int64);
        let sum = agg.sum_int64(&batch);

        assert_eq!(sum, 55); // 1+2+...+10 = 55
    }

    #[test]
    fn test_gather_scatter() {
        let gs = VectorGatherScatter::new(8);

        let values = vec![10, 20, 30, 40, 50];
        let indices = vec![0, 2, 4];

        let gathered = gs.gather_int64(&values, &indices);
        assert_eq!(gathered, vec![10, 30, 50]);

        let mut dest = vec![0; 5];
        gs.scatter_int64(&gathered, &indices, &mut dest);
        assert_eq!(dest[0], 10);
        assert_eq!(dest[2], 30);
        assert_eq!(dest[4], 50);
    }

    #[test]
    fn test_branchfree_select() {
        assert_eq!(branchfree_select_i64(true, 42, 99), 42);
        assert_eq!(branchfree_select_i64(false, 42, 99), 99);
    }
}


