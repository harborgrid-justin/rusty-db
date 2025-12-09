// Data Quality Analysis and Validation
//
// This module provides comprehensive data quality analysis capabilities,
// including completeness checks, consistency validation, and quality
// metrics computation.
//
// # Architecture
//
// The quality analysis system operates on multiple dimensions:
// - Completeness: Missing value analysis
// - Consistency: Pattern and constraint validation
// - Accuracy: Outlier and anomaly detection
// - Timeliness: Data freshness tracking
//
// # Example
//
// ```rust,ignore
// use crate::analytics::quality::{DataQualityAnalyzer, QualityMetrics};
//
// let analyzer = DataQualityAnalyzer::new();
// let metrics = analyzer.analyze_column("email", &values);
//
// if metrics.completeness < 0.95 {
//     println!("Warning: {} has significant missing values", metrics.column);
// }
// ```

use std::collections::HashSet;
use parking_lot::RwLock;
use std::collections::HashMap;
use std::sync::Arc;

/// Quality dimension for analysis.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum QualityDimension {
    /// Percentage of non-null values
    Completeness,
    /// Percentage of unique values
    Uniqueness,
    /// Percentage of values matching expected pattern
    Validity,
    /// Consistency across related data
    Consistency,
    /// Accuracy compared to reference
    Accuracy,
    /// Data freshness/timeliness
    Timeliness,
}

impl QualityDimension {
    /// Returns the weight for overall quality calculation.
    pub fn default_weight(&self) -> f64 {
        match self {
            QualityDimension::Completeness => 0.25,
            QualityDimension::Uniqueness => 0.15,
            QualityDimension::Validity => 0.25,
            QualityDimension::Consistency => 0.15,
            QualityDimension::Accuracy => 0.15,
            QualityDimension::Timeliness => 0.05,
        }
    }
}

/// Quality metrics for a column or table.
#[derive(Debug, Clone)]
pub struct QualityMetrics {
    /// Column or table name
    pub name: String,
    /// Completeness score (0.0 to 1.0)
    pub completeness: f64,
    /// Uniqueness score (0.0 to 1.0)
    pub uniqueness: f64,
    /// Validity score (0.0 to 1.0)
    pub validity: f64,
    /// Consistency score (0.0 to 1.0)
    pub consistency: f64,
    /// Accuracy score (0.0 to 1.0)
    pub accuracy: f64,
    /// Timeliness score (0.0 to 1.0)
    pub timeliness: f64,
    /// Overall quality score
    pub overall_score: f64,
    /// Number of rows analyzed
    pub row_count: usize,
    /// Number of issues found
    pub issue_count: usize,
    /// Detailed issues
    pub issues: Vec<QualityIssue>,
    /// Timestamp of analysis
    pub analyzed_at: std::time::Instant,
}

impl QualityMetrics {
    /// Creates new quality metrics with default values.
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            completeness: 1.0,
            uniqueness: 1.0,
            validity: 1.0,
            consistency: 1.0,
            accuracy: 1.0,
            timeliness: 1.0,
            overall_score: 1.0,
            row_count: 0,
            issue_count: 0,
            issues: Vec::new(),
            analyzed_at: std::time::Instant::now(),
        }
    }

    /// Calculates the overall quality score.
    pub fn calculate_overall(&mut self) {
        self.overall_score = self.completeness * QualityDimension::Completeness.default_weight()
            + self.uniqueness * QualityDimension::Uniqueness.default_weight()
            + self.validity * QualityDimension::Validity.default_weight()
            + self.consistency * QualityDimension::Consistency.default_weight()
            + self.accuracy * QualityDimension::Accuracy.default_weight()
            + self.timeliness * QualityDimension::Timeliness.default_weight();

        self.issue_count = self.issues.len();
    }

    /// Returns whether quality meets the threshold.
    pub fn meets_threshold(&self, threshold: f64) -> bool {
        self.overall_score >= threshold
    }

    /// Returns the lowest dimension score.
    pub fn weakest_dimension(&self) -> (QualityDimension, f64) {
        let dimensions = [
            (QualityDimension::Completeness, self.completeness),
            (QualityDimension::Uniqueness, self.uniqueness),
            (QualityDimension::Validity, self.validity),
            (QualityDimension::Consistency, self.consistency),
            (QualityDimension::Accuracy, self.accuracy),
            (QualityDimension::Timeliness, self.timeliness),
        ];

        dimensions
            .into_iter()
            .min_by(|a, b| a.1.partial_cmp(&b.1).unwrap())
            .unwrap()
    }

    /// Adds an issue to the metrics.
    pub fn add_issue(&mut self, issue: QualityIssue) {
        self.issues.push(issue);
        self.issue_count = self.issues.len();
    }
}

/// A data quality issue.
#[derive(Debug, Clone)]
pub struct QualityIssue {
    /// Issue type/category
    pub issue_type: QualityIssueType,
    /// Severity (1-5, higher = more severe)
    pub severity: u8,
    /// Human-readable description
    pub description: String,
    /// Affected column(s)
    pub columns: Vec<String>,
    /// Number of affected rows
    pub affected_rows: usize,
    /// Example problematic values
    pub examples: Vec<String>,
    /// Suggested fix
    pub suggested_fix: Option<String>,
}

/// Types of quality issues.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum QualityIssueType {
    /// Missing/null values
    MissingValues,
    /// Duplicate values where uniqueness expected
    Duplicates,
    /// Values not matching expected pattern
    InvalidFormat,
    /// Values outside expected range
    OutOfRange,
    /// Inconsistent values across related columns
    Inconsistency,
    /// Orphaned foreign key references
    OrphanedReference,
    /// Stale or outdated data
    StaleData,
    /// Outlier values
    Outlier,
    /// Data type mismatch
    TypeMismatch,
    /// Constraint violation
    ConstraintViolation,
}

impl QualityIssueType {
    /// Returns the default severity for this issue type.
    pub fn default_severity(&self) -> u8 {
        match self {
            QualityIssueType::MissingValues => 3,
            QualityIssueType::Duplicates => 4,
            QualityIssueType::InvalidFormat => 3,
            QualityIssueType::OutOfRange => 3,
            QualityIssueType::Inconsistency => 4,
            QualityIssueType::OrphanedReference => 5,
            QualityIssueType::StaleData => 2,
            QualityIssueType::Outlier => 2,
            QualityIssueType::TypeMismatch => 4,
            QualityIssueType::ConstraintViolation => 5,
        }
    }
}

/// Validation rule for data quality.
#[derive(Debug, Clone)]
pub struct ValidationRule {
    /// Rule identifier
    pub id: String,
    /// Rule name
    pub name: String,
    /// Target column
    pub column: String,
    /// Rule type
    pub rule_type: ValidationRuleType,
    /// Parameters for the rule
    pub parameters: HashMap<String, String>,
    /// Whether rule is enabled
    pub enabled: bool,
}

/// Types of validation rules.
#[derive(Debug, Clone)]
pub enum ValidationRuleType {
    /// Value must not be null
    NotNull,
    /// Value must be unique
    Unique,
    /// Value must match regex pattern
    Pattern(String),
    /// Value must be in range
    Range { min: f64, max: f64 },
    /// Value must be in allowed list
    AllowedValues(Vec<String>),
    /// Value must reference existing value in another column
    ForeignKey { table: String, column: String },
    /// Custom SQL expression
    CustomExpression(String),
}

/// Data quality analyzer.
#[derive(Debug)]
pub struct DataQualityAnalyzer {
    /// Validation rules
    rules: Vec<ValidationRule>,
    /// Quality thresholds by dimension
    thresholds: HashMap<QualityDimension, f64>,
    /// Cached results
    cache: Arc<RwLock<HashMap<String, QualityMetrics>>>,
}

impl Default for DataQualityAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

impl DataQualityAnalyzer {
    /// Creates a new data quality analyzer.
    pub fn new() -> Self {
        let mut thresholds = HashMap::new();
        thresholds.insert(QualityDimension::Completeness, 0.95);
        thresholds.insert(QualityDimension::Uniqueness, 0.99);
        thresholds.insert(QualityDimension::Validity, 0.98);
        thresholds.insert(QualityDimension::Consistency, 0.99);
        thresholds.insert(QualityDimension::Accuracy, 0.95);
        thresholds.insert(QualityDimension::Timeliness, 0.90);

        Self {
            rules: Vec::new(),
            thresholds,
            cache: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Adds a validation rule.
    pub fn add_rule(&mut self, rule: ValidationRule) {
        self.rules.push(rule);
    }

    /// Sets a quality threshold.
    pub fn set_threshold(&mut self, dimension: QualityDimension, threshold: f64) {
        self.thresholds.insert(dimension, threshold);
    }

    /// Analyzes a column for data quality.
    pub fn analyze_column(
        &self,
        column_name: &str,
        values: &[Option<String>],
    ) -> QualityMetrics {
        let mut metrics = QualityMetrics::new(column_name);
        metrics.row_count = values.len();

        if values.is_empty() {
            return metrics;
        }

        // Completeness: non-null percentage
        let non_null_count = values.iter().filter(|v| v.is_some()).count();
        metrics.completeness = non_null_count as f64 / values.len() as f64;

        if metrics.completeness < self.thresholds.get(&QualityDimension::Completeness).copied().unwrap_or(0.95) {
            let missing = values.len() - non_null_count;
            metrics.add_issue(QualityIssue {
                issue_type: QualityIssueType::MissingValues,
                severity: 3,
                description: format!(
                    "{} has {} missing values ({:.1}%)",
                    column_name,
                    missing,
                    (1.0 - metrics.completeness) * 100.0
                ),
                columns: vec![column_name.to_string()],
                affected_rows: missing,
                examples: Vec::new(),
                suggested_fix: Some("Consider adding NOT NULL constraint or default value".to_string()),
            });
        }

        // Uniqueness: distinct values / total values
        let unique_values: HashSet<&Option<String>> = values.iter().collect();
        metrics.uniqueness = unique_values.len() as f64 / values.len() as f64;

        // Find duplicates
        let mut value_counts: HashMap<&str, usize> = HashMap::new();
        for value in values.iter().flatten() {
            *value_counts.entry(value.as_str()).or_insert(0) += 1;
        }

        let duplicates: Vec<(&str, usize)> = value_counts
            .iter()
            .filter(|(_, &count)| count > 1)
            .map(|(&v, &c)| (v, c))
            .collect();

        if !duplicates.is_empty() {
            let dup_count: usize = duplicates.iter().map(|(_, c)| c - 1).sum();
            if dup_count > values.len() / 100 {
                // More than 1% duplicates
                metrics.add_issue(QualityIssue {
                    issue_type: QualityIssueType::Duplicates,
                    severity: 2,
                    description: format!(
                        "{} has {} duplicate values",
                        column_name, dup_count
                    ),
                    columns: vec![column_name.to_string()],
                    affected_rows: dup_count,
                    examples: duplicates.iter().take(3).map(|(v, _)| v.to_string()).collect(),
                    suggested_fix: None,
                });
            }
        }

        // Validity: apply validation rules for this column
        let column_rules: Vec<&ValidationRule> = self
            .rules
            .iter()
            .filter(|r| r.column == column_name && r.enabled)
            .collect();

        let mut valid_count = non_null_count;
        for rule in column_rules {
            let invalid = self.count_invalid_by_rule(rule, values);
            valid_count = valid_count.saturating_sub(invalid);
        }
        metrics.validity = valid_count as f64 / non_null_count.max(1) as f64;

        // Detect outliers for numeric columns
        let numeric_values: Vec<f64> = values
            .iter()
            .flatten()
            .filter_map(|v| v.parse::<f64>().ok())
            .collect();

        if numeric_values.len() > 10 {
            let outliers = self.detect_outliers(&numeric_values);
            if !outliers.is_empty() {
                metrics.add_issue(QualityIssue {
                    issue_type: QualityIssueType::Outlier,
                    severity: 2,
                    description: format!(
                        "{} has {} potential outliers",
                        column_name,
                        outliers.len()
                    ),
                    columns: vec![column_name.to_string()],
                    affected_rows: outliers.len(),
                    examples: outliers.iter().take(3).map(|v| v.to_string()).collect(),
                    suggested_fix: Some("Review outlier values for data entry errors".to_string()),
                });

                // Adjust accuracy based on outliers
                metrics.accuracy = 1.0 - (outliers.len() as f64 / numeric_values.len() as f64);
            }
        }

        // Default scores for dimensions we couldn't measure
        // (consistency and timeliness need external context)

        metrics.calculate_overall();
        metrics
    }

    /// Counts values that violate a rule.
    fn count_invalid_by_rule(&self, rule: &ValidationRule, values: &[Option<String>]) -> usize {
        match &rule.rule_type {
            ValidationRuleType::NotNull => {
                values.iter().filter(|v| v.is_none()).count()
            }
            ValidationRuleType::Pattern(pattern) => {
                if let Ok(regex) = regex::Regex::new(pattern) {
                    values
                        .iter()
                        .flatten()
                        .filter(|v| !regex.is_match(v))
                        .count()
                } else {
                    0
                }
            }
            ValidationRuleType::Range { min, max } => {
                values
                    .iter()
                    .flatten()
                    .filter_map(|v| v.parse::<f64>().ok())
                    .filter(|v| *v < *min || *v > *max)
                    .count()
            }
            ValidationRuleType::AllowedValues(allowed) => {
                let allowed_set: HashSet<&str> = allowed.iter().map(|s| s.as_str()).collect();
                values
                    .iter()
                    .flatten()
                    .filter(|v| !allowed_set.contains(v.as_str()))
                    .count()
            }
            _ => 0,
        }
    }

    /// Detects outliers using IQR method.
    fn detect_outliers(&self, values: &[f64]) -> Vec<f64> {
        if values.len() < 4 {
            return Vec::new();
        }

        let mut sorted = values.to_vec();
        sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());

        let q1_idx = sorted.len() / 4;
        let q3_idx = (sorted.len() * 3) / 4;

        let q1 = sorted[q1_idx];
        let q3 = sorted[q3_idx];
        let iqr = q3 - q1;

        let lower_bound = q1 - 1.5 * iqr;
        let upper_bound = q3 + 1.5 * iqr;

        values
            .iter()
            .filter(|&&v| v < lower_bound || v > upper_bound)
            .copied()
            .collect()
    }

    /// Analyzes multiple columns and returns aggregate metrics.
    pub fn analyze_table(
        &self,
        columns: &[(&str, &[Option<String>])],
    ) -> TableQualityReport {
        let mut column_metrics: Vec<QualityMetrics> = Vec::new();

        for (name, values) in columns {
            column_metrics.push(self.analyze_column(name, values));
        }

        let overall = if column_metrics.is_empty() {
            1.0
        } else {
            column_metrics.iter().map(|m| m.overall_score).sum::<f64>()
                / column_metrics.len() as f64
        };

        let all_issues: Vec<QualityIssue> = column_metrics
            .iter()
            .flat_map(|m| m.issues.clone())
            .collect();

        TableQualityReport {
            column_metrics,
            overall_score: overall,
            total_issues: all_issues.len(),
            critical_issues: all_issues.iter().filter(|i| i.severity >= 4).count(),
            issues: all_issues,
            analyzed_at: std::time::Instant::now(),
        }
    }

    /// Returns cached metrics for a column.
    pub fn get_cached(&self, column: &str) -> Option<QualityMetrics> {
        self.cache.read().get(column).cloned()
    }

    /// Clears the cache.
    pub fn clear_cache(&self) {
        self.cache.write().clear();
    }
}

/// Quality report for an entire table.
#[derive(Debug)]
pub struct TableQualityReport {
    /// Metrics per column
    pub column_metrics: Vec<QualityMetrics>,
    /// Overall quality score
    pub overall_score: f64,
    /// Total issues found
    pub total_issues: usize,
    /// Number of critical (severity >= 4) issues
    pub critical_issues: usize,
    /// All issues across columns
    pub issues: Vec<QualityIssue>,
    /// Timestamp of analysis
    pub analyzed_at: std::time::Instant,
}

impl TableQualityReport {
    /// Returns columns below quality threshold.
    pub fn failing_columns(&self, threshold: f64) -> Vec<&QualityMetrics> {
        self.column_metrics
            .iter()
            .filter(|m| m.overall_score < threshold)
            .collect()
    }

    /// Returns issues by severity.
    pub fn issues_by_severity(&self, minseverity: u8) -> Vec<&QualityIssue> {
        self.issues
            .iter()
            .filter(|i| i.severity >= min_severity)
            .collect()
    }
}

/// Query performance tracker for monitoring query execution.
#[derive(Debug)]
pub struct QueryPerformanceTracker {
    /// Execution times by query hash
    execution_times: Arc<RwLock<HashMap<u64, Vec<u64>>>>,
    /// Slow query threshold in milliseconds
    slow_threshold_ms: u64,
    /// Maximum entries per query
    max_samples: usize,
}

impl Default for QueryPerformanceTracker {
    fn default() -> Self {
        Self::new()
    }
}

impl QueryPerformanceTracker {
    /// Creates a new performance tracker.
    pub fn new() -> Self {
        Self {
            execution_times: Arc::new(RwLock::new(HashMap::new())),
            slow_threshold_ms: 1000,
            max_samples: 100,
        }
    }

    /// Sets the slow query threshold.
    pub fn with_slow_threshold(mut self, threshold_ms: u64) -> Self {
        self.slow_threshold_ms = threshold_ms;
        self
    }

    /// Records a query execution.
    pub fn record(&self, query_hash: u64, execution_time_ms: u64) {
        let mut times = self.execution_times.write();
        let samples = times.entry(query_hash).or_insert_with(Vec::new);

        samples.push(execution_time_ms);

        if samples.len() > self.max_samples {
            samples.remove(0);
        }
    }

    /// Returns performance metrics for a query.
    pub fn get_metrics(&self, query_hash: u64) -> Option<PerformanceMetrics> {
        let times = self.execution_times.read();
        let samples = times.get(&query_hash)?;

        if samples.is_empty() {
            return None;
        }

        let sum: u64 = samples.iter().sum();
        let avg = sum as f64 / samples.len() as f64;
        let min = *samples.iter().min().unwrap();
        let max = *samples.iter().max().unwrap();

        let mut sorted = samples.clone();
        sorted.sort_unstable();
        let p50 = sorted[sorted.len() / 2];
        let p95 = sorted[(sorted.len() * 95) / 100];
        let p99 = sorted[(sorted.len() * 99) / 100];

        let variance: f64 = samples
            .iter()
            .map(|&x| (x as f64 - avg).powi(2))
            .sum::<f64>()
            / samples.len() as f64;

        Some(PerformanceMetrics {
            query_hash,
            sample_count: samples.len(),
            avg_time_ms: avg,
            min_time_ms: min,
            max_time_ms: max,
            p50_time_ms: p50,
            p95_time_ms: p95,
            p99_time_ms: p99,
            stddev_ms: variance.sqrt(),
            slow_count: samples.iter().filter(|&&t| t >= self.slow_threshold_ms).count(),
        })
    }

    /// Returns all slow queries.
    pub fn get_slow_queries(&self) -> Vec<(u64, PerformanceMetrics)> {
        let times = self.execution_times.read();

        times
            .keys()
            .filter_map(|&hash| {
                let metrics = self.get_metrics(hash)?;
                if metrics.slow_count > 0 {
                    Some((hash, metrics))
                } else {
                    None
                }
            })
            .collect()
    }

    /// Clears all recorded metrics.
    pub fn clear(&self) {
        self.execution_times.write().clear();
    }
}

/// Performance metrics for a query.
#[derive(Debug, Clone)]
pub struct PerformanceMetrics {
    /// Query hash
    pub query_hash: u64,
    /// Number of samples
    pub sample_count: usize,
    /// Average execution time
    pub avg_time_ms: f64,
    /// Minimum execution time
    pub min_time_ms: u64,
    /// Maximum execution time
    pub max_time_ms: u64,
    /// Median execution time
    pub p50_time_ms: u64,
    /// 95th percentile
    pub p95_time_ms: u64,
    /// 99th percentile
    pub p99_time_ms: u64,
    /// Standard deviation
    pub stddev_ms: f64,
    /// Count of executions exceeding slow threshold
    pub slow_count: usize,
}

#[cfg(test)]
mod tests {
    use super::*;
use std::time::Instant;

    #[test]
    fn test_quality_metrics_calculation() {
        let mut metrics = QualityMetrics::new("test_column");
        metrics.completeness = 0.90;
        metrics.uniqueness = 0.95;
        metrics.validity = 0.98;
        metrics.consistency = 1.0;
        metrics.accuracy = 0.99;
        metrics.timeliness = 1.0;

        metrics.calculate_overall();

        assert!(metrics.overall_score > 0.9);
        assert!(metrics.overall_score < 1.0);
    }

    #[test]
    fn test_column_analysis() {
        let analyzer = DataQualityAnalyzer::new();
        let values: Vec<Option<String>> = vec![
            Some("a".to_string()),
            Some("b".to_string()),
            None,
            Some("c".to_string()),
            Some("a".to_string()),
        ];

        let metrics = analyzer.analyze_column("test", &values);

        assert_eq!(metrics.row_count, 5);
        assert_eq!(metrics.completeness, 0.8);
        assert!(!metrics.issues.is_empty());
    }

    #[test]
    fn test_outlier_detection() {
        let analyzer = DataQualityAnalyzer::new();
        let values: Vec<Option<String>> = (1..=100)
            .map(|i| Some(i.to_string()))
            .chain(std::iter::once(Some("1000".to_string())))
            .collect();

        let metrics = analyzer.analyze_column("numbers", &values);

        // Should detect 1000 as outlier
        let outlier_issues: Vec<&QualityIssue> = metrics
            .issues
            .iter()
            .filter(|i| i.issue_type == QualityIssueType::Outlier)
            .collect();

        assert!(!outlier_issues.is_empty());
    }

    #[test]
    fn test_performance_tracker() {
        let tracker = QueryPerformanceTracker::new();

        for i in 0..50 {
            tracker.record(123, 50 + i);
        }

        let metrics = tracker.get_metrics(123).unwrap();

        assert_eq!(metrics.sample_count, 50);
        assert!(metrics.avg_time_ms > 50.0);
        assert!(metrics.avg_time_ms < 100.0);
    }
}
