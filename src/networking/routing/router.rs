//! Message router for cluster communication
//!
//! This module implements the message router that handles request/response correlation,
//! async request handling, timeout management, and priority routing.

use crate::error::{DbError, Result};
use crate::networking::routing::delivery::{DeliveryGuarantee, DeliveryTracker};
use crate::networking::routing::queue::{QueueManager, QueuedMessage};
use crate::networking::routing::serialization::{ClusterMessage, RequestId};
use crate::networking::routing::table::RoutingTable;
use crate::networking::types::{MessagePriority, NodeId};
use async_trait::async_trait;
use parking_lot::RwLock;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, SystemTime};
use tokio::sync::oneshot;

/// Message handler trait for processing incoming messages
#[async_trait]
pub trait MessageHandler: Send + Sync {
    /// Handle an incoming message
    async fn handle(&self, from: NodeId, message: ClusterMessage) -> Result<Option<ClusterMessage>>;

    /// Get the message types this handler can process
    fn message_types(&self) -> Vec<&'static str>;
}

/// Message type identifier
pub type MessageType = &'static str;

/// Pending request awaiting response
pub struct PendingRequest {
    /// When the request was sent
    sent_at: SystemTime,
    /// Timeout duration
    timeout: Duration,
    /// Channel to send response
    response_tx: oneshot::Sender<Result<ClusterMessage>>,
    /// Number of retry attempts
    attempts: u32,
    /// Maximum retries allowed
    max_retries: u32,
}

/// Message router for cluster communication
pub struct MessageRouter {
    /// Routing table for node addressing
    routing_table: RoutingTable,

    /// Message handlers by type
    handlers: Arc<RwLock<HashMap<MessageType, Arc<dyn MessageHandler>>>>,

    /// Pending requests awaiting responses
    pending_requests: Arc<RwLock<HashMap<RequestId, PendingRequest>>>,

    /// Queue manager for outbound messages
    queue_manager: QueueManager,

    /// Delivery tracker for reliability
    delivery_tracker: DeliveryTracker,

    /// Default timeout for requests
    default_timeout: Duration,

    /// Default maximum retries
    default_max_retries: u32,
}

impl MessageRouter {
    /// Create a new message router
    pub fn new(routing_table: RoutingTable) -> Self {
        Self {
            routing_table,
            handlers: Arc::new(RwLock::new(HashMap::new())),
            pending_requests: Arc::new(RwLock::new(HashMap::new())),
            queue_manager: QueueManager::new(),
            delivery_tracker: DeliveryTracker::new(DeliveryGuarantee::AtLeastOnce),
            default_timeout: Duration::from_secs(30),
            default_max_retries: 3,
        }
    }

    /// Create with custom configuration
    pub fn with_config(
        routing_table: RoutingTable,
        default_timeout: Duration,
        default_max_retries: u32,
        delivery_guarantee: DeliveryGuarantee,
    ) -> Self {
        Self {
            routing_table,
            handlers: Arc::new(RwLock::new(HashMap::new())),
            pending_requests: Arc::new(RwLock::new(HashMap::new())),
            queue_manager: QueueManager::new(),
            delivery_tracker: DeliveryTracker::new(delivery_guarantee),
            default_timeout,
            default_max_retries,
        }
    }

    /// Register a message handler
    pub fn register_handler<H: MessageHandler + 'static>(&self, handler: H) {
        let mut handlers = self.handlers.write();
        let handler_arc: Arc<dyn MessageHandler> = Arc::new(handler);

        for msg_type in handler_arc.message_types() {
            handlers.insert(msg_type, Arc::clone(&handler_arc));
        }
    }

    /// Send a message to a node (fire and forget)
    pub fn send_message(
        &self,
        destination: NodeId,
        message: ClusterMessage,
        priority: MessagePriority,
    ) -> Result<()> {
        // Generate request ID
        let request_id = RequestId::new();

        // Create queued message
        let queued_message = QueuedMessage::new(request_id.clone(), destination.clone(), message, priority);

        // Register with delivery tracker
        self.delivery_tracker.register_message(
            request_id,
            Some(DeliveryGuarantee::AtMostOnce),
            self.default_max_retries,
            self.default_timeout,
        )?;

        // Enqueue for sending
        self.queue_manager.enqueue(destination, queued_message)?;

        Ok(())
    }

    /// Send a request and wait for response
    pub async fn send_request(
        &self,
        destination: NodeId,
        message: ClusterMessage,
        priority: MessagePriority,
        timeout: Option<Duration>,
    ) -> Result<ClusterMessage> {
        let request_id = RequestId::new();
        let timeout = timeout.unwrap_or(self.default_timeout);

        // Create response channel
        let (response_tx, response_rx) = oneshot::channel();

        // Register pending request
        {
            let mut pending = self.pending_requests.write();
            pending.insert(
                request_id.clone(),
                PendingRequest {
                    sent_at: SystemTime::now(),
                    timeout,
                    response_tx,
                    attempts: 1,
                    max_retries: self.default_max_retries,
                },
            );
        }

        // Create queued message
        let queued_message = QueuedMessage::new(request_id.clone(), destination.clone(), message, priority)
            .with_max_attempts(self.default_max_retries);

        // Register with delivery tracker
        self.delivery_tracker.register_message(
            request_id.clone(),
            Some(DeliveryGuarantee::AtLeastOnce),
            self.default_max_retries,
            timeout,
        )?;

        // Enqueue for sending
        self.queue_manager.enqueue(destination, queued_message)?;

        // Wait for response with timeout
        match tokio::time::timeout(timeout, response_rx).await {
            Ok(Ok(response)) => response,
            Ok(Err(_)) => Err(DbError::Network("Response channel closed".to_string())),
            Err(_) => {
                // Timeout - clean up pending request
                self.pending_requests.write().remove(&request_id);
                Err(DbError::Network("Request timeout".to_string()))
            }
        }
    }

    /// Handle an incoming message
    pub async fn handle_incoming(
        &self,
        from: NodeId,
        request_id: RequestId,
        message: ClusterMessage,
    ) -> Result<()> {
        // Check if this is a response to a pending request
        if let Some(pending) = self.pending_requests.write().remove(&request_id) {
            // This is a response - send it through the channel
            let _ = pending.response_tx.send(Ok(message));
            self.delivery_tracker.mark_acknowledged(&request_id);
            return Ok(());
        }

        // Check for duplicate
        if self.delivery_tracker.is_duplicate(&request_id) {
            // Already processed, ignore
            return Ok(());
        }

        // Get handler for message type
        let message_type = message.message_type();
        let handler = {
            let handlers = self.handlers.read();
            handlers.get(message_type).cloned()
        };

        if let Some(handler) = handler {
            // Handle the message
            match handler.handle(from.clone(), message).await {
                Ok(Some(response)) => {
                    // Send response back
                    self.send_message(from, response, MessagePriority::Normal)?;
                }
                Ok(None) => {
                    // No response needed
                }
                Err(e) => {
                    // Send error response
                    let error_msg = ClusterMessage::Error(
                        crate::networking::routing::serialization::ErrorResponse::new(
                            request_id.clone(),
                            "HANDLER_ERROR",
                            format!("Message handler failed: {}", e),
                        ),
                    );
                    self.send_message(from, error_msg, MessagePriority::High)?;
                }
            }

            // Mark as acknowledged
            self.delivery_tracker.mark_acknowledged(&request_id);
        } else {
            // No handler registered
            let error_msg = ClusterMessage::Error(
                crate::networking::routing::serialization::ErrorResponse::new(
                    request_id,
                    "NO_HANDLER",
                    format!("No handler registered for message type: {}", message_type),
                ),
            );
            self.send_message(from, error_msg, MessagePriority::Normal)?;
        }

        Ok(())
    }

    /// Route a message to the appropriate destination based on routing strategy
    pub fn route_message(
        &self,
        message: ClusterMessage,
        priority: MessagePriority,
        destination: Option<NodeId>,
    ) -> Result<()> {
        if let Some(dest) = destination {
            // Direct routing
            self.send_message(dest, message, priority)
        } else {
            // Need to determine destination from routing table
            // For now, just return an error
            Err(DbError::Network(
                "No destination specified and auto-routing not implemented".to_string(),
            ))
        }
    }

    /// Get the next message to send for a destination
    pub fn get_next_message(&self, destination: &NodeId) -> Option<QueuedMessage> {
        self.queue_manager.dequeue(destination)
    }

    /// Check for timed-out requests and retry or fail them
    pub async fn check_timeouts(&self) {
        let now = SystemTime::now();
        let mut timed_out = Vec::new();

        {
            let pending = self.pending_requests.read();
            for (request_id, request) in pending.iter() {
                if let Ok(elapsed) = now.duration_since(request.sent_at) {
                    if elapsed >= request.timeout {
                        timed_out.push(request_id.clone());
                    }
                }
            }
        }

        // Handle timed-out requests
        for request_id in timed_out {
            if let Some(pending) = self.pending_requests.write().remove(&request_id) {
                let _ = pending.response_tx.send(Err(DbError::Network("Request timeout".to_string())));
            }
        }
    }

    /// Get statistics about the router
    pub fn get_stats(&self) -> RouterStats {
        let pending_count = self.pending_requests.read().len();
        let queue_stats = self.queue_manager.get_stats();
        let delivery_stats = self.delivery_tracker.get_stats();

        RouterStats {
            pending_requests: pending_count,
            queued_messages: queue_stats.total_queued,
            total_sent: queue_stats.total_dequeued,
            total_failed: queue_stats.total_failed,
            dlq_size: queue_stats.dlq_size,
            delivery_stats,
        }
    }

    /// Get the routing table
    pub fn routing_table(&self) -> &RoutingTable {
        &self.routing_table
    }

    /// Get the queue manager
    pub fn queue_manager(&self) -> &QueueManager {
        &self.queue_manager
    }

    /// Cleanup expired entries
    pub fn cleanup(&self) {
        self.delivery_tracker.cleanup();
    }
}

/// Statistics about the router
#[derive(Debug, Clone)]
pub struct RouterStats {
    pub pending_requests: usize,
    pub queued_messages: usize,
    pub total_sent: u64,
    pub total_failed: u64,
    pub dlq_size: usize,
    pub delivery_stats: crate::networking::routing::delivery::DeliveryStats,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::networking::routing::serialization::HeartbeatMessage;
    use crate::networking::types::NodeAddress;

    struct TestHandler;

    #[async_trait]
    impl MessageHandler for TestHandler {
        async fn handle(&self, _from: NodeId, _message: ClusterMessage) -> Result<Option<ClusterMessage>> {
            Ok(None)
        }

        fn message_types(&self) -> Vec<&'static str> {
            vec!["Heartbeat"]
        }
    }

    #[test]
    fn test_register_handler() {
        let table = RoutingTable::new();
        let router = MessageRouter::new(table);

        router.register_handler(TestHandler);

        let handlers = router.handlers.read();
        assert!(handlers.contains_key("Heartbeat"));
    }

    #[test]
    fn test_send_message() {
        let table = RoutingTable::new();
        let router = MessageRouter::new(table);

        let node_id = NodeId::new("node1");
        let message = ClusterMessage::Heartbeat(HeartbeatMessage {
            node_id: node_id.clone(),
            timestamp: 0,
            sequence: 0,
        });

        router
            .send_message(node_id, message, MessagePriority::Normal)
            .unwrap();

        let stats = router.get_stats();
        assert_eq!(stats.queued_messages, 1);
    }

    #[tokio::test]
    async fn test_request_timeout() {
        let table = RoutingTable::new();
        let router = MessageRouter::new(table);

        let node_id = NodeId::new("node1");
        let message = ClusterMessage::Heartbeat(HeartbeatMessage {
            node_id: node_id.clone(),
            timestamp: 0,
            sequence: 0,
        });

        let timeout = Duration::from_millis(100);
        let result = router
            .send_request(node_id, message, MessagePriority::Normal, Some(timeout))
            .await;

        assert!(result.is_err());
    }
}
