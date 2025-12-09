// Window Functions for Analytics
//
// This module provides SQL window function implementations for
// performing calculations across sets of rows related to the current row:
//
// - **Ranking Functions**: ROW_NUMBER, RANK, DENSE_RANK, NTILE
// - **Value Functions**: LEAD, LAG, FIRST_VALUE, LAST_VALUE, NTH_VALUE
// - **Distribution Functions**: PERCENT_RANK, CUME_DIST

use serde::{Deserialize, Serialize};
use crate::error::{Result, DbError};

// =============================================================================
// Window Function Types
// =============================================================================

/// Window function specification.
///
/// Defines the type of window function and its parameters.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum WindowFunction {
    /// Row number within the partition
    RowNumber,

    /// Rank with gaps for ties
    Rank,

    /// Rank without gaps for ties
    DenseRank,

    /// Value from following row
    Lead {
        /// Number of rows to look ahead
        offset: usize,
        /// Default value if beyond partition
        default: Option<String>,
    },

    /// Value from preceding row
    Lag {
        /// Number of rows to look back
        offset: usize,
        /// Default value if beyond partition
        default: Option<String>,
    },

    /// First value in the window frame
    FirstValue,

    /// Last value in the window frame
    LastValue,

    /// Nth value in the window frame
    NthValue {
        /// Position (1-based)
        n: usize,
    },

    /// Divide rows into N buckets
    NTile {
        /// Number of buckets
        buckets: usize,
    },

    /// Relative rank: (rank - 1) / (total rows - 1)
    PercentRank,

    /// Cumulative distribution: rows <= current / total rows
    CumeDist,
}

// =============================================================================
// Window Frame Specification
// =============================================================================

/// Window frame specification defining the range of rows.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct WindowFrame {
    /// Type of frame: ROWS, RANGE, or GROUPS
    pub frame_type: FrameType,

    /// Start boundary of the frame
    pub start_bound: FrameBound,

    /// End boundary of the frame
    pub end_bound: FrameBound,
}

impl Default for WindowFrame {
    fn default() -> Self {
        Self {
            frame_type: FrameType::Rows,
            start_bound: FrameBound::UnboundedPreceding,
            end_bound: FrameBound::CurrentRow,
        }
    }
}

impl WindowFrame {
    /// Create a frame covering the entire partition.
    pub fn whole_partition() -> Self {
        Self {
            frame_type: FrameType::Rows,
            start_bound: FrameBound::UnboundedPreceding,
            end_bound: FrameBound::UnboundedFollowing,
        }
    }

    /// Create a sliding window of N preceding rows.
    pub fn sliding(preceding: usize) -> Self {
        Self {
            frame_type: FrameType::Rows,
            start_bound: FrameBound::Preceding(preceding),
            end_bound: FrameBound::CurrentRow,
        }
    }

    /// Create a centered window.
    pub fn centered(before: usize, after: usize) -> Self {
        Self {
            frame_type: FrameType::Rows,
            start_bound: FrameBound::Preceding(before),
            end_bound: FrameBound::Following(after),
        }
    }
}

/// Frame type determining how boundaries are interpreted.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum FrameType {
    /// Physical row offsets
    Rows,
    /// Logical value ranges
    Range,
    /// Groups of peers
    Groups,
}

/// Frame boundary specification.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum FrameBound {
    /// Start of partition
    UnboundedPreceding,
    /// N rows/values before current
    Preceding(usize),
    /// Current row
    CurrentRow,
    /// N rows/values after current
    Following(usize),
    /// End of partition
    UnboundedFollowing,
}

// =============================================================================
// Window Function Execution
// =============================================================================

/// Apply a window function to data.
///
/// # Arguments
/// * `data` - The input data rows
/// * `partition_by` - Column indices for partitioning
/// * `order_by` - Column indices for ordering
/// * `function` - The window function to apply
/// * `value_column` - Column index for value-based functions (LEAD, LAG, etc.)
///
/// # Returns
/// A vector of computed values, one per input row.
pub fn apply_window_function(
    data: &[Vec<String>],
    _partition_by: &[usize],
    order_by: &[usize],
    function: &WindowFunction,
    value_column: usize,
) -> Result<Vec<String>> {
    let mut result = Vec::with_capacity(data.len());

    match function {
        WindowFunction::RowNumber => {
            for i in 0..data.len() {
                result.push((i + 1).to_string());
            }
        }

        WindowFunction::Rank => {
            result = compute_rank(data, order_by, false);
        }

        WindowFunction::DenseRank => {
            result = compute_rank(data, order_by, true);
        }

        WindowFunction::Lead { offset, default } => {
            for i in 0..data.len() {
                let lead_index = i + offset;
                let value = if lead_index < data.len() {
                    data[lead_index]
                        .get(value_column)
                        .cloned()
                        .unwrap_or_else(|| "NULL".to_string())
                } else {
                    default.clone().unwrap_or_else(|| "NULL".to_string())
                };
                result.push(value);
            }
        }

        WindowFunction::Lag { offset, default } => {
            for i in 0..data.len() {
                let value = if i >= *offset {
                    data[i - offset]
                        .get(value_column)
                        .cloned()
                        .unwrap_or_else(|| "NULL".to_string())
                } else {
                    default.clone().unwrap_or_else(|| "NULL".to_string())
                };
                result.push(value);
            }
        }

        WindowFunction::FirstValue => {
            let first = data
                .first()
                .and_then(|r| r.get(value_column))
                .cloned()
                .unwrap_or_else(|| "NULL".to_string());
            result = vec![first; data.len()];
        }

        WindowFunction::LastValue => {
            let last = data
                .last()
                .and_then(|r| r.get(value_column))
                .cloned()
                .unwrap_or_else(|| "NULL".to_string());
            result = vec![last; data.len()];
        }

        WindowFunction::NthValue { n } => {
            let nth = if *n > 0 && *n <= data.len() {
                data[n - 1]
                    .get(value_column)
                    .cloned()
                    .unwrap_or_else(|| "NULL".to_string())
            } else {
                "NULL".to_string()
            };
            result = vec![nth; data.len()];
        }

        WindowFunction::NTile { buckets } => {
            let bucket_size = (data.len() + buckets - 1) / buckets;
            for i in 0..data.len() {
                let bucket = (i / bucket_size) + 1;
                result.push(bucket.min(*buckets).to_string());
            }
        }

        WindowFunction::PercentRank => {
            let n = data.len();
            for i in 0..n {
                let percent_rank = if n > 1 {
                    i as f64 / (n - 1) as f64
                } else {
                    0.0
                };
                result.push(format!("{:.6}", percent_rank));
            }
        }

        WindowFunction::CumeDist => {
            let n = data.len();
            for i in 0..n {
                let cume_dist = (i + 1) as f64 / n as f64;
                result.push(format!("{:.6}", cume_dist));
            }
        }
    }

    Ok(result)
}

/// Compute RANK or DENSE_RANK.
fn compute_rank(data: &[Vec<String>], order_by: &[usize], dense: bool) -> Vec<String> {
    let mut result = Vec::with_capacity(data.len());

    if dense {
        // DENSE_RANK: no gaps
        let mut dense_rank = 1;
        let mut prev_values: Option<Vec<String>> = None;

        for row in data {
            let current_values: Vec<String> = order_by
                .iter()
                .filter_map(|&i| row.get(i).cloned())
                .collect();

            if let Some(prev) = &prev_values {
                if current_values != *prev {
                    dense_rank += 1;
                }
            }

            result.push(dense_rank.to_string());
            prev_values = Some(current_values);
        }
    } else {
        // RANK: with gaps
        let mut rank = 1;
        let mut current_rank = 1;
        let mut prev_values: Option<Vec<String>> = None;

        for row in data {
            let current_values: Vec<String> = order_by
                .iter()
                .filter_map(|&i| row.get(i).cloned())
                .collect();

            if let Some(prev) = &prev_values {
                if current_values != *prev {
                    current_rank = rank;
                }
            }

            result.push(current_rank.to_string());
            prev_values = Some(current_values);
            rank += 1;
        }
    }

    result
}

// =============================================================================
// Window Function Builder
// =============================================================================

/// Builder for constructing window function specifications.
#[derive(Debug, Clone)]
pub struct WindowFunctionBuilder {
    function: Option<WindowFunction>,
    partition_by: Vec<usize>,
    order_by: Vec<usize>,
    frame: WindowFrame,
    value_column: usize,
}

impl Default for WindowFunctionBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl WindowFunctionBuilder {
    /// Create a new builder.
    pub fn new() -> Self {
        Self {
            function: None,
            partition_by: Vec::new(),
            order_by: Vec::new(),
            frame: WindowFrame::default(),
            value_column: 0,
        }
    }

    /// Set the window function.
    pub fn function(mut self, function: WindowFunction) -> Self {
        self.function = Some(function);
        self
    }

    /// Set partition columns.
    pub fn partition_by(mut self, columns: Vec<usize>) -> Self {
        self.partition_by = columns;
        self
    }

    /// Set order columns.
    pub fn order_by(mut self, columns: Vec<usize>) -> Self {
        self.order_by = columns;
        self
    }

    /// Set the window frame.
    pub fn frame(mut self, frame: WindowFrame) -> Self {
        self.frame = frame;
        self
    }

    /// Set the value column for LEAD/LAG/etc.
    pub fn value_column(mut self, column: usize) -> Self {
        self.value_column = column;
        self
    }

    /// Execute the window function.
    pub fn execute(self, data: &[Vec<String>]) -> Result<Vec<String>> {
        let function = self
            .function
            .ok_or_else(|| crate::error::DbError::Execution("No window function specified".to_string()))?;

        apply_window_function(
            data,
            &self.partition_by,
            &self.order_by,
            &function,
            self.value_column,
        )
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn make_data(values: &[&[&str]]) -> Vec<Vec<String>> {
        values
            .iter()
            .map(|row| row.iter().map(|v| v.to_string()).collect())
            .collect()
    }

    #[test]
    fn test_row_number() {
        let data = make_data(&[&["a"], &["b"], &["c"]]);
        let result = apply_window_function(&data, &[], &[], &WindowFunction::RowNumber, 0).unwrap();
        assert_eq!(result, vec!["1", "2", "3"]);
    }

    #[test]
    fn test_rank() {
        let data = make_data(&[&["1"], &["1"], &["2"]]);
        let result = apply_window_function(&data, &[], &[0], &WindowFunction::Rank, 0).unwrap();
        assert_eq!(result, vec!["1", "1", "3"]);
    }

    #[test]
    fn test_dense_rank() {
        let data = make_data(&[&["1"], &["1"], &["2"]]);
        let result = apply_window_function(&data, &[], &[0], &WindowFunction::DenseRank, 0).unwrap();
        assert_eq!(result, vec!["1", "1", "2"]);
    }

    #[test]
    fn test_lead() {
        let data = make_data(&[&["a"], &["b"], &["c"]]);
        let result = apply_window_function(
            &data,
            &[],
            &[],
            &WindowFunction::Lead {
                offset: 1,
                default: Some("X".to_string()),
            },
            0,
        )
        .unwrap();
        assert_eq!(result, vec!["b", "c", "X"]);
    }

    #[test]
    fn test_lag() {
        let data = make_data(&[&["a"], &["b"], &["c"]]);
        let result = apply_window_function(
            &data,
            &[],
            &[],
            &WindowFunction::Lag {
                offset: 1,
                default: Some("X".to_string()),
            },
            0,
        )
        .unwrap();
        assert_eq!(result, vec!["X", "a", "b"]);
    }

    #[test]
    fn test_ntile() {
        let data = make_data(&[&["1"], &["2"], &["3"], &["4"]]);
        let result = apply_window_function(
            &data,
            &[],
            &[],
            &WindowFunction::NTile { buckets: 2 },
            0,
        )
        .unwrap();
        assert_eq!(result, vec!["1", "1", "2", "2"]);
    }

    #[test]
    fn test_window_frame() {
        let frame = WindowFrame::sliding(3);
        assert_eq!(frame.start_bound, FrameBound::Preceding(3));
        assert_eq!(frame.end_bound, FrameBound::CurrentRow);
    }

    #[test]
    fn test_window_builder() {
        let data = make_data(&[&["1"], &["2"], &["3"]]);
        let result = WindowFunctionBuilder::new()
            .function(WindowFunction::RowNumber)
            .execute(&data)
            .unwrap();
        assert_eq!(result, vec!["1", "2", "3"]);
    }
}
