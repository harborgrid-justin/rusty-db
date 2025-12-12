// # Virtual Private Database (VPD) API Handlers
//
// REST API endpoints for managing VPD policies, including row-level security
// and dynamic predicate injection.

use axum::{
    extract::{State, Path},
    response::Json,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::collections::HashMap;
use parking_lot::RwLock;
use crate::api::rest::types::{ApiState, ApiResult, ApiError};
use crate::security_vault::{SecurityVaultManager, VpdPolicy, SecurityPredicate};

// Request/Response Types

/// VPD policy response
#[derive(Debug, Serialize, Deserialize)]
pub struct VpdPolicyResponse {
    pub name: String,
    pub table_name: String,
    pub schema_name: Option<String>,
    pub predicate: String,
    pub policy_scope: String,
    pub enabled: bool,
    pub created_at: i64,
}

/// Create VPD policy request
#[derive(Debug, Deserialize)]
pub struct CreateVpdPolicy {
    pub name: String,
    pub table_name: String,
    pub schema_name: Option<String>,
    pub predicate: String,
    pub policy_scope: Option<String>,
}

/// Update VPD policy request
#[derive(Debug, Deserialize)]
pub struct UpdateVpdPolicy {
    pub enabled: Option<bool>,
    pub predicate: Option<String>,
    pub policy_scope: Option<String>,
}

/// Test VPD predicate request
#[derive(Debug, Deserialize)]
pub struct TestVpdPredicate {
    pub predicate: String,
    pub context: HashMap<String, String>,
}

/// Test VPD predicate result
#[derive(Debug, Serialize, Deserialize)]
pub struct TestVpdPredicateResult {
    pub original_predicate: String,
    pub evaluated_predicate: String,
    pub context_used: HashMap<String, String>,
    pub valid_sql: bool,
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

// Convert internal VpdPolicy to API response
fn policy_to_response(policy: &VpdPolicy) -> VpdPolicyResponse {
    VpdPolicyResponse {
        name: policy.name.clone(),
        table_name: policy.table_name.clone(),
        schema_name: policy.schema_name.clone(),
        predicate: format!("{:?}", policy.predicate),
        policy_scope: format!("{:?}", policy.scope),
        enabled: policy.enabled,
        created_at: policy.created_at,
    }
}

// API Handlers

/// GET /api/v1/security/vpd/policies
///
/// List all VPD policies.
pub async fn list_vpd_policies(
    State(_state): State<Arc<ApiState>>,
) -> ApiResult<Json<Vec<VpdPolicyResponse>>> {
    let vault_ref = get_or_init_vault()?;
    let vault_guard = vault_ref.read();

    if let Some(vault) = vault_guard.as_ref() {
        let vpd_engine = vault.vpd_engine();
        let vpd_guard = vpd_engine.read();

        let policy_names = vpd_guard.list_policies();
        let mut responses = Vec::new();

        for name in policy_names {
            if let Some(policy) = vpd_guard.get_policy(&name) {
                responses.push(policy_to_response(&policy));
            }
        }

        Ok(Json(responses))
    } else {
        Err(ApiError::new("VAULT_NOT_INITIALIZED", "Security vault not initialized"))
    }
}

/// GET /api/v1/security/vpd/policies/{name}
///
/// Get a specific VPD policy by name.
pub async fn get_vpd_policy(
    State(_state): State<Arc<ApiState>>,
    Path(name): Path<String>,
) -> ApiResult<Json<VpdPolicyResponse>> {
    let vault_ref = get_or_init_vault()?;
    let vault_guard = vault_ref.read();

    if let Some(vault) = vault_guard.as_ref() {
        let vpd_engine = vault.vpd_engine();
        let vpd_guard = vpd_engine.read();

        match vpd_guard.get_policy(&name) {
            Some(policy) => Ok(Json(policy_to_response(&policy))),
            None => Err(ApiError::new("POLICY_NOT_FOUND", format!("Policy '{}' not found", name))),
        }
    } else {
        Err(ApiError::new("VAULT_NOT_INITIALIZED", "Security vault not initialized"))
    }
}

/// POST /api/v1/security/vpd/policies
///
/// Create a new VPD policy.
pub async fn create_vpd_policy(
    State(_state): State<Arc<ApiState>>,
    Json(request): Json<CreateVpdPolicy>,
) -> ApiResult<Json<VpdPolicyResponse>> {
    let vault_ref = get_or_init_vault()?;
    let mut vault_guard = vault_ref.write();

    if let Some(vault) = vault_guard.as_mut() {
        match vault.create_vpd_policy(
            &request.table_name,
            &request.predicate,
        ).await {
            Ok(_) => {
                // Retrieve the created policy
                let vpd_engine = vault.vpd_engine();
                let vpd_guard = vpd_engine.read();

                match vpd_guard.get_policy(&request.name) {
                    Some(policy) => Ok(Json(policy_to_response(&policy))),
                    None => Err(ApiError::new("POLICY_RETRIEVAL_ERROR", format!("Policy '{}' not found after creation", request.name))),
                }
            }
            Err(e) => Err(ApiError::new("POLICY_CREATE_ERROR", e.to_string())),
        }
    } else {
        Err(ApiError::new("VAULT_NOT_INITIALIZED", "Security vault not initialized"))
    }
}

/// PUT /api/v1/security/vpd/policies/{name}
///
/// Update an existing VPD policy.
pub async fn update_vpd_policy(
    State(_state): State<Arc<ApiState>>,
    Path(name): Path<String>,
    Json(request): Json<UpdateVpdPolicy>,
) -> ApiResult<Json<VpdPolicyResponse>> {
    let vault_ref = get_or_init_vault()?;
    let vault_guard = vault_ref.read();

    if let Some(vault) = vault_guard.as_ref() {
        let vpd_engine = vault.vpd_engine();
        let vpd_guard = vpd_engine.write();

        // Get the current policy
        match vpd_guard.get_policy(&name) {
            Some(mut policy) => {
                // Update fields if provided
                if let Some(enabled) = request.enabled {
                    if enabled {
                        drop(vpd_guard);
                        let vpd_engine = vault.vpd_engine();
                        let mut vpd_guard = vpd_engine.write();
                        vpd_guard.enable_policy(&name)
                            .map_err(|e| ApiError::new("POLICY_UPDATE_ERROR", e.to_string()))?;
                    } else {
                        drop(vpd_guard);
                        let vpd_engine = vault.vpd_engine();
                        let mut vpd_guard = vpd_engine.write();
                        vpd_guard.disable_policy(&name)
                            .map_err(|e| ApiError::new("POLICY_UPDATE_ERROR", e.to_string()))?;
                    }
                    policy.enabled = enabled;
                }
                if let Some(predicate_str) = request.predicate {
                    match SecurityPredicate::parse(&predicate_str) {
                        Ok(predicate) => policy.predicate = predicate,
                        Err(e) => return Err(ApiError::new("INVALID_PREDICATE", e.to_string())),
                    }
                }
                Ok(Json(policy_to_response(&policy)))
            }
            None => Err(ApiError::new("POLICY_NOT_FOUND", format!("Policy '{}' not found", name))),
        }
    } else {
        Err(ApiError::new("VAULT_NOT_INITIALIZED", "Security vault not initialized"))
    }
}

/// DELETE /api/v1/security/vpd/policies/{name}
///
/// Delete a VPD policy.
pub async fn delete_vpd_policy(
    State(_state): State<Arc<ApiState>>,
    Path(name): Path<String>,
) -> ApiResult<Json<serde_json::Value>> {
    let vault_ref = get_or_init_vault()?;
    let vault_guard = vault_ref.read();

    if let Some(vault) = vault_guard.as_ref() {
        let vpd_engine = vault.vpd_engine();
        let mut vpd_guard = vpd_engine.write();

        match vpd_guard.drop_policy(&name) {
            Ok(_) => {
                Ok(Json(serde_json::json!({
                    "success": true,
                    "message": format!("VPD policy '{}' deleted successfully", name),
                })))
            }
            Err(e) => Err(ApiError::new("POLICY_DELETE_ERROR", e.to_string())),
        }
    } else {
        Err(ApiError::new("VAULT_NOT_INITIALIZED", "Security vault not initialized"))
    }
}

/// POST /api/v1/security/vpd/test-predicate
///
/// Test a VPD predicate with sample context.
pub async fn test_vpd_predicate(
    State(_state): State<Arc<ApiState>>,
    Json(request): Json<TestVpdPredicate>,
) -> ApiResult<Json<TestVpdPredicateResult>> {
    // Parse the predicate
    match SecurityPredicate::parse(&request.predicate) {
        Ok(predicate) => {
            // Evaluate the predicate with the provided context
            match predicate.evaluate(&request.context) {
                Ok(evaluated) => {
                    // Simple SQL validation (check for basic SQL keywords)
                    let valid_sql = evaluated.contains("=") ||
                                    evaluated.contains(">") ||
                                    evaluated.contains("<") ||
                                    evaluated.contains("AND") ||
                                    evaluated.contains("OR");

                    Ok(Json(TestVpdPredicateResult {
                        original_predicate: request.predicate,
                        evaluated_predicate: evaluated,
                        context_used: request.context,
                        valid_sql,
                    }))
                }
                Err(e) => Err(ApiError::new("PREDICATE_EVAL_ERROR", e.to_string())),
            }
        }
        Err(e) => Err(ApiError::new("PREDICATE_PARSE_ERROR", e.to_string())),
    }
}

/// GET /api/v1/security/vpd/policies/table/{table_name}
///
/// Get all VPD policies for a specific table.
pub async fn get_table_policies(
    State(_state): State<Arc<ApiState>>,
    Path(table_name): Path<String>,
) -> ApiResult<Json<Vec<VpdPolicyResponse>>> {
    let vault_ref = get_or_init_vault()?;
    let vault_guard = vault_ref.read();

    if let Some(vault) = vault_guard.as_ref() {
        let vpd_engine = vault.vpd_engine();
        let vpd_guard = vpd_engine.read();

        // Filter policies by table name
        let policy_names = vpd_guard.list_policies();
        let mut responses = Vec::new();

        for name in policy_names {
            if let Some(policy) = vpd_guard.get_policy(&name) {
                if policy.table_name == table_name {
                    responses.push(policy_to_response(&policy));
                }
            }
        }

        Ok(Json(responses))
    } else {
        Err(ApiError::new("VAULT_NOT_INITIALIZED", "Security vault not initialized"))
    }
}

/// POST /api/v1/security/vpd/policies/{name}/enable
///
/// Enable a VPD policy.
pub async fn enable_vpd_policy(
    State(_state): State<Arc<ApiState>>,
    Path(name): Path<String>,
) -> ApiResult<Json<serde_json::Value>> {
    let vault_ref = get_or_init_vault()?;
    let vault_guard = vault_ref.read();

    if let Some(vault) = vault_guard.as_ref() {
        let vpd_engine = vault.vpd_engine();
        let mut vpd_guard = vpd_engine.write();

        match vpd_guard.enable_policy(&name) {
            Ok(_) => {
                Ok(Json(serde_json::json!({
                    "success": true,
                    "message": format!("VPD policy '{}' enabled", name),
                })))
            }
            Err(e) => Err(ApiError::new("POLICY_ENABLE_ERROR", e.to_string())),
        }
    } else {
        Err(ApiError::new("VAULT_NOT_INITIALIZED", "Security vault not initialized"))
    }
}

/// POST /api/v1/security/vpd/policies/{name}/disable
///
/// Disable a VPD policy.
pub async fn disable_vpd_policy(
    State(_state): State<Arc<ApiState>>,
    Path(name): Path<String>,
) -> ApiResult<Json<serde_json::Value>> {
    let vault_ref = get_or_init_vault()?;
    let vault_guard = vault_ref.read();

    if let Some(vault) = vault_guard.as_ref() {
        let vpd_engine = vault.vpd_engine();
        let mut vpd_guard = vpd_engine.write();

        match vpd_guard.disable_policy(&name) {
            Ok(_) => {
                Ok(Json(serde_json::json!({
                    "success": true,
                    "message": format!("VPD policy '{}' disabled", name),
                })))
            }
            Err(e) => Err(ApiError::new("POLICY_DISABLE_ERROR", e.to_string())),
        }
    } else {
        Err(ApiError::new("VAULT_NOT_INITIALIZED", "Security vault not initialized"))
    }
}
