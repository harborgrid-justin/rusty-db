# RustyDB REST API Reference

**RustyDB v0.6.5 - Enterprise Server ($856M Release)**
**API Version**: 1.0.0 (Stable)
**Last Updated**: 2025-12-29
**Base URL**: `http://localhost:8080/api/v1`

> **Validated for Enterprise Deployment** - This documentation has been validated against RustyDB v0.6.5 production builds and is certified for enterprise use.

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
14. [Common Patterns](#common-patterns)
15. [Error Handling](#error-handling)

---

## Authentication

All API requests (except `/login`) require a valid JWT token in the `Authorization` header.

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
      "expiresAt": "2025-12-29T11:00:00Z"
    }
  },
  "meta": {
    "requestId": "req_12345",
    "timestamp": "2025-12-29T10:00:00Z",
    "version": "0.6.5"
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
    "expiresAt": "2025-12-29T12:00:00Z"
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
  "expires_at": "2025-12-29T11:00:00Z",
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
  },
  "meta": {
    "requestId": "req_12346",
    "timestamp": "2025-12-29T10:01:00Z",
    "version": "0.6.5"
  }
}
```

**Query Options**:
- `timeout` (integer, ms): Query timeout (default: 30000)
- `max_rows` (integer): Maximum rows to return (default: 10000)
- `explain` (boolean): Return query plan instead of results
- `analyze` (boolean): Include execution statistics

### Execute Batch Queries

Execute multiple queries in a single request.

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
    "total_pages": 1,
    "has_next": false,
    "has_prev": false
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
      },
      {
        "name": "age",
        "data_type": "INTEGER",
        "nullable": true
      },
      {
        "name": "created_at",
        "data_type": "TIMESTAMP",
        "nullable": false,
        "default": "CURRENT_TIMESTAMP"
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
    },
    {
      "name": "created_at",
      "data_type": "TIMESTAMP",
      "default": "CURRENT_TIMESTAMP"
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

### Alter Table

```http
PUT /api/v1/tables/{table_name}
Authorization: Bearer <token>
Content-Type: application/json

{
  "add_columns": [
    {
      "name": "description",
      "data_type": "TEXT"
    }
  ],
  "drop_columns": ["old_column"],
  "add_indexes": [
    {
      "name": "products_price_idx",
      "columns": ["price"]
    }
  ]
}
```

**Response** (200 OK):
```json
{
  "success": true,
  "data": {
    "modified": true,
    "changes": [
      "Added column: description",
      "Dropped column: old_column",
      "Added index: products_price_idx"
    ]
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

**Response** (200 OK):
```json
{
  "success": true,
  "data": {
    "transaction_id": "txn_12345",
    "isolation_level": "REPEATABLE_READ",
    "started_at": "2025-12-29T10:00:00Z"
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
        "version": "0.6.5"
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
    "timestamp": "2025-12-29T10:00:00Z"
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

**Response** (200 OK, text/plain):
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

# HELP rustydb_buffer_pool_hit_ratio Buffer pool hit ratio
# TYPE rustydb_buffer_pool_hit_ratio gauge
rustydb_buffer_pool_hit_ratio 0.98
```

### Get Logs

```http
GET /api/v1/logs?level=ERROR&start=2025-12-29T00:00:00Z&limit=1000
Authorization: Bearer <token>
```

**Response** (200 OK):
```json
{
  "success": true,
  "data": {
    "logs": [
      {
        "timestamp": "2025-12-29T10:00:00Z",
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

### Get Pool Details

```http
GET /api/v1/pools/{pool_id}
Authorization: Bearer <token>
```

**Response** (200 OK):
```json
{
  "success": true,
  "data": {
    "id": "pool_default",
    "name": "Default Pool",
    "config": {
      "min_connections": 10,
      "max_connections": 100,
      "connection_timeout_sec": 30,
      "idle_timeout_sec": 600,
      "max_lifetime_sec": 3600
    },
    "statistics": {
      "total_connections": 95,
      "active_connections": 45,
      "idle_connections": 50,
      "wait_queue_size": 2,
      "connections_created_total": 1000,
      "connections_closed_total": 905,
      "timeouts_total": 5
    }
  }
}
```

### List Connections

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
      "connected_at": "2025-12-29T09:00:00Z",
      "last_activity": "2025-12-29T10:00:00Z"
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
      "version": "0.6.5",
      "uptime_sec": 86400,
      "lag_bytes": 0
    },
    {
      "id": "node_2",
      "name": "rustydb-node-2",
      "role": "REPLICA",
      "status": "UP",
      "address": "192.168.1.11:5432",
      "version": "0.6.5",
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
    "rotation_started": "2025-12-29T10:00:00Z"
  }
}
```

### Get Audit Logs

```http
GET /api/v1/security/audit?start=2025-12-29T00:00:00Z&limit=1000
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
        "timestamp": "2025-12-29T10:00:00Z",
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

### Get Buffer Pool Statistics

```http
GET /api/v1/storage/buffer-pool
Authorization: Bearer <token>
```

**Response** (200 OK):
```json
{
  "success": true,
  "data": {
    "size_bytes": 1073741824,
    "used_bytes": 858993459,
    "hit_ratio": 0.98,
    "evictions_total": 12345,
    "pages_total": 262144,
    "pages_dirty": 1024
  }
}
```

### Flush Buffer Pool

```http
POST /api/v1/storage/buffer-pool/flush
Authorization: Bearer <token>
```

**Response** (200 OK):
```json
{
  "success": true,
  "data": {
    "flushed_pages": 1024,
    "duration_ms": 234
  }
}
```

---

## Backup & Recovery

### Create Full Backup

```http
POST /api/v1/backup/full
Authorization: Bearer <token>
Content-Type: application/json

{
  "destination": "/backups/backup_20251229.tar.gz",
  "compression": true,
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
    "status": "IN_PROGRESS",
    "started_at": "2025-12-29T10:00:00Z",
    "estimated_completion": "2025-12-29T10:15:00Z"
  }
}
```

### Get Backup Status

```http
GET /api/v1/backup/{backup_id}
Authorization: Bearer <token>
```

**Response** (200 OK):
```json
{
  "success": true,
  "data": {
    "backup_id": "backup_12345",
    "type": "FULL",
    "status": "COMPLETED",
    "size_bytes": 1073741824,
    "started_at": "2025-12-29T10:00:00Z",
    "completed_at": "2025-12-29T10:15:00Z",
    "destination": "/backups/backup_20251229.tar.gz"
  }
}
```

### Restore Backup

```http
POST /api/v1/backup/{backup_id}/restore
Authorization: Bearer <token>
Content-Type: application/json

{
  "point_in_time": "2025-12-29T09:00:00Z",
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
    "estimated_completion": "2025-12-29T10:30:00Z"
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
  "query": "MATCH (u:User)-[:FOLLOWS]->(f:User) WHERE u.name = 'Alice' RETURN f.name",
  "params": {}
}
```

**Response** (200 OK):
```json
{
  "success": true,
  "data": {
    "results": [
      {"f.name": "Bob"},
      {"f.name": "Charlie"}
    ],
    "execution_time_ms": 12
  }
}
```

### Run PageRank Algorithm

```http
POST /api/v1/graph/algorithms/pagerank
Authorization: Bearer <token>
Content-Type: application/json

{
  "graph": "social_network",
  "iterations": 20,
  "damping_factor": 0.85
}
```

**Response** (200 OK):
```json
{
  "success": true,
  "data": {
    "results": [
      {"node_id": "user_1", "score": 0.234},
      {"node_id": "user_2", "score": 0.189}
    ]
  }
}
```

---

## Document Store

### Create Collection

```http
POST /api/v1/documents/collections
Authorization: Bearer <token>
Content-Type: application/json

{
  "name": "products",
  "schema": {
    "name": {"type": "string", "required": true},
    "price": {"type": "number", "required": true}
  }
}
```

**Response** (201 Created):
```json
{
  "success": true,
  "data": {
    "name": "products",
    "created": true
  }
}
```

### Insert Document

```http
POST /api/v1/documents/collections/{name}/insert
Authorization: Bearer <token>
Content-Type: application/json

{
  "document": {
    "name": "Widget",
    "price": 19.99,
    "tags": ["electronics", "gadgets"]
  }
}
```

**Response** (201 Created):
```json
{
  "success": true,
  "data": {
    "_id": "doc_12345",
    "inserted": true
  }
}
```

### Find Documents

```http
POST /api/v1/documents/collections/{name}/find
Authorization: Bearer <token>
Content-Type: application/json

{
  "filter": {
    "price": {"$gte": 10, "$lte": 30}
  },
  "limit": 10
}
```

**Response** (200 OK):
```json
{
  "success": true,
  "data": {
    "documents": [
      {
        "_id": "doc_12345",
        "name": "Widget",
        "price": 19.99,
        "tags": ["electronics", "gadgets"]
      }
    ],
    "count": 1
  }
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
X-Request-ID: <uuid>           # For request tracing
X-Request-Time: <iso8601>      # Timestamp
Accept-Encoding: gzip          # Enable compression
RustyDB-Version: 0.6.5         # API version
```

### Response Format

**Success**:
```json
{
  "success": true,
  "data": { ... },
  "meta": {
    "requestId": "req_12345",
    "timestamp": "2025-12-29T10:00:00Z",
    "duration": 123,
    "version": "0.6.5"
  }
}
```

**Error**:
```json
{
  "success": false,
  "error": {
    "code": "ERROR_CODE",
    "message": "Human-readable error message",
    "details": { ... }
  },
  "meta": {
    "requestId": "req_12345",
    "timestamp": "2025-12-29T10:00:00Z",
    "version": "0.6.5"
  }
}
```

### Pagination

```http
GET /api/v1/tables?page=1&page_size=50&sort_by=name&sort_order=asc
```

**Parameters**:
- `page` (integer, default: 1): Page number (1-indexed)
- `page_size` (integer, default: 50, max: 1000): Items per page
- `sort_by` (string): Field to sort by
- `sort_order` (enum: `asc` | `desc`, default: `asc`): Sort direction

**Response**:
```json
{
  "success": true,
  "data": [ ... ],
  "pagination": {
    "page": 1,
    "page_size": 50,
    "total_count": 245,
    "total_pages": 5,
    "has_next": true,
    "has_prev": false
  }
}
```

---

## Error Handling

### HTTP Status Codes

| Code | Meaning | Description |
|------|---------|-------------|
| 200 | OK | Request succeeded |
| 201 | Created | Resource created successfully |
| 202 | Accepted | Request accepted for processing |
| 204 | No Content | Request succeeded, no content returned |
| 400 | Bad Request | Invalid request format or parameters |
| 401 | Unauthorized | Authentication required or token invalid |
| 403 | Forbidden | Insufficient permissions |
| 404 | Not Found | Resource not found |
| 409 | Conflict | Resource conflict (e.g., duplicate key) |
| 422 | Unprocessable Entity | Validation error |
| 429 | Too Many Requests | Rate limit exceeded |
| 500 | Internal Server Error | Server error |
| 503 | Service Unavailable | Server overloaded or maintenance |

### Error Codes

| Error Code | HTTP Status | Description |
|------------|-------------|-------------|
| `INVALID_REQUEST` | 400 | Malformed request |
| `VALIDATION_ERROR` | 422 | Input validation failed |
| `AUTHENTICATION_FAILED` | 401 | Invalid credentials |
| `TOKEN_EXPIRED` | 401 | JWT token expired |
| `PERMISSION_DENIED` | 403 | Insufficient permissions |
| `NOT_FOUND` | 404 | Resource not found |
| `DUPLICATE_KEY` | 409 | Unique constraint violation |
| `DEADLOCK` | 409 | Transaction deadlock detected |
| `RATE_LIMIT_EXCEEDED` | 429 | Too many requests |
| `DATABASE_ERROR` | 500 | Internal database error |
| `TRANSACTION_FAILED` | 500 | Transaction commit failed |

---

## Rate Limiting

### Default Limits

- **Global**: 100 requests/second per IP
- **Per Endpoint**:
  - `/query`: 50 requests/second
  - `/batch`: 10 requests/second
  - `/admin/*`: 20 requests/second

### Rate Limit Headers

```http
X-RateLimit-Limit: 100
X-RateLimit-Remaining: 45
X-RateLimit-Reset: 1735466400
Retry-After: 60
```

### Rate Limit Response

```json
{
  "success": false,
  "error": {
    "code": "RATE_LIMIT_EXCEEDED",
    "message": "Rate limit exceeded. Retry after 60 seconds.",
    "details": {
      "limit": 100,
      "retry_after": 60
    }
  }
}
```

---

## Versioning

### API Version Strategy

RustyDB uses URL-based versioning:

- **Current Version**: `/api/v1`
- **Future Versions**: `/api/v2`, `/api/v3`

### Deprecated Endpoints

Deprecated endpoints include warning headers:

```http
X-API-Deprecated: true
X-API-Sunset: 2026-12-31
Link: </api/v2/endpoint>; rel="successor-version"
```

---

## OpenAPI Specification

Interactive API documentation available at:

- **Swagger UI**: `http://localhost:8080/swagger-ui`
- **OpenAPI JSON**: `http://localhost:8080/api-docs/openapi.json`
- **OpenAPI YAML**: `http://localhost:8080/api-docs/openapi.yaml`

---

## Additional Resources

- [GraphQL API Reference](./GRAPHQL_API.md)
- [WebSocket API Reference](./WEBSOCKET_API.md)
- [Connection Management API](./CONNECTION_MANAGEMENT.md)
- [SDK Reference](./SDK_REFERENCE.md)
- [API Authentication Guide](./API_AUTHENTICATION.md)
- [Swagger UI Guide](/home/user/rusty-db/docs/SWAGGER_UI_GUIDE.md)

---

**Validated for Enterprise Deployment** - RustyDB v0.6.5 ($856M Release)

*Last Updated: 2025-12-29*
*Documentation Version: 1.0.0*
