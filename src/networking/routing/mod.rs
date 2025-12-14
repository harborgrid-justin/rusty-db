// Message routing and serialization for RustyDB cluster communication
//
// This module provides enterprise-grade message routing with efficient serialization,
// request routing, and message delivery guarantees for distributed database operations.
//
// # Architecture
//
// The routing system is organized into several key components:
//
// ## Core Components
//
// - **Router** (`router`): Central message router with request/response correlation,
//   async request handling, timeout management, and priority routing
// - **Routing Table** (`table`): Maps nodes to addresses, shards to nodes, with
//   datacenter-awareness and route versioning
// - **Dispatcher** (`dispatcher`): Implements fan-out messaging, scatter-gather,
//   broadcast, and multicast patterns for cluster-wide operations
//
// ## Reliability & Delivery
//
// - **Delivery Guarantees** (`delivery`): Implements at-most-once, at-least-once,
//   and exactly-once delivery semantics with idempotency support
// - **Message Queue** (`queue`): Priority-based queuing with backpressure,
//   per-peer outbound queues, and dead letter queue for failed messages
//
// ## Serialization
//
// - **Binary Codec** (`serialization::binary`): Efficient binary serialization using
//   bincode with optional compression (LZ4/Snappy/Zstd) and checksum verification
// - **Message Definitions** (`serialization::messages`): Type-safe message definitions
//   for membership, data operations, replication, and coordination
//
// ## High-Level Abstractions
//
// - **RPC Framework** (`rpc`): Type-safe RPC client/server with request/response
//   patterns, automatic retry, and timeout handling
//
// # Example Usage
//
// ## Setting up the routing infrastructure
//
// ```rust
// use rusty_db::networking::routing::{
//     MessageRouter, RoutingTable, MessageDispatcher, RpcClient,
// };
// use rusty_db::networking::types::{NodeId, NodeAddress};
// use std::sync::Arc;
// use std::time::Duration;
//
// # async fn example() -> rusty_db::Result<()> {
// // Create routing table
// let routing_table = RoutingTable::new();
//
// // Add nodes
// routing_table.add_node(
//     NodeId::new("node1"),
//     NodeAddress::new("192.168.1.10", 7000),
//     Some("dc1".to_string()),
// );
//
// // Create router
// let router = Arc::new(MessageRouter::new(routing_table));
//
// // Create dispatcher for cluster operations
// let dispatcher = MessageDispatcher::new(Arc::clone(&router));
//
// // Create RPC client
// let rpc_client = RpcClient::new(Arc::clone(&router));
// # Ok(())
// # }
// ```
//
// ## Sending messages
//
// ```rust,no_run
// use rusty_db::networking::routing::serialization::{ClusterMessage, HeartbeatMessage};
// use rusty_db::networking::types::{NodeId, MessagePriority};
// # use rusty_db::networking::routing::MessageRouter;
// # use rusty_db::networking::routing::RoutingTable;
// # use std::sync::Arc;
//
// # async fn example() -> rusty_db::Result<()> {
// # let router = Arc::new(MessageRouter::new(RoutingTable::new()));
// let message = ClusterMessage::Heartbeat(HeartbeatMessage {
//     node_id: NodeId::new("node1"),
//     timestamp: 12345,
//     sequence: 1,
// });
//
// // Fire-and-forget send
// router.send_message(
//     NodeId::new("node2"),
//     message,
//     MessagePriority::Normal,
// )?;
// # Ok(())
// # }
// ```
//
// ## Making RPC calls
//
// ```rust,no_run
// use rusty_db::networking::routing::rpc::{RpcClient, PingRequest};
// use rusty_db::networking::types::NodeId;
// # use rusty_db::networking::routing::MessageRouter;
// # use rusty_db::networking::routing::RoutingTable;
// # use std::sync::Arc;
//
// # async fn example() -> rusty_db::Result<()> {
// # let router = Arc::new(MessageRouter::new(RoutingTable::new()));
// let rpc_client = RpcClient::new(router);
//
// // Make a ping RPC call
// let response = rpc_client.call(
//     NodeId::new("node1"),
//     PingRequest { timestamp: 12345 },
// ).await?;
// # Ok(())
// # }
// ```
//
// ## Broadcasting to all nodes
//
// ```rust,no_run
// use rusty_db::networking::routing::{MessageDispatcher, MessageRouter};
// use rusty_db::networking::routing::serialization::{ClusterMessage, HeartbeatMessage};
// use rusty_db::networking::types::{NodeId, MessagePriority};
// # use rusty_db::networking::routing::RoutingTable;
// # use std::sync::Arc;
//
// # async fn example() -> rusty_db::Result<()> {
// # let router = Arc::new(MessageRouter::new(RoutingTable::new()));
// let dispatcher = MessageDispatcher::new(router);
//
// let message = ClusterMessage::Heartbeat(HeartbeatMessage {
//     node_id: NodeId::new("coordinator"),
//     timestamp: 12345,
//     sequence: 1,
// });
//
// // Broadcast to all nodes
// let result = dispatcher.broadcast(
//     message,
//     MessagePriority::High,
//     vec![], // no exclusions
// ).await?;
//
// println!("Broadcast sent to {} nodes", result.total_nodes);
// # Ok(())
// # }
// ```
//
// ## Scatter-gather pattern
//
// ```rust,no_run
// use rusty_db::networking::routing::{MessageDispatcher, MessageRouter};
// use rusty_db::networking::routing::serialization::{
//     ClusterMessage, QueryRequest, RequestId,
// };
// use rusty_db::networking::types::{NodeId, MessagePriority};
// use std::time::Duration;
// # use rusty_db::networking::routing::RoutingTable;
// # use std::sync::Arc;
//
// # async fn example() -> rusty_db::Result<()> {
// # let router = Arc::new(MessageRouter::new(RoutingTable::new()));
// let dispatcher = MessageDispatcher::new(router);
//
// let nodes = vec![
//     NodeId::new("node1"),
//     NodeId::new("node2"),
//     NodeId::new("node3"),
// ];
//
// let query = ClusterMessage::QueryRequest(QueryRequest {
//     request_id: RequestId::new(),
//     query: "SELECT * FROM table".to_string(),
//     params: vec![],
//     timeout_ms: 5000,
// });
//
// // Send to all nodes and collect responses
// let result = dispatcher.scatter_gather(
//     nodes,
//     query,
//     MessagePriority::Normal,
//     Duration::from_secs(5),
// ).await?;
//
// println!("Received {} responses", result.response_count);
// # Ok(())
// # }
// ```
//
// # Features
//
// - **Type-safe messaging**: Strongly-typed message definitions with serde
// - **Efficient serialization**: Bincode with optional compression
// - **Reliability**: Multiple delivery guarantees (at-most-once, at-least-once, exactly-once)
// - **Priority queuing**: Message prioritization for critical operations
// - **Backpressure**: Flow control to prevent overwhelming nodes
// - **Timeout management**: Configurable timeouts with automatic cleanup
// - **Dead letter queue**: Capture and analyze failed messages
// - **Datacenter awareness**: Route messages based on datacenter location
// - **Scatter-gather**: Parallel requests with response aggregation
// - **Quorum operations**: Wait for majority responses
// - **RPC abstraction**: High-level type-safe RPC interface

pub mod delivery;
pub mod dispatcher;
pub mod queue;
pub mod router;
pub mod rpc;
pub mod serialization;
pub mod table;

pub use delivery::{DeliveryGuarantee, DeliveryTracker, IdempotencyKey};
pub use dispatcher::{
    BroadcastResult, CoordinatedBroadcastResult, FanOutResult, MessageDispatcher, MulticastResult,
    QuorumResult, ScatterGatherResult, ShardResult,
};
pub use queue::{DeadLetterMessage, QueueManager, QueueStats, QueuedMessage};
pub use router::{MessageHandler, MessageRouter, RouterStats};
pub use rpc::{
    DataReadRequest, DataReadResponse, DataWriteRequest, DataWriteResponse, PingRequest,
    PingResponse, QueryRpcRequest, QueryRpcResponse, Request, RpcClient, RpcHandler, RpcServer,
};
pub use serialization::{BinaryCodec, ClusterMessage, RequestId};
pub use table::{DatacenterId, RouteVersion, RoutingTable, RoutingTableSnapshot, ShardId};
