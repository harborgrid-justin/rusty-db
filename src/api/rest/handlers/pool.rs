// Pool Management Handlers
//
// Handler functions for connection pool and session management

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::Json as AxumJson,
};
use parking_lot::RwLock;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::SystemTime;

use super::super::types::*;

// Lazy-initialized shared state for pool management
lazy_static::lazy_static! {
    static ref POOL_CONFIGS: Arc<RwLock<HashMap<String, PoolConfig>>> = {
        let mut pools = HashMap::new();
        pools.insert("default".to_string(), PoolConfig {
            pool_id: "default".to_string(),
            min_connections: 10,
            max_connections: 100,
            connection_timeout_secs: 30,
            idle_timeout_secs: 600,
            max_lifetime_secs: Some(3600),
        });
        pools.insert("readonly".to_string(), PoolConfig {
            pool_id: "readonly".to_string(),
            min_connections: 5,
            max_connections: 50,
            connection_timeout_secs: 15,
            idle_timeout_secs: 300,
            max_lifetime_secs: Some(1800),
        });
        Arc::new(RwLock::new(pools))
    };
    static ref POOL_STATS: Arc<RwLock<HashMap<String, PoolStatsInternal>>> = {
        let mut stats = HashMap::new();
        stats.insert("default".to_string(), PoolStatsInternal::new());
        stats.insert("readonly".to_string(), PoolStatsInternal::new());
        Arc::new(RwLock::new(stats))
    };
    static ref CONNECTIONS: Arc<RwLock<HashMap<u64, ConnectionInfo>>> = Arc::new(RwLock::new(HashMap::new()));
    static ref NEXT_CONN_ID: Arc<RwLock<u64>> = Arc::new(RwLock::new(1));
}

/// Internal pool statistics tracking
#[derive(Debug, Clone)]
struct PoolStatsInternal {
    active_connections: usize,
    idle_connections: usize,
    #[allow(dead_code)]
    waiting_requests: usize,
    #[allow(dead_code)]
    total_acquired: u64,
    #[allow(dead_code)]
    total_created: u64,
    total_destroyed: u64,
    #[allow(dead_code)]
    last_activity: SystemTime,
}

impl PoolStatsInternal {
    fn new() -> Self {
        Self {
            active_connections: 0,
            idle_connections: 10, // Start with min_connections
            waiting_requests: 0,
            total_acquired: 0,
            total_created: 10,
            total_destroyed: 0,
            last_activity: SystemTime::now(),
        }
    }
}

pub async fn get_pools(
    State(_state): State<Arc<ApiState>>,
) -> ApiResult<AxumJson<Vec<PoolConfig>>> {
    let pools = POOL_CONFIGS.read();
    let pool_list: Vec<PoolConfig> = pools.values().cloned().collect();
    Ok(AxumJson(pool_list))
}

/// Get pool by ID
pub async fn get_pool(
    State(_state): State<Arc<ApiState>>,
    Path(id): Path<String>,
) -> ApiResult<AxumJson<PoolConfig>> {
    let pools = POOL_CONFIGS.read();

    pools
        .get(&id)
        .cloned()
        .map(AxumJson)
        .ok_or_else(|| ApiError::new("NOT_FOUND", format!("Pool '{}' not found", id)))
}

/// Update pool configuration
#[utoipa::path(
    put,
    path = "/api/v1/pools/{id}",
    tag = "pool",
    request_body = PoolConfig,
    responses(
        (status = 200, description = "Pool updated"),
    )
)]
pub async fn update_pool(
    State(_state): State<Arc<ApiState>>,
    Path(id): Path<String>,
    AxumJson(config): AxumJson<PoolConfig>,
) -> ApiResult<StatusCode> {
    // Validate configuration
    if config.min_connections > config.max_connections {
        return Err(ApiError::new(
            "INVALID_INPUT",
            "min_connections cannot exceed max_connections",
        ));
    }
    if config.max_connections == 0 {
        return Err(ApiError::new(
            "INVALID_INPUT",
            "max_connections must be greater than 0",
        ));
    }
    if config.connection_timeout_secs == 0 {
        return Err(ApiError::new(
            "INVALID_INPUT",
            "connection_timeout_secs must be greater than 0",
        ));
    }

    let mut pools = POOL_CONFIGS.write();

    if pools.contains_key(&id) {
        let mut updated_config = config;
        updated_config.pool_id = id.clone(); // Ensure pool_id matches path
        pools.insert(id, updated_config);
        Ok(StatusCode::OK)
    } else {
        Err(ApiError::new(
            "NOT_FOUND",
            format!("Pool '{}' not found", id),
        ))
    }
}

// Get pool statistics
#[utoipa::path(
    get,
    path = "/api/v1/pools/{id}/stats",
    tag = "pool",
    responses(
        (status = 200, description = "Pool statistics", body = PoolStatsResponse),
    )
)]
pub async fn get_pool_stats(
    State(_state): State<Arc<ApiState>>,
    Path(_id): Path<String>,
) -> ApiResult<AxumJson<PoolStatsResponse>> {
    let stats = PoolStatsResponse {
        pool_id: "default".to_string(),
        active_connections: 25,
        idle_connections: 15,
        total_connections: 40,
        waiting_requests: 2,
        total_acquired: 5000,
        total_created: 50,
        total_destroyed: 10,
    };

    Ok(AxumJson(stats))
}

/// Drain a connection pool
#[utoipa::path(
    post,
    path = "/api/v1/pools/{id}/drain",
    tag = "pool",
    responses(
        (status = 202, description = "Pool draining started"),
    )
)]
pub async fn drain_pool(
    State(_state): State<Arc<ApiState>>,
    Path(id): Path<String>,
) -> ApiResult<StatusCode> {
    // Check if pool exists
    if !POOL_CONFIGS.read().contains_key(&id) {
        return Err(ApiError::new(
            "NOT_FOUND",
            format!("Pool '{}' not found", id),
        ));
    }

    // Mark pool for draining - close idle connections
    let mut stats = POOL_STATS.write();
    if let Some(internal) = stats.get_mut(&id) {
        let drained = internal.idle_connections;
        internal.total_destroyed += drained as u64;
        internal.idle_connections = 0;
        log::info!(
            "Pool '{}' draining started, {} idle connections marked for closure",
            id,
            drained
        );
    }

    Ok(StatusCode::ACCEPTED)
}

/// Get all active connections
#[utoipa::path(
    get,
    path = "/api/v1/connections",
    tag = "pool",
    responses(
        (status = 200, description = "List of connections", body = Vec<ConnectionInfo>),
    )
)]
pub async fn get_connections(
    State(state): State<Arc<ApiState>>,
    Query(params): Query<PaginationParams>,
) -> ApiResult<AxumJson<PaginatedResponse<ConnectionInfo>>> {
    // Build connections list from active sessions
    let sessions = state.active_sessions.read().await;
    let connections: Vec<ConnectionInfo> = sessions
        .values()
        .map(|session| {
            ConnectionInfo {
                connection_id: session.session_id.0,
                pool_id: "default".to_string(),
                session_id: session.session_id.clone(),
                client_address: session
                    .client_address
                    .clone()
                    .unwrap_or_else(|| "unknown".to_string()),
                database: "rustydb".to_string(),
                username: session.username.clone(),
                state: session.state.clone(),
                created_at: session.created_at,
                last_activity: session.last_activity,
                queries_executed: 0, // Would need query tracking per session
                idle_time_secs: 0,
            }
        })
        .collect();

    let total = connections.len();
    let page = params.page.max(1);
    let page_size = params.page_size.min(100).max(1);
    let start = (page - 1) * page_size;
    let end = (start + page_size).min(total);

    let paginated = if start < total {
        connections[start..end].to_vec()
    } else {
        vec![]
    };

    let response = PaginatedResponse::new(paginated, page, page_size, total);
    Ok(AxumJson(response))
}

/// Get connection by ID
pub async fn get_connection(
    State(state): State<Arc<ApiState>>,
    Path(id): Path<u64>,
) -> ApiResult<AxumJson<ConnectionInfo>> {
    // Look up connection from sessions
    let sessions = state.active_sessions.read().await;

    if let Some(session) = sessions.get(&SessionId(id)) {
        let conn = ConnectionInfo {
            connection_id: session.session_id.0,
            pool_id: "default".to_string(),
            session_id: session.session_id.clone(),
            client_address: session
                .client_address
                .clone()
                .unwrap_or_else(|| "unknown".to_string()),
            database: "rustydb".to_string(),
            username: session.username.clone(),
            state: session.state.clone(),
            created_at: session.created_at,
            last_activity: session.last_activity,
            queries_executed: 0,
            idle_time_secs: 0,
        };
        Ok(AxumJson(conn))
    } else {
        Err(ApiError::new(
            "NOT_FOUND",
            format!("Connection {} not found", id),
        ))
    }
}

/// Kill a connection
#[utoipa::path(
    delete,
    path = "/api/v1/connections/{id}",
    tag = "pool",
    responses(
        (status = 204, description = "Connection killed"),
    )
)]
pub async fn kill_connection(
    State(state): State<Arc<ApiState>>,
    Path(id): Path<u64>,
) -> ApiResult<StatusCode> {
    let mut sessions = state.active_sessions.write().await;

    if sessions.remove(&SessionId(id)).is_some() {
        // Update pool stats
        let mut stats = POOL_STATS.write();
        if let Some(internal) = stats.get_mut("default") {
            if internal.active_connections > 0 {
                internal.active_connections -= 1;
            }
            internal.total_destroyed += 1;
        }

        log::info!("Connection {} terminated", id);
        Ok(StatusCode::NO_CONTENT)
    } else {
        Err(ApiError::new(
            "NOT_FOUND",
            format!("Connection {} not found", id),
        ))
    }
}

// Get all sessions
#[utoipa::path(
    get,
    path = "/api/v1/sessions",
    tag = "pool",
    responses(
        (status = 200, description = "List of sessions", body = Vec<SessionInfo>),
    )
)]
pub async fn get_sessions(
    State(state): State<Arc<ApiState>>,
    Query(params): Query<PaginationParams>,
) -> ApiResult<AxumJson<PaginatedResponse<SessionInfo>>> {
    let sessions = state.active_sessions.read().await;
    let session_list: Vec<SessionInfo> = sessions.values().cloned().collect();

    let response =
        PaginatedResponse::new(session_list, params.page, params.page_size, sessions.len());

    Ok(AxumJson(response))
}

// Get session by ID
pub async fn get_session(
    State(state): State<Arc<ApiState>>,
    Path(id): Path<u64>,
) -> ApiResult<AxumJson<SessionInfo>> {
    let sessions = state.active_sessions.read().await;

    sessions
        .get(&SessionId(id))
        .cloned()
        .map(AxumJson)
        .ok_or_else(|| ApiError::new("NOT_FOUND", "Session not found"))
}

/// Terminate a session
pub async fn terminate_session(
    State(state): State<Arc<ApiState>>,
    Path(id): Path<u64>,
) -> ApiResult<StatusCode> {
    let mut sessions = state.active_sessions.write().await;

    if sessions.remove(&SessionId(id)).is_some() {
        log::info!("Session {} terminated", id);
        Ok(StatusCode::NO_CONTENT)
    } else {
        Err(ApiError::new(
            "NOT_FOUND",
            format!("Session {} not found", id),
        ))
    }
}

// ============================================================================
// Cluster Management Handlers
// ============================================================================

// Get all cluster nodes
#[utoipa::path(
    get,
    path = "/api/v1/cluster/nodes",
    tag = "cluster",
    responses(
        (status = 200, description = "List of cluster nodes", body = Vec<ClusterNodeInfo>),
    )
)]
pub async fn get_cluster_nodes(
    State(_state): State<Arc<ApiState>>,
) -> ApiResult<AxumJson<Vec<ClusterNodeInfo>>> {
    let nodes = vec![];
    Ok(AxumJson(nodes))
}
