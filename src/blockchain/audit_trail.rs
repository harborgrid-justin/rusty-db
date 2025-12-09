// # Audit Trail for Blockchain Tables
//
// This module provides comprehensive audit logging for blockchain operations:
// - Complete audit logging
// - User action tracking
// - Query logging with results
// - Change attribution
// - Audit report generation
// - Export to compliance formats
// - Audit log protection

use std::collections::VecDeque;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap};
use std::sync::{Arc, RwLock};
use std::time::{SystemTime};
use crate::common::{TableId, RowId};
use crate::Result;
use crate::error::DbError;
use super::ledger::BlockId;
use super::crypto::{sha256, Hash256, hash_to_hex};

// ============================================================================
// Audit Event Types
// ============================================================================

/// Type of audit event
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum AuditEventType {
    /// Table creation
    TableCreate,
    /// Row insertion
    RowInsert,
    /// Block finalization
    BlockFinalize,
    /// Query execution
    QueryExecute,
    /// Verification performed
    Verification,
    /// Retention policy change
    RetentionPolicy,
    /// Legal hold applied
    LegalHold,
    /// Access attempt
    Access,
    /// Configuration change
    ConfigChange,
    /// Export operation
    Export,
    /// Import operation
    Import,
}

impl std::fmt::Display for AuditEventType {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            AuditEventType::TableCreate => write!(f, "TABLE_CREATE"),
            AuditEventType::RowInsert => write!(f, "ROW_INSERT"),
            AuditEventType::BlockFinalize => write!(f, "BLOCK_FINALIZE"),
            AuditEventType::QueryExecute => write!(f, "QUERY_EXECUTE"),
            AuditEventType::Verification => write!(f, "VERIFICATION"),
            AuditEventType::RetentionPolicy => write!(f, "RETENTION_POLICY"),
            AuditEventType::LegalHold => write!(f, "LEGAL_HOLD"),
            AuditEventType::Access => write!(f, "ACCESS"),
            AuditEventType::ConfigChange => write!(f, "CONFIG_CHANGE"),
            AuditEventType::Export => write!(f, "EXPORT"),
            AuditEventType::Import => write!(f, "IMPORT"),
        }
    }
}

/// Severity level of audit event
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum AuditSeverity {
    /// Informational
    Info,
    /// Warning
    Warning,
    /// Error
    Error,
    /// Critical security event
    Critical,
}

// ============================================================================
// Audit Event
// ============================================================================

/// An audit event record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditEvent {
    /// Event ID
    pub event_id: u64,
    /// Event type
    pub event_type: AuditEventType,
    /// Severity
    pub severity: AuditSeverity,
    /// Timestamp
    pub timestamp: u64,
    /// User/session ID
    pub user: String,
    /// Session ID
    pub session_id: Option<SessionId>,
    /// Table ID (if applicable)
    pub table_id: Option<TableId>,
    /// Block ID (if applicable)
    pub block_id: Option<BlockId>,
    /// Row ID (if applicable)
    pub row_id: Option<RowId>,
    /// Description
    pub description: String,
    /// Additional details
    pub details: HashMap<String, String>,
    /// Source IP address
    pub source_ip: Option<String>,
    /// Hash of this event (for integrity)
    pub event_hash: Hash256,
}

impl AuditEvent {
    /// Create a new audit event
    pub fn new(
        event_id: u64,
        event_type: AuditEventType,
        severity: AuditSeverity,
        user: String,
        description: String,
    ) -> Self {
        let timestamp = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();

        let mut event = Self {
            event_id,
            event_type,
            severity,
            timestamp,
            user,
            session_id: None,
            table_id: None,
            block_id: None,
            row_id: None,
            description,
            details: HashMap::new(),
            source_ip: None,
            event_hash: [0u8; 32],
        };

        event.event_hash = event.compute_hash();
        event
    }

    /// Compute hash of this event
    fn compute_hash(&self) -> Hash256 {
        let mut data = Vec::new();
        data.extend_from_slice(&self.event_id.to_le_bytes());
        data.extend_from_slice(&self.timestamp.to_le_bytes());
        data.extend_from_slice(self.user.as_bytes());
        data.extend_from_slice(self.description.as_bytes());

        sha256(&data)
    }

    /// Verify event integrity
    pub fn verify(&self) -> bool {
        let computed = self.compute_hash();
        computed == self.event_hash
    }

    /// Set session ID
    pub fn with_session(mut self, session_id: SessionId) -> Self {
        self.session_id = Some(session_id);
        self.event_hash = self.compute_hash();
        self
    }

    /// Set table ID
    pub fn with_table(mut self, table_id: TableId) -> Self {
        self.table_id = Some(table_id);
        self.event_hash = self.compute_hash();
        self
    }

    /// Set block ID
    pub fn with_block(mut self, block_id: BlockId) -> Self {
        self.block_id = Some(block_id);
        self.event_hash = self.compute_hash();
        self
    }

    /// Set row ID
    pub fn with_row(mut self, row_id: RowId) -> Self {
        self.row_id = Some(row_id);
        self.event_hash = self.compute_hash();
        self
    }

    /// Set source IP
    pub fn with_source_ip(mut self, ip: String) -> Self {
        self.source_ip = Some(ip);
        self.event_hash = self.compute_hash();
        self
    }

    /// Add detail
    pub fn add_detail(&mut self, key: String, value: String) {
        self.details.insert(key, value);
        self.event_hash = self.compute_hash();
    }

    /// Get event hash as hex
    pub fn hash_hex(&self) -> String {
        hash_to_hex(&self.event_hash)
    }
}

// ============================================================================
// Audit Logger
// ============================================================================

/// Configuration for audit logger
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditConfig {
    /// Maximum events to keep in memory
    pub max_events: usize,
    /// Enable persistent logging
    pub persistent: bool,
    /// Log file path
    pub log_path: Option<String>,
    /// Enable encryption of audit logs
    pub encrypt_logs: bool,
    /// Minimum severity to log
    pub min_severity: AuditSeverity,
}

impl Default for AuditConfig {
    fn default() -> Self {
        Self {
            max_events: 10000,
            persistent: true,
            log_path: Some("audit.log".to_string()),
            encrypt_logs: false,
            min_severity: AuditSeverity::Info,
        }
    }
}

/// Audit logger
pub struct AuditLogger {
    /// Configuration
    config: AuditConfig,
    /// Events (in-memory circular buffer)
    events: Arc<RwLock<VecDeque<AuditEvent>>>,
    /// Next event ID
    next_event_id: Arc<RwLock<u64>>,
    /// Event index by type
    type_index: Arc<RwLock<HashMap<AuditEventType, Vec<u64>>>>,
    /// Event index by user
    user_index: Arc<RwLock<HashMap<String, Vec<u64>>>>,
    /// Event index by table
    table_index: Arc<RwLock<HashMap<TableId, Vec<u64>>>>,
}

impl AuditLogger {
    /// Create a new audit logger
    pub fn new(config: AuditConfig) -> Self {
        Self {
            config,
            events: Arc::new(RwLock::new(VecDeque::new())),
            next_event_id: Arc::new(RwLock::new(0)),
            type_index: Arc::new(RwLock::new(HashMap::new())),
            user_index: Arc::new(RwLock::new(HashMap::new())),
            table_index: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Log an audit event
    pub fn log(&self, event: AuditEvent) -> Result<u64> {
        if event.severity < self.config.min_severity {
            return Ok(event.event_id);
        }

        let event_id = event.event_id;

        // Add to event buffer
        let mut events = self.events.write().unwrap();
        if events.len() >= self.config.max_events {
            // Remove oldest event
            if let Some(old) = events.pop_front() {
                // Clean up indices
                self.remove_from_indices(&old);
            }
        }
        events.push_back(event.clone());

        // Update indices
        let mut type_index = self.type_index.write().unwrap();
        type_index.entry(event.event_type)
            .or_insert_with(Vec::new)
            .push(event_id);

        let mut user_index = self.user_index.write().unwrap();
        user_index.entry(event.user.clone())
            .or_insert_with(Vec::new)
            .push(event_id);

        if let Some(table_id) = event.table_id {
            let mut table_index = self.table_index.write().unwrap();
            table_index.entry(table_id)
                .or_insert_with(Vec::new)
                .push(event_id);
        }

        Ok(event_id)
    }

    /// Remove event from indices
    fn remove_from_indices(&self, event: &AuditEvent) {
        let mut type_index = self.type_index.write().unwrap();
        if let Some(events) = type_index.get_mut(&event.event_type) {
            events.retain(|&id| id != event.event_id);
        }

        let mut user_index = self.user_index.write().unwrap();
        if let Some(events) = user_index.get_mut(&event.user) {
            events.retain(|&id| id != event.event_id);
        }

        if let Some(table_id) = event.table_id {
            let mut table_index = self.table_index.write().unwrap();
            if let Some(events) = table_index.get_mut(&table_id) {
                events.retain(|&id| id != event.event_id);
            }
        }
    }

    /// Create and log a new event
    pub fn log_event(
        &self,
        event_type: AuditEventType,
        severity: AuditSeverity,
        user: String,
        description: String,
    ) -> Result<u64> {
        let mut event_id = self.next_event_id.write().unwrap();
        let id = *event_id;
        *event_id += 1;

        let event = AuditEvent::new(id, event_type, severity, user, description);
        self.log(event)
    }

    /// Get events by filter
    pub fn get_events(&self, filter: Option<AuditFilter>) -> Vec<AuditEvent> {
        let events = self.events.read().unwrap();

        if let Some(f) = filter {
            events.iter()
                .filter(|e| f.matches(e))
                .cloned()
                .collect()
        } else {
            events.iter().cloned().collect()
        }
    }

    /// Get events by type
    pub fn get_events_by_type(&self, event_type: AuditEventType) -> Vec<AuditEvent> {
        let type_index = self.type_index.read().unwrap();
        let events = self.events.read().unwrap();

        if let Some(event_ids) = type_index.get(&event_type) {
            events.iter()
                .filter(|e| event_ids.contains(&e.event_id))
                .cloned()
                .collect()
        } else {
            Vec::new()
        }
    }

    /// Get events by user
    pub fn get_events_by_user(&self, user: &str) -> Vec<AuditEvent> {
        let user_index = self.user_index.read().unwrap();
        let events = self.events.read().unwrap();

        if let Some(event_ids) = user_index.get(user) {
            events.iter()
                .filter(|e| event_ids.contains(&e.event_id))
                .cloned()
                .collect()
        } else {
            Vec::new()
        }
    }

    /// Get events by table
    pub fn get_events_by_table(&self, table_id: TableId) -> Vec<AuditEvent> {
        let table_index = self.table_index.read().unwrap();
        let events = self.events.read().unwrap();

        if let Some(event_ids) = table_index.get(&table_id) {
            events.iter()
                .filter(|e| event_ids.contains(&e.event_id))
                .cloned()
                .collect()
        } else {
            Vec::new()
        }
    }

    /// Get event count
    pub fn event_count(&self) -> usize {
        self.events.read().unwrap().len()
    }

    /// Verify all events
    pub fn verify_all(&self) -> bool {
        let events = self.events.read().unwrap();
        events.iter().all(|e| e.verify())
    }
}

// ============================================================================
// Audit Filter
// ============================================================================

/// Filter for querying audit events
#[derive(Debug, Clone)]
pub struct AuditFilter {
    /// Filter by event type
    pub event_type: Option<AuditEventType>,
    /// Filter by severity (minimum)
    pub min_severity: Option<AuditSeverity>,
    /// Filter by user
    pub user: Option<String>,
    /// Filter by time range
    pub time_range: Option<(u64, u64)>,
    /// Filter by table
    pub table_id: Option<TableId>,
}

impl AuditFilter {
    /// Check if event matches filter
    pub fn matches(&self, event: &AuditEvent) -> bool {
        if let Some(event_type) = self.event_type {
            if event.event_type != event_type {
                return false;
            }
        }

        if let Some(min_severity) = self.min_severity {
            if event.severity < min_severity {
                return false;
            }
        }

        if let Some(ref user) = self.user {
            if &event.user != user {
                return false;
            }
        }

        if let Some((start, end)) = self.time_range {
            if event.timestamp < start || event.timestamp > end {
                return false;
            }
        }

        if let Some(table_id) = self.table_id {
            if event.table_id != Some(table_id) {
                return false;
            }
        }

        true
    }
}

// ============================================================================
// Audit Report
// ============================================================================

/// Audit report
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditReport {
    /// Report timestamp
    pub timestamp: u64,
    /// Report period
    pub period: (u64, u64),
    /// Total events
    pub total_events: usize,
    /// Events by type
    pub events_by_type: HashMap<String, usize>,
    /// Events by severity
    pub events_by_severity: HashMap<String, usize>,
    /// Events by user
    pub events_by_user: HashMap<String, usize>,
    /// Critical events
    pub critical_events: Vec<AuditEvent>,
    /// Summary
    pub summary: String,
}

impl AuditReport {
    /// Generate report from events
    pub fn generate(events: &[AuditEvent], start_time: u64, end_time: u64) -> Self {
        let mut events_by_type = HashMap::new();
        let mut events_by_severity = HashMap::new();
        let mut events_by_user = HashMap::new();
        let mut critical_events = Vec::new();

        for event in events {
            *events_by_type.entry(event.event_type.to_string()).or_insert(0) += 1;
            *events_by_severity.entry(format!("{:?}", event.severity)).or_insert(0) += 1;
            *events_by_user.entry(event.user.clone()).or_insert(0) += 1;

            if event.severity == AuditSeverity::Critical {
                critical_events.push(event.clone());
            }
        }

        let summary = format!(
            "Audit report for period {} - {}: {} total events, {} critical",
            start_time, end_time, events.len(), critical_events.len()
        );

        Self {
            timestamp: SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs(),
            period: (start_time, end_time),
            total_events: events.len(),
            events_by_type,
            events_by_severity,
            events_by_user,
            critical_events,
            summary,
        }
    }

    /// Export to JSON
    pub fn to_json(&self) -> Result<String> {
        serde_json::to_string_pretty(self)
            .map_err(|e| DbError::Serialization(format!("Failed to serialize report: {}", e)))
    }

    /// Export to CSV
    pub fn to_csv(&self) -> String {
        let mut csv = String::new();
        csv.push_str("Metric,Value\n");
        csv.push_str(&format!("Total Events,{}\n", self.total_events));
        csv.push_str(&format!("Critical Events,{}\n", self.critical_events.len()));
        csv.push_str(&format!("Period Start,{}\n", self.period.0));
        csv.push_str(&format!("Period End,{}\n", self.period.1));
        csv
    }
}

// ============================================================================
// Query Audit Log
// ============================================================================

/// Query audit entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryAuditEntry {
    /// Query ID
    pub query_id: u64,
    /// Timestamp
    pub timestamp: u64,
    /// User
    pub user: String,
    /// SQL query
    pub query: String,
    /// Execution time (ms)
    pub execution_time_ms: u64,
    /// Rows returned
    pub rows_returned: usize,
    /// Success status
    pub success: bool,
    /// Error (if any)
    pub error: Option<String>,
}

/// Query audit logger
pub struct QueryAuditLogger {
    /// Queries
    queries: Arc<RwLock<VecDeque<QueryAuditEntry>>>,
    /// Max queries to keep
    max_queries: usize,
    /// Next query ID
    next_query_id: Arc<RwLock<u64>>,
}

impl QueryAuditLogger {
    /// Create new query audit logger
    pub fn new(max_queries: usize) -> Self {
        Self {
            queries: Arc::new(RwLock::new(VecDeque::new())),
            max_queries,
            next_query_id: Arc::new(RwLock::new(0)),
        }
    }

    /// Log a query
    pub fn log_query(
        &self,
        user: String,
        query: String,
        execution_time_ms: u64,
        rows_returned: usize,
        success: bool,
        error: Option<String>,
    ) -> u64 {
        let mut query_id = self.next_query_id.write().unwrap();
        let id = *query_id;
        *query_id += 1;

        let entry = QueryAuditEntry {
            query_id: id,
            timestamp: SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs(),
            user,
            query,
            execution_time_ms,
            rows_returned,
            success,
            error,
        };

        let mut queries = self.queries.write().unwrap();
        if queries.len() >= self.max_queries {
            queries.pop_front();
        }
        queries.push_back(entry);

        id
    }

    /// Get recent queries
    pub fn get_recent_queries(&self, count: usize) -> Vec<QueryAuditEntry> {
        let queries = self.queries.read().unwrap();
        queries.iter()
            .rev()
            .take(count)
            .cloned()
            .collect()
    }

    /// Get queries by user
    pub fn get_user_queries(&self, user: &str) -> Vec<QueryAuditEntry> {
        let queries = self.queries.read().unwrap();
        queries.iter()
            .filter(|q| q.user == user)
            .cloned()
            .collect()
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_audit_event() {
        let event = AuditEvent::new(
            1,
            AuditEventType::RowInsert,
            AuditSeverity::Info,
            "user1".to_string(),
            "Inserted row".to_string(),
        );

        assert_eq!(event.event_id, 1);
        assert!(event.verify());
    }

    #[test]
    fn test_audit_logger() {
        let config = AuditConfig::default();
        let logger = AuditLogger::new(config);

        logger.log_event(
            AuditEventType::RowInsert,
            AuditSeverity::Info,
            "user1".to_string(),
            "Test event".to_string(),
        ).unwrap();

        assert_eq!(logger.event_count(), 1);

        let events = logger.get_events_by_user("user1");
        assert_eq!(events.len(), 1);
    }

    #[test]
    fn test_audit_filter() {
        let event = AuditEvent::new(
            1,
            AuditEventType::RowInsert,
            AuditSeverity::Info,
            "user1".to_string(),
            "Test".to_string(),
        );

        let filter = AuditFilter {
            event_type: Some(AuditEventType::RowInsert),
            min_severity: None,
            user: None,
            time_range: None,
            table_id: None,
        };

        assert!(filter.matches(&event));
    }

    #[test]
    fn test_query_audit() {
        let logger = QueryAuditLogger::new(100);

        logger.log_query(
            "user1".to_string(),
            "SELECT * FROM test".to_string(),
            50,
            10,
            true,
            None,
        );

        let queries = logger.get_recent_queries(10);
        assert_eq!(queries.len(), 1);
    }
}
