// # Stream Integration Patterns
//
// Provides integration patterns including Outbox, Event Sourcing, CQRS,
// external system connectors, webhooks, and schema registry.

use std::collections::VecDeque;
use std::sync::Mutex;
use std::time::Duration;
use std::collections::{HashMap};
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, AtomicBool, Ordering};
use std::time::{SystemTime};
use parking_lot::{RwLock};
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc;
use tokio::time::interval;
use crate::error::{DbError, Result};
use crate::common::{TransactionId, Value};
use super::cdc::{ChangeEvent, ChangeType};
use super::publisher::{PublishedEvent, EventPublisher};

// ============================================================================
// Outbox Pattern Implementation
// ============================================================================

/// Outbox entry for transactional outbox pattern
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutboxEntry {
    /// Entry ID
    pub id: u64,
    /// Aggregate ID
    pub aggregate_id: String,
    /// Aggregate type
    pub aggregate_type: String,
    /// Event type
    pub event_type: String,
    /// Event payload
    pub payload: Vec<u8>,
    /// Event metadata
    pub metadata: HashMap<String, String>,
    /// Transaction ID that created this entry
    pub txn_id: TransactionId,
    /// Created timestamp
    pub created_at: SystemTime,
    /// Processed flag
    pub processed: bool,
    /// Processed timestamp
    pub processed_at: Option<SystemTime>,
    /// Retry count
    pub retry_count: u32,
    /// Last error
    pub last_error: Option<String>,
}

impl OutboxEntry {
    pub fn new(
        aggregate_id: String,
        aggregate_type: String,
        event_type: String,
        payload: Vec<u8>,
    ) -> Self {
        Self {
            id: 0,
            aggregate_id,
            aggregate_type,
            event_type,
            payload,
            metadata: HashMap::new(),
            txn_id: 0,
            created_at: SystemTime::now(),
            processed: false,
            processed_at: None,
            retry_count: 0,
            last_error: None,
        }
    }

    pub fn with_metadata(mut self, key: String, value: String) -> Self {
        self.metadata.insert(key, value);
        self
    }
}

/// Outbox processor configuration
#[derive(Debug, Clone)]
pub struct OutboxConfig {
    /// Polling interval
    pub poll_interval: Duration,
    /// Batch size
    pub batch_size: usize,
    /// Max retry attempts
    pub max_retries: u32,
    /// Retry delay
    pub retry_delay: Duration,
    /// Delete processed entries after
    pub retention_period: Duration,
}

impl Default for OutboxConfig {
    fn default() -> Self {
        Self {
            poll_interval: Duration::from_millis(100),
            batch_size: 100,
            max_retries: 3,
            retry_delay: Duration::from_secs(5),
            retention_period: Duration::from_secs(3600),
        }
    }
}

/// Outbox pattern implementation
pub struct OutboxProcessor {
    /// Configuration
    config: OutboxConfig,
    /// Outbox table (in-memory for this implementation)
    outbox: Arc<Mutex<VecDeque<OutboxEntry>>>,
    /// Next entry ID
    next_id: Arc<AtomicU64>,
    /// Event publisher
    publisher: Arc<EventPublisher>,
    /// Statistics
    processed_count: Arc<AtomicU64>,
    failed_count: Arc<AtomicU64>,
    /// Shutdown flag
    shutdown: Arc<AtomicBool>,
}

impl OutboxProcessor {
    pub fn new(config: OutboxConfig, publisher: Arc<EventPublisher>) -> Self {
        Self {
            config,
            outbox: Arc::new(Mutex::new(VecDeque::new())),
            next_id: Arc::new(AtomicU64::new(1)),
            publisher,
            processed_count: Arc::new(AtomicU64::new(0)),
            failed_count: Arc::new(AtomicU64::new(0)),
            shutdown: Arc::new(AtomicBool::new(false)),
        }
    }

    /// Add entry to outbox
    pub fn add_entry(&self, mut entry: OutboxEntry) -> Result<u64> {
        entry.id = self.next_id.fetch_add(1, Ordering::SeqCst);
        entry.created_at = SystemTime::now();

        self.outbox.lock().unwrap().push_back(entry.clone());
        Ok(entry.id)
    }

    /// Start processing outbox entries
    pub fn start(&self) {
        let outbox = self.outbox.clone();
        let publisher = self.publisher.clone();
        let config = self.config.clone();
        let processed_count = self.processed_count.clone();
        let failed_count = self.failed_count.clone();
        let shutdown = self.shutdown.clone();

        tokio::spawn(async move {
            let mut interval = interval(config.poll_interval);

            while !shutdown.load(Ordering::SeqCst) {
                interval.tick().await;

                // Get batch of unprocessed entries
                let entries: Vec<OutboxEntry> = {
                    let mut outbox = outbox.lock().unwrap();
                    let mut batch = Vec::new();

                    for _ in 0..config.batch_size {
                        if let Some(entry) = outbox.pop_front() {
                            if !entry.processed && entry.retry_count < config.max_retries {
                                batch.push(entry);
                            }
                        } else {
                            break;
                        }
                    }
                    batch
                };

                // Process entries
                for mut entry in entries {
                    // Create published event
                    let event = PublishedEvent::new(
                        entry.aggregate_type.clone(),
                        entry.payload.clone(),
                    )
                    .with_key(entry.aggregate_id.as_bytes().to_vec());

                    // Publish
                    match publisher.publish(event).await {
                        Ok(_) => {
                            entry.processed = true;
                            entry.processed_at = Some(SystemTime::now());
                            processed_count.fetch_add(1, Ordering::SeqCst);
                        }
                        Err(e) => {
                            entry.retry_count += 1;
                            entry.last_error = Some(e.to_string());
                            failed_count.fetch_add(1, Ordering::SeqCst);

                            // Re-queue if retries remain
                            if entry.retry_count < config.max_retries {
                                outbox.lock().unwrap().push_back(entry);
                            }
                        }
                    }
                }
            }
        });
    }

    /// Stop processing
    pub fn stop(&self) {
        self.shutdown.store(true, Ordering::SeqCst);
    }

    /// Get statistics
    pub fn get_stats(&self) -> (u64, u64) {
        (
            self.processed_count.load(Ordering::SeqCst),
            self.failed_count.load(Ordering::SeqCst),
        )
    }
}

// ============================================================================
// Event Sourcing Support
// ============================================================================

/// Domain event for event sourcing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DomainEvent {
    /// Event ID
    pub event_id: u64,
    /// Aggregate ID
    pub aggregate_id: String,
    /// Aggregate type
    pub aggregate_type: String,
    /// Event type
    pub event_type: String,
    /// Event version
    pub version: u64,
    /// Event data
    pub data: HashMap<String, Value>,
    /// Event metadata
    pub metadata: HashMap<String, String>,
    /// Timestamp
    pub timestamp: SystemTime,
    /// Causation ID (which command caused this event)
    pub causation_id: Option<String>,
    /// Correlation ID (for distributed tracing)
    pub correlation_id: Option<String>,
}

impl DomainEvent {
    pub fn new(aggregate_id: String, aggregate_type: String, event_type: String) -> Self {
        Self {
            event_id: 0,
            aggregate_id,
            aggregate_type,
            event_type,
            version: 1,
            data: HashMap::new(),
            metadata: HashMap::new(),
            timestamp: SystemTime::now(),
            causation_id: None,
            correlation_id: None,
        }
    }

    pub fn with_data(mut self, key: String, value: Value) -> Self {
        self.data.insert(key, value);
        self
    }

    pub fn with_correlation(mut self, correlation_id: String) -> Self {
        self.correlation_id = Some(correlation_id);
        self
    }
}

/// Event store for event sourcing
pub struct EventStore {
    /// Events indexed by aggregate ID
    events: Arc<RwLock<HashMap<String, Vec<DomainEvent>>>>,
    /// Next event ID
    next_event_id: Arc<AtomicU64>,
    /// Snapshots
    snapshots: Arc<RwLock<HashMap<String, AggregateSnapshot>>>,
}

impl EventStore {
    pub fn new() -> Self {
        Self {
            events: Arc::new(RwLock::new(HashMap::new())),
            next_event_id: Arc::new(AtomicU64::new(1)),
            snapshots: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Append event to stream
    pub fn append_event(&self, mut event: DomainEvent) -> Result<u64> {
        event.event_id = self.next_event_id.fetch_add(1, Ordering::SeqCst);

        let mut events = self.events.write();
        let stream = events.entry(event.aggregate_id.clone()).or_insert_with(Vec::new);

        // Check version (optimistic concurrency)
        let expected_version = stream.len() as u64 + 1;
        if event.version != expected_version {
            return Err(DbError::InvalidOperation(format!(
                "Version mismatch: expected {}, got {}",
                expected_version, event.version
            )));
        }

        stream.push(event.clone());
        Ok(event.event_id)
    }

    /// Get events for aggregate
    pub fn get_events(&self, aggregate_id: &str) -> Vec<DomainEvent> {
        self.events.read()
            .get(aggregate_id)
            .cloned()
            .unwrap_or_default()
    }

    /// Get events from version
    pub fn get_events_from_version(&self, aggregate_id: &str, from_version: u64) -> Vec<DomainEvent> {
        self.get_events(aggregate_id)
            .into_iter()
            .filter(|e| e.version >= from_version)
            .collect()
    }

    /// Save snapshot
    pub fn save_snapshot(&self, snapshot: AggregateSnapshot) {
        self.snapshots.write().insert(snapshot.aggregate_id.clone(), snapshot);
    }

    /// Load snapshot
    pub fn load_snapshot(&self, aggregate_id: &str) -> Option<AggregateSnapshot> {
        self.snapshots.read().get(aggregate_id).cloned()
    }
}

/// Aggregate snapshot for optimization
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AggregateSnapshot {
    pub aggregate_id: String,
    pub aggregate_type: String,
    pub version: u64,
    pub state: Vec<u8>,
    pub timestamp: SystemTime,
}

// ============================================================================
// CQRS Integration
// ============================================================================

/// Command for CQRS pattern
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Command {
    /// Command ID
    pub id: String,
    /// Command type
    pub command_type: String,
    /// Target aggregate ID
    pub aggregate_id: String,
    /// Command payload
    pub payload: HashMap<String, Value>,
    /// Metadata
    pub metadata: HashMap<String, String>,
    /// Issued timestamp
    pub issued_at: SystemTime,
}

/// Query for CQRS pattern
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Query {
    /// Query ID
    pub id: String,
    /// Query type
    pub query_type: String,
    /// Query parameters
    pub parameters: HashMap<String, Value>,
    /// Issued timestamp
    pub issued_at: SystemTime,
}

/// CQRS coordinator
pub struct CQRSCoordinator {
    /// Event store
    event_store: Arc<EventStore>,
    /// Read model projections
    projections: Arc<RwLock<HashMap<String, ReadModelProjection>>>,
}

impl CQRSCoordinator {
    pub fn new(event_store: Arc<EventStore>) -> Self {
        Self {
            event_store,
            projections: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Execute command (write side)
    pub async fn execute_command(&self, command: Command) -> Result<Vec<DomainEvent>> {
        // Load aggregate from event store
        let events = self.event_store.get_events(&command.aggregate_id);

        // Apply command logic and generate events
        let new_events = self.handle_command(&command, &events)?;

        // Persist events
        for event in &new_events {
            self.event_store.append_event(event.clone())?;
        }

        // Update projections
        self.update_projections(&new_events).await?;

        Ok(new_events)
    }

    /// Execute query (read side)
    pub async fn execute_query(&self, query: Query) -> Result<QueryResult> {
        // Query from read models/projections
        let projection_name = &query.query_type;

        let projections = self.projections.read();
        if let Some(projection) = projections.get(projection_name) {
            projection.query(&query)
        } else {
            Err(DbError::NotFound(format!("Projection '{}' not found", projection_name)))
        }
    }

    /// Register a read model projection
    pub fn register_projection(&self, name: String, projection: ReadModelProjection) {
        self.projections.write().insert(name, projection);
    }

    fn handle_command(&self, _command: &Command, events: &[DomainEvent]) -> Result<Vec<DomainEvent>> {
        // Simplified - in production, load aggregate, validate, and generate events
        Ok(Vec::new())
    }

    async fn update_projections(&self, events: &[DomainEvent]) -> Result<()> {
        let projections = self.projections.read();
        for projection in projections.values() {
            for event in events {
                projection.apply_event(event)?;
            }
        }
        Ok(())
    }
}

/// Read model projection
#[derive(Debug, Clone)]
pub struct ReadModelProjection {
    pub name: String,
    pub data: Arc<RwLock<HashMap<String, Vec<u8>>>>,
}

impl ReadModelProjection {
    pub fn new(name: String) -> Self {
        Self {
            name,
            data: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub fn apply_event(&self, event: &DomainEvent) -> Result<()> {
        // Update read model based on event
        // Simplified implementation
        let key = format!("{}:{}", event.aggregate_type, event.aggregate_id);
        self.data.write().insert(key, bincode::serialize(&event)?);
        Ok(())
    }

    pub fn query(&self, query: &Query) -> Result<QueryResult> {
        // Execute query against read model
        let results = self.data.read().values().cloned().collect();
        Ok(QueryResult {
            query_id: query.id.clone(),
            data: results,
            timestamp: SystemTime::now(),
        })
    }
}

/// Query result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryResult {
    pub query_id: String,
    pub data: Vec<Vec<u8>>,
    pub timestamp: SystemTime,
}

// ============================================================================
// External System Connectors
// ============================================================================

/// External system connector
pub trait ExternalConnector: Send + Sync {
    /// Send event to external system
    fn send_event(&self, event: &ChangeEvent) -> Result<()>;

    /// Get connector name
    fn name(&self) -> &str;

    /// Health check
    fn health_check(&self) -> Result<bool>;
}

/// HTTP webhook connector
pub struct WebhookConnector {
    name: String,
    url: String,
    headers: HashMap<String, String>,
    timeout: Duration,
    retry_count: u32,
}

impl WebhookConnector {
    pub fn new(name: String, url: String) -> Self {
        Self {
            name,
            url,
            headers: HashMap::new(),
            timeout: Duration::from_secs(30),
            retry_count: 3,
        }
    }

    pub fn with_header(mut self, key: String, value: String) -> Self {
        self.headers.insert(key, value);
        self
    }
}

impl ExternalConnector for WebhookConnector {
    fn send_event(&self, event: &ChangeEvent) -> Result<()> {
        // In production, use reqwest or similar to send HTTP POST
        let payload = serde_json::to_string(event)
            .map_err(|e| DbError::SerializationError(e.to_string()))?;

        // Simulate HTTP request
        // reqwest::Client::new()
        //     .post(&self.url)
        //     .json(&payload)
        //     .send()
        //     .await?;

        Ok(())
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn health_check(&self) -> Result<bool> {
        // Ping the webhook endpoint
        Ok(true)
    }
}

/// Kafka connector
pub struct KafkaConnector {
    name: String,
    bootstrap_servers: Vec<String>,
    topic_prefix: String,
}

impl KafkaConnector {
    pub fn new(name: String, bootstrap_servers: Vec<String>) -> Self {
        Self {
            name,
            bootstrap_servers,
            topic_prefix: "rustydb.".to_string(),
        }
    }
}

impl ExternalConnector for KafkaConnector {
    fn send_event(&self, event: &ChangeEvent) -> Result<()> {
        let topic = format!("{}{}", self.topic_prefix, event.table_name);
        // Use rdkafka to send to Kafka
        // producer.send(topic, event)?;
        let _ = topic; // Suppress warning
        Ok(())
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn health_check(&self) -> Result<bool> {
        Ok(true)
    }
}

// ============================================================================
// Event Schema Registry
// ============================================================================

/// Event schema definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventSchema {
    /// Schema ID
    pub id: u64,
    /// Event type
    pub event_type: String,
    /// Schema version
    pub version: u32,
    /// Schema format (JSON Schema, Avro, etc.)
    pub format: String,
    /// Schema definition
    pub schema: String,
    /// Compatibility mode
    pub compatibility: SchemaCompatibility,
    /// Created timestamp
    pub created_at: SystemTime,
}

/// Schema compatibility mode
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SchemaCompatibility {
    None,
    Backward,
    Forward,
    Full,
}

/// Schema registry
pub struct SchemaRegistry {
    /// Schemas indexed by (event_type, version)
    schemas: Arc<RwLock<HashMap<(String, u32), EventSchema>>>,
    /// Latest version per event type
    latest_versions: Arc<RwLock<HashMap<String, u32>>>,
    /// Next schema ID
    next_id: Arc<AtomicU64>,
}

impl SchemaRegistry {
    pub fn new() -> Self {
        Self {
            schemas: Arc::new(RwLock::new(HashMap::new())),
            latest_versions: Arc::new(RwLock::new(HashMap::new())),
            next_id: Arc::new(AtomicU64::new(1)),
        }
    }

    /// Register a new schema
    pub fn register_schema(&self, mut schema: EventSchema) -> Result<u64> {
        schema.id = self.next_id.fetch_add(1, Ordering::SeqCst);

        // Check compatibility if not first version
        if schema.version > 1 {
            let prev_version = schema.version - 1;
            if let Some(prev_schema) = self.get_schema(&schema.event_type, prev_version) {
                if !self.is_compatible(&schema, &prev_schema) {
                    return Err(DbError::InvalidOperation(
                        "Schema is not compatible with previous version".to_string()
                    ));
                }
            }
        }

        // Store schema
        let key = (schema.event_type.clone(), schema.version);
        self.schemas.write().insert(key, schema.clone());

        // Update latest version
        self.latest_versions.write()
            .entry(schema.event_type.clone())
            .and_modify(|v| *v = (*v).max(schema.version))
            .or_insert(schema.version);

        Ok(schema.id)
    }

    /// Get schema by event type and version
    pub fn get_schema(&self, event_type: &str, version: u32) -> Option<EventSchema> {
        let key = (event_type.to_string(), version);
        self.schemas.read().get(&key).cloned()
    }

    /// Get latest schema for event type
    pub fn get_latest_schema(&self, event_type: &str) -> Option<EventSchema> {
        let version = *self.latest_versions.read().get(event_type)?;
        self.get_schema(event_type, version)
    }

    /// Validate event against schema
    pub fn validate_event(&self, event_type: &str, event_data: &[u8]) -> Result<bool> {
        let schema = self.get_latest_schema(event_type)
            .ok_or_else(|| DbError::NotFound(format!("Schema for '{}' not found", event_type)))?;

        // In production, use jsonschema or avro validation
        let _ = (schema, event_data);
        Ok(true)
    }

    fn is_compatible(&self, new_schema: &EventSchema, old_schema: &EventSchema) -> bool {
        match new_schema.compatibility {
            SchemaCompatibility::None => true,
            SchemaCompatibility::Backward => {
                // New schema can read old data
                true // Simplified
            }
            SchemaCompatibility::Forward => {
                // Old schema can read new data
                true // Simplified
            }
            SchemaCompatibility::Full => {
                // Both backward and forward compatible
                true // Simplified
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_outbox_entry() {
        let entry = OutboxEntry::new(
            "agg-1".to_string(),
            "Order".to_string(),
            "OrderCreated".to_string(),
            vec![1, 2, 3],
        ).with_metadata("key".to_string(), "value".to_string());

        assert_eq!(entry.aggregate_id, "agg-1");
        assert!(!entry.processed);
    }

    #[test]
    fn test_domain_event() {
        let event = DomainEvent::new(
            "user-123".to_string(),
            "User".to_string(),
            "UserCreated".to_string(),
        ).with_data("name".to_string(), Value::String("Alice".to_string()));

        assert_eq!(event.aggregate_id, "user-123");
        assert_eq!(event.version, 1);
    }

    #[test]
    fn test_event_store() {
        let store = EventStore::new();

        let mut event = DomainEvent::new(
            "agg-1".to_string(),
            "Order".to_string(),
            "OrderCreated".to_string(),
        );
        event.version = 1;

        let id = store.append_event(event).unwrap();
        assert!(id > 0);

        let events = store.get_events("agg-1");
        assert_eq!(events.len(), 1);
    }

    #[test]
    fn test_schema_registry() {
        let registry = SchemaRegistry::new();

        let schema = EventSchema {
            id: 0,
            event_type: "UserCreated".to_string(),
            version: 1,
            format: "json".to_string(),
            schema: "{}".to_string(),
            compatibility: SchemaCompatibility::Backward,
            created_at: SystemTime::now(),
        };

        let id = registry.register_schema(schema).unwrap();
        assert!(id > 0);

        let retrieved = registry.get_latest_schema("UserCreated");
        assert!(retrieved.is_some());
    }
}
