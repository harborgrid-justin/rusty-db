// Session state management
//
// Core session state types and management functionality

use std::collections::{HashMap, HashSet};
use std::time::{SystemTime, Duration};
use serde::{Serialize, Deserialize};
use crate::common::{TransactionId, Value};

// Session identifier type
pub type SID = u64;

// Cursor identifier
pub type CursorId = u64;

// Prepared statement identifier
pub type StatementId = u64;

// Session state representing all context for a database session
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionState {
    pub session_id: SID,
    pub serial_number: u32,
    pub username: String,
    pub current_schema: String,
    pub session_variables: HashMap<String, Value>,
    pub settings: SessionSettings,
    pub transaction_state: TransactionState,
    pub cursors: HashMap<CursorId, CursorState>,
    pub prepared_statements: HashMap<StatementId, PreparedStatement>,
    pub temp_tables: HashSet<String>,
    pub created_at: SystemTime,
    pub last_active: SystemTime,
    pub status: SessionStatus,
    pub client_info: ClientInfo,
    pub resource_usage: ResourceUsage,
    pub tags: HashMap<String, String>,
    pub affinity: Option<String>,
}

impl SessionState {
    pub fn new(session_id: SID, username: String, schema: String) -> Self {
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

    pub fn touch(&mut self) {
        self.last_active = SystemTime::now();
    }

    pub fn age(&self) -> Duration {
        SystemTime::now().duration_since(self.created_at).unwrap_or_default()
    }

    pub fn idle_time(&self) -> Duration {
        SystemTime::now().duration_since(self.last_active).unwrap_or_default()
    }

    pub fn is_idle(&self, timeout: Duration) -> bool {
        self.idle_time() > timeout
    }
}

// Session settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionSettings {
    pub autocommit: bool,
    pub query_timeout: Option<u64>,
    pub isolation_level: IsolationLevel,
    pub date_format: String,
    pub timestamp_format: String,
    pub timezone: String,
    pub nls_language: String,
    pub nls_territory: String,
    pub optimizer_mode: OptimizerMode,
    pub parallel_degree: u32,
    pub statement_cache_size: usize,
    pub cursor_sharing: CursorSharingMode,
}

impl Default for SessionSettings {
    fn default() -> Self {
        Self {
            autocommit: false,
            query_timeout: Some(3600),
            isolation_level: IsolationLevel::ReadCommitted,
            date_format: "YYYY-MM-DD".to_string(),
            timestamp_format: "YYYY-MM-DD HH24:MI:SS".to_string(),
            timezone: "UTC".to_string(),
            nls_language: "AMERICAN".to_string(),
            nls_territory: "AMERICA".to_string(),
            optimizer_mode: OptimizerMode::AllRows,
            parallel_degree: 1,
            statement_cache_size: 100,
            cursor_sharing: CursorSharingMode::Exact,
        }
    }
}

// Transaction state
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum TransactionState {
    None,
    Active {
        transaction_id: TransactionId,
        started_at: SystemTime,
        isolation_level: IsolationLevel,
        read_only: bool,
    },
    Prepared {
        transaction_id: TransactionId,
        xa_id: Option<String>,
    },
}

// Isolation level
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum IsolationLevel {
    ReadUncommitted,
    ReadCommitted,
    RepeatableRead,
    Serializable,
    SnapshotIsolation,
}

// Optimizer mode
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum OptimizerMode {
    AllRows,
    FirstRows(u32),
    Rule,
    Choose,
}

// Cursor sharing mode
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum CursorSharingMode {
    Exact,
    Force,
    Similar,
}

// Cursor state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CursorState {
    pub cursor_id: CursorId,
    pub name: Option<String>,
    pub sql: String,
    pub status: CursorStatus,
    pub current_row: usize,
    pub rows_fetched: usize,
    pub opened_at: SystemTime,
    pub last_fetch: SystemTime,
    pub holdable: bool,
    pub scrollable: bool,
}

// Cursor status
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum CursorStatus {
    Open,
    Closed,
    Parsing,
    Executing,
    Fetching,
}

// Prepared statement
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PreparedStatement {
    pub statement_id: StatementId,
    pub name: Option<String>,
    pub sql: String,
    pub plan: Vec<u8>,
    pub param_count: usize,
    pub created_at: SystemTime,
    pub last_executed: Option<SystemTime>,
    pub execution_count: u64,
    pub total_execution_time: u64,
}

// Session status
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum SessionStatus {
    Active,
    Inactive,
    Killed,
    Waiting,
    Blocked,
    Migrating,
}

// Client information
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ClientInfo {
    pub program: Option<String>,
    pub machine: Option<String>,
    pub process_id: Option<u32>,
    pub ip_address: Option<String>,
    pub port: Option<u16>,
    pub protocol: Option<String>,
    pub module: Option<String>,
    pub action: Option<String>,
    pub client_identifier: Option<String>,
}

// Resource usage statistics
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ResourceUsage {
    pub cpu_time_ms: u64,
    pub memory_bytes: u64,
    pub io_read_bytes: u64,
    pub io_write_bytes: u64,
    pub temp_space_bytes: u64,
    pub queries_executed: u64,
    pub transactions_committed: u64,
    pub transactions_rolled_back: u64,
    pub parse_calls: u64,
    pub execute_calls: u64,
    pub fetch_calls: u64,
}

impl ResourceUsage {
    pub fn add_cpu_time(&mut self, ms: u64) {
        self.cpu_time_ms += ms;
    }

    pub fn add_memory(&mut self, bytes: u64) {
        self.memory_bytes += bytes;
    }

    pub fn add_io_read(&mut self, bytes: u64) {
        self.io_read_bytes += bytes;
    }

    pub fn add_io_write(&mut self, bytes: u64) {
        self.io_write_bytes += bytes;
    }
}
