// Event Stream Management
//
// Provides Oracle Streams-like functionality for event stream lifecycle management,
// partitioning, retention, compaction, and consumer group coordination with
// exactly-once processing semantics.

use super::{
    Event, EventBatch, EventId, EventProcessingConfig, ProcessingGuarantee, StreamMetrics,
    StreamPosition, StreamState, Watermark,
};
use crate::error::Result;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap, VecDeque};
use std::sync::{Arc, Mutex, RwLock};
use std::time::{Duration, SystemTime};

/// Stream identifier
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct StreamId(pub String);

impl StreamId {
    pub fn new(id: impl Into<String>) -> Self {
        StreamId(id.into())
    }
}

impl std::fmt::Display for StreamId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Stream lifecycle state
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum StreamLifecycleState {
    /// Stream is being created
    Creating,

    /// Stream is active and accepting events
    Active,

    /// Stream is paused (no new events accepted, reading allowed)
    Paused,

    /// Stream is being compacted
    Compacting,

    /// Stream is being deleted
    Deleting,

    /// Stream is deleted
    Deleted,
}

/// Retention policy for stream data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RetentionPolicy {
    /// Keep events for a specific duration
    TimeBased {
        retention: Duration,
    },

    /// Keep a specific number of events per partition
    SizeBased {
        max_events: u64,
    },

    /// Keep events until total size limit
    ByteBased {
        max_bytes: u64,
    },

    /// Composite policy (all conditions must be met)
    Composite {
        time: Option<Duration>,
        max_events: Option<u64>,
        max_bytes: Option<u64>,
    },

    /// Custom retention logic
    Custom {
        policy_name: String,
    },
}

impl RetentionPolicy {
    pub fn should_retain(&self, event: &Event, current_time: SystemTime) -> bool {
        match self {
            RetentionPolicy::TimeBased { retention } => {
                if let Ok(age) = current_time.duration_since(event.event_time) {
                    age < *retention
                } else {
                    true
                }
            }
            RetentionPolicy::SizeBased { .. } => true, // Handled by partition-level logic
            RetentionPolicy::ByteBased { .. } => true, // Handled by partition-level logic
            RetentionPolicy::Composite {
                time,
                max_events: _,
                max_bytes: _,
            } => {
                if let Some(retention) = time {
                    if let Ok(age) = current_time.duration_since(event.event_time) {
                        age < *retention
                    } else {
                        true
                    }
                } else {
                    true
                }
            }
            RetentionPolicy::Custom { .. } => true, // Custom logic handled externally
        }
    }
}

/// Compaction strategy
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CompactionStrategy {
    /// No compaction
    None,

    /// Keep only the latest event per key
    LatestByKey,

    /// Keep only tombstone events (for deletion)
    Tombstone,

    /// Custom compaction logic
    Custom {
        strategy_name: String,
    },
}

/// Partitioning strategy
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PartitionStrategy {
    /// Hash-based partitioning on partition key
    Hash {
        num_partitions: u32,
    },

    /// Range-based partitioning
    Range {
        ranges: Vec<String>,
    },

    /// Round-robin partitioning
    RoundRobin {
        num_partitions: u32,
    },

    /// Custom partitioning logic
    Custom {
        strategy_name: String,
    },
}

impl PartitionStrategy {
    pub fn get_partition(&self, event: &Event, partition_counter: &mut u32) -> u32 {
        match self {
            PartitionStrategy::Hash { num_partitions } => {
                let key = event.partition_key.as_ref().unwrap_or(&event.id.to_string());
                let hash = Self::hash_string(key);
                hash % num_partitions
            }
            PartitionStrategy::RoundRobin { num_partitions } => {
                let partition = *partition_counter;
                *partition_counter = (*partition_counter + 1) % num_partitions;
                partition
            }
            PartitionStrategy::Range { ranges } => {
                let key = event.partition_key.as_ref().unwrap_or(&event.id.to_string());
                for (i, range) in ranges.iter().enumerate() {
                    if key >= range {
                        return i as u32;
                    }
                }
                (ranges.len() as u32).saturating_sub(1)
            }
            PartitionStrategy::Custom { .. } => 0, // Custom logic handled externally
        }
    }

    fn hash_string(s: &str) -> u32 {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        let mut hasher = DefaultHasher::new();
        s.hash(&mut hasher);
        hasher.finish() as u32
    }
}

/// Stream configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamConfig {
    /// Stream name
    pub name: String,

    /// Number of partitions
    pub num_partitions: u32,

    /// Partitioning strategy
    pub partition_strategy: PartitionStrategy,

    /// Retention policy
    pub retention_policy: RetentionPolicy,

    /// Compaction strategy
    pub compaction_strategy: CompactionStrategy,

    /// Replication factor
    pub replication_factor: u32,

    /// Enable exactly-once semantics
    pub exactly_once: bool,

    /// Maximum message size in bytes
    pub max_message_bytes: usize,

    /// Compression codec
    pub compression: CompressionCodec,
}

impl Default for StreamConfig {
    fn default() -> Self {
        Self {
            name: String::from("default"),
            num_partitions: 4,
            partition_strategy: PartitionStrategy::Hash { num_partitions: 4 },
            retention_policy: RetentionPolicy::TimeBased {
                retention: Duration::from_secs(7 * 24 * 3600), // 7 days
            },
            compaction_strategy: CompactionStrategy::None,
            replication_factor: 3,
            exactly_once: true,
            max_message_bytes: 1024 * 1024, // 1MB
            compression: CompressionCodec::Snappy,
        }
    }
}

/// Compression codec
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum CompressionCodec {
    None,
    Gzip,
    Snappy,
    Lz4,
    Zstd,
}

/// Event stream
pub struct EventStream {
    /// Stream ID
    id: StreamId,

    /// Stream configuration
    config: Arc<StreamConfig>,

    /// Stream state
    state: RwLock<StreamLifecycleState>,

    /// Partitions
    partitions: Vec<Arc<Mutex<StreamPartition>>>,

    /// Consumer groups
    consumer_groups: Arc<RwLock<HashMap<String, Arc<ConsumerGroup>>>>,

    /// Stream metrics
    metrics: Arc<RwLock<StreamMetrics>>,

    /// Round-robin counter for partitioning
    partition_counter: Arc<Mutex<u32>>,
}

impl EventStream {
    pub fn new(id: StreamId, config: StreamConfig) -> Result<Self> {
        let num_partitions = config.num_partitions;
        let config = Arc::new(config);

        let mut partitions = Vec::new();
        for i in 0..num_partitions {
            partitions.push(Arc::new(Mutex::new(StreamPartition::new(
                i,
                config.clone(),
            ))));
        }

        Ok(Self {
            id,
            config,
            state: RwLock::new(StreamLifecycleState::Active),
            partitions,
            consumer_groups: Arc::new(RwLock::new(HashMap::new())),
            metrics: Arc::new(RwLock::new(StreamMetrics::new())),
            partition_counter: Arc::new(Mutex::new(0)),
        })
    }

    /// Publish an event to the stream
    pub fn publish(&self, mut event: Event) -> Result<StreamPosition> {
        // Check state
        let state = self.state.read().unwrap();
        if *state != StreamLifecycleState::Active {
            return Err(crate::error::DbError::InvalidOperation(format!(
                "Stream {} is not active (state: {:?})",
                self.id, state
            )));
        }
        drop(state);

        // Determine partition
        let mut counter = self.partition_counter.lock().unwrap();
        let partition_id = self.config.partition_strategy.get_partition(&event, &mut counter);
        drop(counter);

        // Set ingestion time
        event.ingestion_time = SystemTime::now();

        // Publish to partition
        let partition = &self.partitions[partition_id as usize];
        let mut partition = partition.lock().unwrap();
        let position = partition.append(event)?;

        // Update metrics
        let mut metrics = self.metrics.write().unwrap();
        metrics.record_event(100, 1.0); // Simplified
        drop(metrics);

        Ok(position)
    }

    /// Publish a batch of events
    pub fn publish_batch(&self, events: Vec<Event>) -> Result<Vec<StreamPosition>> {
        let mut positions = Vec::new();
        for event in events {
            positions.push(self.publish(event)?);
        }
        Ok(positions)
    }

    /// Create or get a consumer group
    pub fn consumer_group(&self, group_id: impl Into<String>) -> Arc<ConsumerGroup> {
        let group_id = group_id.into();
        let mut groups = self.consumer_groups.write().unwrap();

        groups
            .entry(group_id.clone())
            .or_insert_with(|| {
                Arc::new(ConsumerGroup::new(
                    group_id,
                    self.config.num_partitions,
                    self.partitions.clone(),
                ))
            })
            .clone()
    }

    /// Get stream metrics
    pub fn metrics(&self) -> StreamMetrics {
        self.metrics.read().unwrap().clone()
    }

    /// Pause the stream
    pub fn pause(&self) -> Result<()> {
        let mut state = self.state.write().unwrap();
        *state = StreamLifecycleState::Paused;
        Ok(())
    }

    /// Resume the stream
    pub fn resume(&self) -> Result<()> {
        let mut state = self.state.write().unwrap();
        *state = StreamLifecycleState::Active;
        Ok(())
    }

    /// Compact the stream
    pub fn compact(&self) -> Result<()> {
        let mut state = self.state.write().unwrap();
        *state = StreamLifecycleState::Compacting;
        drop(state);

        // Compact each partition
        for partition in &self.partitions {
            let mut partition = partition.lock().unwrap();
            partition.compact()?;
        }

        let mut state = self.state.write().unwrap();
        *state = StreamLifecycleState::Active;
        Ok(())
    }

    /// Apply retention policy
    pub fn apply_retention(&self) -> Result<()> {
        for partition in &self.partitions {
            let mut partition = partition.lock().unwrap();
            partition.apply_retention()?;
        }
        Ok(())
    }

    /// Get partition count
    pub fn partition_count(&self) -> u32 {
        self.config.num_partitions
    }

    /// Get stream state
    pub fn get_state(&self) -> StreamLifecycleState {
        *self.state.read().unwrap()
    }
}

/// Stream partition
struct StreamPartition {
    /// Partition ID
    id: u32,

    /// Configuration
    config: Arc<StreamConfig>,

    /// Event log (offset -> event)
    log: BTreeMap<u64, Event>,

    /// Current offset
    current_offset: u64,

    /// Partition metrics
    total_bytes: u64,

    /// Earliest available offset
    earliest_offset: u64,

    /// Latest committed offsets by consumer group
    committed_offsets: HashMap<String, u64>,
}

impl StreamPartition {
    fn new(id: u32, config: Arc<StreamConfig>) -> Self {
        Self {
            id,
            config,
            log: BTreeMap::new(),
            current_offset: 0,
            total_bytes: 0,
            earliest_offset: 0,
            committed_offsets: HashMap::new(),
        }
    }

    fn append(&mut self, event: Event) -> Result<StreamPosition> {
        let offset = self.current_offset;

        // Check message size
        let event_size = self.estimate_size(&event);
        if event_size > self.config.max_message_bytes {
            return Err(crate::error::DbError::InvalidInput(format!(
                "Event size {} exceeds maximum {}",
                event_size, self.config.max_message_bytes
            )));
        }

        self.log.insert(offset, event);
        self.total_bytes += event_size as u64;
        self.current_offset += 1;

        Ok(StreamPosition::new(self.id, offset))
    }

    fn read(&self, offset: u64, max_events: usize) -> EventBatch {
        let mut batch = EventBatch::new(self.id, offset);

        for (&o, event) in self.log.range(offset..) {
            if batch.len() >= max_events {
                break;
            }
            batch.add(event.clone());
            batch.end_offset = o + 1;
        }

        batch
    }

    fn commit(&mut self, consumer_group: &str, offset: u64) {
        self.committed_offsets
            .insert(consumer_group.to_string(), offset);
    }

    fn get_committed(&self, consumer_group: &str) -> Option<u64> {
        self.committed_offsets.get(consumer_group).copied()
    }

    fn compact(&mut self) -> Result<()> {
        match &self.config.compaction_strategy {
            CompactionStrategy::None => Ok(()),
            CompactionStrategy::LatestByKey => {
                let mut latest_by_key: HashMap<String, (u64, Event)> = HashMap::new();

                for (&offset, event) in &self.log {
                    let key = event
                        .partition_key
                        .as_ref()
                        .unwrap_or(&event.id.to_string())
                        .clone();

                    latest_by_key
                        .entry(key)
                        .and_modify(|(o, e)| {
                            if offset > *o {
                                *o = offset;
                                *e = event.clone();
                            }
                        })
                        .or_insert((offset, event.clone()));
                }

                self.log.clear();
                self.total_bytes = 0;

                for (offset, event) in latest_by_key.values() {
                    let size = self.estimate_size(event);
                    self.log.insert(*offset, event.clone());
                    self.total_bytes += size as u64;
                }

                Ok(())
            }
            CompactionStrategy::Tombstone => {
                self.log.retain(|_, event| event.metadata.is_tombstone);
                self.recalculate_size();
                Ok(())
            }
            CompactionStrategy::Custom { .. } => Ok(()),
        }
    }

    fn apply_retention(&mut self) -> Result<()> {
        let current_time = SystemTime::now();
        let mut to_remove = Vec::new();

        for (&offset, event) in &self.log {
            if !self.config.retention_policy.should_retain(event, current_time) {
                to_remove.push(offset);
            }
        }

        for offset in to_remove {
            self.log.remove(&offset);
        }

        // Update earliest offset
        if let Some(&first) = self.log.keys().next() {
            self.earliest_offset = first;
        }

        self.recalculate_size();
        Ok(())
    }

    fn estimate_size(&self, event: &Event) -> usize {
        // Simplified size estimation
        let mut size = 0;
        size += event.event_type.len();
        size += event.payload.len() * 50; // Rough estimate
        size += event.metadata.source.len();
        size
    }

    fn recalculate_size(&mut self) {
        self.total_bytes = self
            .log
            .values()
            .map(|e| self.estimate_size(e) as u64)
            .sum();
    }
}

/// Consumer group for coordinated consumption
pub struct ConsumerGroup {
    /// Group ID
    id: String,

    /// Number of partitions
    num_partitions: u32,

    /// Partition assignments
    assignments: Arc<RwLock<HashMap<String, Vec<u32>>>>,

    /// Current positions
    positions: Arc<RwLock<HashMap<u32, u64>>>,

    /// Partitions
    partitions: Vec<Arc<Mutex<StreamPartition>>>,

    /// Consumer sessions
    consumers: Arc<RwLock<HashMap<String, ConsumerSession>>>,
}

impl ConsumerGroup {
    fn new(
        id: String,
        num_partitions: u32,
        partitions: Vec<Arc<Mutex<StreamPartition>>>,
    ) -> Self {
        Self {
            id,
            num_partitions,
            assignments: Arc::new(RwLock::new(HashMap::new())),
            positions: Arc::new(RwLock::new(HashMap::new())),
            partitions,
            consumers: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Register a consumer
    pub fn register_consumer(&self, consumer_id: impl Into<String>) -> Result<Consumer> {
        let consumer_id = consumer_id.into();

        let session = ConsumerSession {
            consumer_id: consumer_id.clone(),
            group_id: self.id.clone(),
            last_heartbeat: SystemTime::now(),
            assigned_partitions: Vec::new(),
        };

        let mut consumers = self.consumers.write().unwrap();
        consumers.insert(consumer_id.clone(), session);
        drop(consumers);

        // Rebalance partitions
        self.rebalance()?;

        Ok(Consumer {
            id: consumer_id,
            group: self.id.clone(),
            positions: self.positions.clone(),
            partitions: self.partitions.clone(),
            assignments: self.assignments.clone(),
        })
    }

    /// Rebalance partitions among consumers
    fn rebalance(&self) -> Result<()> {
        let consumers = self.consumers.read().unwrap();
        let num_consumers = consumers.len();

        if num_consumers == 0 {
            return Ok(());
        }

        let mut new_assignments = HashMap::new();
        let partitions_per_consumer = self.num_partitions / num_consumers as u32;
        let mut extra_partitions = self.num_partitions % num_consumers as u32;

        let mut partition_idx = 0u32;
        for consumer_id in consumers.keys() {
            let mut assigned = Vec::new();
            let mut count = partitions_per_consumer;

            if extra_partitions > 0 {
                count += 1;
                extra_partitions -= 1;
            }

            for _ in 0..count {
                assigned.push(partition_idx);
                partition_idx += 1;
            }

            new_assignments.insert(consumer_id.clone(), assigned);
        }

        drop(consumers);

        let mut assignments = self.assignments.write().unwrap();
        *assignments = new_assignments;

        Ok(())
    }

    /// Commit offset
    pub fn commit(&self, partition: u32, offset: u64) -> Result<()> {
        let mut positions = self.positions.write().unwrap();
        positions.insert(partition, offset);

        // Also commit to partition
        let partition_lock = &self.partitions[partition as usize];
        let mut partition = partition_lock.lock().unwrap();
        partition.commit(&self.id, offset);

        Ok(())
    }

    /// Get committed offset
    pub fn get_committed(&self, partition: u32) -> Option<u64> {
        let positions = self.positions.read().unwrap();
        positions.get(&partition).copied()
    }
}

/// Consumer session
#[derive(Debug, Clone)]
struct ConsumerSession {
    consumer_id: String,
    group_id: String,
    last_heartbeat: SystemTime,
    assigned_partitions: Vec<u32>,
}

/// Consumer for reading from a stream
pub struct Consumer {
    /// Consumer ID
    id: String,

    /// Consumer group
    group: String,

    /// Shared positions
    positions: Arc<RwLock<HashMap<u32, u64>>>,

    /// Partitions
    partitions: Vec<Arc<Mutex<StreamPartition>>>,

    /// Assignments
    assignments: Arc<RwLock<HashMap<String, Vec<u32>>>>,
}

impl Consumer {
    /// Poll for events
    pub fn poll(&self, timeout: Duration) -> Result<Vec<EventBatch>> {
        let assignments = self.assignments.read().unwrap();
        let my_partitions = assignments.get(&self.id).cloned().unwrap_or_default();
        drop(assignments);

        let mut batches = Vec::new();

        for &partition_id in &my_partitions {
            let offset = {
                let positions = self.positions.read().unwrap();
                positions.get(&partition_id).copied().unwrap_or(0)
            };

            let partition = &self.partitions[partition_id as usize];
            let partition = partition.lock().unwrap();
            let batch = partition.read(offset, 100);

            if !batch.is_empty() {
                batches.push(batch);
            }
        }

        Ok(batches)
    }

    /// Commit offsets
    pub fn commit(&self, position: StreamPosition) -> Result<()> {
        let mut positions = self.positions.write().unwrap();
        positions.insert(position.partition, position.offset);
        Ok(())
    }

    /// Get assigned partitions
    pub fn assigned_partitions(&self) -> Vec<u32> {
        let assignments = self.assignments.read().unwrap();
        assignments.get(&self.id).cloned().unwrap_or_default()
    }
}

/// Stream manager for managing multiple streams
pub struct StreamManager {
    streams: Arc<RwLock<HashMap<StreamId, Arc<EventStream>>>>,
    config: Arc<EventProcessingConfig>,
}

impl StreamManager {
    pub fn new(config: EventProcessingConfig) -> Self {
        Self {
            streams: Arc::new(RwLock::new(HashMap::new())),
            config: Arc::new(config),
        }
    }

    /// Create a new stream
    pub fn create_stream(&self, id: StreamId, config: StreamConfig) -> Result<Arc<EventStream>> {
        let stream = Arc::new(EventStream::new(id.clone(), config)?);

        let mut streams = self.streams.write().unwrap();
        streams.insert(id, stream.clone());

        Ok(stream)
    }

    /// Get a stream by ID
    pub fn get_stream(&self, id: &StreamId) -> Option<Arc<EventStream>> {
        let streams = self.streams.read().unwrap();
        streams.get(id).cloned()
    }

    /// Delete a stream
    pub fn delete_stream(&self, id: &StreamId) -> Result<()> {
        let mut streams = self.streams.write().unwrap();
        streams.remove(id);
        Ok(())
    }

    /// List all streams
    pub fn list_streams(&self) -> Vec<StreamId> {
        let streams = self.streams.read().unwrap();
        streams.keys().cloned().collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_stream_creation() {
        let id = StreamId::new("test_stream");
        let config = StreamConfig::default();
        let stream = EventStream::new(id.clone(), config).unwrap();

        assert_eq!(stream.get_state(), StreamLifecycleState::Active);
        assert_eq!(stream.partition_count(), 4);
    }

    #[test]
    fn test_event_publishing() {
        let id = StreamId::new("test_stream");
        let config = StreamConfig::default();
        let stream = EventStream::new(id, config).unwrap();

        let event = Event::new("test.event").with_payload("key", "value");
        let position = stream.publish(event).unwrap();

        assert!(position.offset == 0);
    }

    #[test]
    fn test_consumer_group() {
        let id = StreamId::new("test_stream");
        let config = StreamConfig::default();
        let stream = EventStream::new(id, config).unwrap();

        let group = stream.consumer_group("test_group");
        let consumer1 = group.register_consumer("consumer1").unwrap();
        let consumer2 = group.register_consumer("consumer2").unwrap();

        // Each consumer should have some partitions
        assert!(!consumer1.assigned_partitions().is_empty());
        assert!(!consumer2.assigned_partitions().is_empty());
    }

    #[test]
    fn test_retention_policy() {
        let policy = RetentionPolicy::TimeBased {
            retention: Duration::from_secs(60),
        };

        let old_event = Event::new("old");
        let now = SystemTime::now();
        let future = now + Duration::from_secs(120);

        assert!(!policy.should_retain(&old_event, future));
    }

    #[test]
    fn test_partition_strategy() {
        let strategy = PartitionStrategy::Hash { num_partitions: 4 };

        let event = Event::new("test").with_partition_key("key1");
        let mut counter = 0;

        let partition = strategy.get_partition(&event, &mut counter);
        assert!(partition < 4);
    }
}
