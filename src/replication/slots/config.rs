#![allow(dead_code)]
// Replication slot configuration

use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::time::Duration;

// Replication slot manager configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SlotManagerConfig {
    // Maximum number of slots
    pub max_slots: usize,
    // Storage path for slot state
    pub storage_path: PathBuf,
    // Enable automatic cleanup
    pub enable_auto_cleanup: bool,
    // Cleanup interval
    pub cleanup_interval: Duration,
    // Maximum inactive duration before cleanup
    pub max_inactive_duration: Duration,
    // Enable slot monitoring
    pub enable_monitoring: bool,
    // Monitoring interval
    pub monitoring_interval: Duration,
    // Enable slot statistics
    pub enable_statistics: bool,
    // Statistics collection interval
    pub statistics_interval: Duration,
    // Maximum WAL retention size per slot
    pub max_wal_retention_size: u64,
    // Enable slot backup
    pub enable_backup: bool,
    // Backup interval
    pub backup_interval: Duration,
}

impl Default for SlotManagerConfig {
    fn default() -> Self {
        Self {
            max_slots: 100,
            storage_path: PathBuf::from("/data/replication/slots"),
            enable_auto_cleanup: true,
            cleanup_interval: Duration::from_secs(300), // 5 minutes
            max_inactive_duration: Duration::from_secs(86400), // 24 hours
            enable_monitoring: true,
            monitoring_interval: Duration::from_secs(60), // 1 minute
            enable_statistics: true,
            statistics_interval: Duration::from_secs(30), // 30 seconds
            max_wal_retention_size: 1024 * 1024 * 1024 * 10, // 10GB
            enable_backup: true,
            backup_interval: Duration::from_secs(3600), // 1 hour
        }
    }
}
