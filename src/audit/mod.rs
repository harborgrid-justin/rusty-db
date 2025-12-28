// # Audit Trail Module
//
// Enterprise-grade audit trail system for SOC2/HIPAA compliance.
//
// ## Features
//
// - **Comprehensive Event Tracking**: DDL, DML, DCL, authentication, and administrative events
// - **High Performance**: Async buffered logging with configurable flush intervals
// - **Multiple Destinations**: File, database, and syslog output
// - **Tamper-Evident**: SHA-256 checksums for audit event integrity
// - **Advanced Querying**: Time-range, user, object, and full-text search capabilities
// - **Export Support**: JSON, CSV, and XML export formats
// - **Compliance**: SOC2 and HIPAA compliant audit trail implementation
//
// ## Usage
//
// ### Basic Audit Logging
//
// ```rust,no_run
// use rusty_db::audit::{
//     AuditLogger, AuditLoggerConfig, AuditEvent, AuditEventType,
//     EventCategory, AuditSeverity, ActionOutcome
// };
//
// # async fn example() -> rusty_db::Result<()> {
// // Create and initialize audit logger
// let config = AuditLoggerConfig::default();
// let mut logger = AuditLogger::new(config).await?;
// logger.initialize().await?;
//
// // Log an event
// let event = AuditEvent::new(
//     0, // Event ID assigned by logger
//     EventCategory::Authentication,
//     AuditSeverity::Info,
//     ActionOutcome::Success,
//     AuditEventType::LoginSuccess {
//         authentication_method: "password".to_string(),
//     },
//     "admin".to_string(),
//     "production_db".to_string(),
// );
//
// logger.log_event(event).await?;
// # Ok(())
// # }
// ```
//
// ### Querying Audit Trail
//
// ```rust,no_run
// use rusty_db::audit::{AuditQueryEngine, AuditQueryFilter, EventCategory};
//
// # async fn example() -> rusty_db::Result<()> {
// let engine = AuditQueryEngine::new("./data/audit/audit.log".to_string());
//
// // Query DDL events from last 24 hours
// let filter = AuditQueryFilter::new()
//     .with_category(EventCategory::DDL)
//     .with_username("admin".to_string())
//     .with_limit(100);
//
// let results = engine.query(filter).await?;
// println!("Found {} DDL events", results.total_matched);
// # Ok(())
// # }
// ```
//
// ### Integrity Verification
//
// ```rust,no_run
// use rusty_db::audit::AuditQueryEngine;
//
// # async fn example() -> rusty_db::Result<()> {
// let engine = AuditQueryEngine::new("./data/audit/audit.log".to_string());
// let report = engine.verify_integrity().await?;
//
// if report.integrity_ok {
//     println!("Audit trail integrity verified: {} events checked", report.total_events);
// } else {
//     println!("WARNING: {} events have invalid checksums!", report.invalid_checksums);
// }
// # Ok(())
// # }
// ```
//
// ## Compliance Considerations
//
// ### SOC2 Requirements
//
// - All security-relevant events are logged
// - Audit logs are tamper-evident (checksums)
// - Logs include timestamp, user, source IP, and outcome
// - Failed access attempts are logged
//
// ### HIPAA Requirements
//
// - Access to Protected Health Information (PHI) is logged
// - User authentication and authorization events are tracked
// - Administrative actions are audited
// - Audit logs are protected from unauthorized modification

// ============================================================================
// Module Declarations
// ============================================================================

pub mod audit_events;
pub mod audit_logger;
pub mod audit_query;

// ============================================================================
// Re-exports
// ============================================================================

// Event types
pub use audit_events::{
    ActionOutcome, AuditEvent, AuditEventId, AuditEventType, AuditSeverity, EventCategory,
};

// Logger
pub use audit_logger::{
    create_audit_logger, AuditLogger, AuditLoggerConfig, AuditLoggerStatistics, SharedAuditLogger,
};

// Query engine
pub use audit_query::{
    export_events, AuditIntegrityReport, AuditQueryEngine, AuditQueryFilter, AuditQueryResult,
    ExportFormat, TimeRangeFilter,
};

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_module_imports() {
        // Verify all public types are accessible
        let _ = EventCategory::Authentication;
        let _ = AuditSeverity::Info;
        let _ = ActionOutcome::Success;
    }
}
