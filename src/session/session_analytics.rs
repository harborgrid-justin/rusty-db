// # Session Analytics
//
// Comprehensive session analytics with duration tracking, activity monitoring,
// user behavior analytics, and resource usage tracking.

use crate::common::SessionId;
use crate::error::{DbError, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

// ============================================================================
// Analytics Types
// ============================================================================

/// Session activity type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ActivityType {
    /// Query execution
    Query,
    /// Transaction operation
    Transaction,
    /// Schema modification
    DDL,
    /// Data modification
    DML,
    /// Connection event
    Connection,
    /// Authentication event
    Authentication,
}

/// Activity log entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActivityLog {
    /// Session identifier
    pub session_id: SessionId,
    /// Activity type
    pub activity_type: ActivityType,
    /// Timestamp
    pub timestamp: u64,
    /// Duration in milliseconds
    pub duration_ms: u64,
    /// Activity description
    pub description: String,
    /// Success flag
    pub success: bool,
    /// Error message if failed
    pub error: Option<String>,
}

impl ActivityLog {
    /// Create new activity log entry
    pub fn new(
        session_id: SessionId,
        activity_type: ActivityType,
        description: String,
    ) -> Self {
        Self {
            session_id,
            activity_type,
            timestamp: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
            duration_ms: 0,
            description,
            success: true,
            error: None,
        }
    }

    /// Mark activity as failed
    pub fn with_error(mut self, error: String) -> Self {
        self.success = false;
        self.error = Some(error);
        self
    }

    /// Set activity duration
    pub fn with_duration(mut self, duration_ms: u64) -> Self {
        self.duration_ms = duration_ms;
        self
    }
}

/// Resource usage metrics
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ResourceMetrics {
    /// Memory used (bytes)
    pub memory_bytes: u64,
    /// CPU time (milliseconds)
    pub cpu_time_ms: u64,
    /// I/O operations count
    pub io_operations: u64,
    /// Network bytes sent
    pub network_bytes_sent: u64,
    /// Network bytes received
    pub network_bytes_received: u64,
    /// Temporary space used (bytes)
    pub temp_space_bytes: u64,
}

impl ResourceMetrics {
    /// Add resource usage
    pub fn add(&mut self, other: &ResourceMetrics) {
        self.memory_bytes += other.memory_bytes;
        self.cpu_time_ms += other.cpu_time_ms;
        self.io_operations += other.io_operations;
        self.network_bytes_sent += other.network_bytes_sent;
        self.network_bytes_received += other.network_bytes_received;
        self.temp_space_bytes += other.temp_space_bytes;
    }

    /// Calculate total network bytes
    pub fn total_network_bytes(&self) -> u64 {
        self.network_bytes_sent + self.network_bytes_received
    }
}

/// Session performance metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionMetrics {
    /// Session identifier
    pub session_id: SessionId,
    /// User identifier
    pub user_id: String,
    /// Session start time
    pub start_time: u64,
    /// Session end time (if terminated)
    pub end_time: Option<u64>,
    /// Total queries executed
    pub query_count: u64,
    /// Successful queries
    pub successful_queries: u64,
    /// Failed queries
    pub failed_queries: u64,
    /// Total transactions
    pub transaction_count: u64,
    /// Committed transactions
    pub committed_transactions: u64,
    /// Rolled back transactions
    pub rolled_back_transactions: u64,
    /// Average query duration (ms)
    pub avg_query_duration_ms: f64,
    /// Max query duration (ms)
    pub max_query_duration_ms: u64,
    /// Resource usage
    pub resources: ResourceMetrics,
}

impl SessionMetrics {
    /// Create new session metrics
    pub fn new(session_id: SessionId, user_id: String) -> Self {
        Self {
            session_id,
            user_id,
            start_time: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
            end_time: None,
            query_count: 0,
            successful_queries: 0,
            failed_queries: 0,
            transaction_count: 0,
            committed_transactions: 0,
            rolled_back_transactions: 0,
            avg_query_duration_ms: 0.0,
            max_query_duration_ms: 0,
            resources: ResourceMetrics::default(),
        }
    }

    /// Update query statistics
    pub fn record_query(&mut self, duration_ms: u64, success: bool) {
        self.query_count += 1;

        if success {
            self.successful_queries += 1;
        } else {
            self.failed_queries += 1;
        }

        // Update average duration
        let total_duration = self.avg_query_duration_ms * (self.query_count - 1) as f64;
        self.avg_query_duration_ms = (total_duration + duration_ms as f64) / self.query_count as f64;

        // Update max duration
        if duration_ms > self.max_query_duration_ms {
            self.max_query_duration_ms = duration_ms;
        }
    }

    /// Calculate session duration
    pub fn session_duration(&self) -> Duration {
        let end = self.end_time.unwrap_or_else(|| {
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs()
        });

        Duration::from_secs(end.saturating_sub(self.start_time))
    }

    /// Calculate success rate
    pub fn success_rate(&self) -> f64 {
        if self.query_count == 0 {
            return 0.0;
        }
        (self.successful_queries as f64 / self.query_count as f64) * 100.0
    }
}

/// User behavior pattern
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BehaviorPattern {
    /// User identifier
    pub user_id: String,
    /// Peak activity hours (0-23)
    pub peak_hours: Vec<u8>,
    /// Common query patterns
    pub query_patterns: HashMap<String, u64>,
    /// Average session duration (seconds)
    pub avg_session_duration: u64,
    /// Total sessions
    pub total_sessions: u64,
    /// Preferred database
    pub preferred_database: Option<String>,
}

impl BehaviorPattern {
    /// Create new behavior pattern
    pub fn new(user_id: String) -> Self {
        Self {
            user_id,
            peak_hours: Vec::new(),
            query_patterns: HashMap::new(),
            avg_session_duration: 0,
            total_sessions: 0,
            preferred_database: None,
        }
    }

    /// Update with session data
    pub fn update(&mut self, session_duration: u64, _database: &str) {
        self.total_sessions += 1;

        // Update average session duration
        let total_duration = self.avg_session_duration * (self.total_sessions - 1);
        self.avg_session_duration = (total_duration + session_duration) / self.total_sessions;
    }
}

/// User activity summary
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserActivity {
    /// User identifier
    pub user_id: String,
    /// Active sessions count
    pub active_sessions: usize,
    /// Total sessions created
    pub total_sessions: u64,
    /// Total queries executed
    pub total_queries: u64,
    /// Total resource usage
    pub total_resources: ResourceMetrics,
    /// Last activity timestamp
    pub last_activity: u64,
}

impl UserActivity {
    /// Create new user activity
    pub fn new(user_id: String) -> Self {
        Self {
            user_id,
            active_sessions: 0,
            total_sessions: 0,
            total_queries: 0,
            total_resources: ResourceMetrics::default(),
            last_activity: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
        }
    }
}

// ============================================================================
// Session Analytics Manager
// ============================================================================

/// Session analytics manager
pub struct SessionAnalytics {
    /// Session metrics
    metrics: Arc<RwLock<HashMap<SessionId, SessionMetrics>>>,
    /// Activity logs
    activity_logs: Arc<RwLock<Vec<ActivityLog>>>,
    /// User behavior patterns
    behavior_patterns: Arc<RwLock<HashMap<String, BehaviorPattern>>>,
    /// User activity tracking
    user_activity: Arc<RwLock<HashMap<String, UserActivity>>>,
    /// Maximum activity logs to keep
    max_logs: usize,
}

impl SessionAnalytics {
    /// Create new session analytics manager
    pub fn new(max_logs: usize) -> Self {
        Self {
            metrics: Arc::new(RwLock::new(HashMap::new())),
            activity_logs: Arc::new(RwLock::new(Vec::new())),
            behavior_patterns: Arc::new(RwLock::new(HashMap::new())),
            user_activity: Arc::new(RwLock::new(HashMap::new())),
            max_logs,
        }
    }

    /// Initialize session metrics
    pub fn init_session(&self, session_id: SessionId, user_id: String) -> Result<()> {
        let mut metrics = self.metrics.write()
            .map_err(|e| DbError::Internal(format!("Lock error: {}", e)))?;

        metrics.insert(session_id, SessionMetrics::new(session_id, user_id.clone()));

        // Update user activity
        let mut user_activity = self.user_activity.write()
            .map_err(|e| DbError::Internal(format!("Lock error: {}", e)))?;

        let user_id_clone = user_id.clone();
        let activity = user_activity.entry(user_id).or_insert_with(|| {
            UserActivity::new(user_id_clone)
        });
        activity.active_sessions += 1;
        activity.total_sessions += 1;

        Ok(())
    }

    /// Record query execution
    pub fn record_query(
        &self,
        session_id: SessionId,
        duration_ms: u64,
        success: bool,
        description: String,
    ) -> Result<()> {
        // Update metrics
        let mut metrics = self.metrics.write()
            .map_err(|e| DbError::Internal(format!("Lock error: {}", e)))?;

        if let Some(metric) = metrics.get_mut(&session_id) {
            metric.record_query(duration_ms, success);

            // Update user activity
            let mut user_activity = self.user_activity.write()
                .map_err(|e| DbError::Internal(format!("Lock error: {}", e)))?;

            if let Some(activity) = user_activity.get_mut(&metric.user_id) {
                activity.total_queries += 1;
                activity.last_activity = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs();
            }
        }

        // Log activity
        let mut log = ActivityLog::new(session_id, ActivityType::Query, description);
        log = log.with_duration(duration_ms);
        if !success {
            log = log.with_error("Query failed".to_string());
        }

        self.log_activity(log)?;

        Ok(())
    }

    /// Record transaction event
    pub fn record_transaction(
        &self,
        session_id: SessionId,
        committed: bool,
    ) -> Result<()> {
        let mut metrics = self.metrics.write()
            .map_err(|e| DbError::Internal(format!("Lock error: {}", e)))?;

        if let Some(metric) = metrics.get_mut(&session_id) {
            metric.transaction_count += 1;

            if committed {
                metric.committed_transactions += 1;
            } else {
                metric.rolled_back_transactions += 1;
            }
        }

        Ok(())
    }

    /// Update resource usage
    pub fn update_resources(
        &self,
        session_id: SessionId,
        resources: ResourceMetrics,
    ) -> Result<()> {
        let mut metrics = self.metrics.write()
            .map_err(|e| DbError::Internal(format!("Lock error: {}", e)))?;

        if let Some(metric) = metrics.get_mut(&session_id) {
            metric.resources.add(&resources);

            // Update user activity
            let mut user_activity = self.user_activity.write()
                .map_err(|e| DbError::Internal(format!("Lock error: {}", e)))?;

            if let Some(activity) = user_activity.get_mut(&metric.user_id) {
                activity.total_resources.add(&resources);
            }
        }

        Ok(())
    }

    /// Log activity
    fn log_activity(&self, log: ActivityLog) -> Result<()> {
        let mut activity_logs = self.activity_logs.write()
            .map_err(|e| DbError::Internal(format!("Lock error: {}", e)))?;

        activity_logs.push(log);

        // Trim logs if exceeding max
        if activity_logs.len() > self.max_logs {
            let drain_count = activity_logs.len() - self.max_logs;
            activity_logs.drain(0..drain_count);
        }

        Ok(())
    }

    /// Get session metrics
    pub fn get_metrics(&self, session_id: SessionId) -> Result<SessionMetrics> {
        let metrics = self.metrics.read()
            .map_err(|e| DbError::Internal(format!("Lock error: {}", e)))?;

        metrics.get(&session_id)
            .cloned()
            .ok_or_else(|| DbError::NotFound(format!("Session {} metrics not found", session_id)))
    }

    /// Get user activity
    pub fn get_user_activity(&self, user_id: &str) -> Result<UserActivity> {
        let user_activity = self.user_activity.read()
            .map_err(|e| DbError::Internal(format!("Lock error: {}", e)))?;

        user_activity.get(user_id)
            .cloned()
            .ok_or_else(|| DbError::NotFound(format!("User {} activity not found", user_id)))
    }

    /// Get activity logs for session
    pub fn get_session_logs(&self, session_id: SessionId) -> Result<Vec<ActivityLog>> {
        let activity_logs = self.activity_logs.read()
            .map_err(|e| DbError::Internal(format!("Lock error: {}", e)))?;

        Ok(activity_logs.iter()
            .filter(|log| log.session_id == session_id)
            .cloned()
            .collect())
    }

    /// Get behavior pattern for user
    pub fn get_behavior_pattern(&self, user_id: &str) -> Result<Option<BehaviorPattern>> {
        let patterns = self.behavior_patterns.read()
            .map_err(|e| DbError::Internal(format!("Lock error: {}", e)))?;

        Ok(patterns.get(user_id).cloned())
    }

    /// Update behavior pattern
    pub fn update_behavior_pattern(
        &self,
        user_id: String,
        session_duration: u64,
        database: String,
    ) -> Result<()> {
        let mut patterns = self.behavior_patterns.write()
            .map_err(|e| DbError::Internal(format!("Lock error: {}", e)))?;

        let pattern = patterns.entry(user_id.clone())
            .or_insert_with(|| BehaviorPattern::new(user_id));

        pattern.update(session_duration, &database);
        pattern.preferred_database = Some(database);

        Ok(())
    }

    /// Terminate session analytics
    pub fn terminate_session(&self, session_id: SessionId) -> Result<()> {
        let mut metrics = self.metrics.write()
            .map_err(|e| DbError::Internal(format!("Lock error: {}", e)))?;

        if let Some(metric) = metrics.get_mut(&session_id) {
            metric.end_time = Some(SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs());

            // Update user activity
            let mut user_activity = self.user_activity.write()
                .map_err(|e| DbError::Internal(format!("Lock error: {}", e)))?;

            if let Some(activity) = user_activity.get_mut(&metric.user_id) {
                activity.active_sessions = activity.active_sessions.saturating_sub(1);
            }

            // Update behavior pattern
            let duration = metric.session_duration().as_secs();
            if let Some(db) = metric.user_id.split('@').next() {
                self.update_behavior_pattern(
                    metric.user_id.clone(),
                    duration,
                    db.to_string(),
                )?;
            }
        }

        Ok(())
    }

    /// Get all active session metrics
    pub fn get_all_metrics(&self) -> Result<Vec<SessionMetrics>> {
        let metrics = self.metrics.read()
            .map_err(|e| DbError::Internal(format!("Lock error: {}", e)))?;

        Ok(metrics.values().cloned().collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_session_analytics_init() {
        let analytics = SessionAnalytics::new(1000);
        analytics.init_session(1, "user1".to_string()).unwrap();

        let metrics = analytics.get_metrics(1).unwrap();
        assert_eq!(metrics.session_id, 1);
        assert_eq!(metrics.user_id, "user1");
        assert_eq!(metrics.query_count, 0);
    }

    #[test]
    fn test_query_recording() {
        let analytics = SessionAnalytics::new(1000);
        analytics.init_session(1, "user1".to_string()).unwrap();

        analytics.record_query(1, 100, true, "SELECT * FROM users".to_string()).unwrap();

        let metrics = analytics.get_metrics(1).unwrap();
        assert_eq!(metrics.query_count, 1);
        assert_eq!(metrics.successful_queries, 1);
        assert_eq!(metrics.avg_query_duration_ms, 100.0);
    }

    #[test]
    fn test_user_activity() {
        let analytics = SessionAnalytics::new(1000);
        analytics.init_session(1, "user1".to_string()).unwrap();
        analytics.init_session(2, "user1".to_string()).unwrap();

        let activity = analytics.get_user_activity("user1").unwrap();
        assert_eq!(activity.active_sessions, 2);
        assert_eq!(activity.total_sessions, 2);
    }

    #[test]
    fn test_resource_tracking() {
        let analytics = SessionAnalytics::new(1000);
        analytics.init_session(1, "user1".to_string()).unwrap();

        let mut resources = ResourceMetrics::default();
        resources.memory_bytes = 1024;
        resources.cpu_time_ms = 100;

        analytics.update_resources(1, resources).unwrap();

        let metrics = analytics.get_metrics(1).unwrap();
        assert_eq!(metrics.resources.memory_bytes, 1024);
        assert_eq!(metrics.resources.cpu_time_ms, 100);
    }
}
