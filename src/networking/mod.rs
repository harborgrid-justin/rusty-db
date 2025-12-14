// P2P Networking layer for distributed RustyDB
//
// This module provides the core peer-to-peer communication infrastructure
// for RustyDB's distributed database architecture. It enables nodes in a
// cluster to communicate reliably and efficiently.
//
// # Architecture
//
// The networking layer is organized into two main subsystems:
//
// ## Transport Layer (`transport`)
//
// Handles the low-level mechanics of establishing and maintaining connections:
//
// - **TCP Transport**: Reliable, widely-supported TCP/IP networking
// - **QUIC Transport**: Modern UDP-based transport with built-in encryption (planned)
// - **Connection Management**: Pooling, health monitoring, and reconnection
// - **Metrics**: Comprehensive connection and throughput tracking
//
// ## Protocol Layer (`protocol`)
//
// Defines the wire protocol for message exchange:
//
// - **Message Framing**: Length-prefixed binary protocol
// - **Serialization**: Efficient binary encoding with bincode
// - **Compression**: Optional LZ4/Zstd compression
// - **Handshake**: Protocol versioning and capability negotiation
// - **Checksums**: Data integrity verification
//
// # Example Usage
//
// ## Server-side: Accept connections
//
// ```rust,no_run
// use rusty_db::networking::transport::{TcpTransport, TcpConfig};
// use rusty_db::networking::protocol::{MessageCodec, Message};
//
// # async fn example() -> rusty_db::Result<()> {
// let config = TcpConfig::default();
// let mut transport = TcpTransport::new(config);
// transport.bind().await?;
//
// // Accept a connection
// let connection = transport.accept().await?;
// println!("Accepted connection from {}", connection.peer_addr());
// # Ok(())
// # }
// ```
//
// ## Client-side: Connect to a peer
//
// ```rust,no_run
// use rusty_db::networking::transport::{TcpTransport, TcpConfig, ConnectionPool, PoolConfig};
// use std::sync::Arc;
//
// # async fn example() -> rusty_db::Result<()> {
// // Create transport
// let tcp_config = TcpConfig::default();
// let transport = TcpTransport::new(tcp_config);
//
// // Connect to a peer
// let peer_addr = "127.0.0.1:9000".parse().unwrap();
// let peer_id = "node2".to_string();
// let connection = transport.connect(peer_addr, peer_id).await?;
// # Ok(())
// # }
// ```
//
// ## Message exchange
//
// ```rust,no_run
// use rusty_db::networking::protocol::{MessageCodec, Message};
//
// # fn example() -> rusty_db::Result<()> {
// let codec = MessageCodec::new();
//
// // Encode a message
// let message = Message::Ping { timestamp: 12345 };
// let encoded = codec.encode(1, &message)?;
//
// // Decode a message
// let (message_id, decoded) = codec.decode(encoded)?;
// # Ok(())
// # }
// ```
//
// # Features
//
// - **Multi-transport**: Support for TCP and QUIC (future)
// - **Auto-reconnection**: Exponential backoff retry logic
// - **Connection pooling**: Efficient connection reuse
// - **Health monitoring**: Automatic detection of failed connections
// - **Load balancing**: Multiple connection selection strategies
// - **Protocol versioning**: Backward-compatible protocol evolution
// - **Compression**: Optional message compression
// - **Checksums**: Data integrity verification
//
// # Design Principles
//
// 1. **Reliability**: Automatic reconnection and error recovery
// 2. **Performance**: Low-latency, high-throughput communication
// 3. **Scalability**: Efficient handling of many concurrent connections
// 4. **Security**: Support for TLS/encryption (QUIC built-in, TCP via upgrade)
// 5. **Observability**: Comprehensive metrics and logging

// Core modules
pub mod autodiscovery;
pub mod discovery;
pub mod loadbalancer;
pub mod pool;
pub mod protocol;
pub mod security;
pub mod transport;

// High-level coordination modules
pub mod api;
pub mod graphql;
pub mod manager;
pub mod traits;
pub mod types;

// Message routing and delivery
mod health;
mod membership;
pub mod routing;

// Re-export commonly used types for convenience
pub use protocol::{
    CompressionType, Handshake, HandshakeRequest, HandshakeResponse, Message, MessageCodec,
    NodeCapabilities, ProtocolCodec, ProtocolFlags,
};

pub use transport::{
    Connection, ConnectionPool, ConnectionState, PoolConfig, PoolStatistics, QuicConfig,
    QuicConnection, QuicTransport, SelectionStrategy, TcpConfig, TcpConnection, TcpTransport,
    TransportManager, TransportType,
};

// Re-export pool types (with aliases to avoid conflicts)
pub use pool::{
    ChannelPool, ChannelRequest, ConnectionMetrics, EvictionManager, EvictionPolicy,
    MultiplexedConnection, NodeConnection, NodePool, PoolConfig as NodePoolConfig,
    PoolManager as NodePoolManager, PoolMetrics, RequestChannel, Stream, StreamId, StreamMetrics,
    StreamPriority, WarmupManager, WarmupStrategy,
};

// Re-export load balancer types
pub use loadbalancer::{
    AdaptiveBalancer, Backend, BackendStatistics, ConsistentHashBalancer, LeastConnectionsBalancer,
    LoadBalancer, LoadBalancerContext, LoadBalancingStrategy, RoundRobinBalancer,
};

// Re-export circuit breaker types
pub use loadbalancer::circuit_breaker::{CircuitBreaker, CircuitState};

// Re-export retry types
pub use loadbalancer::retry::{RetryBudget, RetryPolicy, RetryStrategy};

// Re-export traffic shaping types
pub use loadbalancer::traffic_shaping::{RateLimiter, TrafficShaper};

// Re-export security types
pub use security::{
    AclRule, Action, ApplicationFirewall, AuthContext, CertificateConfig, CertificateManager,
    CipherSuite, EncryptionConfig, FirewallConfig, IdentityProvider, MessageEncryption,
    MtlsAuthenticator, MtlsConfig, NetworkAcl, NodeIdentity, SecurityConfig, SecurityManager,
    TlsConfig, TlsVersion,
};

// ============================================================================
// High-Level Coordination Layer - Re-exports
// ============================================================================

// Re-export types from the coordination layer
pub use types::{
    ClusterMessage as CoordClusterMessage, CompressionType as CoordCompressionType, ConnectionInfo,
    HealthCheckConfig, HealthCheckResult, LoadBalancingConfig,
    LoadBalancingStrategy as CoordLoadBalancingStrategy, MembershipEvent, MessageEnvelope,
    MessagePriority, NetworkConfig, NetworkStats, NodeAddress, NodeId, NodeInfo, NodeState,
    PeerInfo, RoutingStrategy, SelectionCriteria, ServiceDiscoveryConfig, ServiceDiscoveryType,
    TlsConfig as CoordTlsConfig, TlsVersion as CoordTlsVersion,
};

// Re-export traits from the coordination layer
pub use traits::{
    CircuitBreaker as CoordCircuitBreaker, ClusterMembership, Connection as CoordConnection,
    ConnectionPool as CoordConnectionPool, ConnectionStats, HealthChangeEvent, HealthMonitor,
    LoadBalancer as CoordLoadBalancer, MessageHandler, NetworkTransport, ServiceDiscovery,
};

// Re-export manager
pub use manager::{create_default_manager, NetworkManager, NetworkManagerBuilder};

// Re-export API components
pub use api::{
    create_router as create_api_router, ApiState, HealthResponse, JoinClusterRequest,
    JoinClusterResponse, LeaveClusterResponse, NodeHealthResponse, PeersQuery, PeersResponse,
    StatsResponse, TopologyResponse,
};

// Re-export GraphQL components
pub use graphql::{
    create_schema as create_graphql_schema, GqlContext, GqlMembershipEvent, GqlNetworkStats,
    GqlNodeInfo, GqlPeerInfo, GqlTopology, MutationRoot, NetworkSchema, QueryRoot,
    SubscriptionRoot,
};

// Re-export autodiscovery components
pub use autodiscovery::{
    AntiEntropyEngine, AutoDiscovery, BeaconProtocol, BroadcastDiscovery, CrdtCounter,
    DiscoveryBackend, DiscoveryConfig, DiscoveryEvent, DiscoveryProtocol, GossipDiscovery,
    MdnsDiscovery, Member, MemberState, MembershipDelta, MembershipList, MembershipSnapshot,
    MerkleTree, NodeInfo as DiscoveryNodeInfo, NodeStatus, SerfProtocol, VersionVector,
};

// Re-export discovery components (enterprise service discovery)
pub use discovery::{
    CloudConfig, CloudProvider, ConsulConfig, DiscoveryConfig as EnterpriseDiscoveryConfig,
    DiscoveryEvent as ServiceDiscoveryEvent, DnsConfig, EtcdConfig,
    HealthStatus as NodeHealthStatus, KubernetesConfig, Node as DiscoveredNode,
    Registry as ServiceDiscoveryRegistry, StaticConfig,
};

// Re-export routing components
pub use routing::{
    // Serialization
    BinaryCodec,
    // Dispatcher results
    BroadcastResult,
    ClusterMessage as RoutingClusterMessage,
    CoordinatedBroadcastResult,

    DataReadRequest,
    DataReadResponse,
    DataWriteRequest,
    DataWriteResponse,
    DatacenterId,

    DeadLetterMessage,

    // Delivery guarantees
    DeliveryGuarantee,
    DeliveryTracker,
    FanOutResult,
    IdempotencyKey,

    MessageDispatcher,
    // Message handler
    MessageHandler as RoutingMessageHandler,

    // Core routing
    MessageRouter,
    MulticastResult,
    PingRequest,
    PingResponse,
    QueryRpcRequest,
    QueryRpcResponse,

    // Message queue
    QueueManager,
    QueueStats,
    QueuedMessage,
    QuorumResult,
    Request as RpcRequest,
    RequestId,

    RouteVersion,
    // Statistics
    RouterStats,
    RoutingTable,
    RoutingTableSnapshot,
    // RPC framework
    RpcClient,
    RpcHandler,
    RpcServer,
    ScatterGatherResult,
    ShardId as RoutingShardId,
    ShardResult,
};
