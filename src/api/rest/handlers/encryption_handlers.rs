// # Encryption API Handlers
//
// REST API endpoints for Transparent Data Encryption (TDE), key management,
// and encryption status monitoring.

use axum::{
    extract::{State, Path, Query},
    response::Json,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use parking_lot::RwLock;
use crate::api::rest::types::{ApiState, ApiResult, ApiError};
use crate::security_vault::{SecurityVaultManager, EncryptionAlgorithm, TdeConfig};

// Request/Response Types

/// Encryption status response
#[derive(Debug, Serialize, Deserialize)]
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
#[derive(Debug, Serialize, Deserialize)]
pub struct TablespaceEncryption {
    pub tablespace_name: String,
    pub algorithm: String,
    pub key_id: String,
    pub key_version: u32,
    pub enabled: bool,
    pub created_at: i64,
}

/// Column encryption info
#[derive(Debug, Serialize, Deserialize)]
pub struct ColumnEncryption {
    pub table_name: String,
    pub column_name: String,
    pub algorithm: String,
    pub key_id: String,
    pub enabled: bool,
}

/// Key rotation status
#[derive(Debug, Serialize, Deserialize)]
pub struct KeyRotationStatus {
    pub last_rotation: Option<i64>,
    pub next_scheduled_rotation: Option<i64>,
    pub keys_rotated_total: u64,
}

/// Enable TDE request
#[derive(Debug, Deserialize)]
pub struct EnableEncryptionRequest {
    pub tablespace_name: String,
    pub algorithm: String,
    pub compress_before_encrypt: Option<bool>,
}

/// Enable column encryption request
#[derive(Debug, Deserialize)]
pub struct EnableColumnEncryptionRequest {
    pub table_name: String,
    pub column_name: String,
    pub algorithm: String,
}

/// DDL operation result
#[derive(Debug, Serialize, Deserialize)]
pub struct DdlResult {
    pub success: bool,
    pub message: String,
    pub affected_objects: Vec<String>,
}

/// Key generation request
#[derive(Debug, Deserialize)]
pub struct KeyGenerationRequest {
    pub key_type: String,
    pub algorithm: String,
    pub key_name: String,
}

/// Key generation result
#[derive(Debug, Serialize, Deserialize)]
pub struct KeyResult {
    pub success: bool,
    pub key_id: String,
    pub key_version: u32,
    pub algorithm: String,
    pub created_at: i64,
}

// Global vault instance (in a real implementation, this would be in AppState)
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
            // Create vault with temp directory for testing
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

/// GET /api/v1/security/encryption/status
///
/// Get current encryption status including TDE configuration, encrypted objects,
/// and encryption statistics.
pub async fn get_encryption_status(
    State(_state): State<Arc<ApiState>>,
) -> ApiResult<Json<EncryptionStatus>> {
    let vault_ref = get_or_init_vault()?;
    let vault_guard = vault_ref.read();

    if let Some(vault) = vault_guard.as_ref() {
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
    } else {
        Err(ApiError::new("VAULT_NOT_INITIALIZED", "Security vault not initialized"))
    }
}

/// POST /api/v1/security/encryption/enable
///
/// Enable transparent data encryption for a tablespace.
pub async fn enable_encryption(
    State(_state): State<Arc<ApiState>>,
    Json(request): Json<EnableEncryptionRequest>,
) -> ApiResult<Json<DdlResult>> {
    let vault_ref = get_or_init_vault()?;
    let mut vault_guard = vault_ref.write();

    if let Some(vault) = vault_guard.as_mut() {
        match vault.enable_tablespace_encryption(
            &request.tablespace_name,
            &request.algorithm,
        ).await {
            Ok(_) => Ok(Json(DdlResult {
                success: true,
                message: format!("Encryption enabled for tablespace '{}'", request.tablespace_name),
                affected_objects: vec![request.tablespace_name],
            })),
            Err(e) => Err(ApiError::new("ENCRYPTION_ERROR", e.to_string())),
        }
    } else {
        Err(ApiError::new("VAULT_NOT_INITIALIZED", "Security vault not initialized"))
    }
}

/// POST /api/v1/security/encryption/column
///
/// Enable column-level encryption for a specific column.
pub async fn enable_column_encryption(
    State(_state): State<Arc<ApiState>>,
    Json(request): Json<EnableColumnEncryptionRequest>,
) -> ApiResult<Json<DdlResult>> {
    let vault_ref = get_or_init_vault()?;
    let mut vault_guard = vault_ref.write();

    if let Some(vault) = vault_guard.as_mut() {
        match vault.enable_column_encryption(
            &request.table_name,
            &request.column_name,
            &request.algorithm,
        ).await {
            Ok(_) => Ok(Json(DdlResult {
                success: true,
                message: format!(
                    "Encryption enabled for column '{}.{}'",
                    request.table_name, request.column_name
                ),
                affected_objects: vec![format!("{}.{}", request.table_name, request.column_name)],
            })),
            Err(e) => Err(ApiError::new("ENCRYPTION_ERROR", e.to_string())),
        }
    } else {
        Err(ApiError::new("VAULT_NOT_INITIALIZED", "Security vault not initialized"))
    }
}

/// POST /api/v1/security/keys/generate
///
/// Generate a new encryption key.
pub async fn generate_key(
    State(_state): State<Arc<ApiState>>,
    Json(request): Json<KeyGenerationRequest>,
) -> ApiResult<Json<KeyResult>> {
    let vault_ref = get_or_init_vault()?;
    let vault_guard = vault_ref.read();

    if let Some(vault) = vault_guard.as_ref() {
        let key_store = vault.key_store();
        let mut key_store_guard = key_store.lock().await;

        match key_store_guard.generate_dek(&request.key_name, &request.algorithm) {
            Ok(dek) => {
                Ok(Json(KeyResult {
                    success: true,
                    key_id: dek.key_id,
                    key_version: dek.version,
                    algorithm: format!("{:?}", dek.algorithm),
                    created_at: dek.created_at,
                }))
            }
            Err(e) => Err(ApiError::new("KEY_GENERATION_ERROR", e.to_string())),
        }
    } else {
        Err(ApiError::new("VAULT_NOT_INITIALIZED", "Security vault not initialized"))
    }
}

/// POST /api/v1/security/keys/{id}/rotate
///
/// Rotate an encryption key.
pub async fn rotate_key(
    State(_state): State<Arc<ApiState>>,
    Path(key_id): Path<String>,
) -> ApiResult<Json<KeyResult>> {
    let vault_ref = get_or_init_vault()?;
    let vault_guard = vault_ref.read();

    if let Some(vault) = vault_guard.as_ref() {
        let key_store = vault.key_store();
        let mut key_store_guard = key_store.lock().await;

        match key_store_guard.rotate_dek(&key_id) {
            Ok(new_dek) => {
                Ok(Json(KeyResult {
                    success: true,
                    key_id: new_dek.key_id,
                    key_version: new_dek.version,
                    algorithm: format!("{:?}", new_dek.algorithm),
                    created_at: new_dek.created_at,
                }))
            }
            Err(e) => Err(ApiError::new("KEY_ROTATION_ERROR", e.to_string())),
        }
    } else {
        Err(ApiError::new("VAULT_NOT_INITIALIZED", "Security vault not initialized"))
    }
}

/// GET /api/v1/security/keys
///
/// List all encryption keys.
pub async fn list_keys(
    State(_state): State<Arc<ApiState>>,
) -> ApiResult<Json<Vec<KeyResult>>> {
    let vault_ref = get_or_init_vault()?;
    let vault_guard = vault_ref.read();

    if let Some(vault) = vault_guard.as_ref() {
        let key_store = vault.key_store();
        let key_store_guard = key_store.lock().await;

        match key_store_guard.list_deks() {
            Ok(keys) => {
                let key_results: Vec<KeyResult> = keys.into_iter().map(|dek| KeyResult {
                    success: true,
                    key_id: dek.key_id,
                    key_version: dek.version,
                    algorithm: format!("{:?}", dek.algorithm),
                    created_at: dek.created_at,
                }).collect();

                Ok(Json(key_results))
            }
            Err(e) => Err(ApiError::new("KEY_LIST_ERROR", e.to_string())),
        }
    } else {
        Err(ApiError::new("VAULT_NOT_INITIALIZED", "Security vault not initialized"))
    }
}
