// REST API Handlers Module
//
// Organizes handler functions into logical groups

pub mod db;
pub mod admin;
pub mod monitoring;
pub mod pool;
pub mod cluster;
pub mod sql;
pub mod string_functions;
pub mod storage_handlers;
pub mod transaction_handlers;
pub mod network_handlers;

use std::sync::Arc;
use crate::catalog::Catalog;
use crate::transaction::TransactionManager;
use crate::parser::SqlParser;
use parking_lot::RwLock;

lazy_static::lazy_static! {
    pub static ref CATALOG: Arc<RwLock<Catalog>> = Arc::new(RwLock::new(Catalog::new()));
    pub static ref TXN_MANAGER: Arc<TransactionManager> = Arc::new(TransactionManager::new());
    pub static ref SQL_PARSER: SqlParser = SqlParser::new();
}

// Re-export all handler functions for convenience
// Using explicit imports to avoid ambiguous glob re-exports
pub use db::*;

// Admin handlers
pub use admin::{
    get_config, update_config, create_backup, get_health, run_maintenance,
    get_users, create_user, get_user, update_user, delete_user,
    get_roles, create_role, get_role, update_role, delete_role,
    get_metrics as get_admin_metrics
};

// Monitoring handlers
pub use monitoring::{
    get_metrics as get_monitoring_metrics, get_prometheus_metrics,
    get_session_stats, get_query_stats, get_performance_data,
    get_logs, get_alerts, acknowledge_alert, get_all_pools
};

// Pool handlers
pub use pool::{
    get_pools, get_pool, update_pool, get_pool_stats, drain_pool,
    get_connections, get_connection, kill_connection,
    get_sessions, get_session, terminate_session
};

// Cluster handlers
pub use cluster::{
    get_cluster_nodes, add_cluster_node, get_cluster_node, remove_cluster_node,
    get_cluster_topology, trigger_failover, get_replication_status,
    get_cluster_config, update_cluster_config
};

// SQL handlers
pub use sql::*;

// Storage handlers
pub use storage_handlers::{
    get_storage_status, get_disks, get_partitions, create_partition, delete_partition,
    get_buffer_pool_stats, flush_buffer_pool, get_tablespaces, create_tablespace,
    update_tablespace, delete_tablespace, get_io_stats
};

// Transaction handlers
pub use transaction_handlers::{
    get_active_transactions, get_transaction, rollback_transaction,
    get_locks, get_lock_waiters, get_deadlocks, detect_deadlocks,
    get_mvcc_status, trigger_vacuum, get_wal_status, force_checkpoint
};

// Network handlers
pub use network_handlers::{
    get_network_status, get_connections as get_network_connections,
    get_connection as get_network_connection, kill_connection as kill_network_connection,
    get_protocols, update_protocols, get_cluster_status as get_network_cluster_status,
    get_cluster_nodes as get_network_cluster_nodes, add_cluster_node as add_network_cluster_node,
    remove_cluster_node as remove_network_cluster_node, get_loadbalancer_stats,
    configure_loadbalancer, get_circuit_breakers
};
