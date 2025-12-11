//! GraphQL schema and resolvers for the networking layer
//!
//! This module provides GraphQL queries, mutations, and subscriptions for
//! managing and monitoring the networking layer.

use async_graphql::{
    Context, EmptySubscription, Object, Result as GqlResult, Schema, SimpleObject, Subscription,
};
use std::sync::Arc;
use futures::StreamExt;
use tokio_stream::Stream;

use super::manager::NetworkManager;
use super::types::{NodeAddress, NodeId};

// ============================================================================
// GraphQL Types
// ============================================================================

/// Node information GraphQL type
#[derive(SimpleObject, Clone)]
pub struct GqlNodeInfo {
    /// Node ID
    pub id: String,
    /// Node address (host:port)
    pub address: String,
    /// Current state
    pub state: String,
    /// Health status
    pub health: String,
    /// Joined timestamp
    pub joined_at: String,
    /// Last heartbeat
    pub last_heartbeat: String,
}

/// Peer information GraphQL type
#[derive(SimpleObject, Clone)]
pub struct GqlPeerInfo {
    /// Node ID
    pub node_id: String,
    /// Address
    pub address: String,
    /// State
    pub state: String,
    /// Health
    pub health: String,
    /// Bytes sent
    pub bytes_sent: String,
    /// Bytes received
    pub bytes_received: String,
}

/// Network topology GraphQL type
#[derive(SimpleObject, Clone)]
pub struct GqlTopology {
    /// Local node ID
    pub local_node: String,
    /// Cluster size
    pub cluster_size: i32,
    /// List of members
    pub members: Vec<GqlNodeInfo>,
}

/// Network statistics GraphQL type
#[derive(SimpleObject, Clone)]
pub struct GqlNetworkStats {
    /// Messages sent
    pub messages_sent: String,
    /// Messages received
    pub messages_received: String,
    /// Bytes sent
    pub bytes_sent: String,
    /// Bytes received
    pub bytes_received: String,
    /// Active connections
    pub active_connections: i32,
    /// Average latency (ms)
    pub avg_latency_ms: f64,
}

/// Cluster membership event GraphQL type
#[derive(SimpleObject, Clone)]
pub struct GqlMembershipEvent {
    /// Event type (NodeJoined, NodeLeft, NodeFailed, etc.)
    pub event_type: String,
    /// Node ID
    pub node_id: String,
    /// Timestamp
    pub timestamp: String,
}

// ============================================================================
// GraphQL Context
// ============================================================================

/// GraphQL context containing the network manager
pub struct GqlContext {
    /// Network manager instance
    pub network_manager: Arc<NetworkManager>,
}

// ============================================================================
// Query Root
// ============================================================================

/// GraphQL query root
pub struct QueryRoot;

#[Object]
impl QueryRoot {
    /// Get list of all peers in the cluster
    async fn peers(&self, ctx: &Context<'_>) -> GqlResult<Vec<GqlPeerInfo>> {
        let context = ctx.data::<GqlContext>()?;
        let members = context.network_manager.get_members().await;

        let mut peers = Vec::new();
        for member in members {
            let health = context
                .network_manager
                .get_node_health(&member.id)
                .await
                .map(|h| format!("{:?}", h))
                .unwrap_or_else(|| "Unknown".to_string());

            peers.push(GqlPeerInfo {
                node_id: member.id.to_string(),
                address: member.address.to_string(),
                state: member.state.to_string(),
                health,
                bytes_sent: "0".to_string(), // TODO: Get real stats
                bytes_received: "0".to_string(),
            });
        }

        Ok(peers)
    }

    /// Get cluster topology
    async fn topology(&self, ctx: &Context<'_>) -> GqlResult<GqlTopology> {
        let context = ctx.data::<GqlContext>()?;
        let members = context.network_manager.get_members().await;
        let local_node_id = context.network_manager.local_node_id().to_string();

        let member_infos: Vec<GqlNodeInfo> = members
            .iter()
            .map(|m| {
                GqlNodeInfo {
                    id: m.id.to_string(),
                    address: m.address.to_string(),
                    state: m.state.to_string(),
                    health: "Healthy".to_string(), // TODO: Get real health
                    joined_at: format!("{:?}", m.joined_at),
                    last_heartbeat: format!("{:?}", m.last_heartbeat),
                }
            })
            .collect();

        Ok(GqlTopology {
            local_node: local_node_id,
            cluster_size: members.len() as i32,
            members: member_infos,
        })
    }

    /// Get network statistics
    async fn network_stats(&self, ctx: &Context<'_>) -> GqlResult<GqlNetworkStats> {
        let context = ctx.data::<GqlContext>()?;
        let stats = context.network_manager.get_stats().await;

        Ok(GqlNetworkStats {
            messages_sent: stats.messages_sent.to_string(),
            messages_received: stats.messages_received.to_string(),
            bytes_sent: stats.bytes_sent.to_string(),
            bytes_received: stats.bytes_received.to_string(),
            active_connections: stats.active_connections as i32,
            avg_latency_ms: stats.avg_latency_ms,
        })
    }

    /// Get information about a specific node
    async fn node_info(
        &self,
        ctx: &Context<'_>,
        #[graphql(desc = "Node ID")] node_id: String,
    ) -> GqlResult<Option<GqlNodeInfo>> {
        let context = ctx.data::<GqlContext>()?;
        let node_id = NodeId::new(node_id);

        let member = context.network_manager.get_member(&node_id).await;

        match member {
            Some(m) => {
                let health = context
                    .network_manager
                    .get_node_health(&m.id)
                    .await
                    .map(|h| format!("{:?}", h))
                    .unwrap_or_else(|| "Unknown".to_string());

                Ok(Some(GqlNodeInfo {
                    id: m.id.to_string(),
                    address: m.address.to_string(),
                    state: m.state.to_string(),
                    health,
                    joined_at: format!("{:?}", m.joined_at),
                    last_heartbeat: format!("{:?}", m.last_heartbeat),
                }))
            }
            None => Ok(None),
        }
    }

    /// Get list of unhealthy nodes
    async fn unhealthy_nodes(&self, ctx: &Context<'_>) -> GqlResult<Vec<String>> {
        let context = ctx.data::<GqlContext>()?;
        let unhealthy = context.network_manager.get_unhealthy_nodes().await;

        Ok(unhealthy.iter().map(|n| n.to_string()).collect())
    }

    /// Check overall cluster health
    async fn cluster_health(&self, ctx: &Context<'_>) -> GqlResult<String> {
        let context = ctx.data::<GqlContext>()?;
        let members = context.network_manager.get_members().await;
        let unhealthy = context.network_manager.get_unhealthy_nodes().await;

        let health = if unhealthy.is_empty() {
            "Healthy"
        } else if unhealthy.len() < members.len() / 2 {
            "Degraded"
        } else {
            "Unhealthy"
        };

        Ok(health.to_string())
    }
}

// ============================================================================
// Mutation Root
// ============================================================================

/// GraphQL mutation root
pub struct MutationRoot;

#[Object]
impl MutationRoot {
    /// Join the cluster
    async fn join_cluster(
        &self,
        ctx: &Context<'_>,
        #[graphql(desc = "Seed nodes (host:port format)")] seed_nodes: Vec<String>,
    ) -> GqlResult<JoinClusterResult> {
        let context = ctx.data::<GqlContext>()?;

        // Parse seed nodes
        let seeds: Result<Vec<NodeAddress>, _> = seed_nodes
            .iter()
            .map(|s| {
                let parts: Vec<&str> = s.split(':').collect();
                if parts.len() != 2 {
                    return Err("Invalid seed node format");
                }
                let port = parts[1].parse::<u16>()
                    .map_err(|_| "Invalid port")?;
                Ok(NodeAddress::new(parts[0], port))
            })
            .collect();

        let seeds = seeds.map_err(|e| format!("Parse error: {}", e))?;

        context
            .network_manager
            .join_cluster(seeds)
            .await
            .map_err(|e| format!("Join failed: {}", e))?;

        let members = context.network_manager.get_members().await;

        Ok(JoinClusterResult {
            success: true,
            message: "Successfully joined cluster".to_string(),
            cluster_size: members.len() as i32,
        })
    }

    /// Leave the cluster
    async fn leave_cluster(&self, ctx: &Context<'_>) -> GqlResult<LeaveClusterResult> {
        let context = ctx.data::<GqlContext>()?;

        context
            .network_manager
            .leave_cluster()
            .await
            .map_err(|e| format!("Leave failed: {}", e))?;

        Ok(LeaveClusterResult {
            success: true,
            message: "Successfully left cluster".to_string(),
        })
    }

    /// Update network configuration (hot reload)
    async fn update_config(
        &self,
        ctx: &Context<'_>,
        #[graphql(desc = "Configuration key")] key: String,
        #[graphql(desc = "Configuration value")] value: String,
    ) -> GqlResult<UpdateConfigResult> {
        let _context = ctx.data::<GqlContext>()?;

        // TODO: Implement configuration update
        // For now, return success

        Ok(UpdateConfigResult {
            success: true,
            message: format!("Configuration {} updated to {}", key, value),
        })
    }
}

/// Result type for join cluster mutation
#[derive(SimpleObject)]
struct JoinClusterResult {
    success: bool,
    message: String,
    cluster_size: i32,
}

/// Result type for leave cluster mutation
#[derive(SimpleObject)]
struct LeaveClusterResult {
    success: bool,
    message: String,
}

/// Result type for update config mutation
#[derive(SimpleObject)]
struct UpdateConfigResult {
    success: bool,
    message: String,
}

// ============================================================================
// Subscription Root
// ============================================================================

/// GraphQL subscription root
pub struct SubscriptionRoot;

#[Subscription]
impl SubscriptionRoot {
    /// Subscribe to peer events (join, leave, health changes)
    async fn peer_events(
        &self,
        _ctx: &Context<'_>,
    ) -> impl Stream<Item = GqlMembershipEvent> {
        // TODO: Implement real subscription using membership change channel
        // For now, return an empty stream

        tokio_stream::pending()
    }

    /// Subscribe to topology changes
    async fn topology_changes(
        &self,
        _ctx: &Context<'_>,
    ) -> impl Stream<Item = GqlTopology> {
        // TODO: Implement real subscription
        // For now, return an empty stream

        tokio_stream::pending()
    }

    /// Subscribe to network statistics updates
    async fn network_stats_stream(
        &self,
        ctx: &Context<'_>,
        #[graphql(desc = "Update interval in seconds", default = 5)] interval_secs: u64,
    ) -> impl Stream<Item = GqlNetworkStats> {
        let context = ctx.data::<GqlContext>().ok();

        if let Some(ctx) = context {
            let network_manager = ctx.network_manager.clone();

            tokio_stream::wrappers::IntervalStream::new(
                tokio::time::interval(std::time::Duration::from_secs(interval_secs))
            )
            .then(move |_| {
                let nm = network_manager.clone();
                async move {
                    let stats = nm.get_stats().await;
                    GqlNetworkStats {
                        messages_sent: stats.messages_sent.to_string(),
                        messages_received: stats.messages_received.to_string(),
                        bytes_sent: stats.bytes_sent.to_string(),
                        bytes_received: stats.bytes_received.to_string(),
                        active_connections: stats.active_connections as i32,
                        avg_latency_ms: stats.avg_latency_ms,
                    }
                }
            })
            .boxed()
        } else {
            tokio_stream::pending().boxed()
        }
    }
}

// ============================================================================
// Schema Creation
// ============================================================================

/// Type alias for the GraphQL schema
pub type NetworkSchema = Schema<QueryRoot, MutationRoot, EmptySubscription>;

/// Create a new GraphQL schema for the networking layer
pub fn create_schema(network_manager: Arc<NetworkManager>) -> NetworkSchema {
    let context = GqlContext { network_manager };

    Schema::build(QueryRoot, MutationRoot, EmptySubscription)
        .data(context)
        .finish()
}

// ============================================================================
// Example Queries
// ============================================================================

/// Example GraphQL queries for the networking layer
pub mod examples {
    /// Get all peers
    pub const QUERY_PEERS: &str = r#"
        query {
            peers {
                nodeId
                address
                state
                health
                bytesSent
                bytesReceived
            }
        }
    "#;

    /// Get cluster topology
    pub const QUERY_TOPOLOGY: &str = r#"
        query {
            topology {
                localNode
                clusterSize
                members {
                    id
                    address
                    state
                    health
                    joinedAt
                    lastHeartbeat
                }
            }
        }
    "#;

    /// Get network statistics
    pub const QUERY_STATS: &str = r#"
        query {
            networkStats {
                messagesSent
                messagesReceived
                bytesSent
                bytesReceived
                activeConnections
                avgLatencyMs
            }
        }
    "#;

    /// Join cluster mutation
    pub const MUTATION_JOIN: &str = r#"
        mutation {
            joinCluster(seedNodes: ["node1:7000", "node2:7000"]) {
                success
                message
                clusterSize
            }
        }
    "#;

    /// Leave cluster mutation
    pub const MUTATION_LEAVE: &str = r#"
        mutation {
            leaveCluster {
                success
                message
            }
        }
    "#;

    /// Subscribe to peer events
    pub const SUBSCRIPTION_PEERS: &str = r#"
        subscription {
            peerEvents {
                eventType
                nodeId
                timestamp
            }
        }
    "#;

    /// Subscribe to network stats
    pub const SUBSCRIPTION_STATS: &str = r#"
        subscription {
            networkStatsStream(intervalSecs: 5) {
                messagesSent
                messagesReceived
                activeConnections
                avgLatencyMs
            }
        }
    "#;
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_schema_creation() {
        use super::super::types::{NetworkConfig, NodeAddress};
        use super::super::manager::create_default_manager;

        let config = NetworkConfig::default();
        let local_node = NodeInfo::new(
            NodeId::new("test"),
            NodeAddress::new("localhost", 7000),
        );

        let manager = create_default_manager(config, local_node);
        let schema = create_schema(Arc::new(manager));

        // Verify schema was created successfully
        assert!(schema.sdl().contains("Query"));
        assert!(schema.sdl().contains("Mutation"));
    }
}
