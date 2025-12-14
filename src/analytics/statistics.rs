// Column Statistics and Histograms
//
// This module provides types and operations for maintaining column-level
// statistics used for query optimization:
//
// - **Column Statistics**: Cardinality, null fraction, min/max values
// - **Histograms**: Value distribution for selectivity estimation
// - **Most Common Values**: Frequency tracking for skewed data

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::collections::HashSet;
use std::time::SystemTime;

// =============================================================================
// Column Statistics
// =============================================================================

// Statistics for a single column.
//
// Used by the query optimizer to estimate cardinality and selectivity.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ColumnStatistics {
    // Table containing this column
    pub table_name: String,

    // Column name
    pub column_name: String,

    // Number of distinct values
    pub distinct_count: u64,

    // Number of NULL values
    pub null_count: u64,

    // Total number of rows
    pub total_count: u64,

    // Minimum value (as string)
    pub min_value: Option<String>,

    // Maximum value (as string)
    pub max_value: Option<String>,

    // Average length for string columns
    pub avg_length: f64,

    // Value distribution histogram
    pub histogram: Option<Histogram>,

    // Most common values with frequencies
    pub most_common_values: Vec<(String, u64)>,

    // When statistics were last updated
    pub last_updated: SystemTime,
    pub num_values: (),
}

impl ColumnStatistics {
    // Create new empty statistics for a column.
    pub fn new(table: String, column: String) -> Self {
        Self {
            table_name: table,
            column_name: column,
            distinct_count: 0,
            null_count: 0,
            total_count: 0,
            min_value: None,
            max_value: None,
            avg_length: 0.0,
            histogram: None,
            most_common_values: Vec::new(),
            last_updated: SystemTime::now(),
            num_values: (),
        }
    }

    // Calculate selectivity (distinct values / total rows).
    //
    // Returns 1.0 if there are no rows.
    pub fn selectivity(&self) -> f64 {
        if self.total_count == 0 {
            return 1.0;
        }
        self.distinct_count as f64 / self.total_count as f64
    }

    // Calculate null fraction.
    pub fn null_fraction(&self) -> f64 {
        if self.total_count == 0 {
            return 0.0;
        }
        self.null_count as f64 / self.total_count as f64
    }

    // Estimate the selectivity for an equality predicate.
    pub fn estimate_equality_selectivity(&self, value: &str) -> f64 {
        // Check most common values first
        for (mcv, count) in &self.most_common_values {
            if mcv == value {
                return *count as f64 / self.total_count as f64;
            }
        }

        // Use histogram if available
        if let Some(histogram) = &self.histogram {
            return histogram.estimate_selectivity(value);
        }

        // Fall back to uniform distribution assumption
        if self.distinct_count == 0 {
            return 0.0;
        }
        1.0 / self.distinct_count as f64
    }

    // Estimate the selectivity for a range predicate.
    pub fn estimate_range_selectivity(&self, lower: Option<&str>, upper: Option<&str>) -> f64 {
        if let Some(histogram) = &self.histogram {
            let lower_str = lower.unwrap_or("");
            let upper_str = upper.unwrap_or("\u{FFFF}");
            return histogram.estimate_range_selectivity(lower_str, upper_str);
        }

        // Default range selectivity
        0.3
    }

    // Check if statistics are stale.
    pub fn is_stale(&self, max_age_seconds: u64) -> bool {
        SystemTime::now()
            .duration_since(self.last_updated)
            .map(|d| d.as_secs() > max_age_seconds)
            .unwrap_or(true)
    }

    // Collect statistics from data.
    pub fn collect(&mut self, data: &[String]) {
        self.total_count = data.len() as u64;
        self.null_count = data.iter().filter(|v| v.is_empty() || *v == "NULL").count() as u64;

        let non_null: Vec<_> = data
            .iter()
            .filter(|v| !v.is_empty() && *v != "NULL")
            .collect();

        let distinct: HashSet<_> = non_null.iter().collect();
        self.distinct_count = distinct.len() as u64;

        self.min_value = non_null.iter().min().map(|v| (*v).clone());
        self.max_value = non_null.iter().max().map(|v| (*v).clone());

        let total_length: usize = non_null.iter().map(|v| v.len()).sum();
        self.avg_length = if non_null.is_empty() {
            0.0
        } else {
            total_length as f64 / non_null.len() as f64
        };

        // Collect most common values
        let mut value_counts: HashMap<&str, u64> = HashMap::new();
        for value in &non_null {
            *value_counts.entry(value.as_str()).or_insert(0) += 1;
        }

        let mut sorted: Vec<_> = value_counts.into_iter().collect();
        sorted.sort_by(|a, b| b.1.cmp(&a.1));

        self.most_common_values = sorted
            .into_iter()
            .take(10)
            .map(|(k, v)| (k.to_string(), v))
            .collect();

        self.last_updated = SystemTime::now();
    }
}

// =============================================================================
// Histogram Types
// =============================================================================

// Histogram for value distribution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Histogram {
    // Histogram buckets
    pub buckets: Vec<HistogramBucket>,

    // Type of histogram
    pub bucket_type: HistogramType,
}

// Type of histogram bucketing.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum HistogramType {
    // Equal-width buckets (same range size)
    Equiwidth,
    // Equal-depth buckets (same row count)
    Equidepth,
    // One bucket per distinct value
    Singleton,
}

// Single histogram bucket.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistogramBucket {
    // Lower bound (inclusive)
    pub lower_bound: String,

    // Upper bound (inclusive)
    pub upper_bound: String,

    // Number of rows in this bucket
    pub count: u64,

    // Number of distinct values in this bucket
    pub distinct_count: u64,
}

impl Histogram {
    // Create a new empty histogram.
    pub fn new(bucket_type: HistogramType) -> Self {
        Self {
            buckets: Vec::new(),
            bucket_type,
        }
    }

    // Estimate selectivity for an equality predicate.
    pub fn estimate_selectivity(&self, value: &str) -> f64 {
        for bucket in &self.buckets {
            let in_range = Self::value_in_range(value, &bucket.lower_bound, &bucket.upper_bound);

            if in_range {
                if bucket.count == 0 || bucket.distinct_count == 0 {
                    return 0.0;
                }
                return 1.0 / bucket.distinct_count as f64;
            }
        }
        0.0
    }

    // Estimate selectivity for a range predicate.
    pub fn estimate_range_selectivity(&self, lower: &str, upper: &str) -> f64 {
        let mut total_count = 0u64;
        let mut matched_count = 0u64;

        for bucket in &self.buckets {
            total_count += bucket.count;

            // Check if bucket overlaps with range
            if bucket.upper_bound.as_str() >= lower && bucket.lower_bound.as_str() <= upper {
                matched_count += bucket.count;
            }
        }

        if total_count == 0 {
            return 0.0;
        }

        matched_count as f64 / total_count as f64
    }

    // Check if a value is in a range.
    fn value_in_range(value: &str, lower: &str, upper: &str) -> bool {
        // Try numeric comparison first
        if let (Ok(val_num), Ok(lower_num), Ok(upper_num)) = (
            value.parse::<f64>(),
            lower.parse::<f64>(),
            upper.parse::<f64>(),
        ) {
            val_num >= lower_num && val_num <= upper_num
        } else {
            // Fall back to string comparison
            value >= lower && value <= upper
        }
    }

    // Get the total row count across all buckets.
    pub fn total_count(&self) -> u64 {
        self.buckets.iter().map(|b| b.count).sum()
    }

    // Get the number of buckets.
    pub fn num_buckets(&self) -> usize {
        self.buckets.len()
    }
}

// =============================================================================
// Histogram Manager
// =============================================================================

// Manager for building and maintaining histograms.
pub struct HistogramManager {
    // Stored histograms by key
    histograms: HashMap<String, Histogram>,

    // Whether to auto-update histograms
    pub auto_update: bool,

    // Row count threshold for auto-update
    pub update_threshold: u64,
}

impl HistogramManager {
    // Create a new histogram manager.
    pub fn new() -> Self {
        Self {
            histograms: HashMap::new(),
            auto_update: true,
            update_threshold: 1000,
        }
    }

    // Create a histogram from data.
    pub fn create_histogram(&mut self, key: String, data: Vec<String>, bucket_type: HistogramType) {
        let mut histogram = Histogram::new(bucket_type.clone());

        match bucket_type {
            HistogramType::Equiwidth => {
                self.build_equiwidth_histogram(&mut histogram, data, 10);
            }
            HistogramType::Equidepth => {
                self.build_equidepth_histogram(&mut histogram, data, 10);
            }
            HistogramType::Singleton => {
                self.build_singleton_histogram(&mut histogram, data);
            }
        }

        self.histograms.insert(key, histogram);
    }

    // Build an equiwidth histogram.
    fn build_equiwidth_histogram(
        &self,
        histogram: &mut Histogram,
        mut data: Vec<String>,
        numbuckets: usize,
    ) {
        if data.is_empty() {
            return;
        }

        data.sort();
        let min = data.first().unwrap().clone();
        let max = data.last().unwrap().clone();

        let bucket_size = data.len() / numbuckets;

        for i in 0..numbuckets {
            let start = i * bucket_size;
            let end = if i == numbuckets - 1 {
                data.len()
            } else {
                (i + 1) * bucket_size
            };

            if start < data.len() {
                let bucket_data = &data[start..end.min(data.len())];
                let distinct: HashSet<_> = bucket_data.iter().collect();

                histogram.buckets.push(HistogramBucket {
                    lower_bound: bucket_data.first().unwrap_or(&min).clone(),
                    upper_bound: bucket_data.last().unwrap_or(&max).clone(),
                    count: bucket_data.len() as u64,
                    distinct_count: distinct.len() as u64,
                });
            }
        }
    }

    // Build an equidepth histogram.
    fn build_equidepth_histogram(
        &self,
        histogram: &mut Histogram,
        mut data: Vec<String>,
        numbuckets: usize,
    ) {
        if data.is_empty() {
            return;
        }

        data.sort();
        let bucket_size = (data.len() + numbuckets - 1) / numbuckets;

        for chunk in data.chunks(bucket_size) {
            let distinct: HashSet<_> = chunk.iter().collect();

            histogram.buckets.push(HistogramBucket {
                lower_bound: chunk.first().unwrap().clone(),
                upper_bound: chunk.last().unwrap().clone(),
                count: chunk.len() as u64,
                distinct_count: distinct.len() as u64,
            });
        }
    }

    // Build a singleton histogram (one bucket per value).
    fn build_singleton_histogram(&self, histogram: &mut Histogram, data: Vec<String>) {
        let mut value_counts: HashMap<String, u64> = HashMap::new();

        for value in data {
            *value_counts.entry(value).or_insert(0) += 1;
        }

        for (value, count) in value_counts {
            histogram.buckets.push(HistogramBucket {
                lower_bound: value.clone(),
                upper_bound: value,
                count,
                distinct_count: 1,
            });
        }

        // Sort buckets by lower bound
        histogram
            .buckets
            .sort_by(|a, b| a.lower_bound.cmp(&b.lower_bound));
    }

    // Get a histogram by key.
    pub fn get_histogram(&self, key: &str) -> Option<&Histogram> {
        self.histograms.get(key)
    }

    // Remove a histogram.
    pub fn remove_histogram(&mut self, key: &str) -> Option<Histogram> {
        self.histograms.remove(key)
    }

    // List all histogram keys.
    pub fn list_histograms(&self) -> Vec<String> {
        self.histograms.keys().cloned().collect()
    }
}

impl Default for HistogramManager {
    fn default() -> Self {
        Self::new()
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_column_statistics_creation() {
        let stats = ColumnStatistics::new("users".to_string(), "age".to_string());
        assert_eq!(stats.table_name, "users");
        assert_eq!(stats.column_name, "age");
        assert_eq!(stats.selectivity(), 1.0);
    }

    #[test]
    fn test_column_statistics_collect() {
        let mut stats = ColumnStatistics::new("users".to_string(), "name".to_string());
        let data = vec![
            "Alice".to_string(),
            "Bob".to_string(),
            "Alice".to_string(),
            "Charlie".to_string(),
            "".to_string(), // NULL
        ];

        stats.collect(&data);

        assert_eq!(stats.total_count, 5);
        assert_eq!(stats.null_count, 1);
        assert_eq!(stats.distinct_count, 3);
        assert_eq!(stats.null_fraction(), 0.2);
    }

    #[test]
    fn test_selectivity() {
        let stats = ColumnStatistics {
            table_name: "users".to_string(),
            column_name: "age".to_string(),
            distinct_count: 50,
            null_count: 10,
            total_count: 100,
            min_value: Some("18".to_string()),
            max_value: Some("65".to_string()),
            avg_length: 2.0,
            histogram: None,
            most_common_values: vec![],
            last_updated: SystemTime::now(),
            num_values: (),
        };

        assert!((stats.selectivity() - 0.5).abs() < 0.001);
        assert!((stats.null_fraction() - 0.1).abs() < 0.001);
    }

    #[test]
    fn test_histogram_equidepth() {
        let mut manager = HistogramManager::new();
        let data = vec![
            "1".to_string(),
            "2".to_string(),
            "3".to_string(),
            "4".to_string(),
            "5".to_string(),
        ];

        manager.create_histogram("test".to_string(), data, HistogramType::Equidepth);

        let histogram = manager.get_histogram("test").unwrap();
        assert!(!histogram.buckets.is_empty());
        assert_eq!(histogram.total_count(), 5);
    }

    #[test]
    fn test_histogram_selectivity() {
        let mut histogram = Histogram::new(HistogramType::Equidepth);
        histogram.buckets.push(HistogramBucket {
            lower_bound: "0".to_string(),
            upper_bound: "10".to_string(),
            count: 100,
            distinct_count: 10,
        });

        let selectivity = histogram.estimate_selectivity("5");
        assert!((selectivity - 0.1).abs() < 0.001);
    }

    #[test]
    fn test_histogram_range_selectivity() {
        let mut histogram = Histogram::new(HistogramType::Equidepth);
        histogram.buckets.push(HistogramBucket {
            lower_bound: "0".to_string(),
            upper_bound: "50".to_string(),
            count: 50,
            distinct_count: 50,
        });
        histogram.buckets.push(HistogramBucket {
            lower_bound: "51".to_string(),
            upper_bound: "100".to_string(),
            count: 50,
            distinct_count: 50,
        });

        let selectivity = histogram.estimate_range_selectivity("0", "50");
        assert!((selectivity - 0.5).abs() < 0.001);
    }

    #[test]
    fn test_most_common_values() {
        let mut stats = ColumnStatistics::new("test".to_string(), "col".to_string());
        let data = vec![
            "a".to_string(),
            "b".to_string(),
            "b".to_string(),
            "c".to_string(),
            "c".to_string(),
            "c".to_string(),
        ];

        stats.collect(&data);

        assert!(!stats.most_common_values.is_empty());
        assert_eq!(stats.most_common_values[0].0, "c");
        assert_eq!(stats.most_common_values[0].1, 3);
    }
}
