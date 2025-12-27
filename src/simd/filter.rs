// # SIMD Filter Operations
//
// Vectorized predicate evaluation for filtering rows in table scans.
// Supports AVX2 SIMD instructions processing 8-16 elements per operation.
//
// SECURITY NOTES:
// - All SIMD operations include bounds checking to prevent buffer overruns
// - Input data does NOT need to be aligned (using _mm256_loadu_* functions)
// - Result buffers must be properly sized: (data.len() + 7) / 8 for i32/f32
// - Unsafe code is contained within target_feature functions with proper checks

use super::{SelectionVector, SimdContext};
use crate::common::Value;
use crate::error::{DbError, Result};

#[cfg(target_arch = "x86_64")]
use std::arch::x86_64::*;

// SECURITY: Maximum buffer size for SIMD operations (1GB)
// Prevents unbounded memory allocation for result buffers
const MAX_SIMD_BUFFER_SIZE: usize = 1024 * 1024 * 1024;

/// Predicate types supported by SIMD filters
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PredicateType {
    /// Equality comparison (=)
    Equal,
    /// Not equal comparison (!=)
    NotEqual,
    /// Less than comparison (<)
    LessThan,
    /// Less than or equal (<=)
    LessThanOrEqual,
    /// Greater than comparison (>)
    GreaterThan,
    /// Greater than or equal (>=)
    GreaterThanOrEqual,
    /// Between range (BETWEEN)
    Between,
    /// In list (IN)
    In,
    /// NULL check (IS NULL)
    IsNull,
    /// NOT NULL check (IS NOT NULL)
    IsNotNull,
}

/// Filter operation configuration
#[derive(Debug, Clone)]
pub struct FilterOp {
    /// Column index to filter
    pub column_index: usize,
    /// Predicate type
    pub predicate: PredicateType,
    /// Comparison value(s)
    pub values: Vec<Value>,
}

impl FilterOp {
    /// Create equality filter
    pub fn equal(column_index: usize, value: Value) -> Self {
        Self {
            column_index,
            predicate: PredicateType::Equal,
            values: vec![value],
        }
    }

    /// Create range filter (BETWEEN)
    pub fn between(column_index: usize, low: Value, high: Value) -> Self {
        Self {
            column_index,
            predicate: PredicateType::Between,
            values: vec![low, high],
        }
    }

    /// Create IN list filter
    pub fn in_list(column_index: usize, values: Vec<Value>) -> Self {
        Self {
            column_index,
            predicate: PredicateType::In,
            values,
        }
    }
}

// ============================================================================
// i32 SIMD Filters
// ============================================================================

/// SIMD filter for i32 equality - processes 8 i32s at once using AVX2
///
/// # Safety
/// Requires AVX2 support. Use cpu_features() to check before calling.
/// - data.len() must be <= MAX_SIMD_BUFFER_SIZE elements
/// - result.len() must be >= (data.len() + 7) / 8
/// - Alignment: Input data does NOT require alignment (using loadu)
#[inline]
#[cfg(target_arch = "x86_64")]
#[target_feature(enable = "avx2")]
pub unsafe fn filter_i32_eq_avx2(data: &[i32], value: i32, result: &mut [u8]) {
    // SECURITY: Bounds checking to prevent buffer overruns
    debug_assert!(
        data.len() <= MAX_SIMD_BUFFER_SIZE,
        "SIMD buffer size {} exceeds maximum {}",
        data.len(),
        MAX_SIMD_BUFFER_SIZE
    );
    debug_assert!(
        result.len() >= (data.len() + 7) / 8,
        "Result buffer too small: {} < {}",
        result.len(),
        (data.len() + 7) / 8
    );

    let val = _mm256_set1_epi32(value);
    let len = data.len();
    let chunks = len / 8;

    for i in 0..chunks {
        let offset = i * 8;
        let vec = _mm256_loadu_si256(data.as_ptr().add(offset) as *const __m256i);
        let cmp = _mm256_cmpeq_epi32(vec, val);
        let mask = _mm256_movemask_epi8(cmp);

        // Store bitmask result
        if i < result.len() {
            result[i] = mask as u8;
        }
    }

    // Handle remainder with scalar code
    let remainder_start = chunks * 8;
    simd_remainder!(data, value, ==, remainder_start, len, result, chunks);
}

/// SIMD filter for i32 less-than - processes 8 i32s at once
#[inline]
#[cfg(target_arch = "x86_64")]
#[target_feature(enable = "avx2")]
pub unsafe fn filter_i32_lt_avx2(data: &[i32], value: i32, result: &mut [u8]) {
    let val = _mm256_set1_epi32(value);
    let len = data.len();
    let chunks = len / 8;

    for i in 0..chunks {
        let offset = i * 8;
        let vec = _mm256_loadu_si256(data.as_ptr().add(offset) as *const __m256i);
        let cmp = _mm256_cmpgt_epi32(val, vec);
        let mask = _mm256_movemask_epi8(cmp);

        if i < result.len() {
            result[i] = mask as u8;
        }
    }

    // Handle remainder
    let remainder_start = chunks * 8;
    simd_remainder!(data, value, <, remainder_start, len, result, chunks);
}

/// SIMD filter for i32 greater-than - processes 8 i32s at once
#[inline]
#[cfg(target_arch = "x86_64")]
#[target_feature(enable = "avx2")]
pub unsafe fn filter_i32_gt_avx2(data: &[i32], value: i32, result: &mut [u8]) {
    let val = _mm256_set1_epi32(value);
    let len = data.len();
    let chunks = len / 8;

    for i in 0..chunks {
        let offset = i * 8;
        let vec = _mm256_loadu_si256(data.as_ptr().add(offset) as *const __m256i);
        let cmp = _mm256_cmpgt_epi32(vec, val);
        let mask = _mm256_movemask_epi8(cmp);

        if i < result.len() {
            result[i] = mask as u8;
        }
    }

    // Handle remainder
    let remainder_start = chunks * 8;
    simd_remainder!(data, value, >, remainder_start, len, result, chunks);
}

/// SIMD filter for i32 range (BETWEEN) - processes 8 i32s at once
#[inline]
#[cfg(target_arch = "x86_64")]
#[target_feature(enable = "avx2")]
pub unsafe fn filter_i32_between_avx2(data: &[i32], low: i32, high: i32, result: &mut [u8]) {
    let low_vec = _mm256_set1_epi32(low);
    let high_vec = _mm256_set1_epi32(high);
    let len = data.len();
    let chunks = len / 8;

    for i in 0..chunks {
        let offset = i * 8;
        let vec = _mm256_loadu_si256(data.as_ptr().add(offset) as *const __m256i);

        // vec >= low
        let cmp_low = _mm256_cmpgt_epi32(vec, low_vec);
        let eq_low = _mm256_cmpeq_epi32(vec, low_vec);
        let ge_low = _mm256_or_si256(cmp_low, eq_low);

        // vec <= high
        let cmp_high = _mm256_cmpgt_epi32(high_vec, vec);
        let eq_high = _mm256_cmpeq_epi32(vec, high_vec);
        let le_high = _mm256_or_si256(cmp_high, eq_high);

        // Combine
        let cmp = _mm256_and_si256(ge_low, le_high);
        let mask = _mm256_movemask_epi8(cmp);

        if i < result.len() {
            result[i] = mask as u8;
        }
    }

    // Handle remainder
    let remainder_start = chunks * 8;
    if remainder_start < len && chunks < result.len() {
        let mut remainder_mask = 0u8;
        for j in 0..(len - remainder_start) {
            let val = data[remainder_start + j];
            if val >= low && val <= high {
                remainder_mask |= 1 << j;
            }
        }
        result[chunks] = remainder_mask;
    }
}

// ============================================================================
// i64 SIMD Filters
// ============================================================================

/// SIMD filter for i64 equality - processes 4 i64s at once using AVX2
#[inline]
#[cfg(target_arch = "x86_64")]
#[target_feature(enable = "avx2")]
pub unsafe fn filter_i64_eq_avx2(data: &[i64], value: i64, result: &mut [u8]) {
    let val = _mm256_set1_epi64x(value);
    let len = data.len();
    let chunks = len / 4;

    for i in 0..chunks {
        let offset = i * 4;
        let vec = _mm256_loadu_si256(data.as_ptr().add(offset) as *const __m256i);
        let cmp = _mm256_cmpeq_epi64(vec, val);
        let mask = _mm256_movemask_epi8(cmp);

        if i < result.len() {
            result[i] = mask as u8;
        }
    }

    // Handle remainder
    let remainder_start = chunks * 4;
    if remainder_start < len && chunks < result.len() {
        let mut remainder_mask = 0u8;
        for j in 0..(len - remainder_start) {
            if data[remainder_start + j] == value {
                remainder_mask |= 1 << (j * 2);
            }
        }
        result[chunks] = remainder_mask;
    }
}

/// SIMD filter for i64 less-than
#[inline]
#[cfg(target_arch = "x86_64")]
#[target_feature(enable = "avx2")]
pub unsafe fn filter_i64_lt_avx2(data: &[i64], value: i64, result: &mut [u8]) {
    let val = _mm256_set1_epi64x(value);
    let len = data.len();
    let chunks = len / 4;

    for i in 0..chunks {
        let offset = i * 4;
        let vec = _mm256_loadu_si256(data.as_ptr().add(offset) as *const __m256i);
        let cmp = _mm256_cmpgt_epi64(val, vec);
        let mask = _mm256_movemask_epi8(cmp);

        if i < result.len() {
            result[i] = mask as u8;
        }
    }

    // Handle remainder
    let remainder_start = chunks * 4;
    if remainder_start < len && chunks < result.len() {
        let mut remainder_mask = 0u8;
        for j in 0..(len - remainder_start) {
            if data[remainder_start + j] < value {
                remainder_mask |= 1 << (j * 2);
            }
        }
        result[chunks] = remainder_mask;
    }
}

/// SIMD filter for i64 greater-than
#[inline]
#[cfg(target_arch = "x86_64")]
#[target_feature(enable = "avx2")]
pub unsafe fn filter_i64_gt_avx2(data: &[i64], value: i64, result: &mut [u8]) {
    let val = _mm256_set1_epi64x(value);
    let len = data.len();
    let chunks = len / 4;

    for i in 0..chunks {
        let offset = i * 4;
        let vec = _mm256_loadu_si256(data.as_ptr().add(offset) as *const __m256i);
        let cmp = _mm256_cmpgt_epi64(vec, val);
        let mask = _mm256_movemask_epi8(cmp);

        if i < result.len() {
            result[i] = mask as u8;
        }
    }

    // Handle remainder
    let remainder_start = chunks * 4;
    if remainder_start < len && chunks < result.len() {
        let mut remainder_mask = 0u8;
        for j in 0..(len - remainder_start) {
            if data[remainder_start + j] > value {
                remainder_mask |= 1 << (j * 2);
            }
        }
        result[chunks] = remainder_mask;
    }
}

// ============================================================================
// f32 SIMD Filters
// ============================================================================

/// SIMD filter for f32 equality - processes 8 f32s at once
#[inline]
#[cfg(target_arch = "x86_64")]
#[target_feature(enable = "avx2")]
pub unsafe fn filter_f32_eq_avx2(data: &[f32], value: f32, result: &mut [u8]) {
    let val = _mm256_set1_ps(value);
    let len = data.len();
    let chunks = len / 8;

    for i in 0..chunks {
        let offset = i * 8;
        let vec = _mm256_loadu_ps(data.as_ptr().add(offset));
        let cmp = _mm256_cmp_ps(vec, val, _CMP_EQ_OQ);
        let mask = _mm256_movemask_ps(cmp);

        if i < result.len() {
            result[i] = mask as u8;
        }
    }

    // Handle remainder
    let remainder_start = chunks * 8;
    if remainder_start < len && chunks < result.len() {
        let mut remainder_mask = 0u8;
        for j in 0..(len - remainder_start) {
            if data[remainder_start + j] == value {
                remainder_mask |= 1 << j;
            }
        }
        result[chunks] = remainder_mask;
    }
}

/// SIMD filter for f32 less-than
#[inline]
#[cfg(target_arch = "x86_64")]
#[target_feature(enable = "avx2")]
pub unsafe fn filter_f32_lt_avx2(data: &[f32], value: f32, result: &mut [u8]) {
    let val = _mm256_set1_ps(value);
    let len = data.len();
    let chunks = len / 8;

    for i in 0..chunks {
        let offset = i * 8;
        let vec = _mm256_loadu_ps(data.as_ptr().add(offset));
        let cmp = _mm256_cmp_ps(vec, val, _CMP_LT_OQ);
        let mask = _mm256_movemask_ps(cmp);

        if i < result.len() {
            result[i] = mask as u8;
        }
    }

    // Handle remainder
    let remainder_start = chunks * 8;
    if remainder_start < len && chunks < result.len() {
        let mut remainder_mask = 0u8;
        for j in 0..(len - remainder_start) {
            if data[remainder_start + j] < value {
                remainder_mask |= 1 << j;
            }
        }
        result[chunks] = remainder_mask;
    }
}

/// SIMD filter for f32 greater-than
#[inline]
#[cfg(target_arch = "x86_64")]
#[target_feature(enable = "avx2")]
pub unsafe fn filter_f32_gt_avx2(data: &[f32], value: f32, result: &mut [u8]) {
    let val = _mm256_set1_ps(value);
    let len = data.len();
    let chunks = len / 8;

    for i in 0..chunks {
        let offset = i * 8;
        let vec = _mm256_loadu_ps(data.as_ptr().add(offset));
        let cmp = _mm256_cmp_ps(vec, val, _CMP_GT_OQ);
        let mask = _mm256_movemask_ps(cmp);

        if i < result.len() {
            result[i] = mask as u8;
        }
    }

    // Handle remainder
    let remainder_start = chunks * 8;
    if remainder_start < len && chunks < result.len() {
        let mut remainder_mask = 0u8;
        for j in 0..(len - remainder_start) {
            if data[remainder_start + j] > value {
                remainder_mask |= 1 << j;
            }
        }
        result[chunks] = remainder_mask;
    }
}

// ============================================================================
// f64 SIMD Filters
// ============================================================================

/// SIMD filter for f64 equality - processes 4 f64s at once
#[inline]
#[cfg(target_arch = "x86_64")]
#[target_feature(enable = "avx2")]
pub unsafe fn filter_f64_eq_avx2(data: &[f64], value: f64, result: &mut [u8]) {
    let val = _mm256_set1_pd(value);
    let len = data.len();
    let chunks = len / 4;

    for i in 0..chunks {
        let offset = i * 4;
        let vec = _mm256_loadu_pd(data.as_ptr().add(offset));
        let cmp = _mm256_cmp_pd(vec, val, _CMP_EQ_OQ);
        let mask = _mm256_movemask_pd(cmp);

        if i < result.len() {
            result[i] = mask as u8;
        }
    }

    // Handle remainder
    let remainder_start = chunks * 4;
    if remainder_start < len && chunks < result.len() {
        let mut remainder_mask = 0u8;
        for j in 0..(len - remainder_start) {
            if data[remainder_start + j] == value {
                remainder_mask |= 1 << (j * 2);
            }
        }
        result[chunks] = remainder_mask;
    }
}

/// SIMD filter for f64 less-than
#[inline]
#[cfg(target_arch = "x86_64")]
#[target_feature(enable = "avx2")]
pub unsafe fn filter_f64_lt_avx2(data: &[f64], value: f64, result: &mut [u8]) {
    let val = _mm256_set1_pd(value);
    let len = data.len();
    let chunks = len / 4;

    for i in 0..chunks {
        let offset = i * 4;
        let vec = _mm256_loadu_pd(data.as_ptr().add(offset));
        let cmp = _mm256_cmp_pd(vec, val, _CMP_LT_OQ);
        let mask = _mm256_movemask_pd(cmp);

        if i < result.len() {
            result[i] = mask as u8;
        }
    }

    // Handle remainder
    let remainder_start = chunks * 4;
    if remainder_start < len && chunks < result.len() {
        let mut remainder_mask = 0u8;
        for j in 0..(len - remainder_start) {
            if data[remainder_start + j] < value {
                remainder_mask |= 1 << (j * 2);
            }
        }
        result[chunks] = remainder_mask;
    }
}

/// SIMD filter for f64 greater-than
#[inline]
#[cfg(target_arch = "x86_64")]
#[target_feature(enable = "avx2")]
pub unsafe fn filter_f64_gt_avx2(data: &[f64], value: f64, result: &mut [u8]) {
    let val = _mm256_set1_pd(value);
    let len = data.len();
    let chunks = len / 4;

    for i in 0..chunks {
        let offset = i * 4;
        let vec = _mm256_loadu_pd(data.as_ptr().add(offset));
        let cmp = _mm256_cmp_pd(vec, val, _CMP_GT_OQ);
        let mask = _mm256_movemask_pd(cmp);

        if i < result.len() {
            result[i] = mask as u8;
        }
    }

    // Handle remainder
    let remainder_start = chunks * 4;
    if remainder_start < len && chunks < result.len() {
        let mut remainder_mask = 0u8;
        for j in 0..(len - remainder_start) {
            if data[remainder_start + j] > value {
                remainder_mask |= 1 << (j * 2);
            }
        }
        result[chunks] = remainder_mask;
    }
}

// ============================================================================
// Scalar Fallback Implementations
// ============================================================================

/// Scalar fallback for i32 equality filter
pub fn filter_i32_eq_scalar(data: &[i32], value: i32, selection: &mut SelectionVector) {
    for (i, &val) in data.iter().enumerate() {
        if val == value {
            let _ = selection.add(i);
        }
    }
}

/// Scalar fallback for i32 less-than filter
pub fn filter_i32_lt_scalar(data: &[i32], value: i32, selection: &mut SelectionVector) {
    for (i, &val) in data.iter().enumerate() {
        if val < value {
            let _ = selection.add(i);
        }
    }
}

/// Scalar fallback for i32 greater-than filter
pub fn filter_i32_gt_scalar(data: &[i32], value: i32, selection: &mut SelectionVector) {
    for (i, &val) in data.iter().enumerate() {
        if val > value {
            let _ = selection.add(i);
        }
    }
}

/// Scalar fallback for i32 BETWEEN filter
pub fn filter_i32_between_scalar(
    data: &[i32],
    low: i32,
    high: i32,
    selection: &mut SelectionVector,
) {
    for (i, &val) in data.iter().enumerate() {
        if val >= low && val <= high {
            let _ = selection.add(i);
        }
    }
}

// ============================================================================
// High-level Filter API
// ============================================================================

/// SIMD-accelerated filter executor
pub struct SimdFilter {
    /// SIMD context
    context: SimdContext,
}

impl SimdFilter {
    /// Create new SIMD filter
    pub fn new() -> Self {
        Self {
            context: SimdContext::new(),
        }
    }

    /// Create with custom context
    pub fn with_context(context: SimdContext) -> Self {
        Self { context }
    }

    /// Apply filter to i32 column
    pub fn filter_i32(
        &mut self,
        data: &[i32],
        predicate: PredicateType,
        values: &[Value],
        selection: &mut SelectionVector,
    ) -> Result<()> {
        if values.is_empty() {
            return Err(DbError::InvalidArgument(
                "Filter requires at least one value".to_string(),
            ));
        }

        let value = match &values[0] {
            Value::Integer(v) => *v as i32,
            _ => {
                return Err(DbError::InvalidArgument(
                    "Expected integer value".to_string(),
                ))
            }
        };

        if self.context.has_avx2() {
            self.filter_i32_avx2(data, predicate, value, values, selection)
        } else {
            self.filter_i32_fallback(data, predicate, value, values, selection)
        }
    }

    #[cfg(target_arch = "x86_64")]
    fn filter_i32_avx2(
        &mut self,
        data: &[i32],
        predicate: PredicateType,
        value: i32,
        values: &[Value],
        selection: &mut SelectionVector,
    ) -> Result<()> {
        let mut result = vec![0u8; (data.len() + 7) / 8];

        unsafe {
            match predicate {
                PredicateType::Equal => {
                    filter_i32_eq_avx2(data, value, &mut result);
                }
                PredicateType::LessThan => {
                    filter_i32_lt_avx2(data, value, &mut result);
                }
                PredicateType::GreaterThan => {
                    filter_i32_gt_avx2(data, value, &mut result);
                }
                PredicateType::Between => {
                    if values.len() < 2 {
                        return Err(DbError::InvalidArgument(
                            "BETWEEN requires two values".to_string(),
                        ));
                    }
                    let high = match &values[1] {
                        Value::Integer(v) => *v as i32,
                        _ => {
                            return Err(DbError::InvalidArgument(
                                "Expected integer value".to_string(),
                            ))
                        }
                    };
                    filter_i32_between_avx2(data, value, high, &mut result);
                }
                _ => {
                    return self.filter_i32_fallback(data, predicate, value, values, selection);
                }
            }
        }

        // Convert bitmask to selection vector
        self.bitmask_to_selection(&result, data.len(), selection);
        self.context.record_simd_op(data.len() as u64);
        Ok(())
    }

    #[cfg(not(target_arch = "x86_64"))]
    fn filter_i32_avx2(
        &mut self,
        data: &[i32],
        predicate: PredicateType,
        value: i32,
        values: &[Value],
        selection: &mut SelectionVector,
    ) -> Result<()> {
        self.filter_i32_fallback(data, predicate, value, values, selection)
    }

    fn filter_i32_fallback(
        &mut self,
        data: &[i32],
        predicate: PredicateType,
        value: i32,
        values: &[Value],
        selection: &mut SelectionVector,
    ) -> Result<()> {
        match predicate {
            PredicateType::Equal => {
                filter_i32_eq_scalar(data, value, selection);
            }
            PredicateType::LessThan => {
                filter_i32_lt_scalar(data, value, selection);
            }
            PredicateType::GreaterThan => {
                filter_i32_gt_scalar(data, value, selection);
            }
            PredicateType::Between => {
                if values.len() < 2 {
                    return Err(DbError::InvalidArgument(
                        "BETWEEN requires two values".to_string(),
                    ));
                }
                let high = match &values[1] {
                    Value::Integer(v) => *v as i32,
                    _ => {
                        return Err(DbError::InvalidArgument(
                            "Expected integer value".to_string(),
                        ))
                    }
                };
                filter_i32_between_scalar(data, value, high, selection);
            }
            _ => {
                return Err(DbError::InvalidArgument(format!(
                    "Unsupported predicate: {:?}",
                    predicate
                )));
            }
        }

        self.context.record_scalar_op(data.len() as u64);
        Ok(())
    }

    /// Convert bitmask result to selection vector
    fn bitmask_to_selection(
        &mut self,
        bitmask: &[u8],
        datalen: usize,
        selection: &mut SelectionVector,
    ) {
        for (chunk_idx, &mask) in bitmask.iter().enumerate() {
            if mask == 0 {
                continue;
            }

            let base_idx = chunk_idx * 8;
            for bit in 0..8 {
                let idx = base_idx + bit;
                if idx >= datalen {
                    break;
                }
                if (mask & (1 << bit)) != 0 {
                    let _ = selection.add(idx);
                }
            }
        }

        self.context.record_selection(selection.len() as u64);
    }

    /// Get statistics
    pub fn stats(&self) -> &super::SimdStats {
        &self.context.stats
    }
}

impl Default for SimdFilter {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_i32_eq_filter() {
        let data = vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10];
        let mut selection = SelectionVector::with_capacity(10);
        let mut filter = SimdFilter::new();

        filter
            .filter_i32(
                &data,
                PredicateType::Equal,
                &[Value::Integer(5)],
                &mut selection,
            )
            .unwrap();

        assert_eq!(selection.indices(), &[4]);
    }

    #[test]
    fn test_i32_lt_filter() {
        let data = vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10];
        let mut selection = SelectionVector::with_capacity(10);
        let mut filter = SimdFilter::new();

        filter
            .filter_i32(
                &data,
                PredicateType::LessThan,
                &[Value::Integer(5)],
                &mut selection,
            )
            .unwrap();

        assert_eq!(selection.indices(), &[0, 1, 2, 3]);
    }

    #[test]
    fn test_i32_between_filter() {
        let data = vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10];
        let mut selection = SelectionVector::with_capacity(10);
        let mut filter = SimdFilter::new();

        filter
            .filter_i32(
                &data,
                PredicateType::Between,
                &[Value::Integer(3), Value::Integer(7)],
                &mut selection,
            )
            .unwrap();

        assert_eq!(selection.indices(), &[2, 3, 4, 5, 6]);
    }

    #[test]
    #[cfg(target_arch = "x86_64")]
    fn test_avx2_i32_eq() {
        if !is_x86_feature_detected!("avx2") {
            return;
        }

        let data = vec![1, 2, 3, 4, 5, 6, 7, 8];
        let mut result = vec![0u8; 1];

        unsafe {
            filter_i32_eq_avx2(&data, 5, &mut result);
        }

        // Check that bit 4 is set (5 is at index 4)
        assert_ne!(result[0] & (1 << 4), 0);
    }
}
