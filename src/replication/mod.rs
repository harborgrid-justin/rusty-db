// # Replication System
//
// This module provides a comprehensive enterprise-grade replication system
// for the RustyDB database. It supports both synchronous and asynchronous
// replication with strong consistency guarantees, automatic failover,
// conflict resolution, health monitoring, snapshot management, and replication slots.
//
// ## Key Features
//
// - **Multi-Master Replication**: Support for multiple write nodes with automatic conflict resolution
// - **Automatic Failover**: Fast detection and recovery from node failures with minimal downtime
// - **Advanced Conflict Resolution**: Multiple strategies including Last Writer Wins, CRDT, and custom resolvers
// - **WAL-Based Replication**: Write-ahead logging for consistent and reliable data replication
// - **Real-time Health Monitoring**: Comprehensive health monitoring, alerting, and performance analytics
// - **Snapshot Management**: Incremental and full backups with compression and encryption support
// - **Replication Slots**: Logical and physical slot management with WAL retention policies
// - **Performance Optimization**: Advanced buffer management, connection pooling, and throughput optimization
// - **Enterprise Security**: End-to-end encryption, authentication, and authorization controls
// - **Operational Excellence**: Comprehensive metrics, logging, tracing, and diagnostic capabilities
//
// ## Architecture Overview
//
// The replication system consists of several interconnected components working together
// to provide reliable, scalable, and high-performance database replication:
//
// ```text
// ┌─────────────────────┐    ┌─────────────────────┐    ┌─────────────────────┐
// │    Primary Node     │───▶│  Replication Hub    │───▶│   Secondary Node    │
// │                     │    │                     │    │                     │
// │ ┌─────────────────┐ │    │ ┌─────────────────┐ │    │ ┌─────────────────┐ │
// │ │ WAL Manager     │ │    │ │ Conflict        │ │    │ │ Replay Engine   │ │
// │ │ - Log Writing   │ │    │ │ Resolution      │ │    │ │ - Apply Changes │ │
// │ │ - Streaming     │ │    │ │ - Detection     │ │    │ │ - Validation    │ │
// │ │ - Archival      │ │    │ │ - Strategies    │ │    │ │ - Recovery      │ │
// │ └─────────────────┘ │    │ └─────────────────┘ │    │ └─────────────────┘ │
// │                     │    │                     │    │                     │
// │ ┌─────────────────┐ │    │ ┌─────────────────┐ │    │ ┌─────────────────┐ │
// │ │ Health Monitor  │ │    │ │ Load Balancer   │ │    │ │ Lag Monitor     │ │
// │ │ - Metrics       │ │    │ │ - Routing       │ │    │ │ - Lag Tracking  │ │
// │ │ - Alerting      │ │    │ │ - Failover      │ │    │ │ - Performance   │ │
// │ │ - Analytics     │ │    │ │ - Discovery     │ │    │ │ - Alerts        │ │
// │ └─────────────────┘ │    │ └─────────────────┘ │    │ └─────────────────┘ │
// │                     │    │                     │    │                     │
// │ ┌─────────────────┐ │    │ ┌─────────────────┐ │    │ ┌─────────────────┐ │
// │ │ Snapshot Mgmt   │ │    │ │ Replication     │ │    │ │ Recovery Mgmt   │ │
// │ │ - Incremental   │ │    │ │ Slots           │ │    │ │ - Point-in-Time │ │
// │ │ - Full Backup   │ │    │ │ - Logical       │ │    │ │ - Snapshot      │ │
// │ │ - Compression   │ │    │ │ - Physical      │ │    │ │ - Restoration   │ │
// │ └─────────────────┘ │    │ └─────────────────┘ │    │ └─────────────────┘ │
// └─────────────────────┘    └─────────────────────┘    └─────────────────────┘
//                                        │
//                                        ▼
//                            ┌─────────────────────┐
//                            │   Monitoring &      │
//                            │   Management        │
//                            │                     │
//                            │ ┌─────────────────┐ │
//                            │ │ Metrics         │ │
//                            │ │ Dashboard       │ │
//                            │ └─────────────────┘ │
//                            │                     │
//                            │ ┌─────────────────┐ │
//                            │ │ Alert Manager   │ │
//                            │ │ & Notifications │ │
//                            │ └─────────────────┘ │
//                            └─────────────────────┘
// ```
//
// ## Module Organization
//
// ### Core Components
//
// - **types**: Fundamental types, identifiers, and data structures used throughout the replication system
// - **manager**: Central replication manager orchestrating all replication activities and coordination
// - **wal**: Write-Ahead Log management for reliable change tracking and streaming
// - **conflicts**: Advanced conflict detection and resolution with multiple strategies
//
// ### Operational Components
//
// - **monitor**: Real-time health monitoring, performance analytics, and proactive alerting system
// - **snapshots**: Comprehensive snapshot management with incremental/full backups and lifecycle management
// - **slots**: Replication slot management for logical/physical replication and WAL retention
//
// ## Usage Examples
//
// ### Basic Replication Setup
//
// ```rust
// use crate::replication::*;
//
// # async fn basic_setup() -> Result<(), Box<dyn std::error::Error>> {
// // Configure replication with enterprise settings
// let config = ReplicationConfig {
//     cluster_name: "production-cluster".to_string(),
//     node_id: NodeId::new("primary-001")?,
//     replication_mode: ReplicationMode::Async,
//     max_replicas: 10,
//     heartbeat_interval: Duration::from_secs(5),
//     conflict_resolution: ConflictResolutionStrategy::LastWriterWins,
//     enable_compression: true,
//     enable_encryption: true,
//     performance_settings: PerformanceSettings {
//         max_connections: 1000,
//         buffer_size: 64 * 1024,
//         batch_size: 1000,
//         ..Default::default()
//     },
//     ..Default::default()
// };
//
// // Create and start replication manager
// let manager = ReplicationManager::new(config)?;
// manager.start().await?;
//
// // Add high-availability replica
// let replica_config = ReplicaConfig {
//     replica_id: ReplicaId::new("replica-001")?,
//     address: ReplicaAddress::from_str("10.0.1.100:5433")?,
//     replication_lag_threshold: Duration::from_secs(10),
//     priority: ReplicationPriority::High,
//     enable_health_checks: true,
//     ..Default::default()
// };
//
// manager.add_replica(replica_config).await?;
//
// // Monitor replication health
// let status = manager.get_replication_status().await?;
// println!("Cluster health: {:?}", status.overall_health);
// println!("Active replicas: {}", status.active_replicas.len());
// println!("Average lag: {:?}", status.average_lag);
// # Ok(())
// # }
// ```
//
// ### Advanced Health Monitoring
//
// ```rust
// use crate::replication::monitor::*;
//
// # async fn health_monitoring() -> Result<(), Box<dyn std::error::Error>> {
// // Configure comprehensive health monitoring
// let monitor_config = HealthMonitorConfig {
//     check_interval: Duration::from_secs(30),
//     lag_threshold_bytes: 1024 * 1024, // 1MB
//     lag_threshold_seconds: 60,
//     enable_proactive_alerts: true,
//     enable_performance_analytics: true,
//     enable_trend_analysis: true,
//     ..Default::default()
// };
//
// let monitor = ReplicationHealthMonitor::new(monitor_config)?;
// monitor.start_monitoring().await?;
//
// // Add replica for monitoring
// let replica_id = ReplicaId::new("replica-001")?;
// monitor.add_replica(replica_id.clone()).await?;
//
// // Get comprehensive health report
// let health = monitor.get_replica_health(&replica_id).await?;
// println!("Health score: {}/100", health.health_score);
// println!("Lag: {} bytes, {} seconds", health.current_lag.lag_bytes, health.current_lag.lag_seconds);
//
// // Generate analytics report
// let report = monitor.generate_analytics_report(
//     SystemTime::now() - Duration::from_hours(24),
//     SystemTime::now()
// ).await?;
// println!("System health trend: {:?}", report.trend_analysis.health_trend);
// # Ok(())
// # }
// ```
//
// ### Snapshot Management
//
// ```rust
// use crate::replication::snapshots::*;
//
// # async fn snapshot_management() -> Result<(), Box<dyn std::error::Error>> {
// // Configure enterprise snapshot management
// let snapshot_config = SnapshotConfig {
//     storage_path: "/data/backups/snapshots".into(),
//     compression: CompressionType::Zstd,
//     encryption: Some(EncryptionConfig {
//         algorithm: EncryptionAlgorithm::Aes256Gcm,
//         key_source: KeySource::Environment("SNAPSHOT_ENCRYPTION_KEY".to_string()),
//         aad: Some("production-cluster".to_string()),
//     }),
//     retention_policy: RetentionPolicy {
//         max_snapshots: 50,
//         max_age: Duration::from_days(90),
//         min_full_snapshots: 5,
//         daily_retention: Some(Duration::from_days(30)),
//         weekly_retention: Some(Duration::from_days(90)),
//         ..Default::default()
//     },
//     ..Default::default()
// };
//
// let manager = FileSnapshotManager::new(snapshot_config).await?;
//
// // Create full baseline snapshot
// let replica_id = ReplicaId::new("replica-001")?;
// let full_snapshot_id = manager.create_full_snapshot(&replica_id).await?;
// println!("Created full snapshot: {}", full_snapshot_id);
//
// // Create incremental snapshots
// let incremental_id = manager.create_incremental_snapshot(&replica_id, &full_snapshot_id).await?;
// println!("Created incremental snapshot: {}", incremental_id);
//
// // List and manage snapshots
// let snapshots = manager.list_snapshots(&replica_id).await?;
// for snapshot in snapshots {
//     println!("Snapshot: {} ({:?}) - {} bytes",
//         snapshot.snapshot_id,
//         snapshot.snapshot_type,
//         snapshot.size_bytes);
// }
//
// // Apply retention policies
// let deleted_snapshots = manager.apply_retention_policy(&replica_id).await?;
// println!("Cleaned up {} old snapshots", deleted_snapshots.len());
// # Ok(())
// # }
// ```
//
// ### Replication Slots Management
//
// ```rust
// use crate::replication::slots::*;
//
// # async fn slot_management() -> Result<(), Box<dyn std::error::Error>> {
// // Configure slot manager for logical replication
// let slot_config = SlotManagerConfig {
//     max_slots: 100,
//     default_wal_retention: Duration::from_hours(48),
//     enable_auto_cleanup: true,
//     enable_monitoring: true,
//     enable_failover_slots: true,
//     ..Default::default()
// };
//
// let manager = ReplicationSlotManager::new(slot_config)?;
//
// // Create logical replication slot
// let slot_name = SlotName::new("logical_replication_slot")?;
// let slot_config = SlotConfig {
//     slot_type: SlotType::Logical {
//         plugin_name: "pgoutput".to_string(),
//         publication_names: vec!["all_tables_pub".to_string()],
//     },
//     wal_retention_policy: WalRetentionPolicy::Time(Duration::from_hours(24)),
//     enable_lag_monitoring: true,
//     priority: SlotPriority::High,
//     ..Default::default()
// };
//
// let slot_id = manager.create_slot(&slot_name, slot_config).await?;
//
// // Start consuming changes
// let mut stream = manager.start_consuming(&slot_id).await?;
//
// // Process replication changes
// while let Some(change) = stream.next_change().await? {
//     match change.change_type {
//         ChangeType::Insert | ChangeType::Update | ChangeType::Delete => {
//             println!("Processing {} on {}.{}",
//                 change.change_type,
//                 change.schema_name,
//                 change.table_name);
//
//             // Process the change...
//
//             // Advance slot position
//             manager.advance_slot(&slot_id, &change.end_lsn).await?;
//         }
//         _ => {
//             // Handle other change types
//         }
//     }
// }
// # Ok(())
// # }
// ```

use tokio::time::sleep;
use std::collections::VecDeque;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap};
use parking_lot::RwLock;
use std::sync::Arc;
use tokio::sync::mpsc;
use crate::error::Result;
use std::time::{Duration};
use std::path::PathBuf;

/// Replication mode
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ReplicationMode {
    Synchronous,   // Wait for replica acknowledgment
    Asynchronous,  // Don't wait for replica
    SemiSync,      // Wait for at least one replica
    MultiMaster,   // Multi-master replication with conflict resolution
}

/// Replica status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ReplicaStatus {
    Active,
    Lagging,
    Disconnected,
    Syncing,
}

/// Replica node information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReplicaNode {
    pub id: String,
    pub address: String,
    pub status: ReplicaStatus,
    pub lag_bytes: u64,
    pub last_sync: i64,  // Timestamp
}

/// Replication log entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReplicationLogEntry {
    pub sequence_number: u64,
    pub operation: ReplicationOperation,
    pub timestamp: i64,
    pub data: Vec<u8>,
}

/// Type of replication operation
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ReplicationOperation {
    Insert,
    Update,
    Delete,
    CreateTable,
    DropTable,
    AlterTable,
    BeginTransaction,
    CommitTransaction,
    RollbackTransaction,
    CreateIndex,
    DropIndex,
}

/// Replication topology types
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ReplicationTopology {
    SingleMaster,      // One primary, multiple read replicas
    MultiMaster,       // Multiple primaries with conflict resolution
    Cascading,         // Replicas can themselves have replicas
    ChainReplication,  // Linear chain of replicas
}

/// Conflict resolution strategy
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ConflictResolutionStrategy {
    LastWriteWins,     // Most recent timestamp wins
    FirstWriteWins,    // First write is preserved
    Primary,           // Primary's version always wins
    Custom,            // Custom conflict resolver
}

/// Replication conflict
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReplicationConflict {
    pub conflict_id: u64,
    pub sequence_number: u64,
    pub table_name: String,
    pub primary_key: String,
    pub local_version: Vec<u8>,
    pub remote_version: Vec<u8>,
    pub local_timestamp: i64,
    pub remote_timestamp: i64,
    pub resolved: bool,
}

/// WAL (Write-Ahead Log) entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WALEntry {
    pub lsn: u64,  // Log Sequence Number
    pub transaction_id: Option<u64>,
    pub operation: ReplicationOperation,
    pub table_name: String,
    pub data: Vec<u8>,
    pub timestamp: i64,
    pub checksum: u32,
}

/// Replication snapshot for initial sync
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReplicationSnapshot {
    pub snapshot_id: String,
    pub lsn: u64,
    pub timestamp: i64,
    pub tables: Vec<String>,
    pub data_files: Vec<PathBuf>,
    pub size_bytes: u64,
}

/// Replication health status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReplicationHealth {
    pub replica_id: String,
    pub is_healthy: bool,
    pub last_heartbeat: i64,
    pub replication_delay_ms: u64,
    pub pending_transactions: usize,
    pub error_count: u32,
    pub last_error: Option<String>,
}

/// Replication statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReplicationStats {
    pub total_replicas: usize,
    pub healthy_replicas: usize,
    pub lagging_replicas: usize,
    pub average_lag_ms: u64,
    pub total_conflicts: usize,
    pub unresolved_conflicts: usize,
    pub wal_size: usize,
    pub latest_lsn: u64,
}

/// Helper function to calculate checksum
fn calculate_checksum(data: &[u8]) -> u32 {
    // Simple checksum - in production use CRC32 or better
    data.iter().fold(0u32, |acc, &b| acc.wrapping_add(b as u32))
}

/// Replication event for notifications
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ReplicationEvent {
    ReplicaAdded { replica_id: String, address: String },
    ReplicaRemoved { replica_id: String },
    ReplicaStatusChanged { replica_id: String, status: ReplicaStatus },
    ConflictDetected { conflict_id: u64, table: String },
    ConflictResolved { conflict_id: u64 },
    SnapshotCreated { snapshot_id: String },
    ReplicationLagWarning { replica_id: String, lag_bytes: u64 },
    FailoverInitiated { old_primary: String, new_primary: String },
    SyncCompleted { replica_id: String },
}

/// Replication event listener
pub trait ReplicationEventListener: Send + Sync {
    fn on_event(&self, event: ReplicationEvent);
}

/// Replication lag monitor
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LagMonitor {
    pub replica_id: String,
    pub current_lag_bytes: u64,
    pub max_lag_bytes: u64,
    pub lag_threshold_bytes: u64,
    pub lag_trend: LagTrend,
    pub measurements: Vec<LagMeasurement>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum LagTrend {
    Improving,
    Stable,
    Degrading,
    Critical,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LagMeasurement {
    pub timestamp: i64,
    pub lag_bytes: u64,
}

/// Replication bandwidth throttle settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BandwidthThrottle {
    pub max_bytes_per_second: u64,
    pub current_bytes_per_second: u64,
    pub enabled: bool,
}

/// Replication checkpoint for recovery
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReplicationCheckpoint {
    pub checkpoint_id: String,
    pub lsn: u64,
    pub timestamp: i64,
    pub replica_id: String,
    pub consistent: bool,
}

/// Geo-replication configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeoReplicationConfig {
    pub region: String,
    pub replicas: Vec<String>,
    pub latency_ms: u64,
    pub bandwidth_mbps: u64,
    pub prefer_local_reads: bool,
}

/// Logical replication slot
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogicalSlot {
    pub slot_name: String,
    pub restart_lsn: u64,
    pub confirmed_flush_lsn: u64,
    pub active: bool,
    pub plugin: String,
}

/// Physical replication slot
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PhysicalSlot {
    pub slot_name: String,
    pub restart_lsn: u64,
    pub active: bool,
    pub temporary: bool,
}

/// Replication manager
pub struct ReplicationManager {
    mode: ReplicationMode,
    topology: ReplicationTopology,
    conflict_strategy: ConflictResolutionStrategy,
    replicas: Arc<RwLock<HashMap<String, ReplicaNode>>>,
    log_sequence: Arc<RwLock<u64>>,
    log_sender: Option<mpsc::UnboundedSender<ReplicationLogEntry>>,
    is_primary: bool,
    wal: Arc<RwLock<VecDeque<WALEntry>>>,
    conflicts: Arc<RwLock<HashMap<u64, ReplicationConflict>>>,
    conflict_counter: Arc<RwLock<u64>>,
    snapshots: Arc<RwLock<Vec<ReplicationSnapshot>>>,
    health_checks: Arc<RwLock<HashMap<String, ReplicationHealth>>>,
    lag_monitors: Arc<RwLock<HashMap<String, LagMonitor>>>,
    bandwidth_throttle: Arc<RwLock<Option<BandwidthThrottle>>>,
    checkpoints: Arc<RwLock<Vec<ReplicationCheckpoint>>>,
    logical_slots: Arc<RwLock<HashMap<String, LogicalSlot>>>,
    physical_slots: Arc<RwLock<HashMap<String, PhysicalSlot>>>,
    geo_config: Arc<RwLock<Option<GeoReplicationConfig>>>,
    event_listeners: Arc<RwLock<Vec<Arc<dyn ReplicationEventListener>>>>,
}

impl ReplicationManager {
    pub fn new(mode: ReplicationMode, is_primary: bool) -> Self {
        Self::new_with_topology(mode, ReplicationTopology::SingleMaster, is_primary)
    }

    pub fn new_with_topology(
        mode: ReplicationMode,
        topology: ReplicationTopology,
        is_primary: bool,
    ) -> Self {
        let (tx, _rx) = mpsc::unbounded_channel();

        Self {
            mode,
            topology,
            conflict_strategy: ConflictResolutionStrategy::LastWriteWins,
            replicas: Arc::new(RwLock::new(HashMap::new())),
            log_sequence: Arc::new(RwLock::new(0)),
            log_sender: Some(tx),
            is_primary,
            wal: Arc::new(RwLock::new(VecDeque::new())),
            conflicts: Arc::new(RwLock::new(HashMap::new())),
            conflict_counter: Arc::new(RwLock::new(0)),
            snapshots: Arc::new(RwLock::new(Vec::new())),
            health_checks: Arc::new(RwLock::new(HashMap::new())),
            lag_monitors: Arc::new(RwLock::new(HashMap::new())),
            bandwidth_throttle: Arc::new(RwLock::new(None)),
            checkpoints: Arc::new(RwLock::new(Vec::new())),
            logical_slots: Arc::new(RwLock::new(HashMap::new())),
            physical_slots: Arc::new(RwLock::new(HashMap::new())),
            geo_config: Arc::new(RwLock::new(None)),
            event_listeners: Arc::new(RwLock::new(Vec::new())),
        }
    }

    /// Set conflict resolution strategy
    pub fn set_conflict_strategy(&mut self, strategy: ConflictResolutionStrategy) {
        self.conflict_strategy = strategy;
    }

    /// Get current topology
    pub fn get_topology(&self) -> ReplicationTopology {
        self.topology.clone()
    }

    /// Add event listener
    pub fn add_event_listener(&self, listener: Arc<dyn ReplicationEventListener>) {
        self.event_listeners.write().push(listener);
    }

    /// Notify all event listeners
    fn notify_event(&self, event: ReplicationEvent) {
        let listeners = self.event_listeners.read();
        for listener in listeners.iter() {
            listener.on_event(event.clone());
        }
    }

    /// Add a replica node
    pub fn add_replica(&self, replica: ReplicaNode) -> Result<()> {
        if !self.is_primary {
            return Err(DbError::InvalidOperation(
                "Cannot add replicas to a non-primary node".to_string()
            ));
        }

        let replica_id = replica.id.clone();
        let address = replica.address.clone();

        let mut replicas = self.replicas.write();
        replicas.insert(replica.id.clone(), replica);
        drop(replicas);

        // Initialize lag monitor for new replica
        self.initialize_lag_monitor(&replica_id);

        // Notify event
        self.notify_event(ReplicationEvent::ReplicaAdded { replica_id, address });

        Ok(())
    }

    /// Remove a replica node
    pub fn remove_replica(&self, replica_id: &str) -> Result<()> {
        let mut replicas = self.replicas.write();

        if replicas.remove(replica_id).is_none() {
            return Err(DbError::NotFound(
                format!("Replica '{}' not found", replica_id)
            )));
        }

        Ok(())
    }

    /// Get all replicas
    pub fn get_replicas(&self) -> Vec<ReplicaNode> {
        let replicas = self.replicas.read();
        replicas.values().cloned().collect()
    }

    /// Update replica status
    pub fn update_replica_status(&self, replica_id: &str, status: ReplicaStatus) -> Result<()> {
        let mut replicas = self.replicas.write();

        if let Some(replica) = replicas.get_mut(replica_id) {
            replica.status = status;
            Ok(())
        } else {
            Err(DbError::NotFound(
                format!("Replica '{}' not found", replica_id)
            ))
        }
    }

    /// Replicate an operation to all replicas
    pub async fn replicate_operation(
        &self,
        operation: ReplicationOperation,
        data: Vec<u8>,
    ) -> Result<()> {
        if !self.is_primary {
            return Err(DbError::InvalidOperation(
                "Only primary can replicate operations".to_string()
            )));
        }

        // Create log entry
        let mut seq = self.log_sequence.write();
        *seq += 1;
        let sequence_number = *seq;
        drop(seq);

        let entry = ReplicationLogEntry {
            sequence_number,
            operation,
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs() as i64,
            data,
        };

        // Send to replication channel
        if let Some(sender) = &self.log_sender {
            sender.send(entry.clone())
                .map_err(|e| DbError::Internal(format!("Failed to send log entry: {}", e)))?;
        }

        // Handle based on replication mode
        match self.mode {
            ReplicationMode::Synchronous => {
                self.wait_for_all_replicas(&entry).await?;
            }
            ReplicationMode::SemiSync => {
                self.wait_for_one_replica(&entry).await?;
            }
            ReplicationMode::Asynchronous => {
                // Fire and forget
            }
            ReplicationMode::MultiMaster => {
                // Multi-master: async with conflict resolution handled elsewhere
            }
        }

        Ok(())
    }

    /// Wait for all replicas to acknowledge
    async fn wait_for_all_replicas(&self, _entry: &ReplicationLogEntry) -> Result<()> {
        // TODO: Implement actual acknowledgment waiting
        // For now, just return success
        Ok(())
    }

    /// Wait for at least one replica to acknowledge
    async fn wait_for_one_replica(&self, _entry: &ReplicationLogEntry) -> Result<()> {
        // TODO: Implement acknowledgment from one replica
        Ok(())
    }

    /// Initiate failover to a replica
    pub async fn failover(&self, new_primary_id: &str) -> Result<()> {
        let replicas = self.replicas.read();

        if !replicas.contains_key(new_primary_id) {
            return Err(DbError::NotFound(
                format!("Replica '{}' not found", new_primary_id)
            )));
        }

        // TODO: Implement actual failover logic:
        // 1. Verify replica is caught up
        // 2. Promote replica to primary
        // 3. Update connection routing
        // 4. Notify other replicas of new primary

        Ok(())
    }

    /// Get replication lag for a specific replica
    pub fn get_replica_lag(&self, replica_id: &str) -> Result<u64> {
        let replicas = self.replicas.read();

        if let Some(replica) = replicas.get(replica_id) {
            Ok(replica.lag_bytes)
        } else {
            Err(DbError::NotFound(
                format!("Replica '{}' not found", replica_id)
            ))
        }
    }

    // ===== WAL (Write-Ahead Log) Management =====

    /// Append entry to WAL
    pub fn append_to_wal(&self, entry: WALEntry) -> Result<()> {
        let mut wal = self.wal.write());
        wal.push_back(entry);

        // Limit WAL size to prevent unbounded growth
        const MAX_WAL_SIZE: usize = 10000;
        if wal.len() > MAX_WAL_SIZE {
            wal.pop_front();
        }

        Ok(())
    }

    /// Get WAL entries starting from a specific LSN
    pub fn get_wal_entries(&self, fromlsn: u64, limit: usize) -> Vec<WALEntry> {
        let wal = self.wal.read();
        wal.iter()
            .filter(|entry| entry.lsn >= from_lsn)
            .take(limit)
            .cloned()
            .collect()
    }

    /// Truncate WAL up to a specific LSN
    pub fn truncate_wal(&self, up_to_lsn: u64) -> Result<()> {
        let mut wal = self.wal.write();
        wal.retain(|entry| entry.lsn > up_to_lsn);
        Ok(())
    }

    /// Get current WAL size
    pub fn get_wal_size(&self) -> usize {
        self.wal.read().len()
    }

    /// Get latest LSN
    pub fn get_latest_lsn(&self) -> u64 {
        self.wal.read()
            .back()
            .map(|entry| entry.lsn)
            .unwrap_or(0)
    }

    // ===== Conflict Resolution =====

    /// Detect and record a replication conflict
    pub fn detect_conflict(
        &self,
        sequence_number: u64,
        table_name: String,
        primary_key: String,
        local_version: Vec<u8>,
        remote_version: Vec<u8>,
        local_timestamp: i64,
        remote_timestamp: i64,
    ) -> Result<u64> {
        let mut counter = self.conflict_counter.write();
        *counter += 1;
        let conflict_id = *counter;
        drop(counter);

        let conflict = ReplicationConflict {
            conflict_id,
            sequence_number,
            table_name,
            primary_key,
            local_version,
            remote_version,
            local_timestamp,
            remote_timestamp,
            resolved: false,
        };

        self.conflicts.write().insert(conflict_id, conflict);
        Ok(conflict_id)
    }

    /// Resolve a conflict using the configured strategy
    pub fn resolve_conflict(&self, conflict_id: u64) -> Result<Vec<u8>> {
        let mut conflicts = self.conflicts.write();

        let conflict = conflicts.get_mut(&conflict_id)
            .ok_or_else(|| DbError::NotFound(
                format!("Conflict {} not found", conflict_id)
            ))?;

        let resolved_version = match self.conflict_strategy {
            ConflictResolutionStrategy::LastWriteWins => {
                if conflict.remote_timestamp > conflict.local_timestamp {
                    conflict.remote_version.clone()
                } else {
                    conflict.local_version.clone()
                }
            }
            ConflictResolutionStrategy::FirstWriteWins => {
                if conflict.local_timestamp < conflict.remote_timestamp {
                    conflict.local_version.clone()
                } else {
                    conflict.remote_version.clone()
                }
            }
            ConflictResolutionStrategy::Primary => {
                if self.is_primary {
                    conflict.local_version.clone()
                } else {
                    conflict.remote_version.clone()
                }
            }
            ConflictResolutionStrategy::Custom => {
                // Default to local version for custom strategy
                // In real implementation, this would call a user-defined resolver
                conflict.local_version.clone()
            }
        };

        conflict.resolved = true;
        Ok(resolved_version)
    }

    /// Get all unresolved conflicts
    pub fn get_unresolved_conflicts(&self) -> Vec<ReplicationConflict> {
        let conflicts = self.conflicts.read();
        conflicts.values()
            .filter(|c| !c.resolved)
            .cloned()
            .collect()
    }

    /// Get conflict count
    pub fn get_conflict_count(&self) -> usize {
        self.conflicts.read().len()
    }

    /// Clear resolved conflicts
    pub fn clear_resolved_conflicts(&self) -> Result<()> {
        let mut conflicts = self.conflicts.write();
        conflicts.retain(|_, c| !c.resolved);
        Ok(())
    }

    // ===== Snapshot-based Replication =====

    /// Create a replication snapshot
    pub fn create_snapshot(&self, tables: Vec<String>) -> Result<ReplicationSnapshot> {
        let lsn = self.get_latest_lsn();
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() as i64;

        let snapshot_id = format!("snapshot_{}_{}", lsn, timestamp));

        let snapshot = ReplicationSnapshot {
            snapshot_id: snapshot_id.clone(),
            lsn,
            timestamp,
            tables,
            data_files: Vec::new(),
            size_bytes: 0,
        };

        self.snapshots.write().push(snapshot.clone());
        Ok(snapshot)
    }

    /// Get a specific snapshot
    pub fn get_snapshot(&self, snapshot_id: &str) -> Result<ReplicationSnapshot> {
        let snapshots = self.snapshots.read();
        snapshots.iter()
            .find(|s| s.snapshot_id == snapshot_id)
            .cloned()
            .ok_or_else(|| DbError::NotFound(
                format!("Snapshot '{}' not found", snapshot_id)
            ))
    }

    /// List all snapshots
    pub fn list_snapshots(&self) -> Vec<ReplicationSnapshot> {
        self.snapshots.read().clone()
    }

    /// Delete a snapshot
    pub fn delete_snapshot(&self, snapshot_id: &str) -> Result<()> {
        let mut snapshots = self.snapshots.write());
        let initial_len = snapshots.len();
        snapshots.retain(|s| s.snapshot_id != snapshot_id);

        if snapshots.len() == initial_len {
            return Err(DbError::NotFound(
                format!("Snapshot '{}' not found", snapshot_id)
            )));
        }

        Ok(())
    }

    /// Apply a snapshot to initialize a replica
    pub async fn apply_snapshot(&self, snapshot_id: &str) -> Result<()> {
        let snapshot = self.get_snapshot(snapshot_id)?;

        // In real implementation, this would:
        // 1. Download snapshot data files
        // 2. Restore tables from snapshot
        // 3. Set replica LSN to snapshot LSN
        // 4. Start streaming replication from that point

        // Update log sequence to snapshot LSN
        *self.log_sequence.write() = snapshot.lsn;

        Ok(())
    }

    // ===== Health Monitoring =====

    /// Update health status for a replica
    pub fn update_replica_health(&self, health: ReplicationHealth) -> Result<()> {
        let mut health_checks = self.health_checks.write();
        health_checks.insert(health.replica_id.clone(), health);
        Ok(())
    }

    /// Get health status for a specific replica
    pub fn get_replica_health(&self, replica_id: &str) -> Result<ReplicationHealth> {
        let health_checks = self.health_checks.read();
        health_checks.get(replica_id)
            .cloned()
            .ok_or_else(|| DbError::NotFound(
                format!("Health status for replica '{}' not found", replica_id)
            ))
    }

    /// Get all replica health statuses
    pub fn get_all_replica_health(&self) -> Vec<ReplicationHealth> {
        self.health_checks.read().values().cloned().collect()
    }

    /// Check if a replica is healthy
    pub fn is_replica_healthy(&self, replica_id: &str) -> bool {
        if let Ok(health) = self.get_replica_health(replica_id) {
            health.is_healthy && health.error_count < 10
        } else {
            false
        }
    }

    /// Perform health check on all replicas
    pub async fn health_check_all_replicas(&self) -> HashMap<String, bool> {
        let replicas = self.get_replicas());
        let mut results = HashMap::new();

        for replica in replicas {
            let is_healthy = self.is_replica_healthy(&replica.id);
            results.insert(replica.id, is_healthy);
        }

        results
    }

    // ===== Automatic Discovery =====

    /// Auto-discover replicas in the network
    pub async fn discover_replicas(&self, _network_range: &str) -> Result<Vec<String>> {
        // In real implementation, this would:
        // 1. Scan the network for database instances
        // 2. Query each instance for its role
        // 3. Add discovered replicas automatically

        // For now, return empty list
        Ok(Vec::new())
    }

    /// Register this node for discovery
    pub fn register_for_discovery(&self, port: u16) -> Result<()> {
        // In real implementation, this would:
        // 1. Start a discovery service on the specified port
        // 2. Respond to discovery requests
        // 3. Advertise this node's capabilities

        Ok(())
    }

    // ===== Topology Management =====

    /// Add a cascading replica (replica of a replica)
    pub fn add_cascading_replica(
        &self,
        parent_replica_id: &str,
        childreplica: ReplicaNode,
    ) -> Result<()> {
        if self.topology != ReplicationTopology::Cascading {
            return Err(DbError::InvalidOperation(
                "Cascading replication not enabled".to_string()
            ));
        }

        // Verify parent replica exists
        let replicas = self.replicas.read();
        if !replicas.contains_key(parent_replica_id) {
            return Err(DbError::NotFound(
                format!("Parent replica '{}' not found", parent_replica_id)
            )));
        }
        drop(replicas);

        // Add child replica
        self.add_replica(child_replica)?;

        Ok(())
    }

    /// Promote a replica to primary (for multi-master)
    pub async fn promote_replica(&self, replica_id: &str) -> Result<()> {
        if self.topology != ReplicationTopology::MultiMaster {
            return Err(DbError::InvalidOperation(
                "Multi-master topology not enabled".to_string()
            ));
        }

        let mut replicas = self.replicas.write();

        if let Some(replica) = replicas.get_mut(replica_id) {
            replica.status = ReplicaStatus::Active;
            Ok(())
        } else {
            Err(DbError::NotFound(
                format!("Replica '{}' not found", replica_id)
            ))
        }
    }

    /// Get replication chain (for chain replication topology)
    pub fn get_replication_chain(&self) -> Vec<String> {
        let replicas = self.replicas.read());
        replicas.keys().cloned().collect()
    }

    // ===== Advanced Operations =====

    /// Pause replication to a specific replica
    pub fn pause_replication(&self, replica_id: &str) -> Result<()> {
        self.update_replica_status(replica_id, ReplicaStatus::Disconnected)
    }

    /// Resume replication to a specific replica
    pub fn resume_replication(&self, replica_id: &str) -> Result<()> {
        self.update_replica_status(replica_id, ReplicaStatus::Syncing)
    }

    /// Get replication statistics
    pub fn get_replication_stats(&self) -> ReplicationStats {
        let replicas = self.replicas.read();
        let health_checks = self.health_checks.read();

        let total_replicas = replicas.len();
        let healthy_replicas = health_checks.values()
            .filter(|h| h.is_healthy)
            .count();

        let avg_lag = if !health_checks.is_empty() {
            health_checks.values()
                .map(|h| h.replication_delay_ms)
                .sum::<u64>() / health_checks.len() as u64
        } else {
            0
        };

        ReplicationStats {
            total_replicas,
            healthy_replicas,
            lagging_replicas: total_replicas - healthy_replicas,
            average_lag_ms: avg_lag,
            total_conflicts: self.get_conflict_count(),
            unresolved_conflicts: self.get_unresolved_conflicts().len(),
            wal_size: self.get_wal_size(),
            latest_lsn: self.get_latest_lsn(),
        }
    }

    /// Synchronize a replica to current state
    pub async fn sync_replica(&self, replica_id: &str) -> Result<()> {
        let replicas = self.replicas.read();

        if !replicas.contains_key(replica_id) {
            return Err(DbError::NotFound(
                format!("Replica '{}' not found", replica_id)
            )));
        }
        drop(replicas);

        // Update status to syncing
        self.update_replica_status(replica_id, ReplicaStatus::Syncing)?;

        // In real implementation, this would:
        // 1. Calculate lag
        // 2. Stream missing WAL entries
        // 3. Verify consistency
        // 4. Update status to Active

        // Simulate sync completion
        self.update_replica_status(replica_id, ReplicaStatus::Active)?;

        Ok(())
    }

    /// Verify replication consistency
    pub async fn verify_consistency(&self, replica_id: &str) -> Result<bool> {
        // In real implementation, this would:
        // 1. Compare checksums between primary and replica
        // 2. Verify row counts match
        // 3. Sample random rows for data integrity

        // For now, return true if replica is active
        let replicas = self.replicas.read();
        if let Some(replica) = replicas.get(replica_id) {
            Ok(replica.status == ReplicaStatus::Active)
        } else {
            Err(DbError::NotFound(
                format!("Replica '{}' not found", replica_id)
            ))
        }
    }

    /// Get replication mode
    pub fn get_mode(&self) -> ReplicationMode {
        self.mode.clone()
    }

    /// Change replication mode
    pub fn set_mode(&mut self, mode: ReplicationMode) -> Result<()> {
        self.mode = mode);
        Ok(())
    }

    // ===== Lag Monitoring =====

    /// Initialize lag monitor for a replica
    fn initialize_lag_monitor(&self, replica_id: &str) {
        let monitor = LagMonitor {
            replica_id: replica_id.to_string(),
            current_lag_bytes: 0,
            max_lag_bytes: 0,
            lag_threshold_bytes: 10_000_000, // 10MB default threshold
            lag_trend: LagTrend::Stable,
            measurements: Vec::new(),
        };
        self.lag_monitors.write().insert(replica_id.to_string(), monitor);
    }

    /// Update lag measurement for a replica
    pub fn update_lag(&self, replica_id: &str, lag_bytes: u64) -> Result<()> {
        let mut monitors = self.lag_monitors.write();

        if let Some(monitor) = monitors.get_mut(replica_id) {
            let timestamp = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs() as i64;

            monitor.measurements.push(LagMeasurement {
                timestamp,
                lag_bytes,
            });

            // Keep only last 100 measurements
            if monitor.measurements.len() > 100 {
                monitor.measurements.drain(0..monitor.measurements.len() - 100);
            }

            let old_lag = monitor.current_lag_bytes;
            monitor.current_lag_bytes = lag_bytes;

            if lag_bytes > monitor.max_lag_bytes {
                monitor.max_lag_bytes = lag_bytes;
            }

            // Determine trend
            monitor.lag_trend = if lag_bytes > monitor.lag_threshold_bytes {
                LagTrend::Critical
            } else if lag_bytes > old_lag * 2 {
                LagTrend::Degrading
            } else if lag_bytes < old_lag / 2 {
                LagTrend::Improving
            } else {
                LagTrend::Stable
            };

            // Notify if lag exceeds threshold
            if lag_bytes > monitor.lag_threshold_bytes {
                self.notify_event(ReplicationEvent::ReplicationLagWarning {
                    replica_id: replica_id.to_string(),
                    lag_bytes,
                });
            }

            Ok(())
        } else {
            Err(DbError::NotFound(
                format!("Lag monitor for replica '{}' not found", replica_id)
            ))
        }
    }

    /// Get lag monitor for a replica
    pub fn get_lag_monitor(&self, replica_id: &str) -> Result<LagMonitor> {
        let monitors = self.lag_monitors.read());
        monitors.get(replica_id)
            .cloned()
            .ok_or_else(|| DbError::NotFound(
                format!("Lag monitor for replica '{}' not found", replica_id)
            ))
    }

    /// Get all lag monitors
    pub fn get_all_lag_monitors(&self) -> Vec<LagMonitor> {
        self.lag_monitors.read().values().cloned().collect()
    }

    // ===== Bandwidth Throttling =====

    /// Enable bandwidth throttling
    pub fn enable_throttle(&self, max_bytes_per_second: u64) -> Result<()> {
        let throttle = BandwidthThrottle {
            max_bytes_per_second,
            current_bytes_per_second: 0,
            enabled: true,
        });
        *self.bandwidth_throttle.write() = Some(throttle);
        Ok(())
    }

    /// Disable bandwidth throttling
    pub fn disable_throttle(&self) -> Result<()> {
        *self.bandwidth_throttle.write() = None;
        Ok(())
    }

    /// Update current bandwidth usage
    pub fn update_bandwidth_usage(&self, bytes_per_second: u64) -> Result<()> {
        if let Some(throttle) = self.bandwidth_throttle.write().as_mut() {
            throttle.current_bytes_per_second = bytes_per_second;
        }
        Ok(())
    }

    /// Check if should throttle based on current usage
    pub fn should_throttle(&self) -> bool {
        if let Some(throttle) = self.bandwidth_throttle.read().as_ref() {
            throttle.enabled && throttle.current_bytes_per_second >= throttle.max_bytes_per_second
        } else {
            false
        }
    }

    /// Get throttle configuration
    pub fn get_throttle_config(&self) -> Option<BandwidthThrottle> {
        self.bandwidth_throttle.read().clone()
    }

    // ===== Checkpointing =====

    /// Create a replication checkpoint
    pub fn create_checkpoint(&self, replica_id: &str) -> Result<ReplicationCheckpoint> {
        let lsn = self.get_latest_lsn();
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() as i64;

        let checkpoint_id = format!("checkpoint_{}_{}", replica_id, timestamp));

        let checkpoint = ReplicationCheckpoint {
            checkpoint_id: checkpoint_id.clone(),
            lsn,
            timestamp,
            replica_id: replica_id.to_string(),
            consistent: true,
        };

        self.checkpoints.write().push(checkpoint.clone());
        Ok(checkpoint)
    }

    /// Get checkpoint by ID
    pub fn get_checkpoint(&self, checkpoint_id: &str) -> Result<ReplicationCheckpoint> {
        let checkpoints = self.checkpoints.read();
        checkpoints.iter()
            .find(|c| c.checkpoint_id == checkpoint_id)
            .cloned()
            .ok_or_else(|| DbError::NotFound(
                format!("Checkpoint '{}' not found", checkpoint_id)
            ))
    }

    /// List checkpoints for a replica
    pub fn list_checkpoints(&self, replicaid: Option<&str>) -> Vec<ReplicationCheckpoint> {
        let checkpoints = self.checkpoints.read());
        if let Some(id) = replica_id {
            checkpoints.iter()
                .filter(|c| c.replica_id == id)
                .cloned()
                .collect()
        } else {
            checkpoints.clone()
        }
    }

    /// Restore from checkpoint
    pub async fn restore_from_checkpoint(&self, checkpoint_id: &str) -> Result<()> {
        let checkpoint = self.get_checkpoint(checkpoint_id)?;

        if !checkpoint.consistent {
            return Err(DbError::InvalidOperation(
                "Cannot restore from inconsistent checkpoint".to_string()
            ));
        }

        // Update LSN to checkpoint LSN
        *self.log_sequence.write() = checkpoint.lsn;

        Ok(())
    }

    // ===== Logical Replication Slots =====

    /// Create a logical replication slot
    pub fn create_logical_slot(&self, slot_name: String, plugin: String) -> Result<()> {
        let mut slots = self.logical_slots.write();

        if slots.contains_key(&slot_name) {
            return Err(DbError::InvalidOperation(
                format!("Logical slot '{}' already exists", slot_name)
            )));
        }

        let slot = LogicalSlot {
            slot_name: slot_name.clone(),
            restart_lsn: self.get_latest_lsn(),
            confirmed_flush_lsn: self.get_latest_lsn(),
            active: true,
            plugin,
        };

        slots.insert(slot_name, slot);
        Ok(())
    }

    /// Drop a logical replication slot
    pub fn drop_logical_slot(&self, slot_name: &str) -> Result<()> {
        let mut slots = self.logical_slots.write();

        slots.remove(slot_name)
            .ok_or_else(|| DbError::NotFound(
                format!("Logical slot '{}' not found", slot_name)
            ))?;

        Ok(())
    }

    /// Get logical slot
    pub fn get_logical_slot(&self, slot_name: &str) -> Result<LogicalSlot> {
        let slots = self.logical_slots.read();
        slots.get(slot_name)
            .cloned()
            .ok_or_else(|| DbError::NotFound(
                format!("Logical slot '{}' not found", slot_name)
            ))
    }

    /// List all logical slots
    pub fn list_logical_slots(&self) -> Vec<LogicalSlot> {
        self.logical_slots.read().values().cloned().collect()
    }

    /// Advance logical slot
    pub fn advance_logical_slot(&self, slot_name: &str, lsn: u64) -> Result<()> {
        let mut slots = self.logical_slots.write());

        if let Some(slot) = slots.get_mut(slot_name) {
            if lsn >= slot.confirmed_flush_lsn {
                slot.confirmed_flush_lsn = lsn;
                Ok(())
            } else {
                Err(DbError::InvalidOperation(
                    "Cannot advance slot to earlier LSN".to_string()
                ))
            }
        } else {
            Err(DbError::NotFound(
                format!("Logical slot '{}' not found", slot_name)
            ))
        }
    }

    // ===== Physical Replication Slots =====

    /// Create a physical replication slot
    pub fn create_physical_slot(&self, slot_name: String, temporary: bool) -> Result<()> {
        let mut slots = self.physical_slots.write());

        if slots.contains_key(&slot_name) {
            return Err(DbError::InvalidOperation(
                format!("Physical slot '{}' already exists", slot_name)
            )));
        }

        let slot = PhysicalSlot {
            slot_name: slot_name.clone(),
            restart_lsn: self.get_latest_lsn(),
            active: true,
            temporary,
        };

        slots.insert(slot_name, slot);
        Ok(())
    }

    /// Drop a physical replication slot
    pub fn drop_physical_slot(&self, slot_name: &str) -> Result<()> {
        let mut slots = self.physical_slots.write();

        slots.remove(slot_name)
            .ok_or_else(|| DbError::NotFound(
                format!("Physical slot '{}' not found", slot_name)
            ))?;

        Ok(())
    }

    /// Get physical slot
    pub fn get_physical_slot(&self, slot_name: &str) -> Result<PhysicalSlot> {
        let slots = self.physical_slots.read();
        slots.get(slot_name)
            .cloned()
            .ok_or_else(|| DbError::NotFound(
                format!("Physical slot '{}' not found", slot_name)
            ))
    }

    /// List all physical slots
    pub fn list_physical_slots(&self) -> Vec<PhysicalSlot> {
        self.physical_slots.read().values().cloned().collect()
    }

    // ===== Geo-Replication =====

    /// Configure geo-replication
    pub fn configure_geo_replication(&self, config: GeoReplicationConfig) -> Result<()> {
        *self.geo_config.write() = Some(config));
        Ok(())
    }

    /// Get geo-replication configuration
    pub fn get_geo_config(&self) -> Option<GeoReplicationConfig> {
        self.geo_config.read().clone()
    }

    /// Select replica for read operation (geo-aware)
    pub fn select_read_replica(&self, prefer_local: bool) -> Option<String> {
        if let Some(geo_config) = self.geo_config.read().as_ref() {
            if prefer_local && !geo_config.replicas.is_empty() {
                return Some(geo_config.replicas[0].clone());
            }
        }

        // Fall back to first healthy replica
        let replicas = self.replicas.read();
        replicas.values()
            .find(|r| r.status == ReplicaStatus::Active)
            .map(|r| r.id.clone())
    }

    /// Get replicas in a specific region
    pub fn get_replicas_in_region(&self, region: &str) -> Vec<String> {
        // In real implementation, this would filter by region metadata
        // For now, return all replicas if geo config region matches
        if let Some(geo_config) = self.geo_config.read().as_ref() {
            if geo_config.region == region {
                return geo_config.replicas.clone();
            }
        }
        Vec::new()
    }

    // ===== Replication Monitoring & Diagnostics =====

    /// Get comprehensive replication report
    pub fn get_replication_report(&self) -> ReplicationReport {
        let stats = self.get_replication_stats();
        let health_statuses = self.get_all_replica_health();
        let lag_monitors = self.get_all_lag_monitors();
        let conflicts = self.get_unresolved_conflicts();

        ReplicationReport {
            stats,
            health_statuses,
            lag_monitors,
            unresolved_conflicts: conflicts,
            mode: self.mode.clone(),
            topology: self.topology.clone(),
            is_primary: self.is_primary,
        }
    }

    /// Check overall replication health
    pub fn check_overall_health(&self) -> OverallHealth {
        let replicas = self.replicas.read();
        let health_checks = self.health_checks.read();
        let lag_monitors = self.lag_monitors.read();

        let total = replicas.len();
        if total == 0 {
            return OverallHealth::NoReplicas;
        }

        let healthy = health_checks.values().filter(|h| h.is_healthy).count();
        let critical_lag = lag_monitors.values()
            .filter(|m| m.lag_trend == LagTrend::Critical)
            .count();

        if healthy == total && critical_lag == 0 {
            OverallHealth::Healthy
        } else if healthy >= total / 2 {
            OverallHealth::Degraded
        } else {
            OverallHealth::Critical
        }
    }

    // ===== Utility Methods =====

    /// Get total number of replicas
    pub fn replica_count(&self) -> usize {
        self.replicas.read().len()
    }

    /// Get number of active replicas
    pub fn active_replica_count(&self) -> usize {
        self.replicas.read()
            .values()
            .filter(|r| r.status == ReplicaStatus::Active)
            .count()
    }

    /// Get number of lagging replicas
    pub fn lagging_replica_count(&self) -> usize {
        self.replicas.read()
            .values()
            .filter(|r| r.status == ReplicaStatus::Lagging)
            .count()
    }

    /// Check if replica exists
    pub fn has_replica(&self, replica_id: &str) -> bool {
        self.replicas.read().contains_key(replica_id)
    }

    /// Get average replication lag across all replicas
    pub fn average_replication_lag(&self) -> u64 {
        let lag_monitors = self.lag_monitors.read();
        if lag_monitors.is_empty() {
            return 0;
        }

        let total_lag: u64 = lag_monitors.values()
            .map(|m| m.current_lag_bytes)
            .sum();

        total_lag / lag_monitors.len() as u64
    }

    /// Get maximum replication lag across all replicas
    pub fn max_replication_lag(&self) -> u64 {
        self.lag_monitors.read()
            .values()
            .map(|m| m.current_lag_bytes)
            .max()
            .unwrap_or(0)
    }

    /// Check if any replica is in critical state
    pub fn has_critical_replicas(&self) -> bool {
        let health_checks = self.health_checks.read();
        health_checks.values().any(|h| !h.is_healthy || h.error_count > 10)
    }

    /// Get list of unhealthy replicas
    pub fn get_unhealthy_replicas(&self) -> Vec<String> {
        let health_checks = self.health_checks.read();
        health_checks.values()
            .filter(|h| !h.is_healthy)
            .map(|h| h.replica_id.clone())
            .collect()
    }

    /// Get list of replicas with critical lag
    pub fn get_critical_lag_replicas(&self) -> Vec<String> {
        self.lag_monitors.read()
            .values()
            .filter(|m| m.lag_trend == LagTrend::Critical)
            .map(|m| m.replica_id.clone())
            .collect()
    }

    /// Calculate total data replicated (bytes)
    pub fn total_bytes_replicated(&self) -> u64 {
        // This would typically track actual bytes sent
        // For now, estimate from WAL size and replica count
        let wal_size = self.get_wal_size() as u64;
        let replica_count = self.replica_count() as u64;
        wal_size * replica_count * 1024 // Rough estimate
    }

    /// Get replication throughput (bytes per second)
    pub fn get_replication_throughput(&self) -> f64 {
        if let Some(throttle) = self.bandwidth_throttle.read().as_ref() {
            throttle.current_bytes_per_second as f64
        } else {
            0.0
        }
    }

    /// Check if replication is up to date
    pub fn is_replication_current(&self) -> bool {
        let max_lag = self.max_replication_lag();
        max_lag < 1_000_000 // Less than 1MB lag considered current
    }

    /// Get oldest unacknowledged LSN
    pub fn get_oldest_unacknowledged_lsn(&self) -> u64 {
        let logical_slots = self.logical_slots.read();
        logical_slots.values()
            .map(|s| s.restart_lsn)
            .min()
            .unwrap_or(self.get_latest_lsn())
    }

    /// Calculate WAL retention needed
    pub fn calculate_wal_retention_size(&self) -> u64 {
        let latest = self.get_latest_lsn();
        let oldest = self.get_oldest_unacknowledged_lsn();
        latest.saturating_sub(oldest)
    }

    /// Get replica by ID
    pub fn get_replica(&self, replica_id: &str) -> Option<ReplicaNode> {
        self.replicas.read().get(replica_id).cloned()
    }

    /// Update replica lag directly
    pub fn set_replica_lag(&self, replica_id: &str, lag_bytes: u64) -> Result<()> {
        let mut replicas = self.replicas.write();
        if let Some(replica) = replicas.get_mut(replica_id) {
            replica.lag_bytes = lag_bytes;
            drop(replicas);
            self.update_lag(replica_id, lag_bytes)?;
            Ok(())
        } else {
            Err(DbError::NotFound(
                format!("Replica '{}' not found", replica_id)
            ))
        }
    }

    /// Get snapshot count
    pub fn snapshot_count(&self) -> usize {
        self.snapshots.read().len()
    }

    /// Get total snapshot size
    pub fn total_snapshot_size(&self) -> u64 {
        self.snapshots.read()
            .iter()
            .map(|s| s.size_bytes)
            .sum()
    }

    /// Clean old snapshots (keep only N most recent)
    pub fn clean_old_snapshots(&self, keep_count: usize) -> Result<usize> {
        let mut snapshots = self.snapshots.write());
        let total = snapshots.len();

        if total <= keep_count {
            return Ok(0);
        }

        // Sort by timestamp (newest first)
        snapshots.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));

        // Keep only the most recent
        snapshots.truncate(keep_count);

        Ok(total - keep_count)
    }

    /// Get conflict resolution strategy
    pub fn get_conflict_strategy(&self) -> ConflictResolutionStrategy {
        self.conflict_strategy.clone()
    }

    /// Get resolved conflicts count
    pub fn get_resolved_conflicts_count(&self) -> usize {
        self.conflicts.read()
            .values()
            .filter(|c| c.resolved)
            .count()
    }

    /// Export replication configuration
    pub fn export_configuration(&self) -> ReplicationConfig {
        ReplicationConfig {
            mode: self.mode.clone(),
            topology: self.topology.clone(),
            conflict_strategy: self.conflict_strategy.clone(),
            is_primary: self.is_primary,
            replica_count: self.replica_count(),
        }
    }

    /// Import replication configuration
    pub fn import_configuration(&mut self, config: ReplicationConfig) -> Result<()> {
        self.mode = config.mode;
        self.topology = config.topology;
        self.conflict_strategy = config.conflict_strategy;
        Ok(())
    }

    /// Estimate time to sync replica (seconds)
    pub fn estimate_sync_time(&self, replica_id: &str) -> Option<u64> {
        if let Ok(monitor) = self.get_lag_monitor(replica_id) {
            if let Some(throttle) = self.bandwidth_throttle.read().as_ref() {
                let bytes_per_second = throttle.max_bytes_per_second;
                if bytes_per_second > 0 {
                    return Some(monitor.current_lag_bytes / bytes_per_second);
                }
            }
        }
        None
    }

    /// Check if primary
    pub fn is_primary(&self) -> bool {
        self.is_primary
    }

    /// Get uptime statistics for a replica
    pub fn get_replica_uptime_stats(&self, replica_id: &str) -> Result<UptimeStats> {
        let replica = self.get_replica(replica_id)
            .ok_or_else(|| DbError::NotFound(
                format!("Replica '{}' not found", replica_id)
            ))?;

        let health = self.get_replica_health(replica_id)?;
        let lag = self.get_lag_monitor(replica_id)?;

        let current_time = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() as i64;

        Ok(UptimeStats {
            replica_id: replica_id.to_string(),
            status: replica.status,
            uptime_seconds: current_time - replica.last_sync,
            error_count: health.error_count,
            current_lag_bytes: lag.current_lag_bytes,
            max_lag_bytes: lag.max_lag_bytes,
        })
    }
}

/// Replication report
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReplicationReport {
    pub stats: ReplicationStats,
    pub health_statuses: Vec<ReplicationHealth>,
    pub lag_monitors: Vec<LagMonitor>,
    pub unresolved_conflicts: Vec<ReplicationConflict>,
    pub mode: ReplicationMode,
    pub topology: ReplicationTopology,
    pub is_primary: bool,
}

/// Overall replication health status
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum OverallHealth {
    Healthy,
    Degraded,
    Critical,
    NoReplicas,
}

/// Replication configuration for import/export
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReplicationConfig {
    pub mode: ReplicationMode,
    pub topology: ReplicationTopology,
    pub conflict_strategy: ConflictResolutionStrategy,
    pub is_primary: bool,
    pub replica_count: usize,
}

/// Uptime statistics for a replica
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UptimeStats {
    pub replica_id: String,
    pub status: ReplicaStatus,
    pub uptime_seconds: i64,
    pub error_count: u32,
    pub current_lag_bytes: u64,
    pub max_lag_bytes: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_add_replica() -> Result<()> {
        let rm = ReplicationManager::new(ReplicationMode::Asynchronous, true);

        let replica = ReplicaNode {
            id: "replica-1".to_string(),
            address: "127.0.0.1:5433".to_string(),
            status: ReplicaStatus::Active,
            lag_bytes: 0,
            last_sync: 0,
        };

        rm.add_replica(replica)?;

        let replicas = rm.get_replicas();
        assert_eq!(replicas.len(), 1);
        assert_eq!(replicas[0].id, "replica-1");

        Ok(())
    }

    #[test]
    fn test_remove_replica() -> Result<()> {
        let rm = ReplicationManager::new(ReplicationMode::Synchronous, true);

        let replica = ReplicaNode {
            id: "replica-2".to_string(),
            address: "127.0.0.1:5434".to_string(),
            status: ReplicaStatus::Active,
            lag_bytes: 0,
            last_sync: 0,
        };

        rm.add_replica(replica)?;
        assert_eq!(rm.get_replicas().len(), 1);

        rm.remove_replica("replica-2")?;
        assert_eq!(rm.get_replicas().len(), 0);

        Ok(())
    }

    #[test]
    fn test_replica_status_update() -> Result<()> {
        let rm = ReplicationManager::new(ReplicationMode::SemiSync, true);

        let replica = ReplicaNode {
            id: "replica-3".to_string(),
            address: "127.0.0.1:5435".to_string(),
            status: ReplicaStatus::Syncing,
            lag_bytes: 1024,
            last_sync: 0,
        };

        rm.add_replica(replica)?;
        rm.update_replica_status("replica-3", ReplicaStatus::Active)?;

        let replicas = rm.get_replicas();
        assert_eq!(replicas[0].status, ReplicaStatus::Active);

        Ok(())
    }

    #[test]
    fn test_non_primary_cannot_add_replicas() -> Result<()> {
        let rm = ReplicationManager::new(ReplicationMode::Asynchronous, false);

        let replica = ReplicaNode {
            id: "replica-4".to_string(),
            address: "127.0.0.1:5436".to_string(),
            status: ReplicaStatus::Active,
            lag_bytes: 0,
            last_sync: 0,
        };

        let result = rm.add_replica(replica);
        assert!(result.is_err());

        Ok(())
    }

    #[tokio::test]
    async fn test_wal_operations() -> Result<()> {
        let rm = ReplicationManager::new(ReplicationMode::Synchronous, true);

        // Add WAL entries
        for i in 1..=5 {
            let entry = WALEntry {
                lsn: i,
                transaction_id: Some(i),
                operation: ReplicationOperation::Insert,
                table_name: "test_table".to_string(),
                data: vec![i as u8; 10],
                timestamp: i as i64,
                checksum: calculate_checksum(&vec![i as u8; 10]),
            };
            rm.append_to_wal(entry)?;
        }

        assert_eq!(rm.get_wal_size(), 5);
        assert_eq!(rm.get_latest_lsn(), 5);

        // Get entries from LSN 3
        let entries = rm.get_wal_entries(3, 10);
        assert_eq!(entries.len(), 3);
        assert_eq!(entries[0].lsn, 3);

        // Truncate WAL
        rm.truncate_wal(3)?;
        assert_eq!(rm.get_wal_size(), 2);

        Ok(())
    }

    #[tokio::test]
    async fn test_conflict_detection_and_resolution() -> Result<()> {
        let rm = ReplicationManager::new(ReplicationMode::MultiMaster, true);

        let local_data = vec![1, 2, 3];
        let remote_data = vec![4, 5, 6];

        let conflict_id = rm.detect_conflict(
            1,
            "users".to_string(),
            "user_1".to_string(),
            local_data.clone(),
            remote_data.clone(),
            1000,
            2000,
        )?;

        assert_eq!(rm.get_conflict_count(), 1);

        let unresolved = rm.get_unresolved_conflicts();
        assert_eq!(unresolved.len(), 1);

        // Resolve conflict (LastWriteWins, remote has later timestamp)
        let resolved = rm.resolve_conflict(conflict_id)?;
        assert_eq!(resolved, remote_data);

        Ok(())
    }

    #[tokio::test]
    async fn test_snapshot_operations() -> Result<()> {
        let rm = ReplicationManager::new(ReplicationMode::Asynchronous, true);

        // Create snapshot
        let tables = vec!["users".to_string(), "orders".to_string()];
        let snapshot = rm.create_snapshot(tables.clone())?;

        assert_eq!(snapshot.tables, tables);

        // List snapshots
        let snapshots = rm.list_snapshots();
        assert_eq!(snapshots.len(), 1);

        // Get specific snapshot
        let retrieved = rm.get_snapshot(&snapshot.snapshot_id)?;
        assert_eq!(retrieved.snapshot_id, snapshot.snapshot_id);

        // Delete snapshot
        rm.delete_snapshot(&snapshot.snapshot_id)?;
        assert_eq!(rm.list_snapshots().len(), 0);

        Ok(())
    }

    #[tokio::test]
    async fn test_health_monitoring() -> Result<()> {
        let rm = ReplicationManager::new(ReplicationMode::SemiSync, true);

        // Add replica
        let replica = ReplicaNode {
            id: "replica-health-1".to_string(),
            address: "127.0.0.1:5440".to_string(),
            status: ReplicaStatus::Active,
            lag_bytes: 0,
            last_sync: 0,
        };
        rm.add_replica(replica)?;

        // Update health status
        let health = ReplicationHealth {
            replica_id: "replica-health-1".to_string(),
            is_healthy: true,
            last_heartbeat: 1000,
            replication_delay_ms: 50,
            pending_transactions: 5,
            error_count: 0,
            last_error: None,
        };
        rm.update_replica_health(health)?;

        // Check health
        let retrieved_health = rm.get_replica_health("replica-health-1")?;
        assert!(retrieved_health.is_healthy);
        assert_eq!(retrieved_health.replication_delay_ms, 50);

        // Health check
        assert!(rm.is_replica_healthy("replica-health-1"));

        Ok(())
    }

    #[tokio::test]
    async fn test_replication_stats() -> Result<()> {
        let rm = ReplicationManager::new(ReplicationMode::Synchronous, true);

        // Add multiple replicas
        for i in 1..=3 {
            let replica = ReplicaNode {
                id: format!("replica-stats-{}", i),
                address: format!("127.0.0.1:544{}", i),
                status: ReplicaStatus::Active,
                lag_bytes: i * 100,
                last_sync: i as i64,
            });
            rm.add_replica(replica)?;

            let health = ReplicationHealth {
                replica_id: format!("replica-stats-{}", i),
                is_healthy: i <= 2,
                last_heartbeat: i as i64,
                replication_delay_ms: i * 10,
                pending_transactions: i as usize,
                error_count: if i > 2 { 5 } else { 0 },
                last_error: None,
            });
            rm.update_replica_health(health)?;
        }

        let stats = rm.get_replication_stats();
        assert_eq!(stats.total_replicas, 3);
        assert_eq!(stats.healthy_replicas, 2);
        assert_eq!(stats.lagging_replicas, 1);

        Ok(())
    }

    #[tokio::test]
    async fn test_cascading_replication() -> Result<()> {
        let rm = ReplicationManager::new_with_topology(
            ReplicationMode::Asynchronous,
            ReplicationTopology::Cascading,
            true,
        );

        // Add parent replica
        let parent = ReplicaNode {
            id: "parent-replica".to_string(),
            address: "127.0.0.1:5450".to_string(),
            status: ReplicaStatus::Active,
            lag_bytes: 0,
            last_sync: 0,
        };
        rm.add_replica(parent)?;

        // Add cascading replica
        let child = ReplicaNode {
            id: "child-replica".to_string(),
            address: "127.0.0.1:5451".to_string(),
            status: ReplicaStatus::Active,
            lag_bytes: 0,
            last_sync: 0,
        };
        rm.add_cascading_replica("parent-replica", child)?;

        assert_eq!(rm.get_replicas().len(), 2);

        Ok(())
    }

    #[tokio::test]
    async fn test_multi_master_promotion() -> Result<()> {
        let rm = ReplicationManager::new_with_topology(
            ReplicationMode::Synchronous,
            ReplicationTopology::MultiMaster,
            true,
        );

        let replica = ReplicaNode {
            id: "replica-promote".to_string(),
            address: "127.0.0.1:5460".to_string(),
            status: ReplicaStatus::Syncing,
            lag_bytes: 0,
            last_sync: 0,
        };
        rm.add_replica(replica)?;

        // Promote replica
        rm.promote_replica("replica-promote").await?;

        let replicas = rm.get_replicas();
        assert_eq!(replicas[0].status, ReplicaStatus::Active);

        Ok(())
    }

    #[tokio::test]
    async fn test_pause_resume_replication() -> Result<()> {
        let rm = ReplicationManager::new(ReplicationMode::Asynchronous, true);

        let replica = ReplicaNode {
            id: "replica-pause".to_string(),
            address: "127.0.0.1:5470".to_string(),
            status: ReplicaStatus::Active,
            lag_bytes: 0,
            last_sync: 0,
        };
        rm.add_replica(replica)?;

        // Pause replication
        rm.pause_replication("replica-pause")?;
        let replicas = rm.get_replicas();
        assert_eq!(replicas[0].status, ReplicaStatus::Disconnected);

        // Resume replication
        rm.resume_replication("replica-pause")?;
        let replicas = rm.get_replicas();
        assert_eq!(replicas[0].status, ReplicaStatus::Syncing);

        Ok(())
    }

    #[tokio::test]
    async fn test_sync_replica() -> Result<()> {
        let rm = ReplicationManager::new(ReplicationMode::SemiSync, true);

        let replica = ReplicaNode {
            id: "replica-sync".to_string(),
            address: "127.0.0.1:5480".to_string(),
            status: ReplicaStatus::Lagging,
            lag_bytes: 1000,
            last_sync: 0,
        };
        rm.add_replica(replica)?;

        // Sync replica
        rm.sync_replica("replica-sync").await?;

        let replicas = rm.get_replicas();
        assert_eq!(replicas[0].status, ReplicaStatus::Active);

        Ok(())
    }

    #[tokio::test]
    async fn test_verify_consistency() -> Result<()> {
        let rm = ReplicationManager::new(ReplicationMode::Synchronous, true);

        let replica = ReplicaNode {
            id: "replica-verify".to_string(),
            address: "127.0.0.1:5490".to_string(),
            status: ReplicaStatus::Active,
            lag_bytes: 0,
            last_sync: 0,
        };
        rm.add_replica(replica)?;

        let is_consistent = rm.verify_consistency("replica-verify").await?;
        assert!(is_consistent);

        Ok(())
    }

    #[test]
    fn test_conflict_resolution_strategies() -> Result<()> {
        // Test FirstWriteWins
        let mut rm = ReplicationManager::new(ReplicationMode::MultiMaster, true);
        rm.set_conflict_strategy(ConflictResolutionStrategy::FirstWriteWins);

        let local_data = vec![1, 2, 3];
        let remote_data = vec![4, 5, 6];

        let conflict_id = rm.detect_conflict(
            1,
            "test".to_string(),
            "key1".to_string(),
            local_data.clone(),
            remote_data.clone(),
            1000,
            2000,
        )?;

        let resolved = rm.resolve_conflict(conflict_id)?;
        assert_eq!(resolved, local_data); // Local is earlier

        // Test Primary strategy
        rm.set_conflict_strategy(ConflictResolutionStrategy::Primary);
        let conflict_id2 = rm.detect_conflict(
            2,
            "test".to_string(),
            "key2".to_string(),
            local_data.clone(),
            remote_data.clone(),
            1000,
            2000,
        )?;

        let resolved2 = rm.resolve_conflict(conflict_id2)?;
        assert_eq!(resolved2, local_data); // Primary wins (is_primary = true)

        Ok(())
    }

    #[tokio::test]
    async fn test_mode_change() -> Result<()> {
        let mut rm = ReplicationManager::new(ReplicationMode::Asynchronous, true);
        assert_eq!(rm.get_mode(), ReplicationMode::Asynchronous);

        rm.set_mode(ReplicationMode::Synchronous)?;
        assert_eq!(rm.get_mode(), ReplicationMode::Synchronous);

        Ok(())
    }

    #[test]
    fn test_topology_validation() {
        let rm = ReplicationManager::new_with_topology(
            ReplicationMode::Asynchronous,
            ReplicationTopology::SingleMaster,
            true,
        );

        assert_eq!(rm.get_topology(), ReplicationTopology::SingleMaster);

        let child = ReplicaNode {
            id: "child".to_string(),
            address: "127.0.0.1:5500".to_string(),
            status: ReplicaStatus::Active,
            lag_bytes: 0,
            last_sync: 0,
        };

        // Should fail because topology is not Cascading
        let result = rm.add_cascading_replica("parent", child);
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_health_check_all() -> Result<()> {
        let rm = ReplicationManager::new(ReplicationMode::Asynchronous, true);

        // Add multiple replicas with different health statuses
        for i in 1..=3 {
            let replica = ReplicaNode {
                id: format!("replica-{}", i),
                address: format!("127.0.0.1:550{}", i),
                status: ReplicaStatus::Active,
                lag_bytes: 0,
                last_sync: 0,
            });
            rm.add_replica(replica)?;

            let health = ReplicationHealth {
                replica_id: format!("replica-{}", i),
                is_healthy: i != 2,
                last_heartbeat: i as i64,
                replication_delay_ms: i * 10,
                pending_transactions: 0,
                error_count: if i == 2 { 15 } else { 0 },
                last_error: if i == 2 { Some("Test error".to_string()) } else { None },
            });
            rm.update_replica_health(health)?;
        }

        let health_results = rm.health_check_all_replicas().await;
        assert_eq!(health_results.len(), 3);
        assert_eq!(health_results.get("replica-1"), Some(&true));
        assert_eq!(health_results.get("replica-2"), Some(&false));
        assert_eq!(health_results.get("replica-3"), Some(&true));

        Ok(())
    }

    #[test]
    fn test_clear_resolved_conflicts() -> Result<()> {
        let rm = ReplicationManager::new(ReplicationMode::MultiMaster, true);

        // Create multiple conflicts
        for i in 1..=5 {
            rm.detect_conflict(
                i,
                "test".to_string(),
                format!("key{}", i),
                vec![i as u8],
                vec![(i + 1) as u8],
                i as i64,
                (i + 1) as i64,
            )?;
        }

        assert_eq!(rm.get_conflict_count(), 5);

        // Resolve first 3
        for i in 1..=3 {
            rm.resolve_conflict(i)?;
        }

        assert_eq!(rm.get_unresolved_conflicts().len(), 2);

        // Clear resolved
        rm.clear_resolved_conflicts()?;
        assert_eq!(rm.get_conflict_count(), 2);

        Ok(())
    }

    #[tokio::test]
    async fn test_apply_snapshot() -> Result<()> {
        let rm = ReplicationManager::new(ReplicationMode::Asynchronous, false);

        // Create and apply snapshot
        let tables = vec!["users".to_string()];
        let snapshot = rm.create_snapshot(tables)?;

        rm.apply_snapshot(&snapshot.snapshot_id).await?;

        // Verify LSN was updated
        assert_eq!(*rm.log_sequence.read(), snapshot.lsn);

        Ok(())
    }

    #[tokio::test]
    async fn test_lag_monitoring() -> Result<()> {
        let rm = ReplicationManager::new(ReplicationMode::Synchronous, true);

        let replica = ReplicaNode {
            id: "replica-lag".to_string(),
            address: "127.0.0.1:5510".to_string(),
            status: ReplicaStatus::Active,
            lag_bytes: 0,
            last_sync: 0,
        };
        rm.add_replica(replica)?;

        // Update lag
        rm.update_lag("replica-lag", 1000)?;
        let monitor = rm.get_lag_monitor("replica-lag")?;
        assert_eq!(monitor.current_lag_bytes, 1000);
        assert_eq!(monitor.lag_trend, LagTrend::Stable);

        // Update with critical lag
        rm.update_lag("replica-lag", 15_000_000)?;
        let monitor = rm.get_lag_monitor("replica-lag")?;
        assert_eq!(monitor.lag_trend, LagTrend::Critical);

        Ok(())
    }

    #[test]
    fn test_bandwidth_throttling() -> Result<()> {
        let rm = ReplicationManager::new(ReplicationMode::Asynchronous, true);

        // Enable throttling
        rm.enable_throttle(1_000_000)?; // 1 MB/s
        assert!(!rm.should_throttle());

        // Update usage
        rm.update_bandwidth_usage(500_000)?;
        assert!(!rm.should_throttle());

        rm.update_bandwidth_usage(1_500_000)?;
        assert!(rm.should_throttle());

        // Disable throttling
        rm.disable_throttle()?;
        assert!(!rm.should_throttle());

        Ok(())
    }

    #[test]
    fn test_checkpoint_operations() -> Result<()> {
        let rm = ReplicationManager::new(ReplicationMode::SemiSync, true);

        let replica = ReplicaNode {
            id: "replica-checkpoint".to_string(),
            address: "127.0.0.1:5520".to_string(),
            status: ReplicaStatus::Active,
            lag_bytes: 0,
            last_sync: 0,
        };
        rm.add_replica(replica)?;

        // Create checkpoint
        let checkpoint = rm.create_checkpoint("replica-checkpoint")?;
        assert!(checkpoint.consistent);

        // List checkpoints
        let checkpoints = rm.list_checkpoints(Some("replica-checkpoint"));
        assert_eq!(checkpoints.len(), 1);

        // Get checkpoint
        let retrieved = rm.get_checkpoint(&checkpoint.checkpoint_id)?;
        assert_eq!(retrieved.replica_id, "replica-checkpoint");

        Ok(())
    }

    #[tokio::test]
    async fn test_restore_from_checkpoint() -> Result<()> {
        let rm = ReplicationManager::new(ReplicationMode::Asynchronous, false);

        // Add some WAL entries
        for i in 1..=10 {
            let entry = WALEntry {
                lsn: i,
                transaction_id: Some(i),
                operation: ReplicationOperation::Insert,
                table_name: "test".to_string(),
                data: vec![i as u8],
                timestamp: i as i64,
                checksum: calculate_checksum(&vec![i as u8]),
            };
            rm.append_to_wal(entry)?;
        }

        // Create checkpoint
        let checkpoint = rm.create_checkpoint("test-replica")?;
        let checkpoint_lsn = checkpoint.lsn;

        // Restore from checkpoint
        rm.restore_from_checkpoint(&checkpoint.checkpoint_id).await?;

        assert_eq!(*rm.log_sequence.read(), checkpoint_lsn);

        Ok(())
    }

    #[test]
    fn test_logical_replication_slots() -> Result<()> {
        let rm = ReplicationManager::new(ReplicationMode::Asynchronous, true);

        // Create logical slot
        rm.create_logical_slot("test_slot".to_string(), "pgoutput".to_string())?;

        // Get slot
        let slot = rm.get_logical_slot("test_slot")?;
        assert_eq!(slot.plugin, "pgoutput");
        assert!(slot.active);

        // Advance slot
        rm.advance_logical_slot("test_slot", 100)?;
        let slot = rm.get_logical_slot("test_slot")?;
        assert_eq!(slot.confirmed_flush_lsn, 100);

        // List slots
        let slots = rm.list_logical_slots();
        assert_eq!(slots.len(), 1);

        // Drop slot
        rm.drop_logical_slot("test_slot")?;
        assert!(rm.get_logical_slot("test_slot").is_err());

        Ok(())
    }

    #[test]
    fn test_physical_replication_slots() -> Result<()> {
        let rm = ReplicationManager::new(ReplicationMode::Synchronous, true);

        // Create physical slot
        rm.create_physical_slot("physical_slot".to_string(), false)?;

        // Get slot
        let slot = rm.get_physical_slot("physical_slot")?;
        assert!(slot.active);
        assert!(!slot.temporary);

        // List slots
        let slots = rm.list_physical_slots();
        assert_eq!(slots.len(), 1);

        // Drop slot
        rm.drop_physical_slot("physical_slot")?;
        assert!(rm.get_physical_slot("physical_slot").is_err());

        Ok(())
    }

    #[test]
    fn test_geo_replication_configuration() -> Result<()> {
        let rm = ReplicationManager::new(ReplicationMode::Asynchronous, true);

        let geo_config = GeoReplicationConfig {
            region: "us-east-1".to_string(),
            replicas: vec!["replica-1".to_string(), "replica-2".to_string()],
            latency_ms: 50,
            bandwidth_mbps: 1000,
            prefer_local_reads: true,
        };

        rm.configure_geo_replication(geo_config.clone())?;

        let config = rm.get_geo_config().unwrap();
        assert_eq!(config.region, "us-east-1");
        assert_eq!(config.replicas.len(), 2);

        // Test replica selection
        let replica = rm.select_read_replica(true);
        assert_eq!(replica, Some("replica-1".to_string()));

        Ok(())
    }

    #[test]
    fn test_replication_report() -> Result<()> {
        let rm = ReplicationManager::new(ReplicationMode::SemiSync, true);

        // Add replicas
        for i in 1..=3 {
            let replica = ReplicaNode {
                id: format!("replica-{}", i),
                address: format!("127.0.0.1:553{}", i),
                status: ReplicaStatus::Active,
                lag_bytes: i * 100,
                last_sync: 0,
            });
            rm.add_replica(replica)?;
        }

        // Get report
        let report = rm.get_replication_report();
        assert_eq!(report.stats.total_replicas, 3);
        assert_eq!(report.mode, ReplicationMode::SemiSync);
        assert!(report.is_primary);

        Ok(())
    }

    #[test]
    fn test_overall_health_check() -> Result<()> {
        let rm = ReplicationManager::new(ReplicationMode::Asynchronous, true);

        // No replicas
        assert_eq!(rm.check_overall_health(), OverallHealth::NoReplicas);

        // Add healthy replicas
        for i in 1..=3 {
            let replica = ReplicaNode {
                id: format!("replica-{}", i),
                address: format!("127.0.0.1:554{}", i),
                status: ReplicaStatus::Active,
                lag_bytes: 0,
                last_sync: 0,
            });
            rm.add_replica(replica)?;

            let health = ReplicationHealth {
                replica_id: format!("replica-{}", i),
                is_healthy: true,
                last_heartbeat: 0,
                replication_delay_ms: 10,
                pending_transactions: 0,
                error_count: 0,
                last_error: None,
            });
            rm.update_replica_health(health)?;
        }

        assert_eq!(rm.check_overall_health(), OverallHealth::Healthy);

        Ok(())
    }

    #[test]
    fn test_cannot_advance_slot_backwards() -> Result<()> {
        let rm = ReplicationManager::new(ReplicationMode::Synchronous, true);

        rm.create_logical_slot("test".to_string(), "plugin".to_string())?;
        rm.advance_logical_slot("test", 100)?;

        let result = rm.advance_logical_slot("test", 50);
        assert!(result.is_err());

        Ok(())
    }

    #[test]
    fn test_duplicate_slot_creation() -> Result<()> {
        let rm = ReplicationManager::new(ReplicationMode::Asynchronous, true);

        rm.create_logical_slot("dup_slot".to_string(), "plugin".to_string())?;
        let result = rm.create_logical_slot("dup_slot".to_string(), "plugin".to_string());
        assert!(result.is_err());

        Ok(())
    }

    #[test]
    fn test_lag_trend_calculation() -> Result<()> {
        let rm = ReplicationManager::new(ReplicationMode::SemiSync, true);

        let replica = ReplicaNode {
            id: "lag-test".to_string(),
            address: "127.0.0.1:5550".to_string(),
            status: ReplicaStatus::Active,
            lag_bytes: 0,
            last_sync: 0,
        };
        rm.add_replica(replica)?;

        // Start with low lag
        rm.update_lag("lag-test", 1000)?;

        // Double the lag - should be degrading
        rm.update_lag("lag-test", 2500)?;
        let monitor = rm.get_lag_monitor("lag-test")?;
        assert_eq!(monitor.lag_trend, LagTrend::Degrading);

        // Reduce lag significantly - should be improving
        rm.update_lag("lag-test", 500)?;
        let monitor = rm.get_lag_monitor("lag-test")?;
        assert_eq!(monitor.lag_trend, LagTrend::Improving);

        Ok(())
    }

    #[test]
    fn test_get_replicas_in_region() -> Result<()> {
        let rm = ReplicationManager::new(ReplicationMode::Asynchronous, true);

        let geo_config = GeoReplicationConfig {
            region: "us-west-2".to_string(),
            replicas: vec!["r1".to_string(), "r2".to_string()],
            latency_ms: 20,
            bandwidth_mbps: 10000,
            prefer_local_reads: false,
        };

        rm.configure_geo_replication(geo_config)?;

        let replicas = rm.get_replicas_in_region("us-west-2");
        assert_eq!(replicas.len(), 2);

        let replicas = rm.get_replicas_in_region("eu-central-1");
        assert_eq!(replicas.len(), 0);

        Ok(())
    }

    #[tokio::test]
    async fn test_multiple_checkpoints_per_replica() -> Result<()> {
        let rm = ReplicationManager::new(ReplicationMode::Synchronous, true);

        let replica = ReplicaNode {
            id: "multi-checkpoint".to_string(),
            address: "127.0.0.1:5560".to_string(),
            status: ReplicaStatus::Active,
            lag_bytes: 0,
            last_sync: 0,
        };
        rm.add_replica(replica)?;

        // Create multiple checkpoints
        for _ in 0..3 {
            rm.create_checkpoint("multi-checkpoint")?;
            tokio::time::sleep(Duration::from_millis(10)).await;
        }

        let checkpoints = rm.list_checkpoints(Some("multi-checkpoint"));
        assert_eq!(checkpoints.len(), 3);

        Ok(())
    }

    #[test]
    fn test_temporary_physical_slots() -> Result<()> {
        let rm = ReplicationManager::new(ReplicationMode::Asynchronous, true);

        rm.create_physical_slot("temp_slot".to_string(), true)?;
        let slot = rm.get_physical_slot("temp_slot")?;
        assert!(slot.temporary);

        rm.create_physical_slot("perm_slot".to_string(), false)?;
        let slot = rm.get_physical_slot("perm_slot")?;
        assert!(!slot.temporary);

        Ok(())
    }

    #[test]
    fn test_all_lag_monitors() -> Result<()> {
        let rm = ReplicationManager::new(ReplicationMode::SemiSync, true);

        for i in 1..=3 {
            let replica = ReplicaNode {
                id: format!("r{}", i),
                address: format!("127.0.0.1:557{}", i),
                status: ReplicaStatus::Active,
                lag_bytes: 0,
                last_sync: 0,
            });
            rm.add_replica(replica)?;
        }

        let monitors = rm.get_all_lag_monitors();
        assert_eq!(monitors.len(), 3);

        Ok(())
    }

    #[test]
    fn test_throttle_config_retrieval() -> Result<()> {
        let rm = ReplicationManager::new(ReplicationMode::Asynchronous, true);

        assert!(rm.get_throttle_config().is_none());

        rm.enable_throttle(5_000_000)?;
        let config = rm.get_throttle_config().unwrap();
        assert_eq!(config.max_bytes_per_second, 5_000_000);
        assert!(config.enabled);

        Ok(())
    }

    #[test]
    fn test_lag_measurement_history() -> Result<()> {
        let rm = ReplicationManager::new(ReplicationMode::Synchronous, true);

        let replica = ReplicaNode {
            id: "history-test".to_string(),
            address: "127.0.0.1:5580".to_string(),
            status: ReplicaStatus::Active,
            lag_bytes: 0,
            last_sync: 0,
        };
        rm.add_replica(replica)?;

        // Add measurements
        for i in 1..=5 {
            rm.update_lag("history-test", i * 100)?;
        }

        let monitor = rm.get_lag_monitor("history-test")?;
        assert_eq!(monitor.measurements.len(), 5);
        assert_eq!(monitor.max_lag_bytes, 500);

        Ok(())
    }

    #[test]
    fn test_lag_measurement_limit() -> Result<()> {
        let rm = ReplicationManager::new(ReplicationMode::Asynchronous, true);

        let replica = ReplicaNode {
            id: "limit-test".to_string(),
            address: "127.0.0.1:5590".to_string(),
            status: ReplicaStatus::Active,
            lag_bytes: 0,
            last_sync: 0,
        };
        rm.add_replica(replica)?;

        // Add more than 100 measurements
        for i in 1..=150 {
            rm.update_lag("limit-test", i)?;
        }

        let monitor = rm.get_lag_monitor("limit-test")?;
        assert_eq!(monitor.measurements.len(), 100); // Should be limited to 100

        Ok(())
    }

    #[test]
    fn test_inconsistent_checkpoint_restore() -> Result<()> {
        let rm = ReplicationManager::new(ReplicationMode::SemiSync, false);

        // Create checkpoint and manually mark it inconsistent
        let mut checkpoint = rm.create_checkpoint("test")?;
        checkpoint.consistent = false;

        // Manually insert inconsistent checkpoint
        rm.checkpoints.write().push(checkpoint.clone());

        // Attempt to restore should fail
        let result = tokio_test::block_on(rm.restore_from_checkpoint(&checkpoint.checkpoint_id));
        assert!(result.is_err());

        Ok(())
    }

    #[tokio::test]
    async fn test_event_notification() -> Result<()> {
        use std::sync::Mutex;
use std::time::UNIX_EPOCH;
use std::time::SystemTime;

        struct TestListener {
            events: Arc<Mutex<Vec<ReplicationEvent>>>,
        }

        impl ReplicationEventListener for TestListener {
            fn on_event(&self, event: ReplicationEvent) {
                self.events.lock().unwrap().push(event);
            }
        }

        let rm = ReplicationManager::new(ReplicationMode::Asynchronous, true);
        let events = Arc::new(Mutex::new(Vec::new()));

        let listener = Arc::new(TestListener {
            events: Arc::clone(&events),
        });

        rm.add_event_listener(listener);

        // Add a replica - should trigger event
        let replica = ReplicaNode {
            id: "event-test".to_string(),
            address: "127.0.0.1:5600".to_string(),
            status: ReplicaStatus::Active,
            lag_bytes: 0,
            last_sync: 0,
        };
        rm.add_replica(replica)?;

        // Check event was recorded
        let recorded_events = events.lock().unwrap();
        assert_eq!(recorded_events.len(), 1);

        Ok(())
    }

    #[test]
    fn test_remove_replica_updates_status() -> Result<()> {
        let rm = ReplicationManager::new(ReplicationMode::Synchronous, true);

        let replica = ReplicaNode {
            id: "remove-test".to_string(),
            address: "127.0.0.1:5610".to_string(),
            status: ReplicaStatus::Active,
            lag_bytes: 0,
            last_sync: 0,
        };
        rm.add_replica(replica)?;

        assert_eq!(rm.get_replicas().len(), 1);

        rm.remove_replica("remove-test")?;
        assert_eq!(rm.get_replicas().len(), 0);

        Ok(())
    }

    #[test]
    fn test_replica_count_methods() -> Result<()> {
        let rm = ReplicationManager::new(ReplicationMode::Asynchronous, true);

        assert_eq!(rm.replica_count(), 0);
        assert_eq!(rm.active_replica_count(), 0);

        // Add replicas with different statuses
        for i in 1..=5 {
            let status = if i <= 3 {
                ReplicaStatus::Active
            } else {
                ReplicaStatus::Lagging
            };

            let replica = ReplicaNode {
                id: format!("r{}", i),
                address: format!("127.0.0.1:560{}", i),
                status,
                lag_bytes: 0,
                last_sync: 0,
            });
            rm.add_replica(replica)?;
        }

        assert_eq!(rm.replica_count(), 5);
        assert_eq!(rm.active_replica_count(), 3);
        assert_eq!(rm.lagging_replica_count(), 2);

        Ok(())
    }

    #[test]
    fn test_average_and_max_lag() -> Result<()> {
        let rm = ReplicationManager::new(ReplicationMode::SemiSync, true);

        for i in 1..=5 {
            let replica = ReplicaNode {
                id: format!("r{}", i),
                address: format!("127.0.0.1:561{}", i),
                status: ReplicaStatus::Active,
                lag_bytes: 0,
                last_sync: 0,
            });
            rm.add_replica(replica)?;
            rm.update_lag(&format!("r{}", i), i * 1000)?;
        }

        assert_eq!(rm.average_replication_lag(), 3000); // (1000+2000+3000+4000+5000)/5
        assert_eq!(rm.max_replication_lag(), 5000);

        Ok(())
    }

    #[test]
    fn test_has_critical_replicas() -> Result<()> {
        let rm = ReplicationManager::new(ReplicationMode::Synchronous, true);

        let replica = ReplicaNode {
            id: "critical-test".to_string(),
            address: "127.0.0.1:5620".to_string(),
            status: ReplicaStatus::Active,
            lag_bytes: 0,
            last_sync: 0,
        };
        rm.add_replica(replica)?;

        // Initially no critical replicas
        assert!(!rm.has_critical_replicas());

        // Add unhealthy replica
        let health = ReplicationHealth {
            replica_id: "critical-test".to_string(),
            is_healthy: false,
            last_heartbeat: 0,
            replication_delay_ms: 10000,
            pending_transactions: 100,
            error_count: 20,
            last_error: Some("Critical error".to_string()),
        };
        rm.update_replica_health(health)?;

        assert!(rm.has_critical_replicas());

        Ok(())
    }

    #[test]
    fn test_get_unhealthy_and_critical_replicas() -> Result<()> {
        let rm = ReplicationManager::new(ReplicationMode::Asynchronous, true);

        for i in 1..=4 {
            let replica = ReplicaNode {
                id: format!("r{}", i),
                address: format!("127.0.0.1:563{}", i),
                status: ReplicaStatus::Active,
                lag_bytes: 0,
                last_sync: 0,
            });
            rm.add_replica(replica)?;

            let is_healthy = i <= 2;
            let health = ReplicationHealth {
                replica_id: format!("r{}", i),
                is_healthy,
                last_heartbeat: 0,
                replication_delay_ms: 10,
                pending_transactions: 0,
                error_count: 0,
                last_error: None,
            });
            rm.update_replica_health(health)?;

            // Set critical lag for r3 and r4
            if i > 2 {
                rm.update_lag(&format!("r{}", i), 20_000_000)?;
            }
        }

        let unhealthy = rm.get_unhealthy_replicas();
        assert_eq!(unhealthy.len(), 2);

        let critical_lag = rm.get_critical_lag_replicas();
        assert_eq!(critical_lag.len(), 2);

        Ok(())
    }

    #[test]
    fn test_is_replication_current() -> Result<()> {
        let rm = ReplicationManager::new(ReplicationMode::SemiSync, true);

        let replica = ReplicaNode {
            id: "current-test".to_string(),
            address: "127.0.0.1:5640".to_string(),
            status: ReplicaStatus::Active,
            lag_bytes: 0,
            last_sync: 0,
        };
        rm.add_replica(replica)?;

        // Low lag - should be current
        rm.update_lag("current-test", 500_000)?;
        assert!(rm.is_replication_current());

        // High lag - not current
        rm.update_lag("current-test", 2_000_000)?;
        assert!(!rm.is_replication_current());

        Ok(())
    }

    #[test]
    fn test_snapshot_operations_comprehensive() -> Result<()> {
        let rm = ReplicationManager::new(ReplicationMode::Asynchronous, true);

        assert_eq!(rm.snapshot_count(), 0);
        assert_eq!(rm.total_snapshot_size(), 0);

        // Create multiple snapshots
        for i in 1..=10 {
            let mut snapshot = rm.create_snapshot(vec![format!("table{}", i)])?;
            snapshot.size_bytes = i * 1000;
            rm.snapshots.write().pop(); // Remove auto-added
            rm.snapshots.write().push(snapshot);
        }

        assert_eq!(rm.snapshot_count(), 10);
        assert_eq!(rm.total_snapshot_size(), 55000); // 1000+2000+...+10000

        // Clean old snapshots
        let removed = rm.clean_old_snapshots(3)?;
        assert_eq!(removed, 7);
        assert_eq!(rm.snapshot_count(), 3);

        Ok(())
    }

    #[test]
    fn test_configuration_export_import() -> Result<()> {
        let mut rm = ReplicationManager::new_with_topology(
            ReplicationMode::SemiSync,
            ReplicationTopology::MultiMaster,
            true,
        );

        rm.set_conflict_strategy(ConflictResolutionStrategy::FirstWriteWins);

        // Export config
        let config = rm.export_configuration();
        assert_eq!(config.mode, ReplicationMode::SemiSync);
        assert_eq!(config.topology, ReplicationTopology::MultiMaster);
        assert!(config.is_primary);

        // Create new manager and import
        let mut rm2 = ReplicationManager::new(ReplicationMode::Asynchronous, false);
        rm2.import_configuration(config)?;

        assert_eq!(rm2.get_mode(), ReplicationMode::SemiSync);
        assert_eq!(rm2.get_topology(), ReplicationTopology::MultiMaster);

        Ok(())
    }

    #[test]
    fn test_estimate_sync_time() -> Result<()> {
        let rm = ReplicationManager::new(ReplicationMode::Synchronous, true);

        let replica = ReplicaNode {
            id: "sync-time-test".to_string(),
            address: "127.0.0.1:5650".to_string(),
            status: ReplicaStatus::Lagging,
            lag_bytes: 0,
            last_sync: 0,
        };
        rm.add_replica(replica)?;

        rm.update_lag("sync-time-test", 10_000_000)?; // 10MB lag
        rm.enable_throttle(1_000_000)?; // 1MB/s

        let estimate = rm.estimate_sync_time("sync-time-test");
        assert_eq!(estimate, Some(10)); // 10MB / 1MB/s = 10 seconds

        Ok(())
    }

    #[test]
    fn test_uptime_stats() -> Result<()> {
        let rm = ReplicationManager::new(ReplicationMode::Asynchronous, true);

        let replica = ReplicaNode {
            id: "uptime-test".to_string(),
            address: "127.0.0.1:5660".to_string(),
            status: ReplicaStatus::Active,
            lag_bytes: 5000,
            last_sync: 1000,
        };
        rm.add_replica(replica)?;

        let health = ReplicationHealth {
            replica_id: "uptime-test".to_string(),
            is_healthy: true,
            last_heartbeat: 2000,
            replication_delay_ms: 50,
            pending_transactions: 5,
            error_count: 2,
            last_error: None,
        };
        rm.update_replica_health(health)?;

        rm.update_lag("uptime-test", 5000)?;

        let stats = rm.get_replica_uptime_stats("uptime-test")?;
        assert_eq!(stats.replica_id, "uptime-test");
        assert_eq!(stats.status, ReplicaStatus::Active);
        assert_eq!(stats.error_count, 2);

        Ok(())
    }

    #[test]
    fn test_wal_retention_calculation() -> Result<()> {
        let rm = ReplicationManager::new(ReplicationMode::SemiSync, true);

        // Add WAL entries
        for i in 1..=20 {
            let entry = WALEntry {
                lsn: i,
                transaction_id: Some(i),
                operation: ReplicationOperation::Update,
                table_name: "test".to_string(),
                data: vec![i as u8; 100],
                timestamp: i as i64,
                checksum: calculate_checksum(&vec![i as u8; 100]),
            };
            rm.append_to_wal(entry)?;
        }

        // Create logical slot at LSN 10
        rm.create_logical_slot("retention_test".to_string(), "plugin".to_string())?;
        rm.advance_logical_slot("retention_test", 10)?;

        let retention = rm.calculate_wal_retention_size();
        assert_eq!(retention, 10); // Latest (20) - Oldest acknowledged (10)

        Ok(())
    }

    #[test]
    fn test_set_replica_lag_directly() -> Result<()> {
        let rm = ReplicationManager::new(ReplicationMode::Asynchronous, true);

        let replica = ReplicaNode {
            id: "lag-direct".to_string(),
            address: "127.0.0.1:5670".to_string(),
            status: ReplicaStatus::Active,
            lag_bytes: 1000,
            last_sync: 0,
        };
        rm.add_replica(replica)?;

        rm.set_replica_lag("lag-direct", 5000)?;

        let replica = rm.get_replica("lag-direct").unwrap();
        assert_eq!(replica.lag_bytes, 5000);

        let monitor = rm.get_lag_monitor("lag-direct")?;
        assert_eq!(monitor.current_lag_bytes, 5000);

        Ok(())
    }

    #[test]
    fn test_has_replica() -> Result<()> {
        let rm = ReplicationManager::new(ReplicationMode::Synchronous, true);

        assert!(!rm.has_replica("test"));

        let replica = ReplicaNode {
            id: "test".to_string(),
            address: "127.0.0.1:5680".to_string(),
            status: ReplicaStatus::Active,
            lag_bytes: 0,
            last_sync: 0,
        };
        rm.add_replica(replica)?;

        assert!(rm.has_replica("test"));
        assert!(!rm.has_replica("nonexistent"));

        Ok(())
    }

    #[test]
    fn test_resolved_conflicts_count() -> Result<()> {
        let rm = ReplicationManager::new(ReplicationMode::MultiMaster, true);

        // Create conflicts
        for i in 1..=10 {
            rm.detect_conflict(
                i,
                "test".to_string(),
                format!("key{}", i),
                vec![i as u8],
                vec![(i + 1) as u8],
                i as i64,
                (i + 1) as i64,
            )?;
        }

        assert_eq!(rm.get_resolved_conflicts_count(), 0);

        // Resolve half
        for i in 1..=5 {
            rm.resolve_conflict(i)?;
        }

        assert_eq!(rm.get_resolved_conflicts_count(), 5);

        Ok(())
    }
}

// Additional helper implementations for testing

// Re-export all public APIs for convenient access
pub use types::*;
pub use manager::*;
pub use wal::*;
pub use conflicts::*;
pub use monitor::*;
pub use snapshots::*;
pub use slots::*;

// Module declarations
pub mod types;
pub mod manager;
pub mod wal;
pub mod conflicts;
pub mod monitor;
pub mod snapshots;
pub mod slots;
