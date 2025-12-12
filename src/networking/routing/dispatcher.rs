// Message dispatcher for cluster-wide operations
//
// This module implements message dispatching patterns including fan-out,
// scatter-gather, broadcast, and multicast for coordinated cluster operations.

use crate::error::{DbError, Result};
use crate::networking::routing::router::MessageRouter;
use crate::networking::routing::serialization::ClusterMessage;
use crate::networking::routing::table::ShardId;
use crate::networking::types::{MessagePriority, NodeId};
use std::sync::Arc;
use std::time::Duration;
use tokio::task::JoinSet;

/// Dispatcher for cluster-wide message operations
pub struct MessageDispatcher {
    /// Message router
    router: Arc<MessageRouter>,
}

impl MessageDispatcher {
    /// Create a new message dispatcher
    pub fn new(router: Arc<MessageRouter>) -> Self {
        Self { router }
    }

    /// Send a message to all nodes (broadcast)
    pub async fn broadcast(
        &self,
        message: ClusterMessage,
        priority: MessagePriority,
        exclude: Vec<NodeId>,
    ) -> Result<BroadcastResult> {
        let all_nodes = self.router.routing_table().get_all_nodes();
        let target_nodes: Vec<NodeId> = all_nodes
            .into_iter()
            .filter(|node| !exclude.contains(node))
            .collect();

        let total_nodes = target_nodes.len();
        let mut success_count = 0;
        let mut failed_nodes = Vec::new();

        for node in target_nodes {
            match self.router.send_message(node.clone(), message.clone(), priority) {
                Ok(_) => success_count += 1,
                Err(_) => failed_nodes.push(node),
            }
        }

        Ok(BroadcastResult {
            total_nodes,
            success_count,
            failed_nodes,
        })
    }

    /// Send a message to a group of nodes (multicast)
    pub async fn multicast(
        &self,
        nodes: Vec<NodeId>,
        message: ClusterMessage,
        priority: MessagePriority,
    ) -> Result<MulticastResult> {
        let total_nodes = nodes.len();
        let mut success_count = 0;
        let mut failed_nodes = Vec::new();

        for node in nodes {
            match self.router.send_message(node.clone(), message.clone(), priority) {
                Ok(_) => success_count += 1,
                Err(_) => failed_nodes.push(node),
            }
        }

        Ok(MulticastResult {
            total_nodes,
            success_count,
            failed_nodes,
        })
    }

    /// Send to all replicas of a shard
    pub async fn send_to_shard(
        &self,
        shard_id: ShardId,
        message: ClusterMessage,
        priority: MessagePriority,
        include_primary: bool,
    ) -> Result<ShardResult> {
        let mut target_nodes = Vec::new();

        if include_primary {
            if let Some(primary) = self.router.routing_table().get_shard_primary(shard_id) {
                target_nodes.push(primary);
            }
        }

        let replicas = self.router.routing_table().get_shard_replicas(shard_id);
        target_nodes.extend(replicas);

        let total_nodes = target_nodes.len();
        let mut success_count = 0;
        let mut failed_nodes = Vec::new();

        for node in target_nodes {
            match self.router.send_message(node.clone(), message.clone(), priority) {
                Ok(_) => success_count += 1,
                Err(_) => failed_nodes.push(node),
            }
        }

        Ok(ShardResult {
            shard_id,
            total_nodes,
            success_count,
            failed_nodes,
        })
    }

    /// Fan-out request: Send to multiple nodes and don't wait for responses
    pub async fn fan_out(
        &self,
        nodes: Vec<NodeId>,
        message: ClusterMessage,
        priority: MessagePriority,
    ) -> Result<FanOutResult> {
        let total_nodes = nodes.len();
        let mut success_count = 0;
        let mut failures = Vec::new();

        for node in nodes {
            match self.router.send_message(node.clone(), message.clone(), priority) {
                Ok(_) => success_count += 1,
                Err(e) => failures.push((node, e.to_string())),
            }
        }

        Ok(FanOutResult {
            total_nodes,
            success_count,
            failures,
        })
    }

    /// Scatter-gather: Send to multiple nodes and collect all responses
    pub async fn scatter_gather(
        &self,
        nodes: Vec<NodeId>,
        message: ClusterMessage,
        priority: MessagePriority,
        timeout: Duration,
    ) -> Result<ScatterGatherResult> {
        let total_nodes = nodes.len();
        let mut tasks = JoinSet::new();

        // Send requests to all nodes
        for node in nodes.clone() {
            let router = Arc::clone(&self.router);
            let msg = message.clone();

            tasks.spawn(async move {
                let result = router.send_request(node.clone(), msg, priority, Some(timeout)).await;
                (node, result)
            });
        }

        // Collect responses
        let mut responses = Vec::new();
        let mut failures = Vec::new();

        while let Some(result) = tasks.join_next().await {
            match result {
                Ok((node, Ok(response))) => {
                    responses.push((node, response));
                }
                Ok((node, Err(e))) => {
                    failures.push((node, e.to_string()));
                }
                Err(e) => {
                    failures.push((NodeId::new("unknown"), e.to_string()));
                }
            }
        }

        Ok(ScatterGatherResult {
            total_nodes,
            response_count: responses.len(),
            responses,
            failures,
        })
    }

    /// Quorum read: Send to multiple nodes and wait for quorum responses
    pub async fn quorum_read(
        &self,
        nodes: Vec<NodeId>,
        message: ClusterMessage,
        priority: MessagePriority,
        quorum_size: usize,
        timeout: Duration,
    ) -> Result<QuorumResult> {
        if quorum_size > nodes.len() {
            return Err(DbError::Network(format!(
                "Quorum size {} exceeds available nodes {}",
                quorum_size,
                nodes.len()
            )));
        }

        let total_nodes = nodes.len();
        let mut tasks = JoinSet::new();

        // Send requests to all nodes
        for node in nodes.clone() {
            let router = Arc::clone(&self.router);
            let msg = message.clone();

            tasks.spawn(async move {
                let result = router.send_request(node.clone(), msg, priority, Some(timeout)).await;
                (node, result)
            });
        }

        // Collect responses until quorum is reached or all complete
        let mut responses = Vec::new();
        let mut failures = Vec::new();

        while let Some(result) = tasks.join_next().await {
            match result {
                Ok((node, Ok(response))) => {
                    responses.push((node, response));

                    // Check if we've reached quorum
                    if responses.len() >= quorum_size {
                        // Abort remaining tasks
                        tasks.abort_all();
                        break;
                    }
                }
                Ok((node, Err(e))) => {
                    failures.push((node, e.to_string()));

                    // Check if quorum is still possible
                    let remaining = total_nodes - responses.len() - failures.len();
                    if responses.len() + remaining < quorum_size {
                        // Quorum not possible
                        tasks.abort_all();
                        return Err(DbError::Network(format!(
                            "Quorum not possible: {} successes, {} failures, {} remaining, need {}",
                            responses.len(),
                            failures.len(),
                            remaining,
                            quorum_size
                        )));
                    }
                }
                Err(e) => {
                    failures.push((NodeId::new("unknown"), e.to_string()));
                }
            }
        }

        let quorum_reached = responses.len() >= quorum_size;

        Ok(QuorumResult {
            total_nodes,
            quorum_size,
            response_count: responses.len(),
            quorum_reached,
            responses,
            failures,
        })
    }

    /// Coordinated broadcast: Send to all nodes and wait for acknowledgments
    pub async fn coordinated_broadcast(
        &self,
        message: ClusterMessage,
        priority: MessagePriority,
        timeout: Duration,
        min_acks: usize,
    ) -> Result<CoordinatedBroadcastResult> {
        let all_nodes = self.router.routing_table().get_all_nodes();
        let total_nodes = all_nodes.len();

        if min_acks > total_nodes {
            return Err(DbError::Network(format!(
                "Minimum acks {} exceeds available nodes {}",
                min_acks, total_nodes
            )));
        }

        let mut tasks = JoinSet::new();

        // Send to all nodes
        for node in all_nodes.clone() {
            let router = Arc::clone(&self.router);
            let msg = message.clone();

            tasks.spawn(async move {
                let result = router.send_request(node.clone(), msg, priority, Some(timeout)).await;
                (node, result)
            });
        }

        // Collect acknowledgments
        let mut acks = Vec::new();
        let mut failures = Vec::new();

        while let Some(result) = tasks.join_next().await {
            match result {
                Ok((node, Ok(_))) => {
                    acks.push(node);
                }
                Ok((node, Err(e))) => {
                    failures.push((node, e.to_string()));
                }
                Err(e) => {
                    failures.push((NodeId::new("unknown"), e.to_string()));
                }
            }
        }

        let success = acks.len() >= min_acks;

        Ok(CoordinatedBroadcastResult {
            total_nodes,
            min_acks,
            ack_count: acks.len(),
            success,
            acked_nodes: acks,
            failures,
        })
    }
}

/// Result of a broadcast operation
#[derive(Debug, Clone)]
pub struct BroadcastResult {
    pub total_nodes: usize,
    pub success_count: usize,
    pub failed_nodes: Vec<NodeId>,
}

/// Result of a multicast operation
#[derive(Debug, Clone)]
pub struct MulticastResult {
    pub total_nodes: usize,
    pub success_count: usize,
    pub failed_nodes: Vec<NodeId>,
}

/// Result of sending to a shard
#[derive(Debug, Clone)]
pub struct ShardResult {
    pub shard_id: ShardId,
    pub total_nodes: usize,
    pub success_count: usize,
    pub failed_nodes: Vec<NodeId>,
}

/// Result of a fan-out operation
#[derive(Debug, Clone)]
pub struct FanOutResult {
    pub total_nodes: usize,
    pub success_count: usize,
    pub failures: Vec<(NodeId, String)>,
}

/// Result of a scatter-gather operation
#[derive(Debug, Clone)]
pub struct ScatterGatherResult {
    pub total_nodes: usize,
    pub response_count: usize,
    pub responses: Vec<(NodeId, ClusterMessage)>,
    pub failures: Vec<(NodeId, String)>,
}

/// Result of a quorum operation
#[derive(Debug, Clone)]
pub struct QuorumResult {
    pub total_nodes: usize,
    pub quorum_size: usize,
    pub response_count: usize,
    pub quorum_reached: bool,
    pub responses: Vec<(NodeId, ClusterMessage)>,
    pub failures: Vec<(NodeId, String)>,
}

/// Result of a coordinated broadcast
#[derive(Debug, Clone)]
pub struct CoordinatedBroadcastResult {
    pub total_nodes: usize,
    pub min_acks: usize,
    pub ack_count: usize,
    pub success: bool,
    pub acked_nodes: Vec<NodeId>,
    pub failures: Vec<(NodeId, String)>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::networking::routing::serialization::HeartbeatMessage;
    use crate::networking::routing::table::RoutingTable;
    use crate::networking::types::NodeAddress;

    #[tokio::test]
    async fn test_broadcast() {
        let table = RoutingTable::new();

        // Add some nodes
        table.add_node(NodeId::new("node1"), NodeAddress::new("localhost", 8001), None);
        table.add_node(NodeId::new("node2"), NodeAddress::new("localhost", 8002), None);

        let router = Arc::new(MessageRouter::new(table));
        let dispatcher = MessageDispatcher::new(router);

        let message = ClusterMessage::Heartbeat(HeartbeatMessage {
            node_id: NodeId::new("coordinator"),
            timestamp: 0,
            sequence: 0,
        });

        let result = dispatcher
            .broadcast(message, MessagePriority::Normal, vec![])
            .await
            .unwrap();

        assert_eq!(result.total_nodes, 2);
    }

    #[tokio::test]
    async fn test_fan_out() {
        let table = RoutingTable::new();
        let router = Arc::new(MessageRouter::new(table));
        let dispatcher = MessageDispatcher::new(router);

        let nodes = vec![NodeId::new("node1"), NodeId::new("node2")];
        let message = ClusterMessage::Heartbeat(HeartbeatMessage {
            node_id: NodeId::new("coordinator"),
            timestamp: 0,
            sequence: 0,
        });

        let result = dispatcher
            .fan_out(nodes, message, MessagePriority::Normal)
            .await
            .unwrap();

        assert_eq!(result.total_nodes, 2);
    }
}
