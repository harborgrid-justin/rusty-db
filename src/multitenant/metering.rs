// # Metering, Billing, and Quota Enforcement
//
// Resource usage tracking, per-tenant metrics, usage reports,
// billing integration hooks, and quota enforcement.
//
// ## Features
//
// - **Resource Usage Tracking**: Monitor all tenant resource consumption
// - **Per-Tenant Metrics**: Detailed metrics per tenant
// - **Usage Reports**: Generate billing-ready usage reports
// - **Billing Integration**: Hooks for external billing systems
// - **Quota Enforcement**: Enforce resource quotas
// - **Cost Estimation**: Predict costs based on usage patterns
// - **Usage Alerts**: Notify when approaching limits
//
// ## Architecture
//
// Metering runs continuously in the background:
// - Collectors: Gather metrics from various subsystems
// - Aggregators: Roll up metrics into time windows
// - Exporters: Send data to billing systems
// - Enforcers: Apply quota limits

use std::time::{SystemTime, UNIX_EPOCH};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration};
use tokio::sync::RwLock;
use serde::{Serialize, Deserialize};
use crate::error::{Result, DbError};
use super::{TenantId, ResourceConsumption};

// Metering engine
pub struct MeteringEngine {
    // Resource usage tracker
    tracker: Arc<ResourceUsageTracker>,

    // Quota enforcer
    enforcer: Arc<QuotaEnforcer>,

    // Billing integration
    billing: Arc<BillingIntegration>,

    // Metrics store
    metrics_store: Arc<RwLock<HashMap<TenantId, Vec<TenantMetrics>>>>,

    // Active metering jobs
    jobs: Arc<RwLock<HashMap<TenantId, MeteringJob>>>,
}

impl MeteringEngine {
    // Create a new metering engine
    pub fn new() -> Self {
        Self {
            tracker: Arc::new(ResourceUsageTracker::new()),
            enforcer: Arc::new(QuotaEnforcer::new()),
            billing: Arc::new(BillingIntegration::new()),
            metrics_store: Arc::new(RwLock::new(HashMap::new())),
            jobs: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    // Start metering for a tenant
    pub async fn start_metering(&self, tenant_id: TenantId, quota: ResourceQuota) -> Result<()> {
        // Register quota
        self.enforcer.set_quota(tenant_id, quota).await?;

        // Create metering job
        let job = MeteringJob {
            tenant_id,
            started_at: SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs(),
            collection_interval_secs: 60, // 1 minute
            last_collection_at: 0,
        };

        self.jobs.write().await.insert(tenant_id, job);

        // Start background collection
        let tracker = self.tracker.clone();
        let metrics_store = self.metrics_store.clone();

        tokio::spawn(async move {
            loop {
                tokio::time::sleep(Duration::from_secs(60)).await;

                if let Ok(metrics) = tracker.collect_metrics(tenant_id).await {
                    metrics_store
                        .write()
                        .await
                        .entry(tenant_id)
                        .or_insert_with(Vec::new)
                        .push(metrics);
                }
            }
        });

        Ok(())
    }

    // Stop metering for a tenant
    pub async fn stop_metering(&self, tenant_id: TenantId) -> Result<()> {
        self.jobs.write().await.remove(&tenant_id);
        Ok(())
    }

    // Get tenant metrics
    pub async fn get_tenant_metrics(&self, tenant_id: TenantId) -> Result<TenantMetrics> {
        self.tracker.collect_metrics(tenant_id).await
    }

    // Generate usage report
    pub async fn generate_usage_report(
        &self,
        tenant_id: TenantId,
        start_time: u64,
        end_time: u64,
    ) -> Result<UsageReport> {
        let all_metrics = self.metrics_store.read().await;

        let tenant_metrics = all_metrics
            .get(&tenant_id)
            .ok_or_else(|| DbError::NotFound(format!("No metrics for tenant {:?}", tenant_id)))?;

        // Filter metrics by time range
        let filtered: Vec<_> = tenant_metrics
            .iter()
            .filter(|m| m.timestamp >= start_time && m.timestamp <= end_time)
            .cloned()
            .collect();

        if filtered.is_empty() {
            return Err(DbError::NotFound("No metrics in time range".to_string()));
        }

        // Aggregate metrics
        let mut total_consumption = ResourceConsumption::zero();
        for metrics in &filtered {
            total_consumption.add(&metrics.resource_consumption);
        }

        // Calculate costs
        let cost = self.billing.calculate_cost(&total_consumption).await;

        Ok(UsageReport {
            tenant_id,
            start_time,
            end_time,
            total_consumption,
            cost,
            metrics_count: filtered.len(),
            peak_memory_bytes: filtered.iter().map(|m| m.resource_consumption.memory_bytes).max().unwrap_or(0),
            peak_connections: filtered.iter().map(|m| m.resource_consumption.active_connections).max().unwrap_or(0),
        })
    }

    // Check quota compliance
    pub async fn check_quota(&self, tenant_id: TenantId) -> Result<QuotaStatus> {
        self.enforcer.check_quota(tenant_id).await
    }

    // Export metrics to billing system
    pub async fn export_to_billing(&self, tenant_id: TenantId) -> Result<()> {
        let metrics = self.get_tenant_metrics(tenant_id).await?;
        self.billing.export_metrics(tenant_id, metrics).await
    }
}

#[derive(Debug, Clone)]
struct MeteringJob {
    #[allow(dead_code)]
    tenant_id: TenantId,
    #[allow(dead_code)]
    started_at: u64,
    #[allow(dead_code)]
    collection_interval_secs: u64,
    #[allow(dead_code)]
    last_collection_at: u64,
}

// Resource usage tracker
pub struct ResourceUsageTracker {
    // Current usage per tenant
    current_usage: Arc<RwLock<HashMap<TenantId, ResourceConsumption>>>,

    // Historical usage
    history: Arc<RwLock<HashMap<TenantId, Vec<UsageSnapshot>>>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct UsageSnapshot {
    timestamp: u64,
    consumption: ResourceConsumption,
}

impl ResourceUsageTracker {
    // Create a new resource usage tracker
    pub fn new() -> Self {
        Self {
            current_usage: Arc::new(RwLock::new(HashMap::new())),
            history: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    // Record resource usage
    pub async fn record_usage(&self, tenant_id: TenantId, consumption: ResourceConsumption) {
        // Update current usage
        self.current_usage
            .write()
            .await
            .insert(tenant_id, consumption.clone());

        // Add to history
        let snapshot = UsageSnapshot {
            timestamp: SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs(),
            consumption,
        };

        self.history
            .write()
            .await
            .entry(tenant_id)
            .or_insert_with(Vec::new)
            .push(snapshot);
    }

    // Get current usage
    pub async fn get_current_usage(&self, tenant_id: TenantId) -> Option<ResourceConsumption> {
        self.current_usage.read().await.get(&tenant_id).cloned()
    }

    // Collect metrics for a tenant
    pub async fn collect_metrics(&self, tenant_id: TenantId) -> Result<TenantMetrics> {
        let consumption = self
            .get_current_usage(tenant_id)
            .await
            .unwrap_or_else(ResourceConsumption::zero);

        Ok(TenantMetrics {
            tenant_id,
            timestamp: SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs(),
            resource_consumption: consumption,
            query_count: 0,
            transaction_count: 0,
            error_count: 0,
            avg_query_time_ms: 0,
        })
    }

    // Get usage history
    #[allow(private_interfaces)]
    pub async fn get_history(
        &self,
        tenant_id: TenantId,
        start_time: u64,
        end_time: u64,
    ) -> Vec<UsageSnapshot> {
        let history = self.history.read().await;

        history
            .get(&tenant_id)
            .map(|snapshots| {
                snapshots
                    .iter()
                    .filter(|s| s.timestamp >= start_time && s.timestamp <= end_time)
                    .cloned()
                    .collect()
            })
            .unwrap_or_default()
    }
}

// Tenant metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TenantMetrics {
    // Tenant ID
    pub tenant_id: TenantId,

    // Timestamp
    pub timestamp: u64,

    // Resource consumption
    pub resource_consumption: ResourceConsumption,

    // Query count
    pub query_count: u64,

    // Transaction count
    pub transaction_count: u64,

    // Error count
    pub error_count: u64,

    // Average query time (milliseconds)
    pub avg_query_time_ms: u64,
}

// Usage report
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UsageReport {
    // Tenant ID
    pub tenant_id: TenantId,

    // Report start time
    pub start_time: u64,

    // Report end time
    pub end_time: u64,

    // Total resource consumption
    pub total_consumption: ResourceConsumption,

    // Total cost
    pub cost: f64,

    // Number of metrics in report
    pub metrics_count: usize,

    // Peak memory usage
    pub peak_memory_bytes: u64,

    // Peak connections
    pub peak_connections: u32,
}

// Resource quota
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceQuota {
    // Memory quota (bytes)
    pub memory_quota_bytes: u64,

    // CPU quota (microseconds per period)
    pub cpu_quota_micros: u64,

    // Storage quota (bytes)
    pub storage_quota_bytes: u64,

    // I/O quota (bytes)
    pub io_quota_bytes: u64,

    // Connection quota
    pub connection_quota: u32,

    // Monthly budget (dollars)
    pub monthly_budget: Option<f64>,

    // Enforcement mode
    pub enforcement_mode: EnforcementMode,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EnforcementMode {
    // Soft limit (warn but allow)
    Soft,
    // Hard limit (block when exceeded)
    Hard,
    // Throttle (slow down when exceeded)
    Throttle,
}

impl Default for ResourceQuota {
    fn default() -> Self {
        Self {
            memory_quota_bytes: 1024 * 1024 * 1024, // 1 GB
            cpu_quota_micros: 1_000_000,             // 1 second
            storage_quota_bytes: 10 * 1024 * 1024 * 1024, // 10 GB
            io_quota_bytes: 100 * 1024 * 1024,       // 100 MB
            connection_quota: 100,
            monthly_budget: Some(100.0),
            enforcement_mode: EnforcementMode::Hard,
        }
    }
}

// Quota status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuotaStatus {
    pub tenant_id: TenantId,
    pub memory_usage_percent: f64,
    pub cpu_usage_percent: f64,
    pub storage_usage_percent: f64,
    pub io_usage_percent: f64,
    pub connection_usage_percent: f64,
    pub budget_usage_percent: Option<f64>,
    pub quota_exceeded: bool,
    pub warnings: Vec<String>,
}

// Quota enforcer
pub struct QuotaEnforcer {
    // Quotas per tenant
    quotas: Arc<RwLock<HashMap<TenantId, ResourceQuota>>>,

    // Current usage
    current_usage: Arc<RwLock<HashMap<TenantId, ResourceConsumption>>>,

    // Violation counts
    violations: Arc<RwLock<HashMap<TenantId, u64>>>,
}

impl QuotaEnforcer {
    // Create a new quota enforcer
    pub fn new() -> Self {
        Self {
            quotas: Arc::new(RwLock::new(HashMap::new())),
            current_usage: Arc::new(RwLock::new(HashMap::new())),
            violations: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    // Set quota for a tenant
    pub async fn set_quota(&self, tenant_id: TenantId, quota: ResourceQuota) -> Result<()> {
        self.quotas.write().await.insert(tenant_id, quota);
        Ok(())
    }

    // Update current usage
    pub async fn update_usage(&self, tenant_id: TenantId, consumption: ResourceConsumption) {
        self.current_usage.write().await.insert(tenant_id, consumption);
    }

    // Check quota compliance
    pub async fn check_quota(&self, tenant_id: TenantId) -> Result<QuotaStatus> {
        let quotas = self.quotas.read().await;
        let quota = quotas
            .get(&tenant_id)
            .ok_or_else(|| DbError::NotFound(format!("No quota for tenant {:?}", tenant_id)))?;

        let usage = self
            .current_usage
            .read()
            .await
            .get(&tenant_id)
            .cloned()
            .unwrap_or_else(ResourceConsumption::zero);

        let memory_usage_percent = (usage.memory_bytes as f64 / quota.memory_quota_bytes as f64) * 100.0;
        let storage_usage_percent = (usage.storage_bytes as f64 / quota.storage_quota_bytes as f64) * 100.0;
        let connection_usage_percent = (usage.active_connections as f64 / quota.connection_quota as f64) * 100.0;

        let mut warnings = Vec::new();
        let mut quota_exceeded = false;

        if memory_usage_percent > 90.0 {
            warnings.push(format!("Memory usage at {:.1}%", memory_usage_percent));
            if memory_usage_percent >= 100.0 {
                quota_exceeded = true;
            }
        }

        if storage_usage_percent > 90.0 {
            warnings.push(format!("Storage usage at {:.1}%", storage_usage_percent));
            if storage_usage_percent >= 100.0 {
                quota_exceeded = true;
            }
        }

        if connection_usage_percent > 90.0 {
            warnings.push(format!("Connection usage at {:.1}%", connection_usage_percent));
            if connection_usage_percent >= 100.0 {
                quota_exceeded = true;
            }
        }

        if quota_exceeded {
            let mut violations = self.violations.write().await;
            *violations.entry(tenant_id).or_insert(0) += 1;
        }

        Ok(QuotaStatus {
            tenant_id,
            memory_usage_percent,
            cpu_usage_percent: 0.0,
            storage_usage_percent,
            io_usage_percent: 0.0,
            connection_usage_percent,
            budget_usage_percent: None,
            quota_exceeded,
            warnings,
        })
    }

    // Enforce quota
    pub async fn enforce(&self, tenant_id: TenantId, resource_type: ResourceType) -> Result<()> {
        let status = self.check_quota(tenant_id).await?;

        if !status.quota_exceeded {
            return Ok(());
        }

        let quotas = self.quotas.read().await;
        let quota = quotas
            .get(&tenant_id)
            .ok_or_else(|| DbError::NotFound(format!("No quota for tenant {:?}", tenant_id)))?;

        match quota.enforcement_mode {
            EnforcementMode::Soft => {
                // Log warning but allow
                eprintln!("Quota exceeded for tenant {:?}: {:?}", tenant_id, status.warnings);
                Ok(())
            }
            EnforcementMode::Hard => {
                // Block operation
                Err(DbError::QuotaExceeded(format!(
                    "Quota exceeded for {:?}: {:?}",
                    resource_type, status.warnings
                )))
            }
            EnforcementMode::Throttle => {
                // Throttle operation
                tokio::time::sleep(Duration::from_millis(100)).await;
                Ok(())
            }
        }
    }

    // Get violation count
    pub async fn get_violations(&self, tenant_id: TenantId) -> u64 {
        self.violations.read().await.get(&tenant_id).copied().unwrap_or(0)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResourceType {
    Memory,
    Cpu,
    Storage,
    Io,
    Connection,
}

// Billing integration
pub struct BillingIntegration {
    // Pricing configuration
    pricing: Arc<RwLock<PricingConfig>>,

    // Exported metrics
    exported_metrics: Arc<RwLock<HashMap<TenantId, Vec<TenantMetrics>>>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct PricingConfig {
    // Price per GB-hour of memory
    memory_price_per_gb_hour: f64,

    // Price per CPU-hour
    cpu_price_per_hour: f64,

    // Price per GB of storage per month
    storage_price_per_gb_month: f64,

    // Price per GB of I/O
    io_price_per_gb: f64,

    // Price per connection-hour
    connection_price_per_hour: f64,
}

impl Default for PricingConfig {
    fn default() -> Self {
        Self {
            memory_price_per_gb_hour: 0.01,
            cpu_price_per_hour: 0.05,
            storage_price_per_gb_month: 0.10,
            io_price_per_gb: 0.001,
            connection_price_per_hour: 0.001,
        }
    }
}

impl BillingIntegration {
    // Create a new billing integration
    pub fn new() -> Self {
        Self {
            pricing: Arc::new(RwLock::new(PricingConfig::default())),
            exported_metrics: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    // Calculate cost for resource consumption
    pub async fn calculate_cost(&self, consumption: &ResourceConsumption) -> f64 {
        let pricing = self.pricing.read().await;

        let memory_gb_hours = (consumption.memory_bytes as f64 / (1024.0 * 1024.0 * 1024.0)) / 3600.0;
        let cpu_hours = (consumption.cpu_micros as f64 / 1_000_000.0) / 3600.0;
        let storage_gb_months = (consumption.storage_bytes as f64 / (1024.0 * 1024.0 * 1024.0)) / (30.0 * 24.0);
        let io_gb = (consumption.io_read_bytes + consumption.io_write_bytes) as f64 / (1024.0 * 1024.0 * 1024.0);

        let memory_cost = memory_gb_hours * pricing.memory_price_per_gb_hour;
        let cpu_cost = cpu_hours * pricing.cpu_price_per_hour;
        let storage_cost = storage_gb_months * pricing.storage_price_per_gb_month;
        let io_cost = io_gb * pricing.io_price_per_gb;

        memory_cost + cpu_cost + storage_cost + io_cost
    }

    // Export metrics to external billing system
    pub async fn export_metrics(&self, tenant_id: TenantId, metrics: TenantMetrics) -> Result<()> {
        self.exported_metrics
            .write()
            .await
            .entry(tenant_id)
            .or_insert_with(Vec::new)
            .push(metrics);

        // In real implementation, would send to external system
        Ok(())
    }

    // Get exported metrics
    pub async fn get_exported_metrics(&self, tenant_id: TenantId) -> Vec<TenantMetrics> {
        self.exported_metrics
            .read()
            .await
            .get(&tenant_id)
            .cloned()
            .unwrap_or_default()
    }

    // Update pricing configuration
    #[allow(private_interfaces)]
    pub async fn update_pricing(&self, pricing: PricingConfig) {
        *self.pricing.write().await = pricing;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_metering_engine() {
        let engine = MeteringEngine::new();
        let tenant_id = TenantId::new(1);
        let quota = ResourceQuota::default();

        engine.start_metering(tenant_id, quota).await.unwrap();

        let metrics = engine.get_tenant_metrics(tenant_id).await.unwrap();
        assert_eq!(metrics.tenant_id, tenant_id);
    }

    #[tokio::test]
    async fn test_resource_usage_tracker() {
        let tracker = ResourceUsageTracker::new();
        let tenant_id = TenantId::new(1);

        let consumption = ResourceConsumption {
            memory_bytes: 1024 * 1024 * 1024,
            cpu_micros: 1_000_000,
            io_read_bytes: 1024,
            io_write_bytes: 1024,
            active_connections: 10,
            storage_bytes: 5 * 1024 * 1024 * 1024,
            temp_bytes: 100 * 1024 * 1024,
        };

        tracker.record_usage(tenant_id, consumption.clone()).await;

        let current = tracker.get_current_usage(tenant_id).await.unwrap();
        assert_eq!(current.memory_bytes, consumption.memory_bytes);
    }

    #[tokio::test]
    async fn test_quota_enforcer() {
        let enforcer = QuotaEnforcer::new();
        let tenant_id = TenantId::new(1);

        let quota = ResourceQuota::default();
        enforcer.set_quota(tenant_id, quota).await.unwrap();

        let consumption = ResourceConsumption {
            memory_bytes: 512 * 1024 * 1024,
            cpu_micros: 500_000,
            io_read_bytes: 1024,
            io_write_bytes: 1024,
            active_connections: 50,
            storage_bytes: 2 * 1024 * 1024 * 1024,
            temp_bytes: 50 * 1024 * 1024,
        };

        enforcer.update_usage(tenant_id, consumption).await;

        let status = enforcer.check_quota(tenant_id).await.unwrap();
        assert!(!status.quota_exceeded);
    }

    #[tokio::test]
    async fn test_billing_integration() {
        let billing = BillingIntegration::new();

        let consumption = ResourceConsumption {
            memory_bytes: 1024 * 1024 * 1024,
            cpu_micros: 3_600_000_000,
            io_read_bytes: 1024 * 1024 * 1024,
            io_write_bytes: 1024 * 1024 * 1024,
            active_connections: 10,
            storage_bytes: 10 * 1024 * 1024 * 1024,
            temp_bytes: 0,
        };

        let cost = billing.calculate_cost(&consumption).await;
        assert!(cost > 0.0);
    }
}
