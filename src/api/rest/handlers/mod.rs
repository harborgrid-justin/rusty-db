// REST API Handlers Module
//
// Organizes handler functions into logical groups

pub mod auth;
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
pub mod system;

// Enterprise Integration Handlers
pub mod enterprise_auth_handlers;
pub mod backup_handlers;
pub mod replication_handlers;
pub mod audit_handlers;

// Security Handlers
pub mod encryption_handlers;
pub mod masking_handlers;
pub mod vpd_handlers;
pub mod privileges_handlers;
pub mod labels_handlers;
pub mod analytics_handlers;
mod diagnostics_handlers;
mod gateway_handlers;
mod flashback_handlers;
pub mod health_handlers;
mod index_handlers;
mod streams_handlers;
pub mod spatial_handlers;
mod security_handlers;
mod optimizer_handlers;
mod rac_handlers;
pub mod ml_handlers;
mod memory_handlers;
pub mod inmemory_handlers;
pub mod graph_handlers;
pub mod document_handlers;
mod dashboard_handlers;
pub mod websocket_handlers;
pub mod websocket_types;

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

// Auth handlers
pub use auth::{login, logout, refresh, validate};

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

// System handlers
pub use system::{
    get_server_config, get_clustering_status, get_replication_status_info,
    get_security_features, get_server_info
};

// Encryption handlers
pub use encryption_handlers::{
    get_encryption_status, enable_encryption, enable_column_encryption,
    generate_key, rotate_key, list_keys
};

// Masking handlers
pub use masking_handlers::{
    list_masking_policies, get_masking_policy, create_masking_policy,
    update_masking_policy, delete_masking_policy, test_masking,
    enable_masking_policy, disable_masking_policy
};

// VPD handlers
pub use vpd_handlers::{
    list_vpd_policies, get_vpd_policy, create_vpd_policy,
    update_vpd_policy, delete_vpd_policy, test_vpd_predicate,
    get_table_policies, enable_vpd_policy, disable_vpd_policy
};

// Privilege handlers
pub use privileges_handlers::{
    grant_privilege, revoke_privilege, get_user_privileges,
    analyze_user_privileges, get_role_privileges, get_object_privileges,
    validate_privilege
};

// Security labels handlers
pub use labels_handlers::{
    list_compartments, create_compartment, get_compartment, delete_compartment,
    get_user_clearance, set_user_clearance, check_label_dominance,
    validate_label_access, list_classifications
};

// Health handlers
pub use health_handlers::{
    liveness_probe, readiness_probe, startup_probe, full_health_check,
    LivenessProbeResponse, ReadinessProbeResponse, StartupProbeResponse, FullHealthResponse
};

// WebSocket handlers
pub use websocket_handlers::{
    ws_upgrade_handler, ws_query_stream, ws_metrics_stream,
    ws_events_stream, ws_replication_stream,
    // WebSocket management REST endpoints
    get_websocket_status, list_connections,
    get_connection as get_ws_connection, disconnect_connection,
    broadcast_message, list_subscriptions, create_subscription,
    delete_subscription
};
