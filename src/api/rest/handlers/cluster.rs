// Cluster Management Handlers
//
// Handler functions for cluster coordination and replication

use axum::{
    extract::{Path, State},
    response::{Json as AxumJson},
    http::StatusCode,
};
use serde_json::json;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::SystemTime;
use parking_lot::RwLock;

use super::super::types::*;
use std::time::UNIX_EPOCH;

// Lazy-initialized shared state for cluster management
lazy_static::lazy_static! {
    static ref CLUSTER_NODES: Arc<RwLock<HashMap<String, ClusterNodeInfo>>> = {
        let mut nodes = HashMap::new();
        // Initialize with local node
        let local_node = ClusterNodeInfo {
            node_id: "node-local".to_string(),
            address: "127.0.0.1:5432".to_string(),
            role: "leader".to_string(),
            status: "healthy".to_string(),
            version: env!("CARGO_PKG_VERSION").to_string(),
            uptime_seconds: 0,
            last_heartbeat: SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs() as i64,
        };
        nodes.insert("node-local".to_string(), local_node);
        Arc::new(RwLock::new(nodes))
    };
    static ref CLUSTER_CONFIG: Arc<RwLock<HashMap<String, serde_json::Value>>> = {
        let mut config = HashMap::new();
        config.insert("cluster_name".to_string(), json!("rustydb-cluster"));
        config.insert("replication_factor".to_string(), json!(3));
        config.insert("heartbeat_interval_ms".to_string(), json!(1000));
        config.insert("election_timeout_ms".to_string(), json!(5000));
        config.insert("sync_replication".to_string(), json!(true));
        Arc::new(RwLock::new(config))
    };
    static ref CLUSTER_START_TIME: SystemTime = SystemTime::now();
}

pub async fn get_cluster_nodes(
    State(_state): State<Arc<ApiState>>,
) -> ApiResult<AxumJson<Vec<ClusterNodeInfo>>> {
    let nodes = CLUSTER_NODES.read();

    // Update uptime and heartbeat for local node
    let uptime = SystemTime::now().duration_since(*CLUSTER_START_TIME).unwrap_or_default().as_secs();

    let node_list: Vec<ClusterNodeInfo> = nodes.values().map(|node| {
        let mut n = node.clone();
        if n.node_id == "node-local" {
            n.uptime_seconds = uptime;
            n.last_heartbeat = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs() as i64;
        }
        n
    }).collect();

    Ok(AxumJson(node_list))
}

// Add a new cluster node
#[utoipa::path(
    post,
    path = "/api/v1/cluster/nodes",
    tag = "cluster",
    request_body = AddNodeRequest,
    responses(
        (status = 201, description = "Node added", body = ClusterNodeInfo),
    )
)]
pub async fn add_cluster_node(
    State(_state): State<Arc<ApiState>>,
    AxumJson(request): AxumJson<AddNodeRequest>,
) -> ApiResult<AxumJson<ClusterNodeInfo>> {
    // Check if node already exists
    {
        let nodes = CLUSTER_NODES.read();
        if nodes.contains_key(&request.node_id) {
            return Err(ApiError::new("CONFLICT", format!("Node '{}' already exists", request.node_id)));
        }
    }

    let node = ClusterNodeInfo {
        node_id: request.node_id.clone(),
        address: request.address,
        role: request.role.unwrap_or_else(|| "follower".to_string()),
        status: "initializing".to_string(),
        version: "1.0.0".to_string(),
        uptime_seconds: 0,
        last_heartbeat: SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap().as_secs() as i64,
    };

    // Persist the node to the CLUSTER_NODES state
    {
        let mut nodes = CLUSTER_NODES.write();
        nodes.insert(node.node_id.clone(), node.clone());
        log::info!("Added cluster node: {} at {}", node.node_id, node.address);
    }

    Ok(AxumJson(node))
}

/// Get cluster node by ID
pub async fn get_cluster_node(
    State(_state): State<Arc<ApiState>>,
    Path(id): Path<String>,
) -> ApiResult<AxumJson<ClusterNodeInfo>> {
    let nodes = CLUSTER_NODES.read();

    nodes.get(&id)
        .cloned()
        .map(|mut node| {
            // Update uptime for local node
            if node.node_id == "node-local" {
                node.uptime_seconds = SystemTime::now().duration_since(*CLUSTER_START_TIME).unwrap_or_default().as_secs();
                node.last_heartbeat = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs() as i64;
            }
            AxumJson(node)
        })
        .ok_or_else(|| ApiError::new("NOT_FOUND", format!("Node '{}' not found", id)))
}

/// Remove a cluster node
pub async fn remove_cluster_node(
    State(_state): State<Arc<ApiState>>,
    Path(id): Path<String>,
) -> ApiResult<StatusCode> {
    // Cannot remove local node
    if id == "node-local" {
        return Err(ApiError::new("FORBIDDEN", "Cannot remove local node"));
    }

    let mut nodes = CLUSTER_NODES.write();

    if nodes.remove(&id).is_some() {
        log::info!("Removed cluster node: {}", id);
        Ok(StatusCode::NO_CONTENT)
    } else {
        Err(ApiError::new("NOT_FOUND", format!("Node '{}' not found", id)))
    }
}

// Get cluster topology
#[utoipa::path(
    get,
    path = "/api/v1/cluster/topology",
    tag = "cluster",
    responses(
        (status = 200, description = "Cluster topology", body = TopologyResponse),
    )
)]
pub async fn get_cluster_topology(
    State(_state): State<Arc<ApiState>>,
) -> ApiResult<AxumJson<TopologyResponse>> {
    let nodes = CLUSTER_NODES.read();
    let uptime = SystemTime::now().duration_since(*CLUSTER_START_TIME).unwrap_or_default().as_secs();

    let node_list: Vec<ClusterNodeInfo> = nodes.values().map(|node| {
        let mut n = node.clone();
        if n.node_id == "node-local" {
            n.uptime_seconds = uptime;
            n.last_heartbeat = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs() as i64;
        }
        n
    }).collect();

    // Find leader node
    let leader_node = node_list.iter()
        .find(|n| n.role == "leader")
        .map(|n| n.node_id.clone());

    let total_nodes = node_list.len();
    let quorum_size = (total_nodes / 2) + 1;

    let response = TopologyResponse {
        cluster_id: "rustydb-cluster-1".to_string(),
        nodes: node_list,
        leader_node,
        quorum_size,
        total_nodes,
    };

    Ok(AxumJson(response))
}

/// Trigger manual failover
#[utoipa::path(
    post,
    path = "/api/v1/cluster/failover",
    tag = "cluster",
    request_body = FailoverRequest,
    responses(
        (status = 202, description = "Failover initiated"),
    )
)]
pub async fn trigger_failover(
    State(_state): State<Arc<ApiState>>,
    AxumJson(request): AxumJson<FailoverRequest>,
) -> ApiResult<StatusCode> {
    // Validate target node exists if specified
    if let Some(ref target) = request.target_node {
        let nodes = CLUSTER_NODES.read();
        if !nodes.contains_key(target) {
            return Err(ApiError::new("NOT_FOUND", format!("Target node '{}' not found", target)));
        }

        let node = nodes.get(target).unwrap();
        if node.status != "healthy" {
            return Err(ApiError::new("INVALID_INPUT", format!("Target node '{}' is not healthy", target)));
        }
    }

    // Check for quorum
    let nodes = CLUSTER_NODES.read();
    let healthy_count = nodes.values().filter(|n| n.status == "healthy").count();
    let quorum = (nodes.len() / 2) + 1;

    if healthy_count < quorum {
        return Err(ApiError::new("FORBIDDEN", format!(
            "Cannot initiate failover: insufficient healthy nodes ({} of {} required)",
            healthy_count, quorum
        )));
    }

    log::info!("Failover initiated, target: {:?}, force: {:?}",
        request.target_node, request.force.unwrap_or(false));

    Ok(StatusCode::ACCEPTED)
}

// Get replication status
#[utoipa::path(
    get,
    path = "/api/v1/cluster/replication",
    tag = "cluster",
    responses(
        (status = 200, description = "Replication status", body = ReplicationStatusResponse),
    )
)]
pub async fn get_replication_status(
    State(_state): State<Arc<ApiState>>,
) -> ApiResult<AxumJson<ReplicationStatusResponse>> {
    let nodes = CLUSTER_NODES.read();

    // Find the primary (leader) node
    let primary_node = nodes.values()
        .find(|n| n.role == "leader")
        .map(|n| n.node_id.clone())
        .unwrap_or_else(|| "unknown".to_string());

    // Build replica status for follower nodes
    let replicas: Vec<ReplicaStatus> = nodes.values()
        .filter(|n| n.role == "follower")
        .map(|n| {
            let state = match n.status.as_str() {
                "healthy" => "streaming",
                "initializing" => "catchup",
                _ => "disconnected",
            };
            ReplicaStatus {
                node_id: n.node_id.clone(),
                state: state.to_string(),
                lag_bytes: 0,
                lag_ms: if n.status == "healthy" { 5 } else { 1000 },
                last_sync: n.last_heartbeat,
            }
        })
        .collect();

    // Calculate max replication lag
    let max_lag = replicas.iter().map(|r| r.lag_ms).max().unwrap_or(0);

    let sync_state = if replicas.is_empty() {
        "single_node"
    } else if replicas.iter().all(|r| r.state == "streaming" && r.lag_ms < 100) {
        "synchronous"
    } else {
        "asynchronous"
    };

    let response = ReplicationStatusResponse {
        primary_node,
        replicas,
        replication_lag_ms: max_lag,
        sync_state: sync_state.to_string(),
    };

    Ok(AxumJson(response))
}

/// Get cluster configuration
pub async fn get_cluster_config(
    State(_state): State<Arc<ApiState>>,
) -> ApiResult<AxumJson<HashMap<String, serde_json::Value>>> {
    let config = CLUSTER_CONFIG.read();
    Ok(AxumJson(config.clone()))
}

/// Update cluster configuration
#[utoipa::path(
    put,
    path = "/api/v1/cluster/config",
    tag = "cluster",
    request_body = HashMap<String, serde_json::Value>,
    responses(
        (status = 200, description = "Cluster configuration updated"),
    )
)]
pub async fn update_cluster_config(
    State(_state): State<Arc<ApiState>>,
    AxumJson(new_config): AxumJson<HashMap<String, serde_json::Value>>,
) -> ApiResult<StatusCode> {
    // Validate configuration keys
    let valid_keys = [
        "cluster_name", "replication_factor", "heartbeat_interval_ms",
        "election_timeout_ms", "sync_replication"
    ];

    for key in new_config.keys() {
        if !valid_keys.contains(&key.as_str()) {
            return Err(ApiError::new("INVALID_INPUT", format!("Unknown configuration key: {}", key)));
        }
    }

    // Validate specific settings
    if let Some(rf) = new_config.get("replication_factor") {
        if let Some(n) = rf.as_u64() {
            if n < 1 || n > 7 {
                return Err(ApiError::new("INVALID_INPUT", "replication_factor must be between 1 and 7"));
            }
        }
    }

    if let Some(hb) = new_config.get("heartbeat_interval_ms") {
        if let Some(n) = hb.as_u64() {
            if n < 100 || n > 10000 {
                return Err(ApiError::new("INVALID_INPUT", "heartbeat_interval_ms must be between 100 and 10000"));
            }
        }
    }

    // Apply configuration changes
    let mut config = CLUSTER_CONFIG.write();
    for (key, value) in new_config {
        config.insert(key, value);
    }

    log::info!("Cluster configuration updated");
    Ok(StatusCode::OK)
}
