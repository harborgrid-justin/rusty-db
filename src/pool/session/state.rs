// Session state management
//
// This module handles the core session state including variables, settings,
// transactions, cursors, and prepared statements.

use super::types::{CursorId, SchemaName, SessionId, StatementId, Username};
use crate::common::{TransactionId, Value};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::collections::HashSet;
use std::fmt;
use std::time::SystemTime;

// Session state representing all context for a database session
//
// This structure maintains the complete state for an active session,
// including user context, variables, transaction state, and resources.
//
// # Invariants
//
// - `session_id` is unique across all active sessions
// - `serial_number` increments on session reuse
// - `last_active` is updated on every operation
// - `created_at` never changes after session creation
//
// # Examples
//
// ```rust,ignore
// use rusty_db::pool::session::{SessionState, Username, SchemaName};
//
// let session = SessionState::new(
//     SessionId::new(1),
//     Username::new("alice").unwrap(),
//     SchemaName::new("public").unwrap(),
// );
//
// assert_eq!(session.status(), &SessionStatus::Active);
// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionState {
    // Unique session identifier
    session_id: SessionId,

    // Session serial number (incremented on reuse)
    serial_number: u32,

    // User who owns this session
    username: Username,

    // Schema context
    current_schema: SchemaName,

    // Session variables (NLS_*, optimizer settings, etc.)
    session_variables: HashMap<String, Value>,

    // Session settings
    settings: SessionSettings,

    // Current transaction state
    transaction_state: TransactionState,

    // Active cursors
    cursors: HashMap<CursorId, CursorState>,

    // Prepared statements cache
    prepared_statements: HashMap<StatementId, PreparedStatement>,

    // Temporary tables created in this session
    temp_tables: HashSet<String>,

    // Session creation time
    created_at: SystemTime,

    // Last active time
    last_active: SystemTime,

    // Session status
    status: SessionStatus,

    // Client information
    client_info: ClientInfo,

    // Resource usage statistics
    resource_usage: ResourceUsage,

    // Session tags for pooling
    tags: HashMap<String, String>,

    // Session affinity (node preference)
    affinity: Option<String>,
}

impl SessionState {
    // Create a new session state
    //
    // # Arguments
    //
    // * `session_id` - Unique identifier for this session
    // * `username` - Authenticated user
    // * `schema` - Default schema context
    //
    // # Examples
    //
    // ```rust,ignore
    // let session = SessionState::new(
    //     SessionId::new(42),
    //     Username::new("bob").unwrap(),
    //     SchemaName::new("public").unwrap(),
    // );
    // ```
    pub fn new(session_id: SessionId, username: Username, schema: SchemaName) -> Self {
        let now = SystemTime::now();
        Self {
            session_id,
            serial_number: 1,
            username,
            current_schema: schema,
            session_variables: HashMap::new(),
            settings: SessionSettings::default(),
            transaction_state: TransactionState::None,
            cursors: HashMap::new(),
            prepared_statements: HashMap::new(),
            temp_tables: HashSet::new(),
            created_at: now,
            last_active: now,
            status: SessionStatus::Active,
            client_info: ClientInfo::default(),
            resource_usage: ResourceUsage::default(),
            tags: HashMap::new(),
            affinity: None,
        }
    }

    // Get session ID
    pub fn id(&self) -> SessionId {
        self.session_id
    }

    // Get username
    pub fn username(&self) -> &Username {
        &self.username
    }

    // Get current schema
    pub fn schema(&self) -> &SchemaName {
        &self.current_schema
    }

    // Set current schema
    pub fn set_schema(&mut self, schema: SchemaName) {
        self.current_schema = schema;
        self.touch();
    }

    // Get session variable
    pub fn get_variable(&self, name: &str) -> Option<&Value> {
        self.session_variables.get(name)
    }

    // Set session variable
    //
    // # Arguments
    //
    // * `name` - Variable name (e.g., "NLS_DATE_FORMAT")
    // * `value` - Variable value
    pub fn set_variable(&mut self, name: String, value: Value) {
        self.session_variables.insert(name, value);
        self.touch();
    }

    // Remove session variable
    pub fn remove_variable(&mut self, name: &str) -> Option<Value> {
        let result = self.session_variables.remove(name);
        self.touch();
        result
    }

    // Get session status
    pub fn status(&self) -> &SessionStatus {
        &self.status
    }

    // Set session status
    pub fn set_status(&mut self, status: SessionStatus) {
        self.status = status;
        self.touch();
    }

    // Update last active timestamp
    pub fn touch(&mut self) {
        self.last_active = SystemTime::now();
    }

    // Get time since last activity
    pub fn idle_duration(&self) -> std::time::Duration {
        SystemTime::now()
            .duration_since(self.last_active)
            .unwrap_or_default()
    }

    // Check if session has active transaction
    pub fn has_active_transaction(&self) -> bool {
        !matches!(self.transaction_state, TransactionState::None)
    }

    // Get transaction state
    pub fn transaction_state(&self) -> &TransactionState {
        &self.transaction_state
    }

    // Set transaction state
    pub fn set_transaction_state(&mut self, state: TransactionState) {
        self.transaction_state = state;
        self.touch();
    }

    // Add a tag for session pooling
    pub fn add_tag(&mut self, key: String, value: String) {
        self.tags.insert(key, value);
    }

    // Check if session matches requested tags
    pub fn matches_tags(&self, requested_tags: &HashMap<String, String>) -> bool {
        requested_tags
            .iter()
            .all(|(k, v)| self.tags.get(k).map_or(false, |val| val == v))
    }

    // Get resource usage statistics
    pub fn resource_usage(&self) -> &ResourceUsage {
        &self.resource_usage
    }

    // Get mutable resource usage statistics
    pub fn resource_usage_mut(&mut self) -> &mut ResourceUsage {
        &mut self.resource_usage
    }
}

// Session configuration settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionSettings {
    pub isolation_level: IsolationLevel,
    pub optimizer_mode: OptimizerMode,
    pub cursor_sharing: CursorSharingMode,
    pub query_timeout: Option<u64>,
    pub idle_timeout: Option<u64>,
    pub autocommit: bool,
    pub parallel_degree: u32,
}

impl Default for SessionSettings {
    fn default() -> Self {
        Self {
            isolation_level: IsolationLevel::ReadCommitted,
            optimizer_mode: OptimizerMode::AllRows,
            cursor_sharing: CursorSharingMode::Exact,
            query_timeout: None,
            idle_timeout: Some(1800), // 30 minutes
            autocommit: false,
            parallel_degree: 1,
        }
    }
}

// Transaction state for a session
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TransactionState {
    // No active transaction
    None,
    // Active transaction
    Active {
        transaction_id: TransactionId,
        isolation_level: IsolationLevel,
        read_only: bool,
        started_at: SystemTime,
    },
    // Transaction marked for rollback
    RollbackOnly {
        transaction_id: TransactionId,
        reason: String,
    },
}

// Transaction isolation levels
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum IsolationLevel {
    ReadUncommitted,
    ReadCommitted,
    RepeatableRead,
    Serializable,
    Snapshot,
}

impl fmt::Display for IsolationLevel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            IsolationLevel::ReadUncommitted => write!(f, "READ_UNCOMMITTED"),
            IsolationLevel::ReadCommitted => write!(f, "READ_COMMITTED"),
            IsolationLevel::RepeatableRead => write!(f, "REPEATABLE_READ"),
            IsolationLevel::Serializable => write!(f, "SERIALIZABLE"),
            IsolationLevel::Snapshot => write!(f, "SNAPSHOT"),
        }
    }
}

// Query optimizer mode
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum OptimizerMode {
    FirstRows,
    AllRows,
    Rule,
}

// Cursor sharing strategy
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CursorSharingMode {
    Exact,
    Force,
    Similar,
}

// State of an open cursor
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CursorState {
    pub cursor_id: CursorId,
    pub sql_text: String,
    pub status: CursorStatus,
    pub current_row: usize,
    pub fetched_rows: usize,
    pub opened_at: SystemTime,
}

// Cursor status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CursorStatus {
    Open,
    Parsing,
    Executing,
    Fetching,
    Closed,
}

// Cached prepared statement
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PreparedStatement {
    pub statement_id: StatementId,
    pub sql_text: String,
    pub parameter_count: usize,
    pub prepared_at: SystemTime,
    pub last_executed: Option<SystemTime>,
    pub execution_count: u64,
}

// Session status enumeration
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SessionStatus {
    Active,
    Inactive,
    Killed,
    Sniped,
    Cached,
}

impl fmt::Display for SessionStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SessionStatus::Active => write!(f, "ACTIVE"),
            SessionStatus::Inactive => write!(f, "INACTIVE"),
            SessionStatus::Killed => write!(f, "KILLED"),
            SessionStatus::Sniped => write!(f, "SNIPED"),
            SessionStatus::Cached => write!(f, "CACHED"),
        }
    }
}

// Client connection information
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ClientInfo {
    pub program: String,
    pub machine: String,
    pub os_user: String,
    pub process_id: u32,
    pub terminal: String,
    pub client_identifier: Option<String>,
    pub module: Option<String>,
    pub action: Option<String>,
}

// Resource usage tracking for a session
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ResourceUsage {
    pub cpu_time_ms: u64,
    pub logical_reads: u64,
    pub physical_reads: u64,
    pub sorts_memory: u64,
    pub sorts_disk: u64,
    pub parse_calls: u64,
    pub execute_calls: u64,
    pub fetch_calls: u64,
    pub temp_space_bytes: u64,
    pub undo_bytes: u64,
}

impl ResourceUsage {
    // Update CPU time
    pub fn add_cpu_time(&mut self, ms: u64) {
        self.cpu_time_ms = self.cpu_time_ms.saturating_add(ms);
    }

    // Record logical read
    pub fn add_logical_reads(&mut self, count: u64) {
        self.logical_reads = self.logical_reads.saturating_add(count);
    }

    // Record physical read
    pub fn add_physical_reads(&mut self, count: u64) {
        self.physical_reads = self.physical_reads.saturating_add(count);
    }

    // Reset all counters
    pub fn reset(&mut self) {
        *self = ResourceUsage::default();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn test_session_state_creation() {
        let session = SessionState::new(
            SessionId::new(1),
            Username::new("alice").unwrap(),
            SchemaName::new("public").unwrap(),
        );

        assert_eq!(session.id(), SessionId::new(1));
        assert_eq!(session.username().as_str(), "alice");
        assert_eq!(session.schema().as_str(), "public");
        assert_eq!(session.status(), &SessionStatus::Active);
        assert!(!session.has_active_transaction());
    }

    #[test]
    fn test_session_variables() {
        let mut session = SessionState::new(
            SessionId::new(2),
            Username::new("bob").unwrap(),
            SchemaName::new("test").unwrap(),
        );

        session.set_variable("TZ".to_string(), Value::String("UTC".to_string()));
        assert_eq!(
            session.get_variable("TZ"),
            Some(&Value::String("UTC".to_string()))
        );

        session.remove_variable("TZ");
        assert_eq!(session.get_variable("TZ"), None);
    }

    #[test]
    fn test_transaction_state() {
        let mut session = SessionState::new(
            SessionId::new(3),
            Username::new("carol").unwrap(),
            SchemaName::new("prod").unwrap(),
        );

        assert!(!session.has_active_transaction());

        session.set_transaction_state(TransactionState::Active {
            transaction_id: 100u64,
            isolation_level: IsolationLevel::Serializable,
            read_only: false,
            started_at: SystemTime::now(),
        });

        assert!(session.has_active_transaction());
    }

    #[test]
    fn test_session_tags() {
        let mut session = SessionState::new(
            SessionId::new(4),
            Username::new("dave").unwrap(),
            SchemaName::new("dev").unwrap(),
        );

        session.add_tag("app".to_string(), "api".to_string());
        session.add_tag("version".to_string(), "1.0".to_string());

        let mut requested = HashMap::new();
        requested.insert("app".to_string(), "api".to_string());
        assert!(session.matches_tags(&requested));

        requested.insert("version".to_string(), "2.0".to_string());
        assert!(!session.matches_tags(&requested));
    }

    #[test]
    fn test_isolation_level_display() {
        assert_eq!(
            format!("{}", IsolationLevel::ReadCommitted),
            "READ_COMMITTED"
        );
        assert_eq!(format!("{}", IsolationLevel::Serializable), "SERIALIZABLE");
    }

    #[test]
    fn test_session_status_display() {
        assert_eq!(format!("{}", SessionStatus::Active), "ACTIVE");
        assert_eq!(format!("{}", SessionStatus::Killed), "KILLED");
    }

    #[test]
    fn test_resource_usage_tracking() {
        let mut usage = ResourceUsage::default();

        usage.add_cpu_time(100);
        usage.add_logical_reads(500);
        usage.add_physical_reads(50);

        assert_eq!(usage.cpu_time_ms, 100);
        assert_eq!(usage.logical_reads, 500);
        assert_eq!(usage.physical_reads, 50);

        usage.reset();
        assert_eq!(usage.cpu_time_ms, 0);
        assert_eq!(usage.logical_reads, 0);
    }
}
