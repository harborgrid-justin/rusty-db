// Lock Manager Implementation
// Provides hierarchical locking with intent locks, deadlock detection,
// lock escalation, and multi-granularity locking

use std::collections::HashSet;
use std::collections::VecDeque;
use std::time::Instant;
use std::collections::{HashMap};
use std::sync::Arc;
use std::time::{Duration};
use parking_lot::{Mutex, RwLock, Condvar};
use serde::{Deserialize, Serialize};
use crate::error::Result;
use super::TransactionId;

/// Lock granularity levels in hierarchical locking
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub enum LockGranularity {
    Database = 0,
    Table = 1,
    Page = 2,
    Row = 3,
}

impl LockGranularity {
    /// Get parent granularity level
    pub fn parent(&self) -> Option<LockGranularity> {
        match self {
            LockGranularity::Database => None,
            LockGranularity::Table => Some(LockGranularity::Database),
            LockGranularity::Page => Some(LockGranularity::Table),
            LockGranularity::Row => Some(LockGranularity::Page),
        }
    }

    /// Get child granularity level
    pub fn child(&self) -> Option<LockGranularity> {
        match self {
            LockGranularity::Database => Some(LockGranularity::Table),
            LockGranularity::Table => Some(LockGranularity::Page),
            LockGranularity::Page => Some(LockGranularity::Row),
            LockGranularity::Row => None,
        }
    }
}

/// Lock modes with hierarchical intent locks
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum LockMode {
    /// Intent Shared - intent to acquire S locks at finer granularity
    IS,
    /// Intent Exclusive - intent to acquire X locks at finer granularity
    IX,
    /// Shared - read lock
    S,
    /// Shared with Intent Exclusive - S lock with intent for X locks
    SIX,
    /// Update - special lock to prevent upgrade deadlocks
    U,
    /// Exclusive - write lock
    X,
}

impl LockMode {
    /// Check if two lock modes are compatible
    pub fn is_compatible(&self, other: &LockMode) -> bool {
        use LockMode::*;

        // Compatibility matrix
        matches!(
            (self, other),
            // IS is compatible with IS, IX, S, SIX, U
            (IS, IS) | (IS, IX) | (IS, S) | (IS, SIX) | (IS, U) |
            (IX, IS) | (S, IS) | (SIX, IS) | (U, IS) |

            // IX is compatible with IS, IX
            (IX, IX) |

            // S is compatible with IS, S, U
            (S, S) | (S, U) |
            (U, S) |

            // SIX is compatible with IS
            (SIX, IS) |

            // U is compatible with IS, S
            (U, U)
        )
    }

    /// Get lock strength (higher value = stronger lock)
    pub fn strength(&self) -> u8 {
        match self {
            LockMode::IS => 1,
            LockMode::IX => 2,
            LockMode::S => 3,
            LockMode::U => 4,
            LockMode::SIX => 5,
            LockMode::X => 6,
        }
    }

    /// Check if this mode can be upgraded to another mode
    pub fn can_upgrade_to(&self, target: &LockMode) -> bool {
        self.strength() < target.strength()
    }

    /// Get required intent lock for parent
    pub fn required_intent_lock(&self) -> Option<LockMode> {
        match self {
            LockMode::S => Some(LockMode::IS),
            LockMode::X => Some(LockMode::IX),
            LockMode::U => Some(LockMode::IX),
            LockMode::SIX => Some(LockMode::IX),
            _ => None,
        }
    }
}

/// Lock resource identifier
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct LockResource {
    pub granularity: LockGranularity,
    pub database_id: Option<u64>,
    pub table_id: Option<u64>,
    pub page_id: Option<u64>,
    pub row_id: Option<u64>,
}

impl LockResource {
    /// Create a database-level resource
    pub fn database(db_id: u64) -> Self {
        Self {
            granularity: LockGranularity::Database,
            database_id: Some(db_id),
            table_id: None,
            page_id: None,
            row_id: None,
        }
    }

    /// Create a table-level resource
    pub fn table(db_id: u64, table_id: u64) -> Self {
        Self {
            granularity: LockGranularity::Table,
            database_id: Some(db_id),
            table_id: Some(table_id),
            page_id: None,
            row_id: None,
        }
    }

    /// Create a page-level resource
    pub fn page(db_id: u64, table_id: u64, page_id: u64) -> Self {
        Self {
            granularity: LockGranularity::Page,
            database_id: Some(db_id),
            table_id: Some(table_id),
            page_id: Some(page_id),
            row_id: None,
        }
    }

    /// Create a row-level resource
    pub fn row(db_id: u64, table_id: u64, page_id: u64, row_id: u64) -> Self {
        Self {
            granularity: LockGranularity::Row,
            database_id: Some(db_id),
            table_id: Some(table_id),
            page_id: Some(page_id),
            row_id: Some(row_id),
        }
    }

    /// Get parent resource
    pub fn parent(&self) -> Option<LockResource> {
        match self.granularity {
            LockGranularity::Database => None,
            LockGranularity::Table => {
                Some(LockResource::database(self.database_id?))
            }
            LockGranularity::Page => {
                Some(LockResource::table(self.database_id?, self.table_id?))
            }
            LockGranularity::Row => {
                Some(LockResource::page(self.database_id?, self.table_id?, self.page_id?))
            }
        }
    }

    /// Get all ancestors in hierarchy
    pub fn ancestors(&self) -> Vec<LockResource> {
        let mut ancestors = Vec::new();
        let mut current = self.parent();

        while let Some(resource) = current {
            ancestors.push(resource.clone());
            current = resource.parent();
        }

        ancestors
    }
}

/// Lock request in the queue
#[derive(Debug, Clone)]
struct LockRequest {
    txn_id: TransactionId,
    mode: LockMode,
    resource: LockResource,
    granted: bool,
    request_time: Instant,
}

/// Lock table entry for a resource
struct LockTableEntry {
    resource: LockResource,
    /// Granted locks
    granted: HashMap<TransactionId, LockMode>,
    /// Waiting queue
    waiting: VecDeque<LockRequest>,
    /// Condvar for waiting transactions
    condvar: Arc<Condvar>,
}

impl LockTableEntry {
    fn new(resource: LockResource) -> Self {
        Self {
            resource,
            granted: HashMap::new(),
            waiting: VecDeque::new(),
            condvar: Arc::new(Condvar::new()),
        }
    }

    /// Check if a lock mode is compatible with currently granted locks
    fn is_compatible(&self, mode: LockMode, requesting_txn: TransactionId) -> bool {
        for (&txn_id, &granted_mode) in &self.granted {
            // Skip self-compatibility check
            if txn_id == requesting_txn {
                continue;
            }

            if !mode.is_compatible(&granted_mode) {
                return false;
            }
        }
        true
    }

    /// Grant a lock
    fn grant(&mut self, txn_id: TransactionId, mode: LockMode) {
        self.granted.insert(txn_id, mode);
    }

    /// Release a lock
    fn release(&mut self, txn_id: TransactionId) -> bool {
        self.granted.remove(&txn_id).is_some()
    }

    /// Upgrade a lock
    fn upgrade(&mut self, txn_id: TransactionId, new_mode: LockMode) -> std::result::Result<(), DbError> {
        if let Some(&current_mode) = self.granted.get(&txn_id) {
            if current_mode.can_upgrade_to(&new_mode) {
                self.granted.insert(txn_id, new_mode);
                Ok(())
            } else {
                Err(DbError::LockError("Cannot upgrade lock".to_string()))
            }
        } else {
            Err(DbError::LockError("Lock not held".to_string()))
        }
    }
}

/// Hierarchical Lock Manager
pub struct HierarchicalLockManager {
    /// Lock table indexed by resource
    lock_table: Arc<RwLock<HashMap<LockResource, Arc<Mutex<LockTableEntry>>>>>,
    /// Transaction lock holdings (for easy release)
    txn_locks: Arc<RwLock<HashMap<TransactionId<LockResource>>>>,
    /// Wait-for graph for deadlock detection
    wait_for_graph: Arc<RwLock<WaitForGraph>>,
    /// Configuration
    config: LockManagerConfig,
    /// Statistics
    stats: Arc<RwLock<LockManagerStats>>,
}

#[derive(Debug, Clone)]
pub struct LockManagerConfig {
    /// Enable deadlock detection
    pub enable_deadlock_detection: bool,
    /// Lock timeout (milliseconds)
    pub lock_timeout_ms: u64,
    /// Enable lock escalation
    pub enable_escalation: bool,
    /// Escalation threshold (number of row locks before escalating)
    pub escalation_threshold: usize,
}

impl Default for LockManagerConfig {
    fn default() -> Self {
        Self {
            enable_deadlock_detection: true,
            lock_timeout_ms: 5000,
            enable_escalation: true,
            escalation_threshold: 1000,
        }
    }
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct LockManagerStats {
    pub locks_acquired: u64,
    pub locks_released: u64,
    pub lock_upgrades: u64,
    pub lock_escalations: u64,
    pub deadlocks_detected: u64,
    pub lock_timeouts: u64,
    pub avg_wait_time_ms: f64,
}

impl HierarchicalLockManager {
    /// Create a new hierarchical lock manager
    pub fn new(config: LockManagerConfig) -> Self {
        Self {
            lock_table: Arc::new(RwLock::new(HashMap::new())),
            txn_locks: Arc::new(RwLock::new(HashMap::new())),
            wait_for_graph: Arc::new(RwLock::new(WaitForGraph::new())),
            config,
            stats: Arc::new(RwLock::new(LockManagerStats::default())),
        }
    }

    /// Acquire a lock with hierarchical locking protocol
    pub fn acquire_lock(
        &self,
        txn_id: TransactionId,
        resource: LockResource,
        mode: LockMode,
    ) -> std::result::Result<(), DbError> {
        let start = Instant::now();

        // First, acquire intent locks on all ancestors
        self.acquire_intent_locks(txn_id, &resource, mode)?;

        // Now acquire the actual lock
        let result = self.acquire_lock_internal(txn_id, resource.clone(), mode);

        // Update statistics
        if result.is_ok() {
            let wait_time = start.elapsed().as_millis() as f64;
            let mut stats = self.stats.write();
            stats.locks_acquired += 1;
            stats.avg_wait_time_ms =
                (stats.avg_wait_time_ms * (stats.locks_acquired - 1) as f64 + wait_time)
                    / stats.locks_acquired as f64;

            // Track transaction locks
            self.txn_locks.write()
                .entry(txn_id)
                .or_insert_with(HashSet::new)
                .insert(resource);
        }

        result
    }

    /// Acquire intent locks on ancestors
    fn acquire_intent_locks(
        &self,
        txn_id: TransactionId,
        resource: &LockResource,
        mode: LockMode,
    ) -> std::result::Result<(), DbError> {
        if let Some(intent_mode) = mode.required_intent_lock() {
            for ancestor in resource.ancestors() {
                self.acquire_lock_internal(txn_id, ancestor, intent_mode)?;
            }
        }
        Ok(())
    }

    /// Internal lock acquisition
    fn acquire_lock_internal(
        &self,
        txn_id: TransactionId,
        resource: LockResource,
        mode: LockMode,
    ) -> std::result::Result<(), DbError> {
        let timeout = Duration::from_millis(self.config.lock_timeout_ms);
        let deadline = Instant::now() + timeout;

        loop {
            // Get or create lock table entry
            let entry_arc = {
                let mut lock_table = self.lock_table.write();
                lock_table
                    .entry(resource.clone())
                    .or_insert_with(|| Arc::new(Mutex::new(LockTableEntry::new(resource.clone()))))
                    .clone()
            };

            let mut entry = entry_arc.lock();

            // Check if lock is already held by this transaction
            if let Some(&held_mode) = entry.granted.get(&txn_id) {
                return if held_mode == mode {
                    Ok(()) // Already have this lock
                } else if held_mode.strength() >= mode.strength() {
                    Ok(()) // Have stronger lock
                } else {
                    // Need to upgrade
                    self.upgrade_lock_internal(txn_id, resource, mode)
                }
            }

            // Check compatibility
            if entry.is_compatible(mode, txn_id) && entry.waiting.is_empty() {
                // Grant immediately
                entry.grant(txn_id, mode);
                return Ok(());
            }

            // Need to wait - check for deadlock first
            if self.config.enable_deadlock_detection {
                // Add to wait-for graph
                for &holder_txn in entry.granted.keys() {
                    if holder_txn != txn_id {
                        self.wait_for_graph.write().add_edge(txn_id, holder_txn);
                    }
                }

                // Check for deadlock
                if let Some(cycle) = self.wait_for_graph.read().detect_cycle_from(txn_id) {
                    self.stats.write().deadlocks_detected += 1;
                    // Clean up wait-for graph
                    self.wait_for_graph.write().remove_transaction(txn_id);
                    return Err(DbError::DeadlockDetected(format!(
                        "Deadlock detected involving transactions: {:?}",
                        cycle
                    ))));
                }
            }

            // Add to waiting queue
            let request = LockRequest {
                txn_id,
                mode,
                resource: resource.clone(),
                granted: false,
                request_time: Instant::now(),
            };
            entry.waiting.push_back(request);

            // Wait with timeout
            let condvar = entry.condvar.clone();
            let remaining = deadline.saturating_duration_since(Instant::now());

            if remaining.is_zero() {
                self.stats.write().lock_timeouts += 1;
                self.wait_for_graph.write().remove_transaction(txn_id);
                return Err(DbError::Timeout(format!(
                    "Lock timeout for transaction {} on resource {:?}",
                    txn_id, resource
                ))));
            }

            // Wait for notification
            condvar.wait_for(&mut entry, remaining);

            // Check if granted
            if entry.granted.contains_key(&txn_id) {
                // Remove from waiting queue
                entry.waiting.retain(|req| req.txn_id != txn_id);
                self.wait_for_graph.write().remove_transaction(txn_id);
                return Ok(());
            }
        }
    }

    /// Upgrade a lock
    fn upgrade_lock_internal(
        &self,
        txn_id: TransactionId,
        resource: LockResource,
        new_mode: LockMode,
    ) -> std::result::Result<(), DbError> {
        let lock_table = self.lock_table.read();
        if let Some(entry_arc) = lock_table.get(&resource) {
            let mut entry = entry_arc.lock();
            entry.upgrade(txn_id, new_mode)?;
            self.stats.write().lock_upgrades += 1;
            Ok(())
        } else {
            Err(DbError::LockError("Resource not locked".to_string()))
        }
    }

    /// Release a lock
    pub fn release_lock(&self, txn_id: TransactionId, resource: &LockResource) -> std::result::Result<(), DbError> {
        let lock_table = self.lock_table.read();
        if let Some(entry_arc) = lock_table.get(resource) {
            let mut entry = entry_arc.lock();

            if entry.release(txn_id) {
                self.stats.write().locks_released += 1;

                // Remove from transaction locks
                if let Some(locks) = self.txn_locks.write().get_mut(&txn_id) {
                    locks.remove(resource);
                }

                // Try to grant waiting locks
                self.grant_waiting_locks(&mut entry);

                Ok(())
            } else {
                Err(DbError::LockError("Lock not held".to_string()))
            }
        } else {
            Err(DbError::LockError("Resource not locked".to_string()))
        }
    }

    /// Release all locks for a transaction
    pub fn release_all_locks(&self, txn_id: TransactionId) -> std::result::Result<(), DbError> {
        let resources: Vec<LockResource> = {
            let txn_locks = self.txn_locks.read();
            txn_locks.get(&txn_id)
                .map(|locks| locks.iter().cloned().collect())
                .unwrap_or_default()
        };

        for resource in resources {
            self.release_lock(txn_id, &resource)?;
        }

        // Clean up transaction locks entry
        self.txn_locks.write().remove(&txn_id);

        // Clean up wait-for graph
        self.wait_for_graph.write().remove_transaction(txn_id);

        Ok(())
    }

    /// Try to grant waiting locks
    fn grant_waiting_locks(&self, entry: &mut LockTableEntry) {
        let mut granted_any = false;

        loop {
            let mut granted_this_round = false;

            // Try to grant locks from the front of the queue
            while let Some(request) = entry.waiting.front() {
                if entry.is_compatible(request.mode, request.txn_id) {
                    let request = entry.waiting.pop_front().unwrap();
                    entry.grant(request.txn_id, request.mode);
                    granted_this_round = true;
                    granted_any = true;
                } else {
                    break; // Can't grant this one, stop trying
                }
            }

            if !granted_this_round {
                break; // No more locks can be granted
            }
        }

        // Notify waiting transactions if any were granted
        if granted_any {
            entry.condvar.notify_all();
        }
    }

    /// Escalate row locks to page lock
    pub fn escalate_locks(&self, txn_id: TransactionId, table: &LockResource) -> std::result::Result<(), DbError> {
        if !self.config.enable_escalation {
            return Ok(());
        }

        let txn_locks_read = self.txn_locks.read();
        let locks = txn_locks_read.get(&txn_id).ok_or_else(|| {
            DbError::LockError("Transaction not found".to_string())
        })?;

        // Count row locks for this table
        let row_locks: Vec<_> = locks
            .iter()
            .filter(|r| {
                r.granularity == LockGranularity::Row &&
                r.database_id == table.database_id &&
                r.table_id == table.table_id
            })
            .cloned()
            .collect();

        if row_locks.len() >= self.config.escalation_threshold {
            drop(txn_locks_read);

            // Release all row locks
            for row_lock in &row_locks {
                self.release_lock(txn_id, row_lock)?;
            }

            // Acquire table lock
            self.acquire_lock(txn_id, table.clone(), LockMode::X)?;

            self.stats.write().lock_escalations += 1;
        }

        Ok(())
    }

    /// Get statistics
    pub fn get_stats(&self) -> LockManagerStats {
        (*self.stats.read()).clone()
    }

    /// Get locks held by a transaction
    pub fn get_transaction_locks(&self, txn_id: TransactionId) -> Vec<LockResource> {
        self.txn_locks.read()
            .get(&txn_id)
            .map(|locks| locks.iter().cloned().collect())
            .unwrap_or_default()
    }
}

/// Wait-for graph for deadlock detection
#[derive(Debug)]
pub struct WaitForGraph {
    /// Edges: waiting_txn -> set of transactions it's waiting for
    edges: HashMap<TransactionId<TransactionId>>,
    /// Transaction start times for victim selection
    timestamps: HashMap<TransactionId>,
}

impl WaitForGraph {
    pub fn new() -> Self {
        Self {
            edges: HashMap::new(),
            timestamps: HashMap::new(),
        }
    }

    /// Add a wait edge
    pub fn add_edge(&mut self, waiter: TransactionId, holder: TransactionId) {
        self.edges
            .entry(waiter)
            .or_insert_with(HashSet::new)
            .insert(holder);

        self.timestamps.entry(waiter).or_insert_with(Instant::now);
        self.timestamps.entry(holder).or_insert_with(Instant::now);
    }

    /// Remove a transaction from the graph
    pub fn remove_transaction(&mut self, txn_id: TransactionId) {
        self.edges.remove(&txn_id);
        self.timestamps.remove(&txn_id);

        // Remove from all waiting lists
        for waiting_set in self.edges.values_mut() {
            waiting_set.remove(&txn_id);
        }
    }

    /// Detect cycle starting from a specific transaction
    pub fn detect_cycle_from(&self, start_txn: TransactionId) -> Option<Vec<TransactionId>> {
        let mut visited = HashSet::new();
        let mut rec_stack = HashSet::new();
        let mut path = Vec::new();

        self.dfs_cycle(start_txn, &mut visited, &mut rec_stack, &mut path)
    }

    /// Detect any cycle in the graph
    pub fn detect_cycle(&self) -> Option<Vec<TransactionId>> {
        let mut visited = HashSet::new();
        let mut rec_stack = HashSet::new();
        let mut path = Vec::new();

        for &txn_id in self.edges.keys() {
            if !visited.contains(&txn_id) {
                if let Some(cycle) = self.dfs_cycle(txn_id, &mut visited, &mut rec_stack, &mut path) {
                    return Some(cycle);
                }
            }
        }

        None
    }

    fn dfs_cycle(
        &self,
        txnid: TransactionId,
        visited: &mut HashSet<TransactionId>,
        rec_stack: &mut HashSet<TransactionId>,
        path: &mut Vec<TransactionId>,
    ) -> Option<Vec<TransactionId>> {
        visited.insert(txn_id);
        rec_stack.insert(txn_id);
        path.push(txn_id);

        if let Some(waiting_for) = self.edges.get(&txn_id) {
            for &next_txn in waiting_for {
                if !visited.contains(&next_txn) {
                    if let Some(cycle) = self.dfs_cycle(next_txn, visited, rec_stack, path) {
                        return Some(cycle);
                    }
                } else if rec_stack.contains(&next_txn) {
                    // Found a cycle
                    let cycle_start = path.iter().position(|&id| id == next_txn).unwrap();
                    return Some(path[cycle_start..].to_vec());
                }
            }
        }

        path.pop();
        rec_stack.remove(&txn_id);
        None
    }
}

/// Lock escalation policy manager
pub struct LockEscalationManager {
    lock_manager: Arc<HierarchicalLockManager>,
    /// Escalation thresholds per table
    thresholds: Arc<RwLock<HashMap<u64, usize>>>,
    /// Statistics
    stats: Arc<RwLock<EscalationStats>>,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct EscalationStats {
    pub total_escalations: u64,
    pub row_to_page: u64,
    pub page_to_table: u64,
}

impl LockEscalationManager {
    pub fn new(lock_manager: Arc<HierarchicalLockManager>) -> Self {
        Self {
            lock_manager,
            thresholds: Arc::new(RwLock::new(HashMap::new())),
            stats: Arc::new(RwLock::new(EscalationStats::default())),
        }
    }

    /// Set escalation threshold for a table
    pub fn set_threshold(&self, table_id: u64, threshold: usize) {
        self.thresholds.write().insert(table_id, threshold);
    }

    /// Check and perform escalation if needed
    pub fn check_escalation(&self, txn_id: TransactionId, table_id: u64) -> std::result::Result<(), DbError> {
        let threshold = self.thresholds.read()
            .get(&table_id)
            .copied()
            .unwrap_or(1000);

        let locks = self.lock_manager.get_transaction_locks(txn_id);
        let row_count = locks.iter()
            .filter(|r| {
                r.granularity == LockGranularity::Row &&
                r.table_id == Some(table_id)
            })
            .count();

        if row_count >= threshold {
            // Perform escalation
            let table_resource = LockResource::table(0, table_id);
            self.lock_manager.escalate_locks(txn_id, &table_resource)?;

            let mut stats = self.stats.write();
            stats.total_escalations += 1;
            stats.row_to_page += 1;
        }

        Ok(())
    }

    pub fn get_stats(&self) -> EscalationStats {
        (*self.stats.read()).clone()
    }
}

#[cfg(test)]
mod tests {

    #[test]
    fn test_lock_compatibility() {
        assert!(LockMode::IS.is_compatible(&LockMode::IS));
        assert!(LockMode::IS.is_compatible(&LockMode::IX));
        assert!(LockMode::S.is_compatible(&LockMode::S));
        assert!(!LockMode::X.is_compatible(&LockMode::S));
        assert!(!LockMode::X.is_compatible(&LockMode::X));
    }

    #[test]
    fn test_hierarchical_locking() {
        let config = LockManagerConfig::default();
        let lock_mgr = HierarchicalLockManager::new(config);

        let row = LockResource::row(1, 1, 1, 1);

        // Acquire row lock
        lock_mgr.acquire_lock(1, row.clone(), LockMode::X).unwrap();

        // Should have intent locks on ancestors
        let txn_locks = lock_mgr.get_transaction_locks(1);
        assert!(txn_locks.len() > 1); // Row + intent locks
    }

    #[test]
    fn test_wait_for_graph_cycle() {
        let mut graph = WaitForGraph::new();

        // Create cycle: 1 -> 2 -> 3 -> 1
        graph.add_edge(1, 2);
        graph.add_edge(2, 3);
        graph.add_edge(3, 1);

        let cycle = graph.detect_cycle();
        assert!(cycle.is_some());
    }

    #[test]
    fn test_lock_release() {
        let config = LockManagerConfig::default();
        let lock_mgr = HierarchicalLockManager::new(config);

        let row = LockResource::row(1, 1, 1, 1);

        lock_mgr.acquire_lock(1, row.clone(), LockMode::S).unwrap();
        assert!(lock_mgr.release_lock(1, &row).is_ok());

        let txn_locks = lock_mgr.get_transaction_locks(1);
        assert!(!txn_locks.contains(&row));
    }
}


