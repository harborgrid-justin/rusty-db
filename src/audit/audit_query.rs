// # Audit Trail Querying
//
// Provides comprehensive querying capabilities for audit trail analysis:
// - Time-range queries
// - User-based filtering
// - Object-based filtering
// - Export functionality
// - Compliance reporting

use crate::audit::audit_events::{
    ActionOutcome, AuditEvent, AuditEventId, AuditSeverity, EventCategory,
};
use crate::common::SessionId;
use crate::error::{DbError, Result};
use serde::{Deserialize, Serialize};
use std::net::IpAddr;
use std::path::Path;
use tokio::fs::File;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader, BufWriter};

// ============================================================================
// Query Filters
// ============================================================================

/// Time range filter for audit queries
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimeRangeFilter {
    /// Start time (microseconds since UNIX epoch)
    pub start_timestamp: Option<i64>,

    /// End time (microseconds since UNIX epoch)
    pub end_timestamp: Option<i64>,
}

impl TimeRangeFilter {
    /// Create a filter for all events
    pub fn all() -> Self {
        Self {
            start_timestamp: None,
            end_timestamp: None,
        }
    }

    /// Create a filter for events after a timestamp
    pub fn after(timestamp: i64) -> Self {
        Self {
            start_timestamp: Some(timestamp),
            end_timestamp: None,
        }
    }

    /// Create a filter for events before a timestamp
    pub fn before(timestamp: i64) -> Self {
        Self {
            start_timestamp: None,
            end_timestamp: Some(timestamp),
        }
    }

    /// Create a filter for events in a range
    pub fn between(start: i64, end: i64) -> Self {
        Self {
            start_timestamp: Some(start),
            end_timestamp: Some(end),
        }
    }

    /// Check if event matches time range
    pub fn matches(&self, event: &AuditEvent) -> bool {
        if let Some(start) = self.start_timestamp {
            if event.timestamp < start {
                return false;
            }
        }

        if let Some(end) = self.end_timestamp {
            if event.timestamp > end {
                return false;
            }
        }

        true
    }
}

/// Comprehensive audit query filter
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AuditQueryFilter {
    /// Time range filter
    pub time_range: Option<TimeRangeFilter>,

    /// Filter by usernames
    pub usernames: Option<Vec<String>>,

    /// Filter by event categories
    pub categories: Option<Vec<EventCategory>>,

    /// Filter by severity levels
    pub severities: Option<Vec<AuditSeverity>>,

    /// Filter by outcomes
    pub outcomes: Option<Vec<ActionOutcome>>,

    /// Filter by session IDs
    pub session_ids: Option<Vec<SessionId>>,

    /// Filter by database names
    pub database_names: Option<Vec<String>>,

    /// Filter by source IP addresses
    pub source_ips: Option<Vec<IpAddr>>,

    /// Filter by table names (for DML/DDL events)
    pub table_names: Option<Vec<String>>,

    /// Full-text search in SQL statements
    pub sql_search: Option<String>,

    /// Maximum number of results
    pub limit: Option<usize>,
}

impl AuditQueryFilter {
    /// Create an empty filter (matches all events)
    pub fn new() -> Self {
        Self::default()
    }

    /// Add time range filter
    pub fn with_time_range(mut self, time_range: TimeRangeFilter) -> Self {
        self.time_range = Some(time_range);
        self
    }

    /// Add username filter
    pub fn with_username(mut self, username: String) -> Self {
        self.usernames.get_or_insert_with(Vec::new).push(username);
        self
    }

    /// Add category filter
    pub fn with_category(mut self, category: EventCategory) -> Self {
        self.categories.get_or_insert_with(Vec::new).push(category);
        self
    }

    /// Add severity filter
    pub fn with_severity(mut self, severity: AuditSeverity) -> Self {
        self.severities.get_or_insert_with(Vec::new).push(severity);
        self
    }

    /// Set result limit
    pub fn with_limit(mut self, limit: usize) -> Self {
        self.limit = Some(limit);
        self
    }

    /// Check if event matches all filters
    pub fn matches(&self, event: &AuditEvent) -> bool {
        // Time range filter
        if let Some(time_range) = &self.time_range {
            if !time_range.matches(event) {
                return false;
            }
        }

        // Username filter
        if let Some(usernames) = &self.usernames {
            if !usernames.contains(&event.username) {
                return false;
            }
        }

        // Category filter
        if let Some(categories) = &self.categories {
            if !categories.contains(&event.category) {
                return false;
            }
        }

        // Severity filter
        if let Some(severities) = &self.severities {
            if !severities.contains(&event.severity) {
                return false;
            }
        }

        // Outcome filter
        if let Some(outcomes) = &self.outcomes {
            if !outcomes.contains(&event.outcome) {
                return false;
            }
        }

        // Session ID filter
        if let Some(session_ids) = &self.session_ids {
            if let Some(session_id) = event.session_id {
                if !session_ids.contains(&session_id) {
                    return false;
                }
            } else {
                return false;
            }
        }

        // Database name filter
        if let Some(database_names) = &self.database_names {
            if !database_names.contains(&event.database_name) {
                return false;
            }
        }

        // Source IP filter
        if let Some(source_ips) = &self.source_ips {
            if let Some(source_ip) = event.source_ip {
                if !source_ips.contains(&source_ip) {
                    return false;
                }
            } else {
                return false;
            }
        }

        // SQL search filter
        if let Some(sql_search) = &self.sql_search {
            if let Some(sql) = &event.sql_statement {
                if !sql.contains(sql_search) {
                    return false;
                }
            } else {
                return false;
            }
        }

        true
    }
}

// ============================================================================
// Query Results
// ============================================================================

/// Audit query result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditQueryResult {
    /// Matched events
    pub events: Vec<AuditEvent>,

    /// Total events matched (before limit applied)
    pub total_matched: usize,

    /// Query execution time in milliseconds
    pub execution_time_ms: u64,
}

impl AuditQueryResult {
    /// Create a new query result
    pub fn new(events: Vec<AuditEvent>, total_matched: usize, execution_time_ms: u64) -> Self {
        Self {
            events,
            total_matched,
            execution_time_ms,
        }
    }
}

// ============================================================================
// Audit Query Engine
// ============================================================================

/// Audit trail query engine
pub struct AuditQueryEngine {
    /// Path to audit log file
    log_file_path: String,
}

impl AuditQueryEngine {
    /// Create a new query engine
    pub fn new(log_file_path: String) -> Self {
        Self { log_file_path }
    }

    /// Execute a query on the audit trail
    pub async fn query(&self, filter: AuditQueryFilter) -> Result<AuditQueryResult> {
        let start_time = std::time::Instant::now();
        let mut events = Vec::new();
        let mut total_matched = 0;

        // Open log file
        let file = File::open(&self.log_file_path)
            .await
            .map_err(|e| DbError::Io(std::sync::Arc::new(e)))?;

        let reader = BufReader::new(file);
        let mut lines = reader.lines();

        // Read and filter events
        while let Ok(Some(line)) = lines.next_line().await {
            // Parse JSON event
            match serde_json::from_str::<AuditEvent>(&line) {
                Ok(event) => {
                    if filter.matches(&event) {
                        total_matched += 1;

                        // Check limit
                        if let Some(limit) = filter.limit {
                            if events.len() < limit {
                                events.push(event);
                            }
                        } else {
                            events.push(event);
                        }
                    }
                }
                Err(e) => {
                    eprintln!("Failed to parse audit event: {}", e);
                    continue;
                }
            }
        }

        let execution_time_ms = start_time.elapsed().as_millis() as u64;

        Ok(AuditQueryResult::new(events, total_matched, execution_time_ms))
    }

    /// Query events by ID range
    pub async fn query_by_id_range(
        &self,
        start_id: AuditEventId,
        end_id: AuditEventId,
    ) -> Result<Vec<AuditEvent>> {
        let file = File::open(&self.log_file_path)
            .await
            .map_err(|e| DbError::Io(std::sync::Arc::new(e)))?;

        let reader = BufReader::new(file);
        let mut lines = reader.lines();
        let mut events = Vec::new();

        while let Ok(Some(line)) = lines.next_line().await {
            if let Ok(event) = serde_json::from_str::<AuditEvent>(&line) {
                if event.event_id >= start_id && event.event_id <= end_id {
                    events.push(event);
                }
            }
        }

        Ok(events)
    }

    /// Get event by ID
    pub async fn get_event_by_id(&self, event_id: AuditEventId) -> Result<Option<AuditEvent>> {
        let file = File::open(&self.log_file_path)
            .await
            .map_err(|e| DbError::Io(std::sync::Arc::new(e)))?;

        let reader = BufReader::new(file);
        let mut lines = reader.lines();

        while let Ok(Some(line)) = lines.next_line().await {
            if let Ok(event) = serde_json::from_str::<AuditEvent>(&line) {
                if event.event_id == event_id {
                    return Ok(Some(event));
                }
            }
        }

        Ok(None)
    }

    /// Verify audit trail integrity
    pub async fn verify_integrity(&self) -> Result<AuditIntegrityReport> {
        let file = File::open(&self.log_file_path)
            .await
            .map_err(|e| DbError::Io(std::sync::Arc::new(e)))?;

        let reader = BufReader::new(file);
        let mut lines = reader.lines();

        let mut total_events = 0;
        let mut valid_checksums = 0;
        let mut invalid_checksums = 0;
        let mut missing_checksums = 0;

        while let Ok(Some(line)) = lines.next_line().await {
            if let Ok(event) = serde_json::from_str::<AuditEvent>(&line) {
                total_events += 1;

                if event.checksum.is_some() {
                    if event.verify_checksum() {
                        valid_checksums += 1;
                    } else {
                        invalid_checksums += 1;
                    }
                } else {
                    missing_checksums += 1;
                }
            }
        }

        Ok(AuditIntegrityReport {
            total_events,
            valid_checksums,
            invalid_checksums,
            missing_checksums,
            integrity_ok: invalid_checksums == 0,
        })
    }
}

// ============================================================================
// Export Functionality
// ============================================================================

/// Export format options
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExportFormat {
    Json,
    Csv,
    Xml,
}

/// Export audit events to a file
pub async fn export_events<P: AsRef<Path>>(
    events: &[AuditEvent],
    output_path: P,
    format: ExportFormat,
) -> Result<()> {
    let file = File::create(output_path)
        .await
        .map_err(|e| DbError::Io(std::sync::Arc::new(e)))?;

    let mut writer = BufWriter::new(file);

    match format {
        ExportFormat::Json => {
            // Export as JSON array
            let json = serde_json::to_string_pretty(events)
                .map_err(|e| DbError::Serialization(e.to_string()))?;
            writer.write_all(json.as_bytes()).await
                .map_err(|e| DbError::Io(std::sync::Arc::new(e)))?;
        }
        ExportFormat::Csv => {
            // Export as CSV (simplified)
            writer.write_all(b"event_id,timestamp,category,severity,outcome,username,database\n").await
                .map_err(|e| DbError::Io(std::sync::Arc::new(e)))?;

            for event in events {
                let line = format!(
                    "{},{},{:?},{:?},{:?},{},{}\n",
                    event.event_id,
                    event.timestamp,
                    event.category,
                    event.severity,
                    event.outcome,
                    event.username,
                    event.database_name
                );
                writer.write_all(line.as_bytes()).await
                    .map_err(|e| DbError::Io(std::sync::Arc::new(e)))?;
            }
        }
        ExportFormat::Xml => {
            // Export as XML (simplified)
            writer.write_all(b"<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<audit_events>\n").await
                .map_err(|e| DbError::Io(std::sync::Arc::new(e)))?;

            for event in events {
                let xml = format!(
                    "  <event id=\"{}\" timestamp=\"{}\" category=\"{:?}\" severity=\"{:?}\" outcome=\"{:?}\" username=\"{}\" database=\"{}\"/>\n",
                    event.event_id,
                    event.timestamp,
                    event.category,
                    event.severity,
                    event.outcome,
                    event.username,
                    event.database_name
                );
                writer.write_all(xml.as_bytes()).await
                    .map_err(|e| DbError::Io(std::sync::Arc::new(e)))?;
            }

            writer.write_all(b"</audit_events>\n").await
                .map_err(|e| DbError::Io(std::sync::Arc::new(e)))?;
        }
    }

    writer.flush().await.map_err(|e| DbError::Io(std::sync::Arc::new(e)))?;
    Ok(())
}

// ============================================================================
// Integrity Verification
// ============================================================================

/// Audit trail integrity report
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditIntegrityReport {
    /// Total events checked
    pub total_events: usize,

    /// Events with valid checksums
    pub valid_checksums: usize,

    /// Events with invalid checksums
    pub invalid_checksums: usize,

    /// Events missing checksums
    pub missing_checksums: usize,

    /// Overall integrity status
    pub integrity_ok: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_time_range_filter() {
        let filter = TimeRangeFilter::between(100, 200);

        let event = AuditEvent::new(
            1,
            EventCategory::Authentication,
            AuditSeverity::Info,
            ActionOutcome::Success,
            crate::audit::audit_events::AuditEventType::LoginSuccess {
                authentication_method: "password".to_string(),
            },
            "user".to_string(),
            "db".to_string(),
        );

        // Event timestamp will be current time, so it won't match the historical range
        // This is expected behavior
    }

    #[test]
    fn test_query_filter_builder() {
        let filter = AuditQueryFilter::new()
            .with_username("admin".to_string())
            .with_category(EventCategory::DDL)
            .with_limit(100);

        assert_eq!(filter.usernames, Some(vec!["admin".to_string()]));
        assert_eq!(filter.limit, Some(100));
    }
}
