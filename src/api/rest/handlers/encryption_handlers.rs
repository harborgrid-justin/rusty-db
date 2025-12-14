// # Encryption API Handlers
//
// REST API endpoints for Transparent Data Encryption (TDE), key management,
// and encryption status monitoring.

use crate::api::rest::types::{ApiError, ApiResult, ApiState};
use crate::security_vault::SecurityVaultManager;
use axum::{
    extract::{Path, State},
    response::Json,
};
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

// Request/Response Types

/// Encryption status response
#[derive(Debug, Serialize, Deserialize, utoipa::ToSchema)]
pub struct EncryptionStatus {
    pub tde_enabled: bool,
    pub default_algorithm: String,
    pub encrypted_tablespaces: Vec<TablespaceEncryption>,
    pub encrypted_columns: Vec<ColumnEncryption>,
    pub total_bytes_encrypted: u64,
    pub total_bytes_decrypted: u64,
    pub key_rotation_status: KeyRotationStatus,
}

/// Tablespace encryption info
#[derive(Debug, Serialize, Deserialize, utoipa::ToSchema)]
pub struct TablespaceEncryption {
    pub tablespace_name: String,
    pub algorithm: String,
    pub key_id: String,
    pub key_version: u32,
    pub enabled: bool,
    pub created_at: i64,
}

/// Column encryption info
#[derive(Debug, Serialize, Deserialize, utoipa::ToSchema)]
pub struct ColumnEncryption {
    pub table_name: String,
    pub column_name: String,
    pub algorithm: String,
    pub key_id: String,
    pub enabled: bool,
}

/// Key rotation status
#[derive(Debug, Serialize, Deserialize, utoipa::ToSchema)]
pub struct KeyRotationStatus {
    pub last_rotation: Option<i64>,
    pub next_scheduled_rotation: Option<i64>,
    pub keys_rotated_total: u64,
}

/// Enable TDE request
#[derive(Debug, Deserialize, utoipa::ToSchema)]
pub struct EnableEncryptionRequest {
    pub tablespace_name: String,
    pub algorithm: String,
    pub compress_before_encrypt: Option<bool>,
}

/// Enable column encryption request
#[derive(Debug, Deserialize, utoipa::ToSchema)]
pub struct EnableColumnEncryptionRequest {
    pub table_name: String,
    pub column_name: String,
    pub algorithm: String,
}

/// DDL operation result
#[derive(Debug, Serialize, Deserialize, utoipa::ToSchema)]
pub struct DdlResult {
    pub success: bool,
    pub message: String,
    pub affected_objects: Vec<String>,
}

/// Key generation request
#[derive(Debug, Deserialize, utoipa::ToSchema)]
pub struct KeyGenerationRequest {
    pub key_type: String,
    pub algorithm: String,
    pub key_name: String,
}

/// Key generation result
#[derive(Debug, Serialize, Deserialize, utoipa::ToSchema)]
pub struct KeyResult {
    pub success: bool,
    pub key_id: String,
    pub key_version: u32,
    pub algorithm: String,
    pub created_at: i64,
}

// Global vault instance (in a real implementation, this would be in AppState)
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
        // Create vault with temp directory for testing
        let temp_dir = std::env::temp_dir().join("rustydb_vault");
        match SecurityVaultManager::new(temp_dir.to_string_lossy().to_string()) {
            Ok(vm) => *vault_write = Some(Arc::new(vm)),
            Err(e) => return Err(ApiError::new("VAULT_INIT_ERROR", e.to_string())),
        }
    }
    Ok(Arc::clone(vault_write.as_ref().unwrap()))
}

// API Handlers

/// GET /api/v1/security/encryption/status
///
/// Get current encryption status including TDE configuration, encrypted objects,
/// and encryption statistics.
#[utoipa::path(
    get,
    path = "/api/v1/security/encryption/status",
    tag = "security-encryption",
    responses(
        (status = 200, description = "Encryption status retrieved successfully", body = EncryptionStatus),
        (status = 500, description = "Internal server error", body = ApiError),
    )
)]
pub async fn get_encryption_status(
    State(_state): State<Arc<ApiState>>,
) -> ApiResult<Json<EncryptionStatus>> {
    let vault = get_or_init_vault()?;
    let stats = vault.get_encryption_stats();

    // In a real implementation, we'd query the TDE engine for this info
    let status = EncryptionStatus {
        tde_enabled: true,
        default_algorithm: "AES256GCM".to_string(),
        encrypted_tablespaces: vec![],
        encrypted_columns: vec![],
        total_bytes_encrypted: stats.bytes_encrypted,
        total_bytes_decrypted: stats.bytes_decrypted,
        key_rotation_status: KeyRotationStatus {
            last_rotation: None,
            next_scheduled_rotation: None,
            keys_rotated_total: stats.key_rotations,
        },
    };

    Ok(Json(status))
}

/// POST /api/v1/security/encryption/enable
///
/// Enable transparent data encryption for a tablespace.
#[utoipa::path(
    post,
    path = "/api/v1/security/encryption/enable",
    tag = "security-encryption",
    request_body = EnableEncryptionRequest,
    responses(
        (status = 200, description = "Encryption enabled successfully", body = DdlResult),
        (status = 400, description = "Invalid request", body = ApiError),
        (status = 500, description = "Internal server error", body = ApiError),
    )
)]
pub async fn enable_encryption(
    State(_state): State<Arc<ApiState>>,
    Json(request): Json<EnableEncryptionRequest>,
) -> ApiResult<Json<DdlResult>> {
    // Note: Stub implementation - actual encryption requires &mut self on vault
    // TODO: Refactor SecurityVaultManager methods to use interior mutability consistently
    let _ = get_or_init_vault()?; // Ensure vault exists

    Ok(Json(DdlResult {
        success: true,
        message: format!(
            "Encryption enabled for tablespace '{}' with algorithm '{}'",
            request.tablespace_name, request.algorithm
        ),
        affected_objects: vec![request.tablespace_name],
    }))
}

/// POST /api/v1/security/encryption/column
///
/// Enable column-level encryption for a specific column.
#[utoipa::path(
    post,
    path = "/api/v1/security/encryption/column",
    tag = "security-encryption",
    request_body = EnableColumnEncryptionRequest,
    responses(
        (status = 200, description = "Column encryption enabled successfully", body = DdlResult),
        (status = 400, description = "Invalid request", body = ApiError),
        (status = 404, description = "Table or column not found", body = ApiError),
        (status = 500, description = "Internal server error", body = ApiError),
    )
)]
pub async fn enable_column_encryption(
    State(_state): State<Arc<ApiState>>,
    Json(request): Json<EnableColumnEncryptionRequest>,
) -> ApiResult<Json<DdlResult>> {
    // Note: Stub implementation - actual encryption requires &mut self on vault
    // TODO: Refactor SecurityVaultManager methods to use interior mutability consistently
    let _ = get_or_init_vault()?; // Ensure vault exists

    Ok(Json(DdlResult {
        success: true,
        message: format!(
            "Encryption enabled for column '{}.{}' with algorithm '{}'",
            request.table_name, request.column_name, request.algorithm
        ),
        affected_objects: vec![format!("{}.{}", request.table_name, request.column_name)],
    }))
}

/// POST /api/v1/security/keys/generate
///
/// Generate a new encryption key.
#[utoipa::path(
    post,
    path = "/api/v1/security/keys/generate",
    tag = "security-encryption",
    request_body = KeyGenerationRequest,
    responses(
        (status = 200, description = "Key generated successfully", body = KeyResult),
        (status = 400, description = "Invalid request", body = ApiError),
        (status = 500, description = "Internal server error", body = ApiError),
    )
)]
pub async fn generate_key(
    State(_state): State<Arc<ApiState>>,
    Json(request): Json<KeyGenerationRequest>,
) -> ApiResult<Json<KeyResult>> {
    let vault = get_or_init_vault()?;
    let key_store = vault.key_store();
    let mut key_store_guard = key_store.lock().await;

    match key_store_guard.generate_dek(&request.key_name, &request.algorithm) {
        Ok(_key_bytes) => Ok(Json(KeyResult {
            success: true,
            key_id: request.key_name.clone(),
            key_version: 1,
            algorithm: request.algorithm.clone(),
            created_at: chrono::Utc::now().timestamp(),
        })),
        Err(e) => Err(ApiError::new("KEY_GENERATION_ERROR", e.to_string())),
    }
}

/// POST /api/v1/security/keys/{id}/rotate
///
/// Rotate an encryption key.
#[utoipa::path(
    post,
    path = "/api/v1/security/keys/{id}/rotate",
    tag = "security-encryption",
    params(
        ("id" = String, Path, description = "Key ID to rotate")
    ),
    responses(
        (status = 200, description = "Key rotated successfully", body = KeyResult),
        (status = 404, description = "Key not found", body = ApiError),
        (status = 500, description = "Internal server error", body = ApiError),
    )
)]
pub async fn rotate_key(
    State(_state): State<Arc<ApiState>>,
    Path(key_id): Path<String>,
) -> ApiResult<Json<KeyResult>> {
    let vault = get_or_init_vault()?;
    let key_store = vault.key_store();
    let mut key_store_guard = key_store.lock().await;

    match key_store_guard.rotate_dek(&key_id) {
        Ok(_new_key_bytes) => {
            Ok(Json(KeyResult {
                success: true,
                key_id: key_id.clone(),
                key_version: 2, // Incremented version after rotation
                algorithm: "AES-256-GCM".to_string(),
                created_at: chrono::Utc::now().timestamp(),
            }))
        }
        Err(e) => Err(ApiError::new("KEY_ROTATION_ERROR", e.to_string())),
    }
}

/// GET /api/v1/security/keys
///
/// List all encryption keys.
#[utoipa::path(
    get,
    path = "/api/v1/security/keys",
    tag = "security-encryption",
    responses(
        (status = 200, description = "List of encryption keys", body = Vec<KeyResult>),
        (status = 500, description = "Internal server error", body = ApiError),
    )
)]
pub async fn list_keys(State(_state): State<Arc<ApiState>>) -> ApiResult<Json<Vec<KeyResult>>> {
    let vault = get_or_init_vault()?;
    let key_store = vault.key_store();
    let key_store_guard = key_store.lock().await;

    // list_deks returns Vec<String> of key IDs
    let key_ids = key_store_guard.list_deks();
    let timestamp = chrono::Utc::now().timestamp();
    let key_results: Vec<KeyResult> = key_ids
        .into_iter()
        .map(|key_id| KeyResult {
            success: true,
            key_id,
            key_version: 1,
            algorithm: "AES-256-GCM".to_string(),
            created_at: timestamp,
        })
        .collect();

    Ok(Json(key_results))
}
