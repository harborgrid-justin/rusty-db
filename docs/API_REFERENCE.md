# RustyDB API Reference

**Version**: 1.0.0
**Last Updated**: 2025-12-11
**Base URL**: `http://localhost:8080/api/v1`

## Table of Contents

1. [Overview](#overview)
2. [Authentication & Authorization](#authentication--authorization)
3. [Common Patterns](#common-patterns)
4. [Error Handling](#error-handling)
5. [REST API Endpoints](#rest-api-endpoints)
   - [Core Database Operations](#core-database-operations)
   - [Schema Management](#schema-management)
   - [Transaction Management](#transaction-management)
   - [Administration](#administration)
   - [Monitoring & Metrics](#monitoring--metrics)
   - [Pool & Connection Management](#pool--connection-management)
   - [Cluster Management](#cluster-management)
   - [Security Management](#security-management)
   - [Backup & Recovery](#backup--recovery)
6. [GraphQL API](#graphql-api)
7. [WebSocket API](#websocket-api)
8. [Rate Limiting](#rate-limiting)
9. [Versioning](#versioning)

---

## Overview

RustyDB provides multiple API interfaces for database interaction:

- **REST API**: HTTP/JSON endpoints for all database operations
- **GraphQL API**: Flexible query language with real-time subscriptions
- **WebSocket API**: Streaming query results and real-time updates
- **PostgreSQL Wire Protocol**: Native PostgreSQL compatibility

### API Features

- **OpenAPI/Swagger Documentation**: Auto-generated API documentation
- **Request Validation**: Automatic input validation
- **Response Pagination**: Paginated responses for large datasets
- **Rate Limiting**: Configurable rate limits per endpoint
- **CORS Support**: Cross-origin resource sharing
- **Compression**: Gzip compression for responses
- **Authentication**: JWT-based token authentication
- **Authorization**: Fine-grained RBAC permissions

### API Configuration

Default configuration (can be customized):

```rust
ApiConfig {
    host: "0.0.0.0",
    port: 8080,
    max_connections: 1000,
    request_timeout_secs: 30,
    max_body_size: 10_485_760, // 10 MB
    enable_cors: true,
    enable_swagger: true,
    rate_limit_rps: 100, // 100 requests per second
}
```

---

## Authentication & Authorization

### JWT Token Authentication

All API requests (except `/login`) require a valid JWT token in the `Authorization` header.

#### Login

```http
POST /api/v1/auth/login
Content-Type: application/json

{
  "username": "admin",
  "password": "secret123"
}
```

**Response**:
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
      "expiresAt": "2025-12-12T10:00:00Z"
    }
  }
}
```

#### Using the Token

Include the token in the `Authorization` header for all subsequent requests:

```http
GET /api/v1/tables
Authorization: Bearer eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9...
```

#### Refresh Token

```http
POST /api/v1/auth/refresh
Content-Type: application/json

{
  "refreshToken": "refresh_token_here"
}
```

**Response**:
```json
{
  "success": true,
  "data": {
    "token": "new_jwt_token",
    "refreshToken": "new_refresh_token",
    "expiresAt": "2025-12-12T11:00:00Z"
  }
}
```

#### Logout

```http
POST /api/v1/auth/logout
Authorization: Bearer <token>
```

**Response**:
```json
{
  "success": true,
  "message": "Logged out successfully"
}
```

### Role-Based Access Control (RBAC)

Permissions are checked for each endpoint based on user roles:

| Resource | Action | Required Permission |
|----------|--------|-------------------|
| Tables | Read | `tables.read` |
| Tables | Create | `tables.create` |
| Tables | Update | `tables.update` |
| Tables | Delete | `tables.delete` |
| Users | Read | `users.read` |
| Users | Create | `users.create` |
| Users | Update | `users.update` |
| Users | Delete | `users.delete` |
| Config | Read | `config.read` |
| Config | Update | `config.update` |
| Cluster | Manage | `cluster.manage` |
| Backup | Create | `backup.create` |
| Backup | Restore | `backup.restore` |

**Admin Role**: Has wildcard permission `*` granting all access.

---

## Common Patterns

### Request Headers

**Required Headers**:
```http
Authorization: Bearer <token>
Content-Type: application/json
Accept: application/json
```

**Optional Headers**:
```http
X-Request-ID: <uuid>           # For request tracing
X-Request-Time: <iso8601>      # Timestamp
Accept-Encoding: gzip          # Enable compression
```

### Response Format

All responses follow this structure:

**Success Response**:
```json
{
  "success": true,
  "data": { ... },
  "meta": {
    "requestId": "req_12345",
    "timestamp": "2025-12-11T10:00:00Z",
    "duration": 123
  }
}
```

**Error Response**:
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
    "timestamp": "2025-12-11T10:00:00Z"
  }
}
```

### Pagination

For endpoints returning lists, use these query parameters:

```http
GET /api/v1/tables?page=1&page_size=50&sort_by=name&sort_order=asc
```

**Parameters**:
- `page` (integer, default: 1): Page number (1-indexed)
- `page_size` (integer, default: 50, max: 1000): Items per page
- `sort_by` (string): Field to sort by
- `sort_order` (enum: `asc` | `desc`, default: `asc`): Sort direction

**Paginated Response**:
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

### Filtering

Use query parameters for filtering:

```http
GET /api/v1/sessions?status=active&username=john
```

---

## Error Handling

### HTTP Status Codes

| Code | Meaning | Description |
|------|---------|-------------|
| 200 | OK | Request succeeded |
| 201 | Created | Resource created successfully |
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

### Error Response Examples

**Validation Error**:
```json
{
  "success": false,
  "error": {
    "code": "VALIDATION_ERROR",
    "message": "Validation failed",
    "details": {
      "fields": {
        "email": "Invalid email format",
        "age": "Must be a positive integer"
      }
    }
  }
}
```

**Permission Denied**:
```json
{
  "success": false,
  "error": {
    "code": "PERMISSION_DENIED",
    "message": "Insufficient permissions to perform this action",
    "details": {
      "required": "tables.delete",
      "user_permissions": ["tables.read", "tables.create"]
    }
  }
}
```

---

## REST API Endpoints

### Core Database Operations

#### Execute Query

Execute a SQL query and return results.

```http
POST /api/v1/query
Authorization: Bearer <token>
Content-Type: application/json

{
  "sql": "SELECT * FROM users WHERE age > 18",
  "params": [],
  "options": {
    "timeout": 30000,
    "max_rows": 10000
  }
}
```

**Response**:
```json
{
  "success": true,
  "data": {
    "columns": ["id", "name", "email", "age"],
    "rows": [
      ["1", "Alice", "alice@example.com", "25"],
      ["2", "Bob", "bob@example.com", "30"]
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

#### Execute Batch Queries

Execute multiple queries in a single request.

```http
POST /api/v1/batch
Authorization: Bearer <token>
Content-Type: application/json

{
  "queries": [
    { "sql": "INSERT INTO users (name, email) VALUES ('Alice', 'alice@example.com')" },
    { "sql": "INSERT INTO users (name, email) VALUES ('Bob', 'bob@example.com')" },
    { "sql": "SELECT * FROM users" }
  ],
  "transaction": true
}
```

**Response**:
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
          ["1", "Alice", "alice@example.com"],
          ["2", "Bob", "bob@example.com"]
        ]
      }
    ],
    "total_time_ms": 150
  }
}
```

#### Explain Query

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

**Response**:
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

### Schema Management

#### List Tables

```http
GET /api/v1/tables?page=1&page_size=50
Authorization: Bearer <token>
```

**Response**:
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

#### Get Table Details

```http
GET /api/v1/tables/{table_name}
Authorization: Bearer <token>
```

**Response**:
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
      "last_vacuum": "2025-12-10T00:00:00Z",
      "last_analyze": "2025-12-10T00:00:00Z"
    }
  }
}
```

#### Create Table

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

**Response**:
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

#### Alter Table

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

**Response**:
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

#### Drop Table

```http
DELETE /api/v1/tables/{table_name}?cascade=true
Authorization: Bearer <token>
```

**Response**:
```json
{
  "success": true,
  "data": {
    "dropped": true,
    "cascaded": ["dependent_view1", "foreign_key_constraint"]
  }
}
```

#### Get Schema

Get complete database schema.

```http
GET /api/v1/schema
Authorization: Bearer <token>
```

**Response**:
```json
{
  "success": true,
  "data": {
    "schemas": [
      {
        "name": "public",
        "tables": ["users", "orders", "products"],
        "views": ["active_users"],
        "sequences": ["users_id_seq"]
      }
    ],
    "version": "1.0.0"
  }
}
```

---

### Transaction Management

#### Begin Transaction

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

**Response**:
```json
{
  "success": true,
  "data": {
    "transaction_id": "txn_12345",
    "isolation_level": "REPEATABLE_READ",
    "started_at": "2025-12-11T10:00:00Z"
  }
}
```

#### Commit Transaction

```http
POST /api/v1/transactions/{transaction_id}/commit
Authorization: Bearer <token>
```

**Response**:
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

#### Rollback Transaction

```http
POST /api/v1/transactions/{transaction_id}/rollback
Authorization: Bearer <token>
```

**Response**:
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

### Administration

#### Get Configuration

```http
GET /api/v1/admin/config
Authorization: Bearer <token>
```

**Response**:
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

#### Update Configuration

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

**Response**:
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

#### Create Backup

```http
POST /api/v1/admin/backup
Authorization: Bearer <token>
Content-Type: application/json

{
  "type": "FULL",
  "compression": true,
  "destination": "/backups/backup_20251211.tar.gz",
  "include_wal": true
}
```

**Backup Types**:
- `FULL`: Full database backup
- `INCREMENTAL`: Incremental backup since last backup
- `DIFFERENTIAL`: Differential backup since last full backup

**Response**:
```json
{
  "success": true,
  "data": {
    "backup_id": "backup_12345",
    "type": "FULL",
    "size_bytes": 1073741824,
    "started_at": "2025-12-11T10:00:00Z",
    "completed_at": "2025-12-11T10:15:00Z",
    "destination": "/backups/backup_20251211.tar.gz"
  }
}
```

#### Restore Backup

```http
POST /api/v1/admin/restore
Authorization: Bearer <token>
Content-Type: application/json

{
  "backup_id": "backup_12345",
  "point_in_time": "2025-12-11T09:00:00Z",
  "target_database": "rustydb_restore"
}
```

**Response**:
```json
{
  "success": true,
  "data": {
    "restore_id": "restore_12345",
    "status": "IN_PROGRESS",
    "estimated_completion": "2025-12-11T10:30:00Z"
  }
}
```

#### Health Check

```http
GET /api/v1/admin/health
Authorization: Bearer <token>
```

**Response**:
```json
{
  "success": true,
  "data": {
    "status": "HEALTHY",
    "components": {
      "database": {
        "status": "UP",
        "uptime_sec": 86400,
        "version": "1.0.0"
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
    "timestamp": "2025-12-11T10:00:00Z"
  }
}
```

#### Run Maintenance

```http
POST /api/v1/admin/maintenance
Authorization: Bearer <token>
Content-Type: application/json

{
  "operations": ["VACUUM", "ANALYZE", "REINDEX"],
  "tables": ["users", "orders"],
  "full": false
}
```

**Response**:
```json
{
  "success": true,
  "data": {
    "maintenance_id": "maint_12345",
    "operations": ["VACUUM", "ANALYZE", "REINDEX"],
    "status": "IN_PROGRESS",
    "started_at": "2025-12-11T10:00:00Z"
  }
}
```

#### User Management

**List Users**:
```http
GET /api/v1/admin/users?page=1&page_size=50
Authorization: Bearer <token>
```

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

#### Role Management

**List Roles**:
```http
GET /api/v1/admin/roles
Authorization: Bearer <token>
```

**Create Role**:
```http
POST /api/v1/admin/roles
Authorization: Bearer <token>
Content-Type: application/json

{
  "name": "analyst",
  "permissions": [
    "tables.read",
    "query.execute",
    "stats.read"
  ]
}
```

**Get Role**:
```http
GET /api/v1/admin/roles/{role_id}
Authorization: Bearer <token>
```

**Update Role**:
```http
PUT /api/v1/admin/roles/{role_id}
Authorization: Bearer <token>
Content-Type: application/json

{
  "permissions": [
    "tables.read",
    "tables.create",
    "query.execute"
  ]
}
```

**Delete Role**:
```http
DELETE /api/v1/admin/roles/{role_id}
Authorization: Bearer <token>
```

---

### Monitoring & Metrics

#### Get Metrics

```http
GET /api/v1/metrics
Authorization: Bearer <token>
```

**Response**:
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

#### Get Prometheus Metrics

```http
GET /api/v1/metrics/prometheus
```

**Response** (Prometheus text format):
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

#### Get Session Statistics

```http
GET /api/v1/stats/sessions
Authorization: Bearer <token>
```

**Response**:
```json
{
  "success": true,
  "data": {
    "total_sessions": 100,
    "active_sessions": 45,
    "idle_sessions": 50,
    "sessions_by_state": {
      "active": 45,
      "idle": 50,
      "idle_in_transaction": 3,
      "waiting": 2
    },
    "avg_session_duration_sec": 1234
  }
}
```

#### Get Query Statistics

```http
GET /api/v1/stats/queries?limit=100
Authorization: Bearer <token>
```

**Response**:
```json
{
  "success": true,
  "data": {
    "slow_queries": [
      {
        "query_id": "query_12345",
        "sql": "SELECT * FROM large_table WHERE ...",
        "avg_time_ms": 5678.90,
        "calls": 1000,
        "total_time_ms": 5678900
      }
    ],
    "top_queries": [
      {
        "query_id": "query_67890",
        "sql": "SELECT id, name FROM users WHERE id = $1",
        "calls": 1000000,
        "avg_time_ms": 1.23
      }
    ]
  }
}
```

#### Get Performance Data

```http
GET /api/v1/stats/performance?start=2025-12-11T00:00:00Z&end=2025-12-11T23:59:59Z
Authorization: Bearer <token>
```

**Response**:
```json
{
  "success": true,
  "data": {
    "time_series": [
      {
        "timestamp": "2025-12-11T10:00:00Z",
        "queries_per_second": 5678.90,
        "transactions_per_second": 1234.56,
        "buffer_pool_hit_ratio": 0.98,
        "cpu_usage_percent": 45.6,
        "memory_usage_percent": 67.8
      }
    ]
  }
}
```

#### Get Logs

```http
GET /api/v1/logs?level=ERROR&start=2025-12-11T00:00:00Z&limit=1000
Authorization: Bearer <token>
```

**Response**:
```json
{
  "success": true,
  "data": {
    "logs": [
      {
        "timestamp": "2025-12-11T10:00:00Z",
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

#### Get Alerts

```http
GET /api/v1/alerts?status=active
Authorization: Bearer <token>
```

**Response**:
```json
{
  "success": true,
  "data": {
    "alerts": [
      {
        "id": "alert_12345",
        "severity": "WARNING",
        "title": "High buffer pool usage",
        "message": "Buffer pool usage is at 95%",
        "status": "active",
        "created_at": "2025-12-11T10:00:00Z",
        "acknowledged_at": null
      }
    ]
  }
}
```

#### Acknowledge Alert

```http
POST /api/v1/alerts/{alert_id}/acknowledge
Authorization: Bearer <token>
Content-Type: application/json

{
  "note": "Acknowledged, investigating"
}
```

**Response**:
```json
{
  "success": true,
  "data": {
    "alert_id": "alert_12345",
    "acknowledged": true,
    "acknowledged_by": "admin",
    "acknowledged_at": "2025-12-11T10:05:00Z"
  }
}
```

---

### Pool & Connection Management

#### List Connection Pools

```http
GET /api/v1/pools
Authorization: Bearer <token>
```

**Response**:
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

#### Get Pool Details

```http
GET /api/v1/pools/{pool_id}
Authorization: Bearer <token>
```

**Response**:
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

#### Update Pool Configuration

```http
PUT /api/v1/pools/{pool_id}
Authorization: Bearer <token>
Content-Type: application/json

{
  "min_connections": 20,
  "max_connections": 200,
  "idle_timeout_sec": 300
}
```

**Response**:
```json
{
  "success": true,
  "data": {
    "pool_id": "pool_default",
    "updated": true,
    "requires_restart": false
  }
}
```

#### Get Pool Statistics

```http
GET /api/v1/pools/{pool_id}/stats
Authorization: Bearer <token>
```

**Response**:
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

#### Drain Pool

Gracefully drain a connection pool.

```http
POST /api/v1/pools/{pool_id}/drain
Authorization: Bearer <token>
Content-Type: application/json

{
  "timeout_sec": 60,
  "force": false
}
```

**Response**:
```json
{
  "success": true,
  "data": {
    "pool_id": "pool_default",
    "drained": true,
    "connections_closed": 95,
    "duration_ms": 5678
  }
}
```

#### List Connections

```http
GET /api/v1/connections?status=active
Authorization: Bearer <token>
```

**Response**:
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
      "connected_at": "2025-12-11T09:00:00Z",
      "last_activity": "2025-12-11T10:00:00Z"
    }
  ]
}
```

#### Get Connection Details

```http
GET /api/v1/connections/{connection_id}
Authorization: Bearer <token>
```

**Response**:
```json
{
  "success": true,
  "data": {
    "id": "conn_12345",
    "user": "admin",
    "database": "rustydb",
    "client_addr": "192.168.1.100:54321",
    "state": "active",
    "current_query": "SELECT * FROM users",
    "queries_executed": 1234,
    "transactions": 567,
    "connected_at": "2025-12-11T09:00:00Z",
    "last_activity": "2025-12-11T10:00:00Z"
  }
}
```

#### Kill Connection

```http
DELETE /api/v1/connections/{connection_id}?force=true
Authorization: Bearer <token>
```

**Response**:
```json
{
  "success": true,
  "data": {
    "connection_id": "conn_12345",
    "killed": true
  }
}
```

#### List Sessions

```http
GET /api/v1/sessions?status=active
Authorization: Bearer <token>
```

**Response**:
```json
{
  "success": true,
  "data": [
    {
      "id": "session_12345",
      "user": "admin",
      "client_addr": "192.168.1.100",
      "state": "active",
      "current_query": "SELECT * FROM orders",
      "started_at": "2025-12-11T09:00:00Z",
      "queries_count": 100
    }
  ]
}
```

#### Get Session Details

```http
GET /api/v1/sessions/{session_id}
Authorization: Bearer <token>
```

**Response**:
```json
{
  "success": true,
  "data": {
    "id": "session_12345",
    "user": "admin",
    "database": "rustydb",
    "client_addr": "192.168.1.100",
    "state": "active",
    "transaction_id": "txn_67890",
    "isolation_level": "READ_COMMITTED",
    "variables": {
      "timezone": "UTC",
      "date_format": "YYYY-MM-DD"
    },
    "statistics": {
      "queries_executed": 1234,
      "transactions_committed": 567,
      "transactions_rolled_back": 12,
      "bytes_sent": 1048576,
      "bytes_received": 524288
    }
  }
}
```

#### Terminate Session

```http
DELETE /api/v1/sessions/{session_id}
Authorization: Bearer <token>
```

**Response**:
```json
{
  "success": true,
  "data": {
    "session_id": "session_12345",
    "terminated": true
  }
}
```

---

### Cluster Management

#### List Cluster Nodes

```http
GET /api/v1/cluster/nodes
Authorization: Bearer <token>
```

**Response**:
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
      "version": "1.0.0",
      "uptime_sec": 86400,
      "lag_bytes": 0
    },
    {
      "id": "node_2",
      "name": "rustydb-node-2",
      "role": "REPLICA",
      "status": "UP",
      "address": "192.168.1.11:5432",
      "version": "1.0.0",
      "uptime_sec": 86400,
      "lag_bytes": 1024
    }
  ]
}
```

#### Add Cluster Node

```http
POST /api/v1/cluster/nodes
Authorization: Bearer <token>
Content-Type: application/json

{
  "name": "rustydb-node-3",
  "address": "192.168.1.12:5432",
  "role": "REPLICA",
  "replication_mode": "SYNCHRONOUS"
}
```

**Response**:
```json
{
  "success": true,
  "data": {
    "node_id": "node_3",
    "name": "rustydb-node-3",
    "status": "JOINING",
    "estimated_sync_time_sec": 600
  }
}
```

#### Get Node Details

```http
GET /api/v1/cluster/nodes/{node_id}
Authorization: Bearer <token>
```

**Response**:
```json
{
  "success": true,
  "data": {
    "id": "node_1",
    "name": "rustydb-node-1",
    "role": "PRIMARY",
    "status": "UP",
    "address": "192.168.1.10:5432",
    "version": "1.0.0",
    "uptime_sec": 86400,
    "resources": {
      "cpu_usage_percent": 45.6,
      "memory_usage_percent": 67.8,
      "disk_usage_percent": 34.5
    },
    "statistics": {
      "transactions_per_second": 1234.56,
      "queries_per_second": 5678.90,
      "replication_lag_bytes": 0
    }
  }
}
```

#### Remove Node

```http
DELETE /api/v1/cluster/nodes/{node_id}?force=false
Authorization: Bearer <token>
```

**Response**:
```json
{
  "success": true,
  "data": {
    "node_id": "node_3",
    "removed": true,
    "data_redistributed": true
  }
}
```

#### Get Cluster Topology

```http
GET /api/v1/cluster/topology
Authorization: Bearer <token>
```

**Response**:
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

#### Trigger Failover

```http
POST /api/v1/cluster/failover
Authorization: Bearer <token>
Content-Type: application/json

{
  "target_node_id": "node_2",
  "force": false
}
```

**Response**:
```json
{
  "success": true,
  "data": {
    "failover_id": "failover_12345",
    "old_primary": "node_1",
    "new_primary": "node_2",
    "status": "IN_PROGRESS",
    "estimated_completion": "2025-12-11T10:05:00Z"
  }
}
```

#### Get Replication Status

```http
GET /api/v1/cluster/replication
Authorization: Bearer <token>
```

**Response**:
```json
{
  "success": true,
  "data": {
    "primary_node": "node_1",
    "replicas": [
      {
        "node_id": "node_2",
        "replication_mode": "SYNCHRONOUS",
        "lag_bytes": 1024,
        "lag_sec": 0.5,
        "status": "STREAMING",
        "last_wal_received": "2025-12-11T10:00:00Z"
      }
    ],
    "total_lag_bytes": 1024
  }
}
```

#### Get/Update Cluster Config

```http
GET /api/v1/cluster/config
PUT /api/v1/cluster/config
Authorization: Bearer <token>
```

---

### Security Management

#### List Encryption Keys

```http
GET /api/v1/security/keys
Authorization: Bearer <token>
```

**Response**:
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

#### Rotate Encryption Key

```http
POST /api/v1/security/keys/{key_id}/rotate
Authorization: Bearer <token>
```

**Response**:
```json
{
  "success": true,
  "data": {
    "key_id": "key_master",
    "old_version": 5,
    "new_version": 6,
    "rotation_started": "2025-12-11T10:00:00Z"
  }
}
```

#### List Data Masking Policies

```http
GET /api/v1/security/masking
Authorization: Bearer <token>
```

**Response**:
```json
{
  "success": true,
  "data": [
    {
      "id": "policy_email",
      "table": "users",
      "column": "email",
      "masking_type": "EMAIL",
      "enabled": true
    }
  ]
}
```

#### Create Masking Policy

```http
POST /api/v1/security/masking
Authorization: Bearer <token>
Content-Type: application/json

{
  "table": "users",
  "column": "ssn",
  "masking_type": "PARTIAL",
  "pattern": "XXX-XX-####"
}
```

**Response**:
```json
{
  "success": true,
  "data": {
    "policy_id": "policy_ssn",
    "created": true
  }
}
```

#### Get Audit Logs

```http
GET /api/v1/security/audit?start=2025-12-11T00:00:00Z&limit=1000
Authorization: Bearer <token>
```

**Response**:
```json
{
  "success": true,
  "data": {
    "logs": [
      {
        "id": "audit_12345",
        "timestamp": "2025-12-11T10:00:00Z",
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

## GraphQL API

### Endpoint

```
POST /graphql
GET /graphql (GraphQL Playground)
```

**Base URL**: `http://localhost:8080/graphql` (default)

### Authentication

Authentication for GraphQL API is currently **not required** in development mode. Production deployments should enable authentication via configuration.

```http
POST /graphql
Content-Type: application/json
```

### Schema

The following schema has been **verified through comprehensive testing** (101 tests, 69.3% pass rate). See `TRANSACTION_TEST_RESULTS.md` for detailed test results.

#### Mutation Types (Tested & Working)

```graphql
type Mutation {
  # Transaction Management (✅ Tested - 100% working)
  beginTransaction(
    isolationLevel: IsolationLevel
  ): TransactionResponse!

  commitTransaction(
    transactionId: String!
  ): TransactionStatusResponse!

  rollbackTransaction(
    transactionId: String!
  ): TransactionStatusResponse!

  # Atomic Transaction Execution (✅ Tested - 68% pass rate)
  executeTransaction(
    operations: [TransactionOperation!]!
    isolationLevel: IsolationLevel
  ): ExecuteTransactionResponse!
}

# Types
type TransactionResponse {
  transactionId: String!
  status: TransactionStatus!
  timestamp: String!
  isolationLevel: IsolationLevel
}

type TransactionStatusResponse {
  success: Boolean!
  transactionId: String!
  error: String
}

type ExecuteTransactionResponse {
  success: Boolean!
  transactionId: String
  executionTimeMs: Float
  error: String
}

# Enums
enum IsolationLevel {
  READ_UNCOMMITTED
  READ_COMMITTED      # Default
  REPEATABLE_READ
  SERIALIZABLE
}

enum TransactionStatus {
  ACTIVE
  COMMITTED
  ABORTED
}

enum TransactionOpType {
  INSERT
  UPDATE
  DELETE
  SELECT
}

# Input Types
input TransactionOperation {
  operationType: TransactionOpType!
  table: String!
  data: JSON
  where: JSON
}
```

#### Query Types (Schema - Implementation Status Unknown)

```graphql
type Query {
  # Note: These queries are part of the schema but have not been
  # verified through automated testing. Implementation status unknown.

  # Schema introspection
  schema: DatabaseSchema
  table(name: String!): TableType
  tables(schema: String): [TableType!]

  # Data queries
  query(
    table: String!
    where: WhereClause
    orderBy: [OrderBy!]
    limit: Int
    offset: Int
  ): QueryResult

  # Statistics
  tableStatistics(table: String!): TableStatistics
  queryPlan(sql: String!): QueryPlan
}
```

**⚠️ Implementation Note**: Query types listed above are part of the GraphQL schema but have not been tested in the comprehensive test suite. Availability and exact behavior should be verified before use in production.

#### Subscription Types

**⚠️ Status**: WebSocket subscriptions referenced in older documentation but not verified in current test suite. Implementation status unknown.

```graphql
type Subscription {
  # Note: Subscription support is documented but not confirmed through testing
  # Verify availability before implementing in production applications

  # Real-time data changes
  tableChanges(
    table: String!
    operations: [ChangeType!]
  ): TableChange

  # Metrics updates
  metrics: MetricsUpdate

  # Heartbeat
  heartbeat: Heartbeat
}
```

### Example Mutations (Tested & Verified)

#### Begin Transaction

**✅ Tested**: 100% success rate across all isolation levels

```graphql
mutation BeginTransaction {
  beginTransaction(isolationLevel: SERIALIZABLE) {
    transactionId
    status
    timestamp
    isolationLevel
  }
}
```

**Example Response**:
```json
{
  "data": {
    "beginTransaction": {
      "transactionId": "88790068-3f05-42fb-a5f8-126ccedff088",
      "status": "ACTIVE",
      "timestamp": "2025-12-11T15:45:43.903567264+00:00",
      "isolationLevel": "SERIALIZABLE"
    }
  }
}
```

**Supported Isolation Levels**:
- `READ_UNCOMMITTED`: Allows dirty reads
- `READ_COMMITTED`: Default, sees only committed data
- `REPEATABLE_READ`: Consistent snapshot per transaction
- `SERIALIZABLE`: Strictest isolation, full serializability

#### Commit Transaction

```graphql
mutation CommitTransaction {
  commitTransaction(transactionId: "88790068-3f05-42fb-a5f8-126ccedff088") {
    success
    transactionId
    error
  }
}
```

#### Rollback Transaction

```graphql
mutation RollbackTransaction {
  rollbackTransaction(transactionId: "88790068-3f05-42fb-a5f8-126ccedff088") {
    success
    transactionId
    error
  }
}
```

#### Execute Atomic Transaction

**✅ Tested**: 68% pass rate (17/25 tests)

```graphql
mutation ExecuteAtomicTransaction {
  executeTransaction(
    operations: [
      {
        operationType: INSERT
        table: "test_table"
        data: {id: 1, name: "test"}
      }
      {
        operationType: UPDATE
        table: "test_table"
        data: {name: "updated"}
        where: {id: 1}
      }
      {
        operationType: DELETE
        table: "test_table"
        where: {id: 1}
      }
    ]
    isolationLevel: SERIALIZABLE
  ) {
    success
    executionTimeMs
    error
  }
}
```

**Example Response**:
```json
{
  "data": {
    "executeTransaction": {
      "success": true,
      "executionTimeMs": 0.002826,
      "error": null
    }
  }
}
```

**Performance**: Average execution time ~0.002-0.003ms for multi-operation transactions

#### Subscription Example

```graphql
subscription WatchUserChanges {
  tableChanges(
    table: "users"
    operations: [INSERT, UPDATE, DELETE]
  ) {
    operation
    table
    row {
      id
      name
      email
    }
    timestamp
  }
}
```

---

## WebSocket API

### Connection

```javascript
const ws = new WebSocket('ws://localhost:8080/api/v1/stream');

ws.onopen = () => {
  ws.send(JSON.stringify({
    type: 'authenticate',
    token: 'your_jwt_token'
  }));
};
```

### Query Streaming

Stream query results in real-time:

```javascript
ws.send(JSON.stringify({
  type: 'query',
  sql: 'SELECT * FROM large_table',
  stream: true
}));

ws.onmessage = (event) => {
  const data = JSON.parse(event.data);
  if (data.type === 'row') {
    console.log('Row:', data.row);
  } else if (data.type === 'complete') {
    console.log('Query complete');
  }
};
```

### Real-Time Notifications

Subscribe to table changes:

```javascript
ws.send(JSON.stringify({
  type: 'subscribe',
  table: 'users',
  events: ['INSERT', 'UPDATE', 'DELETE']
}));

ws.onmessage = (event) => {
  const data = JSON.parse(event.data);
  console.log('Change:', data);
};
```

---

## Rate Limiting

### Default Limits

- **Global**: 100 requests/second per IP
- **Per Endpoint**:
  - `/query`: 50 requests/second
  - `/batch`: 10 requests/second
  - `/admin/*`: 20 requests/second

### Rate Limit Headers

Response includes rate limit info:

```http
X-RateLimit-Limit: 100
X-RateLimit-Remaining: 45
X-RateLimit-Reset: 1639132800
Retry-After: 60
```

### Rate Limit Exceeded Response

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

### API Versioning Strategy

RustyDB uses URL-based versioning:

- **Current Version**: `/api/v1`
- **Future Versions**: `/api/v2`, `/api/v3`

### Version Compatibility

- **v1**: Stable, backward compatible
- Breaking changes require new major version
- Deprecated endpoints marked in response headers:

```http
X-API-Deprecated: true
X-API-Sunset: 2026-12-31
Link: </api/v2/endpoint>; rel="successor-version"
```

---

**For more information, visit the GraphQL Playground at `/graphql` or OpenAPI documentation at `/swagger-ui`.**
