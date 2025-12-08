//! # Retention Policy Management
//!
//! This module implements retention policies for blockchain tables:
//! - Retention policy enforcement
//! - Time-based retention locks
//! - Legal hold implementation
//! - Retention period tracking
//! - Expiration and archival
//! - Retention inheritance
//! - Compliance reporting

use serde::{Deserialize, Serialize};
use std::collections::{HashMap};
use std::sync::{Arc, RwLock};
use std::time::{Duration};
use crate::common::{TableId, RowId};
use crate::Result;
use crate::error::DbError;
use super::ledger::{BlockId, LedgerRow, Block};

// ============================================================================
// Retention Period
// ============================================================================

/// Retention period specification
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RetentionPeriod {
    /// No retention (immediate deletion allowed)
    None,
    /// Days
    Days(u32),
    /// Months
    Months(u32),
    /// Years
    Years(u32),
    /// Forever (never delete)
    Forever,
}

impl RetentionPeriod {
    /// Convert to duration in seconds
    pub fn to_seconds(&self) -> Option<u64> {
        match self {
            RetentionPeriod::None => Some(0),
            RetentionPeriod::Days(d) => Some(*d as u64 * 86400),
            RetentionPeriod::Months(m) => Some(*m as u64 * 30 * 86400), // Approximate
            RetentionPeriod::Years(y) => Some(*y as u64 * 365 * 86400), // Approximate
            RetentionPeriod::Forever => None,
        }
    }

    /// Check if this period is longer than another
    pub fn is_longer_than(&self, other: &RetentionPeriod) -> bool {
        match (self.to_seconds(), other.to_seconds()) {
            (None, _) => true, // Forever is longest
            (_, None) => false,
            (Some(a), Some(b)) => a > b,
        }
    }
}

impl std::fmt::Display for RetentionPeriod {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            RetentionPeriod::None => write!(f, "None"),
            RetentionPeriod::Days(d) => write!(f, "{} days", d),
            RetentionPeriod::Months(m) => write!(f, "{} months", m),
            RetentionPeriod::Years(y) => write!(f, "{} years", y),
            RetentionPeriod::Forever => write!(f, "Forever"),
        }
    }
}

// ============================================================================
// Retention Policy
// ============================================================================

/// Retention policy for data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetentionPolicy {
    /// Policy ID
    pub policy_id: String,
    /// Policy name
    pub name: String,
    /// Retention period
    pub period: RetentionPeriod,
    /// Description
    pub description: String,
    /// Compliance regulations
    pub regulations: Vec<String>,
    /// Created timestamp
    pub created_at: u64,
    /// Updated timestamp
    pub updated_at: u64,
    /// Whether policy is active
    pub active: bool,
}

impl RetentionPolicy {
    /// Create a new retention policy
    pub fn new(policy_id: String, name: String, period: RetentionPeriod, description: String) -> Self {
        let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();

        Self {
            policy_id,
            name,
            period,
            description,
            regulations: Vec::new(),
            created_at: now,
            updated_at: now,
            active: true,
        }
    }

    /// Add a regulation reference
    pub fn add_regulation(&mut self, regulation: String) {
        self.regulations.push(regulation);
        self.updated_at = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();
    }

    /// Check if data created at timestamp should be retained
    pub fn should_retain(&self, created_at: u64) -> bool {
        if !self.active {
            return false;
        }

        match self.period.to_seconds() {
            None => true, // Forever
            Some(0) => false, // None
            Some(period_secs) => {
                let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();
                let age = now.saturating_sub(created_at);
                age < period_secs
            }
        }
    }

    /// Get expiration timestamp for data created at timestamp
    pub fn expiration_time(&self, created_at: u64) -> Option<u64> {
        self.period.to_seconds().map(|secs| created_at + secs)
    }
}

// ============================================================================
// Legal Hold
// ============================================================================

/// Legal hold status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LegalHoldStatus {
    /// Active hold
    Active,
    /// Hold released
    Released,
    /// Hold expired
    Expired,
}

/// Legal hold on data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LegalHold {
    /// Hold ID
    pub hold_id: String,
    /// Case/matter ID
    pub case_id: String,
    /// Description
    pub description: String,
    /// Custodian/requester
    pub custodian: String,
    /// Start timestamp
    pub started_at: u64,
    /// End timestamp (if released)
    pub ended_at: Option<u64>,
    /// Status
    pub status: LegalHoldStatus,
    /// Affected tables
    pub tables: Vec<TableId>,
    /// Affected blocks
    pub blocks: Vec<BlockId>,
    /// Affected rows
    pub rows: Vec<RowId>,
    /// Metadata
    pub metadata: HashMap<String, String>,
}

impl LegalHold {
    /// Create a new legal hold
    pub fn new(hold_id: String, case_id: String, description: String, custodian: String) -> Self {
        Self {
            hold_id,
            case_id,
            description,
            custodian,
            started_at: SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs(),
            ended_at: None,
            status: LegalHoldStatus::Active,
            tables: Vec::new(),
            blocks: Vec::new(),
            rows: Vec::new(),
            metadata: HashMap::new(),
        }
    }

    /// Add a table to the hold
    pub fn add_table(&mut self, table_id: TableId) {
        if !self.tables.contains(&table_id) {
            self.tables.push(table_id);
        }
    }

    /// Add a block to the hold
    pub fn add_block(&mut self, block_id: BlockId) {
        if !self.blocks.contains(&block_id) {
            self.blocks.push(block_id);
        }
    }

    /// Add a row to the hold
    pub fn add_row(&mut self, row_id: RowId) {
        if !self.rows.contains(&row_id) {
            self.rows.push(row_id);
        }
    }

    /// Release the hold
    pub fn release(&mut self) {
        self.status = LegalHoldStatus::Released;
        self.ended_at = Some(SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs());
    }

    /// Check if hold is active
    pub fn is_active(&self) -> bool {
        self.status == LegalHoldStatus::Active
    }

    /// Check if a row is under this hold
    pub fn covers_row(&self, row_id: RowId) -> bool {
        self.rows.contains(&row_id)
    }

    /// Check if a block is under this hold
    pub fn covers_block(&self, block_id: BlockId) -> bool {
        self.blocks.contains(&block_id)
    }

    /// Check if a table is under this hold
    pub fn covers_table(&self, table_id: TableId) -> bool {
        self.tables.contains(&table_id)
    }
}

// ============================================================================
// Retention Manager
// ============================================================================

/// Manages retention policies and legal holds
pub struct RetentionManager {
    /// Active retention policies
    policies: Arc<RwLock<HashMap<String, RetentionPolicy>>>,
    /// Table to policy mapping
    table_policies: Arc<RwLock<HashMap<TableId, String>>>,
    /// Active legal holds
    legal_holds: Arc<RwLock<HashMap<String, LegalHold>>>,
    /// Retention locks (row_id -> expiration)
    retention_locks: Arc<RwLock<HashMap<RowId, u64>>>,
    /// Default policy
    default_policy: Arc<RwLock<Option<String>>>,
}

impl RetentionManager {
    /// Create a new retention manager
    pub fn new() -> Self {
        Self {
            policies: Arc::new(RwLock::new(HashMap::new())),
            table_policies: Arc::new(RwLock::new(HashMap::new())),
            legal_holds: Arc::new(RwLock::new(HashMap::new())),
            retention_locks: Arc::new(RwLock::new(HashMap::new())),
            default_policy: Arc::new(RwLock::new(None)),
        }
    }

    /// Add a retention policy
    pub fn add_policy(&self, policy: RetentionPolicy) -> Result<()> {
        let mut policies = self.policies.write().unwrap();
        policies.insert(policy.policy_id.clone(), policy);
        Ok(())
    }

    /// Get a policy by ID
    pub fn get_policy(&self, policy_id: &str) -> Option<RetentionPolicy> {
        let policies = self.policies.read().unwrap();
        policies.get(policy_id).cloned()
    }

    /// Set default retention policy
    pub fn set_default_policy(&self, policy_id: String) -> Result<()> {
        let policies = self.policies.read().unwrap();
        if !policies.contains_key(&policy_id) {
            return Err(DbError::NotFound(format!("Policy {} not found", policy_id)));
        }

        let mut default = self.default_policy.write().unwrap();
        *default = Some(policy_id);
        Ok(())
    }

    /// Assign a policy to a table
    pub fn assign_policy_to_table(&self, table_id: TableId, policy_id: String) -> Result<()> {
        let policies = self.policies.read().unwrap();
        if !policies.contains_key(&policy_id) {
            return Err(DbError::NotFound(format!("Policy {} not found", policy_id)));
        }

        let mut table_policies = self.table_policies.write().unwrap();
        table_policies.insert(table_id, policy_id);
        Ok(())
    }

    /// Get policy for a table
    pub fn get_table_policy(&self, table_id: TableId) -> Option<RetentionPolicy> {
        let table_policies = self.table_policies.read().unwrap();
        let policies = self.policies.read().unwrap();

        if let Some(policy_id) = table_policies.get(&table_id) {
            return policies.get(policy_id).cloned();
        }

        // Fall back to default policy
        let default = self.default_policy.read().unwrap();
        if let Some(ref policy_id) = *default {
            return policies.get(policy_id).cloned();
        }

        None
    }

    /// Add a legal hold
    pub fn add_legal_hold(&self, hold: LegalHold) -> Result<()> {
        let mut holds = self.legal_holds.write().unwrap();
        holds.insert(hold.hold_id.clone(), hold);
        Ok(())
    }

    /// Get a legal hold by ID
    pub fn get_legal_hold(&self, hold_id: &str) -> Option<LegalHold> {
        let holds = self.legal_holds.read().unwrap();
        holds.get(hold_id).cloned()
    }

    /// Release a legal hold
    pub fn release_legal_hold(&self, hold_id: &str) -> Result<()> {
        let mut holds = self.legal_holds.write().unwrap();
        let hold = holds.get_mut(hold_id)
            .ok_or_else(|| DbError::NotFound(format!("Legal hold {} not found", hold_id)))?;
        hold.release();
        Ok(())
    }

    /// Check if a row is under any active legal hold
    pub fn is_under_legal_hold(&self, row_id: RowId, table_id: TableId, block_id: BlockId) -> bool {
        let holds = self.legal_holds.read().unwrap();

        for hold in holds.values() {
            if !hold.is_active() {
                continue;
            }

            if hold.covers_row(row_id) || hold.covers_block(block_id) || hold.covers_table(table_id) {
                return true;
            }
        }

        false
    }

    /// Check if data can be deleted
    pub fn can_delete(&self, row: &LedgerRow) -> bool {
        // Check legal holds
        if self.is_under_legal_hold(row.row_id, row.table_id, row.block_id) {
            return false;
        }

        // Check retention policy
        if let Some(policy) = self.get_table_policy(row.table_id) {
            if policy.should_retain(row.timestamp) {
                return false;
            }
        }

        // Check retention locks
        let locks = self.retention_locks.read().unwrap();
        if let Some(&expiration) = locks.get(&row.row_id) {
            let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();
            if now < expiration {
                return false;
            }
        }

        true
    }

    /// Set a retention lock on a row
    pub fn set_retention_lock(&self, row_id: RowId, expiration: u64) {
        let mut locks = self.retention_locks.write().unwrap();
        locks.insert(row_id, expiration);
    }

    /// Remove a retention lock
    pub fn remove_retention_lock(&self, row_id: RowId) {
        let mut locks = self.retention_locks.write().unwrap();
        locks.remove(&row_id);
    }

    /// Get all active legal holds
    pub fn get_active_legal_holds(&self) -> Vec<LegalHold> {
        let holds = self.legal_holds.read().unwrap();
        holds.values()
            .filter(|h| h.is_active())
            .cloned()
            .collect()
    }

    /// Get all policies
    pub fn get_all_policies(&self) -> Vec<RetentionPolicy> {
        let policies = self.policies.read().unwrap();
        policies.values().cloned().collect()
    }
}

impl Default for RetentionManager {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Retention Enforcement
// ============================================================================

/// Enforces retention policies
pub struct RetentionEnforcer {
    /// Retention manager
    manager: Arc<RetentionManager>,
}

impl RetentionEnforcer {
    /// Create a new enforcer
    pub fn new(manager: Arc<RetentionManager>) -> Self {
        Self { manager }
    }

    /// Find rows eligible for expiration
    pub fn find_expired_rows(&self, rows: &[LedgerRow]) -> Vec<RowId> {
        let mut expired = Vec::new();

        for row in rows {
            if self.manager.can_delete(row) {
                // Check if retention period has expired
                if let Some(policy) = self.manager.get_table_policy(row.table_id) {
                    if !policy.should_retain(row.timestamp) {
                        expired.push(row.row_id);
                    }
                }
            }
        }

        expired
    }

    /// Generate expiration report
    pub fn generate_expiration_report(&self, rows: &[LedgerRow]) -> ExpirationReport {
        let expired = self.find_expired_rows(rows);
        let under_hold: Vec<RowId> = rows.iter()
            .filter(|r| self.manager.is_under_legal_hold(r.row_id, r.table_id, r.block_id))
            .map(|r| r.row_id)
            .collect();

        let mut expiring_soon = Vec::new();
        let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();
        let seven_days = 7 * 86400;

        for row in rows {
            if let Some(policy) = self.manager.get_table_policy(row.table_id) {
                if let Some(exp_time) = policy.expiration_time(row.timestamp) {
                    if exp_time > now && exp_time - now < seven_days {
                        expiring_soon.push(row.row_id);
                    }
                }
            }
        }

        ExpirationReport {
            timestamp: now,
            total_rows: rows.len(),
            expired_rows: expired.len(),
            under_legal_hold: under_hold.len(),
            expiring_soon: expiring_soon.len(),
            expired_row_ids: expired,
            held_row_ids: under_hold,
            expiring_soon_row_ids: expiring_soon,
        }
    }
}

/// Expiration report
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExpirationReport {
    /// Report timestamp
    pub timestamp: u64,
    /// Total rows checked
    pub total_rows: usize,
    /// Number of expired rows
    pub expired_rows: usize,
    /// Number of rows under legal hold
    pub under_legal_hold: usize,
    /// Number of rows expiring soon
    pub expiring_soon: usize,
    /// IDs of expired rows
    pub expired_row_ids: Vec<RowId>,
    /// IDs of rows under legal hold
    pub held_row_ids: Vec<RowId>,
    /// IDs of rows expiring soon
    pub expiring_soon_row_ids: Vec<RowId>,
}

// ============================================================================
// Compliance Reporting
// ============================================================================

/// Compliance report
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComplianceReport {
    /// Report timestamp
    pub timestamp: u64,
    /// Report period (start, end)
    pub period: (u64, u64),
    /// Policies in effect
    pub policies: Vec<PolicyCompliance>,
    /// Legal holds
    pub legal_holds: Vec<LegalHoldCompliance>,
    /// Retention statistics
    pub statistics: RetentionStatistics,
}

/// Policy compliance details
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolicyCompliance {
    /// Policy ID
    pub policy_id: String,
    /// Policy name
    pub name: String,
    /// Retention period
    pub period: RetentionPeriod,
    /// Tables using this policy
    pub tables: Vec<TableId>,
    /// Compliance status
    pub compliant: bool,
    /// Issues (if any)
    pub issues: Vec<String>,
}

/// Legal hold compliance details
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LegalHoldCompliance {
    /// Hold ID
    pub hold_id: String,
    /// Case ID
    pub case_id: String,
    /// Status
    pub status: LegalHoldStatus,
    /// Duration (days)
    pub duration_days: u64,
    /// Rows affected
    pub rows_affected: usize,
}

/// Retention statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetentionStatistics {
    /// Total rows
    pub total_rows: u64,
    /// Rows under retention
    pub rows_under_retention: u64,
    /// Rows under legal hold
    pub rows_under_legal_hold: u64,
    /// Rows eligible for deletion
    pub rows_eligible_for_deletion: u64,
    /// Average retention period (days)
    pub avg_retention_days: f64,
}

/// Compliance reporter
pub struct ComplianceReporter {
    /// Retention manager
    manager: Arc<RetentionManager>,
}

impl ComplianceReporter {
    /// Create a new compliance reporter
    pub fn new(manager: Arc<RetentionManager>) -> Self {
        Self { manager }
    }

    /// Generate a compliance report
    pub fn generate_report(&self, start_time: u64, end_time: u64) -> ComplianceReport {
        let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();

        // Get all policies
        let policies = self.manager.get_all_policies();
        let policy_compliance: Vec<PolicyCompliance> = policies.iter()
            .map(|p| {
                let table_policies = self.manager.table_policies.read().unwrap();
                let tables: Vec<TableId> = table_policies.iter()
                    .filter(|(_, pid)| *pid == &p.policy_id)
                    .map(|(tid, _)| *tid)
                    .collect();

                PolicyCompliance {
                    policy_id: p.policy_id.clone(),
                    name: p.name.clone(),
                    period: p.period,
                    tables,
                    compliant: p.active,
                    issues: Vec::new(),
                }
            })
            .collect();

        // Get legal holds
        let holds = self.manager.legal_holds.read().unwrap();
        let hold_compliance: Vec<LegalHoldCompliance> = holds.values()
            .map(|h| {
                let duration_days = (now - h.started_at) / 86400;
                LegalHoldCompliance {
                    hold_id: h.hold_id.clone(),
                    case_id: h.case_id.clone(),
                    status: h.status,
                    duration_days,
                    rows_affected: h.rows.len(),
                }
            })
            .collect();

        let statistics = RetentionStatistics {
            total_rows: 0, // Would need to be calculated from actual data
            rows_under_retention: 0,
            rows_under_legal_hold: holds.values()
                .filter(|h| h.is_active())
                .map(|h| h.rows.len() as u64)
                .sum(),
            rows_eligible_for_deletion: 0,
            avg_retention_days: 0.0,
        };

        ComplianceReport {
            timestamp: now,
            period: (start_time, end_time),
            policies: policy_compliance,
            legal_holds: hold_compliance,
            statistics,
        }
    }

    /// Export report to JSON
    pub fn export_json(&self, report: &ComplianceReport) -> Result<String> {
        serde_json::to_string_pretty(report)
            .map_err(|e| DbError::Serialization(format!("Failed to serialize report: {}", e)))
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_retention_period() {
        let p1 = RetentionPeriod::Days(30);
        let p2 = RetentionPeriod::Years(7);

        assert_eq!(p1.to_seconds(), Some(30 * 86400));
        assert!(p2.is_longer_than(&p1));
    }

    #[test]
    fn test_retention_policy() {
        let _policy = RetentionPolicy::new(
            "policy1".to_string(),
            "30 Day Retention".to_string(),
            RetentionPeriod::Days(30),
            "Standard retention".to_string(),
        );

        let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();
        assert!(policy.should_retain(now));
        assert!(!policy.should_retain(now - 31 * 86400));
    }

    #[test]
    fn test_legal_hold() {
        let mut hold = LegalHold::new(
            "hold1".to_string(),
            "case123".to_string(),
            "Litigation hold".to_string(),
            "Legal Dept".to_string(),
        );

        assert!(hold.is_active());

        hold.add_row(1);
        hold.add_row(2);
        assert!(hold.covers_row(1));
        assert!(!hold.covers_row(3));

        hold.release();
        assert!(!hold.is_active());
    }

    #[test]
    fn test_retention_manager() {
        let manager = RetentionManager::new();

        let _policy = RetentionPolicy::new(
            "policy1".to_string(),
            "Test Policy".to_string(),
            RetentionPeriod::Days(30),
            "Test".to_string(),
        );

        manager.add_policy(policy).unwrap();
        manager.assign_policy_to_table(1, "policy1".to_string()).unwrap();

        let retrieved = manager.get_table_policy(1);
        assert!(retrieved.is_some());
    }
}


