// Index Advisor Module
//
// Provides intelligent index recommendations based on:
// - Query workload analysis
// - Missing index detection
// - Unused index identification
// - Index consolidation opportunities
// - Cost-benefit analysis

use crate::Result;
use std::collections::{HashMap, HashSet};
use serde::{Deserialize, Serialize};

// Index Advisor
pub struct IndexAdvisor {
    // Workload tracker
    workload: WorkloadTracker,
    // Existing indexes
    existing_indexes: Vec<IndexMetadata>,
    // Configuration
    config: AdvisorConfig,
}

impl IndexAdvisor {
    // Create a new index advisor
    pub fn new(config: AdvisorConfig) -> Self {
        Self {
            workload: WorkloadTracker::new(),
            existing_indexes: Vec::new(),
            config,
        }
    }

    // Register an existing index
    pub fn register_index(&mut self, metadata: IndexMetadata) {
        self.existing_indexes.push(metadata);
    }

    // Record a query for workload analysis
    pub fn record_query(&mut self, query: &Query) {
        self.workload.record_query(query);
    }

    // Analyze workload and generate recommendations
    pub fn analyze(&self) -> Result<Vec<IndexRecommendation>> {
        let mut recommendations = Vec::new();

        // Detect missing indexes
        recommendations.extend(self.detect_missing_indexes()?);

        // Detect unused indexes
        recommendations.extend(self.detect_unused_indexes()?);

        // Suggest index consolidation
        recommendations.extend(self.suggest_consolidation()?);

        // Identify redundant indexes
        recommendations.extend(self.identify_redundant_indexes()?);


        // Sort by priority
        recommendations.sort_by(|a, b| b.priority.cmp(&a.priority));

        Ok(recommendations)
    }

    // Detect missing indexes based on query patterns
    fn detect_missing_indexes(&self) -> Result<Vec<IndexRecommendation>> {
        let mut recommendations = Vec::new();

        // Analyze SELECT queries
        for (query_pattern, stats) in &self.workload.query_patterns {
            if stats.execution_count < self.config.min_query_count {
                continue;
            }

            // Check WHERE clauses
            for condition in &query_pattern.where_conditions {
                if !self.has_index_on_column(&condition.column) {
                    let benefit = self.estimate_benefit(stats);
                    let cost = self.estimate_index_cost(&condition.column);

                    if benefit > cost * self.config.benefit_threshold {
                        recommendations.push(IndexRecommendation {
                            recommendation_type: RecommendationType::CreateIndex,
                            table: query_pattern.table.clone(),
                            columns: vec![condition.column.clone()],
                            reason: format!(
                                "Frequently used in WHERE clause ({} executions, avg time: {:.2}ms)",
                                stats.execution_count,
                                stats.avg_execution_time_ms
                            ),
                            priority: self.calculate_priority(benefit, cost),
                            estimated_benefit: benefit,
                            estimated_cost: cost,
                        });
                    }
                }
            }

            // Check JOIN conditions
            for join in &query_pattern.joins {
                if !self.has_index_on_column(&join.join_column) {
                    let benefit = self.estimate_benefit(stats) * 1.5; // JOINs benefit more
                    let cost = self.estimate_index_cost(&join.join_column);

                    if benefit > cost * self.config.benefit_threshold {
                        recommendations.push(IndexRecommendation {
                            recommendation_type: RecommendationType::CreateIndex,
                            table: join.table.clone(),
                            columns: vec![join.join_column.clone()],
                            reason: format!(
                                "Used in JOIN condition ({} executions)",
                                stats.execution_count
                            ),
                            priority: self.calculate_priority(benefit, cost),
                            estimated_benefit: benefit,
                            estimated_cost: cost,
                        });
                    }
                }
            }

            // Check ORDER BY clauses
            if !query_pattern.order_by.is_empty() {
                let columns: Vec<_> = query_pattern.order_by.iter()
                    .map(|o| o.column.clone())
                    .collect();

                if !self.has_composite_index(&query_pattern.table, &columns) {
                    let benefit = self.estimate_benefit(stats);
                    let cost = self.estimate_composite_index_cost(&columns);

                    if benefit > cost * self.config.benefit_threshold {
                        recommendations.push(IndexRecommendation {
                            recommendation_type: RecommendationType::CreateIndex,
                            table: query_pattern.table.clone(),
                            columns,
                            reason: format!(
                                "Used in ORDER BY clause ({} executions)",
                                stats.execution_count
                            ),
                            priority: self.calculate_priority(benefit, cost),
                            estimated_benefit: benefit,
                            estimated_cost: cost,
                        });
                    }
                }
            }
        }

        Ok(recommendations)
    }

    // Detect unused indexes
    fn detect_unused_indexes(&self) -> Result<Vec<IndexRecommendation>> {
        let mut recommendations = Vec::new();

        for index in &self.existing_indexes {
            let usage_count = self.workload.get_index_usage_count(&index.name);

            if usage_count == 0 && index.age_days > self.config.unused_index_age_threshold {
                recommendations.push(IndexRecommendation {
                    recommendation_type: RecommendationType::DropIndex,
                    table: index.table.clone(),
                    columns: index.columns.clone(),
                    reason: format!(
                        "Index '{}' has not been used in {} days",
                        index.name, index.age_days
                    ),
                    priority: Priority::Medium,
                    estimated_benefit: index.maintenance_cost,
                    estimated_cost: 0.0,
                });
            }
        }

        Ok(recommendations)
    }

    // Suggest index consolidation
    fn suggest_consolidation(&self) -> Result<Vec<IndexRecommendation>> {
        let mut recommendations = Vec::new();

        // Group indexes by table
        let mut indexes_by_table: HashMap<String, Vec<&IndexMetadata>> = HashMap::new();
        for index in &self.existing_indexes {
            indexes_by_table
                .entry(index.table.clone())
                .or_insert_with(Vec::new)
                .push(index);
        }

        for (table, indexes) in indexes_by_table {
            // Look for indexes that could be consolidated
            for i in 0..indexes.len() {
                for j in (i + 1)..indexes.len() {
                    if self.can_consolidate(indexes[i], indexes[j]) {
                        let combined_columns = self.combine_columns(
                            &indexes[i].columns,
                            &indexes[j].columns,
                        );

                        recommendations.push(IndexRecommendation {
                            recommendation_type: RecommendationType::ConsolidateIndexes,
                            table: table.clone(),
                            columns: combined_columns,
                            reason: format!(
                                "Consolidate indexes '{}' and '{}' into single composite index",
                                indexes[i].name, indexes[j].name
                            ),
                            priority: Priority::Low,
                            estimated_benefit: indexes[i].maintenance_cost
                                + indexes[j].maintenance_cost,
                            estimated_cost: self.estimate_composite_index_cost(
                                &indexes[i].columns,
                            ),
                        });
                    }
                }
            }
        }

        Ok(recommendations)
    }

    // Identify redundant indexes
    fn identify_redundant_indexes(&self) -> Result<Vec<IndexRecommendation>> {
        let mut recommendations = Vec::new();

        for i in 0..self.existing_indexes.len() {
            for j in (i + 1)..self.existing_indexes.len() {
                let idx1 = &self.existing_indexes[i];
                let idx2 = &self.existing_indexes[j];

                if idx1.table == idx2.table {
                    // Check if one index is a prefix of another
                    if self.is_prefix_index(idx1, idx2) {
                        recommendations.push(IndexRecommendation {
                            recommendation_type: RecommendationType::DropIndex,
                            table: idx1.table.clone(),
                            columns: idx1.columns.clone(),
                            reason: format!(
                                "Index '{}' is redundant - covered by '{}'",
                                idx1.name, idx2.name
                            ),
                            priority: Priority::Medium,
                            estimated_benefit: idx1.maintenance_cost,
                            estimated_cost: 0.0,
                        });
                    } else if self.is_prefix_index(idx2, idx1) {
                        recommendations.push(IndexRecommendation {
                            recommendation_type: RecommendationType::DropIndex,
                            table: idx2.table.clone(),
                            columns: idx2.columns.clone(),
                            reason: format!(
                                "Index '{}' is redundant - covered by '{}'",
                                idx2.name, idx1.name
                            ),
                            priority: Priority::Medium,
                            estimated_benefit: idx2.maintenance_cost,
                            estimated_cost: 0.0,
                        });
                    }
                }
            }
        }

        Ok(recommendations)
    }

    // Check if an index exists on a column
    fn has_index_on_column(&self, column: &str) -> bool {
        self.existing_indexes
            .iter()
            .any(|idx| idx.columns.contains(&column.to_string()))
    }

    // Check if a composite index exists
    fn has_composite_index(&self, table: &str, columns: &[String]) -> bool {
        self.existing_indexes.iter().any(|idx| {
            idx.table == table && idx.columns == columns
        })
    }

    // Check if indexes can be consolidated
    fn can_consolidate(&self, idx1: &IndexMetadata, idx2: &IndexMetadata) -> bool {
        if idx1.table != idx2.table {
            return false;
        }

        // Check if columns overlap significantly
        let set1: HashSet<_> = idx1.columns.iter().collect();
        let set2: HashSet<_> = idx2.columns.iter().collect();
        let intersection: HashSet<_> = set1.intersection(&set2).collect();

        intersection.len() > 0
    }

    // Combine columns from two indexes
    fn combine_columns(&self, cols1: &[String], cols2: &[String]) -> Vec<String> {
        let mut combined = cols1.to_vec();
        for col in cols2 {
            if !combined.contains(col) {
                combined.push(col.clone());
            }
        }
        combined
    }

    // Check if one index is a prefix of another
    fn is_prefix_index(&self, short: &IndexMetadata, long: &IndexMetadata) -> bool {
        if short.columns.len() >= long.columns.len() {
            return false;
        }

        for (i, col) in short.columns.iter().enumerate() {
            if long.columns.get(i) != Some(col) {
                return false;
            }
        }

        true
    }

    // Estimate benefit of creating an index
    fn estimate_benefit(&self, stats: &QueryStats) -> f64 {
        // Simple model: benefit = frequency * avg_time_saved
        let frequency = stats.execution_count as f64;
        let time_saved = stats.avg_execution_time_ms * 0.7; // Assume 70% improvement
        frequency * time_saved
    }

    // Estimate cost of creating an index
    fn estimate_index_cost(&self, column: &str) -> f64 {
        // Simplified cost model
        100.0 // Base cost for single-column index
    }

    // Estimate cost of creating a composite index
    fn estimate_composite_index_cost(&self, columns: &[String]) -> f64 {
        100.0 + (columns.len() as f64 - 1.0) * 50.0
    }

    // Calculate priority based on benefit and cost
    fn calculate_priority(&self, benefit: f64, cost: f64) -> Priority {
        let ratio = benefit / cost.max(1.0);

        if ratio > 10.0 {
            Priority::High
        } else if ratio > 3.0 {
            Priority::Medium
        } else {
            Priority::Low
        }
    }
}

// Workload tracker
struct WorkloadTracker {
    query_patterns: HashMap<QueryPattern, QueryStats>,
    index_usage: HashMap<String, usize>,
}

impl WorkloadTracker {
    fn new() -> Self {
        Self {
            query_patterns: HashMap::new(),
            index_usage: HashMap::new(),
        }
    }

    fn record_query(&mut self, query: &Query) {
        let pattern = QueryPattern::from_query(query);
        let stats = self.query_patterns.entry(pattern).or_insert_with(QueryStats::new);

        stats.execution_count += 1;
        stats.total_execution_time_ms += query.execution_time_ms;
        stats.avg_execution_time_ms = stats.total_execution_time_ms / stats.execution_count as f64;

        // Record index usage
        for index in &query.indexes_used {
            *self.index_usage.entry(index.clone()).or_insert(0) += 1;
        }
    }

    fn get_index_usage_count(&self, index_name: &str) -> usize {
        *self.index_usage.get(index_name).unwrap_or(&0)
    }
}

// Query pattern (normalized query for analysis)
#[derive(Debug, Clone, Hash, Eq, PartialEq)]
struct QueryPattern {
    table: String,
    where_conditions: Vec<Condition>,
    joins: Vec<JoinInfo>,
    order_by: Vec<OrderByInfo>,
}

impl QueryPattern {
    fn from_query(query: &Query) -> Self {
        Self {
            table: query.table.clone(),
            where_conditions: query.where_conditions.clone(),
            joins: query.joins.clone(),
            order_by: query.order_by.clone(),
        }
    }
}

// Query statistics
#[derive(Debug, Clone)]
struct QueryStats {
    execution_count: usize,
    total_execution_time_ms: f64,
    avg_execution_time_ms: f64,
}

impl QueryStats {
    fn new() -> Self {
        Self {
            execution_count: 0,
            total_execution_time_ms: 0.0,
            avg_execution_time_ms: 0.0,
        }
    }
}

// Query information
#[derive(Debug, Clone)]
pub struct Query {
    pub table: String,
    pub where_conditions: Vec<Condition>,
    pub joins: Vec<JoinInfo>,
    pub order_by: Vec<OrderByInfo>,
    pub execution_time_ms: f64,
    pub indexes_used: Vec<String>,
}

// WHERE condition
#[derive(Debug, Clone, Hash, Eq, PartialEq)]
pub struct Condition {
    pub column: String,
    pub operator: String,
}

// JOIN information
#[derive(Debug, Clone, Hash, Eq, PartialEq)]
pub struct JoinInfo {
    pub table: String,
    pub join_column: String,
}

// ORDER BY information
#[derive(Debug, Clone, Hash, Eq, PartialEq)]
pub struct OrderByInfo {
    pub column: String,
    pub direction: String,
}

// Index metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexMetadata {
    pub name: String,
    pub table: String,
    pub columns: Vec<String>,
    pub index_type: String,
    pub age_days: usize,
    pub size_bytes: usize,
    pub maintenance_cost: f64,
}

// Index recommendation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexRecommendation {
    pub recommendation_type: RecommendationType,
    pub table: String,
    pub columns: Vec<String>,
    pub reason: String,
    pub priority: Priority,
    pub estimated_benefit: f64,
    pub estimated_cost: f64,
}

// Recommendation type
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum RecommendationType {
    CreateIndex,
    DropIndex,
    ConsolidateIndexes,
    RebuildIndex,
}

// Priority level
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub enum Priority {
    Low,
    Medium,
    High,
}

// Advisor configuration
#[derive(Debug, Clone)]
pub struct AdvisorConfig {
    pub min_query_count: usize,
    pub unused_index_age_threshold: usize,
    pub benefit_threshold: f64,
}

impl Default for AdvisorConfig {
    fn default() -> Self {
        Self {
            min_query_count: 10,
            unused_index_age_threshold: 30,
            benefit_threshold: 2.0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_advisor_creation() {
        let advisor = IndexAdvisor::new(AdvisorConfig::default());
        assert_eq!(advisor.existing_indexes.len(), 0);
    }

    #[test]
    fn test_detect_missing_index() {
        let mut advisor = IndexAdvisor::new(AdvisorConfig {
            min_query_count: 1,
            ..Default::default()
        });

        // Record a query
        let query = Query {
            table: "users".to_string(),
            where_conditions: vec![Condition {
                column: "email".to_string(),
                operator: "=".to_string(),
            }],
            joins: vec![],
            order_by: vec![],
            execution_time_ms: 100.0,
            indexes_used: vec![],
        };

        for _ in 0..20 {
            advisor.record_query(&query);
        }

        let recommendations = advisor.analyze().unwrap();
        assert!(!recommendations.is_empty());
    }

    #[test]
    fn test_detect_unused_index() {
        let mut advisor = IndexAdvisor::new(AdvisorConfig {
            unused_index_age_threshold: 10,
            ..Default::default()
        });

        // Register an old, unused index
        advisor.register_index(IndexMetadata {
            name: "idx_old".to_string(),
            table: "users".to_string(),
            columns: vec!["old_column".to_string()],
            index_type: "btree".to_string(),
            age_days: 60,
            size_bytes: 1024 * 1024,
            maintenance_cost: 10.0,
        });

        let recommendations = advisor.analyze().unwrap();
        let drop_recs: Vec<_> = recommendations
            .iter()
            .filter(|r| r.recommendation_type == RecommendationType::DropIndex)
            .collect();

        assert!(!drop_recs.is_empty());
    }

    #[test]
    fn test_redundant_index_detection() {
        let mut advisor = IndexAdvisor::new(AdvisorConfig::default());

        // Register two indexes where one is a prefix of the other
        advisor.register_index(IndexMetadata {
            name: "idx_single".to_string(),
            table: "users".to_string(),
            columns: vec!["email".to_string()],
            index_type: "btree".to_string(),
            age_days: 10,
            size_bytes: 1024,
            maintenance_cost: 5.0,
        });

        advisor.register_index(IndexMetadata {
            name: "idx_composite".to_string(),
            table: "users".to_string(),
            columns: vec!["email".to_string(), "name".to_string()],
            index_type: "btree".to_string(),
            age_days: 10,
            size_bytes: 2048,
            maintenance_cost: 8.0,
        });

        let recommendations = advisor.analyze().unwrap();
        let redundant: Vec<_> = recommendations
            .iter()
            .filter(|r| r.reason.contains("redundant"))
            .collect();

        assert!(!redundant.is_empty());
    }
}
