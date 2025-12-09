// # Event Publisher
//
// Kafka-like event publishing with topics, partitions, ordering guarantees,
// acknowledgments, and backpressure management.

use tokio::time::sleep;
use std::collections::VecDeque;
use std::sync::Mutex;
use std::time::Duration;
use std::collections::{HashMap};
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, AtomicBool, Ordering};
use std::time::{Instant, SystemTime};
use std::hash::{Hash, Hasher};
use parking_lot::{RwLock};
use serde::{Deserialize, Serialize};
use tokio::sync::{mpsc, Semaphore};
use tokio::time::interval;
use crate::{Result, DbError};

/// Event serialization format
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SerializationFormat {
    Json,
    Avro,
    Protobuf,
    MessagePack,
    Binary,
}

/// Published event with metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PublishedEvent {
    /// Event ID
    pub id: u64,
    /// Topic name
    pub topic: String,
    /// Partition ID
    pub partition: u32,
    /// Offset within partition
    pub offset: u64,
    /// Event key for partitioning and ordering
    pub key: Option<Vec<u8>>,
    /// Event payload
    pub payload: Vec<u8>,
    /// Event headers
    pub headers: HashMap<String, String>,
    /// Timestamp
    pub timestamp: SystemTime,
    /// Serialization format
    pub format: SerializationFormat,
}

impl PublishedEvent {
    pub fn new(topic: String, payload: Vec<u8>) -> Self {
        Self {
            id: 0,
            topic,
            partition: 0,
            offset: 0,
            key: None,
            payload,
            headers: HashMap::new(),
            timestamp: SystemTime::now(),
            format: SerializationFormat::Json,
        }
    }

    pub fn with_key(mut self, key: Vec<u8>) -> Self {
        self.key = Some(key);
        self
    }

    pub fn with_header(mut self, name: String, value: String) -> Self {
        self.headers.insert(name, value);
        self
    }

    pub fn with_format(mut self, format: SerializationFormat) -> Self {
        self.format = format;
        self
    }

    pub fn size_bytes(&self) -> usize {
        self.payload.len() + self.key.as_ref().map(|k| k.len()).unwrap_or(0)
    }
}

/// Topic configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TopicConfig {
    /// Topic name
    pub name: String,
    /// Number of partitions
    pub num_partitions: u32,
    /// Replication factor
    pub replication_factor: u32,
    /// Retention period
    pub retention_period: Duration,
    /// Maximum message size
    pub max_message_size: usize,
    /// Compression enabled
    pub compression_enabled: bool,
    /// Ordering guarantee
    pub ordering: OrderingGuarantee,
}

impl TopicConfig {
    pub fn new(name: String, num_partitions: u32) -> Self {
        Self {
            name,
            num_partitions,
            replication_factor: 1,
            retention_period: Duration::from_secs(7 * 24 * 3600), // 7 days
            max_message_size: 1024 * 1024, // 1 MB
            compression_enabled: true,
            ordering: OrderingGuarantee::PartitionOrdered,
        }
    }
}

/// Ordering guarantee level
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum OrderingGuarantee {
    /// No ordering guarantees
    Unordered,
    /// Events with same key are ordered
    KeyOrdered,
    /// Events within same partition are ordered
    PartitionOrdered,
    /// All events are globally ordered
    GloballyOrdered,
}

/// Acknowledgment mode
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AckMode {
    /// No acknowledgment
    NoAck,
    /// Acknowledge after writing to leader
    Leader,
    /// Acknowledge after replication to all replicas
    AllReplicas,
}

/// Publisher acknowledgment
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PublishAck {
    /// Event ID
    pub event_id: u64,
    /// Topic
    pub topic: String,
    /// Partition
    pub partition: u32,
    /// Offset
    pub offset: u64,
    /// Acknowledgment timestamp
    pub timestamp: SystemTime,
}

/// Partition state
#[derive(Debug)]
struct PartitionState {
    partition_id: u32,
    next_offset: AtomicU64,
    event_queue: Mutex<VecDeque<PublishedEvent>>,
    high_watermark: AtomicU64,
}

impl PartitionState {
    fn new(partition_id: u32) -> Self {
        Self {
            partition_id,
            next_offset: AtomicU64::new(0),
            event_queue: Mutex::new(VecDeque::new()),
            high_watermark: AtomicU64::new(0),
        }
    }

    fn allocate_offset(&self) -> u64 {
        self.next_offset.fetch_add(1, Ordering::SeqCst)
    }

    fn enqueue(&self, event: PublishedEvent) {
        self.event_queue.lock().push_back(event);
    }

    fn dequeue_batch(&self, max_size: usize) -> Vec<PublishedEvent> {
        let mut queue = self.event_queue.lock();
        let mut batch = Vec::new();

        for _ in 0..max_size {
            if let Some(event) = queue.pop_front() {
                batch.push(event);
            } else {
                break;
            }
        }

        batch
    }
}

/// Topic instance
struct Topic {
    config: TopicConfig,
    partitions: Vec<Arc<PartitionState>>,
    created_at: SystemTime,
    total_events: AtomicU64,
    total_bytes: AtomicU64,
}

impl Topic {
    fn new(config: TopicConfig) -> Self {
        let partitions = (0..config.num_partitions)
            .map(|i| Arc::new(PartitionState::new(i)))
            .collect();

        Self {
            config,
            partitions,
            created_at: SystemTime::now(),
            total_events: AtomicU64::new(0),
            total_bytes: AtomicU64::new(0),
        }
    }

    fn get_partition(&self, partition_id: u32) -> Option<&Arc<PartitionState>> {
        self.partitions.get(partition_id as usize)
    }

    fn select_partition(&self, key: Option<&[u8]>) -> u32 {
        match (key, self.config.ordering) {
            (Some(k), OrderingGuarantee::KeyOrdered | OrderingGuarantee::PartitionOrdered) => {
                // Hash-based partitioning for key ordering
                let mut hasher = std::collections::hash_map::DefaultHasher::new();
                k.hash(&mut hasher);
                let hash = hasher.finish();
                (hash % self.config.num_partitions as u64) as u32
            }
            (_, OrderingGuarantee::GloballyOrdered) => {
                // Always use partition 0 for global ordering
                0
            }
            _ => {
                // Round-robin or random for unordered
                (self.total_events.load(Ordering::Relaxed) % self.config.num_partitions as u64) as u32
            }
        }
    }
}

/// Publisher statistics
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PublisherStats {
    /// Total events published
    pub total_events: u64,
    /// Total bytes published
    pub total_bytes: u64,
    /// Events published per second
    pub events_per_second: f64,
    /// Average event size
    pub avg_event_size: f64,
    /// Average publish latency (ms)
    pub avg_publish_latency_ms: f64,
    /// P95 publish latency (ms)
    pub p95_publish_latency_ms: f64,
    /// P99 publish latency (ms)
    pub p99_publish_latency_ms: f64,
    /// Number of failed publishes
    pub failed_publishes: u64,
    /// Backpressure activations
    pub backpressure_events: u64,
    /// Current backpressure state
    pub is_backpressured: bool,
}

/// Publisher configuration
#[derive(Debug, Clone)]
pub struct PublisherConfig {
    /// Maximum inflight events per partition
    pub max_inflight: usize,
    /// Batch size for event processing
    pub batch_size: usize,
    /// Batch timeout
    pub batch_timeout: Duration,
    /// Acknowledgment mode
    pub ack_mode: AckMode,
    /// Enable compression
    pub enable_compression: bool,
    /// Compression threshold (bytes)
    pub compression_threshold: usize,
    /// Default serialization format
    pub default_format: SerializationFormat,
    /// Enable backpressure
    pub enable_backpressure: bool,
    /// Backpressure threshold
    pub backpressure_threshold: usize,
    /// Publish timeout
    pub publish_timeout: Duration,
}

impl Default for PublisherConfig {
    fn default() -> Self {
        Self {
            max_inflight: 10000,
            batch_size: 1000,
            batch_timeout: Duration::from_millis(10),
            ack_mode: AckMode::Leader,
            enable_compression: true,
            compression_threshold: 1024,
            default_format: SerializationFormat::Json,
            enable_backpressure: true,
            backpressure_threshold: 100000,
            publish_timeout: Duration::from_secs(30),
        }
    }
}

/// Event Publisher
pub struct EventPublisher {
    /// Configuration
    config: PublisherConfig,
    /// Topics
    topics: Arc<RwLock<HashMap<String, Arc<Topic>>>>,
    /// Next event ID
    next_event_id: Arc<AtomicU64>,
    /// Statistics
    stats: Arc<RwLock<PublisherStats>>,
    /// Backpressure semaphore
    backpressure_sem: Arc<Semaphore>,
    /// Pending acknowledgments
    pending_acks: Arc<RwLock<HashMap<u64::Sender<Result<PublishAck>>>>>,
    /// Shutdown flag
    shutdown: Arc<AtomicBool>,
}

impl EventPublisher {
    /// Create a new event publisher
    pub fn new(config: PublisherConfig) -> Self {
        let max_permits = if config.enable_backpressure {
            config.backpressure_threshold
        } else {
            usize::MAX
        };

        Self {
            config,
            topics: Arc::new(RwLock::new(HashMap::new())),
            next_event_id: Arc::new(AtomicU64::new(1)),
            stats: Arc::new(RwLock::new(PublisherStats::default())),
            backpressure_sem: Arc::new(Semaphore::new(max_permits)),
            pending_acks: Arc::new(RwLock::new(HashMap::new())),
            shutdown: Arc::new(AtomicBool::new(false)),
        }
    }

    /// Create a new topic
    pub async fn create_topic(&self, config: TopicConfig) -> Result<()> {
        let topic_name = config.name.clone();
        let topic = Arc::new(Topic::new(config));

        let mut topics = self.topics.write();
        if topics.contains_key(&topic_name) {
            return Err(DbError::InvalidOperation(
                format!("Topic '{}' already exists", topic_name)
            ))));
        }

        topics.insert(topic_name, topic);
        Ok(())
    }

    /// Delete a topic
    pub async fn delete_topic(&self, topic_name: &str) -> Result<()> {
        let mut topics = self.topics.write();
        topics.remove(topic_name)
            .ok_or_else(|| DbError::NotFound(format!("Topic '{}' not found", topic_name)))?);
        Ok(())
    }

    /// List all topics
    pub fn list_topics(&self) -> Vec<String> {
        self.topics.read().keys().cloned().collect()
    }

    /// Get topic configuration
    pub fn get_topic_config(&self, topic_name: &str) -> Result<TopicConfig> {
        let topics = self.topics.read();
        let topic = topics.get(topic_name)
            .ok_or_else(|| DbError::NotFound(format!("Topic '{}' not found", topic_name)))?);
        Ok(topic.config.clone())
    }

    /// Publish an event
    pub async fn publish(&self, mut event: PublishedEvent) -> Result<PublishAck> {
        let start_time = Instant::now();

        // Acquire backpressure permit
        let permit = if self.config.enable_backpressure {
            Some(self.backpressure_sem.acquire().await.map_err(|e| {
                DbError::InvalidOperation(format!("Backpressure semaphore error: {}", e))
            })?)
        } else {
            None
        }));

        // Get topic
        let topics = self.topics.read();
        let topic = topics.get(&event.topic)
            .ok_or_else(|| DbError::NotFound(format!("Topic '{}' not found", event.topic)))?
            .clone()));
        drop(topics);

        // Assign event ID
        event.id = self.next_event_id.fetch_add(1, Ordering::SeqCst);

        // Select partition
        let partition_id = if event.partition > 0 {
            event.partition
        } else {
            topic.select_partition(event.key.as_deref())
        };
        event.partition = partition_id;

        // Get partition
        let partition = topic.get_partition(partition_id)
            .ok_or_else(|| DbError::InvalidOperation(
                format!("Partition {} not found", partition_id)
            ))?);

        // Allocate offset
        let offset = partition.allocate_offset();
        event.offset = offset;

        // Compress if needed
        let event = self.compress_event(event)?;

        // Update statistics
        let event_size = event.size_bytes();
        {
            let mut stats = self.stats.write();
            stats.total_events += 1;
            stats.total_bytes += event_size as u64;
            topic.total_events.fetch_add(1, Ordering::SeqCst);
            topic.total_bytes.fetch_add(event_size as u64, Ordering::SeqCst);

            let latency_ms = start_time.elapsed().as_millis() as f64;
            stats.avg_publish_latency_ms =
                (stats.avg_publish_latency_ms * 0.95) + (latency_ms * 0.05);
        }

        // Create acknowledgment
        let ack = PublishAck {
            event_id: event.id,
            topic: event.topic.clone(),
            partition: event.partition,
            offset: event.offset,
            timestamp: SystemTime::now(),
        };

        // Enqueue event
        partition.enqueue(event);

        // Release permit
        drop(permit);

        Ok(ack)
    }

    /// Publish a batch of events
    pub async fn publish_batch(&self, events: Vec<PublishedEvent>) -> Result<Vec<PublishAck>> {
        let mut acks = Vec::new();

        for event in events {
            let ack = self.publish(event).await?;
            acks.push(ack);
        }

        Ok(acks)
    }

    /// Publish with custom partitioner
    pub async fn publish_with_partitioner<F>(
        &self,
        event: PublishedEvent,
        partitioner: F,
    ) -> Result<PublishAck>
    where
        F: FnOnce(&PublishedEvent, u32) -> u32,
    {
        let topics = self.topics.read();
        let topic = topics.get(&event.topic)
            .ok_or_else(|| DbError::NotFound(format!("Topic '{}' not found", event.topic)))?);

        let num_partitions = topic.config.num_partitions;
        drop(topics);

        let mut event = event;
        event.partition = partitioner(&event, num_partitions);

        self.publish(event).await
    }

    /// Get events from partition (for internal consumption)
    pub async fn read_partition(
        &self,
        topic_name: &str,
        partition_id: u32,
        max_events: usize,
    ) -> Result<Vec<PublishedEvent>> {
        let topics = self.topics.read();
        let topic = topics.get(topic_name)
            .ok_or_else(|| DbError::NotFound(format!("Topic '{}' not found", topic_name)))?);

        let partition = topic.get_partition(partition_id)
            .ok_or_else(|| DbError::InvalidOperation(
                format!("Partition {} not found", partition_id)
            ))?);

        Ok(partition.dequeue_batch(max_events))
    }

    /// Get partition offset
    pub fn get_partition_offset(&self, topic_name: &str, partition_id: u32) -> Result<u64> {
        let topics = self.topics.read();
        let topic = topics.get(topic_name)
            .ok_or_else(|| DbError::NotFound(format!("Topic '{}' not found", topic_name)))?);

        let partition = topic.get_partition(partition_id)
            .ok_or_else(|| DbError::InvalidOperation(
                format!("Partition {} not found", partition_id)
            ))?);

        Ok(partition.next_offset.load(Ordering::SeqCst))
    }

    /// Flush all pending events
    pub async fn flush(&self) -> Result<()> {
        // Wait for all pending acknowledgments
        while self.pending_acks.read().len() > 0 {
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
        Ok(())
    }

    /// Get statistics
    pub fn get_statistics(&self) -> PublisherStats {
        self.stats.read().clone()
    }

    /// Check if backpressure is active
    pub fn is_backpressured(&self) -> bool {
        self.backpressure_sem.available_permits() == 0
    }

    /// Shutdown the publisher
    pub async fn shutdown(&self) -> Result<()> {
        self.shutdown.store(true, Ordering::SeqCst);
        self.flush().await?;
        Ok(())
    }

    // Helper methods

    fn compress_event(&self, mut event: PublishedEvent) -> Result<PublishedEvent> {
        if !self.config.enable_compression {
            return Ok(event);
        }

        if event.payload.len() < self.config.compression_threshold {
            return Ok(event);
        }

        // Simplified compression (use proper compression in production)
        event.headers.insert("compressed".to_string(), "true".to_string());
        Ok(event)
    }
}

/// Event serializer trait
pub trait EventSerializer {
    fn serialize<T: Serialize>(&self, value: &T, format: SerializationFormat) -> Result<Vec<u8>>;
    fn deserialize<T: for<'de> Deserialize<'de>>(&self, data: &[u8], format: SerializationFormat) -> Result<T>;
}

/// Default event serializer
pub struct DefaultSerializer;

impl EventSerializer for DefaultSerializer {
    fn serialize<T: Serialize>(&self, value: &T, format: SerializationFormat) -> Result<Vec<u8>> {
        match format {
            SerializationFormat::Json => {
                serde_json::to_vec(value)
                    .map_err(|e| DbError::SerializationError(e.to_string()))
            }
            SerializationFormat::MessagePack => {
                rmp_serde::to_vec(value)
                    .map_err(|e| DbError::SerializationError(e.to_string()))
            }
            SerializationFormat::Binary => {
                bincode::serialize(value)
                    .map_err(|e| DbError::SerializationError(e.to_string()))
            }
            _ => Err(DbError::NotImplemented("Serialization format not supported".to_string())),
        }
    }

    fn deserialize<T: for<'de> Deserialize<'de>>(&self, data: &[u8], format: SerializationFormat) -> Result<T> {
        match format {
            SerializationFormat::Json => {
                serde_json::from_slice(data)
                    .map_err(|e| DbError::SerializationError(e.to_string()))
            }
            SerializationFormat::MessagePack => {
                rmp_serde::from_slice(data)
                    .map_err(|e| DbError::SerializationError(e.to_string()))
            }
            SerializationFormat::Binary => {
                bincode::deserialize(data)
                    .map_err(|e| DbError::SerializationError(e.to_string()))
            }
            _ => Err(DbError::NotImplemented("Deserialization format not supported".to_string())),
        }
    }
}

/// Partitioner trait for custom partitioning logic
pub trait Partitioner: Send + Sync {
    fn partition(&self, event: &PublishedEvent, num_partitions: u32) -> u32;
}

/// Round-robin partitioner
pub struct RoundRobinPartitioner {
    counter: AtomicU64,
}

impl RoundRobinPartitioner {
    pub fn new() -> Self {
        Self {
            counter: AtomicU64::new(0),
        }
    }
}

impl Partitioner for RoundRobinPartitioner {
    fn partition(&self, _event: &PublishedEvent, num_partitions: u32) -> u32 {
        let count = self.counter.fetch_add(1, Ordering::SeqCst);
        (count % num_partitions as u64) as u32
    }
}

/// Hash-based partitioner
pub struct HashPartitioner;

impl Partitioner for HashPartitioner {
    fn partition(&self, event: &PublishedEvent, num_partitions: u32) -> u32 {
        if let Some(key) = &event.key {
            let mut hasher = std::collections::hash_map::DefaultHasher::new();
            key.hash(&mut hasher);
            (hasher.finish() % num_partitions as u64) as u32
        } else {
            0
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_published_event() {
        let event = PublishedEvent::new("test".to_string(), vec![1, 2, 3])
            .with_key(vec![4, 5, 6])
            .with_header("type".to_string(), "insert".to_string());

        assert_eq!(event.topic, "test");
        assert_eq!(event.payload, vec![1, 2, 3]);
        assert!(event.key.is_some());
    }

    #[test]
    fn test_topic_config() {
        let config = TopicConfig::new("events".to_string(), 4);
        assert_eq!(config.name, "events");
        assert_eq!(config.num_partitions, 4);
    }

    #[tokio::test]
    async fn test_create_topic() {
        let publisher = EventPublisher::new(PublisherConfig::default());
        let config = TopicConfig::new("test".to_string(), 2);

        publisher.create_topic(config).await.unwrap();
        assert!(publisher.list_topics().contains(&"test".to_string()));
    }

    #[tokio::test]
    async fn test_publish_event() {
        let publisher = EventPublisher::new(PublisherConfig::default());
        let config = TopicConfig::new("test".to_string(), 2);
        publisher.create_topic(config).await.unwrap();

        let event = PublishedEvent::new("test".to_string(), vec![1, 2, 3]);
        let ack = publisher.publish(event).await.unwrap();

        assert_eq!(ack.topic, "test");
        assert!(ack.offset >= 0);
    }

    #[test]
    fn test_round_robin_partitioner() {
        let partitioner = RoundRobinPartitioner::new();
        let event = PublishedEvent::new("test".to_string(), vec![]);

        let p1 = partitioner.partition(&event, 3);
        let p2 = partitioner.partition(&event, 3);
        let p3 = partitioner.partition(&event, 3);

        assert_eq!(p1, 0);
        assert_eq!(p2, 1);
        assert_eq!(p3, 2);
    }

    #[test]
    fn test_serializer() {
        let serializer = DefaultSerializer;

        #[derive(Serialize, Deserialize, PartialEq, Debug)]
        struct TestData {
            value: i32,
        }

        let data = TestData { value: 42 };
        let bytes = serializer.serialize(&data, SerializationFormat::Json).unwrap();
        let deserialized: TestData = serializer.deserialize(&bytes, SerializationFormat::Json).unwrap();

        assert_eq!(data, deserialized);
    }
}
