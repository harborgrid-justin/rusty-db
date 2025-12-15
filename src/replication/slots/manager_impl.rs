#![allow(dead_code)]
// SlotManager trait implementation for ReplicationSlotManager

use async_trait::async_trait;
use std::time::SystemTime;

use super::errors::SlotError;
use super::manager::{ReplicationSlotManager, SlotManager};
use super::types::*;
use crate::replication::types::{LogSequenceNumber, ReplicaId};

#[async_trait]
impl SlotManager for ReplicationSlotManager {
    async fn create_slot(
        &self,
        slot_name: SlotName,
        replica_id: ReplicaId,
        slot_type: SlotType,
        config: Option<SlotConfig>,
    ) -> Result<SlotId, SlotError> {
        {
            let slots = self.slots.read();
            if slots.contains_key(&slot_name) {
                return Err(SlotError::SlotAlreadyExists {
                    slot_name: slot_name.to_string(),
                });
            }
            if slots.len() >= self.config.max_slots {
                return Err(SlotError::TooManySlots {
                    current: slots.len(),
                    max: self.config.max_slots,
                });
            }
        }

        let slot_id = SlotId::new(uuid::Uuid::new_v4().to_string())?;
        let now = SystemTime::now();

        let slot_info = SlotInfo {
            slot_id: slot_id.clone(),
            slot_name: slot_name.clone(),
            replica_id,
            slot_type,
            current_lsn: LogSequenceNumber::new(0),
            restart_lsn: LogSequenceNumber::new(0),
            confirmed_flush_lsn: None,
            status: SlotStatus::Active,
            created_at: now,
            last_active: now,
            active: true,
            active_pid: None,
            config: config.unwrap_or_default(),
            metadata: std::collections::HashMap::new(),
            statistics: SlotStatistics::default(),
        };

        self.save_slot_to_disk(&slot_info).await?;

        {
            let mut slots = self.slots.write();
            slots.insert(
                slot_name.clone(),
                std::sync::Arc::new(parking_lot::RwLock::new(slot_info)),
            );
        }

        {
            let mut active = self.active_slots.write();
            active.insert(slot_name.clone());
        }

        {
            let mut metrics = self.metrics.write();
            metrics.insert(slot_name, AtomicSlotMetrics::default());
        }

        Ok(slot_id)
    }

    async fn drop_slot(&self, slot_name: &SlotName) -> Result<(), SlotError> {
        {
            let slots = self.slots.read();
            if let Some(slot_info) = slots.get(slot_name) {
                let info = slot_info.read();
                if info.active {
                    return Err(SlotError::SlotActive {
                        slot_name: slot_name.to_string(),
                    });
                }
            } else {
                return Err(SlotError::SlotNotFound {
                    slot_name: slot_name.to_string(),
                });
            }
        }

        let path = self.config.storage_path.join(format!("{}.slot", slot_name));
        if path.exists() {
            tokio::fs::remove_file(&path)
                .await
                .map_err(|e| SlotError::WriteFailed {
                    slot_name: slot_name.to_string(),
                    reason: e.to_string(),
                })?;
        }

        {
            let mut slots = self.slots.write();
            slots.remove(slot_name);
        }

        {
            let mut active = self.active_slots.write();
            active.remove(slot_name);
        }

        {
            let mut metrics = self.metrics.write();
            metrics.remove(slot_name);
        }

        Ok(())
    }

    async fn get_slot_info(&self, slot_name: &SlotName) -> Result<SlotInfo, SlotError> {
        let slots = self.slots.read();
        slots
            .get(slot_name)
            .map(|slot| slot.read().clone())
            .ok_or_else(|| SlotError::SlotNotFound {
                slot_name: slot_name.to_string(),
            })
    }

    async fn list_slots(&self) -> Result<Vec<SlotInfo>, SlotError> {
        let slots = self.slots.read();
        Ok(slots.values().map(|slot| slot.read().clone()).collect())
    }

    async fn advance_slot(
        &self,
        slot_name: &SlotName,
        target_lsn: LogSequenceNumber,
    ) -> Result<(), SlotError> {
        let slot_arc = {
            let slots = self.slots.read();
            slots
                .get(slot_name)
                .cloned()
                .ok_or_else(|| SlotError::SlotNotFound {
                    slot_name: slot_name.to_string(),
                })?
        };

        {
            let mut slot_info = slot_arc.write();

            if target_lsn < slot_info.current_lsn {
                return Err(SlotError::InvalidLsn {
                    lsn: target_lsn.to_string(),
                    slot_name: slot_name.to_string(),
                });
            }

            slot_info.current_lsn = target_lsn;
            slot_info.confirmed_flush_lsn = Some(target_lsn);
            slot_info.last_active = SystemTime::now();
        }

        let slot_info = slot_arc.read().clone();
        self.save_slot_to_disk(&slot_info).await?;

        Ok(())
    }

    async fn get_slot_health(&self, slot_name: &SlotName) -> Result<SlotHealth, SlotError> {
        let slot_info = self.get_slot_info(slot_name).await?;
        Ok(self.calculate_slot_health(&slot_info))
    }

    async fn check_all_slots_health(&self) -> Result<Vec<SlotHealth>, SlotError> {
        let slots = self.list_slots().await?;
        Ok(slots
            .iter()
            .map(|info| self.calculate_slot_health(info))
            .collect())
    }
}
