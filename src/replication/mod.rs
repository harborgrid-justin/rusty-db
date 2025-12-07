use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use parking_lot::RwLock;
use std::sync::Arc;
use tokio::sync::mpsc;
use crate::Result;
use crate::error::DbError;

/// Replication mode
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ReplicationMode {
    Synchronous,   // Wait for replica acknowledgment
    Asynchronous,  // Don't wait for replica
    SemiSync,      // Wait for at least one replica
}

/// Replica status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ReplicaStatus {
    Active,
    Lagging,
    Disconnected,
    Syncing,
}

/// Replica node information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReplicaNode {
    pub id: String,
    pub address: String,
    pub status: ReplicaStatus,
    pub lag_bytes: u64,
    pub last_sync: i64,  // Timestamp
}

/// Replication log entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReplicationLogEntry {
    pub sequence_number: u64,
    pub operation: ReplicationOperation,
    pub timestamp: i64,
    pub data: Vec<u8>,
}

/// Type of replication operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ReplicationOperation {
    Insert,
    Update,
    Delete,
    CreateTable,
    DropTable,
    AlterTable,
}

/// Replication manager
pub struct ReplicationManager {
    mode: ReplicationMode,
    replicas: Arc<RwLock<HashMap<String, ReplicaNode>>>,
    log_sequence: Arc<RwLock<u64>>,
    log_sender: Option<mpsc::UnboundedSender<ReplicationLogEntry>>,
    is_primary: bool,
}

impl ReplicationManager {
    pub fn new(mode: ReplicationMode, is_primary: bool) -> Self {
        let (tx, _rx) = mpsc::unbounded_channel();
        
        Self {
            mode,
            replicas: Arc::new(RwLock::new(HashMap::new())),
            log_sequence: Arc::new(RwLock::new(0)),
            log_sender: Some(tx),
            is_primary,
        }
    }
    
    /// Add a replica node
    pub fn add_replica(&self, replica: ReplicaNode) -> Result<()> {
        if !self.is_primary {
            return Err(DbError::InvalidOperation(
                "Cannot add replicas to a non-primary node".to_string()
            ));
        }
        
        let mut replicas = self.replicas.write();
        replicas.insert(replica.id.clone(), replica);
        Ok(())
    }
    
    /// Remove a replica node
    pub fn remove_replica(&self, replica_id: &str) -> Result<()> {
        let mut replicas = self.replicas.write();
        
        if replicas.remove(replica_id).is_none() {
            return Err(DbError::NotFound(
                format!("Replica '{}' not found", replica_id)
            ));
        }
        
        Ok(())
    }
    
    /// Get all replicas
    pub fn get_replicas(&self) -> Vec<ReplicaNode> {
        let replicas = self.replicas.read();
        replicas.values().cloned().collect()
    }
    
    /// Update replica status
    pub fn update_replica_status(&self, replica_id: &str, status: ReplicaStatus) -> Result<()> {
        let mut replicas = self.replicas.write();
        
        if let Some(replica) = replicas.get_mut(replica_id) {
            replica.status = status;
            Ok(())
        } else {
            Err(DbError::NotFound(
                format!("Replica '{}' not found", replica_id)
            ))
        }
    }
    
    /// Replicate an operation to all replicas
    pub async fn replicate_operation(
        &self,
        operation: ReplicationOperation,
        data: Vec<u8>,
    ) -> Result<()> {
        if !self.is_primary {
            return Err(DbError::InvalidOperation(
                "Only primary can replicate operations".to_string()
            ));
        }
        
        // Create log entry
        let mut seq = self.log_sequence.write();
        *seq += 1;
        let sequence_number = *seq;
        drop(seq);
        
        let entry = ReplicationLogEntry {
            sequence_number,
            operation,
            timestamp: chrono::Utc::now().timestamp(),
            data,
        };
        
        // Send to replication channel
        if let Some(sender) = &self.log_sender {
            sender.send(entry.clone())
                .map_err(|e| DbError::Internal(format!("Failed to send log entry: {}", e)))?;
        }
        
        // Handle based on replication mode
        match self.mode {
            ReplicationMode::Synchronous => {
                self.wait_for_all_replicas(&entry).await?;
            }
            ReplicationMode::SemiSync => {
                self.wait_for_one_replica(&entry).await?;
            }
            ReplicationMode::Asynchronous => {
                // Fire and forget
            }
        }
        
        Ok(())
    }
    
    /// Wait for all replicas to acknowledge
    async fn wait_for_all_replicas(&self, _entry: &ReplicationLogEntry) -> Result<()> {
        // TODO: Implement actual acknowledgment waiting
        // For now, just return success
        Ok(())
    }
    
    /// Wait for at least one replica to acknowledge
    async fn wait_for_one_replica(&self, _entry: &ReplicationLogEntry) -> Result<()> {
        // TODO: Implement acknowledgment from one replica
        Ok(())
    }
    
    /// Initiate failover to a replica
    pub async fn failover(&self, new_primary_id: &str) -> Result<()> {
        let replicas = self.replicas.read();
        
        if !replicas.contains_key(new_primary_id) {
            return Err(DbError::NotFound(
                format!("Replica '{}' not found", new_primary_id)
            ));
        }
        
        // TODO: Implement actual failover logic:
        // 1. Verify replica is caught up
        // 2. Promote replica to primary
        // 3. Update connection routing
        // 4. Notify other replicas of new primary
        
        Ok(())
    }
    
    /// Get replication lag for a specific replica
    pub fn get_replica_lag(&self, replica_id: &str) -> Result<u64> {
        let replicas = self.replicas.read();
        
        if let Some(replica) = replicas.get(replica_id) {
            Ok(replica.lag_bytes)
        } else {
            Err(DbError::NotFound(
                format!("Replica '{}' not found", replica_id)
            ))
        }
    }
}

// Note: chrono is not in dependencies, so using a placeholder
// In production, add chrono to Cargo.toml
mod chrono {
    pub struct Utc;
    impl Utc {
        pub fn now() -> DateTime {
            DateTime
        }
    }
    pub struct DateTime;
    impl DateTime {
        pub fn timestamp(&self) -> i64 {
            0
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_add_replica() -> Result<()> {
        let rm = ReplicationManager::new(ReplicationMode::Asynchronous, true);
        
        let replica = ReplicaNode {
            id: "replica-1".to_string(),
            address: "127.0.0.1:5433".to_string(),
            status: ReplicaStatus::Active,
            lag_bytes: 0,
            last_sync: 0,
        };
        
        rm.add_replica(replica)?;
        
        let replicas = rm.get_replicas();
        assert_eq!(replicas.len(), 1);
        assert_eq!(replicas[0].id, "replica-1");
        
        Ok(())
    }
    
    #[test]
    fn test_remove_replica() -> Result<()> {
        let rm = ReplicationManager::new(ReplicationMode::Synchronous, true);
        
        let replica = ReplicaNode {
            id: "replica-2".to_string(),
            address: "127.0.0.1:5434".to_string(),
            status: ReplicaStatus::Active,
            lag_bytes: 0,
            last_sync: 0,
        };
        
        rm.add_replica(replica)?;
        assert_eq!(rm.get_replicas().len(), 1);
        
        rm.remove_replica("replica-2")?;
        assert_eq!(rm.get_replicas().len(), 0);
        
        Ok(())
    }
    
    #[test]
    fn test_replica_status_update() -> Result<()> {
        let rm = ReplicationManager::new(ReplicationMode::SemiSync, true);
        
        let replica = ReplicaNode {
            id: "replica-3".to_string(),
            address: "127.0.0.1:5435".to_string(),
            status: ReplicaStatus::Syncing,
            lag_bytes: 1024,
            last_sync: 0,
        };
        
        rm.add_replica(replica)?;
        rm.update_replica_status("replica-3", ReplicaStatus::Active)?;
        
        let replicas = rm.get_replicas();
        assert_eq!(replicas[0].status, ReplicaStatus::Active);
        
        Ok(())
    }
    
    #[test]
    fn test_non_primary_cannot_add_replicas() -> Result<()> {
        let rm = ReplicationManager::new(ReplicationMode::Asynchronous, false);
        
        let replica = ReplicaNode {
            id: "replica-4".to_string(),
            address: "127.0.0.1:5436".to_string(),
            status: ReplicaStatus::Active,
            lag_bytes: 0,
            last_sync: 0,
        };
        
        let result = rm.add_replica(replica);
        assert!(result.is_err());
        
        Ok(())
    }
}
