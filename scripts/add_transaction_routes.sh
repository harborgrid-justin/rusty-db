#!/bin/bash

# Script to add transaction routes to server.rs

FILE="src/api/rest/server.rs"

# Find the line with the rollback route and add routes after it
sed -i '/post(rollback_transaction),$/a\
            )\
            \/\/ Transaction Management API - Extended\
            .route("\/api\/v1\/transactions\/active", get(get_active_transactions))\
            .route("\/api\/v1\/transactions\/{id}", get(get_transaction))\
            \/\/ Savepoint Operations\
            .route("\/api\/v1\/transactions\/{id}\/savepoint", post(create_savepoint))\
            .route("\/api\/v1\/transactions\/{id}\/release-savepoint", post(release_savepoint))\
            .route("\/api\/v1\/transactions\/{id}\/rollback-to-savepoint", post(rollback_to_savepoint))\
            \/\/ Isolation Level Control\
            .route("\/api\/v1\/transactions\/{id}\/isolation-level", put(update_isolation_level))\
            \/\/ Lock Management\
            .route("\/api\/v1\/transactions\/locks", get(get_locks))\
            .route("\/api\/v1\/transactions\/locks\/waiters", get(get_lock_waiters))\
            .route("\/api\/v1\/transactions\/locks\/graph", get(get_lock_graph))\
            .route("\/api\/v1\/transactions\/locks\/{id}\/release", post(release_lock))\
            .route("\/api\/v1\/transactions\/locks\/release-all", post(release_all_locks))\
            \/\/ Deadlock Detection\
            .route("\/api\/v1\/transactions\/deadlocks", get(get_deadlocks))\
            .route("\/api\/v1\/transactions\/deadlocks\/detect", post(detect_deadlocks))\
            \/\/ MVCC Operations\
            .route("\/api\/v1\/transactions\/mvcc\/status", get(get_mvcc_status))\
            .route("\/api\/v1\/transactions\/mvcc\/snapshots", get(get_mvcc_snapshots))\
            .route("\/api\/v1\/transactions\/mvcc\/versions\/{table}\/{row}", get(get_row_versions))\
            .route("\/api\/v1\/transactions\/mvcc\/vacuum", post(trigger_vacuum))\
            .route("\/api\/v1\/transactions\/mvcc\/vacuum\/full", post(trigger_full_vacuum))\
            \/\/ WAL Operations\
            .route("\/api\/v1\/transactions\/wal\/status", get(get_wal_status))\
            .route("\/api\/v1\/transactions\/wal\/segments", get(get_wal_segments))\
            .route("\/api\/v1\/transactions\/wal\/checkpoint", post(force_checkpoint))\
            .route("\/api\/v1\/transactions\/wal\/archive", post(archive_wal))\
            .route("\/api\/v1\/transactions\/wal\/replay-status", get(get_wal_replay_status))\
            .route("\/api\/v1\/transactions\/wal\/switch", post(switch_wal_segment))\
            .route("/api/v1/stream", get(websocket_stream))' "$FILE.backup"

mv "$FILE.backup" "$FILE"

echo "Transaction routes added to server.rs"
