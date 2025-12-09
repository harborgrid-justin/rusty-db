// GraphQL Subscription Operations
//
// Real-time subscription resolvers for the GraphQL API

use async_graphql::{
    Context, Enum, Object, SimpleObject, Subscription, ID,
};
use futures_util::stream::Stream;
use futures_util::StreamExt;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{broadcast, RwLock};
use tokio_stream::wrappers::BroadcastStream;
use super::types::*;
use super::models::*;
use super::GraphQLEngine;

// ============================================================================
// PART 4: SUBSCRIPTION SYSTEM (600+ lines)
// ============================================================================

/// Root subscription type
pub struct SubscriptionRoot;

#[Subscription]
impl SubscriptionRoot {
    /// Subscribe to all changes on a table
    async fn table_changes<'ctx>(
        &self,
        ctx: &Context<'ctx>,
        table: String,
        where_clause: Option<WhereClause>,
    ) -> impl Stream<Item = TableChange> + 'ctx {
        let engine = ctx.data::<Arc<GraphQLEngine>>().unwrap().clone();
        let (tx, rx) = broadcast::channel(1000);

        // Register subscription
        let _subscription_id = engine.register_table_subscription(&table, where_clause, tx).await;

        BroadcastStream::new(rx).filter_map(|result| async move {
            result.ok()
        })
    }

    /// Subscribe to row insertions
    async fn row_inserted<'ctx>(
        &self,
        ctx: &Context<'ctx>,
        table: String,
        where_clause: Option<WhereClause>,
    ) -> impl Stream<Item = RowInserted> + 'ctx {
        let engine = ctx.data::<Arc<GraphQLEngine>>().unwrap().clone();
        let (tx, rx) = broadcast::channel(1000);

        // Register subscription
        engine.register_insert_subscription(&table, where_clause, tx).await;

        BroadcastStream::new(rx).filter_map(|result| async move {
            result.ok()
        })
    }

    /// Subscribe to row updates
    async fn row_updated<'ctx>(
        &self,
        ctx: &Context<'ctx>,
        table: String,
        where_clause: Option<WhereClause>,
    ) -> impl Stream<Item = RowUpdated> + 'ctx {
        let engine = ctx.data::<Arc<GraphQLEngine>>().unwrap().clone();
        let (tx, rx) = broadcast::channel(1000);

        // Register subscription
        engine.register_update_subscription(&table, where_clause, tx).await;

        BroadcastStream::new(rx).filter_map(|result| async move {
            result.ok()
        })
    }

    /// Subscribe to row deletions
    async fn row_deleted<'ctx>(
        &self,
        ctx: &Context<'ctx>,
        table: String,
        where_clause: Option<WhereClause>,
    ) -> impl Stream<Item = RowDeleted> + 'ctx {
        let engine = ctx.data::<Arc<GraphQLEngine>>().unwrap().clone();
        let (tx, rx) = broadcast::channel(1000);

        // Register subscription
        engine.register_delete_subscription(&table, where_clause, tx).await;

        BroadcastStream::new(rx).filter_map(|result| async move {
            result.ok()
        })
    }

    /// Subscribe to specific row changes by ID
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

        BroadcastStream::new(rx).filter_map(|result| async move {
            result.ok()
        })
    }

    /// Subscribe to aggregation changes
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

    /// Subscribe to query result changes
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

    /// Heartbeat subscription for connection keepalive
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
}

/// Table change event (union of all change types)
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

/// Change type enum
#[derive(Enum, Copy, Clone, Eq, PartialEq, Debug)]
pub enum ChangeType {
    Insert,
    Update,
    Delete,
}

/// Row inserted event
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

/// Row updated event
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

/// Row deleted event
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

/// Row change event (for specific row subscriptions)
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

/// Aggregate change event
#[derive(SimpleObject, Clone, Debug)]
pub struct AggregateChange {
    pub table: String,
    pub results: Vec<AggregateResult>,
    pub timestamp: DateTime,
}

/// Query change event
#[derive(SimpleObject, Clone, Debug)]
pub struct QueryChange {
    pub table: String,
    pub rows: Vec<RowType>,
    pub total_count: BigInt,
    pub timestamp: DateTime,
}

/// Heartbeat event
#[derive(SimpleObject, Clone, Debug)]
pub struct Heartbeat {
    pub sequence: u64,
    pub timestamp: DateTime,
}

/// Subscription manager for tracking active subscriptions
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

    pub async fn register_subscription(
        &self,
        table: &str,
        filter: Option<WhereClause>,
    ) -> String {
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

/// Subscription information
#[derive(Clone, Debug)]
pub struct SubscriptionInfo {
    pub id: String,
    pub table: String,
    pub filter: Option<WhereClause>,
    pub created_at: DateTime,
    pub last_event: Option<DateTime>,
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
