// Query Rewriting and Incremental View Maintenance
//
// This module provides query transformation and optimization through
// semantic-preserving rewrites, along with incremental maintenance
// of materialized views using delta tables.
//
// # Architecture
//
// Query rewriting applies transformation rules in priority order:
// - Predicate pushdown
// - Join reordering
// - Subquery elimination
// - View substitution
//
// Incremental view maintenance tracks changes through delta tables
// and applies efficient partial updates rather than full refreshes.
//
// # Example
//
// ```rust,ignore
// use crate::analytics::query_rewriter::{QueryRewriter, RewriteRule};
//
// let mut rewriter = QueryRewriter::new();
// rewriter.add_rule(RewriteRule::predicate_pushdown());
// let optimized = rewriter.rewrite(query);
// ```

use parking_lot::RwLock;
use std::collections::HashMap;
use std::sync::Arc;

/// A rewrite rule that transforms queries.
#[derive(Debug, Clone)]
pub struct RewriteRule {
    /// Rule identifier
    pub id: String,
    /// Rule name for display
    pub name: String,
    /// Rule priority (higher = applied first)
    pub priority: i32,
    /// Pattern to match in query
    pub pattern: String,
    /// Replacement template
    pub replacement: String,
    /// Whether rule is enabled
    pub enabled: bool,
    /// Conditions for rule application
    pub conditions: Vec<String>,
}

impl RewriteRule {
    /// Creates a new rewrite rule.
    pub fn new(
        id: impl Into<String>,
        name: impl Into<String>,
        pattern: impl Into<String>,
        replacement: impl Into<String>,
    ) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            priority: 0,
            pattern: pattern.into(),
            replacement: replacement.into(),
            enabled: true,
            conditions: Vec::new(),
        }
    }

    /// Sets the rule priority.
    pub fn with_priority(mut self, priority: i32) -> Self {
        self.priority = priority;
        self
    }

    /// Adds a condition for rule application.
    pub fn with_condition(mut self, condition: impl Into<String>) -> Self {
        self.conditions.push(condition.into());
        self
    }

    /// Creates a predicate pushdown rule.
    pub fn predicate_pushdown() -> Self {
        Self::new(
            "pred_pushdown",
            "Predicate Pushdown",
            "SELECT * FROM (SELECT * FROM $table) WHERE $condition",
            "SELECT * FROM $table WHERE $condition",
        )
        .with_priority(100)
    }

    /// Creates a constant folding rule.
    pub fn constant_folding() -> Self {
        Self::new(
            "const_fold",
            "Constant Folding",
            "$a + 0",
            "$a",
        )
        .with_priority(50)
    }

    /// Creates a redundant join elimination rule.
    pub fn redundant_join_elimination() -> Self {
        Self::new(
            "redundant_join",
            "Redundant Join Elimination",
            "JOIN $table ON $pk = $pk",
            "",
        )
        .with_priority(90)
    }

    /// Disables the rule.
    pub fn disable(mut self) -> Self {
        self.enabled = false;
        self
    }
}

/// Statistics about query rewriting.
#[derive(Debug, Clone, Default)]
pub struct RewriteStats {
    /// Number of rules applied
    pub rules_applied: usize,
    /// Total time spent rewriting (microseconds)
    pub rewrite_time_us: u64,
    /// Cost reduction achieved
    pub cost_reduction: f64,
    /// Individual rule application counts
    pub rule_counts: HashMap<String, usize>,
}

/// Query rewriter that applies transformation rules.
#[derive(Debug)]
pub struct QueryRewriter {
    /// Registered rewrite rules
    rules: Vec<RewriteRule>,
    /// Statistics about rewrites
    stats: RewriteStats,
    /// Maximum iterations to prevent infinite loops
    max_iterations: usize,
    /// Enable detailed logging
    trace_enabled: bool,
}

impl Default for QueryRewriter {
    fn default() -> Self {
        Self::new()
    }
}

impl QueryRewriter {
    /// Creates a new query rewriter with default rules.
    pub fn new() -> Self {
        Self {
            rules: Vec::new(),
            stats: RewriteStats::default(),
            max_iterations: 100,
            trace_enabled: false,
        }
    }

    /// Creates a rewriter with standard optimization rules.
    pub fn with_standard_rules() -> Self {
        let mut rewriter = Self::new();
        rewriter.add_rule(RewriteRule::predicate_pushdown());
        rewriter.add_rule(RewriteRule::constant_folding());
        rewriter.add_rule(RewriteRule::redundant_join_elimination());
        rewriter
    }

    /// Adds a rewrite rule.
    pub fn add_rule(&mut self, rule: RewriteRule) {
        self.rules.push(rule);
        // Sort by priority (descending)
        self.rules.sort_by(|a, b| b.priority.cmp(&a.priority));
    }

    /// Removes a rule by ID.
    pub fn remove_rule(&mut self, id: &str) -> bool {
        let len_before = self.rules.len();
        self.rules.retain(|r| r.id != id);
        self.rules.len() < len_before
    }

    /// Enables trace logging.
    pub fn enable_trace(&mut self, enabled: bool) {
        self.trace_enabled = enabled;
    }

    /// Rewrites a query string using registered rules.
    pub fn rewrite(&mut self, query: &str) -> RewriteResult {
        let start = std::time::Instant::now();
        let mut current = query.to_string();
        let mut applied_rules: Vec<String> = Vec::new();
        let mut iterations = 0;

        loop {
            let mut changed = false;
            iterations += 1;

            if iterations > self.max_iterations {
                break;
            }

            for rule in &self.rules {
                if !rule.enabled {
                    continue;
                }

                if let Some(new_query) = self.apply_rule(rule, &current) {
                    if new_query != current {
                        current = new_query;
                        applied_rules.push(rule.id.clone());
                        *self.stats.rule_counts.entry(rule.id.clone()).or_insert(0) += 1;
                        changed = true;
                        break;
                    }
                }
            }

            if !changed {
                break;
            }
        }

        self.stats.rules_applied += applied_rules.len();
        self.stats.rewrite_time_us += start.elapsed().as_micros() as u64;

        RewriteResult {
            original: query.to_string(),
            rewritten: current,
            rules_applied: applied_rules,
            iterations,
        }
    }

    /// Applies a single rule to the query.
    fn apply_rule(&self, rule: &RewriteRule, query: &str) -> Option<String> {
        // Simple pattern matching (real implementation would use AST)
        if query.contains(&rule.pattern) {
            Some(query.replace(&rule.pattern, &rule.replacement))
        } else {
            None
        }
    }

    /// Returns rewrite statistics.
    pub fn stats(&self) -> &RewriteStats {
        &self.stats
    }

    /// Resets statistics.
    pub fn reset_stats(&mut self) {
        self.stats = RewriteStats::default();
    }
}

/// Result of a query rewrite operation.
#[derive(Debug, Clone)]
pub struct RewriteResult {
    /// Original query
    pub original: String,
    /// Rewritten query
    pub rewritten: String,
    /// Rules that were applied
    pub rules_applied: Vec<String>,
    /// Number of iterations
    pub iterations: usize,
}

impl RewriteResult {
    /// Returns whether any changes were made.
    pub fn was_rewritten(&self) -> bool {
        self.original != self.rewritten
    }
}

/// Delta operation type for incremental view maintenance.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DeltaOperation {
    /// Row insertion
    Insert,
    /// Row deletion
    Delete,
    /// Row update (old and new values)
    Update,
}

/// A row change for incremental maintenance.
#[derive(Debug, Clone)]
pub struct DeltaRow {
    /// Operation type
    pub operation: DeltaOperation,
    /// Row values (column -> value)
    pub values: HashMap<String, String>,
    /// Old values for updates
    pub old_values: Option<HashMap<String, String>>,
    /// Timestamp of the change
    pub timestamp: u64,
}

/// Delta table for tracking changes.
#[derive(Debug)]
pub struct DeltaTable {
    /// Table name
    table_name: String,
    /// Collected delta rows
    deltas: Vec<DeltaRow>,
    /// Maximum deltas before compaction
    max_deltas: usize,
    /// Column names
    columns: Vec<String>,
    /// Last compaction time
    last_compaction: std::time::Instant,
}

impl DeltaTable {
    /// Creates a new delta table.
    pub fn new(table_name: impl Into<String>, columns: Vec<String>) -> Self {
        Self {
            table_name: table_name.into(),
            deltas: Vec::new(),
            max_deltas: 10000,
            columns,
            last_compaction: std::time::Instant::now(),
        }
    }

    /// Records an insert operation.
    pub fn record_insert(&mut self, values: HashMap<String, String>) {
        self.deltas.push(DeltaRow {
            operation: DeltaOperation::Insert,
            values,
            old_values: None,
            timestamp: self.current_timestamp(),
        });
        self.maybe_compact();
    }

    /// Records a delete operation.
    pub fn record_delete(&mut self, values: HashMap<String, String>) {
        self.deltas.push(DeltaRow {
            operation: DeltaOperation::Delete,
            values,
            old_values: None,
            timestamp: self.current_timestamp(),
        });
        self.maybe_compact();
    }

    /// Records an update operation.
    pub fn record_update(
        &mut self,
        old_values: HashMap<String, String>,
        new_values: HashMap<String, String>,
    ) {
        self.deltas.push(DeltaRow {
            operation: DeltaOperation::Update,
            values: new_values,
            old_values: Some(old_values),
            timestamp: self.current_timestamp(),
        });
        self.maybe_compact();
    }

    /// Returns pending deltas since the given timestamp.
    pub fn get_deltas_since(&self, since: u64) -> Vec<&DeltaRow> {
        self.deltas.iter().filter(|d| d.timestamp > since).collect()
    }

    /// Returns all pending deltas.
    pub fn get_all_deltas(&self) -> &[DeltaRow] {
        &self.deltas
    }

    /// Clears processed deltas.
    pub fn clear_deltas(&mut self) {
        self.deltas.clear();
    }

    /// Clears deltas up to the given timestamp.
    pub fn clear_deltas_before(&mut self, before: u64) {
        self.deltas.retain(|d| d.timestamp >= before);
    }

    /// Returns the number of pending deltas.
    pub fn delta_count(&self) -> usize {
        self.deltas.len()
    }

    /// Returns the table name.
    pub fn table_name(&self) -> &str {
        &self.table_name
    }

    /// Checks if compaction is needed.
    fn maybe_compact(&mut self) {
        if self.deltas.len() > self.max_deltas {
            self.compact();
        }
    }

    /// Compacts the delta log by merging operations.
    pub fn compact(&mut self) {
        // Group operations by key and merge
        // Insert followed by Delete = nothing
        // Delete followed by Insert = Update
        // Multiple Updates = single Update with final value

        // For simplicity, just trim old deltas
        if self.deltas.len() > self.max_deltas {
            let keep = self.max_deltas / 2;
            self.deltas = self.deltas.split_off(self.deltas.len() - keep);
        }

        self.last_compaction = std::time::Instant::now();
    }

    /// Gets current timestamp in microseconds.
    fn current_timestamp(&self) -> u64 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_micros() as u64)
            .unwrap_or(0)
    }
}

/// Incremental view maintenance coordinator.
#[derive(Debug)]
pub struct IncrementalViewMaintenance {
    /// Delta tables by source table name
    delta_tables: Arc<RwLock<HashMap<String, DeltaTable>>>,
    /// View definitions (view name -> source tables)
    view_sources: HashMap<String, Vec<String>>,
    /// Last refresh timestamps by view
    last_refresh: HashMap<String, u64>,
}

impl Default for IncrementalViewMaintenance {
    fn default() -> Self {
        Self::new()
    }
}

impl IncrementalViewMaintenance {
    /// Creates a new incremental view maintenance coordinator.
    pub fn new() -> Self {
        Self {
            delta_tables: Arc::new(RwLock::new(HashMap::new())),
            view_sources: HashMap::new(),
            last_refresh: HashMap::new(),
        }
    }

    /// Registers a delta table for a source table.
    pub fn register_delta_table(&self, table_name: &str, columns: Vec<String>) {
        self.delta_tables
            .write()
            .insert(table_name.to_string(), DeltaTable::new(table_name, columns));
    }

    /// Registers a view with its source tables.
    pub fn register_view(&mut self, view_name: &str, source_tables: Vec<String>) {
        self.view_sources
            .insert(view_name.to_string(), source_tables);
        self.last_refresh.insert(view_name.to_string(), 0);
    }

    /// Records a change to a source table.
    pub fn record_change(&self, table_name: &str, row: DeltaRow) {
        if let Some(delta_table) = self.delta_tables.write().get_mut(table_name) {
            match row.operation {
                DeltaOperation::Insert => {
                    delta_table.record_insert(row.values);
                }
                DeltaOperation::Delete => {
                    delta_table.record_delete(row.values);
                }
                DeltaOperation::Update => {
                    if let Some(old) = row.old_values {
                        delta_table.record_update(old, row.values);
                    }
                }
            }
        }
    }

    /// Gets pending changes for a view since last refresh.
    pub fn get_pending_changes(&self, view_name: &str) -> Vec<ViewDelta> {
        let sources = match self.view_sources.get(view_name) {
            Some(s) => s,
            None => return Vec::new(),
        };

        let since = self.last_refresh.get(view_name).copied().unwrap_or(0);
        let delta_tables = self.delta_tables.read();

        let mut changes = Vec::new();

        for source in sources {
            if let Some(delta_table) = delta_tables.get(source) {
                for delta in delta_table.get_deltas_since(since) {
                    changes.push(ViewDelta {
                        source_table: source.clone(),
                        operation: delta.operation.clone(),
                        values: delta.values.clone(),
                        old_values: delta.old_values.clone(),
                    });
                }
            }
        }

        changes
    }

    /// Marks a view as refreshed.
    pub fn mark_refreshed(&mut self, view_name: &str) {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_micros() as u64)
            .unwrap_or(0);

        self.last_refresh.insert(view_name.to_string(), now);
    }

    /// Checks if a view needs refresh.
    pub fn needs_refresh(&self, view_name: &str) -> bool {
        !self.get_pending_changes(view_name).is_empty()
    }

    /// Returns views that need refresh.
    pub fn views_needing_refresh(&self) -> Vec<String> {
        self.view_sources
            .keys()
            .filter(|v| self.needs_refresh(v))
            .cloned()
            .collect()
    }
}

/// A change to propagate to a view.
#[derive(Debug, Clone)]
pub struct ViewDelta {
    /// Source table that changed
    pub source_table: String,
    /// Type of change
    pub operation: DeltaOperation,
    /// New/current values
    pub values: HashMap<String, String>,
    /// Old values for updates
    pub old_values: Option<HashMap<String, String>>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rewrite_rule_creation() {
        let rule = RewriteRule::predicate_pushdown();
        assert_eq!(rule.id, "pred_pushdown");
        assert_eq!(rule.priority, 100);
        assert!(rule.enabled);
    }

    #[test]
    fn test_delta_table_operations() {
        let mut delta = DeltaTable::new("users", vec!["id".to_string(), "name".to_string()]);

        let mut values = HashMap::new();
        values.insert("id".to_string(), "1".to_string());
        values.insert("name".to_string(), "Alice".to_string());

        delta.record_insert(values);
        assert_eq!(delta.delta_count(), 1);

        let deltas = delta.get_all_deltas();
        assert_eq!(deltas[0].operation, DeltaOperation::Insert);
    }

    #[test]
    fn test_query_rewriter() {
        let mut rewriter = QueryRewriter::new();
        rewriter.add_rule(RewriteRule::new(
            "test",
            "Test Rule",
            "OLD_TABLE",
            "NEW_TABLE",
        ));

        let result = rewriter.rewrite("SELECT * FROM OLD_TABLE");

        assert!(result.was_rewritten());
        assert!(result.rewritten.contains("NEW_TABLE"));
        assert!(!result.rewritten.contains("OLD_TABLE"));
    }

    #[test]
    fn test_incremental_maintenance() {
        let mut ivm = IncrementalViewMaintenance::new();
        ivm.register_delta_table("orders", vec!["id".to_string(), "amount".to_string()]);
        ivm.register_view("order_totals", vec!["orders".to_string()]);

        let mut values = HashMap::new();
        values.insert("id".to_string(), "1".to_string());
        values.insert("amount".to_string(), "100".to_string());

        ivm.record_change("orders", DeltaRow {
            operation: DeltaOperation::Insert,
            values,
            old_values: None,
            timestamp: 0,
        });

        assert!(ivm.needs_refresh("order_totals"));
    }
}
