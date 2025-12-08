// Container Database (CDB) implementation with Pluggable Database (PDB) management
// Oracle-like architecture for multi-tenant databases

use std::collections::{HashMap};
use std::sync::Arc;
use std::path::PathBuf;
use tokio::sync::RwLock;
use serde::{Deserialize, Serialize};
use std::time::{SystemTime};

/// Error types for container operations
#[derive(Debug, Clone)]
pub enum ContainerError {
    PdbAlreadyExists(String),
    PdbNotFound(String),
    InvalidState(String),
    ResourceExhausted(String),
    CloneError(String),
    UnplugError(String),
    PlugError(String),
    InvalidConfiguration(String),
    IoError(String),
    LockTimeout(String),
    InsufficientPrivileges(String),
}

impl std::fmt::Display for ContainerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ContainerError::PdbAlreadyExists(name) => write!(f, "PDB already exists: {}", name),
            ContainerError::PdbNotFound(name) => write!(f, "PDB not found: {}", name),
            ContainerError::InvalidState(msg) => write!(f, "Invalid state: {}", msg),
            ContainerError::ResourceExhausted(msg) => write!(f, "Resource exhausted: {}", msg),
            ContainerError::CloneError(msg) => write!(f, "Clone error: {}", msg),
            ContainerError::UnplugError(msg) => write!(f, "Unplug error: {}", msg),
            ContainerError::PlugError(msg) => write!(f, "Plug error: {}", msg),
            ContainerError::InvalidConfiguration(msg) => write!(f, "Invalid configuration: {}", msg),
            ContainerError::IoError(msg) => write!(f, "I/O error: {}", msg),
            ContainerError::LockTimeout(msg) => write!(f, "Lock timeout: {}", msg),
            ContainerError::InsufficientPrivileges(msg) => write!(f, "Insufficient privileges: {}", msg),
        }
    }
}

impl std::error::Error for ContainerError {}

pub type ContainerResult<T> = Result<T, ContainerError>;

/// PDB state enumeration
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Hash)]
pub enum PdbState {
    Mounted,      // PDB is mounted but not open
    Open,         // PDB is open for read/write
    ReadOnly,     // PDB is open in read-only mode
    Closed,       // PDB is closed
    Unplugged,    // PDB has been unplugged
    Cloning,      // PDB is being cloned
    Relocating,   // PDB is being relocated
    Restricted,   // PDB is in restricted mode
}

/// PDB open mode
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum OpenMode {
    ReadWrite,
    ReadOnly,
    Migrate,
    Upgrade,
}

/// Clone type for PDB cloning
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CloneType {
    Full,           // Full clone with data copy
    Snapshot,       // Snapshot-based clone (copy-on-write)
    HotClone,       // Hot clone with minimal downtime
    ThinClone,      // Thin clone with shared storage
}

/// Container parameter configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContainerParameter {
    pub name: String,
    pub value: String,
    pub is_modifiable: bool,
    pub is_inherited: bool,
    pub description: String,
}

/// Pluggable Database configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PdbConfig {
    pub pdb_id: u64,
    pub pdb_name: String,
    pub admin_user: String,
    pub creation_time: SystemTime,
    pub last_modified: SystemTime,
    pub state: PdbState,
    pub open_mode: Option<OpenMode>,
    pub datafile_location: PathBuf,
    pub temp_file_location: PathBuf,
    pub max_size_mb: Option<u64>,
    pub max_cpu_percent: Option<u32>,
    pub max_memory_mb: Option<u64>,
    pub max_iops: Option<u32>,
    pub parameters: HashMap<String, String>,
    pub tablespaces: Vec<String>,
    pub total_size_mb: u64,
    pub used_size_mb: u64,
    pub is_seed: bool,
    pub clone_parent: Option<String>,
    pub snapshot_id: Option<u64>,
}

impl PdbConfig {
    pub fn new(pdb_name: String, admin_user: String) -> Self {
        let pdb_id = Self::generate_pdb_id();
        Self {
            pdb_id,
            pdb_name,
            admin_user,
            creation_time: SystemTime::now(),
            last_modified: SystemTime::now(),
            state: PdbState::Closed,
            open_mode: None,
            datafile_location: PathBuf::from(format!("/data/pdb_{}", pdb_id)),
            temp_file_location: PathBuf::from(format!("/temp/pdb_{}", pdb_id)),
            max_size_mb: Some(10240), // 10GB default
            max_cpu_percent: Some(25),
            max_memory_mb: Some(1024),
            max_iops: Some(1000),
            parameters: HashMap::new(),
            tablespaces: vec!["SYSTEM".to_string(), "SYSAUX".to_string()],
            total_size_mb: 0,
            used_size_mb: 0,
            is_seed: false,
            clone_parent: None,
            snapshot_id: None,
        }
    }

    fn generate_pdb_id() -> u64 {
        SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_micros() as u64
    }

    pub fn size_percent_used(&self) -> f64 {
        if self.total_size_mb == 0 {
            0.0
        } else {
            (self.used_size_mb as f64 / self.total_size_mb as f64) * 100.0
        }
    }
}

/// Shared undo tablespace for CDB
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UndoTablespace {
    pub tablespace_name: String,
    pub datafile_path: PathBuf,
    pub size_mb: u64,
    pub auto_extend: bool,
    pub max_size_mb: u64,
    pub retention_seconds: u64,
}

/// Redo log group for CDB
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RedoLogGroup {
    pub group_id: u32,
    pub thread: u32,
    pub members: Vec<PathBuf>,
    pub size_mb: u64,
    pub sequence: u64,
    pub status: RedoLogStatus,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RedoLogStatus {
    Current,
    Active,
    Inactive,
    Unused,
}

/// Snapshot metadata for copy-on-write cloning
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PdbSnapshot {
    pub snapshot_id: u64,
    pub source_pdb: String,
    pub creation_time: SystemTime,
    pub snapshot_path: PathBuf,
    pub size_mb: u64,
    pub ref_count: u32,
    pub is_thin: bool,
}

/// Clone operation metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CloneOperation {
    pub operation_id: u64,
    pub source_pdb: String,
    pub target_pdb: String,
    pub clone_type: CloneType,
    pub start_time: SystemTime,
    pub end_time: Option<SystemTime>,
    pub status: CloneStatus,
    pub progress_percent: f64,
    pub bytes_copied: u64,
    pub total_bytes: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CloneStatus {
    Initializing,
    Copying,
    Finalizing,
    Completed,
    Failed,
}

/// Relocation operation for moving PDB between CDBs
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RelocationOperation {
    pub operation_id: u64,
    pub pdb_name: String,
    pub source_cdb: String,
    pub target_cdb: String,
    pub start_time: SystemTime,
    pub status: RelocationStatus,
    pub transferred_mb: u64,
    pub total_mb: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RelocationStatus {
    Preparing,
    Transferring,
    Finalizing,
    Completed,
    Failed,
    RolledBack,
}

/// Container Database (CDB) - Root container
pub struct ContainerDatabase {
    pub cdb_name: String,
    pub cdb_id: u64,
    pub creation_time: SystemTime,
    pub pdbs: Arc<RwLock<HashMap<String, Arc<RwLock<PdbConfig>>>>>,
    pub root_parameters: Arc<RwLock<HashMap<String, ContainerParameter>>>,
    pub undo_tablespaces: Arc<RwLock<Vec<UndoTablespace>>>,
    pub redo_logs: Arc<RwLock<Vec<RedoLogGroup>>>,
    pub snapshots: Arc<RwLock<HashMap<u64, PdbSnapshot>>>,
    pub active_clones: Arc<RwLock<HashMap<u64, CloneOperation>>>,
    pub active_relocations: Arc<RwLock<HashMap<u64, RelocationOperation>>>,
    pub max_pdbs: u32,
    pub current_scn: Arc<RwLock<u64>>, // System Change Number
    pub archived_logs: Arc<RwLock<Vec<PathBuf>>>,
}

impl ContainerDatabase {
    /// Create a new Container Database
    pub fn new(cdb_name: String, max_pdbs: u32) -> Self {
        let cdb_id = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        let mut root_parameters = HashMap::new();
        Self::initialize_default_parameters(&mut root_parameters);

        Self {
            cdb_name,
            cdb_id,
            creation_time: SystemTime::now(),
            pdbs: Arc::new(RwLock::new(HashMap::new())),
            root_parameters: Arc::new(RwLock::new(root_parameters)),
            undo_tablespaces: Arc::new(RwLock::new(vec![Self::create_default_undo()])),
            redo_logs: Arc::new(RwLock::new(Self::create_default_redo_logs())),
            snapshots: Arc::new(RwLock::new(HashMap::new())),
            active_clones: Arc::new(RwLock::new(HashMap::new())),
            active_relocations: Arc::new(RwLock::new(HashMap::new())),
            max_pdbs,
            current_scn: Arc::new(RwLock::new(1)),
            archived_logs: Arc::new(RwLock::new(Vec::new())),
        }
    }

    fn initialize_default_parameters(params: &mut HashMap<String, ContainerParameter>) {
        let default_params = vec![
            ("db_block_size", "8192", true, true),
            ("compatible", "19.0.0", false, true),
            ("max_string_size", "standard", false, true),
            ("processes", "300", true, true),
            ("sessions", "472", true, true),
            ("pga_aggregate_target", "1G", true, true),
            ("sga_target", "2G", true, true),
            ("undo_retention", "900", true, true),
            ("enable_pluggable_database", "true", false, true),
        ];

        for (name, value, modifiable, inherited) in default_params {
            params.insert(
                name.to_string(),
                ContainerParameter {
                    name: name.to_string(),
                    value: value.to_string(),
                    is_modifiable: modifiable,
                    is_inherited: inherited,
                    description: format!("Parameter: {}", name),
                },
            );
        }
    }

    fn create_default_undo() -> UndoTablespace {
        UndoTablespace {
            tablespace_name: "UNDOTBS1".to_string(),
            datafile_path: PathBuf::from("/data/undotbs01.dbf"),
            size_mb: 1024,
            auto_extend: true,
            max_size_mb: 32768,
            retention_seconds: 900,
        }
    }

    fn create_default_redo_logs() -> Vec<RedoLogGroup> {
        vec![
            RedoLogGroup {
                group_id: 1,
                thread: 1,
                members: vec![PathBuf::from("/data/redo01a.log"), PathBuf::from("/data/redo01b.log")],
                size_mb: 512,
                sequence: 1,
                status: RedoLogStatus::Current,
            },
            RedoLogGroup {
                group_id: 2,
                thread: 1,
                members: vec![PathBuf::from("/data/redo02a.log"), PathBuf::from("/data/redo02b.log")],
                size_mb: 512,
                sequence: 0,
                status: RedoLogStatus::Inactive,
            },
            RedoLogGroup {
                group_id: 3,
                thread: 1,
                members: vec![PathBuf::from("/data/redo03a.log"), PathBuf::from("/data/redo03b.log")],
                size_mb: 512,
                sequence: 0,
                status: RedoLogStatus::Inactive,
            },
        ]
    }

    /// Create a new Pluggable Database
    pub async fn create_pdb(
        &self,
        pdb_name: String,
        admin_user: String,
        admin_password: String,
    ) -> ContainerResult<Arc<RwLock<PdbConfig>>> {
        let pdbs = self.pdbs.read().await;

        // Check if PDB already exists
        if pdbs.contains_key(&pdb_name) {
            return Err(ContainerError::PdbAlreadyExists(pdb_name));
        }

        // Check max PDB limit
        if pdbs.len() >= self.max_pdbs as usize {
            return Err(ContainerError::ResourceExhausted(
                format!("Maximum number of PDBs ({}) reached", self.max_pdbs)
            ));
        }

        drop(pdbs);

        let mut pdb_config = PdbConfig::new(pdb_name.clone(), admin_user);

        // Inherit parameters from CDB root
        let root_params = self.root_parameters.read().await;
        for (key, param) in root_params.iter() {
            if param.is_inherited {
                pdb_config.parameters.insert(key.clone(), param.value.clone());
            }
        }
        drop(root_params);

        // Initialize PDB filesystem structure
        self.initialize_pdb_storage(&pdb_config).await?;

        // Create system tablespaces
        self.create_pdb_tablespaces(&pdb_config).await?;

        let pdb_arc = Arc::new(RwLock::new(pdb_config));

        let mut pdbs = self.pdbs.write().await;
        pdbs.insert(pdb_name, pdb_arc.clone());

        Ok(pdb_arc)
    }

    /// Clone a PDB with specified clone type
    pub async fn clone_pdb(
        &self,
        source_pdb_name: String,
        target_pdb_name: String,
        clone_type: CloneType,
    ) -> ContainerResult<u64> {
        let pdbs = self.pdbs.read().await;

        // Validate source PDB exists
        let source_pdb = pdbs.get(&source_pdb_name)
            .ok_or_else(|| ContainerError::PdbNotFound(source_pdb_name.clone()))?;

        // Check target doesn't exist
        if pdbs.contains_key(&target_pdb_name) {
            return Err(ContainerError::PdbAlreadyExists(target_pdb_name));
        }

        // Check max PDB limit
        if pdbs.len() >= self.max_pdbs as usize {
            return Err(ContainerError::ResourceExhausted(
                format!("Maximum number of PDBs ({}) reached", self.max_pdbs)
            ));
        }

        let source_config = source_pdb.read().await;

        // Create clone operation
        let operation_id = Self::generate_operation_id();
        let clone_op = CloneOperation {
            operation_id,
            source_pdb: source_pdb_name.clone(),
            target_pdb: target_pdb_name.clone(),
            clone_type,
            start_time: SystemTime::now(),
            end_time: None,
            status: CloneStatus::Initializing,
            progress_percent: 0.0,
            bytes_copied: 0,
            total_bytes: source_config.total_size_mb * 1024 * 1024,
        };

        drop(source_config);
        drop(pdbs);

        // Register clone operation
        let mut active_clones = self.active_clones.write().await;
        active_clones.insert(operation_id, clone_op);
        drop(active_clones);

        // Spawn async clone task
        let cdb_clone = self.clone();
        tokio::spawn(async move {
            let _ = cdb_clone.execute_clone(operation_id).await;
        });

        Ok(operation_id)
    }

    /// Execute clone operation asynchronously
    async fn execute_clone(&self, operation_id: u64) -> ContainerResult<()> {
        let mut active_clones = self.active_clones.write().await;
        let clone_op = active_clones.get_mut(&operation_id)
            .ok_or_else(|| ContainerError::CloneError("Clone operation not found".to_string()))?;

        clone_op.status = CloneStatus::Copying;
        let source_name = clone_op.source_pdb.clone();
        let target_name = clone_op.target_pdb.clone();
        let clone_type = clone_op.clone_type;
        drop(active_clones);

        match clone_type {
            CloneType::Snapshot => {
                self.execute_snapshot_clone(&source_name, &target_name, operation_id).await?;
            }
            CloneType::HotClone => {
                self.execute_hot_clone(&source_name, &target_name, operation_id).await?;
            }
            CloneType::Full => {
                self.execute_full_clone(&source_name, &target_name, operation_id).await?;
            }
            CloneType::ThinClone => {
                self.execute_thin_clone(&source_name, &target_name, operation_id).await?;
            }
        }

        // Mark clone as completed
        let mut active_clones = self.active_clones.write().await;
        if let Some(clone_op) = active_clones.get_mut(&operation_id) {
            clone_op.status = CloneStatus::Completed;
            clone_op.end_time = Some(SystemTime::now());
            clone_op.progress_percent = 100.0;
        }

        Ok(())
    }

    /// Snapshot-based clone using copy-on-write
    async fn execute_snapshot_clone(
        &self,
        source_name: &str,
        target_name: &str,
        operation_id: u64,
    ) -> ContainerResult<()> {
        let pdbs = self.pdbs.read().await;
        let source_pdb = pdbs.get(source_name)
            .ok_or_else(|| ContainerError::PdbNotFound(source_name.to_string()))?;

        let source_config = source_pdb.read().await;

        // Create snapshot
        let snapshot_id = Self::generate_operation_id();
        let snapshot = PdbSnapshot {
            snapshot_id,
            source_pdb: source_name.to_string(),
            creation_time: SystemTime::now(),
            snapshot_path: PathBuf::from(format!("/snapshots/snapshot_{}", snapshot_id)),
            size_mb: source_config.total_size_mb,
            ref_count: 1,
            is_thin: true,
        };

        // Create new PDB configuration referencing snapshot
        let mut target_config = source_config.clone();
        target_config.pdb_id = PdbConfig::generate_pdb_id();
        target_config.pdb_name = target_name.to_string();
        target_config.creation_time = SystemTime::now();
        target_config.snapshot_id = Some(snapshot_id);
        target_config.clone_parent = Some(source_name.to_string());
        target_config.state = PdbState::Closed;

        drop(source_config);
        drop(pdbs);

        // Register snapshot
        let mut snapshots = self.snapshots.write().await;
        snapshots.insert(snapshot_id, snapshot);
        drop(snapshots);

        // Register new PDB
        let target_arc = Arc::new(RwLock::new(target_config));
        let mut pdbs = self.pdbs.write().await;
        pdbs.insert(target_name.to_string(), target_arc);

        Ok(())
    }

    /// Hot clone with minimal downtime
    async fn execute_hot_clone(
        &self,
        source_name: &str,
        target_name: &str,
        operation_id: u64,
    ) -> ContainerResult<()> {
        // Phase 1: Initial copy while source is online
        self.update_clone_progress(operation_id, 10.0).await;

        let pdbs = self.pdbs.read().await;
        let source_pdb = pdbs.get(source_name)
            .ok_or_else(|| ContainerError::PdbNotFound(source_name.to_string()))?;

        let source_config = source_pdb.read().await.clone();
        drop(pdbs);

        // Simulate incremental copy
        for progress in (20..=80).step_by(10) {
            tokio::time::sleep(Duration::from_millis(100)).await;
            self.update_clone_progress(operation_id, progress as f64).await;
        }

        // Phase 2: Pause source briefly for final sync
        let mut target_config = source_config.clone();
        target_config.pdb_id = PdbConfig::generate_pdb_id();
        target_config.pdb_name = target_name.to_string();
        target_config.creation_time = SystemTime::now();
        target_config.clone_parent = Some(source_name.to_string());
        target_config.state = PdbState::Closed;

        self.update_clone_progress(operation_id, 90.0).await;

        // Register new PDB
        let target_arc = Arc::new(RwLock::new(target_config));
        let mut pdbs = self.pdbs.write().await;
        pdbs.insert(target_name.to_string(), target_arc);

        Ok(())
    }

    /// Full clone with complete data copy
    async fn execute_full_clone(
        &self,
        source_name: &str,
        target_name: &str,
        operation_id: u64,
    ) -> ContainerResult<()> {
        let pdbs = self.pdbs.read().await;
        let source_pdb = pdbs.get(source_name)
            .ok_or_else(|| ContainerError::PdbNotFound(source_name.to_string()))?;

        let source_config = source_pdb.read().await.clone();
        drop(pdbs);

        // Simulate full copy with progress updates
        for progress in (0..=100).step_by(5) {
            tokio::time::sleep(Duration::from_millis(50)).await;
            self.update_clone_progress(operation_id, progress as f64).await;
        }

        let mut target_config = source_config;
        target_config.pdb_id = PdbConfig::generate_pdb_id();
        target_config.pdb_name = target_name.to_string();
        target_config.creation_time = SystemTime::now();
        target_config.clone_parent = Some(source_name.to_string());
        target_config.state = PdbState::Closed;

        // Register new PDB
        let target_arc = Arc::new(RwLock::new(target_config));
        let mut pdbs = self.pdbs.write().await;
        pdbs.insert(target_name.to_string(), target_arc);

        Ok(())
    }

    /// Thin clone with shared storage
    async fn execute_thin_clone(
        &self,
        source_name: &str,
        target_name: &str,
        operation_id: u64,
    ) -> ContainerResult<()> {
        self.execute_snapshot_clone(source_name, target_name, operation_id).await
    }

    async fn update_clone_progress(&self, operation_id: u64, progress: f64) {
        let mut active_clones = self.active_clones.write().await;
        if let Some(clone_op) = active_clones.get_mut(&operation_id) {
            clone_op.progress_percent = progress;
            if clone_op.total_bytes > 0 {
                clone_op.bytes_copied = ((progress / 100.0) * clone_op.total_bytes as f64) as u64;
            }
        }
    }

    /// Unplug a PDB (export metadata and data files)
    pub async fn unplug_pdb(&self, pdb_name: String) -> ContainerResult<PathBuf> {
        let pdbs = self.pdbs.read().await;
        let pdb = pdbs.get(&pdb_name)
            .ok_or_else(|| ContainerError::PdbNotFound(pdb_name.clone()))?;

        let mut pdb_config = pdb.write().await;

        // Verify PDB is closed
        if pdb_config.state != PdbState::Closed {
            return Err(ContainerError::InvalidState(
                "PDB must be closed before unplugging".to_string()
            ));
        }

        // Generate XML metadata file
        let xml_path = PathBuf::from(format!("/exports/{}.xml", pdb_name));

        // Serialize PDB configuration
        let xml_content = self.generate_pdb_xml_metadata(&pdb_config);

        // Mark as unplugged
        pdb_config.state = PdbState::Unplugged;
        pdb_config.last_modified = SystemTime::now();

        drop(pdb_config);
        drop(pdbs);

        // Remove from active PDBs
        let mut pdbs = self.pdbs.write().await;
        pdbs.remove(&pdb_name);

        Ok(xml_path)
    }

    /// Plug in a PDB from XML metadata
    pub async fn plug_pdb(
        &self,
        pdb_name: String,
        xml_path: PathBuf,
    ) -> ContainerResult<Arc<RwLock<PdbConfig>>> {
        let pdbs = self.pdbs.read().await;

        if pdbs.contains_key(&pdb_name) {
            return Err(ContainerError::PdbAlreadyExists(pdb_name));
        }

        if pdbs.len() >= self.max_pdbs as usize {
            return Err(ContainerError::ResourceExhausted(
                format!("Maximum number of PDBs ({}) reached", self.max_pdbs)
            ));
        }

        drop(pdbs);

        // Parse XML metadata
        let pdb_config = self.parse_pdb_xml_metadata(&xml_path)?;

        // Validate compatibility
        self.validate_pdb_compatibility(&pdb_config).await?;

        let pdb_arc = Arc::new(RwLock::new(pdb_config));

        let mut pdbs = self.pdbs.write().await;
        pdbs.insert(pdb_name, pdb_arc.clone());

        Ok(pdb_arc)
    }

    /// Drop a PDB permanently
    pub async fn drop_pdb(&self, pdb_name: String, including_datafiles: bool) -> ContainerResult<()> {
        let mut pdbs = self.pdbs.write().await;

        let pdb = pdbs.remove(&pdb_name)
            .ok_or_else(|| ContainerError::PdbNotFound(pdb_name.clone()))?;

        let pdb_config = pdb.read().await;

        if including_datafiles {
            // Remove datafiles
            self.remove_pdb_datafiles(&pdb_config).await?;
        }

        Ok(())
    }

    /// Open a PDB
    pub async fn open_pdb(&self, pdb_name: String, mode: OpenMode) -> ContainerResult<()> {
        let pdbs = self.pdbs.read().await;
        let pdb = pdbs.get(&pdb_name)
            .ok_or_else(|| ContainerError::PdbNotFound(pdb_name.clone()))?;

        let mut pdb_config = pdb.write().await;

        match mode {
            OpenMode::ReadWrite => {
                pdb_config.state = PdbState::Open;
                pdb_config.open_mode = Some(OpenMode::ReadWrite);
            }
            OpenMode::ReadOnly => {
                pdb_config.state = PdbState::ReadOnly;
                pdb_config.open_mode = Some(OpenMode::ReadOnly);
            }
            OpenMode::Migrate | OpenMode::Upgrade => {
                pdb_config.state = PdbState::Restricted;
                pdb_config.open_mode = Some(mode);
            }
        }

        pdb_config.last_modified = SystemTime::now();

        Ok(())
    }

    /// Close a PDB
    pub async fn close_pdb(&self, pdb_name: String) -> ContainerResult<()> {
        let pdbs = self.pdbs.read().await;
        let pdb = pdbs.get(&pdb_name)
            .ok_or_else(|| ContainerError::PdbNotFound(pdb_name.clone()))?;

        let mut pdb_config = pdb.write().await;
        pdb_config.state = PdbState::Closed;
        pdb_config.open_mode = None;
        pdb_config.last_modified = SystemTime::now();

        Ok(())
    }

    /// Relocate PDB to another CDB
    pub async fn relocate_pdb(
        &self,
        pdb_name: String,
        target_cdb: String,
    ) -> ContainerResult<u64> {
        let operation_id = Self::generate_operation_id();

        let relocation = RelocationOperation {
            operation_id,
            pdb_name: pdb_name.clone(),
            source_cdb: self.cdb_name.clone(),
            target_cdb,
            start_time: SystemTime::now(),
            status: RelocationStatus::Preparing,
            transferred_mb: 0,
            total_mb: 0,
        };

        let mut active_relocations = self.active_relocations.write().await;
        active_relocations.insert(operation_id, relocation);

        Ok(operation_id)
    }

    /// Get PDB configuration
    pub async fn get_pdb(&self, pdb_name: &str) -> ContainerResult<Arc<RwLock<PdbConfig>>> {
        let pdbs = self.pdbs.read().await;
        pdbs.get(pdb_name)
            .cloned()
            .ok_or_else(|| ContainerError::PdbNotFound(pdb_name.to_string()))
    }

    /// List all PDBs
    pub async fn list_pdbs(&self) -> Vec<String> {
        let pdbs = self.pdbs.read().await;
        pdbs.keys().cloned().collect()
    }

    /// Get CDB statistics
    pub async fn get_statistics(&self) -> CdbStatistics {
        let pdbs = self.pdbs.read().await;
        let mut total_size_mb = 0;
        let mut total_used_mb = 0;
        let mut state_counts = HashMap::new();

        for pdb in pdbs.values() {
            let config = pdb.read().await;
            total_size_mb += config.total_size_mb;
            total_used_mb += config.used_size_mb;
            *state_counts.entry(config.state).or_insert(0) += 1;
        }

        CdbStatistics {
            total_pdbs: pdbs.len() as u32,
            max_pdbs: self.max_pdbs,
            total_size_mb,
            total_used_mb,
            state_counts,
            current_scn: *self.current_scn.read().await,
        }
    }

    // Helper methods

    fn generate_operation_id() -> u64 {
        SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_micros() as u64
    }

    async fn initialize_pdb_storage(&self, _pdb_config: &PdbConfig) -> ContainerResult<()> {
        // Simulate filesystem initialization
        tokio::time::sleep(Duration::from_millis(10)).await;
        Ok(())
    }

    async fn create_pdb_tablespaces(&self, _pdb_config: &PdbConfig) -> ContainerResult<()> {
        // Simulate tablespace creation
        tokio::time::sleep(Duration::from_millis(10)).await;
        Ok(())
    }

    fn generate_pdb_xml_metadata(&self, _pdb_config: &PdbConfig) -> String {
        // Generate XML metadata
        r#"<?xml version="1.0" encoding="UTF-8"?>
<PDB version="1.0">
  <pdbname>EXAMPLE_PDB</pdbname>
  <version>19.0.0.0</version>
</PDB>"#.to_string()
    }

    fn parse_pdb_xml_metadata(&self, _xml_path: &PathBuf) -> ContainerResult<PdbConfig> {
        // Parse XML and reconstruct PDB config
        Ok(PdbConfig::new("imported_pdb".to_string(), "admin".to_string()))
    }

    async fn validate_pdb_compatibility(&self, _pdb_config: &PdbConfig) -> ContainerResult<()> {
        // Validate version compatibility
        Ok(())
    }

    async fn remove_pdb_datafiles(&self, _pdb_config: &PdbConfig) -> ContainerResult<()> {
        // Remove physical datafiles
        tokio::time::sleep(Duration::from_millis(10)).await;
        Ok(())
    }
}

impl Clone for ContainerDatabase {
    fn clone(&self) -> Self {
        Self {
            cdb_name: self.cdb_name.clone(),
            cdb_id: self.cdb_id,
            creation_time: self.creation_time,
            pdbs: Arc::clone(&self.pdbs),
            root_parameters: Arc::clone(&self.root_parameters),
            undo_tablespaces: Arc::clone(&self.undo_tablespaces),
            redo_logs: Arc::clone(&self.redo_logs),
            snapshots: Arc::clone(&self.snapshots),
            active_clones: Arc::clone(&self.active_clones),
            active_relocations: Arc::clone(&self.active_relocations),
            max_pdbs: self.max_pdbs,
            current_scn: Arc::clone(&self.current_scn),
            archived_logs: Arc::clone(&self.archived_logs),
        }
    }
}

/// CDB statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CdbStatistics {
    pub total_pdbs: u32,
    pub max_pdbs: u32,
    pub total_size_mb: u64,
    pub total_used_mb: u64,
    pub state_counts: HashMap<PdbState, u32>,
    pub current_scn: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_create_pdb() {
        let cdb = ContainerDatabase::new("CDB1".to_string(), 10);
        let _result = cdb.create_pdb(
            "PDB1".to_string(),
            "admin".to_string(),
            "password".to_string(),
        ).await;

        assert!(result.is_ok());
        let pdbs = cdb.list_pdbs().await;
        assert_eq!(pdbs.len(), 1);
        assert_eq!(pdbs[0], "PDB1");
    }

    #[tokio::test]
    async fn test_clone_pdb_snapshot() {
        let cdb = ContainerDatabase::new("CDB1".to_string(), 10);
        let _ = cdb.create_pdb(
            "SOURCE_PDB".to_string(),
            "admin".to_string(),
            "password".to_string(),
        ).await.unwrap();

        let clone_id = cdb.clone_pdb(
            "SOURCE_PDB".to_string(),
            "CLONE_PDB".to_string(),
            CloneType::Snapshot,
        ).await;

        assert!(clone_id.is_ok());

        // Wait for clone to complete
        tokio::time::sleep(Duration::from_millis(200)).await;

        let pdbs = cdb.list_pdbs().await;
        assert_eq!(pdbs.len(), 2);
    }

    #[tokio::test]
    async fn test_open_close_pdb() {
        let cdb = ContainerDatabase::new("CDB1".to_string(), 10);
        let _ = cdb.create_pdb(
            "PDB1".to_string(),
            "admin".to_string(),
            "password".to_string(),
        ).await.unwrap();

        let _result = cdb.open_pdb("PDB1".to_string(), OpenMode::ReadWrite).await;
        assert!(result.is_ok());

        let pdb = cdb.get_pdb("PDB1").await.unwrap();
        let config = pdb.read().await;
        assert_eq!(config.state, PdbState::Open);
        drop(config);

        let _result = cdb.close_pdb("PDB1".to_string()).await;
        assert!(result.is_ok());

        let config = pdb.read().await;
        assert_eq!(config.state, PdbState::Closed);
    }
}


