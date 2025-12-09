/// Window Functions Engine
///
/// This module provides comprehensive support for SQL window functions:
/// - OVER clause with PARTITION BY and ORDER BY
/// - Frame specifications (ROWS, RANGE, GROUPS)
/// - Running aggregates (SUM, AVG, COUNT, etc.)
/// - Ranking functions (ROW_NUMBER, RANK, DENSE_RANK)
/// - Value functions (LEAD, LAG, FIRST_VALUE, LAST_VALUE, NTH_VALUE)
/// - Distribution functions (PERCENT_RANK, CUME_DIST, NTILE)

use crate::error::Result;
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
use std::collections::HashMap;

/// Window function specification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum WindowFunction {
    // Ranking functions
    RowNumber,
    Rank,
    DenseRank,
    PercentRank,
    CumeDist,
    Ntile { buckets: usize },

    // Value functions
    Lead { offset: usize, default: Option<String> },
    Lag { offset: usize, default: Option<String> },
    FirstValue { column: String },
    LastValue { column: String },
    NthValue { column: String, n: usize },

    // Aggregate functions
    Sum { column: String },
    Avg { column: String },
    Count { column: Option<String> },
    Min { column: String },
    Max { column: String },
    StdDev { column: String },
    Variance { column: String },
}

/// Window specification (OVER clause)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WindowSpec {
    /// PARTITION BY columns
    pub partition_by: Vec<String>,
    /// ORDER BY columns and directions
    pub order_by: Vec<OrderByColumn>,
    /// Frame specification
    pub frame: Option<WindowFrame>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderByColumn {
    pub column: String,
    pub direction: SortDirection,
    pub nulls: NullsOrdering,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SortDirection {
    Ascending,
    Descending,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum NullsOrdering {
    First,
    Last,
}

/// Window frame specification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WindowFrame {
    /// Frame type (ROWS, RANGE, or GROUPS)
    pub frame_type: FrameType,
    /// Frame start boundary
    pub start_bound: FrameBound,
    /// Frame end boundary
    pub end_bound: FrameBound,
    /// Exclusion clause
    pub exclusion: FrameExclusion,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FrameType {
    /// Physical rows
    Rows,
    /// Logical range based on ORDER BY values
    Range,
    /// Peer groups (rows with same ORDER BY values)
    Groups,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FrameBound {
    /// UNBOUNDED PRECEDING
    UnboundedPreceding,
    /// N PRECEDING
    Preceding(usize),
    /// CURRENT ROW
    CurrentRow,
    /// N FOLLOWING
    Following(usize),
    /// UNBOUNDED FOLLOWING
    UnboundedFollowing,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FrameExclusion {
    NoOthers,
    CurrentRow,
    Group,
    Ties,
}

/// Window function executor
pub struct WindowExecutor {
    /// Input data rows
    rows: Vec<Row>,
    /// Window specifications by name
    windows: HashMap<String, WindowSpec>,
}

impl WindowExecutor {
    pub fn new() -> Self {
        Self {
            rows: Vec::new(),
            windows: HashMap::new(),
        }
    }

    /// Add window specification
    pub fn add_window(&mut self, name: String, spec: WindowSpec) {
        self.windows.insert(name, spec);
    }

    /// Set input rows
    pub fn set_rows(&mut self, rows: Vec<Row>) {
        self.rows = rows;
    }

    /// Execute window function
    pub fn execute(
        &self,
        function: &WindowFunction,
        spec: &WindowSpec,
    ) -> Result<Vec<String>> {
        // Partition rows
        let partitions = self.partition_rows(spec)?;

        // Execute function on each partition
        let mut results = vec![String::new(); self.rows.len()];

        for partition in partitions {
            let partition_results = self.execute_on_partition(
                function,
                spec,
                &partition,
            )?;

            for (row_idx, result) in partition_results {
                results[row_idx] = result;
            }
        }

        Ok(results)
    }

    /// Partition rows by PARTITION BY columns
    fn partition_rows(&self, spec: &WindowSpec) -> Result<Vec<Partition>> {
        if spec.partition_by.is_empty() {
            // No partitioning - all rows in one partition
            return Ok(vec![Partition {
                rows: (0..self.rows.len()).collect(),
            }]);
        }

        let mut partitions: HashMap<Vec<String>, Vec<usize>> = HashMap::new();

        for (idx, row) in self.rows.iter().enumerate() {
            let key: Vec<String> = spec.partition_by.iter()
                .map(|col| row.get_value(col).unwrap_or_default())
                .collect();

            partitions.entry(key)
                .or_insert_with(Vec::new)
                .push(idx);
        }

        Ok(partitions.into_values()
            .map(|rows| Partition { rows })
            .collect())
    }

    /// Execute function on a single partition
    fn execute_on_partition(
        &self,
        function: &WindowFunction,
        spec: &WindowSpec,
        partition: &Partition,
    ) -> Result<Vec<(usize, String)>> {
        // Sort partition by ORDER BY
        let sorted_indices = self.sort_partition(spec, partition)?;

        match function {
            WindowFunction::RowNumber => self.row_number(&sorted_indices),
            WindowFunction::Rank => self.rank(spec, &sorted_indices),
            WindowFunction::DenseRank => self.dense_rank(spec, &sorted_indices),
            WindowFunction::PercentRank => self.percent_rank(spec, &sorted_indices),
            WindowFunction::CumeDist => self.cume_dist(spec, &sorted_indices),
            WindowFunction::Ntile { buckets } => self.ntile(*buckets, &sorted_indices),
            WindowFunction::Lead { offset, default } =>
                self.lead(spec, &sorted_indices, *offset, default),
            WindowFunction::Lag { offset, default } =>
                self.lag(spec, &sorted_indices, *offset, default),
            WindowFunction::FirstValue { column } =>
                self.first_value(spec, &sorted_indices, column),
            WindowFunction::LastValue { column } =>
                self.last_value(spec, &sorted_indices, column),
            WindowFunction::NthValue { column, n } =>
                self.nth_value(spec, &sorted_indices, column, *n),
            WindowFunction::Sum { column } =>
                self.windowed_sum(spec, &sorted_indices, column),
            WindowFunction::Avg { column } =>
                self.windowed_avg(spec, &sorted_indices, column),
            WindowFunction::Count { column } =>
                self.windowed_count(spec, &sorted_indices, column),
            WindowFunction::Min { column } =>
                self.windowed_min(spec, &sorted_indices, column),
            WindowFunction::Max { column } =>
                self.windowed_max(spec, &sorted_indices, column),
            WindowFunction::StdDev { column } =>
                self.windowed_stddev(spec, &sorted_indices, column),
            WindowFunction::Variance { column } =>
                self.windowed_variance(spec, &sorted_indices, column),
        }
    }

    /// Sort partition by ORDER BY
    fn sort_partition(
        &self,
        spec: &WindowSpec,
        partition: &Partition,
    ) -> Result<Vec<usize>> {
        let mut indices = partition.rows.clone();

        if spec.order_by.is_empty() {
            return Ok(indices);
        }

        indices.sort_by(|&a, &b| {
            for order_col in &spec.order_by {
                let val_a = self.rows[a].get_value(&order_col.column).unwrap_or_default();
                let val_b = self.rows[b].get_value(&order_col.column).unwrap_or_default();

                let cmp = val_a.cmp(&val_b);
                let cmp = match order_col.direction {
                    SortDirection::Ascending => cmp,
                    SortDirection::Descending => cmp.reverse(),
                };

                if cmp != Ordering::Equal {
                    return cmp;
                }
            }
            Ordering::Equal
        });

        Ok(indices)
    }

    /// Get frame for current row
    fn get_frame(
        &self,
        spec: &WindowSpec,
        sorted_indices: &[usize],
        current_pos: usize,
    ) -> Result<(usize, usize)> {
        let frame = spec.frame.as_ref().ok_or_else(|| {
            DbError::InvalidInput("Frame specification required".to_string())
        })?;

        let start = match frame.start_bound {
            FrameBound::UnboundedPreceding => 0,
            FrameBound::Preceding(n) => current_pos.saturating_sub(n),
            FrameBound::CurrentRow => current_pos,
            FrameBound::Following(n) => (current_pos + n).min(sorted_indices.len() - 1),
            FrameBound::UnboundedFollowing => sorted_indices.len() - 1,
        };

        let end = match frame.end_bound {
            FrameBound::UnboundedPreceding => 0,
            FrameBound::Preceding(n) => current_pos.saturating_sub(n),
            FrameBound::CurrentRow => current_pos,
            FrameBound::Following(n) => (current_pos + n).min(sorted_indices.len() - 1),
            FrameBound::UnboundedFollowing => sorted_indices.len() - 1,
        };

        Ok((start, end + 1))
    }

    // Ranking functions

    fn row_number(&self, sorted_indices: &[usize]) -> Result<Vec<(usize, String)>> {
        Ok(sorted_indices.iter()
            .enumerate()
            .map(|(i, &idx)| (idx, (i + 1).to_string()))
            .collect())
    }

    fn rank(&self, spec: &WindowSpec, sorted_indices: &[usize]) -> Result<Vec<(usize, String)>> {
        let mut results = Vec::new();
        let mut current_rank = 1;
        let mut rows_at_rank = 0;

        for i in 0..sorted_indices.len() {
            rows_at_rank += 1;

            // Check if next row has different ORDER BY values
            let is_last_in_group = i + 1 >= sorted_indices.len()
                || !self.rows_equal_on_order_by(
                    spec,
                    sorted_indices[i],
                    sorted_indices[i + 1],
                );

            results.push((sorted_indices[i], current_rank.to_string()));

            if is_last_in_group {
                current_rank += rows_at_rank;
                rows_at_rank = 0;
            }
        }

        Ok(results)
    }

    fn dense_rank(&self, spec: &WindowSpec, sorted_indices: &[usize]) -> Result<Vec<(usize, String)>> {
        let mut results = Vec::new();
        let mut current_rank = 1;

        for i in 0..sorted_indices.len() {
            results.push((sorted_indices[i], current_rank.to_string()));

            // Check if next row has different ORDER BY values
            if i + 1 < sorted_indices.len()
                && !self.rows_equal_on_order_by(
                    spec,
                    sorted_indices[i],
                    sorted_indices[i + 1],
                ) {
                current_rank += 1;
            }
        }

        Ok(results)
    }

    fn percent_rank(&self, spec: &WindowSpec, sorted_indices: &[usize]) -> Result<Vec<(usize, String)>> {
        let n = sorted_indices.len();
        if n <= 1 {
            return Ok(sorted_indices.iter()
                .map(|&idx| (idx, "0".to_string()))
                .collect());
        }

        let rank_results = self.rank(spec, sorted_indices)?;

        Ok(rank_results.into_iter()
            .map(|(idx, rank_str)| {
                let rank: usize = rank_str.parse().unwrap_or(1);
                let percent = (rank - 1) as f64 / (n - 1) as f64;
                (idx, format!("{:.6}", percent))
            })
            .collect())
    }

    fn cume_dist(&self, spec: &WindowSpec, sorted_indices: &[usize]) -> Result<Vec<(usize, String)>> {
        let n = sorted_indices.len());
        let mut results = Vec::new();

        for i in 0..n {
            // Count rows with ORDER BY values <= current row
            let mut count = i + 1;

            // Include all rows with same ORDER BY values
            while count < n && self.rows_equal_on_order_by(
                spec,
                sorted_indices[i],
                sorted_indices[count],
            ) {
                count += 1;
            }

            let cume = count as f64 / n as f64;
            results.push((sorted_indices[i], format!("{:.6}", cume))));
        }

        Ok(results)
    }

    fn ntile(&self, buckets: usize, sorted_indices: &[usize]) -> Result<Vec<(usize, String)>> {
        if buckets == 0 {
            return Err(DbError::InvalidInput("Buckets must be > 0".to_string()));
        }

        let n = sorted_indices.len();
        let bucket_size = (n + buckets - 1) / buckets;

        Ok(sorted_indices.iter()
            .enumerate()
            .map(|(i, &idx)| {
                let bucket = (i / bucket_size).min(buckets - 1) + 1;
                (idx, bucket.to_string())
            })
            .collect())
    }

    // Value functions

    fn lead(
        &self,
        spec: &WindowSpec,
        sorted_indices: &[usize],
        offset: usize,
        default: &Option<String>,
    ) -> Result<Vec<(usize, String)>> {
        Ok(sorted_indices.iter()
            .enumerate()
            .map(|(i, &idx)| {
                let target_pos = i + offset;
                let value = if target_pos < sorted_indices.len() {
                    // Get value from future row
                    let target_idx = sorted_indices[target_pos];
                    spec.order_by.first()
                        .and_then(|col| self.rows[target_idx].get_value(&col.column))
                        .unwrap_or_else(|| default.clone().unwrap_or_default())
                } else {
                    default.clone().unwrap_or_default()
                };
                (idx, value)
            })
            .collect())
    }

    fn lag(
        &self,
        spec: &WindowSpec,
        sorted_indices: &[usize],
        offset: usize,
        default: &Option<String>,
    ) -> Result<Vec<(usize, String)>> {
        Ok(sorted_indices.iter()
            .enumerate()
            .map(|(i, &idx)| {
                let value = if i >= offset {
                    // Get value from previous row
                    let target_idx = sorted_indices[i - offset];
                    spec.order_by.first()
                        .and_then(|col| self.rows[target_idx].get_value(&col.column))
                        .unwrap_or_else(|| default.clone().unwrap_or_default())
                } else {
                    default.clone().unwrap_or_default()
                };
                (idx, value)
            })
            .collect())
    }

    fn first_value(
        &self,
        spec: &WindowSpec,
        sorted_indices: &[usize],
        column: &str,
    ) -> Result<Vec<(usize, String)>> {
        let first_idx = sorted_indices.first()
            .ok_or_else(|| DbError::Internal("Empty partition".to_string()))?;
        let first_value = self.rows[*first_idx].get_value(column).unwrap_or_default();

        Ok(sorted_indices.iter()
            .map(|&idx| (idx, first_value.clone()))
            .collect())
    }

    fn last_value(
        &self,
        spec: &WindowSpec,
        sorted_indices: &[usize],
        column: &str,
    ) -> Result<Vec<(usize, String)>> {
        Ok(sorted_indices.iter()
            .enumerate()
            .map(|(i, &idx)| {
                let (_, frame_end) = self.get_frame(spec, sorted_indices, i).unwrap();
                let last_idx = sorted_indices[frame_end - 1];
                let value = self.rows[last_idx].get_value(column).unwrap_or_default();
                (idx, value)
            })
            .collect())
    }

    fn nth_value(
        &self,
        spec: &WindowSpec,
        sorted_indices: &[usize],
        column: &str,
        n: usize,
    ) -> Result<Vec<(usize, String)>> {
        Ok(sorted_indices.iter()
            .enumerate()
            .map(|(i, &idx)| {
                let (frame_start, frame_end) = self.get_frame(spec, sorted_indices, i).unwrap();
                let target_pos = frame_start + n - 1;
                let value = if target_pos < frame_end {
                    let target_idx = sorted_indices[target_pos];
                    self.rows[target_idx].get_value(column).unwrap_or_default()
                } else {
                    String::new()
                };
                (idx, value)
            })
            .collect())
    }

    // Aggregate functions

    fn windowed_sum(
        &self,
        spec: &WindowSpec,
        sorted_indices: &[usize],
        column: &str,
    ) -> Result<Vec<(usize, String)>> {
        Ok(sorted_indices.iter()
            .enumerate()
            .map(|(i, &idx)| {
                let (frame_start, frame_end) = self.get_frame(spec, sorted_indices, i).unwrap();
                let sum: f64 = (frame_start..frame_end)
                    .map(|pos| {
                        let row_idx = sorted_indices[pos];
                        self.rows[row_idx].get_value(column)
                            .and_then(|v| v.parse::<f64>().ok())
                            .unwrap_or(0.0)
                    })
                    .sum();
                (idx, sum.to_string())
            })
            .collect())
    }

    fn windowed_avg(
        &self,
        spec: &WindowSpec,
        sorted_indices: &[usize],
        column: &str,
    ) -> Result<Vec<(usize, String)>> {
        Ok(sorted_indices.iter()
            .enumerate()
            .map(|(i, &idx)| {
                let (frame_start, frame_end) = self.get_frame(spec, sorted_indices, i).unwrap();
                let values: Vec<f64> = (frame_start..frame_end)
                    .filter_map(|pos| {
                        let row_idx = sorted_indices[pos];
                        self.rows[row_idx].get_value(column)
                            .and_then(|v| v.parse::<f64>().ok())
                    })
                    .collect();

                let avg = if values.is_empty() {
                    0.0
                } else {
                    values.iter().sum::<f64>() / values.len() as f64
                };
                (idx, avg.to_string())
            })
            .collect())
    }

    fn windowed_count(
        &self,
        spec: &WindowSpec,
        sorted_indices: &[usize],
        column: &Option<String>,
    ) -> Result<Vec<(usize, String)>> {
        Ok(sorted_indices.iter()
            .enumerate()
            .map(|(i, &idx)| {
                let (frame_start, frame_end) = self.get_frame(spec, sorted_indices, i).unwrap();
                let count = if let Some(col) = column {
                    (frame_start..frame_end)
                        .filter(|&pos| {
                            let row_idx = sorted_indices[pos];
                            self.rows[row_idx].get_value(col).is_some()
                        })
                        .count()
                } else {
                    frame_end - frame_start
                };
                (idx, count.to_string())
            })
            .collect())
    }

    fn windowed_min(
        &self,
        spec: &WindowSpec,
        sorted_indices: &[usize],
        column: &str,
    ) -> Result<Vec<(usize, String)>> {
        Ok(sorted_indices.iter()
            .enumerate()
            .map(|(i, &idx)| {
                let (frame_start, frame_end) = self.get_frame(spec, sorted_indices, i).unwrap();
                let min = (frame_start..frame_end)
                    .filter_map(|pos| {
                        let row_idx = sorted_indices[pos];
                        self.rows[row_idx].get_value(column)
                    })
                    .min()
                    .unwrap_or_default();
                (idx, min)
            })
            .collect())
    }

    fn windowed_max(
        &self,
        spec: &WindowSpec,
        sorted_indices: &[usize],
        column: &str,
    ) -> Result<Vec<(usize, String)>> {
        Ok(sorted_indices.iter()
            .enumerate()
            .map(|(i, &idx)| {
                let (frame_start, frame_end) = self.get_frame(spec, sorted_indices, i).unwrap();
                let max = (frame_start..frame_end)
                    .filter_map(|pos| {
                        let row_idx = sorted_indices[pos];
                        self.rows[row_idx].get_value(column)
                    })
                    .max()
                    .unwrap_or_default();
                (idx, max)
            })
            .collect())
    }

    fn windowed_stddev(
        &self,
        spec: &WindowSpec,
        sorted_indices: &[usize],
        column: &str,
    ) -> Result<Vec<(usize, String)>> {
        Ok(sorted_indices.iter()
            .enumerate()
            .map(|(i, &idx)| {
                let (frame_start, frame_end) = self.get_frame(spec, sorted_indices, i).unwrap();
                let values: Vec<f64> = (frame_start..frame_end)
                    .filter_map(|pos| {
                        let row_idx = sorted_indices[pos];
                        self.rows[row_idx].get_value(column)
                            .and_then(|v| v.parse::<f64>().ok())
                    })
                    .collect();

                let stddev = calculate_stddev(&values);
                (idx, stddev.to_string())
            })
            .collect())
    }

    fn windowed_variance(
        &self,
        spec: &WindowSpec,
        sorted_indices: &[usize],
        column: &str,
    ) -> Result<Vec<(usize, String)>> {
        Ok(sorted_indices.iter()
            .enumerate()
            .map(|(i, &idx)| {
                let (frame_start, frame_end) = self.get_frame(spec, sorted_indices, i).unwrap();
                let values: Vec<f64> = (frame_start..frame_end)
                    .filter_map(|pos| {
                        let row_idx = sorted_indices[pos];
                        self.rows[row_idx].get_value(column)
                            .and_then(|v| v.parse::<f64>().ok())
                    })
                    .collect();

                let variance = calculate_variance(&values);
                (idx, variance.to_string())
            })
            .collect())
    }

    fn rows_equal_on_order_by(
        &self,
        spec: &WindowSpec,
        idx1: usize,
        idx2: usize,
    ) -> bool {
        for order_col in &spec.order_by {
            let val1 = self.rows[idx1].get_value(&order_col.column);
            let val2 = self.rows[idx2].get_value(&order_col.column);
            if val1 != val2 {
                return false;
            }
        }
        true
    }
}

/// Row data structure
#[derive(Debug, Clone)]
pub struct Row {
    values: HashMap<String, String>,
}

impl Row {
    pub fn new() -> Self {
        Self {
            values: HashMap::new(),
        }
    }

    pub fn set_value(&mut self, column: String, value: String) {
        self.values.insert(column, value);
    }

    pub fn get_value(&self, column: &str) -> Option<String> {
        self.values.get(column).cloned()
    }
}

/// Partition of rows
struct Partition {
    rows: Vec<usize>,
}

/// Calculate standard deviation
fn calculate_stddev(values: &[f64]) -> f64 {
    if values.is_empty() {
        return 0.0;
    }

    let variance = calculate_variance(values);
    variance.sqrt()
}

/// Calculate variance
fn calculate_variance(values: &[f64]) -> f64 {
    if values.is_empty() {
        return 0.0;
    }

    let mean = values.iter().sum::<f64>() / values.len() as f64;
    let variance = values.iter()
        .map(|v| (v - mean).powi(2))
        .sum::<f64>() / values.len() as f64;

    variance
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_rows() -> Vec<Row> {
        let mut rows = Vec::new();
        for i in 0..10 {
            let mut row = Row::new();
            row.set_value("id".to_string(), i.to_string());
            row.set_value("value".to_string(), (i * 10).to_string());
            row.set_value("category".to_string(), (i % 3).to_string());
            rows.push(row);
        }
        rows
    }

    #[test]
    fn test_row_number() {
        let mut executor = WindowExecutor::new();
        executor.set_rows(create_test_rows());

        let spec = WindowSpec {
            partition_by: vec![],
            order_by: vec![OrderByColumn {
                column: "id".to_string(),
                direction: SortDirection::Ascending,
                nulls: NullsOrdering::Last,
            }],
            frame: None,
        };

        let results = executor.execute(&WindowFunction::RowNumber, &spec).unwrap();
        assert_eq!(results.len(), 10);
        assert_eq!(results[0], "1");
        assert_eq!(results[9], "10");
    }

    #[test]
    fn test_partitioning() {
        let mut executor = WindowExecutor::new();
        executor.set_rows(create_test_rows());

        let spec = WindowSpec {
            partition_by: vec!["category".to_string()],
            order_by: vec![OrderByColumn {
                column: "id".to_string(),
                direction: SortDirection::Ascending,
                nulls: NullsOrdering::Last,
            }],
            frame: None,
        };

        let results = executor.execute(&WindowFunction::RowNumber, &spec).unwrap();
        assert_eq!(results.len(), 10);
    }
}


