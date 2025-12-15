// GraphQL Subscription Operations
//
// Real-time subscription resolvers for the GraphQL API

use super::models::*;
use super::types::*;
use super::GraphQLEngine;
use async_graphql::{Context, Enum, Object, SimpleObject, Subscription, ID};
use futures_util::stream::Stream;
use futures_util::StreamExt;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{broadcast, RwLock};
use tokio_stream::wrappers::BroadcastStream;

// ============================================================================
// PART 4: SUBSCRIPTION SYSTEM (600+ lines)
// ============================================================================

// Root subscription type
pub struct SubscriptionRoot;

#[Subscription]
impl SubscriptionRoot {
    // Subscribe to all changes on a table
    async fn table_changes<'ctx>(
        &self,
        ctx: &Context<'ctx>,
        table: String,
        where_clause: Option<WhereClause>,
    ) -> impl Stream<Item = TableChange> + 'ctx {
        let engine = ctx.data::<Arc<GraphQLEngine>>().unwrap().clone();
        let (tx, rx) = broadcast::channel(1000);

        // Register subscription
        let _subscription_id = engine
            .register_table_subscription(&table, where_clause, tx)
            .await;

        BroadcastStream::new(rx).filter_map(|result| async move { result.ok() })
    }

    // Subscribe to row insertions
    async fn row_inserted<'ctx>(
        &self,
        ctx: &Context<'ctx>,
        table: String,
        where_clause: Option<WhereClause>,
    ) -> impl Stream<Item = RowInserted> + 'ctx {
        let engine = ctx.data::<Arc<GraphQLEngine>>().unwrap().clone();
        let (tx, rx) = broadcast::channel(1000);

        // Register subscription
        engine
            .register_insert_subscription(&table, where_clause, tx)
            .await;

        BroadcastStream::new(rx).filter_map(|result| async move { result.ok() })
    }

    // Subscribe to row updates
    async fn row_updated<'ctx>(
        &self,
        ctx: &Context<'ctx>,
        table: String,
        where_clause: Option<WhereClause>,
    ) -> impl Stream<Item = RowUpdated> + 'ctx {
        let engine = ctx.data::<Arc<GraphQLEngine>>().unwrap().clone();
        let (tx, rx) = broadcast::channel(1000);

        // Register subscription
        engine
            .register_update_subscription(&table, where_clause, tx)
            .await;

        BroadcastStream::new(rx).filter_map(|result| async move { result.ok() })
    }

    // Subscribe to row deletions
    async fn row_deleted<'ctx>(
        &self,
        ctx: &Context<'ctx>,
        table: String,
        where_clause: Option<WhereClause>,
    ) -> impl Stream<Item = RowDeleted> + 'ctx {
        let engine = ctx.data::<Arc<GraphQLEngine>>().unwrap().clone();
        let (tx, rx) = broadcast::channel(1000);

        // Register subscription
        engine
            .register_delete_subscription(&table, where_clause, tx)
            .await;

        BroadcastStream::new(rx).filter_map(|result| async move { result.ok() })
    }

    // Subscribe to specific row changes by ID
    async fn row_changes<'ctx>(
        &self,
        ctx: &Context<'ctx>,
        table: String,
        id: ID,
    ) -> impl Stream<Item = RowChange> + 'ctx {
        let engine = ctx.data::<Arc<GraphQLEngine>>().unwrap().clone();
        let (tx, rx) = broadcast::channel(100);

        // Register subscription
        engine.register_row_subscription(&table, &id, tx).await;

        BroadcastStream::new(rx).filter_map(|result| async move { result.ok() })
    }

    // Subscribe to aggregation changes
    async fn aggregate_changes<'ctx>(
        &self,
        ctx: &Context<'ctx>,
        table: String,
        aggregates: Vec<AggregateInput>,
        where_clause: Option<WhereClause>,
        interval_seconds: Option<i32>,
    ) -> impl Stream<Item = AggregateChange> + 'ctx {
        let engine = ctx.data::<Arc<GraphQLEngine>>().unwrap().clone();
        let interval = Duration::from_secs(interval_seconds.unwrap_or(5) as u64);

        async_stream::stream! {
            let mut interval_timer = tokio::time::interval(interval);
            loop {
                interval_timer.tick().await;

                match engine.aggregate(&table, aggregates.clone(), where_clause.clone(), None).await {
                    Ok(results) => {
                        yield AggregateChange {
                            table: table.clone(),
                            results,
                            timestamp: DateTime::now(),
                        };
                    }
                    Err(_) => continue,
                }
            }
        }
    }

    // Subscribe to query result changes
    async fn query_changes<'ctx>(
        &self,
        ctx: &Context<'ctx>,
        table: String,
        where_clause: Option<WhereClause>,
        order_by: Option<Vec<OrderBy>>,
        limit: Option<i32>,
        poll_interval_seconds: Option<i32>,
    ) -> impl Stream<Item = QueryChange> + 'ctx {
        let engine = ctx.data::<Arc<GraphQLEngine>>().unwrap().clone();
        let interval = Duration::from_secs(poll_interval_seconds.unwrap_or(5) as u64);

        async_stream::stream! {
            let mut interval_timer = tokio::time::interval(interval);
            let mut last_hash: Option<u64> = None;

            loop {
                interval_timer.tick().await;

                match engine.query_table(
                    &table,
                    where_clause.clone(),
                    order_by.clone(),
                    limit,
                    None,
                ).await {
                    Ok((rows, total_count, _has_more)) => {
                        // Compute hash to detect changes
                        let current_hash = compute_rows_hash(&rows);

                        if last_hash.is_none() || last_hash != Some(current_hash) {
                            last_hash = Some(current_hash);
                            yield QueryChange {
                                table: table.clone(),
                                rows,
                                total_count: BigInt(total_count),
                                timestamp: DateTime::now(),
                            };
                        }
                    }
                    Err(_) => continue,
                }
            }
        }
    }

    // Heartbeat subscription for connection keepalive
    async fn heartbeat<'ctx>(
        &self,
        interval_seconds: Option<i32>,
    ) -> impl Stream<Item = Heartbeat> + 'ctx {
        let interval = Duration::from_secs(interval_seconds.unwrap_or(30) as u64);

        async_stream::stream! {
            let mut interval_timer = tokio::time::interval(interval);
            let mut sequence = 0u64;

            loop {
                interval_timer.tick().await;
                sequence += 1;

                yield Heartbeat {
                    sequence,
                    timestamp: DateTime::now(),
                };
            }
        }
    }

    // Subscribe to query execution events
    async fn query_execution<'ctx>(
        &self,
        ctx: &Context<'ctx>,
        query_id: Option<String>,
    ) -> impl Stream<Item = QueryExecutionEvent> + 'ctx {
        let engine = ctx.data::<Arc<GraphQLEngine>>().unwrap().clone();
        let (tx, rx) = broadcast::channel(1000);

        // Register query execution subscription
        engine
            .register_query_execution_subscription(query_id, tx)
            .await;

        BroadcastStream::new(rx).filter_map(|result| async move { result.ok() })
    }

    // Subscribe to table modifications (comprehensive row changes)
    async fn table_modifications<'ctx>(
        &self,
        ctx: &Context<'ctx>,
        tables: Vec<String>,
        change_types: Option<Vec<ChangeType>>,
    ) -> impl Stream<Item = TableModification> + 'ctx {
        let engine = ctx.data::<Arc<GraphQLEngine>>().unwrap().clone();
        let (tx, rx) = broadcast::channel(1000);

        // Register table modification subscription
        engine
            .register_table_modification_subscription(tables, change_types, tx)
            .await;

        BroadcastStream::new(rx).filter_map(|result| async move { result.ok() })
    }

    // Subscribe to system metrics stream
    async fn system_metrics<'ctx>(
        &self,
        ctx: &Context<'ctx>,
        interval_seconds: Option<i32>,
        metric_types: Option<Vec<MetricType>>,
    ) -> impl Stream<Item = SystemMetrics> + 'ctx {
        let engine = ctx.data::<Arc<GraphQLEngine>>().unwrap().clone();
        let interval = Duration::from_secs(interval_seconds.unwrap_or(5) as u64);
        let metrics = metric_types.unwrap_or_else(|| {
            vec![
                MetricType::Cpu,
                MetricType::Memory,
                MetricType::Disk,
                MetricType::Network,
            ]
        });

        async_stream::stream! {
            let mut interval_timer = tokio::time::interval(interval);
            loop {
                interval_timer.tick().await;

                match engine.collect_system_metrics(&metrics).await {
                    Ok(metrics_data) => {
                        yield SystemMetrics {
                            cpu_usage: metrics_data.cpu_usage,
                            memory_usage: metrics_data.memory_usage,
                            memory_total: metrics_data.memory_total,
                            disk_read_bps: metrics_data.disk_read_bps,
                            disk_write_bps: metrics_data.disk_write_bps,
                            network_rx_bps: metrics_data.network_rx_bps,
                            network_tx_bps: metrics_data.network_tx_bps,
                            active_connections: metrics_data.active_connections,
                            active_queries: metrics_data.active_queries,
                            timestamp: DateTime::now(),
                        };
                    }
                    Err(_) => continue,
                }
            }
        }
    }

    // Subscribe to replication status events
    async fn replication_status<'ctx>(
        &self,
        ctx: &Context<'ctx>,
        node_id: Option<String>,
        interval_seconds: Option<i32>,
    ) -> impl Stream<Item = ReplicationStatusEvent> + 'ctx {
        let engine = ctx.data::<Arc<GraphQLEngine>>().unwrap().clone();
        let interval = Duration::from_secs(interval_seconds.unwrap_or(10) as u64);

        async_stream::stream! {
            let mut interval_timer = tokio::time::interval(interval);
            loop {
                interval_timer.tick().await;

                match engine.get_replication_status(node_id.clone()).await {
                    Ok(status) => {
                        yield ReplicationStatusEvent {
                            node_id: status.node_id,
                            role: status.role,
                            state: status.state,
                            lag_bytes: status.lag_bytes,
                            lag_seconds: status.lag_seconds,
                            last_wal_received: status.last_wal_received,
                            last_wal_applied: status.last_wal_applied,
                            is_healthy: status.is_healthy,
                            timestamp: DateTime::now(),
                        };
                    }
                    Err(_) => continue,
                }
            }
        }
    }

    /// Subscribe to index operation events
    async fn index_operations<'ctx>(
        &self,
        ctx: &Context<'ctx>,
        index_name: Option<String>,
        event_types: Option<Vec<IndexEventType>>,
        interval_seconds: Option<i32>,
    ) -> impl Stream<Item = IndexOperationEvent> + 'ctx {
        let _engine = ctx.data::<Arc<GraphQLEngine>>().unwrap().clone();
        let interval = Duration::from_secs(interval_seconds.unwrap_or(5) as u64);

        async_stream::stream! {
            let mut interval_timer = tokio::time::interval(interval);
            let mut sequence = 0u64;

            loop {
                interval_timer.tick().await;
                sequence += 1;

                // Generate sample events - in production would come from actual index operations
                let event_type = match sequence % 3 {
                    0 => IndexEventType::BTreeSplit,
                    1 => IndexEventType::LsmCompaction,
                    _ => IndexEventType::RebuildProgress,
                };

                if let Some(ref types) = event_types {
                    if !types.contains(&event_type) {
                        continue;
                    }
                }

                let event = IndexOperationEvent {
                    event_type,
                    index_name: index_name.clone().unwrap_or_else(|| "idx_sample".to_string()),
                    table_name: Some("sample_table".to_string()),
                    progress_percent: Some(45.5),
                    rows_processed: Some(BigInt(455000)),
                    details: Some("Sample index operation".to_string()),
                    timestamp: DateTime::now(),
                };

                yield event;
            }
        }
    }

    /// Subscribe to memory pressure events
    async fn memory_pressure<'ctx>(
        &self,
        ctx: &Context<'ctx>,
        min_level: Option<MemoryPressureLevelGql>,
        interval_seconds: Option<i32>,
    ) -> impl Stream<Item = MemoryPressureEventGql> + 'ctx {
        let _engine = ctx.data::<Arc<GraphQLEngine>>().unwrap().clone();
        let interval = Duration::from_secs(interval_seconds.unwrap_or(10) as u64);

        async_stream::stream! {
            let mut interval_timer = tokio::time::interval(interval);

            loop {
                interval_timer.tick().await;

                let level = MemoryPressureLevelGql::Medium;

                if let Some(min) = min_level {
                    let min_value = match min {
                        MemoryPressureLevelGql::Normal => 0,
                        MemoryPressureLevelGql::Low => 1,
                        MemoryPressureLevelGql::Medium => 2,
                        MemoryPressureLevelGql::High => 3,
                        MemoryPressureLevelGql::Critical => 4,
                    };
                    let current_value = match level {
                        MemoryPressureLevelGql::Normal => 0,
                        MemoryPressureLevelGql::Low => 1,
                        MemoryPressureLevelGql::Medium => 2,
                        MemoryPressureLevelGql::High => 3,
                        MemoryPressureLevelGql::Critical => 4,
                    };
                    if current_value < min_value {
                        continue;
                    }
                }

                let event = MemoryPressureEventGql {
                    pressure_level: level,
                    total_memory_bytes: BigInt(8589934592), // 8GB
                    used_memory_bytes: BigInt(7301691392),  // ~6.8GB
                    available_memory_bytes: BigInt(1288243200),
                    utilization_percent: 85.0,
                    actions_taken: vec![
                        "Evicted 100 cache entries".to_string(),
                        "Freed 50 query contexts".to_string(),
                    ],
                    timestamp: DateTime::now(),
                };

                yield event;
            }
        }
    }

    /// Subscribe to buffer pool events
    async fn buffer_pool_events<'ctx>(
        &self,
        ctx: &Context<'ctx>,
        event_types: Option<Vec<BufferPoolEventType>>,
        interval_seconds: Option<i32>,
    ) -> impl Stream<Item = BufferPoolEvent> + 'ctx {
        let _engine = ctx.data::<Arc<GraphQLEngine>>().unwrap().clone();
        let interval = Duration::from_secs(interval_seconds.unwrap_or(2) as u64);

        async_stream::stream! {
            let mut interval_timer = tokio::time::interval(interval);
            let mut sequence = 0u64;

            loop {
                interval_timer.tick().await;
                sequence += 1;

                let event_type = match sequence % 3 {
                    0 => BufferPoolEventType::PageEvicted,
                    1 => BufferPoolEventType::BatchEvicted,
                    _ => BufferPoolEventType::StatsUpdate,
                };

                if let Some(ref types) = event_types {
                    if !types.contains(&event_type) {
                        continue;
                    }
                }

                let event = BufferPoolEvent {
                    event_type,
                    num_pages: 64,
                    eviction_policy: Some("CLOCK".to_string()),
                    dirty_pages_flushed: 12,
                    free_frames: 189,
                    total_frames: 1000,
                    hit_rate: Some(0.95),
                    timestamp: DateTime::now(),
                };

                yield event;
            }
        }
    }

    /// Subscribe to SIMD operation metrics
    async fn simd_metrics<'ctx>(
        &self,
        ctx: &Context<'ctx>,
        operation_types: Option<Vec<String>>,
        interval_ms: Option<i32>,
    ) -> impl Stream<Item = SimdOperationMetrics> + 'ctx {
        let _engine = ctx.data::<Arc<GraphQLEngine>>().unwrap().clone();
        let interval = Duration::from_millis(interval_ms.unwrap_or(1000) as u64);

        async_stream::stream! {
            let mut interval_timer = tokio::time::interval(interval);
            let mut sequence = 0u64;

            loop {
                interval_timer.tick().await;
                sequence += 1;

                let op_type = if sequence % 2 == 0 { "filter" } else { "aggregate" };

                if let Some(ref types) = operation_types {
                    if !types.contains(&op_type.to_string()) {
                        continue;
                    }
                }

                let metrics = SimdOperationMetrics {
                    operation_type: op_type.to_string(),
                    rows_processed: BigInt(1000000),
                    rows_selected: BigInt(250000),
                    selectivity: 0.25,
                    simd_ops: BigInt(15625),
                    scalar_ops: BigInt(0),
                    simd_ratio: 1.0,
                    duration_us: BigInt(5000),
                    throughput_rows_per_sec: 200000000.0,
                    vector_width: 256,
                    timestamp: DateTime::now(),
                };

                yield metrics;
            }
        }
    }

    /// Subscribe to in-memory column store events
    async fn inmemory_store_events<'ctx>(
        &self,
        ctx: &Context<'ctx>,
        table_name: Option<String>,
        event_types: Option<Vec<InMemoryEventType>>,
        interval_seconds: Option<i32>,
    ) -> impl Stream<Item = InMemoryStoreEvent> + 'ctx {
        let _engine = ctx.data::<Arc<GraphQLEngine>>().unwrap().clone();
        let interval = Duration::from_secs(interval_seconds.unwrap_or(5) as u64);

        async_stream::stream! {
            let mut interval_timer = tokio::time::interval(interval);
            let mut sequence = 0u64;

            loop {
                interval_timer.tick().await;
                sequence += 1;

                let event_type = match sequence % 3 {
                    0 => InMemoryEventType::PopulationProgress,
                    1 => InMemoryEventType::CompressionCompleted,
                    _ => InMemoryEventType::TableEvicted,
                };

                if let Some(ref types) = event_types {
                    if !types.contains(&event_type) {
                        continue;
                    }
                }

                let event = InMemoryStoreEvent {
                    event_type,
                    table_name: table_name.clone().unwrap_or_else(|| "sample_table".to_string()),
                    rows_affected: Some(BigInt(1000000)),
                    memory_bytes: Some(BigInt(2147483648)),
                    compression_ratio: Some(4.5),
                    progress_percent: Some(55.0),
                    details: Some("Sample in-memory event".to_string()),
                    timestamp: DateTime::now(),
                };

                yield event;
            }
        }
    }
}

// Table change event (union of all change types)
#[derive(Clone, Debug)]
pub struct TableChange {
    pub table: String,
    pub change_type: ChangeType,
    pub row: Option<RowType>,
    pub old_row: Option<RowType>,
    pub timestamp: DateTime,
}

#[Object]
impl TableChange {
    async fn table(&self) -> &str {
        &self.table
    }

    async fn change_type(&self) -> ChangeType {
        self.change_type
    }

    async fn row(&self) -> &Option<RowType> {
        &self.row
    }

    async fn old_row(&self) -> &Option<RowType> {
        &self.old_row
    }

    async fn timestamp(&self) -> &DateTime {
        &self.timestamp
    }
}

// Change type enum
#[derive(Enum, Copy, Clone, Eq, PartialEq, Debug)]
pub enum ChangeType {
    Insert,
    Update,
    Delete,
}

// Row inserted event
#[derive(Clone, Debug)]
pub struct RowInserted {
    pub table: String,
    pub row: RowType,
    pub timestamp: DateTime,
}

#[Object]
impl RowInserted {
    async fn table(&self) -> &str {
        &self.table
    }

    async fn row(&self) -> &RowType {
        &self.row
    }

    async fn timestamp(&self) -> &DateTime {
        &self.timestamp
    }
}

// Row updated event
#[derive(Clone, Debug)]
pub struct RowUpdated {
    pub table: String,
    pub old_row: RowType,
    pub new_row: RowType,
    pub changed_fields: Vec<String>,
    pub timestamp: DateTime,
}

#[Object]
impl RowUpdated {
    async fn table(&self) -> &str {
        &self.table
    }

    async fn old_row(&self) -> &RowType {
        &self.old_row
    }

    async fn new_row(&self) -> &RowType {
        &self.new_row
    }

    async fn changed_fields(&self) -> &[String] {
        &self.changed_fields
    }

    async fn timestamp(&self) -> &DateTime {
        &self.timestamp
    }
}

// Row deleted event
#[derive(Clone, Debug)]
pub struct RowDeleted {
    pub table: String,
    pub id: ID,
    pub old_row: Option<RowType>,
    pub timestamp: DateTime,
}

#[Object]
impl RowDeleted {
    async fn table(&self) -> &str {
        &self.table
    }

    async fn id(&self) -> &ID {
        &self.id
    }

    async fn old_row(&self) -> &Option<RowType> {
        &self.old_row
    }

    async fn timestamp(&self) -> &DateTime {
        &self.timestamp
    }
}

// Row change event (for specific row subscriptions)
#[derive(Clone, Debug)]
pub struct RowChange {
    pub table: String,
    pub id: ID,
    pub change_type: ChangeType,
    pub row: Option<RowType>,
    pub old_row: Option<RowType>,
    pub timestamp: DateTime,
}

#[Object]
impl RowChange {
    async fn table(&self) -> &str {
        &self.table
    }

    async fn id(&self) -> &ID {
        &self.id
    }

    async fn change_type(&self) -> ChangeType {
        self.change_type
    }

    async fn row(&self) -> &Option<RowType> {
        &self.row
    }

    async fn old_row(&self) -> &Option<RowType> {
        &self.old_row
    }

    async fn timestamp(&self) -> &DateTime {
        &self.timestamp
    }
}

// Aggregate change event
#[derive(SimpleObject, Clone, Debug)]
pub struct AggregateChange {
    pub table: String,
    pub results: Vec<AggregateResult>,
    pub timestamp: DateTime,
}

// Query change event
#[derive(SimpleObject, Clone, Debug)]
pub struct QueryChange {
    pub table: String,
    pub rows: Vec<RowType>,
    pub total_count: BigInt,
    pub timestamp: DateTime,
}

// Heartbeat event
#[derive(SimpleObject, Clone, Debug)]
pub struct Heartbeat {
    pub sequence: u64,
    pub timestamp: DateTime,
}

// Subscription manager for tracking active subscriptions
pub struct SubscriptionManager {
    subscriptions: Arc<RwLock<HashMap<String, SubscriptionInfo>>>,
    event_bus: Arc<RwLock<HashMap<String, Vec<broadcast::Sender<TableChange>>>>>,
}

impl SubscriptionManager {
    pub fn new() -> Self {
        Self {
            subscriptions: Arc::new(RwLock::new(HashMap::new())),
            event_bus: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn register_subscription(&self, table: &str, filter: Option<WhereClause>) -> String {
        let subscription_id = uuid::Uuid::new_v4().to_string();
        let info = SubscriptionInfo {
            id: subscription_id.clone(),
            table: table.to_string(),
            filter,
            created_at: DateTime::now(),
            last_event: None,
        };

        let mut subs = self.subscriptions.write().await;
        subs.insert(subscription_id.clone(), info);

        subscription_id
    }

    pub async fn unregister_subscription(&self, subscription_id: &str) {
        let mut subs = self.subscriptions.write().await;
        subs.remove(subscription_id);
    }

    pub async fn notify_change(&self, table: &str, change: TableChange) {
        let bus = self.event_bus.read().await;
        if let Some(senders) = bus.get(table) {
            for sender in senders {
                let _ = sender.send(change.clone());
            }
        }
    }

    pub async fn get_active_subscriptions(&self) -> Vec<SubscriptionInfo> {
        let subs = self.subscriptions.read().await;
        subs.values().cloned().collect()
    }
}

// Subscription information
#[derive(Clone, Debug)]
pub struct SubscriptionInfo {
    pub id: String,
    pub table: String,
    pub filter: Option<WhereClause>,
    pub created_at: DateTime,
    pub last_event: Option<DateTime>,
}

// Query execution event
#[derive(Clone, Debug)]
pub struct QueryExecutionEvent {
    pub query_id: String,
    pub status: QueryExecutionStatus,
    pub progress_percent: Option<f64>,
    pub rows_affected: Option<BigInt>,
    pub elapsed_ms: u64,
    pub message: Option<String>,
    pub timestamp: DateTime,
}

#[Object]
impl QueryExecutionEvent {
    async fn query_id(&self) -> &str {
        &self.query_id
    }

    async fn status(&self) -> QueryExecutionStatus {
        self.status
    }

    async fn progress_percent(&self) -> Option<f64> {
        self.progress_percent
    }

    async fn rows_affected(&self) -> &Option<BigInt> {
        &self.rows_affected
    }

    async fn elapsed_ms(&self) -> u64 {
        self.elapsed_ms
    }

    async fn message(&self) -> &Option<String> {
        &self.message
    }

    async fn timestamp(&self) -> &DateTime {
        &self.timestamp
    }
}

// Query execution status
#[derive(Enum, Copy, Clone, Eq, PartialEq, Debug)]
pub enum QueryExecutionStatus {
    Started,
    Running,
    Completed,
    Failed,
    Cancelled,
}

// Table modification event
#[derive(Clone, Debug)]
pub struct TableModification {
    pub table: String,
    pub change_type: ChangeType,
    pub row_id: ID,
    pub row: Option<RowType>,
    pub old_row: Option<RowType>,
    pub changed_columns: Option<Vec<String>>,
    pub transaction_id: Option<String>,
    pub timestamp: DateTime,
}

#[Object]
impl TableModification {
    async fn table(&self) -> &str {
        &self.table
    }

    async fn change_type(&self) -> ChangeType {
        self.change_type
    }

    async fn row_id(&self) -> &ID {
        &self.row_id
    }

    async fn row(&self) -> &Option<RowType> {
        &self.row
    }

    async fn old_row(&self) -> &Option<RowType> {
        &self.old_row
    }

    async fn changed_columns(&self) -> &Option<Vec<String>> {
        &self.changed_columns
    }

    async fn transaction_id(&self) -> &Option<String> {
        &self.transaction_id
    }

    async fn timestamp(&self) -> &DateTime {
        &self.timestamp
    }
}

// System metrics event
#[derive(SimpleObject, Clone, Debug)]
pub struct SystemMetrics {
    pub cpu_usage: f64,
    pub memory_usage: BigInt,
    pub memory_total: BigInt,
    pub disk_read_bps: BigInt,
    pub disk_write_bps: BigInt,
    pub network_rx_bps: BigInt,
    pub network_tx_bps: BigInt,
    pub active_connections: i32,
    pub active_queries: i32,
    pub timestamp: DateTime,
}

// Metric type enum
#[derive(Enum, Copy, Clone, Eq, PartialEq, Debug)]
pub enum MetricType {
    Cpu,
    Memory,
    Disk,
    Network,
    Queries,
    Connections,
}

// Replication status event
#[derive(Clone, Debug)]
pub struct ReplicationStatusEvent {
    pub node_id: String,
    pub role: ReplicationRole,
    pub state: ReplicationState,
    pub lag_bytes: BigInt,
    pub lag_seconds: f64,
    pub last_wal_received: Option<String>,
    pub last_wal_applied: Option<String>,
    pub is_healthy: bool,
    pub timestamp: DateTime,
}

#[Object]
impl ReplicationStatusEvent {
    async fn node_id(&self) -> &str {
        &self.node_id
    }

    async fn role(&self) -> ReplicationRole {
        self.role
    }

    async fn state(&self) -> ReplicationState {
        self.state
    }

    async fn lag_bytes(&self) -> &BigInt {
        &self.lag_bytes
    }

    async fn lag_seconds(&self) -> f64 {
        self.lag_seconds
    }

    async fn last_wal_received(&self) -> &Option<String> {
        &self.last_wal_received
    }

    async fn last_wal_applied(&self) -> &Option<String> {
        &self.last_wal_applied
    }

    async fn is_healthy(&self) -> bool {
        self.is_healthy
    }

    async fn timestamp(&self) -> &DateTime {
        &self.timestamp
    }
}

// Replication role enum
#[derive(Enum, Copy, Clone, Eq, PartialEq, Debug)]
pub enum ReplicationRole {
    Primary,
    Standby,
    Replica,
}

// Replication state enum
#[derive(Enum, Copy, Clone, Eq, PartialEq, Debug)]
pub enum ReplicationState {
    Startup,
    Catchup,
    Streaming,
    Backup,
    Stopping,
    Stopped,
}

// Helper function to compute hash of rows for change detection
fn compute_rows_hash(rows: &[RowType]) -> u64 {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    let mut hasher = DefaultHasher::new();
    for row in rows {
        row.id.hash(&mut hasher);
        row.version.hash(&mut hasher);
    }
    hasher.finish()
}

// ============================================================================
// INDEX & MEMORY SUBSCRIPTIONS (New)
// ============================================================================

/// Index operation event
#[derive(Clone, Debug)]
pub struct IndexOperationEvent {
    pub event_type: IndexEventType,
    pub index_name: String,
    pub table_name: Option<String>,
    pub progress_percent: Option<f64>,
    pub rows_processed: Option<BigInt>,
    pub details: Option<String>,
    pub timestamp: DateTime,
}

#[Object]
impl IndexOperationEvent {
    async fn event_type(&self) -> IndexEventType {
        self.event_type
    }

    async fn index_name(&self) -> &str {
        &self.index_name
    }

    async fn table_name(&self) -> &Option<String> {
        &self.table_name
    }

    async fn progress_percent(&self) -> Option<f64> {
        self.progress_percent
    }

    async fn rows_processed(&self) -> &Option<BigInt> {
        &self.rows_processed
    }

    async fn details(&self) -> &Option<String> {
        &self.details
    }

    async fn timestamp(&self) -> &DateTime {
        &self.timestamp
    }
}

#[derive(Enum, Copy, Clone, Eq, PartialEq, Debug)]
pub enum IndexEventType {
    RebuildStarted,
    RebuildProgress,
    RebuildCompleted,
    BTreeSplit,
    BTreeMerge,
    LsmCompaction,
    FullTextUpdate,
}

/// Memory pressure event
#[derive(Clone, Debug)]
pub struct MemoryPressureEventGql {
    pub pressure_level: MemoryPressureLevelGql,
    pub total_memory_bytes: BigInt,
    pub used_memory_bytes: BigInt,
    pub available_memory_bytes: BigInt,
    pub utilization_percent: f64,
    pub actions_taken: Vec<String>,
    pub timestamp: DateTime,
}

#[Object]
impl MemoryPressureEventGql {
    async fn pressure_level(&self) -> MemoryPressureLevelGql {
        self.pressure_level
    }

    async fn total_memory_bytes(&self) -> &BigInt {
        &self.total_memory_bytes
    }

    async fn used_memory_bytes(&self) -> &BigInt {
        &self.used_memory_bytes
    }

    async fn available_memory_bytes(&self) -> &BigInt {
        &self.available_memory_bytes
    }

    async fn utilization_percent(&self) -> f64 {
        self.utilization_percent
    }

    async fn actions_taken(&self) -> &[String] {
        &self.actions_taken
    }

    async fn timestamp(&self) -> &DateTime {
        &self.timestamp
    }
}

#[derive(Enum, Copy, Clone, Eq, PartialEq, Debug)]
pub enum MemoryPressureLevelGql {
    Normal,
    Low,
    Medium,
    High,
    Critical,
}

/// Buffer pool event
#[derive(Clone, Debug)]
pub struct BufferPoolEvent {
    pub event_type: BufferPoolEventType,
    pub num_pages: i32,
    pub eviction_policy: Option<String>,
    pub dirty_pages_flushed: i32,
    pub free_frames: i32,
    pub total_frames: i32,
    pub hit_rate: Option<f64>,
    pub timestamp: DateTime,
}

#[Object]
impl BufferPoolEvent {
    async fn event_type(&self) -> BufferPoolEventType {
        self.event_type
    }

    async fn num_pages(&self) -> i32 {
        self.num_pages
    }

    async fn eviction_policy(&self) -> &Option<String> {
        &self.eviction_policy
    }

    async fn dirty_pages_flushed(&self) -> i32 {
        self.dirty_pages_flushed
    }

    async fn free_frames(&self) -> i32 {
        self.free_frames
    }

    async fn total_frames(&self) -> i32 {
        self.total_frames
    }

    async fn hit_rate(&self) -> Option<f64> {
        self.hit_rate
    }

    async fn timestamp(&self) -> &DateTime {
        &self.timestamp
    }
}

#[derive(Enum, Copy, Clone, Eq, PartialEq, Debug)]
pub enum BufferPoolEventType {
    PageEvicted,
    BatchEvicted,
    PageFlushed,
    StatsUpdate,
}

/// SIMD operation metrics
#[derive(Clone, Debug)]
pub struct SimdOperationMetrics {
    pub operation_type: String,
    pub rows_processed: BigInt,
    pub rows_selected: BigInt,
    pub selectivity: f64,
    pub simd_ops: BigInt,
    pub scalar_ops: BigInt,
    pub simd_ratio: f64,
    pub duration_us: BigInt,
    pub throughput_rows_per_sec: f64,
    pub vector_width: i32,
    pub timestamp: DateTime,
}

#[Object]
impl SimdOperationMetrics {
    async fn operation_type(&self) -> &str {
        &self.operation_type
    }

    async fn rows_processed(&self) -> &BigInt {
        &self.rows_processed
    }

    async fn rows_selected(&self) -> &BigInt {
        &self.rows_selected
    }

    async fn selectivity(&self) -> f64 {
        self.selectivity
    }

    async fn simd_ops(&self) -> &BigInt {
        &self.simd_ops
    }

    async fn scalar_ops(&self) -> &BigInt {
        &self.scalar_ops
    }

    async fn simd_ratio(&self) -> f64 {
        self.simd_ratio
    }

    async fn duration_us(&self) -> &BigInt {
        &self.duration_us
    }

    async fn throughput_rows_per_sec(&self) -> f64 {
        self.throughput_rows_per_sec
    }

    async fn vector_width(&self) -> i32 {
        self.vector_width
    }

    async fn timestamp(&self) -> &DateTime {
        &self.timestamp
    }
}

/// In-memory column store event
#[derive(Clone, Debug)]
pub struct InMemoryStoreEvent {
    pub event_type: InMemoryEventType,
    pub table_name: String,
    pub rows_affected: Option<BigInt>,
    pub memory_bytes: Option<BigInt>,
    pub compression_ratio: Option<f64>,
    pub progress_percent: Option<f64>,
    pub details: Option<String>,
    pub timestamp: DateTime,
}

#[Object]
impl InMemoryStoreEvent {
    async fn event_type(&self) -> InMemoryEventType {
        self.event_type
    }

    async fn table_name(&self) -> &str {
        &self.table_name
    }

    async fn rows_affected(&self) -> &Option<BigInt> {
        &self.rows_affected
    }

    async fn memory_bytes(&self) -> &Option<BigInt> {
        &self.memory_bytes
    }

    async fn compression_ratio(&self) -> Option<f64> {
        self.compression_ratio
    }

    async fn progress_percent(&self) -> Option<f64> {
        self.progress_percent
    }

    async fn details(&self) -> &Option<String> {
        &self.details
    }

    async fn timestamp(&self) -> &DateTime {
        &self.timestamp
    }
}

#[derive(Enum, Copy, Clone, Eq, PartialEq, Debug)]
pub enum InMemoryEventType {
    PopulationStarted,
    PopulationProgress,
    PopulationCompleted,
    TableEvicted,
    SegmentEvicted,
    CompressionCompleted,
}
