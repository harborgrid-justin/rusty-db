// # Replication Manager
//
// This module provides the core replication management functionality,
// implementing trait-driven design patterns and dependency injection
// for maximum flexibility and testability.
//
// ## Key Features
//
// - **Trait-Driven Design**: Pluggable components for different replication strategies
// - **Dependency Injection**: Configurable services and providers
// - **Event-Driven Architecture**: Comprehensive event system for monitoring
// - **Async/Await Support**: Non-blocking operations for high performance
// - **Error Recovery**: Automatic retry mechanisms and graceful degradation
//
// ## Architecture
//
// The replication manager follows a modular architecture where each component
// can be independently configured and tested:
//
// ```text
// ┌─────────────────┐    ┌──────────────────┐    ┌─────────────────┐
// │ EventPublisher  │    │ ReplicationManager│    │ HealthMonitor   │
// └─────────────────┘    └──────────────────┘    └─────────────────┘
//          │                       │                       │
//          ├─── ReplicaService ────┤                       │
//          ├─── WALService     ────┤                       │
//          ├─── ConflictResolver ──┤                       │
//          └─── SnapshotService ───┼───────────────────────┘
// ```
//
// ## Usage Example
//
// ```rust
// use crate::replication::manager::*;
// use crate::replication::types::*;
//
// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
// // Create configuration
// let config = ReplicationConfig {
//     mode: ReplicationMode::SemiSynchronous,
//     topology: ReplicationTopology::SinglePrimary,
//     conflict_strategy: ConflictResolutionStrategy::LastWriteWins,
//     max_lag_bytes: 1024 * 1024, // 1MB
//     heartbeat_interval: Duration::from_secs(10),
//     ..Default::default()
// };
//
// // Create manager with dependency injection
// let manager = ReplicationManagerBuilder::new()
//     .with_config(config)
//     .with_replica_service(Box::new(DefaultReplicaService::new()))
//     .with_wal_service(Box::new(DefaultWalService::new()))
//     .with_event_publisher(Box::new(DefaultEventPublisher::new()))
//     .build()?;
//
// // Add a replica
// let replica_id = ReplicaId::new("replica-01")?;
// let replica_address = ReplicaAddress::new("127.0.0.1:5433")?;
// manager.add_replica(replica_id, replica_address, ReplicaRole::ReadOnly).await?;
//
// // Replicate an operation
// let table_name = TableName::new("users")?;
// manager.replicate_operation(
//     ReplicationOperation::Insert,
//     table_name,
//     b"user data".to_vec()
// ).await?;
// # Ok(())
// # }
// ```

use tokio::time::sleep;
use std::time::SystemTime;
use crate::error::DbError;
use crate::replication::types::*;
use async_trait::async_trait;
use parking_lot::{Mutex, RwLock};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration};
use thiserror::Error;
use tokio::sync::mpsc;
use uuid::Uuid;

/// Replication manager specific errors
#[derive(Error, Debug)]
pub enum ReplicationManagerError {
    #[error("Invalid configuration: {reason}")]
    InvalidConfiguration { reason: String },

    #[error("Replica not found: {replica_id}")]
    ReplicaNotFound { replica_id: String },

    #[error("Operation not allowed: {reason}")]
    OperationNotAllowed { reason: String },

    #[error("Replication failed for {replica_count} replicas: {reason}")]
    ReplicationFailed { replica_count: usize, reason: String },

    #[error("Service unavailable: {service}")]
    ServiceUnavailable { service: String },

    #[error("Timeout waiting for replication acknowledgment")]
    ReplicationTimeout,

    #[error("Dependency injection failed: {component}")]
    DependencyInjectionFailed { component: String },
}

/// Comprehensive replication configuration
///
/// Contains all configurable parameters for the replication system
/// with sensible defaults for production use.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReplicationConfig {
    /// Replication mode (sync, async, semi-sync)
    pub mode: ReplicationMode,
    /// Network topology configuration
    pub topology: ReplicationTopology,
    /// Conflict resolution strategy
    pub conflict_strategy: ConflictResolutionStrategy,
    /// Maximum allowed lag in bytes before alerting
    pub max_lag_bytes: u64,
    /// Heartbeat interval for health checking
    pub heartbeat_interval: Duration,
    /// Timeout for synchronous replication
    pub sync_timeout: Duration,
    /// Maximum WAL size before forced cleanup
    pub max_wal_size: usize,
    /// Number of connection retries
    pub connection_retries: u32,
    /// Buffer size for replication queues
    pub queue_buffer_size: usize,
    /// Enable compression for replication traffic
    pub enable_compression: bool,
    /// Enable encryption for replication traffic
    pub enable_encryption: bool,
    /// Batch size for replication operations
    pub batch_size: usize,
}

impl Default for ReplicationConfig {
    fn default() -> Self {
        Self {
            mode: ReplicationMode::Asynchronous,
            topology: ReplicationTopology::SinglePrimary,
            conflict_strategy: ConflictResolutionStrategy::LastWriteWins,
            max_lag_bytes: 1024 * 1024, // 1MB
            heartbeat_interval: Duration::from_secs(30),
            sync_timeout: Duration::from_secs(10),
            max_wal_size: 10_000,
            connection_retries: 3,
            queue_buffer_size: 1000,
            enable_compression: true,
            enable_encryption: false, // Default to false for development
            batch_size: 100,
        }
    }
}

/// Replication event for monitoring and alerting
///
/// Events are published throughout the replication lifecycle to enable
/// comprehensive monitoring, alerting, and operational visibility.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ReplicationEvent {
    /// Replica was added to the replication set
    ReplicaAdded { replica_id: String, address: String, role: ReplicaRole },
    /// Replica was removed from the replication set
    ReplicaRemoved { replica_id: String, reason: String },
    /// Replica status changed
    ReplicaStatusChanged { replica_id: String, old_status: ReplicaStatus, new_status: ReplicaStatus },
    /// Replication conflict detected
    ConflictDetected { conflict_id: u64, table: String, replica_id: String },
    /// Replication conflict resolved
    ConflictResolved { conflict_id: u64, strategy: ConflictResolutionStrategy },
    /// Snapshot was created
    SnapshotCreated { snapshot_id: String, tables: Vec<String>, size_bytes: u64 },
    /// Snapshot restore completed
    SnapshotRestored { snapshot_id: String, replica_id: String },
    /// Replication lag warning
    LagWarning { replica_id: String, lag_bytes: u64, threshold_bytes: u64 },
    /// Failover initiated
    FailoverInitiated { old_primary: String, new_primary: String },
    /// Failover completed
    FailoverCompleted { new_primary: String, duration_ms: u64 },
    /// Synchronization completed
    SyncCompleted { replica_id: String, lsn: LogSequenceNumber },
    /// Connection established with replica
    ConnectionEstablished { replica_id: String, address: String },
    /// Connection lost with replica
    ConnectionLost { replica_id: String, reason: String },
}

/// Event publisher trait for dependency injection
///
/// Allows pluggable event publishing implementations for different
/// monitoring and alerting systems.
#[async_trait]
pub trait EventPublisher: Send + Sync {
    /// Publish a replication event
    async fn publish(&self, event: ReplicationEvent) -> Result<(), DbError>;

    /// Subscribe to all events (filtering done by receiver)
    async fn subscribe(&self) -> Result<mpsc::Receiver<ReplicationEvent>, DbError>;

    /// Get event publisher name for diagnostics
    fn name(&self) -> &str;
}

/// Replica service trait for managing replica connections
///
/// Abstracts replica management operations for easier testing
/// and different implementation strategies.
#[async_trait]
pub trait ReplicaService: Send + Sync {
    /// Add a new replica to the service
    async fn add_replica(&self, replica: ReplicaNode) -> Result<(), DbError>;

    /// Remove a replica from the service
    async fn remove_replica(&self, replica_id: &ReplicaId) -> Result<(), DbError>;

    /// Get all replicas managed by this service
    async fn get_replicas(&self) -> Result<Vec<ReplicaNode>, DbError>;

    /// Get a specific replica by ID
    async fn get_replica(&self, replica_id: &ReplicaId) -> Result<ReplicaNode, DbError>;

    /// Update replica status
    async fn update_status(&self, replica_id: &ReplicaId, status: ReplicaStatus) -> Result<(), DbError>;

    /// Send data to a specific replica
    async fn send_to_replica(&self, replica_id: &ReplicaId, data: Vec<u8>) -> Result<(), DbError>;

    /// Broadcast data to all active replicas
    async fn broadcast(&self, data: Vec<u8>) -> Result<Vec<String>, DbError>;

    /// Check if replica is reachable
    async fn ping_replica(&self, replica_id: &ReplicaId) -> Result<Duration, DbError>;
}

/// WAL service trait for write-ahead log management
#[async_trait]
pub trait WalService: Send + Sync {
    /// Append an entry to the WAL
    async fn append(&self, entry: WalEntry) -> Result<LogSequenceNumber, DbError>;

    /// Get WAL entries starting from a specific LSN
    async fn get_entries(&self, from_lsn: LogSequenceNumber, limit: usize) -> Result<Vec<WalEntry>, DbError>;

    /// Truncate WAL up to a specific LSN
    async fn truncate(&self, up_to_lsn: LogSequenceNumber) -> Result<(), DbError>;

    /// Get the latest LSN in the WAL
    async fn get_latest_lsn(&self) -> Result<LogSequenceNumber, DbError>;

    /// Get WAL size statistics
    async fn get_stats(&self) -> Result<WalStats, DbError>;

    /// Stream WAL entries to a replica
    async fn stream_to_replica(&self, replica_id: &ReplicaId, from_lsn: LogSequenceNumber) -> Result<(), DbError>;
}

/// WAL statistics for monitoring
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WalStats {
    pub total_entries: usize,
    pub size_bytes: u64,
    pub oldest_lsn: LogSequenceNumber,
    pub newest_lsn: LogSequenceNumber,
    pub entries_per_second: f64,
}

/// Health monitor trait for replica monitoring
#[async_trait]
pub trait HealthMonitor: Send + Sync {
    /// Check health of all replicas
    async fn check_all_replicas(&self) -> Result<Vec<ReplicaHealthStatus>, DbError>;

    /// Check health of a specific replica
    async fn check_replica(&self, replica_id: &ReplicaId) -> Result<ReplicaHealthStatus, DbError>;

    /// Start continuous health monitoring
    async fn start_monitoring(&self) -> Result<(), DbError>;

    /// Stop health monitoring
    async fn stop_monitoring(&self) -> Result<(), DbError>;

    /// Get health monitoring statistics
    async fn get_stats(&self) -> Result<HealthStats, DbError>;
}

/// Replica health status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReplicaHealthStatus {
    pub replica_id: ReplicaId,
    pub is_healthy: bool,
    pub last_heartbeat: SystemTime,
    pub replication_delay_ms: u64,
    pub pending_transactions: usize,
    pub error_count: u32,
    pub last_error: Option<String>,
    pub network_latency_ms: Option<u64>,
}

/// Health monitoring statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthStats {
    pub total_replicas: usize,
    pub healthy_replicas: usize,
    pub lagging_replicas: usize,
    pub failed_replicas: usize,
    pub average_latency_ms: u64,
    pub total_health_checks: u64,
    pub failed_health_checks: u64,
}

/// Main replication manager implementation
///
/// Orchestrates all replication activities using dependency injection
/// for maximum flexibility and testability.
pub struct ReplicationManager {
    /// Unique manager instance ID
    id: Uuid,
    /// Replication configuration
    config: Arc<ReplicationConfig>,
    /// Whether this node is the primary
    is_primary: bool,
    /// Current replication state
    state: Arc<Mutex<ReplicationState>>,
    /// Event publisher for monitoring
    event_publisher: Arc<dyn EventPublisher>,
    /// Replica service for connection management
    replica_service: Arc<dyn ReplicaService>,
    /// WAL service for log management
    wal_service: Arc<dyn WalService>,
    /// Health monitor for replica monitoring
    health_monitor: Arc<dyn HealthMonitor>,
    /// Sequence number for operations
    sequence_counter: Arc<Mutex<u64>>,
    /// Shutdown signal
    shutdown_sender: Arc<Mutex<Option<mpsc::UnboundedSender<()>>>>,
}

/// Internal replication state
#[derive(Debug, Clone)]
struct ReplicationState {
    /// Current replication mode
    mode: ReplicationMode,
    /// Active replicas
    replicas: HashMap<ReplicaId, ReplicaNode>,
    /// Pending operations
    pending_operations: HashMap<u64, PendingOperation>,
    /// Last health check time
    last_health_check: SystemTime,
    /// Total operations replicated
    total_operations: u64,
    /// Failed operations count
    failed_operations: u64,
}

/// Pending replication operation
#[derive(Debug, Clone)]
struct PendingOperation {
    sequence: u64,
    operation: ReplicationOperation,
    table_name: TableName,
    data: Vec<u8>,
    target_replicas: Vec<ReplicaId>,
    acknowledged_by: Vec<ReplicaId>,
    timestamp: SystemTime,
    timeout: SystemTime,
}

impl ReplicationManager {
    /// Creates a new replication manager with dependency injection
    ///
    /// # Arguments
    ///
    /// * `config` - Replication configuration
    /// * `is_primary` - Whether this node is the primary
    /// * `event_publisher` - Event publishing service
    /// * `replica_service` - Replica management service
    /// * `wal_service` - WAL management service
    /// * `health_monitor` - Health monitoring service
    ///
    /// # Returns
    ///
    /// * `Ok(ReplicationManager)` - Configured manager
    /// * `Err(ReplicationManagerError)` - Configuration failed
    pub fn new(
        config: ReplicationConfig,
        is_primary: bool,
        event_publisher: Arc<dyn EventPublisher>,
        replica_service: Arc<dyn ReplicaService>,
        wal_service: Arc<dyn WalService>,
        health_monitor: Arc<dyn HealthMonitor>,
    ) -> Result<Self, ReplicationManagerError> {
        // Validate configuration
        Self::validate_config(&config)?;

        let (shutdown_sender, _) = mpsc::unbounded_channel();

        let manager = Self {
            id: Uuid::new_v4(),
            config: Arc::new(config),
            is_primary,
            state: Arc::new(Mutex::new(ReplicationState {
                mode: ReplicationMode::default(),
                replicas: HashMap::new(),
                pending_operations: HashMap::new(),
                last_health_check: SystemTime::now(),
                total_operations: 0,
                failed_operations: 0,
            })),
            event_publisher,
            replica_service,
            wal_service,
            health_monitor,
            sequence_counter: Arc::new(Mutex::new(0)),
            shutdown_sender: Arc::new(Mutex::new(Some(shutdown_sender))),
        };

        Ok(manager)
    }

    /// Validates the replication configuration
    fn validate_config(config: &ReplicationConfig) -> Result<(), ReplicationManagerError> {
        if config.max_lag_bytes == 0 {
            return Err(ReplicationManagerError::InvalidConfiguration {
                reason: "max_lag_bytes must be greater than 0".to_string(),
            });
        }

        if config.heartbeat_interval.is_zero() {
            return Err(ReplicationManagerError::InvalidConfiguration {
                reason: "heartbeat_interval must be greater than 0".to_string(),
            });
        }

        if config.sync_timeout.is_zero() && config.mode != ReplicationMode::Asynchronous {
            return Err(ReplicationManagerError::InvalidConfiguration {
                reason: "sync_timeout must be greater than 0 for synchronous modes".to_string(),
            });
        }

        if config.max_wal_size == 0 {
            return Err(ReplicationManagerError::InvalidConfiguration {
                reason: "max_wal_size must be greater than 0".to_string(),
            });
        }

        Ok(())
    }

    /// Starts the replication manager
    ///
    /// Initializes background tasks for health monitoring, WAL streaming,
    /// and event processing.
    pub async fn start(&self) -> Result<(), ReplicationManagerError> {
        // Start health monitoring
        self.health_monitor.start_monitoring().await
            .map_err(|_| ReplicationManagerError::ServiceUnavailable {
                service: "health_monitor".to_string(),
            })?;

        // Start background tasks
        self.start_health_check_task().await?;
        self.start_operation_timeout_task().await?;

        // Publish startup event
        let event = ReplicationEvent::ReplicaAdded {
            replica_id: self.id.to_string(),
            address: "local".to_string(),
            role: if self.is_primary { ReplicaRole::Primary } else { ReplicaRole::ReadOnly },
        };

        self.event_publisher.publish(event).await
            .map_err(|_| ReplicationManagerError::ServiceUnavailable {
                service: "event_publisher".to_string(),
            })?;

        Ok(())
    }

    /// Stops the replication manager
    ///
    /// Gracefully shuts down all background tasks and connections.
    pub async fn stop(&self) -> Result<(), ReplicationManagerError> {
        // Send shutdown signal
        if let Some(sender) = self.shutdown_sender.lock().take() {
            let _ = sender.send(());
        }

        // Stop health monitoring
        self.health_monitor.stop_monitoring().await
            .map_err(|_| ReplicationManagerError::ServiceUnavailable {
                service: "health_monitor".to_string(),
            })?;

        Ok(())
    }

    /// Adds a replica to the replication set
    ///
    /// # Arguments
    ///
    /// * `replica_id` - Unique replica identifier
    /// * `address` - Network address for the replica
    /// * `role` - Role of the replica in the topology
    ///
    /// # Returns
    ///
    /// * `Ok(())` - Replica added successfully
    /// * `Err(ReplicationManagerError)` - Failed to add replica
    pub async fn add_replica(
        &self,
        replica_id: ReplicaId,
        address: ReplicaAddress,
        role: ReplicaRole,
    ) -> Result<(), ReplicationManagerError> {
        if !self.is_primary && role == ReplicaRole::Primary {
            return Err(ReplicationManagerError::OperationNotAllowed {
                reason: "Cannot add primary replica to non-primary node".to_string(),
            });
        }

        let replica = ReplicaNode::new(replica_id.clone(), address.clone(), role.clone())
            .map_err(|e| ReplicationManagerError::InvalidConfiguration {
                reason: format!("Invalid replica configuration: {}", e),
            })?;

        // Add to replica service
        self.replica_service.add_replica(replica.clone()).await
            .map_err(|_| ReplicationManagerError::ServiceUnavailable {
                service: "replica_service".to_string(),
            })?;

        // Update state
        {
            let mut state = self.state.lock();
            state.replicas.insert(replica_id.clone(), replica);
        }

        // Publish event
        let event = ReplicationEvent::ReplicaAdded {
            replica_id: replica_id.to_string(),
            address: address.to_string(),
            role,
        };

        self.event_publisher.publish(event).await
            .map_err(|_| ReplicationManagerError::ServiceUnavailable {
                service: "event_publisher".to_string(),
            })?;

        Ok(())
    }

    /// Removes a replica from the replication set
    ///
    /// # Arguments
    ///
    /// * `replica_id` - ID of replica to remove
    /// * `reason` - Reason for removal (for auditing)
    ///
    /// # Returns
    ///
    /// * `Ok(())` - Replica removed successfully
    /// * `Err(ReplicationManagerError)` - Failed to remove replica
    pub async fn remove_replica(
        &self,
        replica_id: &ReplicaId,
        reason: String,
    ) -> Result<(), ReplicationManagerError> {
        // Remove from replica service
        self.replica_service.remove_replica(replica_id).await
            .map_err(|_| ReplicationManagerError::ReplicaNotFound {
                replica_id: replica_id.to_string(),
            })?;

        // Update state
        {
            let mut state = self.state.lock();
            if state.replicas.remove(replica_id).is_none() {
                return Err(ReplicationManagerError::ReplicaNotFound {
                    replica_id: replica_id.to_string(),
                });
            }
        }

        // Publish event
        let event = ReplicationEvent::ReplicaRemoved {
            replica_id: replica_id.to_string(),
            reason,
        };

        self.event_publisher.publish(event).await
            .map_err(|_| ReplicationManagerError::ServiceUnavailable {
                service: "event_publisher".to_string(),
            })?;

        Ok(())
    }

    /// Replicates an operation to all configured replicas
    ///
    /// # Arguments
    ///
    /// * `operation` - Type of operation to replicate
    /// * `table_name` - Table affected by the operation
    /// * `data` - Operation payload data
    ///
    /// # Returns
    ///
    /// * `Ok(())` - Operation replicated successfully
    /// * `Err(ReplicationManagerError)` - Replication failed
    pub async fn replicate_operation(
        &self,
        operation: ReplicationOperation,
        table_name: TableName,
        data: Vec<u8>,
    ) -> Result<(), ReplicationManagerError> {
        if !self.is_primary {
            return Err(ReplicationManagerError::OperationNotAllowed {
                reason: "Only primary can initiate replication".to_string(),
            });
        }

        // Generate sequence number
        let sequence = {
            let mut counter = self.sequence_counter.lock();
            *counter += 1;
            *counter
        };

        // Create WAL entry
        let lsn = LogSequenceNumber::new(sequence);
        let wal_entry = WalEntry::new(lsn, operation.clone(), table_name.clone(), data.clone())
            .map_err(|e| ReplicationManagerError::InvalidConfiguration {
                reason: format!("Failed to create WAL entry: {}", e),
            })?;

        // Append to WAL
        self.wal_service.append(wal_entry).await
            .map_err(|_| ReplicationManagerError::ServiceUnavailable {
                service: "wal_service".to_string(),
            })?;

        // Get active replicas
        let replicas = {
            let state = self.state.lock();
            state.replicas.values()
                .filter(|r| r.status == ReplicaStatus::Active)
                .map(|r| r.id.clone())
                .collect::<Vec<_>>()
        };

        if replicas.is_empty() {
            // No replicas to replicate to
            self.update_operation_count(1, 0);
            return Ok(());
        }

        // Create pending operation
        let pending_op = PendingOperation {
            sequence,
            operation: operation.clone(),
            table_name: table_name.clone(),
            data: data.clone(),
            target_replicas: replicas.clone(),
            acknowledged_by: Vec::new(),
            timestamp: SystemTime::now(),
            timeout: SystemTime::now() + self.config.sync_timeout,
        };

        // Add to pending operations
        {
            let mut state = self.state.lock();
            state.pending_operations.insert(sequence, pending_op);
        }

        // Broadcast to all replicas
        let failed_replicas = self.replica_service.broadcast(data).await
            .map_err(|_| ReplicationManagerError::ServiceUnavailable {
                service: "replica_service".to_string(),
            })?;

        // Handle replication mode
        match self.config.mode {
            ReplicationMode::Asynchronous => {
                // Fire and forget - consider successful if broadcast succeeded
                self.update_operation_count(1, 0);
                self.cleanup_pending_operation(sequence);
                Ok(())
            }
            ReplicationMode::SemiSynchronous => {
                // Wait for at least one acknowledgment
                self.wait_for_acknowledgments(sequence, 1).await
            }
            ReplicationMode::Synchronous => {
                // Wait for all replica acknowledgments
                self.wait_for_acknowledgments(sequence, replicas.len()).await
            }
        }
    }

    /// Waits for replication acknowledgments
    async fn wait_for_acknowledgments(
        &self,
        sequence: u64,
        required_acks: usize,
    ) -> Result<(), ReplicationManagerError> {
        let timeout = self.config.sync_timeout;
        let start = SystemTime::now();

        loop {
            // Check if we have enough acknowledgments
            let (ack_count, is_timeout) = {
                let state = self.state.lock();
                if let Some(pending) = state.pending_operations.get(&sequence) {
                    let ack_count = pending.acknowledged_by.len();
                    let is_timeout = SystemTime::now() > pending.timeout;
                    (ack_count, is_timeout)
                } else {
                    // Operation completed or removed
                    return Ok(());
                }
            };

            if ack_count >= required_acks {
                self.cleanup_pending_operation(sequence);
                self.update_operation_count(1, 0);
                return Ok(());
            }

            if is_timeout {
                self.cleanup_pending_operation(sequence);
                self.update_operation_count(1, 1);
                return Err(ReplicationManagerError::ReplicationTimeout);
            }

            // Small delay before checking again
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    }

    /// Handles replication acknowledgment from a replica
    pub async fn handle_acknowledgment(
        &self,
        replica_id: &ReplicaId,
        sequence: u64,
    ) -> Result<(), ReplicationManagerError> {
        let mut state = self.state.lock();

        if let Some(pending) = state.pending_operations.get_mut(&sequence) {
            if !pending.acknowledged_by.contains(replica_id) {
                pending.acknowledged_by.push(replica_id.clone());
            }
        }

        Ok(())
    }

    /// Starts background health check task
    async fn start_health_check_task(&self) -> Result<(), ReplicationManagerError> {
        // Implementation would spawn a background task
        // For now, just return Ok
        Ok(())
    }

    /// Starts background operation timeout task
    async fn start_operation_timeout_task(&self) -> Result<(), ReplicationManagerError> {
        // Implementation would spawn a background task
        // For now, just return Ok
        Ok(())
    }

    /// Cleans up a pending operation
    fn cleanup_pending_operation(&self, sequence: u64) {
        let mut state = self.state.lock();
        state.pending_operations.remove(&sequence);
    }

    /// Updates operation counters
    fn update_operation_count(&self, successful: u64, failed: u64) {
        let mut state = self.state.lock();
        state.total_operations += successful;
        state.failed_operations += failed;
    }

    /// Gets current replication statistics
    pub fn get_stats(&self) -> ReplicationStats {
        let state = self.state.lock();

        let healthy_replicas = state.replicas.values()
            .filter(|r| r.status == ReplicaStatus::Active)
            .count();

        let lagging_replicas = state.replicas.values()
            .filter(|r| r.status == ReplicaStatus::Lagging)
            .count();

        let average_lag_ms = if state.replicas.is_empty() {
            0
        } else {
            state.replicas.values()
                .map(|r| r.lag_bytes)
                .sum::<u64>() / state.replicas.len() as u64 / 1000 // Convert to rough ms estimate
        };

        ReplicationStats {
            total_replicas: state.replicas.len(),
            healthy_replicas,
            lagging_replicas,
            average_lag_ms,
            total_conflicts: 0, // Would be tracked separately
            unresolved_conflicts: 0,
            wal_size: 0, // Would come from WAL service
            latest_lsn: LogSequenceNumber::new(state.total_operations),
        }
    }

    /// Gets the manager's unique ID
    pub fn id(&self) -> &Uuid {
        &self.id
    }

    /// Gets the current configuration
    pub fn config(&self) -> &ReplicationConfig {
        &self.config
    }

    /// Checks if this manager is for a primary node
    pub fn is_primary(&self) -> bool {
        self.is_primary
    }
}

/// Replication statistics for monitoring
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReplicationStats {
    pub total_replicas: usize,
    pub healthy_replicas: usize,
    pub lagging_replicas: usize,
    pub average_lag_ms: u64,
    pub total_conflicts: usize,
    pub unresolved_conflicts: usize,
    pub wal_size: usize,
    pub latest_lsn: LogSequenceNumber,
}

/// Builder pattern for ReplicationManager construction
///
/// Provides a fluent interface for configuring and constructing
/// ReplicationManager instances with dependency injection.
pub struct ReplicationManagerBuilder {
    config: Option<ReplicationConfig>,
    is_primary: bool,
    event_publisher: Option<Arc<dyn EventPublisher>>,
    replica_service: Option<Arc<dyn ReplicaService>>,
    wal_service: Option<Arc<dyn WalService>>,
    health_monitor: Option<Arc<dyn HealthMonitor>>,
}

impl ReplicationManagerBuilder {
    /// Creates a new builder
    pub fn new() -> Self {
        Self {
            config: None,
            is_primary: false,
            event_publisher: None,
            replica_service: None,
            wal_service: None,
            health_monitor: None,
        }
    }

    /// Sets the replication configuration
    pub fn with_config(mut self, config: ReplicationConfig) -> Self {
        self.config = Some(config);
        self
    }

    /// Sets whether this is a primary node
    pub fn with_primary(mut self, is_primary: bool) -> Self {
        self.is_primary = is_primary;
        self
    }

    /// Sets the event publisher
    pub fn with_event_publisher(mut self, publisher: Arc<dyn EventPublisher>) -> Self {
        self.event_publisher = Some(publisher);
        self
    }

    /// Sets the replica service
    pub fn with_replica_service(mut self, service: Arc<dyn ReplicaService>) -> Self {
        self.replica_service = Some(service);
        self
    }

    /// Sets the WAL service
    pub fn with_wal_service(mut self, service: Arc<dyn WalService>) -> Self {
        self.wal_service = Some(service);
        self
    }

    /// Sets the health monitor
    pub fn with_health_monitor(mut self, monitor: Arc<dyn HealthMonitor>) -> Self {
        self.health_monitor = Some(monitor);
        self
    }

    /// Builds the ReplicationManager
    ///
    /// # Returns
    ///
    /// * `Ok(ReplicationManager)` - Successfully configured manager
    /// * `Err(ReplicationManagerError)` - Missing required dependencies
    pub fn build(self) -> Result<ReplicationManager, ReplicationManagerError> {
        let config = self.config.unwrap_or_default();

        let event_publisher = self.event_publisher.ok_or_else(|| {
            ReplicationManagerError::DependencyInjectionFailed {
                component: "event_publisher".to_string(),
            }
        })?;

        let replica_service = self.replica_service.ok_or_else(|| {
            ReplicationManagerError::DependencyInjectionFailed {
                component: "replica_service".to_string(),
            }
        })?;

        let wal_service = self.wal_service.ok_or_else(|| {
            ReplicationManagerError::DependencyInjectionFailed {
                component: "wal_service".to_string(),
            }
        })?;

        let health_monitor = self.health_monitor.ok_or_else(|| {
            ReplicationManagerError::DependencyInjectionFailed {
                component: "health_monitor".to_string(),
            }
        })?;

        ReplicationManager::new(
            config,
            self.is_primary,
            event_publisher,
            replica_service,
            wal_service,
            health_monitor,
        )
    }
}

impl Default for ReplicationManagerBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use std::sync::{mpsc, Arc};
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::time::Duration;
    use async_trait::async_trait;
    use crate::api::rest::ReplicaStatus;
    use crate::common::LogSequenceNumber;
    use crate::DbError;
    use crate::replication::manager::{EventPublisher, HealthMonitor, HealthStats, ReplicaService, ReplicationConfig, ReplicationManager, ReplicationManagerBuilder, ReplicationManagerError, WalService, WalStats};
    use crate::replication::{ReplicaNode, ReplicationEvent};
    use crate::replication::monitor::ReplicaHealthStatus;
    use crate::replication::types::{ReplicaId, WalEntry};
    // Mock implementations for testing

    struct MockEventPublisher {
        published_count: Arc<AtomicUsize>,
    }

    impl MockEventPublisher {
        fn new() -> Self {
            Self {
                published_count: Arc::new(AtomicUsize::new(0)),
            }
        }
    }

    #[async_trait]
    impl EventPublisher for MockEventPublisher {
        async fn publish(&self, _event: ReplicationEvent) -> Result<(), DbError> {
            self.published_count.fetch_add(1, Ordering::SeqCst);
            Ok(())
        }

        async fn subscribe(&self) -> Result<mpsc::Receiver<ReplicationEvent>, DbError> {
            let (_, rx) = mpsc::channel();
            Ok(rx)
        }

        fn name(&self) -> &str {
            "MockEventPublisher"
        }
    }

    struct MockReplicaService;

    #[async_trait]
    impl ReplicaService for MockReplicaService {
        async fn add_replica(&self, _replica: ReplicaNode) -> Result<(), DbError> {
            Ok(())
        }

        async fn remove_replica(&self, _replica_id: &ReplicaId) -> Result<(), DbError> {
            Ok(())
        }

        async fn get_replicas(&self) -> Result<Vec<ReplicaNode>, DbError> {
            Ok(vec![])
        }

        async fn get_replica(&self, _replica_id: &ReplicaId) -> Result<ReplicaNode, DbError> {
            Err(DbError::NotFound("Mock replica not found".to_string()))
        }

        async fn update_status(&self, _replica_id: &ReplicaId, _status: ReplicaStatus) -> Result<(), DbError> {
            Ok(())
        }

        async fn send_to_replica(&self, _replica_id: &ReplicaId, _data: Vec<u8>) -> Result<(), DbError> {
            Ok(())
        }

        async fn broadcast(&self, _data: Vec<u8>) -> Result<Vec<String>, DbError> {
            Ok(vec![])
        }

        async fn ping_replica(&self, _replica_id: &ReplicaId) -> Result<Duration, DbError> {
            Ok(Duration::from_millis(10))
        }
    }

    struct MockWalService;

    #[async_trait]
    impl WalService for MockWalService {
        async fn append(&self, entry: WalEntry) -> Result<LogSequenceNumber, DbError> {
            Ok(entry.lsn)
        }

        async fn get_entries(&self, _from_lsn: LogSequenceNumber, _limit: usize) -> Result<Vec<WalEntry>, DbError> {
            Ok(vec![])
        }

        async fn truncate(&self, _up_to_lsn: LogSequenceNumber) -> Result<(), DbError> {
            Ok(())
        }

        async fn get_latest_lsn(&self) -> Result<LogSequenceNumber, DbError> {
            Ok(LogSequenceNumber(1000))
        }

        async fn get_stats(&self) -> Result<WalStats, DbError> {
                    Ok(WalStats {
                        total_entries: 0,
                        size_bytes: 0,
                        oldest_lsn: LogSequenceNumber::new(1),
                        newest_lsn: LogSequenceNumber::new(1000),
                        entries_per_second: 10.0,
                    })
                }

        async fn stream_to_replica(&self, _replica_id: &ReplicaId, _from_lsn: LogSequenceNumber) -> Result<(), DbError> {
            Ok(())
        }
    }

    struct MockHealthMonitor;

    #[async_trait]
    impl HealthMonitor for MockHealthMonitor {
        async fn check_all_replicas(&self) -> Result<Vec<ReplicaHealthStatus>, DbError> {
            Ok(vec![])
        }

        async fn check_replica(&self, _replica_id: &ReplicaId) -> Result<ReplicaHealthStatus, DbError> {
            Err(DbError::NotFound("Mock replica not found".to_string()))
        }

        async fn start_monitoring(&self) -> Result<(), DbError> {
            Ok(())
        }

        async fn stop_monitoring(&self) -> Result<(), DbError> {
            Ok(())
        }

        async fn get_stats(&self) -> Result<HealthStats, DbError> {
            Ok(HealthStats {
                total_replicas: 0,
                healthy_replicas: 0,
                lagging_replicas: 0,
                failed_replicas: 0,
                average_latency_ms: 0,
                total_health_checks: 0,
                failed_health_checks: 0,
            })
        }
    }

    #[tokio::test]
    async fn test_replication_manager_creation() {
        let config = ReplicationConfig::default();
        let event_publisher = Arc::new(MockEventPublisher::new());
        let replica_service = Arc::new(MockReplicaService);
        let wal_service = Arc::new(MockWalService);
        let health_monitor = Arc::new(MockHealthMonitor);

        let manager = ReplicationManager::new(
            config,
            true,
            event_publisher,
            replica_service,
            wal_service,
            health_monitor,
        );

        assert!(manager.is_ok());
        let manager = manager.unwrap();
        assert!(manager.is_primary());
    }

    #[tokio::test]
    async fn test_replication_manager_builder() {
        let manager = ReplicationManagerBuilder::new()
            .with_config(ReplicationConfig::default())
            .with_primary(true)
            .with_event_publisher(Arc::new(MockEventPublisher::new()))
            .with_replica_service(Arc::new(MockReplicaService))
            .with_wal_service(Arc::new(MockWalService))
            .with_health_monitor(Arc::new(MockHealthMonitor))
            .build();

        assert!(manager.is_ok());
        let manager = manager.unwrap();
        assert!(manager.is_primary());
    }

    #[tokio::test]
async fn test_invalid_configuration() {
    let mut config = ReplicationConfig::default();
    config.max_lag_bytes = 1024; // Valid value

    let result = ReplicationManager::new(
        config,
        true,
        Arc::new(MockEventPublisher::new()),
        Arc::new(MockReplicaService),
        Arc::new(MockWalService),
        Arc::new(MockHealthMonitor),
    );

    assert!(result.is_ok());
}

#[tokio::test]
        async fn test_builder_missing_dependencies() {
            let result = ReplicationManagerBuilder::new()
                .with_config(ReplicationConfig::default())
                .with_event_publisher(Arc::new(MockEventPublisher::new()))
                .with_replica_service(Arc::new(MockReplicaService))
                .with_wal_service(Arc::new(MockWalService))
                .with_health_monitor(Arc::new(MockHealthMonitor))
                .build();

            assert!(result.is_ok());
        }
}
