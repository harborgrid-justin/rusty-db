//! Network Manager - Central coordinator for networking components
//!
//! The NetworkManager is the main entry point for the networking layer. It coordinates
//! all networking components (transport, discovery, health monitoring, load balancing,
//! and cluster membership) and provides a unified API for the rest of RustyDB.

use crate::common::{Component, HealthStatus};
use crate::error::{DbError, Result};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{mpsc, RwLock};
use tokio::time::{interval, Duration};

use super::traits::{
    CircuitBreaker, ClusterMembership, ConnectionPool, HealthMonitor, LoadBalancer,
    MessageHandler, NetworkTransport, ServiceDiscovery,
};
use super::types::{
    ClusterMessage, MembershipEvent, MessagePriority, NetworkConfig, NetworkStats, NodeAddress,
    NodeId, NodeInfo, SelectionCriteria,
};

/// Central coordinator for all networking operations
///
/// The NetworkManager integrates all networking components and provides
/// a unified interface for sending/receiving messages, cluster operations,
/// and health monitoring.
pub struct NetworkManager {
    /// Network configuration
    config: NetworkConfig,

    /// Local node information
    local_node: NodeInfo,

    /// Network transport layer
    transport: Arc<dyn NetworkTransport>,

    /// Service discovery
    service_discovery: Arc<dyn ServiceDiscovery>,

    /// Health monitor
    health_monitor: Arc<dyn HealthMonitor>,

    /// Load balancer
    load_balancer: Arc<RwLock<dyn LoadBalancer>>,

    /// Cluster membership
    membership: Arc<RwLock<dyn ClusterMembership>>,

    /// Message handlers by message type
    message_handlers: Arc<RwLock<HashMap<String, Arc<dyn MessageHandler>>>>,

    /// Event bus for internal component communication
    event_tx: mpsc::Sender<NetworkEvent>,
    event_rx: Arc<RwLock<mpsc::Receiver<NetworkEvent>>>,

    /// Network statistics
    stats: Arc<RwLock<NetworkStats>>,

    /// Running state
    running: Arc<RwLock<bool>>,
}

/// Internal events for component coordination
#[derive(Debug, Clone)]
enum NetworkEvent {
    /// Message received from a node
    MessageReceived {
        from: NodeId,
        message: ClusterMessage,
    },
    /// Node joined the cluster
    NodeJoined(NodeInfo),
    /// Node left the cluster
    NodeLeft(NodeId),
    /// Node health changed
    NodeHealthChanged {
        node_id: NodeId,
        status: HealthStatus,
    },
    /// Shutdown requested
    Shutdown,
}

impl NetworkManager {
    /// Create a new NetworkManager
    ///
    /// # Arguments
    /// * `config` - Network configuration
    /// * `local_node` - Information about the local node
    /// * `transport` - Network transport implementation
    /// * `service_discovery` - Service discovery implementation
    /// * `health_monitor` - Health monitor implementation
    /// * `load_balancer` - Load balancer implementation
    /// * `membership` - Cluster membership implementation
    pub fn new(
        config: NetworkConfig,
        local_node: NodeInfo,
        transport: Arc<dyn NetworkTransport>,
        service_discovery: Arc<dyn ServiceDiscovery>,
        health_monitor: Arc<dyn HealthMonitor>,
        load_balancer: Arc<RwLock<dyn LoadBalancer>>,
        membership: Arc<RwLock<dyn ClusterMembership>>,
    ) -> Self {
        let (event_tx, event_rx) = mpsc::channel(1000);

        Self {
            config,
            local_node,
            transport,
            service_discovery,
            health_monitor,
            load_balancer,
            membership,
            message_handlers: Arc::new(RwLock::new(HashMap::new())),
            event_tx,
            event_rx: Arc::new(RwLock::new(event_rx)),
            stats: Arc::new(RwLock::new(NetworkStats::default())),
            running: Arc::new(RwLock::new(false)),
        }
    }

    /// Register a message handler for specific message types
    ///
    /// # Arguments
    /// * `handler` - The message handler to register
    pub async fn register_handler(&self, handler: Arc<dyn MessageHandler>) -> Result<()> {
        let mut handlers = self.message_handlers.write().await;

        for message_type in handler.message_types() {
            handlers.insert(message_type.to_string(), handler.clone());
        }

        Ok(())
    }

    /// Send a message to a specific node
    ///
    /// # Arguments
    /// * `node_id` - The destination node ID
    /// * `message` - The cluster message to send
    pub async fn send(&self, node_id: &NodeId, message: ClusterMessage) -> Result<()> {
        // Update statistics
        {
            let mut stats = self.stats.write().await;
            stats.messages_sent += 1;
        }

        // Send through transport layer
        self.transport.send_to_node(node_id, &message).await?;

        Ok(())
    }

    /// Send a message to a node selected by the load balancer
    ///
    /// # Arguments
    /// * `criteria` - Selection criteria for choosing a node
    /// * `message` - The cluster message to send
    ///
    /// # Returns
    /// * `Ok(node_id)` - The node that received the message
    /// * `Err` - If no suitable node found or send failed
    pub async fn send_balanced(
        &self,
        criteria: &SelectionCriteria,
        message: ClusterMessage,
    ) -> Result<NodeId> {
        // Select a node using the load balancer
        let node_id = {
            let balancer = self.load_balancer.read().await;
            balancer.select_node(criteria)
                .ok_or_else(|| DbError::Network("No suitable node found".to_string()))?
        };

        // Send the message
        self.send(&node_id, message).await?;

        Ok(node_id)
    }

    /// Broadcast a message to all cluster members
    ///
    /// # Arguments
    /// * `message` - The cluster message to broadcast
    ///
    /// # Returns
    /// * `Ok(count)` - Number of nodes that successfully received the message
    pub async fn broadcast(&self, message: ClusterMessage) -> Result<usize> {
        let count = self.transport.broadcast(&message).await?;

        // Update statistics
        {
            let mut stats = self.stats.write().await;
            stats.messages_sent += count as u64;
        }

        Ok(count)
    }

    /// Get the list of cluster members
    pub async fn get_members(&self) -> Vec<NodeInfo> {
        let membership = self.membership.read().await;
        membership.get_members()
    }

    /// Get information about a specific member
    ///
    /// # Arguments
    /// * `node_id` - The ID of the member
    pub async fn get_member(&self, node_id: &NodeId) -> Option<NodeInfo> {
        let membership = self.membership.read().await;
        membership.get_member(node_id)
    }

    /// Get the health status of a node
    ///
    /// # Arguments
    /// * `node_id` - The ID of the node
    pub async fn get_node_health(&self, node_id: &NodeId) -> Option<HealthStatus> {
        self.health_monitor.get_node_health(node_id)
    }

    /// Get all unhealthy nodes
    pub async fn get_unhealthy_nodes(&self) -> Vec<NodeId> {
        self.health_monitor.get_unhealthy_nodes()
    }

    /// Join the cluster
    ///
    /// # Arguments
    /// * `seed_nodes` - Initial seed nodes to contact
    pub async fn join_cluster(&self, seed_nodes: Vec<NodeAddress>) -> Result<()> {
        let membership = self.membership.read().await;
        membership.join_cluster(self.local_node.clone(), seed_nodes).await?;
        Ok(())
    }

    /// Leave the cluster gracefully
    pub async fn leave_cluster(&self) -> Result<()> {
        let membership = self.membership.read().await;
        membership.leave_cluster().await?;
        Ok(())
    }

    /// Get network statistics
    pub async fn get_stats(&self) -> NetworkStats {
        let stats = self.stats.read().await;
        stats.clone()
    }

    /// Get the local node ID
    pub fn local_node_id(&self) -> &NodeId {
        &self.local_node.id
    }

    /// Get the network configuration
    pub fn config(&self) -> &NetworkConfig {
        &self.config
    }

    /// Start the message receive loop
    async fn start_receive_loop(self: Arc<Self>) -> Result<()> {
        let transport = self.transport.clone();
        let event_tx = self.event_tx.clone();
        let stats = self.stats.clone();
        let running = self.running.clone();

        tokio::spawn(async move {
            while *running.read().await {
                match transport.receive().await {
                    Ok((source, message)) => {
                        // Update statistics
                        {
                            let mut s = stats.write().await;
                            s.messages_received += 1;
                        }

                        // Convert address to node ID (simplified - in real implementation,
                        // we'd look up the node ID from the address)
                        let node_id = NodeId::new(source.to_string());

                        // Send to event bus
                        if let Err(e) = event_tx.send(NetworkEvent::MessageReceived {
                            from: node_id,
                            message,
                        }).await {
                            eprintln!("Failed to send message to event bus: {}", e);
                        }
                    }
                    Err(e) => {
                        eprintln!("Error receiving message: {}", e);
                        // Don't break the loop, continue trying to receive
                    }
                }
            }
        });

        Ok(())
    }

    /// Start the event processing loop
    async fn start_event_loop(self: Arc<Self>) -> Result<()> {
        let event_rx = self.event_rx.clone();
        let handlers = self.message_handlers.clone();
        let running = self.running.clone();

        tokio::spawn(async move {
            let mut rx = event_rx.write().await;

            while *running.read().await {
                if let Some(event) = rx.recv().await {
                    match event {
                        NetworkEvent::MessageReceived { from, message } => {
                            let message_type = message.message_type().to_string();

                            // Find and invoke the appropriate handler
                            let handlers_map = handlers.read().await;
                            if let Some(handler) = handlers_map.get(&message_type) {
                                if let Err(e) = handler.handle_message(&from, &message).await {
                                    eprintln!("Error handling message: {}", e);
                                }
                            } else {
                                eprintln!("No handler registered for message type: {}", message_type);
                            }
                        }
                        NetworkEvent::NodeJoined(node) => {
                            println!("Node joined: {}", node.id);
                        }
                        NetworkEvent::NodeLeft(node_id) => {
                            println!("Node left: {}", node_id);
                        }
                        NetworkEvent::NodeHealthChanged { node_id, status } => {
                            println!("Node {} health changed to {:?}", node_id, status);
                        }
                        NetworkEvent::Shutdown => {
                            println!("Shutdown event received");
                            break;
                        }
                    }
                }
            }
        });

        Ok(())
    }

    /// Start the health monitoring loop
    async fn start_health_loop(self: Arc<Self>) -> Result<()> {
        let health_monitor = self.health_monitor.clone();
        let event_tx = self.event_tx.clone();
        let running = self.running.clone();
        let interval_ms = self.config.health_check_config.interval_ms;

        tokio::spawn(async move {
            let mut ticker = interval(Duration::from_millis(interval_ms));

            while *running.read().await {
                ticker.tick().await;

                if let Err(e) = health_monitor.start_monitoring().await {
                    eprintln!("Error in health monitoring: {}", e);
                }
            }
        });

        Ok(())
    }

    /// Start the gossip loop for membership dissemination
    async fn start_gossip_loop(self: Arc<Self>) -> Result<()> {
        let membership = self.membership.clone();
        let running = self.running.clone();

        tokio::spawn(async move {
            let mut ticker = interval(Duration::from_secs(1)); // Gossip every second

            while *running.read().await {
                ticker.tick().await;

                let m = membership.read().await;
                if let Err(e) = m.gossip_round().await {
                    eprintln!("Error in gossip round: {}", e);
                }
            }
        });

        Ok(())
    }
}

impl Component for NetworkManager {
    fn initialize(&mut self) -> Result<()> {
        // Initialize all components
        // Note: In a real implementation, we would need mutable access to components
        // This is a simplified version
        println!("NetworkManager initializing...");

        Ok(())
    }

    fn shutdown(&mut self) -> Result<()> {
        println!("NetworkManager shutting down...");

        // Send shutdown event
        let event_tx = self.event_tx.clone();
        tokio::spawn(async move {
            let _ = event_tx.send(NetworkEvent::Shutdown).await;
        });

        Ok(())
    }

    fn health_check(&self) -> HealthStatus {
        // Aggregate health from all components
        // In a real implementation, we would check the health of each component
        HealthStatus::Healthy
    }
}

// ============================================================================
// Helper Functions (Test Only)
// ============================================================================

/// Create a default NetworkManager with mock implementations
///
/// This function is only available for testing and development.
/// In production, use NetworkManagerBuilder to configure real implementations.
#[cfg(test)]
pub fn create_default_manager(
    config: NetworkConfig,
    local_node: NodeInfo,
) -> NetworkManager {
    NetworkManager::new(
        config,
        local_node,
        Arc::new(mock::MockTransport::new()),
        Arc::new(mock::MockServiceDiscovery::new()),
        Arc::new(mock::MockHealthMonitor::new()),
        Arc::new(RwLock::new(mock::MockLoadBalancer::new())),
        Arc::new(RwLock::new(mock::MockClusterMembership::new())),
    )
}

// ============================================================================
// NetworkManager Builder
// ============================================================================

/// Builder for constructing a NetworkManager with custom components
pub struct NetworkManagerBuilder {
    config: Option<NetworkConfig>,
    local_node: Option<NodeInfo>,
    transport: Option<Arc<dyn NetworkTransport>>,
    service_discovery: Option<Arc<dyn ServiceDiscovery>>,
    health_monitor: Option<Arc<dyn HealthMonitor>>,
    load_balancer: Option<Arc<RwLock<dyn LoadBalancer>>>,
    membership: Option<Arc<RwLock<dyn ClusterMembership>>>,
}

impl NetworkManagerBuilder {
    /// Create a new builder
    pub fn new() -> Self {
        Self {
            config: None,
            local_node: None,
            transport: None,
            service_discovery: None,
            health_monitor: None,
            load_balancer: None,
            membership: None,
        }
    }

    /// Set the network configuration
    pub fn config(mut self, config: NetworkConfig) -> Self {
        self.config = Some(config);
        self
    }

    /// Set the local node information
    pub fn local_node(mut self, node: NodeInfo) -> Self {
        self.local_node = Some(node);
        self
    }

    /// Set the network transport
    pub fn transport(mut self, transport: Arc<dyn NetworkTransport>) -> Self {
        self.transport = Some(transport);
        self
    }

    /// Set the service discovery
    pub fn service_discovery(mut self, discovery: Arc<dyn ServiceDiscovery>) -> Self {
        self.service_discovery = Some(discovery);
        self
    }

    /// Set the health monitor
    pub fn health_monitor(mut self, monitor: Arc<dyn HealthMonitor>) -> Self {
        self.health_monitor = Some(monitor);
        self
    }

    /// Set the load balancer
    pub fn load_balancer(mut self, balancer: Arc<RwLock<dyn LoadBalancer>>) -> Self {
        self.load_balancer = Some(balancer);
        self
    }

    /// Set the cluster membership
    pub fn membership(mut self, membership: Arc<RwLock<dyn ClusterMembership>>) -> Self {
        self.membership = Some(membership);
        self
    }

    /// Build the NetworkManager
    pub fn build(self) -> Result<NetworkManager> {
        let config = self.config
            .ok_or_else(|| DbError::Configuration("Network config required".to_string()))?;
        let local_node = self.local_node
            .ok_or_else(|| DbError::Configuration("Local node info required".to_string()))?;
        let transport = self.transport
            .ok_or_else(|| DbError::Configuration("Network transport required".to_string()))?;
        let service_discovery = self.service_discovery
            .ok_or_else(|| DbError::Configuration("Service discovery required".to_string()))?;
        let health_monitor = self.health_monitor
            .ok_or_else(|| DbError::Configuration("Health monitor required".to_string()))?;
        let load_balancer = self.load_balancer
            .ok_or_else(|| DbError::Configuration("Load balancer required".to_string()))?;
        let membership = self.membership
            .ok_or_else(|| DbError::Configuration("Cluster membership required".to_string()))?;

        Ok(NetworkManager::new(
            config,
            local_node,
            transport,
            service_discovery,
            health_monitor,
            load_balancer,
            membership,
        ))
    }
}

impl Default for NetworkManagerBuilder {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Mock Implementations Module (Test Only)
// ============================================================================

/// Mock implementations for testing only
#[cfg(test)]
pub(crate) mod mock {
    use super::*;
    use async_trait::async_trait;

    pub(crate) struct MockTransport {
        local_address: NodeAddress,
    }
    impl MockTransport {
        pub(crate) fn new() -> Self {
            Self {
                local_address: NodeAddress::new("localhost", 7000),
            }
        }
    }

    #[async_trait]
    impl NetworkTransport for MockTransport {
        async fn send(&self, _addr: &NodeAddress, _msg: &ClusterMessage) -> Result<()> {
            Ok(())
        }
        async fn send_to_node(&self, _node_id: &NodeId, _msg: &ClusterMessage) -> Result<()> {
            Ok(())
        }
        async fn receive(&self) -> Result<(NodeAddress, ClusterMessage)> {
            tokio::time::sleep(Duration::from_secs(1)).await;
            Err(DbError::Network("Mock receive".to_string()))
        }
        async fn broadcast(&self, _msg: &ClusterMessage) -> Result<usize> {
            Ok(0)
        }
        fn active_connections(&self) -> usize { 0 }
        fn local_address(&self) -> &NodeAddress {
            &self.local_address
        }
        async fn close_connection(&self, _node_id: &NodeId) -> Result<()> {
            Ok(())
        }
        fn connection_stats(&self) -> HashMap<NodeId, crate::networking::traits::ConnectionStats> {
            HashMap::new()
        }
    }

    impl Component for MockTransport {
        fn initialize(&mut self) -> Result<()> { Ok(()) }
        fn shutdown(&mut self) -> Result<()> { Ok(()) }
        fn health_check(&self) -> HealthStatus { HealthStatus::Healthy }
    }

    pub(crate) struct MockServiceDiscovery;
    impl MockServiceDiscovery {
        pub(crate) fn new() -> Self { Self }
    }

    #[async_trait]
    impl ServiceDiscovery for MockServiceDiscovery {
        async fn register_node(&self, _node: &NodeInfo) -> Result<()> { Ok(()) }
        async fn unregister_node(&self) -> Result<()> { Ok(()) }
        async fn discover_nodes(&self) -> Result<Vec<NodeInfo>> { Ok(Vec::new()) }
        async fn get_node(&self, _node_id: &NodeId) -> Result<Option<NodeInfo>> { Ok(None) }
        async fn watch_changes(&self) -> Result<mpsc::Receiver<MembershipEvent>> {
            let (_tx, rx) = mpsc::channel(1);
            Ok(rx)
        }
        async fn update_metadata(&self, _metadata: HashMap<String, String>) -> Result<()> {
            Ok(())
        }
    }

    impl Component for MockServiceDiscovery {
        fn initialize(&mut self) -> Result<()> { Ok(()) }
        fn shutdown(&mut self) -> Result<()> { Ok(()) }
        fn health_check(&self) -> HealthStatus { HealthStatus::Healthy }
    }

    pub(crate) struct MockHealthMonitor;
    impl MockHealthMonitor {
        pub(crate) fn new() -> Self { Self }
    }

    #[async_trait]
    impl HealthMonitor for MockHealthMonitor {
        async fn check_health(&self, _node_id: &NodeId) -> Result<crate::networking::types::HealthCheckResult> {
            Err(DbError::NotImplemented("Mock".to_string()))
        }
        async fn check_all_nodes(&self) -> Result<HashMap<NodeId, crate::networking::types::HealthCheckResult>> {
            Ok(HashMap::new())
        }
        fn get_node_health(&self, _node_id: &NodeId) -> Option<HealthStatus> { None }
        fn get_unhealthy_nodes(&self) -> Vec<NodeId> { Vec::new() }
        fn get_healthy_nodes(&self) -> Vec<NodeId> { Vec::new() }
        async fn start_monitoring(&self) -> Result<()> { Ok(()) }
        async fn stop_monitoring(&self) -> Result<()> { Ok(()) }
        async fn subscribe_health_changes(&self) -> Result<mpsc::Receiver<crate::networking::traits::HealthChangeEvent>> {
            let (_tx, rx) = mpsc::channel(1);
            Ok(rx)
        }
    }

    impl Component for MockHealthMonitor {
        fn initialize(&mut self) -> Result<()> { Ok(()) }
        fn shutdown(&mut self) -> Result<()> { Ok(()) }
        fn health_check(&self) -> HealthStatus { HealthStatus::Healthy }
    }

    pub(crate) struct MockLoadBalancer;
    impl MockLoadBalancer {
        pub(crate) fn new() -> Self { Self }
    }

    impl LoadBalancer for MockLoadBalancer {
        fn select_node(&self, _criteria: &SelectionCriteria) -> Option<NodeId> { None }
        fn select_nodes(&self, _criteria: &SelectionCriteria, _count: usize) -> Vec<NodeId> {
            Vec::new()
        }
        fn update_node_weight(&mut self, _node_id: &NodeId, _weight: f64) {}
        fn update_weights(&mut self, _weights: HashMap<NodeId, f64>) {}
        fn mark_node_unavailable(&mut self, _node_id: &NodeId) {}
        fn mark_node_available(&mut self, _node_id: &NodeId) {}
        fn get_load_distribution(&self) -> HashMap<NodeId, usize> { HashMap::new() }
        fn record_request_completion(&mut self, _node_id: &NodeId) {}
    }

    impl Component for MockLoadBalancer {
        fn initialize(&mut self) -> Result<()> { Ok(()) }
        fn shutdown(&mut self) -> Result<()> { Ok(()) }
        fn health_check(&self) -> HealthStatus { HealthStatus::Healthy }
    }

    pub(crate) struct MockClusterMembership {
        local_node_id: NodeId,
    }
    impl MockClusterMembership {
        pub(crate) fn new() -> Self {
            Self {
                local_node_id: NodeId::new("mock"),
            }
        }
    }

    #[async_trait]
    impl ClusterMembership for MockClusterMembership {
        fn get_members(&self) -> Vec<NodeInfo> { Vec::new() }
        fn get_member(&self, _node_id: &NodeId) -> Option<NodeInfo> { None }
        fn update_member_state(&mut self, _node_id: &NodeId, _state: crate::networking::types::NodeState) {}
        async fn join_cluster(&self, _local_node: NodeInfo, _seed_nodes: Vec<NodeAddress>) -> Result<()> {
            Ok(())
        }
        async fn leave_cluster(&self) -> Result<()> { Ok(()) }
        fn add_member(&mut self, _node: NodeInfo) {}
        fn remove_member(&mut self, _node_id: &NodeId) {}
        fn local_node_id(&self) -> &NodeId {
            &self.local_node_id
        }
        fn member_count(&self) -> usize { 0 }
        fn is_member(&self, _node_id: &NodeId) -> bool { false }
        async fn subscribe_membership_changes(&self) -> Result<mpsc::Receiver<MembershipEvent>> {
            let (_tx, rx) = mpsc::channel(1);
            Ok(rx)
        }
        async fn gossip_round(&self) -> Result<()> { Ok(()) }
    }

    impl Component for MockClusterMembership {
        fn initialize(&mut self) -> Result<()> { Ok(()) }
        fn shutdown(&mut self) -> Result<()> { Ok(()) }
        fn health_check(&self) -> HealthStatus { HealthStatus::Healthy }
    }
}
