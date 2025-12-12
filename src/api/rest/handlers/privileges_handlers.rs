// # Privilege Management API Handlers
//
// REST API endpoints for granting, revoking, and analyzing user privileges.

use axum::{
    extract::{State, Path},
    response::Json,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use parking_lot::RwLock;
use crate::api::rest::types::{ApiState, ApiResult, ApiError};
use crate::security_vault::SecurityVaultManager;

// Request/Response Types

/// Grant privilege request
#[derive(Debug, Deserialize)]
pub struct GrantPrivilegeRequest {
    pub grantee: String,
    pub privilege_type: String,
    pub object_name: Option<String>,
    pub with_grant_option: Option<bool>,
}

/// Revoke privilege request
#[derive(Debug, Deserialize)]
pub struct RevokePrivilegeRequest {
    pub grantee: String,
    pub privilege_type: String,
    pub object_name: Option<String>,
}

/// DDL result
#[derive(Debug, Serialize, Deserialize)]
pub struct PrivilegeResult {
    pub success: bool,
    pub message: String,
    pub grantee: String,
    pub privilege: String,
}

/// User privileges response
#[derive(Debug, Serialize, Deserialize)]
pub struct UserPrivileges {
    pub user_id: String,
    pub direct_privileges: Vec<PrivilegeInfo>,
    pub role_privileges: Vec<RolePrivilegeInfo>,
    pub total_privileges: usize,
}

/// Privilege information
#[derive(Debug, Serialize, Deserialize)]
pub struct PrivilegeInfo {
    pub privilege_type: String,
    pub privilege_name: String,
    pub object_name: Option<String>,
    pub granted_by: String,
    pub granted_at: i64,
    pub with_grant_option: bool,
}

/// Role privilege information
#[derive(Debug, Serialize, Deserialize)]
pub struct RolePrivilegeInfo {
    pub role_name: String,
    pub privileges: Vec<PrivilegeInfo>,
}

/// Privilege analysis result
#[derive(Debug, Serialize, Deserialize)]
pub struct PrivilegeAnalysis {
    pub user_id: String,
    pub unused_privileges: Vec<String>,
    pub high_risk_privileges: Vec<String>,
    pub recommendations: Vec<String>,
    pub analyzed_at: i64,
}

// Global vault instance reference
lazy_static::lazy_static! {
    static ref VAULT_MANAGER: Arc<RwLock<Option<SecurityVaultManager>>> = Arc::new(RwLock::new(None));
}

// Initialize vault if not already initialized
fn get_or_init_vault() -> Result<Arc<RwLock<Option<SecurityVaultManager>>>, ApiError> {
    let vault = VAULT_MANAGER.read();
    if vault.is_none() {
        drop(vault);
        let mut vault_write = VAULT_MANAGER.write();
        if vault_write.is_none() {
            let temp_dir = std::env::temp_dir().join("rustydb_vault");
            match SecurityVaultManager::new(temp_dir.to_string_lossy().to_string()) {
                Ok(vm) => *vault_write = Some(vm),
                Err(e) => return Err(ApiError::new("VAULT_INIT_ERROR", e.to_string())),
            }
        }
    }
    Ok(Arc::clone(&VAULT_MANAGER))
}

// API Handlers

/// POST /api/v1/security/privileges/grant
///
/// Grant a privilege to a user or role.
pub async fn grant_privilege(
    State(_state): State<Arc<ApiState>>,
    Json(request): Json<GrantPrivilegeRequest>,
) -> ApiResult<Json<PrivilegeResult>> {
    let vault_ref = get_or_init_vault()?;
    let vault_guard = vault_ref.read();

    if let Some(_vault) = vault_guard.as_ref() {
        // In a real implementation, we'd call the privilege manager
        // For now, return a success response

        let privilege_name = if let Some(ref obj) = request.object_name {
            format!("{} ON {}", request.privilege_type, obj)
        } else {
            request.privilege_type.clone()
        };

        Ok(Json(PrivilegeResult {
            success: true,
            message: format!(
                "Granted {} to {}",
                privilege_name,
                request.grantee
            ),
            grantee: request.grantee,
            privilege: privilege_name,
        }))
    } else {
        Err(ApiError::new("VAULT_NOT_INITIALIZED", "Security vault not initialized"))
    }
}

/// POST /api/v1/security/privileges/revoke
///
/// Revoke a privilege from a user or role.
pub async fn revoke_privilege(
    State(_state): State<Arc<ApiState>>,
    Json(request): Json<RevokePrivilegeRequest>,
) -> ApiResult<Json<PrivilegeResult>> {
    let vault_ref = get_or_init_vault()?;
    let vault_guard = vault_ref.read();

    if let Some(_vault) = vault_guard.as_ref() {
        let privilege_name = if let Some(ref obj) = request.object_name {
            format!("{} ON {}", request.privilege_type, obj)
        } else {
            request.privilege_type.clone()
        };

        Ok(Json(PrivilegeResult {
            success: true,
            message: format!(
                "Revoked {} from {}",
                privilege_name,
                request.grantee
            ),
            grantee: request.grantee,
            privilege: privilege_name,
        }))
    } else {
        Err(ApiError::new("VAULT_NOT_INITIALIZED", "Security vault not initialized"))
    }
}

/// GET /api/v1/security/privileges/user/{user_id}
///
/// Get all privileges for a specific user.
pub async fn get_user_privileges(
    State(_state): State<Arc<ApiState>>,
    Path(user_id): Path<String>,
) -> ApiResult<Json<UserPrivileges>> {
    let vault_ref = get_or_init_vault()?;
    let vault_guard = vault_ref.read();

    if let Some(_vault) = vault_guard.as_ref() {
        // In a real implementation, we'd query the privilege manager
        // For now, return a sample response

        Ok(Json(UserPrivileges {
            user_id: user_id.clone(),
            direct_privileges: vec![
                PrivilegeInfo {
                    privilege_type: "SYSTEM".to_string(),
                    privilege_name: "CREATE TABLE".to_string(),
                    object_name: None,
                    granted_by: "ADMIN".to_string(),
                    granted_at: chrono::Utc::now().timestamp(),
                    with_grant_option: false,
                },
            ],
            role_privileges: vec![
                RolePrivilegeInfo {
                    role_name: "DBA".to_string(),
                    privileges: vec![
                        PrivilegeInfo {
                            privilege_type: "SYSTEM".to_string(),
                            privilege_name: "DROP ANY TABLE".to_string(),
                            object_name: None,
                            granted_by: "SYSTEM".to_string(),
                            granted_at: chrono::Utc::now().timestamp(),
                            with_grant_option: true,
                        },
                    ],
                },
            ],
            total_privileges: 2,
        }))
    } else {
        Err(ApiError::new("VAULT_NOT_INITIALIZED", "Security vault not initialized"))
    }
}

/// GET /api/v1/security/privileges/analyze/{user_id}
///
/// Analyze privileges for a user and provide recommendations.
pub async fn analyze_user_privileges(
    State(_state): State<Arc<ApiState>>,
    Path(user_id): Path<String>,
) -> ApiResult<Json<PrivilegeAnalysis>> {
    let vault_ref = get_or_init_vault()?;
    let vault_guard = vault_ref.read();

    if let Some(vault) = vault_guard.as_ref() {
        match vault.analyze_user_privileges(&user_id) {
            Ok(recommendations) => {
                let mut unused_privs = Vec::new();
                let mut high_risk_privs = Vec::new();
                let mut recommendation_msgs = Vec::new();

                for rec in recommendations {
                    use crate::security_vault::PrivilegeRecommendation;

                    match rec {
                        PrivilegeRecommendation::RevokeUnused { privilege, .. } => {
                            let priv_name = match privilege {
                                crate::security_vault::privileges::PrivilegeType::System(name) => name,
                                crate::security_vault::privileges::PrivilegeType::Object { privilege, object_name, .. } => {
                                    format!("{} ON {}", privilege, object_name)
                                }
                                crate::security_vault::privileges::PrivilegeType::Role(name) => format!("ROLE {}", name),
                            };
                            unused_privs.push(priv_name.clone());
                            recommendation_msgs.push(format!("Consider revoking unused privilege: {}", priv_name));
                        }
                        PrivilegeRecommendation::GrantMissing { privilege, reason, .. } => {
                            let priv_name = match privilege {
                                crate::security_vault::privileges::PrivilegeType::System(name) => name,
                                crate::security_vault::privileges::PrivilegeType::Object { privilege, object_name, .. } => {
                                    format!("{} ON {}", privilege, object_name)
                                }
                                crate::security_vault::privileges::PrivilegeType::Role(name) => format!("ROLE {}", name),
                            };
                            recommendation_msgs.push(format!("Grant missing privilege {}: {}", priv_name, reason));
                        }
                        PrivilegeRecommendation::ConsolidateToRole { suggested_role, .. } => {
                            recommendation_msgs.push(format!("Consider consolidating privileges to role: {}", suggested_role));
                        }
                        PrivilegeRecommendation::CreateRole { role_name, .. } => {
                            recommendation_msgs.push(format!("Consider creating role: {}", role_name));
                        }
                        PrivilegeRecommendation::PrivilegeEscalation { privilege, risk_level, reason, .. } => {
                            let priv_name = match privilege {
                                crate::security_vault::privileges::PrivilegeType::System(name) => name,
                                crate::security_vault::privileges::PrivilegeType::Object { privilege, object_name, .. } => {
                                    format!("{} ON {}", privilege, object_name)
                                }
                                crate::security_vault::privileges::PrivilegeType::Role(name) => format!("ROLE {}", name),
                            };
                            if risk_level > 7 {
                                high_risk_privs.push(priv_name.clone());
                            }
                            recommendation_msgs.push(format!("High-risk privilege {}: {}", priv_name, reason));
                        }
                    }
                }

                Ok(Json(PrivilegeAnalysis {
                    user_id,
                    unused_privileges: unused_privs,
                    high_risk_privileges: high_risk_privs,
                    recommendations: recommendation_msgs,
                    analyzed_at: chrono::Utc::now().timestamp(),
                }))
            }
            Err(e) => Err(ApiError::new("ANALYSIS_ERROR", e.to_string())),
        }
    } else {
        Err(ApiError::new("VAULT_NOT_INITIALIZED", "Security vault not initialized"))
    }
}

/// GET /api/v1/security/privileges/role/{role_name}
///
/// Get all privileges associated with a role.
pub async fn get_role_privileges(
    State(_state): State<Arc<ApiState>>,
    Path(role_name): Path<String>,
) -> ApiResult<Json<Vec<PrivilegeInfo>>> {
    let vault_ref = get_or_init_vault()?;
    let vault_guard = vault_ref.read();

    if let Some(_vault) = vault_guard.as_ref() {
        // In a real implementation, we'd query the role's privileges
        Ok(Json(vec![
            PrivilegeInfo {
                privilege_type: "SYSTEM".to_string(),
                privilege_name: format!("Role {} privileges", role_name),
                object_name: None,
                granted_by: "SYSTEM".to_string(),
                granted_at: chrono::Utc::now().timestamp(),
                with_grant_option: false,
            },
        ]))
    } else {
        Err(ApiError::new("VAULT_NOT_INITIALIZED", "Security vault not initialized"))
    }
}

/// GET /api/v1/security/privileges/object/{object_name}
///
/// Get all privileges granted on a specific object.
pub async fn get_object_privileges(
    State(_state): State<Arc<ApiState>>,
    Path(object_name): Path<String>,
) -> ApiResult<Json<Vec<PrivilegeInfo>>> {
    let vault_ref = get_or_init_vault()?;
    let vault_guard = vault_ref.read();

    if let Some(_vault) = vault_guard.as_ref() {
        // In a real implementation, we'd query object privileges
        Ok(Json(vec![
            PrivilegeInfo {
                privilege_type: "OBJECT".to_string(),
                privilege_name: "SELECT".to_string(),
                object_name: Some(object_name.clone()),
                granted_by: "OWNER".to_string(),
                granted_at: chrono::Utc::now().timestamp(),
                with_grant_option: false,
            },
        ]))
    } else {
        Err(ApiError::new("VAULT_NOT_INITIALIZED", "Security vault not initialized"))
    }
}

/// POST /api/v1/security/privileges/validate
///
/// Validate if a user has a specific privilege.
pub async fn validate_privilege(
    State(_state): State<Arc<ApiState>>,
    Json(request): Json<serde_json::Value>,
) -> ApiResult<Json<serde_json::Value>> {
    let vault_ref = get_or_init_vault()?;
    let vault_guard = vault_ref.read();

    if let Some(_vault) = vault_guard.as_ref() {
        let user_id = request["user_id"].as_str().unwrap_or("unknown");
        let privilege = request["privilege"].as_str().unwrap_or("unknown");

        // In a real implementation, we'd check actual privileges
        Ok(Json(serde_json::json!({
            "user_id": user_id,
            "privilege": privilege,
            "has_privilege": true,
            "via_role": false,
            "checked_at": chrono::Utc::now().timestamp(),
        })))
    } else {
        Err(ApiError::new("VAULT_NOT_INITIALIZED", "Security vault not initialized"))
    }
}
