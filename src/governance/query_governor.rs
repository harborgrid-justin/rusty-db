// # Query Governor - Core Governance Engine
//
// This module implements the main query governance engine with resource limits,
// query complexity analysis, throttling, queuing, and cancellation capabilities.

use crate::error::{DbError, Result};
use crate::common::*;
use super::resource_quotas::{QuotaManager, QueryQuota};
use super::governance_policies::{PolicyEngine, ViolationType};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, VecDeque};
use std::sync::{Arc, RwLock, Mutex};
use std::time::{Duration, Instant, SystemTime};

// ============================================================================
// Query Metadata and State
// ============================================================================

/// Unique identifier for queries
pub type QueryId = String;

/// Query execution state
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum QueryState {
    /// Query is queued waiting for resources
    Queued,
    /// Query is currently running
    Running,
    /// Query completed successfully
    Completed,
    /// Query was cancelled
    Cancelled,
    /// Query failed
    Failed,
    /// Query exceeded resource limits
    Throttled,
}

/// Query metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryMetadata {
    /// Query identifier
    pub query_id: QueryId,
    /// User identifier
    pub user_id: String,
    /// Session identifier
    pub session_id: SessionId,
    /// SQL query text
    pub query_text: String,
    /// Query state
    pub state: QueryState,
    /// Query complexity score
    pub complexity_score: u64,
    /// Estimated CPU time (milliseconds)
    pub estimated_cpu_ms: u64,
    /// Estimated memory usage (bytes)
    pub estimated_memory_bytes: u64,
    /// Actual CPU time used (milliseconds)
    pub actual_cpu_ms: u64,
    /// Actual memory used (bytes)
    pub actual_memory_bytes: u64,
    /// Query start time
    #[serde(skip)]
    pub start_time: Option<Instant>,
    /// Query end time
    #[serde(skip)]
    pub end_time: Option<Instant>,
    /// Submission timestamp
    pub submitted_at: SystemTime,
    /// Priority (higher = more important)
    pub priority: u32,
}

impl QueryMetadata {
    /// Create new query metadata
    pub fn new(
        query_id: QueryId,
        user_id: String,
        session_id: SessionId,
        query_text: String,
    ) -> Self {
        Self {
            query_id,
            user_id,
            session_id,
            query_text,
            state: QueryState::Queued,
            complexity_score: 0,
            estimated_cpu_ms: 0,
            estimated_memory_bytes: 0,
            actual_cpu_ms: 0,
            actual_memory_bytes: 0,
            start_time: None,
            end_time: None,
            submitted_at: SystemTime::now(),
            priority: 0,
        }
    }

    /// Get execution duration if query has started
    pub fn execution_duration(&self) -> Option<Duration> {
        if let Some(start) = self.start_time {
            let end = self.end_time.unwrap_or_else(Instant::now);
            Some(end.duration_since(start))
        } else {
            None
        }
    }
}

// ============================================================================
// Query Complexity Analysis
// ============================================================================

/// Query complexity analyzer
pub struct ComplexityAnalyzer {
    /// Base complexity for different operations
    operation_weights: HashMap<String, u64>,
}

impl ComplexityAnalyzer {
    /// Create a new complexity analyzer
    pub fn new() -> Self {
        let mut operation_weights = HashMap::new();

        // Basic operation weights
        operation_weights.insert("SELECT".to_string(), 10);
        operation_weights.insert("INSERT".to_string(), 20);
        operation_weights.insert("UPDATE".to_string(), 30);
        operation_weights.insert("DELETE".to_string(), 30);
        operation_weights.insert("JOIN".to_string(), 50);
        operation_weights.insert("SUBQUERY".to_string(), 100);
        operation_weights.insert("AGGREGATE".to_string(), 40);
        operation_weights.insert("SORT".to_string(), 60);
        operation_weights.insert("GROUP_BY".to_string(), 50);
        operation_weights.insert("WINDOW".to_string(), 80);
        operation_weights.insert("RECURSIVE_CTE".to_string(), 150);

        Self { operation_weights }
    }

    /// Analyze query complexity (simplified heuristic-based analysis)
    pub fn analyze(&self, query_text: &str) -> u64 {
        let query_upper = query_text.to_uppercase();
        let mut complexity = 0u64;

        // Count different SQL constructs
        for (operation, weight) in &self.operation_weights {
            let count = query_upper.matches(operation.as_str()).count() as u64;
            complexity = complexity.saturating_add(count.saturating_mul(*weight));
        }

        // Additional complexity factors

        // Cartesian products (multiple FROMs without JOIN)
        let from_count = query_upper.matches("FROM").count();
        if from_count > 1 && !query_upper.contains("JOIN") {
            complexity = complexity.saturating_add(200 * (from_count as u64 - 1));
        }

        // Nested subqueries
        let subquery_depth = self.count_nesting_depth(&query_upper, '(', ')');
        complexity = complexity.saturating_add(50 * subquery_depth);

        // Wildcards in predicates
        if query_upper.contains("LIKE '%") || query_upper.contains("LIKE \"%") {
            complexity = complexity.saturating_add(100);
        }

        // No WHERE clause on large operations
        if (query_upper.contains("DELETE") || query_upper.contains("UPDATE"))
            && !query_upper.contains("WHERE")
        {
            complexity = complexity.saturating_add(500);
        }

        complexity
    }

    /// Estimate resource requirements based on complexity
    pub fn estimate_resources(&self, complexity: u64) -> (u64, u64) {
        // Very simple estimation (in production, this would use query optimizer stats)
        let estimated_cpu_ms = complexity.saturating_mul(10);
        let estimated_memory_bytes = complexity.saturating_mul(1024 * 100); // 100KB per complexity point

        (estimated_cpu_ms, estimated_memory_bytes)
    }

    /// Count maximum nesting depth
    fn count_nesting_depth(&self, text: &str, open: char, close: char) -> u64 {
        let mut max_depth: u64 = 0;
        let mut current_depth: u64 = 0;

        for ch in text.chars() {
            if ch == open {
                current_depth += 1;
                max_depth = max_depth.max(current_depth);
            } else if ch == close {
                current_depth = current_depth.saturating_sub(1u64);
            }
        }

        max_depth
    }
}

impl Default for ComplexityAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Query Queue
// ============================================================================

/// Query queue for throttling
struct QueryQueue {
    /// Queued queries (FIFO with priority)
    queue: VecDeque<QueryMetadata>,
    /// Maximum queue size
    max_queue_size: usize,
}

impl QueryQueue {
    fn new(max_queue_size: usize) -> Self {
        Self {
            queue: VecDeque::new(),
            max_queue_size,
        }
    }

    /// Add query to queue
    fn enqueue(&mut self, query: QueryMetadata) -> Result<()> {
        if self.queue.len() >= self.max_queue_size {
            return Err(DbError::ResourceExhausted(
                "Query queue is full".to_string(),
            ));
        }

        self.queue.push_back(query);
        self.sort_by_priority();
        Ok(())
    }

    /// Remove and return next query
    fn dequeue(&mut self) -> Option<QueryMetadata> {
        self.queue.pop_front()
    }

    /// Sort queue by priority (descending)
    fn sort_by_priority(&mut self) {
        let mut items: Vec<_> = self.queue.drain(..).collect();
        items.sort_by(|a, b| b.priority.cmp(&a.priority));
        self.queue = items.into_iter().collect();
    }

    /// Get queue length
    fn len(&self) -> usize {
        self.queue.len()
    }

    /// Remove a specific query by ID
    fn remove(&mut self, query_id: &str) -> Option<QueryMetadata> {
        if let Some(pos) = self.queue.iter().position(|q| q.query_id == query_id) {
            self.queue.remove(pos)
        } else {
            None
        }
    }
}

// ============================================================================
// Query Governor
// ============================================================================

/// Main query governor
pub struct QueryGovernor {
    /// Quota manager
    quota_manager: Arc<QuotaManager>,
    /// Policy engine
    policy_engine: Arc<PolicyEngine>,
    /// Complexity analyzer
    complexity_analyzer: Arc<ComplexityAnalyzer>,
    /// Active queries indexed by query ID
    active_queries: Arc<RwLock<HashMap<QueryId, QueryMetadata>>>,
    /// Query queue for throttling
    query_queue: Arc<Mutex<QueryQueue>>,
    /// Maximum concurrent queries globally
    max_concurrent_queries: u32,
    /// Query history (limited size)
    query_history: Arc<RwLock<VecDeque<QueryMetadata>>>,
    /// Maximum history size
    max_history_size: usize,
}

impl QueryGovernor {
    /// Create a new query governor
    pub fn new(
        quota_manager: Arc<QuotaManager>,
        policy_engine: Arc<PolicyEngine>,
    ) -> Self {
        Self {
            quota_manager,
            policy_engine,
            complexity_analyzer: Arc::new(ComplexityAnalyzer::new()),
            active_queries: Arc::new(RwLock::new(HashMap::new())),
            query_queue: Arc::new(Mutex::new(QueryQueue::new(1000))),
            max_concurrent_queries: 100,
            query_history: Arc::new(RwLock::new(VecDeque::new())),
            max_history_size: 10000,
        }
    }

    /// Submit a query for execution
    pub fn submit_query(
        &self,
        user_id: String,
        session_id: SessionId,
        query_text: String,
        user_roles: Vec<String>,
    ) -> Result<QueryId> {
        let query_id = format!("qry_{}", uuid::Uuid::new_v4());

        // Create query metadata
        let mut query = QueryMetadata::new(
            query_id.clone(),
            user_id.clone(),
            session_id,
            query_text.clone(),
        );

        // Analyze query complexity
        let complexity = self.complexity_analyzer.analyze(&query_text);
        query.complexity_score = complexity;

        let (est_cpu, est_mem) = self.complexity_analyzer.estimate_resources(complexity);
        query.estimated_cpu_ms = est_cpu;
        query.estimated_memory_bytes = est_mem;

        // Check against policy limits
        let limits = self.policy_engine.get_effective_limits(&user_id, &user_roles)?;

        if let Some(max_complexity) = limits.max_complexity_score {
            if complexity > max_complexity {
                self.policy_engine.record_violation(
                    "complexity_limit".to_string(),
                    user_id.clone(),
                    session_id,
                    ViolationType::ComplexityExceeded,
                    format!("Query complexity {} exceeds limit {}", complexity, max_complexity),
                )?;

                return Err(DbError::LimitExceeded(format!(
                    "Query complexity {} exceeds limit {}",
                    complexity, max_complexity
                )));
            }
        }

        // Check if we can start the query immediately
        let active_count = {
            let active = self.active_queries.read()
                .map_err(|e| DbError::LockError(format!("Failed to acquire read lock: {}", e)))?;
            active.len()
        };

        if active_count < self.max_concurrent_queries as usize {
            // Can start immediately
            self.start_query(query)?;
        } else {
            // Need to queue
            let mut queue = self.query_queue.lock()
                .map_err(|e| DbError::LockError(format!("Failed to acquire lock: {}", e)))?;
            queue.enqueue(query)?;
        }

        Ok(query_id)
    }

    /// Start executing a query
    fn start_query(&self, mut query: QueryMetadata) -> Result<()> {
        // Check quotas
        self.quota_manager.check_query_start(&query.user_id, query.session_id)?;

        // Mark as running
        query.state = QueryState::Running;
        query.start_time = Some(Instant::now());

        // Add to active queries
        let mut active = self.active_queries.write()
            .map_err(|e| DbError::LockError(format!("Failed to acquire write lock: {}", e)))?;
        active.insert(query.query_id.clone(), query);

        Ok(())
    }

    /// Complete a query
    pub fn complete_query(
        &self,
        query_id: &str,
        cpu_ms: u64,
        mem_bytes: u64,
        result_rows: u64,
    ) -> Result<()> {
        // Remove from active queries
        let mut query = {
            let mut active = self.active_queries.write()
                .map_err(|e| DbError::LockError(format!("Failed to acquire write lock: {}", e)))?;

            active.remove(query_id)
                .ok_or_else(|| DbError::NotFound(format!("Query {} not found", query_id)))?
        };

        // Update query metadata
        query.actual_cpu_ms = cpu_ms;
        query.actual_memory_bytes = mem_bytes;
        query.end_time = Some(Instant::now());
        query.state = QueryState::Completed;

        let exec_ms = query.execution_duration()
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0);

        // Record in quota manager
        self.quota_manager.record_query_completion(
            &query.user_id,
            query.session_id,
            cpu_ms,
            mem_bytes,
            exec_ms,
            result_rows,
        )?;

        // Add to history
        self.add_to_history(query)?;

        // Try to start queued queries
        self.process_queue()?;

        Ok(())
    }

    /// Cancel a query
    pub fn cancel_query(&self, query_id: &str) -> Result<()> {
        // Try to remove from active queries
        let query = {
            let mut active = self.active_queries.write()
                .map_err(|e| DbError::LockError(format!("Failed to acquire write lock: {}", e)))?;

            active.remove(query_id)
        };

        if let Some(mut query) = query {
            query.state = QueryState::Cancelled;
            query.end_time = Some(Instant::now());
            self.add_to_history(query)?;

            // Try to start queued queries
            self.process_queue()?;

            return Ok(());
        }

        // Try to remove from queue
        let mut queue = self.query_queue.lock()
            .map_err(|e| DbError::LockError(format!("Failed to acquire lock: {}", e)))?;

        if let Some(mut query) = queue.remove(query_id) {
            query.state = QueryState::Cancelled;
            self.add_to_history(query)?;
            Ok(())
        } else {
            Err(DbError::NotFound(format!("Query {} not found", query_id)))
        }
    }

    /// Get query status
    pub fn get_query_status(&self, query_id: &str) -> Result<QueryMetadata> {
        // Check active queries
        {
            let active = self.active_queries.read()
                .map_err(|e| DbError::LockError(format!("Failed to acquire read lock: {}", e)))?;

            if let Some(query) = active.get(query_id) {
                return Ok(query.clone());
            }
        }

        // Check queue
        let queue = self.query_queue.lock()
            .map_err(|e| DbError::LockError(format!("Failed to acquire lock: {}", e)))?;

        for query in &queue.queue {
            if query.query_id == query_id {
                return Ok(query.clone());
            }
        }

        // Check history
        let history = self.query_history.read()
            .map_err(|e| DbError::LockError(format!("Failed to acquire read lock: {}", e)))?;

        for query in history.iter() {
            if query.query_id == query_id {
                return Ok(query.clone());
            }
        }

        Err(DbError::NotFound(format!("Query {} not found", query_id)))
    }

    /// Get all active queries
    pub fn get_active_queries(&self) -> Result<Vec<QueryMetadata>> {
        let active = self.active_queries.read()
            .map_err(|e| DbError::LockError(format!("Failed to acquire read lock: {}", e)))?;

        Ok(active.values().cloned().collect())
    }

    /// Get queued queries
    pub fn get_queued_queries(&self) -> Result<Vec<QueryMetadata>> {
        let queue = self.query_queue.lock()
            .map_err(|e| DbError::LockError(format!("Failed to acquire lock: {}", e)))?;

        Ok(queue.queue.iter().cloned().collect())
    }

    /// Process query queue and start queries if possible
    fn process_queue(&self) -> Result<()> {
        loop {
            let active_count = {
                let active = self.active_queries.read()
                    .map_err(|e| DbError::LockError(format!("Failed to acquire read lock: {}", e)))?;
                active.len()
            };

            if active_count >= self.max_concurrent_queries as usize {
                break;
            }

            let query = {
                let mut queue = self.query_queue.lock()
                    .map_err(|e| DbError::LockError(format!("Failed to acquire lock: {}", e)))?;
                queue.dequeue()
            };

            if let Some(query) = query {
                if let Err(e) = self.start_query(query.clone()) {
                    // If we can't start the query, mark it as failed
                    let mut failed_query = query;
                    failed_query.state = QueryState::Failed;
                    failed_query.end_time = Some(Instant::now());
                    self.add_to_history(failed_query)?;
                    return Err(e);
                }
            } else {
                break;
            }
        }

        Ok(())
    }

    /// Add query to history
    fn add_to_history(&self, query: QueryMetadata) -> Result<()> {
        let mut history = self.query_history.write()
            .map_err(|e| DbError::LockError(format!("Failed to acquire write lock: {}", e)))?;

        history.push_back(query);

        // Trim history if too large
        while history.len() > self.max_history_size {
            history.pop_front();
        }

        Ok(())
    }

    /// Get governor statistics
    pub fn get_statistics(&self) -> Result<GovernorStatistics> {
        let active_count = {
            let active = self.active_queries.read()
                .map_err(|e| DbError::LockError(format!("Failed to acquire read lock: {}", e)))?;
            active.len()
        };

        let queued_count = {
            let queue = self.query_queue.lock()
                .map_err(|e| DbError::LockError(format!("Failed to acquire lock: {}", e)))?;
            queue.len()
        };

        let history_count = {
            let history = self.query_history.read()
                .map_err(|e| DbError::LockError(format!("Failed to acquire read lock: {}", e)))?;
            history.len()
        };

        Ok(GovernorStatistics {
            active_queries: active_count,
            queued_queries: queued_count,
            total_queries_processed: history_count,
            max_concurrent_queries: self.max_concurrent_queries as usize,
        })
    }
}

/// Governor statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GovernorStatistics {
    pub active_queries: usize,
    pub queued_queries: usize,
    pub total_queries_processed: usize,
    pub max_concurrent_queries: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_complexity_analysis() {
        let analyzer = ComplexityAnalyzer::new();

        let simple_query = "SELECT * FROM users WHERE id = 1";
        let complex_query = "SELECT * FROM users u JOIN orders o ON u.id = o.user_id WHERE u.name LIKE '%test%' GROUP BY u.id ORDER BY count(*) DESC";

        let simple_score = analyzer.analyze(simple_query);
        let complex_score = analyzer.analyze(complex_query);

        assert!(complex_score > simple_score);
    }

    #[test]
    fn test_query_queue() {
        let mut queue = QueryQueue::new(10);

        let query = QueryMetadata::new(
            "q1".to_string(),
            "user1".to_string(),
            1,
            "SELECT * FROM test".to_string(),
        );

        queue.enqueue(query).unwrap();
        assert_eq!(queue.len(), 1);

        let dequeued = queue.dequeue();
        assert!(dequeued.is_some());
        assert_eq!(queue.len(), 0);
    }
}
