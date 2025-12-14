// # Cache Fusion Coordinator
//
// High-level coordinator that integrates Global Cache Service (GCS) and
// Global Enqueue Service (GES) to provide a unified Cache Fusion interface.

use super::global_cache::{
    BlockGrant, BlockMode, GcsConfig, GcsStatistics, GlobalCacheService, ResourceId,
};
use super::lock_management::{GesStatistics, GlobalEnqueueService, LockGrant, LockType};
use crate::common::{NodeId, TransactionId};
use crate::error::DbError;
use std::sync::Arc;

// ============================================================================
// Cache Fusion Coordinator
// ============================================================================

// Cache Fusion coordinator - integrates GCS and GES
pub struct CacheFusionCoordinator {
    // Global Cache Service
    gcs: Arc<GlobalCacheService>,

    // Global Enqueue Service
    ges: Arc<GlobalEnqueueService>,

    // Node identifier
    _node_id: NodeId,
}

impl CacheFusionCoordinator {
    // Create a new Cache Fusion coordinator
    pub fn new(node_id: NodeId, gcs_config: GcsConfig) -> Self {
        Self {
            gcs: Arc::new(GlobalCacheService::new(node_id.clone(), gcs_config)),
            ges: Arc::new(GlobalEnqueueService::new(node_id.clone())),
            _node_id: node_id,
        }
    }

    // Request block with automatic lock acquisition
    pub async fn request_block_with_lock(
        &self,
        resource_id: ResourceId,
        mode: BlockMode,
        transaction_id: TransactionId,
    ) -> Result<(BlockGrant, LockGrant), DbError> {
        // First acquire lock
        let lock_type = match mode {
            BlockMode::Exclusive | BlockMode::ExclusiveCurrent => LockType::Exclusive,
            BlockMode::Shared | BlockMode::SharedCurrent => LockType::ConcurrentRead,
            _ => LockType::Null,
        };

        let lock_grant = self
            .ges
            .acquire_lock(resource_id.clone(), lock_type)
            .await?;

        // Then request block
        let block_grant = self
            .gcs
            .request_block(resource_id, mode, transaction_id, false)
            .await?;

        Ok((block_grant, lock_grant))
    }

    // Release block and lock
    pub async fn release_block_with_lock(&self, resource_id: ResourceId) -> Result<(), DbError> {
        // Release lock
        self.ges.release_lock(resource_id).await?;

        // GCS automatically handles block state
        Ok(())
    }

    // Get combined statistics
    pub fn get_statistics(&self) -> CacheFusionStatistics {
        CacheFusionStatistics {
            gcs: self.gcs.get_statistics(),
            ges: self.ges.get_statistics(),
        }
    }
}

// Combined Cache Fusion statistics
#[derive(Debug, Clone)]
pub struct CacheFusionStatistics {
    pub gcs: GcsStatistics,
    pub ges: GesStatistics,
}

#[cfg(test)]
mod tests {
    use super::super::global_cache::ResourceClass;
    use super::*;

    #[tokio::test]
    async fn test_block_mode_compatibility() {
        assert!(BlockMode::Shared.is_compatible(&BlockMode::Shared));
        assert!(!BlockMode::Exclusive.is_compatible(&BlockMode::Exclusive));
        assert!(BlockMode::Null.is_compatible(&BlockMode::Exclusive));
    }

    #[tokio::test]
    async fn test_gcs_local_cache() {
        let gcs = GlobalCacheService::new("node1".to_string(), GcsConfig::default());

        let resource_id = ResourceId {
            file_id: 1,
            block_number: 100,
            class: ResourceClass::Data,
        };

        let grant = gcs
            .request_block(resource_id, BlockMode::Shared, 1, false)
            .await;

        assert!(grant.is_ok());
    }

    #[tokio::test]
    async fn test_ges_lock_acquisition() {
        let ges = GlobalEnqueueService::new("node1".to_string());

        let resource_id = ResourceId {
            file_id: 1,
            block_number: 100,
            class: ResourceClass::Data,
        };

        let grant = ges
            .acquire_lock(resource_id, LockType::ConcurrentRead)
            .await;
        assert!(grant.is_ok());
    }
}
