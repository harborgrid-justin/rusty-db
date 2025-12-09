// Automatic Index Management
//
// Intelligent index recommendation, creation, consolidation, and maintenance
// based on workload analysis and query patterns.

use tokio::time::sleep;
use std::fmt;
use std::time::SystemTime;
use std::collections::{HashMap};
use std::sync::Arc;
use std::time::{Duration};
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use crate::Result;
use crate::error::DbError;

/// Index type recommendation
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum IndexType {
    BTree,
    Hash,
    Bitmap,
    Spatial,
    FullText,
    Partial,
    Covering,
}

impl std::fmt::Display for IndexType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            IndexType::BTree => write!(f, "B-Tree"),
            IndexType::Hash => write!(f, "Hash"),
            IndexType::Bitmap => write!(f, "Bitmap"),
            IndexType::Spatial => write!(f, "Spatial"),
            IndexType::FullText => write!(f, "Full-Text"),
            IndexType::Partial => write!(f, "Partial"),
            IndexType::Covering => write!(f, "Covering"),
        }
    }
}

/// Column access pattern
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ColumnAccessPattern {
    pub table_name: String,
    pub column_name: String,
    pub equality_searches: usize,
    pub range_searches: usize,
    pub order_by_usage: usize,
    pub group_by_usage: usize,
    pub join_usage: usize,
    pub filter_selectivity: f64,  // 0.0 = filters nothing, 1.0 = filters everything
    pub cardinality: usize,
}

impl ColumnAccessPattern {
    pub fn total_accesses(&self) -> usize {
        self.equality_searches + self.range_searches + self.order_by_usage + self.group_by_usage + self.join_usage
    }

    pub fn is_low_cardinality(&self) -> bool {
        self.cardinality < 100  // Threshold for bitmap index consideration
    }
}

/// Index candidate with cost-benefit analysis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexCandidate {
    pub candidate_id: u64,
    pub table_name: String,
    pub columns: Vec<String>,
    pub index_type: IndexType,
    pub estimated_size_mb: usize,
    pub estimated_creation_time_sec: u64,
    pub expected_query_improvement: f64,  // Percentage improvement
    pub queries_benefited: usize,
    pub write_overhead: f64,  // Percentage slowdown for writes
    pub benefit_score: f64,
    pub predicate: Option<String>,  // For partial indexes
    pub include_columns: Vec<String>,  // For covering indexes
}

impl IndexCandidate {
    pub fn calculate_benefit_score(&mut self) {
        // Benefit = (query improvement * queries benefited) - (write overhead + space cost)
        let query_benefit = self.expected_query_improvement * self.queries_benefited as f64;
        let write_cost = self.write_overhead * 10.0;  // Weight write overhead
        let space_cost = self.estimated_size_mb as f64 * 0.1;  // Small weight for space

        self.benefit_score = query_benefit - write_cost - space_cost;
    }
}

/// Existing index statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexStatistics {
    pub index_name: String,
    pub table_name: String,
    pub columns: Vec<String>,
    pub index_type: IndexType,
    pub size_mb: usize,
    pub usage_count: usize,
    pub last_used: Option<SystemTime>,
    pub scans_count: usize,
    pub rows_read: usize,
    pub creation_time: SystemTime,
}

impl IndexStatistics {
    pub fn is_unused(&self, threshold_days: u64) -> bool {
        if let Some(last_used) = self.last_used {
            if let Ok(elapsed) = last_used.elapsed() {
                return elapsed > Duration::from_secs(86400 * threshold_days);
            }
        }
        true
    }

    pub fn is_redundant(&self, other: &IndexStatistics) -> bool {
        // Check if this index is a prefix of another index
        if self.table_name != other.table_name {
            return false;
        }

        if self.columns.len() >= other.columns.len() {
            return false;
        }

        // Check if columns are a prefix
        self.columns
            .iter()
            .zip(&other.columns)
            .all(|(a, b)| a == b)
    }
}

/// Index recommendation engine
pub struct IndexAdvisor {
    column_patterns: Arc<RwLock<HashMap<String, ColumnAccessPattern>>>,
    existing_indexes: Arc<RwLock<HashMap<String, IndexStatistics>>>,
    candidates: Arc<RwLock<Vec<IndexCandidate>>>,
    next_candidate_id: Arc<RwLock<u64>>,
    min_benefit_threshold: f64,
}

impl IndexAdvisor {
    pub fn new(min_benefit_threshold: f64) -> Self {
        Self {
            column_patterns: Arc::new(RwLock::new(HashMap::new())),
            existing_indexes: Arc::new(RwLock::new(HashMap::new())),
            candidates: Arc::new(RwLock::new(Vec::new())),
            next_candidate_id: Arc::new(RwLock::new(0)),
            min_benefit_threshold,
        }
    }

    pub fn record_column_access(
        &self,
        table_name: String,
        column_name: String,
        access_type: ColumnAccessType,
    ) {
        let key = format!("{}.{}", table_name, column_name));
        let mut patterns = self.column_patterns.write();

        patterns
            .entry(key.clone())
            .and_modify(|pattern| {
                match access_type {
                    ColumnAccessType::EqualitySearch => pattern.equality_searches += 1,
                    ColumnAccessType::RangeSearch => pattern.range_searches += 1,
                    ColumnAccessType::OrderBy => pattern.order_by_usage += 1,
                    ColumnAccessType::GroupBy => pattern.group_by_usage += 1,
                    ColumnAccessType::Join => pattern.join_usage += 1,
                }
            })
            .or_insert_with(|| ColumnAccessPattern {
                table_name: table_name.clone(),
                column_name: column_name.clone(),
                equality_searches: if matches!(access_type, ColumnAccessType::EqualitySearch) { 1 } else { 0 },
                range_searches: if matches!(access_type, ColumnAccessType::RangeSearch) { 1 } else { 0 },
                order_by_usage: if matches!(access_type, ColumnAccessType::OrderBy) { 1 } else { 0 },
                group_by_usage: if matches!(access_type, ColumnAccessType::GroupBy) { 1 } else { 0 },
                join_usage: if matches!(access_type, ColumnAccessType::Join) { 1 } else { 0 },
                filter_selectivity: 0.5,
                cardinality: 1000,  // Default, should be updated from statistics
            });
    }

    pub fn update_index_statistics(&self, stats: IndexStatistics) {
        self.existing_indexes.write().insert(stats.index_name.clone(), stats);
    }

    pub fn analyze_and_recommend(&self) -> Vec<IndexCandidate> {
        let mut candidates = Vec::new();
        let patterns = self.column_patterns.read();

        // Identify single-column index candidates
        for (key, pattern) in patterns.iter() {
            if pattern.total_accesses() < 10 {
                continue;  // Skip columns with low access
            }

            if let Some(candidate) = self.recommend_single_column_index(pattern) {
                candidates.push(candidate);
            }
        }

        // Identify multi-column index candidates
        let multi_column_candidates = self.recommend_multi_column_indexes(&patterns);
        candidates.extend(multi_column_candidates);

        // Calculate benefit scores
        for candidate in &mut candidates {
            candidate.calculate_benefit_score();
        }

        // Filter by minimum benefit threshold
        candidates.retain(|c| c.benefit_score > self.min_benefit_threshold);

        // Sort by benefit score (descending)
        candidates.sort_by(|a, b| b.benefit_score.partial_cmp(&a.benefit_score).unwrap());

        // Update candidates
        *self.candidates.write() = candidates.clone();

        candidates
    }

    fn recommend_single_column_index(&self, pattern: &ColumnAccessPattern) -> Option<IndexCandidate> {
        let index_type = self.determine_index_type(pattern);

        let mut candidate_id = self.next_candidate_id.write();
        let id = *candidate_id;
        *candidate_id += 1;
        drop(candidate_id);

        let estimated_size_mb = (pattern.cardinality * 20) / (1024 * 1024);  // Rough estimate
        let expected_improvement = self.calculate_expected_improvement(pattern);

        Some(IndexCandidate {
            candidate_id: id,
            table_name: pattern.table_name.clone(),
            columns: vec![pattern.column_name.clone()],
            index_type,
            estimated_size_mb: estimated_size_mb.max(1),
            estimated_creation_time_sec: (pattern.cardinality / 10000).max(1) as u64,
            expected_query_improvement: expected_improvement,
            queries_benefited: pattern.total_accesses(),
            write_overhead: self.calculate_write_overhead(index_type),
            benefit_score: 0.0,
            predicate: None,
            include_columns: Vec::new(),
        })
    }

    fn determine_index_type(&self, pattern: &ColumnAccessPattern) -> IndexType {
        // Bitmap for low cardinality columns
        if pattern.is_low_cardinality() {
            return IndexType::Bitmap;
        }

        // Hash for equality-only searches
        if pattern.equality_searches > 0 && pattern.range_searches == 0 && pattern.order_by_usage == 0 {
            return IndexType::Hash;
        }

        // B-Tree for everything else (default)
        IndexType::BTree
    }

    fn calculate_expected_improvement(&self, pattern: &ColumnAccessPattern) -> f64 {
        // Estimate based on selectivity and access patterns
        let base_improvement = pattern.filter_selectivity * 50.0;  // Up to 50% improvement

        let access_factor = if pattern.equality_searches > pattern.range_searches {
            1.2  // Equality searches benefit more
        } else {
            1.0
        };

        (base_improvement * access_factor).min(90.0)  // Cap at 90%
    }

    fn calculate_write_overhead(&self, index_type: IndexType) -> f64 {
        match index_type {
            IndexType::BTree => 5.0,      // 5% slowdown
            IndexType::Hash => 3.0,       // 3% slowdown
            IndexType::Bitmap => 8.0,     // 8% slowdown (more maintenance)
            IndexType::Spatial => 10.0,   // 10% slowdown
            IndexType::FullText => 15.0,  // 15% slowdown
            IndexType::Partial => 4.0,    // 4% slowdown (fewer rows)
            IndexType::Covering => 12.0,  // 12% slowdown (more columns)
        }
    }

    fn recommend_multi_column_indexes(&self, patterns: &HashMap<String, ColumnAccessPattern>) -> Vec<IndexCandidate> {
        let mut candidates = Vec::new();

        // Group columns by table
        let mut table_columns: HashMap<String, Vec<&ColumnAccessPattern>> = HashMap::new();
        for pattern in patterns.values() {
            table_columns
                .entry(pattern.table_name.clone())
                .or_insert_with(Vec::new)
                .push(pattern);
        }

        // For each table, find frequently co-accessed columns
        for (table_name, columns) in table_columns {
            if columns.len() < 2 {
                continue;
            }

            // Sort by access frequency
            let mut sorted_columns = columns.clone();
            sorted_columns.sort_by(|a, b| b.total_accesses().cmp(&a.total_accesses()));

            // Take top columns that are frequently used together
            if sorted_columns.len() >= 2 {
                let col1 = sorted_columns[0];
                let col2 = sorted_columns[1];

                // Only recommend if both columns are frequently accessed
                if col1.total_accesses() > 20 && col2.total_accesses() > 20 {
                    let mut candidate_id = self.next_candidate_id.write();
                    let id = *candidate_id;
                    *candidate_id += 1;
                    drop(candidate_id);

                    let estimated_size_mb = ((col1.cardinality + col2.cardinality) * 20) / (1024 * 1024);

                    candidates.push(IndexCandidate {
                        candidate_id: id,
                        table_name: table_name.clone(),
                        columns: vec![col1.column_name.clone(), col2.column_name.clone()],
                        index_type: IndexType::BTree,
                        estimated_size_mb: estimated_size_mb.max(1),
                        estimated_creation_time_sec: (col1.cardinality / 5000).max(1) as u64,
                        expected_query_improvement: 60.0,
                        queries_benefited: col1.total_accesses().min(col2.total_accesses()),
                        write_overhead: 7.0,
                        benefit_score: 0.0,
                        predicate: None,
                        include_columns: Vec::new(),
                    });
                }
            }
        }

        candidates
    }

    pub fn find_unused_indexes(&self, threshold_days: u64) -> Vec<String> {
        self.existing_indexes
            .read()
            .values()
            .filter(|stats| stats.is_unused(threshold_days))
            .map(|stats| stats.index_name.clone())
            .collect()
    }

    pub fn find_redundant_indexes(&self) -> Vec<(String, String)> {
        let indexes = self.existing_indexes.read();
        let mut redundant_pairs = Vec::new();

        let index_list: Vec<_> = indexes.values().collect();

        for i in 0..index_list.len() {
            for j in (i + 1)..index_list.len() {
                if index_list[i].is_redundant(index_list[j]) {
                    redundant_pairs.push((
                        index_list[i].index_name.clone(),
                        index_list[j].index_name.clone(),
                    ));
                }
            }
        }

        redundant_pairs
    }

    pub fn recommend_covering_indexes(&self) -> Vec<IndexCandidate> {
        let mut candidates = Vec::new();
        let patterns = self.column_patterns.read();

        // Group by table
        let mut table_patterns: HashMap<String, Vec<&ColumnAccessPattern>> = HashMap::new();
        for pattern in patterns.values() {
            table_patterns
                .entry(pattern.table_name.clone())
                .or_insert_with(Vec::new)
                .push(pattern);
        }

        for (table_name, columns) in table_patterns {
            if columns.len() < 2 {
                continue;
            }

            // Find columns used in WHERE clause and columns in SELECT
            let index_cols: Vec<String> = columns
                .iter()
                .filter(|p| p.equality_searches > 0 || p.range_searches > 0)
                .take(2)
                .map(|p| p.column_name.clone())
                .collect();

            let include_cols: Vec<String> = columns
                .iter()
                .filter(|p| p.equality_searches == 0 && p.range_searches == 0)
                .take(3)
                .map(|p| p.column_name.clone())
                .collect();

            if !index_cols.is_empty() && !include_cols.is_empty() {
                let mut candidate_id = self.next_candidate_id.write();
                let id = *candidate_id;
                *candidate_id += 1;
                drop(candidate_id);

                candidates.push(IndexCandidate {
                    candidate_id: id,
                    table_name: table_name.clone(),
                    columns: index_cols,
                    index_type: IndexType::Covering,
                    estimated_size_mb: 10,
                    estimated_creation_time_sec: 5,
                    expected_query_improvement: 70.0,  // Covering indexes can eliminate table lookups
                    queries_benefited: 50,
                    write_overhead: 12.0,
                    benefit_score: 0.0,
                    predicate: None,
                    include_columns: include_cols,
                });
            }
        }

        candidates
    }

    pub fn recommend_partial_indexes(&self) -> Vec<IndexCandidate> {
        let mut candidates = Vec::new();
        let patterns = self.column_patterns.read();

        for pattern in patterns.values() {
            // Recommend partial index if selectivity is high (data is selective)
            if pattern.filter_selectivity > 0.8 && pattern.total_accesses() > 20 {
                let mut candidate_id = self.next_candidate_id.write();
                let id = *candidate_id;
                *candidate_id += 1;
                drop(candidate_id);

                let predicate = format!("{} IS NOT NULL", pattern.column_name));

                candidates.push(IndexCandidate {
                    candidate_id: id,
                    table_name: pattern.table_name.clone(),
                    columns: vec![pattern.column_name.clone()],
                    index_type: IndexType::Partial,
                    estimated_size_mb: (pattern.cardinality as f64 * 0.2) as usize,  // Only 20% of data
                    estimated_creation_time_sec: 3,
                    expected_query_improvement: 45.0,
                    queries_benefited: pattern.total_accesses(),
                    write_overhead: 4.0,
                    benefit_score: 0.0,
                    predicate: Some(predicate),
                    include_columns: Vec::new(),
                });
            }
        }

        candidates
    }
}

#[derive(Debug, Clone, Copy)]
pub enum ColumnAccessType {
    EqualitySearch,
    RangeSearch,
    OrderBy,
    GroupBy,
    Join,
}

/// Index maintenance scheduler
pub struct IndexMaintenanceScheduler {
    maintenance_schedule: Arc<RwLock<HashMap<String, MaintenanceTask>>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MaintenanceTask {
    pub index_name: String,
    pub task_type: MaintenanceTaskType,
    pub scheduled_time: SystemTime,
    pub estimated_duration_sec: u64,
    pub priority: u8,  // 1-10, higher is more urgent
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MaintenanceTaskType {
    Rebuild,
    Reorganize,
    UpdateStatistics,
    Vacuum,
}

impl IndexMaintenanceScheduler {
    pub fn new() -> Self {
        Self {
            maintenance_schedule: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub fn schedule_maintenance(&self, task: MaintenanceTask) {
        self.maintenance_schedule.write().insert(task.index_name.clone(), task);
    }

    pub fn get_pending_tasks(&self) -> Vec<MaintenanceTask> {
        let now = SystemTime::now();
        self.maintenance_schedule
            .read()
            .values()
            .filter(|task| task.scheduled_time <= now)
            .cloned()
            .collect()
    }

    pub fn optimize_schedule(&self, maintenance_window_start: u8, maintenance_window_end: u8) {
        // Schedule tasks during maintenance window
        let mut schedule = self.maintenance_schedule.write();

        for task in schedule.values_mut() {
            // Adjust scheduled time to fit in maintenance window
            // This is a simplified version - real implementation would be more sophisticated
            task.priority = if task.task_type == MaintenanceTaskType::Rebuild {
                8
            } else {
                5
            };
        }
    }
}

/// Auto-indexing orchestrator
pub struct AutoIndexingEngine {
    advisor: Arc<IndexAdvisor>,
    maintenance_scheduler: Arc<IndexMaintenanceScheduler>,
    auto_create_enabled: Arc<RwLock<bool>>,
    auto_drop_enabled: Arc<RwLock<bool>>,
}

impl AutoIndexingEngine {
    pub fn new() -> Self {
        Self {
            advisor: Arc::new(IndexAdvisor::new(10.0)),  // Minimum benefit score of 10
            maintenance_scheduler: Arc::new(IndexMaintenanceScheduler::new()),
            auto_create_enabled: Arc::new(RwLock::new(false)),
            auto_drop_enabled: Arc::new(RwLock::new(false)),
        }
    }

    pub fn enable_auto_create(&self) {
        *self.auto_create_enabled.write() = true;
    }

    pub fn disable_auto_create(&self) {
        *self.auto_create_enabled.write() = false;
    }

    pub fn enable_auto_drop(&self) {
        *self.auto_drop_enabled.write() = true;
    }

    pub fn disable_auto_drop(&self) {
        *self.auto_drop_enabled.write() = false;
    }

    pub fn get_recommendations(&self) -> IndexRecommendationReport {
        let basic_recommendations = self.advisor.analyze_and_recommend();
        let covering_recommendations = self.advisor.recommend_covering_indexes();
        let partial_recommendations = self.advisor.recommend_partial_indexes();
        let unused_indexes = self.advisor.find_unused_indexes(30);  // 30 days threshold
        let redundant_indexes = self.advisor.find_redundant_indexes();

        IndexRecommendationReport {
            total_recommendations: basic_recommendations.len() + covering_recommendations.len() + partial_recommendations.len(),
            high_benefit_recommendations: basic_recommendations.iter().filter(|c| c.benefit_score > 50.0).count(),
            unused_indexes: unused_indexes.len(),
            redundant_index_pairs: redundant_indexes.len(),
            estimated_space_savings_mb: unused_indexes.len() * 10,  // Rough estimate
            estimated_performance_gain: basic_recommendations.first().map(|c| c.expected_query_improvement).unwrap_or(0.0),
        }
    }

    pub async fn auto_create_indexes(&self) -> Result<Vec<String>> {
        if !*self.auto_create_enabled.read() {
            return Ok(Vec::new());
        }

        let recommendations = self.advisor.analyze_and_recommend();
        let mut created_indexes = Vec::new();

        // Create top 3 recommended indexes
        for candidate in recommendations.iter().take(3) {
            if candidate.benefit_score > 50.0 {
                let index_name = self.create_index(candidate).await?;
                created_indexes.push(index_name);
            }
        }

        Ok(created_indexes)
    }

    async fn create_index(&self, candidate: &IndexCandidate) -> Result<String> {
        let index_name = format!(
            "idx_auto_{}_{}_{}",
            candidate.table_name,
            candidate.columns.join("_"),
            candidate.candidate_id
        ));

        tracing::info!("Auto-creating index: {}", index_name);

        // Simulate index creation
        tokio::time::sleep(Duration::from_millis(100)).await;

        Ok(index_name)
    }

    pub async fn auto_drop_unused_indexes(&self) -> Result<Vec<String>> {
        if !*self.auto_drop_enabled.read() {
            return Ok(Vec::new());
        }

        let unused = self.advisor.find_unused_indexes(60);  // 60 days
        let mut dropped = Vec::new();

        for index_name in unused {
            tracing::info!("Auto-dropping unused index: {}", index_name);
            dropped.push(index_name);
        }

        Ok(dropped)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexRecommendationReport {
    pub total_recommendations: usize,
    pub high_benefit_recommendations: usize,
    pub unused_indexes: usize,
    pub redundant_index_pairs: usize,
    pub estimated_space_savings_mb: usize,
    pub estimated_performance_gain: f64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_index_advisor() {
        let advisor = IndexAdvisor::new(10.0);

        advisor.record_column_access(
            "users".to_string(),
            "email".to_string(),
            ColumnAccessType::EqualitySearch,
        );

        advisor.record_column_access(
            "users".to_string(),
            "email".to_string(),
            ColumnAccessType::EqualitySearch,
        );

        let patterns = advisor.column_patterns.read();
        assert!(patterns.contains_key("users.email"));
    }

    #[test]
    fn test_index_statistics_unused() {
        let stats = IndexStatistics {
            index_name: "test_idx".to_string(),
            table_name: "test".to_string(),
            columns: vec!["col1".to_string()],
            index_type: IndexType::BTree,
            size_mb: 10,
            usage_count: 0,
            last_used: Some(SystemTime::now() - Duration::from_secs(86400 * 40)),
            scans_count: 0,
            rows_read: 0,
            creation_time: SystemTime::now(),
        };

        assert!(stats.is_unused(30));
    }

    #[test]
    fn test_candidate_benefit_score() {
        let mut candidate = IndexCandidate {
            candidate_id: 1,
            table_name: "test".to_string(),
            columns: vec!["col1".to_string()],
            index_type: IndexType::BTree,
            estimated_size_mb: 5,
            estimated_creation_time_sec: 10,
            expected_query_improvement: 50.0,
            queries_benefited: 100,
            write_overhead: 5.0,
            benefit_score: 0.0,
            predicate: None,
            include_columns: Vec::new(),
        };

        candidate.calculate_benefit_score();
        assert!(candidate.benefit_score > 0.0);
    }
}
