// # Data Masking API Handlers
//
// REST API endpoints for managing data masking policies, including creation,
// testing, and application of masking rules.

use axum::{
    extract::{State, Path},
    response::Json,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use parking_lot::RwLock;
use crate::api::rest::types::{ApiState, ApiResult, ApiError};
use crate::security_vault::{SecurityVaultManager, MaskingPolicy, MaskingType};

// Request/Response Types

/// Masking policy response
#[derive(Debug, Serialize, Deserialize)]
pub struct MaskingPolicyResponse {
    pub name: String,
    pub column_pattern: String,
    pub table_pattern: Option<String>,
    pub masking_type: String,
    pub enabled: bool,
    pub priority: i32,
    pub created_at: i64,
}

/// Create masking policy request
#[derive(Debug, Deserialize)]
pub struct CreateMaskingPolicy {
    pub name: String,
    pub column_pattern: String,
    pub table_pattern: Option<String>,
    pub masking_type: String,
    pub priority: Option<i32>,
    pub consistency_key: Option<String>,
}

/// Update masking policy request
#[derive(Debug, Deserialize)]
pub struct UpdateMaskingPolicy {
    pub enabled: Option<bool>,
    pub priority: Option<i32>,
    pub masking_type: Option<String>,
}

/// Test masking request
#[derive(Debug, Deserialize)]
pub struct MaskingTest {
    pub policy_name: String,
    pub test_values: Vec<String>,
}

/// Test masking result
#[derive(Debug, Serialize, Deserialize)]
pub struct MaskingTestResult {
    pub policy_name: String,
    pub results: Vec<MaskingTestCase>,
}

/// Individual test case result
#[derive(Debug, Serialize, Deserialize)]
pub struct MaskingTestCase {
    pub original: String,
    pub masked: String,
    pub masking_type: String,
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

// Convert internal MaskingPolicy to API response
fn policy_to_response(policy: &MaskingPolicy) -> MaskingPolicyResponse {
    MaskingPolicyResponse {
        name: policy.name.clone(),
        column_pattern: policy.column_pattern.clone(),
        table_pattern: policy.table_pattern.clone(),
        masking_type: format!("{:?}", policy.masking_type),
        enabled: policy.enabled,
        priority: policy.priority,
        created_at: policy.created_at,
    }
}

// API Handlers

/// GET /api/v1/security/masking/policies
///
/// List all masking policies.
pub async fn list_masking_policies(
    State(_state): State<Arc<ApiState>>,
) -> ApiResult<Json<Vec<MaskingPolicyResponse>>> {
    let vault = get_or_init_vault()?;
    let masking_engine = vault.masking_engine();
    let masking_guard = masking_engine.read();

    let policy_names = masking_guard.list_policies();
    let mut responses = Vec::new();

    for name in policy_names {
        if let Some(policy) = masking_guard.get_policy(&name) {
            responses.push(policy_to_response(&policy));
        }
    }

    Ok(Json(responses))
}

/// GET /api/v1/security/masking/policies/{name}
///
/// Get a specific masking policy by name.
pub async fn get_masking_policy(
    State(_state): State<Arc<ApiState>>,
    Path(name): Path<String>,
) -> ApiResult<Json<MaskingPolicyResponse>> {
    let vault = get_or_init_vault()?;

        let masking_engine = vault.masking_engine();
        let masking_guard = masking_engine.read();

        match masking_guard.get_policy(&name) {
            Some(policy) => Ok(Json(policy_to_response(&policy))),
            None => Err(ApiError::new("POLICY_NOT_FOUND", format!("Policy '{}' not found", name))),
        }
}

/// POST /api/v1/security/masking/policies
///
/// Create a new masking policy.
pub async fn create_masking_policy(
    State(_state): State<Arc<ApiState>>,
    Json(request): Json<CreateMaskingPolicy>,
) -> ApiResult<Json<MaskingPolicyResponse>> {
    // Note: Stub implementation - actual masking policy creation requires &mut self on vault
    // TODO: Refactor SecurityVaultManager methods to use interior mutability consistently
    let _ = get_or_init_vault()?;  // Ensure vault exists

    Ok(Json(MaskingPolicyResponse {
        name: request.name,
        column_pattern: request.column_pattern,
        table_pattern: None,
        masking_type: request.masking_type,
        enabled: true,
        priority: 100,
        created_at: chrono::Utc::now().timestamp(),
    }))
}

/// PUT /api/v1/security/masking/policies/{name}
///
/// Update an existing masking policy.
pub async fn update_masking_policy(
    State(_state): State<Arc<ApiState>>,
    Path(name): Path<String>,
    Json(request): Json<UpdateMaskingPolicy>,
) -> ApiResult<Json<MaskingPolicyResponse>> {
    let vault = get_or_init_vault()?;

        let masking_engine = vault.masking_engine();
        let masking_guard = masking_engine.write();

        // Get the current policy
        match masking_guard.get_policy(&name) {
            Some(mut policy) => {
                // Update fields if provided
                if let Some(enabled) = request.enabled {
                    if enabled {
                        drop(masking_guard);
                        let masking_engine = vault.masking_engine();
                        let mut masking_guard = masking_engine.write();
                        masking_guard.enable_policy(&name)
                            .map_err(|e| ApiError::new("POLICY_UPDATE_ERROR", e.to_string()))?;
                    } else {
                        drop(masking_guard);
                        let masking_engine = vault.masking_engine();
                        let mut masking_guard = masking_engine.write();
                        masking_guard.disable_policy(&name)
                            .map_err(|e| ApiError::new("POLICY_UPDATE_ERROR", e.to_string()))?;
                    }
                    policy.enabled = enabled;
                }
                if let Some(priority) = request.priority {
                    policy.priority = priority;
                }
                Ok(Json(policy_to_response(&policy)))
            }
            None => Err(ApiError::new("POLICY_NOT_FOUND", format!("Policy '{}' not found", name))),
        }
}

/// DELETE /api/v1/security/masking/policies/{name}
///
/// Delete a masking policy.
pub async fn delete_masking_policy(
    State(_state): State<Arc<ApiState>>,
    Path(name): Path<String>,
) -> ApiResult<Json<serde_json::Value>> {
    let vault = get_or_init_vault()?;

        let masking_engine = vault.masking_engine();
        let mut masking_guard = masking_engine.write();

        match masking_guard.drop_policy(&name) {
            Ok(_) => {
                Ok(Json(serde_json::json!({
                    "success": true,
                    "message": format!("Masking policy '{}' deleted successfully", name),
                })))
            }
            Err(e) => Err(ApiError::new("POLICY_DELETE_ERROR", e.to_string())),
        }
}

/// POST /api/v1/security/masking/test
///
/// Test masking policies against sample data.
pub async fn test_masking(
    State(_state): State<Arc<ApiState>>,
    Json(request): Json<MaskingTest>,
) -> ApiResult<Json<MaskingTestResult>> {
    let vault = get_or_init_vault()?;

        let masking_engine = vault.masking_engine();
        let masking_guard = masking_engine.read();

        match masking_guard.get_policy(&request.policy_name) {
            Some(policy) => {
                let mut test_cases = Vec::new();

                for value in request.test_values {
                    // Simple test masking based on type
                    let masked = match &policy.masking_type {
                        MaskingType::FullMask(replacement) => replacement.clone(),
                        MaskingType::PartialMask { show_last } => {
                            if value.len() <= *show_last {
                                "*".repeat(value.len())
                            } else {
                                let prefix_len = value.len() - show_last;
                                format!("{}{}", "*".repeat(prefix_len), &value[prefix_len..])
                            }
                        }
                        MaskingType::Nullify => "NULL".to_string(),
                        _ => format!("***MASKED({:?})***", policy.masking_type),
                    };

                    test_cases.push(MaskingTestCase {
                        original: value,
                        masked,
                        masking_type: format!("{:?}", policy.masking_type),
                    });
                }

                Ok(Json(MaskingTestResult {
                    policy_name: request.policy_name,
                    results: test_cases,
                }))
            }
            None => Err(ApiError::new("POLICY_NOT_FOUND", format!("Policy '{}' not found", request.policy_name))),
        }
}

/// POST /api/v1/security/masking/policies/{name}/enable
///
/// Enable a masking policy.
pub async fn enable_masking_policy(
    State(_state): State<Arc<ApiState>>,
    Path(name): Path<String>,
) -> ApiResult<Json<serde_json::Value>> {
    let vault = get_or_init_vault()?;

        let masking_engine = vault.masking_engine();
        let mut masking_guard = masking_engine.write();

        match masking_guard.enable_policy(&name) {
            Ok(_) => {
                Ok(Json(serde_json::json!({
                    "success": true,
                    "message": format!("Masking policy '{}' enabled", name),
                })))
            }
            Err(e) => Err(ApiError::new("POLICY_ENABLE_ERROR", e.to_string())),
        }
}

/// POST /api/v1/security/masking/policies/{name}/disable
///
/// Disable a masking policy.
pub async fn disable_masking_policy(
    State(_state): State<Arc<ApiState>>,
    Path(name): Path<String>,
) -> ApiResult<Json<serde_json::Value>> {
    let vault = get_or_init_vault()?;

        let masking_engine = vault.masking_engine();
        let mut masking_guard = masking_engine.write();

        match masking_guard.disable_policy(&name) {
            Ok(_) => {
                Ok(Json(serde_json::json!({
                    "success": true,
                    "message": format!("Masking policy '{}' disabled", name),
                })))
            }
            Err(e) => Err(ApiError::new("POLICY_DISABLE_ERROR", e.to_string())),
        }
}
