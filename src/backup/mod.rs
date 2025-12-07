use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::fs::{File, OpenOptions};
use std::io::{Write, Read};
use std::time::SystemTime;
use crate::Result;
use crate::error::DbError;

/// Backup configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackupConfig {
    pub backup_dir: PathBuf,
    pub incremental: bool,
    pub compression: bool,
}

/// Backup metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackupMetadata {
    pub backup_id: String,
    pub timestamp: SystemTime,
    pub backup_type: BackupType,
    pub size_bytes: u64,
    pub checksum: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BackupType {
    Full,
    Incremental,
    Differential,
}

/// Point-in-time recovery manager
pub struct BackupManager {
    config: BackupConfig,
    backups: Vec<BackupMetadata>,
}

impl BackupManager {
    pub fn new(config: BackupConfig) -> Result<Self> {
        std::fs::create_dir_all(&config.backup_dir)?;
        
        Ok(Self {
            config,
            backups: Vec::new(),
        })
    }
    
    pub fn create_backup(&mut self, data_dir: &Path, backup_type: BackupType) -> Result<BackupMetadata> {
        let timestamp = SystemTime::now();
        let backup_id = format!("backup_{}", timestamp.duration_since(SystemTime::UNIX_EPOCH)
            .unwrap().as_secs());
        
        let backup_path = self.config.backup_dir.join(&backup_id);
        std::fs::create_dir_all(&backup_path)?;
        
        // Copy data files
        let size_bytes = self.copy_data_files(data_dir, &backup_path)?;
        
        let metadata = BackupMetadata {
            backup_id: backup_id.clone(),
            timestamp,
            backup_type,
            size_bytes,
            checksum: "sha256_placeholder".to_string(),
        };
        
        // Save metadata
        let metadata_path = backup_path.join("metadata.json");
        let metadata_json = serde_json::to_string_pretty(&metadata)
            .map_err(|e| DbError::Serialization(e.to_string()))?;
        std::fs::write(metadata_path, metadata_json)?;
        
        self.backups.push(metadata.clone());
        
        Ok(metadata)
    }
    
    pub fn restore_backup(&self, backup_id: &str, target_dir: &Path) -> Result<()> {
        let backup_path = self.config.backup_dir.join(backup_id);
        
        if !backup_path.exists() {
            return Err(DbError::Storage(format!("Backup {} not found", backup_id)));
        }
        
        std::fs::create_dir_all(target_dir)?;
        
        // Copy backup files to target directory
        self.copy_data_files(&backup_path, target_dir)?;
        
        Ok(())
    }
    
    pub fn list_backups(&self) -> Vec<BackupMetadata> {
        self.backups.clone()
    }
    
    fn copy_data_files(&self, src: &Path, dst: &Path) -> Result<u64> {
        let mut total_size = 0u64;
        
        if src.is_dir() {
            for entry in std::fs::read_dir(src)? {
                let entry = entry?;
                let path = entry.path();
                
                if path.is_file() {
                    let file_name = path.file_name().unwrap();
                    let dst_path = dst.join(file_name);
                    
                    std::fs::copy(&path, &dst_path)?;
                    total_size += entry.metadata()?.len();
                }
            }
        }
        
        Ok(total_size)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;
    
    #[test]
    fn test_backup_creation() -> Result<()> {
        let backup_dir = tempdir().unwrap();
        let data_dir = tempdir().unwrap();
        
        let config = BackupConfig {
            backup_dir: backup_dir.path().to_path_buf(),
            incremental: false,
            compression: false,
        };
        
        let mut manager = BackupManager::new(config)?;
        let metadata = manager.create_backup(data_dir.path(), BackupType::Full)?;
        
        assert!(!metadata.backup_id.is_empty());
        Ok(())
    }
}
