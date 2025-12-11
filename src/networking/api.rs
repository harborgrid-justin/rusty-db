//! REST API routes for the networking layer
//!
//! This module provides HTTP REST endpoints for managing and monitoring
//! the networking layer. It exposes cluster operations, topology queries,
//! health checks, and statistics.

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;

use crate::error::{DbError, Result};
use super::manager::NetworkManager;
use super::types::{
    ClusterMessage, LoadBalancingStrategy, NetworkStats, NodeAddress, NodeId, NodeInfo,
    SelectionCriteria,
};

// ============================================================================
// API Request/Response Types
// ============================================================================

/// Response for peer list endpoint
#[derive(Debug, Serialize)]
pub struct PeersResponse {
    /// Total number of peers
    pub total: usize,
    /// List of peer information
    pub peers: Vec<PeerInfoResponse>,
}

/// Peer information in API response
#[derive(Debug, Serialize)]
pub struct PeerInfoResponse {
    /// Node ID
    pub node_id: String,
    /// Node address
    pub address: String,
    /// Node state
    pub state: String,
    /// Health status
    pub health: String,
    /// Metadata
    pub metadata: HashMap<String, String>,
}

/// Response for topology endpoint
#[derive(Debug, Serialize)]
pub struct TopologyResponse {
    /// Local node ID
    pub local_node: String,
    /// Total cluster size
    pub cluster_size: usize,
    /// Cluster members
    pub members: Vec<NodeInfoResponse>,
    /// Connections between nodes
    pub connections: Vec<ConnectionResponse>,
}

/// Node information in topology response
#[derive(Debug, Serialize)]
pub struct NodeInfoResponse {
    /// Node ID
    pub id: String,
    /// Node address
    pub address: String,
    /// Node state
    pub state: String,
    /// Joined at timestamp
    pub joined_at: String,
    /// Last heartbeat
    pub last_heartbeat: String,
}

/// Connection information in topology response
#[derive(Debug, Serialize)]
pub struct ConnectionResponse {
    /// Source node ID
    pub from: String,
    /// Destination node ID
    pub to: String,
    /// Connection type
    pub connection_type: String,
}

/// Request to join cluster
#[derive(Debug, Deserialize)]
pub struct JoinClusterRequest {
    /// Seed nodes to contact
    pub seed_nodes: Vec<String>,
}

/// Response from join cluster operation
#[derive(Debug, Serialize)]
pub struct JoinClusterResponse {
    /// Success status
    pub success: bool,
    /// Message
    pub message: String,
    /// Cluster size after join
    pub cluster_size: usize,
}

/// Response from leave cluster operation
#[derive(Debug, Serialize)]
pub struct LeaveClusterResponse {
    /// Success status
    pub success: bool,
    /// Message
    pub message: String,
}

/// Network statistics response
#[derive(Debug, Serialize)]
pub struct StatsResponse {
    /// Total messages sent
    pub messages_sent: u64,
    /// Total messages received
    pub messages_received: u64,
    /// Total bytes sent
    pub bytes_sent: u64,
    /// Total bytes received
    pub bytes_received: u64,
    /// Active connections
    pub active_connections: usize,
    /// Connection errors
    pub connection_errors: u64,
    /// Average latency in milliseconds
    pub avg_latency_ms: f64,
}

/// Health check response
#[derive(Debug, Serialize)]
pub struct HealthResponse {
    /// Overall health status
    pub status: String,
    /// Number of healthy nodes
    pub healthy_nodes: usize,
    /// Number of unhealthy nodes
    pub unhealthy_nodes: usize,
    /// Details per node
    pub nodes: Vec<NodeHealthResponse>,
}

/// Per-node health information
#[derive(Debug, Serialize)]
pub struct NodeHealthResponse {
    /// Node ID
    pub node_id: String,
    /// Health status
    pub status: String,
    /// Last check time
    pub last_check: String,
}

/// Query parameters for peer list
#[derive(Debug, Deserialize)]
pub struct PeersQuery {
    /// Filter by state
    pub state: Option<String>,
    /// Filter by health
    pub health: Option<String>,
}

/// Detailed health information for a specific node
#[derive(Debug, Serialize)]
pub struct NodeHealthDetail {
    /// Node ID
    pub node_id: String,
    /// Health status
    pub status: String,
}

// ============================================================================
// API State
// ============================================================================

/// Shared state for API handlers
#[derive(Clone)]
pub struct ApiState {
    /// Network manager instance
    pub network_manager: Arc<NetworkManager>,
}

// ============================================================================
// API Routes
// ============================================================================

/// Create the router for networking API endpoints
pub fn create_router(network_manager: Arc<NetworkManager>) -> Router {
    let state = ApiState { network_manager };

    Router::new()
        // Peer management
        .route("/api/v1/network/peers", get(list_peers))
        .route("/api/v1/network/peers/:node_id", get(get_peer))

        // Topology
        .route("/api/v1/network/topology", get(get_topology))

        // Cluster operations
        .route("/api/v1/network/join", post(join_cluster))
        .route("/api/v1/network/leave", post(leave_cluster))

        // Statistics and monitoring
        .route("/api/v1/network/stats", get(get_stats))
        .route("/api/v1/network/health", get(get_health))

        // Node information
        .route("/api/v1/network/node/:node_id/health", get(get_node_health))

        .with_state(state)
}

// ============================================================================
// API Handlers
// ============================================================================

/// GET /api/v1/network/peers - List all connected peers
async fn list_peers(
    State(state): State<ApiState>,
    Query(query): Query<PeersQuery>,
) -> impl IntoResponse {
    let members = state.network_manager.get_members().await;

    let mut peers: Vec<PeerInfoResponse> = Vec::new();

    for member in members {
        // Apply filters
        if let Some(ref state_filter) = query.state {
            if member.state.to_string() != *state_filter {
                continue;
            }
        }

        let health = state.network_manager
            .get_node_health(&member.id)
            .await
            .map(|h| format!("{:?}", h))
            .unwrap_or_else(|| "Unknown".to_string());

        if let Some(ref health_filter) = query.health {
            if health != *health_filter {
                continue;
            }
        }

        peers.push(PeerInfoResponse {
            node_id: member.id.to_string(),
            address: member.address.to_string(),
            state: member.state.to_string(),
            health,
            metadata: member.metadata.clone(),
        });
    }

    let response = PeersResponse {
        total: peers.len(),
        peers,
    };

    Json(response)
}

/// GET /api/v1/network/peers/:node_id - Get information about a specific peer
async fn get_peer(
    State(state): State<ApiState>,
    Path(node_id): Path<String>,
) -> std::result::Result<Json<PeerInfoResponse>, AppError> {
    let node_id = NodeId::new(node_id);

    let member = state.network_manager
        .get_member(&node_id)
        .await
        .ok_or_else(|| AppError::NotFound(format!("Node {} not found", node_id)))?;

    let health = state.network_manager
        .get_node_health(&member.id)
        .await
        .map(|h| format!("{:?}", h))
        .unwrap_or_else(|| "Unknown".to_string());

    let response = PeerInfoResponse {
        node_id: member.id.to_string(),
        address: member.address.to_string(),
        state: member.state.to_string(),
        health,
        metadata: member.metadata.clone(),
    };

    Ok(Json(response))
}

/// GET /api/v1/network/topology - Get cluster topology
async fn get_topology(
    State(state): State<ApiState>,
) -> impl IntoResponse {
    let members = state.network_manager.get_members().await;
    let local_node_id = state.network_manager.local_node_id().to_string();

    let member_responses: Vec<NodeInfoResponse> = members
        .iter()
        .map(|m| NodeInfoResponse {
            id: m.id.to_string(),
            address: m.address.to_string(),
            state: m.state.to_string(),
            joined_at: format!("{:?}", m.joined_at),
            last_heartbeat: format!("{:?}", m.last_heartbeat),
        })
        .collect();

    // For now, create empty connections list
    // In a real implementation, we would query actual connection information
    let connections: Vec<ConnectionResponse> = Vec::new();

    let response = TopologyResponse {
        local_node: local_node_id,
        cluster_size: members.len(),
        members: member_responses,
        connections,
    };

    Json(response)
}

/// POST /api/v1/network/join - Join the cluster
async fn join_cluster(
    State(state): State<ApiState>,
    Json(request): Json<JoinClusterRequest>,
) -> std::result::Result<Json<JoinClusterResponse>, AppError> {
    // Parse seed nodes
    let seed_nodes: Vec<NodeAddress> = request
        .seed_nodes
        .iter()
        .map(|s| {
            let parts: Vec<&str> = s.split(':').collect();
            if parts.len() != 2 {
                return Err(AppError::BadRequest("Invalid seed node format".to_string()));
            }
            let port = parts[1].parse::<u16>()
                .map_err(|_| AppError::BadRequest("Invalid port".to_string()))?;
            Ok(NodeAddress::new(parts[0], port))
        })
        .collect::<std::result::Result<Vec<_>, _>>()?;

    state.network_manager
        .join_cluster(seed_nodes)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;

    let members = state.network_manager.get_members().await;

    let response = JoinClusterResponse {
        success: true,
        message: "Successfully joined cluster".to_string(),
        cluster_size: members.len(),
    };

    Ok(Json(response))
}

/// POST /api/v1/network/leave - Leave the cluster
async fn leave_cluster(
    State(state): State<ApiState>,
) -> std::result::Result<Json<LeaveClusterResponse>, AppError> {
    state.network_manager
        .leave_cluster()
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;

    let response = LeaveClusterResponse {
        success: true,
        message: "Successfully left cluster".to_string(),
    };

    Ok(Json(response))
}

/// GET /api/v1/network/stats - Get network statistics
async fn get_stats(
    State(state): State<ApiState>,
) -> impl IntoResponse {
    let stats = state.network_manager.get_stats().await;

    let response = StatsResponse {
        messages_sent: stats.messages_sent,
        messages_received: stats.messages_received,
        bytes_sent: stats.bytes_sent,
        bytes_received: stats.bytes_received,
        active_connections: stats.active_connections,
        connection_errors: stats.connection_errors,
        avg_latency_ms: stats.avg_latency_ms,
    };

    Json(response)
}

/// GET /api/v1/network/health - Get overall network health
async fn get_health(
    State(state): State<ApiState>,
) -> impl IntoResponse {
    let members = state.network_manager.get_members().await;
    let unhealthy = state.network_manager.get_unhealthy_nodes().await;

    let mut node_health: Vec<NodeHealthResponse> = Vec::new();

    for member in &members {
        let health = state.network_manager
            .get_node_health(&member.id)
            .await
            .map(|h| format!("{:?}", h))
            .unwrap_or_else(|| "Unknown".to_string());

        node_health.push(NodeHealthResponse {
            node_id: member.id.to_string(),
            status: health,
            last_check: format!("{:?}", member.last_heartbeat),
        });
    }

    let healthy_count = members.len() - unhealthy.len();
    let status = if unhealthy.is_empty() {
        "Healthy"
    } else if unhealthy.len() < members.len() / 2 {
        "Degraded"
    } else {
        "Unhealthy"
    };

    let response = HealthResponse {
        status: status.to_string(),
        healthy_nodes: healthy_count,
        unhealthy_nodes: unhealthy.len(),
        nodes: node_health,
    };

    Json(response)
}

/// GET /api/v1/network/node/:node_id/health - Get health of a specific node
async fn get_node_health(
    State(state): State<ApiState>,
    Path(node_id): Path<String>,
) -> std::result::Result<Json<NodeHealthDetail>, AppError> {
    let node_id = NodeId::new(node_id);

    let health = state.network_manager
        .get_node_health(&node_id)
        .await
        .ok_or_else(|| AppError::NotFound(format!("Health info not available for node {}", node_id)))?;

    let response = NodeHealthDetail {
        node_id: node_id.to_string(),
        status: format!("{:?}", health),
    };

    Ok(Json(response))
}

// ============================================================================
// Error Handling
// ============================================================================

/// Application-specific error type for API handlers
#[derive(Debug)]
pub enum AppError {
    /// Not found error
    NotFound(String),
    /// Bad request error
    BadRequest(String),
    /// Internal server error
    Internal(String),
}

impl IntoResponse for AppError {
    fn into_response(self) -> axum::response::Response {
        let (status, error_message) = match self {
            AppError::NotFound(msg) => (StatusCode::NOT_FOUND, msg),
            AppError::BadRequest(msg) => (StatusCode::BAD_REQUEST, msg),
            AppError::Internal(msg) => (StatusCode::INTERNAL_SERVER_ERROR, msg),
        };

        #[derive(Serialize)]
        struct ErrorResponse {
            error: String,
        }

        let body = Json(ErrorResponse {
            error: error_message,
        });

        (status, body).into_response()
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_router_creation() {
        use super::super::types::{NetworkConfig, NodeAddress};
        use super::super::manager::create_default_manager;

        let config = NetworkConfig::default();
        let local_node = NodeInfo::new(
            NodeId::new("test"),
            NodeAddress::new("localhost", 7000),
        );

        let manager = create_default_manager(config, local_node);
        let router = create_router(Arc::new(manager));

        // Just verify router was created successfully
        assert!(true);
    }
}
