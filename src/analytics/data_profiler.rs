// Data Profiling and Column Analysis
//
// This module provides comprehensive data profiling capabilities for analyzing
// column characteristics, inferring data types, and building bitmap indexes
// for efficient query processing.
//
// # Architecture
//
// The profiler follows a streaming analysis pattern:
// - Single-pass column scanning for basic statistics
// - Pattern-based type inference
// - Bitmap index construction for low-cardinality columns
//
// # Example
//
// ```rust,ignore
// use crate::analytics::data_profiler::{DataProfiler, InferredType};
//
// let mut profiler = DataProfiler::new();
// let profile = profiler.profile_column("user_id", &values);
//
// match profile.inferred_type {
//     InferredType::Integer => println!("Column contains integers"),
//     InferredType::Email => println!("Column contains email addresses"),
//     _ => {}
// }
// ```

use std::collections::HashSet;
use std::collections::HashMap;

/// Inferred data type from column analysis.
///
/// The profiler can detect semantic types beyond basic SQL types,
/// enabling more intelligent query optimization and validation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InferredType {
    /// Integer numeric type
    Integer,
    /// Floating-point numeric type
    Float,
    /// Boolean values (true/false, 1/0, yes/no)
    Boolean,
    /// Date values (YYYY-MM-DD pattern)
    Date,
    /// Timestamp with time component
    Timestamp,
    /// Email address pattern
    Email,
    /// URL pattern
    Url,
    /// Phone number pattern
    Phone,
    /// IP address (v4 or v6)
    IpAddress,
    /// UUID/GUID pattern
    Uuid,
    /// JSON structure
    Json,
    /// Generic string (no specific pattern detected)
    String,
    /// NULL or empty values only
    Null,
    /// Mixed or unknown type
    Unknown,
}

impl InferredType {
    /// Infers the type from a string value.
    ///
    /// Uses pattern matching and parsing attempts to determine
    /// the most likely semantic type of the value.
    pub fn infer(value: &str) -> Self {
        let trimmed = value.trim();

        if trimmed.is_empty() {
            return InferredType::Null;
        }

        // Try integer
        if trimmed.parse::<i64>().is_ok() {
            return InferredType::Integer;
        }

        // Try float
        if trimmed.parse::<f64>().is_ok() {
            return InferredType::Float;
        }

        // Boolean patterns
        let lower = trimmed.to_lowercase();
        if matches!(lower.as_str(), "true" | "false" | "yes" | "no" | "1" | "0") {
            return InferredType::Boolean;
        }

        // Date pattern (YYYY-MM-DD)
        if trimmed.len() == 10
            && trimmed.chars().nth(4) == Some('-')
            && trimmed.chars().nth(7) == Some('-')
        {
            if trimmed[0..4].parse::<u16>().is_ok()
                && trimmed[5..7].parse::<u8>().is_ok()
                && trimmed[8..10].parse::<u8>().is_ok()
            {
                return InferredType::Date;
            }
        }

        // Timestamp pattern (contains 'T' or space between date and time)
        if (trimmed.contains('T') || trimmed.contains(' '))
            && trimmed.len() >= 19
            && trimmed.contains(':')
        {
            return InferredType::Timestamp;
        }

        // Email pattern
        if trimmed.contains('@') && trimmed.contains('.') && !trimmed.contains(' ') {
            return InferredType::Email;
        }

        // URL pattern
        if trimmed.starts_with("http://")
            || trimmed.starts_with("https://")
            || trimmed.starts_with("ftp://")
        {
            return InferredType::Url;
        }

        // UUID pattern (8-4-4-4-12 hex)
        if trimmed.len() == 36 && trimmed.chars().filter(|c| *c == '-').count() == 4 {
            let parts: Vec<&str> = trimmed.split('-').collect();
            if parts.len() == 5
                && parts.iter().all(|p| p.chars().all(|c| c.is_ascii_hexdigit()))
            {
                return InferredType::Uuid;
            }
        }

        // IP address pattern (simple check)
        if trimmed.split('.').count() == 4
            && trimmed.split('.').all(|p| p.parse::<u8>().is_ok())
        {
            return InferredType::IpAddress;
        }

        // IPv6 check
        if trimmed.contains(':') && trimmed.split(':').count() >= 4 {
            if trimmed
                .split(':')
                .all(|p| p.is_empty() || p.chars().all(|c| c.is_ascii_hexdigit()))
            {
                return InferredType::IpAddress;
            }
        }

        // JSON pattern
        if (trimmed.starts_with('{') && trimmed.ends_with('}'))
            || (trimmed.starts_with('[') && trimmed.ends_with(']'))
        {
            return InferredType::Json;
        }

        // Phone number (simplified)
        let digits_only: String = trimmed.chars().filter(|c| c.is_ascii_digit()).collect();
        if digits_only.len() >= 10 && digits_only.len() <= 15 {
            let non_digit_chars: String = trimmed
                .chars()
                .filter(|c| !c.is_ascii_digit())
                .collect();
            if non_digit_chars.chars().all(|c| "+-() .".contains(c)) {
                return InferredType::Phone;
            }
        }

        InferredType::String
    }
}

/// Profile of a single column after analysis.
///
/// Contains comprehensive statistics about the column's data distribution,
/// value patterns, and inferred characteristics.
#[derive(Debug, Clone)]
pub struct ColumnProfile {
    /// Column name
    pub name: String,
    /// Total number of rows analyzed
    pub row_count: usize,
    /// Number of NULL values
    pub null_count: usize,
    /// Number of distinct values
    pub distinct_count: usize,
    /// Inferred semantic type
    pub inferred_type: InferredType,
    /// Minimum value (as string for comparison)
    pub min_value: Option<String>,
    /// Maximum value (as string for comparison)
    pub max_value: Option<String>,
    /// Average length for string columns
    pub avg_length: Option<f64>,
    /// Maximum length for string columns
    pub max_length: Option<usize>,
    /// Most frequent values with counts
    pub top_values: Vec<(String, usize)>,
    /// Sample of unique values
    pub sample_values: Vec<String>,
    /// Whether column appears to be a primary key candidate
    pub is_unique: bool,
    /// Type distribution when mixed types detected
    pub type_distribution: HashMap<InferredType, usize>,
}

impl ColumnProfile {
    /// Creates a new column profile with the given name.
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            row_count: 0,
            null_count: 0,
            distinct_count: 0,
            inferred_type: InferredType::Unknown,
            min_value: None,
            max_value: None,
            avg_length: None,
            max_length: None,
            top_values: Vec::new(),
            sample_values: Vec::new(),
            is_unique: false,
            type_distribution: HashMap::new(),
        }
    }

    /// Calculates the null percentage.
    pub fn null_percentage(&self) -> f64 {
        if self.row_count == 0 {
            0.0
        } else {
            (self.null_count as f64 / self.row_count as f64) * 100.0
        }
    }

    /// Calculates the uniqueness ratio (distinct/total).
    pub fn uniqueness_ratio(&self) -> f64 {
        if self.row_count == 0 {
            0.0
        } else {
            self.distinct_count as f64 / self.row_count as f64
        }
    }
}

/// Bitmap index for efficient column value lookups.
///
/// Particularly effective for low-cardinality columns where
/// bitmap operations can significantly speed up filtering.
#[derive(Debug, Clone)]
pub struct BitmapIndex {
    /// Column name this index is for
    pub column: String,
    /// Mapping from values to their row positions
    bitmaps: HashMap<String, Vec<usize>>,
    /// Total number of rows indexed
    row_count: usize,
}

impl BitmapIndex {
    /// Creates a new empty bitmap index for the given column.
    pub fn new(column: impl Into<String>) -> Self {
        Self {
            column: column.into(),
            bitmaps: HashMap::new(),
            row_count: 0,
        }
    }

    /// Builds a bitmap index from column values.
    ///
    /// Each unique value gets a list of row positions where it appears.
    pub fn build(column: impl Into<String>, values: &[String]) -> Self {
        let mut index = Self::new(column);
        index.row_count = values.len();

        for (row_idx, value) in values.iter().enumerate() {
            index
                .bitmaps
                .entry(value.clone())
                .or_insert_with(Vec::new)
                .push(row_idx);
        }

        index
    }

    /// Returns row positions where the column equals the given value.
    pub fn lookup(&self, value: &str) -> Option<&Vec<usize>> {
        self.bitmaps.get(value)
    }

    /// Returns row positions matching any of the given values (OR).
    pub fn lookup_any(&self, values: &[&str]) -> Vec<usize> {
        let mut result: HashSet<usize> = HashSet::new();

        for value in values {
            if let Some(positions) = self.bitmaps.get(*value) {
                result.extend(positions);
            }
        }

        let mut sorted: Vec<usize> = result.into_iter().collect();
        sorted.sort_unstable();
        sorted
    }

    /// Returns row positions matching all given criteria (AND).
    ///
    /// Takes another bitmap index and returns intersection of matches.
    pub fn intersect(&self, other: &BitmapIndex, self_value: &str, other_value: &str) -> Vec<usize> {
        let self_positions = self.lookup(self_value);
        let other_positions = other.lookup(other_value);

        match (self_positions, other_positions) {
            (Some(a), Some(b)) => {
                let set_a: HashSet<usize> = a.iter().cloned().collect();
                b.iter().filter(|p| set_a.contains(p)).cloned().collect()
            }
            _ => Vec::new(),
        }
    }

    /// Returns the number of distinct values in the index.
    pub fn cardinality(&self) -> usize {
        self.bitmaps.len()
    }

    /// Returns all distinct values in the index.
    pub fn distinct_values(&self) -> Vec<&String> {
        self.bitmaps.keys().collect()
    }

    /// Returns the total row count.
    pub fn row_count(&self) -> usize {
        self.row_count
    }

    /// Checks if the index is suitable for the column.
    ///
    /// Bitmap indexes are most effective for low-cardinality columns.
    pub fn is_efficient(&self) -> bool {
        // Generally efficient if cardinality is less than 10% of row count
        // and cardinality is reasonable (< 10000 distinct values)
        self.cardinality() < 10000 && (self.cardinality() as f64) < (self.row_count as f64 * 0.1)
    }
}

/// Data profiler for comprehensive column analysis.
///
/// Analyzes column data to extract statistics, infer types,
/// and identify data quality issues.
#[derive(Debug, Default)]
pub struct DataProfiler {
    /// Maximum number of top values to track per column
    max_top_values: usize,
    /// Maximum sample size for unique values
    max_sample_size: usize,
    /// Cached profiles for reuse
    profiles: HashMap<String, ColumnProfile>,
}

impl DataProfiler {
    /// Creates a new data profiler with default settings.
    pub fn new() -> Self {
        Self {
            max_top_values: 10,
            max_sample_size: 100,
            profiles: HashMap::new(),
        }
    }

    /// Creates a profiler with custom settings.
    pub fn with_settings(max_top_values: usize, max_sample_size: usize) -> Self {
        Self {
            max_top_values,
            max_sample_size,
            profiles: HashMap::new(),
        }
    }

    /// Profiles a column from its values.
    ///
    /// Analyzes the values to extract comprehensive statistics
    /// and infer the semantic type of the column.
    pub fn profile_column(&mut self, name: &str, values: &[Option<String>]) -> ColumnProfile {
        let mut profile = ColumnProfile::new(name);
        profile.row_count = values.len();

        let mut distinct: HashSet<String> = HashSet::new();
        let mut value_counts: HashMap<String, usize> = HashMap::new();
        let mut type_counts: HashMap<InferredType, usize> = HashMap::new();
        let mut total_length: usize = 0;
        let mut max_len: usize = 0;

        for value in values {
            match value {
                None => {
                    profile.null_count += 1;
                }
                Some(v) if v.is_empty() => {
                    profile.null_count += 1;
                }
                Some(v) => {
                    distinct.insert(v.clone());
                    *value_counts.entry(v.clone()).or_insert(0) += 1;

                    let inferred = InferredType::infer(v);
                    *type_counts.entry(inferred).or_insert(0) += 1;

                    total_length += v.len();
                    max_len = max_len.max(v.len());

                    // Track min/max
                    match &profile.min_value {
                        None => profile.min_value = Some(v.clone()),
                        Some(min) if v < min => profile.min_value = Some(v.clone()),
                        _ => {}
                    }
                    match &profile.max_value {
                        None => profile.max_value = Some(v.clone()),
                        Some(max) if v > max => profile.max_value = Some(v.clone()),
                        _ => {}
                    }
                }
            }
        }

        profile.distinct_count = distinct.len();
        profile.is_unique = profile.distinct_count == profile.row_count - profile.null_count;
        profile.max_length = Some(max_len);

        let non_null_count = profile.row_count - profile.null_count;
        if non_null_count > 0 {
            profile.avg_length = Some(total_length as f64 / non_null_count as f64);
        }

        // Top values
        let mut sorted_counts: Vec<(String, usize)> = value_counts.into_iter().collect();
        sorted_counts.sort_by(|a, b| b.1.cmp(&a.1));
        profile.top_values = sorted_counts
            .into_iter()
            .take(self.max_top_values)
            .collect();

        // Sample values
        profile.sample_values = distinct
            .into_iter()
            .take(self.max_sample_size)
            .collect();

        // Determine primary type
        if let Some((primary_type, _)) = type_counts.iter().max_by_key(|(_, count)| *count) {
            profile.inferred_type = primary_type.clone();
        }
        profile.type_distribution = type_counts
            .into_iter()
            .map(|(k, v)| (k, v))
            .collect();

        // Cache the profile
        self.profiles.insert(name.to_string(), profile.clone());

        profile
    }

    /// Profiles multiple columns efficiently.
    pub fn profile_columns(
        &mut self,
        columns: &[(&str, &[Option<String>])],
    ) -> Vec<ColumnProfile> {
        columns
            .iter()
            .map(|(name, values)| self.profile_column(name, values))
            .collect()
    }

    /// Retrieves a cached profile if available.
    pub fn get_cached_profile(&self, column: &str) -> Option<&ColumnProfile> {
        self.profiles.get(column)
    }

    /// Clears the profile cache.
    pub fn clear_cache(&mut self) {
        self.profiles.clear();
    }

    /// Suggests an appropriate index type for the column.
    pub fn suggest_index_type(&self, profile: &ColumnProfile) -> IndexSuggestion {
        // High cardinality - B-tree works well
        if profile.uniqueness_ratio() > 0.9 {
            return IndexSuggestion::BTree;
        }

        // Low cardinality - bitmap is efficient
        if profile.distinct_count < 1000 && profile.uniqueness_ratio() < 0.1 {
            return IndexSuggestion::Bitmap;
        }

        // Text search patterns
        if matches!(profile.inferred_type, InferredType::String)
            && profile.avg_length.unwrap_or(0.0) > 50.0
        {
            return IndexSuggestion::FullText;
        }

        // Spatial/temporal types
        if matches!(
            profile.inferred_type,
            InferredType::Date | InferredType::Timestamp
        ) {
            return IndexSuggestion::BTree;
        }

        IndexSuggestion::BTree
    }
}

/// Index type suggestion based on column characteristics.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IndexSuggestion {
    /// B-tree index (default for high cardinality)
    BTree,
    /// Bitmap index (optimal for low cardinality)
    Bitmap,
    /// Hash index (for equality comparisons only)
    Hash,
    /// Full-text index (for text search)
    FullText,
    /// No index recommended
    None,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_type_inference() {
        assert_eq!(InferredType::infer("123"), InferredType::Integer);
        assert_eq!(InferredType::infer("12.34"), InferredType::Float);
        assert_eq!(InferredType::infer("true"), InferredType::Boolean);
        assert_eq!(InferredType::infer("2024-01-15"), InferredType::Date);
        assert_eq!(
            InferredType::infer("test@example.com"),
            InferredType::Email
        );
        assert_eq!(
            InferredType::infer("https://example.com"),
            InferredType::Url
        );
        assert_eq!(InferredType::infer("192.168.1.1"), InferredType::IpAddress);
    }

    #[test]
    fn test_bitmap_index() {
        let values = vec![
            "A".to_string(),
            "B".to_string(),
            "A".to_string(),
            "C".to_string(),
            "A".to_string(),
        ];
        let index = BitmapIndex::build("category", &values);

        assert_eq!(index.cardinality(), 3);
        assert_eq!(index.lookup("A"), Some(&vec![0, 2, 4]));
        assert_eq!(index.lookup("B"), Some(&vec![1]));
    }

    #[test]
    fn test_column_profiler() {
        let mut profiler = DataProfiler::new();
        let values: Vec<Option<String>> = vec![
            Some("1".to_string()),
            Some("2".to_string()),
            None,
            Some("3".to_string()),
        ];

        let profile = profiler.profile_column("test", &values);

        assert_eq!(profile.row_count, 4);
        assert_eq!(profile.null_count, 1);
        assert_eq!(profile.distinct_count, 3);
        assert_eq!(profile.inferred_type, InferredType::Integer);
    }
}
