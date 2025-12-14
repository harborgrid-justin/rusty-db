// Backup Catalog - RMAN-style backup metadata repository
// Centralized backup tracking and management across databases

use crate::error::DbError;
use crate::Result;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::collections::HashMap;
use std::fs::{create_dir_all, File};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;
use std::time::SystemTime;

// Catalog database configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CatalogConfig {
    pub catalog_path: PathBuf,
    pub max_retention_days: u64,
    pub auto_register_backups: bool,
    pub cross_database_tracking: bool,
    pub enable_reporting: bool,
    pub backup_history_limit: usize,
}

impl Default for CatalogConfig {
    fn default() -> Self {
        Self {
            catalog_path: PathBuf::from("/var/lib/rustydb/catalog"),
            max_retention_days: 365,
            auto_register_backups: true,
            cross_database_tracking: true,
            enable_reporting: true,
            backup_history_limit: 10000,
        }
    }
}

// Backup piece - individual backup file component
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackupPiece {
    pub piece_id: String,
    pub backup_set_id: String,
    pub piece_number: u32,
    pub file_path: PathBuf,
    pub size_bytes: u64,
    pub compressed_size_bytes: u64,
    pub checksum: String,
    pub creation_time: SystemTime,
    pub completion_time: Option<SystemTime>,
    pub status: PieceStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum PieceStatus {
    Available,
    Expired,
    Obsolete,
    Corrupted,
    Archived,
}

// Backup set - logical grouping of backup pieces
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackupSet {
    pub set_id: String,
    pub database_id: String,
    pub backup_type: BackupSetType,
    pub start_time: SystemTime,
    pub completion_time: Option<SystemTime>,
    pub scn_start: u64,
    pub scn_end: u64,
    pub pieces: Vec<String>,
    pub total_size_bytes: u64,
    pub compressed_size_bytes: u64,
    pub encryption_enabled: bool,
    pub compression_enabled: bool,
    pub tags: HashMap<String, String>,
    pub keep_until: Option<SystemTime>,
    pub obsolete: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum BackupSetType {
    Full,
    Incremental { level: u32 },
    Differential,
    ArchiveLog,
}

impl BackupSet {
    pub fn is_complete(&self) -> bool {
        self.completion_time.is_some()
    }

    pub fn is_obsolete(&self) -> bool {
        self.obsolete || self.is_expired()
    }

    pub fn is_expired(&self) -> bool {
        if let Some(keep_until) = self.keep_until {
            SystemTime::now() > keep_until
        } else {
            false
        }
    }

    pub fn duration(&self) -> Option<Duration> {
        self.completion_time
            .and_then(|ct| ct.duration_since(self.start_time).ok())
    }
}

// Database registration in catalog
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseRegistration {
    pub database_id: String,
    pub database_name: String,
    pub registration_time: SystemTime,
    pub last_backup_time: Option<SystemTime>,
    pub total_backups: u64,
    pub total_backup_size_bytes: u64,
    pub version: String,
    pub platform: String,
    pub tags: HashMap<String, String>,
}

// Backup report
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackupReport {
    pub report_id: String,
    pub report_type: ReportType,
    pub generated_at: SystemTime,
    pub database_filter: Option<String>,
    pub time_range_start: Option<SystemTime>,
    pub time_range_end: Option<SystemTime>,
    pub summary: ReportSummary,
    pub details: Vec<ReportDetail>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ReportType {
    BackupSummary,
    ObsoleteBackups,
    BackupHistory,
    StorageUsage,
    ComplianceReport,
    RecoverabilityReport,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReportSummary {
    pub total_databases: usize,
    pub total_backup_sets: usize,
    pub total_backup_pieces: usize,
    pub total_size_bytes: u64,
    pub total_compressed_size_bytes: u64,
    pub compression_ratio: f64,
    pub oldest_backup: Option<SystemTime>,
    pub newest_backup: Option<SystemTime>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReportDetail {
    pub database_id: String,
    pub database_name: String,
    pub backup_count: usize,
    pub total_size_bytes: u64,
    pub last_backup_time: Option<SystemTime>,
    pub recovery_window_compliant: bool,
}

// Restore point catalog entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RestorePointCatalog {
    pub restore_point_id: String,
    pub database_id: String,
    pub name: String,
    pub scn: u64,
    pub creation_time: SystemTime,
    pub guaranteed: bool,
    pub preserve_until: Option<SystemTime>,
}

// Archived redo log entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArchivedRedoLog {
    pub log_id: String,
    pub database_id: String,
    pub sequence_number: u64,
    pub thread_number: u32,
    pub file_path: PathBuf,
    pub size_bytes: u64,
    pub first_change_scn: u64,
    pub next_change_scn: u64,
    pub archived_time: SystemTime,
    pub status: ArchiveLogStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ArchiveLogStatus {
    Available,
    BackedUp,
    Deleted,
    Expired,
}

// Backup catalog - main catalog manager
pub struct BackupCatalog {
    #[allow(dead_code)]
    config: CatalogConfig,
    databases: Arc<RwLock<HashMap<String, DatabaseRegistration>>>,
    backup_sets: Arc<RwLock<BTreeMap<String, BackupSet>>>,
    backup_pieces: Arc<RwLock<HashMap<String, BackupPiece>>>,
    restore_points: Arc<RwLock<HashMap<String, RestorePointCatalog>>>,
    archived_logs: Arc<RwLock<BTreeMap<u64, ArchivedRedoLog>>>,
    reports: Arc<RwLock<HashMap<String, BackupReport>>>,
}

impl BackupCatalog {
    pub fn new(config: CatalogConfig) -> Result<Self> {
        create_dir_all(&config.catalog_path).map_err(|e| {
            DbError::BackupError(format!("Failed to create catalog directory: {}", e))
        })?;

        Ok(Self {
            config,
            databases: Arc::new(RwLock::new(HashMap::new())),
            backup_sets: Arc::new(RwLock::new(BTreeMap::new())),
            backup_pieces: Arc::new(RwLock::new(HashMap::new())),
            restore_points: Arc::new(RwLock::new(HashMap::new())),
            archived_logs: Arc::new(RwLock::new(BTreeMap::new())),
            reports: Arc::new(RwLock::new(HashMap::new())),
        })
    }

    // Register a database in the catalog
    pub fn register_database(
        &self,
        database_id: String,
        database_name: String,
        version: String,
        platform: String,
    ) -> Result<()> {
        let registration = DatabaseRegistration {
            database_id: database_id.clone(),
            database_name,
            registration_time: SystemTime::now(),
            last_backup_time: None,
            total_backups: 0,
            total_backup_size_bytes: 0,
            version,
            platform,
            tags: HashMap::new(),
        };

        self.databases.write().insert(database_id, registration);
        Ok(())
    }

    // Unregister a database
    pub fn unregister_database(&self, database_id: &str) -> Result<()> {
        self.databases
            .write()
            .remove(database_id)
            .ok_or_else(|| DbError::BackupError("Database not found".to_string()))?;
        Ok(())
    }

    // Register a backup set
    pub fn register_backup_set(&self, backup_set: BackupSet) -> Result<()> {
        let database_id = backup_set.database_id.clone();
        let set_id = backup_set.set_id.clone();
        let size = backup_set.total_size_bytes;

        // Update database stats
        if let Some(db) = self.databases.write().get_mut(&database_id) {
            db.total_backups += 1;
            db.total_backup_size_bytes += size;
            if backup_set.completion_time.is_some() {
                db.last_backup_time = backup_set.completion_time;
            }
        }

        self.backup_sets.write().insert(set_id, backup_set);
        Ok(())
    }

    // Register a backup piece
    pub fn register_backup_piece(&self, piece: BackupPiece) -> Result<()> {
        let piece_id = piece.piece_id.clone();
        self.backup_pieces.write().insert(piece_id, piece);
        Ok(())
    }

    // Mark backup set as obsolete
    pub fn mark_obsolete(&self, set_id: &str) -> Result<()> {
        let mut sets = self.backup_sets.write();
        let set = sets
            .get_mut(set_id)
            .ok_or_else(|| DbError::BackupError("Backup set not found".to_string()))?;

        set.obsolete = true;

        // Mark all pieces as obsolete
        let mut pieces = self.backup_pieces.write();
        for piece_id in &set.pieces {
            if let Some(piece) = pieces.get_mut(piece_id) {
                piece.status = PieceStatus::Obsolete;
            }
        }

        Ok(())
    }

    // Delete obsolete backups
    pub fn delete_obsolete(&self) -> Result<Vec<String>> {
        let mut deleted = Vec::new();
        let mut sets = self.backup_sets.write();

        let obsolete_sets: Vec<String> = sets
            .iter()
            .filter(|(_, set)| set.is_obsolete())
            .map(|(id, _)| id.clone())
            .collect();

        for set_id in obsolete_sets {
            if let Some(set) = sets.remove(&set_id) {
                // Delete all pieces
                let mut pieces = self.backup_pieces.write();
                for piece_id in &set.pieces {
                    pieces.remove(piece_id);
                }
                deleted.push(set_id);
            }
        }

        Ok(deleted)
    }

    // List backup sets for a database
    pub fn list_backup_sets(&self, database_id: &str) -> Vec<BackupSet> {
        self.backup_sets
            .read()
            .values()
            .filter(|set| set.database_id == database_id)
            .cloned()
            .collect()
    }

    // Find backup sets for point-in-time recovery
    pub fn find_recovery_path(&self, database_id: &str, target_scn: u64) -> Result<Vec<BackupSet>> {
        let sets = self.backup_sets.read();
        let mut recovery_sets = Vec::new();

        // Find the most recent full backup before target SCN
        let mut full_backup: Option<BackupSet> = None;
        for set in sets.values() {
            if set.database_id == database_id
                && matches!(set.backup_type, BackupSetType::Full)
                && set.scn_end <= target_scn
                && !set.is_obsolete()
            {
                if full_backup.is_none() || set.scn_end > full_backup.as_ref().unwrap().scn_end {
                    full_backup = Some(set.clone());
                }
            }
        }

        if let Some(full) = full_backup {
            recovery_sets.push(full.clone());

            // Find incremental backups after full backup
            for set in sets.values() {
                if set.database_id == database_id
                    && matches!(
                        set.backup_type,
                        BackupSetType::Incremental { .. } | BackupSetType::Differential
                    )
                    && set.scn_start >= full.scn_end
                    && set.scn_end <= target_scn
                    && !set.is_obsolete()
                {
                    recovery_sets.push(set.clone());
                }
            }

            // Sort by SCN
            recovery_sets.sort_by_key(|s| s.scn_start);
        } else {
            return Err(DbError::BackupError(
                "No suitable full backup found for recovery".to_string(),
            ));
        }

        Ok(recovery_sets)
    }

    // Register a restore point
    pub fn register_restore_point(&self, restore_point: RestorePointCatalog) -> Result<()> {
        let id = restore_point.restore_point_id.clone();
        self.restore_points.write().insert(id, restore_point);
        Ok(())
    }

    // Register archived redo log
    pub fn register_archived_log(&self, log: ArchivedRedoLog) -> Result<()> {
        let sequence = log.sequence_number;
        self.archived_logs.write().insert(sequence, log);
        Ok(())
    }

    // Find archived logs for recovery
    pub fn find_archived_logs(
        &self,
        database_id: &str,
        start_scn: u64,
        end_scn: u64,
    ) -> Vec<ArchivedRedoLog> {
        self.archived_logs
            .read()
            .values()
            .filter(|log| {
                log.database_id == database_id
                    && log.first_change_scn <= end_scn
                    && log.next_change_scn >= start_scn
                    && log.status != ArchiveLogStatus::Deleted
            })
            .cloned()
            .collect()
    }

    // Generate backup report
    pub fn generate_report(
        &self,
        report_type: ReportType,
        database_filter: Option<String>,
    ) -> Result<String> {
        let report_id = format!("REPORT-{}", uuid::Uuid::new_v4());

        let databases = self.databases.read();
        let sets = self.backup_sets.read();
        let pieces = self.backup_pieces.read();

        let filtered_sets: Vec<&BackupSet> = if let Some(ref db_id) = database_filter {
            sets.values().filter(|s| &s.database_id == db_id).collect()
        } else {
            sets.values().collect()
        };

        // Calculate summary
        let total_size: u64 = filtered_sets.iter().map(|s| s.total_size_bytes).sum();
        let total_compressed: u64 = filtered_sets.iter().map(|s| s.compressed_size_bytes).sum();

        let oldest = filtered_sets.iter().map(|s| s.start_time).min();

        let newest = filtered_sets.iter().filter_map(|s| s.completion_time).max();

        let summary = ReportSummary {
            total_databases: if database_filter.is_some() {
                1
            } else {
                databases.len()
            },
            total_backup_sets: filtered_sets.len(),
            total_backup_pieces: pieces.len(),
            total_size_bytes: total_size,
            total_compressed_size_bytes: total_compressed,
            compression_ratio: if total_compressed > 0 {
                total_size as f64 / total_compressed as f64
            } else {
                1.0
            },
            oldest_backup: oldest,
            newest_backup: newest,
        };

        // Generate details per database
        let mut details = Vec::new();
        for (db_id, db) in databases.iter() {
            if database_filter.is_some() && database_filter.as_ref() != Some(db_id) {
                continue;
            }

            let db_sets: Vec<&BackupSet> =
                sets.values().filter(|s| &s.database_id == db_id).collect();

            let db_size: u64 = db_sets.iter().map(|s| s.total_size_bytes).sum();

            let recovery_window_compliant = db
                .last_backup_time
                .map(|t| {
                    SystemTime::now()
                        .duration_since(t)
                        .map(|d| d.as_secs() < 86400) // Within 24 hours
                        .unwrap_or(false)
                })
                .unwrap_or(false);

            details.push(ReportDetail {
                database_id: db_id.clone(),
                database_name: db.database_name.clone(),
                backup_count: db_sets.len(),
                total_size_bytes: db_size,
                last_backup_time: db.last_backup_time,
                recovery_window_compliant,
            });
        }

        let report = BackupReport {
            report_id: report_id.clone(),
            report_type,
            generated_at: SystemTime::now(),
            database_filter,
            time_range_start: None,
            time_range_end: None,
            summary,
            details,
        };

        self.reports.write().insert(report_id.clone(), report);

        Ok(report_id)
    }

    // Get report by ID
    pub fn get_report(&self, report_id: &str) -> Option<BackupReport> {
        self.reports.read().get(report_id).cloned()
    }

    // Export catalog to file
    pub fn export_catalog(&self, export_path: &Path) -> Result<()> {
        let catalog_data = CatalogExport {
            databases: self.databases.read().clone(),
            backup_sets: self.backup_sets.read().clone(),
            backup_pieces: self.backup_pieces.read().clone(),
            restore_points: self.restore_points.read().clone(),
            archived_logs: self.archived_logs.read().clone(),
        };

        let json = serde_json::to_string_pretty(&catalog_data)
            .map_err(|e| DbError::BackupError(format!("Failed to serialize catalog: {}", e)))?;

        let mut file = File::create(export_path)
            .map_err(|e| DbError::BackupError(format!("Failed to create export file: {}", e)))?;

        file.write_all(json.as_bytes())
            .map_err(|e| DbError::BackupError(format!("Failed to write export: {}", e)))?;

        Ok(())
    }

    // Import catalog from file
    pub fn import_catalog(&self, import_path: &Path) -> Result<()> {
        let mut file = File::open(import_path)
            .map_err(|e| DbError::BackupError(format!("Failed to open import file: {}", e)))?;

        let mut json = String::new();
        file.read_to_string(&mut json)
            .map_err(|e| DbError::BackupError(format!("Failed to read import file: {}", e)))?;

        let catalog_data: CatalogExport = serde_json::from_str(&json)
            .map_err(|e| DbError::BackupError(format!("Failed to parse catalog: {}", e)))?;

        *self.databases.write() = catalog_data.databases;
        *self.backup_sets.write() = catalog_data.backup_sets;
        *self.backup_pieces.write() = catalog_data.backup_pieces;
        *self.restore_points.write() = catalog_data.restore_points;
        *self.archived_logs.write() = catalog_data.archived_logs;

        Ok(())
    }

    // Get catalog statistics
    pub fn get_statistics(&self) -> CatalogStatistics {
        let databases = self.databases.read();
        let sets = self.backup_sets.read();
        let pieces = self.backup_pieces.read();
        let logs = self.archived_logs.read();

        let total_backups = sets.len();
        let obsolete_backups = sets.values().filter(|s| s.is_obsolete()).count();
        let total_size: u64 = sets.values().map(|s| s.total_size_bytes).sum();
        let total_compressed: u64 = sets.values().map(|s| s.compressed_size_bytes).sum();

        CatalogStatistics {
            total_databases: databases.len(),
            total_backup_sets: total_backups,
            total_backup_pieces: pieces.len(),
            total_archived_logs: logs.len(),
            obsolete_backups,
            total_size_bytes: total_size,
            total_compressed_size_bytes: total_compressed,
            compression_ratio: if total_compressed > 0 {
                total_size as f64 / total_compressed as f64
            } else {
                1.0
            },
        }
    }
}

// Catalog export format
#[derive(Debug, Clone, Serialize, Deserialize)]
struct CatalogExport {
    databases: HashMap<String, DatabaseRegistration>,
    backup_sets: BTreeMap<String, BackupSet>,
    backup_pieces: HashMap<String, BackupPiece>,
    restore_points: HashMap<String, RestorePointCatalog>,
    archived_logs: BTreeMap<u64, ArchivedRedoLog>,
}

// Catalog statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CatalogStatistics {
    pub total_databases: usize,
    pub total_backup_sets: usize,
    pub total_backup_pieces: usize,
    pub total_archived_logs: usize,
    pub obsolete_backups: usize,
    pub total_size_bytes: u64,
    pub total_compressed_size_bytes: u64,
    pub compression_ratio: f64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_backup_catalog() {
        let config = CatalogConfig::default();
        let catalog = BackupCatalog::new(config).unwrap();

        catalog
            .register_database(
                "db1".to_string(),
                "TestDB".to_string(),
                "1.0".to_string(),
                "Linux".to_string(),
            )
            .unwrap();

        let databases = catalog.databases.read();
        assert_eq!(databases.len(), 1);
    }

    #[test]
    fn test_backup_set() {
        let set = BackupSet {
            set_id: "set1".to_string(),
            database_id: "db1".to_string(),
            backup_type: BackupSetType::Full,
            start_time: SystemTime::now(),
            completion_time: Some(SystemTime::now()),
            scn_start: 1000,
            scn_end: 2000,
            pieces: vec![],
            total_size_bytes: 1024 * 1024,
            compressed_size_bytes: 512 * 1024,
            encryption_enabled: false,
            compression_enabled: true,
            tags: HashMap::new(),
            keep_until: None,
            obsolete: false,
        };

        assert!(set.is_complete());
        assert!(!set.is_obsolete());
    }

    #[test]
    fn test_recovery_path() {
        let config = CatalogConfig::default();
        let catalog = BackupCatalog::new(config).unwrap();

        catalog
            .register_database(
                "db1".to_string(),
                "TestDB".to_string(),
                "1.0".to_string(),
                "Linux".to_string(),
            )
            .unwrap();

        let full_set = BackupSet {
            set_id: "full1".to_string(),
            database_id: "db1".to_string(),
            backup_type: BackupSetType::Full,
            start_time: SystemTime::now(),
            completion_time: Some(SystemTime::now()),
            scn_start: 1000,
            scn_end: 2000,
            pieces: vec![],
            total_size_bytes: 1024 * 1024,
            compressed_size_bytes: 512 * 1024,
            encryption_enabled: false,
            compression_enabled: true,
            tags: HashMap::new(),
            keep_until: None,
            obsolete: false,
        };

        catalog.register_backup_set(full_set).unwrap();

        let path = catalog.find_recovery_path("db1", 2500).unwrap();
        assert!(!path.is_empty());
    }
}
