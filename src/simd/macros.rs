// SIMD Utility Macros
//
// This module provides reusable macros for common SIMD patterns,
// eliminating code duplication across filter and aggregate operations.

/// Handle remainder elements with scalar code after SIMD processing
///
/// This macro consolidates the duplicative remainder handling pattern
/// that appears 23+ times across filter.rs and aggregate.rs.
///
/// # Arguments
/// * `$data` - Input data slice
/// * `$value` - Comparison value
/// * `$op` - Comparison operator (==, <, >, <=, >=, !=)
/// * `$remainder_start` - Starting index for remainder processing
/// * `$len` - Total length of data
/// * `$result` - Result bitmask array
/// * `$chunks` - Current chunk index
///
/// # Example
/// ```ignore
/// simd_remainder!(data, value, ==, remainder_start, len, result, chunks);
/// ```
#[macro_export]
macro_rules! simd_remainder {
    ($data:expr, $value:expr, $op:tt, $remainder_start:expr, $len:expr, $result:expr, $chunks:expr) => {
        if $remainder_start < $len && $chunks < $result.len() {
            let mut remainder_mask = 0u8;
            for j in 0..($len - $remainder_start) {
                if $data[$remainder_start + j] $op $value {
                    remainder_mask |= 1 << j;
                }
            }
            $result[$chunks] = remainder_mask;
        }
    };
}

/// Handle remainder elements for aggregate operations (sum/min/max)
///
/// This macro handles the scalar tail processing for aggregation operations
/// where no comparison is needed, just accumulation.
///
/// # Arguments
/// * `$data` - Input data slice
/// * `$remainder_start` - Starting index for remainder processing
/// * `$len` - Total length of data
/// * `$accumulator` - Accumulator variable to update
/// * `$op` - Operation to perform (sum uses +=, min uses .min(), max uses .max())
///
/// # Example
/// ```ignore
/// simd_aggregate_remainder!(data, remainder_start, len, sum, +=);
/// ```
#[macro_export]
macro_rules! simd_aggregate_remainder {
    // For sum operations
    ($data:expr, $remainder_start:expr, $len:expr, $accumulator:expr, +=) => {
        for i in $remainder_start..$len {
            $accumulator += $data[i];
        }
    };
    // For min operations
    ($data:expr, $remainder_start:expr, $len:expr, $accumulator:expr, min) => {
        for i in $remainder_start..$len {
            if $data[i] < $accumulator {
                $accumulator = $data[i];
            }
        }
    };
    // For max operations
    ($data:expr, $remainder_start:expr, $len:expr, $accumulator:expr, max) => {
        for i in $remainder_start..$len {
            if $data[i] > $accumulator {
                $accumulator = $data[i];
            }
        }
    };
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_simd_remainder_eq() {
        let data = vec![1i32, 2, 3, 4, 5];
        let value = 3i32;
        let mut result = vec![0u8; 1];
        let chunks = 0;
        let remainder_start = 0;
        let len = 5;

        simd_remainder!(data, value, ==, remainder_start, len, result, chunks);

        // Bit 2 should be set (value 3 at index 2)
        assert_eq!(result[0] & (1 << 2), 1 << 2);
    }

    #[test]
    fn test_simd_aggregate_remainder_sum() {
        let data = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let mut sum = 0.0;
        simd_aggregate_remainder!(data, 0, data.len(), sum, +=);
        assert_eq!(sum, 15.0);
    }

    #[test]
    fn test_simd_aggregate_remainder_min() {
        let data = vec![5.0, 2.0, 8.0, 1.0, 9.0];
        let mut min_val = f64::INFINITY;
        simd_aggregate_remainder!(data, 0, data.len(), min_val, min);
        assert_eq!(min_val, 1.0);
    }

    #[test]
    fn test_simd_aggregate_remainder_max() {
        let data = vec![5.0, 2.0, 8.0, 1.0, 9.0];
        let mut max_val = f64::NEG_INFINITY;
        simd_aggregate_remainder!(data, 0, data.len(), max_val, max);
        assert_eq!(max_val, 9.0);
    }
}
