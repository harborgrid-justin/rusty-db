//! # Conflict Resolution Engine
//!
//! Advanced conflict detection and resolution for multi-master replication.
//! Supports multiple strategies including CRDT-based conflict-free resolution.
//! Optimized with per-core sharding and lock-free operations for maximum throughput.

use serde::{Deserialize, Serialize};
use std::collections::{HashMap, VecDeque, BTreeMap};
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, AtomicUsize, AtomicBool, Ordering};
use parking_lot::RwLock;
use std::time::{SystemTime, UNIX_EPOCH};
use crate::Result;
use crate::error::DbError;

/// Conflict resolution strategy
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ConflictResolutionStrategy {
    /// Last writer wins based on timestamp
    LastWriterWins,
    /// First writer wins - reject later writes
    FirstWriterWins,
    /// Site with higher priority wins
    PriorityBased(u32),
    /// Custom resolution function
    Custom(String),
    /// CRDT-based automatic resolution
    CrdtMerge,
    /// Send to manual resolution queue
    Manual,
    /// Maximum value wins
    MaxValue,
    /// Minimum value wins
    MinValue,
    /// Additive resolution (for counters)
    Additive,
}

/// Type of conflict detected
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ConflictType {
    /// Update-Update conflict (same row, different values)
    UpdateUpdate,
    /// Update-Delete conflict
    UpdateDelete,
    /// Delete-Update conflict
    DeleteUpdate,
    /// Delete-Delete conflict (usually not a conflict)
    DeleteDelete,
    /// Insert-Insert conflict (same primary key)
    InsertInsert,
    /// Uniqueness violation
    UniqueConstraintViolation,
    /// Foreign key violation
    ForeignKeyViolation,
}

/// Represents a conflicting change
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConflictingChange {
    /// Change identifier
    pub change_id: String,
    /// Site that originated the change
    pub site_id: String,
    /// Timestamp of the change
    pub timestamp: u64,
    /// Table being modified
    pub table: String,
    /// Primary key of the row
    pub row_key: Vec<u8>,
    /// Old value (if any)
    pub old_value: Option<Vec<u8>>,
    /// New value (if any)
    pub new_value: Option<Vec<u8>>,
    /// Site priority for conflict resolution
    pub priority: u32,
    /// Vector clock for causality tracking
    pub vector_clock: HashMap<String, u64>,
}

/// Detected conflict between changes
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Conflict {
    /// Conflict identifier
    pub id: String,
    /// Type of conflict
    pub conflict_type: ConflictType,
    /// Local change
    pub local_change: ConflictingChange,
    /// Remote change
    pub remote_change: ConflictingChange,
    /// When the conflict was detected
    pub detected_at: u64,
    /// Resolution strategy to use
    pub strategy: ConflictResolutionStrategy,
    /// Whether the conflict has been resolved
    pub resolved: bool,
    /// Resolution result
    pub resolution: Option<ConflictResolution>,
}

/// Result of conflict resolution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConflictResolution {
    /// Which change won
    pub winning_change: String,
    /// Final value to apply
    pub final_value: Option<Vec<u8>>,
    /// Resolution method used
    pub method: String,
    /// When it was resolved
    pub resolved_at: u64,
    /// Whether manual intervention was required
    pub manual: bool,
}

/// CRDT (Conflict-free Replicated Data Type) implementations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CrdtType {
    /// Last-Write-Wins Register
    LwwRegister {
        value: Vec<u8>,
        timestamp: u64,
        site_id: String,
    },
    /// Grow-only Counter
    GCounter(HashMap<String, u64>),
    /// Positive-Negative Counter
    PnCounter {
        positive: HashMap<String, u64>,
        negative: HashMap<String, u64>,
    },
    /// Grow-only Set
    GSet(Vec<Vec<u8>>),
    /// Two-Phase Set
    TwoPhaseSet {
        added: Vec<Vec<u8>>,
        removed: Vec<Vec<u8>>,
    },
    /// Observed-Remove Set
    OrSet {
        elements: HashMap<Vec<u8>, Vec<String>>, // element -> unique tags
    },
}

impl CrdtType {
    /// Merge two CRDT values
    pub fn merge(&mut self, other: &CrdtType) -> Result<()> {
        match (self, other) {
            (
                CrdtType::LwwRegister { value, timestamp, site_id },
                CrdtType::LwwRegister {
                    value: other_value,
                    timestamp: other_timestamp,
                    site_id: other_site_id
                },
            ) => {
                // Last writer wins, use site_id as tie-breaker
                if other_timestamp > timestamp ||
                   (other_timestamp == timestamp && other_site_id > site_id) {
                    *value = other_value.clone();
                    *timestamp = *other_timestamp;
                    *site_id = other_site_id.clone();
                }
                Ok(())
            }
            (CrdtType::GCounter(map), CrdtType::GCounter(other_map)) => {
                // Merge by taking max of each site's counter
                for (site, count) in other_map {
                    let entry = map.entry(site.clone()).or_insert(0);
                    *entry = (*entry).max(*count);
                }
                Ok(())
            }
            (
                CrdtType::PnCounter { positive, negative },
                CrdtType::PnCounter { positive: other_pos, negative: other_neg },
            ) => {
                // Merge positive and negative counters separately
                for (site, count) in other_pos {
                    let entry = positive.entry(site.clone()).or_insert(0);
                    *entry = (*entry).max(*count);
                }
                for (site, count) in other_neg {
                    let entry = negative.entry(site.clone()).or_insert(0);
                    *entry = (*entry).max(*count);
                }
                Ok(())
            }
            (CrdtType::GSet(set), CrdtType::GSet(other_set)) => {
                // Union of sets
                for elem in other_set {
                    if !set.contains(elem) {
                        set.push(elem.clone());
                    }
                }
                Ok(())
            }
            (
                CrdtType::TwoPhaseSet { added, removed },
                CrdtType::TwoPhaseSet { added: other_added, removed: other_removed },
            ) => {
                // Merge added and removed sets
                for elem in other_added {
                    if !added.contains(elem) {
                        added.push(elem.clone());
                    }
                }
                for elem in other_removed {
                    if !removed.contains(elem) {
                        removed.push(elem.clone());
                    }
                }
                Ok(())
            }
            (CrdtType::OrSet { elements }, CrdtType::OrSet { elements: other_elements }) => {
                // Merge element tags
                for (elem, tags) in other_elements {
                    let entry = elements.entry(elem.clone()).or_insert_with(Vec::new);
                    for tag in tags {
                        if !entry.contains(tag) {
                            entry.push(tag.clone());
                        }
                    }
                }
                Ok(())
            }
            _ => Err(DbError::Replication(
                "CRDT type mismatch during merge".to_string()
            )),
        }
    }

    /// Get the current value for reads
    pub fn value(&self) -> Result<Vec<u8>> {
        match self {
            CrdtType::LwwRegister { value, .. } => Ok(value.clone()),
            CrdtType::GCounter(map) => {
                let sum: u64 = map.values().sum();
                Ok(sum.to_le_bytes().to_vec())
            }
            CrdtType::PnCounter { positive, negative } => {
                let pos_sum: u64 = positive.values().sum();
                let neg_sum: u64 = negative.values().sum();
                let result = pos_sum.saturating_sub(neg_sum);
                Ok(result.to_le_bytes().to_vec())
            }
            CrdtType::GSet(set) => {
                let encoded = bincode::serialize(set)
                    .map_err(|e| DbError::Serialization(e.to_string()))?;
                Ok(encoded)
            }
            CrdtType::TwoPhaseSet { added, removed } => {
                let current: Vec<_> = added.iter()
                    .filter(|e| !removed.contains(e))
                    .cloned()
                    .collect();
                let encoded = bincode::serialize(&current)
                    .map_err(|e| DbError::Serialization(e.to_string()))?;
                Ok(encoded)
            }
            CrdtType::OrSet { elements } => {
                let current: Vec<_> = elements.keys().cloned().collect();
                let encoded = bincode::serialize(&current)
                    .map_err(|e| DbError::Serialization(e.to_string()))?;
                Ok(encoded)
            }
        }
    }
}

/// Conflict resolver engine with per-core sharding for high-performance
pub struct ConflictResolver {
    /// Per-core shards to minimize contention
    shards: Arc<Vec<ConflictShard>>,
    /// Custom resolution handlers
    custom_handlers: Arc<RwLock<HashMap<String, Box<dyn Fn(&Conflict) -> Result<ConflictResolution> + Send + Sync>>>>,
    /// CRDT state for conflict-free columns
    crdt_state: Arc<RwLock<HashMap<String, CrdtType>>>,
}

/// Conflict resolution statistics with cache alignment to avoid false sharing
#[repr(C, align(64))]
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ConflictStats {
    pub total_conflicts: u64,
    pub auto_resolved: u64,
    pub manual_resolved: u64,
    pub pending: u64,
    pub lww_resolutions: u64,
    pub fww_resolutions: u64,
    pub crdt_resolutions: u64,
    pub custom_resolutions: u64,
    pub conflicts_by_type: HashMap<String, u64>,
}

/// Atomic conflict statistics for lock-free updates
#[repr(C, align(64))]
struct AtomicConflictStats {
    total_conflicts: AtomicU64,
    auto_resolved: AtomicU64,
    manual_resolved: AtomicU64,
    pending: AtomicU64,
    lww_resolutions: AtomicU64,
    fww_resolutions: AtomicU64,
    crdt_resolutions: AtomicU64,
    custom_resolutions: AtomicU64,
}

impl Default for AtomicConflictStats {
    #[inline]
    fn default() -> Self {
        Self {
            total_conflicts: AtomicU64::new(0),
            auto_resolved: AtomicU64::new(0),
            manual_resolved: AtomicU64::new(0),
            pending: AtomicU64::new(0),
            lww_resolutions: AtomicU64::new(0),
            fww_resolutions: AtomicU64::new(0),
            crdt_resolutions: AtomicU64::new(0),
            custom_resolutions: AtomicU64::new(0),
        }
    }
}

impl AtomicConflictStats {
    #[inline]
    fn snapshot(&self) -> ConflictStats {
        ConflictStats {
            total_conflicts: self.total_conflicts.load(Ordering::Relaxed),
            auto_resolved: self.auto_resolved.load(Ordering::Relaxed),
            manual_resolved: self.manual_resolved.load(Ordering::Relaxed),
            pending: self.pending.load(Ordering::Relaxed),
            lww_resolutions: self.lww_resolutions.load(Ordering::Relaxed),
            fww_resolutions: self.fww_resolutions.load(Ordering::Relaxed),
            crdt_resolutions: self.crdt_resolutions.load(Ordering::Relaxed),
            custom_resolutions: self.custom_resolutions.load(Ordering::Relaxed),
            conflicts_by_type: HashMap::new(),
        }
    }
}

/// Per-core shard for conflict detection to minimize contention
#[repr(C, align(64))]
struct ConflictShard {
    pending_conflicts: RwLock<VecDeque<Conflict>>,
    resolved_conflicts: RwLock<VecDeque<Conflict>>,
    stats: AtomicConflictStats,
}

const NUM_SHARDS: usize = 64;

impl ConflictResolver {
    /// Create a new conflict resolver with per-core sharding
    pub fn new() -> Self {
        let mut shards = Vec::with_capacity(NUM_SHARDS);
        for _ in 0..NUM_SHARDS {
            shards.push(ConflictShard {
                pending_conflicts: RwLock::new(VecDeque::new()),
                resolved_conflicts: RwLock::new(VecDeque::new()),
                stats: AtomicConflictStats::default(),
            });
        }

        Self {
            shards: Arc::new(shards),
            custom_handlers: Arc::new(RwLock::new(HashMap::new())),
            crdt_state: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Select shard for a conflict based on its ID for load distribution
    #[inline(always)]
    fn select_shard(&self, conflict_id: &str) -> &ConflictShard {
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        std::hash::Hash::hash(conflict_id, &mut hasher);
        let hash = std::hash::Hasher::finish(&hasher);
        let index = (hash as usize) % NUM_SHARDS;
        &self.shards[index]
    }

    /// Detect conflict between local and remote changes
    pub fn detect_conflict(
        &self,
        local: ConflictingChange,
        remote: ConflictingChange,
    ) -> Result<Option<Conflict>> {
        // Check if changes are to the same row
        if local.table != remote.table || local.row_key != remote.row_key {
            return Ok(None);
        }

        // Check vector clocks for causality
        if self.is_causally_ordered(&local.vector_clock, &remote.vector_clock) {
            // Remote change is causally after local, no conflict
            return Ok(None);
        }

        if self.is_causally_ordered(&remote.vector_clock, &local.vector_clock) {
            // Local change is causally after remote, no conflict
            return Ok(None);
        }

        // Concurrent changes - determine conflict type
        let conflict_type = self.determine_conflict_type(&local, &remote);

        let conflict = Conflict {
            id: format!("conflict-{}", uuid::Uuid::new_v4()),
            conflict_type: conflict_type.clone(),
            local_change: local,
            remote_change: remote,
            detected_at: Self::current_timestamp(),
            strategy: ConflictResolutionStrategy::LastWriterWins, // default
            resolved: false,
            resolution: None,
        };

        // Update statistics atomically (lock-free)
        let shard = self.select_shard(&conflict.id);
        shard.stats.total_conflicts.fetch_add(1, Ordering::Relaxed);
        shard.stats.pending.fetch_add(1, Ordering::Relaxed);

        Ok(Some(conflict))
    }

    /// Check if one vector clock is causally before another
    #[inline]
    fn is_causally_ordered(
        &self,
        clock1: &HashMap<String, u64>,
        clock2: &HashMap<String, u64>,
    ) -> bool {
        let mut less_than_or_equal = true;
        let mut strictly_less = false;

        for (site, count1) in clock1 {
            let count2 = clock2.get(site).unwrap_or(&0);
            if count1 > count2 {
                less_than_or_equal = false;
                break;
            }
            if count1 < count2 {
                strictly_less = true;
            }
        }

        for (site, count2) in clock2 {
            if !clock1.contains_key(site) && *count2 > 0 {
                strictly_less = true;
            }
        }

        less_than_or_equal && strictly_less
    }

    /// Determine the type of conflict
    #[inline]
    fn determine_conflict_type(
        &self,
        local: &ConflictingChange,
        remote: &ConflictingChange,
    ) -> ConflictType {
        match (&local.old_value, &local.new_value, &remote.old_value, &remote.new_value) {
            (Some(_), Some(_), Some(_), Some(_)) => ConflictType::UpdateUpdate,
            (Some(_), Some(_), Some(_), None) => ConflictType::UpdateDelete,
            (Some(_), None, Some(_), Some(_)) => ConflictType::DeleteUpdate,
            (Some(_), None, Some(_), None) => ConflictType::DeleteDelete,
            (None, Some(_), None, Some(_)) => ConflictType::InsertInsert,
            _ => ConflictType::UpdateUpdate, // default
        }
    }

    /// Resolve a conflict using the specified strategy
    pub fn resolve_conflict(&self, conflict: &mut Conflict) -> Result<ConflictResolution> {
        let resolution = match &conflict.strategy {
            ConflictResolutionStrategy::LastWriterWins => {
                self.resolve_lww(conflict)?
            }
            ConflictResolutionStrategy::FirstWriterWins => {
                self.resolve_fww(conflict)?
            }
            ConflictResolutionStrategy::PriorityBased(_) => {
                self.resolve_priority(conflict)?
            }
            ConflictResolutionStrategy::Custom(handler_name) => {
                self.resolve_custom(conflict, handler_name)?
            }
            ConflictResolutionStrategy::CrdtMerge => {
                self.resolve_crdt(conflict)?
            }
            ConflictResolutionStrategy::Manual => {
                // Add to manual resolution queue
                self.pending_conflicts.write().push_back(conflict.clone());
                return Err(DbError::Replication(
                    "Conflict requires manual resolution".to_string()
                ));
            }
            ConflictResolutionStrategy::MaxValue => {
                self.resolve_max_value(conflict)?
            }
            ConflictResolutionStrategy::MinValue => {
                self.resolve_min_value(conflict)?
            }
            ConflictResolutionStrategy::Additive => {
                self.resolve_additive(conflict)?
            }
        };

        conflict.resolved = true;
        conflict.resolution = Some(resolution.clone());

        // Update statistics atomically (lock-free)
        let shard = self.select_shard(&conflict.id);
        shard.stats.auto_resolved.fetch_add(1, Ordering::Relaxed);
        shard.stats.pending.fetch_sub(1, Ordering::Relaxed);

        match &conflict.strategy {
            ConflictResolutionStrategy::LastWriterWins => {
                shard.stats.lww_resolutions.fetch_add(1, Ordering::Relaxed);
            }
            ConflictResolutionStrategy::FirstWriterWins => {
                shard.stats.fww_resolutions.fetch_add(1, Ordering::Relaxed);
            }
            ConflictResolutionStrategy::CrdtMerge => {
                shard.stats.crdt_resolutions.fetch_add(1, Ordering::Relaxed);
            }
            ConflictResolutionStrategy::Custom(_) => {
                shard.stats.custom_resolutions.fetch_add(1, Ordering::Relaxed);
            }
            _ => {}
        }

        // Move to resolved queue
        shard.resolved_conflicts.write().push_back(conflict.clone());

        Ok(resolution)
    }

    /// Last-Writer-Wins resolution
    #[inline]
    fn resolve_lww(&self, conflict: &Conflict) -> Result<ConflictResolution> {
        let winning_change = if conflict.remote_change.timestamp > conflict.local_change.timestamp {
            &conflict.remote_change
        } else if conflict.remote_change.timestamp < conflict.local_change.timestamp {
            &conflict.local_change
        } else {
            // Tie-breaker: use site_id
            if conflict.remote_change.site_id > conflict.local_change.site_id {
                &conflict.remote_change
            } else {
                &conflict.local_change
            }
        };

        Ok(ConflictResolution {
            winning_change: winning_change.change_id.clone(),
            final_value: winning_change.new_value.clone(),
            method: "LastWriterWins".to_string(),
            resolved_at: Self::current_timestamp(),
            manual: false,
        })
    }

    /// First-Writer-Wins resolution
    #[inline]
    fn resolve_fww(&self, conflict: &Conflict) -> Result<ConflictResolution> {
        let winning_change = if conflict.local_change.timestamp < conflict.remote_change.timestamp {
            &conflict.local_change
        } else if conflict.local_change.timestamp > conflict.remote_change.timestamp {
            &conflict.remote_change
        } else {
            // Tie-breaker: use site_id
            if conflict.local_change.site_id < conflict.remote_change.site_id {
                &conflict.local_change
            } else {
                &conflict.remote_change
            }
        };

        Ok(ConflictResolution {
            winning_change: winning_change.change_id.clone(),
            final_value: winning_change.new_value.clone(),
            method: "FirstWriterWins".to_string(),
            resolved_at: Self::current_timestamp(),
            manual: false,
        })
    }

    /// Priority-based resolution
    fn resolve_priority(&self, conflict: &Conflict) -> Result<ConflictResolution> {
        let winning_change = if conflict.remote_change.priority > conflict.local_change.priority {
            &conflict.remote_change
        } else {
            &conflict.local_change
        };

        Ok(ConflictResolution {
            winning_change: winning_change.change_id.clone(),
            final_value: winning_change.new_value.clone(),
            method: "PriorityBased".to_string(),
            resolved_at: Self::current_timestamp(),
            manual: false,
        })
    }

    /// Custom handler resolution
    fn resolve_custom(&self, conflict: &Conflict, handler_name: &str) -> Result<ConflictResolution> {
        let handlers = self.custom_handlers.read();
        let handler = handlers.get(handler_name)
            .ok_or_else(|| DbError::Replication(
                format!("Custom handler '{}' not found", handler_name)
            ))?;

        handler(conflict)
    }

    /// CRDT-based resolution
    fn resolve_crdt(&self, conflict: &Conflict) -> Result<ConflictResolution> {
        let key = format!("{}:{}", conflict.local_change.table,
                         String::from_utf8_lossy(&conflict.local_change.row_key));

        let mut crdt_state = self.crdt_state.write();

        // Get or create CRDT for this key
        let crdt = crdt_state.entry(key.clone()).or_insert_with(|| {
            CrdtType::LwwRegister {
                value: conflict.local_change.new_value.clone().unwrap_or_default(),
                timestamp: conflict.local_change.timestamp,
                site_id: conflict.local_change.site_id.clone(),
            }
        });

        // Create CRDT from remote change
        let remote_crdt = CrdtType::LwwRegister {
            value: conflict.remote_change.new_value.clone().unwrap_or_default(),
            timestamp: conflict.remote_change.timestamp,
            site_id: conflict.remote_change.site_id.clone(),
        };

        // Merge
        crdt.merge(&remote_crdt)?;

        let final_value = crdt.value()?;

        Ok(ConflictResolution {
            winning_change: "merged".to_string(),
            final_value: Some(final_value),
            method: "CrdtMerge".to_string(),
            resolved_at: Self::current_timestamp(),
            manual: false,
        })
    }

    /// Max value resolution
    fn resolve_max_value(&self, conflict: &Conflict) -> Result<ConflictResolution> {
        let local_val = conflict.local_change.new_value.as_ref()
            .ok_or_else(|| DbError::Replication("No local value".to_string()))?;
        let remote_val = conflict.remote_change.new_value.as_ref()
            .ok_or_else(|| DbError::Replication("No remote value".to_string()))?;

        let winning_change = if remote_val > local_val {
            &conflict.remote_change
        } else {
            &conflict.local_change
        };

        Ok(ConflictResolution {
            winning_change: winning_change.change_id.clone(),
            final_value: winning_change.new_value.clone(),
            method: "MaxValue".to_string(),
            resolved_at: Self::current_timestamp(),
            manual: false,
        })
    }

    /// Min value resolution
    fn resolve_min_value(&self, conflict: &Conflict) -> Result<ConflictResolution> {
        let local_val = conflict.local_change.new_value.as_ref()
            .ok_or_else(|| DbError::Replication("No local value".to_string()))?;
        let remote_val = conflict.remote_change.new_value.as_ref()
            .ok_or_else(|| DbError::Replication("No remote value".to_string()))?;

        let winning_change = if remote_val < local_val {
            &conflict.remote_change
        } else {
            &conflict.local_change
        };

        Ok(ConflictResolution {
            winning_change: winning_change.change_id.clone(),
            final_value: winning_change.new_value.clone(),
            method: "MinValue".to_string(),
            resolved_at: Self::current_timestamp(),
            manual: false,
        })
    }

    /// Additive resolution (for counters)
    fn resolve_additive(&self, conflict: &Conflict) -> Result<ConflictResolution> {
        // Try to parse as integers and add them
        let local_val = conflict.local_change.new_value.as_ref()
            .ok_or_else(|| DbError::Replication("No local value".to_string()))?;
        let remote_val = conflict.remote_change.new_value.as_ref()
            .ok_or_else(|| DbError::Replication("No remote value".to_string()))?;

        // Assuming 8-byte integers
        if local_val.len() == 8 && remote_val.len() == 8 {
            let local_int = i64::from_le_bytes(local_val[..8].try_into().unwrap());
            let remote_int = i64::from_le_bytes(remote_val[..8].try_into().unwrap());
            let sum = local_int + remote_int;

            Ok(ConflictResolution {
                winning_change: "sum".to_string(),
                final_value: Some(sum.to_le_bytes().to_vec()),
                method: "Additive".to_string(),
                resolved_at: Self::current_timestamp(),
                manual: false,
            })
        } else {
            Err(DbError::Replication("Values are not compatible for additive resolution".to_string()))
        }
    }

    /// Register a custom conflict resolution handler
    pub fn register_custom_handler<F>(&self, name: String, handler: F)
    where
        F: Fn(&Conflict) -> Result<ConflictResolution> + Send + Sync + 'static,
    {
        self.custom_handlers.write().insert(name, Box::new(handler));
    }

    /// Get pending conflicts for manual resolution
    pub fn get_pending_conflicts(&self) -> Vec<Conflict> {
        let mut all_conflicts = Vec::new();
        for shard in self.shards.iter() {
            let conflicts = shard.pending_conflicts.read();
            all_conflicts.extend(conflicts.iter().cloned());
        }
        all_conflicts
    }

    /// Manually resolve a conflict
    pub fn resolve_manually(&self, conflict_id: &str, resolution: ConflictResolution) -> Result<()> {
        let shard = self.select_shard(conflict_id);
        let mut pending = shard.pending_conflicts.write();

        if let Some(pos) = pending.iter().position(|c| c.id == conflict_id) {
            let mut conflict = pending.remove(pos).unwrap();
            conflict.resolved = true;
            conflict.resolution = Some(resolution);

            shard.resolved_conflicts.write().push_back(conflict);

            shard.stats.manual_resolved.fetch_add(1, Ordering::Relaxed);
            shard.stats.pending.fetch_sub(1, Ordering::Relaxed);

            Ok(())
        } else {
            self.conflict_not_found_error(conflict_id)
        }
    }

    /// Error for conflict not found - marked cold as this is the error path
    #[cold]
    #[inline(never)]
    fn conflict_not_found_error(&self, conflict_id: &str) -> Result<()> {
        Err(DbError::Replication(format!("Conflict {} not found", conflict_id)))
    }

    /// Get conflict statistics aggregated across all shards
    pub fn get_stats(&self) -> ConflictStats {
        let mut stats = ConflictStats::default();

        // Aggregate stats from all shards
        for shard in self.shards.iter() {
            stats.total_conflicts += shard.stats.total_conflicts.load(Ordering::Relaxed);
            stats.auto_resolved += shard.stats.auto_resolved.load(Ordering::Relaxed);
            stats.manual_resolved += shard.stats.manual_resolved.load(Ordering::Relaxed);
            stats.pending += shard.stats.pending.load(Ordering::Relaxed);
            stats.lww_resolutions += shard.stats.lww_resolutions.load(Ordering::Relaxed);
            stats.fww_resolutions += shard.stats.fww_resolutions.load(Ordering::Relaxed);
            stats.crdt_resolutions += shard.stats.crdt_resolutions.load(Ordering::Relaxed);
            stats.custom_resolutions += shard.stats.custom_resolutions.load(Ordering::Relaxed);
        }

        stats
    }

    /// Clear resolved conflicts older than specified days
    pub fn cleanup_resolved(&self, days: u64) -> Result<u64> {
        let cutoff = Self::current_timestamp() - (days * 24 * 60 * 60 * 1000);
        let mut total_removed = 0u64;

        for shard in self.shards.iter() {
            let mut resolved = shard.resolved_conflicts.write();
            let original_len = resolved.len();

            resolved.retain(|c| {
                c.resolution.as_ref()
                    .map(|r| r.resolved_at >= cutoff)
                    .unwrap_or(true)
            });

            total_removed += (original_len - resolved.len()) as u64;
        }

        Ok(total_removed)
    }

    /// Current timestamp in milliseconds
    fn current_timestamp() -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64
    }
}

impl Default for ConflictResolver {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lww_resolution() {
        let resolver = ConflictResolver::new();

        let local = ConflictingChange {
            change_id: "local-1".to_string(),
            site_id: "site-a".to_string(),
            timestamp: 1000,
            table: "users".to_string(),
            row_key: vec![1, 2, 3],
            old_value: Some(vec![1]),
            new_value: Some(vec![2]),
            priority: 1,
            vector_clock: HashMap::new(),
        };

        let remote = ConflictingChange {
            change_id: "remote-1".to_string(),
            site_id: "site-b".to_string(),
            timestamp: 2000,
            table: "users".to_string(),
            row_key: vec![1, 2, 3],
            old_value: Some(vec![1]),
            new_value: Some(vec![3]),
            priority: 1,
            vector_clock: HashMap::new(),
        };

        let mut conflict = Conflict {
            id: "conflict-1".to_string(),
            conflict_type: ConflictType::UpdateUpdate,
            local_change: local,
            remote_change: remote,
            detected_at: 3000,
            strategy: ConflictResolutionStrategy::LastWriterWins,
            resolved: false,
            resolution: None,
        };

        let resolution = resolver.resolve_conflict(&mut conflict).unwrap();
        assert_eq!(resolution.winning_change, "remote-1");
        assert_eq!(resolution.final_value, Some(vec![3]));
    }

    #[test]
    fn test_crdt_merge() {
        let mut lww1 = CrdtType::LwwRegister {
            value: vec![1, 2, 3],
            timestamp: 1000,
            site_id: "site-a".to_string(),
        };

        let lww2 = CrdtType::LwwRegister {
            value: vec![4, 5, 6],
            timestamp: 2000,
            site_id: "site-b".to_string(),
        };

        lww1.merge(&lww2).unwrap();

        if let CrdtType::LwwRegister { value, timestamp, site_id } = lww1 {
            assert_eq!(value, vec![4, 5, 6]);
            assert_eq!(timestamp, 2000);
            assert_eq!(site_id, "site-b");
        }
    }

    #[test]
    fn test_g_counter_merge() {
        let mut gc1 = CrdtType::GCounter(
            vec![("site-a".to_string(), 5), ("site-b".to_string(), 3)]
                .into_iter()
                .collect()
        );

        let gc2 = CrdtType::GCounter(
            vec![("site-a".to_string(), 4), ("site-b".to_string(), 6), ("site-c".to_string(), 2)]
                .into_iter()
                .collect()
        );

        gc1.merge(&gc2).unwrap();

        if let CrdtType::GCounter(map) = gc1 {
            assert_eq!(map.get("site-a"), Some(&5));
            assert_eq!(map.get("site-b"), Some(&6));
            assert_eq!(map.get("site-c"), Some(&2));
        }
    }
}
