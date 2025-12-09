// RustyDB Backup Module - Enterprise-grade backup and recovery
// Comprehensive backup, point-in-time recovery, and disaster recovery capabilities

// Module declarations
use crate::error::DbError;
pub mod manager;
pub mod pitr;
pub mod snapshots;
pub mod cloud;
pub mod backup_encryption;
pub mod disaster_recovery;
pub mod verification;
pub mod catalog;

// Re-export key types for convenience
pub use manager::{
    BackupManager, BackupMetadata, BackupType, BackupStatus, BackupConfig,
    RetentionPolicy, BlockChangeMap, BackupStatistics,
};

pub use pitr::{
    PitrManager, RecoveryTarget, RecoveryMode, RecoverySession, RecoveryStatus,
    LogMiner, FlashbackQuery, RestorePoint, TransactionLogEntry,
};

pub use snapshots::{
    SnapshotManager, Snapshot, SnapshotType, SnapshotStatus, SnapshotClone,
    SnapshotSchedule, SnapshotFrequency, CowTracker, SnapshotStatistics,
};

pub use cloud::{
    CloudBackupManager, CloudProvider, CloudStorageConfig, CloudBackup,
    StorageClass, UploadSession, UploadStatus, BandwidthThrottler,
};

pub use backup_encryption::{
    KeyManager, BackupEncryptionManager, EncryptionKey, EncryptionAlgorithm,
    KeyManagementConfig, EncryptedBackup, KeyDerivationFunction,
};

pub use disaster_recovery::{
    DisasterRecoveryManager, StandbyConfig, StandbyStatus, ReplicationMode,
    DatabaseRole, RtoConfig, RpoConfig, FailoverEvent, FailoverTrigger,
};

pub use verification::{
    VerificationManager, VerificationResult, VerificationType, VerificationStatus,
    RestoreTestConfig, RestoreTestResult, BlockChecksum, ChecksumAlgorithm,
};

pub use catalog::{
    BackupCatalog, CatalogConfig, BackupSet, BackupPiece, BackupSetType,
    DatabaseRegistration, BackupReport, ReportType, CatalogStatistics,
};

use serde::{Deserialize, Serialize};
use std::sync::Arc;
use crate::Result;

/// Unified backup system integrating all modules
pub struct BackupSystem {
    backup_manager: Arc<BackupManager>,
    pitr_manager: Arc<PitrManager>,
    snapshot_manager: Arc<SnapshotManager>,
    cloud_manager: Option<Arc<CloudBackupManager>>,
    encryption_manager: Arc<BackupEncryptionManager>,
    dr_manager: Arc<DisasterRecoveryManager>,
    verification_manager: Arc<VerificationManager>,
    catalog: Arc<BackupCatalog>,
}

impl BackupSystem {
    pub fn new(
        backup_config: BackupConfig,
        retention_policy: RetentionPolicy,
        catalog_config: CatalogConfig,
    ) -> Result<Self> {
        // Initialize core managers
        let backup_manager = Arc::new(BackupManager::new(backup_config.clone(), retention_policy)?);

        let pitr_manager = Arc::new(PitrManager::new(
            backup_config.backup_dir.join("logs")
        ));

        let snapshot_manager = Arc::new(SnapshotManager::new(
            backup_config.backup_dir.join("snapshots")
        )?);

        // Initialize encryption
        let key_mgmt_config = KeyManagementConfig::default();
        let key_manager = Arc::new(KeyManager::new(key_mgmt_config)?);
        let encryption_manager = Arc::new(BackupEncryptionManager::new(key_manager));

        // Initialize disaster recovery
        let standby_config = StandbyConfig::default();
        let rto_config = RtoConfig::default();
        let rpo_config = RpoConfig::default();
        let dr_manager = Arc::new(DisasterRecoveryManager::new(
            standby_config,
            rto_config,
            rpo_config,
        ));

        // Initialize verification
        let restore_test_config = RestoreTestConfig::default();
        let verification_manager = Arc::new(VerificationManager::new(restore_test_config));

        // Initialize catalog
        let catalog = Arc::new(BackupCatalog::new(catalog_config)?);

        Ok(Self {
            backup_manager,
            pitr_manager,
            snapshot_manager,
            cloud_manager: None,
            encryption_manager,
            dr_manager,
            verification_manager,
            catalog,
        })
    }

    /// Enable cloud backup integration
    pub fn enable_cloud_backup(&mut self, cloud_config: CloudStorageConfig) {
        self.cloud_manager = Some(Arc::new(CloudBackupManager::new(cloud_config)));
    }

    /// Access backup manager
    pub fn backup_manager(&self) -> Arc<BackupManager> {
        Arc::clone(&self.backup_manager)
    }

    /// Access PITR manager
    pub fn pitr_manager(&self) -> Arc<PitrManager> {
        Arc::clone(&self.pitr_manager)
    }

    /// Access snapshot manager
    pub fn snapshot_manager(&self) -> Arc<SnapshotManager> {
        Arc::clone(&self.snapshot_manager)
    }

    /// Access cloud backup manager
    pub fn cloud_manager(&self) -> Option<Arc<CloudBackupManager>> {
        self.cloud_manager.as_ref().map(Arc::clone)
    }

    /// Access encryption manager
    pub fn encryption_manager(&self) -> Arc<BackupEncryptionManager> {
        Arc::clone(&self.encryption_manager)
    }

    /// Access disaster recovery manager
    pub fn dr_manager(&self) -> Arc<DisasterRecoveryManager> {
        Arc::clone(&self.dr_manager)
    }

    /// Access verification manager
    pub fn verification_manager(&self) -> Arc<VerificationManager> {
        Arc::clone(&self.verification_manager)
    }

    /// Access backup catalog
    pub fn catalog(&self) -> Arc<BackupCatalog> {
        Arc::clone(&self.catalog)
    }

    /// Perform a complete backup workflow
    pub fn perform_full_backup(&self, database_name: &str) -> Result<String> {
        // Create full backup
        let backup_id = self.backup_manager.create_full_backup(database_name)?;

        // Register in catalog
        let backup = self.backup_manager.get_backup(&backup_id).unwrap();
        let backup_set = BackupSet {
            set_id: backup_id.clone(),
            database_id: database_name.to_string(),
            backup_type: BackupSetType::Full,
            start_time: backup.start_time,
            completion_time: backup.end_time,
            scn_start: backup.scn,
            scn_end: backup.scn,
            pieces: vec![],
            total_size_bytes: backup.size_bytes,
            compressed_size_bytes: backup.compressed_size_bytes,
            encryption_enabled: backup.encryption_enabled,
            compression_enabled: backup.compression_enabled,
            tags: std::collections::HashMap::new(),
            keep_until: backup.retention_until,
            obsolete: false,
        };
        self.catalog.register_backup_set(backup_set)?;

        // Verify backup
        self.verification_manager.verify_backup(
            backup_id.clone(),
            backup.backup_path.clone(),
            VerificationType::Standard,
        )?;

        Ok(backup_id)
    }

    /// Perform point-in-time recovery
    pub fn perform_pitr(
        &self,
        database_name: &str,
        target: RecoveryTarget,
        recovery_path: std::path::PathBuf,
    ) -> Result<String> {
        // Find recovery path using catalog
        let target_scn = match &target {
            RecoveryTarget::Scn(scn) => *scn,
            RecoveryTarget::Latest => u64::MAX,
            _ => 1000, // Default for simulation
        };

        let recovery_sets = self.catalog.find_recovery_path(database_name, target_scn)?;

        if recovery_sets.is_empty() {
            return Err(crate::error::DbError::BackupError(
                "No suitable backup found for recovery".to_string()
            ));
        }

        // Start recovery session
        let backup_id = recovery_sets[0].set_id.clone();
        let session_id = self.pitr_manager.start_recovery(
            backup_id,
            target,
            RecoveryMode::Complete,
            recovery_path,
        )?;

        // Perform recovery
        self.pitr_manager.perform_recovery(&session_id)?;

        Ok(session_id)
    }

    /// Create a snapshot for testing
    pub fn create_test_snapshot(&self, database_name: &str) -> Result<String> {
        let snapshot_id = self.snapshot_manager.create_snapshot(
            format!("{}-snapshot", database_name),
            database_name.to_string(),
            SnapshotType::Manual,
        )?;

        Ok(snapshot_id)
    }

    /// Trigger disaster recovery failover
    pub fn trigger_failover(&self, target_standby: &str) -> Result<String> {
        let event_id = self.dr_manager.trigger_failover(
            FailoverTrigger::Manual,
            target_standby,
        )?;

        Ok(event_id)
    }

    /// Get comprehensive backup system statistics
    pub fn get_system_statistics(&self) -> BackupSystemStatistics {
        let backup_stats = self.backup_manager.get_statistics();
        let catalog_stats = self.catalog.get_statistics();
        let snapshot_stats = self.snapshot_manager.get_storage_statistics();
        let verification_stats = self.verification_manager.get_statistics();

        BackupSystemStatistics {
            backup_stats,
            catalog_stats,
            snapshot_stats,
            verification_stats,
        }
    }
}

/// Comprehensive backup system statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackupSystemStatistics {
    pub backup_stats: BackupStatistics,
    pub catalog_stats: CatalogStatistics,
    pub snapshot_stats: SnapshotStatistics,
    pub verification_stats: verification::VerificationStatistics,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_backup_system_initialization() {
        let backup_config = BackupConfig::default();
        let retention_policy = RetentionPolicy::default();
        let catalog_config = CatalogConfig::default();

        let system = BackupSystem::new(
            backup_config,
            retention_policy,
            catalog_config,
        ).unwrap();

        // Verify all managers are accessible
        assert!(system.backup_manager().list_backups().is_empty());
        assert_eq!(system.catalog().get_statistics().total_backup_sets, 0);
    }

    #[test]
    fn test_full_backup_workflow() {
        let backup_config = BackupConfig::default();
        let retention_policy = RetentionPolicy::default();
        let catalog_config = CatalogConfig::default();

        let system = BackupSystem::new(
            backup_config,
            retention_policy,
            catalog_config,
        ).unwrap();

        // Register database
        system.catalog().register_database(
            "testdb".to_string(),
            "TestDB".to_string(),
            "1.0".to_string(),
            "Linux".to_string(),
        ).unwrap();

        // Perform backup
        let backup_id = system.perform_full_backup("testdb").unwrap();
        assert!(!backup_id.is_empty());

        // Verify backup was registered
        let stats = system.get_system_statistics();
        assert!(stats.backup_stats.total_backups > 0);
    }

    #[test]
    fn test_snapshot_creation() {
        let backup_config = BackupConfig::default();
        let retention_policy = RetentionPolicy::default();
        let catalog_config = CatalogConfig::default();

        let system = BackupSystem::new(
            backup_config,
            retention_policy,
            catalog_config,
        ).unwrap();

        let snapshot_id = system.create_test_snapshot("testdb").unwrap();
        assert!(!snapshot_id.is_empty());

        let stats = system.get_system_statistics();
        assert!(stats.snapshot_stats.total_snapshots > 0);
    }
}


