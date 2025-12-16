// Cluster Network Module
//
// This module provides enterprise-grade network clustering and high availability.
//
// REFACTORING STRUCTURE (COMPLETED):
// - topology: SWIM protocol, ClusterTopologyManager, PartitionDetector (COMPLETED)
// - communication: Inter-node messaging, NodeConnectionPool, GossipProtocol (COMPLETED)
// - load_balancing: ClusterLoadBalancer, routing strategies, hotspot detection (COMPLETED)
// - failover: FailoverCoordinator, RaftLeaderElection, session migration (COMPLETED)
// - health_monitoring: NetworkHealthMonitor, metrics tracking, route optimization (COMPLETED)

// Module declarations
pub mod topology;
pub mod communication;
pub mod load_balancing;
pub mod failover;
pub mod health_monitoring;

// Re-export core types
pub type NodeId = u64;

// Node state and info
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NodeState {
    Alive,
    Suspect,
    Dead,
    Left,
    Joining,
}

#[derive(Debug, Clone)]
pub struct NodeInfo {
    pub id: NodeId,
    pub address: std::net::SocketAddr,
    pub state: NodeState,
    pub metadata: std::collections::HashMap<String, String>,
    pub incarnation: i32,
    pub last_seen: std::time::Instant,
    pub datacenter: String,
    pub rack: String,
    pub capacity: NodeCapacity,
}

#[derive(Debug, Clone)]
pub struct NodeCapacity {
    pub cpu: f64,
    pub memory: u64,
    pub connections: usize,
    pub cpu_cores: i32,
    pub memory_gb: i32,
    pub max_connections: i32,
    pub current_connections: i32,
    pub query_latency_ms: f64,
    pub disk_io_utilization: f64,
}

// Re-export topology types
pub use topology::{
    ClusterTopologyManager, MembershipEvent, NodeUpdate, PartitionDetector, PartitionStatus,
    QuorumConfig, SwimConfig, SwimMessage, TopologyMetrics,
};

// Re-export communication types
pub use communication::{
    ClusterMessage, CommunicationMetrics, GossipProtocol, MessagePriority, NodeConnection,
    NodeConnectionPool, ReliableMessaging, TlsConfig,
};

// Re-export load balancing types
pub use load_balancing::{
    ClusterLoadBalancer, ConnectionAffinity, HotspotDetector, LoadBalancerMetrics, LocalityMap,
    RoutingStrategy,
};

// Re-export failover types
pub use failover::{
    FailoverCoordinator, FailoverMetrics, RaftLeaderElection, RollingRestartCoordinator,
    SessionMigrationManager, TransactionRecoveryManager,
};

// Re-export health monitoring types
pub use health_monitoring::{
    BandwidthMonitor, HealthCheckResult, HealthMetrics, LatencyTracker, NetworkHealthMonitor,
    NetworkQualityScorer, NodeNetworkMetrics, PacketLossDetector, RouteOptimization,
    RouteOptimizer,
};

// ============================================================================
// Public API - Cluster Network Manager
// ============================================================================

pub struct ClusterNetworkManager {
    topology: ClusterTopologyManager,
    load_balancer: ClusterLoadBalancer,
    health_monitor: NetworkHealthMonitor,
}

impl ClusterNetworkManager {
    pub fn new(strategy: RoutingStrategy) -> Self {
        Self {
            topology: ClusterTopologyManager::new(),
            load_balancer: ClusterLoadBalancer::new(strategy),
            health_monitor: NetworkHealthMonitor::default(),
        }
    }

    pub fn topology(&self) -> &ClusterTopologyManager {
        &self.topology
    }

    pub fn topology_mut(&mut self) -> &mut ClusterTopologyManager {
        &mut self.topology
    }

    pub fn load_balancer(&self) -> &ClusterLoadBalancer {
        &self.load_balancer
    }

    pub fn load_balancer_mut(&mut self) -> &mut ClusterLoadBalancer {
        &mut self.load_balancer
    }

    pub fn health_monitor(&self) -> &NetworkHealthMonitor {
        &self.health_monitor
    }

    pub fn health_monitor_mut(&mut self) -> &mut NetworkHealthMonitor {
        &mut self.health_monitor
    }
}
