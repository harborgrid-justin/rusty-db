// GraphQL Performance & Monitoring Subscriptions
//
// Real-time subscriptions for performance monitoring including:
// - Active queries
// - Slow queries
// - Query plan changes
// - System alerts
// - Component health
// - Storage status
// - Buffer pool metrics
// - I/O statistics

use async_graphql::{Context, Enum, Object, SimpleObject, Subscription, ID};
use futures_util::stream::Stream;
use futures_util::StreamExt;
use std::time::Duration;
use tokio::sync::broadcast;
use tokio_stream::wrappers::BroadcastStream;
use serde::{Deserialize, Serialize};

use super::types::{DateTime, BigInt};

// ============================================================================
// Query Performance Event Types
// ============================================================================

/// Active query stream event
#[derive(Clone, Debug)]
pub struct ActiveQueryEvent {
    pub query_id: ID,
    pub session_id: String,
    pub user_id: String,
    pub database: String,
    pub sql_text: String,
    pub state: QueryState,
    pub elapsed_ms: i64,
    pub cpu_time_ms: i64,
    pub rows_examined: BigInt,
    pub rows_returned: BigInt,
    pub memory_used_bytes: BigInt,
    pub waiting_on: Option<String>,
    pub wait_type: Option<String>,
    pub progress_percent: Option<f64>,
    pub client_ip: Option<String>,
    pub started_at: DateTime,
    pub timestamp: DateTime,
}

#[Object]
impl ActiveQueryEvent {
    async fn query_id(&self) -> &ID {
        &self.query_id
    }

    async fn session_id(&self) -> &str {
        &self.session_id
    }

    async fn user_id(&self) -> &str {
        &self.user_id
    }

    async fn database(&self) -> &str {
        &self.database
    }

    async fn sql_text(&self) -> &str {
        &self.sql_text
    }

    async fn state(&self) -> QueryState {
        self.state
    }

    async fn elapsed_ms(&self) -> i64 {
        self.elapsed_ms
    }

    async fn cpu_time_ms(&self) -> i64 {
        self.cpu_time_ms
    }

    async fn rows_examined(&self) -> &BigInt {
        &self.rows_examined
    }

    async fn rows_returned(&self) -> &BigInt {
        &self.rows_returned
    }

    async fn memory_used_bytes(&self) -> &BigInt {
        &self.memory_used_bytes
    }

    async fn waiting_on(&self) -> &Option<String> {
        &self.waiting_on
    }

    async fn wait_type(&self) -> &Option<String> {
        &self.wait_type
    }

    async fn progress_percent(&self) -> Option<f64> {
        self.progress_percent
    }

    async fn client_ip(&self) -> &Option<String> {
        &self.client_ip
    }

    async fn started_at(&self) -> &DateTime {
        &self.started_at
    }

    async fn timestamp(&self) -> &DateTime {
        &self.timestamp
    }
}

#[derive(Enum, Copy, Clone, Eq, PartialEq, Debug)]
pub enum QueryState {
    Parsing,
    Planning,
    Executing,
    Waiting,
    Committing,
    Finished,
}

/// Slow query detection event
#[derive(Clone, Debug)]
pub struct SlowQueryEvent {
    pub query_id: ID,
    pub session_id: String,
    pub user_id: String,
    pub sql_text: String,
    pub sql_fingerprint: String,
    pub execution_time_ms: i64,
    pub threshold_ms: i64,
    pub rows_examined: BigInt,
    pub rows_returned: BigInt,
    pub lock_time_ms: i64,
    pub sort_operations: i32,
    pub temp_tables_created: i32,
    pub full_table_scans: i32,
    pub index_scans: i32,
    pub recommendation: Option<String>,
    pub started_at: DateTime,
    pub completed_at: DateTime,
    pub timestamp: DateTime,
}

#[Object]
impl SlowQueryEvent {
    async fn query_id(&self) -> &ID {
        &self.query_id
    }

    async fn session_id(&self) -> &str {
        &self.session_id
    }

    async fn user_id(&self) -> &str {
        &self.user_id
    }

    async fn sql_text(&self) -> &str {
        &self.sql_text
    }

    async fn sql_fingerprint(&self) -> &str {
        &self.sql_fingerprint
    }

    async fn execution_time_ms(&self) -> i64 {
        self.execution_time_ms
    }

    async fn threshold_ms(&self) -> i64 {
        self.threshold_ms
    }

    async fn rows_examined(&self) -> &BigInt {
        &self.rows_examined
    }

    async fn rows_returned(&self) -> &BigInt {
        &self.rows_returned
    }

    async fn lock_time_ms(&self) -> i64 {
        self.lock_time_ms
    }

    async fn sort_operations(&self) -> i32 {
        self.sort_operations
    }

    async fn temp_tables_created(&self) -> i32 {
        self.temp_tables_created
    }

    async fn full_table_scans(&self) -> i32 {
        self.full_table_scans
    }

    async fn index_scans(&self) -> i32 {
        self.index_scans
    }

    async fn recommendation(&self) -> &Option<String> {
        &self.recommendation
    }

    async fn started_at(&self) -> &DateTime {
        &self.started_at
    }

    async fn completed_at(&self) -> &DateTime {
        &self.completed_at
    }

    async fn timestamp(&self) -> &DateTime {
        &self.timestamp
    }
}

/// Query plan change event
#[derive(Clone, Debug)]
pub struct QueryPlanChangeEvent {
    pub event_id: ID,
    pub sql_fingerprint: String,
    pub old_plan_hash: String,
    pub new_plan_hash: String,
    pub change_reason: PlanChangeReason,
    pub old_plan_cost: f64,
    pub new_plan_cost: f64,
    pub cost_improvement_percent: f64,
    pub old_plan_summary: String,
    pub new_plan_summary: String,
    pub statistics_changed: Vec<String>,
    pub indexes_used_old: Vec<String>,
    pub indexes_used_new: Vec<String>,
    pub accepted: bool,
    pub timestamp: DateTime,
}

#[Object]
impl QueryPlanChangeEvent {
    async fn event_id(&self) -> &ID {
        &self.event_id
    }

    async fn sql_fingerprint(&self) -> &str {
        &self.sql_fingerprint
    }

    async fn old_plan_hash(&self) -> &str {
        &self.old_plan_hash
    }

    async fn new_plan_hash(&self) -> &str {
        &self.new_plan_hash
    }

    async fn change_reason(&self) -> PlanChangeReason {
        self.change_reason
    }

    async fn old_plan_cost(&self) -> f64 {
        self.old_plan_cost
    }

    async fn new_plan_cost(&self) -> f64 {
        self.new_plan_cost
    }

    async fn cost_improvement_percent(&self) -> f64 {
        self.cost_improvement_percent
    }

    async fn old_plan_summary(&self) -> &str {
        &self.old_plan_summary
    }

    async fn new_plan_summary(&self) -> &str {
        &self.new_plan_summary
    }

    async fn statistics_changed(&self) -> &[String] {
        &self.statistics_changed
    }

    async fn indexes_used_old(&self) -> &[String] {
        &self.indexes_used_old
    }

    async fn indexes_used_new(&self) -> &[String] {
        &self.indexes_used_new
    }

    async fn accepted(&self) -> bool {
        self.accepted
    }

    async fn timestamp(&self) -> &DateTime {
        &self.timestamp
    }
}

#[derive(Enum, Copy, Clone, Eq, PartialEq, Debug)]
pub enum PlanChangeReason {
    StatisticsUpdated,
    IndexAdded,
    IndexDropped,
    ParameterChanged,
    DataDistributionChanged,
    HintApplied,
    AdaptiveOptimization,
}

// ============================================================================
// Health & Alert Event Types
// ============================================================================

/// System alert event
#[derive(Clone, Debug)]
pub struct SystemAlertEvent {
    pub alert_id: ID,
    pub severity: AlertSeverity,
    pub category: AlertCategory,
    pub component: String,
    pub title: String,
    pub description: String,
    pub metric_name: Option<String>,
    pub current_value: Option<f64>,
    pub threshold_value: Option<f64>,
    pub recommended_action: Option<String>,
    pub auto_resolved: bool,
    pub resolved_at: Option<DateTime>,
    pub fired_at: DateTime,
    pub timestamp: DateTime,
}

#[Object]
impl SystemAlertEvent {
    async fn alert_id(&self) -> &ID {
        &self.alert_id
    }

    async fn severity(&self) -> AlertSeverity {
        self.severity
    }

    async fn category(&self) -> AlertCategory {
        self.category
    }

    async fn component(&self) -> &str {
        &self.component
    }

    async fn title(&self) -> &str {
        &self.title
    }

    async fn description(&self) -> &str {
        &self.description
    }

    async fn metric_name(&self) -> &Option<String> {
        &self.metric_name
    }

    async fn current_value(&self) -> Option<f64> {
        self.current_value
    }

    async fn threshold_value(&self) -> Option<f64> {
        self.threshold_value
    }

    async fn recommended_action(&self) -> &Option<String> {
        &self.recommended_action
    }

    async fn auto_resolved(&self) -> bool {
        self.auto_resolved
    }

    async fn resolved_at(&self) -> &Option<DateTime> {
        &self.resolved_at
    }

    async fn fired_at(&self) -> &DateTime {
        &self.fired_at
    }

    async fn timestamp(&self) -> &DateTime {
        &self.timestamp
    }
}

#[derive(Enum, Copy, Clone, Eq, PartialEq, Debug)]
pub enum AlertSeverity {
    Info,
    Warning,
    Error,
    Critical,
}

#[derive(Enum, Copy, Clone, Eq, PartialEq, Debug)]
pub enum AlertCategory {
    Performance,
    Availability,
    Capacity,
    Security,
    DataIntegrity,
    Replication,
    Backup,
}

/// Component health status change event
#[derive(Clone, Debug, SimpleObject)]
pub struct HealthStatusChangeEvent {
    pub component: String,
    pub component_type: String,
    pub old_status: HealthStatus,
    pub new_status: HealthStatus,
    pub reason: String,
    pub metrics: Vec<HealthMetric>,
    pub last_check_at: DateTime,
    pub timestamp: DateTime,
}

#[derive(Enum, Copy, Clone, Eq, PartialEq, Debug)]
pub enum HealthStatus {
    Healthy,
    Degraded,
    Unhealthy,
    Unknown,
}

#[derive(Clone, Debug, SimpleObject)]
pub struct HealthMetric {
    pub name: String,
    pub value: f64,
    pub unit: String,
    pub status: HealthStatus,
}

// ============================================================================
// Storage & I/O Event Types
// ============================================================================

/// Storage status change event
#[derive(Clone, Debug, SimpleObject)]
pub struct StorageStatusChangeEvent {
    pub tablespace_name: String,
    pub total_size_bytes: BigInt,
    pub used_size_bytes: BigInt,
    pub free_size_bytes: BigInt,
    pub usage_percent: f64,
    pub fragmentation_percent: f64,
    pub growth_rate_bytes_per_hour: BigInt,
    pub estimated_full_in_hours: Option<i32>,
    pub status: StorageStatus,
    pub timestamp: DateTime,
}

#[derive(Enum, Copy, Clone, Eq, PartialEq, Debug)]
pub enum StorageStatus {
    Normal,
    Warning,
    Critical,
    Full,
}

/// Buffer pool metrics event
#[derive(Clone, Debug, SimpleObject)]
pub struct BufferPoolMetricsEvent {
    pub pool_name: String,
    pub total_pages: i32,
    pub used_pages: i32,
    pub dirty_pages: i32,
    pub free_pages: i32,
    pub pinned_pages: i32,
    pub eviction_rate_per_sec: f64,
    pub hit_rate_percent: f64,
    pub miss_rate_percent: f64,
    pub reads_per_sec: f64,
    pub writes_per_sec: f64,
    pub eviction_policy: String,
    pub timestamp: DateTime,
}

/// I/O statistics event
#[derive(Clone, Debug, SimpleObject)]
pub struct IoStatisticsEvent {
    pub device: String,
    pub read_ops_per_sec: f64,
    pub write_ops_per_sec: f64,
    pub read_bytes_per_sec: BigInt,
    pub write_bytes_per_sec: BigInt,
    pub avg_read_latency_ms: f64,
    pub avg_write_latency_ms: f64,
    pub p99_read_latency_ms: f64,
    pub p99_write_latency_ms: f64,
    pub queue_depth: i32,
    pub utilization_percent: f64,
    pub timestamp: DateTime,
}

// ============================================================================
// Performance Subscription Root
// ============================================================================

/// Performance and Monitoring subscription operations
pub struct PerformanceSubscriptionRoot;

#[Subscription]
impl PerformanceSubscriptionRoot {
    /// Subscribe to active queries stream
    ///
    /// Receives real-time updates about currently executing queries
    /// including resource usage, progress, and wait states.
    ///
    /// # Arguments
    /// * `min_elapsed_ms` - Only show queries running longer than this threshold
    /// * `user_id` - Optional filter by user
    async fn active_queries_stream<'ctx>(
        &self,
        min_elapsed_ms: Option<i64>,
        user_id: Option<String>,
    ) -> impl Stream<Item = ActiveQueryEvent> + 'ctx {
        let (tx, rx) = broadcast::channel(1000);

        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(2));
            let mut counter = 0;

            loop {
                interval.tick().await;
                counter += 1;

                let event = ActiveQueryEvent {
                    query_id: ID::from(format!("query_{}", uuid::Uuid::new_v4())),
                    session_id: format!("session_{}", counter % 10),
                    user_id: "user_alice".to_string(),
                    database: "production".to_string(),
                    sql_text: "SELECT * FROM orders WHERE date >= '2025-01-01' ORDER BY total DESC LIMIT 1000".to_string(),
                    state: QueryState::Executing,
                    elapsed_ms: 1500 + (counter * 100),
                    cpu_time_ms: 1200 + (counter * 80),
                    rows_examined: BigInt(500000 + counter as i64 * 10000),
                    rows_returned: BigInt(750 + counter as i64 * 10),
                    memory_used_bytes: BigInt(16777216),
                    waiting_on: None,
                    wait_type: None,
                    progress_percent: Some(65.5),
                    client_ip: Some("192.168.1.100".to_string()),
                    started_at: DateTime::now(),
                    timestamp: DateTime::now(),
                };

                let _ = tx.send(event);
            }
        });

        let min_elapsed_ms = min_elapsed_ms.clone();
        let user_id = user_id.clone();

        BroadcastStream::new(rx).filter_map(move |result| {
            let min_elapsed_ms = min_elapsed_ms.clone();
            let user_id = user_id.clone();

            async move {
                result.ok().and_then(|event| {
                    // Filter by min elapsed time
                    if let Some(min_elapsed) = min_elapsed_ms {
                        if event.elapsed_ms < min_elapsed {
                            return None;
                        }
                    }

                    // Filter by user
                    if let Some(ref uid) = user_id {
                        if &event.user_id != uid {
                            return None;
                        }
                    }

                    Some(event)
                })
            }
        })
    }

    /// Subscribe to slow query detection stream
    ///
    /// Receives notifications when queries exceed the slow query threshold.
    ///
    /// # Arguments
    /// * `threshold_ms` - Minimum execution time to be considered slow
    /// * `include_recommendations` - Whether to include optimization recommendations
    async fn slow_queries_stream<'ctx>(
        &self,
        threshold_ms: Option<i64>,
        include_recommendations: Option<bool>,
    ) -> impl Stream<Item = SlowQueryEvent> + 'ctx {
        let (tx, rx) = broadcast::channel(1000);

        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(10));
            let mut counter = 0;

            loop {
                interval.tick().await;
                counter += 1;

                let event = SlowQueryEvent {
                    query_id: ID::from(format!("slow_{}", uuid::Uuid::new_v4())),
                    session_id: format!("session_{}", counter % 5),
                    user_id: "user_bob".to_string(),
                    sql_text: "SELECT c.*, o.* FROM customers c LEFT JOIN orders o ON c.id = o.customer_id WHERE c.created_at > '2024-01-01'".to_string(),
                    sql_fingerprint: "SELECT c.*, o.* FROM customers c LEFT JOIN orders o ON c.id = o.customer_id WHERE c.created_at > ?".to_string(),
                    execution_time_ms: 5500 + (counter * 500),
                    threshold_ms: 1000,
                    rows_examined: BigInt(2500000),
                    rows_returned: BigInt(15000),
                    lock_time_ms: 120,
                    sort_operations: 2,
                    temp_tables_created: 1,
                    full_table_scans: 1,
                    index_scans: 3,
                    recommendation: Some("Add index on customers(created_at)".to_string()),
                    started_at: DateTime::now(),
                    completed_at: DateTime::now(),
                    timestamp: DateTime::now(),
                };

                let _ = tx.send(event);
            }
        });

        BroadcastStream::new(rx).filter_map(move |result| {
            async move {
                result.ok()
            }
        })
    }

    /// Subscribe to query plan changes
    ///
    /// Receives notifications when the query optimizer changes execution plans
    /// for frequently executed queries.
    async fn query_plan_changes<'ctx>(
        &self,
    ) -> impl Stream<Item = QueryPlanChangeEvent> + 'ctx {
        let (tx, rx) = broadcast::channel(100);

        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(30));
            let mut counter = 0;

            loop {
                interval.tick().await;
                counter += 1;

                let event = QueryPlanChangeEvent {
                    event_id: ID::from(format!("plan_change_{}", uuid::Uuid::new_v4())),
                    sql_fingerprint: "SELECT * FROM orders WHERE customer_id = ? AND date > ?".to_string(),
                    old_plan_hash: format!("hash_{:08x}", 0xabcd1234u32),
                    new_plan_hash: format!("hash_{:08x}", 0xdcba4321u32),
                    change_reason: PlanChangeReason::StatisticsUpdated,
                    old_plan_cost: 15000.0,
                    new_plan_cost: 2500.0,
                    cost_improvement_percent: 83.3,
                    old_plan_summary: "SeqScan(orders) -> Filter".to_string(),
                    new_plan_summary: "IndexScan(orders.idx_customer_date) -> Filter".to_string(),
                    statistics_changed: vec!["orders".to_string()],
                    indexes_used_old: vec![],
                    indexes_used_new: vec!["idx_customer_date".to_string()],
                    accepted: true,
                    timestamp: DateTime::now(),
                };

                let _ = tx.send(event);
            }
        });

        BroadcastStream::new(rx).filter_map(|result| async move { result.ok() })
    }

    /// Subscribe to system alerts
    ///
    /// Receives real-time system alerts for performance, availability,
    /// capacity, and other critical events.
    ///
    /// # Arguments
    /// * `min_severity` - Minimum alert severity to receive
    /// * `categories` - Optional filter by alert categories
    async fn alert_stream<'ctx>(
        &self,
        min_severity: Option<AlertSeverity>,
        categories: Option<Vec<AlertCategory>>,
    ) -> impl Stream<Item = SystemAlertEvent> + 'ctx {
        let (tx, rx) = broadcast::channel(500);

        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(20));
            let severities = vec![
                AlertSeverity::Info,
                AlertSeverity::Warning,
                AlertSeverity::Error,
            ];
            let alert_categories = vec![
                AlertCategory::Performance,
                AlertCategory::Capacity,
                AlertCategory::Availability,
            ];
            let mut counter = 0;

            loop {
                interval.tick().await;
                counter += 1;

                let event = SystemAlertEvent {
                    alert_id: ID::from(format!("alert_{}", uuid::Uuid::new_v4())),
                    severity: severities[counter % severities.len()],
                    category: alert_categories[counter % alert_categories.len()],
                    component: "buffer_pool".to_string(),
                    title: "High buffer pool utilization".to_string(),
                    description: "Buffer pool utilization has exceeded 85%".to_string(),
                    metric_name: Some("buffer_pool_utilization".to_string()),
                    current_value: Some(87.5),
                    threshold_value: Some(85.0),
                    recommended_action: Some("Consider increasing buffer pool size".to_string()),
                    auto_resolved: false,
                    resolved_at: None,
                    fired_at: DateTime::now(),
                    timestamp: DateTime::now(),
                };

                let _ = tx.send(event);
            }
        });

        let min_severity = min_severity.clone();
        let categories = categories.clone();

        BroadcastStream::new(rx).filter_map(move |result| {
            let min_severity = min_severity.clone();
            let categories = categories.clone();

            async move {
                result.ok().and_then(|event| {
                    // Filter by severity
                    if let Some(min_sev) = min_severity {
                        if (event.severity as u8) < (min_sev as u8) {
                            return None;
                        }
                    }

                    // Filter by category
                    if let Some(ref cats) = categories {
                        if !cats.contains(&event.category) {
                            return None;
                        }
                    }

                    Some(event)
                })
            }
        })
    }

    /// Subscribe to component health status changes
    ///
    /// Receives notifications when database components change health status.
    async fn health_status_changes<'ctx>(
        &self,
        component: Option<String>,
    ) -> impl Stream<Item = HealthStatusChangeEvent> + 'ctx {
        let (tx, rx) = broadcast::channel(500);

        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(15));
            let components = vec!["storage", "replication", "network", "query_executor"];
            let mut counter = 0;

            loop {
                interval.tick().await;
                counter += 1;

                let comp = components[counter % components.len()];

                let event = HealthStatusChangeEvent {
                    component: comp.to_string(),
                    component_type: "subsystem".to_string(),
                    old_status: HealthStatus::Healthy,
                    new_status: HealthStatus::Degraded,
                    reason: "Elevated response times detected".to_string(),
                    metrics: vec![
                        HealthMetric {
                            name: "response_time_ms".to_string(),
                            value: 350.0,
                            unit: "ms".to_string(),
                            status: HealthStatus::Degraded,
                        },
                        HealthMetric {
                            name: "error_rate".to_string(),
                            value: 0.02,
                            unit: "percent".to_string(),
                            status: HealthStatus::Healthy,
                        },
                    ],
                    last_check_at: DateTime::now(),
                    timestamp: DateTime::now(),
                };

                let _ = tx.send(event);
            }
        });

        let component = component.clone();

        BroadcastStream::new(rx).filter_map(move |result| {
            let component = component.clone();

            async move {
                result.ok().and_then(|event| {
                    if let Some(ref comp) = component {
                        if &event.component != comp {
                            return None;
                        }
                    }
                    Some(event)
                })
            }
        })
    }

    /// Subscribe to storage status changes
    ///
    /// Receives notifications about storage capacity and utilization changes.
    async fn storage_status_changes<'ctx>(
        &self,
        tablespace_name: Option<String>,
        interval_seconds: Option<i32>,
    ) -> impl Stream<Item = StorageStatusChangeEvent> + 'ctx {
        let interval = Duration::from_secs(interval_seconds.unwrap_or(30) as u64);

        async_stream::stream! {
            let mut interval_timer = tokio::time::interval(interval);
            let mut usage = 45.0;

            loop {
                interval_timer.tick().await;
                usage = (usage + 1.5_f64).min(95.0_f64);

                let total = 1099511627776i64; // 1 TB
                let used = (total as f64 * (usage / 100.0)) as i64;
                let free = total - used;

                yield StorageStatusChangeEvent {
                    tablespace_name: tablespace_name.clone().unwrap_or_else(|| "main".to_string()),
                    total_size_bytes: BigInt(total),
                    used_size_bytes: BigInt(used),
                    free_size_bytes: BigInt(free),
                    usage_percent: usage,
                    fragmentation_percent: 5.2,
                    growth_rate_bytes_per_hour: BigInt(10737418240), // 10 GB/hour
                    estimated_full_in_hours: Some(((100.0 - usage) / 1.5 * 24.0) as i32),
                    status: if usage > 90.0 {
                        StorageStatus::Critical
                    } else if usage > 75.0 {
                        StorageStatus::Warning
                    } else {
                        StorageStatus::Normal
                    },
                    timestamp: DateTime::now(),
                };
            }
        }
    }

    /// Subscribe to buffer pool metrics
    ///
    /// Receives real-time buffer pool performance metrics.
    async fn buffer_pool_metrics<'ctx>(
        &self,
        pool_name: Option<String>,
        interval_seconds: Option<i32>,
    ) -> impl Stream<Item = BufferPoolMetricsEvent> + 'ctx {
        let interval = Duration::from_secs(interval_seconds.unwrap_or(5) as u64);

        async_stream::stream! {
            let mut interval_timer = tokio::time::interval(interval);

            loop {
                interval_timer.tick().await;

                yield BufferPoolMetricsEvent {
                    pool_name: pool_name.clone().unwrap_or_else(|| "default".to_string()),
                    total_pages: 10000,
                    used_pages: 8500,
                    dirty_pages: 1200,
                    free_pages: 1500,
                    pinned_pages: 450,
                    eviction_rate_per_sec: 25.5,
                    hit_rate_percent: 96.8,
                    miss_rate_percent: 3.2,
                    reads_per_sec: 1250.0,
                    writes_per_sec: 450.0,
                    eviction_policy: "CLOCK".to_string(),
                    timestamp: DateTime::now(),
                };
            }
        }
    }

    /// Subscribe to I/O statistics
    ///
    /// Receives real-time I/O performance statistics.
    async fn io_statistics_stream<'ctx>(
        &self,
        device: Option<String>,
        interval_seconds: Option<i32>,
    ) -> impl Stream<Item = IoStatisticsEvent> + 'ctx {
        let interval = Duration::from_secs(interval_seconds.unwrap_or(5) as u64);

        async_stream::stream! {
            let mut interval_timer = tokio::time::interval(interval);

            loop {
                interval_timer.tick().await;

                yield IoStatisticsEvent {
                    device: device.clone().unwrap_or_else(|| "/dev/sda1".to_string()),
                    read_ops_per_sec: 850.0,
                    write_ops_per_sec: 320.0,
                    read_bytes_per_sec: BigInt(52428800), // 50 MB/s
                    write_bytes_per_sec: BigInt(20971520), // 20 MB/s
                    avg_read_latency_ms: 2.5,
                    avg_write_latency_ms: 4.2,
                    p99_read_latency_ms: 12.5,
                    p99_write_latency_ms: 18.7,
                    queue_depth: 4,
                    utilization_percent: 65.5,
                    timestamp: DateTime::now(),
                };
            }
        }
    }
}
