// # Replication Slot Management
//
// This module provides comprehensive replication slot management, supporting
// both physical and logical replication with monitoring, health checks, and
// automatic cleanup.

mod config;
mod errors;
mod manager;
mod manager_impl;
mod types;

// Re-export public types
// pub use config::SlotManagerConfig;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::replication::types::{LogSequenceNumber, ReplicaId};
    use tempfile::TempDir;

    async fn create_test_manager() -> (ReplicationSlotManager, TempDir) {
        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path().to_path_buf();

        let config = SlotManagerConfig {
            storage_path: temp_path.clone(),
            ..SlotManagerConfig::default()
        };

        let manager = ReplicationSlotManager::new(config).await.unwrap();
        (manager, temp_dir)
    }

    #[tokio::test]
    async fn test_slot_name_creation() {
        let name = SlotName::new("test_slot");
        assert!(name.is_ok());

        let empty_name = SlotName::new("");
        assert!(empty_name.is_err());

        let invalid_name = SlotName::new("test@slot");
        assert!(invalid_name.is_err());
    }

    #[tokio::test]
    async fn test_slot_manager_creation() {
        let (_manager, _temp_dir) = create_test_manager().await;
    }

    #[tokio::test]
    async fn test_create_slot() {
        let (manager, _temp_dir) = create_test_manager().await;

        let slot_name = SlotName::new("test_slot").unwrap();
        let replica_id = ReplicaId::new("test_replica").unwrap();

        let slot_id = manager
            .create_slot(slot_name.clone(), replica_id, SlotType::Physical, None)
            .await
            .unwrap();

        assert!(!slot_id.as_str().is_empty());

        let info = manager.get_slot_info(&slot_name).await.unwrap();
        assert_eq!(info.slot_name, slot_name);
        assert_eq!(info.slot_type, SlotType::Physical);
        assert_eq!(info.status, SlotStatus::Active);
    }

    #[tokio::test]
    async fn test_duplicate_slot_creation() {
        let (manager, _temp_dir) = create_test_manager().await;

        let slot_name = SlotName::new("test_slot").unwrap();
        let replica_id = ReplicaId::new("test_replica").unwrap();

        manager
            .create_slot(
                slot_name.clone(),
                replica_id.clone(),
                SlotType::Physical,
                None,
            )
            .await
            .unwrap();

        let result = manager
            .create_slot(slot_name, replica_id, SlotType::Physical, None)
            .await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_list_slots() {
        let (manager, _temp_dir) = create_test_manager().await;

        let replica_id = ReplicaId::new("test_replica").unwrap();

        manager
            .create_slot(
                SlotName::new("slot1").unwrap(),
                replica_id.clone(),
                SlotType::Physical,
                None,
            )
            .await
            .unwrap();

        manager
            .create_slot(
                SlotName::new("slot2").unwrap(),
                replica_id,
                SlotType::Logical,
                None,
            )
            .await
            .unwrap();

        let slots = manager.list_slots().await.unwrap();
        assert_eq!(slots.len(), 2);
    }

    #[tokio::test]
    async fn test_advance_slot() {
        let (manager, _temp_dir) = create_test_manager().await;

        let slot_name = SlotName::new("test_slot").unwrap();
        let replica_id = ReplicaId::new("test_replica").unwrap();

        manager
            .create_slot(slot_name.clone(), replica_id, SlotType::Physical, None)
            .await
            .unwrap();

        let target_lsn = LogSequenceNumber::new(1000);
        assert!(manager.advance_slot(&slot_name, target_lsn).await.is_ok());

        let info = manager.get_slot_info(&slot_name).await.unwrap();
        assert_eq!(info.current_lsn, target_lsn);
    }

    #[tokio::test]
    async fn test_slot_health() {
        let (manager, _temp_dir) = create_test_manager().await;

        let slot_name = SlotName::new("test_slot").unwrap();
        let replica_id = ReplicaId::new("test_replica").unwrap();

        manager
            .create_slot(slot_name.clone(), replica_id, SlotType::Physical, None)
            .await
            .unwrap();

        let health = manager.get_slot_health(&slot_name).await.unwrap();
        assert_eq!(health.status, SlotHealthStatus::Healthy);
    }
}
