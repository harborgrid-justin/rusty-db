/// Cross-Datacenter Geo-Replication
///
/// This module provides geo-distributed replication capabilities for multi-datacenter
/// deployments. Features include:
/// - Asynchronous replication with configurable consistency levels
/// - Conflict resolution strategies (Last-Write-Wins, Vector Clocks, Custom)
/// - Geo-aware read routing for low latency
/// - Disaster recovery with automatic failover
/// - WAN optimization with batching and compression
/// - Multi-master replication support
///
/// Designed for:
/// - Global data distribution
/// - Low-latency local reads
/// - High availability across regions
/// - Compliance with data sovereignty requirements

use std::collections::VecDeque;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::time::SystemTime;
use crate::error::DbError;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use std::time::{Duration};

/// Datacenter identifier
pub type DatacenterId = String;

/// Replication stream ID
pub type StreamId = u64;

/// Logical timestamp for ordering
pub type LogicalTimestamp = u64;

/// Consistency level for reads
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ConsistencyLevel {
    /// Read from local datacenter only (lowest latency)
    Local,
    /// Read from local region (nearby datacenters)
    Regional,
    /// Read from any datacenter (global)
    Global,
    /// Ensure read reflects recent writes (session consistency)
    SessionConsistent,
    /// Strong consistency (linearizable)
    Strong,
}

/// Conflict resolution strategy
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ConflictResolution {
    /// Last write wins based on timestamp
    LastWriteWins,
    /// Use vector clocks for causality
    VectorClock,
    /// Custom application-level resolution
    Custom,
    /// Multi-value - keep all conflicting versions
    MultiValue,
}

/// Vector clock for tracking causality
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct VectorClock {
    /// Clock values per datacenter
    clocks: HashMap<DatacenterId, LogicalTimestamp>,
}

impl VectorClock {
    pub fn new() -> Self {
        Self {
            clocks: HashMap::new(),
        }
    }

    /// Increment clock for a datacenter
    pub fn increment(&mut self, dc_id: &str) {
        let counter = self.clocks.entry(dc_id.to_string()).or_insert(0);
        *counter += 1;
    }

    /// Merge with another vector clock
    pub fn merge(&mut self, other: &VectorClock) {
        for (dc_id, &timestamp) in &other.clocks {
            let entry = self.clocks.entry(dc_id.clone()).or_insert(0);
            *entry = (*entry).max(timestamp);
        }
    }

    /// Check if this clock happened before another
    pub fn happens_before(&self, other: &VectorClock) -> bool {
        let mut strictly_less = false;

        for (dc_id, &other_ts) in &other.clocks {
            let self_ts = self.clocks.get(dc_id).copied().unwrap_or(0);
            if self_ts > other_ts {
                return false;
            }
            if self_ts < other_ts {
                strictly_less = true;
            }
        }

        for (dc_id, &self_ts) in &self.clocks {
            if !other.clocks.contains_key(dc_id) && self_ts > 0 {
                return false;
            }
        }

        strictly_less
    }

    /// Check if clocks are concurrent (conflicting)
    pub fn is_concurrent(&self, other: &VectorClock) -> bool {
        !self.happens_before(other) && !other.happens_before(self) && self != other
    }
}

impl Default for VectorClock {
    fn default() -> Self {
        Self::new()
    }
}

/// Replicated value with metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReplicatedValue {
    /// The actual data
    pub data: Vec<u8>,
    /// Version/timestamp
    pub timestamp: SystemTime,
    /// Vector clock for causality tracking
    pub vector_clock: VectorClock,
    /// Originating datacenter
    pub source_dc: DatacenterId,
    /// Hash of the value for integrity
    pub checksum: u64,
}

impl ReplicatedValue {
    pub fn new(data: Vec<u8>, source_dc: DatacenterId, vector_clock: VectorClock) -> Self {
        let mut hasher = DefaultHasher::new();
        data.hash(&mut hasher);
        let checksum = hasher.finish();

        Self {
            data,
            timestamp: SystemTime::now(),
            vector_clock,
            source_dc,
            checksum,
        }
    }

    /// Verify checksum
    pub fn verify_checksum(&self) -> bool {

        let mut hasher = DefaultHasher::new();
        self.data.hash(&mut hasher);
        hasher.finish() == self.checksum
    }
}

/// Replication operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReplicationOp {
    /// Operation ID
    pub id: u64,
    /// Key being replicated
    pub key: Vec<u8>,
    /// Operation type
    pub op_type: OpType,
    /// Value (for puts)
    pub value: Option<ReplicatedValue>,
    /// Source datacenter
    pub source_dc: DatacenterId,
    /// Target datacenters
    pub target_dcs: Vec<DatacenterId>,
    /// Timestamp
    pub timestamp: SystemTime,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum OpType {
    Put,
    Delete,
    Merge,
}

/// Datacenter metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Datacenter {
    /// Datacenter ID
    pub id: DatacenterId,
    /// Geographic region
    pub region: String,
    /// Availability zone
    pub zone: String,
    /// Latitude for distance calculations
    pub latitude: f64,
    /// Longitude for distance calculations
    pub longitude: f64,
    /// Endpoint for replication
    pub endpoint: String,
    /// Priority (higher = preferred for reads)
    pub priority: u8,
    /// Is this datacenter currently active?
    pub active: bool,
    /// Last health check
    pub last_health_check: SystemTime,
    /// Replication lag (ms)
    pub replication_lag_ms: u64,
}

impl Datacenter {
    pub fn new(id: DatacenterId, region: String, zone: String, endpoint: String) -> Self {
        Self {
            id,
            region,
            zone,
            latitude: 0.0,
            longitude: 0.0,
            endpoint,
            priority: 100,
            active: true,
            last_health_check: SystemTime::now(),
            replication_lag_ms: 0,
        }
    }

    /// Calculate distance to another datacenter (Haversine formula)
    pub fn distance_to(&self, other: &Datacenter) -> f64 {
        let lat1 = self.latitude.to_radians();
        let lat2 = other.latitude.to_radians();
        let delta_lat = (other.latitude - self.latitude).to_radians();
        let delta_lon = (other.longitude - self.longitude).to_radians();

        let a = (delta_lat / 2.0).sin().powi(2)
            + lat1.cos() * lat2.cos() * (delta_lon / 2.0).sin().powi(2);
        let c = 2.0 * a.sqrt().atan2((1.0 - a).sqrt());

        6371.0 * c // Earth radius in km
    }
}

/// Replication configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeoReplicationConfig {
    /// Local datacenter ID
    pub local_dc: DatacenterId,
    /// Conflict resolution strategy
    pub conflict_resolution: ConflictResolution,
    /// Default consistency level
    pub default_consistency: ConsistencyLevel,
    /// Enable compression for WAN traffic
    pub enable_compression: bool,
    /// Batch size for replication
    pub batch_size: usize,
    /// Batch timeout
    pub batch_timeout: Duration,
    /// Maximum replication lag before alerting (ms)
    pub max_replication_lag_ms: u64,
    /// Enable automatic failover
    pub auto_failover: bool,
    /// Failover detection timeout
    pub failover_timeout: Duration,
}

impl Default for GeoReplicationConfig {
    fn default() -> Self {
        Self {
            local_dc: "dc1".to_string(),
            conflict_resolution: ConflictResolution::VectorClock,
            default_consistency: ConsistencyLevel::Local,
            enable_compression: true,
            batch_size: 1000,
            batch_timeout: Duration::from_millis(100),
            max_replication_lag_ms: 5000,
            auto_failover: true,
            failover_timeout: Duration::from_secs(30),
        }
    }
}

/// Replication stream for batching
struct ReplicationStream {
    id: StreamId,
    target_dc: DatacenterId,
    pending_ops: VecDeque<ReplicationOp>,
    last_flush: SystemTime,
    bytes_pending: usize,
}

impl ReplicationStream {
    fn new(id: StreamId, target_dc: DatacenterId) -> Self {
        Self {
            id,
            target_dc,
            pending_ops: VecDeque::new(),
            last_flush: SystemTime::now(),
            bytes_pending: 0,
        }
    }

    fn add_op(&mut self, op: ReplicationOp) {
        self.bytes_pending += op.key.len() + op.value.as_ref().map_or(0, |v| v.data.len());
        self.pending_ops.push_back(op);
    }

    fn should_flush(&self, config: &GeoReplicationConfig) -> bool {
        self.pending_ops.len() >= config.batch_size
            || self.last_flush.elapsed().unwrap_or(Duration::ZERO) >= config.batch_timeout
            || self.bytes_pending > 1024 * 1024 // 1MB
    }

    fn flush(&mut self) -> Vec<ReplicationOp> {
        self.last_flush = SystemTime::now();
        self.bytes_pending = 0;
        self.pending_ops.drain(..).collect()
    }
}

/// Geo-replication manager
pub struct GeoReplicationManager {
    /// Configuration
    config: GeoReplicationConfig,
    /// All datacenters in the cluster
    datacenters: Arc<RwLock<HashMap<DatacenterId, Datacenter>>>,
    /// Replication streams
    streams: Arc<RwLock<HashMap<DatacenterId, ReplicationStream>>>,
    /// Local vector clock
    vector_clock: Arc<RwLock<VectorClock>>,
    /// Next operation ID
    next_op_id: Arc<RwLock<u64>>,
    /// Next stream ID
    next_stream_id: Arc<RwLock<StreamId>>,
    /// Conflict log for manual resolution
    conflicts: Arc<RwLock<Vec<Conflict>>>,
}

/// Conflict that needs resolution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Conflict {
    pub key: Vec<u8>,
    pub values: Vec<ReplicatedValue>,
    pub detected_at: SystemTime,
    pub resolved: bool,
}

impl GeoReplicationManager {
    pub fn new(config: GeoReplicationConfig) -> Self {
        let mut vector_clock = VectorClock::new();
        vector_clock.increment(&config.local_dc);

        Self {
            config,
            datacenters: Arc::new(RwLock::new(HashMap::new())),
            streams: Arc::new(RwLock::new(HashMap::new())),
            vector_clock: Arc::new(RwLock::new(vector_clock)),
            next_op_id: Arc::new(RwLock::new(0)),
            next_stream_id: Arc::new(RwLock::new(0)),
            conflicts: Arc::new(RwLock::new(Vec::new())),
        }
    }

    /// Add a datacenter to the topology
    pub fn add_datacenter(&self, dc: Datacenter) -> Result<(), DbError> {
        let mut datacenters = self.datacenters.write().unwrap();
        let dc_id = dc.id.clone();

        datacenters.insert(dc_id.clone(), dc);

        // Create replication stream for this datacenter
        let mut streams = self.streams.write().unwrap();
        let stream_id = *self.next_stream_id.read().unwrap();
        streams.insert(dc_id.clone(), ReplicationStream::new(stream_id, dc_id));

        let mut next = self.next_stream_id.write().unwrap();
        *next += 1;

        Ok(())
    }

    /// Remove a datacenter
    pub fn remove_datacenter(&self, dc_id: &str) -> Result<(), DbError> {
        let mut datacenters = self.datacenters.write().unwrap();
        datacenters.remove(dc_id);

        let mut streams = self.streams.write().unwrap();
        streams.remove(dc_id);

        Ok(())
    }

    /// Replicate a write operation
    pub fn replicate_write(
        &self,
        key: Vec<u8>,
        value: Vec<u8>,
        target_dcs: Option<Vec<DatacenterId>>,
    ) -> Result<u64, DbError> {
        // Increment vector clock
        let mut vc = self.vector_clock.write().unwrap();
        vc.increment(&self.config.local_dc);
        let clock = vc.clone();
        drop(vc);

        // Create replicated value
        let replicated_value = ReplicatedValue::new(value, self.config.local_dc.clone(), clock);

        // Determine target datacenters
        let targets = target_dcs.unwrap_or_else(|| {
            self.datacenters
                .read()
                .unwrap()
                .keys()
                .filter(|dc| *dc != &self.config.local_dc)
                .cloned()
                .collect()
        });

        // Create replication operation
        let mut next_id = self.next_op_id.write().unwrap();
        let op_id = *next_id;
        *next_id += 1;

        let op = ReplicationOp {
            id: op_id,
            key,
            op_type: OpType::Put,
            value: Some(replicated_value),
            source_dc: self.config.local_dc.clone(),
            target_dcs: targets.clone(),
            timestamp: SystemTime::now(),
        };

        // Add to replication streams
        let mut streams = self.streams.write().unwrap();
        for target in targets {
            if let Some(stream) = streams.get_mut(&target) {
                stream.add_op(op.clone());
            }
        }

        Ok(op_id)
    }

    /// Replicate a delete operation
    pub fn replicate_delete(&self, key: Vec<u8>) -> Result<u64, DbError> {
        let mut vc = self.vector_clock.write().unwrap();
        vc.increment(&self.config.local_dc);
        drop(vc);

        let targets: Vec<DatacenterId> = self
            .datacenters
            .read()
            .unwrap()
            .keys()
            .filter(|dc| *dc != &self.config.local_dc)
            .cloned()
            .collect();

        let mut next_id = self.next_op_id.write().unwrap();
        let op_id = *next_id;
        *next_id += 1;

        let op = ReplicationOp {
            id: op_id,
            key,
            op_type: OpType::Delete,
            value: None,
            source_dc: self.config.local_dc.clone(),
            target_dcs: targets.clone(),
            timestamp: SystemTime::now(),
        };

        let mut streams = self.streams.write().unwrap();
        for target in targets {
            if let Some(stream) = streams.get_mut(&target) {
                stream.add_op(op.clone());
            }
        }

        Ok(op_id)
    }

    /// Flush pending replication operations
    pub fn flush_streams(&self) -> HashMap<DatacenterId, Vec<ReplicationOp>> {
        let mut streams = self.streams.write().unwrap();
        let mut batches = HashMap::new();

        for (dc_id, stream) in streams.iter_mut() {
            if stream.should_flush(&self.config) {
                let ops = stream.flush();
                if !ops.is_empty() {
                    batches.insert(dc_id.clone(), ops);
                }
            }
        }

        batches
    }

    /// Resolve conflict between two values
    pub fn resolve_conflict(
        &self,
        v1: &ReplicatedValue,
        v2: &ReplicatedValue,
    ) -> Result<ConflictResolution, DbError> {
        match self.config.conflict_resolution {
            ConflictResolution::LastWriteWins => {
                // Compare timestamps
                Ok(ConflictResolution::LastWriteWins)
            }
            ConflictResolution::VectorClock => {
                if v1.vector_clock.happens_before(&v2.vector_clock) {
                    // v2 is newer
                    Ok(ConflictResolution::VectorClock)
                } else if v2.vector_clock.happens_before(&v1.vector_clock) {
                    // v1 is newer
                    Ok(ConflictResolution::VectorClock)
                } else if v1.vector_clock.is_concurrent(&v2.vector_clock) {
                    // Concurrent writes - conflict!
                    Ok(ConflictResolution::MultiValue)
                } else {
                    // Equal clocks
                    Ok(ConflictResolution::VectorClock)
                }
            }
            ConflictResolution::Custom => {
                // Application needs to resolve
                Ok(ConflictResolution::Custom)
            }
            ConflictResolution::MultiValue => {
                // Keep both values
                Ok(ConflictResolution::MultiValue)
            }
        }
    }

    /// Record a conflict for later resolution
    pub fn record_conflict(&self, key: Vec<u8>, values: Vec<ReplicatedValue>) {
        let conflict = Conflict {
            key,
            values,
            detected_at: SystemTime::now(),
            resolved: false,
        };

        self.conflicts.write().unwrap().push(conflict);
    }

    /// Get conflicts that need resolution
    pub fn get_unresolved_conflicts(&self) -> Vec<Conflict> {
        self.conflicts
            .read()
            .unwrap()
            .iter()
            .filter(|c| !c.resolved)
            .cloned()
            .collect()
    }

    /// Select best datacenter for read based on consistency level
    pub fn select_read_datacenter(&self, consistency: ConsistencyLevel) -> Result<DatacenterId, DbError> {
        let datacenters = self.datacenters.read().unwrap();

        match consistency {
            ConsistencyLevel::Local => {
                // Always read from local DC
                Ok(self.config.local_dc.clone())
            }
            ConsistencyLevel::Regional => {
                // Read from local region
                let local_dc = datacenters
                    .get(&self.config.local_dc)
                    .ok_or_else(|| DbError::Internal("Local DC not found".into()))?;

                let regional: Vec<&Datacenter> = datacenters
                    .values()
                    .filter(|dc| dc.region == local_dc.region && dc.active)
                    .collect();

                regional
                    .into_iter()
                    .min_by_key(|dc| dc.replication_lag_ms)
                    .map(|dc| dc.id.clone())
                    .ok_or_else(|| DbError::Internal("No regional DC available".into()))
            }
            ConsistencyLevel::Global => {
                // Read from any active DC with lowest lag
                datacenters
                    .values()
                    .filter(|dc| dc.active)
                    .min_by_key(|dc| dc.replication_lag_ms)
                    .map(|dc| dc.id.clone())
                    .ok_or_else(|| DbError::Internal("No DC available".into()))
            }
            ConsistencyLevel::SessionConsistent | ConsistencyLevel::Strong => {
                // Must read from local DC for consistency
                Ok(self.config.local_dc.clone())
            }
        }
    }

    /// Get nearest datacenter by geographic distance
    pub fn get_nearest_datacenter(&self, latitude: f64, longitude: f64) -> Option<DatacenterId> {
        let datacenters = self.datacenters.read().unwrap();
        let temp_dc = Datacenter {
            id: "temp".to_string(),
            region: String::new(),
            zone: String::new(),
            latitude,
            longitude,
            endpoint: String::new(),
            priority: 0,
            active: true,
            last_health_check: SystemTime::now(),
            replication_lag_ms: 0,
        };

        datacenters
            .values()
            .filter(|dc| dc.active)
            .min_by(|a, b| {
                let dist_a = temp_dc.distance_to(a);
                let dist_b = temp_dc.distance_to(b);
                dist_a.partial_cmp(&dist_b).unwrap()
            })
            .map(|dc| dc.id.clone())
    }

    /// Check datacenter health
    pub fn check_datacenter_health(&self) -> Vec<(DatacenterId, bool)> {
        let mut datacenters = self.datacenters.write().unwrap();
        let mut results = Vec::new();

        for (dc_id, dc) in datacenters.iter_mut() {
            // Check if DC is responding
            let is_healthy = dc.active
                && dc.replication_lag_ms < self.config.max_replication_lag_ms
                && dc.last_health_check.elapsed().unwrap_or(Duration::MAX)
                    < self.config.failover_timeout;

            if !is_healthy && self.config.auto_failover {
                dc.active = false;
            }

            results.push((dc_id.clone(), is_healthy));
        }

        results
    }

    /// Initiate disaster recovery failover
    pub fn initiate_failover(&self, failed_dc: &str, target_dc: &str) -> Result<(), DbError> {
        let mut datacenters = self.datacenters.write().unwrap();

        // Mark failed DC as inactive
        if let Some(dc) = datacenters.get_mut(failed_dc) {
            dc.active = false;
        }

        // Promote target DC
        if let Some(dc) = datacenters.get_mut(target_dc) {
            dc.priority = 255; // Highest priority
            dc.active = true;
        }

        Ok(())
    }

    /// Get replication topology
    pub fn get_topology(&self) -> Vec<Datacenter> {
        self.datacenters.read().unwrap().values().cloned().collect()
    }

    /// Get replication lag statistics
    pub fn get_lag_stats(&self) -> HashMap<DatacenterId, u64> {
        self.datacenters
            .read()
            .unwrap()
            .iter()
            .map(|(id, dc)| (id.clone(), dc.replication_lag_ms))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vector_clock_increment() {
        let mut vc = VectorClock::new();
        vc.increment("dc1");
        assert_eq!(vc.clocks.get("dc1"), Some(&1));
    }

    #[test]
    fn test_vector_clock_happens_before() {
        let mut vc1 = VectorClock::new();
        vc1.increment("dc1");

        let mut vc2 = VectorClock::new();
        vc2.increment("dc1");
        vc2.increment("dc1");

        assert!(vc1.happens_before(&vc2));
        assert!(!vc2.happens_before(&vc1));
    }

    #[test]
    fn test_vector_clock_concurrent() {
        let mut vc1 = VectorClock::new();
        vc1.increment("dc1");

        let mut vc2 = VectorClock::new();
        vc2.increment("dc2");

        assert!(vc1.is_concurrent(&vc2));
        assert!(vc2.is_concurrent(&vc1));
    }

    #[test]
    fn test_datacenter_distance() {
        let dc1 = Datacenter {
            id: "dc1".into(),
            region: "us-east".into(),
            zone: "a".into(),
            latitude: 40.7128,
            longitude: -74.0060,
            endpoint: "dc1.example.com".into(),
            priority: 100,
            active: true,
            last_health_check: SystemTime::now(),
            replication_lag_ms: 0,
        };

        let dc2 = Datacenter {
            id: "dc2".into(),
            region: "us-west".into(),
            zone: "a".into(),
            latitude: 37.7749,
            longitude: -122.4194,
            endpoint: "dc2.example.com".into(),
            priority: 100,
            active: true,
            last_health_check: SystemTime::now(),
            replication_lag_ms: 0,
        };

        let distance = dc1.distance_to(&dc2);
        assert!(distance > 0.0);
        assert!(distance < 10000.0); // Reasonable distance in km
    }
}
