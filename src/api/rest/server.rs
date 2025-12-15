// # REST API Server Implementation
//
// Server setup, routing, and core functionality for the REST API.
// Uses dependency injection and proper error handling.

use super::cors::build_cors_layer;
use super::handlers::admin::{
    create_backup, create_role, create_user, delete_role, delete_user, get_config, get_health,
    get_role, get_roles, get_user, get_users, run_maintenance, update_config, update_role,
    update_user,
};
use super::handlers::auth::{login, logout, refresh, validate};
use super::handlers::cluster::{
    add_cluster_node, get_cluster_config, get_cluster_node, get_cluster_nodes,
    get_cluster_topology, get_replication_status, migrate_node, remove_cluster_node,
    trigger_failover, update_cluster_config,
};
use super::handlers::db::{
    begin_transaction, commit_transaction, create_table, delete_table, execute_batch,
    execute_query, get_schema, get_table, rollback_transaction, update_table,
};
use super::handlers::transaction_handlers::{
    get_active_transactions, get_transaction, create_savepoint, release_savepoint,
    rollback_to_savepoint, update_isolation_level, get_locks, get_lock_waiters,
    get_lock_graph, release_lock, release_all_locks, get_deadlocks, detect_deadlocks,
    get_mvcc_status, get_mvcc_snapshots, get_row_versions, trigger_vacuum, trigger_full_vacuum,
    get_wal_status, get_wal_segments, force_checkpoint, archive_wal, get_wal_replay_status,
    switch_wal_segment,
};
use super::handlers::transaction_websocket_handlers::{
    ws_transaction_lifecycle, ws_lock_events, ws_deadlock_events, ws_mvcc_events, ws_wal_events,
};
use super::handlers::optimizer_handlers::{
    list_hints, get_active_hints, apply_hints, remove_hint, get_hint_recommendations,
    list_baselines, create_baseline, get_baseline, update_baseline, delete_baseline,
    evolve_baseline, load_baselines, explain_query, explain_analyze_query,
    explain_query_with_visualization, get_adaptive_status, enable_adaptive_execution,
    get_adaptive_statistics, get_parallel_config, update_parallel_config, get_parallel_statistics,
};
use super::handlers::query_operations::{
    execute_query_with_monitoring, cancel_query, get_query_status, get_query_plan,
    execute_parallel_query, execute_cte_query, execute_adaptive_query, execute_vectorized_query,
    list_active_queries,
};
use super::handlers::query_websocket::{
    ws_query_execution, ws_result_streaming, ws_cte_monitoring, ws_parallel_execution,
    ws_adaptive_optimization,
};
use super::handlers::monitoring::{
    acknowledge_alert, get_alerts, get_logs, get_metrics, get_performance_data,
    get_prometheus_metrics, get_query_stats, get_session_stats,
};
use super::handlers::pool::{
    drain_pool, get_connection, get_connections, get_pool, get_pool_stats, get_pools, get_session,
    get_sessions, kill_connection, terminate_session, update_pool,
};
use super::handlers::system::{
    get_clustering_status, get_replication_status_info, get_security_features, get_server_config,
    get_server_info,
};
use super::handlers::{CATALOG, SQL_PARSER, TXN_MANAGER};
use super::middleware::{auth_middleware, rate_limit_middleware, request_logger_middleware};
use super::types::{ApiMetrics, ApiState, QueryRequest, RateLimiter};
use crate::api::graphql::{
    AuthorizationContext, GraphQLEngine, MutationRoot, QueryRoot, SubscriptionRoot,
};
use crate::api::ApiConfig;
use crate::error::DbError;
use crate::execution::Executor;
use crate::networking::{
    create_api_router, create_default_manager, NetworkConfig, NodeAddress, NodeId, NodeInfo,
};
use async_graphql::{http::GraphQLPlaygroundConfig, Schema};
use async_graphql_axum::{GraphQLRequest, GraphQLResponse};
use axum::{
    extract::{ws::WebSocket, State, WebSocketUpgrade},
    middleware,
    response::{Html, IntoResponse, Response},
    routing::{delete, get, post, put},
    Router,
};
use futures::SinkExt;
use serde_json::json;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{RwLock, Semaphore};
use tower_http::{limit::RequestBodyLimitLayer, timeout::TimeoutLayer, trace::TraceLayer};

// Enterprise Integration Handlers
use super::handlers::advanced_replication_handlers;
use super::handlers::audit_handlers;
use super::handlers::backup_handlers;
use super::handlers::enterprise_auth_handlers;
use super::handlers::rac_handlers;
use super::handlers::replication_handlers;

// Security Handlers
use super::handlers::encryption_handlers;
use super::handlers::labels_handlers;
use super::handlers::masking_handlers;
use super::handlers::privileges_handlers;
use super::handlers::security_handlers;
use super::handlers::vpd_handlers;

// Advanced Business Logic Handlers
use super::handlers::analytics_handlers;
use super::handlers::document_handlers;
use super::handlers::graph_handlers;
use super::handlers::inmemory_handlers;
use super::handlers::storage_handlers;
use super::handlers::storage_websocket_handlers;
use super::handlers::ml_handlers;
use super::handlers::spatial_handlers;

// Enterprise Features Handlers
use super::handlers::autonomous_handlers;
use super::handlers::blockchain_handlers;
use super::handlers::event_processing_handlers;
use super::handlers::flashback_handlers;
use super::handlers::multitenant_handlers;
use super::handlers::streams_handlers;

// WebSocket Handlers
use super::handlers::websocket_handlers;
use super::handlers::ml_websocket_handlers;
use super::handlers::analytics_websocket_handlers;

// Health Probe Handlers
use super::handlers::health_handlers;

// Diagnostics Handlers
use super::handlers::diagnostics_handlers;

// Dashboard Handlers
use super::handlers::dashboard_handlers;

// Type alias for the GraphQL schema
type GraphQLSchema = Schema<QueryRoot, MutationRoot, SubscriptionRoot>;

// REST API server with dependency injection
pub struct RestApiServer {
    config: ApiConfig,
    state: Arc<ApiState>,
    graphql_schema: GraphQLSchema,
}

impl RestApiServer {
    // Create a new REST API server with injected dependencies
    pub async fn new(config: ApiConfig) -> Result<Self, DbError> {
        // Create NetworkManager for networking API
        let network_config = NetworkConfig::default();
        let local_node = NodeInfo::new(
            NodeId::new("rest-api-server"),
            NodeAddress::new("localhost", config.port),
        );
        let network_manager = Arc::new(create_default_manager(network_config, local_node));

        let state = Arc::new(ApiState {
            config: config.clone(),
            connection_semaphore: Arc::new(Semaphore::new(config.max_connections)),
            active_queries: Arc::new(RwLock::new(HashMap::new())),
            active_sessions: Arc::new(RwLock::new(HashMap::new())),
            metrics: Arc::new(RwLock::new(ApiMetrics::default())),
            rate_limiter: Arc::new(RwLock::new(RateLimiter::new(config.rate_limit_rps, 1))),
            network_manager: Some(network_manager),
        });

        // Build GraphQL schema with engine and authorization context
        let graphql_engine = Arc::new(GraphQLEngine::new());

        // Create admin authorization context
        let auth_context = Arc::new(AuthorizationContext::new(
            "admin".to_string(),
            vec!["admin".to_string()],
            vec!["admin.*".to_string()],
        ));

        let graphql_schema = Schema::build(QueryRoot, MutationRoot, SubscriptionRoot)
            .data(graphql_engine)
            .data(auth_context)
            .finish();

        Ok(Self {
            config,
            state,
            graphql_schema,
        })
    }

    // Build the router with all endpoints and middleware
    fn build_router(&self) -> Router {
        // Public auth routes (no authentication required)
        let auth_routes = Router::new()
            .route("/api/v1/auth/login", post(login))
            .route("/api/v1/auth/logout", post(logout))
            .route("/api/v1/auth/refresh", post(refresh))
            .route("/api/v1/auth/validate", get(validate))
            .with_state(self.state.clone());

        // Protected admin routes (require authentication)
        let protected_admin_routes = Router::new()
            .route("/api/v1/admin/config", get(get_config))
            .route("/api/v1/admin/config", put(update_config))
            .route("/api/v1/admin/backup", post(create_backup))
            .route("/api/v1/admin/maintenance", post(run_maintenance))
            .route("/api/v1/admin/users", get(get_users))
            .route("/api/v1/admin/users", post(create_user))
            .route("/api/v1/admin/users/{id}", get(get_user))
            .route("/api/v1/admin/users/{id}", put(update_user))
            .route("/api/v1/admin/users/{id}", delete(delete_user))
            .route("/api/v1/admin/roles", get(get_roles))
            .route("/api/v1/admin/roles", post(create_role))
            .route("/api/v1/admin/roles/{id}", get(get_role))
            .route("/api/v1/admin/roles/{id}", put(update_role))
            .route("/api/v1/admin/roles/{id}", delete(delete_role))
            .route_layer(middleware::from_fn_with_state(
                self.state.clone(),
                auth_middleware,
            ))
            .with_state(self.state.clone());

        // Protected cluster routes (require authentication)
        let protected_cluster_routes = Router::new()
            .route("/api/v1/cluster/nodes", get(get_cluster_nodes))
            .route("/api/v1/cluster/nodes", post(add_cluster_node))
            .route("/api/v1/cluster/nodes/{id}", get(get_cluster_node))
            .route("/api/v1/cluster/nodes/{id}", delete(remove_cluster_node))
            .route("/api/v1/cluster/topology", get(get_cluster_topology))
            .route("/api/v1/cluster/failover", post(trigger_failover))
            .route("/api/v1/cluster/migrate", post(migrate_node))
            .route("/api/v1/cluster/replication", get(get_replication_status))
            .route("/api/v1/cluster/config", get(get_cluster_config))
            .route("/api/v1/cluster/config", put(update_cluster_config))
            .route_layer(middleware::from_fn_with_state(
                self.state.clone(),
                auth_middleware,
            ))
            .with_state(self.state.clone());

        // Create GraphQL router with its own state
        let graphql_schema = self.graphql_schema.clone();
        let graphql_router = Router::new()
            .route("/graphql", post(graphql_handler).get(graphql_playground))
            .route("/graphql/ws", get(graphql_subscription))
            .with_state(graphql_schema);

        // Create networking router if NetworkManager is available
        let networking_router = self
            .state
            .network_manager
            .as_ref()
            .map(|nm| create_api_router(nm.clone()));

        let mut router = Router::new()
            // Merge GraphQL router
            .merge(graphql_router)
            // Merge auth routes
            .merge(auth_routes)
            // Merge protected routes
            .merge(protected_admin_routes)
            .merge(protected_cluster_routes)
            // Core Database Operations API
            .route("/api/v1/query", post(execute_query))
            .route("/api/v1/batch", post(execute_batch))
            .route("/api/v1/tables/{name}", get(get_table))
            .route("/api/v1/tables/{name}", post(create_table))
            .route("/api/v1/tables/{name}", put(update_table))
            .route("/api/v1/tables/{name}", delete(delete_table))
            .route("/api/v1/schema", get(get_schema))
            .route("/api/v1/transactions", post(begin_transaction))
            .route("/api/v1/transactions/{id}/commit", post(commit_transaction))
            .route(
                "/api/v1/transactions/{id}/rollback",
                post(rollback_transaction),
            )
            // Transaction Management API - Extended
            .route("/api/v1/transactions/active", get(get_active_transactions))
            .route("/api/v1/transactions/{id}", get(get_transaction))
            // Savepoint Operations
            .route("/api/v1/transactions/{id}/savepoint", post(create_savepoint))
            .route("/api/v1/transactions/{id}/release-savepoint", post(release_savepoint))
            .route("/api/v1/transactions/{id}/rollback-to-savepoint", post(rollback_to_savepoint))
            // Isolation Level Control
            .route("/api/v1/transactions/{id}/isolation-level", put(update_isolation_level))
            // Lock Management
            .route("/api/v1/transactions/locks", get(get_locks))
            .route("/api/v1/transactions/locks/waiters", get(get_lock_waiters))
            .route("/api/v1/transactions/locks/graph", get(get_lock_graph))
            .route("/api/v1/transactions/locks/{id}/release", post(release_lock))
            .route("/api/v1/transactions/locks/release-all", post(release_all_locks))
            // Deadlock Detection
            .route("/api/v1/transactions/deadlocks", get(get_deadlocks))
            .route("/api/v1/transactions/deadlocks/detect", post(detect_deadlocks))
            // MVCC Operations
            .route("/api/v1/transactions/mvcc/status", get(get_mvcc_status))
            .route("/api/v1/transactions/mvcc/snapshots", get(get_mvcc_snapshots))
            .route("/api/v1/transactions/mvcc/versions/{table}/{row}", get(get_row_versions))
            .route("/api/v1/transactions/mvcc/vacuum", post(trigger_vacuum))
            .route("/api/v1/transactions/mvcc/vacuum/full", post(trigger_full_vacuum))
            // WAL Operations
            .route("/api/v1/transactions/wal/status", get(get_wal_status))
            .route("/api/v1/transactions/wal/segments", get(get_wal_segments))
            .route("/api/v1/transactions/wal/checkpoint", post(force_checkpoint))
            .route("/api/v1/transactions/wal/archive", post(archive_wal))
            .route("/api/v1/transactions/wal/replay-status", get(get_wal_replay_status))
            .route("/api/v1/transactions/wal/switch", post(switch_wal_segment))
.route("/api/v1/stream", get(websocket_stream))
            // WebSocket API - Real-time Streaming
            .route("/api/v1/ws", get(websocket_handlers::ws_upgrade_handler))
            .route("/api/v1/ws/query", get(websocket_handlers::ws_query_stream))
            .route(
                "/api/v1/ws/metrics",
                get(websocket_handlers::ws_metrics_stream),
            )
            .route(
                "/api/v1/ws/events",
                get(websocket_handlers::ws_events_stream),
            )
            .route(
                "/api/v1/ws/replication",
                get(websocket_handlers::ws_replication_stream),
            )
            // Transaction WebSocket Streams
            .route("/api/v1/ws/transactions/lifecycle", get(ws_transaction_lifecycle))
            .route("/api/v1/ws/transactions/locks", get(ws_lock_events))
            .route("/api/v1/ws/transactions/deadlocks", get(ws_deadlock_events))
            .route("/api/v1/ws/transactions/mvcc", get(ws_mvcc_events))
            .route("/api/v1/ws/transactions/wal", get(ws_wal_events))

            .route(
                "/api/v1/ws/dashboard",
                get(dashboard_handlers::ws_dashboard_stream),
            )
            // WebSocket API - ML Real-time Streams
            .route(
                "/api/v1/ws/ml/training",
                get(ml_websocket_handlers::ws_ml_training_progress),
            )
            .route(
                "/api/v1/ws/ml/predictions",
                get(ml_websocket_handlers::ws_ml_predictions),
            )
            .route(
                "/api/v1/ws/ml/automl",
                get(ml_websocket_handlers::ws_ml_automl_progress),
            )
            .route(
                "/api/v1/ws/ml/lifecycle",
                get(ml_websocket_handlers::ws_ml_lifecycle_events),
            )
            // WebSocket API - Analytics Real-time Streams
            .route(
                "/api/v1/ws/analytics/olap",
                get(analytics_websocket_handlers::ws_analytics_olap),
            )
            .route(
                "/api/v1/ws/analytics/timeseries",
                get(analytics_websocket_handlers::ws_analytics_timeseries),
            )
            .route(
                "/api/v1/ws/analytics/profiling",
                get(analytics_websocket_handlers::ws_analytics_profiling),
            )
            .route(
                "/api/v1/ws/analytics/workload",
                get(analytics_websocket_handlers::ws_analytics_workload),
            )
            .route(
                "/api/v1/ws/analytics/cache",
                get(analytics_websocket_handlers::ws_analytics_cache_events),
            )
            // Health endpoint (public, no auth required)
            .route("/api/v1/admin/health", get(get_health))
            // Health Probe API (Kubernetes-compatible, public)
            .route(
                "/api/v1/health/liveness",
                get(health_handlers::liveness_probe),
            )
            .route(
                "/api/v1/health/readiness",
                get(health_handlers::readiness_probe),
            )
            .route(
                "/api/v1/health/startup",
                get(health_handlers::startup_probe),
            )
            .route(
                "/api/v1/health/full",
                get(health_handlers::full_health_check),
            )
            // Monitoring & Metrics API
            .route("/api/v1/metrics", get(get_metrics))
            .route("/api/v1/metrics/prometheus", get(get_prometheus_metrics))
            .route("/api/v1/stats/sessions", get(get_session_stats))
            .route("/api/v1/stats/queries", get(get_query_stats))
            .route("/api/v1/stats/performance", get(get_performance_data))
            .route("/api/v1/logs", get(get_logs))
            .route("/api/v1/alerts", get(get_alerts))
            .route("/api/v1/alerts/{id}/acknowledge", post(acknowledge_alert))
            // Diagnostics & Profiling API
            .route(
                "/api/v1/diagnostics/incidents",
                get(diagnostics_handlers::get_incidents),
            )
            .route(
                "/api/v1/diagnostics/dump",
                post(diagnostics_handlers::create_dump),
            )
            .route(
                "/api/v1/diagnostics/dump/{id}",
                get(diagnostics_handlers::get_dump_status),
            )
            .route(
                "/api/v1/diagnostics/dump/{id}/download",
                get(diagnostics_handlers::download_dump),
            )
            .route(
                "/api/v1/profiling/queries",
                get(diagnostics_handlers::get_query_profiling),
            )
            .route(
                "/api/v1/monitoring/ash",
                get(diagnostics_handlers::get_active_session_history),
            )
            // Pool & Connection Management API
            .route("/api/v1/pools", get(get_pools))
            .route("/api/v1/pools/{id}", get(get_pool))
            .route("/api/v1/pools/{id}", put(update_pool))
            .route("/api/v1/pools/{id}/stats", get(get_pool_stats))
            .route("/api/v1/pools/{id}/drain", post(drain_pool))
            .route("/api/v1/connections", get(get_connections))
            .route("/api/v1/connections/{id}", get(get_connection))
            .route("/api/v1/connections/{id}", delete(kill_connection))
            .route("/api/v1/sessions", get(get_sessions))
            .route("/api/v1/sessions/{id}", get(get_session))
            .route("/api/v1/sessions/{id}", delete(terminate_session))
            // System Information API
            .route("/api/v1/config", get(get_server_config))
            .route("/api/v1/clustering/status", get(get_clustering_status))
            .route(
                "/api/v1/replication/status",
                get(get_replication_status_info),
            )
            .route("/api/v1/security/features", get(get_security_features))
            .route("/api/v1/server/info", get(get_server_info))
            // Enterprise Authentication API
            .route(
                "/api/v1/auth/ldap/configure",
                post(enterprise_auth_handlers::configure_ldap),
            )
            .route(
                "/api/v1/auth/ldap/config",
                get(enterprise_auth_handlers::get_ldap_config),
            )
            .route(
                "/api/v1/auth/ldap/test",
                post(enterprise_auth_handlers::test_ldap_connection),
            )
            .route(
                "/api/v1/auth/oauth/configure",
                post(enterprise_auth_handlers::configure_oauth),
            )
            .route(
                "/api/v1/auth/oauth/providers",
                get(enterprise_auth_handlers::get_oauth_providers),
            )
            .route(
                "/api/v1/auth/sso/configure",
                post(enterprise_auth_handlers::configure_sso),
            )
            .route(
                "/api/v1/auth/sso/metadata",
                get(enterprise_auth_handlers::get_saml_metadata),
            )
            // Backup & Disaster Recovery API
            .route(
                "/api/v1/backup/full",
                post(backup_handlers::create_full_backup),
            )
            .route(
                "/api/v1/backup/incremental",
                post(backup_handlers::create_incremental_backup),
            )
            .route("/api/v1/backup/list", get(backup_handlers::list_backups))
            .route("/api/v1/backup/{id}", get(backup_handlers::get_backup))
            .route(
                "/api/v1/backup/{id}",
                delete(backup_handlers::delete_backup),
            )
            .route(
                "/api/v1/backup/{id}/restore",
                post(backup_handlers::restore_backup),
            )
            .route(
                "/api/v1/backup/schedule",
                get(backup_handlers::get_backup_schedule),
            )
            .route(
                "/api/v1/backup/schedule",
                put(backup_handlers::update_backup_schedule),
            )
            // Replication Management API
            .route(
                "/api/v1/replication/configure",
                post(replication_handlers::configure_replication),
            )
            .route(
                "/api/v1/replication/config",
                get(replication_handlers::get_replication_config),
            )
            .route(
                "/api/v1/replication/slots",
                get(replication_handlers::list_replication_slots),
            )
            .route(
                "/api/v1/replication/slots",
                post(replication_handlers::create_replication_slot),
            )
            .route(
                "/api/v1/replication/slots/{name}",
                get(replication_handlers::get_replication_slot),
            )
            .route(
                "/api/v1/replication/slots/{name}",
                delete(replication_handlers::delete_replication_slot),
            )
            .route(
                "/api/v1/replication/conflicts",
                get(replication_handlers::get_replication_conflicts),
            )
            .route(
                "/api/v1/replication/resolve-conflict",
                post(replication_handlers::resolve_replication_conflict),
            )
            .route(
                "/api/v1/replication/conflicts/simulate",
                post(replication_handlers::simulate_replication_conflict),
            )
            // Replica Control API
            .route(
                "/api/v1/replication/replicas/{id}/pause",
                post(replication_handlers::pause_replica),
            )
            .route(
                "/api/v1/replication/replicas/{id}/resume",
                post(replication_handlers::resume_replica),
            )
            .route(
                "/api/v1/replication/lag",
                get(replication_handlers::get_replication_lag),
            )
            // Advanced Replication API - Groups
            .route(
                "/api/v1/replication/groups",
                get(advanced_replication_handlers::list_replication_groups),
            )
            .route(
                "/api/v1/replication/groups",
                post(advanced_replication_handlers::create_replication_group),
            )
            .route(
                "/api/v1/replication/groups/{id}",
                get(advanced_replication_handlers::get_replication_group),
            )
            .route(
                "/api/v1/replication/groups/{id}",
                delete(advanced_replication_handlers::delete_replication_group),
            )
            // Advanced Replication API - Publications
            .route(
                "/api/v1/replication/publications",
                get(advanced_replication_handlers::list_publications),
            )
            .route(
                "/api/v1/replication/publications",
                post(advanced_replication_handlers::create_publication),
            )
            .route(
                "/api/v1/replication/publications/{id}",
                get(advanced_replication_handlers::get_publication),
            )
            .route(
                "/api/v1/replication/publications/{id}",
                delete(advanced_replication_handlers::delete_publication),
            )
            // Advanced Replication API - Subscriptions
            .route(
                "/api/v1/replication/subscriptions",
                get(advanced_replication_handlers::list_subscriptions),
            )
            .route(
                "/api/v1/replication/subscriptions",
                post(advanced_replication_handlers::create_subscription),
            )
            .route(
                "/api/v1/replication/subscriptions/{id}",
                get(advanced_replication_handlers::get_subscription),
            )
            .route(
                "/api/v1/replication/subscriptions/{id}",
                delete(advanced_replication_handlers::delete_subscription),
            )
            // Sharding API
            .route(
                "/api/v1/sharding/tables",
                post(advanced_replication_handlers::create_sharded_table),
            )
            .route(
                "/api/v1/sharding/rebalance",
                post(advanced_replication_handlers::trigger_shard_rebalance),
            )
            .route(
                "/api/v1/sharding/stats",
                get(advanced_replication_handlers::get_sharding_stats),
            )
            // Global Data Services (GDS) API
            .route(
                "/api/v1/gds/services",
                post(advanced_replication_handlers::register_global_service),
            )
            .route(
                "/api/v1/gds/services",
                get(advanced_replication_handlers::list_global_services),
            )
            // XA Transactions API
            .route(
                "/api/v1/xa/start",
                post(advanced_replication_handlers::start_xa_transaction),
            )
            .route(
                "/api/v1/xa/prepare",
                post(advanced_replication_handlers::prepare_xa_transaction),
            )
            .route(
                "/api/v1/xa/commit",
                post(advanced_replication_handlers::commit_xa_transaction),
            )
            // RAC (Real Application Clusters) API
            .route(
                "/api/v1/rac/cluster/status",
                get(rac_handlers::get_cluster_status),
            )
            .route(
                "/api/v1/rac/cluster/nodes",
                get(rac_handlers::get_cluster_nodes),
            )
            .route(
                "/api/v1/rac/cluster/stats",
                get(rac_handlers::get_cluster_stats),
            )
            .route(
                "/api/v1/rac/cluster/rebalance",
                post(rac_handlers::trigger_cluster_rebalance),
            )
            // RAC Cache Fusion API
            .route(
                "/api/v1/rac/cache-fusion/status",
                get(rac_handlers::get_cache_fusion_status),
            )
            .route(
                "/api/v1/rac/cache-fusion/stats",
                get(rac_handlers::get_cache_fusion_stats),
            )
            .route(
                "/api/v1/rac/cache-fusion/transfers",
                get(rac_handlers::get_cache_fusion_transfers),
            )
            .route(
                "/api/v1/rac/cache-fusion/flush",
                post(rac_handlers::flush_cache_fusion),
            )
            // RAC GRD (Global Resource Directory) API
            .route(
                "/api/v1/rac/grd/topology",
                get(rac_handlers::get_grd_topology),
            )
            .route(
                "/api/v1/rac/grd/resources",
                get(rac_handlers::get_grd_resources),
            )
            .route(
                "/api/v1/rac/grd/remaster",
                post(rac_handlers::trigger_grd_remaster),
            )
            // RAC Interconnect API
            .route(
                "/api/v1/rac/interconnect/status",
                get(rac_handlers::get_interconnect_status),
            )
            .route(
                "/api/v1/rac/interconnect/stats",
                get(rac_handlers::get_interconnect_stats),
            )
            // RAC Parallel Query API
            .route(
                "/api/v1/rac/parallel-query",
                post(rac_handlers::execute_parallel_query),
            )
            // RAC Recovery API
            .route(
                "/api/v1/rac/recovery/status/{node_id}",
                get(rac_handlers::get_recovery_status),
            )
            .route(
                "/api/v1/rac/recovery/initiate",
                post(rac_handlers::initiate_recovery),
            )
            // Audit Logging API
            .route(
                "/api/v1/security/audit/logs",
                get(audit_handlers::query_audit_logs),
            )
            .route(
                "/api/v1/security/audit/export",
                post(audit_handlers::export_audit_logs),
            )
            .route(
                "/api/v1/security/audit/compliance",
                get(audit_handlers::compliance_report),
            )
            .route(
                "/api/v1/security/audit/stats",
                get(audit_handlers::get_audit_stats),
            )
            .route(
                "/api/v1/security/audit/verify",
                post(audit_handlers::verify_audit_integrity),
            )
            // RBAC (Role-Based Access Control) API
            .route("/api/v1/security/roles", get(security_handlers::list_roles))
            .route(
                "/api/v1/security/roles",
                post(security_handlers::create_role),
            )
            .route(
                "/api/v1/security/roles/{id}",
                get(security_handlers::get_role),
            )
            .route(
                "/api/v1/security/roles/{id}",
                put(security_handlers::update_role),
            )
            .route(
                "/api/v1/security/roles/{id}",
                delete(security_handlers::delete_role),
            )
            .route(
                "/api/v1/security/permissions",
                get(security_handlers::list_permissions),
            )
            .route(
                "/api/v1/security/roles/{id}/permissions",
                post(security_handlers::assign_permissions),
            )
            // Threat Detection API
            .route(
                "/api/v1/security/threats",
                get(security_handlers::get_threat_status),
            )
            .route(
                "/api/v1/security/threats/history",
                get(security_handlers::get_threat_history),
            )
            .route(
                "/api/v1/security/insider-threats",
                get(security_handlers::get_insider_threat_status),
            )
            // Encryption Management API
            .route(
                "/api/v1/security/encryption/status",
                get(encryption_handlers::get_encryption_status),
            )
            .route(
                "/api/v1/security/encryption/enable",
                post(encryption_handlers::enable_encryption),
            )
            .route(
                "/api/v1/security/encryption/column",
                post(encryption_handlers::enable_column_encryption),
            )
            .route("/api/v1/security/keys", get(encryption_handlers::list_keys))
            .route(
                "/api/v1/security/keys/generate",
                post(encryption_handlers::generate_key),
            )
            .route(
                "/api/v1/security/keys/{id}/rotate",
                post(encryption_handlers::rotate_key),
            )
            // Data Masking API
            .route(
                "/api/v1/security/masking/policies",
                get(masking_handlers::list_masking_policies),
            )
            .route(
                "/api/v1/security/masking/policies",
                post(masking_handlers::create_masking_policy),
            )
            .route(
                "/api/v1/security/masking/policies/{name}",
                get(masking_handlers::get_masking_policy),
            )
            .route(
                "/api/v1/security/masking/policies/{name}",
                put(masking_handlers::update_masking_policy),
            )
            .route(
                "/api/v1/security/masking/policies/{name}",
                delete(masking_handlers::delete_masking_policy),
            )
            .route(
                "/api/v1/security/masking/policies/{name}/enable",
                post(masking_handlers::enable_masking_policy),
            )
            .route(
                "/api/v1/security/masking/policies/{name}/disable",
                post(masking_handlers::disable_masking_policy),
            )
            .route(
                "/api/v1/security/masking/test",
                post(masking_handlers::test_masking),
            )
            // Virtual Private Database (VPD) API
            .route(
                "/api/v1/security/vpd/policies",
                get(vpd_handlers::list_vpd_policies),
            )
            .route(
                "/api/v1/security/vpd/policies",
                post(vpd_handlers::create_vpd_policy),
            )
            .route(
                "/api/v1/security/vpd/policies/{name}",
                get(vpd_handlers::get_vpd_policy),
            )
            .route(
                "/api/v1/security/vpd/policies/{name}",
                put(vpd_handlers::update_vpd_policy),
            )
            .route(
                "/api/v1/security/vpd/policies/{name}",
                delete(vpd_handlers::delete_vpd_policy),
            )
            .route(
                "/api/v1/security/vpd/policies/{name}/enable",
                post(vpd_handlers::enable_vpd_policy),
            )
            .route(
                "/api/v1/security/vpd/policies/{name}/disable",
                post(vpd_handlers::disable_vpd_policy),
            )
            .route(
                "/api/v1/security/vpd/test-predicate",
                post(vpd_handlers::test_vpd_predicate),
            )
            .route(
                "/api/v1/security/vpd/policies/table/{table_name}",
                get(vpd_handlers::get_table_policies),
            )
            // Privilege Management API
            .route(
                "/api/v1/security/privileges/grant",
                post(privileges_handlers::grant_privilege),
            )
            .route(
                "/api/v1/security/privileges/revoke",
                post(privileges_handlers::revoke_privilege),
            )
            .route(
                "/api/v1/security/privileges/user/{user_id}",
                get(privileges_handlers::get_user_privileges),
            )
            .route(
                "/api/v1/security/privileges/analyze/{user_id}",
                get(privileges_handlers::analyze_user_privileges),
            )
            .route(
                "/api/v1/security/privileges/role/{role_name}",
                get(privileges_handlers::get_role_privileges),
            )
            .route(
                "/api/v1/security/privileges/object/{object_name}",
                get(privileges_handlers::get_object_privileges),
            )
            .route(
                "/api/v1/security/privileges/validate",
                post(privileges_handlers::validate_privilege),
            )
            // Security Labels & MAC API
            .route(
                "/api/v1/security/labels/compartments",
                get(labels_handlers::list_compartments),
            )
            .route(
                "/api/v1/security/labels/compartments",
                post(labels_handlers::create_compartment),
            )
            .route(
                "/api/v1/security/labels/compartments/{id}",
                get(labels_handlers::get_compartment),
            )
            .route(
                "/api/v1/security/labels/compartments/{id}",
                delete(labels_handlers::delete_compartment),
            )
            .route(
                "/api/v1/security/labels/clearances/{user_id}",
                get(labels_handlers::get_user_clearance),
            )
            .route(
                "/api/v1/security/labels/clearances",
                post(labels_handlers::set_user_clearance),
            )
            .route(
                "/api/v1/security/labels/check-dominance",
                post(labels_handlers::check_label_dominance),
            )
            .route(
                "/api/v1/security/labels/validate-access",
                post(labels_handlers::validate_label_access),
            )
            .route(
                "/api/v1/security/labels/classifications",
                get(labels_handlers::list_classifications),
            )
            // Machine Learning API
            .route("/api/v1/ml/models", get(ml_handlers::list_models))
            .route("/api/v1/ml/models", post(ml_handlers::create_model))
            .route("/api/v1/ml/models/{id}", get(ml_handlers::get_model))
            .route("/api/v1/ml/models/{id}", delete(ml_handlers::delete_model))
            .route(
                "/api/v1/ml/models/{id}/train",
                post(ml_handlers::train_model),
            )
            .route("/api/v1/ml/models/{id}/predict", post(ml_handlers::predict))
            .route(
                "/api/v1/ml/models/{id}/metrics",
                get(ml_handlers::get_model_metrics),
            )
            .route(
                "/api/v1/ml/models/{id}/evaluate",
                post(ml_handlers::evaluate_model),
            )
            .route(
                "/api/v1/ml/models/{id}/export",
                get(ml_handlers::export_model),
            )
            // Graph Database API
            .route(
                "/api/v1/graph/query",
                post(graph_handlers::execute_graph_query),
            )
            .route("/api/v1/graph/vertices", post(graph_handlers::add_vertex))
            .route(
                "/api/v1/graph/vertices/{id}",
                get(graph_handlers::get_vertex),
            )
            .route("/api/v1/graph/edges", post(graph_handlers::add_edge))
            .route("/api/v1/graph/pagerank", post(graph_handlers::run_pagerank))
            .route(
                "/api/v1/graph/shortest-path",
                post(graph_handlers::shortest_path),
            )
            .route(
                "/api/v1/graph/communities",
                post(graph_handlers::detect_communities),
            )
            .route("/api/v1/graph/stats", get(graph_handlers::get_graph_stats))
            // Document Store API
            .route(
                "/api/v1/documents/collections",
                get(document_handlers::list_collections),
            )
            .route(
                "/api/v1/documents/collections",
                post(document_handlers::create_collection),
            )
            .route(
                "/api/v1/documents/collections/{name}",
                get(document_handlers::get_collection),
            )
            .route(
                "/api/v1/documents/collections/{name}",
                delete(document_handlers::drop_collection),
            )
            .route(
                "/api/v1/documents/collections/{name}/documents",
                get(document_handlers::find_documents),
            )
            .route(
                "/api/v1/documents/collections/{name}/documents",
                post(document_handlers::insert_document),
            )
            .route(
                "/api/v1/documents/collections/{name}/documents/bulk",
                post(document_handlers::bulk_insert_documents),
            )
            .route(
                "/api/v1/documents/collections/{name}/documents/update",
                put(document_handlers::update_documents),
            )
            .route(
                "/api/v1/documents/collections/{name}/documents/delete",
                delete(document_handlers::delete_documents),
            )
            .route(
                "/api/v1/documents/collections/{name}/aggregate",
                post(document_handlers::aggregate_documents),
            )
            .route(
                "/api/v1/documents/collections/{name}/count",
                get(document_handlers::count_documents),
            )
            .route(
                "/api/v1/documents/collections/{name}/watch",
                get(document_handlers::watch_collection),
            )
            // Spatial Database API (15 endpoints - 100% coverage)
            .route(
                "/api/v1/spatial/query",
                post(spatial_handlers::spatial_query),
            )
            .route(
                "/api/v1/spatial/nearest",
                post(spatial_handlers::find_nearest),
            )
            .route(
                "/api/v1/spatial/route",
                post(spatial_handlers::calculate_route),
            )
            .route(
                "/api/v1/spatial/buffer",
                post(spatial_handlers::create_buffer),
            )
            .route(
                "/api/v1/spatial/transform",
                post(spatial_handlers::transform_geometry),
            )
            .route(
                "/api/v1/spatial/within",
                post(spatial_handlers::find_within),
            )
            .route(
                "/api/v1/spatial/intersects",
                post(spatial_handlers::check_intersects),
            )
            .route(
                "/api/v1/spatial/distance",
                get(spatial_handlers::calculate_distance),
            )
            .route(
                "/api/v1/spatial/create",
                post(spatial_handlers::create_spatial_table),
            )
            .route(
                "/api/v1/spatial/index",
                post(spatial_handlers::create_spatial_index),
            )
            .route("/api/v1/spatial/srid", get(spatial_handlers::list_srids))
            .route(
                "/api/v1/spatial/union",
                post(spatial_handlers::union_geometries),
            )
            .route(
                "/api/v1/spatial/intersection",
                post(spatial_handlers::intersection_geometries),
            )
            .route(
                "/api/v1/spatial/network/nodes",
                post(spatial_handlers::add_network_node),
            )
            .route(
                "/api/v1/spatial/network/edges",
                post(spatial_handlers::add_network_edge),
            )
            // Analytics API - OLAP Operations (14 endpoints - 100% coverage)
            .route(
                "/api/v1/analytics/olap/cubes",
                post(analytics_handlers::create_olap_cube),
            )
            .route(
                "/api/v1/analytics/olap/cubes",
                get(analytics_handlers::list_olap_cubes),
            )
            .route(
                "/api/v1/analytics/olap/cubes/{cube_id}/query",
                post(analytics_handlers::query_olap_cube),
            )
            .route(
                "/api/v1/analytics/olap/cubes/{cube_id}",
                delete(analytics_handlers::delete_olap_cube),
            )
            // Analytics API - Query Analytics
            .route(
                "/api/v1/analytics/query-stats",
                get(analytics_handlers::get_query_statistics),
            )
            .route(
                "/api/v1/analytics/workload",
                get(analytics_handlers::analyze_workload),
            )
            .route(
                "/api/v1/analytics/recommendations",
                get(analytics_handlers::get_recommendations),
            )
            // Analytics API - Data Quality
            .route(
                "/api/v1/analytics/profile/{table_name}",
                post(analytics_handlers::profile_table),
            )
            .route(
                "/api/v1/analytics/quality/{table_name}",
                get(analytics_handlers::get_quality_metrics),
            )
            .route(
                "/api/v1/analytics/quality/{table_name}/issues",
                get(analytics_handlers::get_quality_issues),
            )
            // Analytics API - Materialized Views
            .route(
                "/api/v1/analytics/materialized-views",
                post(analytics_handlers::create_materialized_view),
            )
            .route(
                "/api/v1/analytics/materialized-views",
                get(analytics_handlers::list_materialized_views),
            )
            .route(
                "/api/v1/analytics/materialized-views/{view_id}/refresh",
                post(analytics_handlers::refresh_materialized_view),
            )
            // In-Memory Column Store API (12 endpoints - 100% coverage)
            .route(
                "/api/v1/inmemory/enable",
                post(inmemory_handlers::enable_inmemory),
            )
            .route(
                "/api/v1/inmemory/disable",
                post(inmemory_handlers::disable_inmemory),
            )
            .route(
                "/api/v1/inmemory/status",
                get(inmemory_handlers::inmemory_status),
            )
            .route(
                "/api/v1/inmemory/stats",
                get(inmemory_handlers::inmemory_stats),
            )
            .route(
                "/api/v1/inmemory/populate",
                post(inmemory_handlers::populate_table),
            )
            .route(
                "/api/v1/inmemory/tables/{table}/status",
                get(inmemory_handlers::get_table_status),
            )
            .route(
                "/api/v1/inmemory/evict",
                post(inmemory_handlers::evict_tables),
            )
            .route(
                "/api/v1/inmemory/compact",
                post(inmemory_handlers::compact_memory),
            )
            .route(
                "/api/v1/inmemory/config",
                get(inmemory_handlers::get_inmemory_config),
            )
            .route(
                "/api/v1/inmemory/config",
                put(inmemory_handlers::update_inmemory_config),
            )
            // Index Management API (7 endpoints)
            .route("/api/v1/indexes", get(super::handlers::list_indexes))
            .route("/api/v1/indexes/{name}/stats", get(super::handlers::get_index_stats))
            .route("/api/v1/indexes/{name}/advisor", get(super::handlers::get_index_advisor))
            .route("/api/v1/indexes/{name}/rebuild", post(super::handlers::rebuild_index))
            .route("/api/v1/indexes/{name}/analyze", post(super::handlers::analyze_index))
            .route("/api/v1/indexes/{name}/coalesce", post(super::handlers::coalesce_index))
            .route("/api/v1/indexes/recommendations", get(super::handlers::get_index_recommendations))
            // Memory Management API (9 endpoints)
            .route("/api/v1/memory/status", get(super::handlers::get_memory_status))
            .route("/api/v1/memory/allocator/stats", get(super::handlers::get_allocator_stats))
            .route("/api/v1/memory/allocators", get(super::handlers::list_allocators))
            .route("/api/v1/memory/allocators/{name}/stats", get(super::handlers::get_allocator_stats_by_name))
            .route("/api/v1/memory/gc", post(super::handlers::trigger_gc))
            .route("/api/v1/memory/pressure", get(super::handlers::get_memory_pressure))
            .route("/api/v1/memory/pressure/release", post(super::handlers::release_memory_pressure))
            .route("/api/v1/memory/pools", get(super::handlers::list_memory_pools))
            .route("/api/v1/memory/config", put(super::handlers::update_memory_config))
            // Buffer Pool Management API (12 endpoints)
            .route("/api/v1/buffer/stats", get(super::handlers::get_buffer_pool_stats))
            .route("/api/v1/buffer/config", get(super::handlers::get_buffer_pool_config))
            .route("/api/v1/buffer/config", put(super::handlers::update_buffer_pool_config))
            .route("/api/v1/buffer/flush", post(super::handlers::flush_buffer_pool_handler))
            .route("/api/v1/buffer/eviction/stats", get(super::handlers::get_eviction_stats))
            .route("/api/v1/buffer/prefetch/config", get(super::handlers::get_prefetch_config))
            .route("/api/v1/buffer/prefetch/config", put(super::handlers::update_prefetch_config))
            .route("/api/v1/buffer/hugepages", get(super::handlers::get_hugepages_config))
            .route("/api/v1/buffer/pages/{page_id}/pin", post(super::handlers::pin_page))
            .route("/api/v1/buffer/pages/{page_id}/unpin", post(super::handlers::unpin_page))
            .route("/api/v1/buffer-pool/prefetch", post(super::handlers::prefetch_pages))
            .route("/api/v1/buffer-pool/hit-ratio", get(super::handlers::get_hit_ratio))
            // SIMD Operations API (13 endpoints)
            .route("/api/v1/simd/features", get(super::handlers::get_cpu_features))
            .route("/api/v1/simd/status", get(super::handlers::get_simd_status))
            .route("/api/v1/simd/capabilities", get(super::handlers::get_simd_capabilities))
            .route("/api/v1/simd/stats", get(super::handlers::get_simd_stats))
            .route("/api/v1/simd/metrics", get(super::handlers::get_all_simd_metrics))
            .route("/api/v1/simd/config", get(super::handlers::get_simd_config))
            .route("/api/v1/simd/config", put(super::handlers::update_simd_config))
            .route("/api/v1/simd/operations/filter/stats", get(super::handlers::get_filter_stats))
            .route("/api/v1/simd/operations/aggregate/stats", get(super::handlers::get_aggregate_stats))
            .route("/api/v1/simd/operations/scan/stats", get(super::handlers::get_scan_stats))
            .route("/api/v1/simd/operations/hash/stats", get(super::handlers::get_hash_stats))
            .route("/api/v1/simd/operations/string/stats", get(super::handlers::get_string_stats))
            .route("/api/v1/simd/stats/reset", post(super::handlers::reset_simd_stats))
            // Storage Layer API - Page Management (5 endpoints)
            .route("/api/v1/storage/pages", post(storage_handlers::create_page))
            .route("/api/v1/storage/pages", get(storage_handlers::list_pages))
            .route("/api/v1/storage/pages/{id}", get(storage_handlers::get_page))
            .route("/api/v1/storage/pages/{id}/compact", post(storage_handlers::compact_page))
            .route("/api/v1/storage/pages/{id}/flush", post(storage_handlers::flush_page))
            // Storage Layer API - LSM Tree (6 endpoints)
            .route("/api/v1/storage/lsm", post(storage_handlers::create_lsm_tree))
            .route("/api/v1/storage/lsm/put", put(storage_handlers::lsm_put))
            .route("/api/v1/storage/lsm/get/{key}", get(storage_handlers::lsm_get))
            .route("/api/v1/storage/lsm/delete/{key}", delete(storage_handlers::lsm_delete))
            .route("/api/v1/storage/lsm/compact", post(storage_handlers::lsm_compact))
            .route("/api/v1/storage/lsm/stats", get(storage_handlers::get_lsm_stats))
            // Storage Layer API - Columnar Storage (5 endpoints)
            .route("/api/v1/storage/columnar", post(storage_handlers::create_columnar_table))
            .route("/api/v1/storage/columnar/batch-insert", post(storage_handlers::columnar_batch_insert))
            .route("/api/v1/storage/columnar/scan", post(storage_handlers::columnar_scan))
            .route("/api/v1/storage/columnar/project", post(storage_handlers::columnar_project))
            .route("/api/v1/storage/columnar/{table_name}/stats", get(storage_handlers::get_columnar_stats))
            // Storage Layer API - Tiered Storage (3 endpoints)
            .route("/api/v1/storage/tiers", get(storage_handlers::get_tier_stats))
            .route("/api/v1/storage/tiers/{tier}", get(storage_handlers::get_tier_info))
            .route("/api/v1/storage/tiers/migrate", post(storage_handlers::migrate_tier))
            // Storage Layer API - JSON Storage (4 endpoints)
            .route("/api/v1/storage/json/extract", post(storage_handlers::json_extract))
            .route("/api/v1/storage/json/set", post(storage_handlers::json_set))
            .route("/api/v1/storage/json/delete", post(storage_handlers::json_delete))
            .route("/api/v1/storage/json/merge", post(storage_handlers::json_merge))
            // Storage Layer API - Vectored I/O (2 endpoints)
            .route("/api/v1/storage/io/vectored-read", post(storage_handlers::vectored_read))
            .route("/api/v1/storage/io/vectored-write", post(storage_handlers::vectored_write))
            // Storage Layer WebSocket API - Real-time Events (6 WebSocket streams)
            .route("/api/v1/ws/storage/buffer-pool", get(storage_websocket_handlers::ws_buffer_pool_events))
            .route("/api/v1/ws/storage/lsm", get(storage_websocket_handlers::ws_lsm_events))
            .route("/api/v1/ws/storage/disk-io", get(storage_websocket_handlers::ws_disk_io_events))
            .route("/api/v1/ws/storage/tiers", get(storage_websocket_handlers::ws_tier_events))
            .route("/api/v1/ws/storage/pages", get(storage_websocket_handlers::ws_page_events))
            .route("/api/v1/ws/storage/columnar", get(storage_websocket_handlers::ws_columnar_events))
            // Index & Memory WebSocket API (5 endpoints)
            .route("/api/v1/ws/index/events", get(super::handlers::ws_index_events_stream))
            .route("/api/v1/ws/memory/events", get(super::handlers::ws_memory_events_stream))
            .route("/api/v1/ws/buffer/events", get(super::handlers::ws_buffer_pool_events_stream))
            .route("/api/v1/ws/simd/metrics", get(super::handlers::ws_simd_metrics_stream))
            .route("/api/v1/ws/inmemory/events", get(super::handlers::ws_inmemory_events_stream))
            // Query Optimizer API
            // Optimizer Hints
            .route("/api/v1/optimizer/hints", get(list_hints))
            .route("/api/v1/optimizer/hints/active", get(get_active_hints))
            .route("/api/v1/optimizer/hints", post(apply_hints))
            .route("/api/v1/optimizer/hints/recommendations", post(get_hint_recommendations))
            .route("/api/v1/optimizer/hints/{id}", delete(remove_hint))
            // Plan Baselines
            .route("/api/v1/optimizer/baselines", get(list_baselines))
            .route("/api/v1/optimizer/baselines", post(create_baseline))
            .route("/api/v1/optimizer/baselines/load", post(load_baselines))
            .route("/api/v1/optimizer/baselines/{id}", get(get_baseline))
            .route("/api/v1/optimizer/baselines/{id}", put(update_baseline))
            .route("/api/v1/optimizer/baselines/{id}", delete(delete_baseline))
            .route("/api/v1/optimizer/baselines/{id}/evolve", post(evolve_baseline))
            // EXPLAIN endpoints
            .route("/api/v1/query/explain", post(explain_query))
            .route("/api/v1/query/explain/analyze", post(explain_analyze_query))
            .route("/api/v1/query/explain/visualize", post(explain_query_with_visualization))
            // Adaptive Execution
            .route("/api/v1/optimizer/adaptive/status", get(get_adaptive_status))
            .route("/api/v1/optimizer/adaptive/enable", post(enable_adaptive_execution))
            .route("/api/v1/optimizer/adaptive/statistics", get(get_adaptive_statistics))
            // Parallel Query Configuration
            .route("/api/v1/optimizer/parallel/config", get(get_parallel_config))
            .route("/api/v1/optimizer/parallel/config", put(update_parallel_config))
            .route("/api/v1/optimizer/parallel/statistics", get(get_parallel_statistics))
            // Query Execution API
            .route("/api/v1/query/execute", post(execute_query_with_monitoring))
            .route("/api/v1/query/{query_id}/cancel", post(cancel_query))
            .route("/api/v1/query/{query_id}/status", get(get_query_status))
            .route("/api/v1/query/plan", post(get_query_plan))
            .route("/api/v1/query/parallel", post(execute_parallel_query))
            .route("/api/v1/query/cte", post(execute_cte_query))
            .route("/api/v1/query/adaptive", post(execute_adaptive_query))
            .route("/api/v1/query/vectorized", post(execute_vectorized_query))
            .route("/api/v1/query/active", get(list_active_queries))
            // Query WebSocket endpoints
            .route("/api/v1/ws/query/execution", get(ws_query_execution))
            .route("/api/v1/ws/query/results", get(ws_result_streaming))
            .route("/api/v1/ws/query/cte", get(ws_cte_monitoring))
            .route("/api/v1/ws/query/parallel", get(ws_parallel_execution))
            .route("/api/v1/ws/query/adaptive", get(ws_adaptive_optimization))
            // Dashboard Management API
            .route("/api/v1/dashboards", post(dashboard_handlers::create_dashboard))
            .route("/api/v1/dashboards", get(dashboard_handlers::list_dashboards))
            .route("/api/v1/dashboards/{id}", get(dashboard_handlers::get_dashboard))
            .route("/api/v1/dashboards/{id}", put(dashboard_handlers::update_dashboard))
            .route("/api/v1/dashboards/{id}", delete(dashboard_handlers::delete_dashboard))
            // ============================================================================
            // ENTERPRISE FEATURES API - 100% Coverage
            // ============================================================================
            // Multi-Tenant Database API (14 endpoints - 100% coverage)
            .route("/api/v1/multitenant/tenants", post(multitenant_handlers::provision_tenant))
            .route("/api/v1/multitenant/tenants", get(multitenant_handlers::list_tenants))
            .route("/api/v1/multitenant/tenants/{tenant_id}", get(multitenant_handlers::get_tenant))
            .route("/api/v1/multitenant/tenants/{tenant_id}/suspend", post(multitenant_handlers::suspend_tenant))
            .route("/api/v1/multitenant/tenants/{tenant_id}/resume", post(multitenant_handlers::resume_tenant))
            .route("/api/v1/multitenant/tenants/{tenant_id}", delete(multitenant_handlers::delete_tenant))
            .route("/api/v1/multitenant/pdbs", post(multitenant_handlers::create_pdb))
            .route("/api/v1/multitenant/pdbs/{pdb_name}/open", post(multitenant_handlers::open_pdb))
            .route("/api/v1/multitenant/pdbs/{pdb_name}/close", post(multitenant_handlers::close_pdb))
            .route("/api/v1/multitenant/pdbs/{pdb_name}/clone", post(multitenant_handlers::clone_pdb))
            .route("/api/v1/multitenant/pdbs/{pdb_name}/relocate", post(multitenant_handlers::relocate_pdb))
            .route("/api/v1/multitenant/system/stats", get(multitenant_handlers::get_system_stats))
            .route("/api/v1/multitenant/metering/report", post(multitenant_handlers::get_metering_report))
            // Blockchain Tables API (13 endpoints - 100% coverage)
            .route("/api/v1/blockchain/tables", post(blockchain_handlers::create_blockchain_table))
            .route("/api/v1/blockchain/tables/{table_name}", get(blockchain_handlers::get_blockchain_table))
            .route("/api/v1/blockchain/tables/{table_name}/rows", post(blockchain_handlers::insert_blockchain_row))
            .route("/api/v1/blockchain/tables/{table_name}/finalize-block", post(blockchain_handlers::finalize_block))
            .route("/api/v1/blockchain/tables/{table_name}/verify", post(blockchain_handlers::verify_integrity))
            .route("/api/v1/blockchain/tables/{table_name}/blocks/{block_id}", get(blockchain_handlers::get_block_details))
            .route("/api/v1/blockchain/retention-policies", post(blockchain_handlers::create_retention_policy))
            .route("/api/v1/blockchain/tables/{table_name}/retention-policy", post(blockchain_handlers::assign_retention_policy))
            .route("/api/v1/blockchain/legal-holds", post(blockchain_handlers::create_legal_hold))
            .route("/api/v1/blockchain/legal-holds/{hold_id}/release", post(blockchain_handlers::release_legal_hold))
            .route("/api/v1/blockchain/tables/{table_name}/audit", get(blockchain_handlers::get_audit_events))
            .route("/api/v1/blockchain/tables/{table_name}/stats", get(blockchain_handlers::get_blockchain_stats))
            // Autonomous Database API (11 endpoints - 100% coverage)
            .route("/api/v1/autonomous/config", get(autonomous_handlers::get_autonomous_config))
            .route("/api/v1/autonomous/config", put(autonomous_handlers::update_autonomous_config))
            .route("/api/v1/autonomous/tuning/report", get(autonomous_handlers::get_tuning_report))
            .route("/api/v1/autonomous/healing/report", get(autonomous_handlers::get_healing_report))
            .route("/api/v1/autonomous/indexing/recommendations", get(autonomous_handlers::get_index_recommendations))
            .route("/api/v1/autonomous/indexing/apply", post(autonomous_handlers::apply_index_recommendation))
            .route("/api/v1/autonomous/workload/analysis", get(autonomous_handlers::get_workload_analysis))
            .route("/api/v1/autonomous/capacity/forecast", get(autonomous_handlers::get_capacity_forecast))
            .route("/api/v1/autonomous/status", get(autonomous_handlers::get_autonomous_status))
            .route("/api/v1/autonomous/tuning/run", post(autonomous_handlers::trigger_tuning_run))
            .route("/api/v1/autonomous/healing/run", post(autonomous_handlers::trigger_healing_run))
            // Complex Event Processing API (13 endpoints - 100% coverage)
            .route("/api/v1/event-processing/streams", post(event_processing_handlers::create_stream))
            .route("/api/v1/event-processing/streams", get(event_processing_handlers::list_streams))
            .route("/api/v1/event-processing/streams/{stream_name}", get(event_processing_handlers::get_stream))
            .route("/api/v1/event-processing/patterns", post(event_processing_handlers::create_cep_pattern))
            .route("/api/v1/event-processing/patterns/{pattern_id}/matches", get(event_processing_handlers::get_pattern_matches))
            .route("/api/v1/event-processing/continuous-queries", post(event_processing_handlers::create_continuous_query))
            .route("/api/v1/event-processing/continuous-queries/{query_id}", get(event_processing_handlers::get_continuous_query))
            .route("/api/v1/event-processing/windows", post(event_processing_handlers::create_window_operation))
            .route("/api/v1/event-processing/analytics", post(event_processing_handlers::get_event_analytics))
            .route("/api/v1/event-processing/streams/{stream_name}/metrics", get(event_processing_handlers::get_stream_metrics))
            .route("/api/v1/event-processing/connectors", post(event_processing_handlers::create_connector))
            .route("/api/v1/event-processing/connectors/{connector_id}", get(event_processing_handlers::get_connector))
            .route("/api/v1/event-processing/connectors/{connector_id}/stop", post(event_processing_handlers::stop_connector))
            // Flashback & Time-Travel API (10 endpoints - 100% coverage)
            .route("/api/v1/flashback/query", post(flashback_handlers::flashback_query))
            .route("/api/v1/flashback/table", post(flashback_handlers::flashback_table))
            .route("/api/v1/flashback/versions", post(flashback_handlers::query_versions))
            .route("/api/v1/flashback/restore-points", post(flashback_handlers::create_restore_point))
            .route("/api/v1/flashback/restore-points", get(flashback_handlers::list_restore_points))
            .route("/api/v1/flashback/restore-points/{name}", delete(flashback_handlers::delete_restore_point))
            .route("/api/v1/flashback/database", post(flashback_handlers::flashback_database))
            .route("/api/v1/flashback/stats", get(flashback_handlers::get_flashback_stats))
            .route("/api/v1/flashback/transaction", post(flashback_handlers::flashback_transaction))
            .route("/api/v1/flashback/current-scn", get(flashback_handlers::get_current_scn))
            // Streams & CDC API (11 endpoints - 100% coverage)
            .route("/api/v1/streams/publish", post(streams_handlers::publish_event))
            .route("/api/v1/streams/topics", post(streams_handlers::create_topic))
            .route("/api/v1/streams/topics", get(streams_handlers::list_topics))
            .route("/api/v1/streams/subscribe", post(streams_handlers::subscribe_topics))
            .route("/api/v1/cdc/start", post(streams_handlers::start_cdc))
            .route("/api/v1/cdc/changes", get(streams_handlers::get_changes))
            .route("/api/v1/cdc/{id}/stop", post(streams_handlers::stop_cdc))
            .route("/api/v1/cdc/{id}/stats", get(streams_handlers::get_cdc_stats))
            .route("/api/v1/streams/stream", get(streams_handlers::stream_events))
            .route("/api/v1/streams/topics/{topic}/offsets", get(streams_handlers::get_topic_offsets))
            .route("/api/v1/streams/consumer/{group_id}/commit", post(streams_handlers::commit_offsets))
            .with_state(self.state.clone());

        // Merge networking router if available
        if let Some(net_router) = networking_router {
            router = router.merge(net_router);
        }

        // Add middleware layers
        router = router
            .layer(TraceLayer::new_for_http())
            .layer(TimeoutLayer::with_status_code(
                http::StatusCode::REQUEST_TIMEOUT,
                Duration::from_secs(self.config.request_timeout_secs),
            ))
            .layer(RequestBodyLimitLayer::new(
                self.config.max_body_size as u64 as usize,
            ))
            .layer(middleware::from_fn_with_state(
                self.state.clone(),
                request_logger_middleware,
            ))
            .layer(middleware::from_fn_with_state(
                self.state.clone(),
                rate_limit_middleware,
            ));

        // Add CORS if enabled - uses secure origin validation
        // SECURITY: NEVER use allow_origin(Any) in production!
        // This configuration uses a trie-based origin matcher for efficient validation
        if self.config.enable_cors {
            router = router.layer(build_cors_layer(&self.config.cors_origins));
        }

        // Add Swagger UI if enabled
        if self.config.enable_swagger {
            router = router.merge(super::swagger::create_api_docs_router());
        }

        router
    }

    // Run the API server
    pub async fn run(&self, addr: &str) -> Result<(), DbError> {
        let router = self.build_router();

        let listener = tokio::net::TcpListener::bind(addr)
            .await
            .map_err(|e| DbError::Network(format!("Failed to bind to {}: {}", addr, e)))?;

        tracing::info!("REST API server listening on {}", addr);

        axum::serve(listener, router)
            .await
            .map_err(|e| DbError::Network(format!("Server error: {}", e)))?;

        Ok(())
    }
}

impl Default for RestApiServer {
    fn default() -> Self {
        // This would normally inject dependencies, but for default we create them
        futures::executor::block_on(async { Self::new(ApiConfig::default()).await.unwrap() })
    }
}

// WebSocket handler for streaming query results
async fn websocket_stream(ws: WebSocketUpgrade, State(state): State<Arc<ApiState>>) -> Response {
    ws.on_upgrade(|socket| handle_websocket(socket, state))
}

fn get_executor() -> Executor {
    let catalog_guard = CATALOG.read();
    let catalog_snapshot = (*catalog_guard).clone();
    Executor::new(Arc::new(catalog_snapshot), TXN_MANAGER.clone())
}

async fn handle_websocket(mut socket: WebSocket, _state: Arc<ApiState>) {
    use axum::extract::ws::Message;

    while let Some(msg) = socket.recv().await {
        if let Ok(msg) = msg {
            match msg {
                Message::Text(text) => {
                    if let Ok(request) = serde_json::from_str::<QueryRequest>(&text) {
                        let executor = get_executor();

                        let response = match SQL_PARSER.parse(&request.sql) {
                            Ok(stmts) => {
                                let stmt = match stmts.into_iter().next() {
                                    Some(s) => s,
                                    None => {
                                        let _ = socket.send(Message::Text(json!({"status": "error", "message": "No valid SQL statement"}).to_string().into())).await;
                                        continue;
                                    }
                                };
                                match executor.execute(stmt) {
                                    Ok(result) => {
                                        let rows: Vec<Vec<serde_json::Value>> = result
                                            .rows
                                            .iter()
                                            .map(|row| {
                                                row.iter()
                                                    .map(|val| {
                                                        serde_json::Value::String(val.clone())
                                                    })
                                                    .collect()
                                            })
                                            .collect();

                                        json!({
                                            "status": "success",
                                            "rows": rows,
                                            "columns": result.columns,
                                            "rows_affected": result.rows_affected
                                        })
                                    }
                                    Err(e) => json!({
                                        "status": "error",
                                        "message": e.to_string()
                                    }),
                                }
                            }
                            Err(e) => json!({
                                "status": "error",
                                "message": e.to_string()
                            }),
                        };

                        if socket
                            .send(Message::Text(response.to_string().into()))
                            .await
                            .is_err()
                        {
                            break;
                        }
                    }
                }
                Message::Close(_) => break,
                _ => {}
            }
        }
    }
}

// ============================================================================
// GRAPHQL HANDLERS
// ============================================================================

// GraphQL query/mutation handler
async fn graphql_handler(
    State(schema): State<GraphQLSchema>,
    req: GraphQLRequest,
) -> GraphQLResponse {
    schema.execute(req.into_inner()).await.into()
}

// GraphQL Playground UI
async fn graphql_playground() -> impl IntoResponse {
    Html(async_graphql::http::playground_source(
        GraphQLPlaygroundConfig::new("/graphql").subscription_endpoint("/graphql/ws"),
    ))
}

// GraphQL WebSocket subscriptions
async fn graphql_subscription(
    ws: WebSocketUpgrade,
    State(schema): State<GraphQLSchema>,
) -> impl IntoResponse {
    use axum::extract::ws::{Message, Utf8Bytes};
    use futures_util::StreamExt;

    ws.on_upgrade(move |socket| async move {
        let (mut sink, mut stream) = socket.split();

        while let Some(msg) = stream.next().await {
            if let Ok(msg) = msg {
                if let Message::Text(text) = msg {
                    if let Ok(request) = serde_json::from_str::<async_graphql::Request>(&text) {
                        let response = schema.execute(request).await;
                        let response_text = serde_json::to_string(&response).unwrap_or_default();
                        // Convert String to Utf8Bytes for axum 0.8
                        if let Ok(utf8_bytes) = Utf8Bytes::try_from(response_text.into_bytes()) {
                            let _ = sink.send(Message::Text(utf8_bytes)).await;
                        }
                    }
                }
            }
        }
    })
}

#[cfg(test)]
mod tests {
    use crate::api::rest_api::{PaginatedResponse, PaginationParams, RateLimiter};
    use crate::api::{ApiConfig, ApiError, RestApiServer};

    #[test]
    fn test_api_config_default() {
        let config = ApiConfig::default();
        assert_eq!(config.port, 8080);
        assert_eq!(config.enable_cors, true);
        assert_eq!(config.rate_limit_rps, 100);
    }

    #[test]
    fn test_api_error_creation() {
        let error = ApiError::new("TEST_ERROR", "Test message");
        assert_eq!(error.code, "TEST_ERROR");
        assert_eq!(error.message, "Test message");
        assert!(error.details.is_none());
    }

    #[test]
    fn test_pagination_params() {
        let params = PaginationParams {
            page: 2,
            page_size: 25,
            sort_by: None,
            sort_order: None,
        };
        assert_eq!(params.page, 2);
        assert_eq!(params.page_size, 25);
    }

    #[test]
    fn test_paginated_response() {
        let data = vec![1, 2, 3, 4, 5];
        let response = PaginatedResponse::new(data, 1, 5, 20);

        assert_eq!(response.page, 1);
        assert_eq!(response.page_size, 5);
        assert_eq!(response.total_pages, 4);
        assert_eq!(response.total_count, 20);
        assert!(response.has_next);
        assert!(!response.has_prev);
    }

    #[test]
    fn test_rate_limiter() {
        let mut limiter = RateLimiter::new(5, 1);

        // First 5 requests should succeed
        for _ in 0..5 {
            assert!(limiter.check_limit("test"));
        }

        // 6th request should fail
        assert!(!limiter.check_limit("test"));
    }

    #[tokio::test]
    async fn test_server_creation() {
        let config = ApiConfig::default();
        let server = RestApiServer::new(config).await;
        assert!(server.is_ok());
    }
}
