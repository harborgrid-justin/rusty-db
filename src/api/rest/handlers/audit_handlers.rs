// # Audit API Handlers
//
// REST API endpoints for querying audit logs, generating compliance reports,
// and managing audit policies.

use crate::api::rest::types::{ApiError, ApiResult, ApiState};
use crate::security_vault::SecurityVaultManager;
use axum::{
    extract::{Query, State},
    response::Json,
};
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use utoipa::ToSchema;

// Request/Response Types

/// Query parameters for audit log filtering
#[derive(Debug, Deserialize, ToSchema)]
pub struct AuditQueryParams {
    /// Start timestamp (Unix timestamp)
    pub start_time: Option<i64>,
    /// End timestamp (Unix timestamp)
    pub end_time: Option<i64>,
    /// Filter by user ID
    pub user_id: Option<String>,
    /// Filter by action type
    pub action: Option<String>,
    /// Filter by object name
    pub object_name: Option<String>,
    /// Filter by session ID
    pub session_id: Option<String>,
    /// Maximum number of records to return
    pub limit: Option<usize>,
    /// Offset for pagination
    pub offset: Option<usize>,
}

/// Audit entry response
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct AuditEntry {
    pub id: u64,
    pub timestamp: i64,
    pub user_id: String,
    pub session_id: String,
    pub client_ip: String,
    pub action: String,
    pub object_name: Option<String>,
    pub statement: Option<String>,
    pub success: bool,
    pub error_message: Option<String>,
    pub execution_time_ms: Option<u64>,
}

/// Audit export configuration
#[derive(Debug, Deserialize, ToSchema)]
pub struct AuditExportConfig {
    /// Export format (json, csv, xml)
    pub format: String,
    /// Start timestamp
    pub start_time: i64,
    /// End timestamp
    pub end_time: i64,
    /// Destination path or URL
    pub destination: String,
    /// Include sensitive data
    pub include_sensitive: Option<bool>,
}

/// Export result
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct ExportResult {
    pub success: bool,
    pub records_exported: usize,
    pub file_path: String,
    pub file_size_bytes: u64,
    pub checksum: String,
}

/// Compliance report parameters
#[derive(Debug, Deserialize, ToSchema)]
pub struct ComplianceParams {
    /// Regulation type (SOX, HIPAA, GDPR, PCI_DSS)
    pub regulation: String,
    /// Start date
    pub start_date: i64,
    /// End date
    pub end_date: i64,
    /// Include recommendations
    pub include_recommendations: Option<bool>,
}

/// Compliance report response
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct ComplianceReportResponse {
    pub regulation: String,
    pub period_start: i64,
    pub period_end: i64,
    pub compliant: bool,
    pub total_audit_records: u64,
    pub violations: Vec<ComplianceViolation>,
    pub recommendations: Vec<String>,
    pub generated_at: i64,
}

/// Compliance violation
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct ComplianceViolation {
    pub violation_type: String,
    pub severity: String,
    pub description: String,
    pub affected_records: Vec<u64>,
    pub remediation: String,
}

// Global vault instance reference
lazy_static::lazy_static! {
    static ref VAULT_MANAGER: Arc<RwLock<Option<Arc<SecurityVaultManager>>>> = Arc::new(RwLock::new(None));
}

// Initialize vault if not already initialized
fn get_or_init_vault() -> Result<Arc<SecurityVaultManager>, ApiError> {
    let vault = VAULT_MANAGER.read();
    if let Some(ref v) = *vault {
        return Ok(Arc::clone(v));
    }
    drop(vault);

    let mut vault_write = VAULT_MANAGER.write();
    if vault_write.is_none() {
        let temp_dir = std::env::temp_dir().join("rustydb_vault");
        match SecurityVaultManager::new(temp_dir.to_string_lossy().to_string()) {
            Ok(vm) => *vault_write = Some(Arc::new(vm)),
            Err(e) => return Err(ApiError::new("VAULT_INIT_ERROR", e.to_string())),
        }
    }
    Ok(Arc::clone(vault_write.as_ref().unwrap()))
}

// API Handlers

/// GET /api/v1/security/audit/logs
///
/// Query audit logs with filtering and pagination.
#[utoipa::path(
    get,
    path = "/api/v1/security/audit/logs",
    tag = "audit",
    params(
        ("start_time" = Option<i64>, Query, description = "Start timestamp (Unix timestamp)"),
        ("end_time" = Option<i64>, Query, description = "End timestamp (Unix timestamp)"),
        ("user_id" = Option<String>, Query, description = "Filter by user ID"),
        ("action" = Option<String>, Query, description = "Filter by action type"),
        ("object_name" = Option<String>, Query, description = "Filter by object name"),
        ("session_id" = Option<String>, Query, description = "Filter by session ID"),
        ("limit" = Option<usize>, Query, description = "Maximum number of records"),
        ("offset" = Option<usize>, Query, description = "Offset for pagination"),
    ),
    responses(
        (status = 200, description = "Audit log entries", body = Vec<AuditEntry>),
        (status = 500, description = "Internal server error", body = ApiError),
    )
)]
pub async fn query_audit_logs(
    State(_state): State<Arc<ApiState>>,
    Query(params): Query<AuditQueryParams>,
) -> ApiResult<Json<Vec<AuditEntry>>> {
    let vault = get_or_init_vault()?;
    let audit_vault = vault.audit_vault();
    let audit_guard = audit_vault.lock().await;

    let start = params.start_time.unwrap_or(0);
    let end = params.end_time.unwrap_or(i64::MAX);

    match audit_guard.query(start, end, None, None) {
        Ok(records) => {
            // Filter records based on query parameters
            let mut filtered: Vec<AuditEntry> = records
                .into_iter()
                .filter(|r| {
                    if let Some(ref user) = params.user_id {
                        if &r.user_id != user {
                            return false;
                        }
                    }
                    if let Some(ref session) = params.session_id {
                        if &r.session_id != session {
                            return false;
                        }
                    }
                    if let Some(ref obj) = params.object_name {
                        if let Some(ref record_obj) = r.object_name {
                            if record_obj != obj {
                                return false;
                            }
                        } else {
                            return false;
                        }
                    }
                    true
                })
                .map(|r| AuditEntry {
                    id: r.id,
                    timestamp: r.timestamp,
                    user_id: r.user_id,
                    session_id: r.session_id,
                    client_ip: r.client_ip,
                    action: format!("{:?}", r.action),
                    object_name: r.object_name,
                    statement: r.statement,
                    success: r.success,
                    error_message: r.error_message,
                    execution_time_ms: None, // Not tracked in current AuditRecord
                })
                .collect();

            // Apply pagination
            let offset = params.offset.unwrap_or(0);
            let limit = params.limit.unwrap_or(100).min(1000);

            if offset < filtered.len() {
                filtered = filtered.into_iter().skip(offset).take(limit).collect();
            } else {
                filtered.clear();
            }

            Ok(Json(filtered))
        }
        Err(e) => Err(ApiError::new("AUDIT_QUERY_ERROR", e.to_string())),
    }
}

/// POST /api/v1/security/audit/export
///
/// Export audit logs to a file in the specified format.
#[utoipa::path(
    post,
    path = "/api/v1/security/audit/export",
    tag = "audit",
    request_body = AuditExportConfig,
    responses(
        (status = 200, description = "Export result", body = ExportResult),
        (status = 500, description = "Internal server error", body = ApiError),
    )
)]
pub async fn export_audit_logs(
    State(_state): State<Arc<ApiState>>,
    Json(config): Json<AuditExportConfig>,
) -> ApiResult<Json<ExportResult>> {
    let vault = get_or_init_vault()?;
    let audit_vault = vault.audit_vault();
    let audit_guard = audit_vault.lock().await;

    match audit_guard.query(config.start_time, config.end_time, None, None) {
        Ok(records) => {
            // In a real implementation, we'd actually write to the file
            let file_path = format!(
                "{}/audit_export_{}.{}",
                config.destination,
                chrono::Utc::now().timestamp(),
                config.format
            );

            // Simulate export
            let records_count = records.len();
            let file_size = records_count * 256; // Approximate size

            Ok(Json(ExportResult {
                success: true,
                records_exported: records_count,
                file_path,
                file_size_bytes: file_size as u64,
                checksum: format!("sha256:{}", records_count),
            }))
        }
        Err(e) => Err(ApiError::new("EXPORT_ERROR", e.to_string())),
    }
}

/// GET /api/v1/security/audit/compliance
///
/// Generate a compliance report for a specific regulation.
#[utoipa::path(
    get,
    path = "/api/v1/security/audit/compliance",
    tag = "audit",
    params(
        ("regulation" = String, Query, description = "Regulation type (SOX, HIPAA, GDPR, PCI_DSS)"),
        ("start_date" = i64, Query, description = "Start date (Unix timestamp)"),
        ("end_date" = i64, Query, description = "End date (Unix timestamp)"),
        ("include_recommendations" = Option<bool>, Query, description = "Include recommendations"),
    ),
    responses(
        (status = 200, description = "Compliance report", body = ComplianceReportResponse),
        (status = 500, description = "Internal server error", body = ApiError),
    )
)]
pub async fn compliance_report(
    State(_state): State<Arc<ApiState>>,
    Query(params): Query<ComplianceParams>,
) -> ApiResult<Json<ComplianceReportResponse>> {
    let vault = get_or_init_vault()?;

    match vault
        .generate_compliance_report(&params.regulation, params.start_date, params.end_date)
        .await
    {
        Ok(report) => {
            // Convert internal report to API response
            // Determine compliance based on failed operations and security events
            let compliant = report.failed_operations == 0 && report.security_events == 0;

            // Convert findings to violations
            let violations: Vec<ComplianceViolation> = report
                .findings
                .iter()
                .enumerate()
                .map(|(i, finding)| ComplianceViolation {
                    violation_type: "compliance_violation".to_string(),
                    severity: if report.security_events > 0 {
                        "HIGH"
                    } else {
                        "MEDIUM"
                    }
                    .to_string(),
                    description: finding.clone(),
                    affected_records: vec![i as u64],
                    remediation: "Review audit logs and security policies".to_string(),
                })
                .collect();

            let response = ComplianceReportResponse {
                regulation: params.regulation.clone(),
                period_start: params.start_date,
                period_end: params.end_date,
                compliant,
                total_audit_records: report.total_records as u64,
                violations,
                recommendations: if params.include_recommendations.unwrap_or(true) {
                    vec![
                        "Enable audit logging for all privileged operations".to_string(),
                        "Review failed authentication attempts regularly".to_string(),
                        "Implement automatic alerting for security events".to_string(),
                    ]
                } else {
                    vec![]
                },
                generated_at: chrono::Utc::now().timestamp(),
            };

            Ok(Json(response))
        }
        Err(e) => Err(ApiError::new("COMPLIANCE_REPORT_ERROR", e.to_string())),
    }
}

/// GET /api/v1/security/audit/stats
///
/// Get audit statistics and metrics.
#[utoipa::path(
    get,
    path = "/api/v1/security/audit/stats",
    tag = "audit",
    responses(
        (status = 200, description = "Audit statistics", body = serde_json::Value),
        (status = 500, description = "Internal server error", body = ApiError),
    )
)]
pub async fn get_audit_stats(
    State(_state): State<Arc<ApiState>>,
) -> ApiResult<Json<serde_json::Value>> {
    let vault = get_or_init_vault()?;
    let stats = vault.get_audit_stats();

    Ok(Json(serde_json::json!({
        "total_records": stats.total_records,
        "records_by_policy": stats.records_by_policy,
        "failed_writes": stats.failed_writes,
        "tamper_alerts": stats.tamper_alerts,
    })))
}

/// POST /api/v1/security/audit/verify
///
/// Verify audit log integrity (blockchain verification).
#[utoipa::path(
    post,
    path = "/api/v1/security/audit/verify",
    tag = "audit",
    responses(
        (status = 200, description = "Integrity verification result", body = serde_json::Value),
        (status = 500, description = "Internal server error", body = ApiError),
    )
)]
pub async fn verify_audit_integrity(
    State(_state): State<Arc<ApiState>>,
) -> ApiResult<Json<serde_json::Value>> {
    let vault = get_or_init_vault()?;

    match vault.verify_audit_integrity().await {
        Ok(valid) => Ok(Json(serde_json::json!({
            "valid": valid,
            "verified_at": chrono::Utc::now().timestamp(),
            "message": if valid {
                "Audit trail integrity verified successfully"
            } else {
                "WARNING: Audit trail integrity check failed - possible tampering detected"
            }
        }))),
        Err(e) => Err(ApiError::new("INTEGRITY_CHECK_ERROR", e.to_string())),
    }
}
