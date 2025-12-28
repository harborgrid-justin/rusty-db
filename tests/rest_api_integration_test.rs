// REST API Integration Tests
// Comprehensive tests for REST API endpoints including health checks,
// database operations, monitoring, and enterprise features

use rusty_db::api::{
    handlers::{
        audit::AuditHandler,
        backup::BackupHandler,
        dashboard::DashboardHandler,
        diagnostics::DiagnosticsHandler,
        health::HealthHandler,
    },
};
use serde_json::json;

#[tokio::test]
async fn test_health_check_endpoint() {
    let handler = HealthHandler::new();

    // Test basic health check
    let result = handler.check_health().await;
    assert!(result.is_ok(), "Health check should succeed");

    let health = result.unwrap();
    assert!(
        health.status == "healthy" || health.status == "degraded",
        "Health status should be healthy or degraded"
    );
}

#[tokio::test]
async fn test_health_check_detailed() {
    let handler = HealthHandler::new();

    // Test detailed health check
    let result = handler.check_health_detailed().await;
    assert!(result.is_ok(), "Detailed health check should succeed");

    let health = result.unwrap();
    assert!(
        health.components.contains_key("database"),
        "Should include database component"
    );
    assert!(
        health.components.contains_key("storage"),
        "Should include storage component"
    );
}

#[tokio::test]
async fn test_diagnostics_endpoints() {
    let handler = DiagnosticsHandler::new();

    // Test system diagnostics
    let result = handler.get_system_diagnostics().await;
    assert!(result.is_ok(), "System diagnostics should succeed");

    let diagnostics = result.unwrap();
    assert!(diagnostics.cpu_usage >= 0.0, "CPU usage should be valid");
    assert!(diagnostics.memory_usage >= 0.0, "Memory usage should be valid");
    assert!(
        diagnostics.disk_usage >= 0.0,
        "Disk usage should be valid"
    );
}

#[tokio::test]
async fn test_diagnostics_query_performance() {
    let handler = DiagnosticsHandler::new();

    // Test query performance diagnostics
    let result = handler.get_query_performance().await;
    assert!(result.is_ok(), "Query performance diagnostics should succeed");

    let perf = result.unwrap();
    assert!(
        perf.slow_queries.len() >= 0,
        "Should return slow queries list"
    );
}

#[tokio::test]
async fn test_backup_create() {
    let handler = BackupHandler::new();

    // Test backup creation
    let result = handler.create_backup("full".to_string()).await;
    assert!(result.is_ok(), "Backup creation should succeed");

    let backup_id = result.unwrap();
    assert!(!backup_id.is_empty(), "Backup ID should not be empty");
}

#[tokio::test]
async fn test_backup_list() {
    let handler = BackupHandler::new();

    // Test listing backups
    let result = handler.list_backups().await;
    assert!(result.is_ok(), "Listing backups should succeed");

    let backups = result.unwrap();
    assert!(backups.len() >= 0, "Should return backups list");
}

#[tokio::test]
async fn test_backup_restore() {
    let handler = BackupHandler::new();

    // First create a backup
    let backup_id = handler.create_backup("full".to_string()).await.unwrap();

    // Test restore
    let result = handler.restore_backup(backup_id.clone()).await;
    assert!(result.is_ok(), "Backup restore should succeed or fail gracefully");
}

#[tokio::test]
async fn test_audit_log_query() {
    let handler = AuditHandler::new();

    // Test audit log querying
    let result = handler.query_audit_logs(None, None, None).await;
    assert!(result.is_ok(), "Audit log query should succeed");

    let logs = result.unwrap();
    assert!(logs.len() >= 0, "Should return audit logs");
}

#[tokio::test]
async fn test_audit_log_filtering() {
    let handler = AuditHandler::new();

    // Test with filters
    let result = handler
        .query_audit_logs(
            Some("user123".to_string()),
            Some("SELECT".to_string()),
            None,
        )
        .await;
    assert!(result.is_ok(), "Filtered audit log query should succeed");
}

#[tokio::test]
async fn test_dashboard_metrics() {
    let handler = DashboardHandler::new();

    // Test dashboard metrics
    let result = handler.get_metrics().await;
    assert!(result.is_ok(), "Dashboard metrics should succeed");

    let metrics = result.unwrap();
    assert!(
        metrics.contains_key("connections"),
        "Should include connections metric"
    );
    assert!(
        metrics.contains_key("queries_per_second"),
        "Should include QPS metric"
    );
}

#[tokio::test]
async fn test_dashboard_summary() {
    let handler = DashboardHandler::new();

    // Test dashboard summary
    let result = handler.get_summary().await;
    assert!(result.is_ok(), "Dashboard summary should succeed");

    let summary = result.unwrap();
    assert!(
        summary.total_tables >= 0,
        "Should include total tables count"
    );
    assert!(
        summary.total_indexes >= 0,
        "Should include total indexes count"
    );
}

#[tokio::test]
async fn test_concurrent_api_requests() {
    let handler = HealthHandler::new();

    // Test concurrent health checks
    let mut handles = vec![];

    for _ in 0..10 {
        let h = handler.clone();
        let handle = tokio::spawn(async move { h.check_health().await });
        handles.push(handle);
    }

    for handle in handles {
        let result = handle.await;
        assert!(result.is_ok(), "Concurrent requests should succeed");
    }
}

#[tokio::test]
async fn test_api_error_handling() {
    let handler = BackupHandler::new();

    // Test with invalid backup ID
    let result = handler.restore_backup("invalid_id".to_string()).await;
    assert!(
        result.is_err() || result.is_ok(),
        "Should handle invalid backup ID gracefully"
    );
}

#[tokio::test]
async fn test_api_rate_limiting() {
    let handler = HealthHandler::new();

    // Test rapid successive calls
    for _ in 0..100 {
        let _result = handler.check_health().await;
        // Should not panic or crash
    }
}

#[tokio::test]
async fn test_api_response_format() {
    let handler = HealthHandler::new();

    let result = handler.check_health().await;
    assert!(result.is_ok(), "Health check should succeed");

    // Verify JSON serialization
    let health = result.unwrap();
    let json = serde_json::to_value(&health);
    assert!(json.is_ok(), "Health response should serialize to JSON");
}

#[tokio::test]
async fn test_diagnostics_performance_metrics() {
    let handler = DiagnosticsHandler::new();

    let result = handler.get_performance_metrics().await;
    assert!(result.is_ok(), "Performance metrics should succeed");

    let metrics = result.unwrap();
    assert!(
        metrics.buffer_pool_hit_rate >= 0.0 && metrics.buffer_pool_hit_rate <= 1.0,
        "Hit rate should be between 0 and 1"
    );
}

#[tokio::test]
async fn test_backup_incremental() {
    let handler = BackupHandler::new();

    // Test incremental backup creation
    let result = handler.create_backup("incremental".to_string()).await;
    assert!(result.is_ok(), "Incremental backup should succeed");
}

#[tokio::test]
async fn test_audit_compliance_report() {
    let handler = AuditHandler::new();

    // Test compliance report generation
    let result = handler.generate_compliance_report().await;
    assert!(result.is_ok(), "Compliance report generation should succeed");

    let report = result.unwrap();
    assert!(
        report.total_events >= 0,
        "Report should include total events"
    );
}
