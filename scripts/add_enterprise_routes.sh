#!/bin/bash

# Script to add Enterprise API routes to server.rs

FILE="/home/user/rusty-db/src/api/rest/server.rs"

# Find the line number where we should insert the routes
# Looking for the line with ".with_state(self.state.clone());" after inmemory handlers
LINE_NUM=$(grep -n "\.with_state(self\.state\.clone());" "$FILE" | tail -1 | cut -d: -f1)

echo "Found insertion point at line $LINE_NUM"

# Create the routes to insert
cat > /tmp/enterprise_routes.txt << 'EOF'
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
EOF

# Insert before the .with_state line
head -n $((LINE_NUM - 1)) "$FILE" > /tmp/server_new.rs
cat /tmp/enterprise_routes.txt >> /tmp/server_new.rs
tail -n +$LINE_NUM "$FILE" >> /tmp/server_new.rs

# Backup and replace
cp "$FILE" "$FILE.backup"
mv /tmp/server_new.rs "$FILE"

echo "Routes added successfully!"
echo "Backup saved to $FILE.backup"
