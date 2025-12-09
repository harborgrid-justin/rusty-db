// Tenant management with isolation guarantees and resource governance
// Provides tenant-level isolation, resource controls, and SLA enforcement

use std::fmt;
use std::collections::BTreeMap;
use std::collections::VecDeque;
use std::collections::{HashMap};
use std::sync::Arc;
use std::path::PathBuf;
use tokio::sync::RwLock;
use serde::{Deserialize, Serialize};
use std::time::{SystemTime};

/// Tenant-specific errors
#[derive(Debug, Clone)]
pub enum TenantError {
    TenantNotFound(String),
    TenantAlreadyExists(String),
    InvalidState(String),
    ResourceLimitExceeded(String),
    QuotaExceeded(String),
    IsolationViolation(String),
    SlaViolation(String),
    PermissionDenied(String),
    InvalidConfiguration(String),
}

impl std::fmt::Display for TenantError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TenantError::TenantNotFound(id) => write!(f, "Tenant not found: {}", id),
            TenantError::TenantAlreadyExists(id) => write!(f, "Tenant already exists: {}", id),
            TenantError::InvalidState(msg) => write!(f, "Invalid state: {}", msg),
            TenantError::ResourceLimitExceeded(msg) => write!(f, "Resource limit exceeded: {}", msg),
            TenantError::QuotaExceeded(msg) => write!(f, "Quota exceeded: {}", msg),
            TenantError::IsolationViolation(msg) => write!(f, "Isolation violation: {}", msg),
            TenantError::SlaViolation(msg) => write!(f, "SLA violation: {}", msg),
            TenantError::PermissionDenied(msg) => write!(f, "Permission denied: {}", msg),
            TenantError::InvalidConfiguration(msg) => write!(f, "Invalid configuration: {}", msg),
        }
    }
}

impl std::error::Error for TenantError {}

pub type TenantResult<T> = Result<T, TenantError>;

/// Tenant operational state
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TenantState {
    Active,
    Suspended,
    Maintenance,
    Migrating,
    Terminated,
}

/// Tenant priority levels for resource allocation
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum TenantPriority {
    Critical = 4,    // Highest priority, guaranteed resources
    High = 3,        // High priority, preferential treatment
    Medium = 2,      // Standard priority
    Low = 1,         // Low priority, best effort
    BestEffort = 0,  // Lowest priority, use spare resources only
}

/// Service tier defining resource allocation and SLA
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceTier {
    pub tier_name: String,
    pub cpu_cores: f64,
    pub memory_mb: u64,
    pub storage_gb: u64,
    pub iops_limit: u32,
    pub network_mbps: u32,
    pub max_connections: u32,
    pub backup_retention_days: u32,
    pub sla_uptime_percent: f64,
    pub sla_response_time_ms: u64,
    pub monthly_cost: f64,
}

impl ServiceTier {
    pub fn bronze() -> Self {
        Self {
            tier_name: "Bronze".to_string(),
            cpu_cores: 1.0,
            memory_mb: 2048,
            storage_gb: 50,
            iops_limit: 1000,
            network_mbps: 100,
            max_connections: 50,
            backup_retention_days: 7,
            sla_uptime_percent: 99.0,
            sla_response_time_ms: 100,
            monthly_cost: 100.0,
        }
    }

    pub fn silver() -> Self {
        Self {
            tier_name: "Silver".to_string(),
            cpu_cores: 2.0,
            memory_mb: 4096,
            storage_gb: 100,
            iops_limit: 3000,
            network_mbps: 250,
            max_connections: 100,
            backup_retention_days: 14,
            sla_uptime_percent: 99.5,
            sla_response_time_ms: 50,
            monthly_cost: 250.0,
        }
    }

    pub fn gold() -> Self {
        Self {
            tier_name: "Gold".to_string(),
            cpu_cores: 4.0,
            memory_mb: 8192,
            storage_gb: 250,
            iops_limit: 10000,
            network_mbps: 500,
            max_connections: 250,
            backup_retention_days: 30,
            sla_uptime_percent: 99.9,
            sla_response_time_ms: 25,
            monthly_cost: 500.0,
        }
    }

    pub fn platinum() -> Self {
        Self {
            tier_name: "Platinum".to_string(),
            cpu_cores: 8.0,
            memory_mb: 16384,
            storage_gb: 500,
            iops_limit: 25000,
            network_mbps: 1000,
            max_connections: 500,
            backup_retention_days: 90,
            sla_uptime_percent: 99.99,
            sla_response_time_ms: 10,
            monthly_cost: 1000.0,
        }
    }
}

/// Resource quota enforcement
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceQuota {
    pub cpu_percent: u32,
    pub memory_mb: u64,
    pub storage_gb: u64,
    pub iops: u32,
    pub network_bandwidth_mbps: u32,
    pub max_sessions: u32,
    pub max_transactions_per_sec: u32,
}

/// Current resource usage
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceUsage {
    pub cpu_percent: f64,
    pub memory_mb: u64,
    pub storage_used_gb: u64,
    pub current_iops: u32,
    pub network_bandwidth_mbps: u32,
    pub active_sessions: u32,
    pub transactions_per_sec: f64,
    pub last_updated: SystemTime,
}

impl Default for ResourceUsage {
    fn default() -> Self {
        Self {
            cpu_percent: 0.0,
            memory_mb: 0,
            storage_used_gb: 0,
            current_iops: 0,
            network_bandwidth_mbps: 0,
            active_sessions: 0,
            transactions_per_sec: 0.0,
            last_updated: SystemTime::now(),
        }
    }
}

/// Tenant-specific tablespace
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TenantTablespace {
    pub tablespace_name: String,
    pub datafile_path: PathBuf,
    pub size_mb: u64,
    pub used_mb: u64,
    pub auto_extend: bool,
    pub max_size_mb: u64,
    pub block_size: u32,
    pub is_encrypted: bool,
}

/// Tenant-specific schema
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TenantSchema {
    pub schema_name: String,
    pub owner: String,
    pub creation_time: SystemTime,
    pub tablespaces: Vec<String>,
    pub object_count: u64,
    pub size_mb: u64,
}

/// SLA metrics and compliance tracking
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SlaMetrics {
    pub uptime_percent: f64,
    pub avg_response_time_ms: f64,
    pub p95_response_time_ms: f64,
    pub p99_response_time_ms: f64,
    pub error_rate_percent: f64,
    pub sla_violations: Vec<SlaViolation>,
    pub measurement_period_start: SystemTime,
    pub measurement_period_end: SystemTime,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SlaViolation {
    pub violation_type: String,
    pub timestamp: SystemTime,
    pub severity: ViolationSeverity,
    pub description: String,
    pub remediation: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ViolationSeverity {
    Critical,
    High,
    Medium,
    Low,
}

/// Tenant metadata and configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TenantMetadata {
    pub tenant_id: String,
    pub tenant_name: String,
    pub description: String,
    pub owner_email: String,
    pub organization: String,
    pub department: String,
    pub cost_center: String,
    pub tags: HashMap<String, String>,
    pub creation_time: SystemTime,
    pub last_modified: SystemTime,
}

/// Tenant statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TenantStatistics {
    pub total_queries: u64,
    pub total_transactions: u64,
    pub total_reads: u64,
    pub total_writes: u64,
    pub total_errors: u64,
    pub avg_query_time_ms: f64,
    pub peak_connections: u32,
    pub data_transferred_mb: u64,
    pub cache_hit_ratio: f64,
    pub collection_start: SystemTime,
}

impl Default for TenantStatistics {
    fn default() -> Self {
        Self {
            total_queries: 0,
            total_transactions: 0,
            total_reads: 0,
            total_writes: 0,
            total_errors: 0,
            avg_query_time_ms: 0.0,
            peak_connections: 0,
            data_transferred_mb: 0,
            cache_hit_ratio: 0.0,
            collection_start: SystemTime::now(),
        }
    }
}

/// Query history for cross-tenant prevention
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryHistoryEntry {
    pub query_id: u64,
    pub query_text: String,
    pub schemas_accessed: Vec<String>,
    pub start_time: SystemTime,
    pub duration_ms: u64,
    pub status: QueryStatus,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum QueryStatus {
    Running,
    Completed,
    Failed,
    Blocked,
}

/// Tenant structure with complete isolation
pub struct Tenant {
    pub tenant_id: String,
    pub metadata: Arc<RwLock<TenantMetadata>>,
    pub state: Arc<RwLock<TenantState>>,
    pub priority: Arc<RwLock<TenantPriority>>,
    pub service_tier: Arc<RwLock<ServiceTier>>,
    pub quota: Arc<RwLock<ResourceQuota>>,
    pub usage: Arc<RwLock<ResourceUsage>>,
    pub tablespaces: Arc<RwLock<HashMap<String, TenantTablespace>>>,
    pub schemas: Arc<RwLock<HashMap<String, TenantSchema>>>,
    pub statistics: Arc<RwLock<TenantStatistics>>,
    pub sla_metrics: Arc<RwLock<SlaMetrics>>,
    pub query_history: Arc<RwLock<VecDeque<QueryHistoryEntry>>>,
    pub allowed_schemas: Arc<RwLock<Vec<String>>>,
    pub blocked_tenants: Arc<RwLock<Vec<String>>>,
    pub resource_governor: Arc<RwLock<ResourceGovernor>>,
}

/// Resource governor for fine-grained control
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceGovernor {
    pub cpu_allocation: CpuAllocation,
    pub memory_allocation: MemoryAllocation,
    pub io_allocation: IoAllocation,
    pub parallel_degree_limit: u32,
    pub max_execution_time_sec: u64,
    pub max_idle_time_sec: u64,
    pub max_parse_time_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CpuAllocation {
    pub min_percent: u32,
    pub max_percent: u32,
    pub shares: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryAllocation {
    pub min_mb: u64,
    pub max_mb: u64,
    pub sort_area_mb: u64,
    pub hash_area_mb: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IoAllocation {
    pub min_iops: u32,
    pub max_iops: u32,
    pub min_mbps: u32,
    pub max_mbps: u32,
}

impl Tenant {
    /// Create a new tenant with specified tier
    pub fn new(
        tenant_id: String,
        tenant_name: String,
        owner_email: String,
        service_tier: ServiceTier,
    ) -> Self {
        let metadata = TenantMetadata {
            tenant_id: tenant_id.clone(),
            tenant_name,
            description: String::new(),
            owner_email,
            organization: String::new(),
            department: String::new(),
            cost_center: String::new(),
            tags: HashMap::new(),
            creation_time: SystemTime::now(),
            last_modified: SystemTime::now(),
        };

        let quota = ResourceQuota {
            cpu_percent: (service_tier.cpu_cores * 100.0) as u32,
            memory_mb: service_tier.memory_mb,
            storage_gb: service_tier.storage_gb,
            iops: service_tier.iops_limit,
            network_bandwidth_mbps: service_tier.network_mbps,
            max_sessions: service_tier.max_connections,
            max_transactions_per_sec: service_tier.iops_limit / 10,
        };

        let resource_governor = ResourceGovernor {
            cpu_allocation: CpuAllocation {
                min_percent: 10,
                max_percent: (service_tier.cpu_cores * 100.0) as u32,
                shares: 1000,
            },
            memory_allocation: MemoryAllocation {
                min_mb: service_tier.memory_mb / 4,
                max_mb: service_tier.memory_mb,
                sort_area_mb: 64,
                hash_area_mb: 64,
            },
            io_allocation: IoAllocation {
                min_iops: service_tier.iops_limit / 4,
                max_iops: service_tier.iops_limit,
                min_mbps: service_tier.network_mbps / 4,
                max_mbps: service_tier.network_mbps,
            },
            parallel_degree_limit: (service_tier.cpu_cores * 2.0) as u32,
            max_execution_time_sec: 3600,
            max_idle_time_sec: 1800,
            max_parse_time_ms: 5000,
        };

        let sla_metrics = SlaMetrics {
            uptime_percent: 100.0,
            avg_response_time_ms: 0.0,
            p95_response_time_ms: 0.0,
            p99_response_time_ms: 0.0,
            error_rate_percent: 0.0,
            sla_violations: Vec::new(),
            measurement_period_start: SystemTime::now(),
            measurement_period_end: SystemTime::now(),
        };

        Self {
            tenant_id,
            metadata: Arc::new(RwLock::new(metadata)),
            state: Arc::new(RwLock::new(TenantState::Active)),
            priority: Arc::new(RwLock::new(TenantPriority::Medium)),
            service_tier: Arc::new(RwLock::new(service_tier)),
            quota: Arc::new(RwLock::new(quota)),
            usage: Arc::new(RwLock::new(ResourceUsage::default())),
            tablespaces: Arc::new(RwLock::new(HashMap::new())),
            schemas: Arc::new(RwLock::new(HashMap::new())),
            statistics: Arc::new(RwLock::new(TenantStatistics::default())),
            sla_metrics: Arc::new(RwLock::new(sla_metrics)),
            query_history: Arc::new(RwLock::new(VecDeque::new())),
            allowed_schemas: Arc::new(RwLock::new(Vec::new())),
            blocked_tenants: Arc::new(RwLock::new(Vec::new())),
            resource_governor: Arc::new(RwLock::new(resource_governor)),
        }
    }

    /// Suspend tenant operations
    pub async fn suspend(&self, reason: String) -> TenantResult<()> {
        let mut state = self.state.write().await;

        if *state == TenantState::Terminated {
            return Err(TenantError::InvalidState("Cannot suspend terminated tenant".to_string()));
        }

        *state = TenantState::Suspended;

        // Log suspension
        println!("Tenant {} suspended: {}", self.tenant_id, reason);

        Ok(())
    }

    /// Resume tenant operations
    pub async fn resume(&self) -> TenantResult<()> {
        let mut state = self.state.write().await;

        if *state != TenantState::Suspended {
            return Err(TenantError::InvalidState("Tenant is not suspended".to_string()));
        }

        *state = TenantState::Active;

        println!("Tenant {} resumed", self.tenant_id);

        Ok(())
    }

    /// Create tenant-specific tablespace
    pub async fn create_tablespace(
        &self,
        tablespace_name: String,
        size_mb: u64,
        is_encrypted: bool,
    ) -> TenantResult<()> {
        let quota = self.quota.read().await;
        let usage = self.usage.read().await;

        // Check quota
        let projected_usage = usage.storage_used_gb + (size_mb / 1024);
        if projected_usage > quota.storage_gb {
            return Err(TenantError::QuotaExceeded(
                format!("Storage quota exceeded: {} GB > {} GB", projected_usage, quota.storage_gb)
            )));
        }

        drop(quota);
        drop(usage);

        let tablespace = TenantTablespace {
            tablespace_name: tablespace_name.clone(),
            datafile_path: PathBuf::from(format!("/data/{}/{}.dbf", self.tenant_id, tablespace_name)),
            size_mb,
            used_mb: 0,
            auto_extend: true,
            max_size_mb: size_mb * 2,
            block_size: 8192,
            is_encrypted,
        });

        let mut tablespaces = self.tablespaces.write().await;
        tablespaces.insert(tablespace_name, tablespace);

        Ok(())
    }

    /// Create tenant-specific schema
    pub async fn create_schema(
        &self,
        schema_name: String,
        owner: String,
    ) -> TenantResult<()> {
        let schemas = self.schemas.read().await;

        if schemas.contains_key(&schema_name) {
            return Err(TenantError::InvalidConfiguration(
                format!("Schema {} already exists", schema_name)
            )));
        }

        drop(schemas);

        let schema = TenantSchema {
            schema_name: schema_name.clone(),
            owner,
            creation_time: SystemTime::now(),
            tablespaces: Vec::new(),
            object_count: 0,
            size_mb: 0,
        };

        let mut schemas = self.schemas.write().await;
        schemas.insert(schema_name.clone(), schema);

        // Add to allowed schemas
        let mut allowed = self.allowed_schemas.write().await;
        allowed.push(schema_name);

        Ok(())
    }

    /// Validate query doesn't access other tenants' data
    pub async fn validate_query(&self, query: &str, schemas_accessed: &[String]) -> TenantResult<()> {
        let allowed = self.allowed_schemas.read().await;

        for schema in schemas_accessed {
            if !allowed.contains(schema) {
                return Err(TenantError::IsolationViolation(
                    format!("Access to schema {} not allowed for tenant {}", schema, self.tenant_id)
                )));
            }
        }

        Ok(())
    }

    /// Check if resource allocation is within quota
    pub async fn check_resource_quota(&self, resource_type: ResourceType, amount: u64) -> TenantResult<bool> {
        let quota = self.quota.read().await;
        let usage = self.usage.read().await;

        let within_quota = match resource_type {
            ResourceType::Cpu => {
                usage.cpu_percent + (amount as f64) <= quota.cpu_percent as f64
            }
            ResourceType::Memory => {
                usage.memory_mb + amount <= quota.memory_mb
            }
            ResourceType::Storage => {
                usage.storage_used_gb + amount <= quota.storage_gb
            }
            ResourceType::Iops => {
                usage.current_iops + (amount as u32) <= quota.iops
            }
            ResourceType::Sessions => {
                usage.active_sessions + (amount as u32) <= quota.max_sessions
            }
        };

        if !within_quota {
            return Err(TenantError::ResourceLimitExceeded(
                format!("{:?} limit exceeded for tenant {}", resource_type, self.tenant_id)
            )));
        }

        Ok(true)
    }

    /// Update resource usage
    pub async fn update_usage(&self, cpu: f64, memory: u64, iops: u32, sessions: u32) {
        let mut usage = self.usage.write().await;
        usage.cpu_percent = cpu;
        usage.memory_mb = memory;
        usage.current_iops = iops;
        usage.active_sessions = sessions;
        usage.last_updated = SystemTime::now();
    }

    /// Record query in history
    pub async fn record_query(
        &self,
        query_text: String,
        schemas_accessed: Vec<String>,
        duration_ms: u64,
        status: QueryStatus,
    ) {
        let query_id = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_micros() as u64;

        let entry = QueryHistoryEntry {
            query_id,
            query_text,
            schemas_accessed,
            start_time: SystemTime::now(),
            duration_ms,
            status,
        };

        let mut history = self.query_history.write().await;

        // Keep last 1000 queries
        if history.len() >= 1000 {
            history.pop_front();
        }

        history.push_back(entry);

        // Update statistics
        let mut stats = self.statistics.write().await;
        stats.total_queries += 1;

        if status == QueryStatus::Completed {
            // Update average query time
            let total_time = stats.avg_query_time_ms * (stats.total_queries - 1) as f64;
            stats.avg_query_time_ms = (total_time + duration_ms as f64) / stats.total_queries as f64;
        } else if status == QueryStatus::Failed {
            stats.total_errors += 1;
        }
    }

    /// Check SLA compliance
    pub async fn check_sla_compliance(&self) -> TenantResult<bool> {
        let sla_metrics = self.sla_metrics.read().await;
        let service_tier = self.service_tier.read().await;

        let mut compliant = true;
        let mut violations = Vec::new();

        // Check uptime
        if sla_metrics.uptime_percent < service_tier.sla_uptime_percent {
            compliant = false;
            violations.push(SlaViolation {
                violation_type: "Uptime".to_string(),
                timestamp: SystemTime::now(),
                severity: ViolationSeverity::Critical,
                description: format!(
                    "Uptime {}% below SLA {}%",
                    sla_metrics.uptime_percent, service_tier.sla_uptime_percent
                ),
                remediation: "Investigate service interruptions".to_string(),
            }));
        }

        // Check response time
        if sla_metrics.avg_response_time_ms > service_tier.sla_response_time_ms as f64 {
            compliant = false;
            violations.push(SlaViolation {
                violation_type: "ResponseTime".to_string(),
                timestamp: SystemTime::now(),
                severity: ViolationSeverity::High,
                description: format!(
                    "Response time {}ms exceeds SLA {}ms",
                    sla_metrics.avg_response_time_ms, service_tier.sla_response_time_ms
                ),
                remediation: "Optimize query performance or upgrade tier".to_string(),
            }));
        }

        if !compliant {
            drop(sla_metrics);
            drop(service_tier);

            let mut sla_metrics = self.sla_metrics.write().await;
            sla_metrics.sla_violations.extend(violations);
        }

        Ok(compliant)
    }

    /// Upgrade service tier
    pub async fn upgrade_tier(&self, newtier: ServiceTier) -> TenantResult<()> {
        let current_tier = self.service_tier.read().await;

        if new_tier.monthly_cost <= current_tier.monthly_cost {
            return Err(TenantError::InvalidConfiguration(
                "New tier must be higher than current tier".to_string()
            ));
        }

        drop(current_tier);

        // Update tier
        let mut tier = self.service_tier.write().await;
        *tier = new_tier.clone();
        drop(tier);

        // Update quota
        let mut quota = self.quota.write().await;
        quota.cpu_percent = (new_tier.cpu_cores * 100.0) as u32;
        quota.memory_mb = new_tier.memory_mb;
        quota.storage_gb = new_tier.storage_gb;
        quota.iops = new_tier.iops_limit;
        quota.network_bandwidth_mbps = new_tier.network_mbps;
        quota.max_sessions = new_tier.max_connections;

        Ok(())
    }

    /// Set tenant priority
    pub async fn set_priority(&self, priority: TenantPriority) -> TenantResult<()> {
        let mut current_priority = self.priority.write().await;
        *current_priority = priority;

        Ok(())
    }

    /// Get tenant statistics
    pub async fn get_statistics(&self) -> TenantStatistics {
        self.statistics.read().await.clone()
    }

    /// Get SLA metrics
    pub async fn get_sla_metrics(&self) -> SlaMetrics {
        self.sla_metrics.read().await.clone()
    }

    /// Export tenant configuration
    pub async fn export_config(&self) -> TenantConfig {
        TenantConfig {
            tenant_id: self.tenant_id.clone(),
            metadata: self.metadata.read().await.clone(),
            state: *self.state.read().await,
            priority: *self.priority.read().await,
            service_tier: self.service_tier.read().await.clone(),
            quota: self.quota.read().await.clone(),
        }
    }

    /// Terminate tenant
    pub async fn terminate(&self) -> TenantResult<()> {
        let mut state = self.state.write().await;
        *state = TenantState::Terminated;

        // Clear resources
        self.tablespaces.write().await.clear();
        self.schemas.write().await.clear();
        self.query_history.write().await.clear();

        println!("Tenant {} terminated", self.tenant_id);

        Ok(())
    }
}

/// Resource type enumeration
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResourceType {
    Cpu,
    Memory,
    Storage,
    Iops,
    Sessions,
}

/// Tenant configuration for export/import
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TenantConfig {
    pub tenant_id: String,
    pub metadata: TenantMetadata,
    pub state: TenantState,
    pub priority: TenantPriority,
    pub service_tier: ServiceTier,
    pub quota: ResourceQuota,
}

/// Tenant manager for managing multiple tenants
pub struct TenantManager {
    tenants: Arc<RwLock<HashMap<String, Arc<Tenant>>>>,
    tenant_index: Arc<RwLock<BTreeMap<TenantPriority, Vec<String>>>>,
}

impl TenantManager {
    pub fn new() -> Self {
        Self {
            tenants: Arc::new(RwLock::new(HashMap::new())),
            tenant_index: Arc::new(RwLock::new(BTreeMap::new())),
        }
    }

    /// Register a new tenant
    pub async fn register_tenant(&self, tenant: Arc<Tenant>) -> TenantResult<()> {
        let tenant_id = tenant.tenant_id.clone();
        let priority = *tenant.priority.read().await;

        let mut tenants = self.tenants.write().await;

        if tenants.contains_key(&tenant_id) {
            return Err(TenantError::TenantAlreadyExists(tenant_id));
        }

        tenants.insert(tenant_id.clone(), tenant);

        drop(tenants);

        // Update index
        let mut index = self.tenant_index.write().await;
        index.entry(priority).or_insert_with(Vec::new).push(tenant_id);

        Ok(())
    }

    /// Get tenant by ID
    pub async fn get_tenant(&self, tenant_id: &str) -> TenantResult<Arc<Tenant>> {
        let tenants = self.tenants.read().await;
        tenants
            .get(tenant_id)
            .cloned()
            .ok_or_else(|| TenantError::TenantNotFound(tenant_id.to_string()))
    }

    /// Remove tenant
    pub async ffn remove_tenant(&self, tenantid: &str)-> TenantResult<()> {
        let mut tenants = self.tenants.write().await;

        let tenant = tenants.remove(tenant_id)
            .ok_or_else(|| TenantError::TenantNotFound(tenant_id.to_string()))?;

        let priority = *tenant.priority.read().await;

        drop(tenants);

        // Update index
        let mut index = self.tenant_index.write().await;
        if let Some(tenant_list) = index.get_mut(&priority) {
            tenant_list.retain(|id| id != tenant_id);
        }

        Ok(())
    }

    /// List all tenants
    pub async fn list_tenants(&self) -> Vec<String> {
        let tenants = self.tenants.read().await;
        tenants.keys().cloned().collect()
    }

    /// List tenants by priority
    pub async fn list_by_priority(&self, priority: TenantPriority) -> Vec<String> {
        let index = self.tenant_index.read().await;
        index.get(&priority).cloned().unwrap_or_default()
    }

    /// Get active tenant count
    pub async fn active_count(&self) -> usize {
        let tenants = self.tenants.read().await;
        let mut count = 0;

        for tenant in tenants.values() {
            if *tenant.state.read().await == TenantState::Active {
                count += 1;
            }
        }

        count
    }
}

impl Default for TenantManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
use std::time::UNIX_EPOCH;

    #[tokio::test]
    async fn test_create_tenant() {
        let tenant = Tenant::new(
            "tenant1".to_string(),
            "Test Tenant".to_string(),
            "admin@example.com".to_string(),
            ServiceTier::silver(),
        );

        assert_eq!(tenant.tenant_id, "tenant1");
        let state = tenant.state.read().await;
        assert_eq!(*state, TenantState::Active);
    }

    #[tokio::test]
    async fn test_suspend_resume() {
        let tenant = Tenant::new(
            "tenant1".to_string(),
            "Test Tenant".to_string(),
            "admin@example.com".to_string(),
            ServiceTier::bronze(),
        );

        let result = tenant.suspend("Testing".to_string()).await;
        assert!(result.is_ok());

        let state = tenant.state.read().await;
        assert_eq!(*state, TenantState::Suspended);
        drop(state);

        let result = tenant.resume().await;
        assert!(result.is_ok());

        let state = tenant.state.read().await;
        assert_eq!(*state, TenantState::Active);
    }

    #[tokio::test]
    async fn test_resource_quota() {
        let tenant = Tenant::new(
            "tenant1".to_string(),
            "Test Tenant".to_string(),
            "admin@example.com".to_string(),
            ServiceTier::bronze(),
        );

        let result = tenant.check_resource_quota(ResourceType::Memory, 1024).await;
        assert!(result.is_ok());

        let result = tenant.check_resource_quota(ResourceType::Memory, 10000).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_tenant_manager() {
        let manager = TenantManager::new();

        let tenant1 = Arc::new(Tenant::new(
            "tenant1".to_string(),
            "Tenant 1".to_string(),
            "admin1@example.com".to_string(),
            ServiceTier::silver(),
        ));

        let result = manager.register_tenant(tenant1).await;
        assert!(result.is_ok());

        let tenant = manager.get_tenant("tenant1").await;
        assert!(tenant.is_ok());

        let count = manager.active_count().await;
        assert_eq!(count, 1);
    }
}


