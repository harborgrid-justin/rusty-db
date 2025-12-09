/// Advanced Query Optimization Techniques
///
/// This module provides enterprise-grade query optimization:
/// - Query plan caching for repeated queries
/// - Adaptive query optimization based on runtime statistics
/// - Materialized view automatic rewrite
/// - Enhanced cost model with statistics
/// - Join order optimization with dynamic programming
/// - Index selection optimization

use std::time::SystemTime;
use crate::error::DbError;
use crate::execution::planner::PlanNode;
use parking_lot::RwLock;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;


/// Query plan cache
pub struct PlanCache {
    cache: Arc<RwLock<HashMap<String, CachedPlan>>>,
    max_size: usize,
    hit_count: Arc<RwLock<u64>>,
    miss_count: Arc<RwLock<u64>>,
}

impl PlanCache {
    pub fn new(max_size: usize) -> Self {
        Self {
            cache: Arc::new(RwLock::new(HashMap::new())),
            max_size,
            hit_count: Arc::new(RwLock::new(0)),
            miss_count: Arc::new(RwLock::new(0)),
        }
    }
    
    /// Get cached plan for query
    pub fn get(&self, query_hash: &str) -> Option<PlanNode> {
        let cache = self.cache.read();
        if let Some(cached) = cache.get(query_hash) {
            // Check if plan is still valid
            if !cached.is_expired() {
                *self.hit_count.write() += 1;
                return Some(cached.plan.clone());
            }
        }
        *self.miss_count.write() += 1;
        None
    }
    
    /// Cache a query plan
    pub fn put(&self, query_hash: String, plan: PlanNode, ttl: Duration) {
        let mut cache = self.cache.write();
        
        // Evict if at capacity
        if cache.len() >= self.max_size {
            // Simple LRU: remove oldest entry
            if let Some(oldest_key) = cache.keys().next().cloned() {
                cache.remove(&oldest_key);
            }
        }
        
        cache.insert(
            query_hash,
            CachedPlan {
                plan,
                created_at: SystemTime::now(),
                ttl,
                access_count: 0,
            },
        );
    }
    
    /// Get cache statistics
    pub fn stats(&self) -> CacheStats {
        let hits = *self.hit_count.read();
        let misses = *self.miss_count.read();
        let total = hits + misses;
        let hit_rate = if total > 0 {
            hits as f64 / total as f64
        } else {
            0.0
        };
        
        CacheStats {
            hits,
            misses,
            hit_rate,
            size: self.cache.read().len(),
        }
    }
    
    /// Clear cache
    pub fn clear(&self) {
        self.cache.write().clear();
    }
}

/// Cached plan entry
#[derive(Debug, Clone)]
struct CachedPlan {
    plan: PlanNode,
    created_at: SystemTime,
    ttl: Duration,
    access_count: u64,
}

impl CachedPlan {
    fn is_expired(&self) -> bool {
        if let Ok(elapsed) = self.created_at.elapsed() {
            elapsed > self.ttl
        } else {
            true
        }
    }
}

/// Cache statistics
#[derive(Debug, Clone)]
pub struct CacheStats {
    pub hits: u64,
    pub misses: u64,
    pub hit_rate: f64,
    pub size: usize,
}

/// Statistics collector for query optimization
pub struct StatisticsCollector {
    /// Table statistics: table name -> stats
    table_stats: Arc<RwLock<HashMap<String, TableStatistics>>>,
    /// Column statistics: (table, column) -> stats
    column_stats: Arc<RwLock<HashMap<(String, String), ColumnStatistics>>>,
}

impl StatisticsCollector {
    pub fn new() -> Self {
        Self {
            table_stats: Arc::new(RwLock::new(HashMap::new())),
            column_stats: Arc::new(RwLock::new(HashMap::new())),
        }
    }
    
    /// Collect statistics for a table
    pub fn collect_table_stats(&self, table_name: String, row_count: u64, size_bytes: u64) {
        let mut stats = self.table_stats.write();
        stats.insert(
            table_name.clone(),
            TableStatistics {
                table_name,
                row_count,
                size_bytes,
                last_updated: SystemTime::now(),
            },
        );
    }
    
    /// Collect statistics for a column
    pub fn collect_column_stats(
        &self,
        table_name: String,
        column_name: String,
        distinct_count: u64,
        null_count: u64,
        min_value: Option<String>,
        max_value: Option<String>,
    ) {
        let mut stats = self.column_stats.write();
        stats.insert(
            (table_name.clone(), column_name.clone()),
            ColumnStatistics {
                table_name,
                column_name,
                distinct_count,
                null_count,
                min_value,
                max_value,
                last_updated: SystemTime::now(),
            },
        );
    }
    
    /// Get table statistics
    pub fn get_table_stats(&self, table_name: &str) -> Option<TableStatistics> {
        self.table_stats.read().get(table_name).cloned()
    }
    
    /// Get column statistics
    pub fn get_column_stats(&self, table_name: &str, column_name: &str) -> Option<ColumnStatistics> {
        self.column_stats
            .read()
            .get(&(table_name.to_string(), column_name.to_string()))
            .cloned()
    }
    
    /// Estimate selectivity of a predicate
    pub fn estimate_selectivity(&self, table: &str, column: &str, predicate: &str) -> f64 {
        if let Some(col_stats) = self.get_column_stats(table, column) {
            if let Some(table_stats) = self.get_table_stats(table) {
                // Simple selectivity estimation
                if predicate.contains('=') {
                    // Equality: 1 / distinct_count
                    return 1.0 / col_stats.distinct_count.max(1) as f64;
                } else if predicate.contains('>') || predicate.contains('<') {
                    // Range: assume 30% selectivity
                    return 0.3;
                }
            }
        }
        
        // Default selectivity
        0.1
    }
}

/// Table statistics
#[derive(Debug, Clone)]
pub struct TableStatistics {
    pub table_name: String,
    pub row_count: u64,
    pub size_bytes: u64,
    pub last_updated: SystemTime,
}

/// Column statistics
#[derive(Debug, Clone)]
pub struct ColumnStatistics {
    pub table_name: String,
    pub column_name: String,
    pub distinct_count: u64,
    pub null_count: u64,
    pub min_value: Option<String>,
    pub max_value: Option<String>,
    pub last_updated: SystemTime,
}

/// Adaptive query optimizer
/// Learns from query execution patterns to improve future plans
pub struct AdaptiveOptimizer {
    /// Query execution history
    execution_history: Arc<RwLock<Vec<ExecutionRecord>>>,
    /// Learned join orders
    join_order_hints: Arc<RwLock<HashMap<String, Vec<String>>>>,
}

impl AdaptiveOptimizer {
    pub fn new() -> Self {
        Self {
            execution_history: Arc::new(RwLock::new(Vec::new())),
            join_order_hints: Arc::new(RwLock::new(HashMap::new())),
        }
    }
    
    /// Record query execution
    pub fn record_execution(
        &self,
        query_hash: String,
        plan: &PlanNode,
        execution_time_ms: u64,
        rows_returned: u64,
    ) {
        let mut history = self.execution_history.write();
        history.push(ExecutionRecord {
            query_hash,
            plan: plan.clone(),
            execution_time_ms,
            rows_returned,
            timestamp: SystemTime::now(),
        });
        
        // Keep history bounded
        if history.len() > 1000 {
            history.remove(0);
        }
    }
    
    /// Get optimal join order based on history
    pub fn suggest_join_order(&self, tables: &[String]) -> Option<Vec<String>> {
        let hints = self.join_order_hints.read();
        let key = tables.join(",");
        hints.get(&key).cloned()
    }
    
    /// Learn join order from execution history
    pub fn learn_join_orders(&self) {
        let history = self.execution_history.read();
        let mut hints = self.join_order_hints.write();
        
        // Analyze execution records to find optimal join orders
        // This is a simplified implementation
        for record in history.iter() {
            if let Some(tables) = Self::extract_tables(&record.plan) {
                let key = tables.join(",");
                hints.entry(key).or_insert(tables);
            }
        }
    }
    
    fn extract_tables(plan: &PlanNode) -> Option<Vec<String>> {
        match plan {
            PlanNode::Join { left, right, .. } => {
                let mut tables = Vec::new();
                if let Some(left_tables) = Self::extract_tables(left) {
                    tables.extend(left_tables);
                }
                if let Some(right_tables) = Self::extract_tables(right) {
                    tables.extend(right_tables);
                }
                if !tables.is_empty() {
                    Some(tables)
                } else {
                    None
                }
            }
            PlanNode::TableScan { table, .. } => Some(vec![table.clone()]),
            _ => None,
        }
    }
}

/// Execution record for adaptive optimization
#[derive(Debug, Clone)]
struct ExecutionRecord {
    query_hash: String,
    plan: PlanNode,
    execution_time_ms: u64,
    rows_returned: u64,
    timestamp: SystemTime,
}

/// Materialized view rewriter
/// Automatically rewrites queries to use materialized views when beneficial
pub struct MaterializedViewRewriter {
    /// Available materialized views: name -> definition
    views: Arc<RwLock<HashMap<String, MaterializedViewDef>>>,
}

impl MaterializedViewRewriter {
    pub fn new() -> Self {
        Self {
            views: Arc::new(RwLock::new(HashMap::new())),
        }
    }
    
    /// Register a materialized view
    pub fn register_view(&self, name: String, query: String, base_tables: Vec<String>) {
        let mut views = self.views.write();
        views.insert(
            name.clone(),
            MaterializedViewDef {
                name,
                query,
                base_tables,
                last_refresh: SystemTime::now(),
            },
        );
    }
    
    /// Try to rewrite query to use materialized views
    pub fn rewrite_query(&self, plan: &PlanNode) -> Option<PlanNode> {
        let views = self.views.read();
        
        // Extract tables from query
        let query_tables = Self::get_query_tables(plan);
        
        // Find matching materialized view
        for view in views.values() {
            if Self::tables_match(&query_tables, &view.base_tables) {
                // Rewrite to use materialized view
                return Some(PlanNode::TableScan {
                    table: view.name.clone(),
                    columns: vec!["*".to_string()],
                });
            }
        }
        
        None
    }
    
    fn get_query_tables(plan: &PlanNode) -> Vec<String> {
        match plan {
            PlanNode::TableScan { table, .. } => vec![table.clone()],
            PlanNode::Join { left, right, .. } => {
                let mut tables = Self::get_query_tables(left);
                tables.extend(Self::get_query_tables(right));
                tables
            }
            PlanNode::Filter { input, .. }
            | PlanNode::Project { input, .. }
            | PlanNode::Sort { input, .. }
            | PlanNode::Limit { input, .. } => Self::get_query_tables(input),
            PlanNode::Aggregate { input, .. } => Self::get_query_tables(input),
            PlanNode::Subquery { plan, .. } => Self::get_query_tables(plan),
        }
    }
    
    fn tables_match(query_tables: &[String], view_tables: &[String]) -> bool {
        if query_tables.len() != view_tables.len() {
            return false;
        }
        
        let mut query_sorted = query_tables.to_vec();
        let mut view_sorted = view_tables.to_vec();
        query_sorted.sort();
        view_sorted.sort();
        
        query_sorted == view_sorted
    }
}

/// Materialized view definition
#[derive(Debug, Clone)]
struct MaterializedViewDef {
    name: String,
    query: String,
    base_tables: Vec<String>,
    last_refresh: SystemTime,
}

/// Enhanced cost model with statistics
pub struct EnhancedCostModel {
    stats_collector: Arc<StatisticsCollector>,
}

impl EnhancedCostModel {
    pub fn new(stats_collector: Arc<StatisticsCollector>) -> Self {
        Self { stats_collector }
    }
    
    /// Estimate cost of a plan node
    pub fn estimate_cost(&self, plan: &PlanNode) -> f64 {
        match plan {
            PlanNode::TableScan { table, .. } => {
                if let Some(stats) = self.stats_collector.get_table_stats(table) {
                    stats.row_count as f64
                } else {
                    1000.0 // Default estimate
                }
            }
            PlanNode::Filter { input, predicate } => {
                let input_cost = self.estimate_cost(input);
                let selectivity = self.estimate_filter_selectivity(predicate);
                input_cost * selectivity
            }
            PlanNode::Join { left, right, .. } => {
                let left_cost = self.estimate_cost(left);
                let right_cost = self.estimate_cost(right);
                left_cost * right_cost * 0.1 // Assume 10% join selectivity
            }
            PlanNode::Aggregate { input, .. } => {
                let input_cost = self.estimate_cost(input);
                input_cost * 1.2 // Aggregation overhead
            }
            PlanNode::Sort { input, .. } => {
                let input_cost = self.estimate_cost(input);
                input_cost * input_cost.ln() // O(n log n)
            }
            PlanNode::Limit { input, limit, .. } => {
                let input_cost = self.estimate_cost(input);
                input_cost.min(*limit as f64)
            }
            PlanNode::Project { input, .. } => {
                self.estimate_cost(input) * 1.05 // Small projection overhead
            }
            PlanNode::Subquery { plan, .. } => {
                self.estimate_cost(plan)
            }
        }
    }
    
    fn estimate_filter_selectivity(&self, _predicate: &str) -> f64 {
        // Simplified: return default selectivity
        // In a full implementation, would parse predicate and use column statistics
        0.3
    }
}

/// Join order optimizer using dynamic programming
pub struct JoinOrderOptimizer {
    cost_model: Arc<EnhancedCostModel>,
}

impl JoinOrderOptimizer {
    pub fn new(cost_model: Arc<EnhancedCostModel>) -> Self {
        Self { cost_model }
    }
    
    /// Find optimal join order for a set of tables
    pub fn optimize_join_order(&self, tables: Vec<PlanNode>) -> Result<PlanNode, DbError> {
        if tables.is_empty() {
            return Err(DbError::InvalidInput("No tables to join".to_string()));
        }
        
        if tables.len() == 1 {
            return Ok(tables[0].clone());
        }
        
        // For 2 tables, simple comparison
        if tables.len() == 2 {
            let left = &tables[0];
            let right = &tables[1];
            
            let cost_lr = self.estimate_join_cost(left, right);
            let cost_rl = self.estimate_join_cost(right, left);

            return if cost_lr <= cost_rl {
                Ok(self.create_join(left.clone(), right.clone()))
            } else {
                Ok(self.create_join(right.clone(), left.clone()))
            }
        }
        
        // For more tables, use dynamic programming (simplified)
        // In production, would implement full DP algorithm
        self.greedy_join_order(tables)
    }
    
    fn greedy_join_order(&self, mut tables: Vec<PlanNode>) -> Result<PlanNode, DbError> {
        while tables.len() > 1 {
            let mut best_pair = (0, 1);
            let mut best_cost = f64::MAX;
            
            // Find best pair to join
            for _i in 0..tables.len() {
                for j in (i + 1)..tables.len() {
                    let cost = self.estimate_join_cost(&tables[i], &tables[j]);
                    if cost < best_cost {
                        best_cost = cost;
                        best_pair = (i, j);
                    }
                }
            }
            
            // Join best pair
            let (i, j) = best_pair;
            let left = tables.remove(j); // Remove larger index first
            let right = tables.remove(i);
            let joined = self.create_join(right, left);
            tables.push(joined);
        }
        
        Ok(tables.pop().unwrap())
    }
    
    fn estimate_join_cost(&self, left: &PlanNode, right: &PlanNode) -> f64 {
        let left_cost = self.cost_model.estimate_cost(left);
        let right_cost = self.cost_model.estimate_cost(right);
        left_cost * right_cost * 0.1
    }
    
    fn create_join(&self, left: PlanNode, right: PlanNode) -> PlanNode {
        PlanNode::Join {
            join_type: crate::parser::JoinType::Inner,
            left: Box::new(left),
            right: Box::new(right),
            condition: "true".to_string(),
        }
    }
}

/// Index selection optimizer
pub struct IndexSelector {
    /// Available indexes: table -> column -> index type
    indexes: Arc<RwLock<HashMap<String, HashMap<String, IndexType>>>>,
}

impl IndexSelector {
    pub fn new() -> Self {
        Self {
            indexes: Arc::new(RwLock::new(HashMap::new())),
        }
    }
    
    /// Register an index
    pub fn register_index(&self, table: String, column: String, index_type: IndexType) {
        let mut indexes = self.indexes.write();
        indexes
            .entry(table)
            .or_insert_with(HashMap::new)
            .insert(column, index_type);
    }
    
    /// Select best index for a query
    pub fn select_index(&self, table: &str, column: &str, predicate: &str) -> Option<IndexType> {
        let indexes = self.indexes.read();
        
        if let Some(table_indexes) = indexes.get(table) {
            if let Some(index_type) = table_indexes.get(column) {
                // Check if index is suitable for predicate
                if Self::is_index_suitable(index_type, predicate) {
                    return Some(*index_type);
                }
            }
        }
        
        None
    }
    
    fn is_index_suitable(index_type: &IndexType, predicate: &str) -> bool {
        match index_type {
            IndexType::BTree => {
                // BTree good for range queries and equality
                true
            }
            IndexType::Hash => {
                // Hash only good for equality
                predicate.contains('=') && !predicate.contains('>')  && !predicate.contains('<')
            }
            IndexType::FullText => {
                // Full-text for text search
                predicate.contains("MATCH") || predicate.contains("CONTAINS")
            }
        }
    }
}

/// Index type enum
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum IndexType {
    BTree,
    Hash,
    FullText,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_plan_cache() {
        let cache = PlanCache::new(10);
        
        let plan = PlanNode::TableScan {
            table: "users".to_string(),
            columns: vec!["*".to_string()],
        };
        
        let ttl = Duration::from_secs(60);
        cache.put("query1".to_string(), plan.clone(), ttl);
        
        let cached = cache.get("query1");
        assert!(cached.is_some());
        
        let _stats = cache.stats();
        assert_eq!(stats.hits, 1);
        assert_eq!(stats.misses, 0);
    }
    
    #[test]
    fn test_statistics_collector() {
        let collector = StatisticsCollector::new();
        
        collector.collect_table_stats("users".to_string(), 1000, 1024000);
        
        let _stats = collector.get_table_stats("users");
        assert!(stats.is_some());
        assert_eq!(stats.unwrap().row_count, 1000);
        
        collector.collect_column_stats(
            "users".to_string(),
            "email".to_string(),
            950,
            50,
            None,
            None,
        );
        
        let col_stats = collector.get_column_stats("users", "email");
        assert!(col_stats.is_some());
        assert_eq!(col_stats.unwrap().distinct_count, 950);
    }
    
    #[test]
    fn test_adaptive_optimizer() {
        let optimizer = AdaptiveOptimizer::new();
        
        let plan = PlanNode::TableScan {
            table: "users".to_string(),
            columns: vec!["*".to_string()],
        };
        
        optimizer.record_execution("query1".to_string(), &plan, 100, 500);
        
        // Learning should not crash
        optimizer.learn_join_orders();
    }
    
    #[test]
    fn test_materialized_view_rewriter() {
        let rewriter = MaterializedViewRewriter::new();
        
        rewriter.register_view(
            "mv_users".to_string(),
            "SELECT * FROM users".to_string(),
            vec!["users".to_string()],
        );
        
        let plan = PlanNode::TableScan {
            table: "users".to_string(),
            columns: vec!["*".to_string()],
        };
        
        let rewritten = rewriter.rewrite_query(&plan);
        assert!(rewritten.is_some());
    }
    
    #[test]
    fn test_index_selector() {
        let selector = IndexSelector::new();
        
        selector.register_index(
            "users".to_string(),
            "id".to_string(),
            IndexType::BTree,
        );
        
        selector.register_index(
            "users".to_string(),
            "email".to_string(),
            IndexType::Hash,
        );
        
        let index = selector.select_index("users", "id", "id > 100");
        assert_eq!(index, Some(IndexType::BTree));
        
        let hash_index = selector.select_index("users", "email", "email = 'test@example.com'");
        assert_eq!(hash_index, Some(IndexType::Hash));
    }
}


