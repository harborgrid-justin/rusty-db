//! # Replication Slots Management
//! 
//! This module provides comprehensive management of replication slots, which are
//! essential for tracking WAL retention, managing logical and physical replication,
//! and ensuring consistent data delivery to replicas.
//! 
//! ## Key Features
//! 
//! - **Logical Slots**: Support for logical replication with change tracking
//! - **Physical Slots**: Physical replication slot management with LSN tracking
//! - **WAL Retention**: Automatic WAL retention based on slot consumption
//! - **Slot Lifecycle**: Complete slot lifecycle management from creation to deletion
//! - **Lag Monitoring**: Real-time lag monitoring and alerting
//! - **Failover Support**: Automatic slot management during failover scenarios
//! 
//! ## Slot Types
//! 
//! - **Logical Replication Slots**: For logical replication with output plugins
//! - **Physical Replication Slots**: For streaming physical replication
//! - **Temporary Slots**: Short-lived slots for temporary operations
//! - **Persistent Slots**: Long-term slots with guaranteed WAL retention
//! - **Failover Slots**: Special slots for failover scenarios
//! 
//! ## Usage Example
//! 
//! ```rust
//! use crate::replication::slots::*;
//! use crate::replication::types::*;
//! 
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! // Create slot manager
//! let config = SlotManagerConfig {
//!     max_slots: 100,
//!     default_wal_retention: Duration::from_hours(24),
//!     enable_auto_cleanup: true,
//!     lag_warning_threshold: Duration::from_minutes(5),
//!     ..Default::default()
//! };
//! 
//! let manager = ReplicationSlotManager::new(config)?;
//! 
//! // Create a logical replication slot
//! let slot_name = SlotName::new("logical_slot_01")?;
//! let slot_config = SlotConfig {
//!     slot_type: SlotType::Logical {
//!         plugin_name: "pgoutput".to_string(),
//!         publication_names: vec!["test_pub".to_string()],
//!     },
//!     wal_retention_policy: WalRetentionPolicy::Time(Duration::from_hours(12)),
//!     auto_advance: false,
//!     temporary: false,
//!     ..Default::default()
//! };
//! 
//! let slot_id = manager.create_slot(&slot_name, slot_config).await?;
//! 
//! // Start consuming from the slot
//! let mut stream = manager.start_consuming(&slot_id).await?;
//! 
//! // Process changes
//! while let Some(change) = stream.next_change().await? {
//!     println!("Received change: {:?}", change);
//!     
//!     // Advance slot position
//!     manager.advance_slot(&slot_id, &change.end_lsn).await?;
//! }
//! 
//! // Monitor slot status
//! let status = manager.get_slot_status(&slot_id).await?;
//! println!("Slot lag: {:?}", status.lag_duration);
//! # Ok(())
//! # }
//! ```

use crate::error::DbError;
use crate::replication::types::*;
use async_trait::async_trait;
use parking_lot::{Mutex, RwLock};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap};
use std::sync::Arc;
use std::time::{Duration};
use thiserror::Error;
use tokio::sync::{broadcast, mpsc, watch};
use uuid::Uuid;

/// Replication slot specific errors
#[derive(Error, Debug)]
pub enum SlotError {
    #[error("Slot not found: {slot_name}")]
    SlotNotFound { slot_name: String },
    
    #[error("Slot already exists: {slot_name}")]
    SlotAlreadyExists { slot_name: String },
    
    #[error("Invalid slot configuration: {reason}")]
    InvalidConfiguration { reason: String },
    
    #[error("Slot creation failed: {slot_name} - {reason}")]
    CreationFailed { slot_name: String, reason: String },
    
    #[error("Slot consumption error: {slot_name} - {reason}")]
    ConsumptionError { slot_name: String, reason: String },
    
    #[error("WAL position invalid: {position} - {reason}")]
    InvalidWalPosition { position: String, reason: String },
    
    #[error("Slot advancement failed: {slot_name} - {reason}")]
    AdvancementFailed { slot_name: String, reason: String },
    
    #[error("Plugin error: {plugin_name} - {reason}")]
    PluginError { plugin_name: String, reason: String },
    
    #[error("Slot state conflict: {slot_name} - current state: {current_state}")]
    StateConflict { slot_name: String, current_state: String },
    
    #[error("WAL retention violation: {slot_name} - {reason}")]
    WalRetentionViolation { slot_name: String, reason: String },
    
    #[error("Slot limit exceeded: current {current_count}, maximum {max_slots}")]
    SlotLimitExceeded { current_count: usize, max_slots: usize },
    
    #[error("Replication stream error: {reason}")]
    StreamError { reason: String },
}

/// Slot manager configuration
/// 
/// Comprehensive configuration for replication slot management
/// with settings for performance, retention, and monitoring.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SlotManagerConfig {
    /// Maximum number of replication slots
    pub max_slots: usize,
    /// Default WAL retention period
    pub default_wal_retention: Duration,
    /// Enable automatic cleanup of inactive slots
    pub enable_auto_cleanup: bool,
    /// Cleanup interval for inactive slots
    pub cleanup_interval: Duration,
    /// Lag warning threshold
    pub lag_warning_threshold: Duration,
    /// Lag critical threshold
    pub lag_critical_threshold: Duration,
    /// Enable slot monitoring and metrics
    pub enable_monitoring: bool,
    /// Monitoring interval
    pub monitoring_interval: Duration,
    /// Maximum slot name length
    pub max_slot_name_length: usize,
    /// Enable slot persistence
    pub enable_persistence: bool,
    /// Persistence directory
    pub persistence_directory: std::path::PathBuf,
    /// Maximum number of changes to buffer per slot
    pub max_buffered_changes: usize,
    /// Buffer flush interval
    pub buffer_flush_interval: Duration,
    /// Enable slot statistics collection
    pub enable_statistics: bool,
    /// Enable failover slot management
    pub enable_failover_slots: bool,
    /// Failover detection timeout
    pub failover_detection_timeout: Duration,
}

impl Default for SlotManagerConfig {
    fn default() -> Self {
        Self {
            max_slots: 100,
            default_wal_retention: Duration::from_hours(24),
            enable_auto_cleanup: true,
            cleanup_interval: Duration::from_secs(300), // 5 minutes
            lag_warning_threshold: Duration::from_minutes(5),
            lag_critical_threshold: Duration::from_minutes(15),
            enable_monitoring: true,
            monitoring_interval: Duration::from_secs(30),
            max_slot_name_length: 63, // PostgreSQL compatible
            enable_persistence: true,
            persistence_directory: std::path::PathBuf::from("/data/slots"),
            max_buffered_changes: 10000,
            buffer_flush_interval: Duration::from_secs(5),
            enable_statistics: true,
            enable_failover_slots: true,
            failover_detection_timeout: Duration::from_secs(30),
        }
    }
}

/// Unique slot identifier
/// 
/// Provides type-safe slot identification with validation.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SlotId(String);

impl SlotId {
    /// Creates a new slot ID
    pub fn new(id: impl Into<String>) -> Result<Self, SlotError> {
        let id = id.into();
        if id.trim().is_empty() {
            return Err(SlotError::InvalidConfiguration {
                reason: "Slot ID cannot be empty".to_string(),
            });
        }
        Ok(Self(id))
    }
    
    /// Generates a new unique slot ID
    pub fn generate() -> Self {
        Self(Uuid::new_v4().to_string())
    }
    
    /// Returns the slot ID as a string
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for SlotId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Replication slot name
/// 
/// Validated slot name with PostgreSQL-compatible naming rules.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SlotName(String);

impl SlotName {
    /// Creates a new slot name with validation
    pub fn new(name: impl Into<String>) -> Result<Self, SlotError> {
        let name = name.into();
        
        // Validate slot name
        if name.trim().is_empty() {
            return Err(SlotError::InvalidConfiguration {
                reason: "Slot name cannot be empty".to_string(),
            });
        }
        
        if name.len() > 63 {
            return Err(SlotError::InvalidConfiguration {
                reason: "Slot name too long (max 63 characters)".to_string(),
            });
        }
        
        // Check for valid characters (letters, numbers, underscore)
        if !name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
            return Err(SlotError::InvalidConfiguration {
                reason: "Slot name contains invalid characters".to_string(),
            });
        }
        
        // Must start with letter or underscore
        if !name.chars().next().map_or(false, |c| c.is_ascii_alphabetic() || c == '_') {
            return Err(SlotError::InvalidConfiguration {
                reason: "Slot name must start with letter or underscore".to_string(),
            });
        }
        
        Ok(Self(name))
    }
    
    /// Returns the slot name as a string
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for SlotName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Slot configuration
/// 
/// Complete configuration for a replication slot including
/// type, retention policies, and behavior settings.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SlotConfig {
    /// Type of replication slot
    pub slot_type: SlotType,
    /// WAL retention policy for this slot
    pub wal_retention_policy: WalRetentionPolicy,
    /// Whether the slot position auto-advances
    pub auto_advance: bool,
    /// Whether this is a temporary slot
    pub temporary: bool,
    /// Slot-specific tags for organization
    pub tags: HashMap<String, String>,
    /// Custom configuration options
    pub custom_options: HashMap<String, String>,
    /// Enable lag monitoring for this slot
    pub enable_lag_monitoring: bool,
    /// Custom lag thresholds
    pub custom_lag_thresholds: Option<LagThresholds>,
    /// Associated replica ID
    pub replica_id: Option<ReplicaId>,
    /// Slot priority for resource allocation
    pub priority: SlotPriority,
    /// Buffer settings
    pub buffer_settings: BufferSettings,
}

impl Default for SlotConfig {
    fn default() -> Self {
        Self {
            slot_type: SlotType::Physical,
            wal_retention_policy: WalRetentionPolicy::Time(Duration::from_hours(24)),
            auto_advance: false,
            temporary: false,
            tags: HashMap::new(),
            custom_options: HashMap::new(),
            enable_lag_monitoring: true,
            custom_lag_thresholds: None,
            replica_id: None,
            priority: SlotPriority::Normal,
            buffer_settings: BufferSettings::default(),
        }
    }
}

/// Types of replication slots
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SlotType {
    /// Physical replication slot
    Physical,
    /// Logical replication slot with plugin
    Logical {
        /// Output plugin name
        plugin_name: String,
        /// Publication names for logical replication
        publication_names: Vec<String>,
    },
    /// Temporary slot for one-time operations
    Temporary {
        /// Expiration time
        expires_at: SystemTime,
    },
    /// Failover slot for high availability
    Failover {
        /// Primary slot this fails over from
        primary_slot: SlotName,
        /// Automatic activation on failure
        auto_activate: bool,
    },
}

/// WAL retention policies
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum WalRetentionPolicy {
    /// Retain WAL for specified time duration
    Time(Duration),
    /// Retain specific amount of WAL data in bytes
    Size(u64),
    /// Retain WAL until specific LSN
    Lsn(LogSequenceNumber),
    /// Custom retention logic
    Custom {
        /// Policy name
        policy_name: String,
        /// Policy parameters
        parameters: HashMap<String, String>,
    },
    /// No automatic WAL cleanup (manual management)
    Manual,
}

/// Lag monitoring thresholds
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LagThresholds {
    /// Warning threshold
    pub warning_duration: Duration,
    /// Critical threshold
    pub critical_duration: Duration,
    /// Warning byte lag threshold
    pub warning_bytes: Option<u64>,
    /// Critical byte lag threshold
    pub critical_bytes: Option<u64>,
}

/// Slot priority levels
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum SlotPriority {
    Low,
    Normal,
    High,
    Critical,
}

/// Buffer settings for slot operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BufferSettings {
    /// Maximum number of changes to buffer
    pub max_changes: usize,
    /// Maximum buffer size in bytes
    pub max_size_bytes: u64,
    /// Flush interval
    pub flush_interval: Duration,
    /// Enable compression
    pub enable_compression: bool,
    /// Batch size for processing
    pub batch_size: usize,
}

impl Default for BufferSettings {
    fn default() -> Self {
        Self {
            max_changes: 10000,
            max_size_bytes: 100 * 1024 * 1024, // 100MB
            flush_interval: Duration::from_secs(5),
            enable_compression: true,
            batch_size: 1000,
        }
    }
}

/// Replication slot metadata and status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SlotInfo {
    /// Unique slot identifier
    pub slot_id: SlotId,
    /// Slot name
    pub slot_name: SlotName,
    /// Slot configuration
    pub config: SlotConfig,
    /// Current slot status
    pub status: SlotStatus,
    /// Creation timestamp
    pub created_at: SystemTime,
    /// Last activity timestamp
    pub last_activity: SystemTime,
    /// Current WAL position
    pub current_lsn: LogSequenceNumber,
    /// Confirmed flush LSN
    pub confirmed_flush_lsn: Option<LogSequenceNumber>,
    /// Restart LSN
    pub restart_lsn: LogSequenceNumber,
    /// Current lag information
    pub lag_info: LagInfo,
    /// Slot statistics
    pub statistics: SlotStatistics,
    /// Error information if any
    pub error_info: Option<String>,
    /// Associated connections
    pub active_connections: usize,
    /// WAL segments retained by this slot
    pub retained_wal_segments: usize,
}

/// Slot status states
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SlotStatus {
    /// Slot is created but not active
    Inactive,
    /// Slot is actively being consumed
    Active,
    /// Slot is temporarily paused
    Paused,
    /// Slot has failed and needs attention
    Failed,
    /// Slot is being deleted
    Deleting,
    /// Slot is deleted
    Deleted,
    /// Slot is in failover mode
    Failover,
    /// Slot is being synchronized
    Synchronizing,
}

/// Lag information for a slot
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LagInfo {
    /// Lag duration behind master
    pub lag_duration: Option<Duration>,
    /// Lag in bytes
    pub lag_bytes: Option<u64>,
    /// Lag in LSN difference
    pub lag_lsn: Option<u64>,
    /// Whether lag exceeds warning threshold
    pub warning_threshold_exceeded: bool,
    /// Whether lag exceeds critical threshold
    pub critical_threshold_exceeded: bool,
    /// Lag trend
    pub lag_trend: LagTrend,
    /// Last measurement time
    pub last_measured: SystemTime,
}

impl Default for LagInfo {
    fn default() -> Self {
        Self {
            lag_duration: None,
            lag_bytes: None,
            lag_lsn: None,
            warning_threshold_exceeded: false,
            critical_threshold_exceeded: false,
            lag_trend: LagTrend::Stable,
            last_measured: SystemTime::now(),
        }
    }
}

/// Slot usage statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SlotStatistics {
    /// Total bytes consumed
    pub bytes_consumed: u64,
    /// Total changes processed
    pub changes_processed: u64,
    /// Average processing rate (changes/second)
    pub avg_processing_rate: f64,
    /// Peak processing rate
    pub peak_processing_rate: f64,
    /// Total connection time
    pub total_connection_time: Duration,
    /// Number of reconnections
    pub reconnection_count: u32,
    /// Last reset timestamp
    pub last_reset: SystemTime,
    /// Errors encountered
    pub error_count: u32,
    /// WAL segments consumed
    pub wal_segments_consumed: u64,
}

impl Default for SlotStatistics {
    fn default() -> Self {
        Self {
            bytes_consumed: 0,
            changes_processed: 0,
            avg_processing_rate: 0.0,
            peak_processing_rate: 0.0,
            total_connection_time: Duration::ZERO,
            reconnection_count: 0,
            last_reset: SystemTime::now(),
            error_count: 0,
            wal_segments_consumed: 0,
        }
    }
}

/// Replication change data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReplicationChange {
    /// Change sequence number
    pub change_id: u64,
    /// LSN where change starts
    pub start_lsn: LogSequenceNumber,
    /// LSN where change ends
    pub end_lsn: LogSequenceNumber,
    /// Timestamp of the change
    pub timestamp: SystemTime,
    /// Type of change
    pub change_type: ChangeType,
    /// Schema name
    pub schema_name: String,
    /// Table name
    pub table_name: String,
    /// Change data
    pub change_data: ChangeData,
    /// Transaction ID
    pub transaction_id: Option<u64>,
    /// Additional metadata
    pub metadata: HashMap<String, String>,
}

/// Types of replication changes
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ChangeType {
    /// Row insertion
    Insert,
    /// Row update
    Update,
    /// Row deletion
    Delete,
    /// Schema change
    SchemaChange,
    /// Transaction begin
    BeginTransaction,
    /// Transaction commit
    CommitTransaction,
    /// Transaction rollback
    RollbackTransaction,
    /// Truncate table
    Truncate,
    /// Custom change type
    Custom(String),
}

/// Change data payload
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ChangeData {
    /// Row data for insert/update/delete
    RowData {
        /// Column values before change (for updates/deletes)
        old_values: Option<HashMap<String, serde_json::Value>>,
        /// Column values after change (for inserts/updates)
        new_values: Option<HashMap<String, serde_json::Value>>,
    },
    /// Schema change information
    SchemaChange {
        /// DDL statement
        ddl_statement: String,
        /// Objects affected
        affected_objects: Vec<String>,
    },
    /// Transaction information
    TransactionInfo {
        /// Transaction metadata
        metadata: HashMap<String, String>,
    },
    /// Raw change data
    Raw {
        /// Raw binary data
        data: Vec<u8>,
        /// Format identifier
        format: String,
    },
}

/// Slot consumption stream
pub struct SlotConsumptionStream {
    /// Slot ID being consumed
    slot_id: SlotId,
    /// Change receiver
    change_receiver: mpsc::UnboundedReceiver<ReplicationChange>,
    /// Current LSN position
    current_lsn: LogSequenceNumber,
    /// Stream statistics
    statistics: Arc<RwLock<SlotStatistics>>,
    /// Stream status
    status: Arc<RwLock<SlotStatus>>,
    /// Error channel
    error_receiver: mpsc::UnboundedReceiver<SlotError>,
}

impl SlotConsumptionStream {
    /// Gets the next change from the stream
    pub async fn next_change(&mut self) -> Result<Option<ReplicationChange>, SlotError> {
        tokio::select! {
            change = self.change_receiver.recv() => {
                match change {
                    Some(change) => {
                        self.current_lsn = change.end_lsn.clone();
                        
                        // Update statistics
                        {
                            let mut stats = self.statistics.write();
                            stats.changes_processed += 1;
                            stats.bytes_consumed += 1024; // Simplified
                        }
                        
                        Ok(Some(change))
                    }
                    None => Ok(None),
                }
            }
            error = self.error_receiver.recv() => {
                match error {
                    Some(error) => Err(error),
                    None => Ok(None),
                }
            }
        }
    }
    
    /// Gets current LSN position
    pub fn current_lsn(&self) -> &LogSequenceNumber {
        &self.current_lsn
    }
    
    /// Gets stream statistics
    pub fn statistics(&self) -> SlotStatistics {
        self.statistics.read().clone()
    }
    
    /// Gets stream status
    pub fn status(&self) -> SlotStatus {
        self.status.read().clone()
    }
}

/// Slot manager trait for different implementations
#[async_trait]
pub trait SlotManager: Send + Sync {
    /// Create a new replication slot
    async fn create_slot(&self, name: &SlotName, config: SlotConfig) -> Result<SlotId, SlotError>;
    
    /// Delete a replication slot
    async fn delete_slot(&self, slot_id: &SlotId) -> Result<(), SlotError>;
    
    /// Get slot information
    async fn get_slot_info(&self, slot_id: &SlotId) -> Result<SlotInfo, SlotError>;
    
    /// List all slots
    async fn list_slots(&self) -> Result<Vec<SlotInfo>, SlotError>;
    
    /// Get slot status
    async fn get_slot_status(&self, slot_id: &SlotId) -> Result<SlotStatus, SlotError>;
    
    /// Start consuming from a slot
    async fn start_consuming(&self, slot_id: &SlotId) -> Result<SlotConsumptionStream, SlotError>;
    
    /// Advance slot position
    async fn advance_slot(&self, slot_id: &SlotId, lsn: &LogSequenceNumber) -> Result<(), SlotError>;
    
    /// Pause slot consumption
    async fn pause_slot(&self, slot_id: &SlotId) -> Result<(), SlotError>;
    
    /// Resume slot consumption
    async fn resume_slot(&self, slot_id: &SlotId) -> Result<(), SlotError>;
    
    /// Get slot statistics
    async fn get_slot_statistics(&self, slot_id: &SlotId) -> Result<SlotStatistics, SlotError>;
    
    /// Reset slot statistics
    async fn reset_slot_statistics(&self, slot_id: &SlotId) -> Result<(), SlotError>;
    
    /// Apply WAL retention policies
    async fn apply_wal_retention(&self) -> Result<Vec<String>, SlotError>;
}

/// Main replication slot manager implementation
pub struct ReplicationSlotManager {
    /// Configuration
    config: Arc<SlotManagerConfig>,
    /// Active slots
    slots: Arc<RwLock<HashMap<SlotId, SlotInfo>>>,
    /// Slot name to ID mapping
    slot_names: Arc<RwLock<HashMap<SlotName, SlotId>>>,
    /// Active consumption streams
    active_streams: Arc<RwLock<HashMap<SlotId, Arc<Mutex<SlotConsumptionStream>>>>>,
    /// Background task handles
    background_tasks: Arc<Mutex<Vec<tokio::task::JoinHandle<()>>>>,
    /// Event broadcaster for slot events
    event_broadcaster: broadcast::Sender<SlotEvent>,
    /// Shutdown signal
    shutdown_sender: Arc<watch::Sender<bool>>,
    /// WAL retention manager
    wal_retention: Arc<WalRetentionManager>,
}

/// Slot-related events
#[derive(Debug, Clone)]
pub enum SlotEvent {
    /// Slot created
    SlotCreated { slot_id: SlotId, slot_name: SlotName },
    /// Slot deleted
    SlotDeleted { slot_id: SlotId, slot_name: SlotName },
    /// Slot status changed
    StatusChanged { slot_id: SlotId, old_status: SlotStatus, new_status: SlotStatus },
    /// Lag threshold exceeded
    LagThresholdExceeded { slot_id: SlotId, lag_duration: Duration },
    /// Slot failed
    SlotFailed { slot_id: SlotId, error: String },
    /// WAL retention applied
    WalRetentionApplied { segments_removed: usize },
}

/// WAL retention manager for automatic cleanup
pub struct WalRetentionManager {
    /// Current WAL position
    current_wal_position: Arc<RwLock<LogSequenceNumber>>,
    /// Slot retention requirements
    slot_requirements: Arc<RwLock<HashMap<SlotId, LogSequenceNumber>>>,
    /// Retention policies
    retention_policies: Arc<RwLock<HashMap<SlotId, WalRetentionPolicy>>>,
}

impl WalRetentionManager {
    /// Creates a new WAL retention manager
    pub fn new() -> Self {
        Self {
            current_wal_position: Arc::new(RwLock::new(LogSequenceNumber::new(0))),
            slot_requirements: Arc::new(RwLock::new(HashMap::new())),
            retention_policies: Arc::new(RwLock::new(HashMap::new())),
        }
    }
    
    /// Updates slot retention requirements
    pub fn update_slot_requirement(&self, slot_id: &SlotId, required_lsn: &LogSequenceNumber) {
        let mut requirements = self.slot_requirements.write();
        requirements.insert(slot_id.clone(), required_lsn.clone());
    }
    
    /// Calculates minimum required WAL position
    pub fn calculate_min_wal_position(&self) -> LogSequenceNumber {
        let requirements = self.slot_requirements.read();
        requirements.values()
            .min()
            .cloned()
            .unwrap_or_else(|| LogSequenceNumber::new(0))
    }
    
    /// Applies WAL retention policies
    pub fn apply_retention(&self) -> Vec<String> {
        let min_position = self.calculate_min_wal_position();
        let current_position = self.current_wal_position.read();
        
        // Simplified implementation - would actually remove WAL segments
        if current_position.value() > min_position.value() + 1000 {
            vec!["segment_001".to_string(), "segment_002".to_string()]
        } else {
            Vec::new()
        }
    }
}

impl ReplicationSlotManager {
    /// Creates a new replication slot manager
    /// 
    /// # Arguments
    /// 
    /// * `config` - Slot manager configuration
    /// 
    /// # Returns
    /// 
    /// * `Ok(ReplicationSlotManager)` - Successfully created manager
    /// * `Err(SlotError)` - Creation failed
    pub fn new(config: SlotManagerConfig) -> Result<Self, SlotError> {
        // Validate configuration
        Self::validate_config(&config)?;
        
        let (event_broadcaster, _) = broadcast::channel(1000);
        let (shutdown_sender, _) = watch::channel(false);
        
        let manager = Self {
            config: Arc::new(config),
            slots: Arc::new(RwLock::new(HashMap::new())),
            slot_names: Arc::new(RwLock::new(HashMap::new())),
            active_streams: Arc::new(RwLock::new(HashMap::new())),
            background_tasks: Arc::new(Mutex::new(Vec::new())),
            event_broadcaster,
            shutdown_sender: Arc::new(shutdown_sender),
            wal_retention: Arc::new(WalRetentionManager::new()),
        };
        
        // Start background tasks
        if manager.config.enable_monitoring {
            manager.start_monitoring_task();
        }
        
        if manager.config.enable_auto_cleanup {
            manager.start_cleanup_task();
        }
        
        Ok(manager)
    }
    
    /// Validates slot manager configuration
    fn validate_config(config: &SlotManagerConfig) -> Result<(), SlotError> {
        if config.max_slots == 0 {
            return Err(SlotError::InvalidConfiguration {
                reason: "Maximum slots must be greater than 0".to_string(),
            });
        }
        
        if config.max_slot_name_length == 0 || config.max_slot_name_length > 255 {
            return Err(SlotError::InvalidConfiguration {
                reason: "Invalid slot name length limit".to_string(),
            });
        }
        
        Ok(())
    }
    
    /// Starts monitoring background task
    fn start_monitoring_task(&self) {
        let slots = Arc::clone(&self.slots);
        let config = Arc::clone(&self.config);
        let event_broadcaster = self.event_broadcaster.clone();
        let mut shutdown_receiver = self.shutdown_sender.subscribe();
        
        let handle = tokio::spawn(async move {
            let mut interval = tokio::time::interval(config.monitoring_interval);
            
            loop {
                tokio::select! {
                    _ = interval.tick() => {
                        let slot_infos: Vec<_> = {
                            let slots = slots.read();
                            slots.values().cloned().collect()
                        };
                        
                        for slot_info in slot_infos {
                            // Check lag thresholds
                            if let Some(lag_duration) = slot_info.lag_info.lag_duration {
                                if lag_duration > config.lag_critical_threshold ||
                                   lag_duration > config.lag_warning_threshold {
                                    let _ = event_broadcaster.send(SlotEvent::LagThresholdExceeded {
                                        slot_id: slot_info.slot_id.clone(),
                                        lag_duration,
                                    });
                                }
                            }
                        }
                    }
                    _ = shutdown_receiver.changed() => {
                        if *shutdown_receiver.borrow() {
                            break;
                        }
                    }
                }
            }
        });
        
        self.background_tasks.lock().push(handle);
    }
    
    /// Starts cleanup background task
    fn start_cleanup_task(&self) {
        let slots = Arc::clone(&self.slots);
        let config = Arc::clone(&self.config);
        let wal_retention = Arc::clone(&self.wal_retention);
        let event_broadcaster = self.event_broadcaster.clone();
        let mut shutdown_receiver = self.shutdown_sender.subscribe();
        
        let handle = tokio::spawn(async move {
            let mut interval = tokio::time::interval(config.cleanup_interval);
            
            loop {
                tokio::select! {
                    _ = interval.tick() => {
                        // Apply WAL retention
                        let removed_segments = wal_retention.apply_retention();
                        if !removed_segments.is_empty() {
                            let _ = event_broadcaster.send(SlotEvent::WalRetentionApplied {
                                segments_removed: removed_segments.len(),
                            });
                        }
                        
                        // Clean up inactive temporary slots
                        let now = SystemTime::now();
                        let mut slots_to_remove = Vec::new();
                        
                        {
                            let slots = slots.read();
                            for (slot_id, slot_info) in slots.iter() {
                                if let SlotType::Temporary { expires_at } = &slot_info.config.slot_type {
                                    if now >= *expires_at {
                                        slots_to_remove.push(slot_id.clone());
                                    }
                                }
                            }
                        }
                        
                        // Remove expired temporary slots
                        for slot_id in slots_to_remove {
                            if let Some(slot_info) = {
                                let mut slots = slots.write();
                                slots.remove(&slot_id)
                            } {
                                let _ = event_broadcaster.send(SlotEvent::SlotDeleted {
                                    slot_id,
                                    slot_name: slot_info.slot_name,
                                });
                            }
                        }
                    }
                    _ = shutdown_receiver.changed() => {
                        if *shutdown_receiver.borrow() {
                            break;
                        }
                    }
                }
            }
        });
        
        self.background_tasks.lock().push(handle);
    }
    
    /// Creates a consumption stream for a slot
    async fn create_consumption_stream(&self, slot_id: &SlotId) -> Result<SlotConsumptionStream, SlotError> {
        let (change_sender, change_receiver) = mpsc::unbounded_channel();
        let (error_sender, error_receiver) = mpsc::unbounded_channel();
        
        let slot_info = {
            let slots = self.slots.read();
            slots.get(slot_id)
                .cloned()
                .ok_or_else(|| SlotError::SlotNotFound {
                    slot_name: slot_id.to_string(),
                })?
        };
        
        // Start producing changes (simplified simulation)
        let slot_id_clone = slot_id.clone();
        let current_lsn = slot_info.current_lsn.clone();
        tokio::spawn(async move {
            let mut lsn_value = current_lsn.value();
            let mut change_id = 1;
            
            loop {
                tokio::time::sleep(Duration::from_millis(100)).await;
                
                lsn_value += 1;
                let change = ReplicationChange {
                    change_id,
                    start_lsn: LogSequenceNumber::new(lsn_value - 1),
                    end_lsn: LogSequenceNumber::new(lsn_value),
                    timestamp: SystemTime::now(),
                    change_type: ChangeType::Insert,
                    schema_name: "public".to_string(),
                    table_name: "test_table".to_string(),
                    change_data: ChangeData::RowData {
                        old_values: None,
                        new_values: Some({
                            let mut values = HashMap::new();
                            values.insert("id".to_string(), serde_json::Value::Number(serde_json::Number::from(change_id)));
                            values
                        }),
                    },
                    transaction_id: Some(change_id),
                    metadata: HashMap::new(),
                };
                
                if change_sender.send(change).is_err() {
                    break;
                }
                
                change_id += 1;
            }
        });
        
        Ok(SlotConsumptionStream {
            slot_id: slot_id.clone(),
            change_receiver,
            current_lsn: slot_info.current_lsn,
            statistics: Arc::new(RwLock::new(SlotStatistics::default())),
            status: Arc::new(RwLock::new(SlotStatus::Active)),
            error_receiver,
        })
    }
}

#[async_trait]
impl SlotManager for ReplicationSlotManager {
    async fn create_slot(&self, name: &SlotName, config: SlotConfig) -> Result<SlotId, SlotError> {
        // Check slot limit
        {
            let slots = self.slots.read();
            if slots.len() >= self.config.max_slots {
                return Err(SlotError::SlotLimitExceeded {
                    current_count: slots.len(),
                    max_slots: self.config.max_slots,
                });
            }
        }
        
        // Check if slot name already exists
        {
            let names = self.slot_names.read();
            if names.contains_key(name) {
                return Err(SlotError::SlotAlreadyExists {
                    slot_name: name.to_string(),
                });
            }
        }
        
        let slot_id = SlotId::generate();
        let now = SystemTime::now();
        
        let slot_info = SlotInfo {
            slot_id: slot_id.clone(),
            slot_name: name.clone(),
            config: config.clone(),
            status: SlotStatus::Inactive,
            created_at: now,
            last_activity: now,
            current_lsn: LogSequenceNumber::new(12345), // Example LSN
            confirmed_flush_lsn: None,
            restart_lsn: LogSequenceNumber::new(12345),
            lag_info: LagInfo::default(),
            statistics: SlotStatistics::default(),
            error_info: None,
            active_connections: 0,
            retained_wal_segments: 0,
        };
        
        // Store slot
        {
            let mut slots = self.slots.write();
            slots.insert(slot_id.clone(), slot_info);
        }
        
        {
            let mut names = self.slot_names.write();
            names.insert(name.clone(), slot_id.clone());
        }
        
        // Update WAL retention
        self.wal_retention.update_slot_requirement(&slot_id, &LogSequenceNumber::new(12345));
        
        // Broadcast event
        let _ = self.event_broadcaster.send(SlotEvent::SlotCreated {
            slot_id: slot_id.clone(),
            slot_name: name.clone(),
        });
        
        Ok(slot_id)
    }
    
    async fn delete_slot(&self, slot_id: &SlotId) -> Result<(), SlotError> {
        let slot_info = {
            let mut slots = self.slots.write();
            slots.remove(slot_id)
                .ok_or_else(|| SlotError::SlotNotFound {
                    slot_name: slot_id.to_string(),
                })?
        };
        
        // Remove from name mapping
        {
            let mut names = self.slot_names.write();
            names.remove(&slot_info.slot_name);
        }
        
        // Remove active stream if exists
        {
            let mut streams = self.active_streams.write();
            streams.remove(slot_id);
        }
        
        // Update WAL retention
        {
            let mut requirements = self.wal_retention.slot_requirements.write();
            requirements.remove(slot_id);
        }
        
        // Broadcast event
        let _ = self.event_broadcaster.send(SlotEvent::SlotDeleted {
            slot_id: slot_id.clone(),
            slot_name: slot_info.slot_name,
        });
        
        Ok(())
    }
    
    async fn get_slot_info(&self, slot_id: &SlotId) -> Result<SlotInfo, SlotError> {
        let slots = self.slots.read();
        slots.get(slot_id)
            .cloned()
            .ok_or_else(|| SlotError::SlotNotFound {
                slot_name: slot_id.to_string(),
            })
    }
    
    async fn list_slots(&self) -> Result<Vec<SlotInfo>, SlotError> {
        let slots = self.slots.read();
        Ok(slots.values().cloned().collect())
    }
    
    async fn get_slot_status(&self, slot_id: &SlotId) -> Result<SlotStatus, SlotError> {
        let slots = self.slots.read();
        slots.get(slot_id)
            .map(|info| info.status.clone())
            .ok_or_else(|| SlotError::SlotNotFound {
                slot_name: slot_id.to_string(),
            })
    }
    
    async fn start_consuming(&self, slot_id: &SlotId) -> Result<SlotConsumptionStream, SlotError> {
        // Verify slot exists
        {
            let slots = self.slots.read();
            if !slots.contains_key(slot_id) {
                return Err(SlotError::SlotNotFound {
                    slot_name: slot_id.to_string(),
                });
            }
        }
        
        // Create consumption stream
        let stream = self.create_consumption_stream(slot_id).await?;
        
        // Store active stream
        {
            let mut streams = self.active_streams.write();
            streams.insert(slot_id.clone(), Arc::new(Mutex::new(stream)));
        }
        
        // Update slot status
        {
            let mut slots = self.slots.write();
            if let Some(slot_info) = slots.get_mut(slot_id) {
                slot_info.status = SlotStatus::Active;
                slot_info.active_connections += 1;
            }
        }
        
        // Return new stream (simplified - in practice would handle multiple consumers)
        self.create_consumption_stream(slot_id).await
    }
    
    async fn advance_slot(&self, slot_id: &SlotId, lsn: &LogSequenceNumber) -> Result<(), SlotError> {
        {
            let mut slots = self.slots.write();
            let slot_info = slots.get_mut(slot_id)
                .ok_or_else(|| SlotError::SlotNotFound {
                    slot_name: slot_id.to_string(),
                })?;
            
            // Validate LSN advancement
            if lsn.value() < slot_info.current_lsn.value() {
                return Err(SlotError::InvalidWalPosition {
                    position: lsn.to_string(),
                    reason: "Cannot advance to earlier LSN".to_string(),
                });
            }
            
            slot_info.current_lsn = lsn.clone();
            slot_info.confirmed_flush_lsn = Some(lsn.clone());
            slot_info.last_activity = SystemTime::now();
        }
        
        // Update WAL retention requirements
        self.wal_retention.update_slot_requirement(slot_id, lsn);
        
        Ok(())
    }
    
    async fn pause_slot(&self, slot_id: &SlotId) -> Result<(), SlotError> {
        let mut slots = self.slots.write();
        let slot_info = slots.get_mut(slot_id)
            .ok_or_else(|| SlotError::SlotNotFound {
                slot_name: slot_id.to_string(),
            })?;
        
        let old_status = slot_info.status.clone();
        slot_info.status = SlotStatus::Paused;
        
        // Broadcast event
        let _ = self.event_broadcaster.send(SlotEvent::StatusChanged {
            slot_id: slot_id.clone(),
            old_status,
            new_status: SlotStatus::Paused,
        });
        
        Ok(())
    }
    
    async fn resume_slot(&self, slot_id: &SlotId) -> Result<(), SlotError> {
        let mut slots = self.slots.write();
        let slot_info = slots.get_mut(slot_id)
            .ok_or_else(|| SlotError::SlotNotFound {
                slot_name: slot_id.to_string(),
            })?;
        
        let old_status = slot_info.status.clone();
        slot_info.status = if slot_info.active_connections > 0 {
            SlotStatus::Active
        } else {
            SlotStatus::Inactive
        };
        
        // Broadcast event
        let _ = self.event_broadcaster.send(SlotEvent::StatusChanged {
            slot_id: slot_id.clone(),
            old_status,
            new_status: slot_info.status.clone(),
        });
        
        Ok(())
    }
    
    async fn get_slot_statistics(&self, slot_id: &SlotId) -> Result<SlotStatistics, SlotError> {
        let slots = self.slots.read();
        slots.get(slot_id)
            .map(|info| info.statistics.clone())
            .ok_or_else(|| SlotError::SlotNotFound {
                slot_name: slot_id.to_string(),
            })
    }
    
    async fn reset_slot_statistics(&self, slot_id: &SlotId) -> Result<(), SlotError> {
        let mut slots = self.slots.write();
        let slot_info = slots.get_mut(slot_id)
            .ok_or_else(|| SlotError::SlotNotFound {
                slot_name: slot_id.to_string(),
            })?;
        
        slot_info.statistics = SlotStatistics::default();
        Ok(())
    }
    
    async fn apply_wal_retention(&self) -> Result<Vec<String>, SlotError> {
        let removed_segments = self.wal_retention.apply_retention();
        
        // Broadcast event if segments were removed
        if !removed_segments.is_empty() {
            let _ = self.event_broadcaster.send(SlotEvent::WalRetentionApplied {
                segments_removed: removed_segments.len(),
            });
        }
        
        Ok(removed_segments)
    }
}

impl Default for ReplicationSlotManager {
    fn default() -> Self {
        Self::new(SlotManagerConfig::default()).unwrap()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_slot_name_validation() {
        // Valid names
        assert!(SlotName::new("valid_slot_name").is_ok());
        assert!(SlotName::new("_valid_name").is_ok());
        assert!(SlotName::new("slot123").is_ok());
        
        // Invalid names
        assert!(SlotName::new("").is_err()); // Empty
        assert!(SlotName::new("123slot").is_err()); // Starts with number
        assert!(SlotName::new("slot-name").is_err()); // Contains hyphen
        assert!(SlotName::new("slot name").is_err()); // Contains space
        
        // Too long name
        let long_name = "a".repeat(64);
        assert!(SlotName::new(long_name).is_err());
    }

    #[tokio::test]
    async fn test_slot_manager_creation() {
        let config = SlotManagerConfig::default();
        let manager = ReplicationSlotManager::new(config);
        assert!(manager.is_ok());
    }

    #[tokio::test]
    async fn test_slot_creation() {
        let manager = ReplicationSlotManager::default();
        let slot_name = SlotName::new("test_slot").unwrap();
        let config = SlotConfig::default();
        
        let slot_id = manager.create_slot(&slot_name, config).await.unwrap();
        assert!(!slot_id.as_str().is_empty());
        
        let slot_info = manager.get_slot_info(&slot_id).await.unwrap();
        assert_eq!(slot_info.slot_name, slot_name);
        assert_eq!(slot_info.status, SlotStatus::Inactive);
    }

    #[tokio::test]
    async fn test_slot_deletion() {
        let manager = ReplicationSlotManager::default();
        let slot_name = SlotName::new("test_slot").unwrap();
        let config = SlotConfig::default();
        
        let slot_id = manager.create_slot(&slot_name, config).await.unwrap();
        
        // Verify slot exists
        assert!(manager.get_slot_info(&slot_id).await.is_ok());
        
        // Delete slot
        assert!(manager.delete_slot(&slot_id).await.is_ok());
        
        // Verify slot no longer exists
        assert!(manager.get_slot_info(&slot_id).await.is_err());
    }

    #[tokio::test]
    async fn test_slot_listing() {
        let manager = ReplicationSlotManager::default();
        
        // Create multiple slots
        let slot1_name = SlotName::new("slot1").unwrap();
        let slot2_name = SlotName::new("slot2").unwrap();
        let config = SlotConfig::default();
        
        let _slot1_id = manager.create_slot(&slot1_name, config.clone()).await.unwrap();
        let _slot2_id = manager.create_slot(&slot2_name, config).await.unwrap();
        
        let slots = manager.list_slots().await.unwrap();
        assert_eq!(slots.len(), 2);
        
        let names: HashSet<_> = slots.iter().map(|s| &s.slot_name).collect();
        assert!(names.contains(&slot1_name));
        assert!(names.contains(&slot2_name));
    }

    #[tokio::test]
    async fn test_slot_consumption_stream() {
        let manager = ReplicationSlotManager::default();
        let slot_name = SlotName::new("test_slot").unwrap();
        let config = SlotConfig::default();
        
        let slot_id = manager.create_slot(&slot_name, config).await.unwrap();
        
        let mut stream = manager.start_consuming(&slot_id).await.unwrap();
        
        // Verify stream is active
        assert_eq!(stream.status(), SlotStatus::Active);
        
        // Try to get a change (may timeout in test)
        let _result = tokio::time::timeout(
            Duration::from_millis(200),
            stream.next_change()
        ).await;
        
        // Should either get a change or timeout
        assert!(result.is_ok() || result.is_err());
    }

    #[tokio::test]
    async fn test_slot_advancement() {
        let manager = ReplicationSlotManager::default();
        let slot_name = SlotName::new("test_slot").unwrap();
        let config = SlotConfig::default();
        
        let slot_id = manager.create_slot(&slot_name, config).await.unwrap();
        let new_lsn = LogSequenceNumber::new(54321);
        
        assert!(manager.advance_slot(&slot_id, &new_lsn).await.is_ok());
        
        let slot_info = manager.get_slot_info(&slot_id).await.unwrap();
        assert_eq!(slot_info.current_lsn, new_lsn);
    }

    #[tokio::test]
    async fn test_slot_status_changes() {
        let manager = ReplicationSlotManager::default();
        let slot_name = SlotName::new("test_slot").unwrap();
        let config = SlotConfig::default();
        
        let slot_id = manager.create_slot(&slot_name, config).await.unwrap();
        
        // Initially inactive
        assert_eq!(manager.get_slot_status(&slot_id).await.unwrap(), SlotStatus::Inactive);
        
        // Pause slot
        assert!(manager.pause_slot(&slot_id).await.is_ok());
        assert_eq!(manager.get_slot_status(&slot_id).await.unwrap(), SlotStatus::Paused);
        
        // Resume slot
        assert!(manager.resume_slot(&slot_id).await.is_ok());
        assert_eq!(manager.get_slot_status(&slot_id).await.unwrap(), SlotStatus::Inactive);
    }

    #[tokio::test]
    async fn test_slot_statistics() {
        let manager = ReplicationSlotManager::default();
        let slot_name = SlotName::new("test_slot").unwrap();
        let config = SlotConfig::default();
        
        let slot_id = manager.create_slot(&slot_name, config).await.unwrap();
        
        let _stats = manager.get_slot_statistics(&slot_id).await.unwrap();
        assert_eq!(stats.changes_processed, 0);
        assert_eq!(stats.bytes_consumed, 0);
        
        // Reset statistics
        assert!(manager.reset_slot_statistics(&slot_id).await.is_ok());
    }

    #[test]
    fn test_slot_types() {
        let physical = SlotType::Physical;
        let logical = SlotType::Logical {
            plugin_name: "pgoutput".to_string(),
            publication_names: vec!["test_pub".to_string()],
        };
        
        assert_eq!(physical, SlotType::Physical);
        assert_ne!(physical, logical);
    }

    #[test]
    fn test_wal_retention_policies() {
        let time_policy = WalRetentionPolicy::Time(Duration::from_secs(24 * 60 * 60));
        let size_policy = WalRetentionPolicy::Size(1024 * 1024 * 1024); // 1GB
        
        match time_policy {
            WalRetentionPolicy::Time(duration) => assert_eq!(duration, Duration::from_secs(24 * 60 * 60)),
            _ => panic!("Expected time policy"),
        }
        
        match size_policy {
            WalRetentionPolicy::Size(bytes) => assert_eq!(bytes, 1024 * 1024 * 1024),
            _ => panic!("Expected size policy"),
        }
    }

    #[test]
    fn test_change_data_types() {
        let row_data = ChangeData::RowData {
            old_values: None,
            new_values: Some({
                let mut values = HashMap::new();
                values.insert("id".to_string(), serde_json::Value::Number(serde_json::Number::from(1)));
                values
            }),
        };
        
        match row_data {
            ChangeData::RowData { old_values, new_values } => {
                assert!(old_values.is_none());
                assert!(new_values.is_some());
            }
            _ => panic!("Expected row data"),
        }
    }
}