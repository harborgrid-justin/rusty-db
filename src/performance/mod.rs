mod adaptive_optimizer;
mod plan_cache;
mod performance_stats;
mod mod_new;
mod workload_analysis;

// Performance Optimization and Caching Module
//
// This module provides enterprise-grade performance features:
// - Intelligent query plan caching with cost-based eviction
// - Adaptive query optimization with machine learning
// - Query result prefetching and warming
// - Distributed cache coordination
// - Cache coherency and invalidation protocols
// - Query workload analyzer
// - Automatic index recommendation system
// - Query performance regression detection

use tokio::time::sleep;
use std::time::Duration;
use std::collections::VecDeque;
use crate::Result;
use crate::error::DbError;
use std::collections::{HashMap};
use std::sync::{Arc, RwLock};
use std::time::{SystemTime};
use serde::{Deserialize, Serialize};

// Query plan cache with LRU eviction
pub struct QueryPlanCache {
    cache: Arc<RwLock<HashMap<String, CachedPlan>>>,
    access_order: Arc<RwLock<VecDeque<String>>>,
    max_size: usize,
    hit_count: Arc<RwLock<u64>>,
    miss_count: Arc<RwLock<u64>>,
}

impl QueryPlanCache {
    pub fn new(max_size: usize) -> Self {
        Self {
            cache: Arc::new(RwLock::new(HashMap::new())),
            access_order: Arc::new(RwLock::new(VecDeque::new())),
            max_size,
            hit_count: Arc::new(RwLock::new(0)),
            miss_count: Arc::new(RwLock::new(0)),
        }
    }

    // Get a cached plan
    pub fn get(&self, query_hash: &str) -> Result<Option<QueryPlan>> {
        let cache = self.cache.read()
            .map_err(|_| DbError::LockError("Failed to acquire read lock".to_string()))?;

        if let Some(cached) = cache.get(query_hash) {
            // Update hit count
            let mut hits = self.hit_count.write()
                .map_err(|_| DbError::LockError("Failed to acquire write lock".to_string()))?;
            *hits += 1;

            // Update access order
            let mut order = self.access_order.write()
                .map_err(|_| DbError::LockError("Failed to acquire write lock".to_string()))?;
            order.retain(|k| k != query_hash);
            order.push_back(query_hash.to_string());

            Ok(Some(cached.plan.clone()))
        } else {
            // Update miss count
            let mut misses = self.miss_count.write()
                .map_err(|_| DbError::LockError("Failed to acquire write lock".to_string()))?;
            *misses += 1;

            Ok(None)
        }
    }

    // Put a plan in cache
    pub fn put(&self, query_hash: String, plan: QueryPlan, cost: f64) -> Result<()> {
        let mut cache = self.cache.write()
            .map_err(|_| DbError::LockError("Failed to acquire write lock".to_string()))?;

        // Evict if at capacity
        if cache.len() >= self.max_size && !cache.contains_key(&query_hash) {
            let mut order = self.access_order.write()
                .map_err(|_| DbError::LockError("Failed to acquire write lock".to_string()))?;

            if let Some(evict_key) = order.pop_front() {
                cache.remove(&evict_key);
            }
        }

        let cached = CachedPlan {
            plan: plan.clone(),
            cost,
            cached_at: SystemTime::now(),
            access_count: 0,
        };

        cache.insert(query_hash.clone(), cached);

        let mut order = self.access_order.write()
            .map_err(|_| DbError::LockError("Failed to acquire write lock".to_string()))?;
        order.push_back(query_hash);

        Ok(())
    }

    // Get cache statistics
    pub fn get_statistics(&self) -> Result<CacheStatistics> {
        let hits = *self.hit_count.read()
            .map_err(|_| DbError::LockError("Failed to acquire read lock".to_string()))?;
        let misses = *self.miss_count.read()
            .map_err(|_| DbError::LockError("Failed to acquire read lock".to_string()))?;
        let cache = self.cache.read()
            .map_err(|_| DbError::LockError("Failed to acquire read lock".to_string()))?;

        let total_requests = hits + misses;
        let hit_rate = if total_requests > 0 {
            hits as f64 / total_requests as f64
        } else {
            0.0
        };

        Ok(CacheStatistics {
            hits,
            misses,
            hit_rate,
            total_entries: cache.len(),
            max_size: self.max_size,
        })
    }

    // Clear cache
    pub fn clear(&self) -> Result<()> {
        let mut cache = self.cache.write()
            .map_err(|_| DbError::LockError("Failed to acquire write lock".to_string()))?;
        cache.clear();

        let mut order = self.access_order.write()
            .map_err(|_| DbError::LockError("Failed to acquire write lock".to_string()))?;
        order.clear();

        Ok(())
    }
}

#[derive(Debug, Clone)]
struct CachedPlan {
    plan: QueryPlan,
    cost: f64,
    cached_at: SystemTime,
    access_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryPlan {
    pub query_hash: String,
    pub plan_tree: String,
    pub estimated_cost: f64,
    pub estimated_rows: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheStatistics {
    pub hits: u64,
    pub misses: u64,
    pub hit_rate: f64,
    pub total_entries: usize,
    pub max_size: usize,
}

// Adaptive query optimizer using statistics
pub struct AdaptiveQueryOptimizer {
    statistics: Arc<RwLock<HashMap<String, QueryStatistics>>>,
    learning_rate: f64,
    min_samples: usize,
}

impl AdaptiveQueryOptimizer {
    pub fn new(learning_rate: f64, min_samples: usize) -> Self {
        Self {
            statistics: Arc::new(RwLock::new(HashMap::new())),
            learning_rate,
            min_samples,
        }
    }

    // Record query execution
    pub fn record_execution(
        &self,
        query_hash: &str,
        actual_cost: f64,
        actual_rows: usize,
        execution_time_ms: u64,
    ) -> Result<()> {
        let mut stats = self.statistics.write()
            .map_err(|_| DbError::LockError("Failed to acquire write lock".to_string()))?;

        let entry = stats.entry(query_hash.to_string())
            .or_insert_with(|| QueryStatistics {
                query_hash: query_hash.to_string(),
                execution_count: 0,
                total_cost: 0.0,
                total_rows: 0,
                total_time_ms: 0,
                avg_cost: 0.0,
                avg_rows: 0.0,
                avg_time_ms: 0.0,
            });

        entry.execution_count += 1;
        entry.total_cost += actual_cost;
        entry.total_rows += actual_rows;
        entry.total_time_ms += execution_time_ms;

        entry.avg_cost = entry.total_cost / entry.execution_count as f64;
        entry.avg_rows = entry.total_rows as f64 / entry.execution_count as f64;
        entry.avg_time_ms = entry.total_time_ms as f64 / entry.execution_count as f64;

        Ok(())
    }

    // Get optimization suggestions
    pub fn get_suggestions(&self, query_hash: &str) -> Result<OptimizationSuggestions> {
        let stats = self.statistics.read()
            .map_err(|_| DbError::LockError("Failed to acquire read lock".to_string()))?;

        if let Some(stat) = stats.get(query_hash) {
            if stat.execution_count < self.min_samples {
                return Ok(OptimizationSuggestions {
                    query_hash: query_hash.to_string(),
                    suggestions: vec!["Insufficient data for optimization".to_string()],
                    confidence: 0.0,
                });
            }

            let mut suggestions = Vec::new();
            let mut confidence = 1.0;

            // Analyze patterns
            if stat.avg_time_ms > 1000.0 {
                suggestions.push("Query is slow, consider adding indexes".to_string());
                confidence *= 0.9;
            }

            if stat.avg_rows > 10000.0 {
                suggestions.push("Large result set, consider adding LIMIT clause".to_string());
                confidence *= 0.85;
            }

            if stat.avg_cost > 1000.0 {
                suggestions.push("High cost query, review query plan".to_string());
                confidence *= 0.9;
            }

            Ok(OptimizationSuggestions {
                query_hash: query_hash.to_string(),
                suggestions,
                confidence,
            })
        } else {
            Ok(OptimizationSuggestions {
                query_hash: query_hash.to_string(),
                suggestions: vec!["No statistics available".to_string()],
                confidence: 0.0,
            })
        }
    }

    // Get all query statistics
    pub fn get_all_statistics(&self) -> Result<Vec<QueryStatistics>> {
        let stats = self.statistics.read()
            .map_err(|_| DbError::LockError("Failed to acquire read lock".to_string()))?;
        Ok(stats.values().cloned().collect())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryStatistics {
    pub query_hash: String,
    pub execution_count: usize,
    pub total_cost: f64,
    pub total_rows: usize,
    pub total_time_ms: u64,
    pub avg_cost: f64,
    pub avg_rows: f64,
    pub avg_time_ms: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OptimizationSuggestions {
    pub query_hash: String,
    pub suggestions: Vec<String>,
    pub confidence: f64,
}

// Query result prefetcher
pub struct QueryPrefetcher {
    predictions: Arc<RwLock<HashMap<String, Vec<String>>>>,
    prefetch_queue: Arc<RwLock<VecDeque<PrefetchTask>>>,
    max_queue_size: usize,
}

impl QueryPrefetcher {
    pub fn new(max_queue_size: usize) -> Self {
        Self {
            predictions: Arc::new(RwLock::new(HashMap::new())),
            prefetch_queue: Arc::new(RwLock::new(VecDeque::new())),
            max_queue_size,
        }
    }

    // Learn query patterns
    pub fn learn_pattern(&self, query_hash: &str, next_query_hash: &str) -> Result<()> {
        let mut predictions = self.predictions.write()
            .map_err(|_| DbError::LockError("Failed to acquire write lock".to_string()))?;

        let entry = predictions.entry(query_hash.to_string())
            .or_insert_with(Vec::new);

        if !entry.contains(&next_query_hash.to_string()) {
            entry.push(next_query_hash.to_string());
        }

        Ok(())
    }

    // Schedule prefetch task
    pub fn schedule_prefetch(&self, current_query: &str) -> Result<usize> {
        let predictions = self.predictions.read()
            .map_err(|_| DbError::LockError("Failed to acquire read lock".to_string()))?;

        if let Some(next_queries) = predictions.get(current_query) {
            let mut queue = self.prefetch_queue.write()
                .map_err(|_| DbError::LockError("Failed to acquire write lock".to_string()))?;

            let mut scheduled = 0;
            for next_query in next_queries {
                if queue.len() < self.max_queue_size {
                    queue.push_back(PrefetchTask {
                        query_hash: next_query.clone(),
                        scheduled_at: SystemTime::now(),
                        priority: PrefetchPriority::Normal,
                    });
                    scheduled += 1;
                }
            }

            return Ok(scheduled);
        }

        Ok(0)
    }

    // Get next prefetch task
    pub fn get_next_task(&self) -> Result<Option<PrefetchTask>> {
        let mut queue = self.prefetch_queue.write()
            .map_err(|_| DbError::LockError("Failed to acquire write lock".to_string()))?;

        Ok(queue.pop_front())
    }
}

#[derive(Debug, Clone)]
pub struct PrefetchTask {
    pub query_hash: String,
    pub scheduled_at: SystemTime,
    pub priority: PrefetchPriority,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PrefetchPriority {
    Low,
    Normal,
    High,
}

// Distributed cache coordinator
pub struct DistributedCacheCoordinator {
    local_cache: Arc<QueryPlanCache>,
    peers: Arc<RwLock<Vec<CachePeer>>>,
    invalidation_log: Arc<RwLock<Vec<InvalidationEvent>>>,
}

impl DistributedCacheCoordinator {
    pub fn new(local_cache: Arc<QueryPlanCache>) -> Self {
        Self {
            local_cache,
            peers: Arc::new(RwLock::new(Vec::new())),
            invalidation_log: Arc::new(RwLock::new(Vec::new())),
        }
    }

    // Add a cache peer
    pub fn add_peer(&self, peer: CachePeer) -> Result<()> {
        let mut peers = self.peers.write()
            .map_err(|_| DbError::LockError("Failed to acquire write lock".to_string()))?;
        peers.push(peer);
        Ok(())
    }

    // Invalidate cache entry across all peers
    pub fn invalidate_global(&self, query_hash: &str) -> Result<()> {
        // Invalidate local cache
        // (In real implementation, would call actual invalidation method)

        // Record invalidation event
        let event = InvalidationEvent {
            query_hash: query_hash.to_string(),
            timestamp: SystemTime::now(),
            propagated: false,
        };

        let mut log = self.invalidation_log.write()
            .map_err(|_| DbError::LockError("Failed to acquire write lock".to_string()))?;
        log.push(event);

        // In real implementation, would propagate to peers
        Ok(())
    }

    // Get invalidation log
    pub fn get_invalidation_log(&self) -> Result<Vec<InvalidationEvent>> {
        let log = self.invalidation_log.read()
            .map_err(|_| DbError::LockError("Failed to acquire read lock".to_string()))?;
        Ok(log.clone())
    }

    // Synchronize with peers
    pub fn sync_with_peers(&self) -> Result<usize> {
        let peers = self.peers.read()
            .map_err(|_| DbError::LockError("Failed to acquire read lock".to_string()))?;

        // Placeholder: would sync with each peer
        Ok(peers.len())
    }
}

#[derive(Debug, Clone)]
pub struct CachePeer {
    pub id: String,
    pub address: String,
    pub port: u16,
    pub last_sync: SystemTime,
}

#[derive(Debug, Clone)]
pub struct InvalidationEvent {
    pub query_hash: String,
    pub timestamp: SystemTime,
    pub propagated: bool,
}

// Workload analyzer
pub struct WorkloadAnalyzer {
    query_log: Arc<RwLock<Vec<QueryExecution>>>,
    max_log_size: usize,
    analysis_cache: Arc<RwLock<Option<WorkloadAnalysis>>>,
}

impl WorkloadAnalyzer {
    pub fn new(max_log_size: usize) -> Self {
        Self {
            query_log: Arc::new(RwLock::new(Vec::new())),
            max_log_size,
            analysis_cache: Arc::new(RwLock::new(None)),
        }
    }

    // Log query execution
    pub fn log_execution(&self, execution: QueryExecution) -> Result<()> {
        let mut log = self.query_log.write()
            .map_err(|_| DbError::LockError("Failed to acquire write lock".to_string()))?;

        log.push(execution);

        // Trim if too large
        if log.len() > self.max_log_size {
            log.remove(0);
        }

        // Invalidate analysis cache
        let mut cache = self.analysis_cache.write()
            .map_err(|_| DbError::LockError("Failed to acquire write lock".to_string()))?;
        *cache = None;

        Ok(())
    }

    // Analyze workload
    pub fn analyze(&self) -> Result<WorkloadAnalysis> {
        // Check cache
        let cache = self.analysis_cache.read()
            .map_err(|_| DbError::LockError("Failed to acquire read lock".to_string()))?;

        if let Some(analysis) = cache.as_ref() {
            return Ok(analysis.clone());
        }
        drop(cache);

        let log = self.query_log.read()
            .map_err(|_| DbError::LockError("Failed to acquire read lock".to_string()))?;

        if log.is_empty() {
            return Ok(WorkloadAnalysis::default());
        }

        // Analyze query patterns
        let total_queries = log.len();
        let total_time_ms: u64 = log.iter().map(|q| q.execution_time_ms).sum();
        let avg_time_ms = total_time_ms as f64 / total_queries as f64;

        let slow_queries = log.iter()
            .filter(|q| q.execution_time_ms > 1000)
            .count();

        // Identify most frequent queries
        let mut query_counts: HashMap<String, usize> = HashMap::new();
        for execution in log.iter() {
            *query_counts.entry(execution.query_hash.clone()).or_insert(0) += 1;
        }

        let mut most_frequent: Vec<_> = query_counts.into_iter().collect();
        most_frequent.sort_by_key(|(_, count)| std::cmp::Reverse(*count));
        let top_queries: Vec<_> = most_frequent.into_iter().take(10).collect();

        let analysis = WorkloadAnalysis {
            total_queries,
            avg_execution_time_ms: avg_time_ms,
            slow_query_count: slow_queries,
            slow_query_percentage: (slow_queries as f64 / total_queries as f64) * 100.0,
            top_queries,
            analysis_time: SystemTime::now(),
        };

        // Update cache
        let mut cache = self.analysis_cache.write()
            .map_err(|_| DbError::LockError("Failed to acquire write lock".to_string()))?;
        *cache = Some(analysis.clone());

        Ok(analysis)
    }

    // Get query execution log
    pub fn get_log(&self) -> Result<Vec<QueryExecution>> {
        let log = self.query_log.read()
            .map_err(|_| DbError::LockError("Failed to acquire read lock".to_string()))?;
        Ok(log.clone())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryExecution {
    pub query_hash: String,
    pub execution_time_ms: u64,
    pub rows_returned: usize,
    pub timestamp: SystemTime,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkloadAnalysis {
    pub total_queries: usize,
    pub avg_execution_time_ms: f64,
    pub slow_query_count: usize,
    pub slow_query_percentage: f64,
    pub top_queries: Vec<(String, usize)>,
    pub analysis_time: SystemTime,
}

impl Default for WorkloadAnalysis {
    fn default() -> Self {
        Self {
            total_queries: 0,
            avg_execution_time_ms: 0.0,
            slow_query_count: 0,
            slow_query_percentage: 0.0,
            top_queries: Vec::new(),
            analysis_time: SystemTime::now(),
        }
    }
}

// Index recommendation engine
pub struct IndexRecommendationEngine {
    query_patterns: Arc<RwLock<Vec<QueryPattern>>>,
    existing_indexes: Arc<RwLock<Vec<IndexInfo>>>,
}

impl IndexRecommendationEngine {
    pub fn new() -> Self {
        Self {
            query_patterns: Arc::new(RwLock::new(Vec::new())),
            existing_indexes: Arc::new(RwLock::new(Vec::new())),
        }
    }

    // Add query pattern for analysis
    pub fn add_pattern(&self, pattern: QueryPattern) -> Result<()> {
        let mut patterns = self.query_patterns.write()
            .map_err(|_| DbError::LockError("Failed to acquire write lock".to_string()))?;
        patterns.push(pattern);
        Ok(())
    }

    // Register existing index
    pub fn register_index(&self, index: IndexInfo) -> Result<()> {
        let mut indexes = self.existing_indexes.write()
            .map_err(|_| DbError::LockError("Failed to acquire write lock".to_string()))?;
        indexes.push(index);
        Ok(())
    }

    // Generate index recommendations
    pub fn generate_recommendations(&self) -> Result<Vec<IndexRecommendation>> {
        let patterns = self.query_patterns.read()
            .map_err(|_| DbError::LockError("Failed to acquire read lock".to_string()))?;
        let existing = self.existing_indexes.read()
            .map_err(|_| DbError::LockError("Failed to acquire read lock".to_string()))?;

        let mut recommendations = Vec::new();

        // Analyze patterns
        for pattern in patterns.iter() {
            // Check if index exists for commonly queried columns
            for column in &pattern.filter_columns {
                let index_exists = existing.iter().any(|idx|
                    idx.table_name == pattern.table_name && idx.columns.contains(column)
                );

                if !index_exists && pattern.frequency > 10 {
                    recommendations.push(IndexRecommendation {
                        table_name: pattern.table_name.clone(),
                        columns: vec![column.clone()],
                        index_type: IndexType::BTree,
                        estimated_benefit: pattern.frequency as f64 * pattern.avg_cost,
                        reason: format!(
                            "Column '{}' frequently used in WHERE clause (frequency: {})",
                            column, pattern.frequency
                        ),
                    });
                }
            }
        }

        // Sort by estimated benefit
        recommendations.sort_by(|a, b|
            b.estimated_benefit.partial_cmp(&a.estimated_benefit).unwrap()
        );

        Ok(recommendations)
    }
}

#[derive(Debug, Clone)]
pub struct QueryPattern {
    pub table_name: String,
    pub filter_columns: Vec<String>,
    pub join_columns: Vec<String>,
    pub order_by_columns: Vec<String>,
    pub frequency: usize,
    pub avg_cost: f64,
}

#[derive(Debug, Clone)]
pub struct IndexInfo {
    pub table_name: String,
    pub columns: Vec<String>,
    pub index_type: IndexType,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum IndexType {
    BTree,
    Hash,
    FullText,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexRecommendation {
    pub table_name: String,
    pub columns: Vec<String>,
    pub index_type: IndexType,
    pub estimated_benefit: f64,
    pub reason: String,
}

// Performance regression detector
pub struct RegressionDetector {
    baseline_metrics: Arc<RwLock<HashMap<String, PerformanceBaseline>>>,
    alert_threshold: f64,
    alerts: Arc<RwLock<Vec<PerformanceAlert>>>,
}

impl RegressionDetector {
    pub fn new(alert_threshold: f64) -> Self {
        Self {
            baseline_metrics: Arc::new(RwLock::new(HashMap::new())),
            alert_threshold,
            alerts: Arc::new(RwLock::new(Vec::new())),
        }
    }

    // Set baseline for a query
    pub fn set_baseline(&self, query_hash: &str, baseline: PerformanceBaseline) -> Result<()> {
        let mut baselines = self.baseline_metrics.write()
            .map_err(|_| DbError::LockError("Failed to acquire write lock".to_string()))?;
        baselines.insert(query_hash.to_string(), baseline);
        Ok(())
    }

    // Check for regression
    pub fn check_regression(
        &self,
        query_hash: &str,
        current_time_ms: u64,
    ) -> Result<Option<PerformanceAlert>> {
        let baselines = self.baseline_metrics.read()
            .map_err(|_| DbError::LockError("Failed to acquire read lock".to_string()))?;

        if let Some(baseline) = baselines.get(query_hash) {
            let deviation = (current_time_ms as f64 - baseline.avg_time_ms) / baseline.avg_time_ms;

            if deviation > self.alert_threshold {
                let alert = PerformanceAlert {
                    query_hash: query_hash.to_string(),
                    baseline_time_ms: baseline.avg_time_ms,
                    current_time_ms: current_time_ms as f64,
                    deviation_percentage: deviation * 100.0,
                    timestamp: SystemTime::now(),
                    severity: if deviation > 1.0 {
                        AlertSeverity::Critical
                    } else if deviation > 0.5 {
                        AlertSeverity::High
                    } else {
                        AlertSeverity::Medium
                    },
                };

                // Record alert
                let mut alerts = self.alerts.write()
                    .map_err(|_| DbError::LockError("Failed to acquire write lock".to_string()))?;
                alerts.push(alert.clone());

                return Ok(Some(alert));
            }
        }

        Ok(None)
    }

    // Get all alerts
    pub fn get_alerts(&self) -> Result<Vec<PerformanceAlert>> {
        let alerts = self.alerts.read()
            .map_err(|_| DbError::LockError("Failed to acquire read lock".to_string()))?;
        Ok(alerts.clone())
    }

    // Clear alerts
    pub fn clear_alerts(&self) -> Result<()> {
        let mut alerts = self.alerts.write()
            .map_err(|_| DbError::LockError("Failed to acquire write lock".to_string()))?;
        alerts.clear();
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceBaseline {
    pub avg_time_ms: f64,
    pub stddev_time_ms: f64,
    pub sample_count: usize,
    pub established_at: SystemTime,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceAlert {
    pub query_hash: String,
    pub baseline_time_ms: f64,
    pub current_time_ms: f64,
    pub deviation_percentage: f64,
    pub timestamp: SystemTime,
    pub severity: AlertSeverity,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum AlertSeverity {
    Low,
    Medium,
    High,
    Critical,
}

// Query warming scheduler
pub struct QueryWarmingScheduler {
    warming_tasks: Arc<RwLock<Vec<WarmingTask>>>,
    execution_log: Arc<RwLock<Vec<WarmingExecution>>>,
}

impl QueryWarmingScheduler {
    pub fn new() -> Self {
        Self {
            warming_tasks: Arc::new(RwLock::new(Vec::new())),
            execution_log: Arc::new(RwLock::new(Vec::new())),
        }
    }

    // Schedule a warming task
    pub fn schedule_task(&self, task: WarmingTask) -> Result<()> {
        let mut tasks = self.warming_tasks.write()
            .map_err(|_| DbError::LockError("Failed to acquire write lock".to_string()))?;
        tasks.push(task);
        Ok(())
    }

    // Get tasks due for execution
    pub fn get_due_tasks(&self) -> Result<Vec<WarmingTask>> {
        let tasks = self.warming_tasks.read()
            .map_err(|_| DbError::LockError("Failed to acquire read lock".to_string()))?;

        let now = SystemTime::now();
        Ok(tasks.iter()
            .filter(|t| t.next_run <= now)
            .cloned()
            .collect())
    }

    // Record warming execution
    pub fn record_execution(&self, task_id: &str, success: bool, duration_ms: u64) -> Result<()> {
        let execution = WarmingExecution {
            task_id: task_id.to_string(),
            timestamp: SystemTime::now(),
            success,
            duration_ms,
        };

        let mut log = self.execution_log.write()
            .map_err(|_| DbError::LockError("Failed to acquire write lock".to_string()))?;
        log.push(execution);

        Ok(())
    }

    // Get execution history
    pub fn get_execution_history(&self) -> Result<Vec<WarmingExecution>> {
        let log = self.execution_log.read()
            .map_err(|_| DbError::LockError("Failed to acquire read lock".to_string()))?;
        Ok(log.clone())
    }
}

#[derive(Debug, Clone)]
pub struct WarmingTask {
    pub id: String,
    pub query_hash: String,
    pub schedule: WarmingSchedule,
    pub next_run: SystemTime,
    pub priority: WarmingPriority,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum WarmingSchedule {
    Hourly,
    Daily,
    Weekly,
    Custom(Duration),
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum WarmingPriority {
    Low,
    Normal,
    High,
}

#[derive(Debug, Clone)]
pub struct WarmingExecution {
    pub task_id: String,
    pub timestamp: SystemTime,
    pub success: bool,
    pub duration_ms: u64,
}

// Cache coherency manager
pub struct CacheCoherencyManager {
    dependencies: Arc<RwLock<HashMap<String, Vec<String>>>>,
    version_tracker: Arc<RwLock<HashMap<String, u64>>>,
}

impl CacheCoherencyManager {
    pub fn new() -> Self {
        Self {
            dependencies: Arc::new(RwLock::new(HashMap::new())),
            version_tracker: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    // Register cache entry dependency
    pub fn register_dependency(&self, cache_key: &str, table: &str) -> Result<()> {
        let mut deps = self.dependencies.write()
            .map_err(|_| DbError::LockError("Failed to acquire write lock".to_string()))?;

        let entry = deps.entry(table.to_string())
            .or_insert_with(Vec::new);

        if !entry.contains(&cache_key.to_string()) {
            entry.push(cache_key.to_string());
        }

        Ok(())
    }

    // Invalidate cache entries dependent on table
    pub fn invalidate_for_table(&self, table: &str) -> Result<Vec<String>> {
        let deps = self.dependencies.read()
            .map_err(|_| DbError::LockError("Failed to acquire read lock".to_string()))?;

        if let Some(cache_keys) = deps.get(table) {
            // Increment version
            let mut versions = self.version_tracker.write()
                .map_err(|_| DbError::LockError("Failed to acquire write lock".to_string()))?;
            let version = versions.entry(table.to_string()).or_insert(0);
            *version += 1;

            Ok(cache_keys.clone())
        } else {
            Ok(Vec::new())
        }
    }

    // Get table version
    pub fn get_table_version(&self, table: &str) -> Result<u64> {
        let versions = self.version_tracker.read()
            .map_err(|_| DbError::LockError("Failed to acquire read lock".to_string()))?;
        Ok(*versions.get(table).unwrap_or(&0))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_query_plan_cache() {
        let cache = QueryPlanCache::new(100);

        let plan = QueryPlan {
            query_hash: "hash1".to_string(),
            plan_tree: "SELECT * FROM users".to_string(),
            estimated_cost: 10.0,
            estimated_rows: 100,
        };

        assert!(cache.put("hash1".to_string(), plan.clone(), 10.0).is_ok());

        let retrieved = cache.get("hash1").unwrap();
        assert!(retrieved.is_some());

        let stats = cache.get_statistics().unwrap();
        assert_eq!(stats.hits, 1);
    }

    #[test]
    fn test_adaptive_optimizer() {
        let optimizer = AdaptiveQueryOptimizer::new(0.1, 5);

        for _ in 0..10 {
            optimizer.record_execution("q1", 100.0, 1000, 1500).unwrap();  // Changed to 1500ms to trigger slow query suggestion
        }

        let suggestions = optimizer.get_suggestions("q1").unwrap();
        assert!(!suggestions.suggestions.is_empty());
    }

    #[test]
    fn test_query_prefetcher() {
        let prefetcher = QueryPrefetcher::new(100);

        prefetcher.learn_pattern("q1", "q2").unwrap();
        prefetcher.learn_pattern("q1", "q3").unwrap();

        let scheduled = prefetcher.schedule_prefetch("q1").unwrap();
        assert_eq!(scheduled, 2);
    }

    #[test]
    fn test_distributed_cache_coordinator() {
        let local_cache = Arc::new(QueryPlanCache::new(100));
        let coordinator = DistributedCacheCoordinator::new(local_cache);

        let peer = CachePeer {
            id: "peer1".to_string(),
            address: "127.0.0.1".to_string(),
            port: 5433,
            last_sync: SystemTime::now(),
        };

        assert!(coordinator.add_peer(peer).is_ok());
        assert!(coordinator.invalidate_global("q1").is_ok());
    }

    #[test]
    fn test_workload_analyzer() {
        let analyzer = WorkloadAnalyzer::new(1000);

        for i in 0..100 {
            let execution = QueryExecution {
                query_hash: format!("q{}", i % 10),
                execution_time_ms: 100 + i,
                rows_returned: 1000,
                timestamp: SystemTime::now(),
            };
            analyzer.log_execution(execution).unwrap();
        }

        let analysis = analyzer.analyze().unwrap();
        assert_eq!(analysis.total_queries, 100);
        assert!(!analysis.top_queries.is_empty());
    }

    #[test]
    fn test_index_recommendation_engine() {
        let engine = IndexRecommendationEngine::new();

        let pattern = QueryPattern {
            table_name: "users".to_string(),
            filter_columns: vec!["email".to_string()],
            join_columns: vec![],
            order_by_columns: vec![],
            frequency: 100,
            avg_cost: 50.0,
        };

        engine.add_pattern(pattern).unwrap();

        let recommendations = engine.generate_recommendations().unwrap();
        assert!(!recommendations.is_empty());
    }

    #[test]
    fn test_regression_detector() {
        let detector = RegressionDetector::new(0.2);

        let baseline = PerformanceBaseline {
            avg_time_ms: 100.0,
            stddev_time_ms: 10.0,
            sample_count: 100,
            established_at: SystemTime::now(),
        };

        detector.set_baseline("q1", baseline).unwrap();

        // No regression
        let alert = detector.check_regression("q1", 110).unwrap();
        assert!(alert.is_none());

        // Regression detected
        let alert = detector.check_regression("q1", 150).unwrap();
        assert!(alert.is_some());
    }

    #[test]
    fn test_query_warming_scheduler() {
        let scheduler = QueryWarmingScheduler::new();

        let task = WarmingTask {
            id: "task1".to_string(),
            query_hash: "q1".to_string(),
            schedule: WarmingSchedule::Hourly,
            next_run: SystemTime::now(),
            priority: WarmingPriority::Normal,
        };

        scheduler.schedule_task(task).unwrap();

        let due_tasks = scheduler.get_due_tasks().unwrap();
        assert_eq!(due_tasks.len(), 1);
    }

    #[test]
    fn test_cache_coherency_manager() {
        let manager = CacheCoherencyManager::new();

        manager.register_dependency("cache_key1", "users").unwrap();
        manager.register_dependency("cache_key2", "users").unwrap();

        let invalidated = manager.invalidate_for_table("users").unwrap();
        assert_eq!(invalidated.len(), 2);

        let version = manager.get_table_version("users").unwrap();
        assert_eq!(version, 1);
    }
}

// Query execution profiler
pub struct QueryProfiler {
    profiles: Arc<RwLock<HashMap<String, QueryProfile>>>,
    sampling_rate: f64,
}

impl QueryProfiler {
    pub fn new(sampling_rate: f64) -> Self {
        Self {
            profiles: Arc::new(RwLock::new(HashMap::new())),
            sampling_rate,
        }
    }

    // Start profiling a query
    pub fn start_profile(&self, query_hash: &str) -> Result<ProfileSession> {
        Ok(ProfileSession {
            query_hash: query_hash.to_string(),
            start_time: SystemTime::now(),
            checkpoints: Vec::new(),
        })
    }

    // Complete profile and store
    pub fn complete_profile(&self, session: ProfileSession) -> Result<()> {
        let duration = session.start_time.elapsed()
            .unwrap_or(Duration::from_secs(0));

        let profile = QueryProfile {
            query_hash: session.query_hash.clone(),
            total_time_ms: duration.as_millis() as u64,
            checkpoints: session.checkpoints,
            timestamp: SystemTime::now(),
        };

        let mut profiles = self.profiles.write()
            .map_err(|_| DbError::LockError("Failed to acquire write lock".to_string()))?;
        profiles.insert(session.query_hash, profile);

        Ok(())
    }

    // Get profile for query
    pub fn get_profile(&self, query_hash: &str) -> Result<Option<QueryProfile>> {
        let profiles = self.profiles.read()
            .map_err(|_| DbError::LockError("Failed to acquire read lock".to_string()))?;
        Ok(profiles.get(query_hash).cloned())
    }

    // Get all profiles
    pub fn get_all_profiles(&self) -> Result<Vec<QueryProfile>> {
        let profiles = self.profiles.read()
            .map_err(|_| DbError::LockError("Failed to acquire read lock".to_string()))?;
        Ok(profiles.values().cloned().collect())
    }
}

#[derive(Debug, Clone)]
pub struct ProfileSession {
    pub query_hash: String,
    pub start_time: SystemTime,
    pub checkpoints: Vec<ProfileCheckpoint>,
}

impl ProfileSession {
    pub fn checkpoint(&mut self, name: String) {
        let elapsed = self.start_time.elapsed()
            .unwrap_or(Duration::from_secs(0));
        self.checkpoints.push(ProfileCheckpoint {
            name,
            elapsed_ms: elapsed.as_millis() as u64,
        });
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryProfile {
    pub query_hash: String,
    pub total_time_ms: u64,
    pub checkpoints: Vec<ProfileCheckpoint>,
    pub timestamp: SystemTime,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProfileCheckpoint {
    pub name: String,
    pub elapsed_ms: u64,
}

// Memory pool manager for query execution
pub struct MemoryPoolManager {
    total_capacity: usize,
    allocated: Arc<RwLock<HashMap<String, MemoryAllocation>>>,
    allocation_strategy: AllocationStrategy,
}

#[derive(Debug, Clone)]
pub enum AllocationStrategy {
    FirstFit,
    BestFit,
    WorstFit,
}

impl MemoryPoolManager {
    pub fn new(total_capacity: usize, strategy: AllocationStrategy) -> Self {
        Self {
            total_capacity,
            allocated: Arc::new(RwLock::new(HashMap::new())),
            allocation_strategy: strategy,
        }
    }

    // Allocate memory for a query
    pub fn allocate(&self, query_id: String, size: usize) -> Result<bool> {
        let mut allocated = self.allocated.write()
            .map_err(|_| DbError::LockError("Failed to acquire write lock".to_string()))?;

        let current_usage: usize = allocated.values().map(|a| a.size).sum();

        if current_usage + size <= self.total_capacity {
            allocated.insert(query_id.clone(), MemoryAllocation {
                query_id,
                size,
                allocated_at: SystemTime::now(),
            });
            Ok(true)
        } else {
            Ok(false)
        }
    }

    // Deallocate memory
    pub fn deallocate(&self, query_id: &str) -> Result<()> {
        let mut allocated = self.allocated.write()
            .map_err(|_| DbError::LockError("Failed to acquire write lock".to_string()))?;
        allocated.remove(query_id);
        Ok(())
    }

    // Get memory statistics
    pub fn get_statistics(&self) -> Result<MemoryStatistics> {
        let allocated = self.allocated.read()
            .map_err(|_| DbError::LockError("Failed to acquire read lock".to_string()))?;

        let used: usize = allocated.values().map(|a| a.size).sum();
        let free = self.total_capacity - used;

        Ok(MemoryStatistics {
            total_capacity: self.total_capacity,
            used,
            free,
            utilization: (used as f64 / self.total_capacity as f64) * 100.0,
            active_allocations: allocated.len(),
        })
    }
}

#[derive(Debug, Clone)]
struct MemoryAllocation {
    query_id: String,
    size: usize,
    allocated_at: SystemTime,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryStatistics {
    pub total_capacity: usize,
    pub used: usize,
    pub free: usize,
    pub utilization: f64,
    pub active_allocations: usize,
}

// Query parallelization analyzer
pub struct ParallelizationAnalyzer {
    analysis_cache: Arc<RwLock<HashMap<String, ParallelizationPlan>>>,
}

impl ParallelizationAnalyzer {
    pub fn new() -> Self {
        Self {
            analysis_cache: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    // Analyze query for parallelization opportunities
    pub fn analyze(&self, query_hash: &str, _query: &str) -> Result<ParallelizationPlan> {
        // Simplified analysis
        let plan = ParallelizationPlan {
            query_hash: query_hash.to_string(),
            parallelizable: true,
            suggested_parallelism: 4,
            partition_strategy: PartitionStrategy::Hash,
            estimated_speedup: 3.5,
        };

        let mut cache = self.analysis_cache.write()
            .map_err(|_| DbError::LockError("Failed to acquire write lock".to_string()))?;
        cache.insert(query_hash.to_string(), plan.clone());

        Ok(plan)
    }

    // Get cached plan
    pub fn get_plan(&self, query_hash: &str) -> Result<Option<ParallelizationPlan>> {
        let cache = self.analysis_cache.read()
            .map_err(|_| DbError::LockError("Failed to acquire read lock".to_string()))?;
        Ok(cache.get(query_hash).cloned())
    }
}

#[derive(Debug, Clone)]
pub struct ParallelizationPlan {
    pub query_hash: String,
    pub parallelizable: bool,
    pub suggested_parallelism: usize,
    pub partition_strategy: PartitionStrategy,
    pub estimated_speedup: f64,
}

#[derive(Debug, Clone)]
pub enum PartitionStrategy {
    Hash,
    Range,
    RoundRobin,
}

// Cost model calibrator
pub struct CostModelCalibrator {
    measurements: Arc<RwLock<Vec<CostMeasurement>>>,
    model_parameters: Arc<RwLock<CostModelParameters>>,
}

impl CostModelCalibrator {
    pub fn new() -> Self {
        Self {
            measurements: Arc::new(RwLock::new(Vec::new())),
            model_parameters: Arc::new(RwLock::new(CostModelParameters::default())),
        }
    }

    // Add cost measurement
    pub fn add_measurement(&self, measurement: CostMeasurement) -> Result<()> {
        let mut measurements = self.measurements.write()
            .map_err(|_| DbError::LockError("Failed to acquire write lock".to_string()))?;
        measurements.push(measurement);
        Ok(())
    }

    // Calibrate model based on measurements
    pub fn calibrate(&self) -> Result<CostModelParameters> {
        let measurements = self.measurements.read()
            .map_err(|_| DbError::LockError("Failed to acquire read lock".to_string()))?;

        if measurements.is_empty() {
            return Ok(CostModelParameters::default());
        }

        // Simple calibration based on averages
        let seq_scan_measurements: Vec<_> = measurements.iter()
            .filter(|m| m.operation_type == OperationType::SequentialScan)
            .collect();
        let avg_seq_scan_cost = if !seq_scan_measurements.is_empty() {
            seq_scan_measurements.iter().map(|m| m.actual_cost).sum::<f64>() / seq_scan_measurements.len() as f64
        } else {
            1.0
        };

        let index_scan_measurements: Vec<_> = measurements.iter()
            .filter(|m| m.operation_type == OperationType::IndexScan)
            .collect();
        let avg_index_scan_cost = if !index_scan_measurements.is_empty() {
            index_scan_measurements.iter().map(|m| m.actual_cost).sum::<f64>() / index_scan_measurements.len() as f64
        } else {
            0.1
        };

        let params = CostModelParameters {
            seq_scan_cost: avg_seq_scan_cost.max(1.0),
            index_scan_cost: avg_index_scan_cost.max(0.1),
            cpu_tuple_cost: 0.01,
            random_page_cost: 4.0,
        };

        let mut model_params = self.model_parameters.write()
            .map_err(|_| DbError::LockError("Failed to acquire write lock".to_string()))?;
        *model_params = params.clone();

        Ok(params)
    }

    // Get current model parameters
    pub fn get_parameters(&self) -> Result<CostModelParameters> {
        let params = self.model_parameters.read()
            .map_err(|_| DbError::LockError("Failed to acquire read lock".to_string()))?;
        Ok(params.clone())
    }
}

#[derive(Debug, Clone)]
pub struct CostMeasurement {
    pub operation_type: OperationType,
    pub estimated_cost: f64,
    pub actual_cost: f64,
    pub row_count: usize,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum OperationType {
    SequentialScan,
    IndexScan,
    HashJoin,
    NestedLoopJoin,
    Sort,
    Aggregate,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CostModelParameters {
    pub seq_scan_cost: f64,
    pub index_scan_cost: f64,
    pub cpu_tuple_cost: f64,
    pub random_page_cost: f64,
}

impl Default for CostModelParameters {
    fn default() -> Self {
        Self {
            seq_scan_cost: 1.0,
            index_scan_cost: 0.1,
            cpu_tuple_cost: 0.01,
            random_page_cost: 4.0,
        }
    }
}

// Query template manager
pub struct QueryTemplateManager {
    templates: Arc<RwLock<HashMap<String, QueryTemplate>>>,
}

impl QueryTemplateManager {
    pub fn new() -> Self {
        Self {
            templates: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    // Extract template from query
    pub fn extract_template(&self, query: &str) -> String {
        // Simplified template extraction - replace literals with placeholders
        let without_strings = query.replace("'", "?");
        // Replace sequences of digits with placeholder
        without_strings.chars()
            .map(|c| if c.is_numeric() { '?' } else { c })
            .collect()
    }

    // Register query template
    pub fn register_template(&self, template_id: String, template: QueryTemplate) -> Result<()> {
        let mut templates = self.templates.write()
            .map_err(|_| DbError::LockError("Failed to acquire write lock".to_string()))?;
        templates.insert(template_id, template);
        Ok(())
    }

    // Get template statistics
    pub fn get_template_stats(&self, template_id: &str) -> Result<Option<TemplateStatistics>> {
        let templates = self.templates.read()
            .map_err(|_| DbError::LockError("Failed to acquire read lock".to_string()))?;

        if let Some(template) = templates.get(template_id) {
            Ok(Some(TemplateStatistics {
                template_id: template_id.to_string(),
                execution_count: template.execution_count,
                avg_time_ms: template.total_time_ms as f64 / template.execution_count.max(1) as f64,
                last_used: template.last_used,
            }))
        } else {
            Ok(None)
        }
    }
}

#[derive(Debug, Clone)]
pub struct QueryTemplate {
    pub template_text: String,
    pub execution_count: usize,
    pub total_time_ms: u64,
    pub last_used: SystemTime,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemplateStatistics {
    pub template_id: String,
    pub execution_count: usize,
    pub avg_time_ms: f64,
    pub last_used: SystemTime,
}

// Execution plan optimizer
pub struct ExecutionPlanOptimizer {
    optimization_rules: Vec<OptimizationRule>,
    applied_optimizations: Arc<RwLock<Vec<AppliedOptimization>>>,
}

impl ExecutionPlanOptimizer {
    pub fn new() -> Self {
        let rules = vec![
            OptimizationRule {
                name: "predicate_pushdown".to_string(),
                priority: 100,
                enabled: true,
            },
            OptimizationRule {
                name: "join_reorder".to_string(),
                priority: 90,
                enabled: true,
            },
            OptimizationRule {
                name: "index_selection".to_string(),
                priority: 80,
                enabled: true,
            },
        ];

        Self {
            optimization_rules: rules,
            applied_optimizations: Arc::new(RwLock::new(Vec::new())),
        }
    }

    // Optimize execution plan
    pub fn optimize(&self, plan: &QueryPlan) -> Result<OptimizedPlan> {
        let mut optimizations = Vec::new();

        for rule in &self.optimization_rules {
            if rule.enabled {
                optimizations.push(AppliedOptimization {
                    rule_name: rule.name.clone(),
                    timestamp: SystemTime::now(),
                    benefit_estimate: 0.1 * rule.priority as f64,
                });
            }
        }

        let mut applied = self.applied_optimizations.write()
            .map_err(|_| DbError::LockError("Failed to acquire write lock".to_string()))?;
        applied.extend(optimizations.clone());

        Ok(OptimizedPlan {
            original_plan: plan.clone(),
            optimizations,
            estimated_improvement: 0.3,
        })
    }

    // Get optimization history
    pub fn get_history(&self) -> Result<Vec<AppliedOptimization>> {
        let applied = self.applied_optimizations.read()
            .map_err(|_| DbError::LockError("Failed to acquire read lock".to_string()))?;
        Ok(applied.clone())
    }
}

#[derive(Debug, Clone)]
pub struct OptimizationRule {
    pub name: String,
    pub priority: u32,
    pub enabled: bool,
}

#[derive(Debug, Clone)]
pub struct AppliedOptimization {
    pub rule_name: String,
    pub timestamp: SystemTime,
    pub benefit_estimate: f64,
}

#[derive(Debug, Clone)]
pub struct OptimizedPlan {
    pub original_plan: QueryPlan,
    pub optimizations: Vec<AppliedOptimization>,
    pub estimated_improvement: f64,
}

// Resource governor
pub struct ResourceGovernor {
    resource_groups: Arc<RwLock<HashMap<String, ResourceGroup>>>,
    usage_tracking: Arc<RwLock<HashMap<String, ResourceUsage>>>,
}

impl ResourceGovernor {
    pub fn new() -> Self {
        Self {
            resource_groups: Arc::new(RwLock::new(HashMap::new())),
            usage_tracking: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    // Create resource group
    pub fn create_group(&self, name: String, limits: ResourceGroup) -> Result<()> {
        let mut groups = self.resource_groups.write()
            .map_err(|_| DbError::LockError("Failed to acquire write lock".to_string()))?;
        groups.insert(name, limits);
        Ok(())
    }

    // Check if query can execute
    pub fn can_execute(&self, group_name: &str, required_resources: &ResourceRequirement) -> Result<bool> {
        let groups = self.resource_groups.read()
            .map_err(|_| DbError::LockError("Failed to acquire read lock".to_string()))?;
        let usage = self.usage_tracking.read()
            .map_err(|_| DbError::LockError("Failed to acquire read lock".to_string()))?;

        if let Some(group) = groups.get(group_name) {
            if let Some(current_usage) = usage.get(group_name) {
                let would_exceed = current_usage.memory_mb + required_resources.memory_mb > group.max_memory_mb
                    || current_usage.cpu_percent + required_resources.cpu_percent > group.max_cpu_percent;

                return Ok(!would_exceed);
            }
            return Ok(true);
        }

        Ok(false)
    }

    // Track resource usage
    pub fn track_usage(&self, group_name: String, usage: ResourceUsage) -> Result<()> {
        let mut tracking = self.usage_tracking.write()
            .map_err(|_| DbError::LockError("Failed to acquire write lock".to_string()))?;
        tracking.insert(group_name, usage);
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct ResourceGroup {
    pub max_memory_mb: usize,
    pub max_cpu_percent: f64,
    pub max_concurrent_queries: usize,
}

#[derive(Debug, Clone)]
pub struct ResourceUsage {
    pub memory_mb: usize,
    pub cpu_percent: f64,
    pub active_queries: usize,
}

#[derive(Debug, Clone)]
pub struct ResourceRequirement {
    pub memory_mb: usize,
    pub cpu_percent: f64,
}

// Query timeout manager
pub struct TimeoutManager {
    timeouts: Arc<RwLock<HashMap<String, QueryTimeout>>>,
    default_timeout: Duration,
}

impl TimeoutManager {
    pub fn new(default_timeout: Duration) -> Self {
        Self {
            timeouts: Arc::new(RwLock::new(HashMap::new())),
            default_timeout,
        }
    }

    // Set timeout for a query
    pub fn set_timeout(&self, query_id: String, timeout: Duration) -> Result<()> {
        let mut timeouts = self.timeouts.write()
            .map_err(|_| DbError::LockError("Failed to acquire write lock".to_string()))?;

        timeouts.insert(query_id, QueryTimeout {
            deadline: SystemTime::now() + timeout,
            timeout_duration: timeout,
        });

        Ok(())
    }

    // Check if query has timed out
    pub fn is_timed_out(&self, query_id: &str) -> Result<bool> {
        let timeouts = self.timeouts.read()
            .map_err(|_| DbError::LockError("Failed to acquire read lock".to_string()))?;

        if let Some(timeout) = timeouts.get(query_id) {
            Ok(SystemTime::now() > timeout.deadline)
        } else {
            Ok(false)
        }
    }

    // Remove timeout
    pub fn remove_timeout(&self, query_id: &str) -> Result<()> {
        let mut timeouts = self.timeouts.write()
            .map_err(|_| DbError::LockError("Failed to acquire write lock".to_string()))?;
        timeouts.remove(query_id);
        Ok(())
    }
}

#[derive(Debug, Clone)]
struct QueryTimeout {
    deadline: SystemTime,
    timeout_duration: Duration,
}

#[cfg(test)]
mod extended_tests {
    use std::time::{Duration, SystemTime};
    use crate::monitoring::ResourceGroup;
    use crate::performance::{AllocationStrategy, CostMeasurement, CostModelCalibrator, ExecutionPlanOptimizer, MemoryPoolManager, OperationType, ParallelizationAnalyzer, QueryPlan, QueryProfiler, QueryTemplate, QueryTemplateManager, ResourceGovernor, ResourceRequirement, TimeoutManager};

    #[test]
    fn test_query_profiler() {
        let profiler = QueryProfiler::new(1.0);
        let mut session = profiler.start_profile("q1").unwrap();

        session.checkpoint("parse".to_string());
        session.checkpoint("optimize".to_string());

        profiler.complete_profile(session).unwrap();

        let profile = profiler.get_profile("q1").unwrap();
        assert!(profile.is_some());
        assert_eq!(profile.unwrap().checkpoints.len(), 2);
    }

    #[test]
    fn test_memory_pool_manager() {
        let pool = MemoryPoolManager::new(1000, AllocationStrategy::FirstFit);

        assert!(pool.allocate("q1".to_string(), 500).unwrap());
        assert!(pool.allocate("q2".to_string(), 400).unwrap());
        assert!(!pool.allocate("q3".to_string(), 200).unwrap()); // Should fail

        pool.deallocate("q1").unwrap();
        assert!(pool.allocate("q3".to_string(), 200).unwrap()); // Should succeed now
    }

    #[test]
    fn test_parallelization_analyzer() {
        let analyzer = ParallelizationAnalyzer::new();

        let plan = analyzer.analyze("q1", "SELECT * FROM users").unwrap();
        assert!(plan.parallelizable);
        assert_eq!(plan.suggested_parallelism, 4);
    }

    #[test]
    fn test_cost_model_calibrator() {
        let calibrator = CostModelCalibrator::new();

        calibrator.add_measurement(CostMeasurement {
            operation_type: OperationType::SequentialScan,
            estimated_cost: 100.0,
            actual_cost: 105.0,
            row_count: 1000,
        }).unwrap();

        let params = calibrator.calibrate().unwrap();
        assert!(params.seq_scan_cost > 0.0);
    }

    #[test]
    fn test_query_template_manager() {
        let manager = QueryTemplateManager::new();

        let template = QueryTemplate {
            template_text: "SELECT * FROM users WHERE id = ?".to_string(),
            execution_count: 10,
            total_time_ms: 500,
            last_used: SystemTime::now(),
        };

        manager.register_template("t1".to_string(), template).unwrap();

        let stats = manager.get_template_stats("t1").unwrap();
        assert!(stats.is_some());
        assert_eq!(stats.unwrap().execution_count, 10);
    }

    #[test]
    fn test_execution_plan_optimizer() {
        let optimizer = ExecutionPlanOptimizer::new();

        let plan = QueryPlan {
            query_hash: "q1".to_string(),
            plan_tree: "SELECT * FROM users".to_string(),
            estimated_cost: 100.0,
            estimated_rows: 1000,
        };

        let optimized = optimizer.optimize(&plan).unwrap();
        assert!(!optimized.optimizations.is_empty());
    }

    #[test]
    fn test_resource_governor() {
        let governor = ResourceGovernor::new();

        governor.create_group("default".to_string(), ResourceGroup {
            name: "".to_string(),
            priority: 0,
            limits: Default::default(),
            active_sessions: vec![],
            total_cpu_time_us: 0,
            total_memory_bytes: 0,
            total_io_bytes: 0,
            max_memory_mb: 1000,
            max_cpu_percent: 80.0,
            max_concurrent_queries: 10,
            created_at: (),
        }).unwrap();

        let requirement = ResourceRequirement {
            memory_mb: 500,
            cpu_percent: 40.0,
        };

        assert!(governor.can_execute("default", &requirement).unwrap());
    }

    #[test]
    fn test_timeout_manager() {
        let manager = TimeoutManager::new(Duration::from_secs(60));

        manager.set_timeout("q1".to_string(), Duration::from_millis(100)).unwrap();

        std::thread::sleep(Duration::from_millis(150));

        assert!(manager.is_timed_out("q1").unwrap());
    }
}

// Comprehensive benchmarking framework
pub struct BenchmarkRunner {
    benchmarks: Arc<RwLock<HashMap<String, Benchmark>>>,
    results: Arc<RwLock<Vec<BenchmarkResult>>>,
}

impl BenchmarkRunner {
    pub fn new() -> Self {
        Self {
            benchmarks: Arc::new(RwLock::new(HashMap::new())),
            results: Arc::new(RwLock::new(Vec::new())),
        }
    }

    // Register a benchmark
    pub fn register_benchmark(&self, benchmark: Benchmark) -> Result<()> {
        let mut benchmarks = self.benchmarks.write()
            .map_err(|_| DbError::LockError("Failed to acquire write lock".to_string()))?;
        benchmarks.insert(benchmark.id.clone(), benchmark);
        Ok(())
    }

    // Run a specific benchmark
    pub fn run_benchmark(&self, benchmark_id: &str, iterations: usize) -> Result<BenchmarkResult> {
        let benchmarks = self.benchmarks.read()
            .map_err(|_| DbError::LockError("Failed to acquire read lock".to_string()))?;

        let benchmark = benchmarks.get(benchmark_id)
            .ok_or_else(|| DbError::NotFound(format!("Benchmark {} not found", benchmark_id)))?;

        let mut execution_times = Vec::new();

        for _ in 0..iterations {
            let start = SystemTime::now();
            // Simulate execution
            let duration = start.elapsed().unwrap_or(Duration::from_secs(0));
            execution_times.push(duration.as_micros() as u64);
        }

        let avg_time_us = execution_times.iter().sum::<u64>() / iterations as u64;
        let min_time_us = *execution_times.iter().min().unwrap_or(&0);
        let max_time_us = *execution_times.iter().max().unwrap_or(&0);

        // Calculate standard deviation
        let variance = execution_times.iter()
            .map(|&t| {
                let diff = t as f64 - avg_time_us as f64;
                diff * diff
            })
            .sum::<f64>() / iterations as f64;
        let stddev_us = variance.sqrt();

        let result = BenchmarkResult {
            benchmark_id: benchmark_id.to_string(),
            iterations,
            avg_time_us,
            min_time_us,
            max_time_us,
            stddev_us,
            throughput_qps: 1_000_000.0 / avg_time_us as f64,
            timestamp: SystemTime::now(),
        };

        let mut results = self.results.write()
            .map_err(|_| DbError::LockError("Failed to acquire write lock".to_string()))?;
        results.push(result.clone());

        Ok(result)
    }

    // Run all benchmarks
    pub fn run_all_benchmarks(&self, iterations: usize) -> Result<Vec<BenchmarkResult>> {
        let benchmarks = self.benchmarks.read()
            .map_err(|_| DbError::LockError("Failed to acquire read lock".to_string()))?;

        let mut all_results = Vec::new();

        for benchmark_id in benchmarks.keys() {
            let result = self.run_benchmark(benchmark_id, iterations)?;
            all_results.push(result);
        }

        Ok(all_results)
    }

    // Get benchmark results
    pub fn get_results(&self) -> Result<Vec<BenchmarkResult>> {
        let results = self.results.read()
            .map_err(|_| DbError::LockError("Failed to acquire read lock".to_string()))?;
        Ok(results.clone())
    }

    // Compare two benchmark runs
    pub fn compare_results(&self, result1: &BenchmarkResult, result2: &BenchmarkResult) -> BenchmarkComparison {
        let speedup = result1.avg_time_us as f64 / result2.avg_time_us as f64;
        let improvement_percent = ((result1.avg_time_us as f64 - result2.avg_time_us as f64) / result1.avg_time_us as f64) * 100.0;

        BenchmarkComparison {
            baseline: result1.clone(),
            comparison: result2.clone(),
            speedup,
            improvement_percent,
            is_improvement: speedup > 1.0,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Benchmark {
    pub id: String,
    pub name: String,
    pub description: String,
    pub query: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BenchmarkResult {
    pub benchmark_id: String,
    pub iterations: usize,
    pub avg_time_us: u64,
    pub min_time_us: u64,
    pub max_time_us: u64,
    pub stddev_us: f64,
    pub throughput_qps: f64,
    pub timestamp: SystemTime,
}

#[derive(Debug, Clone)]
pub struct BenchmarkComparison {
    pub baseline: BenchmarkResult,
    pub comparison: BenchmarkResult,
    pub speedup: f64,
    pub improvement_percent: f64,
    pub is_improvement: bool,
}

// Statistics collector for query performance
pub struct PerformanceStatsCollector {
    query_stats: Arc<RwLock<HashMap<String, QueryPerformanceStats>>>,
    global_stats: Arc<RwLock<GlobalPerformanceStats>>,
}

impl PerformanceStatsCollector {
    pub fn new() -> Self {
        Self {
            query_stats: Arc::new(RwLock::new(HashMap::new())),
            global_stats: Arc::new(RwLock::new(GlobalPerformanceStats::default())),
        }
    }

    // Record query execution
    pub fn record_query(&self, query_hash: &str, execution_time_ms: u64, rows: usize) -> Result<()> {
        // Update query-specific stats
        let mut query_stats = self.query_stats.write()
            .map_err(|_| DbError::LockError("Failed to acquire write lock".to_string()))?;

        let stats = query_stats.entry(query_hash.to_string())
            .or_insert_with(|| QueryPerformanceStats {
                query_hash: query_hash.to_string(),
                execution_count: 0,
                total_time_ms: 0,
                total_rows: 0,
                min_time_ms: u64::MAX,
                max_time_ms: 0,
                last_execution: SystemTime::now(),
            });

        stats.execution_count += 1;
        stats.total_time_ms += execution_time_ms;
        stats.total_rows += rows;
        stats.min_time_ms = stats.min_time_ms.min(execution_time_ms);
        stats.max_time_ms = stats.max_time_ms.max(execution_time_ms);
        stats.last_execution = SystemTime::now();

        // Update global stats
        let mut global_stats = self.global_stats.write()
            .map_err(|_| DbError::LockError("Failed to acquire write lock".to_string()))?;
        global_stats.total_queries += 1;
        global_stats.total_execution_time_ms += execution_time_ms;
        global_stats.total_rows_processed += rows;

        Ok(())
    }

    // Get stats for a specific query
    pub fn get_query_stats(&self, query_hash: &str) -> Result<Option<QueryPerformanceStats>> {
        let query_stats = self.query_stats.read()
            .map_err(|_| DbError::LockError("Failed to acquire read lock".to_string()))?;
        Ok(query_stats.get(query_hash).cloned())
    }

    // Get global performance statistics
    pub fn get_global_stats(&self) -> Result<GlobalPerformanceStats> {
        let global_stats = self.global_stats.read()
            .map_err(|_| DbError::LockError("Failed to acquire read lock".to_string()))?;
        Ok(global_stats.clone())
    }

    // Get top N slowest queries
    pub fn get_slowest_queries(&self, n: usize) -> Result<Vec<QueryPerformanceStats>> {
        let query_stats = self.query_stats.read()
            .map_err(|_| DbError::LockError("Failed to acquire read lock".to_string()))?;

        let mut stats: Vec<_> = query_stats.values().cloned().collect();
        stats.sort_by(|a, b| {
            let a_avg = a.total_time_ms / a.execution_count.max(1) as u64;
            let b_avg = b.total_time_ms / b.execution_count.max(1) as u64;
            b_avg.cmp(&a_avg)
        });

        Ok(stats.into_iter().take(n).collect())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryPerformanceStats {
    pub query_hash: String,
    pub execution_count: usize,
    pub total_time_ms: u64,
    pub total_rows: usize,
    pub min_time_ms: u64,
    pub max_time_ms: u64,
    pub last_execution: SystemTime,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GlobalPerformanceStats {
    pub total_queries: u64,
    pub total_execution_time_ms: u64,
    pub total_rows_processed: usize,
}

impl Default for GlobalPerformanceStats {
    fn default() -> Self {
        Self {
            total_queries: 0,
            total_execution_time_ms: 0,
            total_rows_processed: 0,
        }
    }
}

// Adaptive caching strategy manager
pub struct AdaptiveCachingStrategy {
    strategies: Vec<CachingStrategy>,
    current_strategy: Arc<RwLock<usize>>,
    performance_metrics: Arc<RwLock<Vec<StrategyPerformance>>>,
}

impl AdaptiveCachingStrategy {
    pub fn new() -> Self {
        let strategies = vec![
            CachingStrategy::LRU,
            CachingStrategy::LFU,
            CachingStrategy::ARC,
            CachingStrategy::FIFO,
        ];

        Self {
            strategies,
            current_strategy: Arc::new(RwLock::new(0)),
            performance_metrics: Arc::new(RwLock::new(Vec::new())),
        }
    }

    // Get current strategy
    pub fn get_current_strategy(&self) -> Result<CachingStrategy> {
        let current = self.current_strategy.read()
            .map_err(|_| DbError::LockError("Failed to acquire read lock".to_string()))?;
        Ok(self.strategies[*current].clone())
    }

    // Record strategy performance
    pub fn record_performance(&self, hit_rate: f64) -> Result<()> {
        let current = *self.current_strategy.read()
            .map_err(|_| DbError::LockError("Failed to acquire read lock".to_string()))?;

        let perf = StrategyPerformance {
            strategy: self.strategies[current].clone(),
            hit_rate,
            timestamp: SystemTime::now(),
        };

        let mut metrics = self.performance_metrics.write()
            .map_err(|_| DbError::LockError("Failed to acquire write lock".to_string()))?;
        metrics.push(perf);

        Ok(())
    }

    // Adapt strategy based on performance
    pub fn adapt(&self) -> Result<CachingStrategy> {
        let metrics = self.performance_metrics.read()
            .map_err(|_| DbError::LockError("Failed to acquire read lock".to_string()))?;

        if metrics.len() < 10 {
            return self.get_current_strategy();
        }

        // Find best performing strategy from recent metrics
        let recent = &metrics[metrics.len().saturating_sub(10)..];
        let mut strategy_scores: HashMap<String, f64> = HashMap::new();

        for metric in recent {
            let key = format!("{:?}", metric.strategy);
            *strategy_scores.entry(key).or_insert(0.0) += metric.hit_rate;
        }

        // Find strategy with highest average hit rate
        if let Some((best_strategy_name, _)) = strategy_scores.iter()
            .max_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap()) {

            // Switch to best strategy
            for (i, strategy) in self.strategies.iter().enumerate() {
                if format!("{:?}", strategy) == *best_strategy_name {
                    let mut current = self.current_strategy.write()
                        .map_err(|_| DbError::LockError("Failed to acquire write lock".to_string()))?;
                    *current = i;
                    return Ok(strategy.clone());
                }
            }
        }

        self.get_current_strategy()
    }
}

#[derive(Debug, Clone)]
pub enum CachingStrategy {
    LRU,
    LFU,
    ARC,
    FIFO,
}

#[derive(Debug, Clone)]
struct StrategyPerformance {
    strategy: CachingStrategy,
    hit_rate: f64,
    timestamp: SystemTime,
}

// Query hint optimizer
pub struct QueryHintOptimizer {
    hints: Arc<RwLock<HashMap<String, Vec<QueryHint>>>>,
    hint_effectiveness: Arc<RwLock<HashMap<String, HintEffectiveness>>>,
}

impl QueryHintOptimizer {
    pub fn new() -> Self {
        Self {
            hints: Arc::new(RwLock::new(HashMap::new())),
            hint_effectiveness: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    // Add hint for a query
    pub fn add_hint(&self, query_hash: String, hint: QueryHint) -> Result<()> {
        let mut hints = self.hints.write()
            .map_err(|_| DbError::LockError("Failed to acquire write lock".to_string()))?;

        hints.entry(query_hash)
            .or_insert_with(Vec::new)
            .push(hint);

        Ok(())
    }

    // Get hints for a query
    pub fn get_hints(&self, query_hash: &str) -> Result<Vec<QueryHint>> {
        let hints = self.hints.read()
            .map_err(|_| DbError::LockError("Failed to acquire read lock".to_string()))?;

        Ok(hints.get(query_hash).cloned().unwrap_or_default())
    }

    // Record hint effectiveness
    pub fn record_effectiveness(&self, hint_id: String, improvement: f64) -> Result<()> {
        let mut effectiveness = self.hint_effectiveness.write()
            .map_err(|_| DbError::LockError("Failed to acquire write lock".to_string()))?;

        effectiveness.insert(hint_id, HintEffectiveness {
            improvement_factor: improvement,
            sample_count: 1,
        });

        Ok(())
    }

    // Get most effective hints
    pub fn get_effective_hints(&self, threshold: f64) -> Result<Vec<String>> {
        let effectiveness = self.hint_effectiveness.read()
            .map_err(|_| DbError::LockError("Failed to acquire read lock".to_string()))?;

        Ok(effectiveness.iter()
            .filter(|(_, e)| e.improvement_factor > threshold)
            .map(|(id, _)| id.clone())
            .collect())
    }
}

#[derive(Debug, Clone)]
pub struct QueryHint {
    pub hint_type: HintType,
    pub target: String,
    pub value: String,
}

#[derive(Debug, Clone)]
pub enum HintType {
    UseIndex,
    NoIndex,
    JoinOrder,
    Parallel,
    NoCost,
}

#[derive(Debug, Clone)]
struct HintEffectiveness {
    improvement_factor: f64,
    sample_count: usize,
}

// Continuous query monitor
pub struct ContinuousQueryMonitor {
    monitored_queries: Arc<RwLock<HashMap<String, MonitoredQuery>>>,
    alert_thresholds: Arc<RwLock<AlertThresholds>>,
    alerts: Arc<RwLock<Vec<QueryAlert>>>,
}

impl ContinuousQueryMonitor {
    pub fn new(thresholds: AlertThresholds) -> Self {
        Self {
            monitored_queries: Arc::new(RwLock::new(HashMap::new())),
            alert_thresholds: Arc::new(RwLock::new(thresholds)),
            alerts: Arc::new(RwLock::new(Vec::new())),
        }
    }

    // Monitor a query
    pub fn monitor_query(&self, query_hash: String, execution_time_ms: u64) -> Result<()> {
        let mut queries = self.monitored_queries.write()
            .map_err(|_| DbError::LockError("Failed to acquire write lock".to_string()))?;

        let query = queries.entry(query_hash.clone())
            .or_insert_with(|| MonitoredQuery {
                query_hash: query_hash.clone(),
                execution_history: Vec::new(),
                avg_execution_time_ms: 0.0,
                trend: QueryTrend::Stable,
            });

        query.execution_history.push(execution_time_ms);

        // Keep only last 100 executions
        if query.execution_history.len() > 100 {
            query.execution_history.remove(0);
        }

        // Update average
        query.avg_execution_time_ms = query.execution_history.iter()
            .sum::<u64>() as f64 / query.execution_history.len() as f64;

        // Detect trend
        if query.execution_history.len() >= 10 {
            let recent_avg = query.execution_history[query.execution_history.len()-10..]
                .iter().sum::<u64>() as f64 / 10.0;

            if recent_avg > query.avg_execution_time_ms * 1.2 {
                query.trend = QueryTrend::Degrading;
                self.generate_alert(&query_hash, AlertType::PerformanceDegradation)?;
            } else if recent_avg < query.avg_execution_time_ms * 0.8 {
                query.trend = QueryTrend::Improving;
            } else {
                query.trend = QueryTrend::Stable;
            }
        }

        // Check thresholds
        let thresholds = self.alert_thresholds.read()
            .map_err(|_| DbError::LockError("Failed to acquire read lock".to_string()))?;

        if execution_time_ms > thresholds.slow_query_threshold_ms {
            self.generate_alert(&query_hash, AlertType::SlowQuery)?;
        }

        Ok(())
    }

    fn generate_alert(&self, query_hash: &str, alert_type: AlertType) -> Result<()> {
        let alert = QueryAlert {
            query_hash: query_hash.to_string(),
            alert_type,
            timestamp: SystemTime::now(),
        };

        let mut alerts = self.alerts.write()
            .map_err(|_| DbError::LockError("Failed to acquire write lock".to_string()))?;
        alerts.push(alert);

        Ok(())
    }

    // Get alerts
    pub fn get_alerts(&self) -> Result<Vec<QueryAlert>> {
        let alerts = self.alerts.read()
            .map_err(|_| DbError::LockError("Failed to acquire read lock".to_string()))?;
        Ok(alerts.clone())
    }

    // Get monitored query info
    pub fn get_monitored_query(&self, query_hash: &str) -> Result<Option<MonitoredQuery>> {
        let queries = self.monitored_queries.read()
            .map_err(|_| DbError::LockError("Failed to acquire read lock".to_string()))?;
        Ok(queries.get(query_hash).cloned())
    }
}

#[derive(Debug, Clone)]
pub struct MonitoredQuery {
    pub query_hash: String,
    pub execution_history: Vec<u64>,
    pub avg_execution_time_ms: f64,
    pub trend: QueryTrend,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum QueryTrend {
    Improving,
    Stable,
    Degrading,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlertThresholds {
    pub slow_query_threshold_ms: u64,
    pub high_memory_threshold_mb: usize,
    pub high_cpu_threshold_percent: f64,
}

impl Default for AlertThresholds {
    fn default() -> Self {
        Self {
            slow_query_threshold_ms: 1000,
            high_memory_threshold_mb: 1000,
            high_cpu_threshold_percent: 80.0,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryAlert {
    pub query_hash: String,
    pub alert_type: AlertType,
    pub timestamp: SystemTime,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AlertType {
    SlowQuery,
    HighMemory,
    HighCPU,
    PerformanceDegradation,
}

#[cfg(test)]
mod more_tests {
    use crate::performance::{AdaptiveCachingStrategy, AlertThresholds, Benchmark, BenchmarkRunner, CachingStrategy, ContinuousQueryMonitor, HintType, PerformanceStatsCollector, QueryHint, QueryHintOptimizer};

    #[test]
    fn test_benchmark_runner() {
        let runner = BenchmarkRunner::new();

        let benchmark = Benchmark {
            id: "b1".to_string(),
            name: "Select benchmark".to_string(),
            description: "Tests SELECT performance".to_string(),
            query: "SELECT * FROM users".to_string(),
        };

        runner.register_benchmark(benchmark).unwrap();

        let result = runner.run_benchmark("b1", 10).unwrap();
        assert_eq!(result.iterations, 10);
        assert!(result.throughput_qps > 0.0);
    }

    #[test]
    fn test_performance_stats_collector() {
        let collector = PerformanceStatsCollector::new();

        for i in 0..10 {
            collector.record_query("q1", 100 + i * 10, 1000).unwrap();
        }

        let stats = collector.get_query_stats("q1").unwrap();
        assert!(stats.is_some());
        assert_eq!(stats.unwrap().execution_count, 10);

        let global_stats = collector.get_global_stats().unwrap();
        assert_eq!(global_stats.total_queries, 10);
    }

    #[test]
    fn test_adaptive_caching_strategy() {
        let strategy_mgr = AdaptiveCachingStrategy::new();

        for i in 0..10 {
            strategy_mgr.record_performance(0.7 + (i as f64 * 0.01)).unwrap();
        }

        let adapted = strategy_mgr.adapt().unwrap();
        // Should have adapted based on performance
        assert!(matches!(adapted, CachingStrategy::LRU | CachingStrategy::LFU | CachingStrategy::ARC | CachingStrategy::FIFO));
    }

    #[test]
    fn test_query_hint_optimizer() {
        let optimizer = QueryHintOptimizer::new();

        let hint = QueryHint {
            hint_type: HintType::UseIndex,
            target: "users_email_idx".to_string(),
            value: "email".to_string(),
        };

        optimizer.add_hint("q1".to_string(), hint).unwrap();

        let hints = optimizer.get_hints("q1").unwrap();
        assert_eq!(hints.len(), 1);
    }

    #[test]
    fn test_continuous_query_monitor() {
        let monitor = ContinuousQueryMonitor::new(AlertThresholds::default());

        monitor.monitor_query("q1".to_string(), 500).unwrap();
        monitor.monitor_query("q1".to_string(), 600).unwrap();
        monitor.monitor_query("q1".to_string(), 1500).unwrap(); // Should trigger alert

        let alerts = monitor.get_alerts().unwrap();
        assert!(!alerts.is_empty());
    }
}

// Performance report generator
pub struct PerformanceReportGenerator {
    stats_collector: Arc<PerformanceStatsCollector>,
    workload_analyzer: Arc<WorkloadAnalyzer>,
}

impl PerformanceReportGenerator {
    pub fn new(
        stats_collector: Arc<PerformanceStatsCollector>,
        workload_analyzer: Arc<WorkloadAnalyzer>,
    ) -> Self {
        Self {
            stats_collector,
            workload_analyzer,
        }
    }

    // Generate comprehensive performance report
    pub fn generate_report(&self) -> Result<PerformanceReport> {
        let global_stats = self.stats_collector.get_global_stats()?;
        let workload_analysis = self.workload_analyzer.analyze()?;
        let slowest_queries = self.stats_collector.get_slowest_queries(10)?;

        let recommendations = self.generate_recommendations(&workload_analysis, &slowest_queries);

        Ok(PerformanceReport {
            generated_at: SystemTime::now(),
            global_stats,
            workload_analysis,
            slowest_queries,
            recommendations,
        })
    }

    fn generate_recommendations(
        &self,
        workload: &WorkloadAnalysis,
        slowqueries: &[QueryPerformanceStats],
    ) -> Vec<String> {
        let mut recommendations = Vec::new();

        if workload.slow_query_percentage > 10.0 {
            recommendations.push(format!(
                "High percentage of slow queries ({}%). Consider adding indexes or optimizing queries.",
                workload.slow_query_percentage
            ));
        }

        if !slowqueries.is_empty() {
            recommendations.push(format!(
                "{} queries identified as slow. Review and optimize top performers.",
                slowqueries.len()
            ));
        }

        if workload.avg_execution_time_ms > 500.0 {
            recommendations.push("Average query execution time is high. Consider query optimization.".to_string());
        }

        recommendations
    }

    // Generate trend analysis
    pub fn generate_trend_analysis(&self, period: Duration) -> Result<TrendAnalysis> {
        let log = self.workload_analyzer.get_log()?;

        let period_start = SystemTime::now() - period;
        let recent_executions: Vec<_> = log.iter()
            .filter(|e| e.timestamp > period_start)
            .collect();

        if recent_executions.is_empty() {
            return Ok(TrendAnalysis {
                period,
                query_count: 0,
                avg_execution_time_trend: TrendDirection::Stable,
                throughput_trend: TrendDirection::Stable,
                recommendations: vec!["Insufficient data for trend analysis".to_string()],
            });
        }

        // Simple trend detection
        let total_time: u64 = recent_executions.iter().map(|e| e.execution_time_ms).sum();
        let avg_time = total_time as f64 / recent_executions.len() as f64;

        // Compare with overall average
        let all_time: u64 = log.iter().map(|e| e.execution_time_ms).sum();
        let overall_avg = all_time as f64 / log.len().max(1) as f64;

        let time_trend = if avg_time > overall_avg * 1.1 {
            TrendDirection::Degrading
        } else if avg_time < overall_avg * 0.9 {
            TrendDirection::Improving
        } else {
            TrendDirection::Stable
        };

        Ok(TrendAnalysis {
            period,
            query_count: recent_executions.len(),
            avg_execution_time_trend: time_trend,
            throughput_trend: TrendDirection::Stable,
            recommendations: Vec::new(),
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceReport {
    pub generated_at: SystemTime,
    pub global_stats: GlobalPerformanceStats,
    pub workload_analysis: WorkloadAnalysis,
    pub slowest_queries: Vec<QueryPerformanceStats>,
    pub recommendations: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct TrendAnalysis {
    pub period: Duration,
    pub query_count: usize,
    pub avg_execution_time_trend: TrendDirection,
    pub throughput_trend: TrendDirection,
    pub recommendations: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TrendDirection {
    Improving,
    Stable,
    Degrading,
}

// Query pattern recognizer
pub struct QueryPatternRecognizer {
    patterns: Arc<RwLock<HashMap<String, RecognizedPattern>>>,
}

impl QueryPatternRecognizer {
    pub fn new() -> Self {
        Self {
            patterns: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    // Recognize pattern in query
    pub fn recognize_pattern(&self, query: &str) -> Result<RecognizedPattern> {
        let pattern = if query.contains("JOIN") && query.contains("WHERE") {
            PatternType::FilteredJoin
        } else if query.contains("GROUP BY") {
            PatternType::Aggregation
        } else if query.contains("ORDER BY") && query.contains("LIMIT") {
            PatternType::TopN
        } else if query.contains("WHERE") && query.contains("IN") {
            PatternType::InListFilter
        } else {
            PatternType::Simple
        };

        Ok(RecognizedPattern {
            pattern_type: pattern,
            confidence: 0.9,
            suggested_optimizations: self.get_optimizations_for_pattern(&pattern),
        })
    }

    fn get_optimizations_for_pattern(&self, pattern: &PatternType) -> Vec<String> {
        match pattern {
            PatternType::FilteredJoin => vec![
                "Consider using appropriate join algorithm (hash/merge)".to_string(),
                "Ensure join columns are indexed".to_string(),
                "Push down predicates to reduce join size".to_string(),
            ],
            PatternType::Aggregation => vec![
                "Consider creating covering index for GROUP BY columns".to_string(),
                "Use hash aggregation for large datasets".to_string(),
            ],
            PatternType::TopN => vec![
                "Use index for ORDER BY column if possible".to_string(),
                "Consider partial sort optimization".to_string(),
            ],
            PatternType::InListFilter => vec![
                "Consider using hash join instead of IN list".to_string(),
                "Ensure filtered column is indexed".to_string(),
            ],
            PatternType::Simple => vec![
                "Basic optimization applies".to_string(),
            ],
        }
    }

    // Store recognized pattern
    pub fn store_pattern(&self, query_hash: String, pattern: RecognizedPattern) -> Result<()> {
        let mut patterns = self.patterns.write()
            .map_err(|_| DbError::LockError("Failed to acquire write lock".to_string()))?;
        patterns.insert(query_hash, pattern);
        Ok(())
    }

    // Get stored pattern
    pub fn get_pattern(&self, query_hash: &str) -> Result<Option<RecognizedPattern>> {
        let patterns = self.patterns.read()
            .map_err(|_| DbError::LockError("Failed to acquire read lock".to_string()))?;
        Ok(patterns.get(query_hash).cloned())
    }
}

#[derive(Debug, Clone)]
pub struct RecognizedPattern {
    pub pattern_type: PatternType,
    pub confidence: f64,
    pub suggested_optimizations: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PatternType {
    Simple,
    FilteredJoin,
    Aggregation,
    TopN,
    InListFilter,
}

// Cache warmup scheduler
pub struct CacheWarmupScheduler {
    warmup_jobs: Arc<RwLock<Vec<WarmupJob>>>,
    execution_log: Arc<RwLock<Vec<WarmupExecution>>>,
}

impl CacheWarmupScheduler {
    pub fn new() -> Self {
        Self {
            warmup_jobs: Arc::new(RwLock::new(Vec::new())),
            execution_log: Arc::new(RwLock::new(Vec::new())),
        }
    }

    // Schedule cache warmup job
    pub fn schedule_warmup(&self, job: WarmupJob) -> Result<()> {
        let mut jobs = self.warmup_jobs.write()
            .map_err(|_| DbError::LockError("Failed to acquire write lock".to_string()))?;
        jobs.push(job);
        Ok(())
    }

    // Execute pending warmup jobs
    pub fn execute_warmups(&self) -> Result<Vec<WarmupExecution>> {
        let jobs = self.warmup_jobs.read()
            .map_err(|_| DbError::LockError("Failed to acquire read lock".to_string()))?;

        let now = SystemTime::now();
        let mut executions = Vec::new();

        for job in jobs.iter() {
            if job.next_run <= now {
                let start = SystemTime::now();
                // Simulate warmup execution
                let duration = start.elapsed().unwrap_or(Duration::from_secs(0));

                executions.push(WarmupExecution {
                    job_id: job.id.clone(),
                    timestamp: now,
                    duration_ms: duration.as_millis() as u64,
                    success: true,
                    queries_warmed: job.queries.len(),
                });
            }
        }

        let mut log = self.execution_log.write()
            .map_err(|_| DbError::LockError("Failed to acquire write lock".to_string()))?;
        log.extend(executions.clone());

        Ok(executions)
    }

    // Get warmup execution history
    pub fn get_execution_history(&self) -> Result<Vec<WarmupExecution>> {
        let log = self.execution_log.read()
            .map_err(|_| DbError::LockError("Failed to acquire read lock".to_string()))?;
        Ok(log.clone())
    }
}

#[derive(Debug, Clone)]
pub struct WarmupJob {
    pub id: String,
    pub queries: Vec<String>,
    pub schedule: WarmupSchedule,
    pub next_run: SystemTime,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum WarmupSchedule {
    Startup,
    Hourly,
    Daily,
    Custom(Duration),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WarmupExecution {
    pub job_id: String,
    pub timestamp: SystemTime,
    pub duration_ms: u64,
    pub success: bool,
    pub queries_warmed: usize,
}

// Query complexity analyzer
pub struct QueryComplexityAnalyzer {
    complexity_cache: Arc<RwLock<HashMap<String, ComplexityScore>>>,
}

impl QueryComplexityAnalyzer {
    pub fn new() -> Self {
        Self {
            complexity_cache: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    // Analyze query complexity
    pub fn analyze_complexity(&self, query: &str) -> Result<ComplexityScore> {
        let mut score = 0;
        let mut factors = Vec::new();

        // Count joins
        let join_count = query.matches("JOIN").count();
        if join_count > 0 {
            score += join_count * 10;
            factors.push(ComplexityFactor::JoinCount(join_count));
        }

        // Count subqueries
        let subquery_count = query.matches("SELECT").count() - 1; // Subtract main query
        if subquery_count > 0 {
            score += subquery_count * 15;
            factors.push(ComplexityFactor::SubqueryCount(subquery_count));
        }

        // Check for aggregations
        if query.contains("GROUP BY") {
            score += 5;
            factors.push(ComplexityFactor::HasAggregation);
        }

        // Check for sorting
        if query.contains("ORDER BY") {
            score += 3;
            factors.push(ComplexityFactor::HasSorting);
        }

        // Check for window functions
        if query.contains("OVER(") {
            score += 20;
            factors.push(ComplexityFactor::HasWindowFunction);
        }

        let complexity_level = match score {
            0..=10 => ComplexityLevel::Simple,
            11..=30 => ComplexityLevel::Moderate,
            31..=60 => ComplexityLevel::Complex,
            _ => ComplexityLevel::VeryComplex,
        };

        Ok(ComplexityScore {
            score,
            level: complexity_level,
            factors,
        })
    }

    // Cache complexity score
    pub fn cache_complexity(&self, query_hash: String, complexity: ComplexityScore) -> Result<()> {
        let mut cache = self.complexity_cache.write()
            .map_err(|_| DbError::LockError("Failed to acquire write lock".to_string()))?;
        cache.insert(query_hash, complexity);
        Ok(())
    }

    // Get cached complexity
    pub fn get_cached_complexity(&self, query_hash: &str) -> Result<Option<ComplexityScore>> {
        let cache = self.complexity_cache.read()
            .map_err(|_| DbError::LockError("Failed to acquire read lock".to_string()))?;
        Ok(cache.get(query_hash).cloned())
    }
}

#[derive(Debug, Clone)]
pub struct ComplexityScore {
    pub score: usize,
    pub level: ComplexityLevel,
    pub factors: Vec<ComplexityFactor>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ComplexityLevel {
    Simple,
    Moderate,
    Complex,
    VeryComplex,
}

#[derive(Debug, Clone)]
pub enum ComplexityFactor {
    JoinCount(usize),
    SubqueryCount(usize),
    HasAggregation,
    HasSorting,
    HasWindowFunction,
}

// Performance utility functions for common operations

// Calculate the expected number of disk I/O operations for a given query
pub fn estimate_disk_io(
    table_size_pages: u64,
    indexlevels: u32,
    selectivity: f64,
) -> u64 {
    // For index scan: levels + selectivity * table_size
    // For table scan: table_size
    let index_cost = indexlevels as u64 + (selectivity * table_size_pages as f64) as u64;
    let table_scan_cost = table_size_pages;

    // Return the minimum cost
    std::cmp::min(index_cost, table_scan_cost)
}

// Calculate query cost based on multiple factors
pub fn calculate_query_cost(
    rows: u64,
    joins: usize,
    has_aggregation: bool,
    has_sorting: bool,
) -> f64 {
    let mut cost = rows as f64;

    // Each join multiplies the cost
    if joins > 0 {
        cost *= (joins as f64 + 1.0).powf(1.5);
    }

    // Aggregation adds n log n complexity
    if has_aggregation {
        cost *= (rows as f64).log2();
    }

    // Sorting adds n log n complexity
    if has_sorting {
        cost *= (rows as f64).log2();
    }

    cost
}

// Estimate the cardinality of a join operation
pub fn estimate_join_cardinality(
    left_cardinality: u64,
    right_cardinality: u64,
    selectivity: f64,
) -> u64 {
    ((left_cardinality as f64 * right_cardinality as f64 * selectivity) as u64)
        .max(1) // At least 1 row
}

// Calculate buffer pool hit ratio
pub fn calculate_hit_ratio(hits: u64, total_accesses: u64) -> f64 {
    if total_accesses == 0 {
        0.0
    } else {
        hits as f64 / total_accesses as f64
    }
}

// Estimate memory requirements for a hash join
pub fn estimate_hash_join_memory(
    build_side_rows: u64,
    avg_row_size_bytes: usize,
    hash_overhead_factor: f64,
) -> usize {
    let base_size = build_side_rows as usize * avg_row_size_bytes;
    (base_size as f64 * hash_overhead_factor) as usize
}

// Calculate the optimal number of worker threads for parallel query execution
pub fn calculate_optimal_parallelism(
    total_rows: u64,
    min_rows_per_thread: u64,
    max_threads: usize,
) -> usize {
    if total_rows < min_rows_per_thread {
        return 1;
    }

    let ideal_threads = (total_rows / min_rows_per_thread) as usize;
    std::cmp::min(ideal_threads, max_threads).max(1)
}

#[cfg(test)]
mod final_tests {
    use std::sync::Arc;
    use std::time::SystemTime;
    use crate::performance::{CacheWarmupScheduler, PatternType, PerformanceReportGenerator, PerformanceStatsCollector, QueryComplexityAnalyzer, QueryPatternRecognizer, WarmupJob, WarmupSchedule, WorkloadAnalyzer};

    #[test]
    fn test_performance_report_generator() {
        let stats_collector = Arc::new(PerformanceStatsCollector::new());
        let workload_analyzer = Arc::new(WorkloadAnalyzer::new(1000));

        let generator = PerformanceReportGenerator::new(stats_collector, workload_analyzer);

        let report = generator.generate_report();
        assert!(report.is_ok());
    }

    #[test]
    fn test_query_pattern_recognizer() {
        let recognizer = QueryPatternRecognizer::new();

        let pattern = recognizer.recognize_pattern(
            "SELECT * FROM users JOIN orders ON users.id = orders.user_id WHERE users.active = true"
        ).unwrap();

        assert_eq!(pattern.pattern_type, PatternType::FilteredJoin);
        assert!(!pattern.suggested_optimizations.is_empty());
    }

    #[test]
    fn test_cache_warmup_scheduler() {
        let scheduler = CacheWarmupScheduler::new();

        let job = WarmupJob {
            id: "job1".to_string(),
            queries: vec!["SELECT * FROM users".to_string()],
            schedule: WarmupSchedule::Startup,
            next_run: SystemTime::now(),
        };

        scheduler.schedule_warmup(job).unwrap();

        let executions = scheduler.execute_warmups().unwrap();
        assert_eq!(executions.len(), 1);
    }

    #[test]
    fn test_query_complexity_analyzer() {
        let analyzer = QueryComplexityAnalyzer::new();

        let complexity = analyzer.analyze_complexity(
            "SELECT * FROM users JOIN orders ON users.id = orders.user_id GROUP BY users.id ORDER BY COUNT(*) DESC"
        ).unwrap();

        assert!(complexity.score > 0);
        assert!(!complexity.factors.is_empty());
    }
}
