// # SIMD Aggregate Operations
//
// Vectorized aggregate functions (SUM, COUNT, MIN, MAX, AVG) using AVX2 SIMD instructions.

use super::{SimdContext, SimdStats};
use crate::common::Value;
use crate::error::{DbError, Result};

#[cfg(target_arch = "x86_64")]
use std::arch::x86_64::*;
use std::collections::HashMap;

/// Aggregate operation type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AggregateOp {
    /// Sum of values
    Sum,
    /// Count of values
    Count,
    /// Minimum value
    Min,
    /// Maximum value
    Max,
    /// Average value
    Avg,
    /// Count distinct values
    CountDistinct,
    /// Variance
    Variance,
    /// Standard deviation
    StdDev,
}

// ============================================================================
// f64 SIMD Aggregates
// ============================================================================

/// SIMD sum for f64 columns - processes 4 f64s at once
///
/// # Safety
/// Requires AVX2 support. Use cpu_features() to check before calling.
#[inline]
#[cfg(target_arch = "x86_64")]
#[target_feature(enable = "avx2")]
pub unsafe fn sum_f64_avx2(data: &[f64]) -> f64 {
    let mut acc = _mm256_setzero_pd();
    let len = data.len();
    let chunks = len / 4;

    // Process 4 elements at a time
    for i in 0..chunks {
        let offset = i * 4;
        let vec = _mm256_loadu_pd(data.as_ptr().add(offset));
        acc = _mm256_add_pd(acc, vec);
    }

    // Horizontal sum
    let sum = horizontal_sum_pd(acc);

    // Handle remainder
    let mut remainder_sum = 0.0;
    for i in (chunks * 4)..len {
        remainder_sum += data[i];
    }

    sum + remainder_sum
}

/// SIMD minimum for f64 columns
#[inline]
#[cfg(target_arch = "x86_64")]
#[target_feature(enable = "avx2")]
pub unsafe fn min_f64_avx2(data: &[f64]) -> f64 {
    if data.is_empty() {
        return f64::INFINITY;
    }

    let mut min_vec = _mm256_set1_pd(f64::INFINITY);
    let len = data.len();
    let chunks = len / 4;

    for i in 0..chunks {
        let offset = i * 4;
        let vec = _mm256_loadu_pd(data.as_ptr().add(offset));
        min_vec = _mm256_min_pd(min_vec, vec);
    }

    // Horizontal minimum
    let mut min_val = horizontal_min_pd(min_vec);

    // Handle remainder
    for i in (chunks * 4)..len {
        if data[i] < min_val {
            min_val = data[i];
        }
    }

    min_val
}

/// SIMD maximum for f64 columns
#[inline]
#[cfg(target_arch = "x86_64")]
#[target_feature(enable = "avx2")]
pub unsafe fn max_f64_avx2(data: &[f64]) -> f64 {
    if data.is_empty() {
        return f64::NEG_INFINITY;
    }

    let mut max_vec = _mm256_set1_pd(f64::NEG_INFINITY);
    let len = data.len();
    let chunks = len / 4;

    for i in 0..chunks {
        let offset = i * 4;
        let vec = _mm256_loadu_pd(data.as_ptr().add(offset));
        max_vec = _mm256_max_pd(max_vec, vec);
    }

    // Horizontal maximum
    let mut max_val = horizontal_max_pd(max_vec);

    // Handle remainder
    for i in (chunks * 4)..len {
        if data[i] > max_val {
            max_val = data[i];
        }
    }

    max_val
}

/// SIMD average for f64 columns
#[inline]
#[cfg(target_arch = "x86_64")]
#[target_feature(enable = "avx2")]
pub unsafe fn avg_f64_avx2(data: &[f64]) -> f64 {
    if data.is_empty() {
        return 0.0;
    }

    let sum = sum_f64_avx2(data);
    sum / data.len() as f64
}

// ============================================================================
// f32 SIMD Aggregates
// ============================================================================

/// SIMD sum for f32 columns - processes 8 f32s at once
#[inline]
#[cfg(target_arch = "x86_64")]
#[target_feature(enable = "avx2")]
pub unsafe fn sum_f32_avx2(data: &[f32]) -> f32 {
    let mut acc = _mm256_setzero_ps();
    let len = data.len();
    let chunks = len / 8;

    for i in 0..chunks {
        let offset = i * 8;
        let vec = _mm256_loadu_ps(data.as_ptr().add(offset));
        acc = _mm256_add_ps(acc, vec);
    }

    // Horizontal sum
    let sum = horizontal_sum_ps(acc);

    // Handle remainder
    let mut remainder_sum = 0.0;
    for i in (chunks * 8)..len {
        remainder_sum += data[i];
    }

    sum + remainder_sum
}

/// SIMD minimum for f32 columns
#[inline]
#[cfg(target_arch = "x86_64")]
#[target_feature(enable = "avx2")]
pub unsafe fn min_f32_avx2(data: &[f32]) -> f32 {
    if data.is_empty() {
        return f32::INFINITY;
    }

    let mut min_vec = _mm256_set1_ps(f32::INFINITY);
    let len = data.len();
    let chunks = len / 8;

    for i in 0..chunks {
        let offset = i * 8;
        let vec = _mm256_loadu_ps(data.as_ptr().add(offset));
        min_vec = _mm256_min_ps(min_vec, vec);
    }

    // Horizontal minimum
    let mut min_val = horizontal_min_ps(min_vec);

    // Handle remainder
    for i in (chunks * 8)..len {
        if data[i] < min_val {
            min_val = data[i];
        }
    }

    min_val
}

/// SIMD maximum for f32 columns
#[inline]
#[cfg(target_arch = "x86_64")]
#[target_feature(enable = "avx2")]
pub unsafe fn max_f32_avx2(data: &[f32]) -> f32 {
    if data.is_empty() {
        return f32::NEG_INFINITY;
    }

    let mut max_vec = _mm256_set1_ps(f32::NEG_INFINITY);
    let len = data.len();
    let chunks = len / 8;

    for i in 0..chunks {
        let offset = i * 8;
        let vec = _mm256_loadu_ps(data.as_ptr().add(offset));
        max_vec = _mm256_max_ps(max_vec, vec);
    }

    // Horizontal maximum
    let mut max_val = horizontal_max_ps(max_vec);

    // Handle remainder
    for i in (chunks * 8)..len {
        if data[i] > max_val {
            max_val = data[i];
        }
    }

    max_val
}

// ============================================================================
// i32 SIMD Aggregates
// ============================================================================

/// SIMD sum for i32 columns - processes 8 i32s at once
#[inline]
#[cfg(target_arch = "x86_64")]
#[target_feature(enable = "avx2")]
pub unsafe fn sum_i32_avx2(data: &[i32]) -> i64 {
    let mut acc = _mm256_setzero_si256();
    let len = data.len();
    let chunks = len / 8;

    for i in 0..chunks {
        let offset = i * 8;
        let vec = _mm256_loadu_si256(data.as_ptr().add(offset) as *const __m256i);
        acc = _mm256_add_epi32(acc, vec);
    }

    // Horizontal sum
    let sum = horizontal_sum_epi32(acc);

    // Handle remainder
    let mut remainder_sum = 0i64;
    for i in (chunks * 8)..len {
        remainder_sum += data[i] as i64;
    }

    sum + remainder_sum
}

/// SIMD minimum for i32 columns
#[inline]
#[cfg(target_arch = "x86_64")]
#[target_feature(enable = "avx2")]
pub unsafe fn min_i32_avx2(data: &[i32]) -> i32 {
    if data.is_empty() {
        return i32::MAX;
    }

    let mut min_vec = _mm256_set1_epi32(i32::MAX);
    let len = data.len();
    let chunks = len / 8;

    for i in 0..chunks {
        let offset = i * 8;
        let vec = _mm256_loadu_si256(data.as_ptr().add(offset) as *const __m256i);
        min_vec = _mm256_min_epi32(min_vec, vec);
    }

    // Horizontal minimum
    let mut min_val = horizontal_min_epi32(min_vec);

    // Handle remainder
    for i in (chunks * 8)..len {
        if data[i] < min_val {
            min_val = data[i];
        }
    }

    min_val
}

/// SIMD maximum for i32 columns
#[inline]
#[cfg(target_arch = "x86_64")]
#[target_feature(enable = "avx2")]
pub unsafe fn max_i32_avx2(data: &[i32]) -> i32 {
    if data.is_empty() {
        return i32::MIN;
    }

    let mut max_vec = _mm256_set1_epi32(i32::MIN);
    let len = data.len();
    let chunks = len / 8;

    for i in 0..chunks {
        let offset = i * 8;
        let vec = _mm256_loadu_si256(data.as_ptr().add(offset) as *const __m256i);
        max_vec = _mm256_max_epi32(max_vec, vec);
    }

    // Horizontal maximum
    let mut max_val = horizontal_max_epi32(max_vec);

    // Handle remainder
    for i in (chunks * 8)..len {
        if data[i] > max_val {
            max_val = data[i];
        }
    }

    max_val
}

// ============================================================================
// i64 SIMD Aggregates
// ============================================================================

/// SIMD sum for i64 columns - processes 4 i64s at once
#[inline]
#[cfg(target_arch = "x86_64")]
#[target_feature(enable = "avx2")]
pub unsafe fn sum_i64_avx2(data: &[i64]) -> i64 {
    let mut acc = _mm256_setzero_si256();
    let len = data.len();
    let chunks = len / 4;

    for i in 0..chunks {
        let offset = i * 4;
        let vec = _mm256_loadu_si256(data.as_ptr().add(offset) as *const __m256i);
        acc = _mm256_add_epi64(acc, vec);
    }

    // Horizontal sum
    let sum = horizontal_sum_epi64(acc);

    // Handle remainder
    let mut remainder_sum = 0i64;
    for i in (chunks * 4)..len {
        remainder_sum += data[i];
    }

    sum + remainder_sum
}

// ============================================================================
// Horizontal reduction operations
// ============================================================================

/// Horizontal sum for __m256d (4 x f64)
#[inline]
#[cfg(target_arch = "x86_64")]
unsafe fn horizontal_sum_pd(vec: __m256d) -> f64 {
    // Extract high and low 128-bit lanes
    let high = _mm256_extractf128_pd(vec, 1);
    let low = _mm256_castpd256_pd128(vec);

    // Add high and low lanes
    let sum128 = _mm_add_pd(high, low);

    // Horizontal add within 128-bit lane
    let sum_high = _mm_unpackhi_pd(sum128, sum128);
    let sum_final = _mm_add_sd(sum128, sum_high);

    // Extract result
    _mm_cvtsd_f64(sum_final)
}

/// Horizontal minimum for __m256d
#[inline]
#[cfg(target_arch = "x86_64")]
unsafe fn horizontal_min_pd(vec: __m256d) -> f64 {
    let high = _mm256_extractf128_pd(vec, 1);
    let low = _mm256_castpd256_pd128(vec);
    let min128 = _mm_min_pd(high, low);
    let min_high = _mm_unpackhi_pd(min128, min128);
    let min_final = _mm_min_sd(min128, min_high);
    _mm_cvtsd_f64(min_final)
}

/// Horizontal maximum for __m256d
#[inline]
#[cfg(target_arch = "x86_64")]
unsafe fn horizontal_max_pd(vec: __m256d) -> f64 {
    let high = _mm256_extractf128_pd(vec, 1);
    let low = _mm256_castpd256_pd128(vec);
    let max128 = _mm_max_pd(high, low);
    let max_high = _mm_unpackhi_pd(max128, max128);
    let max_final = _mm_max_sd(max128, max_high);
    _mm_cvtsd_f64(max_final)
}

/// Horizontal sum for __m256 (8 x f32)
#[inline]
#[cfg(target_arch = "x86_64")]
unsafe fn horizontal_sum_ps(vec: __m256) -> f32 {
    let high = _mm256_extractf128_ps(vec, 1);
    let low = _mm256_castps256_ps128(vec);
    let sum128 = _mm_add_ps(high, low);

    // Horizontal add
    let shuf = _mm_movehdup_ps(sum128);
    let sums = _mm_add_ps(sum128, shuf);
    let shuf = _mm_movehl_ps(shuf, sums);
    let sums = _mm_add_ss(sums, shuf);

    _mm_cvtss_f32(sums)
}

/// Horizontal minimum for __m256
#[inline]
#[cfg(target_arch = "x86_64")]
unsafe fn horizontal_min_ps(vec: __m256) -> f32 {
    let high = _mm256_extractf128_ps(vec, 1);
    let low = _mm256_castps256_ps128(vec);
    let min128 = _mm_min_ps(high, low);

    let shuf = _mm_movehdup_ps(min128);
    let mins = _mm_min_ps(min128, shuf);
    let shuf = _mm_movehl_ps(shuf, mins);
    let mins = _mm_min_ss(mins, shuf);

    _mm_cvtss_f32(mins)
}

/// Horizontal maximum for __m256
#[inline]
#[cfg(target_arch = "x86_64")]
unsafe fn horizontal_max_ps(vec: __m256) -> f32 {
    let high = _mm256_extractf128_ps(vec, 1);
    let low = _mm256_castps256_ps128(vec);
    let max128 = _mm_max_ps(high, low);

    let shuf = _mm_movehdup_ps(max128);
    let maxs = _mm_max_ps(max128, shuf);
    let shuf = _mm_movehl_ps(shuf, maxs);
    let maxs = _mm_max_ss(maxs, shuf);

    _mm_cvtss_f32(maxs)
}

/// Horizontal sum for __m256i (8 x i32)
#[inline]
#[cfg(target_arch = "x86_64")]
unsafe fn horizontal_sum_epi32(vec: __m256i) -> i64 {
    // Extract as array
    let mut result = [0i32; 8];
    _mm256_storeu_si256(result.as_mut_ptr() as *mut __m256i, vec);

    result.iter().map(|&x| x as i64).sum()
}

/// Horizontal minimum for __m256i (8 x i32)
#[inline]
#[cfg(target_arch = "x86_64")]
unsafe fn horizontal_min_epi32(vec: __m256i) -> i32 {
    let mut result = [0i32; 8];
    _mm256_storeu_si256(result.as_mut_ptr() as *mut __m256i, vec);
    *result.iter().min().unwrap()
}

/// Horizontal maximum for __m256i (8 x i32)
#[inline]
#[cfg(target_arch = "x86_64")]
unsafe fn horizontal_max_epi32(vec: __m256i) -> i32 {
    let mut result = [0i32; 8];
    _mm256_storeu_si256(result.as_mut_ptr() as *mut __m256i, vec);
    *result.iter().max().unwrap()
}

/// Horizontal sum for __m256i (4 x i64)
#[inline]
#[cfg(target_arch = "x86_64")]
unsafe fn horizontal_sum_epi64(vec: __m256i) -> i64 {
    let mut result = [0i64; 4];
    _mm256_storeu_si256(result.as_mut_ptr() as *mut __m256i, vec);
    result.iter().sum()
}

// ============================================================================
// Scalar Fallback Implementations
// ============================================================================

/// Scalar sum for f64
pub fn sum_f64_scalar(data: &[f64]) -> f64 {
    data.iter().sum()
}

/// Scalar min for f64
pub fn min_f64_scalar(data: &[f64]) -> f64 {
    data.iter().copied().fold(f64::INFINITY, f64::min)
}

/// Scalar max for f64
pub fn max_f64_scalar(data: &[f64]) -> f64 {
    data.iter().copied().fold(f64::NEG_INFINITY, f64::max)
}

/// Scalar avg for f64
pub fn avg_f64_scalar(data: &[f64]) -> f64 {
    if data.is_empty() {
        0.0
    } else {
        sum_f64_scalar(data) / data.len() as f64
    }
}

/// Scalar sum for i32
pub fn sum_i32_scalar(data: &[i32]) -> i64 {
    data.iter().map(|&x| x as i64).sum()
}

/// Scalar min for i32
pub fn min_i32_scalar(data: &[i32]) -> i32 {
    data.iter().copied().min().unwrap_or(i32::MAX)
}

/// Scalar max for i32
pub fn max_i32_scalar(data: &[i32]) -> i32 {
    data.iter().copied().max().unwrap_or(i32::MIN)
}

/// Scalar avg for i32
pub fn avg_i32_scalar(data: &[i32]) -> f64 {
    if data.is_empty() {
        0.0
    } else {
        sum_i32_scalar(data) as f64 / data.len() as f64
    }
}

// ============================================================================
// High-level Aggregate API
// ============================================================================

/// SIMD-accelerated aggregator
pub struct SimdAggregator {
    /// SIMD context
    context: SimdContext,
}

impl SimdAggregator {
    /// Create new SIMD aggregator
    pub fn new() -> Self {
        Self {
            context: SimdContext::new(),
        }
    }

    /// Create with custom context
    pub fn with_context(context: SimdContext) -> Self {
        Self { context }
    }

    /// Compute aggregate on f64 column
    pub fn aggregate_f64(&mut self, data: &[f64], op: AggregateOp) -> Result<f64> {
        let result = if self.context.has_avx2() {
            unsafe {
                match op {
                    AggregateOp::Sum => sum_f64_avx2(data),
                    AggregateOp::Min => min_f64_avx2(data),
                    AggregateOp::Max => max_f64_avx2(data),
                    AggregateOp::Avg => avg_f64_avx2(data),
                    AggregateOp::Count => data.len() as f64,
                    _ => {
                        return Err(DbError::InvalidArgument(format!(
                            "Unsupported aggregate operation: {:?}",
                            op
                        )))
                    }
                }
            }
        } else {
            match op {
                AggregateOp::Sum => sum_f64_scalar(data),
                AggregateOp::Min => min_f64_scalar(data),
                AggregateOp::Max => max_f64_scalar(data),
                AggregateOp::Avg => avg_f64_scalar(data),
                AggregateOp::Count => data.len() as f64,
                _ => {
                    return Err(DbError::InvalidArgument(format!(
                        "Unsupported aggregate operation: {:?}",
                        op
                    )))
                }
            }
        };

        self.context.record_simd_op(data.len() as u64);
        Ok(result)
    }

    /// Compute aggregate on f32 column
    pub fn aggregate_f32(&mut self, data: &[f32], op: AggregateOp) -> Result<f32> {
        let result = if self.context.has_avx2() {
            unsafe {
                match op {
                    AggregateOp::Sum => sum_f32_avx2(data),
                    AggregateOp::Min => min_f32_avx2(data),
                    AggregateOp::Max => max_f32_avx2(data),
                    AggregateOp::Avg => sum_f32_avx2(data) / data.len() as f32,
                    AggregateOp::Count => data.len() as f32,
                    _ => {
                        return Err(DbError::InvalidArgument(format!(
                            "Unsupported aggregate operation: {:?}",
                            op
                        )))
                    }
                }
            }
        } else {
            match op {
                AggregateOp::Sum => data.iter().sum(),
                AggregateOp::Min => data.iter().copied().fold(f32::INFINITY, f32::min),
                AggregateOp::Max => data.iter().copied().fold(f32::NEG_INFINITY, f32::max),
                AggregateOp::Avg => {
                    let sum: f32 = data.iter().sum();
                    sum / data.len() as f32
                }
                AggregateOp::Count => data.len() as f32,
                _ => {
                    return Err(DbError::InvalidArgument(format!(
                        "Unsupported aggregate operation: {:?}",
                        op
                    )))
                }
            }
        };

        self.context.record_simd_op(data.len() as u64);
        Ok(result)
    }

    /// Compute aggregate on i32 column
    pub fn aggregate_i32(&mut self, data: &[i32], op: AggregateOp) -> Result<i64> {
        let result = if self.context.has_avx2() {
            unsafe {
                match op {
                    AggregateOp::Sum => sum_i32_avx2(data),
                    AggregateOp::Min => min_i32_avx2(data) as i64,
                    AggregateOp::Max => max_i32_avx2(data) as i64,
                    AggregateOp::Count => data.len() as i64,
                    _ => {
                        return Err(DbError::InvalidArgument(format!(
                            "Unsupported aggregate operation: {:?}",
                            op
                        )))
                    }
                }
            }
        } else {
            match op {
                AggregateOp::Sum => sum_i32_scalar(data),
                AggregateOp::Min => min_i32_scalar(data) as i64,
                AggregateOp::Max => max_i32_scalar(data) as i64,
                AggregateOp::Count => data.len() as i64,
                _ => {
                    return Err(DbError::InvalidArgument(format!(
                        "Unsupported aggregate operation: {:?}",
                        op
                    )))
                }
            }
        };

        self.context.record_simd_op(data.len() as u64);
        Ok(result)
    }

    /// Compute aggregate on i64 column
    pub fn aggregate_i64(&mut self, data: &[i64], op: AggregateOp) -> Result<i64> {
        let result = if self.context.has_avx2() {
            unsafe {
                match op {
                    AggregateOp::Sum => sum_i64_avx2(data),
                    AggregateOp::Min => *data.iter().min().unwrap_or(&i64::MAX),
                    AggregateOp::Max => *data.iter().max().unwrap_or(&i64::MIN),
                    AggregateOp::Count => data.len() as i64,
                    _ => {
                        return Err(DbError::InvalidArgument(format!(
                            "Unsupported aggregate operation: {:?}",
                            op
                        )))
                    }
                }
            }
        } else {
            match op {
                AggregateOp::Sum => data.iter().sum(),
                AggregateOp::Min => *data.iter().min().unwrap_or(&i64::MAX),
                AggregateOp::Max => *data.iter().max().unwrap_or(&i64::MIN),
                AggregateOp::Count => data.len() as i64,
                _ => {
                    return Err(DbError::InvalidArgument(format!(
                        "Unsupported aggregate operation: {:?}",
                        op
                    )))
                }
            }
        };

        self.context.record_simd_op(data.len() as u64);
        Ok(result)
    }

    /// Compute variance
    pub fn variance_f64(&mut self, data: &[f64]) -> Result<f64> {
        if data.is_empty() {
            return Ok(0.0);
        }

        let mean = self.aggregate_f64(data, AggregateOp::Avg)?;
        let mut sum_sq_diff = 0.0;

        for &val in data {
            let diff = val - mean;
            sum_sq_diff += diff * diff;
        }

        Ok(sum_sq_diff / data.len() as f64)
    }

    /// Compute standard deviation
    pub fn stddev_f64(&mut self, data: &[f64]) -> Result<f64> {
        Ok(self.variance_f64(data)?.sqrt())
    }

    /// Get statistics
    pub fn stats(&self) -> &SimdStats {
        &self.context.stats
    }
}

impl Default for SimdAggregator {
    fn default() -> Self {
        Self::new()
    }
}

/// Grouped aggregation
pub struct GroupedAggregator {
    /// Groups (key -> row indices)
    groups: HashMap<Vec<Value>, Vec<usize>>,
}

impl GroupedAggregator {
    /// Create new grouped aggregator
    pub fn new() -> Self {
        Self {
            groups: HashMap::new(),
        }
    }

    /// Add row to group
    pub fn add_row(&mut self, key: Vec<Value>, row_index: usize) {
        self.groups
            .entry(key)
            .or_insert_with(Vec::new)
            .push(row_index);
    }

    /// Get group count
    pub fn group_count(&self) -> usize {
        self.groups.len()
    }

    /// Get all groups
    pub fn groups(&self) -> &HashMap<Vec<Value>, Vec<usize>> {
        &self.groups
    }
}

impl Default for GroupedAggregator {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_f64_sum() {
        let data = vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0];
        let mut agg = SimdAggregator::new();
        let result = agg.aggregate_f64(&data, AggregateOp::Sum).unwrap();
        assert_eq!(result, 36.0);
    }

    #[test]
    fn test_f64_avg() {
        let data = vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0];
        let mut agg = SimdAggregator::new();
        let result = agg.aggregate_f64(&data, AggregateOp::Avg).unwrap();
        assert_eq!(result, 4.5);
    }

    #[test]
    fn test_f64_min_max() {
        let data = vec![5.0, 2.0, 8.0, 1.0, 9.0, 3.0, 7.0, 4.0];
        let mut agg = SimdAggregator::new();

        let min = agg.aggregate_f64(&data, AggregateOp::Min).unwrap();
        assert_eq!(min, 1.0);

        let max = agg.aggregate_f64(&data, AggregateOp::Max).unwrap();
        assert_eq!(max, 9.0);
    }

    #[test]
    fn test_i32_sum() {
        let data = vec![1, 2, 3, 4, 5, 6, 7, 8];
        let mut agg = SimdAggregator::new();
        let result = agg.aggregate_i32(&data, AggregateOp::Sum).unwrap();
        assert_eq!(result, 36);
    }

    #[test]
    fn test_i32_min_max() {
        let data = vec![5, 2, 8, 1, 9, 3, 7, 4];
        let mut agg = SimdAggregator::new();

        let min = agg.aggregate_i32(&data, AggregateOp::Min).unwrap();
        assert_eq!(min, 1);

        let max = agg.aggregate_i32(&data, AggregateOp::Max).unwrap();
        assert_eq!(max, 9);
    }

    #[test]
    #[cfg(target_arch = "x86_64")]
    fn test_avx2_sum_f64() {
        if !is_x86_feature_detected!("avx2") {
            return;
        }

        let data = vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0];
        let result = unsafe { sum_f64_avx2(&data) };
        assert_eq!(result, 36.0);
    }

    #[test]
    fn test_variance() {
        let data = vec![2.0, 4.0, 4.0, 4.0, 5.0, 5.0, 7.0, 9.0];
        let mut agg = SimdAggregator::new();
        let variance = agg.variance_f64(&data).unwrap();
        assert!((variance - 4.0).abs() < 0.01);
    }

    #[test]
    fn test_grouped_aggregator() {
        let mut grouped = GroupedAggregator::new();
        grouped.add_row(vec![Value::Integer(1)], 0);
        grouped.add_row(vec![Value::Integer(1)], 2);
        grouped.add_row(vec![Value::Integer(2)], 1);

        assert_eq!(grouped.group_count(), 2);
    }
}
