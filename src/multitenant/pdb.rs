// # Pluggable Database (PDB) Management
//
// Implements PDB lifecycle operations including creation, cloning, plugging, opening,
// closing, and deletion. Supports application containers, seed PDBs, and snapshots.
//
// ## Features
//
// - **PDB Creation**: Create new PDBs from scratch, clone, or seed
// - **Lifecycle Management**: Open, close, mount, restrict modes
// - **Snapshots**: Point-in-time PDB snapshots
// - **Application Containers**: Multi-level container hierarchy
// - **Seed PDB**: Template PDB for fast provisioning
// - **Hot Clone**: Clone running PDBs without downtime
//
// ## PDB States
//
// - **Mounted**: PDB is mounted but not open
// - **Open**: PDB is open for normal operations
// - **OpenRestricted**: PDB is open in restricted mode
// - **Closed**: PDB is completely closed
// - **Migrating**: PDB is being migrated

use std::fmt;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{SystemTime};
use tokio::sync::RwLock;
use serde::{Serialize, Deserialize};
use crate::error::Result;
use super::isolation::ResourceLimits;

/// Unique identifier for a Pluggable Database
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct PdbId(pub u64);

impl PdbId {
    /// Create a new PDB ID
    pub fn new(id: u64) -> Self {
        Self(id)
    }

    /// Get the underlying ID value
    pub fn value(&self) -> u64 {
        self.0
    }
}

impl std::fmt::Display for PdbId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "PdbId({})", self.0)
    }
}

/// PDB lifecycle state
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PdbLifecycleState {
    /// PDB is being created
    Creating,
    /// PDB is mounted but not open
    Mounted,
    /// PDB is open for normal operations
    Open,
    /// PDB is open in restricted mode (admin only)
    OpenRestricted,
    /// PDB is being closed
    Closing,
    /// PDB is closed
    Closed,
    /// PDB is being dropped
    Dropping,
    /// PDB is being migrated
    Migrating,
    /// PDB is in read-only mode
    ReadOnly,
}

/// PDB open mode
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PdbMode {
    /// Full read-write mode
    ReadWrite,
    /// Read-only mode
    ReadOnly,
    /// Restricted mode (admin only)
    Restricted,
}

/// PDB creation mode
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PdbCreateMode {
    /// Create a new empty PDB
    New,
    /// Create from seed PDB
    FromSeed,
    /// Clone an existing PDB
    Clone,
    /// Plug in an unplugged PDB
    Plug,
    /// Create from backup
    FromBackup,
}

/// PDB configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PdbConfig {
    /// PDB name (must be unique within CDB)
    pub name: String,

    /// Creation mode
    pub create_mode: PdbCreateMode,

    /// Admin username
    pub admin_user: String,

    /// Admin password (hashed)
    pub admin_password: String,

    /// Data file directory
    pub data_dir: PathBuf,

    /// Default tablespace name
    pub default_tablespace: String,

    /// Temporary tablespace name
    pub temp_tablespace: String,

    /// Resource limits
    pub resource_limits: ResourceLimits,

    /// Enable flashback
    pub flashback_enabled: bool,

    /// Enable force logging
    pub force_logging: bool,

    /// Character set
    pub charset: String,

    /// National character set
    pub ncharset: String,

    /// Application container (if this PDB is an app container)
    pub is_application_container: bool,

    /// Parent application container ID (if any)
    pub parent_app_container: Option<PdbId>,

    /// Snapshot interval (seconds, 0 = disabled)
    pub snapshot_interval_secs: u64,
}

impl Default for PdbConfig {
    fn default() -> Self {
        Self {
            name: String::new(),
            create_mode: PdbCreateMode::New,
            admin_user: "pdbadmin".to_string(),
            admin_password: "changeme".to_string(),
            data_dir: PathBuf::from("/var/lib/rustydb/pdb"),
            default_tablespace: "USERS".to_string(),
            temp_tablespace: "TEMP".to_string(),
            resource_limits: ResourceLimits::default(),
            flashback_enabled: false,
            force_logging: false,
            charset: "UTF8".to_string(),
            ncharset: "UTF8".to_string(),
            is_application_container: false,
            parent_app_container: None,
            snapshot_interval_secs: 0,
        }
    }
}

impl PdbConfig {
    /// Create a new PDB config builder
    pub fn builder() -> PdbConfigBuilder {
        PdbConfigBuilder::default()
    }
}

/// Builder for PDB configuration
#[derive(Default)]
pub struct PdbConfigBuilder {
    config: PdbConfig,
}

impl PdbConfigBuilder {
    pub fn name(mut self, name: impl Into<String>) -> Self {
        self.config.name = name.into();
        self
    }

    pub fn admin_user(mut self, user: impl Into<String>) -> Self {
        self.config.admin_user = user.into();
        self
    }

    pub fn admin_password(mut self, password: impl Into<String>) -> Self {
        self.config.admin_password = password.into();
        self
    }

    pub fn storage_quota(mut self, bytes: u64) -> Self {
        self.config.resource_limits.storage_quota_bytes = bytes;
        self
    }

    pub fn memory_limit(mut self, bytes: u64) -> Self {
        self.config.resource_limits.memory_bytes = bytes;
        self
    }

    pub fn cpu_shares(mut self, shares: u32) -> Self {
        self.config.resource_limits.cpu_shares = shares;
        self
    }

    pub fn build(self) -> PdbConfig {
        self.config
    }
}

/// PDB metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PdbMetadata {
    /// PDB GUID (globally unique identifier)
    pub guid: String,

    /// Creation timestamp
    pub created_at: u64,

    /// Last modified timestamp
    pub modified_at: u64,

    /// PDB version
    pub version: String,

    /// Open count (number of times opened)
    pub open_count: u64,

    /// Total size in bytes
    pub total_size_bytes: u64,

    /// Number of tablespaces
    pub tablespace_count: u32,

    /// Number of users
    pub user_count: u32,

    /// Number of tables
    pub table_count: u64,

    /// Source PDB ID (if cloned)
    pub source_pdb_id: Option<PdbId>,

    /// Tags for organization
    pub tags: HashMap<String, String>,
}

impl PdbMetadata {
    /// Create new PDB metadata
    pub fn new() -> Self {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        Self {
            guid: uuid::Uuid::new_v4().to_string(),
            created_at: now,
            modified_at: now,
            version: env!("CARGO_PKG_VERSION").to_string(),
            open_count: 0,
            total_size_bytes: 0,
            tablespace_count: 0,
            user_count: 0,
            table_count: 0,
            source_pdb_id: None,
            tags: HashMap::new(),
        }
    }

    /// Update modification timestamp
    pub fn touch(&mut self) {
        self.modified_at = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
    }
}

/// PDB snapshot
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PdbSnapshot {
    /// Snapshot ID
    pub id: u64,

    /// Snapshot name
    pub name: String,

    /// Source PDB ID
    pub source_pdb_id: PdbId,

    /// Creation timestamp
    pub created_at: u64,

    /// Snapshot SCN (System Change Number)
    pub scn: u64,

    /// Size in bytes
    pub size_bytes: u64,

    /// Snapshot type
    pub snapshot_type: SnapshotType,

    /// Description
    pub description: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SnapshotType {
    /// Full snapshot
    Full,
    /// Incremental snapshot
    Incremental,
    /// Copy-on-write snapshot
    CopyOnWrite,
}

/// Seed PDB for fast provisioning
#[derive(Debug, Clone)]
pub struct SeedPdb {
    /// Seed PDB configuration
    config: PdbConfig,

    /// Template metadata
    metadata: PdbMetadata,

    /// Pre-created objects
    objects: Vec<String>,
}

impl SeedPdb {
    /// Create a new seed PDB
    pub fn new(config: PdbConfig) -> Self {
        Self {
            config,
            metadata: PdbMetadata::new(),
            objects: Vec::new(),
        }
    }

    /// Clone from seed
    pub async fn clone(&self, name: &str) -> Result<PluggableDatabase> {
        let mut config = self.config.clone();
        config.name = name.to_string();
        config.create_mode = PdbCreateMode::FromSeed;

        PluggableDatabase::create(config).await
    }
}

/// Application Container
///
/// A special type of PDB that can contain other PDBs and share common objects
#[derive(Debug, Clone)]
pub struct ApplicationContainer {
    /// Base PDB
    pdb: PluggableDatabase,

    /// Child PDBs
    children: Arc<RwLock<HashMap<PdbId, Arc<RwLock<PluggableDatabase>>>>>,

    /// Common application objects
    common_objects: Arc<RwLock<Vec<String>>>,
}

impl ApplicationContainer {
    /// Create a new application container
    pub async fn new(config: PdbConfig) -> Result<Self> {
        let mut app_config = config;
        app_config.is_application_container = true;

        let pdb = PluggableDatabase::create(app_config).await?;

        Ok(Self {
            pdb,
            children: Arc::new(RwLock::new(HashMap::new())),
            common_objects: Arc::new(RwLock::new(Vec::new())),
        })
    }

    /// Add a child PDB
    pub async fn add_child(&self, child_id: PdbId, child: PluggableDatabase) -> Result<()> {
        self.children.write().await.insert(child_id, Arc::new(RwLock::new(child)));
        Ok(())
    }

    /// Remove a child PDB
    pub async fn remove_child(&self, child_id: PdbId) -> Result<()> {
        self.children.write().await.remove(&child_id);
        Ok(())
    }

    /// List child PDBs
    pub async fn list_children(&self) -> Vec<PdbId> {
        self.children.read().await.keys().copied().collect()
    }

    /// Add a common object
    pub async fn add_common_object(&self, object_name: String) -> Result<()> {
        self.common_objects.write().await.push(object_name);
        Ok(())
    }
}

/// Pluggable Database (PDB)
///
/// A fully isolated database instance within a CDB
#[derive(Debug, Clone)]
pub struct PluggableDatabase {
    /// PDB configuration
    config: PdbConfig,

    /// PDB metadata
    metadata: Arc<RwLock<PdbMetadata>>,

    /// Current lifecycle state
    state: Arc<RwLock<PdbLifecycleState>>,

    /// Current open mode
    mode: Arc<RwLock<Option<PdbMode>>>,

    /// Snapshots
    snapshots: Arc<RwLock<HashMap<u64, PdbSnapshot>>>,

    /// Next snapshot ID
    next_snapshot_id: Arc<RwLock<u64>>,

    /// Tablespaces
    tablespaces: Arc<RwLock<HashMap<String, Tablespace>>>,

    /// Users
    users: Arc<RwLock<HashMap<String, PdbUser>>>,

    /// Connection pool
    connections: Arc<RwLock<Vec<Connection>>>,

    /// Performance metrics
    metrics: Arc<RwLock<PdbMetrics>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tablespace {
    pub name: String,
    pub size_bytes: u64,
    pub used_bytes: u64,
    pub autoextend: bool,
    pub max_size_bytes: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PdbUser {
    pub username: String,
    pub password_hash: String,
    pub default_tablespace: String,
    pub temp_tablespace: String,
    pub locked: bool,
    pub created_at: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Connection {
    pub connection_id: u64,
    pub user: String,
    pub connected_at: u64,
    pub last_activity: u64,
    pub client_info: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PdbMetrics {
    /// Total queries executed
    pub queries_executed: u64,

    /// Total transactions committed
    pub transactions_committed: u64,

    /// Total transactions rolled back
    pub transactions_rolled_back: u64,

    /// Average query time (microseconds)
    pub avg_query_time_micros: u64,

    /// Peak connections
    pub peak_connections: u32,

    /// Current connections
    pub current_connections: u32,

    /// Bytes read
    pub bytes_read: u64,

    /// Bytes written
    pub bytes_written: u64,

    /// Cache hit ratio
    pub cache_hit_ratio: f64,
}

impl Default for PdbMetrics {
    fn default() -> Self {
        Self {
            queries_executed: 0,
            transactions_committed: 0,
            transactions_rolled_back: 0,
            avg_query_time_micros: 0,
            peak_connections: 0,
            current_connections: 0,
            bytes_read: 0,
            bytes_written: 0,
            cache_hit_ratio: 0.0,
        }
    }
}

impl PluggableDatabase {
    /// Create a new PDB
    pub async fn create(config: PdbConfig) -> Result<Self> {
        let metadata = Arc::new(RwLock::new(PdbMetadata::new()));
        let state = Arc::new(RwLock::new(PdbLifecycleState::Creating));

        let pdb = Self {
            config: config.clone(),
            metadata,
            state,
            mode: Arc::new(RwLock::new(None)),
            snapshots: Arc::new(RwLock::new(HashMap::new())),
            next_snapshot_id: Arc::new(RwLock::new(1)),
            tablespaces: Arc::new(RwLock::new(HashMap::new())),
            users: Arc::new(RwLock::new(HashMap::new())),
            connections: Arc::new(RwLock::new(Vec::new())),
            metrics: Arc::new(RwLock::new(PdbMetrics::default())),
        };

        // Initialize default tablespaces
        pdb.create_tablespace(&config.default_tablespace, 100 * 1024 * 1024).await?;
        pdb.create_tablespace(&config.temp_tablespace, 50 * 1024 * 1024).await?;

        // Create admin user
        pdb.create_user(PdbUser {
            username: config.admin_user.clone(),
            password_hash: config.admin_password.clone(),
            default_tablespace: config.default_tablespace.clone(),
            temp_tablespace: config.temp_tablespace.clone(),
            locked: false,
            created_at: SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs(),
        }).await?;

        // Update state to Closed
        *pdb.state.write().await = PdbLifecycleState::Closed;

        Ok(pdb)
    }

    /// Get PDB name
    pub fn name(&self) -> &str {
        &self.config.name
    }

    /// Get current state
    pub async fn state(&self) -> PdbLifecycleState {
        *self.state.read().await
    }

    /// Get current mode
    pub async fn mode(&self) -> Option<PdbMode> {
        *self.mode.read().await
    }

    /// Get resource limits
    pub fn resource_limits(&self) -> &ResourceLimits {
        &self.config.resource_limits
    }

    /// Open the PDB
    pub async fn open(&mut self) -> Result<()> {
        self.open_with_mode(PdbMode::ReadWrite).await
    }

    /// Open the PDB with specific mode
    pub async fn open_with_mode(&mut self, mode: PdbMode) -> Result<()> {
        let current_state = *self.state.read().await;

        if current_state != PdbLifecycleState::Closed
            && current_state != PdbLifecycleState::Mounted {
            return Err(DbError::InvalidState(
                format!("Cannot open PDB in state: {:?}", current_state)
            )));
        }

        // Mount first if not mounted
        if current_state == PdbLifecycleState::Closed {
            *self.state.write().await = PdbLifecycleState::Mounted;
        }

        // Open with specified mode
        match mode {
            PdbMode::ReadWrite => {
                *self.state.write().await = PdbLifecycleState::Open;
            }
            PdbMode::ReadOnly => {
                *self.state.write().await = PdbLifecycleState::ReadOnly;
            }
            PdbMode::Restricted => {
                *self.state.write().await = PdbLifecycleState::OpenRestricted;
            }
        }

        *self.mode.write().await = Some(mode);

        // Update metadata
        let mut metadata = self.metadata.write().await;
        metadata.open_count += 1;
        metadata.touch();

        Ok(())
    }

    /// Close the PDB
    pub async fn close(&mut self) -> Result<()> {
        let current_state = *self.state.read().await;

        if current_state == PdbLifecycleState::Closed {
            return Ok(());
        }

        *self.state.write().await = PdbLifecycleState::Closing;

        // Close all connections
        self.connections.write().await.clear();

        // Update metrics
        self.metrics.write().await.current_connections = 0;

        *self.state.write().await = PdbLifecycleState::Closed;
        *self.mode.write().await = None;

        Ok(())
    }

    /// Create a tablespace
    pub async fn create_tablespace(&self, name: &str, size_bytes: u64) -> Result<()> {
        let tablespace = Tablespace {
            name: name.to_string(),
            size_bytes,
            used_bytes: 0,
            autoextend: true,
            max_size_bytes: size_bytes * 10, // 10x initial size
        };

        self.tablespaces.write().await.insert(name.to_string(), tablespace);
        self.metadata.write().await.tablespace_count += 1;

        Ok(())
    }

    /// Create a user
    pub async fn create_user(&self, user: PdbUser) -> Result<()> {
        self.users.write().await.insert(user.username.clone(), user);
        self.metadata.write().await.user_count += 1;
        Ok(())
    }

    /// Create a snapshot
    pub async fn create_snapshot(&self, name: &str, scn: u64) -> Result<u64> {
        let mut next_id = self.next_snapshot_id.write().await;
        let snapshot_id = *next_id;
        *next_id += 1;

        let snapshot = PdbSnapshot {
            id: snapshot_id,
            name: name.to_string(),
            source_pdb_id: PdbId::new(0), // Will be set by caller
            created_at: SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs(),
            scn,
            size_bytes: self.metadata.read().await.total_size_bytes,
            snapshot_type: SnapshotType::Full,
            description: String::new(),
        };

        self.snapshots.write().await.insert(snapshot_id, snapshot);

        Ok(snapshot_id)
    }

    /// List snapshots
    pub async fn list_snapshots(&self) -> Vec<PdbSnapshot> {
        self.snapshots.read().await.values().cloned().collect()
    }

    /// Delete a snapshot
    pub async fn delete_snapshot(&self, snapshot_id: u64) -> Result<()> {
        self.snapshots.write().await.remove(&snapshot_id);
        Ok(())
    }

    /// Get metadata
    pub async fn metadata(&self) -> PdbMetadata {
        self.metadata.read().await.clone()
    }

    /// Get metrics
    pub async fn metrics(&self) -> PdbMetrics {
        self.metrics.read().await.clone()
    }

    /// Update metrics
    pub async fn update_metrics<F>(&self, f: F)
    where
        F: FnOnce(&mut PdbMetrics),
    {
        let mut metrics = self.metrics.write().await;
        f(&mut metrics);
    }

    /// Add a connection
    pub async fn add_connection(&self, connection: Connection) -> Result<()> {
        let mut connections = self.connections.write().await;
        let mut metrics = self.metrics.write().await;

        connections.push(connection);
        metrics.current_connections = connections.len() as u32;

        if metrics.current_connections > metrics.peak_connections {
            metrics.peak_connections = metrics.current_connections;
        }

        Ok(())
    }

    /// Remove a connection
    pub async fn remove_connection(&self, connection_id: u64) -> Result<()> {
        let mut connections = self.connections.write().await;
        connections.retain(|c| c.connection_id != connection_id);

        let mut metrics = self.metrics.write().await;
        metrics.current_connections = connections.len() as u32;

        Ok(())
    }

    /// Get connection count
    pub async fn connection_count(&self) -> usize {
        self.connections.read().await.len()
    }

    /// Check if connection limit is reached
    pub async fn is_connection_limit_reached(&self) -> bool {
        let count = self.connection_count().await;
        count >= self.config.resource_limits.max_connections as usize
    }

    /// Get tablespace usage
    pub async fn tablespace_usage(&self, name: &str) -> Option<(u64, u64)> {
        self.tablespaces
            .read()
            .await
            .get(name)
            .map(|ts| (ts.used_bytes, ts.size_bytes))
    }

    /// Check storage quota
    pub async fn check_storage_quota(&self) -> Result<()> {
        let metadata = self.metadata.read().await;
        let quota = self.config.resource_limits.storage_quota_bytes;

        if metadata.total_size_bytes >= quota {
            return Err(DbError::QuotaExceeded(
                format!("Storage quota exceeded: {} >= {}", metadata.total_size_bytes, quota)
            )));
        }

        Ok(())
    }

    /// Rename the PDB
    pub async fn rename(&mut self, newname: String) -> Result<()> {
        let current_state = *self.state.read().await;

        if current_state != PdbLifecycleState::Closed {
            return Err(DbError::InvalidState(
                "PDB must be closed to rename".to_string()
            ));
        }

        self.config.name = new_name;
        self.metadata.write().await.touch();

        Ok(())
    }

    /// Check if PDB is seed PDB
    pub fn is_seed(&self) -> bool {
        self.config.name == super::SEED_PDB_NAME
    }

    /// Check if PDB is application container
    pub fn is_application_container(&self) -> bool {
        self.config.is_application_container
    }
}

#[cfg(test)]
mod tests {
    use super::*;
use std::time::UNIX_EPOCH;

    #[tokio::test]
    async fn test_pdb_creation() {
        let config = PdbConfig {
            name: "TEST_PDB".to_string(),
            ..Default::default()
        };

        let pdb = PluggableDatabase::create(config).await;
        assert!(pdb.is_ok());
    }

    #[tokio::test]
    async fn test_pdb_lifecycle() {
        let config = PdbConfig {
            name: "TEST_PDB".to_string(),
            ..Default::default()
        };

        let mut pdb = PluggableDatabase::create(config).await.unwrap();
        assert_eq!(pdb.state().await, PdbLifecycleState::Closed);

        pdb.open().await.unwrap();
        assert_eq!(pdb.state().await, PdbLifecycleState::Open);

        pdb.close().await.unwrap();
        assert_eq!(pdb.state().await, PdbLifecycleState::Closed);
    }

    #[tokio::test]
    async fn test_snapshot_creation() {
        let config = PdbConfig {
            name: "TEST_PDB".to_string(),
            ..Default::default()
        };

        let pdb = PluggableDatabase::create(config).await.unwrap();
        let snapshot_id = pdb.create_snapshot("snap1", 1000).await.unwrap();
        assert_eq!(snapshot_id, 1);

        let snapshots = pdb.list_snapshots().await;
        assert_eq!(snapshots.len(), 1);
    }
}
