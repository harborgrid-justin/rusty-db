// # Event Subscriber
//
// Event subscription with consumer groups, offset tracking, at-least-once
// and exactly-once delivery semantics, subscription filtering, and replay.

use tokio::time::sleep;
use std::collections::HashSet;
use std::collections::VecDeque;
use std::sync::Mutex;
use std::time::Duration;
use std::collections::{HashMap};
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, AtomicBool, Ordering};
use std::time::{Instant, SystemTime};
use parking_lot::{RwLock};
use serde::{Deserialize, Serialize};
use tokio::sync::{mpsc, broadcast};
use tokio::time::interval;
use crate::error::{DbError, Result};
use super::publisher::PublishedEvent;

/// Delivery semantics
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DeliverySemantics {
    /// At-most-once (fire and forget)
    AtMostOnce,
    /// At-least-once (default, may deliver duplicates)
    AtLeastOnce,
    /// Exactly-once (requires deduplication)
    ExactlyOnce,
}

/// Offset commit strategy
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum OffsetCommitStrategy {
    /// Manual commit by consumer
    Manual,
    /// Auto-commit after processing
    AutoCommitAfterProcess,
    /// Auto-commit on interval
    AutoCommitPeriodic,
}

/// Subscription filter
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubscriptionFilter {
    /// Filter by event headers
    pub header_filters: HashMap<String, String>,
    /// Filter by key prefix
    pub key_prefix: Option<Vec<u8>>,
    /// Custom filter expression (simplified)
    pub expression: Option<String>,
}

impl SubscriptionFilter {
    pub fn new() -> Self {
        Self {
            header_filters: HashMap::new(),
            key_prefix: None,
            expression: None,
        }
    }

    pub fn with_header(mut self, name: String, value: String) -> Self {
        self.header_filters.insert(name, value);
        self
    }

    pub fn matches(&self, event: &PublishedEvent) -> bool {
        // Check header filters
        for (key, expected_value) in &self.header_filters {
            if let Some(actual_value) = event.headers.get(key) {
                if actual_value != expected_value {
                    return false;
                }
            } else {
                return false;
            }
        }

        // Check key prefix
        if let Some(prefix) = &self.key_prefix {
            if let Some(key) = &event.key {
                if !key.starts_with(prefix) {
                    return false;
                }
            } else {
                return false;
            }
        }

        true
    }
}

/// Partition offset
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct PartitionOffset {
    pub partition: u32,
    pub offset: u64,
}

impl PartitionOffset {
    pub fn new(partition: u32, offset: u64) -> Self {
        Self { partition, offset }
    }
}

/// Consumer position in a topic
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsumerPosition {
    pub topic: String,
    pub partition_offsets: HashMap<u32, u64>,
    pub last_updated: SystemTime,
}

impl ConsumerPosition {
    pub fn new(topic: String) -> Self {
        Self {
            topic,
            partition_offsets: HashMap::new(),
            last_updated: SystemTime::now(),
        }
    }

    pub fn update_offset(&mut self, partition: u32, offset: u64) {
        self.partition_offsets.insert(partition, offset);
        self.last_updated = SystemTime::now();
    }

    pub fn get_offset(&self, partition: u32) -> u64 {
        self.partition_offsets.get(&partition).copied().unwrap_or(0)
    }
}

/// Consumer group member
#[derive(Debug, Clone)]
struct ConsumerMember {
    consumer_id: String,
    assigned_partitions: Vec<u32>,
    last_heartbeat: SystemTime,
    generation_id: u64,
}

/// Consumer group
#[derive(Debug)]
struct ConsumerGroup {
    group_id: String,
    topic: String,
    members: HashMap<String, ConsumerMember>,
    partition_assignment: HashMap<u32, String>, // partition -> consumer_id
    generation_id: AtomicU64,
    coordinator_epoch: AtomicU64,
}

impl ConsumerGroup {
    fn new(group_id: String, topic: String) -> Self {
        Self {
            group_id,
            topic,
            members: HashMap::new(),
            partition_assignment: HashMap::new(),
            generation_id: AtomicU64::new(1),
            coordinator_epoch: AtomicU64::new(0),
        }
    }

    fn add_member(&mut self, consumer_id: String) -> u64 {
        let generation = self.generation_id.load(Ordering::SeqCst);
        let member = ConsumerMember {
            consumer_id: consumer_id.clone(),
            assigned_partitions: Vec::new(),
            last_heartbeat: SystemTime::now(),
            generation_id: generation,
        };
        self.members.insert(consumer_id, member);
        generation
    }

    fn remove_member(&mut self, consumer_id: &str) -> bool {
        self.members.remove(consumer_id).is_some()
    }

    fn rebalance(&mut self, num_partitions: u32) {
        // Simple round-robin rebalancing
        self.partition_assignment.clear();

        let member_ids: Vec<String> = self.members.keys().cloned().collect();
        if member_ids.is_empty() {
            return;
        }

        for partition in 0..num_partitions {
            let idx = partition as usize % member_ids.len();
            self.partition_assignment.insert(partition, member_ids[idx].clone());
        }

        // Update member assignments
        for (partition, consumer_id) in &self.partition_assignment {
            if let Some(member) = self.members.get_mut(consumer_id) {
                if !member.assigned_partitions.contains(partition) {
                    member.assigned_partitions.push(*partition);
                }
            }
        }

        self.generation_id.fetch_add(1, Ordering::SeqCst);
    }

    fn get_assigned_partitions(&self, consumer_id: &str) -> Vec<u32> {
        self.members
            .get(consumer_id)
            .map(|m| m.assigned_partitions.clone())
            .unwrap_or_default()
    }
}

/// Subscription configuration
#[derive(Debug, Clone)]
pub struct SubscriptionConfig {
    /// Consumer ID
    pub consumer_id: String,
    /// Consumer group ID
    pub group_id: Option<String>,
    /// Topics to subscribe to
    pub topics: Vec<String>,
    /// Delivery semantics
    pub delivery: DeliverySemantics,
    /// Offset commit strategy
    pub commit_strategy: OffsetCommitStrategy,
    /// Auto-commit interval
    pub auto_commit_interval: Duration,
    /// Session timeout
    pub session_timeout: Duration,
    /// Max poll records
    pub max_poll_records: usize,
    /// Enable auto offset store
    pub enable_auto_offset_store: bool,
    /// Subscription filter
    pub filter: Option<SubscriptionFilter>,
}

impl Default for SubscriptionConfig {
    fn default() -> Self {
        Self {
            consumer_id: format!("consumer-{}", uuid::Uuid::new_v4()),
            group_id: None,
            topics: Vec::new(),
            delivery: DeliverySemantics::AtLeastOnce,
            commit_strategy: OffsetCommitStrategy::AutoCommitAfterProcess,
            auto_commit_interval: Duration::from_secs(5),
            session_timeout: Duration::from_secs(30),
            max_poll_records: 500,
            enable_auto_offset_store: true,
            filter: None,
        }
    }
}

/// Consumed event with metadata
#[derive(Debug, Clone)]
pub struct ConsumedEvent {
    /// Original published event
    pub event: PublishedEvent,
    /// Consumer that received this event
    pub consumer_id: String,
    /// Consumption timestamp
    pub consumed_at: SystemTime,
    /// Delivery attempt count
    pub attempt: u32,
}

/// Subscription statistics
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SubscriptionStats {
    /// Total events consumed
    pub total_consumed: u64,
    /// Events consumed per second
    pub events_per_second: f64,
    /// Total bytes consumed
    pub total_bytes: u64,
    /// Average processing time (ms)
    pub avg_processing_time_ms: f64,
    /// Number of redeliveries
    pub redeliveries: u64,
    /// Current lag (events behind)
    pub current_lag: u64,
    /// Number of filtered events
    pub filtered_events: u64,
    /// Dead letter queue size
    pub dlq_size: u64,
}

/// Event Subscriber
pub struct EventSubscriber {
    /// Configuration
    config: SubscriptionConfig,
    /// Consumer positions per topic
    positions: Arc<RwLock<HashMap<String, ConsumerPosition>>>,
    /// Pending commits
    pending_commits: Arc<Mutex<VecDeque<PartitionOffset>>>,
    /// Consumer groups
    groups: Arc<RwLock<HashMap<String, Arc<Mutex<ConsumerGroup>>>>>,
    /// Event buffer
    event_buffer: Arc<Mutex<VecDeque<ConsumedEvent>>>,
    /// Deduplication cache (for exactly-once)
    dedup_cache: Arc<RwLock<HashSet<u64>>>,
    /// Dead letter queue
    dlq: Arc<Mutex<VecDeque<ConsumedEvent>>>,
    /// Statistics
    stats: Arc<RwLock<SubscriptionStats>>,
    /// Event receiver
    event_rx: Arc<Mutex<Option<mpsc::Receiver<PublishedEvent>>>>,
    /// Shutdown flag
    shutdown: Arc<AtomicBool>,
}

impl EventSubscriber {
    /// Create a new event subscriber
    pub fn new(config: SubscriptionConfig) -> Self {
        Self {
            config,
            positions: Arc::new(RwLock::new(HashMap::new())),
            pending_commits: Arc::new(Mutex::new(VecDeque::new())),
            groups: Arc::new(RwLock::new(HashMap::new())),
            event_buffer: Arc::new(Mutex::new(VecDeque::new())),
            dedup_cache: Arc::new(RwLock::new(HashSet::new())),
            dlq: Arc::new(Mutex::new(VecDeque::new())),
            stats: Arc::new(RwLock::new(SubscriptionStats::default())),
            event_rx: Arc::new(Mutex::new(None)),
            shutdown: Arc::new(AtomicBool::new(false)),
        }
    }

    /// Subscribe to topics
    pub async fn subscribe(&self) -> Result<()> {
        // Join consumer group if specified
        if let Some(group_id) = &self.config.group_id {
            for topic in &self.config.topics {
                self.join_consumer_group(group_id, topic).await?;
            }
        }

        // Initialize positions for all topics
        for topic in &self.config.topics {
            self.positions.write()
                .entry(topic.clone())
                .or_insert_with(|| ConsumerPosition::new(topic.clone()));
        }

        // Start background tasks
        self.spawn_commit_task();
        self.spawn_heartbeat_task();

        Ok(())
    }

    /// Unsubscribe from all topics
    pub async fn unsubscribe(&self) -> Result<()> {
        // Leave consumer groups
        if let Some(group_id) = &self.config.group_id {
            for topic in &self.config.topics {
                self.leave_consumer_group(group_id, topic).await?;
            }
        }

        // Commit pending offsets
        self.commit_sync().await?;

        self.shutdown.store(true, Ordering::SeqCst);
        Ok(())
    }

    /// Poll for events
    pub async fn poll(&self, timeout: Duration) -> Result<Vec<ConsumedEvent>> {
        let start = Instant::now();
        let mut consumed = Vec::new();

        while start.elapsed() < timeout && consumed.len() < self.config.max_poll_records {
            // Try to get event from buffer
            if let Some(event) = self.event_buffer.lock().unwrap().pop_front() {
                // Apply filter if configured
                if let Some(filter) = &self.config.filter {
                    if !filter.matches(&event.event) {
                        self.stats.write().filtered_events += 1;
                        continue;
                    }
                }

                // Check deduplication for exactly-once
                if self.config.delivery == DeliverySemantics::ExactlyOnce {
                    if !self.dedup_cache.write().insert(event.event.id) {
                        // Duplicate event, skip
                        continue;
                    }
                }

                consumed.push(event);
            } else {
                // Wait a bit for more events
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
        }

        // Update statistics
        {
            let mut stats = self.stats.write();
            stats.total_consumed += consumed.len() as u64;
            stats.total_bytes += consumed.iter()
                .map(|e| e.event.size_bytes() as u64)
                .sum::<u64>();
        }

        // Auto-commit if enabled
        if self.config.commit_strategy == OffsetCommitStrategy::AutoCommitAfterProcess {
            for event in &consumed {
                self.store_offset(&event.event.topic, event.event.partition, event.event.offset).await?;
            }
        }

        Ok(consumed)
    }

    /// Consume a single event
    pub async fn consume_one(&self, timeout: Duration) -> Result<Option<ConsumedEvent>> {
        let events = self.poll(timeout).await?;
        Ok(events.into_iter().next())
    }

    /// Add event to buffer (called by publisher)
    pub fn add_event(&self, event: PublishedEvent) {
        let consumed = ConsumedEvent {
            event,
            consumer_id: self.config.consumer_id.clone(),
            consumed_at: SystemTime::now(),
            attempt: 1,
        };

        self.event_buffer.lock().unwrap().push_back(consumed);
    }

    /// Commit offsets synchronously
    pub async fn commit_sync(&self) -> Result<()> {
        let pending = self.pending_commits.lock().unwrap().drain(..).collect::<Vec<_>>();

        for offset in pending {
            // Update position
            if let Some(topic) = self.config.topics.first() {
                self.positions.write()
                    .entry(topic.clone())
                    .or_insert_with(|| ConsumerPosition::new(topic.clone()))
                    .update_offset(offset.partition, offset.offset);
            }
        }

        Ok(())
    }

    /// Commit offsets asynchronously
    pub async fn commit_async(&self) -> Result<()> {
        // In a real implementation, this would queue commits for background processing
        self.commit_sync().await
    }

    /// Store offset for later commit
    pub async fn store_offset(&self, topic: &str, partition: u32, offset: u64) -> Result<()> {
        if self.config.enable_auto_offset_store {
            self.pending_commits.lock().unwrap().push_back(PartitionOffset::new(partition, offset));
        }
        Ok(())
    }

    /// Seek to a specific offset
    pub async fn seek(&self, topic: &str, partition: u32, offset: u64) -> Result<()> {
        self.positions.write()
            .entry(topic.to_string())
            .or_insert_with(|| ConsumerPosition::new(topic.to_string()))
            .update_offset(partition, offset);
        Ok(())
    }

    /// Seek to beginning of topic
    pub async fn seek_to_beginning(&self, topic: &str, partition: u32) -> Result<()> {
        self.seek(topic, partition, 0).await
    }

    /// Seek to end of topic
    pub async fn seek_to_end(&self, topic: &str, partition: u32, end_offset: u64) -> Result<()> {
        self.seek(topic, partition, end_offset).await
    }

    /// Get current position
    pub fn get_position(&self, topic: &str) -> Option<ConsumerPosition> {
        self.positions.read().get(topic).cloned()
    }

    /// Get assigned partitions (for consumer groups)
    pub fn get_assigned_partitions(&self, topic: &str) -> Vec<u32> {
        if let Some(group_id) = &self.config.group_id {
            let groups = self.groups.read();
            if let Some(group) = groups.get(group_id) {
                return group.lock().unwrap().get_assigned_partitions(&self.config.consumer_id);
            }
        }
        Vec::new()
    }

    /// Pause consumption from partitions
    pub fn pause(&self, partitions: Vec<u32>) {
        // Implementation would mark partitions as paused
    }

    /// Resume consumption from partitions
    pub fn resume(&self, partitions: Vec<u32>) {
        // Implementation would mark partitions as resumed
    }

    /// Get dead letter queue events
    pub fn get_dlq_events(&self) -> Vec<ConsumedEvent> {
        self.dlq.lock().unwrap().iter().cloned().collect()
    }

    /// Send event to dead letter queue
    pub fn send_to_dlq(&self, event: ConsumedEvent) {
        self.dlq.lock().unwrap().push_back(event);
        self.stats.write().dlq_size += 1;
    }

    /// Get statistics
    pub fn get_statistics(&self) -> SubscriptionStats {
        self.stats.read().clone()
    }

    // Consumer group management

    async fn join_consumer_group(&self, group_id: &str, topic: &str) -> Result<()> {
        let mut groups = self.groups.write();
        let group = groups
            .entry(group_id.to_string())
            .or_insert_with(|| Arc::new(Mutex::new(ConsumerGroup::new(
                group_id.to_string(),
                topic.to_string(),
            ))));

        let mut group = group.lock().unwrap();
        group.add_member(self.config.consumer_id.clone());
        group.rebalance(10); // TODO: Get actual partition count
        Ok(())
    }

    async fn leave_consumer_group(&self, group_id: &str, _topic: &str) -> Result<()> {
        let groups = self.groups.read();
        if let Some(group) = groups.get(group_id) {
            let mut group = group.lock().unwrap();
            group.remove_member(&self.config.consumer_id);
            group.rebalance(10);
        }
        Ok(())
    }

    fn spawn_commit_task(&self) {
        if self.config.commit_strategy != OffsetCommitStrategy::AutoCommitPeriodic {
            return;
        }

        let interval_duration = self.config.auto_commit_interval;
        let pending_commits = self.pending_commits.clone();
        let positions = self.positions.clone();
        let topics = self.config.topics.clone();
        let shutdown = self.shutdown.clone();

        tokio::spawn(async move {
            let mut interval = interval(interval_duration);

            while !shutdown.load(Ordering::SeqCst) {
                interval.tick().await;

                // Commit pending offsets
                let pending = pending_commits.lock().unwrap().drain(..).collect::<Vec<_>>();
                for offset in pending {
                    if let Some(topic) = topics.first() {
                        positions.write()
                            .entry(topic.clone())
                            .or_insert_with(|| ConsumerPosition::new(topic.clone()))
                            .update_offset(offset.partition, offset.offset);
                    }
                }
            }
        });
    }

    fn spawn_heartbeat_task(&self) {
        if self.config.group_id.is_none() {
            return;
        }

        let consumer_id = self.config.consumer_id.clone();
        let groups = self.groups.clone();
        let group_id = self.config.group_id.clone().unwrap();
        let shutdown = self.shutdown.clone();

        tokio::spawn(async move {
            let mut interval = interval(Duration::from_secs(10));

            while !shutdown.load(Ordering::SeqCst) {
                interval.tick().await;

                // Send heartbeat
                let groups = groups.read();
                if let Some(group) = groups.get(&group_id) {
                    let mut group = group.lock().unwrap();
                    if let Some(member) = group.members.get_mut(&consumer_id) {
                        member.last_heartbeat = SystemTime::now();
                    }
                }
            }
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_subscription_filter() {
        let filter = SubscriptionFilter::new()
            .with_header("type".to_string(), "insert".to_string());

        let mut event = PublishedEvent::new("test".to_string(), vec![]);
        assert!(!filter.matches(&event));

        event.headers.insert("type".to_string(), "insert".to_string());
        assert!(filter.matches(&event));
    }

    #[test]
    fn test_consumer_position() {
        let mut position = ConsumerPosition::new("test".to_string());
        assert_eq!(position.get_offset(0), 0);

        position.update_offset(0, 100);
        assert_eq!(position.get_offset(0), 100);
    }

    #[test]
    fn test_consumer_group_rebalance() {
        let mut group = ConsumerGroup::new("test-group".to_string(), "test-topic".to_string());

        group.add_member("consumer-1".to_string());
        group.add_member("consumer-2".to_string());
        group.rebalance(4);

        let p1 = group.get_assigned_partitions("consumer-1");
        let p2 = group.get_assigned_partitions("consumer-2");

        assert!(!p1.is_empty());
        assert!(!p2.is_empty());
        assert_eq!(p1.len() + p2.len(), 4);
    }

    #[tokio::test]
    async fn test_subscriber_lifecycle() {
        let mut config = SubscriptionConfig::default();
        config.topics = vec!["test".to_string()];

        let subscriber = EventSubscriber::new(config);
        subscriber.subscribe().await.unwrap();

        let position = subscriber.get_position("test");
        assert!(position.is_some());

        subscriber.unsubscribe().await.unwrap();
    }

    #[tokio::test]
    async fn test_offset_management() {
        let mut config = SubscriptionConfig::default();
        config.topics = vec!["test".to_string()];
        config.enable_auto_offset_store = true;

        let subscriber = EventSubscriber::new(config);
        subscriber.subscribe().await.unwrap();

        subscriber.store_offset("test", 0, 100).await.unwrap();
        subscriber.commit_sync().await.unwrap();

        let position = subscriber.get_position("test").unwrap();
        assert_eq!(position.get_offset(0), 100);
    }
}
