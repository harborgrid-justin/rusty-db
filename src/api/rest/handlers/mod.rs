// REST API Handlers Module
//
// Organizes handler functions into logical groups

pub mod admin;
pub mod auth;
pub mod cluster;
pub mod config_handlers;
pub mod db;
pub mod monitoring;
pub mod network_handlers;
pub mod pool;
pub mod sql;
pub mod storage_handlers;
pub mod storage_websocket_handlers;
pub mod string_functions;
pub mod system;
pub mod transaction_handlers;

// Enterprise Integration Handlers
pub mod advanced_replication_handlers;
pub mod audit_handlers;
pub mod backup_handlers;
pub mod enterprise_auth_handlers;
pub mod replication_handlers;

// Security Handlers
pub mod analytics_handlers;
pub mod cluster_websocket_handlers;
pub mod dashboard_handlers;
pub mod diagnostics_handlers;
pub mod document_handlers;
pub mod encryption_handlers;
pub mod flashback_handlers;
pub mod gateway_handlers;
pub mod autonomous_handlers;
pub mod blockchain_handlers;
pub mod event_processing_handlers;
pub mod multitenant_handlers;
pub mod graph_handlers;
pub mod health_handlers;
pub mod index_handlers;
pub mod index_memory_websocket_handlers;
pub mod inmemory_handlers;
pub mod labels_handlers;
pub mod masking_handlers;
pub mod memory_handlers;
pub mod buffer_pool_handlers;
pub mod simd_handlers;
pub mod ml_handlers;
pub mod optimizer_handlers;
pub mod privileges_handlers;
pub mod query_operations;
pub mod query_websocket;
pub mod rac_handlers;
pub mod replication_websocket_types;
pub mod security_handlers;
pub mod security_websocket_handlers;
pub mod spatial_handlers;
pub mod streams_handlers;
pub mod vpd_handlers;
pub mod websocket_handlers;
pub mod transaction_websocket_handlers;
pub mod websocket_types;
pub mod ml_websocket_handlers;
pub mod analytics_websocket_handlers;
mod specialized_data_websocket_handlers;
mod transaction_ws_handlers;
mod enterprise_websocket_handlers;
mod transaction_ws_types;

use crate::catalog::Catalog;
use crate::parser::SqlParser;
use crate::transaction::TransactionManager;
use parking_lot::RwLock;
use std::sync::Arc;

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
    create_backup, create_role, create_user, delete_role, delete_user, get_config, get_health,
    get_metrics as get_admin_metrics, get_role, get_roles, get_user, get_users, run_maintenance,
    update_config, update_role, update_user,
};

// Monitoring handlers
pub use monitoring::{
    acknowledge_alert, get_alerts, get_all_pools, get_logs, get_metrics as get_monitoring_metrics,
    get_performance_data, get_prometheus_metrics, get_query_stats, get_session_stats,
};

// Pool handlers
pub use pool::{
    drain_pool, get_connection, get_connections, get_pool, get_pool_stats, get_pools, get_session,
    get_sessions, kill_connection, terminate_session, update_pool,
};

// Cluster handlers
pub use cluster::{
    add_cluster_node, get_cluster_config, get_cluster_node, get_cluster_nodes,
    get_cluster_topology, get_replication_status, remove_cluster_node, trigger_failover,
    update_cluster_config,
};

// SQL handlers
pub use sql::*;

// Storage handlers
pub use storage_handlers::{
    create_partition, create_tablespace, delete_partition, delete_tablespace, flush_buffer_pool,
    get_buffer_pool_stats, get_disks, get_io_stats, get_partitions, get_storage_status,
    get_tablespaces, update_tablespace,
    // Page Management
    create_page, get_page, compact_page, flush_page, list_pages,
    // LSM Tree
    create_lsm_tree, lsm_put, lsm_get, lsm_delete, lsm_compact, get_lsm_stats,
    // Columnar Storage
    create_columnar_table, columnar_batch_insert, columnar_scan, columnar_project, get_columnar_stats,
    // Tiered Storage
    get_tier_stats, migrate_tier, get_tier_info,
    // JSON Storage
    json_extract, json_set, json_delete, json_merge,
    // Vectored I/O
    vectored_read, vectored_write
};

// Storage WebSocket handlers
pub use storage_websocket_handlers::{
    ws_buffer_pool_events, ws_lsm_events, ws_disk_io_events,
    ws_tier_events, ws_page_events, ws_columnar_events
};

// Transaction handlers
pub use transaction_handlers::{
    archive_wal,
    // Savepoint operations
    create_savepoint,
    detect_deadlocks,
    force_checkpoint,
    get_active_transactions,
    get_deadlocks,
    get_lock_graph,
    get_lock_waiters,
    get_locks,
    // MVCC control operations
    get_mvcc_snapshots,
    get_mvcc_status,
    get_row_versions,
    get_transaction,
    get_wal_replay_status,
    // WAL control operations
    get_wal_segments,
    get_wal_status,
    release_all_locks,
    // Lock control operations
    release_lock,
    release_savepoint,
    rollback_to_savepoint,
    rollback_transaction,
    switch_wal_segment,
    trigger_full_vacuum,
    trigger_vacuum,
    // Isolation level control
    update_isolation_level,
};

// Network handlers
pub use network_handlers::{
    add_cluster_node as add_network_cluster_node, configure_loadbalancer, get_circuit_breakers,
    get_cluster_nodes as get_network_cluster_nodes,
    get_cluster_status as get_network_cluster_status, get_connection as get_network_connection,
    get_connections as get_network_connections, get_loadbalancer_stats, get_network_status,
    get_protocols, kill_connection as kill_network_connection,
    remove_cluster_node as remove_network_cluster_node, update_protocols,
};

// System handlers
pub use system::{
    get_clustering_status, get_replication_status_info, get_security_features, get_server_config,
    get_server_info,
};

// Encryption handlers
pub use encryption_handlers::{
    enable_column_encryption, enable_encryption, generate_key, get_encryption_status, list_keys,
    rotate_key,
};

// Masking handlers
pub use masking_handlers::{
    create_masking_policy, delete_masking_policy, disable_masking_policy, enable_masking_policy,
    get_masking_policy, list_masking_policies, test_masking, update_masking_policy,
};

// VPD handlers
pub use vpd_handlers::{
    create_vpd_policy, delete_vpd_policy, disable_vpd_policy, enable_vpd_policy,
    get_table_policies, get_vpd_policy, list_vpd_policies, test_vpd_predicate, update_vpd_policy,
};

// Privilege handlers
pub use privileges_handlers::{
    analyze_user_privileges, get_object_privileges, get_role_privileges, get_user_privileges,
    grant_privilege, revoke_privilege, validate_privilege,
};

// Security labels handlers
pub use labels_handlers::{
    check_label_dominance, create_compartment, delete_compartment, get_compartment,
    get_user_clearance, list_classifications, list_compartments, set_user_clearance,
    validate_label_access,
};

// Health handlers
pub use health_handlers::{
    full_health_check, liveness_probe, readiness_probe, startup_probe, FullHealthResponse,
    LivenessProbeResponse, ReadinessProbeResponse, StartupProbeResponse,
};

// WebSocket handlers
pub use websocket_handlers::{
    broadcast_message,
    create_subscription,
    delete_subscription,
    disconnect_connection,
    get_connection as get_ws_connection,
    // WebSocket management REST endpoints
    get_websocket_status,
    list_connections,
    list_subscriptions,
    ws_events_stream,
    ws_metrics_stream,
    ws_query_stream,
    ws_replication_stream,
    ws_upgrade_handler,
};

// Security WebSocket handlers
pub use security_websocket_handlers::{
    ws_audit_stream, ws_authentication_events, ws_encryption_events, ws_rate_limit_events,
    ws_security_events, ws_threat_alerts,
};

// Cluster WebSocket handlers
pub use cluster_websocket_handlers::{
    ws_cluster_events, ws_rac_events, ws_replication_events, ws_sharding_events,
};

// Cluster event types
pub use replication_websocket_types::*;

// Diagnostics handlers
pub use diagnostics_handlers::{
    create_dump, download_dump, get_active_session_history, get_dump_status, get_incidents,
    get_query_profiling,
};

// Dashboard handlers
pub use dashboard_handlers::{
    create_dashboard, delete_dashboard, get_dashboard, list_dashboards, update_dashboard,
    ws_dashboard_stream,
};

// ML handlers
pub use ml_handlers::{
    create_model, delete_model, evaluate_model, export_model, get_model, get_model_metrics,
    list_models, predict, train_model,
};

// Analytics handlers
pub use analytics_handlers::{
    analyze_workload, create_materialized_view, create_olap_cube, delete_olap_cube,
    get_quality_issues, get_quality_metrics, get_query_statistics, get_recommendations,
    list_materialized_views, list_olap_cubes, profile_table, query_olap_cube,
    refresh_materialized_view,
};

// InMemory handlers
pub use inmemory_handlers::{
    compact_memory, disable_inmemory, enable_inmemory, evict_tables, get_inmemory_config,
    get_table_status, inmemory_stats, inmemory_status, populate_table, update_inmemory_config,
};

// Optimizer handlers
pub use optimizer_handlers::{
    // Adaptive Execution
    enable_adaptive_execution,
    get_adaptive_statistics,
    get_adaptive_status,
    // Optimizer Hints
    apply_hints,
    get_active_hints,
    get_hint_recommendations,
    list_hints,
    remove_hint,
    // EXPLAIN
    explain_analyze_query,
    explain_query,
    explain_query_with_visualization,
    // Parallel Query Config
    get_parallel_config,
    get_parallel_statistics,
    update_parallel_config,
    // Plan Baselines
    create_baseline,
    delete_baseline,
    evolve_baseline,
    get_baseline,
    list_baselines,
    load_baselines,
    update_baseline,
};

// Query Operations handlers
pub use query_operations::{
    cancel_query, execute_adaptive_query, execute_cte_query, execute_parallel_query,
    execute_query_with_monitoring, execute_vectorized_query, get_query_plan, get_query_status,
    list_active_queries,
};

// Query WebSocket handlers
pub use query_websocket::{
    ws_adaptive_optimization, ws_cte_monitoring, ws_parallel_execution, ws_query_execution,
    ws_result_streaming,
};

// Index handlers
pub use index_handlers::{
    list_indexes, get_index_stats, rebuild_index, analyze_index,
    get_index_recommendations, get_index_advisor, coalesce_index,
};

// Memory handlers
pub use memory_handlers::{
    get_memory_status, get_allocator_stats, trigger_gc, get_memory_pressure,
    update_memory_config, list_allocators, get_allocator_stats_by_name,
    release_memory_pressure, list_memory_pools,
};

// Buffer Pool handlers
pub use buffer_pool_handlers::{
    get_buffer_pool_config, update_buffer_pool_config,
    flush_buffer_pool as flush_buffer_pool_handler, get_eviction_stats, get_prefetch_config,
    update_prefetch_config, get_hugepages_config, pin_page, unpin_page,
    prefetch_pages, get_hit_ratio,
};

// SIMD handlers
pub use simd_handlers::{
    get_cpu_features, get_simd_status, get_simd_capabilities, get_simd_stats,
    get_all_simd_metrics, get_simd_config, update_simd_config, get_filter_stats,
    get_aggregate_stats, get_scan_stats, get_hash_stats, get_string_stats,
    reset_simd_stats,
};

// Index & Memory WebSocket handlers
pub use index_memory_websocket_handlers::{
    ws_index_events_stream, ws_memory_events_stream, ws_buffer_pool_events_stream,
    ws_simd_metrics_stream, ws_inmemory_events_stream,
};

// Transaction WebSocket handlers
pub use transaction_websocket_handlers::{
    ws_transaction_lifecycle, ws_lock_events, ws_deadlock_events,
    ws_mvcc_events, ws_wal_events
};
