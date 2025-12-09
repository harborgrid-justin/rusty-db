// Event Processing and Complex Event Processing (CEP) Engine
//
// This module provides Oracle Streams-like event processing capabilities with modern
// innovations including out-of-order event handling, GPU-accelerated pattern matching,
// and ML model serving in streams.

use std::time::SystemTime;
use crate::error::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use std::sync::Arc;
use std::time::{Duration};

pub mod analytics;
pub mod cep;
pub mod connectors;
pub mod cq;
pub mod operators;
pub mod sourcing;
pub mod streams;
pub mod windows;

/// Core event type that flows through the system
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Event {
    /// Unique event identifier
    pub id: EventId,

    /// Event type/name
    pub event_type: String,

    /// Event payload as flexible key-value pairs
    pub payload: HashMap<String, EventValue>,

    /// Event timestamp (when the event occurred)
    pub event_time: SystemTime,

    /// Ingestion timestamp (when the event entered the system)
    pub ingestion_time: SystemTime,

    /// Processing timestamp (when the event is being processed)
    pub processing_time: Option<SystemTime>,

    /// Partition key for stream partitioning
    pub partition_key: Option<String>,

    /// Correlation ID for event tracing
    pub correlation_id: Option<String>,

    /// Event metadata
    pub metadata: EventMetadata,
}

/// Unique event identifier
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct EventId(pub u128);

impl EventId {
    pub fn new() -> Self {
        use std::sync::Mutex;
        static COUNTER: Mutex<u128> = Mutex::new(0);
        let mut counter = COUNTER.lock().unwrap();
        let id = *counter;
        *counter += 1;
        EventId(id)
    }
}

impl Default for EventId {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for EventId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:032x}", self.0)
    }
}

/// Event value supporting various data types
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum EventValue {
    Null,
    Bool(bool),
    Int64(i64),
    Float64(f64),
    String(String),
    Bytes(Vec<u8>),
    Timestamp(SystemTime),
    Array(Vec<EventValue>),
    Object(HashMap<String, EventValue>),
}

impl EventValue {
    pub fn as_i64(&self) -> Option<i64> {
        match self {
            EventValue::Int64(v) => Some(*v),
            EventValue::Float64(v) => Some(*v as i64),
            _ => None,
        }
    }

    pub fn as_f64(&self) -> Option<f64> {
        match self {
            EventValue::Float64(v) => Some(*v),
            EventValue::Int64(v) => Some(*v as f64),
            _ => None,
        }
    }

    pub fn as_str(&self) -> Option<&str> {
        match self {
            EventValue::String(s) => Some(s),
            _ => None,
        }
    }

    pub fn as_bool(&self) -> Option<bool> {
        match self {
            EventValue::Bool(b) => Some(*b),
            _ => None,
        }
    }
}

impl From<bool> for EventValue {
    fn from(v: bool) -> Self {
        EventValue::Bool(v)
    }
}

impl From<i64> for EventValue {
    fn from(v: i64) -> Self {
        EventValue::Int64(v)
    }
}

impl From<f64> for EventValue {
    fn from(v: f64) -> Self {
        EventValue::Float64(v)
    }
}

impl From<String> for EventValue {
    fn from(v: String) -> Self {
        EventValue::String(v)
    }
}

impl From<&str> for EventValue {
    fn from(v: &str) -> Self {
        EventValue::String(v.to_string())
    }
}

/// Event metadata
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct EventMetadata {
    /// Source system/application
    pub source: String,

    /// Schema version
    pub schema_version: String,

    /// Custom headers
    pub headers: HashMap<String, String>,

    /// Event priority (0-255, higher is more important)
    pub priority: u8,

    /// Whether this is a tombstone/delete event
    pub is_tombstone: bool,
}

impl Default for EventMetadata {
    fn default() -> Self {
        Self {
            source: String::from("unknown"),
            schema_version: String::from("1.0"),
            headers: HashMap::new(),
            priority: 128,
            is_tombstone: false,
        }
    }
}

impl Event {
    pub fn new(event_type: impl Into<String>) -> Self {
        let now = SystemTime::now();
        Self {
            id: EventId::new(),
            event_type: event_type.into(),
            payload: HashMap::new(),
            event_time: now,
            ingestion_time: now,
            processing_time: None,
            partition_key: None,
            correlation_id: None,
            metadata: EventMetadata::default(),
        }
    }

    pub fn with_payload(mut self, key: impl Into<String>, value: impl Into<EventValue>) -> Self {
        self.payload.insert(key.into(), value.into());
        self
    }

    pub fn with_partition_key(mut self, key: impl Into<String>) -> Self {
        self.partition_key = Some(key.into());
        self
    }

    pub fn with_correlation_id(mut self, id: impl Into<String>) -> Self {
        self.correlation_id = Some(id.into());
        self
    }

    pub fn with_source(mut self, source: impl Into<String>) -> Self {
        self.metadata.source = source.into();
        self
    }

    pub fn get_payload(&self, key: &str) -> Option<&EventValue> {
        self.payload.get(key)
    }

    pub fn set_processing_time(&mut self) {
        self.processing_time = Some(SystemTime::now());
    }
}

/// Stream position for exactly-once semantics
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct StreamPosition {
    pub partition: u32,
    pub offset: u64,
}

impl StreamPosition {
    pub fn new(partition: u32, offset: u64) -> Self {
        Self { partition, offset }
    }
}

impl fmt::Display for StreamPosition {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}", self.partition, self.offset)
    }
}

/// Watermark for out-of-order event handling
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub struct Watermark {
    /// The watermark timestamp
    pub timestamp: SystemTime,

    /// Maximum allowed lateness beyond the watermark
    pub max_lateness: Duration,
}

impl Watermark {
    pub fn new(timestamp: SystemTime, max_lateness: Duration) -> Self {
        Self {
            timestamp,
            max_lateness,
        }
    }

    /// Check if an event is late according to this watermark
    pub fn is_late(&self, event_time: SystemTime) -> bool {
        if let Ok(duration) = self.timestamp.duration_since(event_time) {
            duration > self.max_lateness
        } else {
            false
        }
    }

    /// Get the effective cutoff time (watermark - max_lateness)
    pub fn cutoff_time(&self) -> SystemTime {
        self.timestamp - self.max_lateness
    }
}

/// Processing guarantee semantics
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum ProcessingGuarantee {
    /// At-most-once: events may be lost but never reprocessed
    AtMostOnce,

    /// At-least-once: events may be reprocessed but never lost
    AtLeastOnce,

    /// Exactly-once: events are processed exactly once
    ExactlyOnce,
}

/// Time characteristic for processing
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum TimeCharacteristic {
    /// Processing time: time when the event is processed
    ProcessingTime,

    /// Event time: time when the event occurred
    EventTime,

    /// Ingestion time: time when the event entered the system
    IngestionTime,
}

/// Stream state for checkpointing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamState {
    /// Consumer group ID
    pub consumer_group: String,

    /// Committed positions per partition
    pub committed_positions: HashMap<u32, u64>,

    /// Current watermarks per partition
    pub watermarks: HashMap<u32, Watermark>,

    /// Checkpoint timestamp
    pub checkpoint_time: SystemTime,

    /// Checkpoint version for compatibility
    pub version: u32,
}

impl StreamState {
    pub fn new(consumer_group: impl Into<String>) -> Self {
        Self {
            consumer_group: consumer_group.into(),
            committed_positions: HashMap::new(),
            watermarks: HashMap::new(),
            checkpoint_time: SystemTime::now(),
            version: 1,
        }
    }

    pub fn commit_position(&mut self, partition: u32, offset: u64) {
        self.committed_positions.insert(partition, offset);
    }

    pub fn get_position(&self, partition: u32) -> Option<u64> {
        self.committed_positions.get(&partition).copied()
    }

    pub fn update_watermark(&mut self, partition: u32, watermark: Watermark) {
        self.watermarks.insert(partition, watermark);
    }
}

/// Event batch for efficient processing
#[derive(Debug, Clone)]
pub struct EventBatch {
    pub events: Vec<Event>,
    pub partition: u32,
    pub start_offset: u64,
    pub end_offset: u64,
}

impl EventBatch {
    pub fn new(partition: u32, start_offset: u64) -> Self {
        Self {
            events: Vec::new(),
            partition,
            start_offset,
            end_offset: start_offset,
        }
    }

    pub fn add(&mut self, event: Event) {
        self.events.push(event);
        self.end_offset += 1;
    }

    pub fn len(&self) -> usize {
        self.events.len()
    }

    pub fn is_empty(&self) -> bool {
        self.events.is_empty()
    }
}

/// Stream metrics for monitoring
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct StreamMetrics {
    /// Total events processed
    pub events_processed: u64,

    /// Total events dropped (late, invalid, etc.)
    pub events_dropped: u64,

    /// Total bytes processed
    pub bytes_processed: u64,

    /// Processing latency histogram (in milliseconds)
    pub latency_ms_p50: f64,
    pub latency_ms_p95: f64,
    pub latency_ms_p99: f64,

    /// Events per second
    pub throughput_eps: f64,

    /// Current lag (offset difference from latest)
    pub lag: u64,

    /// Last update timestamp
    pub last_update: Option<SystemTime>,
}

impl StreamMetrics {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn record_event(&mut self, bytes: usize, latency_ms: f64) {
        self.events_processed += 1;
        self.bytes_processed += bytes as u64;
        // Simplified: in production, use a proper histogram library
        self.latency_ms_p50 = (self.latency_ms_p50 * 0.9) + (latency_ms * 0.1);
        self.latency_ms_p95 = self.latency_ms_p95.max(latency_ms);
        self.latency_ms_p99 = self.latency_ms_p99.max(latency_ms);
        self.last_update = Some(SystemTime::now());
    }

    pub fn record_dropped(&mut self) {
        self.events_dropped += 1;
    }

    pub fn update_throughput(&mut self, events: u64, duration: Duration) {
        let seconds = duration.as_secs_f64();
        if seconds > 0.0 {
            self.throughput_eps = events as f64 / seconds;
        }
    }
}

/// Configuration for event processing engine
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventProcessingConfig {
    /// Processing guarantee
    pub guarantee: ProcessingGuarantee,

    /// Time characteristic
    pub time_characteristic: TimeCharacteristic,

    /// Checkpoint interval
    pub checkpoint_interval: Duration,

    /// Maximum out-of-order lateness
    pub max_lateness: Duration,

    /// Watermark generation interval
    pub watermark_interval: Duration,

    /// Batch size for processing
    pub batch_size: usize,

    /// Buffer size for event queues
    pub buffer_size: usize,

    /// Number of parallel workers
    pub parallelism: usize,

    /// Enable GPU acceleration for pattern matching
    pub enable_gpu: bool,

    /// Enable ML model serving
    pub enable_ml: bool,
}

impl Default for EventProcessingConfig {
    fn default() -> Self {
        Self {
            guarantee: ProcessingGuarantee::AtLeastOnce,
            time_characteristic: TimeCharacteristic::EventTime,
            checkpoint_interval: Duration::from_secs(60),
            max_lateness: Duration::from_secs(10),
            watermark_interval: Duration::from_secs(1),
            batch_size: 1000,
            buffer_size: 10000,
            parallelism: num_cpus::get(),
            enable_gpu: false,
            enable_ml: false,
        }
    }
}

/// Event processing context passed to operators
pub struct ProcessingContext {
    pub config: Arc<EventProcessingConfig>,
    pub watermark: Option<Watermark>,
    pub partition: u32,
    pub metrics: StreamMetrics,
}

impl ProcessingContext {
    pub fn new(config: Arc<EventProcessingConfig>, partition: u32) -> Self {
        Self {
            config,
            watermark: None,
            partition,
            metrics: StreamMetrics::new(),
        }
    }

    pub fn update_watermark(&mut self, watermark: Watermark) {
        self.watermark = Some(watermark);
    }

    pub fn is_late(&self, event: &Event) -> bool {
        if let Some(watermark) = &self.watermark {
            watermark.is_late(event.event_time)
        } else {
            false
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_event_creation() {
        let event = Event::new("user.login")
            .with_payload("user_id", 123i64)
            .with_payload("username", "alice")
            .with_partition_key("user_123")
            .with_correlation_id("corr_456")
            .with_source("web_app");

        assert_eq!(event.event_type, "user.login");
        assert_eq!(event.get_payload("user_id").unwrap().as_i64(), Some(123));
        assert_eq!(
            event.get_payload("username").unwrap().as_str(),
            Some("alice")
        );
        assert_eq!(event.partition_key, Some("user_123".to_string()));
        assert_eq!(event.metadata.source, "web_app");
    }

    #[test]
    fn test_event_value_conversions() {
        let int_val = EventValue::from(42i64);
        assert_eq!(int_val.as_i64(), Some(42));

        let float_val = EventValue::from(3.14);
        assert_eq!(float_val.as_f64(), Some(3.14));

        let str_val = EventValue::from("hello");
        assert_eq!(str_val.as_str(), Some("hello"));

        let bool_val = EventValue::from(true);
        assert_eq!(bool_val.as_bool(), Some(true));
    }

    #[test]
    fn test_watermark_lateness() {
        let now = SystemTime::now();
        let watermark = Watermark::new(now::from_secs(5));

        let recent_event = now - Duration::from_secs(3);
        assert!(!watermark.is_late(recent_event));

        let late_event = now - Duration::from_secs(10);
        assert!(watermark.is_late(late_event));
    }

    #[test]
    fn test_stream_state() {
        let mut state = StreamState::new("consumer_group_1");
        state.commit_position(0, 100);
        state.commit_position(1, 200);

        assert_eq!(state.get_position(0), Some(100));
        assert_eq!(state.get_position(1), Some(200));
        assert_eq!(state.get_position(2), None);
    }

    #[test]
    fn test_event_batch() {
        let mut batch = EventBatch::new(0, 100);
        assert!(batch.is_empty());

        batch.add(Event::new("test"));
        batch.add(Event::new("test"));

        assert_eq!(batch.len(), 2);
        assert_eq!(batch.start_offset, 100);
        assert_eq!(batch.end_offset, 102);
    }

    #[test]
    fn test_stream_metrics() {
        let mut metrics = StreamMetrics::new();
        assert_eq!(metrics.events_processed, 0);

        metrics.record_event(100, 5.0);
        assert_eq!(metrics.events_processed, 1);
        assert_eq!(metrics.bytes_processed, 100);

        metrics.record_dropped();
        assert_eq!(metrics.events_dropped, 1);
    }
}


