// # Shared Services and Common Objects
//
// Shared undo tablespace, temp tablespace, common users and roles,
// application common objects, and lockdown profiles.
//
// ## Features
//
// - **Shared Undo**: Centralized undo management for all PDBs
// - **Shared Temp**: Shared temporary tablespace
// - **Common Users**: Users that exist across all PDBs
// - **Common Roles**: Roles shared across PDBs
// - **Application Objects**: Shared application metadata
// - **Lockdown Profiles**: Security restrictions for PDBs
//
// ## Architecture
//
// Common objects are stored in the CDB root and referenced by PDBs:
// - Common users: C##USER format
// - Local users: Regular naming
// - Lockdown profiles: Restrict PDB capabilities

use super::pdb::PdbId;
use crate::error::{DbError, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::collections::HashSet;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::sync::RwLock;

/// Shared services coordinator
pub struct SharedServices {
    /// Shared undo tablespace
    undo: Arc<UndoTablespace>,

    /// Shared temp tablespace
    temp: Arc<TempTablespace>,

    /// Common users
    common_users: Arc<RwLock<HashMap<String, CommonUser>>>,

    /// Common roles
    common_roles: Arc<RwLock<HashMap<String, CommonRole>>>,

    /// Application common objects
    app_objects: Arc<RwLock<HashMap<String, ApplicationCommonObject>>>,

    /// Lockdown profiles
    lockdown_profiles: Arc<RwLock<HashMap<String, LockdownProfile>>>,
}

impl SharedServices {
    /// Create new shared services
    pub fn new() -> Self {
        Self {
            undo: Arc::new(UndoTablespace::new(2 * 1024 * 1024 * 1024)), // 2 GB
            temp: Arc::new(TempTablespace::new(1 * 1024 * 1024 * 1024)), // 1 GB
            common_users: Arc::new(RwLock::new(HashMap::new())),
            common_roles: Arc::new(RwLock::new(HashMap::new())),
            app_objects: Arc::new(RwLock::new(HashMap::new())),
            lockdown_profiles: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Get undo tablespace
    pub fn undo(&self) -> Arc<UndoTablespace> {
        self.undo.clone()
    }

    /// Get temp tablespace
    pub fn temp(&self) -> Arc<TempTablespace> {
        self.temp.clone()
    }

    /// Create a common user
    pub async fn create_common_user(&self, user: CommonUser) -> Result<()> {
        // Common users must start with C##
        if !user.username.starts_with("C##") {
            return Err(DbError::InvalidInput(
                "Common user names must start with C##".to_string(),
            ));
        }

        self.common_users
            .write()
            .await
            .insert(user.username.clone(), user);
        Ok(())
    }

    /// Get a common user
    pub async fn get_common_user(&self, username: &str) -> Option<CommonUser> {
        self.common_users.read().await.get(username).cloned()
    }

    /// List all common users
    pub async fn list_common_users(&self) -> Vec<CommonUser> {
        self.common_users.read().await.values().cloned().collect()
    }

    /// Create a common role
    pub async fn create_common_role(&self, role: CommonRole) -> Result<()> {
        // Common roles must start with C##
        if !role.name.starts_with("C##") {
            return Err(DbError::InvalidInput(
                "Common role names must start with C##".to_string(),
            ));
        }

        self.common_roles
            .write()
            .await
            .insert(role.name.clone(), role);
        Ok(())
    }

    /// Get a common role
    pub async fn get_common_role(&self, name: &str) -> Option<CommonRole> {
        self.common_roles.read().await.get(name).cloned()
    }

    /// Create an application common object
    pub async fn create_app_object(&self, object: ApplicationCommonObject) -> Result<()> {
        self.app_objects
            .write()
            .await
            .insert(object.name.clone(), object);
        Ok(())
    }

    /// Create a lockdown profile
    pub async fn create_lockdown_profile(&self, profile: LockdownProfile) -> Result<()> {
        self.lockdown_profiles
            .write()
            .await
            .insert(profile.name.clone(), profile);
        Ok(())
    }

    /// Get a lockdown profile
    pub async fn get_lockdown_profile(&self, name: &str) -> Option<LockdownProfile> {
        self.lockdown_profiles.read().await.get(name).cloned()
    }

    /// Apply lockdown profile to PDB
    pub async fn apply_lockdown(&self, pdb_id: PdbId, profile_name: &str) -> Result<()> {
        let _profile = self
            .get_lockdown_profile(profile_name)
            .await
            .ok_or_else(|| {
                DbError::NotFound(format!("Lockdown profile {} not found", profile_name))
            })?;

        // Apply restrictions (in real implementation)
        println!(
            "Applying lockdown profile {} to PDB {:?}",
            profile_name, pdb_id
        );

        Ok(())
    }
}

/// Shared undo tablespace
pub struct UndoTablespace {
    /// Total size
    size_bytes: u64,

    /// Current usage
    usage: Arc<RwLock<UndoUsage>>,

    /// Undo segments
    segments: Arc<RwLock<HashMap<u64, UndoSegment>>>,
}

#[derive(Debug, Clone)]
struct UndoUsage {
    used_bytes: u64,
    free_bytes: u64,
    #[allow(dead_code)]
    active_transactions: u32,
    #[allow(dead_code)]
    retention_secs: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UndoSegment {
    pub segment_id: u64,
    pub pdb_id: PdbId,
    pub size_bytes: u64,
    pub used_bytes: u64,
    pub transaction_count: u32,
}

impl UndoTablespace {
    /// Create a new undo tablespace
    pub fn new(size_bytes: u64) -> Self {
        Self {
            size_bytes,
            usage: Arc::new(RwLock::new(UndoUsage {
                used_bytes: 0,
                free_bytes: size_bytes,
                active_transactions: 0,
                retention_secs: 900, // 15 minutes
            })),
            segments: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Allocate undo segment for a PDB
    pub async fn allocate_segment(&self, pdb_id: PdbId, size_bytes: u64) -> Result<u64> {
        let mut usage = self.usage.write().await;

        if usage.used_bytes + size_bytes > self.size_bytes {
            return Err(DbError::ResourceExhausted(
                "Insufficient undo space".to_string(),
            ));
        }

        usage.used_bytes += size_bytes;
        usage.free_bytes -= size_bytes;
        drop(usage);

        let segment_id = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        let segment = UndoSegment {
            segment_id,
            pdb_id,
            size_bytes,
            used_bytes: 0,
            transaction_count: 0,
        };

        self.segments.write().await.insert(segment_id, segment);

        Ok(segment_id)
    }

    /// Free undo segment
    pub async fn free_segment(&self, segment_id: u64) -> Result<()> {
        let mut segments = self.segments.write().await;

        if let Some(segment) = segments.remove(&segment_id) {
            let mut usage = self.usage.write().await;
            usage.used_bytes -= segment.size_bytes;
            usage.free_bytes += segment.size_bytes;
            Ok(())
        } else {
            Err(DbError::NotFound(format!(
                "Undo segment {} not found",
                segment_id
            )))
        }
    }

    /// Get usage statistics
    pub async fn get_usage(&self) -> (u64, u64) {
        let usage = self.usage.read().await;
        (usage.used_bytes, usage.free_bytes)
    }

    /// List segments for a PDB
    pub async fn list_segments(&self, pdb_id: PdbId) -> Vec<UndoSegment> {
        self.segments
            .read()
            .await
            .values()
            .filter(|s| s.pdb_id == pdb_id)
            .cloned()
            .collect()
    }
}

/// Shared temporary tablespace
pub struct TempTablespace {
    /// Total size
    size_bytes: u64,

    /// Current usage
    usage: Arc<RwLock<TempUsage>>,

    /// Temp files
    temp_files: Arc<RwLock<HashMap<u64, TempFile>>>,
}

#[derive(Debug, Clone)]
struct TempUsage {
    used_bytes: u64,
    free_bytes: u64,
    active_sorts: u32,
    active_hash_joins: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TempFile {
    pub file_id: u64,
    pub pdb_id: PdbId,
    pub size_bytes: u64,
    pub used_bytes: u64,
    pub purpose: TempFilePurpose,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TempFilePurpose {
    Sort,
    HashJoin,
    GroupBy,
    Bitmap,
    General,
}

impl TempTablespace {
    /// Create a new temp tablespace
    pub fn new(size_bytes: u64) -> Self {
        Self {
            size_bytes,
            usage: Arc::new(RwLock::new(TempUsage {
                used_bytes: 0,
                free_bytes: size_bytes,
                active_sorts: 0,
                active_hash_joins: 0,
            })),
            temp_files: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Allocate temp space
    pub async fn allocate(
        &self,
        pdb_id: PdbId,
        size_bytes: u64,
        purpose: TempFilePurpose,
    ) -> Result<u64> {
        let mut usage = self.usage.write().await;

        if usage.used_bytes + size_bytes > self.size_bytes {
            return Err(DbError::ResourceExhausted(
                "Insufficient temp space".to_string(),
            ));
        }

        usage.used_bytes += size_bytes;
        usage.free_bytes -= size_bytes;

        match purpose {
            TempFilePurpose::Sort => usage.active_sorts += 1,
            TempFilePurpose::HashJoin => usage.active_hash_joins += 1,
            _ => {}
        }

        drop(usage);

        let file_id = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        let temp_file = TempFile {
            file_id,
            pdb_id,
            size_bytes,
            used_bytes: 0,
            purpose,
        };

        self.temp_files.write().await.insert(file_id, temp_file);

        Ok(file_id)
    }

    /// Free temp space
    pub async fn free(&self, file_id: u64) -> Result<()> {
        let mut temp_files = self.temp_files.write().await;

        if let Some(file) = temp_files.remove(&file_id) {
            let mut usage = self.usage.write().await;
            usage.used_bytes -= file.size_bytes;
            usage.free_bytes += file.size_bytes;

            match file.purpose {
                TempFilePurpose::Sort => usage.active_sorts -= 1,
                TempFilePurpose::HashJoin => usage.active_hash_joins -= 1,
                _ => {}
            }

            Ok(())
        } else {
            Err(DbError::NotFound(format!(
                "Temp file {} not found",
                file_id
            )))
        }
    }

    /// Get usage statistics
    pub async fn get_usage(&self) -> (u64, u64) {
        let usage = self.usage.read().await;
        (usage.used_bytes, usage.free_bytes)
    }
}

/// Common user (exists in all PDBs)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommonUser {
    /// Username (must start with C##)
    pub username: String,

    /// Password hash
    pub password_hash: String,

    /// Default tablespace
    pub default_tablespace: String,

    /// Temporary tablespace
    pub temp_tablespace: String,

    /// Common roles granted
    pub roles: Vec<String>,

    /// Account status
    pub status: AccountStatus,

    /// Profile
    pub profile: String,

    /// Created timestamp
    pub created_at: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AccountStatus {
    Open,
    Locked,
    Expired,
    ExpiredAndLocked,
}

/// Common role (exists in all PDBs)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommonRole {
    /// Role name (must start with C##)
    pub name: String,

    /// Description
    pub description: String,

    /// Privileges
    pub privileges: Vec<String>,

    /// System privileges
    pub system_privileges: Vec<String>,

    /// Object privileges
    pub object_privileges: Vec<ObjectPrivilege>,

    /// Created timestamp
    pub created_at: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ObjectPrivilege {
    pub object_name: String,
    pub privilege: String,
    pub grantable: bool,
}

/// Application common object
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApplicationCommonObject {
    /// Object name
    pub name: String,

    /// Object type
    pub object_type: ObjectType,

    /// Owner (common user)
    pub owner: String,

    /// Definition (DDL)
    pub definition: String,

    /// Sharing mode
    pub sharing: SharingMode,

    /// Created timestamp
    pub created_at: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ObjectType {
    Table,
    View,
    Index,
    Procedure,
    Function,
    Package,
    Trigger,
    Sequence,
    Synonym,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SharingMode {
    /// Object is shared across all PDBs
    Metadata,
    /// Object and data are shared
    Data,
    /// Extended data sharing
    Extended,
    /// Not shared
    None,
}

/// Lockdown profile
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LockdownProfile {
    /// Profile name
    pub name: String,

    /// Description
    pub description: String,

    /// Disabled statements
    pub disabled_statements: HashSet<String>,

    /// Disabled options
    pub disabled_options: HashSet<String>,

    /// Disabled features
    pub disabled_features: HashSet<String>,

    /// Allowed network access
    pub allowed_network_access: Vec<NetworkAccess>,

    /// Allowed OS access
    pub allow_os_access: bool,

    /// Created timestamp
    pub created_at: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkAccess {
    pub host: String,
    pub port: u16,
    pub protocol: String,
}

impl LockdownProfile {
    /// Create a new lockdown profile
    pub fn new(name: String) -> Self {
        Self {
            name,
            description: String::new(),
            disabled_statements: HashSet::new(),
            disabled_options: HashSet::new(),
            disabled_features: HashSet::new(),
            allowed_network_access: Vec::new(),
            allow_os_access: false,
            created_at: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs(),
        }
    }

    /// Disable a statement
    pub fn disable_statement(&mut self, statement: String) {
        self.disabled_statements.insert(statement);
    }

    /// Disable an option
    pub fn disable_option(&mut self, option: String) {
        self.disabled_options.insert(option);
    }

    /// Disable a feature
    pub fn disable_feature(&mut self, feature: String) {
        self.disabled_features.insert(feature);
    }

    /// Check if statement is allowed
    pub fn is_statement_allowed(&self, statement: &str) -> bool {
        !self.disabled_statements.contains(statement)
    }

    /// Check if option is allowed
    pub fn is_option_allowed(&self, option: &str) -> bool {
        !self.disabled_options.contains(option)
    }

    /// Check if feature is allowed
    pub fn is_feature_allowed(&self, feature: &str) -> bool {
        !self.disabled_features.contains(feature)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_shared_services() {
        let services = SharedServices::new();

        let user = CommonUser {
            username: "C##ADMIN".to_string(),
            password_hash: "hash".to_string(),
            default_tablespace: "USERS".to_string(),
            temp_tablespace: "TEMP".to_string(),
            roles: vec!["C##DBA".to_string()],
            status: AccountStatus::Open,
            profile: "DEFAULT".to_string(),
            created_at: 0,
        };

        services.create_common_user(user).await.unwrap();

        let retrieved = services.get_common_user("C##ADMIN").await;
        assert!(retrieved.is_some());
    }

    #[tokio::test]
    async fn test_undo_tablespace() {
        let undo = UndoTablespace::new(1024 * 1024 * 1024);
        let pdb_id = PdbId::new(1);

        let segment_id = undo
            .allocate_segment(pdb_id, 10 * 1024 * 1024)
            .await
            .unwrap();
        assert!(segment_id > 0);

        let (used, free) = undo.get_usage().await;
        assert_eq!(used, 10 * 1024 * 1024);
    }

    #[tokio::test]
    async fn test_temp_tablespace() {
        let temp = TempTablespace::new(1024 * 1024 * 1024);
        let pdb_id = PdbId::new(1);

        let file_id = temp
            .allocate(pdb_id, 5 * 1024 * 1024, TempFilePurpose::Sort)
            .await
            .unwrap();
        assert!(file_id > 0);

        let (used, free) = temp.get_usage().await;
        assert_eq!(used, 5 * 1024 * 1024);
    }

    #[test]
    fn test_lockdown_profile() {
        let mut profile = LockdownProfile::new("STRICT".to_string());
        profile.disable_statement("ALTER SYSTEM".to_string());
        profile.disable_feature("UTILITL_FILE_DIR".to_string());

        assert!(!profile.is_statement_allowed("ALTER SYSTEM"));
        assert!(profile.is_statement_allowed("SELECT"));
    }
}
