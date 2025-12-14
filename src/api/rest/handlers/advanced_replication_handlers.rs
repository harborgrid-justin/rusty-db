// Advanced Replication Handlers
//
// Handlers for advanced replication features: groups, publications, subscriptions,
// sharding, XA transactions, and Global Data Services (GDS)

use axum::{
    extract::{Path, State},
    response::Json as AxumJson,
    http::StatusCode,
};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use parking_lot::RwLock;
use std::collections::HashMap;
use uuid::Uuid;

use super::super::types::*;

// ============================================================================
// Request/Response Types - Replication Groups
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ReplicationGroup {
    pub group_id: String,
    pub group_name: String,
    pub members: Vec<String>,
    pub primary_node: String,
    pub replication_mode: String, // "sync", "async", "quorum"
    pub quorum_size: Option<usize>,
    pub status: String,
    pub created_at: i64,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct CreateReplicationGroupRequest {
    pub group_name: String,
    pub members: Vec<String>,
    pub primary_node: String,
    pub replication_mode: String,
    pub quorum_size: Option<usize>,
}

// ============================================================================
// Request/Response Types - Publications & Subscriptions
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct Publication {
    pub publication_id: String,
    pub publication_name: String,
    pub database: String,
    pub tables: Vec<String>,
    pub publish_operations: Vec<String>, // "INSERT", "UPDATE", "DELETE"
    pub status: String,
    pub created_at: i64,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct CreatePublicationRequest {
    pub publication_name: String,
    pub database: String,
    pub tables: Vec<String>,
    pub publish_operations: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct Subscription {
    pub subscription_id: String,
    pub subscription_name: String,
    pub publication_name: String,
    pub source_node: String,
    pub target_node: String,
    pub status: String, // "active", "paused", "failed"
    pub last_sync: i64,
    pub lag_ms: u64,
    pub created_at: i64,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct CreateSubscriptionRequest {
    pub subscription_name: String,
    pub publication_name: String,
    pub source_node: String,
}

// ============================================================================
// Request/Response Types - Sharding
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ShardedTable {
    pub table_name: String,
    pub shard_key: String,
    pub shard_count: usize,
    pub distribution_strategy: String, // "hash", "range", "list"
    pub shards: Vec<ShardInfo>,
    pub created_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ShardInfo {
    pub shard_id: String,
    pub node_id: String,
    pub range_start: Option<String>,
    pub range_end: Option<String>,
    pub row_count: u64,
    pub size_bytes: u64,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct CreateShardedTableRequest {
    pub table_name: String,
    pub shard_key: String,
    pub shard_count: usize,
    pub distribution_strategy: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct ShardingStatsResponse {
    pub total_sharded_tables: usize,
    pub total_shards: usize,
    pub total_rows: u64,
    pub total_size_bytes: u64,
    pub balance_score: f64, // 0-100, higher is better
    pub tables: Vec<ShardedTable>,
}

// ============================================================================
// Request/Response Types - Global Data Services (GDS)
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct GlobalService {
    pub service_id: String,
    pub service_name: String,
    pub service_type: String, // "database", "application"
    pub endpoints: Vec<String>,
    pub region: String,
    pub priority: u32,
    pub status: String,
    pub registered_at: i64,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct RegisterServiceRequest {
    pub service_name: String,
    pub service_type: String,
    pub endpoints: Vec<String>,
    pub region: String,
    pub priority: u32,
}

// ============================================================================
// Request/Response Types - XA Transactions
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct XaTransaction {
    pub xid: String,
    pub global_transaction_id: String,
    pub branch_qualifier: String,
    pub status: String, // "active", "prepared", "committed", "rolled_back"
    pub participants: Vec<String>,
    pub started_at: i64,
    pub prepared_at: Option<i64>,
    pub completed_at: Option<i64>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct StartXaRequest {
    pub global_transaction_id: String,
    pub branch_qualifier: String,
    pub timeout_secs: Option<u64>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct PrepareXaRequest {
    pub xid: String,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct CommitXaRequest {
    pub xid: String,
    pub one_phase: Option<bool>,
}

// ============================================================================
// State Management
// ============================================================================

lazy_static::lazy_static! {
    static ref REPLICATION_GROUPS: Arc<RwLock<HashMap<String, ReplicationGroup>>> = Arc::new(RwLock::new(HashMap::new()));
    static ref PUBLICATIONS: Arc<RwLock<HashMap<String, Publication>>> = Arc::new(RwLock::new(HashMap::new()));
    static ref SUBSCRIPTIONS: Arc<RwLock<HashMap<String, Subscription>>> = Arc::new(RwLock::new(HashMap::new()));
    static ref SHARDED_TABLES: Arc<RwLock<HashMap<String, ShardedTable>>> = Arc::new(RwLock::new(HashMap::new()));
    static ref GLOBAL_SERVICES: Arc<RwLock<HashMap<String, GlobalService>>> = Arc::new(RwLock::new(HashMap::new()));
    static ref XA_TRANSACTIONS: Arc<RwLock<HashMap<String, XaTransaction>>> = Arc::new(RwLock::new(HashMap::new()));
}

// ============================================================================
// Replication Groups Handlers
// ============================================================================

/// List all replication groups
#[utoipa::path(
    get,
    path = "/api/v1/replication/groups",
    tag = "advanced-replication",
    responses(
        (status = 200, description = "List of replication groups", body = Vec<ReplicationGroup>),
    )
)]
pub async fn list_replication_groups(
    State(_state): State<Arc<ApiState>>,
) -> ApiResult<AxumJson<Vec<ReplicationGroup>>> {
    let groups = REPLICATION_GROUPS.read();
    let group_list: Vec<ReplicationGroup> = groups.values().cloned().collect();
    Ok(AxumJson(group_list))
}

/// Create a new replication group
#[utoipa::path(
    post,
    path = "/api/v1/replication/groups",
    tag = "advanced-replication",
    request_body = CreateReplicationGroupRequest,
    responses(
        (status = 201, description = "Replication group created", body = ReplicationGroup),
        (status = 400, description = "Invalid request", body = ApiError),
    )
)]
pub async fn create_replication_group(
    State(_state): State<Arc<ApiState>>,
    AxumJson(request): AxumJson<CreateReplicationGroupRequest>,
) -> ApiResult<(StatusCode, AxumJson<ReplicationGroup>)> {
    let group_id = Uuid::new_v4().to_string();
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64;

    let group = ReplicationGroup {
        group_id: group_id.clone(),
        group_name: request.group_name,
        members: request.members,
        primary_node: request.primary_node,
        replication_mode: request.replication_mode,
        quorum_size: request.quorum_size,
        status: "active".to_string(),
        created_at: now,
    };

    REPLICATION_GROUPS.write().insert(group_id.clone(), group.clone());

    log::info!("Created replication group: {}", group_id);
    Ok((StatusCode::CREATED, AxumJson(group)))
}

/// Get replication group by ID
#[utoipa::path(
    get,
    path = "/api/v1/replication/groups/{id}",
    tag = "advanced-replication",
    params(
        ("id" = String, Path, description = "Group ID")
    ),
    responses(
        (status = 200, description = "Replication group details", body = ReplicationGroup),
        (status = 404, description = "Group not found", body = ApiError),
    )
)]
pub async fn get_replication_group(
    State(_state): State<Arc<ApiState>>,
    Path(id): Path<String>,
) -> ApiResult<AxumJson<ReplicationGroup>> {
    let groups = REPLICATION_GROUPS.read();
    groups.get(&id)
        .cloned()
        .map(AxumJson)
        .ok_or_else(|| ApiError::new("NOT_FOUND", format!("Replication group '{}' not found", id)))
}

/// Delete a replication group
#[utoipa::path(
    delete,
    path = "/api/v1/replication/groups/{id}",
    tag = "advanced-replication",
    params(
        ("id" = String, Path, description = "Group ID")
    ),
    responses(
        (status = 204, description = "Group deleted"),
        (status = 404, description = "Group not found", body = ApiError),
    )
)]
pub async fn delete_replication_group(
    State(_state): State<Arc<ApiState>>,
    Path(id): Path<String>,
) -> ApiResult<StatusCode> {
    let mut groups = REPLICATION_GROUPS.write();
    if groups.remove(&id).is_some() {
        log::info!("Deleted replication group: {}", id);
        Ok(StatusCode::NO_CONTENT)
    } else {
        Err(ApiError::new("NOT_FOUND", format!("Replication group '{}' not found", id)))
    }
}

// ============================================================================
// Publications Handlers
// ============================================================================

/// List all publications
#[utoipa::path(
    get,
    path = "/api/v1/replication/publications",
    tag = "advanced-replication",
    responses(
        (status = 200, description = "List of publications", body = Vec<Publication>),
    )
)]
pub async fn list_publications(
    State(_state): State<Arc<ApiState>>,
) -> ApiResult<AxumJson<Vec<Publication>>> {
    let pubs = PUBLICATIONS.read();
    let pub_list: Vec<Publication> = pubs.values().cloned().collect();
    Ok(AxumJson(pub_list))
}

/// Create a new publication
#[utoipa::path(
    post,
    path = "/api/v1/replication/publications",
    tag = "advanced-replication",
    request_body = CreatePublicationRequest,
    responses(
        (status = 201, description = "Publication created", body = Publication),
        (status = 400, description = "Invalid request", body = ApiError),
    )
)]
pub async fn create_publication(
    State(_state): State<Arc<ApiState>>,
    AxumJson(request): AxumJson<CreatePublicationRequest>,
) -> ApiResult<(StatusCode, AxumJson<Publication>)> {
    let pub_id = Uuid::new_v4().to_string();
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64;

    let publication = Publication {
        publication_id: pub_id.clone(),
        publication_name: request.publication_name,
        database: request.database,
        tables: request.tables,
        publish_operations: request.publish_operations,
        status: "active".to_string(),
        created_at: now,
    };

    PUBLICATIONS.write().insert(pub_id.clone(), publication.clone());

    log::info!("Created publication: {}", pub_id);
    Ok((StatusCode::CREATED, AxumJson(publication)))
}

/// Get publication by ID
#[utoipa::path(
    get,
    path = "/api/v1/replication/publications/{id}",
    tag = "advanced-replication",
    params(
        ("id" = String, Path, description = "Publication ID")
    ),
    responses(
        (status = 200, description = "Publication details", body = Publication),
        (status = 404, description = "Publication not found", body = ApiError),
    )
)]
pub async fn get_publication(
    State(_state): State<Arc<ApiState>>,
    Path(id): Path<String>,
) -> ApiResult<AxumJson<Publication>> {
    let pubs = PUBLICATIONS.read();
    pubs.get(&id)
        .cloned()
        .map(AxumJson)
        .ok_or_else(|| ApiError::new("NOT_FOUND", format!("Publication '{}' not found", id)))
}

/// Delete a publication
#[utoipa::path(
    delete,
    path = "/api/v1/replication/publications/{id}",
    tag = "advanced-replication",
    params(
        ("id" = String, Path, description = "Publication ID")
    ),
    responses(
        (status = 204, description = "Publication deleted"),
        (status = 404, description = "Publication not found", body = ApiError),
    )
)]
pub async fn delete_publication(
    State(_state): State<Arc<ApiState>>,
    Path(id): Path<String>,
) -> ApiResult<StatusCode> {
    let mut pubs = PUBLICATIONS.write();
    if pubs.remove(&id).is_some() {
        log::info!("Deleted publication: {}", id);
        Ok(StatusCode::NO_CONTENT)
    } else {
        Err(ApiError::new("NOT_FOUND", format!("Publication '{}' not found", id)))
    }
}

// ============================================================================
// Subscriptions Handlers
// ============================================================================

/// List all subscriptions
#[utoipa::path(
    get,
    path = "/api/v1/replication/subscriptions",
    tag = "advanced-replication",
    responses(
        (status = 200, description = "List of subscriptions", body = Vec<Subscription>),
    )
)]
pub async fn list_subscriptions(
    State(_state): State<Arc<ApiState>>,
) -> ApiResult<AxumJson<Vec<Subscription>>> {
    let subs = SUBSCRIPTIONS.read();
    let sub_list: Vec<Subscription> = subs.values().cloned().collect();
    Ok(AxumJson(sub_list))
}

/// Create a new subscription
#[utoipa::path(
    post,
    path = "/api/v1/replication/subscriptions",
    tag = "advanced-replication",
    request_body = CreateSubscriptionRequest,
    responses(
        (status = 201, description = "Subscription created", body = Subscription),
        (status = 400, description = "Invalid request", body = ApiError),
    )
)]
pub async fn create_subscription(
    State(_state): State<Arc<ApiState>>,
    AxumJson(request): AxumJson<CreateSubscriptionRequest>,
) -> ApiResult<(StatusCode, AxumJson<Subscription>)> {
    let sub_id = Uuid::new_v4().to_string();
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64;

    let subscription = Subscription {
        subscription_id: sub_id.clone(),
        subscription_name: request.subscription_name,
        publication_name: request.publication_name,
        source_node: request.source_node,
        target_node: "local".to_string(),
        status: "active".to_string(),
        last_sync: now,
        lag_ms: 0,
        created_at: now,
    };

    SUBSCRIPTIONS.write().insert(sub_id.clone(), subscription.clone());

    log::info!("Created subscription: {}", sub_id);
    Ok((StatusCode::CREATED, AxumJson(subscription)))
}

/// Get subscription by ID
#[utoipa::path(
    get,
    path = "/api/v1/replication/subscriptions/{id}",
    tag = "advanced-replication",
    params(
        ("id" = String, Path, description = "Subscription ID")
    ),
    responses(
        (status = 200, description = "Subscription details", body = Subscription),
        (status = 404, description = "Subscription not found", body = ApiError),
    )
)]
pub async fn get_subscription(
    State(_state): State<Arc<ApiState>>,
    Path(id): Path<String>,
) -> ApiResult<AxumJson<Subscription>> {
    let subs = SUBSCRIPTIONS.read();
    subs.get(&id)
        .cloned()
        .map(AxumJson)
        .ok_or_else(|| ApiError::new("NOT_FOUND", format!("Subscription '{}' not found", id)))
}

/// Delete a subscription
#[utoipa::path(
    delete,
    path = "/api/v1/replication/subscriptions/{id}",
    tag = "advanced-replication",
    params(
        ("id" = String, Path, description = "Subscription ID")
    ),
    responses(
        (status = 204, description = "Subscription deleted"),
        (status = 404, description = "Subscription not found", body = ApiError),
    )
)]
pub async fn delete_subscription(
    State(_state): State<Arc<ApiState>>,
    Path(id): Path<String>,
) -> ApiResult<StatusCode> {
    let mut subs = SUBSCRIPTIONS.write();
    if subs.remove(&id).is_some() {
        log::info!("Deleted subscription: {}", id);
        Ok(StatusCode::NO_CONTENT)
    } else {
        Err(ApiError::new("NOT_FOUND", format!("Subscription '{}' not found", id)))
    }
}

// ============================================================================
// Sharding Handlers
// ============================================================================

/// Create a sharded table
#[utoipa::path(
    post,
    path = "/api/v1/sharding/tables",
    tag = "sharding",
    request_body = CreateShardedTableRequest,
    responses(
        (status = 201, description = "Sharded table created", body = ShardedTable),
        (status = 400, description = "Invalid request", body = ApiError),
    )
)]
pub async fn create_sharded_table(
    State(_state): State<Arc<ApiState>>,
    AxumJson(request): AxumJson<CreateShardedTableRequest>,
) -> ApiResult<(StatusCode, AxumJson<ShardedTable>)> {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64;

    // Create shard info for each shard
    let mut shards = Vec::new();
    for i in 0..request.shard_count {
        shards.push(ShardInfo {
            shard_id: format!("shard_{}", i),
            node_id: format!("node_{}", i % 3), // Distribute across 3 nodes
            range_start: Some(format!("{}", i * 1000)),
            range_end: Some(format!("{}", (i + 1) * 1000)),
            row_count: 0,
            size_bytes: 0,
        });
    }

    let sharded_table = ShardedTable {
        table_name: request.table_name.clone(),
        shard_key: request.shard_key,
        shard_count: request.shard_count,
        distribution_strategy: request.distribution_strategy,
        shards,
        created_at: now,
    };

    SHARDED_TABLES.write().insert(request.table_name.clone(), sharded_table.clone());

    log::info!("Created sharded table: {}", request.table_name);
    Ok((StatusCode::CREATED, AxumJson(sharded_table)))
}

/// Trigger shard rebalance
#[utoipa::path(
    post,
    path = "/api/v1/sharding/rebalance",
    tag = "sharding",
    responses(
        (status = 202, description = "Rebalance initiated"),
    )
)]
pub async fn trigger_shard_rebalance(
    State(_state): State<Arc<ApiState>>,
) -> ApiResult<AxumJson<serde_json::Value>> {
    log::info!("Shard rebalance initiated");

    Ok(AxumJson(serde_json::json!({
        "status": "accepted",
        "message": "Shard rebalance operation initiated"
    })))
}

/// Get sharding statistics
#[utoipa::path(
    get,
    path = "/api/v1/sharding/stats",
    tag = "sharding",
    responses(
        (status = 200, description = "Sharding statistics", body = ShardingStatsResponse),
    )
)]
pub async fn get_sharding_stats(
    State(_state): State<Arc<ApiState>>,
) -> ApiResult<AxumJson<ShardingStatsResponse>> {
    let tables = SHARDED_TABLES.read();
    let table_list: Vec<ShardedTable> = tables.values().cloned().collect();

    let total_sharded_tables = table_list.len();
    let total_shards: usize = table_list.iter().map(|t| t.shard_count).sum();
    let total_rows: u64 = table_list.iter()
        .flat_map(|t| &t.shards)
        .map(|s| s.row_count)
        .sum();
    let total_size_bytes: u64 = table_list.iter()
        .flat_map(|t| &t.shards)
        .map(|s| s.size_bytes)
        .sum();

    Ok(AxumJson(ShardingStatsResponse {
        total_sharded_tables,
        total_shards,
        total_rows,
        total_size_bytes,
        balance_score: 95.5, // Mock balance score
        tables: table_list,
    }))
}

// ============================================================================
// Global Data Services (GDS) Handlers
// ============================================================================

/// Register a new global service
#[utoipa::path(
    post,
    path = "/api/v1/gds/services",
    tag = "global-data-services",
    request_body = RegisterServiceRequest,
    responses(
        (status = 201, description = "Service registered", body = GlobalService),
        (status = 400, description = "Invalid request", body = ApiError),
    )
)]
pub async fn register_global_service(
    State(_state): State<Arc<ApiState>>,
    AxumJson(request): AxumJson<RegisterServiceRequest>,
) -> ApiResult<(StatusCode, AxumJson<GlobalService>)> {
    let service_id = Uuid::new_v4().to_string();
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64;

    let service = GlobalService {
        service_id: service_id.clone(),
        service_name: request.service_name,
        service_type: request.service_type,
        endpoints: request.endpoints,
        region: request.region,
        priority: request.priority,
        status: "active".to_string(),
        registered_at: now,
    };

    GLOBAL_SERVICES.write().insert(service_id.clone(), service.clone());

    log::info!("Registered global service: {}", service_id);
    Ok((StatusCode::CREATED, AxumJson(service)))
}

/// List all global services
#[utoipa::path(
    get,
    path = "/api/v1/gds/services",
    tag = "global-data-services",
    responses(
        (status = 200, description = "List of global services", body = Vec<GlobalService>),
    )
)]
pub async fn list_global_services(
    State(_state): State<Arc<ApiState>>,
) -> ApiResult<AxumJson<Vec<GlobalService>>> {
    let services = GLOBAL_SERVICES.read();
    let service_list: Vec<GlobalService> = services.values().cloned().collect();
    Ok(AxumJson(service_list))
}

// ============================================================================
// XA Transactions Handlers
// ============================================================================

/// Start an XA transaction
#[utoipa::path(
    post,
    path = "/api/v1/xa/start",
    tag = "xa-transactions",
    request_body = StartXaRequest,
    responses(
        (status = 201, description = "XA transaction started", body = XaTransaction),
        (status = 400, description = "Invalid request", body = ApiError),
    )
)]
pub async fn start_xa_transaction(
    State(_state): State<Arc<ApiState>>,
    AxumJson(request): AxumJson<StartXaRequest>,
) -> ApiResult<(StatusCode, AxumJson<XaTransaction>)> {
    let xid = Uuid::new_v4().to_string();
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64;

    let xa_txn = XaTransaction {
        xid: xid.clone(),
        global_transaction_id: request.global_transaction_id,
        branch_qualifier: request.branch_qualifier,
        status: "active".to_string(),
        participants: vec![],
        started_at: now,
        prepared_at: None,
        completed_at: None,
    };

    XA_TRANSACTIONS.write().insert(xid.clone(), xa_txn.clone());

    log::info!("Started XA transaction: {}", xid);
    Ok((StatusCode::CREATED, AxumJson(xa_txn)))
}

/// Prepare an XA transaction
#[utoipa::path(
    post,
    path = "/api/v1/xa/prepare",
    tag = "xa-transactions",
    request_body = PrepareXaRequest,
    responses(
        (status = 200, description = "XA transaction prepared"),
        (status = 404, description = "XA transaction not found", body = ApiError),
    )
)]
pub async fn prepare_xa_transaction(
    State(_state): State<Arc<ApiState>>,
    AxumJson(request): AxumJson<PrepareXaRequest>,
) -> ApiResult<AxumJson<XaTransaction>> {
    let mut txns = XA_TRANSACTIONS.write();

    if let Some(txn) = txns.get_mut(&request.xid) {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;

        txn.status = "prepared".to_string();
        txn.prepared_at = Some(now);

        log::info!("Prepared XA transaction: {}", request.xid);
        Ok(AxumJson(txn.clone()))
    } else {
        Err(ApiError::new("NOT_FOUND", format!("XA transaction '{}' not found", request.xid)))
    }
}

/// Commit an XA transaction
#[utoipa::path(
    post,
    path = "/api/v1/xa/commit",
    tag = "xa-transactions",
    request_body = CommitXaRequest,
    responses(
        (status = 200, description = "XA transaction committed"),
        (status = 404, description = "XA transaction not found", body = ApiError),
    )
)]
pub async fn commit_xa_transaction(
    State(_state): State<Arc<ApiState>>,
    AxumJson(request): AxumJson<CommitXaRequest>,
) -> ApiResult<AxumJson<XaTransaction>> {
    let mut txns = XA_TRANSACTIONS.write();

    if let Some(txn) = txns.get_mut(&request.xid) {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;

        txn.status = "committed".to_string();
        txn.completed_at = Some(now);

        log::info!("Committed XA transaction: {}", request.xid);
        Ok(AxumJson(txn.clone()))
    } else {
        Err(ApiError::new("NOT_FOUND", format!("XA transaction '{}' not found", request.xid)))
    }
}
