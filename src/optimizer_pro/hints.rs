// Optimizer Hints System - Oracle-compatible optimizer hints
//
// Implements:
// - Optimizer hints parsing
// - Hint validation
// - Conflicting hint resolution
// - Hint reporting

use std::collections::HashSet;
use std::fmt;
use crate::error::{Result, DbError};
use std::collections::{HashMap};


// ============================================================================
// Hint Parser
// ============================================================================

// Optimizer hint parser
pub struct HintParser {
    // Supported hints registry
    supported_hints: HashMap<String, HintDefinition>,
    // Hint validation rules
    validation_rules: Vec<Box<dyn HintValidationRule>>,
}

impl HintParser {
    // Create a new hint parser
    pub fn new() -> Self {
        let mut parser = Self {
            supported_hints: HashMap::new(),
            validation_rules: vec![],
        };

        parser.register_standard_hints();
        parser.register_validation_rules();

        parser
    }

    // Register standard Oracle-compatible hints
    fn register_standard_hints(&mut self) {
        // Access path hints
        self.register_hint(HintDefinition {
            name: "FULL".to_string(),
            category: HintCategory::AccessPath,
            description: "Force full table scan".to_string(),
            parameters: vec!["table".to_string()],
        });

        self.register_hint(HintDefinition {
            name: "INDEX".to_string(),
            category: HintCategory::AccessPath,
            description: "Force index scan".to_string(),
            parameters: vec!["table".to_string(), "index".to_string()],
        });

        self.register_hint(HintDefinition {
            name: "INDEX_FFS".to_string(),
            category: HintCategory::AccessPath,
            description: "Force index fast full scan".to_string(),
            parameters: vec!["table".to_string(), "index".to_string()],
        });

        self.register_hint(HintDefinition {
            name: "NO_INDEX".to_string(),
            category: HintCategory::AccessPath,
            description: "Disable index scan".to_string(),
            parameters: vec!["table".to_string(), "index".to_string()],
        });

        // Join method hints
        self.register_hint(HintDefinition {
            name: "USE_NL".to_string(),
            category: HintCategory::JoinMethod,
            description: "Use nested loop join".to_string(),
            parameters: vec!["table1".to_string(), "table2".to_string()],
        });

        self.register_hint(HintDefinition {
            name: "USE_HASH".to_string(),
            category: HintCategory::JoinMethod,
            description: "Use hash join".to_string(),
            parameters: vec!["table1".to_string(), "table2".to_string()],
        });

        self.register_hint(HintDefinition {
            name: "USE_MERGE".to_string(),
            category: HintCategory::JoinMethod,
            description: "Use merge join".to_string(),
            parameters: vec!["table1".to_string(), "table2".to_string()],
        });

        self.register_hint(HintDefinition {
            name: "NO_USE_NL".to_string(),
            category: HintCategory::JoinMethod,
            description: "Disable nested loop join".to_string(),
            parameters: vec!["table1".to_string(), "table2".to_string()],
        });

        self.register_hint(HintDefinition {
            name: "NO_USE_HASH".to_string(),
            category: HintCategory::JoinMethod,
            description: "Disable hash join".to_string(),
            parameters: vec!["table1".to_string(), "table2".to_string()],
        });

        self.register_hint(HintDefinition {
            name: "NO_USE_MERGE".to_string(),
            category: HintCategory::JoinMethod,
            description: "Disable merge join".to_string(),
            parameters: vec!["table1".to_string(), "table2".to_string()],
        });

        // Join order hints
        self.register_hint(HintDefinition {
            name: "LEADING".to_string(),
            category: HintCategory::JoinOrder,
            description: "Specify join order".to_string(),
            parameters: vec!["tables".to_string()],
        });

        self.register_hint(HintDefinition {
            name: "ORDERED".to_string(),
            category: HintCategory::JoinOrder,
            description: "Use FROM clause order".to_string(),
            parameters: vec![],
        });

        // Parallel execution hints
        self.register_hint(HintDefinition {
            name: "PARALLEL".to_string(),
            category: HintCategory::Parallel,
            description: "Enable parallel execution".to_string(),
            parameters: vec!["table".to_string(), "degree".to_string()],
        });

        self.register_hint(HintDefinition {
            name: "NO_PARALLEL".to_string(),
            category: HintCategory::Parallel,
            description: "Disable parallel execution".to_string(),
            parameters: vec!["table".to_string()],
        });

        // Optimizer mode hints
        self.register_hint(HintDefinition {
            name: "ALL_ROWS".to_string(),
            category: HintCategory::OptimizerMode,
            description: "Optimize for throughput".to_string(),
            parameters: vec![],
        });

        self.register_hint(HintDefinition {
            name: "FIRST_ROWS".to_string(),
            category: HintCategory::OptimizerMode,
            description: "Optimize for response time".to_string(),
            parameters: vec!["n".to_string()],
        });

        // Query transformation hints
        self.register_hint(HintDefinition {
            name: "NO_QUERY_TRANSFORMATION".to_string(),
            category: HintCategory::Transformation,
            description: "Disable query transformations".to_string(),
            parameters: vec![],
        });

        self.register_hint(HintDefinition {
            name: "NO_EXPAND".to_string(),
            category: HintCategory::Transformation,
            description: "Disable OR expansion".to_string(),
            parameters: vec![],
        });

        self.register_hint(HintDefinition {
            name: "USE_CONCAT".to_string(),
            category: HintCategory::Transformation,
            description: "Force OR expansion".to_string(),
            parameters: vec![],
        });

        self.register_hint(HintDefinition {
            name: "MERGE".to_string(),
            category: HintCategory::Transformation,
            description: "Merge view".to_string(),
            parameters: vec!["view".to_string()],
        });

        self.register_hint(HintDefinition {
            name: "NO_MERGE".to_string(),
            category: HintCategory::Transformation,
            description: "Prevent view merge".to_string(),
            parameters: vec!["view".to_string()],
        });

        // Materialized view hints
        self.register_hint(HintDefinition {
            name: "REWRITE".to_string(),
            category: HintCategory::MaterializedView,
            description: "Enable MV rewrite".to_string(),
            parameters: vec![],
        });

        self.register_hint(HintDefinition {
            name: "NO_REWRITE".to_string(),
            category: HintCategory::MaterializedView,
            description: "Disable MV rewrite".to_string(),
            parameters: vec![],
        });

        // Result cache hints
        self.register_hint(HintDefinition {
            name: "RESULT_CACHE".to_string(),
            category: HintCategory::Cache,
            description: "Cache query results".to_string(),
            parameters: vec![],
        });

        self.register_hint(HintDefinition {
            name: "NO_RESULT_CACHE".to_string(),
            category: HintCategory::Cache,
            description: "Disable result cache".to_string(),
            parameters: vec![],
        });

        // Cardinality hints
        self.register_hint(HintDefinition {
            name: "CARDINALITY".to_string(),
            category: HintCategory::Cardinality,
            description: "Specify cardinality".to_string(),
            parameters: vec!["table".to_string(), "rows".to_string()],
        });
    }

    // Register hint definition
    fn register_hint(&mut self, definition: HintDefinition) {
        self.supported_hints.insert(definition.name.clone(), definition);
    }

    // Register validation rules
    fn register_validation_rules(&mut self) {
        // Add validation rules
    }

    // Parse hints from query text
    pub fn parse_hints(&self, query_text: &str) -> Result<Vec<OptimizerHint>> {
        let mut hints = Vec::new();

        // Look for hints in the format: /*+ HINT1 HINT2 ... */
        if let Some(hint_block) = self.extract_hint_block(query_text) {
            let hint_strings = self.split_hints(&hint_block);

            for hint_str in hint_strings {
                if let Some(hint) = self.parse_single_hint(&hint_str)? {
                    hints.push(hint);
                }
            }
        }

        // Validate hints
        let validator = HintValidator::new();
        validator.validate(&hints)?;

        Ok(hints)
    }

    // Extract hint block from query
    fn extract_hint_block(&self, query_text: &str) -> Option<String> {
        // Find /*+ ... */ pattern
        if let Some(start) = query_text.find("/*+") {
            if let Some(end) = query_text[start..].find("*/") {
                let hint_text = &query_text[start + 3..start + end];
                return Some(hint_text.trim().to_string());
            }
        }
        None
    }

    // Split hint block into individual hints
    fn split_hints(&self, hint_block: &str) -> Vec<String> {
        hint_block
            .split_whitespace()
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string())
            .collect()
    }

    // Parse a single hint
    #[inline]
    fn parse_single_hint(&self, hint_str: &str) -> Result<Option<OptimizerHint>> {
        // Parse hint name and parameters
        let parts: Vec<&str> = hint_str.trim_matches(|c| c == '(' || c == ')').split('(').collect();
        let hint_name = parts[0].to_uppercase();

        if !self.supported_hints.contains_key(&hint_name) {
            // Unknown hint - log warning but don't fail
            return Ok(None);
        }

        let params = if parts.len() > 1 {
            parts[1]
                .split(',')
                .map(|s| s.trim().to_string())
                .collect()
        } else {
            vec![]
        };

        // Create OptimizerHint based on hint name
        let hint = match hint_name.as_str() {
            "FULL" => OptimizerHint::FullTableScan {
                table: params.get(0).cloned().unwrap_or_default(),
            },
            "INDEX" => OptimizerHint::UseIndex {
                table: params.get(0).cloned().unwrap_or_default(),
                index: params.get(1).cloned().unwrap_or_default(),
            },
            "NO_INDEX" => OptimizerHint::NoIndex {
                table: params.get(0).cloned().unwrap_or_default(),
                index: params.get(1).cloned().unwrap_or_default(),
            },
            "USE_NL" => OptimizerHint::UseNestedLoop {
                tables: params.clone(),
            },
            "USE_HASH" => OptimizerHint::UseHashJoin {
                tables: params.clone(),
            },
            "USE_MERGE" => OptimizerHint::UseMergeJoin {
                tables: params.clone(),
            },
            "NO_USE_NL" => OptimizerHint::NoNestedLoop,
            "NO_USE_HASH" => OptimizerHint::NoHashJoin,
            "NO_USE_MERGE" => OptimizerHint::NoMergeJoin,
            "LEADING" => OptimizerHint::Leading {
                tables: params.clone(),
            },
            "ORDERED" => OptimizerHint::Ordered,
            "PARALLEL" => OptimizerHint::Parallel {
                table: params.get(0).cloned().unwrap_or_default(),
                degree: params.get(1).and_then(|s| s.parse().ok()).unwrap_or(4),
            },
            "NO_PARALLEL" => OptimizerHint::NoParallel {
                table: params.get(0).cloned().unwrap_or_default(),
            },
            "ALL_ROWS" => OptimizerHint::AllRows,
            "FIRST_ROWS" => OptimizerHint::FirstRows {
                n: params.get(0).and_then(|s| s.parse().ok()).unwrap_or(1),
            },
            "NO_QUERY_TRANSFORMATION" => OptimizerHint::NoQueryTransformation,
            "NO_EXPAND" => OptimizerHint::NoExpand,
            "USE_CONCAT" => OptimizerHint::UseConcat,
            "MERGE" => OptimizerHint::MergeView {
                view: params.get(0).cloned().unwrap_or_default(),
            },
            "NO_MERGE" => OptimizerHint::NoMerge {
                view: params.get(0).cloned().unwrap_or_default(),
            },
            "REWRITE" => OptimizerHint::Rewrite,
            "NO_REWRITE" => OptimizerHint::NoRewrite,
            "RESULT_CACHE" => OptimizerHint::ResultCache,
            "NO_RESULT_CACHE" => OptimizerHint::NoResultCache,
            "CARDINALITY" => OptimizerHint::Cardinality {
                table: params.get(0).cloned().unwrap_or_default(),
                rows: params.get(1).and_then(|s| s.parse().ok()).unwrap_or(1000),
            },
            _ => return Ok(None),
        };

        Ok(Some(hint))
    }

    // Get supported hints
    pub fn get_supported_hints(&self) -> Vec<&HintDefinition> {
        self.supported_hints.values().collect()
    }

    // Get hint definition
    pub fn get_hint_definition(&self, name: &str) -> Option<&HintDefinition> {
        self.supported_hints.get(&name.to_uppercase())
    }
}

// ============================================================================
// Optimizer Hints
// ============================================================================

// Optimizer hint types
#[derive(Debug, Clone, PartialEq)]
pub enum OptimizerHint {
    // Access path hints
    FullTableScan { table: String },
    UseIndex { table: String, index: String },
    NoIndex { table: String, index: String },
    IndexFastFullScan { table: String, index: String },
    NoSeqScan,
    NoIndexScan,
    NoBitmapScan,

    // Join method hints
    UseNestedLoop { tables: Vec<String> },
    UseHashJoin { tables: Vec<String> },
    UseMergeJoin { tables: Vec<String> },
    NoNestedLoop,
    NoHashJoin,
    NoMergeJoin,

    // Join order hints
    Leading { tables: Vec<String> },
    Ordered,

    // Parallel execution hints
    Parallel { table: String, degree: usize },
    NoParallel { table: String },

    // Optimizer mode hints
    AllRows,
    FirstRows { n: usize },

    // Query transformation hints
    NoQueryTransformation,
    NoExpand,
    UseConcat,
    MergeView { view: String },
    NoMerge { view: String },

    // Materialized view hints
    Rewrite,
    NoRewrite,

    // Cache hints
    ResultCache,
    NoResultCache,

    // Cardinality hints
    Cardinality { table: String, rows: usize },
    NoHashAggregate,
}

impl fmt::Display for OptimizerHint {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            OptimizerHint::FullTableScan { table } => write!(f, "FULL({})", table),
            OptimizerHint::UseIndex { table, index } => write!(f, "INDEX({} {})", table, index),
            OptimizerHint::NoIndex { table, index } => write!(f, "NO_INDEX({} {})", table, index),
            OptimizerHint::UseNestedLoop { tables } => {
                write!(f, "USE_NL({})", tables.join(" "))
            }
            OptimizerHint::UseHashJoin { tables } => {
                write!(f, "USE_HASH({})", tables.join(" "))
            }
            OptimizerHint::UseMergeJoin { tables } => {
                write!(f, "USE_MERGE({})", tables.join(" "))
            }
            OptimizerHint::Leading { tables } => {
                write!(f, "LEADING({})", tables.join(" "))
            }
            OptimizerHint::Parallel { table, degree } => {
                write!(f, "PARALLEL({} {})", table, degree)
            }
            OptimizerHint::FirstRows { n } => write!(f, "FIRST_ROWS({})", n),
            OptimizerHint::Cardinality { table, rows } => {
                write!(f, "CARDINALITY({} {})", table, rows)
            }
            _ => write!(f, "{:?}", self),
        }
    }
}

// ============================================================================
// Hint Validation
// ============================================================================

// Hint validator
pub struct HintValidator {
    // Conflict rules
    conflict_rules: Vec<ConflictRule>,
}

impl HintValidator {
    pub fn new() -> Self {
        let mut validator = Self {
            conflict_rules: vec![],
        };

        validator.register_conflict_rules();

        validator
    }

    // Register conflict rules
    fn register_conflict_rules(&mut self) {
        // Join method conflicts
        self.conflict_rules.push(ConflictRule {
            hint1: "USE_NL".to_string(),
            hint2: "USE_HASH".to_string(),
            severity: ConflictSeverity::Error,
            message: "Cannot use both nested loop and hash join".to_string(),
        });

        self.conflict_rules.push(ConflictRule {
            hint1: "USE_NL".to_string(),
            hint2: "NO_USE_NL".to_string(),
            severity: ConflictSeverity::Error,
            message: "Conflicting nested loop hints".to_string(),
        });

        // Access path conflicts
        self.conflict_rules.push(ConflictRule {
            hint1: "FULL".to_string(),
            hint2: "INDEX".to_string(),
            severity: ConflictSeverity::Warning,
            message: "Full scan and index scan hints conflict".to_string(),
        });

        // Transformation conflicts
        self.conflict_rules.push(ConflictRule {
            hint1: "NO_EXPAND".to_string(),
            hint2: "USE_CONCAT".to_string(),
            severity: ConflictSeverity::Error,
            message: "Conflicting expansion hints".to_string(),
        });
    }

    // Validate hints
    pub fn validate(&self, hints: &[OptimizerHint]) -> Result<()> {
        // Check for conflicts
        for i in 0..hints.len() {
            for j in i + 1..hints.len() {
                if let Some(conflict) = self.check_conflict(&hints[i], &hints[j]) {
                    match conflict.severity {
                        ConflictSeverity::Error => {
                            return Err(DbError::Internal(format!(
                                "Hint conflict: {}",
                                conflict.message
                            )));
                        }
                        ConflictSeverity::Warning => {
                            // Log warning
                        }
                        ConflictSeverity::Info => {
                            // Log info
                        }
                    }
                }
            }
        }

        Ok(())
    }

    // Check for conflict between two hints
    fn check_conflict(&self, hint1: &OptimizerHint, hint2: &OptimizerHint) -> Option<&ConflictRule> {
        let name1 = self.get_hint_name(hint1);
        let name2 = self.get_hint_name(hint2);

        self.conflict_rules.iter().find(|rule| {
            (rule.hint1 == name1 && rule.hint2 == name2)
                || (rule.hint1 == name2 && rule.hint2 == name1)
        })
    }

    // Get hint name
    fn get_hint_name(&self, hint: &OptimizerHint) -> String {
        match hint {
            OptimizerHint::FullTableScan { .. } => "FULL".to_string(),
            OptimizerHint::UseIndex { .. } => "INDEX".to_string(),
            OptimizerHint::NoIndex { .. } => "NO_INDEX".to_string(),
            OptimizerHint::UseNestedLoop { .. } => "USE_NL".to_string(),
            OptimizerHint::UseHashJoin { .. } => "USE_HASH".to_string(),
            OptimizerHint::UseMergeJoin { .. } => "USE_MERGE".to_string(),
            OptimizerHint::NoNestedLoop => "NO_USE_NL".to_string(),
            OptimizerHint::NoHashJoin => "NO_USE_HASH".to_string(),
            OptimizerHint::NoMergeJoin => "NO_USE_MERGE".to_string(),
            OptimizerHint::NoExpand => "NO_EXPAND".to_string(),
            OptimizerHint::UseConcat => "USE_CONCAT".to_string(),
            _ => format!("{:?}", hint),
        }
    }

    // Resolve conflicts
    pub fn resolve_conflicts(&self, hints: Vec<OptimizerHint>) -> Vec<OptimizerHint> {
        // Apply resolution strategy (e.g., last hint wins)
        let mut resolved = Vec::new();
        let mut seen_types = HashSet::new();

        for hint in hints.into_iter().rev() {
            let hint_type = self.get_hint_type(&hint);
            if !seen_types.contains(&hint_type) {
                resolved.push(hint);
                seen_types.insert(hint_type);
            }
        }

        resolved.reverse();
        resolved
    }

    // Get hint type for conflict resolution
    fn get_hint_type(&self, hint: &OptimizerHint) -> String {
        match hint {
            OptimizerHint::UseNestedLoop { .. }
            | OptimizerHint::UseHashJoin { .. }
            | OptimizerHint::UseMergeJoin { .. } => "join_method".to_string(),
            OptimizerHint::FullTableScan { .. }
            | OptimizerHint::UseIndex { .. }
            | OptimizerHint::NoIndex { .. } => "access_path".to_string(),
            OptimizerHint::AllRows | OptimizerHint::FirstRows { .. } => {
                "optimizer_mode".to_string()
            }
            _ => "other".to_string(),
        }
    }
}

// ============================================================================
// Supporting Types
// ============================================================================

// Hint definition
#[derive(Debug, Clone)]
pub struct HintDefinition {
    pub name: String,
    pub category: HintCategory,
    pub description: String,
    pub parameters: Vec<String>,
}

// Hint category
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HintCategory {
    AccessPath,
    JoinMethod,
    JoinOrder,
    Parallel,
    OptimizerMode,
    Transformation,
    MaterializedView,
    Cache,
    Cardinality,
}

// Hint validation rule
trait HintValidationRule: Send + Sync {
    fn validate(&self, hint: &OptimizerHint) -> Result<()>;
}

// Conflict rule
#[derive(Debug, Clone)]
struct ConflictRule {
    hint1: String,
    hint2: String,
    severity: ConflictSeverity,
    message: String,
}

// Conflict severity
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ConflictSeverity {
    Error,
    Warning,
    Info,
}

// ============================================================================
// Hint Reporter
// ============================================================================

// Hint usage reporter
pub struct HintReporter {
    // Hint usage statistics
    usage_stats: std::sync::RwLock<HashMap<String, HintUsageStats>>,
}

impl HintReporter {
    pub fn new() -> Self {
        Self {
            usage_stats: std::sync::RwLock::new(HashMap::new()),
        }
    }

    // Record hint usage
    pub fn record_usage(&self, hint: &OptimizerHint, effective: bool) {
        let hint_name = format!("{:?}", hint);
        let mut stats = self.usage_stats.write().unwrap();

        let entry = stats.entry(hint_name).or_insert(HintUsageStats {
            total_uses: 0,
            effective_uses: 0,
            last_used: std::time::SystemTime::now(),
        });

        entry.total_uses += 1;
        if effective {
            entry.effective_uses += 1;
        }
        entry.last_used = std::time::SystemTime::now();
    }

    // Get usage report
    pub fn get_usage_report(&self) -> Vec<(String, HintUsageStats)> {
        self.usage_stats
            .read()
            .unwrap()
            .iter()
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect()
    }

    // Get effectiveness ratio
    pub fn get_effectiveness_ratio(&self, hint_name: &str) -> f64 {
        let stats = self.usage_stats.read().unwrap();

        if let Some(usage) = stats.get(hint_name) {
            if usage.total_uses > 0 {
                return usage.effective_uses as f64 / usage.total_uses as f64;
            }
        }

        0.0
    }
}

// Hint usage statistics
#[derive(Debug, Clone)]
pub struct HintUsageStats {
    pub total_uses: u64,
    pub effective_uses: u64,
    pub last_used: std::time::SystemTime,
}

#[cfg(test)]
mod tests {
    use super::*;
use std::time::SystemTime;

    #[test]
    fn test_hint_parser() {
        let parser = HintParser::new();

        let query = "SELECT /*+ FULL(users) INDEX(orders idx1) */ * FROM users, orders";
        let hints = parser.parse_hints(query).unwrap();

        assert!(!hints.is_empty());
    }

    #[test]
    fn test_hint_validation() {
        let validator = HintValidator::new();

        let hints = vec![
            OptimizerHint::UseNestedLoop {
                tables: vec!["users".to_string()],
            },
            OptimizerHint::UseHashJoin {
                tables: vec!["users".to_string()],
            },
        ];

        // Should detect conflict
        let result = validator.validate(&hints);
        assert!(result.is_err());
    }

    #[test]
    fn test_conflict_resolution() {
        let validator = HintValidator::new();

        let hints = vec![
            OptimizerHint::UseNestedLoop {
                tables: vec!["users".to_string()],
            },
            OptimizerHint::UseHashJoin {
                tables: vec!["users".to_string()],
            },
        ];

        let resolved = validator.resolve_conflicts(hints);
        assert_eq!(resolved.len(), 1);
    }

    #[test]
    fn test_hint_reporter() {
        let reporter = HintReporter::new();

        let hint = OptimizerHint::FullTableScan {
            table: "users".to_string(),
        };

        reporter.record_usage(&hint, true);
        reporter.record_usage(&hint, false);

        let ratio = reporter.get_effectiveness_ratio("FullTableScan { table: \"users\" }");
        assert!(ratio > 0.0);
    }

    #[test]
    fn test_hint_display() {
        let hint = OptimizerHint::UseIndex {
            table: "users".to_string(),
            index: "idx_users".to_string(),
        };

        let display = format!("{}", hint);
        assert!(display.contains("INDEX"));
    }

    #[test]
    fn test_supported_hints() {
        let parser = HintParser::new();
        let hints = parser.get_supported_hints();

        assert!(hints.len() > 0);

        // Check for key hints
        assert!(parser.get_hint_definition("FULL").is_some());
        assert!(parser.get_hint_definition("INDEX").is_some());
        assert!(parser.get_hint_definition("USE_NL").is_some());
        assert!(parser.get_hint_definition("USE_HASH").is_some());
    }
}
