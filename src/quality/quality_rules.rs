// Data Quality Rules Engine
// Implements comprehensive data quality validation rules

use crate::common::{Schema, Tuple, Value, TableId, ColumnId};
use crate::error::{DbError, Result};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use parking_lot::RwLock;
use regex::Regex;

/// Rule severity level
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RuleSeverity {
    Critical,
    High,
    Medium,
    Low,
    Info,
}

/// Rule status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RuleStatus {
    Active,
    Disabled,
    Testing,
}

/// Type of quality rule
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum RuleType {
    Completeness,
    Uniqueness,
    Format,
    Range,
    ReferentialIntegrity,
    Custom,
}

/// Quality rule definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QualityRule {
    pub id: String,
    pub name: String,
    pub description: String,
    pub rule_type: RuleType,
    pub table_name: String,
    pub column_names: Vec<String>,
    pub severity: RuleSeverity,
    pub status: RuleStatus,
    pub config: RuleConfig,
}

/// Rule configuration options
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RuleConfig {
    Completeness(CompletenessConfig),
    Uniqueness(UniquenessConfig),
    Format(FormatConfig),
    Range(RangeConfig),
    ReferentialIntegrity(ReferentialIntegrityConfig),
    Custom(CustomConfig),
}

/// Completeness rule configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompletenessConfig {
    pub allow_null: bool,
    pub min_completeness_percent: f64,
}

/// Uniqueness rule configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UniquenessConfig {
    pub columns: Vec<String>,
    pub allow_duplicates: bool,
    pub max_duplicate_percent: f64,
}

/// Format rule configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FormatConfig {
    pub pattern: String,
    pub case_sensitive: bool,
    pub allow_empty: bool,
}

/// Range rule configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RangeConfig {
    pub min_value: Option<Value>,
    pub max_value: Option<Value>,
    pub allowed_values: Option<Vec<Value>>,
}

/// Referential integrity rule configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReferentialIntegrityConfig {
    pub referenced_table: String,
    pub referenced_columns: Vec<String>,
    pub allow_null: bool,
}

/// Custom rule configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CustomConfig {
    pub validator_name: String,
    pub parameters: HashMap<String, String>,
}

/// Rule violation record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuleViolation {
    pub rule_id: String,
    pub rule_name: String,
    pub table_name: String,
    pub column_name: Option<String>,
    pub row_id: Option<u64>,
    pub severity: RuleSeverity,
    pub message: String,
    pub actual_value: Option<String>,
    pub expected_value: Option<String>,
    pub timestamp: std::time::SystemTime,
}

/// Validation result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationResult {
    pub table_name: String,
    pub total_rows: usize,
    pub violations: Vec<RuleViolation>,
    pub rules_evaluated: usize,
    pub pass_rate: f64,
}

/// Completeness rule implementation
pub struct CompletenessRule {
    config: CompletenessConfig,
}

impl CompletenessRule {
    pub fn new(config: CompletenessConfig) -> Self {
        Self { config }
    }

    pub fn validate(
        &self,
        rule: &QualityRule,
        _schema: &Schema,
        data: &[Tuple],
        column_index: usize,
    ) -> Result<Vec<RuleViolation>> {
        let mut violations = Vec::new();

        if data.is_empty() {
            return Ok(violations);
        }

        let mut null_count = 0;
        for tuple in data {
            if let Some(value) = tuple.get(column_index) {
                if value.is_null() {
                    null_count += 1;
                    if !self.config.allow_null {
                        violations.push(RuleViolation {
                            rule_id: rule.id.clone(),
                            rule_name: rule.name.clone(),
                            table_name: rule.table_name.clone(),
                            column_name: Some(rule.column_names.get(0).unwrap_or(&"unknown".to_string()).clone()),
                            row_id: Some(tuple.row_id),
                            severity: rule.severity,
                            message: "NULL value found in non-nullable column".to_string(),
                            actual_value: Some("NULL".to_string()),
                            expected_value: Some("non-NULL".to_string()),
                            timestamp: std::time::SystemTime::now(),
                        });
                    }
                }
            }
        }

        // Check completeness percentage
        let completeness_percent = ((data.len() - null_count) as f64 / data.len() as f64) * 100.0;
        if completeness_percent < self.config.min_completeness_percent {
            violations.push(RuleViolation {
                rule_id: rule.id.clone(),
                rule_name: rule.name.clone(),
                table_name: rule.table_name.clone(),
                column_name: Some(rule.column_names.get(0).unwrap_or(&"unknown".to_string()).clone()),
                row_id: None,
                severity: rule.severity,
                message: format!(
                    "Completeness below threshold: {:.2}% < {:.2}%",
                    completeness_percent, self.config.min_completeness_percent
                ),
                actual_value: Some(format!("{:.2}%", completeness_percent)),
                expected_value: Some(format!(">= {:.2}%", self.config.min_completeness_percent)),
                timestamp: std::time::SystemTime::now(),
            });
        }

        Ok(violations)
    }
}

/// Uniqueness rule implementation
pub struct UniquenessRule {
    config: UniquenessConfig,
}

impl UniquenessRule {
    pub fn new(config: UniquenessConfig) -> Self {
        Self { config }
    }

    pub fn validate(
        &self,
        rule: &QualityRule,
        schema: &Schema,
        data: &[Tuple],
    ) -> Result<Vec<RuleViolation>> {
        let mut violations = Vec::new();

        if data.is_empty() {
            return Ok(violations);
        }

        // Map column names to indices
        let column_indices: Vec<usize> = self.config.columns
            .iter()
            .filter_map(|col_name| schema.get_column_index(col_name))
            .collect();

        if column_indices.is_empty() {
            return Ok(violations);
        }

        // Track seen value combinations
        let mut seen_values: HashMap<String, Vec<u64>> = HashMap::new();

        for tuple in data {
            // Build key from column values
            let key: String = column_indices
                .iter()
                .map(|&idx| {
                    tuple.get(idx)
                        .map(|v| v.to_display_string())
                        .unwrap_or_else(|| "NULL".to_string())
                })
                .collect::<Vec<_>>()
                .join("|");

            seen_values.entry(key).or_insert_with(Vec::new).push(tuple.row_id);
        }

        // Check for duplicates
        let mut duplicate_count = 0;
        for (value, row_ids) in seen_values.iter() {
            if row_ids.len() > 1 {
                duplicate_count += row_ids.len() - 1;
                if !self.config.allow_duplicates {
                    for &row_id in row_ids.iter().skip(1) {
                        violations.push(RuleViolation {
                            rule_id: rule.id.clone(),
                            rule_name: rule.name.clone(),
                            table_name: rule.table_name.clone(),
                            column_name: Some(self.config.columns.join(", ")),
                            row_id: Some(row_id),
                            severity: rule.severity,
                            message: format!("Duplicate value found: {}", value),
                            actual_value: Some(value.clone()),
                            expected_value: Some("unique value".to_string()),
                            timestamp: std::time::SystemTime::now(),
                        });
                    }
                }
            }
        }

        // Check duplicate percentage
        if self.config.allow_duplicates {
            let duplicate_percent = (duplicate_count as f64 / data.len() as f64) * 100.0;
            if duplicate_percent > self.config.max_duplicate_percent {
                violations.push(RuleViolation {
                    rule_id: rule.id.clone(),
                    rule_name: rule.name.clone(),
                    table_name: rule.table_name.clone(),
                    column_name: Some(self.config.columns.join(", ")),
                    row_id: None,
                    severity: rule.severity,
                    message: format!(
                        "Duplicate percentage exceeds threshold: {:.2}% > {:.2}%",
                        duplicate_percent, self.config.max_duplicate_percent
                    ),
                    actual_value: Some(format!("{:.2}%", duplicate_percent)),
                    expected_value: Some(format!("<= {:.2}%", self.config.max_duplicate_percent)),
                    timestamp: std::time::SystemTime::now(),
                });
            }
        }

        Ok(violations)
    }
}

/// Format rule implementation
pub struct FormatRule {
    config: FormatConfig,
    regex: Regex,
}

impl FormatRule {
    pub fn new(config: FormatConfig) -> Result<Self> {
        let regex = Regex::new(&config.pattern)
            .map_err(|e| DbError::Validation(format!("Invalid regex pattern: {}", e)))?;
        Ok(Self { config, regex })
    }

    pub fn validate(
        &self,
        rule: &QualityRule,
        _schema: &Schema,
        data: &[Tuple],
        column_index: usize,
    ) -> Result<Vec<RuleViolation>> {
        let mut violations = Vec::new();

        for tuple in data {
            if let Some(value) = tuple.get(column_index) {
                let string_value = match value {
                    Value::String(s) => s.clone(),
                    Value::Null if self.config.allow_empty => continue,
                    _ => value.to_display_string(),
                };

                if string_value.is_empty() && !self.config.allow_empty {
                    violations.push(RuleViolation {
                        rule_id: rule.id.clone(),
                        rule_name: rule.name.clone(),
                        table_name: rule.table_name.clone(),
                        column_name: Some(rule.column_names.get(0).unwrap_or(&"unknown".to_string()).clone()),
                        row_id: Some(tuple.row_id),
                        severity: rule.severity,
                        message: "Empty value not allowed".to_string(),
                        actual_value: Some("(empty)".to_string()),
                        expected_value: Some(format!("matches pattern: {}", self.config.pattern)),
                        timestamp: std::time::SystemTime::now(),
                    });
                    continue;
                }

                let test_value = if self.config.case_sensitive {
                    string_value.clone()
                } else {
                    string_value.to_lowercase()
                };

                if !self.regex.is_match(&test_value) {
                    violations.push(RuleViolation {
                        rule_id: rule.id.clone(),
                        rule_name: rule.name.clone(),
                        table_name: rule.table_name.clone(),
                        column_name: Some(rule.column_names.get(0).unwrap_or(&"unknown".to_string()).clone()),
                        row_id: Some(tuple.row_id),
                        severity: rule.severity,
                        message: format!("Value does not match pattern: {}", self.config.pattern),
                        actual_value: Some(string_value),
                        expected_value: Some(format!("matches pattern: {}", self.config.pattern)),
                        timestamp: std::time::SystemTime::now(),
                    });
                }
            }
        }

        Ok(violations)
    }
}

/// Range rule implementation
pub struct RangeRule {
    config: RangeConfig,
}

impl RangeRule {
    pub fn new(config: RangeConfig) -> Self {
        Self { config }
    }

    pub fn validate(
        &self,
        rule: &QualityRule,
        _schema: &Schema,
        data: &[Tuple],
        column_index: usize,
    ) -> Result<Vec<RuleViolation>> {
        let mut violations = Vec::new();

        for tuple in data {
            if let Some(value) = tuple.get(column_index) {
                if value.is_null() {
                    continue;
                }

                // Check allowed values
                if let Some(ref allowed) = self.config.allowed_values {
                    if !allowed.contains(value) {
                        violations.push(RuleViolation {
                            rule_id: rule.id.clone(),
                            rule_name: rule.name.clone(),
                            table_name: rule.table_name.clone(),
                            column_name: Some(rule.column_names.get(0).unwrap_or(&"unknown".to_string()).clone()),
                            row_id: Some(tuple.row_id),
                            severity: rule.severity,
                            message: "Value not in allowed list".to_string(),
                            actual_value: Some(value.to_display_string()),
                            expected_value: Some(format!("one of: {:?}", allowed)),
                            timestamp: std::time::SystemTime::now(),
                        });
                    }
                }

                // Check min value
                if let Some(ref min) = self.config.min_value {
                    if value < min {
                        violations.push(RuleViolation {
                            rule_id: rule.id.clone(),
                            rule_name: rule.name.clone(),
                            table_name: rule.table_name.clone(),
                            column_name: Some(rule.column_names.get(0).unwrap_or(&"unknown".to_string()).clone()),
                            row_id: Some(tuple.row_id),
                            severity: rule.severity,
                            message: "Value below minimum".to_string(),
                            actual_value: Some(value.to_display_string()),
                            expected_value: Some(format!(">= {}", min.to_display_string())),
                            timestamp: std::time::SystemTime::now(),
                        });
                    }
                }

                // Check max value
                if let Some(ref max) = self.config.max_value {
                    if value > max {
                        violations.push(RuleViolation {
                            rule_id: rule.id.clone(),
                            rule_name: rule.name.clone(),
                            table_name: rule.table_name.clone(),
                            column_name: Some(rule.column_names.get(0).unwrap_or(&"unknown".to_string()).clone()),
                            row_id: Some(tuple.row_id),
                            severity: rule.severity,
                            message: "Value above maximum".to_string(),
                            actual_value: Some(value.to_display_string()),
                            expected_value: Some(format!("<= {}", max.to_display_string())),
                            timestamp: std::time::SystemTime::now(),
                        });
                    }
                }
            }
        }

        Ok(violations)
    }
}

/// Referential integrity rule implementation
pub struct ReferentialIntegrityRule {
    config: ReferentialIntegrityConfig,
    reference_cache: Arc<RwLock<HashSet<String>>>,
}

impl ReferentialIntegrityRule {
    pub fn new(config: ReferentialIntegrityConfig) -> Self {
        Self {
            config,
            reference_cache: Arc::new(RwLock::new(HashSet::new())),
        }
    }

    pub fn validate(
        &self,
        rule: &QualityRule,
        schema: &Schema,
        data: &[Tuple],
    ) -> Result<Vec<RuleViolation>> {
        let mut violations = Vec::new();

        // Map column names to indices
        let column_indices: Vec<usize> = rule.column_names
            .iter()
            .filter_map(|col_name| schema.get_column_index(col_name))
            .collect();

        if column_indices.is_empty() {
            return Ok(violations);
        }

        let reference_cache = self.reference_cache.read();

        for tuple in data {
            // Build key from column values
            let key: String = column_indices
                .iter()
                .map(|&idx| {
                    tuple.get(idx)
                        .map(|v| v.to_display_string())
                        .unwrap_or_else(|| "NULL".to_string())
                })
                .collect::<Vec<_>>()
                .join("|");

            // Check if NULL is allowed
            if key.contains("NULL") && self.config.allow_null {
                continue;
            }

            // Check if value exists in referenced table
            if !reference_cache.contains(&key) {
                violations.push(RuleViolation {
                    rule_id: rule.id.clone(),
                    rule_name: rule.name.clone(),
                    table_name: rule.table_name.clone(),
                    column_name: Some(rule.column_names.join(", ")),
                    row_id: Some(tuple.row_id),
                    severity: rule.severity,
                    message: format!(
                        "Foreign key violation: value not found in {}.{}",
                        self.config.referenced_table,
                        self.config.referenced_columns.join(", ")
                    ),
                    actual_value: Some(key),
                    expected_value: Some(format!("exists in {}", self.config.referenced_table)),
                    timestamp: std::time::SystemTime::now(),
                });
            }
        }

        Ok(violations)
    }

    pub fn load_references(&self, data: &[Tuple], column_indices: &[usize]) {
        let mut cache = self.reference_cache.write();
        cache.clear();

        for tuple in data {
            let key: String = column_indices
                .iter()
                .map(|&idx| {
                    tuple.get(idx)
                        .map(|v| v.to_display_string())
                        .unwrap_or_else(|| "NULL".to_string())
                })
                .collect::<Vec<_>>()
                .join("|");
            cache.insert(key);
        }
    }
}

/// Main quality rules engine
pub struct QualityRulesEngine {
    rules: HashMap<String, QualityRule>,
}

impl QualityRulesEngine {
    pub fn new() -> Self {
        Self {
            rules: HashMap::new(),
        }
    }

    pub fn add_rule(&mut self, rule: QualityRule) -> Result<()> {
        self.rules.insert(rule.id.clone(), rule);
        Ok(())
    }

    pub fn remove_rule(&mut self, rule_id: &str) -> Result<()> {
        self.rules.remove(rule_id);
        Ok(())
    }

    pub fn get_rule(&self, rule_id: &str) -> Option<&QualityRule> {
        self.rules.get(rule_id)
    }

    pub fn rule_count(&self) -> usize {
        self.rules.len()
    }

    pub fn validate_table(
        &self,
        table_name: &str,
        schema: &Schema,
        data: &[Tuple],
    ) -> Result<Vec<RuleViolation>> {
        let mut all_violations = Vec::new();

        for rule in self.rules.values() {
            if rule.table_name != table_name || rule.status != RuleStatus::Active {
                continue;
            }

            let violations = match &rule.config {
                RuleConfig::Completeness(config) => {
                    let completeness_rule = CompletenessRule::new(config.clone());
                    if let Some(col_name) = rule.column_names.first() {
                        if let Some(col_idx) = schema.get_column_index(col_name) {
                            completeness_rule.validate(rule, schema, data, col_idx)?
                        } else {
                            Vec::new()
                        }
                    } else {
                        Vec::new()
                    }
                }
                RuleConfig::Uniqueness(config) => {
                    let uniqueness_rule = UniquenessRule::new(config.clone());
                    uniqueness_rule.validate(rule, schema, data)?
                }
                RuleConfig::Format(config) => {
                    let format_rule = FormatRule::new(config.clone())?;
                    if let Some(col_name) = rule.column_names.first() {
                        if let Some(col_idx) = schema.get_column_index(col_name) {
                            format_rule.validate(rule, schema, data, col_idx)?
                        } else {
                            Vec::new()
                        }
                    } else {
                        Vec::new()
                    }
                }
                RuleConfig::Range(config) => {
                    let range_rule = RangeRule::new(config.clone());
                    if let Some(col_name) = rule.column_names.first() {
                        if let Some(col_idx) = schema.get_column_index(col_name) {
                            range_rule.validate(rule, schema, data, col_idx)?
                        } else {
                            Vec::new()
                        }
                    } else {
                        Vec::new()
                    }
                }
                RuleConfig::ReferentialIntegrity(config) => {
                    let ref_rule = ReferentialIntegrityRule::new(config.clone());
                    ref_rule.validate(rule, schema, data)?
                }
                RuleConfig::Custom(_) => {
                    // Custom rules not implemented in this version
                    Vec::new()
                }
            };

            all_violations.extend(violations);
        }

        Ok(all_violations)
    }
}

impl Default for QualityRulesEngine {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rules_engine_creation() {
        let engine = QualityRulesEngine::new();
        assert_eq!(engine.rule_count(), 0);
    }

    #[test]
    fn test_completeness_rule() {
        let config = CompletenessConfig {
            allow_null: false,
            min_completeness_percent: 95.0,
        };
        let _rule = CompletenessRule::new(config);
    }
}
