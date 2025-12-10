//! P2P Networking layer for distributed RustyDB
//!
//! This module provides the core peer-to-peer communication infrastructure
//! for RustyDB's distributed database architecture. It enables nodes in a
//! cluster to communicate reliably and efficiently.
//!
//! # Architecture
//!
//! The networking layer is organized into two main subsystems:
//!
//! ## Transport Layer (`transport`)
//!
//! Handles the low-level mechanics of establishing and maintaining connections:
//!
//! - **TCP Transport**: Reliable, widely-supported TCP/IP networking
//! - **QUIC Transport**: Modern UDP-based transport with built-in encryption (planned)
//! - **Connection Management**: Pooling, health monitoring, and reconnection
//! - **Metrics**: Comprehensive connection and throughput tracking
//!
//! ## Protocol Layer (`protocol`)
//!
//! Defines the wire protocol for message exchange:
//!
//! - **Message Framing**: Length-prefixed binary protocol
//! - **Serialization**: Efficient binary encoding with bincode
//! - **Compression**: Optional LZ4/Zstd compression
//! - **Handshake**: Protocol versioning and capability negotiation
//! - **Checksums**: Data integrity verification
//!
//! # Example Usage
//!
//! ## Server-side: Accept connections
//!
//! ```rust,no_run
//! use rusty_db::networking::transport::{TcpTransport, TcpConfig};
//! use rusty_db::networking::protocol::{MessageCodec, Message};
//!
//! # async fn example() -> rusty_db::Result<()> {
//! let config = TcpConfig::default();
//! let mut transport = TcpTransport::new(config);
//! transport.bind().await?;
//!
//! // Accept a connection
//! let connection = transport.accept().await?;
//! println!("Accepted connection from {}", connection.peer_addr());
//! # Ok(())
//! # }
//! ```
//!
//! ## Client-side: Connect to a peer
//!
//! ```rust,no_run
//! use rusty_db::networking::transport::{TcpTransport, TcpConfig, ConnectionPool, PoolConfig};
//! use std::sync::Arc;
//!
//! # async fn example() -> rusty_db::Result<()> {
//! // Create transport
//! let tcp_config = TcpConfig::default();
//! let transport = TcpTransport::new(tcp_config);
//!
//! // Connect to a peer
//! let peer_addr = "127.0.0.1:9000".parse().unwrap();
//! let peer_id = "node2".to_string();
//! let connection = transport.connect(peer_addr, peer_id).await?;
//! # Ok(())
//! # }
//! ```
//!
//! ## Message exchange
//!
//! ```rust,no_run
//! use rusty_db::networking::protocol::{MessageCodec, Message};
//!
//! # fn example() -> rusty_db::Result<()> {
//! let codec = MessageCodec::new();
//!
//! // Encode a message
//! let message = Message::Ping { timestamp: 12345 };
//! let encoded = codec.encode(1, &message)?;
//!
//! // Decode a message
//! let (message_id, decoded) = codec.decode(encoded)?;
//! # Ok(())
//! # }
//! ```
//!
//! # Features
//!
//! - **Multi-transport**: Support for TCP and QUIC (future)
//! - **Auto-reconnection**: Exponential backoff retry logic
//! - **Connection pooling**: Efficient connection reuse
//! - **Health monitoring**: Automatic detection of failed connections
//! - **Load balancing**: Multiple connection selection strategies
//! - **Protocol versioning**: Backward-compatible protocol evolution
//! - **Compression**: Optional message compression
//! - **Checksums**: Data integrity verification
//!
//! # Design Principles
//!
//! 1. **Reliability**: Automatic reconnection and error recovery
//! 2. **Performance**: Low-latency, high-throughput communication
//! 3. **Scalability**: Efficient handling of many concurrent connections
//! 4. **Security**: Support for TLS/encryption (QUIC built-in, TCP via upgrade)
//! 5. **Observability**: Comprehensive metrics and logging

// Core modules
pub mod protocol;
pub mod transport;
pub mod pool;
pub mod loadbalancer;
pub mod security;
pub mod autodiscovery;
pub mod discovery;

// High-level coordination modules
pub mod types;
pub mod traits;
pub mod manager;
pub mod api;
pub mod graphql;

// Message routing and delivery
pub mod routing;

// Re-export commonly used types for convenience
pub use protocol::{
    Message, MessageCodec, ProtocolCodec,
    Handshake, HandshakeRequest, HandshakeResponse, NodeCapabilities,
    CompressionType, ProtocolFlags,
};

pub use transport::{
    Connection, ConnectionState, TransportType,
    ConnectionPool, PoolConfig, PoolStatistics, SelectionStrategy,
    TcpTransport, TcpConfig, TcpConnection,
    QuicTransport, QuicConfig, QuicConnection,
    TransportManager,
};

// Re-export pool types (with aliases to avoid conflicts)
pub use pool::{
    PoolManager as NodePoolManager,
    PoolConfig as NodePoolConfig,
    NodePool, NodeConnection,
    MultiplexedConnection, Stream, StreamId, StreamPriority,
    ChannelPool, RequestChannel, ChannelRequest,
    WarmupStrategy, WarmupManager,
    EvictionPolicy, EvictionManager,
    PoolMetrics, ConnectionMetrics, StreamMetrics,
};

// Re-export load balancer types
pub use loadbalancer::{
    LoadBalancer, Backend, LoadBalancerContext, BackendStatistics,
    LoadBalancingStrategy, RoundRobinBalancer, LeastConnectionsBalancer,
    ConsistentHashBalancer, AdaptiveBalancer,
};

// Re-export circuit breaker types
pub use loadbalancer::circuit_breaker::{CircuitBreaker, CircuitState};

// Re-export retry types
pub use loadbalancer::retry::{RetryPolicy, RetryStrategy, RetryBudget};

// Re-export traffic shaping types
pub use loadbalancer::traffic_shaping::{TrafficShaper, RateLimiter};

// Re-export security types
pub use security::{
    SecurityConfig, SecurityManager, AuthContext,
    TlsConfig, TlsVersion, CipherSuite,
    MtlsAuthenticator, MtlsConfig,
    CertificateManager, CertificateConfig,
    NodeIdentity, IdentityProvider,
    MessageEncryption, EncryptionConfig,
    NetworkAcl, AclRule, Action,
    ApplicationFirewall, FirewallConfig,
};

// ============================================================================
// High-Level Coordination Layer - Re-exports
// ============================================================================

// Re-export types from the coordination layer
pub use types::{
    ClusterMessage as CoordClusterMessage,
    CompressionType as CoordCompressionType,
    ConnectionInfo, HealthCheckConfig, HealthCheckResult,
    LoadBalancingConfig, LoadBalancingStrategy as CoordLoadBalancingStrategy,
    MembershipEvent, MessageEnvelope, MessagePriority,
    NetworkConfig, NetworkStats,
    NodeAddress, NodeId, NodeInfo, NodeState,
    PeerInfo, RoutingStrategy, SelectionCriteria,
    ServiceDiscoveryConfig, ServiceDiscoveryType,
    TlsConfig as CoordTlsConfig, TlsVersion as CoordTlsVersion,
};

// Re-export traits from the coordination layer
pub use traits::{
    CircuitBreaker as CoordCircuitBreaker,
    ClusterMembership,
    Connection as CoordConnection,
    ConnectionPool as CoordConnectionPool,
    ConnectionStats,
    HealthChangeEvent,
    HealthMonitor,
    LoadBalancer as CoordLoadBalancer,
    MessageHandler,
    NetworkTransport,
    ServiceDiscovery,
};

// Re-export manager
pub use manager::{NetworkManager, NetworkManagerBuilder};

// Re-export API components
pub use api::{
    create_router as create_api_router,
    ApiState, HealthResponse, JoinClusterRequest,
    JoinClusterResponse, LeaveClusterResponse,
    NodeHealthResponse, PeersQuery, PeersResponse,
    StatsResponse, TopologyResponse,
};

// Re-export GraphQL components
pub use graphql::{
    create_schema as create_graphql_schema,
    GqlContext, GqlMembershipEvent, GqlNetworkStats,
    GqlNodeInfo, GqlPeerInfo, GqlTopology,
    MutationRoot, NetworkSchema, QueryRoot,
    SubscriptionRoot,
};

// Re-export autodiscovery components
pub use autodiscovery::{
    AutoDiscovery, DiscoveryConfig, DiscoveryBackend,
    DiscoveryEvent, DiscoveryProtocol,
    NodeInfo as DiscoveryNodeInfo, NodeStatus,
    GossipDiscovery, MemberState,
    MdnsDiscovery,
    BroadcastDiscovery,
    BeaconProtocol,
    SerfProtocol,
    MembershipList, VersionVector, Member, MembershipDelta, MembershipSnapshot,
    AntiEntropyEngine, MerkleTree, CrdtCounter,
};

// Re-export discovery components (enterprise service discovery)
pub use discovery::{
    Registry as ServiceDiscoveryRegistry,
    Node as DiscoveredNode,
    HealthStatus as NodeHealthStatus,
    DiscoveryConfig as EnterpriseDiscoveryConfig,
    DiscoveryEvent as ServiceDiscoveryEvent,
    DnsConfig, StaticConfig, KubernetesConfig, ConsulConfig, EtcdConfig, CloudConfig,
    CloudProvider,
};

// Re-export routing components
pub use routing::{
    // Core routing
    MessageRouter, RoutingTable, RoutingTableSnapshot, MessageDispatcher,
    RouteVersion, ShardId as RoutingShardId, DatacenterId,

    // Delivery guarantees
    DeliveryGuarantee, DeliveryTracker, IdempotencyKey,

    // Message queue
    QueueManager, QueuedMessage, QueueStats, DeadLetterMessage,

    // RPC framework
    RpcClient, RpcServer, Request as RpcRequest, RpcHandler,
    PingRequest, PingResponse, DataReadRequest, DataReadResponse,
    DataWriteRequest, DataWriteResponse, QueryRpcRequest, QueryRpcResponse,

    // Message handler
    MessageHandler as RoutingMessageHandler,

    // Dispatcher results
    BroadcastResult, MulticastResult, ShardResult, FanOutResult,
    ScatterGatherResult, QuorumResult, CoordinatedBroadcastResult,

    // Serialization
    BinaryCodec, ClusterMessage as RoutingClusterMessage, RequestId,

    // Statistics
    RouterStats,
};
