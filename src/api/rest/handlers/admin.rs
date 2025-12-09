// Administration Handlers
//
// Handler functions for administrative operations

use axum::{
    extract::{Path, Query, State},
    response::{Json as AxumJson},
    http::StatusCode,
};
use serde_json::json;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::SystemTime;
use uuid::Uuid;
use parking_lot::RwLock;

use super::super::types::*;
use crate::monitoring::MonitoringSystem;
use std::time::UNIX_EPOCH;

// Lazy-initialized shared state for admin operations
lazy_static::lazy_static! {
    static ref MONITORING: Arc<MonitoringSystem> = Arc::new(MonitoringSystem::new());
    static ref CONFIG_STORE: Arc<RwLock<HashMap<String, serde_json::Value>>> = {
        let mut config = HashMap::new();
        config.insert("max_connections".to_string(), json!(1000));
        config.insert("buffer_pool_size".to_string(), json!(1024));
        config.insert("wal_enabled".to_string(), json!(true));
        config.insert("checkpoint_interval_secs".to_string(), json!(300));
        config.insert("log_level".to_string(), json!("info"));
        config.insert("query_timeout_secs".to_string(), json!(30));
        Arc::new(RwLock::new(config))
    };
    static ref START_TIME: SystemTime = SystemTime::now();
    static ref USERS_STORE: Arc<RwLock<HashMap<u64, UserResponse>>> = Arc::new(RwLock::new(HashMap::new()));
    static ref ROLES_STORE: Arc<RwLock<HashMap<u64, RoleResponse>>> = {
        let mut roles = HashMap::new();
        roles.insert(1, RoleResponse {
            role_id: 1,
            role_name: "admin".to_string(),
            permissions: vec!["ALL".to_string()],
            description: Some("Full administrative access".to_string()),
            created_at: 0,
        });
        roles.insert(2, RoleResponse {
            role_id: 2,
            role_name: "readonly".to_string(),
            permissions: vec!["SELECT".to_string()],
            description: Some("Read-only access".to_string()),
            created_at: 0,
        });
        Arc::new(RwLock::new(roles))
    };
    static ref NEXT_USER_ID: Arc<RwLock<u64>> = Arc::new(RwLock::new(1));
    static ref NEXT_ROLE_ID: Arc<RwLock<u64>> = Arc::new(RwLock::new(3));
}

// Get database configuration
#[utoipa::path(
    get,
    path = "/api/v1/admin/config",
    tag = "admin",
    responses(
        (status = 200, description = "Configuration", body = ConfigResponse),
    )
)]
pub async fn get_config(
    State(_state): State<Arc<ApiState>>,
) -> ApiResult<AxumJson<ConfigResponse>> {
    let mut settings = HashMap::new();
    settings.insert("max_connections".to_string(), json!(1000));
    settings.insert("buffer_pool_size".to_string(), json!(1024));
    settings.insert("wal_enabled".to_string(), json!(true));

    let response = ConfigResponse {
        settings,
        version: "1.0.0".to_string(),
        updated_at: SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap().as_secs() as i64,
    };

    Ok(AxumJson(response))
}

/// Update database configuration
#[utoipa::path(
    put,
    path = "/api/v1/admin/config",
    tag = "admin",
    request_body = HashMap<String, serde_json::Value>,
    responses(
        (status = 200, description = "Configuration updated"),
        (status = 400, description = "Invalid configuration", body = ApiError),
    )
)]
pub async fn update_config(
    State(_state): State<Arc<ApiState>>,
    AxumJson(settings): AxumJson<HashMap<String, serde_json::Value>>,
) -> ApiResult<StatusCode> {
    // Validate configuration values
    let valid_keys = [
        "max_connections", "buffer_pool_size", "wal_enabled",
        "checkpoint_interval_secs", "log_level", "query_timeout_secs"
    ];

    for key in settings.keys() {
        if !valid_keys.contains(&key.as_str()) {
            return Err(ApiError::new("INVALID_INPUT", format!("Unknown configuration key: {}", key)));
        }
    }

    // Validate specific settings
    if let Some(max_conn) = settings.get("max_connections") {
        if let Some(n) = max_conn.as_u64() {
            if n < 1 || n > 10000 {
                return Err(ApiError::new("INVALID_INPUT", "max_connections must be between 1 and 10000"));
            }
        } else {
            return Err(ApiError::new("INVALID_INPUT", "max_connections must be a positive integer"));
        }
    }

    if let Some(log_level) = settings.get("log_level") {
        if let Some(level) = log_level.as_str() {
            let valid_levels = ["trace", "debug", "info", "warn", "error"];
            if !valid_levels.contains(&level) {
                return Err(ApiError::new("INVALID_INPUT", format!("log_level must be one of: {:?}", valid_levels)));
            }
        }
    }

    // Apply configuration changes
    let mut config = CONFIG_STORE.write();
    for (key, value) in settings {
        config.insert(key, value);
    }

    Ok(StatusCode::OK)
}

// Create a backup
#[utoipa::path(
    post,
    path = "/api/v1/admin/backup",
    tag = "admin",
    request_body = BackupRequest,
    responses(
        (status = 202, description = "Backup started", body = BackupResponse),
    )
)]
pub async fn create_backup(
    State(_state): State<Arc<ApiState>>,
    AxumJson(_request): AxumJson<BackupRequest>,
) -> ApiResult<AxumJson<BackupResponse>> {
    let backup_id = Uuid::new_v4();

    let response = BackupResponse {
        backup_id: backup_id.to_string(),
        status: "in_progress".to_string(),
        started_at: SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap().as_secs() as i64,
        completed_at: None,
        size_bytes: None,
        location: "/backups/".to_string() + &backup_id.to_string(),
    };

    Ok(AxumJson(response))
}

// Health check endpoint
#[utoipa::path(
    get,
    path = "/api/v1/admin/health",
    tag = "admin",
    responses(
        (status = 200, description = "Health status", body = HealthResponse),
    )
)]
pub async fn get_health(
    State(_state): State<Arc<ApiState>>,
) -> ApiResult<AxumJson<HealthResponse>> {
    let mut checks = HashMap::new();

    checks.insert("database".to_string(), ComponentHealth {
        status: "healthy".to_string(),
        message: Some("Database is operational".to_string()),
        last_check: SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap().as_secs() as i64,
    });

    checks.insert("storage".to_string(), ComponentHealth {
        status: "healthy".to_string(),
        message: None,
        last_check: SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap().as_secs() as i64,
    });

    let response = HealthResponse {
        status: "healthy".to_string(),
        version: "1.0.0".to_string(),
        uptime_seconds: 3600,
        checks,
    };

    Ok(AxumJson(response))
}

// Run maintenance operations
#[utoipa::path(
    post,
    path = "/api/v1/admin/maintenance",
    tag = "admin",
    request_body = MaintenanceRequest,
    responses(
        (status = 202, description = "Maintenance started"),
        (status = 400, description = "Invalid operation", body = ApiError),
    )
)]
pub async fn run_maintenance(
    State(_state): State<Arc<ApiState>>,
    AxumJson(request): AxumJson<MaintenanceRequest>,
) -> ApiResult<StatusCode> {
    // Validate operation
    match request.operation.as_str() {
        "vacuum" | "analyze" | "reindex" | "checkpoint" => {
            let op = request.operation.clone();
            tokio::spawn(async move {
                tracing::info!("Starting maintenance operation: {}", op);
                tokio::time::sleep(std::time::Duration::from_secs(5)).await; // Simulate work
                tracing::info!("Completed maintenance operation: {}", op);
            });
            Ok(StatusCode::ACCEPTED)
        }
        _ => Err(ApiError::new("INVALID_INPUT", "Invalid maintenance operation")),
    }
}

/// Get all users
#[utoipa::path(
    get,
    path = "/api/v1/admin/users",
    tag = "admin",
    responses(
        (status = 200, description = "List of users", body = Vec<UserResponse>),
    )
)]
pub async fn get_users(
    State(_state): State<Arc<ApiState>>,
    Query(params): Query<PaginationParams>,
) -> ApiResult<AxumJson<PaginatedResponse<UserResponse>>> {
    let users_store = USERS_STORE.read();
    let mut users: Vec<UserResponse> = users_store.values().cloned().collect();

    // Sort by user_id for consistent ordering
    users.sort_by_key(|u| u.user_id);

    let total = users.len();
    let page = params.page.max(1);
    let page_size = params.page_size.min(100).max(1);
    let start = (page - 1) * page_size;
    let end = (start + page_size).min(total);

    let paginated_users = if start < total {
        users[start..end].to_vec()
    } else {
        vec![]
    };

    let response = PaginatedResponse::new(paginated_users, page, page_size, total);
    Ok(AxumJson(response))
}

// Create a new user
#[utoipa::path(
    post,
    path = "/api/v1/admin/users",
    tag = "admin",
    request_body = UserRequest,
    responses(
        (status = 201, description = "User created", body = UserResponse),
        (status = 409, description = "User already exists", body = ApiError),
    )
)]
pub async fn create_user(
    State(_state): State<Arc<ApiState>>,
    AxumJson(request): AxumJson<UserRequest>,
) -> ApiResult<AxumJson<UserResponse>> {
    // Validate username
    if request.username.trim().is_empty() {
        return Err(ApiError::new("INVALID_INPUT", "Username cannot be empty"));
    }

    if request.username.len() < 3 || request.username.len() > 64 {
        return Err(ApiError::new("INVALID_INPUT", "Username must be between 3 and 64 characters"));
    }

    // Check if username already exists
    {
        let users = USERS_STORE.read();
        if users.values().any(|u| u.username == request.username) {
            return Err(ApiError::new("CONFLICT", format!("User '{}' already exists", request.username)));
        }
    }

    // Validate roles exist
    {
        let roles = ROLES_STORE.read();
        for role in &request.roles {
            if !roles.values().any(|r| &r.role_name == role) {
                return Err(ApiError::new("INVALID_INPUT", format!("Role '{}' does not exist", role)));
            }
        }
    }

    // Generate new user ID
    let user_id = {
        let mut next_id = NEXT_USER_ID.write();
        let id = *next_id;
        *next_id += 1;
        id
    };

    let user = UserResponse {
        user_id,
        username: request.username.clone(),
        roles: request.roles,
        enabled: request.enabled.unwrap_or(true),
        created_at: SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs() as i64,
        last_login: None,
    };

    // Store the user
    {
        let mut users = USERS_STORE.write();
        users.insert(user_id, user.clone());
    }

    Ok(AxumJson(user))
}

/// Get user by ID
pub async fn get_user(
    State(_state): State<Arc<ApiState>>,
    Path(id): Path<u64>,
) -> ApiResult<AxumJson<UserResponse>> {
    let users = USERS_STORE.read();

    users.get(&id)
        .cloned()
        .map(AxumJson)
        .ok_or_else(|| ApiError::new("NOT_FOUND", format!("User {} not found", id)))
}

/// Update user
pub async fn update_user(
    State(_state): State<Arc<ApiState>>,
    Path(id): Path<u64>,
    AxumJson(request): AxumJson<UserRequest>,
) -> ApiResult<StatusCode> {
    let mut users = USERS_STORE.write();

    if let Some(user) = users.get_mut(&id) {
        // Validate roles exist
        {
            let roles = ROLES_STORE.read();
            for role in &request.roles {
                if !roles.values().any(|r| &r.role_name == role) {
                    return Err(ApiError::new("INVALID_INPUT", format!("Role '{}' does not exist", role)));
                }
            }
        }

        user.username = request.username;
        user.roles = request.roles;
        if let Some(enabled) = request.enabled {
            user.enabled = enabled;
        }
        Ok(StatusCode::OK)
    } else {
        Err(ApiError::new("NOT_FOUND", format!("User {} not found", id)))
    }
}

/// Delete user
pub async fn delete_user(
    State(_state): State<Arc<ApiState>>,
    Path(id): Path<u64>,
) -> ApiResult<StatusCode> {
    let mut users = USERS_STORE.write();

    if users.remove(&id).is_some() {
        Ok(StatusCode::NO_CONTENT)
    } else {
        Err(ApiError::new("NOT_FOUND", format!("User {} not found", id)))
    }
}

/// Get all roles
#[utoipa::path(
    get,
    path = "/api/v1/admin/roles",
    tag = "admin",
    responses(
        (status = 200, description = "List of roles", body = Vec<RoleResponse>),
    )
)]
pub async fn get_roles(
    State(_state): State<Arc<ApiState>>,
) -> ApiResult<AxumJson<Vec<RoleResponse>>> {
    let roles = ROLES_STORE.read();
    let mut role_list: Vec<RoleResponse> = roles.values().cloned().collect();
    role_list.sort_by_key(|r| r.role_id);
    Ok(AxumJson(role_list))
}

// Create a new role
#[utoipa::path(
    post,
    path = "/api/v1/admin/roles",
    tag = "admin",
    request_body = RoleRequest,
    responses(
        (status = 201, description = "Role created", body = RoleResponse),
    )
)]
pub async fn create_role(
    State(_state): State<Arc<ApiState>>,
    AxumJson(request): AxumJson<RoleRequest>,
) -> ApiResult<AxumJson<RoleResponse>> {
    // Validate role name
    if request.role_name.trim().is_empty() {
        return Err(ApiError::new("INVALID_INPUT", "Role name cannot be empty"));
    }

    // Check if role already exists
    {
        let roles = ROLES_STORE.read();
        if roles.values().any(|r| r.role_name == request.role_name) {
            return Err(ApiError::new("CONFLICT", format!("Role '{}' already exists", request.role_name)));
        }
    }

    // Validate permissions
    let valid_permissions = ["SELECT", "INSERT", "UPDATE", "DELETE", "CREATE", "DROP", "ALTER", "INDEX", "EXECUTE", "ALL"];
    for perm in &request.permissions {
        if !valid_permissions.contains(&perm.to_uppercase().as_str()) {
            return Err(ApiError::new("INVALID_INPUT", format!("Invalid permission: '{}'", perm)));
        }
    }

    // Generate new role ID
    let role_id = {
        let mut next_id = NEXT_ROLE_ID.write();
        let id = *next_id;
        *next_id += 1;
        id
    };

    let role = RoleResponse {
        role_id,
        role_name: request.role_name,
        permissions: request.permissions,
        description: request.description,
        created_at: SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs() as i64,
    };

    // Store the role
    {
        let mut roles = ROLES_STORE.write();
        roles.insert(role_id, role.clone());
    }

    Ok(AxumJson(role))
}

/// Get role by ID
pub async fn get_role(
    State(_state): State<Arc<ApiState>>,
    Path(id): Path<u64>,
) -> ApiResult<AxumJson<RoleResponse>> {
    let roles = ROLES_STORE.read();

    roles.get(&id)
        .cloned()
        .map(AxumJson)
        .ok_or_else(|| ApiError::new("NOT_FOUND", format!("Role {} not found", id)))
}

/// Update role
pub async fn update_role(
    State(_state): State<Arc<ApiState>>,
    Path(id): Path<u64>,
    AxumJson(request): AxumJson<RoleRequest>,
) -> ApiResult<StatusCode> {
    let mut roles = ROLES_STORE.write();

    if let Some(role) = roles.get_mut(&id) {
        // Validate permissions
        let valid_permissions = ["SELECT", "INSERT", "UPDATE", "DELETE", "CREATE", "DROP", "ALTER", "INDEX", "EXECUTE", "ALL"];
        for perm in &request.permissions {
            if !valid_permissions.contains(&perm.to_uppercase().as_str()) {
                return Err(ApiError::new("INVALID_INPUT", format!("Invalid permission: '{}'", perm)));
            }
        }

        role.role_name = request.role_name;
        role.permissions = request.permissions;
        role.description = request.description;
        Ok(StatusCode::OK)
    } else {
        Err(ApiError::new("NOT_FOUND", format!("Role {} not found", id)))
    }
}

/// Delete role
pub async fn delete_role(
    State(_state): State<Arc<ApiState>>,
    Path(id): Path<u64>,
) -> ApiResult<StatusCode> {
    // Check if any users have this role
    let role_name = {
        let roles = ROLES_STORE.read();
        roles.get(&id).map(|r| r.role_name.clone())
    };

    if let Some(name) = role_name {
        let users = USERS_STORE.read();
        if users.values().any(|u| u.roles.contains(&name)) {
            return Err(ApiError::new("CONFLICT", format!("Role '{}' is still assigned to users", name)));
        }
    } else {
        return Err(ApiError::new("NOT_FOUND", format!("Role {} not found", id)));
    }

    let mut roles = ROLES_STORE.write();
    roles.remove(&id);
    Ok(StatusCode::NO_CONTENT)
}

// ============================================================================
// Monitoring Handlers
// ============================================================================

// Get metrics
#[utoipa::path(
    get,
    path = "/api/v1/metrics",
    tag = "monitoring",
    responses(
        (status = 200, description = "Metrics data", body = MetricsResponse),
    )
)]
pub async fn get_metrics(
    State(_state): State<Arc<ApiState>>,
) -> ApiResult<AxumJson<MetricsResponse>> {
    let mut metric_data = HashMap::new();

    metric_data.insert("total_requests".to_string(), MetricData {
        value: 0.0,
        unit: "count".to_string(),
        labels: HashMap::new(),
    });

    let response = MetricsResponse {
        timestamp: SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs() as i64,
        metrics: metric_data,
        prometheus_format: None,
    };

    Ok(AxumJson(response))
}
