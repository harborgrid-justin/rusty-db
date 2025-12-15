// GraphQL Transaction Subscriptions
//
// Real-time GraphQL subscriptions for transaction layer monitoring

use async_graphql::{SimpleObject, Subscription, ID};
use futures_util::stream::Stream;
use std::time::Duration;
use serde::{Deserialize, Serialize};

// ============================================================================
// Transaction Subscription Types
// ============================================================================

/// Transaction lifecycle event
#[derive(Debug, Clone, Serialize, Deserialize, SimpleObject)]
pub struct TransactionLifecycleEvent {
    /// Transaction ID
    pub transaction_id: ID,
    /// Event type: begin, commit, rollback, timeout
    pub event_type: String,
    /// Isolation level
    pub isolation_level: String,
    /// Timestamp (Unix epoch)
    pub timestamp: i64,
    /// Whether transaction is read-only
    pub read_only: bool,
}

/// Lock event for subscription
#[derive(Debug, Clone, Serialize, Deserialize, SimpleObject)]
pub struct LockEventGql {
    /// Transaction ID that owns or waits for the lock
    pub transaction_id: ID,
    /// Resource being locked
    pub resource_id: String,
    /// Lock mode: shared, exclusive, intent_shared, etc.
    pub lock_mode: String,
    /// Event type: acquired, released, waiting, timeout
    pub event_type: String,
    /// Wait time in milliseconds (if applicable)
    pub wait_time_ms: Option<i64>,
    /// Timestamp
    pub timestamp: i64,
}

/// Deadlock detection event
#[derive(Debug, Clone, Serialize, Deserialize, SimpleObject)]
pub struct DeadlockEventGql {
    /// Unique deadlock identifier
    pub deadlock_id: ID,
    /// List of transactions in the deadlock cycle
    pub cycle: Vec<ID>,
    /// Transaction selected as victim
    pub victim: ID,
    /// Resolution strategy used
    pub resolution: String,
    /// Detection timestamp
    pub detected_at: i64,
}

/// MVCC version event
#[derive(Debug, Clone, Serialize, Deserialize, SimpleObject)]
pub struct MvccVersionEvent {
    /// Transaction that created the version
    pub transaction_id: ID,
    /// Table name
    pub table: String,
    /// Row key
    pub key: String,
    /// Event type: created, deleted, garbage_collected
    pub event_type: String,
    /// Number of versions for this key
    pub version_count: i32,
    /// Timestamp
    pub timestamp: i64,
}

/// WAL operation event
#[derive(Debug, Clone, Serialize, Deserialize, SimpleObject)]
pub struct WalOperationEvent {
    /// Log Sequence Number
    pub lsn: String,
    /// Operation type: write, flush, checkpoint
    pub operation: String,
    /// Transaction ID (if applicable)
    pub transaction_id: Option<ID>,
    /// Size in bytes
    pub size_bytes: i64,
    /// Timestamp
    pub timestamp: i64,
}

/// Two-phase commit event
#[derive(Debug, Clone, Serialize, Deserialize, SimpleObject)]
pub struct TwoPhaseCommitEventGql {
    /// Transaction ID
    pub transaction_id: ID,
    /// Coordinator node ID
    pub coordinator_id: String,
    /// Participant node ID
    pub participant_id: String,
    /// Phase: prepare, commit, abort
    pub phase: String,
    /// State: preparing, prepared, committed, aborted
    pub state: String,
    /// Timestamp
    pub timestamp: i64,
}

/// Transaction statistics snapshot
#[derive(Debug, Clone, Serialize, Deserialize, SimpleObject)]
pub struct TransactionStats {
    /// Total commits since start
    pub total_commits: i64,
    /// Total aborts since start
    pub total_aborts: i64,
    /// Total deadlocks detected
    pub total_deadlocks: i64,
    /// Currently active transactions
    pub active_transactions: i32,
    /// Average commit latency in milliseconds
    pub avg_commit_latency_ms: i64,
    /// 99th percentile latency
    pub p99_latency_ms: i64,
    /// Abort rate (0.0 - 1.0)
    pub abort_rate: f64,
    /// Timestamp of this snapshot
    pub timestamp: i64,
}

// ============================================================================
// Subscription Root Extension
// ============================================================================

/// Transaction subscription operations
///
/// Add these to your SubscriptionRoot implementation:
/// ```ignore
/// impl SubscriptionRoot {
///     // ... existing subscriptions ...
///
///     // Transaction subscriptions
///     use super::transaction_subscriptions::*;
///
///     async fn transaction_lifecycle<'ctx>(
///         &self,
///         ctx: &Context<'ctx>,
///         transaction_ids: Option<Vec<ID>>,
///     ) -> impl Stream<Item = TransactionLifecycleEvent> + 'ctx { ... }
/// }
/// ```

pub struct TransactionSubscriptions;

impl TransactionSubscriptions {
    /// Creates a mock transaction lifecycle event stream
    pub fn lifecycle_stream(
        transaction_ids: Option<Vec<ID>>,
    ) -> impl Stream<Item = TransactionLifecycleEvent> {
        async_stream::stream! {
            let mut interval = tokio::time::interval(Duration::from_secs(2));
            loop {
                interval.tick().await;

                let event = TransactionLifecycleEvent {
                    transaction_id: ID::from(format!("txn_{}", rand::random::<u32>())),
                    event_type: "begin".to_string(),
                    isolation_level: "READ_COMMITTED".to_string(),
                    timestamp: chrono::Utc::now().timestamp(),
                    read_only: false,
                };

                // Filter by transaction IDs if specified
                if let Some(ref ids) = transaction_ids {
                    if ids.contains(&event.transaction_id) {
                        yield event;
                    }
                } else {
                    yield event;
                }
            }
        }
    }

    /// Creates a mock lock events stream
    pub fn lock_events_stream(
        transaction_id: Option<ID>,
    ) -> impl Stream<Item = LockEventGql> {
        async_stream::stream! {
            let mut interval = tokio::time::interval(Duration::from_secs(1));
            loop {
                interval.tick().await;

                let txn_id = transaction_id.clone().unwrap_or_else(|| {
                    ID::from(format!("txn_{}", rand::random::<u32>()))
                });

                let event = LockEventGql {
                    transaction_id: txn_id,
                    resource_id: format!("table.users.row_{}", rand::random::<u32>() % 1000),
                    lock_mode: "EXCLUSIVE".to_string(),
                    event_type: "acquired".to_string(),
                    wait_time_ms: Some(rand::random::<i64>() % 100),
                    timestamp: chrono::Utc::now().timestamp(),
                };

                yield event;
            }
        }
    }

    /// Creates a mock deadlock events stream
    pub fn deadlock_events_stream() -> impl Stream<Item = DeadlockEventGql> {
        async_stream::stream! {
            let mut interval = tokio::time::interval(Duration::from_secs(30));
            loop {
                interval.tick().await;

                let txn1 = ID::from(format!("txn_{}", rand::random::<u32>()));
                let txn2 = ID::from(format!("txn_{}", rand::random::<u32>()));

                let event = DeadlockEventGql {
                    deadlock_id: ID::from(uuid::Uuid::new_v4().to_string()),
                    cycle: vec![txn1.clone(), txn2.clone(), txn1.clone()],
                    victim: txn2,
                    resolution: "abort_youngest".to_string(),
                    detected_at: chrono::Utc::now().timestamp(),
                };

                yield event;
            }
        }
    }

    /// Creates a mock MVCC version events stream
    pub fn mvcc_events_stream(
        table: Option<String>,
    ) -> impl Stream<Item = MvccVersionEvent> {
        async_stream::stream! {
            let mut interval = tokio::time::interval(Duration::from_secs(3));
            loop {
                interval.tick().await;

                let target_table = table.clone().unwrap_or_else(|| "users".to_string());

                let event = MvccVersionEvent {
                    transaction_id: ID::from(format!("txn_{}", rand::random::<u32>())),
                    table: target_table,
                    key: format!("key_{}", rand::random::<u32>() % 1000),
                    event_type: "created".to_string(),
                    version_count: rand::random::<i32>() % 5 + 1,
                    timestamp: chrono::Utc::now().timestamp(),
                };

                yield event;
            }
        }
    }

    /// Creates a mock WAL operations stream
    pub fn wal_events_stream() -> impl Stream<Item = WalOperationEvent> {
        async_stream::stream! {
            let mut interval = tokio::time::interval(Duration::from_millis(500));
            let mut lsn_counter = 0u64;

            loop {
                interval.tick().await;
                lsn_counter += 1;

                let event = WalOperationEvent {
                    lsn: format!("0/{:08X}", lsn_counter),
                    operation: "write".to_string(),
                    transaction_id: Some(ID::from(format!("txn_{}", rand::random::<u32>()))),
                    size_bytes: (rand::random::<i64>() % 4096) + 128,
                    timestamp: chrono::Utc::now().timestamp(),
                };

                yield event;
            }
        }
    }

    /// Creates a mock two-phase commit events stream
    pub fn tpc_events_stream() -> impl Stream<Item = TwoPhaseCommitEventGql> {
        async_stream::stream! {
            let mut interval = tokio::time::interval(Duration::from_secs(5));
            loop {
                interval.tick().await;

                let event = TwoPhaseCommitEventGql {
                    transaction_id: ID::from(format!("txn_{}", rand::random::<u32>())),
                    coordinator_id: format!("node_{}", rand::random::<u32>() % 3),
                    participant_id: format!("node_{}", rand::random::<u32>() % 3),
                    phase: "prepare".to_string(),
                    state: "prepared".to_string(),
                    timestamp: chrono::Utc::now().timestamp(),
                };

                yield event;
            }
        }
    }

    /// Creates a transaction statistics stream
    pub fn stats_stream(interval_seconds: Option<i32>) -> impl Stream<Item = TransactionStats> {
        async_stream::stream! {
            let interval_dur = Duration::from_secs(interval_seconds.unwrap_or(5) as u64);
            let mut interval = tokio::time::interval(interval_dur);

            loop {
                interval.tick().await;

                let stats = TransactionStats {
                    total_commits: rand::random::<i64>() % 100000 + 50000,
                    total_aborts: rand::random::<i64>() % 1000 + 500,
                    total_deadlocks: rand::random::<i64>() % 50,
                    active_transactions: rand::random::<i32>() % 100,
                    avg_commit_latency_ms: rand::random::<i64>() % 50 + 10,
                    p99_latency_ms: rand::random::<i64>() % 200 + 50,
                    abort_rate: (rand::random::<u32>() % 5) as f64 / 100.0,
                    timestamp: chrono::Utc::now().timestamp(),
                };

                yield stats;
            }
        }
    }
}

// ============================================================================
// Example Subscription Schema Extension
// ============================================================================

/// Example of how to add these to your existing SubscriptionRoot:
///
/// ```rust,ignore
/// #[Subscription]
/// impl SubscriptionRoot {
///     /// Subscribe to transaction lifecycle events
///     async fn transaction_lifecycle<'ctx>(
///         &self,
///         ctx: &Context<'ctx>,
///         #[graphql(desc = "Filter by specific transaction IDs")]
///         transaction_ids: Option<Vec<ID>>,
///     ) -> impl Stream<Item = TransactionLifecycleEvent> + 'ctx {
///         TransactionSubscriptions::lifecycle_stream(transaction_ids)
///     }
///
///     /// Subscribe to lock events
///     async fn lock_events<'ctx>(
///         &self,
///         #[graphql(desc = "Filter by transaction ID")]
///         transaction_id: Option<ID>,
///     ) -> impl Stream<Item = LockEventGql> + 'ctx {
///         TransactionSubscriptions::lock_events_stream(transaction_id)
///     }
///
///     /// Subscribe to deadlock detection events
///     async fn deadlock_events<'ctx>(
///         &self,
///     ) -> impl Stream<Item = DeadlockEventGql> + 'ctx {
///         TransactionSubscriptions::deadlock_events_stream()
///     }
///
///     /// Subscribe to MVCC version events
///     async fn mvcc_events<'ctx>(
///         &self,
///         #[graphql(desc = "Filter by table name")]
///         table: Option<String>,
///     ) -> impl Stream<Item = MvccVersionEvent> + 'ctx {
///         TransactionSubscriptions::mvcc_events_stream(table)
///     }
///
///     /// Subscribe to WAL operations
///     async fn wal_events<'ctx>(
///         &self,
///     ) -> impl Stream<Item = WalOperationEvent> + 'ctx {
///         TransactionSubscriptions::wal_events_stream()
///     }
///
///     /// Subscribe to two-phase commit events
///     async fn two_phase_commit_events<'ctx>(
///         &self,
///     ) -> impl Stream<Item = TwoPhaseCommitEventGql> + 'ctx {
///         TransactionSubscriptions::tpc_events_stream()
///     }
///
///     /// Subscribe to transaction statistics
///     async fn transaction_stats<'ctx>(
///         &self,
///         #[graphql(desc = "Update interval in seconds", default = 5)]
///         interval_seconds: Option<i32>,
///     ) -> impl Stream<Item = TransactionStats> + 'ctx {
///         TransactionSubscriptions::stats_stream(interval_seconds)
///     }
/// }
/// ```
///
/// This example shows how to integrate transaction subscriptions with your GraphQL schema.
pub struct TransactionSubscriptionsExample;
