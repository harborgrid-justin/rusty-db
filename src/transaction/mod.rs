use std::collections::{HashMap, HashSet};
use parking_lot::{Mutex, RwLock};
use std::sync::Arc;
use crate::Result;
use crate::error::DbError;

pub type TransactionId = u64;

/// Transaction state
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TransactionState {
    Growing,
    Shrinking,
    Committed,
    Aborted,
}

/// Lock mode
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum LockMode {
    Shared,
    Exclusive,
}

/// Transaction metadata
#[derive(Debug)]
pub struct Transaction {
    pub id: TransactionId,
    pub state: TransactionState,
    pub held_locks: HashSet<String>,
}

impl Transaction {
    pub fn new(id: TransactionId) -> Self {
        Self {
            id,
            state: TransactionState::Growing,
            held_locks: HashSet::new(),
        }
    }
}

/// Lock manager implementing two-phase locking (2PL)
pub struct LockManager {
    lock_table: Arc<RwLock<HashMap<String, Vec<(TransactionId, LockMode)>>>>,
    txn_locks: Arc<RwLock<HashMap<TransactionId, HashSet<String>>>>,
}

impl LockManager {
    pub fn new() -> Self {
        Self {
            lock_table: Arc::new(RwLock::new(HashMap::new())),
            txn_locks: Arc::new(RwLock::new(HashMap::new())),
        }
    }
    
    pub fn acquire_lock(
        &self,
        txn_id: TransactionId,
        resource: String,
        mode: LockMode,
    ) -> Result<()> {
        let mut lock_table = self.lock_table.write();
        let mut txn_locks = self.txn_locks.write();
        
        let holders = lock_table.entry(resource.clone()).or_insert_with(Vec::new);
        
        // Check for conflicts
        for &(holder_id, holder_mode) in holders.iter() {
            if holder_id != txn_id {
                // Conflict detection
                if mode == LockMode::Exclusive || holder_mode == LockMode::Exclusive {
                    return Err(DbError::LockTimeout);
                }
            }
        }
        
        // Grant lock
        holders.push((txn_id, mode));
        txn_locks.entry(txn_id).or_insert_with(HashSet::new).insert(resource);
        
        Ok(())
    }
    
    pub fn release_lock(&self, txn_id: TransactionId, resource: &str) -> Result<()> {
        let mut lock_table = self.lock_table.write();
        let mut txn_locks = self.txn_locks.write();
        
        if let Some(holders) = lock_table.get_mut(resource) {
            holders.retain(|(id, _)| *id != txn_id);
        }
        
        if let Some(locks) = txn_locks.get_mut(&txn_id) {
            locks.remove(resource);
        }
        
        Ok(())
    }
    
    pub fn release_all_locks(&self, txn_id: TransactionId) -> Result<()> {
        let txn_locks = self.txn_locks.read();
        
        if let Some(locks) = txn_locks.get(&txn_id) {
            let resources: Vec<String> = locks.iter().cloned().collect();
            drop(txn_locks);
            
            for resource in resources {
                self.release_lock(txn_id, &resource)?;
            }
        }
        
        Ok(())
    }
}

impl Default for LockManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Transaction manager
pub struct TransactionManager {
    next_txn_id: Arc<Mutex<TransactionId>>,
    active_txns: Arc<RwLock<HashMap<TransactionId, Transaction>>>,
    lock_manager: Arc<LockManager>,
}

impl TransactionManager {
    pub fn new() -> Self {
        Self {
            next_txn_id: Arc::new(Mutex::new(0)),
            active_txns: Arc::new(RwLock::new(HashMap::new())),
            lock_manager: Arc::new(LockManager::new()),
        }
    }
    
    pub fn begin(&self) -> Result<TransactionId> {
        let mut next_id = self.next_txn_id.lock();
        let txn_id = *next_id;
        *next_id += 1;
        
        let txn = Transaction::new(txn_id);
        self.active_txns.write().insert(txn_id, txn);
        
        Ok(txn_id)
    }
    
    pub fn commit(&self, txn_id: TransactionId) -> Result<()> {
        let mut active_txns = self.active_txns.write();
        
        if let Some(txn) = active_txns.get_mut(&txn_id) {
            txn.state = TransactionState::Committed;
            self.lock_manager.release_all_locks(txn_id)?;
            active_txns.remove(&txn_id);
            Ok(())
        } else {
            Err(DbError::Transaction(format!("Transaction {} not found", txn_id)))
        }
    }
    
    pub fn abort(&self, txn_id: TransactionId) -> Result<()> {
        let mut active_txns = self.active_txns.write();
        
        if let Some(txn) = active_txns.get_mut(&txn_id) {
            txn.state = TransactionState::Aborted;
            self.lock_manager.release_all_locks(txn_id)?;
            active_txns.remove(&txn_id);
            Ok(())
        } else {
            Err(DbError::Transaction(format!("Transaction {} not found", txn_id)))
        }
    }
    
    pub fn get_lock_manager(&self) -> Arc<LockManager> {
        self.lock_manager.clone()
    }
}

impl Default for TransactionManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_transaction_lifecycle() -> Result<()> {
        let tm = TransactionManager::new();
        
        let txn_id = tm.begin()?;
        assert!(txn_id == 0);
        
        tm.commit(txn_id)?;
        
        Ok(())
    }
    
    #[test]
    fn test_lock_manager() -> Result<()> {
        let lm = LockManager::new();
        
        lm.acquire_lock(1, "resource1".to_string(), LockMode::Shared)?;
        lm.acquire_lock(2, "resource1".to_string(), LockMode::Shared)?;
        
        lm.release_lock(1, "resource1")?;
        lm.release_lock(2, "resource1")?;
        
        Ok(())
    }
}
