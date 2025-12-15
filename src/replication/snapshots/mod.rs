#![allow(dead_code)]
// # Replication Snapshot Management
//
// This module provides comprehensive snapshot management for the replication
// system, supporting both incremental and full backups with compression,
// encryption, and efficient storage management.

mod config;
mod errors;
mod manager;
mod manager_impl;
mod types;

// Re-export public types
// pub use config::CompressionType;
// pub use manager::FileSnapshotManager;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::replication::types::ReplicaId;
    use tempfile::TempDir;

    async fn create_test_manager() -> (FileSnapshotManager, TempDir) {
        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path().to_path_buf();

        let config = SnapshotConfig {
            storage_path: temp_path.clone(),
            temp_directory: temp_path.join("temp"),
            ..SnapshotConfig::default()
        };

        let manager = FileSnapshotManager::new(config).await.unwrap();
        (manager, temp_dir)
    }

    #[tokio::test]
    async fn test_snapshot_id_creation() {
        let id = SnapshotId::new("test-snapshot");
        assert!(id.is_ok());

        let empty_id = SnapshotId::new("");
        assert!(empty_id.is_err());

        let generated_id = SnapshotId::generate();
        assert!(!generated_id.as_str().is_empty());
    }

    #[tokio::test]
    async fn test_snapshot_manager_creation() {
        let (manager, _temp_dir) = create_test_manager().await;

        let stats = manager.get_statistics(None).await.unwrap();
        assert_eq!(stats.total_snapshots, 0);
    }

    #[tokio::test]
    async fn test_full_snapshot_creation() {
        let (manager, _temp_dir) = create_test_manager().await;
        let replica_id = ReplicaId::new("test-replica").unwrap();

        let snapshot_id = manager.create_full_snapshot(&replica_id).await.unwrap();
        assert!(!snapshot_id.as_str().is_empty());

        let metadata = manager.get_snapshot_metadata(&snapshot_id).await.unwrap();
        assert_eq!(metadata.replica_id, replica_id);
        assert_eq!(metadata.snapshot_type, SnapshotType::Full);
        assert_eq!(metadata.status, SnapshotStatus::Completed);
    }

    #[tokio::test]
    async fn test_incremental_snapshot_creation() {
        let (manager, _temp_dir) = create_test_manager().await;
        let replica_id = ReplicaId::new("test-replica").unwrap();

        let parent_id = manager.create_full_snapshot(&replica_id).await.unwrap();
        let incremental_id = manager
            .create_incremental_snapshot(&replica_id, &parent_id)
            .await
            .unwrap();

        let metadata = manager
            .get_snapshot_metadata(&incremental_id)
            .await
            .unwrap();
        assert_eq!(metadata.snapshot_type, SnapshotType::Incremental);
        assert_eq!(metadata.parent_snapshot, Some(parent_id));
    }

    #[tokio::test]
    async fn test_snapshot_listing() {
        let (manager, _temp_dir) = create_test_manager().await;
        let replica_id = ReplicaId::new("test-replica").unwrap();

        let _full1 = manager.create_full_snapshot(&replica_id).await.unwrap();
        let _full2 = manager.create_full_snapshot(&replica_id).await.unwrap();

        let snapshots = manager.list_snapshots(&replica_id).await.unwrap();
        assert_eq!(snapshots.len(), 2);

        for snapshot in snapshots {
            assert_eq!(snapshot.replica_id, replica_id);
            assert_eq!(snapshot.snapshot_type, SnapshotType::Full);
        }
    }

    #[tokio::test]
    async fn test_snapshot_deletion() {
        let (manager, _temp_dir) = create_test_manager().await;
        let replica_id = ReplicaId::new("test-replica").unwrap();

        let snapshot_id = manager.create_full_snapshot(&replica_id).await.unwrap();
        assert!(manager.get_snapshot_metadata(&snapshot_id).await.is_ok());

        assert!(manager.delete_snapshot(&snapshot_id).await.is_ok());
        assert!(manager.get_snapshot_metadata(&snapshot_id).await.is_err());
    }

    #[tokio::test]
    async fn test_snapshot_verification() {
        let (manager, _temp_dir) = create_test_manager().await;
        let replica_id = ReplicaId::new("test-replica").unwrap();

        let snapshot_id = manager.create_full_snapshot(&replica_id).await.unwrap();
        let verified = manager.verify_snapshot(&snapshot_id).await.unwrap();
        assert!(verified);
    }

    #[tokio::test]
    async fn test_snapshot_restore() {
        let (manager, _temp_dir) = create_test_manager().await;
        let replica_id = ReplicaId::new("test-replica").unwrap();

        let snapshot_id = manager.create_full_snapshot(&replica_id).await.unwrap();
        let result = manager.restore_snapshot(&replica_id, &snapshot_id).await;
        assert!(result.is_ok());
    }
}
