// # Session Manager
//
// Advanced session lifecycle management with support for state persistence,
// migration, and timeout handling.

use crate::common::{SessionId, TransactionId, Component, HealthStatus};
use crate::error::{DbError, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

// ============================================================================
// Session State Types
// ============================================================================

/// Session lifecycle status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SessionStatus {
    /// Session is being initialized
    Initializing,
    /// Session is active and ready
    Active,
    /// Session is idle (no activity)
    Idle,
    /// Session is in the process of migrating
    Migrating,
    /// Session is being terminated
    Terminating,
    /// Session has been terminated
    Terminated,
}

/// Session state information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionState {
    /// Session identifier
    pub session_id: SessionId,
    /// Current status
    pub status: SessionStatus,
    /// User identifier
    pub user_id: String,
    /// Database name
    pub database: String,
    /// Client IP address
    pub client_ip: String,
    /// Session creation time
    pub created_at: u64,
    /// Last activity time
    pub last_activity: u64,
    /// Session variables
    pub variables: HashMap<String, String>,
    /// Active transaction ID
    pub active_transaction: Option<TransactionId>,
    /// Session timeout (seconds)
    pub timeout: u64,
    /// Number of queries executed
    pub query_count: u64,
}

impl SessionState {
    /// Create a new session state
    pub fn new(
        session_id: SessionId,
        user_id: String,
        database: String,
        client_ip: String,
        timeout: u64,
    ) -> Self {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        Self {
            session_id,
            status: SessionStatus::Initializing,
            user_id,
            database,
            client_ip,
            created_at: now,
            last_activity: now,
            variables: HashMap::new(),
            active_transaction: None,
            timeout,
            query_count: 0,
        }
    }

    /// Update last activity timestamp
    pub fn touch(&mut self) {
        self.last_activity = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
    }

    /// Check if session has timed out
    pub fn is_timed_out(&self) -> bool {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        now - self.last_activity > self.timeout
    }

    /// Get session duration in seconds
    pub fn duration(&self) -> u64 {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        now - self.created_at
    }

    /// Get idle time in seconds
    pub fn idle_time(&self) -> u64 {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        now - self.last_activity
    }
}

/// Session context with additional metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionContext {
    /// Core session state
    pub state: SessionState,
    /// Prepared statements
    pub prepared_statements: HashMap<String, String>,
    /// Cursors
    pub cursors: Vec<String>,
    /// Temporary tables
    pub temp_tables: Vec<String>,
    /// Session-specific configuration
    pub config: SessionConfig,
}

impl SessionContext {
    /// Create new session context
    pub fn new(state: SessionState, config: SessionConfig) -> Self {
        Self {
            state,
            prepared_statements: HashMap::new(),
            cursors: Vec::new(),
            temp_tables: Vec::new(),
            config,
        }
    }

    /// Serialize context to bytes for persistence
    pub fn to_bytes(&self) -> Result<Vec<u8>> {
        bincode::serde::encode_to_vec(self, bincode::config::standard())
            .map_err(|e| DbError::Serialization(e.to_string()))
    }

    /// Deserialize context from bytes
    pub fn from_bytes(data: &[u8]) -> Result<Self> {
        bincode::serde::decode_from_slice(data, bincode::config::standard())
            .map(|(ctx, _)| ctx)
            .map_err(|e| DbError::Serialization(e.to_string()))
    }
}

/// Session configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionConfig {
    /// Maximum memory per session (bytes)
    pub max_memory: usize,
    /// Query timeout (seconds)
    pub query_timeout: u64,
    /// Enable query logging
    pub enable_logging: bool,
    /// Maximum concurrent queries
    pub max_concurrent_queries: usize,
}

impl Default for SessionConfig {
    fn default() -> Self {
        Self {
            max_memory: 100 * 1024 * 1024, // 100MB
            query_timeout: 300,
            enable_logging: true,
            max_concurrent_queries: 10,
        }
    }
}

/// Migration target for session transfer
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MigrationTarget {
    /// Target node identifier
    pub node_id: String,
    /// Target node address
    pub address: String,
    /// Migration reason
    pub reason: String,
}

// ============================================================================
// Session Manager
// ============================================================================

/// Advanced session manager with lifecycle management
pub struct SessionManager {
    /// Active sessions
    sessions: Arc<RwLock<HashMap<SessionId, SessionContext>>>,
    /// Next session ID
    next_session_id: Arc<RwLock<SessionId>>,
    /// Global configuration
    default_config: SessionConfig,
    /// Session timeout (seconds)
    default_timeout: u64,
    /// Enable automatic timeout checking
    enable_timeout_check: bool,
}

impl SessionManager {
    /// Create new session manager
    pub fn new(default_timeout: u64) -> Self {
        Self {
            sessions: Arc::new(RwLock::new(HashMap::new())),
            next_session_id: Arc::new(RwLock::new(1)),
            default_config: SessionConfig::default(),
            default_timeout,
            enable_timeout_check: true,
        }
    }

    /// Create a new session
    pub fn create_session(
        &self,
        user_id: String,
        database: String,
        client_ip: String,
    ) -> Result<SessionId> {
        // Generate session ID
        let session_id = {
            let mut next_id = self.next_session_id.write()
                .map_err(|e| DbError::Internal(format!("Lock error: {}", e)))?;
            let id = *next_id;
            *next_id += 1;
            id
        };

        // Create session state
        let state = SessionState::new(
            session_id,
            user_id,
            database,
            client_ip,
            self.default_timeout,
        );

        // Create session context
        let mut context = SessionContext::new(state, self.default_config.clone());
        context.state.status = SessionStatus::Active;

        // Store session
        let mut sessions = self.sessions.write()
            .map_err(|e| DbError::Internal(format!("Lock error: {}", e)))?;
        sessions.insert(session_id, context);

        Ok(session_id)
    }

    /// Get session state
    pub fn get_session(&self, session_id: SessionId) -> Result<SessionState> {
        let sessions = self.sessions.read()
            .map_err(|e| DbError::Internal(format!("Lock error: {}", e)))?;

        sessions.get(&session_id)
            .map(|ctx| ctx.state.clone())
            .ok_or_else(|| DbError::NotFound(format!("Session {} not found", session_id)))
    }

    /// Update session activity
    pub fn touch_session(&self, session_id: SessionId) -> Result<()> {
        let mut sessions = self.sessions.write()
            .map_err(|e| DbError::Internal(format!("Lock error: {}", e)))?;

        sessions.get_mut(&session_id)
            .map(|ctx| {
                ctx.state.touch();
                ctx.state.query_count += 1;
            })
            .ok_or_else(|| DbError::NotFound(format!("Session {} not found", session_id)))
    }

    /// Set session variable
    pub fn set_variable(&self, session_id: SessionId, name: String, value: String) -> Result<()> {
        let mut sessions = self.sessions.write()
            .map_err(|e| DbError::Internal(format!("Lock error: {}", e)))?;

        sessions.get_mut(&session_id)
            .map(|ctx| {
                ctx.state.variables.insert(name, value);
            })
            .ok_or_else(|| DbError::NotFound(format!("Session {} not found", session_id)))
    }

    /// Get session variable
    pub fn get_variable(&self, session_id: SessionId, name: &str) -> Result<Option<String>> {
        let sessions = self.sessions.read()
            .map_err(|e| DbError::Internal(format!("Lock error: {}", e)))?;

        sessions.get(&session_id)
            .map(|ctx| ctx.state.variables.get(name).cloned())
            .ok_or_else(|| DbError::NotFound(format!("Session {} not found", session_id)))
    }

    /// Terminate a session
    pub fn terminate_session(&self, session_id: SessionId) -> Result<()> {
        let mut sessions = self.sessions.write()
            .map_err(|e| DbError::Internal(format!("Lock error: {}", e)))?;

        if let Some(ctx) = sessions.get_mut(&session_id) {
            ctx.state.status = SessionStatus::Terminating;
        }

        sessions.remove(&session_id)
            .ok_or_else(|| DbError::NotFound(format!("Session {} not found", session_id)))?;

        Ok(())
    }

    /// Persist session state
    pub fn persist_session(&self, session_id: SessionId) -> Result<Vec<u8>> {
        let sessions = self.sessions.read()
            .map_err(|e| DbError::Internal(format!("Lock error: {}", e)))?;

        sessions.get(&session_id)
            .ok_or_else(|| DbError::NotFound(format!("Session {} not found", session_id)))?
            .to_bytes()
    }

    /// Restore session from persisted state
    pub fn restore_session(&self, data: &[u8]) -> Result<SessionId> {
        let context = SessionContext::from_bytes(data)?;
        let session_id = context.state.session_id;

        let mut sessions = self.sessions.write()
            .map_err(|e| DbError::Internal(format!("Lock error: {}", e)))?;

        sessions.insert(session_id, context);

        Ok(session_id)
    }

    /// Migrate session to another node
    pub fn migrate_session(&self, session_id: SessionId, target: MigrationTarget) -> Result<Vec<u8>> {
        let mut sessions = self.sessions.write()
            .map_err(|e| DbError::Internal(format!("Lock error: {}", e)))?;

        let ctx = sessions.get_mut(&session_id)
            .ok_or_else(|| DbError::NotFound(format!("Session {} not found", session_id)))?;

        ctx.state.status = SessionStatus::Migrating;

        // Serialize session for migration
        let data = ctx.to_bytes()?;

        // Mark session as terminated after migration
        ctx.state.status = SessionStatus::Terminated;

        Ok(data)
    }

    /// Check for timed out sessions
    pub fn check_timeouts(&self) -> Result<Vec<SessionId>> {
        if !self.enable_timeout_check {
            return Ok(Vec::new());
        }

        let mut timed_out = Vec::new();
        let sessions = self.sessions.read()
            .map_err(|e| DbError::Internal(format!("Lock error: {}", e)))?;

        for (session_id, ctx) in sessions.iter() {
            if ctx.state.is_timed_out() && ctx.state.status == SessionStatus::Active {
                timed_out.push(*session_id);
            }
        }

        Ok(timed_out)
    }

    /// Get all active sessions
    pub fn get_active_sessions(&self) -> Result<Vec<SessionId>> {
        let sessions = self.sessions.read()
            .map_err(|e| DbError::Internal(format!("Lock error: {}", e)))?;

        Ok(sessions.keys().copied().collect())
    }

    /// Get session count
    pub fn session_count(&self) -> usize {
        self.sessions.read()
            .map(|s| s.len())
            .unwrap_or(0)
    }
}

impl Component for SessionManager {
    fn initialize(&mut self) -> Result<()> {
        // Clear any existing sessions
        let mut sessions = self.sessions.write()
            .map_err(|e| DbError::Internal(format!("Lock error: {}", e)))?;
        sessions.clear();

        Ok(())
    }

    fn shutdown(&mut self) -> Result<()> {
        // Terminate all active sessions
        let mut sessions = self.sessions.write()
            .map_err(|e| DbError::Internal(format!("Lock error: {}", e)))?;

        for ctx in sessions.values_mut() {
            ctx.state.status = SessionStatus::Terminated;
        }

        sessions.clear();

        Ok(())
    }

    fn health_check(&self) -> HealthStatus {
        match self.sessions.read() {
            Ok(_) => HealthStatus::Healthy,
            Err(_) => HealthStatus::Unhealthy,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_session_creation() {
        let manager = SessionManager::new(3600);
        let session_id = manager.create_session(
            "user1".to_string(),
            "testdb".to_string(),
            "127.0.0.1".to_string(),
        ).unwrap();

        assert!(session_id > 0);
        let state = manager.get_session(session_id).unwrap();
        assert_eq!(state.user_id, "user1");
        assert_eq!(state.status, SessionStatus::Active);
    }

    #[test]
    fn test_session_variables() {
        let manager = SessionManager::new(3600);
        let session_id = manager.create_session(
            "user1".to_string(),
            "testdb".to_string(),
            "127.0.0.1".to_string(),
        ).unwrap();

        manager.set_variable(session_id, "key1".to_string(), "value1".to_string()).unwrap();
        let value = manager.get_variable(session_id, "key1").unwrap();
        assert_eq!(value, Some("value1".to_string()));
    }

    #[test]
    fn test_session_termination() {
        let manager = SessionManager::new(3600);
        let session_id = manager.create_session(
            "user1".to_string(),
            "testdb".to_string(),
            "127.0.0.1".to_string(),
        ).unwrap();

        manager.terminate_session(session_id).unwrap();
        assert!(manager.get_session(session_id).is_err());
    }
}
