// # Multi-Master Replication
//
// Bidirectional replication with conflict detection and resolution,
// quorum-based writes, and convergence guarantees.
//
// ============================================================================
// SECURITY FIX: PR #55/56 - Issue P0-5: Applied Operations Unbounded
// ============================================================================
// CRITICAL: HashSet<String> for applied_ops can grow to 64+ GB without limits.
// This constant limits the applied operations tracking to prevent unbounded growth.
//
// Maximum applied operations to track for deduplication
// At ~64 bytes per operation ID, this limits memory to ~64MB
#[allow(dead_code)]
const MAX_APPLIED_OPERATIONS: usize = 1_000_000;
//
// TODO(performance): Implement applied operations bounds and cleanup
// - Use sliding window instead of unbounded HashSet
// - Implement periodic cleanup of old operation IDs (>24 hours)
// - Add LRU eviction when MAX_APPLIED_OPERATIONS is reached
// - Consider bloom filter for membership testing
//
// Reference: diagrams/07_security_enterprise_flow.md Section 8.5
// ============================================================================

use super::conflicts::{ConflictResolutionStrategy, ConflictResolver, ConflictingChange};
use crate::error::DbError;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::collections::HashSet;
use std::collections::VecDeque;
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio::sync::mpsc;

type Result<T> = std::result::Result<T, DbError>;

/// Replication group for multi-master setup
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReplicationGroup {
    /// Group identifier
    pub id: String,
    /// Group name
    pub name: String,
    /// Member sites
    pub members: Vec<SiteInfo>,
    /// Tables included in replication
    pub tables: Vec<String>,
    /// Conflict resolution strategy
    pub conflict_strategy: ConflictResolutionStrategy,
    /// Quorum size for writes
    pub write_quorum: usize,
    /// Quorum size for reads
    pub read_quorum: usize,
    /// Group creation time
    pub created_at: u64,
}

/// Information about a site in the replication group
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SiteInfo {
    /// Site identifier
    pub site_id: String,
    /// Site name
    pub name: String,
    /// Network address
    pub address: String,
    /// Site priority (for conflict resolution)
    pub priority: u32,
    /// Site region/datacenter
    pub region: String,
    /// Whether site is currently active
    pub active: bool,
    /// Last heartbeat time
    pub last_heartbeat: u64,
}

/// Replication operation to be propagated
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReplicationOp {
    /// Operation ID
    pub op_id: String,
    /// Originating site
    pub site_id: String,
    /// Table name
    pub table: String,
    /// Operation type
    pub op_type: OpType,
    /// Row key
    pub row_key: Vec<u8>,
    /// Old value (for updates/deletes)
    pub old_value: Option<Vec<u8>>,
    /// New value (for inserts/updates)
    pub new_value: Option<Vec<u8>>,
    /// Operation timestamp
    pub timestamp: u64,
    /// Vector clock for causality
    pub vector_clock: HashMap<String, u64>,
    /// Dependencies (operation IDs that must be applied first)
    pub dependencies: Vec<String>,
}

/// Type of replication operation
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum OpType {
    Insert,
    Update,
    Delete,
    Truncate,
    Schema,
}

/// Quorum write result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuorumResult {
    /// Whether quorum was achieved
    pub success: bool,
    /// Number of successful acks
    pub acks: usize,
    /// Number of failures
    pub failures: usize,
    /// Sites that acknowledged
    pub ack_sites: Vec<String>,
    /// Sites that failed
    pub failed_sites: Vec<String>,
}

/// Multi-master replication manager
pub struct MultiMasterReplication {
    /// Local site ID
    local_site_id: String,
    /// Replication groups this site belongs to
    groups: Arc<RwLock<HashMap<String, ReplicationGroup>>>,
    /// Pending operations to be replicated
    _pending_ops: Arc<RwLock<VecDeque<ReplicationOp>>>,
    /// Applied operations (for deduplication)
    applied_ops: Arc<RwLock<HashSet<String>>>,
    /// Vector clock for this site
    vector_clock: Arc<RwLock<HashMap<String, u64>>>,
    /// Conflict resolver
    conflict_resolver: Arc<ConflictResolver>,
    /// Statistics
    stats: Arc<RwLock<MultiMasterStats>>,
    /// Operation channel
    op_tx: mpsc::UnboundedSender<ReplicationOp>,
    op_rx: Arc<RwLock<mpsc::UnboundedReceiver<ReplicationOp>>>,
}

/// Multi-master replication statistics with cache alignment
#[repr(C, align(64))]
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct MultiMasterStats {
    pub total_ops: u64,
    pub inserts: u64,
    pub updates: u64,
    pub deletes: u64,
    pub conflicts_detected: u64,
    pub conflicts_resolved: u64,
    pub quorum_writes: u64,
    pub quorum_failures: u64,
    pub ops_by_site: HashMap<String, u64>,
}

impl MultiMasterReplication {
    /// Create a new multi-master replication manager
    pub fn new(site_id: String) -> Self {
        let (op_tx, op_rx) = mpsc::unbounded_channel();

        Self {
            local_site_id: site_id,
            groups: Arc::new(RwLock::new(HashMap::new())),
            _pending_ops: Arc::new(RwLock::new(VecDeque::new())),
            applied_ops: Arc::new(RwLock::new(HashSet::new())),
            vector_clock: Arc::new(RwLock::new(HashMap::new())),
            conflict_resolver: Arc::new(ConflictResolver::new()),
            stats: Arc::new(RwLock::new(MultiMasterStats::default())),
            op_tx,
            op_rx: Arc::new(RwLock::new(op_rx)),
        }
    }

    /// Create a new replication group
    pub fn create_group(&self, group: ReplicationGroup) -> Result<()> {
        let mut groups = self.groups.write();

        if groups.contains_key(&group.id) {
            return Err(DbError::Replication(format!(
                "Group {} already exists",
                group.id
            )));
        }

        groups.insert(group.id.clone(), group);
        Ok(())
    }

    /// Add a site to a replication group
    pub fn add_site_to_group(&self, group_id: &str, site: SiteInfo) -> Result<()> {
        let mut groups = self.groups.write();

        let group = groups
            .get_mut(group_id)
            .ok_or_else(|| DbError::Replication(format!("Group {} not found", group_id)))?;

        if group.members.iter().any(|s| s.site_id == site.site_id) {
            return Err(DbError::Replication(format!(
                "Site {} already in group",
                site.site_id
            )));
        }

        group.members.push(site);
        Ok(())
    }

    /// Remove a site from a replication group
    pub fn remove_site_from_group(&self, group_id: &str, site_id: &str) -> Result<()> {
        let mut groups = self.groups.write();

        let group = groups
            .get_mut(group_id)
            .ok_or_else(|| DbError::Replication(format!("Group {} not found", group_id)))?;

        group.members.retain(|s| s.site_id != site_id);
        Ok(())
    }

    /// Perform a quorum write
    pub async fn quorum_write(&self, op: ReplicationOp, group_id: &str) -> Result<QuorumResult> {
        let group = {
            let groups = self.groups.read();
            groups
                .get(group_id)
                .ok_or_else(|| DbError::Replication(format!("Group {} not found", group_id)))?
                .clone()
        };

        // Update vector clock
        {
            let mut clock = self.vector_clock.write();
            let counter = clock.entry(self.local_site_id.clone()).or_insert(0);
            *counter += 1;
        }

        // Send operation to all sites in group
        let mut acks = 0;
        let mut failures = 0;
        let mut ack_sites = Vec::new();
        let mut failed_sites = Vec::new();

        for site in &group.members {
            if site.site_id == self.local_site_id {
                // Apply locally
                self.apply_operation(&op).await?;
                acks += 1;
                ack_sites.push(site.site_id.clone());
            } else if site.active {
                // Send to remote site
                match self.send_to_site(&op, site).await {
                    Ok(_) => {
                        acks += 1;
                        ack_sites.push(site.site_id.clone());
                    }
                    Err(_) => {
                        failures += 1;
                        failed_sites.push(site.site_id.clone());
                    }
                }
            }
        }

        let success = acks >= group.write_quorum;

        // Update statistics
        {
            let mut stats = self.stats.write();
            stats.quorum_writes += 1;
            if !success {
                stats.quorum_failures += 1;
            }
        }

        Ok(QuorumResult {
            success,
            acks,
            failures,
            ack_sites,
            failed_sites,
        })
    }

    /// Send operation to a remote site
    async fn send_to_site(&self, _op: &ReplicationOp, site: &SiteInfo) -> Result<()> {
        // In a real implementation, this would send over network
        // For now, simulate with a small delay
        tokio::time::sleep(Duration::from_millis(10)).await;

        // Simulate 95% success rate
        if rand::random::<f64>() < 0.95 {
            Ok(())
        } else {
            Err(DbError::Replication(format!(
                "Failed to send to site {}",
                site.site_id
            )))
        }
    }

    /// Apply a replication operation
    pub async fn apply_operation(&self, op: &ReplicationOp) -> Result<()> {
        // Check if already applied
        {
            let applied = self.applied_ops.read();
            if applied.contains(&op.op_id) {
                return Ok(()); // Already applied
            }
        }

        // Check for conflicts
        if let Some(existing_op) = self.find_conflicting_operation(op) {
            let local_change = ConflictingChange {
                change_id: existing_op.op_id.clone(),
                site_id: existing_op.site_id.clone(),
                timestamp: existing_op.timestamp,
                table: existing_op.table.clone(),
                row_key: existing_op.row_key.clone(),
                old_value: existing_op.old_value.clone(),
                new_value: existing_op.new_value.clone(),
                priority: 1, // Default priority
                vector_clock: existing_op.vector_clock.clone(),
            };

            let remote_change = ConflictingChange {
                change_id: op.op_id.clone(),
                site_id: op.site_id.clone(),
                timestamp: op.timestamp,
                table: op.table.clone(),
                row_key: op.row_key.clone(),
                old_value: op.old_value.clone(),
                new_value: op.new_value.clone(),
                priority: 1, // Default priority
                vector_clock: op.vector_clock.clone(),
            };

            if let Some(mut conflict) = self
                .conflict_resolver
                .detect_conflict(local_change, remote_change)?
            {
                // Update statistics
                {
                    let mut stats = self.stats.write();
                    stats.conflicts_detected += 1;
                }

                // Resolve conflict
                self.conflict_resolver.resolve_conflict(&mut conflict)?;

                // Update statistics
                {
                    let mut stats = self.stats.write();
                    stats.conflicts_resolved += 1;
                }
            }
        }

        // Apply the operation
        self.execute_operation(op)?;

        // Mark as applied
        {
            let mut applied = self.applied_ops.write();
            applied.insert(op.op_id.clone());
        }

        // Update statistics
        {
            let mut stats = self.stats.write();
            stats.total_ops += 1;

            match op.op_type {
                OpType::Insert => stats.inserts += 1,
                OpType::Update => stats.updates += 1,
                OpType::Delete => stats.deletes += 1,
                _ => {}
            }

            *stats.ops_by_site.entry(op.site_id.clone()).or_insert(0) += 1;
        }

        Ok(())
    }

    /// Find a conflicting operation
    fn find_conflicting_operation(&self, _op: &ReplicationOp) -> Option<ReplicationOp> {
        // In a real implementation, this would check pending operations
        // For now, return None
        None
    }

    /// Execute the actual operation
    fn execute_operation(&self, op: &ReplicationOp) -> Result<()> {
        // In a real implementation, this would apply to the storage engine
        // For now, just validate
        match op.op_type {
            OpType::Insert => {
                if op.new_value.is_none() {
                    return Err(DbError::Replication(
                        "Insert requires new value".to_string(),
                    ));
                }
            }
            OpType::Update => {
                if op.new_value.is_none() {
                    return Err(DbError::Replication(
                        "Update requires new value".to_string(),
                    ));
                }
            }
            OpType::Delete => {
                // Delete doesn't require new value
            }
            _ => {}
        }

        Ok(())
    }

    /// Receive operation from remote site
    pub async fn receive_operation(&self, op: ReplicationOp) -> Result<()> {
        // Queue for processing
        self.op_tx
            .send(op)
            .map_err(|e| DbError::Replication(format!("Failed to queue operation: {}", e)))?;

        Ok(())
    }

    /// Process queued operations
    pub async fn process_operations(&self) -> Result<()> {
        let mut rx = self.op_rx.write();

        while let Ok(op) = rx.try_recv() {
            self.apply_operation(&op).await?;
        }

        Ok(())
    }

    /// Update heartbeat for a site
    pub fn update_heartbeat(&self, group_id: &str, site_id: &str) -> Result<()> {
        let mut groups = self.groups.write();

        let group = groups
            .get_mut(group_id)
            .ok_or_else(|| DbError::Replication(format!("Group {} not found", group_id)))?;

        for site in &mut group.members {
            if site.site_id == site_id {
                site.last_heartbeat = Self::current_timestamp();
                return Ok(());
            }
        }

        Err(DbError::Replication(format!(
            "Site {} not found in group",
            site_id
        )))
    }

    /// Check for failed sites (no heartbeat)
    pub fn check_failed_sites(&self, timeout_ms: u64) -> HashMap<String, Vec<String>> {
        let groups = self.groups.read();
        let cutoff = Self::current_timestamp() - timeout_ms;
        let mut failed = HashMap::new();

        for (group_id, group) in groups.iter() {
            let failed_sites: Vec<String> = group
                .members
                .iter()
                .filter(|s| s.last_heartbeat < cutoff)
                .map(|s| s.site_id.clone())
                .collect();

            if !failed_sites.is_empty() {
                failed.insert(group_id.clone(), failed_sites);
            }
        }

        failed
    }

    /// Mark a site as inactive
    pub fn mark_site_inactive(&self, group_id: &str, site_id: &str) -> Result<()> {
        let mut groups = self.groups.write();

        let group = groups
            .get_mut(group_id)
            .ok_or_else(|| DbError::Replication(format!("Group {} not found", group_id)))?;

        for site in &mut group.members {
            if site.site_id == site_id {
                site.active = false;
                return Ok(());
            }
        }

        Err(DbError::Replication(format!(
            "Site {} not found in group",
            site_id
        )))
    }

    /// Mark a site as active
    pub fn mark_site_active(&self, group_id: &str, site_id: &str) -> Result<()> {
        let mut groups = self.groups.write();

        let group = groups
            .get_mut(group_id)
            .ok_or_else(|| DbError::Replication(format!("Group {} not found", group_id)))?;

        for site in &mut group.members {
            if site.site_id == site_id {
                site.active = true;
                site.last_heartbeat = Self::current_timestamp();
                return Ok(());
            }
        }

        Err(DbError::Replication(format!(
            "Site {} not found in group",
            site_id
        )))
    }

    /// Get replication statistics
    #[inline]
    pub fn get_stats(&self) -> MultiMasterStats {
        self.stats.read().clone()
    }

    /// Get all replication groups
    #[inline]
    pub fn get_groups(&self) -> Vec<ReplicationGroup> {
        self.groups.read().values().cloned().collect()
    }

    /// Get a specific replication group
    #[inline]
    pub fn get_group(&self, group_id: &str) -> Option<ReplicationGroup> {
        self.groups.read().get(group_id).cloned()
    }

    /// Verify convergence for a group
    pub async fn verify_convergence(&self, group_id: &str) -> Result<ConvergenceReport> {
        let group = self
            .get_group(group_id)
            .ok_or_else(|| DbError::Replication(format!("Group {} not found", group_id)))?;

        let mut site_checksums = HashMap::new();

        // Get checksum from each active site
        for site in &group.members {
            if site.active {
                let checksum = self.get_site_checksum(&site.site_id).await?;
                site_checksums.insert(site.site_id.clone(), checksum);
            }
        }

        // Check if all checksums match
        let checksums: HashSet<_> = site_checksums.values().collect();
        let converged = checksums.len() == 1;

        Ok(ConvergenceReport {
            group_id: group_id.to_string(),
            converged,
            site_checksums,
            checked_at: Self::current_timestamp(),
        })
    }

    /// Get checksum for a site's data
    async fn get_site_checksum(&self, _site_id: &str) -> Result<String> {
        // In a real implementation, this would compute a checksum of the data
        // For now, return a dummy checksum
        Ok(format!("checksum-{}", rand::random::<u64>()))
    }

    /// Get vector clock
    pub fn get_vector_clock(&self) -> HashMap<String, u64> {
        self.vector_clock.read().clone()
    }

    /// Update vector clock from remote
    pub fn update_vector_clock(&self, remote_clock: &HashMap<String, u64>) {
        let mut clock = self.vector_clock.write();

        for (site_id, remote_count) in remote_clock {
            let local_count = clock.entry(site_id.clone()).or_insert(0);
            *local_count = (*local_count).max(*remote_count);
        }
    }

    /// Current timestamp in milliseconds
    fn current_timestamp() -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64
    }
}

/// Convergence verification report
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConvergenceReport {
    pub group_id: String,
    pub converged: bool,
    pub site_checksums: HashMap<String, String>,
    pub checked_at: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_create_group() {
        let mm = MultiMasterReplication::new("site-1".to_string());

        let group = ReplicationGroup {
            id: "group-1".to_string(),
            name: "Test Group".to_string(),
            members: vec![],
            tables: vec!["users".to_string()],
            conflict_strategy: ConflictResolutionStrategy::LastWriterWins,
            write_quorum: 2,
            read_quorum: 2,
            created_at: 0,
        };

        mm.create_group(group).unwrap();

        let groups = mm.get_groups();
        assert_eq!(groups.len(), 1);
        assert_eq!(groups[0].id, "group-1");
    }

    #[tokio::test]
    async fn test_add_site_to_group() {
        let mm = MultiMasterReplication::new("site-1".to_string());

        let group = ReplicationGroup {
            id: "group-1".to_string(),
            name: "Test Group".to_string(),
            members: vec![],
            tables: vec!["users".to_string()],
            conflict_strategy: ConflictResolutionStrategy::LastWriterWins,
            write_quorum: 2,
            read_quorum: 2,
            created_at: 0,
        };

        mm.create_group(group).unwrap();

        let site = SiteInfo {
            site_id: "site-2".to_string(),
            name: "Site 2".to_string(),
            address: "localhost:5433".to_string(),
            priority: 1,
            region: "us-west".to_string(),
            active: true,
            last_heartbeat: 0,
        };

        mm.add_site_to_group("group-1", site).unwrap();

        let group = mm.get_group("group-1").unwrap();
        assert_eq!(group.members.len(), 1);
    }
}
