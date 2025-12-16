// Advanced Network Protocol Module
//
// This module provides enterprise-grade wire protocol with comprehensive features.
//
// REFACTORING STRUCTURE (COMPLETED):
// - errors: Protocol error types (COMPLETED)
// - message_types: ProtocolVersion, MessageType, Packet, etc. (COMPLETED)
// - protocol_handlers: WireCodec, ProtocolNegotiator (COMPLETED)
// - connection_management: ConnectionState, ConnectionStateMachine (COMPLETED)
// - request_pipeline: RequestResponsePipeline, PriorityQueue (COMPLETED)
// - buffer_management: BufferPool, ScatterGather, etc. (COMPLETED)
// - protocol_extensions: ExtensionRegistry, FeatureFlags (COMPLETED)
// - flow_control: FlowControl, CircuitBreaker, RateLimiter (COMPLETED)

// Module declarations
pub mod errors;
pub mod message_types;
pub mod protocol_handlers;
pub mod connection_management;
pub mod request_pipeline;
pub mod buffer_management;
pub mod protocol_extensions;
pub mod flow_control;

// Re-export commonly used types for convenience
pub use errors::ProtocolError;

// Message types
pub use message_types::{
    CompressionType, MessageType, NegotiatedProtocol, Packet, PacketHeader, ProtocolCapabilities,
    ProtocolVersion, StreamChunk, StreamStats, StreamingResultSet,
};

// Protocol handlers
pub use protocol_handlers::{
    ProtocolNegotiator, WireCodec, WireCodecMetrics, WireCodecStats,
};

// Connection management
pub use connection_management::{
    ConnectionMetadata, ConnectionMetrics, ConnectionMigrator, ConnectionState,
    ConnectionStateMachine, ConnectionStats, StateTransition,
};

// Request pipeline
pub use request_pipeline::{
    PipelineMetrics, PipelineStats, PriorityRequestQueue, ProtocolRequest, ProtocolResponse,
    QueueStats, RequestId, RequestPriority, RequestResponsePipeline,
};

// Buffer management
pub use buffer_management::{
    BufferPool, BufferPoolConfig, BufferPoolMetrics, BufferPoolStats, CoalescingBuffer,
    LargeObjectStream, MemoryMappedTransfer, ScatterGatherBuffer,
};

// Protocol extensions
pub use protocol_extensions::{
    CustomMessageRegistry, ExtensionId, ExtensionNegotiator, ExtensionRegistry, FeatureFlags,
    ProtocolExtension, ProtocolHealthStatus, ProtocolManager,
};

// Flow control
pub use flow_control::{
    AggregateStats, BackendNode, CircuitBreaker, CircuitBreakerStats, CircuitState,
    ConnectionPool, ConnectionPoolStats, FlowControlManager, FlowControlPermit, FlowControlStats,
    LoadBalancerStats, LoadBalancingStrategy, MetricsSnapshot, PoolMetrics, PooledConnection,
    ProtocolLoadBalancer, ProtocolMetricsAggregator, RateLimiter, RateLimiterStats,
};
