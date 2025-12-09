// # XA Transactions
//
// Two-phase commit protocol for distributed transactions across multiple databases.
// Implements XA transaction management with heuristic completion support.

use std::collections::VecDeque;
use std::collections::HashSet;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap};
use std::sync::Arc;
use parking_lot::RwLock;
use std::time::{Duration, SystemTime};
use crate::error::DbError;

type Result<T> = std::result::Result<T, DbError>;

/// Global transaction identifier
#[derive(Debug, Clone, Serialize, Deserialize, Hash, Eq, PartialEq)]
pub struct Xid {
    /// Format identifier
    pub format_id: i32,
    /// Global transaction ID
    pub global_txn_id: Vec<u8>,
    /// Branch qualifier
    pub branch_qualifier: Vec<u8>,
}

impl Xid {
    /// Create a new XID
    pub fn new(format_id: i32, global_txn_id: Vec<u8>, branch_qualifier: Vec<u8>) -> Self {
        Self {
            format_id,
            global_txn_id,
            branch_qualifier,
        }
    }

    /// Generate a new XID
    pub fn generate() -> Self {
        let uuid = uuid::Uuid::new_v4();
        Self {
            format_id: 1,
            global_txn_id: uuid.as_bytes().to_vec(),
            branch_qualifier: vec![0; 8],
        }
    }
}

/// XA transaction state
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum XaState {
    /// Transaction started
    Active,
    /// Idle (between operations)
    Idle,
    /// Prepared (ready to commit)
    Prepared,
    /// Committed
    Committed,
    /// Rolled back
    RolledBack,
    /// In-doubt (coordinator failure during 2PC)
    InDoubt,
    /// Heuristically committed
    HeuristicallyCommitted,
    /// Heuristically rolled back
    HeuristicallyRolledBack,
    /// Heuristic mixed (some branches committed, some rolled back)
    HeuristicMixed,
}

/// XA transaction
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct XaTransaction {
    /// Transaction ID
    pub xid: Xid,
    /// Transaction state
    pub state: XaState,
    /// Participating resource managers
    pub resource_managers: Vec<String>,
    /// Branches that have prepared
    pub prepared_branches: HashSet<String>,
    /// Branches that have committed
    pub committed_branches: HashSet<String>,
    /// Branches that have rolled back
    pub rolled_back_branches: HashSet<String>,
    /// Start time
    pub start_time: u64,
    /// Timeout (seconds)
    pub timeout: u64,
    /// Last activity
    pub last_activity: u64,
}

/// Resource manager (database participant)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceManager {
    /// RM identifier
    pub id: String,
    /// RM name
    pub name: String,
    /// Connection string
    pub connection: String,
    /// Current state
    pub state: RmState,
}

/// Resource manager state
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum RmState {
    Available,
    Busy,
    Failed,
    Recovering,
}

/// Two-phase commit vote
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum Vote {
    /// Ready to commit
    VoteCommit,
    /// Cannot commit, must rollback
    VoteRollback,
    /// No response yet
    VoteNone,
}

/// Heuristic decision
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum HeuristicDecision {
    /// Heuristically commit
    Commit,
    /// Heuristically rollback
    Rollback,
    /// No heuristic decision
    None,
}

/// XA transaction manager (coordinator)
pub struct XaTransactionManager {
    /// Active XA transactions
    transactions: Arc<RwLock<HashMap<Xid, XaTransaction>>>,
    /// Registered resource managers
    resource_managers: Arc<RwLock<HashMap<String, ResourceManager>>>,
    /// In-doubt transactions (for recovery)
    in_doubt_txns: Arc<RwLock<HashMap<Xid, XaTransaction>>>,
    /// Transaction log
    txn_log: Arc<RwLock<VecDeque<XaLogEntry>>>,
    /// Statistics
    stats: Arc<RwLock<XaStats>>,
}

/// XA transaction log entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct XaLogEntry {
    /// Entry ID
    pub id: String,
    /// Transaction ID
    pub xid: Xid,
    /// Log entry type
    pub entry_type: LogEntryType,
    /// Timestamp
    pub timestamp: u64,
    /// Additional data
    pub data: HashMap<String, String>,
}

/// Type of log entry
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum LogEntryType {
    Start,
    Prepare,
    Commit,
    Rollback,
    HeuristicCommit,
    HeuristicRollback,
    Forget,
}

/// XA statistics with cache alignment
#[repr(C, align(64))]
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct XaStats {
    pub total_transactions: u64,
    pub active_transactions: u64,
    pub committed_transactions: u64,
    pub rolled_back_transactions: u64,
    pub in_doubt_transactions: u64,
    pub heuristic_transactions: u64,
    pub avg_transaction_duration_ms: f64,
    pub prepare_failures: u64,
    pub commit_failures: u64,
}

impl XaTransactionManager {
    /// Create a new XA transaction manager
    pub fn new() -> Self {
        Self {
            transactions: Arc::new(RwLock::new(HashMap::new())),
            resource_managers: Arc::new(RwLock::new(HashMap::new())),
            in_doubt_txns: Arc::new(RwLock::new(HashMap::new())),
            txn_log: Arc::new(RwLock::new(VecDeque::new())),
            stats: Arc::new(RwLock::new(XaStats::default())),
        }
    }

    /// Register a resource manager
    pub fn register_resource_manager(&self, rm: ResourceManager) -> Result<()> {
        let mut rms = self.resource_managers.write();

        if rms.contains_key(&rm.id) {
            return Err(DbError::Replication(
                format!("Resource manager {} already registered", rm.id)
            ));
        }

        rms.insert(rm.id.clone(), rm);
        Ok(())
    }

    /// Start an XA transaction
    pub fn xa_start(&self, xid: Xid, rms: Vec<String>, timeout: u64) -> Result<()> {
        let mut transactions = self.transactions.write();

        if transactions.contains_key(&xid) {
            return Err(DbError::Replication(
                format!("Transaction {:?} already exists", xid)
            ));
        }

        let txn = XaTransaction {
            xid: xid.clone(),
            state: XaState::Active,
            resource_managers: rms,
            prepared_branches: HashSet::new(),
            committed_branches: HashSet::new(),
            rolled_back_branches: HashSet::new(),
            start_time: Self::current_timestamp(),
            timeout,
            last_activity: Self::current_timestamp(),
        };

        transactions.insert(xid.clone(), txn);

        // Log
        self.log_entry(XaLogEntry {
            id: format!("log-{}", uuid::Uuid::new_v4()),
            xid,
            entry_type: LogEntryType::Start,
            timestamp: Self::current_timestamp(),
            data: HashMap::new(),
        });

        let mut stats = self.stats.write();
        stats.total_transactions += 1;
        stats.active_transactions += 1;

        Ok(())
    }

    /// End an XA transaction (prepare phase)
    pub fn xa_end(&self, xid: &Xid) -> Result<()> {
        let mut transactions = self.transactions.write();

        let txn = transactions.get_mut(xid)
            .ok_or_else(|| DbError::Replication(
                format!("Transaction {:?} not found", xid)
            ))?;

        if txn.state != XaState::Active {
            return Err(DbError::Replication(
                format!("Transaction {:?} not active", xid)
            ));
        }

        txn.state = XaState::Idle;
        txn.last_activity = Self::current_timestamp();

        Ok(())
    }

    /// Prepare phase of two-phase commit
    pub async fn xa_prepare(&self, xid: &Xid) -> Result<Vec<Vote>> {
        let txn = {
            let transactions = self.transactions.read();
            transactions.get(xid)
                .ok_or_else(|| DbError::Replication(
                    format!("Transaction {:?} not found", xid)
                ))?
                .clone()
        };

        if txn.state != XaState::Idle && txn.state != XaState::Active {
            return Err(DbError::Replication(
                format!("Transaction {:?} not in correct state for prepare", xid)
            ));
        }

        // Send prepare to all resource managers
        let mut votes = Vec::new();

        for rm_id in &txn.resource_managers {
            let vote = self.send_prepare(rm_id, xid).await?;
            votes.push(vote.clone());

            if vote == Vote::VoteCommit {
                let mut transactions = self.transactions.write();
                if let Some(txn) = transactions.get_mut(xid) {
                    txn.prepared_branches.insert(rm_id.clone());
                }
            }
        }

        // Update transaction state
        {
            let mut transactions = self.transactions.write();
            if let Some(txn) = transactions.get_mut(xid) {
                if votes.iter().all(|v| *v == Vote::VoteCommit) {
                    txn.state = XaState::Prepared;
                } else {
                    txn.state = XaState::Active;

                    let mut stats = self.stats.write();
                    stats.prepare_failures += 1;
                }
                txn.last_activity = Self::current_timestamp();
            }
        }

        // Log
        self.log_entry(XaLogEntry {
            id: format!("log-{}", uuid::Uuid::new_v4()),
            xid: xid.clone(),
            entry_type: LogEntryType::Prepare,
            timestamp: Self::current_timestamp(),
            data: HashMap::new(),
        });

        Ok(votes)
    }

    /// Send prepare request to a resource manager
    async fn send_prepare(&self, rm_id: &str, _xid: &Xid) -> Result<Vote> {
        let rms = self.resource_managers.read();

        let _rm = rms.get(rm_id)
            .ok_or_else(|| DbError::Replication(
                format!("Resource manager {} not found", rm_id)
            ))?;

        // In a real implementation, would send prepare over network
        // For now, simulate with a delay and random vote
        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

        // 95% vote commit
        if rand::random::<f64>() < 0.95 {
            Ok(Vote::VoteCommit)
        } else {
            Ok(Vote::VoteRollback)
        }
    }

    /// Commit phase of two-phase commit
    pub async fn xa_commit(&self, xid: &Xid, one_phase: bool) -> Result<()> {
        if one_phase {
            // One-phase commit optimization (single RM)
            return self.xa_commit_one_phase(xid).await;
        }

        let txn = {
            let transactions = self.transactions.read();
            transactions.get(xid)
                .ok_or_else(|| DbError::Replication(
                    format!("Transaction {:?} not found", xid)
                ))?
                .clone()
        };

        if txn.state != XaState::Prepared {
            return Err(DbError::Replication(
                format!("Transaction {:?} not prepared", xid)
            ));
        }

        // Send commit to all prepared resource managers
        for rm_id in &txn.prepared_branches {
            match self.send_commit(rm_id, xid).await {
                Ok(_) => {
                    let mut transactions = self.transactions.write();
                    if let Some(txn) = transactions.get_mut(xid) {
                        txn.committed_branches.insert(rm_id.clone());
                    }
                }
                Err(e) => {
                    // Commit failure - this is a heuristic situation
                    eprintln!("Commit failed for RM {}: {}", rm_id, e);

                    let mut stats = self.stats.write();
                    stats.commit_failures += 1;
                }
            }
        }

        // Update transaction state
        {
            let mut transactions = self.transactions.write();
            if let Some(mut txn) = transactions.remove(xid) {
                if txn.committed_branches.len() == txn.prepared_branches.len() {
                    txn.state = XaState::Committed;
                } else {
                    // Some branches didn't commit - heuristic mixed
                    txn.state = XaState::HeuristicMixed;

                    // Move to in-doubt transactions
                    let mut in_doubt = self.in_doubt_txns.write();
                    in_doubt.insert(xid.clone(), txn.clone());

                    let mut stats = self.stats.write();
                    stats.in_doubt_transactions += 1;
                }

                let duration = Self::current_timestamp() - txn.start_time;

                let mut stats = self.stats.write();
                stats.committed_transactions += 1;
                stats.active_transactions = stats.active_transactions.saturating_sub(1);

                stats.avg_transaction_duration_ms =
                    (stats.avg_transaction_duration_ms * (stats.committed_transactions - 1) as f64
                     + duration as f64) / stats.committed_transactions as f64;
            }
        }

        // Log
        self.log_entry(XaLogEntry {
            id: format!("log-{}", uuid::Uuid::new_v4()),
            xid: xid.clone(),
            entry_type: LogEntryType::Commit,
            timestamp: Self::current_timestamp(),
            data: HashMap::new(),
        });

        Ok(())
    }

    /// One-phase commit optimization
    async fn xa_commit_one_phase(&self, xid: &Xid) -> Result<()> {
        let txn = {
            let transactions = self.transactions.read();
            transactions.get(xid)
                .ok_or_else(|| DbError::Replication(
                    format!("Transaction {:?} not found", xid)
                ))?
                .clone()
        };

        if txn.resource_managers.len() != 1 {
            return Err(DbError::Replication(
                "One-phase commit requires exactly one resource manager".to_string()
            ));
        }

        let rm_id = &txn.resource_managers[0];
        self.send_commit(rm_id, xid).await?;

        // Update state
        {
            let mut transactions = self.transactions.write();
            if let Some(mut txn) = transactions.remove(xid) {
                txn.state = XaState::Committed;
                txn.committed_branches.insert(rm_id.clone());

                let mut stats = self.stats.write();
                stats.committed_transactions += 1;
                stats.active_transactions = stats.active_transactions.saturating_sub(1);
            }
        }

        Ok(())
    }

    /// Send commit request to a resource manager
    async fn send_commit(&self, rm_id: &str, _xid: &Xid) -> Result<()> {
        let rms = self.resource_managers.read();

        let _rm = rms.get(rm_id)
            .ok_or_else(|| DbError::Replication(
                format!("Resource manager {} not found", rm_id)
            ))?;

        // In a real implementation, would send commit over network
        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

        // 99% success rate
        if rand::random::<f64>() < 0.99 {
            Ok(())
        } else {
            Err(DbError::Replication("Commit failed".to_string()))
        }
    }

    /// Rollback an XA transaction
    pub async fn xa_rollback(&self, xid: &Xid) -> Result<()> {
        let txn = {
            let transactions = self.transactions.read();
            transactions.get(xid)
                .ok_or_else(|| DbError::Replication(
                    format!("Transaction {:?} not found", xid)
                ))?
                .clone()
        };

        // Send rollback to all resource managers
        for rm_id in &txn.resource_managers {
            match self.send_rollback(rm_id, xid).await {
                Ok(_) => {
                    let mut transactions = self.transactions.write();
                    if let Some(txn) = transactions.get_mut(xid) {
                        txn.rolled_back_branches.insert(rm_id.clone());
                    }
                }
                Err(e) => {
                    eprintln!("Rollback failed for RM {}: {}", rm_id, e);
                }
            }
        }

        // Update transaction state
        {
            let mut transactions = self.transactions.write();
            if let Some(mut txn) = transactions.remove(xid) {
                txn.state = XaState::RolledBack;

                let mut stats = self.stats.write();
                stats.rolled_back_transactions += 1;
                stats.active_transactions = stats.active_transactions.saturating_sub(1);
            }
        }

        // Log
        self.log_entry(XaLogEntry {
            id: format!("log-{}", uuid::Uuid::new_v4()),
            xid: xid.clone(),
            entry_type: LogEntryType::Rollback,
            timestamp: Self::current_timestamp(),
            data: HashMap::new(),
        });

        Ok(())
    }

    /// Send rollback request to a resource manager
    async fn send_rollback(&self, rm_id: &str, _xid: &Xid) -> Result<()> {
        let rms = self.resource_managers.read();

        let _rm = rms.get(rm_id)
            .ok_or_else(|| DbError::Replication(
                format!("Resource manager {} not found", rm_id)
            ))?;

        // In a real implementation, would send rollback over network
        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

        Ok(())
    }

    /// Recover in-doubt transactions
    pub async fn xa_recover(&self) -> Result<Vec<Xid>> {
        // Query all resource managers for prepared transactions
        let rms = self.resource_managers.read();
        let mut recovered_xids = HashSet::new();

        for (rm_id, _rm) in rms.iter() {
            let xids = self.query_prepared_transactions(rm_id).await?;
            recovered_xids.extend(xids);
        }

        Ok(recovered_xids.into_iter().collect())
    }

    /// Query prepared transactions from a resource manager
    async fn query_prepared_transactions(&self, _rm_id: &str) -> Result<Vec<Xid>> {
        // In a real implementation, would query the RM
        // For now, return empty list
        Ok(Vec::new())
    }

    /// Forget a heuristically completed transaction
    pub fn xa_forget(&self, xid: &Xid) -> Result<()> {
        let mut in_doubt = self.in_doubt_txns.write();

        in_doubt.remove(xid)
            .ok_or_else(|| DbError::Replication(
                format!("In-doubt transaction {:?} not found", xid)
            ))?;

        // Log
        self.log_entry(XaLogEntry {
            id: format!("log-{}", uuid::Uuid::new_v4()),
            xid: xid.clone(),
            entry_type: LogEntryType::Forget,
            timestamp: Self::current_timestamp(),
            data: HashMap::new(),
        });

        let mut stats = self.stats.write();
        stats.in_doubt_transactions = stats.in_doubt_transactions.saturating_sub(1);

        Ok(())
    }

    /// Heuristically complete an in-doubt transaction
    pub async fn heuristic_commit(&self, xid: &Xid) -> Result<()> {
        let mut in_doubt = self.in_doubt_txns.write();

        let mut txn = in_doubt.remove(xid)
            .ok_or_else(|| DbError::Replication(
                format!("In-doubt transaction {:?} not found", xid)
            ))?;

        // Attempt to commit remaining branches
        for rm_id in &txn.resource_managers {
            if !txn.committed_branches.contains(rm_id) {
                match self.send_commit(rm_id, xid).await {
                    Ok(_) => {
                        txn.committed_branches.insert(rm_id.clone());
                    }
                    Err(e) => {
                        eprintln!("Heuristic commit failed for RM {}: {}", rm_id, e);
                    }
                }
            }
        }

        txn.state = XaState::HeuristicallyCommitted;

        // Log
        self.log_entry(XaLogEntry {
            id: format!("log-{}", uuid::Uuid::new_v4()),
            xid: xid.clone(),
            entry_type: LogEntryType::HeuristicCommit,
            timestamp: Self::current_timestamp(),
            data: HashMap::new(),
        });

        let mut stats = self.stats.write();
        stats.heuristic_transactions += 1;
        stats.in_doubt_transactions = stats.in_doubt_transactions.saturating_sub(1);

        Ok(())
    }

    /// Heuristically rollback an in-doubt transaction
    pub async fn heuristic_rollback(&self, xid: &Xid) -> Result<()> {
        let mut in_doubt = self.in_doubt_txns.write();

        let mut txn = in_doubt.remove(xid)
            .ok_or_else(|| DbError::Replication(
                format!("In-doubt transaction {:?} not found", xid)
            ))?;

        // Attempt to rollback all branches
        for rm_id in &txn.resource_managers {
            match self.send_rollback(rm_id, xid).await {
                Ok(_) => {
                    txn.rolled_back_branches.insert(rm_id.clone());
                }
                Err(e) => {
                    eprintln!("Heuristic rollback failed for RM {}: {}", rm_id, e);
                }
            }
        }

        txn.state = XaState::HeuristicallyRolledBack;

        // Log
        self.log_entry(XaLogEntry {
            id: format!("log-{}", uuid::Uuid::new_v4()),
            xid: xid.clone(),
            entry_type: LogEntryType::HeuristicRollback,
            timestamp: Self::current_timestamp(),
            data: HashMap::new(),
        });

        let mut stats = self.stats.write();
        stats.heuristic_transactions += 1;
        stats.in_doubt_transactions = stats.in_doubt_transactions.saturating_sub(1);

        Ok(())
    }

    /// Get transaction state
    #[inline]
    pub fn get_transaction(&self, xid: &Xid) -> Option<XaTransaction> {
        self.transactions.read().get(xid).cloned()
    }

    /// Get in-doubt transactions
    #[inline]
    pub fn get_in_doubt_transactions(&self) -> Vec<XaTransaction> {
        self.in_doubt_txns.read().values().cloned().collect()
    }

    /// Get statistics
    #[inline]
    pub fn get_stats(&self) -> XaStats {
        self.stats.read().clone()
    }

    /// Log an entry
    #[inline]
    fn log_entry(&self, entry: XaLogEntry) {
        let mut log = self.txn_log.write();
        log.push_back(entry);

        // Keep only last 10000 entries
        while log.len() > 10000 {
            log.pop_front();
        }
    }

    /// Get transaction log
    pub fn get_log(&self) -> Vec<XaLogEntry> {
        self.txn_log.read().iter().cloned().collect()
    }

    /// Current timestamp in milliseconds
    fn current_timestamp() -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64
    }
}

impl Default for XaTransactionManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_xa_transaction() {
        let xa_mgr = XaTransactionManager::new();

        // Register resource manager
        let rm = ResourceManager {
            id: "rm-1".to_string(),
            name: "Database 1".to_string(),
            connection: "localhost:5432".to_string(),
            state: RmState::Available,
        };

        xa_mgr.register_resource_manager(rm).unwrap();

        // Start XA transaction
        let xid = Xid::generate();
        xa_mgr.xa_start(xid.clone(), vec!["rm-1".to_string()], 30).unwrap();

        // End transaction
        xa_mgr.xa_end(&xid).unwrap();

        // Prepare
        let votes = xa_mgr.xa_prepare(&xid).await.unwrap();
        assert_eq!(votes.len(), 1);

        // Commit (one-phase)
        xa_mgr.xa_commit(&xid, true).await.unwrap();

        let _stats = xa_mgr.get_stats();
        assert_eq!(stats.total_transactions, 1);
    }
}
