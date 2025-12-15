#!/usr/bin/env python3

import re

# Read the file
with open('src/api/rest/server.rs', 'r') as f:
    content = f.read()

# Define the routes to add
new_routes = '''
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
'''

# Find the pattern and insert routes
pattern = r'(\.route\(\s*"/api/v1/transactions/\{id\}/rollback",\s*post\(rollback_transaction\),\s*\))\s*(\.route\("/api/v1/stream")'
replacement = r'\1' + new_routes + r'\2'

new_content = re.sub(pattern, replacement, content, flags=re.MULTILINE | re.DOTALL)

if new_content != content:
    with open('src/api/rest/server.rs', 'w') as f:
        f.write(new_content)
    print("Transaction routes successfully added!")
else:
    print("Pattern not found or routes already added")
