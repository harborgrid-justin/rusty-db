// Data Profiler
// Analyzes data to extract statistics, patterns, and detect anomalies

use crate::common::{Schema, Tuple, Value};
use crate::error::{DbError, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use regex::Regex;

/// Data type classification for profiling
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DataType {
    Numeric,
    String,
    Date,
    Boolean,
    Json,
    Binary,
    Unknown,
}

/// Column statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ColumnStatistics {
    pub count: usize,
    pub null_count: usize,
    pub distinct_count: usize,
    pub min_value: Option<Value>,
    pub max_value: Option<Value>,
    pub mean: Option<f64>,
    pub median: Option<f64>,
    pub stddev: Option<f64>,
    pub variance: Option<f64>,
    pub sum: Option<f64>,
}

impl Default for ColumnStatistics {
    fn default() -> Self {
        Self {
            count: 0,
            null_count: 0,
            distinct_count: 0,
            min_value: None,
            max_value: None,
            mean: None,
            median: None,
            stddev: None,
            variance: None,
            sum: None,
        }
    }
}

/// Value distribution information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValueDistribution {
    pub total_values: usize,
    pub unique_values: usize,
    pub top_values: Vec<(String, usize)>,
    pub null_percentage: f64,
    pub cardinality: f64,
}

impl Default for ValueDistribution {
    fn default() -> Self {
        Self {
            total_values: 0,
            unique_values: 0,
            top_values: Vec::new(),
            null_percentage: 0.0,
            cardinality: 0.0,
        }
    }
}

/// Pattern detection result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PatternInfo {
    pub pattern_type: String,
    pub pattern: String,
    pub match_count: usize,
    pub match_percentage: f64,
    pub examples: Vec<String>,
}

/// Anomaly detection result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnomalyInfo {
    pub anomaly_type: String,
    pub description: String,
    pub severity: f64,
    pub affected_rows: Vec<u64>,
    pub recommendation: String,
}

/// Column profile result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ColumnProfile {
    pub column_name: String,
    pub data_type: DataType,
    pub statistics: ColumnStatistics,
    pub distribution: ValueDistribution,
    pub patterns: Vec<PatternInfo>,
    pub anomalies: Vec<AnomalyInfo>,
}

/// Profile result for a complete table
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProfileResult {
    pub table_name: String,
    pub row_count: usize,
    pub column_profiles: Vec<ColumnProfile>,
    pub timestamp: std::time::SystemTime,
}

/// Data profiler engine
pub struct DataProfiler {
    max_top_values: usize,
    anomaly_threshold: f64,
}

impl DataProfiler {
    pub fn new() -> Self {
        Self {
            max_top_values: 10,
            anomaly_threshold: 3.0, // Standard deviations
        }
    }

    pub fn with_config(max_top_values: usize, anomaly_threshold: f64) -> Self {
        Self {
            max_top_values,
            anomaly_threshold,
        }
    }

    /// Profile a complete table
    pub fn profile_table(
        &mut self,
        table_name: &str,
        schema: &Schema,
        data: &[Tuple],
    ) -> Result<Vec<ColumnProfile>> {
        let mut profiles = Vec::new();

        for (col_idx, column_def) in schema.columns.iter().enumerate() {
            let profile = self.profile_column(
                &column_def.name,
                data,
                col_idx,
            )?;
            profiles.push(profile);
        }

        Ok(profiles)
    }

    /// Profile a single column
    pub fn profile_column(
        &self,
        column_name: &str,
        data: &[Tuple],
        column_index: usize,
    ) -> Result<ColumnProfile> {
        // Collect values
        let values: Vec<&Value> = data
            .iter()
            .filter_map(|tuple| tuple.get(column_index))
            .collect();

        // Determine data type
        let data_type = self.infer_data_type(&values);

        // Calculate statistics
        let statistics = self.calculate_statistics(&values)?;

        // Analyze distribution
        let distribution = self.analyze_distribution(&values)?;

        // Detect patterns
        let patterns = self.detect_patterns(&values)?;

        // Detect anomalies
        let anomalies = self.detect_anomalies(&values, &statistics, data)?;

        Ok(ColumnProfile {
            column_name: column_name.to_string(),
            data_type,
            statistics,
            distribution,
            patterns,
            anomalies,
        })
    }

    /// Infer data type from values
    fn infer_data_type(&self, values: &[&Value]) -> DataType {
        if values.is_empty() {
            return DataType::Unknown;
        }

        // Count types
        let mut numeric_count = 0;
        let mut string_count = 0;
        let mut date_count = 0;
        let mut boolean_count = 0;
        let mut json_count = 0;
        let mut binary_count = 0;

        for value in values {
            match value {
                Value::Integer(_) | Value::Float(_) => numeric_count += 1,
                Value::String(_) => string_count += 1,
                Value::Date(_) | Value::Timestamp(_) => date_count += 1,
                Value::Boolean(_) => boolean_count += 1,
                Value::Json(_) => json_count += 1,
                Value::Bytes(_) => binary_count += 1,
                _ => {}
            }
        }

        // Return most common type
        let max_count = numeric_count.max(string_count)
            .max(date_count)
            .max(boolean_count)
            .max(json_count)
            .max(binary_count);

        if max_count == numeric_count {
            DataType::Numeric
        } else if max_count == string_count {
            DataType::String
        } else if max_count == date_count {
            DataType::Date
        } else if max_count == boolean_count {
            DataType::Boolean
        } else if max_count == json_count {
            DataType::Json
        } else if max_count == binary_count {
            DataType::Binary
        } else {
            DataType::Unknown
        }
    }

    /// Calculate column statistics
    fn calculate_statistics(&self, values: &[&Value]) -> Result<ColumnStatistics> {
        let mut stats = ColumnStatistics::default();
        stats.count = values.len();

        if values.is_empty() {
            return Ok(stats);
        }

        // Count nulls and distinct values
        let mut distinct_values = std::collections::HashSet::new();
        let mut numeric_values = Vec::new();

        for value in values {
            if value.is_null() {
                stats.null_count += 1;
            } else {
                distinct_values.insert(value.to_display_string());

                // Collect numeric values for statistical calculations
                match value {
                    Value::Integer(i) => numeric_values.push(*i as f64),
                    Value::Float(f) => numeric_values.push(*f),
                    _ => {}
                }
            }
        }

        stats.distinct_count = distinct_values.len();

        // Find min/max
        let non_null_values: Vec<&Value> = values.iter()
            .filter(|v| !v.is_null())
            .copied()
            .collect();

        if !non_null_values.is_empty() {
            stats.min_value = non_null_values.iter().min().map(|&v| v.clone());
            stats.max_value = non_null_values.iter().max().map(|&v| v.clone());
        }

        // Calculate numeric statistics
        if !numeric_values.is_empty() {
            let n = numeric_values.len() as f64;
            let sum: f64 = numeric_values.iter().sum();
            let mean = sum / n;

            stats.sum = Some(sum);
            stats.mean = Some(mean);

            // Calculate variance and standard deviation
            let variance: f64 = numeric_values.iter()
                .map(|&x| (x - mean).powi(2))
                .sum::<f64>() / n;

            stats.variance = Some(variance);
            stats.stddev = Some(variance.sqrt());

            // Calculate median
            let mut sorted = numeric_values.clone();
            sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
            let mid = sorted.len() / 2;
            stats.median = if sorted.len() % 2 == 0 {
                Some((sorted[mid - 1] + sorted[mid]) / 2.0)
            } else {
                Some(sorted[mid])
            };
        }

        Ok(stats)
    }

    /// Analyze value distribution
    fn analyze_distribution(&self, values: &[&Value]) -> Result<ValueDistribution> {
        let mut dist = ValueDistribution::default();
        dist.total_values = values.len();

        if values.is_empty() {
            return Ok(dist);
        }

        // Count value frequencies
        let mut frequency_map: HashMap<String, usize> = HashMap::new();
        let mut null_count = 0;

        for value in values {
            if value.is_null() {
                null_count += 1;
            } else {
                let key = value.to_display_string();
                *frequency_map.entry(key).or_insert(0) += 1;
            }
        }

        dist.unique_values = frequency_map.len();
        dist.null_percentage = (null_count as f64 / values.len() as f64) * 100.0;
        dist.cardinality = dist.unique_values as f64 / values.len() as f64;

        // Get top values
        let mut frequencies: Vec<(String, usize)> = frequency_map.into_iter().collect();
        frequencies.sort_by(|a, b| b.1.cmp(&a.1));
        dist.top_values = frequencies.into_iter()
            .take(self.max_top_values)
            .collect();

        Ok(dist)
    }

    /// Detect common patterns in string values
    fn detect_patterns(&self, values: &[&Value]) -> Result<Vec<PatternInfo>> {
        let mut patterns = Vec::new();

        // Collect string values
        let string_values: Vec<String> = values.iter()
            .filter_map(|v| match v {
                Value::String(s) => Some(s.clone()),
                _ => None,
            })
            .collect();

        if string_values.is_empty() {
            return Ok(patterns);
        }

        // Common patterns to check
        let pattern_checks = vec![
            ("Email", r"^[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\.[a-zA-Z]{2,}$"),
            ("Phone (US)", r"^\d{3}-\d{3}-\d{4}$"),
            ("Phone (International)", r"^\+\d{1,3}\s?\d{6,14}$"),
            ("ZIP Code", r"^\d{5}(-\d{4})?$"),
            ("SSN", r"^\d{3}-\d{2}-\d{4}$"),
            ("Credit Card", r"^\d{4}[\s-]?\d{4}[\s-]?\d{4}[\s-]?\d{4}$"),
            ("URL", r"^https?://[^\s]+$"),
            ("IPv4", r"^\d{1,3}\.\d{1,3}\.\d{1,3}\.\d{1,3}$"),
            ("Date (ISO)", r"^\d{4}-\d{2}-\d{2}$"),
            ("UUID", r"^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$"),
        ];

        for (pattern_name, pattern_str) in pattern_checks {
            if let Ok(regex) = Regex::new(pattern_str) {
                let mut match_count = 0;
                let mut examples = Vec::new();

                for value in &string_values {
                    if regex.is_match(value) {
                        match_count += 1;
                        if examples.len() < 3 {
                            examples.push(value.clone());
                        }
                    }
                }

                if match_count > 0 {
                    let match_percentage = (match_count as f64 / string_values.len() as f64) * 100.0;
                    patterns.push(PatternInfo {
                        pattern_type: pattern_name.to_string(),
                        pattern: pattern_str.to_string(),
                        match_count,
                        match_percentage,
                        examples,
                    });
                }
            }
        }

        Ok(patterns)
    }

    /// Detect anomalies in the data
    fn detect_anomalies(
        &self,
        values: &[&Value],
        statistics: &ColumnStatistics,
        data: &[Tuple],
    ) -> Result<Vec<AnomalyInfo>> {
        let mut anomalies = Vec::new();

        // Check for high null percentage
        if values.is_empty() {
            return Ok(anomalies);
        }

        let null_percentage = (statistics.null_count as f64 / statistics.count as f64) * 100.0;
        if null_percentage > 50.0 {
            anomalies.push(AnomalyInfo {
                anomaly_type: "High Null Percentage".to_string(),
                description: format!(
                    "{:.2}% of values are NULL, which may indicate data quality issues",
                    null_percentage
                ),
                severity: null_percentage / 100.0,
                affected_rows: vec![],
                recommendation: "Review data collection process and consider data validation rules".to_string(),
            });
        }

        // Check for low cardinality in large datasets
        if statistics.count > 1000 && statistics.distinct_count < 10 {
            let cardinality = statistics.distinct_count as f64 / statistics.count as f64;
            anomalies.push(AnomalyInfo {
                anomaly_type: "Low Cardinality".to_string(),
                description: format!(
                    "Only {} distinct values in {} rows (cardinality: {:.4})",
                    statistics.distinct_count, statistics.count, cardinality
                ),
                severity: 1.0 - cardinality,
                affected_rows: vec![],
                recommendation: "Consider if this column should be an enum or categorical type".to_string(),
            });
        }

        // Detect outliers using IQR method for numeric data
        if let (Some(mean), Some(stddev)) = (statistics.mean, statistics.stddev) {
            if stddev > 0.0 {
                let mut outlier_rows = Vec::new();

                for (idx, value) in values.iter().enumerate() {
                    if let Some(numeric_val) = match value {
                        Value::Integer(i) => Some(*i as f64),
                        Value::Float(f) => Some(*f),
                        _ => None,
                    } {
                        let z_score = (numeric_val - mean).abs() / stddev;
                        if z_score > self.anomaly_threshold {
                            if let Some(tuple) = data.get(idx) {
                                outlier_rows.push(tuple.row_id);
                            }
                        }
                    }
                }

                if !outlier_rows.is_empty() {
                    anomalies.push(AnomalyInfo {
                        anomaly_type: "Statistical Outliers".to_string(),
                        description: format!(
                            "Found {} values that are {} standard deviations from the mean",
                            outlier_rows.len(), self.anomaly_threshold
                        ),
                        severity: 0.5,
                        affected_rows: outlier_rows.clone(),
                        recommendation: "Review outlier values to determine if they are valid or errors".to_string(),
                    });
                }
            }
        }

        // Check for all unique values (potential key column)
        if statistics.distinct_count == statistics.count && statistics.null_count == 0 {
            anomalies.push(AnomalyInfo {
                anomaly_type: "Potential Primary Key".to_string(),
                description: "All values are unique and non-null".to_string(),
                severity: 0.0,
                affected_rows: vec![],
                recommendation: "Consider using this column as a primary key or unique index".to_string(),
            });
        }

        Ok(anomalies)
    }
}

impl Default for DataProfiler {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_profiler_creation() {
        let profiler = DataProfiler::new();
        assert_eq!(profiler.max_top_values, 10);
        assert_eq!(profiler.anomaly_threshold, 3.0);
    }

    #[test]
    fn test_data_type_inference() {
        let profiler = DataProfiler::new();
        let values = vec![&Value::Integer(1), &Value::Integer(2), &Value::Integer(3)];
        let data_type = profiler.infer_data_type(&values);
        assert_eq!(data_type, DataType::Numeric);
    }

    #[test]
    fn test_statistics_calculation() {
        let profiler = DataProfiler::new();
        let values = vec![
            &Value::Integer(1),
            &Value::Integer(2),
            &Value::Integer(3),
            &Value::Integer(4),
            &Value::Integer(5),
        ];
        let stats = profiler.calculate_statistics(&values).unwrap();
        assert_eq!(stats.count, 5);
        assert_eq!(stats.null_count, 0);
        assert_eq!(stats.mean, Some(3.0));
    }
}
