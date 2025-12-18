// # Multi-Tenant Architecture Engine
//
// ⚠️ **CRITICAL: DUPLICATE MULTI-TENANCY IMPLEMENTATION** ⚠️
//
// **Issue**: This module (`src/multitenant/`) duplicates functionality from `src/multitenancy/`
//
// **Duplication Analysis**:
// - **Tenant Management**: Both modules have Tenant, TenantManager, TenantMetadata
// - **Resource Isolation**: Both implement MemoryIsolator, CpuScheduler, IoBandwidthAllocator
// - **Container/PDB**: Both have Container Database and Pluggable Database concepts
// - **Provisioning**: Both have provisioning services and lifecycle management
// - **Metering/Billing**: Duplicate resource tracking and quota enforcement
//
// **Unique to multitenant/** (Oracle-specific, preserve during merge):
// - pdb.rs: Full Oracle PDB lifecycle (MOUNTED, READ_ONLY, MIGRATE modes)
// - cdb.rs: Container Database with Oracle-compatible SQL syntax
// - cloning.rs: Hot cloning with advanced copy-on-write
// - relocation.rs: Live PDB migration with minimal downtime
// - shared.rs: Shared undo/temp tablespaces (Oracle feature)
//
// **Unique to multitenancy/** (General multi-tenancy, currently canonical):
// - isolation.rs: More comprehensive resource isolation with token buckets
// - consolidation.rs: Workload consolidation and affinity rules
// - provisioning.rs: Service tier templates and auto-provisioning
// - container.rs: Simpler, more generic container abstraction
//
// **CANONICAL MODULE**: `src/multitenancy/` is the canonical implementation
// **DEPRECATED MODULE**: This module (`src/multitenant/`) should be considered deprecated
//
// **TODO - HIGH PRIORITY CONSOLIDATION**:
// 1. **Preserve Oracle PDB Features**: Migrate pdb.rs, cdb.rs to multitenancy/
// 2. **Merge Tenant Types**: Consolidate Tenant, TenantManager into single implementation
// 3. **Unify Isolation**: Keep multitenancy/isolation.rs, enhance with Oracle features
// 4. **Consolidate Container**: Merge container concepts from both modules
// 5. **Preserve Unique Features**:
//    - Hot cloning (from multitenant/cloning.rs)
//    - Live relocation (from multitenant/relocation.rs)
//    - Shared services (from multitenant/shared.rs)
//    - Service tiers (from multitenancy/provisioning.rs)
//    - Consolidation planning (from multitenancy/consolidation.rs)
// 6. **Update API**: Provide re-exports for backward compatibility
// 7. **Deprecation Path**:
//    - Add #[deprecated] attributes to all public items in this module
//    - Document migration path for existing users
//    - Remove this module in v2.0 release
//
// **Migration Guide for Users**:
// ```rust
// // OLD (deprecated):
// use rusty_db::multitenant::{ContainerDatabase, PdbConfig};
//
// // NEW (canonical):
// use rusty_db::multitenancy::{ContainerDatabase, PdbConfig};
// ```
//
// **Impact**: 4000+ lines of duplication, API confusion, 2x maintenance burden
// **Priority**: BLOCKER - must consolidate before v1.0 release
// **Estimated Effort**: 2-3 days for careful merge preserving all features
//
// Oracle-compatible Pluggable Database (PDB) and Container Database (CDB) architecture
// with complete tenant isolation, resource governance, hot cloning, and self-service provisioning.
//
// ## Overview
//
// This module implements a comprehensive multi-tenant database architecture inspired by
// Oracle's Multitenant option, allowing multiple isolated database instances (PDBs) to
// run within a single container database (CDB) while sharing common infrastructure.
//
// ## Key Features
//
// - **Container Database (CDB)**: Root container managing multiple PDBs
// - **Pluggable Databases (PDB)**: Fully isolated tenant databases
// - **Resource Isolation**: Per-tenant CPU, memory, I/O, and storage limits
// - **Hot Cloning**: Online PDB cloning with copy-on-write
// - **PDB Relocation**: Live migration with minimal downtime
// - **Shared Services**: Common users, undo, and temp spaces
// - **Metering & Billing**: Resource usage tracking and quota enforcement
//
// ## Architecture
//
// ```text
// ┌─────────────────────────────────────────────────────────────┐
// │                  Container Database (CDB)                    │
// │  ┌──────────────────────────────────────────────────────┐   │
// │  │              CDB Root Container                      │   │
// │  │  - System Metadata                                   │   │
// │  │  - Shared Memory Pools                              │   │
// │  │  - Background Processes                             │   │
// │  │  - Container Registry                               │   │
// │  └──────────────────────────────────────────────────────┘   │
// │                                                              │
// │  ┌────────────┐  ┌────────────┐  ┌────────────┐           │
// │  │   PDB 1    │  │   PDB 2    │  │   PDB 3    │  ...      │
// │  │  (Tenant)  │  │  (Tenant)  │  │  (Tenant)  │           │
// │  │            │  │            │  │            │           │
// │  │ - Data     │  │ - Data     │  │ - Data     │           │
// │  │ - Schema   │  │ - Schema   │  │ - Schema   │           │
// │  │ - Users    │  │ - Users    │  │ - Users    │           │
// │  │ - Limits   │  │ - Limits   │  │ - Limits   │           │
// │  └────────────┘  └────────────┘  └────────────┘           │
// │                                                              │
// │  ┌──────────────────────────────────────────────────────┐   │
// │  │              Shared Services                         │   │
// │  │  - Undo Tablespace                                   │   │
// │  │  - Temp Tablespace                                   │   │
// │  │  - Common Users/Roles                                │   │
// │  │  - Background Workers                                │   │
// │  └──────────────────────────────────────────────────────┘   │
// └─────────────────────────────────────────────────────────────┘
// ```
//
// ## Usage Example
//
// ```rust,no_run
// use rusty_db::multitenant::{ContainerDatabase, PdbConfig, PdbLifecycleState};
// use rusty_db::Result;
//
// async fn example() -> Result<()> {
//     // Create a Container Database
//     let mut cdb = ContainerDatabase::new("PROD_CDB").await?;
//
//     // Create a Pluggable Database for a tenant
//     let pdb_config = PdbConfig::builder()
//         .name("TENANT1_PDB")
//         .admin_user("admin")
//         .admin_password("secure_password")
//         .storage_quota(10 * 1024 * 1024 * 1024) // 10 GB
//         .build();
//
//     let pdb_id = cdb.create_pdb(pdb_config).await?;
//
//     // Open the PDB
//     cdb.open_pdb(pdb_id).await?;
//
//     // Clone the PDB (hot clone)
//     let clone_id = cdb.clone_pdb(pdb_id, "TENANT1_DEV").await?;
//
//     Ok(())
// }
// ```
//
// ## Innovations
//
// - **Kubernetes-Native**: Seamless integration with Kubernetes operators
// - **Serverless Scaling**: Auto-scale PDBs based on workload
// - **Cross-Cloud Portability**: Migrate PDBs across cloud providers
// - **AI-Driven Optimization**: ML-based resource allocation and consolidation

// ============================================================================
// Module Declarations
// ============================================================================

// Container Database (CDB) management
//
// Provides the root container that manages multiple Pluggable Databases (PDBs),
// system-level metadata, shared memory pools, and background processes.
use std::fmt;
pub mod cdb;

// Pluggable Database (PDB) management
//
// Implements PDB lifecycle operations including creation, cloning, plugging,
// opening, closing, and deletion. Supports application containers and seed PDBs.
pub mod pdb;

// Resource isolation and governance
//
// Per-tenant resource limits including memory, CPU, I/O bandwidth, connections,
// temp space, and storage quotas. Enforces fair-share scheduling and QoS.
pub mod isolation;

// Tenant management and provisioning
//
// High-level tenant lifecycle management including onboarding workflows,
// metadata management, configuration, backup/restore, and cross-tenant queries.
pub mod tenant;

// Hot cloning capabilities
//
// Online PDB cloning with copy-on-write, thin cloning, snapshot cloning,
// refreshable clones, and cloning from backup.
pub mod cloning;

// PDB relocation and migration
//
// Online PDB relocation with minimal downtime, cross-CDB migration,
// connection draining, and state transfer protocol.
pub mod relocation;

// Shared services and common objects
//
// Shared undo tablespace, temp tablespace, common users and roles,
// application common objects, and lockdown profiles.
pub mod shared;

// Metering, billing, and quota enforcement
//
// Resource usage tracking, per-tenant metrics, usage reports,
// billing integration hooks, and quota enforcement.
pub mod metering;

// ============================================================================
// Re-exports
// ============================================================================

pub use cdb::{
    BackgroundProcessManager, CdbConfig, CdbRegistry, CdbResourcePool, ContainerDatabase,
    SystemMetadata,
};

pub use pdb::{
    ApplicationContainer, PdbConfig, PdbCreateMode, PdbId, PdbLifecycleState, PdbMode, PdbSnapshot,
    PluggableDatabase, SeedPdb,
};

pub use isolation::{
    ConnectionLimiter, CpuScheduler, IoBandwidthAllocator, MemoryIsolator, QosPolicy,
    ResourceIsolator, ResourceLimits, StorageQuotaManager, TempSpaceLimiter,
};

pub use tenant::{
    CrossTenantQueryEngine, Tenant, TenantBackupManager, TenantConfig, TenantMetadata,
    TenantOnboardingWorkflow, TenantProvisioningService,
};

pub use cloning::{
    CloneFromBackup, CloneType, CloningEngine, CopyOnWriteEngine, RefreshableClone, SnapshotClone,
    ThinClone,
};

pub use relocation::{
    ConnectionDrainer, CrossCdbMigrator, RelocationConfig, RelocationEngine, RelocationState,
    StateTransferProtocol,
};

pub use shared::{
    ApplicationCommonObject, CommonRole, CommonUser, LockdownProfile, SharedServices,
    TempTablespace, UndoTablespace,
};

pub use metering::{
    BillingIntegration, MeteringEngine, QuotaEnforcer, ResourceQuota, ResourceUsageTracker,
    TenantMetrics, UsageReport,
};

// ============================================================================
// Common Types and Traits
// ============================================================================

use crate::error::Result;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;

// Unique identifier for a tenant
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TenantId(pub u64);

impl TenantId {
    // Create a new tenant ID
    pub fn new(id: u64) -> Self {
        Self(id)
    }

    // Get the underlying ID value
    pub fn value(&self) -> u64 {
        self.0
    }
}

impl fmt::Display for TenantId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "TenantId({})", self.0)
    }
}

// Multi-tenant capability trait
//
// Types implementing this trait support multi-tenant operations
pub trait MultiTenant {
    // Get the tenant ID associated with this instance
    fn tenant_id(&self) -> TenantId;

    // Check if this instance belongs to a specific tenant
    fn belongs_to(&self, tenant_id: TenantId) -> bool {
        self.tenant_id() == tenant_id
    }

    // Isolate resources for this tenant
    fn isolate_resources(&mut self) -> Result<()>;

    // Get tenant-specific configuration
    fn tenant_config(&self) -> &dyn std::any::Any;
}

// Resource consumption metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceConsumption {
    // Memory usage in bytes
    pub memory_bytes: u64,

    // CPU time in microseconds
    pub cpu_micros: u64,

    // I/O bytes read
    pub io_read_bytes: u64,

    // I/O bytes written
    pub io_write_bytes: u64,

    // Number of active connections
    pub active_connections: u32,

    // Storage used in bytes
    pub storage_bytes: u64,

    // Temporary space used in bytes
    pub temp_bytes: u64,
}

impl ResourceConsumption {
    // Create a new resource consumption metric with zero values
    pub fn zero() -> Self {
        Self {
            memory_bytes: 0,
            cpu_micros: 0,
            io_read_bytes: 0,
            io_write_bytes: 0,
            active_connections: 0,
            storage_bytes: 0,
            temp_bytes: 0,
        }
    }

    // Add another consumption metric to this one
    pub fn add(&mut self, other: &ResourceConsumption) {
        self.memory_bytes += other.memory_bytes;
        self.cpu_micros += other.cpu_micros;
        self.io_read_bytes += other.io_read_bytes;
        self.io_write_bytes += other.io_write_bytes;
        self.active_connections += other.active_connections;
        self.storage_bytes += other.storage_bytes;
        self.temp_bytes += other.temp_bytes;
    }
}

// Multi-tenant database instance
//
// Main entry point for the multi-tenant architecture engine
#[derive(Clone)]
pub struct MultiTenantDatabase {
    // The container database instance
    cdb: Arc<RwLock<ContainerDatabase>>,

    // Tenant provisioning service
    provisioning: Arc<TenantProvisioningService>,

    // Resource isolation manager
    isolation: Arc<ResourceIsolator>,

    // Metering engine
    metering: Arc<MeteringEngine>,

    // Shared services
    shared: Arc<SharedServices>,
}

impl MultiTenantDatabase {
    // Create a new multi-tenant database instance
    pub async fn new(name: &str) -> Result<Self> {
        let cdb = Arc::new(RwLock::new(ContainerDatabase::new(name).await?));
        let provisioning = Arc::new(TenantProvisioningService::new());
        let isolation = Arc::new(ResourceIsolator::new());
        let metering = Arc::new(MeteringEngine::new());
        let shared = Arc::new(SharedServices::new());

        Ok(Self {
            cdb,
            provisioning,
            isolation,
            metering,
            shared,
        })
    }

    // Get the container database
    pub fn cdb(&self) -> Arc<RwLock<ContainerDatabase>> {
        self.cdb.clone()
    }

    // Get the provisioning service
    pub fn provisioning(&self) -> Arc<TenantProvisioningService> {
        self.provisioning.clone()
    }

    // Get the isolation manager
    pub fn isolation(&self) -> Arc<ResourceIsolator> {
        self.isolation.clone()
    }

    // Get the metering engine
    pub fn metering(&self) -> Arc<MeteringEngine> {
        self.metering.clone()
    }

    // Get shared services
    pub fn shared(&self) -> Arc<SharedServices> {
        self.shared.clone()
    }

    // Create a new tenant
    pub async fn create_tenant(&self, config: TenantConfig) -> Result<TenantId> {
        self.provisioning.provision_tenant(config).await
    }

    // Delete a tenant
    pub async fn delete_tenant(&self, tenant_id: TenantId) -> Result<()> {
        self.provisioning.deprovision_tenant(tenant_id).await
    }

    // Get tenant metrics
    pub async fn tenant_metrics(&self, tenant_id: TenantId) -> Result<TenantMetrics> {
        self.metering.get_tenant_metrics(tenant_id).await
    }
}

// ============================================================================
// Constants
// ============================================================================

// Maximum number of PDBs per CDB
pub const MAX_PDBS_PER_CDB: usize = 4096;

// Default memory allocation per PDB (512 MB)
pub const DEFAULT_PDB_MEMORY_MB: u64 = 512;

// Default storage quota per PDB (10 GB)
pub const DEFAULT_PDB_STORAGE_GB: u64 = 10;

// Default CPU shares per PDB
pub const DEFAULT_CPU_SHARES: u32 = 100;

// Default connection limit per PDB
pub const DEFAULT_CONNECTION_LIMIT: u32 = 100;

// Default I/O bandwidth limit per PDB (100 MB/s)
pub const DEFAULT_IO_BANDWIDTH_MBPS: u32 = 100;

// Seed PDB name
pub const SEED_PDB_NAME: &str = "PDB$SEED";

// Root container name
pub const ROOT_CONTAINER_NAME: &str = "CDB$ROOT";

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tenant_id() {
        let id1 = TenantId::new(1);
        let id2 = TenantId::new(1);
        let id3 = TenantId::new(2);

        assert_eq!(id1, id2);
        assert_ne!(id1, id3);
        assert_eq!(id1.value(), 1);
    }

    #[test]
    fn test_resource_consumption() {
        let mut rc1 = ResourceConsumption::zero();
        let rc2 = ResourceConsumption {
            memory_bytes: 1024,
            cpu_micros: 1000,
            io_read_bytes: 512,
            io_write_bytes: 256,
            active_connections: 5,
            storage_bytes: 2048,
            temp_bytes: 128,
        };

        rc1.add(&rc2);
        assert_eq!(rc1.memory_bytes, 1024);
        assert_eq!(rc1.cpu_micros, 1000);
    }

    #[tokio::test]
    async fn test_multitenant_database_creation() {
        let result = MultiTenantDatabase::new("TEST_CDB").await;
        assert!(result.is_ok());
    }
}
