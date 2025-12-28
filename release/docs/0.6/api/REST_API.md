# RustyDB REST API Reference

**RustyDB v0.6.0 - Enterprise Server**
**API Version**: 1.0.0 (Stable)
**Last Updated**: 2025-12-28
**Base URL**: `http://localhost:8080/api/v1`

---

## Table of Contents

1. [Authentication](#authentication)
2. [Core Database Operations](#core-database-operations)
3. [Schema Management](#schema-management)
4. [Transaction Management](#transaction-management)
5. [Administration](#administration)
6. [Monitoring & Metrics](#monitoring--metrics)
7. [Connection & Pool Management](#connection--pool-management)
8. [Cluster Management](#cluster-management)
9. [Security Management](#security-management)
10. [Storage Operations](#storage-operations)
11. [Backup & Recovery](#backup--recovery)
12. [Graph Database](#graph-database)
13. [Document Store](#document-store)

---

## Authentication

### Login

Authenticate with username and password to obtain JWT token.

```http
POST /api/v1/auth/login
Content-Type: application/json

{
  "username": "admin",
  "password": "secure_password"
}
```

**Response** (200 OK):
```json
{
  "success": true,
  "data": {
    "user": {
      "id": "user_001",
      "username": "admin",
      "displayName": "Administrator",
      "email": "admin@rustydb.com",
      "roles": [
        {
          "id": "role_admin",
          "name": "Admin",
          "permissions": ["*"]
        }
      ]
    },
    "session": {
      "token": "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9...",
      "refreshToken": "refresh_token_here",
      "expiresAt": "2025-12-28T11:00:00Z"
    }
  }
}
```

### Logout

```http
POST /api/v1/auth/logout
Authorization: Bearer <token>
```

**Response** (200 OK):
```json
{
  "success": true,
  "message": "Logged out successfully"
}
```

### Refresh Token

```http
POST /api/v1/auth/refresh
Content-Type: application/json

{
  "refreshToken": "refresh_token_here"
}
```

**Response** (200 OK):
```json
{
  "success": true,
  "data": {
    "token": "new_jwt_token",
    "refreshToken": "new_refresh_token",
    "expiresAt": "2025-12-28T12:00:00Z"
  }
}
```

### Validate Token

```http
GET /api/v1/auth/validate
Authorization: Bearer <token>
```

**Response** (200 OK):
```json
{
  "valid": true,
  "user_id": "user_001",
  "expires_at": "2025-12-28T11:00:00Z",
  "roles": ["admin"]
}
```

---

## Core Database Operations

### Execute Query

Execute a SQL query and return results.

```http
POST /api/v1/query
Authorization: Bearer <token>
Content-Type: application/json

{
  "sql": "SELECT * FROM users WHERE age > ?",
  "params": [18],
  "options": {
    "timeout": 30000,
    "max_rows": 10000
  }
}
```

**Response** (200 OK):
```json
{
  "success": true,
  "data": {
    "columns": ["id", "name", "email", "age"],
    "rows": [
      [1, "Alice", "alice@example.com", 25],
      [2, "Bob", "bob@example.com", 30]
    ],
    "rows_affected": 0,
    "execution_time_ms": 45
  }
}
```

**Query Options**:
- `timeout` (integer, ms): Query timeout (default: 30000)
- `max_rows` (integer): Maximum rows to return (default: 10000)
- `explain` (boolean): Return query plan instead of results
- `analyze` (boolean): Include execution statistics

### Execute Batch Queries

```http
POST /api/v1/batch
Authorization: Bearer <token>
Content-Type: application/json

{
  "queries": [
    { "sql": "INSERT INTO users (name, email) VALUES (?, ?)", "params": ["Alice", "alice@example.com"] },
    { "sql": "INSERT INTO users (name, email) VALUES (?, ?)", "params": ["Bob", "bob@example.com"] },
    { "sql": "SELECT * FROM users" }
  ],
  "transaction": true
}
```

**Response** (200 OK):
```json
{
  "success": true,
  "data": {
    "results": [
      { "rows_affected": 1 },
      { "rows_affected": 1 },
      {
        "columns": ["id", "name", "email"],
        "rows": [
          [1, "Alice", "alice@example.com"],
          [2, "Bob", "bob@example.com"]
        ]
      }
    ],
    "total_time_ms": 150
  }
}
```

### Explain Query

Get the query execution plan.

```http
POST /api/v1/query/explain
Authorization: Bearer <token>
Content-Type: application/json

{
  "sql": "SELECT * FROM users JOIN orders ON users.id = orders.user_id WHERE users.age > 18",
  "analyze": true
}
```

**Response** (200 OK):
```json
{
  "success": true,
  "data": {
    "plan": {
      "type": "HashJoin",
      "estimated_cost": 1234.56,
      "estimated_rows": 5000,
      "children": [
        {
          "type": "SeqScan",
          "table": "users",
          "filter": "age > 18",
          "estimated_rows": 8000
        },
        {
          "type": "IndexScan",
          "table": "orders",
          "index": "orders_user_id_idx",
          "estimated_rows": 15000
        }
      ]
    },
    "execution_stats": {
      "actual_time_ms": 234,
      "actual_rows": 4823,
      "buffers_hit": 1234,
      "buffers_miss": 56
    }
  }
}
```

---

## Schema Management

### List Tables

```http
GET /api/v1/tables?page=1&page_size=50
Authorization: Bearer <token>
```

**Response** (200 OK):
```json
{
  "success": true,
  "data": [
    {
      "name": "users",
      "schema": "public",
      "type": "table",
      "row_count": 10000,
      "size_bytes": 2097152,
      "created_at": "2025-01-01T00:00:00Z"
    },
    {
      "name": "orders",
      "schema": "public",
      "type": "table",
      "row_count": 50000,
      "size_bytes": 10485760,
      "created_at": "2025-01-01T00:00:00Z"
    }
  ],
  "pagination": {
    "page": 1,
    "page_size": 50,
    "total_count": 2,
    "total_pages": 1
  }
}
```

### Get Table Details

```http
GET /api/v1/tables/{table_name}
Authorization: Bearer <token>
```

**Response** (200 OK):
```json
{
  "success": true,
  "data": {
    "name": "users",
    "schema": "public",
    "type": "table",
    "columns": [
      {
        "name": "id",
        "data_type": "INTEGER",
        "nullable": false,
        "default": "nextval('users_id_seq')",
        "primary_key": true
      },
      {
        "name": "name",
        "data_type": "VARCHAR(255)",
        "nullable": false
      },
      {
        "name": "email",
        "data_type": "VARCHAR(255)",
        "nullable": false,
        "unique": true
      }
    ],
    "indexes": [
      {
        "name": "users_pkey",
        "type": "btree",
        "columns": ["id"],
        "unique": true,
        "primary": true
      },
      {
        "name": "users_email_idx",
        "type": "btree",
        "columns": ["email"],
        "unique": true
      }
    ],
    "constraints": [
      {
        "name": "users_age_check",
        "type": "check",
        "definition": "age >= 0"
      }
    ],
    "statistics": {
      "row_count": 10000,
      "size_bytes": 2097152,
      "last_vacuum": "2025-12-28T00:00:00Z",
      "last_analyze": "2025-12-28T00:00:00Z"
    }
  }
}
```

### Create Table

```http
POST /api/v1/tables
Authorization: Bearer <token>
Content-Type: application/json

{
  "name": "products",
  "schema": "public",
  "columns": [
    {
      "name": "id",
      "data_type": "INTEGER",
      "primary_key": true,
      "auto_increment": true
    },
    {
      "name": "name",
      "data_type": "VARCHAR(255)",
      "nullable": false
    },
    {
      "name": "price",
      "data_type": "DECIMAL(10,2)",
      "nullable": false
    }
  ],
  "indexes": [
    {
      "name": "products_name_idx",
      "columns": ["name"],
      "type": "btree"
    }
  ]
}
```

**Response** (201 Created):
```json
{
  "success": true,
  "data": {
    "name": "products",
    "created": true,
    "ddl": "CREATE TABLE products (...)"
  }
}
```

### Drop Table

```http
DELETE /api/v1/tables/{table_name}?cascade=true
Authorization: Bearer <token>
```

**Response** (200 OK):
```json
{
  "success": true,
  "data": {
    "dropped": true,
    "cascaded": ["dependent_view1", "foreign_key_constraint"]
  }
}
```

---

## Transaction Management

### Begin Transaction

```http
POST /api/v1/transactions
Authorization: Bearer <token>
Content-Type: application/json

{
  "isolation_level": "REPEATABLE_READ"
}
```

**Isolation Levels**:
- `READ_UNCOMMITTED`
- `READ_COMMITTED` (default)
- `REPEATABLE_READ`
- `SERIALIZABLE`

**Response** (201 Created):
```json
{
  "success": true,
  "data": {
    "transaction_id": "txn_12345",
    "isolation_level": "REPEATABLE_READ",
    "started_at": "2025-12-28T10:00:00Z"
  }
}
```

### Commit Transaction

```http
POST /api/v1/transactions/{transaction_id}/commit
Authorization: Bearer <token>
```

**Response** (200 OK):
```json
{
  "success": true,
  "data": {
    "transaction_id": "txn_12345",
    "committed": true,
    "duration_ms": 1234
  }
}
```

### Rollback Transaction

```http
POST /api/v1/transactions/{transaction_id}/rollback
Authorization: Bearer <token>
```

**Response** (200 OK):
```json
{
  "success": true,
  "data": {
    "transaction_id": "txn_12345",
    "rolled_back": true
  }
}
```

---

## Administration

### Get Configuration

```http
GET /api/v1/admin/config
Authorization: Bearer <token>
```

**Response** (200 OK):
```json
{
  "success": true,
  "data": {
    "buffer_pool_size": 1073741824,
    "max_connections": 1000,
    "shared_memory_size": 268435456,
    "wal_buffer_size": 16777216,
    "checkpoint_interval_sec": 300,
    "vacuum_enabled": true,
    "log_level": "INFO"
  }
}
```

### Update Configuration

```http
PUT /api/v1/admin/config
Authorization: Bearer <token>
Content-Type: application/json

{
  "buffer_pool_size": 2147483648,
  "max_connections": 2000,
  "log_level": "DEBUG"
}
```

**Response** (200 OK):
```json
{
  "success": true,
  "data": {
    "updated": true,
    "restart_required": true,
    "changes": {
      "buffer_pool_size": {
        "old": 1073741824,
        "new": 2147483648
      }
    }
  }
}
```

### Health Check

```http
GET /api/v1/admin/health
Authorization: Bearer <token>
```

**Response** (200 OK):
```json
{
  "success": true,
  "data": {
    "status": "HEALTHY",
    "components": {
      "database": {
        "status": "UP",
        "uptime_sec": 86400,
        "version": "0.6.0"
      },
      "storage": {
        "status": "UP",
        "disk_usage_percent": 45,
        "buffer_pool_hit_ratio": 0.98
      },
      "replication": {
        "status": "UP",
        "lag_bytes": 1024,
        "replicas": 2
      }
    },
    "timestamp": "2025-12-28T10:00:00Z"
  }
}
```

### User Management

**Create User**:
```http
POST /api/v1/admin/users
Authorization: Bearer <token>
Content-Type: application/json

{
  "username": "john_doe",
  "password": "secure_password",
  "email": "john@example.com",
  "roles": ["user", "analyst"]
}
```

**List Users**:
```http
GET /api/v1/admin/users?page=1&page_size=50
Authorization: Bearer <token>
```

**Get User**:
```http
GET /api/v1/admin/users/{user_id}
Authorization: Bearer <token>
```

**Update User**:
```http
PUT /api/v1/admin/users/{user_id}
Authorization: Bearer <token>
Content-Type: application/json

{
  "email": "new_email@example.com",
  "roles": ["user", "analyst", "admin"]
}
```

**Delete User**:
```http
DELETE /api/v1/admin/users/{user_id}
Authorization: Bearer <token>
```

---

## Monitoring & Metrics

### Get Metrics

```http
GET /api/v1/metrics
Authorization: Bearer <token>
```

**Response** (200 OK):
```json
{
  "success": true,
  "data": {
    "database": {
      "uptime_seconds": 86400,
      "active_connections": 45,
      "transactions_per_second": 1234.56,
      "queries_per_second": 5678.90
    },
    "storage": {
      "buffer_pool_size_bytes": 1073741824,
      "buffer_pool_used_bytes": 858993459,
      "buffer_pool_hit_ratio": 0.98,
      "disk_reads_per_second": 123.45,
      "disk_writes_per_second": 67.89
    },
    "transactions": {
      "active_transactions": 12,
      "committed_total": 1000000,
      "rolled_back_total": 1234,
      "deadlocks_total": 5
    },
    "query_performance": {
      "avg_query_time_ms": 45.67,
      "p95_query_time_ms": 123.45,
      "p99_query_time_ms": 234.56,
      "slow_queries_total": 78
    }
  }
}
```

### Get Prometheus Metrics

```http
GET /api/v1/metrics/prometheus
```

**Response** (200 OK - Prometheus text format):
```
# HELP rustydb_uptime_seconds Database uptime in seconds
# TYPE rustydb_uptime_seconds counter
rustydb_uptime_seconds 86400

# HELP rustydb_active_connections Number of active connections
# TYPE rustydb_active_connections gauge
rustydb_active_connections 45

# HELP rustydb_transactions_total Total number of transactions
# TYPE rustydb_transactions_total counter
rustydb_transactions_total{status="committed"} 1000000
rustydb_transactions_total{status="rolled_back"} 1234
```

### Get Logs

```http
GET /api/v1/logs?level=ERROR&start=2025-12-28T00:00:00Z&limit=1000
Authorization: Bearer <token>
```

**Response** (200 OK):
```json
{
  "success": true,
  "data": {
    "logs": [
      {
        "timestamp": "2025-12-28T10:00:00Z",
        "level": "ERROR",
        "message": "Transaction deadlock detected",
        "context": {
          "transaction_id": "txn_12345",
          "victim": "txn_67890"
        }
      }
    ]
  }
}
```

---

## Connection & Pool Management

### List Connection Pools

```http
GET /api/v1/pools
Authorization: Bearer <token>
```

**Response** (200 OK):
```json
{
  "success": true,
  "data": [
    {
      "id": "pool_default",
      "name": "Default Pool",
      "min_connections": 10,
      "max_connections": 100,
      "active_connections": 45,
      "idle_connections": 50,
      "wait_queue_size": 2
    }
  ]
}
```

### Get Pool Statistics

```http
GET /api/v1/pools/{pool_id}/stats
Authorization: Bearer <token>
```

**Response** (200 OK):
```json
{
  "success": true,
  "data": {
    "active_connections": 45,
    "idle_connections": 50,
    "wait_queue_size": 2,
    "connections_per_second": 12.34,
    "avg_wait_time_ms": 56.78,
    "peak_connections": 95,
    "utilization_percent": 95
  }
}
```

### List Active Connections

```http
GET /api/v1/connections?status=active
Authorization: Bearer <token>
```

**Response** (200 OK):
```json
{
  "success": true,
  "data": [
    {
      "id": "conn_12345",
      "user": "admin",
      "database": "rustydb",
      "client_addr": "192.168.1.100:54321",
      "state": "active",
      "query": "SELECT * FROM users",
      "connected_at": "2025-12-28T09:00:00Z",
      "last_activity": "2025-12-28T10:00:00Z"
    }
  ]
}
```

### Kill Connection

```http
DELETE /api/v1/connections/{connection_id}?force=true
Authorization: Bearer <token>
```

**Response** (200 OK):
```json
{
  "success": true,
  "data": {
    "connection_id": "conn_12345",
    "killed": true
  }
}
```

---

## Cluster Management

### List Cluster Nodes

```http
GET /api/v1/cluster/nodes
Authorization: Bearer <token>
```

**Response** (200 OK):
```json
{
  "success": true,
  "data": [
    {
      "id": "node_1",
      "name": "rustydb-node-1",
      "role": "PRIMARY",
      "status": "UP",
      "address": "192.168.1.10:5432",
      "version": "0.6.0",
      "uptime_sec": 86400,
      "lag_bytes": 0
    },
    {
      "id": "node_2",
      "name": "rustydb-node-2",
      "role": "REPLICA",
      "status": "UP",
      "address": "192.168.1.11:5432",
      "version": "0.6.0",
      "uptime_sec": 86400,
      "lag_bytes": 1024
    }
  ]
}
```

### Get Cluster Topology

```http
GET /api/v1/cluster/topology
Authorization: Bearer <token>
```

**Response** (200 OK):
```json
{
  "success": true,
  "data": {
    "cluster_id": "cluster_main",
    "primary": {
      "node_id": "node_1",
      "name": "rustydb-node-1",
      "address": "192.168.1.10:5432"
    },
    "replicas": [
      {
        "node_id": "node_2",
        "name": "rustydb-node-2",
        "address": "192.168.1.11:5432",
        "replication_mode": "SYNCHRONOUS"
      }
    ],
    "shards": [
      {
        "shard_id": "shard_1",
        "range": "0-1000000",
        "primary_node": "node_1",
        "replica_nodes": ["node_2"]
      }
    ]
  }
}
```

---

## Security Management

### List Encryption Keys

```http
GET /api/v1/security/keys
Authorization: Bearer <token>
```

**Response** (200 OK):
```json
{
  "success": true,
  "data": [
    {
      "id": "key_master",
      "type": "MASTER",
      "algorithm": "AES-256-GCM",
      "created_at": "2025-01-01T00:00:00Z",
      "rotated_at": "2025-12-01T00:00:00Z",
      "status": "ACTIVE"
    }
  ]
}
```

### Rotate Encryption Key

```http
POST /api/v1/security/keys/{key_id}/rotate
Authorization: Bearer <token>
```

**Response** (200 OK):
```json
{
  "success": true,
  "data": {
    "key_id": "key_master",
    "old_version": 5,
    "new_version": 6,
    "rotation_started": "2025-12-28T10:00:00Z"
  }
}
```

### Get Audit Logs

```http
GET /api/v1/security/audit?start=2025-12-28T00:00:00Z&limit=1000
Authorization: Bearer <token>
```

**Response** (200 OK):
```json
{
  "success": true,
  "data": {
    "logs": [
      {
        "id": "audit_12345",
        "timestamp": "2025-12-28T10:00:00Z",
        "user": "admin",
        "action": "TABLE_DELETE",
        "resource": "users",
        "result": "SUCCESS",
        "client_ip": "192.168.1.100"
      }
    ]
  }
}
```

---

## Storage Operations

### Get Buffer Pool Status

```http
GET /api/v1/storage/buffer-pool
Authorization: Bearer <token>
```

**Response** (200 OK):
```json
{
  "success": true,
  "data": {
    "total_pages": 262144,
    "used_pages": 196608,
    "free_pages": 65536,
    "dirty_pages": 2048,
    "hit_ratio": 0.98,
    "eviction_policy": "LRU",
    "stats": {
      "reads": 1000000,
      "writes": 500000,
      "hits": 980000,
      "misses": 20000
    }
  }
}
```

### Create Checkpoint

```http
POST /api/v1/storage/checkpoint
Authorization: Bearer <token>
```

**Response** (200 OK):
```json
{
  "success": true,
  "data": {
    "checkpoint_id": "ckpt_12345",
    "lsn": "0/1000000",
    "pages_written": 2048,
    "duration_ms": 5000
  }
}
```

---

## Backup & Recovery

### Create Full Backup

```http
POST /api/v1/admin/backup
Authorization: Bearer <token>
Content-Type: application/json

{
  "type": "FULL",
  "compression": true,
  "destination": "/backups/backup_20251228.tar.gz",
  "include_wal": true
}
```

**Response** (202 Accepted):
```json
{
  "success": true,
  "data": {
    "backup_id": "backup_12345",
    "type": "FULL",
    "started_at": "2025-12-28T10:00:00Z",
    "status": "IN_PROGRESS",
    "estimated_completion": "2025-12-28T10:15:00Z"
  }
}
```

### Restore Backup

```http
POST /api/v1/admin/restore
Authorization: Bearer <token>
Content-Type: application/json

{
  "backup_id": "backup_12345",
  "point_in_time": "2025-12-28T09:00:00Z",
  "target_database": "rustydb_restore"
}
```

**Response** (202 Accepted):
```json
{
  "success": true,
  "data": {
    "restore_id": "restore_12345",
    "status": "IN_PROGRESS",
    "estimated_completion": "2025-12-28T10:30:00Z"
  }
}
```

---

## Graph Database

### Execute Graph Query

```http
POST /api/v1/graph/query
Authorization: Bearer <token>
Content-Type: application/json

{
  "query": "MATCH (n:Person {name: 'Alice'})-[r:KNOWS]->(m:Person) RETURN m",
  "params": {}
}
```

**Response** (200 OK):
```json
{
  "success": true,
  "data": {
    "nodes": [
      {
        "id": "node_1",
        "labels": ["Person"],
        "properties": {
          "name": "Bob",
          "age": 30
        }
      }
    ],
    "relationships": [
      {
        "id": "rel_1",
        "type": "KNOWS",
        "start_node": "node_0",
        "end_node": "node_1"
      }
    ]
  }
}
```

### Find Shortest Path

```http
POST /api/v1/graph/algorithms/shortest-path
Authorization: Bearer <token>
Content-Type: application/json

{
  "start_node": "node_1",
  "end_node": "node_100",
  "algorithm": "dijkstra"
}
```

**Response** (200 OK):
```json
{
  "success": true,
  "data": {
    "path": ["node_1", "node_5", "node_23", "node_100"],
    "distance": 3,
    "cost": 42.5
  }
}
```

---

## Document Store

### Find Documents

```http
POST /api/v1/documents/collections/{collection_name}/find
Authorization: Bearer <token>
Content-Type: application/json

{
  "filter": {
    "age": { "$gt": 18 },
    "status": "active"
  },
  "projection": {
    "name": 1,
    "email": 1
  },
  "limit": 10
}
```

**Response** (200 OK):
```json
{
  "success": true,
  "data": [
    {
      "_id": "doc_12345",
      "name": "Alice",
      "email": "alice@example.com"
    },
    {
      "_id": "doc_67890",
      "name": "Bob",
      "email": "bob@example.com"
    }
  ]
}
```

### Aggregate Documents

```http
POST /api/v1/documents/collections/{collection_name}/aggregate
Authorization: Bearer <token>
Content-Type: application/json

{
  "pipeline": [
    { "$match": { "status": "active" } },
    { "$group": { "_id": "$department", "count": { "$sum": 1 } } },
    { "$sort": { "count": -1 } }
  ]
}
```

**Response** (200 OK):
```json
{
  "success": true,
  "data": [
    { "_id": "engineering", "count": 45 },
    { "_id": "sales", "count": 32 },
    { "_id": "marketing", "count": 18 }
  ]
}
```

---

## Common Patterns

### Request Headers

**Required**:
```http
Authorization: Bearer <token>
Content-Type: application/json
Accept: application/json
```

**Optional**:
```http
X-Request-ID: <uuid>
X-Request-Time: <iso8601>
Accept-Encoding: gzip
```

### Pagination

```http
GET /api/v1/endpoint?page=1&page_size=50&sort_by=created_at&sort_order=desc
```

**Parameters**:
- `page` (integer, default: 1)
- `page_size` (integer, default: 50, max: 1000)
- `sort_by` (string)
- `sort_order` (enum: `asc`|`desc`)

### Filtering

```http
GET /api/v1/endpoint?filter[status]=active&filter[type]=backup
```

---

## Rate Limiting

**Headers**:
```http
X-RateLimit-Limit: 100
X-RateLimit-Remaining: 95
X-RateLimit-Reset: 1735382400
Retry-After: 60
```

**Limits**:
- Global: 100 requests/second
- `/query`: 50 requests/second
- `/batch`: 10 requests/second
- `/admin/*`: 20 requests/second

---

## Error Handling

### Error Response Format

```json
{
  "success": false,
  "error": {
    "code": "ERROR_CODE",
    "message": "Human-readable error message",
    "details": {}
  },
  "meta": {
    "requestId": "req_12345",
    "timestamp": "2025-12-28T10:00:00Z"
  }
}
```

### HTTP Status Codes

| Code | Name | Usage |
|------|------|-------|
| 200 | OK | Request succeeded |
| 201 | Created | Resource created |
| 204 | No Content | Success, no content |
| 400 | Bad Request | Invalid request |
| 401 | Unauthorized | Auth required |
| 403 | Forbidden | Insufficient permissions |
| 404 | Not Found | Resource not found |
| 409 | Conflict | Resource conflict |
| 429 | Too Many Requests | Rate limit exceeded |
| 500 | Internal Server Error | Server error |

---

## Additional Resources

- **OpenAPI Specification**: `http://localhost:8080/api-docs/openapi.json`
- **Swagger UI**: `http://localhost:8080/swagger-ui`
- **API Overview**: [API_OVERVIEW.md](./API_OVERVIEW.md)
- **GraphQL API**: [GRAPHQL_API.md](./GRAPHQL_API.md)
- **WebSocket API**: [WEBSOCKET_API.md](./WEBSOCKET_API.md)

---

**Last Updated**: 2025-12-28
**API Version**: 1.0.0 (Stable)
**Product Version**: RustyDB v0.6.0 Enterprise Server
