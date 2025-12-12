// # Security Core API Handlers
//
// REST API endpoints for RBAC (Role-Based Access Control) and threat detection.
// Provides role management, permission assignment, and insider threat monitoring.

use axum::{
    extract::{State, Path},
    response::Json,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use parking_lot::RwLock;
use crate::api::rest::types::{ApiState, ApiResult, ApiError};
use crate::security::rbac::{RbacManager, Role};
use crate::security::insider_threat::{InsiderThreatManager, ThreatStatistics};
use utoipa::ToSchema;

// ============================================================================
// Request/Response Types
// ============================================================================

/// Create role request
#[derive(Debug, Deserialize, ToSchema)]
pub struct CreateRoleRequest {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub parent_roles: Option<Vec<String>>,
    pub permissions: Option<Vec<String>>,
}

/// Update role request
#[derive(Debug, Deserialize, ToSchema)]
pub struct UpdateRoleRequest {
    pub name: Option<String>,
    pub description: Option<String>,
    pub parent_roles: Option<Vec<String>>,
    pub permissions: Option<Vec<String>>,
    pub is_active: Option<bool>,
}

/// Role response
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct RoleResponse {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub parent_roles: Vec<String>,
    pub permissions: Vec<String>,
    pub is_active: bool,
    pub created_at: i64,
    pub updated_at: i64,
    pub owner: Option<String>,
    pub priority: i32,
}

impl From<Role> for RoleResponse {
    fn from(role: Role) -> Self {
        Self {
            id: role.id,
            name: role.name,
            description: role.description,
            parent_roles: role.parent_roles,
            permissions: role.permissions.into_iter().collect(),
            is_active: role.is_active,
            created_at: role.created_at,
            updated_at: role.updated_at,
            owner: role.owner,
            priority: role.priority,
        }
    }
}

/// Assign permissions request
#[derive(Debug, Deserialize, ToSchema)]
pub struct AssignPermissionsRequest {
    pub permissions: Vec<String>,
}

/// Permission response
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct PermissionResponse {
    pub id: String,
    pub name: String,
    pub description: String,
    pub category: String,
}

/// Threat status response
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct ThreatStatusResponse {
    pub enabled: bool,
    pub total_assessments: usize,
    pub critical_threats: usize,
    pub high_threats: usize,
    pub blocked_queries: usize,
    pub exfiltration_attempts: usize,
    pub escalation_attempts: usize,
    pub baselines_established: usize,
}

impl From<ThreatStatistics> for ThreatStatusResponse {
    fn from(stats: ThreatStatistics) -> Self {
        Self {
            enabled: true,
            total_assessments: stats.total_assessments,
            critical_threats: stats.critical_threats,
            high_threats: stats.high_threats,
            blocked_queries: stats.blocked_queries,
            exfiltration_attempts: stats.exfiltration_attempts,
            escalation_attempts: stats.escalation_attempts,
            baselines_established: stats.baselines_established,
        }
    }
}

/// Threat history item
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct ThreatHistoryItem {
    pub assessment_id: String,
    pub user_id: String,
    pub query_text: String,
    pub total_score: u8,
    pub threat_level: String,
    pub risk_factors: Vec<String>,
    pub timestamp: i64,
    pub action: String,
    pub client_ip: Option<String>,
    pub location: Option<String>,
}

/// Insider threat status response
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct InsiderThreatStatusResponse {
    pub enabled: bool,
    pub auto_block_critical: bool,
    pub require_mfa_high_risk: bool,
    pub alert_threshold: u8,
    pub block_threshold: u8,
    pub behavioral_analytics_enabled: bool,
    pub anomaly_detection_enabled: bool,
    pub exfiltration_prevention_enabled: bool,
    pub escalation_detection_enabled: bool,
}

// ============================================================================
// Global Security Managers
// ============================================================================

lazy_static::lazy_static! {
    static ref RBAC_MANAGER: Arc<RwLock<RbacManager>> = Arc::new(RwLock::new(RbacManager::new()));
    static ref THREAT_MANAGER: Arc<RwLock<InsiderThreatManager>> = Arc::new(RwLock::new(InsiderThreatManager::new()));
}

// ============================================================================
// RBAC Handlers
// ============================================================================

/// GET /api/v1/security/roles
///
/// List all roles in the system.
#[utoipa::path(
    get,
    path = "/api/v1/security/roles",
    tag = "security",
    responses(
        (status = 200, description = "List of roles", body = Vec<RoleResponse>),
        (status = 500, description = "Internal server error", body = ApiError),
    )
)]
pub async fn list_roles(
    State(_state): State<Arc<ApiState>>,
) -> ApiResult<Json<Vec<RoleResponse>>> {
    let rbac = RBAC_MANAGER.read();
    let roles = rbac.get_all_roles();
    let response: Vec<RoleResponse> = roles.into_iter().map(|r| r.into()).collect();
    Ok(Json(response))
}

/// POST /api/v1/security/roles
///
/// Create a new role.
#[utoipa::path(
    post,
    path = "/api/v1/security/roles",
    tag = "security",
    request_body = CreateRoleRequest,
    responses(
        (status = 200, description = "Role created successfully", body = RoleResponse),
        (status = 400, description = "Invalid request", body = ApiError),
        (status = 409, description = "Role already exists", body = ApiError),
        (status = 500, description = "Internal server error", body = ApiError),
    )
)]
pub async fn create_role(
    State(_state): State<Arc<ApiState>>,
    Json(request): Json<CreateRoleRequest>,
) -> ApiResult<Json<RoleResponse>> {
    let mut role = Role::new(request.id.clone(), request.name);

    if let Some(desc) = request.description {
        role.description = Some(desc);
    }

    if let Some(parents) = request.parent_roles {
        role.parent_roles = parents;
    }

    if let Some(perms) = request.permissions {
        role.permissions = perms.into_iter().collect();
    }

    let rbac = RBAC_MANAGER.read();
    rbac.create_role(role.clone())
        .map_err(|e| ApiError::new("RBAC_ERROR", e.to_string()))?;

    Ok(Json(role.into()))
}

/// GET /api/v1/security/roles/{id}
///
/// Get a specific role by ID.
#[utoipa::path(
    get,
    path = "/api/v1/security/roles/{id}",
    tag = "security",
    params(
        ("id" = String, Path, description = "Role ID")
    ),
    responses(
        (status = 200, description = "Role details", body = RoleResponse),
        (status = 404, description = "Role not found", body = ApiError),
        (status = 500, description = "Internal server error", body = ApiError),
    )
)]
pub async fn get_role(
    State(_state): State<Arc<ApiState>>,
    Path(role_id): Path<String>,
) -> ApiResult<Json<RoleResponse>> {
    let rbac = RBAC_MANAGER.read();
    let role = rbac.get_role(&role_id)
        .map_err(|e| ApiError::new("RBAC_ERROR", e.to_string()))?;

    Ok(Json(role.into()))
}

/// PUT /api/v1/security/roles/{id}
///
/// Update an existing role.
#[utoipa::path(
    put,
    path = "/api/v1/security/roles/{id}",
    tag = "security",
    params(
        ("id" = String, Path, description = "Role ID")
    ),
    request_body = UpdateRoleRequest,
    responses(
        (status = 200, description = "Role updated successfully", body = RoleResponse),
        (status = 400, description = "Invalid request", body = ApiError),
        (status = 404, description = "Role not found", body = ApiError),
        (status = 500, description = "Internal server error", body = ApiError),
    )
)]
pub async fn update_role(
    State(_state): State<Arc<ApiState>>,
    Path(role_id): Path<String>,
    Json(request): Json<UpdateRoleRequest>,
) -> ApiResult<Json<RoleResponse>> {
    let rbac = RBAC_MANAGER.read();
    let mut role = rbac.get_role(&role_id)
        .map_err(|e| ApiError::new("RBAC_ERROR", e.to_string()))?;

    if let Some(name) = request.name {
        role.name = name;
    }

    if let Some(desc) = request.description {
        role.description = Some(desc);
    }

    if let Some(parents) = request.parent_roles {
        role.parent_roles = parents;
    }

    if let Some(perms) = request.permissions {
        role.permissions = perms.into_iter().collect();
    }

    if let Some(active) = request.is_active {
        role.is_active = active;
    }

    rbac.update_role(role.clone())
        .map_err(|e| ApiError::new("RBAC_ERROR", e.to_string()))?;

    Ok(Json(role.into()))
}

/// DELETE /api/v1/security/roles/{id}
///
/// Delete a role.
#[utoipa::path(
    delete,
    path = "/api/v1/security/roles/{id}",
    tag = "security",
    params(
        ("id" = String, Path, description = "Role ID")
    ),
    responses(
        (status = 200, description = "Role deleted successfully"),
        (status = 404, description = "Role not found", body = ApiError),
        (status = 409, description = "Cannot delete role (in use)", body = ApiError),
        (status = 500, description = "Internal server error", body = ApiError),
    )
)]
pub async fn delete_role(
    State(_state): State<Arc<ApiState>>,
    Path(role_id): Path<String>,
) -> ApiResult<Json<serde_json::Value>> {
    let rbac = RBAC_MANAGER.read();
    rbac.delete_role(&role_id)
        .map_err(|e| ApiError::new("RBAC_ERROR", e.to_string()))?;

    Ok(Json(serde_json::json!({
        "success": true,
        "message": format!("Role {} deleted successfully", role_id)
    })))
}

/// GET /api/v1/security/permissions
///
/// List all available permissions.
#[utoipa::path(
    get,
    path = "/api/v1/security/permissions",
    tag = "security",
    responses(
        (status = 200, description = "List of permissions", body = Vec<PermissionResponse>),
        (status = 500, description = "Internal server error", body = ApiError),
    )
)]
pub async fn list_permissions(
    State(_state): State<Arc<ApiState>>,
) -> ApiResult<Json<Vec<PermissionResponse>>> {
    // In a real implementation, this would query the permission catalog
    // For now, return a sample list of common permissions
    let permissions = vec![
        PermissionResponse {
            id: "select".to_string(),
            name: "SELECT".to_string(),
            description: "Read data from tables".to_string(),
            category: "DATA".to_string(),
        },
        PermissionResponse {
            id: "insert".to_string(),
            name: "INSERT".to_string(),
            description: "Insert data into tables".to_string(),
            category: "DATA".to_string(),
        },
        PermissionResponse {
            id: "update".to_string(),
            name: "UPDATE".to_string(),
            description: "Update existing data".to_string(),
            category: "DATA".to_string(),
        },
        PermissionResponse {
            id: "delete".to_string(),
            name: "DELETE".to_string(),
            description: "Delete data from tables".to_string(),
            category: "DATA".to_string(),
        },
        PermissionResponse {
            id: "create_table".to_string(),
            name: "CREATE TABLE".to_string(),
            description: "Create new tables".to_string(),
            category: "DDL".to_string(),
        },
        PermissionResponse {
            id: "drop_table".to_string(),
            name: "DROP TABLE".to_string(),
            description: "Drop existing tables".to_string(),
            category: "DDL".to_string(),
        },
        PermissionResponse {
            id: "create_user".to_string(),
            name: "CREATE USER".to_string(),
            description: "Create new users".to_string(),
            category: "ADMIN".to_string(),
        },
        PermissionResponse {
            id: "grant_privilege".to_string(),
            name: "GRANT PRIVILEGE".to_string(),
            description: "Grant privileges to other users".to_string(),
            category: "ADMIN".to_string(),
        },
    ];

    Ok(Json(permissions))
}

/// POST /api/v1/security/roles/{id}/permissions
///
/// Assign permissions to a role.
#[utoipa::path(
    post,
    path = "/api/v1/security/roles/{id}/permissions",
    tag = "security",
    params(
        ("id" = String, Path, description = "Role ID")
    ),
    request_body = AssignPermissionsRequest,
    responses(
        (status = 200, description = "Permissions assigned successfully", body = RoleResponse),
        (status = 404, description = "Role not found", body = ApiError),
        (status = 500, description = "Internal server error", body = ApiError),
    )
)]
pub async fn assign_permissions(
    State(_state): State<Arc<ApiState>>,
    Path(role_id): Path<String>,
    Json(request): Json<AssignPermissionsRequest>,
) -> ApiResult<Json<RoleResponse>> {
    let rbac = RBAC_MANAGER.read();
    let mut role = rbac.get_role(&role_id)
        .map_err(|e| ApiError::new("RBAC_ERROR", e.to_string()))?;

    // Add new permissions to the role
    for permission in request.permissions {
        role.add_permission(permission);
    }

    rbac.update_role(role.clone())
        .map_err(|e| ApiError::new("RBAC_ERROR", e.to_string()))?;

    Ok(Json(role.into()))
}

// ============================================================================
// Threat Detection Handlers
// ============================================================================

/// GET /api/v1/security/threats
///
/// Get current threat detection status and statistics.
#[utoipa::path(
    get,
    path = "/api/v1/security/threats",
    tag = "security",
    responses(
        (status = 200, description = "Threat detection status", body = ThreatStatusResponse),
        (status = 500, description = "Internal server error", body = ApiError),
    )
)]
pub async fn get_threat_status(
    State(_state): State<Arc<ApiState>>,
) -> ApiResult<Json<ThreatStatusResponse>> {
    let threat_mgr = THREAT_MANAGER.read();
    let stats = threat_mgr.get_statistics();

    Ok(Json(stats.into()))
}

/// GET /api/v1/security/threats/history
///
/// Get threat detection history (recent assessments).
#[utoipa::path(
    get,
    path = "/api/v1/security/threats/history",
    tag = "security",
    responses(
        (status = 200, description = "Threat history", body = Vec<ThreatHistoryItem>),
        (status = 500, description = "Internal server error", body = ApiError),
    )
)]
pub async fn get_threat_history(
    State(_state): State<Arc<ApiState>>,
) -> ApiResult<Json<Vec<ThreatHistoryItem>>> {
    // In a real implementation, we would access the forensic logger
    // through the InsiderThreatManager to get actual history
    // For now, return an empty list as the forensic logger is not directly accessible

    // Note: The InsiderThreatManager contains a ForensicLogger which has get_recent_records(),
    // but it's not publicly accessible. In a production system, we'd add a public method
    // to InsiderThreatManager to expose this functionality.

    Ok(Json(vec![]))
}

/// GET /api/v1/security/insider-threats
///
/// Get insider threat detection configuration and status.
#[utoipa::path(
    get,
    path = "/api/v1/security/insider-threats",
    tag = "security",
    responses(
        (status = 200, description = "Insider threat status", body = InsiderThreatStatusResponse),
        (status = 500, description = "Internal server error", body = ApiError),
    )
)]
pub async fn get_insider_threat_status(
    State(_state): State<Arc<ApiState>>,
) -> ApiResult<Json<InsiderThreatStatusResponse>> {
    let threat_mgr = THREAT_MANAGER.read();
    let config = threat_mgr.get_config();

    Ok(Json(InsiderThreatStatusResponse {
        enabled: config.enabled,
        auto_block_critical: config.auto_block_critical,
        require_mfa_high_risk: config.require_mfa_high_risk,
        alert_threshold: config.alert_threshold,
        block_threshold: config.block_threshold,
        behavioral_analytics_enabled: config.behavioral_analytics_enabled,
        anomaly_detection_enabled: config.anomaly_detection_enabled,
        exfiltration_prevention_enabled: config.exfiltration_prevention_enabled,
        escalation_detection_enabled: config.escalation_detection_enabled,
    }))
}
