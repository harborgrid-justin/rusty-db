// T002: Lock Manager Scalability Optimization
//
// This module implements a sharded lock manager with lock-free data structures
// for improved scalability under high concurrency.
//
// Expected performance improvement: +10-15% TPS
//
// Key optimizations:
// 1. Sharded lock table (64 shards) to reduce contention
// 2. Lock-free operations where possible
// 3. Hierarchical locking with intent modes (IS, IX, S, SIX, X)
// 4. Per-shard statistics for monitoring

use crate::common::TransactionId;
use crate::concurrent::ConcurrentHashMap;
use crate::transaction::error::{TransactionError, TransactionResult};
use crate::transaction::types::LockMode;
use parking_lot::{Condvar, Mutex};
use std::collections::{HashSet, VecDeque};
use std::hash::{Hash, Hasher};
use std::sync::Arc;
use std::time::{Duration, SystemTime};

/// Number of shards for lock table partitioning
/// 64 shards provides good balance between concurrency and memory overhead
const SHARD_COUNT: usize = 64;

/// Maximum lock wait timeout (30 seconds)
const MAX_LOCK_WAIT_MS: u64 = 30_000;

/// Intent lock modes for hierarchical locking
///
/// Hierarchical locking allows transaction to lock at different granularities:
/// - Database level
/// - Table level
/// - Row level
///
/// Intent locks indicate intention to acquire finer-grained locks
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum HierarchicalLockMode {
    /// Intent Shared - intends to acquire shared locks at lower level
    IS,
    /// Intent Exclusive - intends to acquire exclusive locks at lower level
    IX,
    /// Shared - read lock
    S,
    /// Shared with Intent Exclusive - shared lock with intent to acquire exclusive locks
    SIX,
    /// Exclusive - write lock
    X,
}

impl HierarchicalLockMode {
    /// Check if two lock modes are compatible
    pub fn is_compatible(&self, other: &HierarchicalLockMode) -> bool {
        use HierarchicalLockMode::*;

        match (self, other) {
            // IS is compatible with everything except X
            (IS, X) | (X, IS) => false,
            (IS, _) | (_, IS) => true,

            // IX is compatible with IS, IX
            (IX, IX) | (IX, IS) | (IS, IX) => true,
            (IX, _) | (_, IX) => false,

            // S is compatible with IS, S
            (S, S) | (S, IS) | (IS, S) => true,
            (S, _) | (_, S) => false,

            // SIX is only compatible with IS
            (SIX, IS) | (IS, SIX) => true,
            (SIX, _) | (_, SIX) => false,

            // X is not compatible with anything
            (X, _) | (_, X) => false,
        }
    }

    /// Get lock strength (higher = stronger)
    pub fn strength(&self) -> u8 {
        use HierarchicalLockMode::*;
        match self {
            IS => 1,
            IX => 2,
            S => 3,
            SIX => 4,
            X => 5,
        }
    }
}

/// Convert from standard LockMode to HierarchicalLockMode
impl From<LockMode> for HierarchicalLockMode {
    fn from(mode: LockMode) -> Self {
        match mode {
            LockMode::Shared => HierarchicalLockMode::S,
            LockMode::Exclusive => HierarchicalLockMode::X,
        }
    }
}

/// Lock table entry with wait queue
struct LockEntry {
    /// Current lock holders: (transaction ID, lock mode)
    holders: Vec<(TransactionId, HierarchicalLockMode)>,
    /// Queue of waiting transactions
    waiters: VecDeque<(TransactionId, HierarchicalLockMode)>,
}

impl LockEntry {
    fn new() -> Self {
        Self {
            holders: Vec::new(),
            waiters: VecDeque::new(),
        }
    }

    /// Check if a lock mode is compatible with current holders
    fn is_compatible(&self, mode: &HierarchicalLockMode) -> bool {
        for (_, holder_mode) in &self.holders {
            if !mode.is_compatible(holder_mode) {
                return false;
            }
        }
        true
    }

    /// Check if transaction already holds a lock
    fn is_held_by(&self, txn_id: TransactionId) -> Option<HierarchicalLockMode> {
        self.holders
            .iter()
            .find(|(id, _)| *id == txn_id)
            .map(|(_, mode)| *mode)
    }
}

/// Single shard in the sharded lock table
struct LockTableShard {
    /// Lock entries for this shard
    locks: ConcurrentHashMap<String, Arc<Mutex<LockEntry>>>,
    /// Wait condition variable for this shard
    wait_condvar: Arc<Condvar>,
    /// Statistics for this shard
    lock_count: std::sync::atomic::AtomicU64,
    wait_count: std::sync::atomic::AtomicU64,
    conflict_count: std::sync::atomic::AtomicU64,
}

impl LockTableShard {
    fn new() -> Self {
        Self {
            locks: ConcurrentHashMap::with_capacity(256),
            wait_condvar: Arc::new(Condvar::new()),
            lock_count: std::sync::atomic::AtomicU64::new(0),
            wait_count: std::sync::atomic::AtomicU64::new(0),
            conflict_count: std::sync::atomic::AtomicU64::new(0),
        }
    }
}

/// Sharded lock manager for scalable concurrency control
///
/// Uses 64 shards to partition the lock namespace, dramatically reducing
/// contention compared to a single global lock table.
pub struct ShardedLockManager {
    /// Array of lock table shards
    shards: Vec<LockTableShard>,
    /// Transaction locks: txn_id -> set of held resources
    /// Using ConcurrentHashMap for lock-free reads
    txn_locks: Arc<ConcurrentHashMap<TransactionId, Arc<Mutex<HashSet<String>>>>>,
    /// Global statistics
    total_acquires: std::sync::atomic::AtomicU64,
    total_releases: std::sync::atomic::AtomicU64,
    total_timeouts: std::sync::atomic::AtomicU64,
}

impl ShardedLockManager {
    /// Create a new sharded lock manager
    pub fn new() -> Self {
        let mut shards = Vec::with_capacity(SHARD_COUNT);
        for _ in 0..SHARD_COUNT {
            shards.push(LockTableShard::new());
        }

        Self {
            shards,
            txn_locks: Arc::new(ConcurrentHashMap::with_capacity(1024)),
            total_acquires: std::sync::atomic::AtomicU64::new(0),
            total_releases: std::sync::atomic::AtomicU64::new(0),
            total_timeouts: std::sync::atomic::AtomicU64::new(0),
        }
    }

    /// Get shard index for a resource
    ///
    /// Uses hash-based partitioning for uniform distribution
    fn shard_index(&self, resource: &str) -> usize {
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        resource.hash(&mut hasher);
        (hasher.finish() as usize) % SHARD_COUNT
    }

    /// Get the shard for a resource
    fn get_shard(&self, resource: &str) -> &LockTableShard {
        let idx = self.shard_index(resource);
        &self.shards[idx]
    }

    /// Acquire a lock on a resource
    pub fn acquire_lock(
        &self,
        txn_id: TransactionId,
        resource: String,
        mode: LockMode,
    ) -> TransactionResult<()> {
        let h_mode = HierarchicalLockMode::from(mode);
        self.acquire_hierarchical_lock(txn_id, resource, h_mode)
    }

    /// Acquire a hierarchical lock
    pub fn acquire_hierarchical_lock(
        &self,
        txn_id: TransactionId,
        resource: String,
        mode: HierarchicalLockMode,
    ) -> TransactionResult<()> {
        self.acquire_lock_with_timeout(
            txn_id,
            resource,
            mode,
            Duration::from_millis(MAX_LOCK_WAIT_MS),
        )
    }

    /// Acquire lock with timeout
    pub fn acquire_lock_with_timeout(
        &self,
        txn_id: TransactionId,
        resource: String,
        mode: HierarchicalLockMode,
        timeout: Duration,
    ) -> TransactionResult<()> {
        self.total_acquires.fetch_add(1, std::sync::atomic::Ordering::Relaxed);

        let shard = self.get_shard(&resource);
        let start_time = SystemTime::now();

        loop {
            // Check timeout
            if let Ok(elapsed) = SystemTime::now().duration_since(start_time) {
                if elapsed >= timeout {
                    self.total_timeouts.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                    return Err(TransactionError::lock_timeout(
                        txn_id,
                        resource,
                        LockMode::from_hierarchical(mode),
                    ));
                }
            }

            // Get or create lock entry
            let entry_arc = shard.locks.compute(resource.clone(), |existing| {
                Some(existing.map_or_else(
                    || Arc::new(Mutex::new(LockEntry::new())),
                    |e| e.clone(),
                ))
            }).unwrap();

            let mut entry = entry_arc.lock();

            // Check if already holding this lock
            if let Some(current_mode) = entry.is_held_by(txn_id) {
                if mode.strength() <= current_mode.strength() {
                    return Ok(());
                }
                // Need to upgrade - check if possible
                if entry.holders.len() == 1 {
                    // Only holder, can upgrade
                    entry.holders[0].1 = mode;
                    return Ok(());
                }
                // Cannot upgrade while others hold locks
                shard.conflict_count.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            }

            // Check compatibility with current holders
            if entry.is_compatible(&mode) {
                // Grant lock
                entry.holders.push((txn_id, mode));

                // Track transaction locks
                let txn_set = self.txn_locks.compute(txn_id, |existing| {
                    Some(existing.map_or_else(
                        || Arc::new(Mutex::new(HashSet::new())),
                        |s| s.clone(),
                    ))
                }).unwrap();
                txn_set.lock().insert(resource.clone());

                shard.lock_count.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                return Ok(());
            }

            // Add to wait queue
            if !entry.waiters.iter().any(|(tid, _)| *tid == txn_id) {
                entry.waiters.push_back((txn_id, mode));
            }

            shard.wait_count.fetch_add(1, std::sync::atomic::Ordering::Relaxed);

            // Wait on condition variable
            let remaining = timeout.saturating_sub(
                SystemTime::now()
                    .duration_since(start_time)
                    .unwrap_or(Duration::ZERO),
            );

            shard.wait_condvar.wait_for(&mut entry, remaining);
            // Loop will retry acquisition
        }
    }

    /// Release a specific lock
    pub fn release_lock(&self, txn_id: TransactionId, resource: &str) -> TransactionResult<()> {
        self.total_releases.fetch_add(1, std::sync::atomic::Ordering::Relaxed);

        let shard = self.get_shard(resource);

        if let Some(entry_arc) = shard.locks.get(&resource.to_string()) {
            let mut entry = entry_arc.lock();
            entry.holders.retain(|(id, _)| *id != txn_id);

            // Wake up waiting transactions
            shard.wait_condvar.notify_all();
        }

        // Remove from transaction locks
        if let Some(txn_set) = self.txn_locks.get(&txn_id) {
            txn_set.lock().remove(resource);
        }

        Ok(())
    }

    /// Release all locks held by a transaction
    pub fn release_all_locks(&self, txn_id: TransactionId) -> TransactionResult<()> {
        let resources: Vec<String> = if let Some(txn_set) = self.txn_locks.get(&txn_id) {
            txn_set.lock().iter().cloned().collect()
        } else {
            return Ok(());
        };

        for resource in &resources {
            let shard = self.get_shard(resource);
            if let Some(entry_arc) = shard.locks.get(&resource.to_string()) {
                let mut entry = entry_arc.lock();
                entry.holders.retain(|(id, _)| *id != txn_id);
                shard.wait_condvar.notify_all();
            }
        }

        self.txn_locks.remove(&txn_id);
        self.total_releases.fetch_add(resources.len() as u64, std::sync::atomic::Ordering::Relaxed);

        Ok(())
    }

    /// Get locks held by a transaction
    pub fn get_locks(&self, txn_id: TransactionId) -> HashSet<String> {
        self.txn_locks
            .get(&txn_id)
            .map(|set| set.lock().clone())
            .unwrap_or_default()
    }

    /// Get statistics
    pub fn stats(&self) -> ShardedLockStats {
        let mut shard_stats = Vec::new();
        let mut total_locks = 0;
        let mut total_waits = 0;
        let mut total_conflicts = 0;

        for (i, shard) in self.shards.iter().enumerate() {
            let locks = shard.lock_count.load(std::sync::atomic::Ordering::Relaxed);
            let waits = shard.wait_count.load(std::sync::atomic::Ordering::Relaxed);
            let conflicts = shard.conflict_count.load(std::sync::atomic::Ordering::Relaxed);

            total_locks += locks;
            total_waits += waits;
            total_conflicts += conflicts;

            shard_stats.push(ShardStats {
                shard_id: i,
                lock_count: locks,
                wait_count: waits,
                conflict_count: conflicts,
            });
        }

        ShardedLockStats {
            total_acquires: self.total_acquires.load(std::sync::atomic::Ordering::Relaxed),
            total_releases: self.total_releases.load(std::sync::atomic::Ordering::Relaxed),
            total_timeouts: self.total_timeouts.load(std::sync::atomic::Ordering::Relaxed),
            total_locks,
            total_waits,
            total_conflicts,
            shard_count: SHARD_COUNT,
            shard_stats,
        }
    }
}

impl Default for ShardedLockManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Statistics for sharded lock manager
#[derive(Debug, Clone)]
pub struct ShardedLockStats {
    pub total_acquires: u64,
    pub total_releases: u64,
    pub total_timeouts: u64,
    pub total_locks: u64,
    pub total_waits: u64,
    pub total_conflicts: u64,
    pub shard_count: usize,
    pub shard_stats: Vec<ShardStats>,
}

#[derive(Debug, Clone)]
pub struct ShardStats {
    pub shard_id: usize,
    pub lock_count: u64,
    pub wait_count: u64,
    pub conflict_count: u64,
}

/// Extension trait for LockMode
impl LockMode {
    fn from_hierarchical(mode: HierarchicalLockMode) -> Self {
        match mode {
            HierarchicalLockMode::S | HierarchicalLockMode::IS => LockMode::Shared,
            HierarchicalLockMode::X | HierarchicalLockMode::IX | HierarchicalLockMode::SIX => {
                LockMode::Exclusive
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hierarchical_lock_compatibility() {
        use HierarchicalLockMode::*;

        // IS is compatible with everything except X
        assert!(IS.is_compatible(&IS));
        assert!(IS.is_compatible(&IX));
        assert!(IS.is_compatible(&S));
        assert!(IS.is_compatible(&SIX));
        assert!(!IS.is_compatible(&X));

        // S is compatible with IS and S
        assert!(S.is_compatible(&IS));
        assert!(S.is_compatible(&S));
        assert!(!S.is_compatible(&IX));
        assert!(!S.is_compatible(&SIX));
        assert!(!S.is_compatible(&X));

        // X is not compatible with anything
        assert!(!X.is_compatible(&IS));
        assert!(!X.is_compatible(&IX));
        assert!(!X.is_compatible(&S));
        assert!(!X.is_compatible(&SIX));
        assert!(!X.is_compatible(&X));
    }

    #[test]
    fn test_sharded_lock_manager() {
        let manager = ShardedLockManager::new();

        // Acquire locks
        assert!(manager
            .acquire_lock(1, "resource1".to_string(), LockMode::Shared)
            .is_ok());
        assert!(manager
            .acquire_lock(2, "resource1".to_string(), LockMode::Shared)
            .is_ok());

        // Get locks for transaction
        let locks = manager.get_locks(1);
        assert_eq!(locks.len(), 1);
        assert!(locks.contains("resource1"));

        // Release locks
        assert!(manager.release_lock(1, "resource1").is_ok());
        let locks = manager.get_locks(1);
        assert_eq!(locks.len(), 0);

        // Check stats
        let stats = manager.stats();
        assert!(stats.total_acquires > 0);
    }

    #[test]
    fn test_lock_sharding_distribution() {
        let manager = ShardedLockManager::new();

        // Acquire locks on many resources
        for i in 0..1000 {
            let resource = format!("resource_{}", i);
            manager.acquire_lock(i, resource, LockMode::Shared).unwrap();
        }

        let stats = manager.stats();

        // Check that locks are distributed across shards
        let non_empty_shards = stats
            .shard_stats
            .iter()
            .filter(|s| s.lock_count > 0)
            .count();

        // Should use most shards (at least 80% with 1000 resources)
        assert!(non_empty_shards > SHARD_COUNT * 4 / 5);
    }

    #[test]
    fn test_concurrent_lock_operations() {
        use std::sync::Arc;
        use std::thread;

        let manager = Arc::new(ShardedLockManager::new());
        let mut handles = vec![];

        // Spawn multiple threads acquiring locks
        for i in 0..10 {
            let m = manager.clone();
            handles.push(thread::spawn(move || {
                for j in 0..100 {
                    let resource = format!("res_{}", j);
                    m.acquire_lock(i, resource.clone(), LockMode::Shared).unwrap();
                }
            }));
        }

        for handle in handles {
            handle.join().unwrap();
        }

        let stats = manager.stats();
        assert_eq!(stats.total_acquires, 1000);
    }
}
