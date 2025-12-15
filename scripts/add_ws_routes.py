#!/usr/bin/env python3

import re

# Read the file
with open('src/api/rest/server.rs', 'r') as f:
    content = f.read()

# Define the WebSocket routes to add
ws_routes = '''
            // Transaction WebSocket Streams
            .route("/api/v1/ws/transactions/lifecycle", get(ws_transaction_lifecycle))
            .route("/api/v1/ws/transactions/locks", get(ws_lock_events))
            .route("/api/v1/ws/transactions/deadlocks", get(ws_deadlock_events))
            .route("/api/v1/ws/transactions/mvcc", get(ws_mvcc_events))
            .route("/api/v1/ws/transactions/wal", get(ws_wal_events))
'''

# Find the pattern and insert routes after the existing ws routes
pattern = r'(\.route\(\s*"/api/v1/ws/replication",\s*get\(websocket_handlers::ws_replication_stream\),\s*\))'
replacement = r'\1' + ws_routes

new_content = re.sub(pattern, replacement, content, flags=re.MULTILINE | re.DOTALL)

if new_content != content:
    with open('src/api/rest/server.rs', 'w') as f:
        f.write(new_content)
    print("Transaction WebSocket routes successfully added!")
else:
    print("Pattern not found or routes already added")
