//! GraphQL Configuration Types for RustyDB
//!
//! Provides GraphQL types for configuration management and instance metadata
//! according to Instance Layout Spec v1.0.

use async_graphql::{Enum, InputObject, Object, SimpleObject};
use serde::{Deserialize, Serialize};

// ============================================================================
// Enums
// ============================================================================

/// Security mode for the database instance
#[derive(Debug, Clone, Copy, PartialEq, Eq, Enum, Serialize, Deserialize)]
pub enum SecurityMode {
    /// Development mode - permissive defaults, local-only
    Dev,
    /// Production mode - safer defaults, stricter validation
    Prod,
}

impl Default for SecurityMode {
    fn default() -> Self {
        Self::Dev
    }
}

/// Authentication mode
#[derive(Debug, Clone, Copy, PartialEq, Eq, Enum, Serialize, Deserialize)]
pub enum AuthMode {
    /// No authentication (dev only)
    None,
    /// Username/password authentication
    Password,
    /// Mutual TLS client certificate
    Mtls,
    /// Token-based authentication (JWT, API keys)
    Token,
    /// LDAP/Active Directory
    Ldap,
    /// Kerberos authentication
    Kerberos,
}

impl Default for AuthMode {
    fn default() -> Self {
        Self::None
    }
}

/// Logging output mode
#[derive(Debug, Clone, Copy, PartialEq, Eq, Enum, Serialize, Deserialize)]
pub enum LoggingMode {
    /// Write logs to files
    File,
    /// Write logs to stdout/stderr
    Stdout,
}

impl Default for LoggingMode {
    fn default() -> Self {
        Self::File
    }
}

/// Log format
#[derive(Debug, Clone, Copy, PartialEq, Eq, Enum, Serialize, Deserialize)]
pub enum LogFormat {
    /// JSON format for log aggregation
    Json,
    /// Human-readable text format
    Text,
}

impl Default for LogFormat {
    fn default() -> Self {
        Self::Json
    }
}

/// Metrics exposure mode
#[derive(Debug, Clone, Copy, PartialEq, Eq, Enum, Serialize, Deserialize)]
pub enum MetricsMode {
    /// Prometheus-like scrape endpoint
    Pull,
    /// Push to metrics collector
    Push,
}

impl Default for MetricsMode {
    fn default() -> Self {
        Self::Pull
    }
}

/// WAL synchronous commit mode
#[derive(Debug, Clone, Copy, PartialEq, Eq, Enum, Serialize, Deserialize)]
pub enum WalSyncMode {
    /// No synchronous commit
    Off,
    /// Local synchronous commit
    Local,
    /// Remote write synchronous commit
    RemoteWrite,
    /// Remote apply synchronous commit
    RemoteApply,
}

impl Default for WalSyncMode {
    fn default() -> Self {
        Self::Local
    }
}

// ============================================================================
// Output Types (SimpleObject)
// ============================================================================

/// Instance configuration
#[derive(Debug, Clone, SimpleObject, Serialize, Deserialize)]
pub struct InstanceConfigGql {
    /// Logical instance name
    pub name: String,
    /// Stable instance identifier (UUID)
    pub instance_id: Option<String>,
    /// Human-readable description
    pub description: String,
}

/// Paths configuration
#[derive(Debug, Clone, SimpleObject, Serialize, Deserialize)]
pub struct PathsConfigGql {
    /// Configuration directory
    pub conf_dir: String,
    /// Data directory
    pub data_dir: String,
    /// Logs directory
    pub logs_dir: String,
    /// Runtime directory (PID, sockets)
    pub run_dir: String,
    /// Cache directory
    pub cache_dir: String,
    /// Temporary files directory
    pub tmp_dir: String,
    /// Backup directory
    pub backup_dir: String,
    /// Diagnostics directory
    pub diag_dir: String,
}

/// Server configuration
#[derive(Debug, Clone, SimpleObject, Serialize, Deserialize)]
pub struct ServerConfigGql {
    /// Listen address
    pub listen_host: String,
    /// Listen port
    pub listen_port: i32,
    /// Maximum connections
    pub max_connections: i32,
    /// Idle timeout in milliseconds
    pub idle_timeout_ms: i64,
    /// Request timeout in milliseconds
    pub request_timeout_ms: i64,
}

/// IPC configuration
#[derive(Debug, Clone, SimpleObject, Serialize, Deserialize)]
pub struct IpcConfigGql {
    /// Whether IPC is enabled
    pub enabled: bool,
    /// Socket/pipe path
    pub path: String,
    /// Optional socket/pipe name
    pub name: Option<String>,
}

/// Security configuration
#[derive(Debug, Clone, SimpleObject, Serialize, Deserialize)]
pub struct SecurityConfigGql {
    /// Security mode (dev/prod)
    pub mode: SecurityMode,
}

/// TLS configuration
#[derive(Debug, Clone, SimpleObject, Serialize, Deserialize)]
pub struct TlsConfigGql {
    /// Whether TLS is enabled
    pub enabled: bool,
    /// Certificate path
    pub cert_path: String,
    /// Key path
    pub key_path: String,
    /// CA path (optional)
    pub ca_path: Option<String>,
    /// Minimum TLS version
    pub min_version: String,
    /// Require client certificate
    pub require_client_cert: bool,
}

/// Authentication configuration
#[derive(Debug, Clone, SimpleObject, Serialize, Deserialize)]
pub struct AuthConfigGql {
    /// Authentication mode
    pub mode: AuthMode,
    /// Session timeout in milliseconds
    pub session_timeout_ms: i64,
    /// Maximum failed login attempts
    pub max_failed_attempts: i32,
    /// Lockout duration in milliseconds
    pub lockout_duration_ms: i64,
}

/// Logging configuration
#[derive(Debug, Clone, SimpleObject, Serialize, Deserialize)]
pub struct LoggingConfigGql {
    /// Logging mode (file/stdout)
    pub mode: LoggingMode,
    /// Log format (json/text)
    pub format: LogFormat,
    /// Log level
    pub level: String,
    /// Audit logging enabled
    pub audit_enabled: bool,
    /// Log rotation enabled
    pub rotate: bool,
    /// Maximum log files to keep
    pub max_files: i32,
    /// Maximum file size in MB
    pub max_file_size_mb: i32,
}

/// Storage configuration
#[derive(Debug, Clone, SimpleObject, Serialize, Deserialize)]
pub struct StorageConfigGql {
    /// Fsync enabled
    pub fsync: bool,
    /// Sync interval in milliseconds
    pub sync_interval_ms: i64,
    /// Page size in bytes
    pub page_size: i32,
    /// Buffer pool size in pages
    pub buffer_pool_pages: i32,
}

/// WAL configuration
#[derive(Debug, Clone, SimpleObject, Serialize, Deserialize)]
pub struct WalConfigGql {
    /// WAL enabled
    pub enabled: bool,
    /// WAL directory
    pub dir: String,
    /// Maximum segment size in MB
    pub max_segment_mb: i32,
    /// Checkpoint interval in milliseconds
    pub checkpoint_interval_ms: i64,
    /// Synchronous commit mode
    pub sync_mode: WalSyncMode,
    /// Archive enabled
    pub archive_enabled: bool,
}

/// Cache configuration
#[derive(Debug, Clone, SimpleObject, Serialize, Deserialize)]
pub struct CacheConfigGql {
    /// Cache enabled
    pub enabled: bool,
    /// Maximum cache size in MB
    pub max_size_mb: i64,
    /// ML model caching enabled
    pub ml_enabled: bool,
    /// Query cache enabled
    pub query_cache_enabled: bool,
    /// Query cache TTL in milliseconds
    pub query_cache_ttl_ms: i64,
    /// Maximum query cache entries
    pub query_cache_max_entries: i32,
}

/// Metrics configuration
#[derive(Debug, Clone, SimpleObject, Serialize, Deserialize)]
pub struct MetricsConfigGql {
    /// Metrics enabled
    pub enabled: bool,
    /// Metrics mode (pull/push)
    pub mode: MetricsMode,
    /// Listen host
    pub listen_host: String,
    /// Listen port
    pub listen_port: i32,
    /// Metrics path
    pub path: String,
}

/// Diagnostics configuration
#[derive(Debug, Clone, SimpleObject, Serialize, Deserialize)]
pub struct DiagnosticsConfigGql {
    /// Write build info on startup
    pub write_build_info: bool,
    /// Write runtime info periodically
    pub write_runtime_info: bool,
    /// Maximum log bytes in diagnostic bundles
    pub max_log_bytes: i64,
    /// Core dumps enabled
    pub core_dumps_enabled: bool,
}

/// Compatibility configuration
#[derive(Debug, Clone, SimpleObject, Serialize, Deserialize)]
pub struct CompatConfigGql {
    /// Fail on unsupported layout version
    pub fail_on_unsupported_layout: bool,
    /// Fail on unsupported data format version
    pub fail_on_unsupported_data_format: bool,
}

/// Full RustyDB configuration
#[derive(Debug, Clone, SimpleObject, Serialize, Deserialize)]
pub struct RustyDbConfigGql {
    /// Instance configuration
    pub instance: InstanceConfigGql,
    /// Paths configuration
    pub paths: PathsConfigGql,
    /// Server configuration
    pub server: ServerConfigGql,
    /// IPC configuration
    pub ipc: IpcConfigGql,
    /// Security configuration
    pub security: SecurityConfigGql,
    /// TLS configuration
    pub tls: TlsConfigGql,
    /// Authentication configuration
    pub auth: AuthConfigGql,
    /// Logging configuration
    pub logging: LoggingConfigGql,
    /// Storage configuration
    pub storage: StorageConfigGql,
    /// WAL configuration
    pub wal: WalConfigGql,
    /// Cache configuration
    pub cache: CacheConfigGql,
    /// Metrics configuration
    pub metrics: MetricsConfigGql,
    /// Diagnostics configuration
    pub diagnostics: DiagnosticsConfigGql,
    /// Compatibility configuration
    pub compat: CompatConfigGql,
}

/// Version information
#[derive(Debug, Clone, SimpleObject, Serialize, Deserialize)]
pub struct VersionInfoGql {
    /// Binary version (e.g., "0.3.001")
    pub binary_version: String,
    /// Layout version (e.g., "1.0")
    pub layout_version: String,
    /// Data format version
    pub data_format_version: i32,
    /// WAL format version
    pub wal_format_version: i32,
    /// Protocol version
    pub protocol_version: i32,
}

/// Instance metadata
#[derive(Debug, Clone, SimpleObject, Serialize, Deserialize)]
pub struct InstanceMetadataGql {
    /// Layout version
    pub layout_version: String,
    /// Instance ID (UUID)
    pub instance_id: String,
    /// Creation timestamp (RFC3339)
    pub created_at: String,
    /// Data format version
    pub data_format_version: i32,
    /// WAL format version
    pub wal_format_version: Option<i32>,
    /// Protocol version
    pub protocol_version: Option<i32>,
    /// Previous version if upgraded
    pub last_upgraded_from: Option<String>,
}

/// Config reload result
#[derive(Debug, Clone, SimpleObject, Serialize, Deserialize)]
pub struct ConfigReloadResultGql {
    /// Whether reload succeeded
    pub success: bool,
    /// Status message
    pub message: String,
    /// Reload timestamp
    pub reloaded_at: i64,
}

// ============================================================================
// Input Types (InputObject)
// ============================================================================

/// Cache configuration input
#[derive(Debug, Clone, InputObject, Serialize, Deserialize)]
pub struct CacheConfigInput {
    /// Cache enabled
    pub enabled: Option<bool>,
    /// Maximum cache size in MB
    pub max_size_mb: Option<i64>,
    /// ML model caching enabled
    pub ml_enabled: Option<bool>,
    /// Query cache enabled
    pub query_cache_enabled: Option<bool>,
    /// Query cache TTL in milliseconds
    pub query_cache_ttl_ms: Option<i64>,
}

/// Logging configuration input
#[derive(Debug, Clone, InputObject, Serialize, Deserialize)]
pub struct LoggingConfigInput {
    /// Log level
    pub level: Option<String>,
    /// Audit logging enabled
    pub audit_enabled: Option<bool>,
}

// ============================================================================
// Default Implementations
// ============================================================================

impl Default for RustyDbConfigGql {
    fn default() -> Self {
        Self {
            instance: InstanceConfigGql {
                name: "default".to_string(),
                instance_id: Some("00000000-0000-0000-0000-000000000000".to_string()),
                description: String::new(),
            },
            paths: PathsConfigGql {
                conf_dir: "conf".to_string(),
                data_dir: "data".to_string(),
                logs_dir: "logs".to_string(),
                run_dir: "run".to_string(),
                cache_dir: "cache".to_string(),
                tmp_dir: "tmp".to_string(),
                backup_dir: "backup".to_string(),
                diag_dir: "diag".to_string(),
            },
            server: ServerConfigGql {
                listen_host: "127.0.0.1".to_string(),
                listen_port: 54321,
                max_connections: 500,
                idle_timeout_ms: 300000,
                request_timeout_ms: 30000,
            },
            ipc: IpcConfigGql {
                enabled: true,
                path: "sockets".to_string(),
                name: None,
            },
            security: SecurityConfigGql {
                mode: SecurityMode::Dev,
            },
            tls: TlsConfigGql {
                enabled: false,
                cert_path: "secrets/tls/server.crt".to_string(),
                key_path: "secrets/tls/server.key".to_string(),
                ca_path: None,
                min_version: "1.2".to_string(),
                require_client_cert: false,
            },
            auth: AuthConfigGql {
                mode: AuthMode::None,
                session_timeout_ms: 1800000,
                max_failed_attempts: 5,
                lockout_duration_ms: 300000,
            },
            logging: LoggingConfigGql {
                mode: LoggingMode::File,
                format: LogFormat::Json,
                level: "info".to_string(),
                audit_enabled: false,
                rotate: true,
                max_files: 10,
                max_file_size_mb: 100,
            },
            storage: StorageConfigGql {
                fsync: true,
                sync_interval_ms: 1000,
                page_size: 4096,
                buffer_pool_pages: 1000,
            },
            wal: WalConfigGql {
                enabled: true,
                dir: "wal".to_string(),
                max_segment_mb: 64,
                checkpoint_interval_ms: 60000,
                sync_mode: WalSyncMode::Local,
                archive_enabled: false,
            },
            cache: CacheConfigGql {
                enabled: true,
                max_size_mb: 512,
                ml_enabled: true,
                query_cache_enabled: true,
                query_cache_ttl_ms: 60000,
                query_cache_max_entries: 10000,
            },
            metrics: MetricsConfigGql {
                enabled: true,
                mode: MetricsMode::Pull,
                listen_host: "127.0.0.1".to_string(),
                listen_port: 9100,
                path: "/metrics".to_string(),
            },
            diagnostics: DiagnosticsConfigGql {
                write_build_info: true,
                write_runtime_info: true,
                max_log_bytes: 10485760,
                core_dumps_enabled: false,
            },
            compat: CompatConfigGql {
                fail_on_unsupported_layout: true,
                fail_on_unsupported_data_format: true,
            },
        }
    }
}

impl Default for VersionInfoGql {
    fn default() -> Self {
        Self {
            binary_version: "0.3.001".to_string(),
            layout_version: "1.0".to_string(),
            data_format_version: 2,
            wal_format_version: 2,
            protocol_version: 2,
        }
    }
}
