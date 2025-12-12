// Message queuing for cluster communication
//
// This module implements priority-based message queuing with backpressure
// and dead letter queue support for failed messages.

use crate::error::{DbError, Result};
use crate::networking::routing::serialization::{ClusterMessage, RequestId};
use crate::networking::types::{MessagePriority, NodeId};
use parking_lot::RwLock;
use std::collections::{BinaryHeap, HashMap, VecDeque};
use std::sync::Arc;
use std::time::{Duration, SystemTime};

/// Maximum size of a message queue
const DEFAULT_QUEUE_SIZE: usize = 10000;

/// Maximum size of the dead letter queue
const DEFAULT_DLQ_SIZE: usize = 1000;

/// Queued message with metadata
#[derive(Debug, Clone)]
pub struct QueuedMessage {
    /// Unique message ID
    pub message_id: RequestId,
    /// Destination node
    pub destination: NodeId,
    /// The actual message
    pub message: ClusterMessage,
    /// Message priority
    pub priority: MessagePriority,
    /// When the message was enqueued
    pub enqueued_at: SystemTime,
    /// Number of delivery attempts
    pub attempt_count: u32,
    /// Maximum delivery attempts
    pub max_attempts: u32,
}

impl QueuedMessage {
    /// Create a new queued message
    pub fn new(
        message_id: RequestId,
        destination: NodeId,
        message: ClusterMessage,
        priority: MessagePriority,
    ) -> Self {
        Self {
            message_id,
            destination,
            message,
            priority,
            enqueued_at: SystemTime::now(),
            attempt_count: 0,
            max_attempts: 3,
        }
    }

    /// Create with custom max attempts
    pub fn with_max_attempts(mut self, max_attempts: u32) -> Self {
        self.max_attempts = max_attempts;
        self
    }
}

/// Priority queue wrapper for ordering messages
#[derive(Debug, Clone)]
struct PriorityQueuedMessage(QueuedMessage);

impl PartialEq for PriorityQueuedMessage {
    fn eq(&self, other: &Self) -> bool {
        self.0.priority == other.0.priority
    }
}

impl Eq for PriorityQueuedMessage {}

impl PartialOrd for PriorityQueuedMessage {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for PriorityQueuedMessage {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        // Higher priority should come first (reverse order)
        self.0.priority.cmp(&other.0.priority)
    }
}

/// Message queue for a single peer node
pub struct MessageQueue {
    /// Priority queue for pending messages
    queue: BinaryHeap<PriorityQueuedMessage>,
    /// Maximum queue size
    max_size: usize,
    /// Whether the queue is accepting new messages
    accepting: bool,
}

impl MessageQueue {
    /// Create a new message queue
    pub fn new(max_size: usize) -> Self {
        Self {
            queue: BinaryHeap::new(),
            max_size,
            accepting: true,
        }
    }

    /// Enqueue a message
    pub fn enqueue(&mut self, message: QueuedMessage) -> Result<()> {
        if !self.accepting {
            return Err(DbError::Network("Queue not accepting messages".to_string()));
        }

        if self.queue.len() >= self.max_size {
            return Err(DbError::Network("Queue full".to_string()));
        }

        self.queue.push(PriorityQueuedMessage(message));
        Ok(())
    }

    /// Dequeue the highest priority message
    pub fn dequeue(&mut self) -> Option<QueuedMessage> {
        self.queue.pop().map(|pqm| pqm.0)
    }

    /// Peek at the highest priority message without removing it
    pub fn peek(&self) -> Option<&QueuedMessage> {
        self.queue.peek().map(|pqm| &pqm.0)
    }

    /// Get the number of messages in the queue
    pub fn len(&self) -> usize {
        self.queue.len()
    }

    /// Check if the queue is empty
    pub fn is_empty(&self) -> bool {
        self.queue.is_empty()
    }

    /// Check if the queue is full
    pub fn is_full(&self) -> bool {
        self.queue.len() >= self.max_size
    }

    /// Pause accepting new messages (backpressure)
    pub fn pause(&mut self) {
        self.accepting = false;
    }

    /// Resume accepting new messages
    pub fn resume(&mut self) {
        self.accepting = true;
    }

    /// Check if the queue is accepting messages
    pub fn is_accepting(&self) -> bool {
        self.accepting
    }

    /// Clear all messages from the queue
    pub fn clear(&mut self) {
        self.queue.clear();
    }

    /// Get the fill percentage (0.0 to 1.0)
    pub fn fill_percentage(&self) -> f64 {
        self.queue.len() as f64 / self.max_size as f64
    }
}

/// Dead letter queue for failed messages
pub struct DeadLetterQueue {
    /// Failed messages
    messages: VecDeque<DeadLetterMessage>,
    /// Maximum size
    max_size: usize,
}

/// Message that failed delivery
#[derive(Debug, Clone)]
pub struct DeadLetterMessage {
    /// Original message
    pub message: QueuedMessage,
    /// Failure reason
    pub failure_reason: String,
    /// When it was moved to DLQ
    pub failed_at: SystemTime,
}

impl DeadLetterQueue {
    /// Create a new dead letter queue
    pub fn new(max_size: usize) -> Self {
        Self {
            messages: VecDeque::new(),
            max_size,
        }
    }

    /// Add a failed message
    pub fn add(&mut self, message: QueuedMessage, failure_reason: String) {
        let dlq_message = DeadLetterMessage {
            message,
            failure_reason,
            failed_at: SystemTime::now(),
        };

        // Remove oldest if at capacity
        if self.messages.len() >= self.max_size {
            self.messages.pop_front();
        }

        self.messages.push_back(dlq_message);
    }

    /// Get all messages in the DLQ
    pub fn get_all(&self) -> Vec<DeadLetterMessage> {
        self.messages.iter().cloned().collect()
    }

    /// Get the number of messages in the DLQ
    pub fn len(&self) -> usize {
        self.messages.len()
    }

    /// Check if the DLQ is empty
    pub fn is_empty(&self) -> bool {
        self.messages.is_empty()
    }

    /// Clear the DLQ
    pub fn clear(&mut self) {
        self.messages.clear();
    }

    /// Remove old messages based on retention period
    pub fn cleanup(&mut self, retention: Duration) {
        let now = SystemTime::now();
        self.messages.retain(|msg| {
            now.duration_since(msg.failed_at)
                .map(|d| d < retention)
                .unwrap_or(false)
        });
    }
}

/// Queue manager for all peer nodes
pub struct QueueManager {
    /// Inner state
    inner: Arc<RwLock<QueueManagerInner>>,
    /// Default queue size
    default_queue_size: usize,
}

struct QueueManagerInner {
    /// Outbound queues per peer
    peer_queues: HashMap<NodeId, MessageQueue>,
    /// Dead letter queue
    dlq: DeadLetterQueue,
    /// Total messages enqueued
    total_enqueued: u64,
    /// Total messages dequeued
    total_dequeued: u64,
    /// Total messages failed
    total_failed: u64,
}

impl QueueManager {
    /// Create a new queue manager
    pub fn new() -> Self {
        Self::with_config(DEFAULT_QUEUE_SIZE, DEFAULT_DLQ_SIZE)
    }

    /// Create with custom configuration
    pub fn with_config(default_queue_size: usize, dlq_size: usize) -> Self {
        Self {
            inner: Arc::new(RwLock::new(QueueManagerInner {
                peer_queues: HashMap::new(),
                dlq: DeadLetterQueue::new(dlq_size),
                total_enqueued: 0,
                total_dequeued: 0,
                total_failed: 0,
            })),
            default_queue_size,
        }
    }

    /// Enqueue a message for a peer
    pub fn enqueue(&self, node_id: NodeId, message: QueuedMessage) -> Result<()> {
        let mut inner = self.inner.write();

        let queue = inner
            .peer_queues
            .entry(node_id)
            .or_insert_with(|| MessageQueue::new(self.default_queue_size));

        queue.enqueue(message)?;
        inner.total_enqueued += 1;

        Ok(())
    }

    /// Dequeue a message from a peer's queue
    pub fn dequeue(&self, node_id: &NodeId) -> Option<QueuedMessage> {
        let mut inner = self.inner.write();

        if let Some(queue) = inner.peer_queues.get_mut(node_id) {
            if let Some(message) = queue.dequeue() {
                inner.total_dequeued += 1;
                return Some(message);
            }
        }

        None
    }

    /// Get the size of a peer's queue
    pub fn queue_size(&self, node_id: &NodeId) -> usize {
        let inner = self.inner.read();
        inner
            .peer_queues
            .get(node_id)
            .map(|q| q.len())
            .unwrap_or(0)
    }

    /// Check if a peer's queue is full
    pub fn is_queue_full(&self, node_id: &NodeId) -> bool {
        let inner = self.inner.read();
        inner
            .peer_queues
            .get(node_id)
            .map(|q| q.is_full())
            .unwrap_or(false)
    }

    /// Pause a peer's queue (backpressure)
    pub fn pause_queue(&self, node_id: &NodeId) {
        let mut inner = self.inner.write();
        if let Some(queue) = inner.peer_queues.get_mut(node_id) {
            queue.pause();
        }
    }

    /// Resume a peer's queue
    pub fn resume_queue(&self, node_id: &NodeId) {
        let mut inner = self.inner.write();
        if let Some(queue) = inner.peer_queues.get_mut(node_id) {
            queue.resume();
        }
    }

    /// Move a message to the dead letter queue
    pub fn move_to_dlq(&self, message: QueuedMessage, failure_reason: String) {
        let mut inner = self.inner.write();
        inner.dlq.add(message, failure_reason);
        inner.total_failed += 1;
    }

    /// Get all messages in the dead letter queue
    pub fn get_dlq_messages(&self) -> Vec<DeadLetterMessage> {
        let inner = self.inner.read();
        inner.dlq.get_all()
    }

    /// Clear the dead letter queue
    pub fn clear_dlq(&self) {
        let mut inner = self.inner.write();
        inner.dlq.clear();
    }

    /// Remove a peer's queue
    pub fn remove_queue(&self, node_id: &NodeId) {
        let mut inner = self.inner.write();
        inner.peer_queues.remove(node_id);
    }

    /// Get statistics about all queues
    pub fn get_stats(&self) -> QueueStats {
        let inner = self.inner.read();

        let total_queued: usize = inner.peer_queues.values().map(|q| q.len()).sum();

        QueueStats {
            total_enqueued: inner.total_enqueued,
            total_dequeued: inner.total_dequeued,
            total_failed: inner.total_failed,
            total_queued,
            dlq_size: inner.dlq.len(),
            peer_count: inner.peer_queues.len(),
        }
    }

    /// Get fill percentages for all queues
    pub fn get_fill_percentages(&self) -> HashMap<NodeId, f64> {
        let inner = self.inner.read();
        inner
            .peer_queues
            .iter()
            .map(|(node_id, queue)| (node_id.clone(), queue.fill_percentage()))
            .collect()
    }
}

impl Default for QueueManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Statistics about message queues
#[derive(Debug, Clone)]
pub struct QueueStats {
    pub total_enqueued: u64,
    pub total_dequeued: u64,
    pub total_failed: u64,
    pub total_queued: usize,
    pub dlq_size: usize,
    pub peer_count: usize,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::networking::routing::serialization::HeartbeatMessage;

    #[test]
    fn test_message_queue_priority() {
        let mut queue = MessageQueue::new(100);

        let low = QueuedMessage::new(
            RequestId::new(),
            NodeId::new("node1"),
            ClusterMessage::Heartbeat(HeartbeatMessage {
                node_id: NodeId::new("node1"),
                timestamp: 0,
                sequence: 0,
            }),
            MessagePriority::Low,
        );

        let high = QueuedMessage::new(
            RequestId::new(),
            NodeId::new("node1"),
            ClusterMessage::Heartbeat(HeartbeatMessage {
                node_id: NodeId::new("node1"),
                timestamp: 0,
                sequence: 0,
            }),
            MessagePriority::High,
        );

        queue.enqueue(low).unwrap();
        queue.enqueue(high).unwrap();

        // High priority should come first
        let msg = queue.dequeue().unwrap();
        assert_eq!(msg.priority, MessagePriority::High);
    }

    #[test]
    fn test_queue_backpressure() {
        let mut queue = MessageQueue::new(10);

        queue.pause();
        assert!(!queue.is_accepting());

        let msg = QueuedMessage::new(
            RequestId::new(),
            NodeId::new("node1"),
            ClusterMessage::Heartbeat(HeartbeatMessage {
                node_id: NodeId::new("node1"),
                timestamp: 0,
                sequence: 0,
            }),
            MessagePriority::Normal,
        );

        assert!(queue.enqueue(msg).is_err());

        queue.resume();
        assert!(queue.is_accepting());
    }

    #[test]
    fn test_queue_manager() {
        let manager = QueueManager::new();
        let node_id = NodeId::new("node1");

        let msg = QueuedMessage::new(
            RequestId::new(),
            node_id.clone(),
            ClusterMessage::Heartbeat(HeartbeatMessage {
                node_id: node_id.clone(),
                timestamp: 0,
                sequence: 0,
            }),
            MessagePriority::Normal,
        );

        manager.enqueue(node_id.clone(), msg).unwrap();
        assert_eq!(manager.queue_size(&node_id), 1);

        let dequeued = manager.dequeue(&node_id).unwrap();
        assert_eq!(manager.queue_size(&node_id), 0);
    }

    #[test]
    fn test_dead_letter_queue() {
        let mut dlq = DeadLetterQueue::new(10);

        let msg = QueuedMessage::new(
            RequestId::new(),
            NodeId::new("node1"),
            ClusterMessage::Heartbeat(HeartbeatMessage {
                node_id: NodeId::new("node1"),
                timestamp: 0,
                sequence: 0,
            }),
            MessagePriority::Normal,
        );

        dlq.add(msg, "Test failure".to_string());
        assert_eq!(dlq.len(), 1);

        let messages = dlq.get_all();
        assert_eq!(messages[0].failure_reason, "Test failure");
    }
}
