// Replication slot manager

use async_trait::async_trait;
use parking_lot::RwLock;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::time::{Duration, SystemTime};
use tokio::sync::mpsc;

use crate::replication::types::{LogSequenceNumber, ReplicaId};
use super::config::SlotManagerConfig;
use super::errors::SlotError;
use super::types::*;

/// Slot manager trait
#[async_trait]
pub trait SlotManager: Send + Sync {
    /// Create a new replication slot
    async fn create_slot(
        &self,
        slot_name: SlotName,
        replica_id: ReplicaId,
        slot_type: SlotType,
        config: Option<SlotConfig>,
    ) -> Result<SlotId, SlotError>;

    /// Drop a replication slot
    async fn drop_slot(&self, slot_name: &SlotName) -> Result<(), SlotError>;

    /// Get slot information
    async fn get_slot_info(&self, slot_name: &SlotName) -> Result<SlotInfo, SlotError>;

    /// List all slots
    async fn list_slots(&self) -> Result<Vec<SlotInfo>, SlotError>;

    /// Advance slot position
    async fn advance_slot(
        &self,
        slot_name: &SlotName,
        target_lsn: LogSequenceNumber,
    ) -> Result<(), SlotError>;

    /// Get slot health status
    async fn get_slot_health(&self, slot_name: &SlotName) -> Result<SlotHealth, SlotError>;

    /// Check all slots health
    async fn check_all_slots_health(&self) -> Result<Vec<SlotHealth>, SlotError>;
}

/// Replication slot manager implementation
pub struct ReplicationSlotManager {
    /// Configuration
    config: Arc<SlotManagerConfig>,
    /// Slots storage
    slots: Arc<RwLock<HashMap<SlotName, Arc<RwLock<SlotInfo>>>>>,
    /// Active slots tracking
    active_slots: Arc<RwLock<HashSet<SlotName>>>,
    /// Slot metrics
    metrics: Arc<RwLock<HashMap<SlotName, AtomicSlotMetrics>>>,
    /// Background task handles
    cleanup_handle: Arc<RwLock<Option<tokio::task::JoinHandle<()>>>>,
    monitoring_handle: Arc<RwLock<Option<tokio::task::JoinHandle<()>>>>,
}

impl ReplicationSlotManager {
    /// Create a new slot manager
    pub async fn new(config: SlotManagerConfig) -> Result<Self, SlotError> {
        Self::validate_config(&config)?;
        Self::create_directories(&config).await?;

        let manager = Self {
            config: Arc::new(config),
            slots: Arc::new(RwLock::new(HashMap::new())),
            active_slots: Arc::new(RwLock::new(HashSet::new())),
            metrics: Arc::new(RwLock::new(HashMap::new())),
            cleanup_handle: Arc::new(RwLock::new(None)),
            monitoring_handle: Arc::new(RwLock::new(None)),
        };

        manager.load_existing_slots().await?;

        if manager.config.enable_auto_cleanup {
            manager.start_background_cleanup().await;
        }

        if manager.config.enable_monitoring {
            manager.start_background_monitoring().await;
        }

        Ok(manager)
    }

    fn validate_config(config: &SlotManagerConfig) -> Result<(), SlotError> {
        if config.max_slots == 0 {
            return Err(SlotError::InvalidConfiguration {
                reason: "max_slots must be greater than 0".to_string(),
            });
        }
        Ok(())
    }

    async fn create_directories(config: &SlotManagerConfig) -> Result<(), SlotError> {
        tokio::fs::create_dir_all(&config.storage_path)
            .await
            .map_err(|e| SlotError::WriteFailed {
                slot_name: "system".to_string(),
                reason: format!("Failed to create storage directory: {}", e),
            })?;
        Ok(())
    }

    async fn load_existing_slots(&self) -> Result<(), SlotError> {
        let mut entries = tokio::fs::read_dir(&self.config.storage_path)
            .await
            .map_err(|e| SlotError::ConsumptionFailed {
                slot_name: "system".to_string(),
                reason: format!("Failed to read storage directory: {}", e),
            })?;

        while let Some(entry) = entries.next_entry()
            .await
            .map_err(|e| SlotError::ConsumptionFailed {
                slot_name: "system".to_string(),
                reason: e.to_string(),
            })? {
            let path = entry.path();
            if path.extension().map_or(false, |ext| ext == "slot") {
                if let Ok(slot_info) = self.load_slot_from_disk(&path).await {
                    let slot_name = slot_info.slot_name.clone();
                    let mut slots = self.slots.write();
                    slots.insert(slot_name.clone(), Arc::new(RwLock::new(slot_info)));

                    let mut metrics = self.metrics.write();
                    metrics.insert(slot_name, AtomicSlotMetrics::default());
                }
            }
        }

        Ok(())
    }

    async fn load_slot_from_disk(&self, path: &std::path::Path) -> Result<SlotInfo, SlotError> {
        let contents = tokio::fs::read_to_string(path)
            .await
            .map_err(|e| SlotError::ConsumptionFailed {
                slot_name: "unknown".to_string(),
                reason: e.to_string(),
            })?;
        serde_json::from_str(&contents)
            .map_err(|e| SlotError::StateCorruption {
                slot_name: "unknown".to_string(),
                reason: format!("Failed to parse slot data: {}", e),
            })
    }

    async fn save_slot_to_disk(&self, slot_info: &SlotInfo) -> Result<(), SlotError> {
        let path = self.config.storage_path.join(format!("{}.slot", slot_info.slot_name));
        let contents = serde_json::to_string_pretty(slot_info)
            .map_err(|e| SlotError::WriteFailed {
                slot_name: slot_info.slot_name.to_string(),
                reason: e.to_string(),
            })?;
        tokio::fs::write(&path, contents)
            .await
            .map_err(|e| SlotError::WriteFailed {
                slot_name: slot_info.slot_name.to_string(),
                reason: e.to_string(),
            })?;
        Ok(())
    }

    async fn start_background_cleanup(&self) {
        let slots = Arc::clone(&self.slots);
        let config = Arc::clone(&self.config);

        let handle = tokio::spawn(async move {
            let mut interval = tokio::time::interval(config.cleanup_interval);
            loop {
                interval.tick().await;
                let now = SystemTime::now();
                let mut to_remove = Vec::new();

                {
                    let slots = slots.read();
                    for (slot_name, slot_info) in slots.iter() {
                        let info = slot_info.read();
                        if !info.active {
                            if let Ok(duration) = now.duration_since(info.last_active) {
                                if duration > config.max_inactive_duration {
                                    to_remove.push(slot_name.clone());
                                }
                            }
                        }
                    }
                }

                for slot_name in to_remove {
                    let mut slots = slots.write();
                    slots.remove(&slot_name);
                }
            }
        });

        *self.cleanup_handle.write() = Some(handle);
    }

    async fn start_background_monitoring(&self) {
        let slots = Arc::clone(&self.slots);
        let metrics = Arc::clone(&self.metrics);
        let config = Arc::clone(&self.config);

        let handle = tokio::spawn(async move {
            let mut interval = tokio::time::interval(config.monitoring_interval);
            loop {
                interval.tick().await;

                let slots = slots.read();
                let mut metrics = metrics.write();

                for (slot_name, slot_info) in slots.iter() {
                    if let Some(slot_metrics) = metrics.get_mut(slot_name) {
                        let info = slot_info.read();
                        let lag = info.statistics.lag_bytes;
                        slot_metrics.update_lag(lag);
                    }
                }
            }
        });

        *self.monitoring_handle.write() = Some(handle);
    }

    fn calculate_slot_health(&self, slot_info: &SlotInfo) -> SlotHealth {
        let mut issues = Vec::new();
        let mut recommendations = Vec::new();

        let status = if slot_info.status == SlotStatus::Error {
            issues.push("Slot is in error state".to_string());
            SlotHealthStatus::Critical
        } else if slot_info.statistics.lag_bytes > slot_info.config.max_lag_bytes {
            issues.push(format!("High lag: {} bytes", slot_info.statistics.lag_bytes));
            recommendations.push("Consider scaling up resources or reducing load".to_string());
            SlotHealthStatus::Degraded
        } else if slot_info.statistics.error_count > 10 {
            issues.push(format!("High error count: {}", slot_info.statistics.error_count));
            SlotHealthStatus::Degraded
        } else if !slot_info.active {
            let now = SystemTime::now();
            if let Ok(duration) = now.duration_since(slot_info.last_active) {
                if duration > Duration::from_secs(3600) {
                    issues.push("Slot has been inactive for over 1 hour".to_string());
                    recommendations.push("Consider dropping unused slot".to_string());
                    SlotHealthStatus::Unhealthy
                }
            }
            SlotHealthStatus::Degraded
        } else {
            SlotHealthStatus::Healthy
        };

        SlotHealth {
            slot_name: slot_info.slot_name.clone(),
            status,
            lag_bytes: slot_info.statistics.lag_bytes,
            lag_time: slot_info.statistics.lag_time,
            last_active: slot_info.last_active,
            issues,
            recommendations,
        }
    }
}

impl Default for ReplicationSlotManager {
    fn default() -> Self {
        futures::executor::block_on(async {
            Self::new(SlotManagerConfig::default()).await.unwrap()
        })
    }
}
