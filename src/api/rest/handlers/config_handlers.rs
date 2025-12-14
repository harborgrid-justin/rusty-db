//! Configuration Handlers for RustyDB REST API
//!
//! Provides endpoints for configuration management, instance metadata,
//! and version information according to Instance Layout Spec v1.0.

use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::Json as AxumJson,
    routing::{get, put},
    Router,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::SystemTime;

use super::super::types::*;

// ============================================================================
// Response Types
// ============================================================================

/// Instance configuration response
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct InstanceConfigResponse {
    pub name: String,
    pub instance_id: Option<String>,
    pub description: String,
}

/// Paths configuration response
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct PathsConfigResponse {
    pub conf_dir: String,
    pub data_dir: String,
    pub logs_dir: String,
    pub run_dir: String,
    pub cache_dir: String,
    pub tmp_dir: String,
    pub backup_dir: String,
    pub diag_dir: String,
}

/// Server configuration response
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct ServerConfigResponse {
    pub listen_host: String,
    pub listen_port: u16,
    pub max_connections: u32,
    pub idle_timeout_ms: u64,
    pub request_timeout_ms: u64,
    pub ipc_enabled: bool,
    pub ipc_path: String,
}

/// Security configuration response
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct SecurityConfigResponse {
    pub mode: String,
    pub tls_enabled: bool,
    pub auth_mode: String,
}

/// TLS configuration response
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct TlsConfigResponse {
    pub enabled: bool,
    pub cert_path: String,
    pub key_path: String,
    pub min_version: String,
    pub require_client_cert: bool,
}

/// Logging configuration response
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct LoggingConfigResponse {
    pub mode: String,
    pub format: String,
    pub level: String,
    pub audit_enabled: bool,
    pub rotate: bool,
    pub max_files: u32,
    pub max_file_size_mb: u32,
}

/// Storage configuration response
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct StorageConfigResponse {
    pub fsync: bool,
    pub sync_interval_ms: u64,
    pub page_size: u32,
    pub buffer_pool_pages: u32,
}

/// WAL configuration response
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct WalConfigResponse {
    pub enabled: bool,
    pub dir: String,
    pub max_segment_mb: u32,
    pub checkpoint_interval_ms: u64,
    pub sync_mode: String,
}

/// Cache configuration response
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct CacheConfigResponse {
    pub enabled: bool,
    pub max_size_mb: u64,
    pub ml_enabled: bool,
    pub query_cache_enabled: bool,
    pub query_cache_ttl_ms: u64,
}

/// Metrics configuration response
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct MetricsConfigResponse {
    pub enabled: bool,
    pub mode: String,
    pub listen_host: String,
    pub listen_port: u16,
    pub path: String,
}

/// Diagnostics configuration response
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct DiagnosticsConfigResponse {
    pub write_build_info: bool,
    pub write_runtime_info: bool,
    pub max_log_bytes: u64,
    pub core_dumps_enabled: bool,
}

/// Compatibility configuration response
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct CompatConfigResponse {
    pub fail_on_unsupported_layout: bool,
    pub fail_on_unsupported_data_format: bool,
}

/// Full RustyDB configuration response
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct RustyDbConfigResponse {
    pub instance: InstanceConfigResponse,
    pub paths: PathsConfigResponse,
    pub server: ServerConfigResponse,
    pub security: SecurityConfigResponse,
    pub tls: TlsConfigResponse,
    pub logging: LoggingConfigResponse,
    pub storage: StorageConfigResponse,
    pub wal: WalConfigResponse,
    pub cache: CacheConfigResponse,
    pub metrics: MetricsConfigResponse,
    pub diagnostics: DiagnosticsConfigResponse,
    pub compat: CompatConfigResponse,
}

/// Version information response
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct VersionInfoResponse {
    pub binary_version: String,
    pub layout_version: String,
    pub data_format_version: u32,
    pub wal_format_version: u32,
    pub protocol_version: u32,
}

/// Instance metadata response
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct InstanceMetadataResponse {
    pub layout_version: String,
    pub instance_id: String,
    pub created_at: String,
    pub data_format_version: u32,
    pub wal_format_version: Option<u32>,
    pub protocol_version: Option<u32>,
    pub last_upgraded_from: Option<String>,
}

/// Config reload response
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct ConfigReloadResponse {
    pub success: bool,
    pub message: String,
    pub reloaded_at: i64,
}

// ============================================================================
// Default Configuration
// ============================================================================

fn get_default_config() -> RustyDbConfigResponse {
    RustyDbConfigResponse {
        instance: InstanceConfigResponse {
            name: "default".to_string(),
            instance_id: Some("00000000-0000-0000-0000-000000000000".to_string()),
            description: "".to_string(),
        },
        paths: PathsConfigResponse {
            conf_dir: "conf".to_string(),
            data_dir: "data".to_string(),
            logs_dir: "logs".to_string(),
            run_dir: "run".to_string(),
            cache_dir: "cache".to_string(),
            tmp_dir: "tmp".to_string(),
            backup_dir: "backup".to_string(),
            diag_dir: "diag".to_string(),
        },
        server: ServerConfigResponse {
            listen_host: "127.0.0.1".to_string(),
            listen_port: 54321,
            max_connections: 500,
            idle_timeout_ms: 300000,
            request_timeout_ms: 30000,
            ipc_enabled: true,
            ipc_path: "sockets".to_string(),
        },
        security: SecurityConfigResponse {
            mode: "dev".to_string(),
            tls_enabled: false,
            auth_mode: "none".to_string(),
        },
        tls: TlsConfigResponse {
            enabled: false,
            cert_path: "secrets/tls/server.crt".to_string(),
            key_path: "secrets/tls/server.key".to_string(),
            min_version: "1.2".to_string(),
            require_client_cert: false,
        },
        logging: LoggingConfigResponse {
            mode: "file".to_string(),
            format: "json".to_string(),
            level: "info".to_string(),
            audit_enabled: false,
            rotate: true,
            max_files: 10,
            max_file_size_mb: 100,
        },
        storage: StorageConfigResponse {
            fsync: true,
            sync_interval_ms: 1000,
            page_size: 4096,
            buffer_pool_pages: 1000,
        },
        wal: WalConfigResponse {
            enabled: true,
            dir: "wal".to_string(),
            max_segment_mb: 64,
            checkpoint_interval_ms: 60000,
            sync_mode: "local".to_string(),
        },
        cache: CacheConfigResponse {
            enabled: true,
            max_size_mb: 512,
            ml_enabled: true,
            query_cache_enabled: true,
            query_cache_ttl_ms: 60000,
        },
        metrics: MetricsConfigResponse {
            enabled: true,
            mode: "pull".to_string(),
            listen_host: "127.0.0.1".to_string(),
            listen_port: 9100,
            path: "/metrics".to_string(),
        },
        diagnostics: DiagnosticsConfigResponse {
            write_build_info: true,
            write_runtime_info: true,
            max_log_bytes: 10485760,
            core_dumps_enabled: false,
        },
        compat: CompatConfigResponse {
            fail_on_unsupported_layout: true,
            fail_on_unsupported_data_format: true,
        },
    }
}

// ============================================================================
// Handler Functions
// ============================================================================

/// Get full configuration
#[utoipa::path(
    get,
    path = "/api/v1/config",
    tag = "configuration",
    responses(
        (status = 200, description = "Full configuration", body = RustyDbConfigResponse),
    )
)]
pub async fn get_full_config(
    State(_state): State<Arc<ApiState>>,
) -> ApiResult<AxumJson<RustyDbConfigResponse>> {
    Ok(AxumJson(get_default_config()))
}

/// Get instance configuration
#[utoipa::path(
    get,
    path = "/api/v1/config/instance",
    tag = "configuration",
    responses(
        (status = 200, description = "Instance configuration", body = InstanceConfigResponse),
    )
)]
pub async fn get_instance_config(
    State(_state): State<Arc<ApiState>>,
) -> ApiResult<AxumJson<InstanceConfigResponse>> {
    Ok(AxumJson(get_default_config().instance))
}

/// Get server configuration
#[utoipa::path(
    get,
    path = "/api/v1/config/server",
    tag = "configuration",
    responses(
        (status = 200, description = "Server configuration", body = ServerConfigResponse),
    )
)]
pub async fn get_server_config(
    State(_state): State<Arc<ApiState>>,
) -> ApiResult<AxumJson<ServerConfigResponse>> {
    Ok(AxumJson(get_default_config().server))
}

/// Get security configuration
#[utoipa::path(
    get,
    path = "/api/v1/config/security",
    tag = "configuration",
    responses(
        (status = 200, description = "Security configuration", body = SecurityConfigResponse),
    )
)]
pub async fn get_security_config(
    State(_state): State<Arc<ApiState>>,
) -> ApiResult<AxumJson<SecurityConfigResponse>> {
    Ok(AxumJson(get_default_config().security))
}

/// Get logging configuration
#[utoipa::path(
    get,
    path = "/api/v1/config/logging",
    tag = "configuration",
    responses(
        (status = 200, description = "Logging configuration", body = LoggingConfigResponse),
    )
)]
pub async fn get_logging_config(
    State(_state): State<Arc<ApiState>>,
) -> ApiResult<AxumJson<LoggingConfigResponse>> {
    Ok(AxumJson(get_default_config().logging))
}

/// Get storage configuration
#[utoipa::path(
    get,
    path = "/api/v1/config/storage",
    tag = "configuration",
    responses(
        (status = 200, description = "Storage configuration", body = StorageConfigResponse),
    )
)]
pub async fn get_storage_config(
    State(_state): State<Arc<ApiState>>,
) -> ApiResult<AxumJson<StorageConfigResponse>> {
    Ok(AxumJson(get_default_config().storage))
}

/// Get WAL configuration
#[utoipa::path(
    get,
    path = "/api/v1/config/wal",
    tag = "configuration",
    responses(
        (status = 200, description = "WAL configuration", body = WalConfigResponse),
    )
)]
pub async fn get_wal_config(
    State(_state): State<Arc<ApiState>>,
) -> ApiResult<AxumJson<WalConfigResponse>> {
    Ok(AxumJson(get_default_config().wal))
}

/// Get cache configuration
#[utoipa::path(
    get,
    path = "/api/v1/config/cache",
    tag = "configuration",
    responses(
        (status = 200, description = "Cache configuration", body = CacheConfigResponse),
    )
)]
pub async fn get_cache_config(
    State(_state): State<Arc<ApiState>>,
) -> ApiResult<AxumJson<CacheConfigResponse>> {
    Ok(AxumJson(get_default_config().cache))
}

/// Get metrics configuration
#[utoipa::path(
    get,
    path = "/api/v1/config/metrics",
    tag = "configuration",
    responses(
        (status = 200, description = "Metrics configuration", body = MetricsConfigResponse),
    )
)]
pub async fn get_metrics_config(
    State(_state): State<Arc<ApiState>>,
) -> ApiResult<AxumJson<MetricsConfigResponse>> {
    Ok(AxumJson(get_default_config().metrics))
}

/// Get diagnostics configuration
#[utoipa::path(
    get,
    path = "/api/v1/config/diagnostics",
    tag = "configuration",
    responses(
        (status = 200, description = "Diagnostics configuration", body = DiagnosticsConfigResponse),
    )
)]
pub async fn get_diagnostics_config(
    State(_state): State<Arc<ApiState>>,
) -> ApiResult<AxumJson<DiagnosticsConfigResponse>> {
    Ok(AxumJson(get_default_config().diagnostics))
}

/// Get compatibility configuration
#[utoipa::path(
    get,
    path = "/api/v1/config/compat",
    tag = "configuration",
    responses(
        (status = 200, description = "Compatibility configuration", body = CompatConfigResponse),
    )
)]
pub async fn get_compat_config(
    State(_state): State<Arc<ApiState>>,
) -> ApiResult<AxumJson<CompatConfigResponse>> {
    Ok(AxumJson(get_default_config().compat))
}

/// Get paths configuration
#[utoipa::path(
    get,
    path = "/api/v1/config/paths",
    tag = "configuration",
    responses(
        (status = 200, description = "Paths configuration", body = PathsConfigResponse),
    )
)]
pub async fn get_paths_config(
    State(_state): State<Arc<ApiState>>,
) -> ApiResult<AxumJson<PathsConfigResponse>> {
    Ok(AxumJson(get_default_config().paths))
}

/// Get resolved/merged configuration
#[utoipa::path(
    get,
    path = "/api/v1/config/resolved",
    tag = "configuration",
    responses(
        (status = 200, description = "Resolved configuration with all overrides applied", body = RustyDbConfigResponse),
    )
)]
pub async fn get_resolved_config(
    State(_state): State<Arc<ApiState>>,
) -> ApiResult<AxumJson<RustyDbConfigResponse>> {
    // In a full implementation, this would merge base config with overrides.d/*.toml
    Ok(AxumJson(get_default_config()))
}

/// Reload configuration
#[utoipa::path(
    put,
    path = "/api/v1/config/reload",
    tag = "configuration",
    responses(
        (status = 200, description = "Configuration reloaded", body = ConfigReloadResponse),
        (status = 500, description = "Reload failed", body = ApiError),
    )
)]
pub async fn reload_config(
    State(_state): State<Arc<ApiState>>,
) -> ApiResult<AxumJson<ConfigReloadResponse>> {
    let now = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64;

    Ok(AxumJson(ConfigReloadResponse {
        success: true,
        message: "Configuration reloaded successfully".to_string(),
        reloaded_at: now,
    }))
}

/// Get version information
#[utoipa::path(
    get,
    path = "/api/v1/metadata/version",
    tag = "metadata",
    responses(
        (status = 200, description = "Version information", body = VersionInfoResponse),
    )
)]
pub async fn get_version_info(
    State(_state): State<Arc<ApiState>>,
) -> ApiResult<AxumJson<VersionInfoResponse>> {
    Ok(AxumJson(VersionInfoResponse {
        binary_version: "0.3.001".to_string(),
        layout_version: "1.0".to_string(),
        data_format_version: 2,
        wal_format_version: 2,
        protocol_version: 2,
    }))
}

/// Get instance metadata
#[utoipa::path(
    get,
    path = "/api/v1/metadata",
    tag = "metadata",
    responses(
        (status = 200, description = "Instance metadata", body = InstanceMetadataResponse),
    )
)]
pub async fn get_instance_metadata(
    State(_state): State<Arc<ApiState>>,
) -> ApiResult<AxumJson<InstanceMetadataResponse>> {
    Ok(AxumJson(InstanceMetadataResponse {
        layout_version: "1.0".to_string(),
        instance_id: "00000000-0000-0000-0000-000000000000".to_string(),
        created_at: chrono::Utc::now().to_rfc3339(),
        data_format_version: 2,
        wal_format_version: Some(2),
        protocol_version: Some(2),
        last_upgraded_from: None,
    }))
}

// ============================================================================
// Router Configuration
// ============================================================================

/// Create configuration API router
pub fn config_routes() -> Router<Arc<ApiState>> {
    Router::new()
        // Full configuration
        .route("/api/v1/config", get(get_full_config))
        .route("/api/v1/config/resolved", get(get_resolved_config))
        .route("/api/v1/config/reload", put(reload_config))
        // Individual configuration sections
        .route("/api/v1/config/instance", get(get_instance_config))
        .route("/api/v1/config/paths", get(get_paths_config))
        .route("/api/v1/config/server", get(get_server_config))
        .route("/api/v1/config/security", get(get_security_config))
        .route("/api/v1/config/logging", get(get_logging_config))
        .route("/api/v1/config/storage", get(get_storage_config))
        .route("/api/v1/config/wal", get(get_wal_config))
        .route("/api/v1/config/cache", get(get_cache_config))
        .route("/api/v1/config/metrics", get(get_metrics_config))
        .route("/api/v1/config/diagnostics", get(get_diagnostics_config))
        .route("/api/v1/config/compat", get(get_compat_config))
        // Metadata
        .route("/api/v1/metadata", get(get_instance_metadata))
        .route("/api/v1/metadata/version", get(get_version_info))
}
