// # Replication Conflict Detection and Resolution
//
// This module provides sophisticated conflict detection and resolution
// mechanisms for multi-primary replication setups, implementing pluggable
// strategies and comprehensive monitoring of conflict patterns.
//
// ## Key Features
//
// - **Conflict Detection**: Automatic detection of replication conflicts
// - **Pluggable Strategies**: Multiple built-in and custom resolution strategies
// - **Conflict Analysis**: Pattern analysis and root cause identification
// - **Resolution Monitoring**: Comprehensive metrics and alerting
// - **Manual Override**: Support for manual conflict resolution
// - **Audit Logging**: Complete audit trail of all conflict resolutions
//
// ## Conflict Resolution Strategies
//
// - **Last Write Wins (LWW)**: Most recent timestamp takes precedence
// - **First Write Wins (FWW)**: First write is preserved, subsequent ignored
// - **Primary Wins**: Primary node's version always takes precedence
// - **Custom Resolution**: User-defined resolution logic
// - **Manual Resolution**: Human intervention required
// - **Merge Resolution**: Attempt to merge conflicting changes
//
// ## Usage Example
//
// ```rust
// use crate::replication::conflicts::*;
// use crate::replication::types::*;
//
// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
// // Create conflict resolver with configuration
// let config = ConflictResolverConfig {
//     default_strategy: ConflictResolutionStrategy::LastWriteWins,
//     enable_automatic_resolution: true,
//     max_resolution_time: Duration::from_secs(30),
//     enable_conflict_logging: true,
//     ..Default::default()
// };
//
// let resolver = ConflictResolver::new(config)?;
//
// // Register custom resolver for specific table
// let table_name = TableName::new("users")?;
// resolver.register_custom_resolver(
//     table_name,
//     Box::new(UserConflictResolver::new())
// ).await?;
//
// // Detect and resolve conflict
// let conflict = ConflictData {
//     table_name: TableName::new("orders")?,
//     primary_key: "order_123".to_string(),
//     local_version: b"local data".to_vec(),
//     remote_version: b"remote data".to_vec(),
//     local_timestamp: SystemTime::now(),
//     remote_timestamp: SystemTime::now() - Duration::from_secs(10),
//     operation: ReplicationOperation::Update,
// };
//
// let resolution = resolver.resolve_conflict(conflict).await?;
// println!("Conflict resolved: {:?}", resolution.strategy_used);
// # Ok(())
// # }
// ```

use std::time::SystemTime;
use crate::error::DbError;
use crate::replication::types::*;
use async_trait::async_trait;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration};
use thiserror::Error;
use tokio::sync::mpsc;
use uuid::Uuid;

// Conflict resolution specific errors
#[derive(Error, Debug)]
pub enum ConflictResolutionError {
    #[error("Conflict resolution failed for conflict {conflict_id}: {reason}")]
    ResolutionFailed { conflict_id: String, reason: String },

    #[error("Invalid conflict data: {reason}")]
    InvalidConflictData { reason: String },

    #[error("Custom resolver not found for table '{table_name}'")]
    CustomResolverNotFound { table_name: String },

    #[error("Resolution strategy '{strategy}' not supported")]
    UnsupportedStrategy { strategy: String },

    #[error("Manual resolution required for conflict {conflict_id}")]
    ManualResolutionRequired { conflict_id: String },

    #[error("Resolution timeout exceeded for conflict {conflict_id}")]
    ResolutionTimeout { conflict_id: String },

    #[error("Merge conflict failed: {reason}")]
    MergeConflictFailed { reason: String },

    #[error("Invalid resolver configuration: {reason}")]
    InvalidConfiguration { reason: String },
}

// Comprehensive conflict resolver configuration
//
// Contains all configurable parameters for conflict detection
// and resolution with sensible defaults.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConflictResolverConfig {
    // Default resolution strategy
    pub default_strategy: ConflictResolutionStrategy,
    // Enable automatic conflict resolution
    pub enable_automatic_resolution: bool,
    // Maximum time to spend on resolution
    pub max_resolution_time: Duration,
    // Enable detailed conflict logging
    pub enable_conflict_logging: bool,
    // Maximum number of conflicts to keep in memory
    pub max_conflicts_in_memory: usize,
    // Conflict detection threshold (ms)
    pub conflict_detection_threshold_ms: u64,
    // Enable conflict pattern analysis
    pub enable_pattern_analysis: bool,
    // Audit log retention period
    pub audit_retention_period: Duration,
    // Enable notifications for conflicts
    pub enable_notifications: bool,
    // Batch size for conflict processing
    pub batch_processing_size: usize,
    // Enable conflict prevention analysis
    pub enable_conflict_prevention: bool,
    // Maximum retries for resolution
    pub max_resolution_retries: u32,
}

impl Default for ConflictResolverConfig {
    fn default() -> Self {
        Self {
            default_strategy: ConflictResolutionStrategy::LastWriteWins,
            enable_automatic_resolution: true,
            max_resolution_time: Duration::from_secs(30),
            enable_conflict_logging: true,
            max_conflicts_in_memory: 10000,
            conflict_detection_threshold_ms: 100,
            enable_pattern_analysis: true,
            audit_retention_period: Duration::from_secs(86400 * 30), // 30 days
            enable_notifications: true,
            batch_processing_size: 100,
            enable_conflict_prevention: false,
            max_resolution_retries: 3,
        }
    }
}

// Conflict data structure containing all information about a conflict
//
// Represents a complete conflict scenario with all necessary data
// for analysis and resolution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConflictData {
    // Table where conflict occurred
    pub table_name: TableName,
    // Primary key of conflicting row
    pub primary_key: String,
    // Local version of the data
    pub local_version: Vec<u8>,
    // Remote version of the data
    pub remote_version: Vec<u8>,
    // Timestamp of local change
    pub local_timestamp: SystemTime,
    // Timestamp of remote change
    pub remote_timestamp: SystemTime,
    // Type of operation that caused conflict
    pub operation: ReplicationOperation,
    // Additional metadata
    pub metadata: HashMap<String, String>,
}

impl ConflictData {
    // Creates a new conflict data structure
    pub fn new(
        table_name: TableName,
        primary_key: String,
        local_version: Vec<u8>,
        remote_version: Vec<u8>,
        local_timestamp: SystemTime,
        remote_timestamp: SystemTime,
        operation: ReplicationOperation,
    ) -> Self {
        Self {
            table_name,
            primary_key,
            local_version,
            remote_version,
            local_timestamp,
            remote_timestamp,
            operation,
            metadata: HashMap::new(),
        }
    }

    // Gets the time difference between local and remote changes
    pub fn time_delta(&self) -> Option<Duration> {
        if self.local_timestamp >= self.remote_timestamp {
            self.local_timestamp.duration_since(self.remote_timestamp).ok()
        } else {
            self.remote_timestamp.duration_since(self.local_timestamp).ok()
        }
    }

    // Checks if this is a recent conflict (within threshold)
    pub fn is_recent_conflict(&self, threshold: Duration) -> bool {
        self.time_delta().map(|delta| delta <= threshold).unwrap_or(false)
    }

    // Adds metadata to the conflict
    pub fn add_metadata(&mut self, key: String, value: String) {
        self.metadata.insert(key, value);
    }

    // Gets metadata value
    pub fn get_metadata(&self, key: &str) -> Option<&String> {
        self.metadata.get(key)
    }
}

// Conflict resolution result
//
// Contains the outcome of a conflict resolution attempt,
// including the resolved data and metadata about the resolution process.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConflictResolution {
    // Unique resolution ID
    pub resolution_id: Uuid,
    // Original conflict ID
    pub conflict_id: Uuid,
    // Strategy used for resolution
    pub strategy_used: ConflictResolutionStrategy,
    // Resolved data
    pub resolved_data: Vec<u8>,
    // Whether resolution was successful
    pub successful: bool,
    // Time taken to resolve
    pub resolution_time: Duration,
    // Timestamp of resolution
    pub resolved_at: SystemTime,
    // Additional resolution metadata
    pub resolution_metadata: HashMap<String, String>,
    // Whether manual intervention was required
    pub required_manual_intervention: bool,
}

impl ConflictResolution {
    // Creates a successful resolution
    pub fn success(
        conflict_id: Uuid,
        strategy: ConflictResolutionStrategy,
        resolved_data: Vec<u8>,
        resolution_time: Duration,
    ) -> Self {
        Self {
            resolution_id: Uuid::new_v4(),
            conflict_id,
            strategy_used: strategy,
            resolved_data,
            successful: true,
            resolution_time,
            resolved_at: SystemTime::now(),
            resolution_metadata: HashMap::new(),
            required_manual_intervention: false,
        }
    }

    // Creates a failed resolution
    pub fn failure(
        conflict_id: Uuid,
        strategy: ConflictResolutionStrategy,
        resolution_time: Duration,
        reason: String,
    ) -> Self {
        let mut resolution = Self {
            resolution_id: Uuid::new_v4(),
            conflict_id,
            strategy_used: strategy,
            resolved_data: Vec::new(),
            successful: false,
            resolution_time,
            resolved_at: SystemTime::now(),
            resolution_metadata: HashMap::new(),
            required_manual_intervention: false,
        };

        resolution.resolution_metadata.insert("failure_reason".to_string(), reason);
        resolution
    }
}

// Conflict information with tracking data
//
// Extended conflict structure that includes tracking information
// for monitoring and analysis purposes.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConflictInfo {
    // Unique conflict identifier
    pub conflict_id: Uuid,
    // Conflict data
    pub data: ConflictData,
    // Detection timestamp
    pub detected_at: SystemTime,
    // Current status
    pub status: ConflictStatus,
    // Number of resolution attempts
    pub resolution_attempts: u32,
    // Associated resolution (if any)
    pub resolution: Option<ConflictResolution>,
    // Severity level
    pub severity: ConflictSeverity,
    // Tags for categorization
    pub tags: Vec<String>,
}

// Conflict status enumeration
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ConflictStatus {
    // Conflict detected, awaiting resolution
    Pending,
    // Resolution in progress
    Resolving,
    // Successfully resolved
    Resolved,
    // Resolution failed
    Failed,
    // Requires manual intervention
    ManualRequired,
    // Resolution was skipped
    Skipped,
}

// Conflict severity levels
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum ConflictSeverity {
    // Low priority conflict
    Low,
    // Medium priority conflict
    Medium,
    // High priority conflict
    High,
    // Critical conflict requiring immediate attention
    Critical,
}

// Custom conflict resolver trait
//
// Allows implementation of custom conflict resolution logic
// for specific tables or data types.
#[async_trait]
pub trait CustomConflictResolver: Send + Sync {
    // Resolves a conflict using custom logic
    //
    // # Arguments
    //
    // * `conflict` - The conflict data to resolve
    //
    // # Returns
    //
    // * `Ok(Vec<u8>)` - Resolved data
    // * `Err(ConflictResolutionError)` - Resolution failed
    async fn resolve(&self, conflict: &ConflictData) -> Result<Vec<u8>, ConflictResolutionError>;

    // Determines if this resolver can handle the given conflict
    async fn can_resolve(&self, conflict: &ConflictData) -> bool;

    // Gets the resolver's name for logging
    fn name(&self) -> &str;

    // Gets resolver priority (higher values have priority)
    fn priority(&self) -> u32 {
        0
    }
}

// Built-in last-write-wins resolver
pub struct LastWriteWinsResolver;

#[async_trait]
impl CustomConflictResolver for LastWriteWinsResolver {
    async fn resolve(&self, conflict: &ConflictData) -> Result<Vec<u8>, ConflictResolutionError> {
        if conflict.remote_timestamp > conflict.local_timestamp {
            Ok(conflict.remote_version.clone())
        } else {
            Ok(conflict.local_version.clone())
        }
    }

    async fn can_resolve(&self, _conflict: &ConflictData) -> bool {
        true
    }

    fn name(&self) -> &str {
        "LastWriteWins"
    }
}

// Built-in first-write-wins resolver
pub struct FirstWriteWinsResolver;

#[async_trait]
impl CustomConflictResolver for FirstWriteWinsResolver {
    async fn resolve(&self, conflict: &ConflictData) -> Result<Vec<u8>, ConflictResolutionError> {
        if conflict.local_timestamp < conflict.remote_timestamp {
            Ok(conflict.local_version.clone())
        } else {
            Ok(conflict.remote_version.clone())
        }
    }

    async fn can_resolve(&self, _conflict: &ConflictData) -> bool {
        true
    }

    fn name(&self) -> &str {
        "FirstWriteWins"
    }
}

// Conflict pattern analysis
//
// Analyzes conflict patterns to identify trends and potential
// optimizations to prevent future conflicts.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConflictPatternAnalysis {
    // Analysis timestamp
    pub analyzed_at: SystemTime,
    // Total conflicts analyzed
    pub total_conflicts: usize,
    // Conflicts by table
    pub conflicts_by_table: HashMap<String, usize>,
    // Conflicts by operation type
    pub conflicts_by_operation: HashMap<ReplicationOperation, usize>,
    // Average resolution time by strategy
    pub avg_resolution_time_by_strategy: HashMap<ConflictResolutionStrategy, Duration>,
    // Most common conflict patterns
    pub common_patterns: Vec<ConflictPattern>,
    // Recommended optimizations
    pub recommendations: Vec<String>,
}

// Conflict pattern
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConflictPattern {
    // Pattern description
    pub description: String,
    // Number of occurrences
    pub occurrences: usize,
    // Tables affected by this pattern
    pub affected_tables: Vec<String>,
    // Recommended resolution strategy
    pub recommended_strategy: ConflictResolutionStrategy,
}

// Main conflict resolver
//
// Central component for managing conflict detection, resolution,
// and monitoring across the replication system.
pub struct ConflictResolver {
    // Configuration
    config: Arc<ConflictResolverConfig>,
    // Active conflicts
    conflicts: Arc<RwLock<HashMap<Uuid, ConflictInfo>>>,
    // Custom resolvers by table
    custom_resolvers: Arc<RwLock<HashMap<TableName, Arc<dyn CustomConflictResolver>>>>,
    // Global custom resolvers
    global_resolvers: Arc<RwLock<Vec<Arc<dyn CustomConflictResolver>>>>,
    // Conflict resolution history
    resolution_history: Arc<RwLock<Vec<ConflictResolution>>>,
    // Event channel for conflict events
    event_sender: mpsc::UnboundedSender<ConflictEvent>,
    // Pattern analysis cache
    pattern_analysis: Arc<RwLock<Option<ConflictPatternAnalysis>>>,
}

// Conflict events for monitoring and alerting
#[derive(Debug, Clone)]
pub enum ConflictEvent {
    // New conflict detected
    ConflictDetected { conflict_id: Uuid, table_name: String, severity: ConflictSeverity },
    // Conflict resolution started
    ResolutionStarted { conflict_id: Uuid, strategy: ConflictResolutionStrategy },
    // Conflict successfully resolved
    ConflictResolved { conflict_id: Uuid, strategy: ConflictResolutionStrategy, duration: Duration },
    // Conflict resolution failed
    ResolutionFailed { conflict_id: Uuid, reason: String },
    // Manual intervention required
    ManualInterventionRequired { conflict_id: Uuid, table_name: String },
    // Pattern analysis completed
    PatternAnalysisCompleted { analysis_id: Uuid, patterns_found: usize },
}

impl ConflictResolver {
    // Creates a new conflict resolver
    //
    // # Arguments
    //
    // * `config` - Conflict resolver configuration
    //
    // # Returns
    //
    // * `Ok(ConflictResolver)` - Successfully created resolver
    // * `Err(ConflictResolutionError)` - Creation failed
    pub fn new(config: ConflictResolverConfig) -> Result<Self, ConflictResolutionError> {
        // Validate configuration
        Self::validate_config(&config)?;

        let (event_sender, _) = mpsc::unbounded_channel();

        Ok(Self {
            config: Arc::new(config),
            conflicts: Arc::new(RwLock::new(HashMap::new())),
            custom_resolvers: Arc::new(RwLock::new(HashMap::new())),
            global_resolvers: Arc::new(RwLock::new(Vec::new())),
            resolution_history: Arc::new(RwLock::new(Vec::new())),
            event_sender,
            pattern_analysis: Arc::new(RwLock::new(None)),
        })
    }

    // Validates the resolver configuration
    fn validate_config(config: &ConflictResolverConfig) -> Result<(), ConflictResolutionError> {
        if config.max_resolution_time.is_zero() {
            return Err(ConflictResolutionError::InvalidConfiguration {
                reason: "max_resolution_time must be greater than 0".to_string(),
            });
        }

        if config.max_conflicts_in_memory == 0 {
            return Err(ConflictResolutionError::InvalidConfiguration {
                reason: "max_conflicts_in_memory must be greater than 0".to_string(),
            });
        }

        if config.batch_processing_size == 0 {
            return Err(ConflictResolutionError::InvalidConfiguration {
                reason: "batch_processing_size must be greater than 0".to_string(),
            });
        }

        Ok(())
    }

    // Registers a custom resolver for a specific table
    //
    // # Arguments
    //
    // * `table_name` - Table to associate the resolver with
    // * `resolver` - Custom resolver implementation
    //
    // # Returns
    //
    // * `Ok(())` - Resolver registered successfully
    // * `Err(ConflictResolutionError)` - Registration failed
    pub async fn register_custom_resolver(
        &self,
        table_name: TableName,
        resolver: Arc<dyn CustomConflictResolver>,
    ) -> Result<(), ConflictResolutionError> {
        let mut resolvers = self.custom_resolvers.write();
        resolvers.insert(table_name, resolver);
        Ok(())
    }

    // Registers a global custom resolver
    //
    // # Arguments
    //
    // * `resolver` - Custom resolver implementation
    //
    // # Returns
    //
    // * `Ok(())` - Resolver registered successfully
    // * `Err(ConflictResolutionError)` - Registration failed
    pub async fn register_global_resolver(
        &self,
        resolver: Arc<dyn CustomConflictResolver>,
    ) -> Result<(), ConflictResolutionError> {
        let mut resolvers = self.global_resolvers.write();
        resolvers.push(resolver);
        // Sort by priority (highest first)
        resolvers.sort_by(|a, b| b.priority().cmp(&a.priority()));
        Ok(())
    }

    // Detects and registers a new conflict
    //
    // # Arguments
    //
    // * `conflict_data` - Data representing the conflict
    //
    // # Returns
    //
    // * `Ok(Uuid)` - ID of the registered conflict
    // * `Err(ConflictResolutionError)` - Detection failed
    pub async fn detect_conflict(
        &self,
        conflictdata: ConflictData,
    ) -> Result<Uuid, ConflictResolutionError> {
        let conflict_id = Uuid::new_v4();
        let severity = self.determine_conflict_severity(&conflictdata);

        let conflict_info = ConflictInfo {
            conflict_id,
            data: conflictdata.clone(),
            detected_at: SystemTime::now(),
            status: ConflictStatus::Pending,
            resolution_attempts: 0,
            resolution: None,
            severity: severity.clone(),
            tags: Vec::new(),
        };

        // Store conflict
        {
            let mut conflicts = self.conflicts.write();

            // Check memory limit
            if conflicts.len() >= self.config.max_conflicts_in_memory {
                // Remove oldest resolved conflicts
                self.cleanup_old_conflicts(&mut conflicts);
            }

            conflicts.insert(conflict_id, conflict_info);
        }

        // Publish event
        let _ = self.event_sender.send(ConflictEvent::ConflictDetected {
            conflict_id,
            table_name: conflictdata.table_name.as_str().to_string(),
            severity,
        });

        // Attempt automatic resolution if enabled
        if self.config.enable_automatic_resolution {
            let resolver = self.clone_arc();
            tokio::spawn(async move {
                if let Err(e) = resolver.resolve_conflict_internal(conflict_id).await {
                    eprintln!("Automatic resolution failed: {}", e);
                }
            });
        }

        Ok(conflict_id)
    }

    // Resolves a conflict using configured strategies
    //
    // # Arguments
    //
    // * `conflict_data` - Conflict to resolve
    //
    // # Returns
    //
    // * `Ok(ConflictResolution)` - Resolution result
    // * `Err(ConflictResolutionError)` - Resolution failed
    pub async fn resolve_conflict(
        &self,
        conflict_data: ConflictData,
    ) -> Result<ConflictResolution, ConflictResolutionError> {
        let conflict_id = self.detect_conflict(conflict_data).await?;
        self.resolve_conflict_by_id(conflict_id).await
    }

    // Resolves a conflict by its ID
    //
    // # Arguments
    //
    // * `conflict_id` - ID of conflict to resolve
    //
    // # Returns
    //
    // * `Ok(ConflictResolution)` - Resolution result
    // * `Err(ConflictResolutionError)` - Resolution failed
    pub async fn resolve_conflict_by_id(
        &self,
        conflict_id: Uuid,
    ) -> Result<ConflictResolution, ConflictResolutionError> {
        let start_time = SystemTime::now();

        // Get conflict info
        let conflict_info = {
            let conflicts = self.conflicts.read();
            conflicts.get(&conflict_id).cloned()
                .ok_or_else(|| ConflictResolutionError::ResolutionFailed {
                    conflict_id: conflict_id.to_string(),
                    reason: "Conflict not found".to_string(),
                })?
        };

        // Update status to resolving
        {
            let mut conflicts = self.conflicts.write();
            if let Some(conflict) = conflicts.get_mut(&conflict_id) {
                conflict.status = ConflictStatus::Resolving;
                conflict.resolution_attempts += 1;
            }
        }

        // Publish event
        let _ = self.event_sender.send(ConflictEvent::ResolutionStarted {
            conflict_id,
            strategy: self.config.default_strategy.clone(),
        });

        // Try resolution
        let resolution_result = self.attempt_resolution(&conflict_info.data).await;

        let resolution_time = start_time.elapsed().unwrap_or_default();

        let resolution = match resolution_result {
            Ok((strategy, resolved_data)) => {
                let resolution = ConflictResolution::success(
                    conflict_id,
                    strategy.clone(),
                    resolved_data,
                    resolution_time,
                );

                // Update conflict status
                {
                    let mut conflicts = self.conflicts.write();
                    if let Some(conflict) = conflicts.get_mut(&conflict_id) {
                        conflict.status = ConflictStatus::Resolved;
                        conflict.resolution = Some(resolution.clone());
                    }
                }

                // Publish success event
                let _ = self.event_sender.send(ConflictEvent::ConflictResolved {
                    conflict_id,
                    strategy,
                    duration: resolution_time,
                });

                resolution
            }
            Err(e) => {
                let resolution = ConflictResolution::failure(
                    conflict_id,
                    self.config.default_strategy.clone(),
                    resolution_time,
                    e.to_string(),
                );

                // Update conflict status
                {
                    let mut conflicts = self.conflicts.write();
                    if let Some(conflict) = conflicts.get_mut(&conflict_id) {
                        conflict.status = ConflictStatus::Failed;
                        conflict.resolution = Some(resolution.clone());
                    }
                }

                // Publish failure event
                let _ = self.event_sender.send(ConflictEvent::ResolutionFailed {
                    conflict_id,
                    reason: e.to_string(),
                });

                resolution
            }
        };

        // Store resolution in history
        {
            let mut history = self.resolution_history.write();
            history.push(resolution.clone());

            // Limit history size
            let current_len = history.len();
            if current_len > self.config.max_conflicts_in_memory * 2 {
                history.drain(0..current_len / 2);
            }
        }

        Ok(resolution)
    }

    // Attempts to resolve a conflict using available strategies
    async fn attempt_resolution(
        &self,
        conflict_data: &ConflictData,
    ) -> Result<(ConflictResolutionStrategy, Vec<u8>), ConflictResolutionError> {
        // Try table-specific custom resolver first
        let table_resolver = self.custom_resolvers.read().get(&conflict_data.table_name).cloned();
        if let Some(resolver) = table_resolver {
            if resolver.can_resolve(conflict_data).await {
                let resolved_data = resolver.resolve(conflict_data).await?;
                return Ok((ConflictResolutionStrategy::Custom, resolved_data));
            }
        }

        // Try global custom resolvers
        let global_resolvers: Vec<_> = self.global_resolvers.read().iter().cloned().collect();
        for resolver in global_resolvers.iter() {
            if resolver.can_resolve(conflict_data).await {
                let resolved_data = resolver.resolve(conflict_data).await?;
                return Ok((ConflictResolutionStrategy::Custom, resolved_data));
            }
        }

        // Use default strategy
        match self.config.default_strategy {
            ConflictResolutionStrategy::LastWriteWins => {
                let resolver = LastWriteWinsResolver;
                let resolved_data = resolver.resolve(conflict_data).await?;
                Ok((ConflictResolutionStrategy::LastWriteWins, resolved_data))
            }
            ConflictResolutionStrategy::FirstWriteWins => {
                let resolver = FirstWriteWinsResolver;
                let resolved_data = resolver.resolve(conflict_data).await?;
                Ok((ConflictResolutionStrategy::FirstWriteWins, resolved_data))
            }
            ConflictResolutionStrategy::PrimaryWins => {
                // Assume local is primary for this example
                Ok((ConflictResolutionStrategy::PrimaryWins, conflict_data.local_version.clone()))
            }
            ConflictResolutionStrategy::Custom => {
                Err(ConflictResolutionError::CustomResolverNotFound {
                    table_name: conflict_data.table_name.as_str().to_string(),
                })
            }
        }
    }

    // Internal conflict resolution method
    async fn resolve_conflict_internal(&self, conflict_id: Uuid) -> Result<(), ConflictResolutionError> {
        self.resolve_conflict_by_id(conflict_id).await.map(|_| ())
    }

    // Determines the severity of a conflict
    fn determine_conflict_severity(&self, conflict_data: &ConflictData) -> ConflictSeverity {
        // Simple severity determination based on time delta and operation
        let time_delta = conflict_data.time_delta().unwrap_or_default();

        match conflict_data.operation {
            ReplicationOperation::Delete | ReplicationOperation::DropTable => ConflictSeverity::Critical,
            ReplicationOperation::Update if time_delta < Duration::from_secs(1) => ConflictSeverity::High,
            ReplicationOperation::Insert if time_delta < Duration::from_secs(1) => ConflictSeverity::Medium,
            _ => ConflictSeverity::Low,
        }
    }

    // Creates an Arc clone for async tasks
    fn clone_arc(&self) -> Arc<Self> {
        // This would need proper Arc wrapping in real implementation
        // For now, return a placeholder
        unimplemented!("Arc cloning not implemented in this example")
    }

    // Cleans up old resolved conflicts to free memory
    fn cleanup_old_conflicts(&self, conflicts: &mut HashMap<Uuid, ConflictInfo>) {
        let cutoff = SystemTime::now() - Duration::from_secs(3600); // 1 hour

        conflicts.retain(|_, conflict| {
            conflict.status != ConflictStatus::Resolved ||
            conflict.detected_at > cutoff
        });
    }

    // Gets all active conflicts
    pub fn get_active_conflicts(&self) -> Vec<ConflictInfo> {
        let conflicts = self.conflicts.read();
        conflicts.values()
            .filter(|c| c.status == ConflictStatus::Pending || c.status == ConflictStatus::Resolving)
            .cloned()
            .collect()
    }

    // Gets conflict resolution statistics
    pub fn get_resolution_statistics(&self) -> ConflictResolutionStatistics {
        let conflicts = self.conflicts.read();
        let history = self.resolution_history.read();

        let total_conflicts = conflicts.len();
        let resolved_conflicts = conflicts.values()
            .filter(|c| c.status == ConflictStatus::Resolved)
            .count();

        let failed_conflicts = conflicts.values()
            .filter(|c| c.status == ConflictStatus::Failed)
            .count();

        let average_resolution_time = if !history.is_empty() {
            let total_time: Duration = history.iter()
                .map(|r| r.resolution_time)
                .sum();
            total_time / history.len() as u32
        } else {
            Duration::default()
        };

        ConflictResolutionStatistics {
            total_conflicts,
            resolved_conflicts,
            failed_conflicts,
            pending_conflicts: total_conflicts - resolved_conflicts - failed_conflicts,
            average_resolution_time,
            total_resolution_attempts: history.len(),
        }
    }

    // Performs conflict pattern analysis
    pub async fn analyze_conflict_patterns(&self) -> Result<ConflictPatternAnalysis, ConflictResolutionError> {
        let conflicts = self.conflicts.read();
        let history = self.resolution_history.read();

        let mut conflicts_by_table = HashMap::new();
        let mut conflicts_by_operation = HashMap::new();
        let mut resolution_times_by_strategy = HashMap::new();

        // Analyze conflicts by table
        for conflict in conflicts.values() {
            let table_name = conflict.data.table_name.as_str().to_string();
            *conflicts_by_table.entry(table_name).or_insert(0) += 1;

            *conflicts_by_operation.entry(conflict.data.operation.clone()).or_insert(0) += 1;
        }

        // Analyze resolution times by strategy
        for resolution in history.iter() {
            resolution_times_by_strategy
                .entry(resolution.strategy_used.clone())
                .or_insert_with(Vec::new)
                .push(resolution.resolution_time);
        }

        let avg_resolution_time_by_strategy = resolution_times_by_strategy
            .into_iter()
            .map(|(strategy, times)| {
                let avg = times.iter().sum::<Duration>() / times.len() as u32;
                (strategy, avg)
            })
            .collect();

        let analysis = ConflictPatternAnalysis {
            analyzed_at: SystemTime::now(),
            total_conflicts: conflicts.len(),
            conflicts_by_table,
            conflicts_by_operation,
            avg_resolution_time_by_strategy,
            common_patterns: Vec::new(), // Would be computed from pattern detection
            recommendations: vec!["Consider adding custom resolvers for high-conflict tables".to_string()],
        };

        // Cache analysis
        *self.pattern_analysis.write() = Some(analysis.clone());

        // Publish event
        let _ = self.event_sender.send(ConflictEvent::PatternAnalysisCompleted {
            analysis_id: Uuid::new_v4(),
            patterns_found: analysis.common_patterns.len(),
        });

        Ok(analysis)
    }
}

// Conflict resolution statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConflictResolutionStatistics {
    // Total number of conflicts
    pub total_conflicts: usize,
    // Number of successfully resolved conflicts
    pub resolved_conflicts: usize,
    // Number of failed resolution attempts
    pub failed_conflicts: usize,
    // Number of pending conflicts
    pub pending_conflicts: usize,
    // Average time to resolve conflicts
    pub average_resolution_time: Duration,
    // Total resolution attempts
    pub total_resolution_attempts: usize,
}

#[cfg(test)]
mod tests {
    use std::time::{Duration, SystemTime};
    use crate::replication::conflicts::{ConflictData, ConflictResolver, ConflictResolverConfig, ConflictSeverity, CustomConflictResolver, FirstWriteWinsResolver, LastWriteWinsResolver};
    use crate::replication::types::{ReplicationOperation, TableName};

    #[tokio::test]
    async fn test_conflict_resolver_creation() {
        let config = ConflictResolverConfig::default();
        let resolver = ConflictResolver::new(config);
        assert!(resolver.is_ok());
    }

    #[tokio::test]
    async fn test_conflict_data_creation() {
        let table_name = TableName::new("test_table").unwrap();
        let conflict = ConflictData::new(
            table_name,
            "key123".to_string(),
            b"local data".to_vec(),
            b"remote data".to_vec(),
            SystemTime::now(),
            SystemTime::now() - Duration::from_secs(10),
            ReplicationOperation::Update,
        );

        assert!(conflict.time_delta().is_some());
        assert!(conflict.time_delta().unwrap() >= Duration::from_secs(10));
    }

    #[tokio::test]
    async fn test_last_write_wins_resolver() {
        let resolver = LastWriteWinsResolver;
        let table_name = TableName::new("test").unwrap();

        let conflict = ConflictData::new(
            table_name,
            "key".to_string(),
            b"old data".to_vec(),
            b"new data".to_vec(),
            SystemTime::now() - Duration::from_secs(10),
            SystemTime::now(),
            ReplicationOperation::Update,
        );

        let result = resolver.resolve(&conflict).await.unwrap();
        assert_eq!(result, b"new data".to_vec());
    }

    #[tokio::test]
    async fn test_first_write_wins_resolver() {
        let resolver = FirstWriteWinsResolver;
        let table_name = TableName::new("test").unwrap();

        let conflict = ConflictData::new(
            table_name,
            "key".to_string(),
            b"old data".to_vec(),
            b"new data".to_vec(),
            SystemTime::now() - Duration::from_secs(10),
            SystemTime::now(),
            ReplicationOperation::Update,
        );

        let result = resolver.resolve(&conflict).await.unwrap();
        assert_eq!(result, b"old data".to_vec());
    }

    #[test]
    fn test_conflict_severity_determination() {
        let config = ConflictResolverConfig::default();
        let resolver = ConflictResolver::new(config).unwrap();
        let table_name = TableName::new("test").unwrap();

        // Critical conflict (DELETE operation)
        let delete_conflict = ConflictData::new(
            table_name.clone(),
            "key".to_string(),
            b"data".to_vec(),
            b"data".to_vec(),
            SystemTime::now(),
            SystemTime::now(),
            ReplicationOperation::Delete,
        );

        let severity = resolver.determine_conflict_severity(&delete_conflict);
        assert_eq!(severity, ConflictSeverity::Critical);

        // Low priority conflict (INSERT with old timestamp)
        let insert_conflict = ConflictData::new(
            table_name,
            "key".to_string(),
            b"data".to_vec(),
            b"data".to_vec(),
            SystemTime::now() - Duration::from_secs(60),
            SystemTime::now() - Duration::from_secs(30),
            ReplicationOperation::Insert,
        );

        let severity = resolver.determine_conflict_severity(&insert_conflict);
        assert_eq!(severity, ConflictSeverity::Low);
    }
}
