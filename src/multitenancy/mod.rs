// Multi-tenant architecture module for RustyDB
// Provides Oracle-like Pluggable Database (PDB) / Container Database (CDB) capabilities

pub mod container;
pub mod tenant;
pub mod isolation;
pub mod consolidation;
pub mod provisioning;

// Re-export main types
pub use container::{
    CdbStatistics, CloneType, ContainerDatabase, ContainerError, ContainerParameter,
    ContainerResult, OpenMode, PdbConfig, PdbState,
};

pub use tenant::{
    ResourceQuota, ResourceUsage, ServiceTier, SlaMetrics, Tenant,
    TenantError, TenantManager, TenantMetadata, TenantPriority,
    TenantResult, TenantState, TenantStatistics,
};

pub use isolation::{
    BufferPoolPartitioner, CpuScheduler, IoBandwidthAllocator, IsolationError,
    IsolationResult, LockContentionIsolator, MemoryIsolator, NetworkIsolator,
};

pub use consolidation::{
    AffinityRule, AffinityType, ConsolidationHost, ConsolidationMetrics,
    ConsolidationPlan, ConsolidationPlanner, WorkloadProfile, WorkloadType,
};

pub use provisioning::{
    DeprovisioningPolicy, DeprovisioningRequest, ProvisioningRequest,
    ProvisioningService, ProvisioningTemplate, ServiceTier as ProvisioningTier,
};

use std::sync::Arc;

/// Unified multi-tenant database manager integrating all components
pub struct MultiTenantDatabase {
    pub container_db: Arc<ContainerDatabase>,
    pub tenant_manager: Arc<TenantManager>,
    pub memory_isolator: Arc<MemoryIsolator>,
    pub io_allocator: Arc<IoBandwidthAllocator>,
    pub cpu_scheduler: Arc<CpuScheduler>,
    pub network_isolator: Arc<NetworkIsolator>,
    pub consolidation_planner: Arc<ConsolidationPlanner>,
    pub provisioning_service: Arc<ProvisioningService>,
}

impl MultiTenantDatabase {
    /// Create a new multi-tenant database instance
    pub fn new(cdb_name: String, max_pdbs: u32) -> Self {
        Self {
            container_db: Arc::new(ContainerDatabase::new(cdb_name, max_pdbs)),
            tenant_manager: Arc::new(TenantManager::new()),
            memory_isolator: Arc::new(MemoryIsolator::new(16384)), // 16GB default
            io_allocator: Arc::new(IoBandwidthAllocator::new()),
            cpu_scheduler: Arc::new(CpuScheduler::new()),
            network_isolator: Arc::new(NetworkIsolator::new(10000, 20000)),
            consolidation_planner: Arc::new(ConsolidationPlanner::new()),
            provisioning_service: Arc::new(ProvisioningService::new()),
        }
    }

    /// Provision a new tenant with full isolation
    pub async fn provision_tenant(
        &self,
        tenant_name: String,
        admin_user: String,
        admin_password: String,
        service_tier: ServiceTier,
    ) -> Result<String, Box<dyn std::error::Error>> {
        // Create PDB in container database
        let pdb = self.container_db.create_pdb(
            tenant_name.clone(),
            admin_user.clone(),
            admin_password,
        ).await?;

        let pdb_config = pdb.read().await;
        let tenant_id = pdb_config.pdb_name.clone();
        drop(pdb_config);

        // Create tenant with isolation
        let tenant = Arc::new(Tenant::new(
            tenant_id.clone(),
            tenant_name,
            admin_user,
            service_tier.clone(),
        ));

        // Register tenant
        self.tenant_manager.register_tenant(tenant.clone()).await?;

        // Configure resource isolation
        self.memory_isolator.set_quota(
            &tenant_id,
            service_tier.memory_mb * 1024 * 1024,
        ).await?;

        self.io_allocator.configure_tenant(
            tenant_id.clone(),
            service_tier.network_mbps,
        ).await;

        self.cpu_scheduler.configure_tenant(
            tenant_id.clone(),
            1000,
            10,
            (service_tier.cpu_cores * 100.0) as u32,
        ).await?;

        self.network_isolator.allocate_tenant(
            tenant_id.clone(),
            service_tier.network_mbps,
            service_tier.max_connections,
        ).await?;

        Ok(tenant_id)
    }

    /// Open a PDB and activate tenant
    pub async fn activate_tenant(
        &self,
        tenant_id: String,
    ) -> Result<(), Box<dyn std::error::Error>> {
        // Open PDB
        self.container_db.open_pdb(tenant_id.clone(), OpenMode::ReadWrite).await?;

        // Resume tenant if suspended
        let tenant = self.tenant_manager.get_tenant(&tenant_id).await?;
        let state = *tenant.state.read().await;

        if state == TenantState::Suspended {
            tenant.resume().await?;
        }

        Ok(())
    }

    /// Suspend a tenant
    pub async fn suspend_tenant(
        &self,
        tenant_id: String,
        reason: String,
    ) -> Result<(), Box<dyn std::error::Error>> {
        // Close PDB
        self.container_db.close_pdb(tenant_id.clone()).await?;

        // Suspend tenant
        let tenant = self.tenant_manager.get_tenant(&tenant_id).await?;
        tenant.suspend(reason).await?;

        Ok(())
    }

    /// Get comprehensive tenant statistics
    pub async fn get_tenant_stats(&self, tenant_id: &str) -> Option<TenantStats> {
        let tenant = self.tenant_manager.get_tenant(tenant_id).await.ok()?;

        let tenant_stats = tenant.get_statistics().await;
        let sla_metrics = tenant.get_sla_metrics().await;
        let memory_stats = self.memory_isolator.get_tenant_stats(tenant_id).await;
        let cpu_stats = self.cpu_scheduler.get_tenant_stats(tenant_id).await;
        let network_stats = self.network_isolator.get_stats(tenant_id).await;

        Some(TenantStats {
            tenant_id: tenant_id.to_string(),
            tenant_stats,
            sla_metrics,
            memory_stats,
            cpu_stats,
            network_stats,
        })
    }

    /// Get overall system statistics
    pub async fn get_system_stats(&self) -> SystemStats {
        let cdb_stats = self.container_db.get_statistics().await;
        let consolidation_metrics = self.consolidation_planner.get_metrics().await;
        let memory_stats = self.memory_isolator.get_global_stats().await;
        let active_tenants = self.tenant_manager.active_count().await;

        SystemStats {
            cdb_stats,
            consolidation_metrics,
            memory_stats,
            active_tenants,
        }
    }
}

/// Comprehensive tenant statistics
#[derive(Debug, Clone)]
pub struct TenantStats {
    pub tenant_id: String,
    pub tenant_stats: TenantStatistics,
    pub sla_metrics: SlaMetrics,
    pub memory_stats: Option<isolation::TenantMemoryAllocation>,
    pub cpu_stats: Option<isolation::CpuStats>,
    pub network_stats: Option<isolation::NetworkConfig>,
}

/// System-wide statistics
#[derive(Debug, Clone)]
pub struct SystemStats {
    pub cdb_stats: CdbStatistics,
    pub consolidation_metrics: ConsolidationMetrics,
    pub memory_stats: isolation::MemoryGlobalStats,
    pub active_tenants: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_multi_tenant_database() {
        let mtdb = MultiTenantDatabase::new("CDB_PROD".to_string(), 100);

        let result = mtdb.provision_tenant(
            "tenant1".to_string(),
            "admin".to_string(),
            "password".to_string(),
            ServiceTier::silver(),
        ).await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_tenant_activation() {
        let mtdb = MultiTenantDatabase::new("CDB_PROD".to_string(), 100);

        let tenant_id = mtdb.provision_tenant(
            "tenant1".to_string(),
            "admin".to_string(),
            "password".to_string(),
            ServiceTier::bronze(),
        ).await.unwrap();

        let result = mtdb.activate_tenant(tenant_id).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_system_stats() {
        let mtdb = MultiTenantDatabase::new("CDB_PROD".to_string(), 100);

        let stats = mtdb.get_system_stats().await;
        assert_eq!(stats.cdb_stats.total_pdbs, 0);
    }
}


