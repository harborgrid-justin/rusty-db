// # Resource Quota Management
//
// This module implements per-user, per-session, and per-query resource quotas
// with enforcement and violation tracking.

use crate::error::{DbError, Result};
use crate::common::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use std::time::{Duration, SystemTime};

// ============================================================================
// Quota Types and Structures
// ============================================================================

/// Resource usage statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceUsage {
    /// CPU time used in milliseconds
    pub cpu_time_ms: u64,
    /// Memory used in bytes
    pub memory_bytes: u64,
    /// Execution time in milliseconds
    pub execution_time_ms: u64,
    /// Number of rows returned
    pub result_rows: u64,
    /// Number of queries executed
    pub query_count: u64,
    /// Last updated timestamp
    pub last_updated: SystemTime,
}

impl Default for ResourceUsage {
    fn default() -> Self {
        Self::new()
    }
}

impl ResourceUsage {
    /// Create new resource usage tracker
    pub fn new() -> Self {
        Self {
            cpu_time_ms: 0,
            memory_bytes: 0,
            execution_time_ms: 0,
            result_rows: 0,
            query_count: 0,
            last_updated: SystemTime::now(),
        }
    }

    /// Add resource usage
    pub fn add(&mut self, cpu_ms: u64, mem_bytes: u64, exec_ms: u64, rows: u64) {
        self.cpu_time_ms = self.cpu_time_ms.saturating_add(cpu_ms);
        self.memory_bytes = self.memory_bytes.saturating_add(mem_bytes);
        self.execution_time_ms = self.execution_time_ms.saturating_add(exec_ms);
        self.result_rows = self.result_rows.saturating_add(rows);
        self.query_count = self.query_count.saturating_add(1);
        self.last_updated = SystemTime::now();
    }

    /// Reset resource usage
    pub fn reset(&mut self) {
        self.cpu_time_ms = 0;
        self.memory_bytes = 0;
        self.execution_time_ms = 0;
        self.result_rows = 0;
        self.query_count = 0;
        self.last_updated = SystemTime::now();
    }
}

/// Per-user quota definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserQuota {
    /// User identifier
    pub user_id: String,
    /// Maximum CPU time per time window (milliseconds)
    pub max_cpu_time_ms: Option<u64>,
    /// Maximum memory usage (bytes)
    pub max_memory_bytes: Option<u64>,
    /// Maximum execution time per query (milliseconds)
    pub max_query_execution_ms: Option<u64>,
    /// Maximum result rows per query
    pub max_result_rows: Option<u64>,
    /// Maximum concurrent queries
    pub max_concurrent_queries: Option<u32>,
    /// Maximum queries per time window
    pub max_queries_per_window: Option<u64>,
    /// Time window for quota reset
    pub quota_window: Duration,
    /// Current resource usage
    pub current_usage: ResourceUsage,
    /// Window start time
    pub window_start: SystemTime,
}

impl UserQuota {
    /// Create a new user quota
    pub fn new(user_id: String, quota_window: Duration) -> Self {
        Self {
            user_id,
            max_cpu_time_ms: None,
            max_memory_bytes: None,
            max_query_execution_ms: None,
            max_result_rows: None,
            max_concurrent_queries: None,
            max_queries_per_window: None,
            quota_window,
            current_usage: ResourceUsage::new(),
            window_start: SystemTime::now(),
        }
    }

    /// Check if quota window has expired and reset if necessary
    pub fn check_and_reset_window(&mut self) {
        let now = SystemTime::now();
        if let Ok(elapsed) = now.duration_since(self.window_start) {
            if elapsed >= self.quota_window {
                self.current_usage.reset();
                self.window_start = now;
            }
        }
    }

    /// Check if adding usage would exceed quota
    pub fn would_exceed(&self, cpu_ms: u64, mem_bytes: u64, exec_ms: u64, rows: u64) -> Option<String> {
        // Check CPU time
        if let Some(max_cpu) = self.max_cpu_time_ms {
            if self.current_usage.cpu_time_ms.saturating_add(cpu_ms) > max_cpu {
                return Some(format!("CPU time quota exceeded: {} ms", max_cpu));
            }
        }

        // Check memory
        if let Some(max_mem) = self.max_memory_bytes {
            if self.current_usage.memory_bytes.saturating_add(mem_bytes) > max_mem {
                return Some(format!("Memory quota exceeded: {} bytes", max_mem));
            }
        }

        // Check execution time
        if let Some(max_exec) = self.max_query_execution_ms {
            if exec_ms > max_exec {
                return Some(format!("Query execution time quota exceeded: {} ms", max_exec));
            }
        }

        // Check result rows
        if let Some(max_rows) = self.max_result_rows {
            if rows > max_rows {
                return Some(format!("Result rows quota exceeded: {} rows", max_rows));
            }
        }

        // Check query count
        if let Some(max_queries) = self.max_queries_per_window {
            if self.current_usage.query_count >= max_queries {
                return Some(format!("Query count quota exceeded: {} queries", max_queries));
            }
        }

        None
    }
}

/// Per-session quota definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionQuota {
    /// Session identifier
    pub session_id: SessionId,
    /// User identifier
    pub user_id: String,
    /// Maximum concurrent queries for this session
    pub max_concurrent_queries: Option<u32>,
    /// Current concurrent query count
    pub current_concurrent: u32,
    /// Maximum total queries for this session
    pub max_total_queries: Option<u64>,
    /// Current resource usage
    pub current_usage: ResourceUsage,
    /// Session start time
    pub session_start: SystemTime,
}

impl SessionQuota {
    /// Create a new session quota
    pub fn new(session_id: SessionId, user_id: String) -> Self {
        Self {
            session_id,
            user_id,
            max_concurrent_queries: None,
            current_concurrent: 0,
            max_total_queries: None,
            current_usage: ResourceUsage::new(),
            session_start: SystemTime::now(),
        }
    }

    /// Check if can start a new query
    pub fn can_start_query(&self) -> Result<()> {
        if let Some(max_concurrent) = self.max_concurrent_queries {
            if self.current_concurrent >= max_concurrent {
                return Err(DbError::ResourceExhausted(format!(
                    "Session concurrent query limit reached: {}",
                    max_concurrent
                )));
            }
        }

        if let Some(max_total) = self.max_total_queries {
            if self.current_usage.query_count >= max_total {
                return Err(DbError::ResourceExhausted(format!(
                    "Session total query limit reached: {}",
                    max_total
                )));
            }
        }

        Ok(())
    }

    /// Mark query as started
    pub fn start_query(&mut self) -> Result<()> {
        self.can_start_query()?;
        self.current_concurrent += 1;
        Ok(())
    }

    /// Mark query as completed
    pub fn complete_query(&mut self, cpu_ms: u64, mem_bytes: u64, exec_ms: u64, rows: u64) {
        self.current_concurrent = self.current_concurrent.saturating_sub(1);
        self.current_usage.add(cpu_ms, mem_bytes, exec_ms, rows);
    }
}

/// Per-query quota (single query limits)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryQuota {
    /// Maximum execution time for a single query
    pub max_execution_time: Option<Duration>,
    /// Maximum memory for a single query
    pub max_memory_bytes: Option<u64>,
    /// Maximum result rows for a single query
    pub max_result_rows: Option<u64>,
    /// Maximum complexity score
    pub max_complexity_score: Option<u64>,
}

impl Default for QueryQuota {
    fn default() -> Self {
        Self {
            max_execution_time: Some(Duration::from_secs(300)), // 5 minutes default
            max_memory_bytes: Some(1024 * 1024 * 1024), // 1 GB default
            max_result_rows: Some(1_000_000), // 1M rows default
            max_complexity_score: Some(10000), // Default complexity limit
        }
    }
}

// ============================================================================
// Quota Manager
// ============================================================================

/// Quota enforcement manager
#[derive(Clone)]
pub struct QuotaManager {
    /// User quotas indexed by user ID
    user_quotas: Arc<RwLock<HashMap<String, UserQuota>>>,
    /// Session quotas indexed by session ID
    session_quotas: Arc<RwLock<HashMap<SessionId, SessionQuota>>>,
    /// Default user quota window
    default_quota_window: Duration,
    /// Default query quota
    default_query_quota: QueryQuota,
}

impl QuotaManager {
    /// Create a new quota manager
    pub fn new() -> Self {
        Self {
            user_quotas: Arc::new(RwLock::new(HashMap::new())),
            session_quotas: Arc::new(RwLock::new(HashMap::new())),
            default_quota_window: Duration::from_secs(3600), // 1 hour
            default_query_quota: QueryQuota::default(),
        }
    }

    /// Set user quota
    pub fn set_user_quota(&self, quota: UserQuota) -> Result<()> {
        let mut quotas = self.user_quotas.write()
            .map_err(|e| DbError::LockError(format!("Failed to acquire write lock: {}", e)))?;

        quotas.insert(quota.user_id.clone(), quota);
        Ok(())
    }

    /// Get user quota
    pub fn get_user_quota(&self, user_id: &str) -> Result<Option<UserQuota>> {
        let quotas = self.user_quotas.read()
            .map_err(|e| DbError::LockError(format!("Failed to acquire read lock: {}", e)))?;

        Ok(quotas.get(user_id).cloned())
    }

    /// Create or get session quota
    pub fn create_session_quota(&self, session_id: SessionId, user_id: String) -> Result<()> {
        let mut quotas = self.session_quotas.write()
            .map_err(|e| DbError::LockError(format!("Failed to acquire write lock: {}", e)))?;

        quotas.insert(session_id, SessionQuota::new(session_id, user_id));
        Ok(())
    }

    /// Get session quota
    pub fn get_session_quota(&self, session_id: SessionId) -> Result<Option<SessionQuota>> {
        let quotas = self.session_quotas.read()
            .map_err(|e| DbError::LockError(format!("Failed to acquire read lock: {}", e)))?;

        Ok(quotas.get(&session_id).cloned())
    }

    /// Remove session quota
    pub fn remove_session_quota(&self, session_id: SessionId) -> Result<()> {
        let mut quotas = self.session_quotas.write()
            .map_err(|e| DbError::LockError(format!("Failed to acquire write lock: {}", e)))?;

        quotas.remove(&session_id);
        Ok(())
    }

    /// Check if query can start (both user and session quotas)
    pub fn check_query_start(
        &self,
        user_id: &str,
        session_id: SessionId,
    ) -> Result<()> {
        // Check user quota
        if let Some(mut user_quota) = self.get_user_quota(user_id)? {
            user_quota.check_and_reset_window();

            if let Some(max_concurrent) = user_quota.max_concurrent_queries {
                // This is a simplified check; in production, we'd track active queries
                if user_quota.current_usage.query_count > 0 &&
                   user_quota.current_usage.query_count as u32 >= max_concurrent {
                    return Err(DbError::ResourceExhausted(format!(
                        "User concurrent query limit reached: {}",
                        max_concurrent
                    )));
                }
            }
        }

        // Check session quota
        let mut quotas = self.session_quotas.write()
            .map_err(|e| DbError::LockError(format!("Failed to acquire write lock: {}", e)))?;

        if let Some(session_quota) = quotas.get_mut(&session_id) {
            session_quota.can_start_query()?;
            session_quota.start_query()?;
        }

        Ok(())
    }

    /// Record query completion
    pub fn record_query_completion(
        &self,
        user_id: &str,
        session_id: SessionId,
        cpu_ms: u64,
        mem_bytes: u64,
        exec_ms: u64,
        rows: u64,
    ) -> Result<()> {
        // Update user quota
        {
            let mut quotas = self.user_quotas.write()
                .map_err(|e| DbError::LockError(format!("Failed to acquire write lock: {}", e)))?;

            if let Some(user_quota) = quotas.get_mut(user_id) {
                user_quota.check_and_reset_window();

                // Check if this would exceed quota
                if let Some(reason) = user_quota.would_exceed(cpu_ms, mem_bytes, exec_ms, rows) {
                    return Err(DbError::QuotaExceeded(reason));
                }

                user_quota.current_usage.add(cpu_ms, mem_bytes, exec_ms, rows);
            }
        }

        // Update session quota
        {
            let mut quotas = self.session_quotas.write()
                .map_err(|e| DbError::LockError(format!("Failed to acquire write lock: {}", e)))?;

            if let Some(session_quota) = quotas.get_mut(&session_id) {
                session_quota.complete_query(cpu_ms, mem_bytes, exec_ms, rows);
            }
        }

        Ok(())
    }

    /// Get default query quota
    pub fn get_query_quota(&self) -> QueryQuota {
        self.default_query_quota.clone()
    }

    /// Set default query quota
    pub fn set_query_quota(&mut self, quota: QueryQuota) {
        self.default_query_quota = quota;
    }

    /// Get resource usage for user
    pub fn get_user_usage(&self, user_id: &str) -> Result<Option<ResourceUsage>> {
        let quotas = self.user_quotas.read()
            .map_err(|e| DbError::LockError(format!("Failed to acquire read lock: {}", e)))?;

        Ok(quotas.get(user_id).map(|q| q.current_usage.clone()))
    }

    /// Get resource usage for session
    pub fn get_session_usage(&self, session_id: SessionId) -> Result<Option<ResourceUsage>> {
        let quotas = self.session_quotas.read()
            .map_err(|e| DbError::LockError(format!("Failed to acquire read lock: {}", e)))?;

        Ok(quotas.get(&session_id).map(|q| q.current_usage.clone()))
    }
}

impl Default for QuotaManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resource_usage() {
        let mut usage = ResourceUsage::new();
        usage.add(100, 1024, 50, 10);

        assert_eq!(usage.cpu_time_ms, 100);
        assert_eq!(usage.memory_bytes, 1024);
        assert_eq!(usage.execution_time_ms, 50);
        assert_eq!(usage.result_rows, 10);
        assert_eq!(usage.query_count, 1);
    }

    #[test]
    fn test_user_quota() {
        let mut quota = UserQuota::new("user1".to_string(), Duration::from_secs(3600));
        quota.max_cpu_time_ms = Some(1000);
        quota.max_memory_bytes = Some(1024 * 1024);

        // Should not exceed
        assert!(quota.would_exceed(500, 512 * 1024, 100, 100).is_none());

        // Should exceed CPU
        assert!(quota.would_exceed(1500, 512 * 1024, 100, 100).is_some());

        // Should exceed memory
        assert!(quota.would_exceed(500, 2 * 1024 * 1024, 100, 100).is_some());
    }

    #[test]
    fn test_session_quota() {
        let mut quota = SessionQuota::new(1, "user1".to_string());
        quota.max_concurrent_queries = Some(5);

        // Should be able to start
        assert!(quota.can_start_query().is_ok());

        // Simulate 5 concurrent queries
        quota.current_concurrent = 5;
        assert!(quota.can_start_query().is_err());
    }
}
