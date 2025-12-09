// # REST API Types
//
// Request and response types for the REST API, along with internal data structures.
// All types are strongly typed with domain-specific newtypes where appropriate.

use std::time::{SystemTime, UNIX_EPOCH};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration};
use tokio::sync::{RwLock, Semaphore};
use utoipa::ToSchema;
use uuid::Uuid;
use axum::{
    response::{IntoResponse, Response},
    http::StatusCode,
    Json,
};

use crate::common::*;
use crate::error::DbError;

// Newtype for API configuration to ensure domain-specific handling
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiConfig {
    // API listen address
    pub listen_addr: String,
    // API port
    pub port: u16,
    // Enable CORS
    pub enable_cors: bool,
    // CORS allowed origins
    pub cors_origins: Vec<String>,
    // Rate limit (requests per second)
    pub rate_limit_rps: u64,
    // Request timeout in seconds
    pub request_timeout_secs: u64,
    // Max request body size in bytes
    pub max_body_size: usize,
    // Enable Swagger UI
    pub enable_swagger: bool,
    // Enable authentication
    pub enable_auth: bool,
    // API key for authentication
    pub api_key: Option<String>,
    // Max concurrent connections
    pub max_connections: usize,
    // Enable request logging
    pub enable_logging: bool,
    // Pagination default page size
    pub default_page_size: usize,
    // Pagination max page size
    pub max_page_size: usize,
}

impl Default for ApiConfig {
    fn default() -> Self {
        Self {
            listen_addr: "0.0.0.0".to_string(),
            port: 8080,
            enable_cors: true,
            cors_origins: vec!["*".to_string()],
            rate_limit_rps: 100,
            request_timeout_secs: 30,
            max_body_size: 10 * 1024 * 1024, // 10MB
            enable_swagger: true,
            enable_auth: false,
            api_key: None,
            max_connections: 1000,
            enable_logging: true,
            default_page_size: 50,
            max_page_size: 1000,
        }
    }
}

// Newtype for session ID to prevent confusion with other IDs
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, ToSchema)]
pub struct SessionId(pub u64);

// Newtype for transaction ID
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, ToSchema)]
pub struct TransactionId(pub u64);

// Shared API state with proper encapsulation
#[derive(Clone)]
pub struct ApiState {
    pub config: ApiConfig,
    pub connection_semaphore: Arc<Semaphore>,
    pub active_queries: Arc<RwLock<HashMap<Uuid, QueryExecution>>>,
    pub active_sessions: Arc<RwLock<HashMap<SessionId, SessionInfo>>>,
    pub metrics: Arc<RwLock<ApiMetrics>>,
    pub rate_limiter: Arc<RwLock<RateLimiter>>,
}

// API error with structured information
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct ApiError {
    pub code: String,
    pub message: String,
    pub details: Option<serde_json::Value>,
    pub timestamp: i64,
    pub request_id: Option<String>,
}

impl ApiError {
    pub fn new(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            code: code.into(),
            message: message.into(),
            details: None,
            timestamp: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs() as i64,
            request_id: None,
        }
    }

    pub fn with_details(mut self, details: serde_json::Value) -> Self {
        self.details = Some(details);
        self
    }

    pub fn with_request_id(mut self, request_id: String) -> Self {
        self.request_id = Some(request_id);
        self
    }
}

impl From<DbError> for ApiError {
    fn from(err: DbError) -> Self {
        ApiError::new("DATABASE_ERROR", err.to_string())
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let status = match self.code.as_str() {
            "NOT_FOUND" => StatusCode::NOT_FOUND,
            "INVALID_INPUT" | "VALIDATION_ERROR" => StatusCode::BAD_REQUEST,
            "UNAUTHORIZED" => StatusCode::UNAUTHORIZED,
            "FORBIDDEN" => StatusCode::FORBIDDEN,
            "CONFLICT" => StatusCode::CONFLICT,
            "RATE_LIMITED" => StatusCode::TOO_MANY_REQUESTS,
            _ => StatusCode::INTERNAL_SERVER_ERROR,
        };
        (status, Json(self)).into_response()
    }
}

pub type ApiResult<T> = Result<T, ApiError>;

// Query execution tracking
#[derive(Debug, Clone)]
pub struct QueryExecution {
    pub query_id: Uuid,
    pub sql: String,
    pub started_at: SystemTime,
    pub session_id: SessionId,
    pub status: String,
}

// API metrics with atomic counters where appropriate
#[derive(Debug, Clone, Default)]
pub struct ApiMetrics {
    pub total_requests: u64,
    pub successful_requests: u64,
    pub failed_requests: u64,
    pub avg_response_time_ms: f64,
    pub requests_by_endpoint: HashMap<String, u64>,
}

// Rate limiter implementation
#[derive(Debug)]
pub struct RateLimiter {
    pub requests: HashMap<String, Vec<SystemTime>>,
    pub window_secs: u64,
    pub max_requests: u64,
}

impl RateLimiter {
    pub fn new(maxrequests: u64, window_secs: u64) -> Self {
        Self {
            requests: HashMap::new(),
            window_secs,
            max_requests: maxrequests,
        }
    }

    pub fn check_limit(&mut self, identifier: &str) -> bool {
        let now = SystemTime::now();
        let cutoff = now - Duration::from_secs(self.window_secs);

        let entry = self.requests.entry(identifier.to_string()).or_insert_with(Vec::new);
        entry.retain(|&t| t > cutoff);

        if entry.len() as u64 >= self.max_requests {
            false
        } else {
            entry.push(now);
            true
        }
    }
}

// ============================================================================
// Request/Response Types - Core Database Operations
// ============================================================================

// Query request with validation
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct QueryRequest {
    // SQL query to execute
    pub sql: String,
    // Query parameters
    pub params: Option<Vec<serde_json::Value>>,
    // Maximum number of rows to return
    pub limit: Option<usize>,
    // Offset for pagination
    pub offset: Option<usize>,
    // Timeout in seconds
    pub timeout: Option<u64>,
    // Return query plan
    pub explain: Option<bool>,
    // Transaction ID (if part of transaction)
    pub transaction_id: Option<TransactionId>,
}

// Query response with comprehensive metadata
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct QueryResponse {
    // Query execution ID
    pub query_id: String,
    // Result rows
    pub rows: Vec<HashMap<String, serde_json::Value>>,
    // Column metadata
    pub columns: Vec<ColumnMetadata>,
    // Number of rows returned
    pub row_count: usize,
    // Number of rows affected (for INSERT/UPDATE/DELETE)
    pub affected_rows: Option<usize>,
    // Execution time in milliseconds
    pub execution_time_ms: u64,
    // Query plan (if requested)
    pub plan: Option<String>,
    // Warnings
    pub warnings: Vec<String>,
    // Has more results
    pub has_more: bool,
}

// Column metadata with type safety
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ColumnMetadata {
    pub name: String,
    pub data_type: String,
    pub nullable: bool,
    pub precision: Option<u32>,
    pub scale: Option<u32>,
}

// Batch request for multiple statements
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct BatchRequest {
    // List of SQL statements to execute
    pub statements: Vec<String>,
    // Execute in transaction
    pub transactional: bool,
    // Stop on error
    pub stop_on_error: bool,
    // Transaction isolation level
    pub isolation: Option<String>,
}

// Batch response with detailed results
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct BatchResponse {
    // Batch execution ID
    pub batch_id: String,
    // Results for each statement
    pub results: Vec<BatchStatementResult>,
    // Total execution time
    pub total_time_ms: u64,
    // Number of successful statements
    pub success_count: usize,
    // Number of failed statements
    pub failure_count: usize,
}

// Result of individual batch statement
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct BatchStatementResult {
    pub statement_index: usize,
    pub success: bool,
    pub affected_rows: Option<usize>,
    pub error: Option<String>,
    pub execution_time_ms: u64,
}

// Table creation/update request
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct TableRequest {
    pub table_name: String,
    pub columns: Vec<TableColumn>,
    pub primary_key: Option<Vec<String>>,
    pub indexes: Option<Vec<IndexDefinition>>,
}

// Table column definition
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct TableColumn {
    pub name: String,
    pub data_type: String,
    pub nullable: bool,
    pub default_value: Option<serde_json::Value>,
}

// Index definition
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct IndexDefinition {
    pub name: String,
    pub columns: Vec<String>,
    pub unique: bool,
    pub index_type: Option<String>,
}

// Schema response with table information
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct SchemaResponse {
    pub database_name: String,
    pub tables: Vec<TableInfo>,
    pub views: Vec<ViewInfo>,
    pub procedures: Vec<ProcedureInfo>,
    pub total_count: usize,
}

// Table information
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct TableInfo {
    pub name: String,
    pub schema: String,
    pub row_count: u64,
    pub size_bytes: u64,
    pub columns: Vec<ColumnMetadata>,
    pub indexes: Vec<IndexInfo>,
}

// View information
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct ViewInfo {
    pub name: String,
    pub definition: String,
    pub is_materialized: bool,
}

// Procedure information
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct ProcedureInfo {
    pub name: String,
    pub parameters: Vec<String>,
    pub return_type: Option<String>,
}

// Index information
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct IndexInfo {
    pub name: String,
    pub columns: Vec<String>,
    pub unique: bool,
    pub index_type: String,
    pub size_bytes: u64,
}

// Transaction request
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct TransactionRequest {
    pub isolation_level: Option<String>,
    pub read_only: Option<bool>,
    pub deferrable: Option<bool>,
}

// Transaction response
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct TransactionResponse {
    pub transaction_id: TransactionId,
    pub isolation_level: String,
    pub started_at: i64,
    pub status: String,
}

// ============================================================================
// Request/Response Types - Administration
// ============================================================================

// Configuration response
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct ConfigResponse {
    pub settings: HashMap<String, serde_json::Value>,
    pub version: String,
    pub updated_at: i64,
}

// Backup request
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct BackupRequest {
    pub backup_type: String, // full, incremental, differential
    pub compression: Option<bool>,
    pub encryption: Option<bool>,
    pub destination: Option<String>,
    pub retention_days: Option<u32>,
}

// Backup response
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct BackupResponse {
    pub backup_id: String,
    pub status: String,
    pub started_at: i64,
    pub completed_at: Option<i64>,
    pub size_bytes: Option<u64>,
    pub location: String,
}

// Health check response
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct HealthResponse {
    pub status: String, // healthy, degraded, unhealthy
    pub version: String,
    pub uptime_seconds: u64,
    pub checks: HashMap<String, ComponentHealth>,
}

// Component health status
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct ComponentHealth {
    pub status: String,
    pub message: Option<String>,
    pub last_check: i64,
}

// Maintenance request
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct MaintenanceRequest {
    pub operation: String, // vacuum, analyze, reindex, checkpoint
    pub target: Option<String>, // table name or database
    pub options: Option<HashMap<String, serde_json::Value>>,
}

// User request
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct UserRequest {
    pub username: String,
    pub password: Option<String>,
    pub roles: Vec<String>,
    pub enabled: Option<bool>,
}

// User response
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct UserResponse {
    pub user_id: u64,
    pub username: String,
    pub roles: Vec<String>,
    pub enabled: bool,
    pub created_at: i64,
    pub last_login: Option<i64>,
}

// Role request
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct RoleRequest {
    pub role_name: String,
    pub permissions: Vec<String>,
    pub description: Option<String>,
}

// Role response
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct RoleResponse {
    pub role_id: u64,
    pub role_name: String,
    pub permissions: Vec<String>,
    pub description: Option<String>,
    pub created_at: i64,
}

// ============================================================================
// Request/Response Types - Monitoring & Metrics
// ============================================================================

// Metrics response
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct MetricsResponse {
    pub timestamp: i64,
    pub metrics: HashMap<String, MetricData>,
    pub prometheus_format: Option<String>,
}

// Metric data point
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct MetricData {
    pub value: f64,
    pub unit: String,
    pub labels: HashMap<String, String>,
}

// Session statistics response
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct SessionStatsResponse {
    pub active_sessions: usize,
    pub idle_sessions: usize,
    pub sessions: Vec<SessionInfo>,
    pub total_connections: u64,
    pub peak_connections: usize,
}

// Session information
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct SessionInfo {
    pub session_id: SessionId,
    #[serde(alias = "user")]
    pub username: String,
    pub database: String,
    pub client_address: Option<String>,
    #[serde(alias = "connected_at")]
    pub created_at: i64,
    pub last_activity: i64,
    pub state: String,
    pub current_query: Option<String>,
    pub transaction_id: Option<TransactionId>,
}

// Query statistics response
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct QueryStatsResponse {
    pub total_queries: u64,
    pub queries_per_second: f64,
    pub avg_execution_time_ms: f64,
    pub slow_queries: Vec<SlowQueryInfo>,
    pub top_queries: Vec<TopQueryInfo>,
}

// Slow query information
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct SlowQueryInfo {
    pub query: String,
    pub execution_time_ms: u64,
    pub timestamp: i64,
    pub user: String,
}

// Top query information
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct TopQueryInfo {
    pub query_pattern: String,
    pub execution_count: u64,
    pub total_time_ms: u64,
    pub avg_time_ms: f64,
}

// Performance data response
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct PerformanceDataResponse {
    pub cpu_usage_percent: f64,
    pub memory_usage_bytes: u64,
    pub memory_usage_percent: f64,
    pub disk_io_read_bytes: u64,
    pub disk_io_write_bytes: u64,
    pub cache_hit_ratio: f64,
    pub transactions_per_second: f64,
    pub locks_held: usize,
    pub deadlocks: u64,
}

// Log response
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct LogResponse {
    pub entries: Vec<LogEntry>,
    pub total_count: usize,
    pub has_more: bool,
}

// Log entry
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct LogEntry {
    pub timestamp: i64,
    pub level: String,
    pub message: String,
    pub context: HashMap<String, serde_json::Value>,
}

// Alert response
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct AlertResponse {
    pub alerts: Vec<Alert>,
    pub active_count: usize,
}

// Alert information
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct Alert {
    pub alert_id: String,
    pub severity: String,
    pub title: String,
    pub description: String,
    pub triggered_at: i64,
    pub acknowledged: bool,
}

// ============================================================================
// Request/Response Types - Pool & Connection Management
// ============================================================================

// Pool configuration
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct PoolConfig {
    pub pool_id: String,
    pub min_connections: usize,
    pub max_connections: usize,
    pub connection_timeout_secs: u64,
    pub idle_timeout_secs: u64,
    pub max_lifetime_secs: Option<u64>,
}

// Pool statistics response
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct PoolStatsResponse {
    pub pool_id: String,
    pub active_connections: usize,
    pub idle_connections: usize,
    pub total_connections: usize,
    pub waiting_requests: usize,
    pub total_acquired: u64,
    pub total_created: u64,
    pub total_destroyed: u64,
}

// Connection information
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ConnectionInfo {
    pub connection_id: u64,
    pub pool_id: String,
    pub session_id: SessionId,
    pub username: String,
    pub database: String,
    pub client_address: String,
    pub created_at: i64,
    pub last_activity: i64,
    pub queries_executed: u64,
    pub state: String,
    pub idle_time_secs: u64,
}

// ============================================================================
// Request/Response Types - Cluster Management
// ============================================================================

// Cluster node information
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ClusterNodeInfo {
    pub node_id: String,
    pub address: String,
    pub role: String, // leader, follower, candidate
    pub status: String, // healthy, degraded, unhealthy
    pub version: String,
    pub uptime_seconds: u64,
    pub last_heartbeat: i64,
}

// Add node request
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct AddNodeRequest {
    pub node_id: String,
    pub address: String,
    pub role: Option<String>,
}

// Cluster topology response
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct TopologyResponse {
    pub cluster_id: String,
    pub nodes: Vec<ClusterNodeInfo>,
    pub leader_node: Option<String>,
    pub quorum_size: usize,
    pub total_nodes: usize,
}

// Failover request
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct FailoverRequest {
    pub target_node: Option<String>,
    pub force: Option<bool>,
}

// Replication status response
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct ReplicationStatusResponse {
    pub primary_node: String,
    pub replicas: Vec<ReplicaStatus>,
    pub replication_lag_ms: u64,
    pub sync_state: String,
}

// Replica status
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct ReplicaStatus {
    pub node_id: String,
    pub state: String,
    pub lag_bytes: u64,
    pub lag_ms: u64,
    pub last_sync: i64,
}

// ============================================================================
// Pagination Support
// ============================================================================

// Pagination parameters
#[derive(Debug, Deserialize, ToSchema)]
pub struct PaginationParams {
    #[serde(default = "default_page")]
    pub page: usize,
    #[serde(default = "default_page_size")]
    pub page_size: usize,
    pub sort_by: Option<String>,
    pub sort_order: Option<String>, // asc, desc
}

fn default_page() -> usize {
    1
}

fn default_page_size() -> usize {
    50
}

// Paginated response wrapper
#[derive(Debug, Serialize, ToSchema)]
pub struct PaginatedResponse<T> {
    pub data: Vec<T>,
    pub page: usize,
    pub page_size: usize,
    pub total_pages: usize,
    pub total_count: usize,
    pub has_next: bool,
    pub has_prev: bool,
}

impl<T> PaginatedResponse<T> {
    pub fn new(data: Vec<T>, page: usize, page_size: usize, total_count: usize) -> Self {
        let total_pages = (total_count + page_size - 1) / page_size;
        Self {
            data,
            page,
            page_size,
            total_pages,
            total_count,
            has_next: page < total_pages,
            has_prev: page > 1,
        }
    }
}
