// # Time Travel Engine
//
// Oracle-like time travel queries with AS OF TIMESTAMP and AS OF SCN support.
// Provides temporal query capabilities for accessing historical database states.
//
// ## Features
//
// - AS OF TIMESTAMP queries
// - AS OF SCN (System Change Number) queries
// - Version chain management
// - Temporal indexes for fast historical access
// - Time-bounded range queries
// - Bi-temporal data support (valid time and transaction time)
// - Historical data optimization and caching
//
// ## Example
//
// ```sql
// SELECT * FROM employees AS OF TIMESTAMP '2024-01-01 12:00:00';
// SELECT * FROM accounts AS OF SCN 12345;
// ```

use std::collections::BTreeMap;
use std::collections::{HashMap};
use std::sync::{Arc, RwLock};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::common::{TransactionId, TableId, RowId, Value};
use crate::error::{Result, DbError};

// ============================================================================
// Type Aliases
// ============================================================================

/// System Change Number - monotonically increasing transaction commit number
pub type SCN = u64;

/// Timestamp in microseconds since Unix epoch
pub type Timestamp = i64;

/// Version identifier (SCN-based)
pub type VersionId = u64;

// ============================================================================
// Time Travel Engine
// ============================================================================

/// Main time travel engine for temporal queries
///
/// Coordinates temporal query execution and manages historical data access.
pub struct TimeTravelEngine {
    /// SCN to timestamp mapping
    scn_timeline: Arc<RwLock<ScnTimeline>>,

    /// Version chain index for fast lookup
    version_index: Arc<RwLock<VersionIndex>>,

    /// Temporal query cache
    query_cache: Arc<RwLock<TemporalQueryCache>>,

    /// Configuration
    #[allow(dead_code)]
    config: TimeTravelConfig,

    /// Statistics
    stats: Arc<RwLock<TimeTravelStats>>,
}

impl TimeTravelEngine {
    /// Create a new time travel engine
    pub fn new(config: TimeTravelConfig) -> Self {
        Self {
            scn_timeline: Arc::new(RwLock::new(ScnTimeline::new())),
            version_index: Arc::new(RwLock::new(VersionIndex::new())),
            query_cache: Arc::new(RwLock::new(TemporalQueryCache::new(config.cache_size))),
            config,
            stats: Arc::new(RwLock::new(TimeTravelStats::default())),
        }
    }

    /// Execute AS OF TIMESTAMP query
    pub fn query_as_of_timestamp(
        &self,
        table_id: TableId,
        timestamp: Timestamp,
        predicate: Option<TemporalPredicate>,
    ) -> Result<Vec<HistoricalRow>> {
        // Convert timestamp to SCN
        let scn = self.timestamp_to_scn(timestamp)?;

        // Delegate to SCN-based query
        self.query_as_of_scn(table_id, scn, predicate)
    }

    /// Execute AS OF SCN query
    pub fn query_as_of_scn(
        &self,
        table_id: TableId,
        scn: SCN,
        predicate: Option<TemporalPredicate>,
    ) -> Result<Vec<HistoricalRow>> {
        let mut stats = self.stats.write().unwrap();
        stats.queries_executed += 1;

        // Check cache first
        let cache_key = TemporalQueryKey {
            table_id,
            scn,
            predicate: predicate.clone(),
        };

        if let Some(cached) = self.query_cache.read().unwrap().get(&cache_key) {
            stats.cache_hits += 1;
            return Ok(cached.clone());
        }

        // Execute query
        let start = SystemTime::now();
        let results = self.execute_temporal_query(table_id, scn, predicate)?;

        let elapsed = start.elapsed().unwrap_or_default();
        stats.total_query_time_ms += elapsed.as_millis() as u64;

        // Cache results
        self.query_cache.write().unwrap().insert(cache_key, results.clone());

        Ok(results)
    }

    /// Execute VERSIONS BETWEEN query for a time range
    pub fn query_versions_between(
        &self,
        table_id: TableId,
        start_scn: SCN,
        end_scn: SCN,
        row_id: Option<RowId>,
    ) -> Result<Vec<VersionChain>> {
        let version_index = self.version_index.read().unwrap();

        let mut results = Vec::new();

        if let Some(rid) = row_id {
            // Single row version history
            if let Some(chain) = version_index.get_version_chain(table_id, rid) {
                let filtered = chain.filter_by_scn_range(start_scn, end_scn);
                results.push(filtered);
            }
        } else {
            // All rows version history
            for chain in version_index.get_table_versions(table_id) {
                let filtered = chain.filter_by_scn_range(start_scn, end_scn);
                if !filtered.versions.is_empty() {
                    results.push(filtered);
                }
            }
        }

        Ok(results)
    }

    /// Execute bi-temporal query (transaction time and valid time)
    pub fn query_bitemporal(
        &self,
        table_id: TableId,
        transactiontime: Timestamp,
        validtime: Timestamp,
    ) -> Result<Vec<BiTemporalRow>> {
        // First, find rows valid at the transaction time
        let scn = self.timestamp_to_scn(transactiontime)?;
        let rows = self.query_as_of_scn(table_id, scn, None)?;

        // Then filter by valid time
        let mut results = Vec::new();
        for row in rows {
            if let Some(bitemporal) = row.bitemporal_metadata {
                if bitemporal.is_valid_at(validtime) {
                    results.push(BiTemporalRow {
                        row_id: row.row_id,
                        values: row.values,
                        transaction_time: row.scn_created,
                        valid_time_start: bitemporal.valid_from,
                        valid_time_end: bitemporal.valid_to,
                    });
                }
            }
        }

        Ok(results)
    }

    /// Convert timestamp to SCN
    pub fn timestamp_to_scn(&self, timestamp: Timestamp) -> Result<SCN> {
        let timeline = self.scn_timeline.read().unwrap();
        timeline.timestamp_to_scn(timestamp)
    }

    /// Convert SCN to timestamp
    pub fn scn_to_timestamp(&self, scn: SCN) -> Result<Timestamp> {
        let timeline = self.scn_timeline.read().unwrap();
        timeline.scn_to_timestamp(scn)
    }

    /// Register a new SCN with timestamp
    pub fn register_scn(&self, scn: SCN, timestamp: Timestamp) -> Result<()> {
        let mut timeline = self.scn_timeline.write().unwrap();
        timeline.register(scn, timestamp)
    }

    /// Add a row version to the index
    pub fn index_version(
        &self,
        table_id: TableId,
        row_id: RowId,
        version: RowVersion,
    ) -> Result<()> {
        let mut index = self.version_index.write().unwrap();
        index.add_version(table_id, row_id, version)
    }

    /// Get current SCN
    pub fn get_current_scn(&self) -> SCN {
        let timeline = self.scn_timeline.read().unwrap();
        timeline.get_latest_scn()
    }

    /// Execute temporal query against version index
    fn execute_temporal_query(
        &self,
        table_id: TableId,
        scn: SCN,
        predicate: Option<TemporalPredicate>,
    ) -> Result<Vec<HistoricalRow>> {
        let version_index = self.version_index.read().unwrap();
        let mut results = Vec::new();

        for chain in version_index.get_table_versions(table_id) {
            if let Some(version) = chain.get_version_at_scn(scn) {
                // Apply predicate if present
                if let Some(ref pred) = predicate {
                    if !pred.evaluate(&version.values) {
                        continue;
                    }
                }

                results.push(HistoricalRow {
                    row_id: chain.row_id,
                    values: version.values.clone(),
                    scn_created: version.scn_created,
                    scn_deleted: version.scn_deleted,
                    txn_id: version.txn_id,
                    bitemporal_metadata: version.bitemporal.clone(),
                });
            }
        }

        Ok(results)
    }

    /// Get statistics
    pub fn get_stats(&self) -> TimeTravelStats {
        self.stats.read().unwrap().clone()
    }

    /// Clear cache
    pub fn clear_cache(&self) {
        self.query_cache.write().unwrap().clear();
    }

    /// Optimize version chains (remove unnecessary intermediate versions)
    pub fn optimize_version_chains(&self, table_id: TableId) -> Result<usize> {
        let mut index = self.version_index.write().unwrap();
        index.compact_table_versions(table_id)
    }
}

// ============================================================================
// SCN Timeline Management
// ============================================================================

/// Maps SCN to timestamps and vice versa
struct ScnTimeline {
    /// SCN to timestamp mapping (ordered)
    scn_to_time: BTreeMap<SCN, Timestamp>,

    /// Timestamp to SCN mapping (approximate)
    time_to_scn: BTreeMap<Timestamp, SCN>,

    /// Latest SCN
    latest_scn: SCN,
}

impl ScnTimeline {
    fn new() -> Self {
        Self {
            scn_to_time: BTreeMap::new(),
            time_to_scn: BTreeMap::new(),
            latest_scn: 0,
        }
    }

    fn register(&mut self, scn: SCN, timestamp: Timestamp) -> Result<()> {
        self.scn_to_time.insert(scn, timestamp);
        self.time_to_scn.insert(timestamp, scn);

        if scn > self.latest_scn {
            self.latest_scn = scn;
        }

        Ok(())
    }

    fn scn_to_timestamp(&self, scn: SCN) -> Result<Timestamp> {
        self.scn_to_time
            .get(&scn)
            .copied()
            .ok_or_else(|| DbError::Validation(format!("Unknown SCN: {}", scn)))
    }

    fn timestamp_to_scn(&self, timestamp: Timestamp) -> Result<SCN> {
        // Find the largest SCN that is <= timestamp
        self.time_to_scn
            .range(..=timestamp)
            .next_back()
            .map(|(_, &scn)| scn)
            .ok_or_else(|| DbError::Validation(format!("No SCN found for timestamp: {}", timestamp)))
    }

    fn get_latest_scn(&self) -> SCN {
        self.latest_scn
    }
}

// ============================================================================
// Version Index
// ============================================================================

/// Index for fast version chain lookup
struct VersionIndex {
    /// Table -> Row -> Version Chain
    chains: HashMap<TableId, HashMap<RowId, VersionChain>>,
}

impl VersionIndex {
    fn new() -> Self {
        Self {
            chains: HashMap::new(),
        }
    }

    fn add_version(
        &mut self,
        table_id: TableId,
        row_id: RowId,
        version: RowVersion,
    ) -> Result<()> {
        let table_chains = self.chains.entry(table_id).or_insert_with(HashMap::new);
        let chain = table_chains.entry(row_id).or_insert_with(|| VersionChain::new(row_id));
        chain.add_version(version);
        Ok(())
    }

    fn get_version_chain(&self, table_id: TableId, row_id: RowId) -> Option<&VersionChain> {
        self.chains.get(&table_id)?.get(&row_id)
    }

    fn get_table_versions(&self, table_id: TableId) -> Vec<&VersionChain> {
        self.chains
            .get(&table_id)
            .map(|chains| chains.values().collect())
            .unwrap_or_default()
    }

    fn compact_table_versions(&mut self, table_id: TableId) -> Result<usize> {
        let mut removed = 0;

        if let Some(table_chains) = self.chains.get_mut(&table_id) {
            for chain in table_chains.values_mut() {
                removed += chain.compact();
            }
        }

        Ok(removed)
    }
}

// ============================================================================
// Version Chain
// ============================================================================

/// Represents all versions of a single row over time
#[repr(C)]
#[derive(Debug, Clone)]
pub struct VersionChain {
    /// Row identifier
    pub row_id: RowId,

    /// Versions in chronological order (oldest to newest)
    pub versions: Vec<RowVersion>,
}

impl VersionChain {
    fn new(row_id: RowId) -> Self {
        Self {
            row_id,
            versions: Vec::new(),
        }
    }

    fn add_version(&mut self, version: RowVersion) {
        // Insert in SCN order
        let pos = self.versions
            .binary_search_by_key(&version.scn_created, |v| v.scn_created)
            .unwrap_or_else(|e| e);
        self.versions.insert(pos, version);
    }

    /// Get the version visible at a specific SCN
    #[inline]
    fn get_version_at_scn(&self, scn: SCN) -> Option<&RowVersion> {
        // Find the latest version created before or at SCN
        // that was not deleted before SCN
        // SAFETY: Using iterator in reverse is safe and avoids bounds checks
        self.versions
            .iter()
            .rev()
            .find(|v| {
                v.scn_created <= scn &&
                v.scn_deleted.map(|del| del > scn).unwrap_or(true)
            })
    }

    /// Filter versions by SCN range
    fn filter_by_scn_range(&self, start_scn: SCN, end_scn: SCN) -> VersionChain {
        let filtered_versions: Vec<RowVersion> = self.versions
            .iter()
            .filter(|v| {
                v.scn_created >= start_scn && v.scn_created <= end_scn
            })
            .cloned()
            .collect();

        VersionChain {
            row_id: self.row_id,
            versions: filtered_versions,
        }
    }

    /// Compact by removing intermediate versions that are fully obsolete
    fn compact(&mut self) -> usize {
        // Keep only versions that are:
        // 1. Still active (scn_deleted is None)
        // 2. The last version before a deletion
        let original_count = self.versions.len();

        let mut keep_indices = Vec::new();
        for (i, version) in self.versions.iter().enumerate() {
            let should_keep = version.scn_deleted.is_none() ||
                             i == self.versions.len() - 1 ||
                             self.versions[i + 1].scn_created != version.scn_deleted.unwrap();

            if should_keep {
                keep_indices.push(i);
            }
        }

        let mut new_versions = Vec::new();
        for idx in keep_indices {
            new_versions.push(self.versions[idx].clone());
        }

        self.versions = new_versions;
        original_count - self.versions.len()
    }
}

// ============================================================================
// Row Version
// ============================================================================

/// A specific version of a row at a point in time
#[repr(C)]
#[derive(Debug, Clone)]
pub struct RowVersion {
    /// Column values
    pub values: Vec<Value>,

    /// SCN when this version was created
    pub scn_created: SCN,

    /// SCN when this version was deleted (None if current)
    pub scn_deleted: Option<SCN>,

    /// Transaction that created this version
    pub txn_id: TransactionId,

    /// Bi-temporal metadata (optional)
    pub bitemporal: Option<BiTemporalMetadata>,
}

// ============================================================================
// Bi-temporal Support
// ============================================================================

/// Bi-temporal metadata for valid time tracking
#[repr(C)]
#[derive(Debug, Clone)]
pub struct BiTemporalMetadata {
    /// Start of valid time period
    pub valid_from: Timestamp,

    /// End of valid time period (None = ongoing)
    pub valid_to: Option<Timestamp>,
}

impl BiTemporalMetadata {
    fn is_valid_at(&self, timestamp: Timestamp) -> bool {
        timestamp >= self.valid_from &&
        self.valid_to.map(|to| timestamp < to).unwrap_or(true)
    }
}

/// Bi-temporal row result
#[derive(Debug, Clone)]
pub struct BiTemporalRow {
    pub row_id: RowId,
    pub values: Vec<Value>,
    pub transaction_time: SCN,
    pub valid_time_start: Timestamp,
    pub valid_time_end: Option<Timestamp>,
}

// ============================================================================
// Historical Row Result
// ============================================================================

/// Result of a temporal query
#[derive(Debug, Clone)]
pub struct HistoricalRow {
    pub row_id: RowId,
    pub values: Vec<Value>,
    pub scn_created: SCN,
    pub scn_deleted: Option<SCN>,
    pub txn_id: TransactionId,
    pub bitemporal_metadata: Option<BiTemporalMetadata>,
}

// ============================================================================
// Temporal Query Cache
// ============================================================================

/// Cache for temporal query results
struct TemporalQueryCache {
    cache: HashMap<TemporalQueryKey, Vec<HistoricalRow>>,
    max_size: usize,
}

impl TemporalQueryCache {
    fn new(max_size: usize) -> Self {
        Self {
            cache: HashMap::new(),
            max_size,
        }
    }

    fn get(&self, key: &TemporalQueryKey) -> Option<&Vec<HistoricalRow>> {
        self.cache.get(key)
    }

    fn insert(&mut self, key: TemporalQueryKey, value: Vec<HistoricalRow>) {
        if self.cache.len() >= self.max_size {
            // Simple eviction: remove first entry
            if let Some(first_key) = self.cache.keys().next().cloned() {
                self.cache.remove(&first_key);
            }
        }
        self.cache.insert(key, value);
    }

    fn clear(&mut self) {
        self.cache.clear();
    }
}

/// Cache key for temporal queries
#[derive(Debug, Clone, Hash, Eq, PartialEq)]
struct TemporalQueryKey {
    table_id: TableId,
    scn: SCN,
    predicate: Option<TemporalPredicate>,
}

// ============================================================================
// Temporal Predicates
// ============================================================================

/// Predicate for filtering temporal query results
#[derive(Debug, Clone, Hash, Eq, PartialEq)]
pub enum TemporalPredicate {
    /// Column equals value
    Equals(usize, Value),

    /// Column greater than value
    GreaterThan(usize, Value),

    /// Column less than value
    LessThan(usize, Value),

    /// AND of predicates
    And(Box<TemporalPredicate>, Box<TemporalPredicate>),

    /// OR of predicates
    Or(Box<TemporalPredicate>, Box<TemporalPredicate>),
}

impl TemporalPredicate {
    fn evaluate(&self, values: &[Value]) -> bool {
        match self {
            TemporalPredicate::Equals(col, val) => {
                values.get(*col).map(|v| v == val).unwrap_or(false)
            }
            TemporalPredicate::GreaterThan(col, val) => {
                values.get(*col).map(|v| v > val).unwrap_or(false)
            }
            TemporalPredicate::LessThan(col, val) => {
                values.get(*col).map(|v| v < val).unwrap_or(false)
            }
            TemporalPredicate::And(left, right) => {
                left.evaluate(values) && right.evaluate(values)
            }
            TemporalPredicate::Or(left, right) => {
                left.evaluate(values) || right.evaluate(values)
            }
        }
    }
}

// ============================================================================
// Configuration
// ============================================================================

/// Time travel engine configuration
#[derive(Debug, Clone)]
pub struct TimeTravelConfig {
    /// Maximum cache size for query results
    pub cache_size: usize,

    /// Enable bi-temporal support
    pub enable_bitemporal: bool,

    /// Automatic version chain compaction threshold
    pub compaction_threshold: usize,

    /// Maximum age of versions to retain (in SCN)
    pub max_version_age: Option<SCN>,
}

impl Default for TimeTravelConfig {
    fn default() -> Self {
        Self {
            cache_size: 1000,
            enable_bitemporal: false,
            compaction_threshold: 100,
            max_version_age: None,
        }
    }
}

// ============================================================================
// Statistics
// ============================================================================

/// Statistics for time travel operations
#[derive(Debug, Clone, Default)]
pub struct TimeTravelStats {
    /// Total queries executed
    pub queries_executed: u64,

    /// Cache hits
    pub cache_hits: u64,

    /// Total query execution time (ms)
    pub total_query_time_ms: u64,

    /// Total versions indexed
    pub versions_indexed: u64,

    /// Compacted versions
    pub versions_compacted: u64,
    pub current_scn: (),
    pub oldest_scn: ()
}

impl TimeTravelStats {
    pub fn cache_hit_rate(&self) -> f64 {
        if self.queries_executed == 0 {
            0.0
        } else {
            (self.cache_hits as f64) / (self.queries_executed as f64)
        }
    }

    pub fn avg_query_time_ms(&self) -> f64 {
        if self.queries_executed == 0 {
            0.0
        } else {
            (self.total_query_time_ms as f64) / (self.queries_executed as f64)
        }
    }
}

// ============================================================================
// Temporal Index Structures
// ============================================================================

/// Temporal B-tree index for efficient historical range queries
pub struct TemporalBTreeIndex {
    /// Index entries by SCN
    entries: BTreeMap<SCN, Vec<TemporalIndexEntry>>,
}

impl TemporalBTreeIndex {
    pub fn new() -> Self {
        Self {
            entries: BTreeMap::new(),
        }
    }

    pub fn insert(&mut self, scn: SCN, entry: TemporalIndexEntry) {
        self.entries.entry(scn).or_insert_with(Vec::new).push(entry);
    }

    pub fn range_query(&self, start_scn: SCN, end_scn: SCN) -> Vec<&TemporalIndexEntry> {
        self.entries
            .range(start_scn..=end_scn)
            .flat_map(|(_, entries)| entries.iter())
            .collect()
    }
}

#[derive(Debug, Clone)]
pub struct TemporalIndexEntry {
    pub row_id: RowId,
    pub scn: SCN,
    pub operation: TemporalOperation,
}

#[derive(Debug, Clone, Copy)]
pub enum TemporalOperation {
    Insert,
    Update,
    Delete,
}

// ============================================================================
// Utility Functions
// ============================================================================

/// Get current timestamp in microseconds
pub fn current_timestamp() -> Timestamp {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_micros() as i64
}

/// Convert system time to timestamp
pub fn system_time_to_timestamp(time: SystemTime) -> Timestamp {
    time.duration_since(UNIX_EPOCH)
        .unwrap()
        .as_micros() as i64
}

/// Convert timestamp to system time
pub fn timestamp_to_system_time(timestamp: Timestamp) -> SystemTime {
    UNIX_EPOCH + std::time::Duration::from_micros(timestamp as u64)
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scn_timeline() {
        let mut timeline = ScnTimeline::new();

        timeline.register(100, 1000000).unwrap();
        timeline.register(200, 2000000).unwrap();
        timeline.register(300, 3000000).unwrap();

        assert_eq!(timeline.scn_to_timestamp(100).unwrap(), 1000000);
        assert_eq!(timeline.scn_to_timestamp(200).unwrap(), 2000000);
        assert_eq!(timeline.timestamp_to_scn(1500000).unwrap(), 100);
        assert_eq!(timeline.timestamp_to_scn(2500000).unwrap(), 200);
    }

    #[test]
    fn test_version_chain() {
        let mut chain = VersionChain::new(1);

        let v1 = RowVersion {
            values: vec![Value::Integer(10)],
            scn_created: 100,
            scn_deleted: Some(200),
            txn_id: 1,
            bitemporal: None,
        };

        let v2 = RowVersion {
            values: vec![Value::Integer(20)],
            scn_created: 200,
            scn_deleted: None,
            txn_id: 2,
            bitemporal: None,
        };

        chain.add_version(v1);
        chain.add_version(v2);

        // At SCN 150, should see v1
        let version = chain.get_version_at_scn(150).unwrap();
        assert_eq!(version.scn_created, 100);

        // At SCN 250, should see v2
        let version = chain.get_version_at_scn(250).unwrap();
        assert_eq!(version.scn_created, 200);
    }

    #[test]
    fn test_time_travel_engine() {
        let config = TimeTravelConfig::default();
        let engine = TimeTravelEngine::new(config);

        // Register SCNs
        engine.register_scn(100, 1000000).unwrap();
        engine.register_scn(200, 2000000).unwrap();

        // Index some versions
        let version = RowVersion {
            values: vec![Value::Integer(42)],
            scn_created: 100,
            scn_deleted: None,
            txn_id: 1,
            bitemporal: None,
        };

        engine.index_version(1, 1, version).unwrap();

        // Query as of SCN
        let results = engine.query_as_of_scn(1, 150, None).unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].values[0], Value::Integer(42));
    }

    #[test]
    fn test_bitemporal_metadata() {
        let metadata = BiTemporalMetadata {
            valid_from: 1000,
            valid_to: Some(2000),
        };

        assert!(metadata.is_valid_at(1500));
        assert!(!metadata.is_valid_at(500));
        assert!(!metadata.is_valid_at(2500));
    }

    #[test]
    fn test_temporal_predicate() {
        let values = vec![Value::Integer(10), Value::String("test".to_string())];

        let pred = TemporalPredicate::Equals(0, Value::Integer(10));
        assert!(pred.evaluate(&values));

        let pred = TemporalPredicate::GreaterThan(0, Value::Integer(5));
        assert!(pred.evaluate(&values));

        let pred = TemporalPredicate::And(
            Box::new(TemporalPredicate::Equals(0, Value::Integer(10))),
            Box::new(TemporalPredicate::Equals(1, Value::String("test".to_string()))),
        );
        assert!(pred.evaluate(&values));
    }
}
