// Session Control for Resource Management
//
// This module implements maximum active sessions, idle timeout management,
// long-running query limits, automatic session termination, and priority boosting.

use std::collections::VecDeque;
use std::sync::Mutex;
use std::collections::HashSet;
use std::time::Instant;
use std::time::SystemTime;
use std::collections::{HashMap};
use std::sync::{Arc, RwLock};
use std::time::{Duration};
use serde::{Deserialize, Serialize};

use crate::error::Result;
use super::consumer_groups::ConsumerGroupId;

/// Session identifier
pub type SessionId = u64;

/// User identifier
pub type UserId = u64;

/// Session state
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SessionState {
    /// Session is inactive (no active query)
    Inactive,
    /// Session has an active query running
    Active,
    /// Session is waiting for a resource
    Waiting,
    /// Session is blocked by a lock
    Blocked,
    /// Session is idle (connected but inactive)
    Idle,
    /// Session is being killed
    Killed,
    /// Session is terminated
    Terminated,
}

/// Session priority
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum SessionPriority {
    /// Critical priority (always allowed)
    Critical,
    /// High priority
    High,
    /// Normal priority
    Normal,
    /// Low priority
    Low,
}

/// Session information
#[derive(Debug, Clone)]
pub struct SessionInfo {
    /// Session identifier
    pub session_id: SessionId,
    /// User identifier
    pub user_id: UserId,
    /// Username
    pub username: String,
    /// Consumer group
    pub group_id: ConsumerGroupId,
    /// Session state
    pub state: SessionState,
    /// Session priority
    pub priority: SessionPriority,
    /// Connection time
    pub connected_at: SystemTime,
    /// Last active time
    pub last_active: Instant,
    /// Current query start time
    pub current_query_start: Option<Instant>,
    /// Total queries executed
    pub total_queries: u64,
    /// Client program name
    pub program_name: Option<String>,
    /// Client machine
    pub machine_name: Option<String>,
    /// Idle time limit
    pub idle_timeout: Option<Duration>,
    /// Maximum execution time limit
    pub max_execution_time: Option<Duration>,
    /// Whether this is a system session
    pub is_system: bool,
    /// Number of times priority was boosted
    pub priority_boosts: u32,
}

impl SessionInfo {
    /// Create a new session
    pub fn new(
        session_id: SessionId,
        user_id: UserId,
        username: String,
        group_id: ConsumerGroupId,
    ) -> Self {
        let now = SystemTime::now();
        let now_instant = Instant::now();

        Self {
            session_id,
            user_id,
            username,
            group_id,
            state: SessionState::Inactive,
            priority: SessionPriority::Normal,
            connected_at: now,
            last_active: now_instant,
            current_query_start: None,
            total_queries: 0,
            program_name: None,
            machine_name: None,
            idle_timeout: None,
            max_execution_time: None,
            is_system: false,
            priority_boosts: 0,
        }
    }

    /// Check if session is idle
    pub fn is_idle(&self) -> bool {
        self.state == SessionState::Idle || self.state == SessionState::Inactive
    }

    /// Get idle duration
    pub fn idle_duration(&self) -> Duration {
        if self.is_idle() {
            Instant::now().duration_since(self.last_active)
        } else {
            Duration::from_secs(0)
        }
    }

    /// Check if idle timeout exceeded
    pub fn is_idle_timeout_exceeded(&self) -> bool {
        if let Some(timeout) = self.idle_timeout {
            self.idle_duration() > timeout
        } else {
            false
        }
    }

    /// Get current query execution time
    pub fn current_query_duration(&self) -> Option<Duration> {
        self.current_query_start.map(|start| Instant::now().duration_since(start))
    }

    /// Check if execution time limit exceeded
    pub fn is_execution_timeout_exceeded(&self) -> bool {
        if let Some(max_time) = self.max_execution_time {
            if let Some(duration) = self.current_query_duration() {
                return duration > max_time;
            }
        }
        false
    }

    /// Start a query
    pub fn start_query(&mut self) {
        self.state = SessionState::Active;
        self.current_query_start = Some(Instant::now());
        self.total_queries += 1;
        self.last_active = Instant::now();
    }

    /// Complete a query
    pub fn complete_query(&mut self) {
        self.state = SessionState::Inactive;
        self.current_query_start = None;
        self.last_active = Instant::now();
    }

    /// Boost priority
    pub fn boost_priority(&mut self) {
        self.priority = match self.priority {
            SessionPriority::Low => SessionPriority::Normal,
            SessionPriority::Normal => SessionPriority::High,
            SessionPriority::High => SessionPriority::Critical,
            SessionPriority::Critical => SessionPriority::Critical,
        };
        self.priority_boosts += 1;
    }
}

/// Active session pool configuration
#[derive(Debug, Clone)]
pub struct ActiveSessionPoolConfig {
    /// Maximum active sessions allowed
    pub max_active_sessions: u32,
    /// Queue timeout for waiting sessions
    pub queue_timeout: Duration,
    /// Whether to allow queuing
    pub allow_queuing: bool,
}

impl Default for ActiveSessionPoolConfig {
    fn default() -> Self {
        Self {
            max_active_sessions: 100,
            queue_timeout: Duration::from_secs(60),
            allow_queuing: true,
        }
    }
}

/// Session waiting in queue
#[derive(Debug, Clone)]
struct QueuedSession {
    session_id: SessionId,
    queued_at: Instant,
    priority: SessionPriority,
}

impl PartialEq for QueuedSession {
    fn eq(&self, other: &Self) -> bool {
        self.session_id == other.session_id
    }
}

impl Eq for QueuedSession {}

/// Session controller
pub struct SessionController {
    /// All sessions
    sessions: Arc<RwLock<HashMap<SessionId, SessionInfo>>>,
    /// Active sessions (currently executing queries)
    active_sessions: Arc<RwLock<HashSet<SessionId>>>,
    /// Session queue (waiting for active session slot)
    session_queue: Arc<Mutex<VecDeque<QueuedSession>>>,
    /// Active session pool configuration per group
    group_configs: Arc<RwLock<HashMap<ConsumerGroupId, ActiveSessionPoolConfig>>>,
    /// Group active session counts
    group_active_counts: Arc<RwLock<HashMap<ConsumerGroupId, u32>>>,
    /// Global maximum sessions
    global_max_sessions: Option<u32>,
    /// Next session ID
    next_session_id: Arc<RwLock<SessionId>>,
    /// Session timeout checker enabled
    timeout_checker_enabled: bool,
    /// Auto-terminate enabled
    auto_terminate_enabled: bool,
    /// Statistics
    stats: Arc<RwLock<SessionStats>>,
}

/// Session statistics
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SessionStats {
    /// Total sessions created
    pub total_sessions_created: u64,
    /// Total sessions terminated
    pub total_sessions_terminated: u64,
    /// Sessions terminated due to idle timeout
    pub idle_timeout_terminations: u64,
    /// Sessions terminated due to execution timeout
    pub execution_timeout_terminations: u64,
    /// Sessions killed manually
    pub manual_kills: u64,
    /// Current active sessions
    pub current_active_sessions: u32,
    /// Peak concurrent sessions
    pub peak_concurrent_sessions: u32,
    /// Sessions queued
    pub sessions_queued: u64,
    /// Queue timeout events
    pub queue_timeouts: u64,
}

impl SessionController {
    /// Create a new session controller
    pub fn new(global_max_sessions: Option<u32>) -> Self {
        Self {
            sessions: Arc::new(RwLock::new(HashMap::new())),
            active_sessions: Arc::new(RwLock::new(HashSet::new())),
            session_queue: Arc::new(Mutex::new(VecDeque::new())),
            group_configs: Arc::new(RwLock::new(HashMap::new())),
            group_active_counts: Arc::new(RwLock::new(HashMap::new())),
            global_max_sessions,
            next_session_id: Arc::new(RwLock::new(1)),
            timeout_checker_enabled: true,
            auto_terminate_enabled: true,
            stats: Arc::new(RwLock::new(SessionStats::default())),
        }
    }

    /// Create a new session
    pub fn create_session(
        &self,
        user_id: UserId,
        username: String,
        group_id: ConsumerGroupId,
    ) -> Result<SessionId> {
        // Check global max sessions
        if let Some(max) = self.global_max_sessions {
            let sessions = self.sessions.read().unwrap();
            if sessions.len() >= max as usize {
                return Err(DbError::ResourceExhausted(
                    "Maximum number of sessions reached".to_string()
                ));
            }
        }

        let session_id = {
            let mut next_id = self.next_session_id.write().unwrap();
            let id = *next_id;
            *next_id += 1;
            id
        };

        let session = SessionInfo::new(session_id, user_id, username, group_id);

        {
            let mut sessions = self.sessions.write().unwrap();
            sessions.insert(session_id, session);
        }

        // Update stats
        {
            let mut stats = self.stats.write().unwrap();
            stats.total_sessions_created += 1;

            let session_count = self.sessions.read().unwrap().len() as u32;
            stats.peak_concurrent_sessions = stats.peak_concurrent_sessions.max(session_count);
        }

        Ok(session_id)
    }

    /// Configure active session pool for a group
    pub fn configure_group_pool(
        &self,
        group_id: ConsumerGroupId,
        config: ActiveSessionPoolConfig,
    ) -> Result<()> {
        let mut configs = self.group_configs.write().unwrap();
        configs.insert(group_id, config);

        let mut counts = self.group_active_counts.write().unwrap();
        counts.insert(group_id, 0);

        Ok(())
    }

    /// Start a query (request active session slot)
    pub fn start_query(&self, session_id: SessionId) -> Result<bool> {
        let group_id = {
            let sessions = self.sessions.read().unwrap();
            let session = sessions.get(&session_id)
                .ok_or_else(|| DbError::NotFound(format!("Session {} not found", session_id)))?;
            session.group_id
        };

        // Check if can become active
        let can_activate = {
            let configs = self.group_configs.read().unwrap();
            let counts = self.group_active_counts.read().unwrap();

            if let Some(config) = configs.get(&group_id) {
                let current_count = counts.get(&group_id).copied().unwrap_or(0);
                current_count < config.max_active_sessions
            } else {
                true // No limit
            }
        };

        if can_activate {
            self.activate_session(session_id)?;
            Ok(true)
        } else {
            // Queue the session
            self.queue_session(session_id)?;
            Ok(false)
        }
    }

    /// Activate a session
    fn activate_session(&self, session_id: SessionId) -> Result<()> {
        let mut sessions = self.sessions.write().unwrap();
        let session = sessions.get_mut(&session_id)
            .ok_or_else(|| DbError::NotFound(format!("Session {} not found", session_id)))?;

        session.start_query();
        let group_id = session.group_id;
        drop(sessions);

        // Add to active set
        {
            let mut active = self.active_sessions.write().unwrap();
            active.insert(session_id);
        }

        // Increment group count
        {
            let mut counts = self.group_active_counts.write().unwrap();
            *counts.entry(group_id).or_insert(0) += 1;
        }

        // Update stats
        {
            let mut stats = self.stats.write().unwrap();
            stats.current_active_sessions += 1;
        }

        Ok(())
    }

    /// Queue a session
    fn queue_session(&self, session_id: SessionId) -> Result<()> {
        let priority = {
            let sessions = self.sessions.read().unwrap();
            let session = sessions.get(&session_id)
                .ok_or_else(|| DbError::NotFound(format!("Session {} not found", session_id)))?;
            session.priority
        };

        let queued = QueuedSession {
            session_id,
            queued_at: Instant::now(),
            priority,
        };

        let mut queue = self.session_queue.lock().unwrap();
        queue.push_back(queued);

        // Update stats
        {
            let mut stats = self.stats.write().unwrap();
            stats.sessions_queued += 1;
        }

        Ok(())
    }

    /// Complete a query
    pub fn complete_query(&self, session_id: SessionId) -> Result<()> {
        let group_id = {
            let mut sessions = self.sessions.write().unwrap();
            let session = sessions.get_mut(&session_id)
                .ok_or_else(|| DbError::NotFound(format!("Session {} not found", session_id)))?;

            session.complete_query();
            session.group_id
        };

        // Remove from active set
        {
            let mut active = self.active_sessions.write().unwrap();
            active.remove(&session_id);
        }

        // Decrement group count
        {
            let mut counts = self.group_active_counts.write().unwrap();
            if let Some(count) = counts.get_mut(&group_id) {
                *count = count.saturating_sub(1);
            }
        }

        // Update stats
        {
            let mut stats = self.stats.write().unwrap();
            if stats.current_active_sessions > 0 {
                stats.current_active_sessions -= 1;
            }
        }

        // Try to activate queued session
        self.process_queue(group_id)?;

        Ok(())
    }

    /// Process session queue
    fn process_queue(&self, group_id: ConsumerGroupId) -> Result<()> {
        let mut queue = self.session_queue.lock().unwrap();

        // Find highest priority session for this group
        let mut best_idx: Option<usize> = None;
        let mut best_priority = SessionPriority::Low;

        for (idx, queued) in queue.iter().enumerate() {
            let sessions = self.sessions.read().unwrap();
            if let Some(session) = sessions.get(&queued.session_id) {
                if session.group_id == group_id && queued.priority >= best_priority {
                    best_priority = queued.priority;
                    best_idx = Some(idx);
                }
            }
        }

        if let Some(idx) = best_idx {
            let queued = queue.remove(idx).unwrap();
            drop(queue);

            // Check queue timeout
            if queued.queued_at.elapsed() > Duration::from_secs(60) {
                let mut stats = self.stats.write().unwrap();
                stats.queue_timeouts += 1;
                return Ok(());
            }

            self.activate_session(queued.session_id)?;
        }

        Ok(())
    }

    /// Terminate a session
    pub fn terminate_session(&self, session_id: SessionId, reason: &str) -> Result<()> {
        let mut sessions = self.sessions.write().unwrap();
        let session = sessions.get_mut(&session_id)
            .ok_or_else(|| DbError::NotFound(format!("Session {} not found", session_id)))?;

        session.state = SessionState::Terminated;
        let group_id = session.group_id;
        let was_active = session.state == SessionState::Active;
        drop(sessions);

        // Remove from active set if needed
        if was_active {
            let mut active = self.active_sessions.write().unwrap();
            active.remove(&session_id);

            let mut counts = self.group_active_counts.write().unwrap();
            if let Some(count) = counts.get_mut(&group_id) {
                *count = count.saturating_sub(1);
            }
        }

        // Update stats
        {
            let mut stats = self.stats.write().unwrap();
            stats.total_sessions_terminated += 1;
        }

        Ok(())
    }

    /// Kill a session
    pub fn kill_session(&self, session_id: SessionId) -> Result<()> {
        self.terminate_session(session_id, "Manual kill")?;

        let mut stats = self.stats.write().unwrap();
        stats.manual_kills += 1;

        Ok(())
    }

    /// Check and terminate idle sessions
    pub fn check_idle_timeouts(&self) -> Vec<SessionId> {
        if !self.timeout_checker_enabled || !self.auto_terminate_enabled {
            return Vec::new();
        }

        let mut to_terminate = Vec::new();
        let sessions = self.sessions.read().unwrap();

        for (session_id, session) in sessions.iter() {
            if session.is_system {
                continue;
            }

            if session.is_idle_timeout_exceeded() {
                to_terminate.push(*session_id);
            }
        }

        drop(sessions);

        // Terminate sessions
        for session_id in &to_terminate {
            let _ = self.terminate_session(*session_id, "Idle timeout");

            let mut stats = self.stats.write().unwrap();
            stats.idle_timeout_terminations += 1;
        }

        to_terminate
    }

    /// Check and terminate long-running queries
    pub fn check_execution_timeouts(&self) -> Vec<SessionId> {
        if !self.timeout_checker_enabled || !self.auto_terminate_enabled {
            return Vec::new();
        }

        let mut to_terminate = Vec::new();
        let sessions = self.sessions.read().unwrap();

        for (session_id, session) in sessions.iter() {
            if session.is_system {
                continue;
            }

            if session.is_execution_timeout_exceeded() {
                to_terminate.push(*session_id);
            }
        }

        drop(sessions);

        // Terminate sessions
        for session_id in &to_terminate {
            let _ = self.terminate_session(*session_id, "Execution timeout");

            let mut stats = self.stats.write().unwrap();
            stats.execution_timeout_terminations += 1;
        }

        to_terminate
    }

    /// Boost session priority
    pub fn boost_session_priority(&self, session_id: SessionId) -> Result<()> {
        let mut sessions = self.sessions.write().unwrap();
        let session = sessions.get_mut(&session_id)
            .ok_or_else(|| DbError::NotFound(format!("Session {} not found", session_id)))?;

        session.boost_priority();
        Ok(())
    }

    /// Get session information
    pub fn get_session(&self, session_id: SessionId) -> Option<SessionInfo> {
        let sessions = self.sessions.read().unwrap();
        sessions.get(&session_id).cloned()
    }

    /// List all sessions
    pub fn list_sessions(&self) -> Vec<SessionInfo> {
        let sessions = self.sessions.read().unwrap();
        sessions.values().cloned().collect()
    }

    /// List active sessions
    pub fn list_active_sessions(&self) -> Vec<SessionInfo> {
        let sessions = self.sessions.read().unwrap();
        let active = self.active_sessions.read().unwrap();

        active.iter()
            .filter_map(|id| sessions.get(id).cloned())
            .collect()
    }

    /// Get statistics
    pub fn get_stats(&self) -> SessionStats {
        self.stats.read().unwrap().clone()
    }

    /// Set session limits
    pub fn set_session_limits(
        &self,
        session_id: SessionId,
        idle_timeout: Option<Duration>,
        max_execution_time: Option<Duration>,
    ) -> Result<()> {
        let mut sessions = self.sessions.write().unwrap();
        let session = sessions.get_mut(&session_id)
            .ok_or_else(|| DbError::NotFound(format!("Session {} not found", session_id)))?;

        session.idle_timeout = idle_timeout;
        session.max_execution_time = max_execution_time;

        Ok(())
    }

    /// Enable/disable timeout checker
    pub fn set_timeout_checker_enabled(&mut self, enabled: bool) {
        self.timeout_checker_enabled = enabled;
    }

    /// Enable/disable auto-terminate
    pub fn set_auto_terminate_enabled(&mut self, enabled: bool) {
        self.auto_terminate_enabled = enabled;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_session_creation() {
        let controller = SessionController::new(Some(1000));
        let session_id = controller.create_session(
            1,
            "testuser".to_string(),
            1,
        ).unwrap();

        assert!(session_id > 0);

        let session = controller.get_session(session_id).unwrap();
        assert_eq!(session.username, "testuser");
    }

    #[test]
    fn test_active_session_pool() {
        let controller = SessionController::new(None);

        let config = ActiveSessionPoolConfig {
            max_active_sessions: 2,
            queue_timeout: Duration::from_secs(60),
            allow_queuing: true,
        };

        controller.configure_group_pool(1, config).unwrap();

        let s1 = controller.create_session(1, "user1".to_string(), 1).unwrap();
        let s2 = controller.create_session(2, "user2".to_string(), 1).unwrap();
        let s3 = controller.create_session(3, "user3".to_string(), 1).unwrap();

        // First two should activate
        assert!(controller.start_query(s1).unwrap());
        assert!(controller.start_query(s2).unwrap());

        // Third should be queued
        assert!(!controller.start_query(s3).unwrap());
    }

    #[test]
    fn test_session_termination() {
        let controller = SessionController::new(None);
        let session_id = controller.create_session(
            1,
            "testuser".to_string(),
            1,
        ).unwrap();

        controller.terminate_session(session_id, "Test").unwrap();

        let session = controller.get_session(session_id).unwrap();
        assert_eq!(session.state, SessionState::Terminated);
    }
}
