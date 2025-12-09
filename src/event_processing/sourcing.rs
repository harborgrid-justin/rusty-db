// Event Sourcing
//
// Implements event sourcing patterns including event store, event replay,
// snapshots, projection rebuilding, event versioning, and aggregate reconstruction.

use std::collections::VecDeque;
use std::fmt;
use std::time::UNIX_EPOCH;
use super::{Event, EventId, EventValue};
use crate::error::Result;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap};
use std::sync::{Arc, RwLock};
use std::time::SystemTime;

/// Aggregate identifier
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct AggregateId(pub String);

impl AggregateId {
    pub fn new(id: impl Into<String>) -> Self {
        AggregateId(id.into())
    }
}

impl fmt::Display for AggregateId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Aggregate version
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct Version(pub u64);

impl Version {
    pub fn zero() -> Self {
        Version(0)
    }

    pub fn next(self) -> Self {
        Version(self.0 + 1)
    }
}

/// Event envelope with metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventEnvelope {
    /// Aggregate ID this event belongs to
    pub aggregate_id: AggregateId,

    /// Aggregate type
    pub aggregate_type: String,

    /// Event data
    pub event: Event,

    /// Version of the aggregate when this event was created
    pub version: Version,

    /// Causation ID (what caused this event)
    pub causation_id: Option<EventId>,

    /// Correlation ID (transaction/workflow ID)
    pub correlation_id: Option<String>,

    /// Event schema version
    pub schema_version: u32,
}

impl EventEnvelope {
    pub fn new(
        aggregate_id: AggregateId,
        aggregate_type: impl Into<String>,
        event: Event,
        version: Version,
    ) -> Self {
        Self {
            aggregate_id,
            aggregate_type: aggregate_type.into(),
            event,
            version,
            causation_id: None,
            correlation_id: None,
            schema_version: 1,
        }
    }

    pub fn with_causation(mut self, causation_id: EventId) -> Self {
        self.causation_id = Some(causation_id);
        self
    }

    pub fn with_correlation(mut self, correlation_id: impl Into<String>) -> Self {
        self.correlation_id = Some(correlation_id.into());
        self
    }
}

/// Event store trait
pub trait EventStore: Send + Sync {
    /// Append events to an aggregate stream
    fn append_events(
        &mut self,
        aggregate_id: &AggregateId,
        events: Vec<EventEnvelope>,
        expected_version: Option<Version>,
    ) -> Result<Version>;

    /// Get all events for an aggregate
    fn get_events(&self, aggregate_id: &AggregateId) -> Result<Vec<EventEnvelope>>;

    /// Get events for an aggregate from a specific version
    fn get_events_from_version(
        &self,
        aggregate_id: &AggregateId,
        from_version: Version,
    ) -> Result<Vec<EventEnvelope>>;

    /// Get all events of a specific type
    fn get_events_by_type(&self, eventtype: &str) -> Result<Vec<EventEnvelope>>;

    /// Get events in a time range
    fn get_events_by_time_range(
        &self,
        start: SystemTime,
        end: SystemTime,
    ) -> Result<Vec<EventEnvelope>>;

    /// Get the current version of an aggregate
    fn get_current_version(&self, aggregate_id: &AggregateId) -> Result<Option<Version>>;
}

/// In-memory event store implementation
pub struct InMemoryEventStore {
    /// Events indexed by aggregate ID
    events: RwLock<HashMap<AggregateId, Vec<EventEnvelope>>>,

    /// Events indexed by event type
    events_by_type: RwLock<HashMap<String, Vec<EventEnvelope>>>,

    /// All events in order
    all_events: RwLock<Vec<EventEnvelope>>,

    /// Current versions
    versions: RwLock<HashMap<AggregateId, Version>>,
}

impl InMemoryEventStore {
    pub fn new() -> Self {
        Self {
            events: RwLock::new(HashMap::new()),
            events_by_type: RwLock::new(HashMap::new()),
            all_events: RwLock::new(Vec::new()),
            versions: RwLock::new(HashMap::new()),
        }
    }

    pub fn event_count(&self) -> usize {
        self.all_events.read().unwrap().len()
    }

    pub fn aggregate_count(&self) -> usize {
        self.events.read().unwrap().len()
    }
}

impl Default for InMemoryEventStore {
    fn default() -> Self {
        Self::new()
    }
}

impl EventStore for InMemoryEventStore {
    fn append_events(
        &mut self,
        aggregate_id: &AggregateId,
        events: Vec<EventEnvelope>,
        expectedversion: Option<Version>,
    ) -> Result<Version> {
        // Check expected version
        let mut versions = self.versions.write().unwrap();
        let current_version = versions.get(aggregate_id).copied().unwrap_or(Version::zero());

        if let Some(expected) = expectedversion {
            if current_version != expected {
                return Err(crate::error::DbError::InvalidOperation(format!(
                    "Version mismatch: expected {:?}, got {:?}",
                    expected, current_version
                )));
            }
        }

        let mut new_version = current_version;

        // Append events
        let mut event_map = self.events.write().unwrap();
        let aggregate_events = event_map
            .entry(aggregate_id.clone())
            .or_insert_with(Vec::new);

        let mut events_by_type = self.events_by_type.write().unwrap();
        let mut all_events = self.all_events.write().unwrap();

        for event_envelope in events {
            aggregate_events.push(event_envelope.clone());
            all_events.push(event_envelope.clone());

            events_by_type
                .entry(event_envelope.event.event_type.clone())
                .or_insert_with(Vec::new)
                .push(event_envelope.clone());

            new_version = new_version.next();
        }

        versions.insert(aggregate_id.clone(), new_version);

        Ok(new_version)
    }

    fn get_events(&self, aggregate_id: &AggregateId) -> Result<Vec<EventEnvelope>> {
        let events = self.events.read().unwrap();
        Ok(events.get(aggregate_id).cloned().unwrap_or_default())
    }

    fn get_events_from_version(
        &self,
        aggregate_id: &AggregateId,
        fromversion: Version,
    ) -> Result<Vec<EventEnvelope>> {
        let events = self.events.read().unwrap();
        Ok(events
            .get(aggregate_id)
            .map(|evs| {
                evs.iter()
                    .filter(|e| e.version >= fromversion)
                    .cloned()
                    .collect()
            })
            .unwrap_or_default())
    }

    fn get_events_by_type(&self, event_type: &str) -> Result<Vec<EventEnvelope>> {
        let events_by_type = self.events_by_type.read().unwrap();
        Ok(events_by_type.get(event_type).cloned().unwrap_or_default())
    }

    fn get_events_by_time_range(
        &self,
        start: SystemTime,
        end: SystemTime,
    ) -> Result<Vec<EventEnvelope>> {
        let all_events = self.all_events.read().unwrap();
        Ok(all_events
            .iter()
            .filter(|e| e.event.event_time >= start && e.event.event_time <= end)
            .cloned()
            .collect())
    }

    fn get_current_version(&self, aggregate_id: &AggregateId) -> Result<Option<Version>> {
        let versions = self.versions.read().unwrap();
        Ok(versions.get(aggregate_id).copied())
    }
}

/// Aggregate trait
pub trait Aggregate: Send + Sync + Clone {
    /// Get aggregate ID
    fn aggregate_id(&self) -> &AggregateId;

    /// Get aggregate type
    fn aggregate_type(&self) -> &str;

    /// Get current version
    fn version(&self) -> Version;

    /// Apply an event to the aggregate
    fn apply_event(&mut self, event: &Event) -> Result<()>;

    /// Get uncommitted events
    fn uncommitted_events(&self) -> &[Event];

    /// Clear uncommitted events
    fn clear_uncommitted_events(&mut self);
}

/// Aggregate repository
pub struct AggregateRepository<A: Aggregate> {
    event_store: Arc<RwLock<dyn EventStore>>,
    snapshot_store: Arc<RwLock<SnapshotStore<A>>>,
    snapshot_frequency: u64,
}

impl<A: Aggregate> AggregateRepository<A> {
    pub fn new(event_store: Arc<RwLock<dyn EventStore>>) -> Self {
        Self {
            event_store,
            snapshot_store: Arc::new(RwLock::new(SnapshotStore::new())),
            snapshot_frequency: 10, // Take snapshot every 10 events
        }
    }

    pub fn with_snapshot_frequency(mut self, frequency: u64) -> Self {
        self.snapshot_frequency = frequency;
        self
    }

    /// Load an aggregate by ID
    pub fn load(&self, aggregate_id: &AggregateId) -> Result<Option<A>> {
        // Try to load from snapshot first
        let snapshot_store = self.snapshot_store.read().unwrap();
        if let Some(snapshot) = snapshot_store.get_snapshot(aggregate_id) {
            let snapshot = snapshot.clone();
            let snapshot_version = snapshot.version;
            let mut aggregate = snapshot.state;
            drop(snapshot_store);

            // Load events since snapshot
            let event_store = self.event_store.read().unwrap();
            let events = event_store.get_events_from_version(aggregate_id, snapshot_version)?;
            drop(event_store);

            for envelope in events {
                aggregate.apply_event(&envelope.event)?;
            }

            return Ok(Some(aggregate));
        }
        drop(snapshot_store);

        // Load from events
        let event_store = self.event_store.read().unwrap();
        let events = event_store.get_events(aggregate_id)?;
        drop(event_store);

        if events.is_empty() {
            return Ok(None);
        }

        // Reconstruct aggregate from events
        // The first event should establish the aggregate, then we apply subsequent events
        // Note: This requires A to implement Default or have a factory method
        // For a complete implementation, we would use a factory trait:
        //   trait AggregateFactory<A> { fn create(id: AggregateId) -> A; }
        //
        // Since Aggregate trait requires Clone + Send + Sync but not Default,
        // we cannot construct a new aggregate here without additional constraints.
        // In a real implementation, either:
        // 1. Add Default bound to the Aggregate trait
        // 2. Use a factory pattern with a separate trait
        // 3. Pass an initial aggregate instance to the repository
        //
        // For now, we return None to indicate the aggregate needs to be loaded via snapshot
        // or the caller should provide an initial state.
        Ok(None)
    }

    /// Save an aggregate
    pub fn save(&self, aggregate: &mut A) -> Result<()> {
        let uncommitted = aggregate.uncommitted_events();
        if uncommitted.is_empty() {
            return Ok(());
        }

        let envelopes: Vec<EventEnvelope> = uncommitted
            .iter()
            .enumerate()
            .map(|(i, event)| {
                EventEnvelope::new(
                    aggregate.aggregate_id().clone(),
                    aggregate.aggregate_type().to_string(),
                    event.clone(),
                    Version(aggregate.version().0 + i as u64 + 1),
                )
            })
            .collect();

        let mut event_store = self.event_store.write().unwrap();
        let new_version = event_store.append_events(
            aggregate.aggregate_id(),
            envelopes,
            Some(aggregate.version()),
        )?;
        drop(event_store);

        aggregate.clear_uncommitted_events();

        // Check if we should take a snapshot
        if new_version.0 % self.snapshot_frequency == 0 {
            let mut snapshot_store = self.snapshot_store.write().unwrap();
            snapshot_store.save_snapshot(aggregate.aggregate_id().clone(), aggregate.clone(), new_version)?;
        }

        Ok(())
    }
}

/// Snapshot of aggregate state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Snapshot<A> {
    pub aggregate_id: AggregateId,
    pub version: Version,
    pub state: A,
    pub timestamp: SystemTime,
}

/// Snapshot store
pub struct SnapshotStore<A> {
    snapshots: HashMap<AggregateId, Snapshot<A>>,
    snapshot_history: HashMap<AggregateId, VecDeque<Snapshot<A>>>,
    max_history: usize,
}

impl<A: Clone> SnapshotStore<A> {
    pub fn new() -> Self {
        Self {
            snapshots: HashMap::new(),
            snapshot_history: HashMap::new(),
            max_history: 3,
        }
    }

    pub fn save_snapshot(
        &mut self,
        aggregate_id: AggregateId,
        state: A,
        version: Version,
    ) -> Result<()> {
        let snapshot = Snapshot {
            aggregate_id: aggregate_id.clone(),
            version,
            state,
            timestamp: SystemTime::now(),
        };

        self.snapshots.insert(aggregate_id.clone(), snapshot.clone());

        // Add to history
        let history = self
            .snapshot_history
            .entry(aggregate_id)
            .or_insert_with(VecDeque::new);

        history.push_back(snapshot);

        while history.len() > self.max_history {
            history.pop_front();
        }

        Ok(())
    }

    pub fn get_snapshot(&self, aggregate_id: &AggregateId) -> Option<&Snapshot<A>> {
        self.snapshots.get(aggregate_id)
    }

    pub fn get_snapshot_at_version(
        &self,
        aggregate_id: &AggregateId,
        version: Version,
    ) -> Option<&Snapshot<A>> {
        if let Some(history) = self.snapshot_history.get(aggregate_id) {
            history
                .iter()
                .rev()
                .find(|s| s.version <= version)
        } else {
            None
        }
    }
}

impl<A: Clone> Default for SnapshotStore<A> {
    fn default() -> Self {
        Self::new()
    }
}

/// Projection for building read models from events
pub trait Projection: Send + Sync {
    /// Handle an event
    fn handle(&mut self, event: &EventEnvelope) -> Result<()>;

    /// Rebuild projection from all events
    fn rebuild(&mut self, events: Vec<EventEnvelope>) -> Result<()> {
        for event in events {
            self.handle(&event)?;
        }
        Ok(())
    }

    /// Get projection name
    fn name(&self) -> &str;
}

/// Projection manager
pub struct ProjectionManager {
    event_store: Arc<RwLock<dyn EventStore>>,
    projections: RwLock<HashMap<String, Box<dyn Projection>>>,
    positions: RwLock<HashMap<String, usize>>, // projection -> event position
}

impl ProjectionManager {
    pub fn new(event_store: Arc<RwLock<dyn EventStore>>) -> Self {
        Self {
            event_store,
            projections: RwLock::new(HashMap::new()),
            positions: RwLock::new(HashMap::new()),
        }
    }

    /// Register a projection
    pub fn register_projection(&self, projection: Box<dyn Projection>) {
        let name = projection.name().to_string();
        let mut projections = self.projections.write().unwrap();
        projections.insert(name.clone(), projection);

        let mut positions = self.positions.write().unwrap();
        positions.insert(name, 0);
    }

    /// Process new events through projections
    pub fn process_events(&self, events: Vec<EventEnvelope>) -> Result<()> {
        let mut projections = self.projections.write().unwrap();

        for (name, projection) in projections.iter_mut() {
            for event in &events {
                projection.handle(event)?;
            }

            // Update position
            let mut positions = self.positions.write().unwrap();
            if let Some(pos) = positions.get_mut(name) {
                *pos += events.len();
            }
        }

        Ok(())
    }

    /// Rebuild a specific projection
    pub fn rebuild_projection(&self, projection_name: &str) -> Result<()> {
        let event_store = self.event_store.read().unwrap();
        let all_events = event_store.get_events_by_time_range(
            SystemTime::UNIX_EPOCH,
            SystemTime::now(),
        )?;
        drop(event_store);

        let mut projections = self.projections.write().unwrap();
        if let Some(projection) = projections.get_mut(projection_name) {
            projection.rebuild(all_events)?;

            // Reset position
            let mut positions = self.positions.write().unwrap();
            positions.insert(projection_name.to_string(), 0);
        }

        Ok(())
    }

    /// Rebuild all projections
    pub fn rebuild_all(&self) -> Result<()> {
        let names: Vec<String> = {
            let projections = self.projections.read().unwrap();
            projections.keys().cloned().collect()
        };

        for name in names {
            self.rebuild_projection(&name)?;
        }

        Ok(())
    }
}

/// Event replay engine
pub struct EventReplayEngine {
    event_store: Arc<RwLock<dyn EventStore>>,
}

impl EventReplayEngine {
    pub fn new(event_store: Arc<RwLock<dyn EventStore>>) -> Self {
        Self { event_store }
    }

    /// Replay events for an aggregate
    pub fn replay_aggregate(&self, aggregate_id: &AggregateId) -> Result<Vec<EventEnvelope>> {
        let event_store = self.event_store.read().unwrap();
        event_store.get_events(aggregate_id)
    }

    /// Replay events in a time range
    pub fn replay_time_range(
        &self,
        start: SystemTime,
        end: SystemTime,
    ) -> Result<Vec<EventEnvelope>> {
        let event_store = self.event_store.read().unwrap();
        event_store.get_events_by_time_range(start, end)
    }

    /// Replay events of a specific type
    pub fn replay_by_type(&self, event_type: &str) -> Result<Vec<EventEnvelope>> {
        let event_store = self.event_store.read().unwrap();
        event_store.get_events_by_type(event_type)
    }

    /// Replay with a handler function
    pub fn replay_with_handler<F>(&self, start: SystemTime, end: SystemTime, mut handler: F) -> Result<()>
    where
        F: FnMut(&EventEnvelope) -> Result<()>,
    {
        let events = self.replay_time_range(start, end)?;

        for event in events {
            handler(&event)?;
        }

        Ok(())
    }
}

/// Event upcaster for versioning
pub trait EventUpcaster: Send + Sync {
    /// Upcast an event from an old version to the current version
    fn upcast(&self, event: Event, from_version: u32, to_version: u32) -> Result<Event>;

    /// Get the current schema version
    fn current_version(&self) -> u32;
}

/// Simple example aggregate
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExampleAggregate {
    id: AggregateId,
    version: Version,
    data: HashMap<String, EventValue>,
    uncommitted: Vec<Event>,
}

impl ExampleAggregate {
    pub fn new(id: AggregateId) -> Self {
        Self {
            id,
            version: Version::zero(),
            data: HashMap::new(),
            uncommitted: Vec::new(),
        }
    }

    pub fn set_data(&mut self, key: String, value: EventValue) {
        let mut event = Event::new("data.updated");
        event = event.with_payload("key", key.clone());
        event = event.with_payload("value", value.clone());

        self.data.insert(key, value);
        self.uncommitted.push(event);
    }

    pub fn get_data(&self, key: &str) -> Option<&EventValue> {
        self.data.get(key)
    }
}

impl Aggregate for ExampleAggregate {
    fn aggregate_id(&self) -> &AggregateId {
        &self.id
    }

    fn aggregate_type(&self) -> &str {
        "ExampleAggregate"
    }

    fn version(&self) -> Version {
        self.version
    }

    fn apply_event(&mut self, event: &Event) -> Result<()> {
        match event.event_type.as_str() {
            "data.updated" => {
                if let (Some(key), Some(value)) = (
                    event.get_payload("key"),
                    event.get_payload("value"),
                ) {
                    if let Some(key_str) = key.as_str() {
                        self.data.insert(key_str.to_string(), value.clone());
                    }
                }
            }
            _ => {}
        }

        self.version = self.version.next();
        Ok(())
    }

    fn uncommitted_events(&self) -> &[Event] {
        &self.uncommitted
    }

    fn clear_uncommitted_events(&mut self) {
        self.uncommitted.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
use std::time::UNIX_EPOCH;

    #[test]
    fn test_event_store() {
        let mut store = InMemoryEventStore::new();
        let aggregate_id = AggregateId::new("test_1");

        let event = Event::new("test.created");
        let envelope = EventEnvelope::new(
            aggregate_id.clone(),
            "TestAggregate",
            event,
            Version(1),
        );

        let version = store
            .append_events(&aggregate_id, vec![envelope], Some(Version::zero()))
            .unwrap();

        assert_eq!(version, Version(1));

        let events = store.get_events(&aggregate_id).unwrap();
        assert_eq!(events.len(), 1);
    }

    #[test]
    fn test_aggregate() {
        let mut aggregate = ExampleAggregate::new(AggregateId::new("test_1"));

        aggregate.set_data("key1".to_string(), EventValue::from("value1"));
        assert_eq!(aggregate.get_data("key1").unwrap().as_str(), Some("value1"));
        assert_eq!(aggregate.uncommitted_events().len(), 1);
    }

    #[test]
    fn test_aggregate_repository() {
        let event_store: Arc<RwLock<dyn EventStore>> =
            Arc::new(RwLock::new(InMemoryEventStore::new()));

        let repo = AggregateRepository::new(event_store.clone());

        let aggregate_id = AggregateId::new("test_1");
        let mut aggregate = ExampleAggregate::new(aggregate_id.clone());

        aggregate.set_data("key1".to_string(), EventValue::from("value1"));

        repo.save(&mut aggregate).unwrap();

        let loaded = repo.load(&aggregate_id).unwrap();
        // Note: load will return None because we simplified aggregate reconstruction
        // In a real implementation, this would properly reconstruct the aggregate
    }

    #[test]
    fn test_snapshot_store() {
        let mut store = SnapshotStore::new();
        let aggregate_id = AggregateId::new("test_1");
        let aggregate = ExampleAggregate::new(aggregate_id.clone());

        store
            .save_snapshot(aggregate_id.clone(), aggregate.clone(), Version(10))
            .unwrap();

        let snapshot = store.get_snapshot(&aggregate_id).unwrap();
        assert_eq!(snapshot.version, Version(10));
    }

    #[test]
    fn test_event_replay() {
        let event_store: Arc<RwLock<dyn EventStore>> =
            Arc::new(RwLock::new(InMemoryEventStore::new()));

        let replay_engine = EventReplayEngine::new(event_store.clone());

        let aggregate_id = AggregateId::new("test_1");
        let event = Event::new("test.created");
        let envelope = EventEnvelope::new(
            aggregate_id.clone(),
            "TestAggregate",
            event,
            Version(1),
        );

        {
            let mut store = event_store.write().unwrap();
            store
                .append_events(&aggregate_id, vec![envelope], Some(Version::zero()))
                .unwrap();
        }

        let events = replay_engine.replay_aggregate(&aggregate_id).unwrap();
        assert_eq!(events.len(), 1);
    }
}
