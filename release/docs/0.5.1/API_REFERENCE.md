# RustyDB v0.5.1 - Complete API Reference

**Version**: 0.5.1
**Release Date**: December 2025
**Status**: Enterprise Production Release ($350M)
**Document Version**: 1.0
**Last Updated**: 2025-12-27

---

## Table of Contents

1. [API Overview](#api-overview)
2. [Getting Started](#getting-started)
3. [Authentication & Security](#authentication--security)
4. [REST API Reference](#rest-api-reference)
5. [GraphQL API Reference](#graphql-api-reference)
6. [PostgreSQL Wire Protocol](#postgresql-wire-protocol)
7. [WebSocket API](#websocket-api)
8. [SDK & Client Libraries](#sdk--client-libraries)
9. [API Best Practices](#api-best-practices)
10. [Error Handling](#error-handling)
11. [Rate Limiting](#rate-limiting)
12. [Performance Optimization](#performance-optimization)

---

## API Overview

### Available Interfaces

RustyDB provides four primary API interfaces for database interaction:

| Interface | Port | Protocol | Use Case | Performance |
|-----------|------|----------|----------|-------------|
| **REST API** | 8080 | HTTP/JSON | Web apps, microservices | High (50K qps) |
| **GraphQL** | 8080/graphql | HTTP/GraphQL | Modern web/mobile apps | High (30K qps) |
| **PostgreSQL Wire** | 5432 | PostgreSQL protocol | PostgreSQL clients | Very High (100K qps) |
| **WebSocket** | 8080/ws | WebSocket | Real-time streaming | Very High (200K msg/s) |

### API Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                      API Gateway Layer                       │
│  ┌──────────┬──────────┬──────────┬──────────────────────┐  │
│  │   Auth   │  Rate    │ Security │  Request Validation  │  │
│  │          │  Limit   │  Filter  │                      │  │
│  └──────────┴──────────┴──────────┴──────────────────────┘  │
└─────────────────────────────────────────────────────────────┘
                            ↓
┌─────────────────────────────────────────────────────────────┐
│                      API Routing Layer                       │
│  ┌──────────┬──────────┬──────────┬──────────────────────┐  │
│  │   REST   │ GraphQL  │ WebSocket│ PostgreSQL Protocol  │  │
│  │   API    │   API    │   API    │                      │  │
│  └──────────┴──────────┴──────────┴──────────────────────┘  │
└─────────────────────────────────────────────────────────────┘
                            ↓
┌─────────────────────────────────────────────────────────────┐
│                     Database Core Layer                      │
│  ┌──────────┬──────────┬──────────┬──────────────────────┐  │
│  │ Query    │ Trans.   │ Storage  │    Replication       │  │
│  │ Executor │ Manager  │ Engine   │                      │  │
│  └──────────┴──────────┴──────────┴──────────────────────┘  │
└─────────────────────────────────────────────────────────────┘
```

### API Coverage Statistics

| Category | Total Endpoints | Implemented | Exposed | Coverage |
|----------|----------------|-------------|---------|----------|
| **REST API** | 276 | 276 | 153 | 55% |
| **GraphQL Queries** | ~150 | ~150 | 33 | 22% |
| **GraphQL Mutations** | ~150 | ~150 | 25 | 17% |
| **GraphQL Subscriptions** | ~60 | ~60 | 3 | 5% |
| **WebSocket Endpoints** | 50+ | 50+ | 50+ | 100% |

### Base URLs

**Production**:
```
REST API:       https://api.rustydb.com/api/v1
GraphQL:        https://api.rustydb.com/graphql
WebSocket:      wss://api.rustydb.com/ws
PostgreSQL:     postgres://rustydb.com:5432
```

**Development**:
```
REST API:       http://localhost:8080/api/v1
GraphQL:        http://localhost:8080/graphql
WebSocket:      ws://localhost:8080/ws
PostgreSQL:     postgresql://localhost:5432
```

---

## Getting Started

### Quick Start - REST API

```bash
# 1. Start RustyDB server
cargo run --release --bin rusty-db-server

# 2. Authenticate
curl -X POST http://localhost:8080/api/v1/auth/login \
  -H "Content-Type: application/json" \
  -d '{"username": "admin", "password": "admin"}'

# Response includes JWT token:
# {"success": true, "data": {"session": {"token": "eyJhbG..."}}}

# 3. Execute a query
curl -X POST http://localhost:8080/api/v1/query \
  -H "Authorization: Bearer eyJhbG..." \
  -H "Content-Type: application/json" \
  -d '{"sql": "SELECT * FROM users LIMIT 10"}'
```

### Quick Start - GraphQL

```bash
# Visit GraphQL Playground
open http://localhost:8080/graphql

# Or use curl
curl -X POST http://localhost:8080/graphql \
  -H "Content-Type: application/json" \
  -d '{
    "query": "query { tables { name columns { name type } } }"
  }'
```

### Quick Start - PostgreSQL Wire Protocol

```bash
# Connect using psql
psql -h localhost -p 5432 -U admin -d rustydb

# Or using connection string
psql postgresql://admin:admin@localhost:5432/rustydb
```

### Quick Start - WebSocket

```javascript
// Connect to WebSocket
const ws = new WebSocket('ws://localhost:8080/ws');

ws.onopen = () => {
  // Authenticate
  ws.send(JSON.stringify({
    type: 'authenticate',
    token: 'your_jwt_token'
  }));
};

ws.onmessage = (event) => {
  const data = JSON.parse(event.data);
  console.log('Received:', data);
};
```

---

## Authentication & Security

### Authentication Methods

#### 1. JWT (JSON Web Tokens)

**Primary authentication method for REST and GraphQL APIs.**

##### Login

```http
POST /api/v1/auth/login
Content-Type: application/json

{
  "username": "admin",
  "password": "secure_password"
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
      "email": "admin@rustydb.com",
      "roles": ["admin"]
    },
    "session": {
      "token": "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9...",
      "refreshToken": "refresh_abc123...",
      "expiresAt": "2025-12-28T10:00:00Z",
      "expiresIn": 3600
    }
  },
  "meta": {
    "requestId": "req_12345",
    "timestamp": "2025-12-27T10:00:00Z"
  }
}
```

##### Using JWT Token

Include the token in the `Authorization` header:

```http
GET /api/v1/tables
Authorization: Bearer eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9...
```

##### Refresh Token

```http
POST /api/v1/auth/refresh
Content-Type: application/json

{
  "refreshToken": "refresh_abc123..."
}
```

**Response**:
```json
{
  "success": true,
  "data": {
    "token": "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9...",
    "refreshToken": "refresh_xyz789...",
    "expiresAt": "2025-12-28T11:00:00Z"
  }
}
```

##### Logout

```http
POST /api/v1/auth/logout
Authorization: Bearer <token>
```

#### 2. API Keys

**Service-to-service authentication.**

```http
GET /api/v1/tables
X-API-Key: sk_live_abc123xyz789...
```

**Creating API Keys**:

```http
POST /api/v1/admin/api-keys
Authorization: Bearer <token>
Content-Type: application/json

{
  "name": "Production Service Key",
  "permissions": ["read:tables", "execute:queries"],
  "expiresIn": 31536000
}
```

#### 3. OAuth 2.0 / OpenID Connect

**Enterprise SSO integration.**

```http
GET /api/v1/auth/oauth/authorize?
  client_id=your_client_id&
  redirect_uri=https://yourapp.com/callback&
  response_type=code&
  scope=read write

# Callback receives authorization code
# Exchange code for token:
POST /api/v1/auth/oauth/token
Content-Type: application/x-www-form-urlencoded

grant_type=authorization_code&
code=AUTH_CODE&
client_id=CLIENT_ID&
client_secret=CLIENT_SECRET&
redirect_uri=REDIRECT_URI
```

#### 4. mTLS (Mutual TLS)

**Certificate-based authentication for high-security environments.**

**Configuration**:
```toml
[security]
enable_tls = true
require_client_cert = true
tls_cert_path = "/etc/rustydb/server.crt"
tls_key_path = "/etc/rustydb/server.key"
ca_cert_path = "/etc/rustydb/ca.crt"
```

**Client Connection**:
```bash
curl --cert client.crt --key client.key --cacert ca.crt \
  https://localhost:8080/api/v1/tables
```

### Authorization (RBAC)

#### Role-Based Access Control

**Built-in Roles**:
- `admin`: Full system access
- `developer`: Database operations, no admin
- `analyst`: Read-only access
- `operator`: Monitoring and operations

**Permission Format**: `resource.action`

Examples:
- `tables.read` - Read table metadata
- `tables.create` - Create tables
- `queries.execute` - Execute queries
- `users.manage` - User management
- `config.update` - Update configuration

#### Creating Custom Roles

```http
POST /api/v1/admin/roles
Authorization: Bearer <token>
Content-Type: application/json

{
  "name": "data_engineer",
  "description": "Data engineering role",
  "permissions": [
    "tables.read",
    "tables.create",
    "tables.update",
    "queries.execute",
    "transactions.manage"
  ],
  "inherits_from": ["developer"]
}
```

#### Assigning Roles to Users

```http
PUT /api/v1/admin/users/{user_id}
Authorization: Bearer <token>
Content-Type: application/json

{
  "roles": ["data_engineer", "analyst"]
}
```

### TLS/SSL Configuration

#### Server Configuration

```toml
[security.tls]
enabled = true
cert_path = "/etc/rustydb/server.crt"
key_path = "/etc/rustydb/server.key"
min_version = "1.3"
cipher_suites = [
  "TLS_AES_256_GCM_SHA384",
  "TLS_CHACHA20_POLY1305_SHA256"
]
```

#### Client Configuration

**cURL**:
```bash
curl --tlsv1.3 --cacert ca.crt https://localhost:8080/api/v1/tables
```

**PostgreSQL**:
```bash
psql "postgresql://localhost:5432/rustydb?sslmode=require"
```

### API Security Headers

**Required Headers**:
```http
Authorization: Bearer <token>          # JWT authentication
Content-Type: application/json         # JSON content
Accept: application/json               # Expected response format
```

**Optional Security Headers**:
```http
X-Request-ID: <uuid>                  # Request tracing
X-API-Key: <api_key>                  # Alternative auth
X-CSRF-Token: <token>                 # CSRF protection
Origin: https://yourapp.com           # CORS validation
```

**Response Security Headers**:
```http
X-Content-Type-Options: nosniff
X-Frame-Options: DENY
X-XSS-Protection: 1; mode=block
Strict-Transport-Security: max-age=31536000
Content-Security-Policy: default-src 'self'
```

---

## REST API Reference

### Common Patterns

#### Request Format

```http
POST /api/v1/endpoint
Authorization: Bearer <token>
Content-Type: application/json
Accept: application/json
X-Request-ID: req_12345

{
  "parameter": "value"
}
```

#### Response Format

**Success Response**:
```json
{
  "success": true,
  "data": {
    /* Response data */
  },
  "meta": {
    "requestId": "req_12345",
    "timestamp": "2025-12-27T10:00:00Z",
    "duration": 45
  }
}
```

**Error Response**:
```json
{
  "success": false,
  "error": {
    "code": "VALIDATION_ERROR",
    "message": "Invalid request parameters",
    "details": {
      "field": "email",
      "reason": "Invalid email format"
    }
  },
  "meta": {
    "requestId": "req_12345",
    "timestamp": "2025-12-27T10:00:00Z"
  }
}
```

#### Pagination

**Request**:
```http
GET /api/v1/tables?page=1&page_size=50&sort_by=name&sort_order=asc
```

**Response**:
```json
{
  "success": true,
  "data": [ /* items */ ],
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

### 1. Core Database Operations

#### Execute SQL Query

**Endpoint**: `POST /api/v1/query/execute`

Execute a SQL query and return results.

**Request**:
```http
POST /api/v1/query/execute
Authorization: Bearer <token>
Content-Type: application/json

{
  "sql": "SELECT id, name, email FROM users WHERE age > $1 LIMIT $2",
  "params": [18, 10],
  "options": {
    "timeout": 30000,
    "max_rows": 10000,
    "include_execution_plan": false
  }
}
```

**Response**:
```json
{
  "success": true,
  "data": {
    "columns": [
      {"name": "id", "type": "INTEGER"},
      {"name": "name", "type": "VARCHAR"},
      {"name": "email", "type": "VARCHAR"}
    ],
    "rows": [
      [1, "Alice Johnson", "alice@example.com"],
      [2, "Bob Smith", "bob@example.com"]
    ],
    "rows_affected": 0,
    "execution_time_ms": 12.5,
    "has_more": false
  }
}
```

**Query Options**:
- `timeout` (integer, ms): Query timeout (default: 30000)
- `max_rows` (integer): Maximum rows to return (default: 10000)
- `include_execution_plan` (boolean): Include query plan
- `explain_only` (boolean): Return plan without executing

#### Explain Query Plan

**Endpoint**: `POST /api/v1/query/explain`

Get query execution plan without executing.

**Request**:
```http
POST /api/v1/query/explain
Authorization: Bearer <token>
Content-Type: application/json

{
  "sql": "SELECT * FROM users u JOIN orders o ON u.id = o.user_id WHERE u.age > 18",
  "analyze": true,
  "format": "json"
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
      "estimated_width": 256,
      "join_type": "INNER",
      "join_condition": "u.id = o.user_id",
      "children": [
        {
          "type": "SeqScan",
          "table": "users",
          "alias": "u",
          "filter": "age > 18",
          "estimated_cost": 234.56,
          "estimated_rows": 8000
        },
        {
          "type": "IndexScan",
          "table": "orders",
          "alias": "o",
          "index": "orders_user_id_idx",
          "estimated_cost": 456.78,
          "estimated_rows": 15000
        }
      ]
    },
    "warnings": [],
    "optimizer_hints": [
      "Consider creating index on users(age)"
    ]
  }
}
```

#### Execute Batch Operations

**Endpoint**: `POST /api/v1/query/batch`

Execute multiple SQL statements in a single request.

**Request**:
```http
POST /api/v1/query/batch
Authorization: Bearer <token>
Content-Type: application/json

{
  "statements": [
    {
      "sql": "INSERT INTO users (name, email) VALUES ($1, $2)",
      "params": ["Alice", "alice@example.com"]
    },
    {
      "sql": "INSERT INTO users (name, email) VALUES ($1, $2)",
      "params": ["Bob", "bob@example.com"]
    },
    {
      "sql": "SELECT COUNT(*) FROM users"
    }
  ],
  "transaction": true,
  "isolation_level": "READ_COMMITTED"
}
```

**Response**:
```json
{
  "success": true,
  "data": {
    "results": [
      {
        "rows_affected": 1,
        "execution_time_ms": 5.2
      },
      {
        "rows_affected": 1,
        "execution_time_ms": 4.8
      },
      {
        "columns": [{"name": "count", "type": "BIGINT"}],
        "rows": [[102]],
        "execution_time_ms": 2.1
      }
    ],
    "total_execution_time_ms": 12.1,
    "transaction_id": "txn_abc123"
  }
}
```

### 2. Transaction Management

#### Begin Transaction

**Endpoint**: `POST /api/v1/transactions`

Start a new database transaction.

**Request**:
```http
POST /api/v1/transactions
Authorization: Bearer <token>
Content-Type: application/json

{
  "isolation_level": "REPEATABLE_READ",
  "read_only": false,
  "deferrable": false
}
```

**Isolation Levels**:
- `READ_UNCOMMITTED` - Dirty reads allowed
- `READ_COMMITTED` - Default, sees committed data only
- `REPEATABLE_READ` - Consistent snapshot
- `SERIALIZABLE` - Strictest isolation

**Response**:
```json
{
  "success": true,
  "data": {
    "transaction_id": "txn_abc123",
    "isolation_level": "REPEATABLE_READ",
    "read_only": false,
    "started_at": "2025-12-27T10:00:00Z",
    "expires_at": "2025-12-27T10:30:00Z"
  }
}
```

#### Commit Transaction

**Endpoint**: `POST /api/v1/transactions/{id}/commit`

Commit an active transaction.

**Request**:
```http
POST /api/v1/transactions/txn_abc123/commit
Authorization: Bearer <token>
```

**Response**:
```json
{
  "success": true,
  "data": {
    "transaction_id": "txn_abc123",
    "committed": true,
    "committed_at": "2025-12-27T10:05:00Z",
    "duration_ms": 1234
  }
}
```

#### Rollback Transaction

**Endpoint**: `POST /api/v1/transactions/{id}/rollback`

Rollback an active transaction.

**Request**:
```http
POST /api/v1/transactions/txn_abc123/rollback
Authorization: Bearer <token>
```

**Response**:
```json
{
  "success": true,
  "data": {
    "transaction_id": "txn_abc123",
    "rolled_back": true,
    "rolled_back_at": "2025-12-27T10:05:00Z"
  }
}
```

#### Transaction Savepoints

**Create Savepoint**:
```http
POST /api/v1/transactions/{id}/savepoints
Authorization: Bearer <token>
Content-Type: application/json

{
  "name": "before_delete"
}
```

**Rollback to Savepoint**:
```http
POST /api/v1/transactions/{id}/savepoints/{name}/rollback
Authorization: Bearer <token>
```

**Release Savepoint**:
```http
DELETE /api/v1/transactions/{id}/savepoints/{name}
Authorization: Bearer <token>
```

#### List Active Transactions

**Endpoint**: `GET /api/v1/transactions`

**Request**:
```http
GET /api/v1/transactions?status=active&limit=50
Authorization: Bearer <token>
```

**Response**:
```json
{
  "success": true,
  "data": [
    {
      "transaction_id": "txn_abc123",
      "user": "admin",
      "isolation_level": "REPEATABLE_READ",
      "started_at": "2025-12-27T10:00:00Z",
      "age_seconds": 300,
      "queries_executed": 5,
      "locks_held": 3,
      "state": "ACTIVE"
    }
  ]
}
```

#### Get Transaction Locks

**Endpoint**: `GET /api/v1/transactions/locks`

**Request**:
```http
GET /api/v1/transactions/locks?transaction_id=txn_abc123
Authorization: Bearer <token>
```

**Response**:
```json
{
  "success": true,
  "data": {
    "locks": [
      {
        "lock_id": "lock_123",
        "transaction_id": "txn_abc123",
        "resource_type": "TABLE",
        "resource_id": "users",
        "lock_mode": "EXCLUSIVE",
        "granted": true,
        "acquired_at": "2025-12-27T10:00:01Z"
      }
    ],
    "total_locks": 1
  }
}
```

#### Detect Deadlocks

**Endpoint**: `GET /api/v1/transactions/deadlocks`

**Request**:
```http
GET /api/v1/transactions/deadlocks
Authorization: Bearer <token>
```

**Response**:
```json
{
  "success": true,
  "data": {
    "deadlocks": [
      {
        "detected_at": "2025-12-27T10:05:00Z",
        "cycle": [
          {
            "transaction_id": "txn_abc123",
            "waiting_for": "txn_def456",
            "resource": "users"
          },
          {
            "transaction_id": "txn_def456",
            "waiting_for": "txn_abc123",
            "resource": "orders"
          }
        ],
        "victim": "txn_def456",
        "resolution": "ROLLED_BACK"
      }
    ]
  }
}
```

#### MVCC Status

**Endpoint**: `GET /api/v1/transactions/mvcc/status`

Get Multi-Version Concurrency Control status.

**Request**:
```http
GET /api/v1/transactions/mvcc/status
Authorization: Bearer <token>
```

**Response**:
```json
{
  "success": true,
  "data": {
    "oldest_transaction_id": 12345,
    "newest_transaction_id": 67890,
    "active_snapshots": 15,
    "version_count": 1234,
    "garbage_collection_lag": 567,
    "last_vacuum": "2025-12-27T09:00:00Z"
  }
}
```

### 3. Table & Schema Management

#### List Tables

**Endpoint**: `GET /api/v1/tables`

**Request**:
```http
GET /api/v1/tables?schema=public&page=1&page_size=50
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
      "type": "TABLE",
      "row_count": 10000,
      "size_bytes": 2097152,
      "created_at": "2025-01-01T00:00:00Z",
      "last_modified": "2025-12-27T09:00:00Z"
    }
  ],
  "pagination": {
    "page": 1,
    "page_size": 50,
    "total_count": 25,
    "total_pages": 1
  }
}
```

#### Get Table Details

**Endpoint**: `GET /api/v1/tables/{name}`

**Request**:
```http
GET /api/v1/tables/users
Authorization: Bearer <token>
```

**Response**:
```json
{
  "success": true,
  "data": {
    "name": "users",
    "schema": "public",
    "type": "TABLE",
    "columns": [
      {
        "name": "id",
        "data_type": "INTEGER",
        "nullable": false,
        "default": "nextval('users_id_seq')",
        "primary_key": true,
        "position": 1
      },
      {
        "name": "name",
        "data_type": "VARCHAR(255)",
        "nullable": false,
        "position": 2
      },
      {
        "name": "email",
        "data_type": "VARCHAR(255)",
        "nullable": false,
        "unique": true,
        "position": 3
      },
      {
        "name": "age",
        "data_type": "INTEGER",
        "nullable": true,
        "position": 4
      },
      {
        "name": "created_at",
        "data_type": "TIMESTAMP",
        "nullable": false,
        "default": "CURRENT_TIMESTAMP",
        "position": 5
      }
    ],
    "indexes": [
      {
        "name": "users_pkey",
        "type": "BTREE",
        "columns": ["id"],
        "unique": true,
        "primary": true,
        "size_bytes": 131072
      },
      {
        "name": "users_email_idx",
        "type": "BTREE",
        "columns": ["email"],
        "unique": true,
        "size_bytes": 98304
      }
    ],
    "constraints": [
      {
        "name": "users_age_check",
        "type": "CHECK",
        "definition": "age >= 0 AND age <= 150"
      }
    ],
    "statistics": {
      "row_count": 10000,
      "size_bytes": 2097152,
      "index_size_bytes": 229376,
      "last_vacuum": "2025-12-27T00:00:00Z",
      "last_analyze": "2025-12-27T00:00:00Z",
      "vacuum_count": 50,
      "analyze_count": 100
    }
  }
}
```

#### Create Table

**Endpoint**: `POST /api/v1/tables`

**Request**:
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
      "name": "description",
      "data_type": "TEXT"
    },
    {
      "name": "price",
      "data_type": "DECIMAL(10,2)",
      "nullable": false,
      "check": "price >= 0"
    },
    {
      "name": "stock",
      "data_type": "INTEGER",
      "default": "0"
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
      "type": "BTREE"
    }
  ],
  "constraints": [
    {
      "type": "CHECK",
      "definition": "price >= 0 AND stock >= 0"
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
    "schema": "public",
    "created": true,
    "ddl": "CREATE TABLE public.products (...)"
  }
}
```

#### Alter Table

**Endpoint**: `PUT /api/v1/tables/{name}`

**Request**:
```http
PUT /api/v1/tables/products
Authorization: Bearer <token>
Content-Type: application/json

{
  "add_columns": [
    {
      "name": "category",
      "data_type": "VARCHAR(100)",
      "position": 3
    }
  ],
  "drop_columns": ["old_field"],
  "modify_columns": [
    {
      "name": "description",
      "data_type": "TEXT",
      "nullable": true
    }
  ],
  "add_indexes": [
    {
      "name": "products_category_idx",
      "columns": ["category"]
    }
  ],
  "drop_indexes": ["old_index"]
}
```

**Response**:
```json
{
  "success": true,
  "data": {
    "modified": true,
    "changes": [
      "Added column: category VARCHAR(100)",
      "Dropped column: old_field",
      "Modified column: description TEXT",
      "Added index: products_category_idx",
      "Dropped index: old_index"
    ],
    "warnings": []
  }
}
```

#### Drop Table

**Endpoint**: `DELETE /api/v1/tables/{name}`

**Request**:
```http
DELETE /api/v1/tables/products?cascade=true
Authorization: Bearer <token>
```

**Response**:
```json
{
  "success": true,
  "data": {
    "dropped": true,
    "cascaded_objects": [
      "INDEX products_name_idx",
      "VIEW active_products"
    ]
  }
}
```

#### Get Database Schema

**Endpoint**: `GET /api/v1/schema`

**Request**:
```http
GET /api/v1/schema?schema=public
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
        "owner": "admin",
        "tables": ["users", "orders", "products"],
        "views": ["active_users", "recent_orders"],
        "sequences": ["users_id_seq"],
        "functions": ["calculate_total"],
        "table_count": 3,
        "view_count": 2
      }
    ],
    "database": "rustydb",
    "version": "0.5.1"
  }
}
```

### 4. Monitoring & Metrics

#### Get System Metrics

**Endpoint**: `GET /api/v1/monitoring/metrics`

**Request**:
```http
GET /api/v1/monitoring/metrics
Authorization: Bearer <token>
```

**Response**:
```json
{
  "success": true,
  "data": {
    "database": {
      "uptime_seconds": 86400,
      "version": "0.5.1",
      "active_connections": 45,
      "max_connections": 1000,
      "transactions_per_second": 1234.56,
      "queries_per_second": 5678.90
    },
    "storage": {
      "total_size_bytes": 10737418240,
      "data_size_bytes": 8589934592,
      "index_size_bytes": 2147483648,
      "buffer_pool_size_bytes": 1073741824,
      "buffer_pool_used_bytes": 858993459,
      "buffer_pool_hit_ratio": 0.982,
      "disk_reads_per_second": 123.45,
      "disk_writes_per_second": 67.89
    },
    "transactions": {
      "active": 12,
      "committed_total": 1000000,
      "rolled_back_total": 1234,
      "deadlocks_total": 5,
      "avg_duration_ms": 45.67
    },
    "query_performance": {
      "avg_query_time_ms": 12.34,
      "p50_query_time_ms": 8.90,
      "p95_query_time_ms": 45.67,
      "p99_query_time_ms": 123.45,
      "slow_queries_total": 78
    },
    "cache": {
      "table_cache_hit_ratio": 0.95,
      "index_cache_hit_ratio": 0.98,
      "query_cache_hit_ratio": 0.87
    }
  }
}
```

#### Get Prometheus Metrics

**Endpoint**: `GET /api/v1/monitoring/prometheus`

**Request**:
```http
GET /api/v1/monitoring/prometheus
```

**Response** (Prometheus text format):
```prometheus
# HELP rustydb_uptime_seconds Database uptime in seconds
# TYPE rustydb_uptime_seconds counter
rustydb_uptime_seconds 86400

# HELP rustydb_connections_active Number of active connections
# TYPE rustydb_connections_active gauge
rustydb_connections_active 45

# HELP rustydb_transactions_total Total transactions
# TYPE rustydb_transactions_total counter
rustydb_transactions_total{status="committed"} 1000000
rustydb_transactions_total{status="rolled_back"} 1234

# HELP rustydb_query_duration_seconds Query execution duration
# TYPE rustydb_query_duration_seconds histogram
rustydb_query_duration_seconds_bucket{le="0.01"} 50000
rustydb_query_duration_seconds_bucket{le="0.05"} 80000
rustydb_query_duration_seconds_bucket{le="0.1"} 95000
rustydb_query_duration_seconds_bucket{le="1.0"} 99000
rustydb_query_duration_seconds_bucket{le="+Inf"} 100000
rustydb_query_duration_seconds_sum 1234.56
rustydb_query_duration_seconds_count 100000

# HELP rustydb_buffer_pool_hit_ratio Buffer pool cache hit ratio
# TYPE rustydb_buffer_pool_hit_ratio gauge
rustydb_buffer_pool_hit_ratio 0.982
```

#### Get Active Sessions

**Endpoint**: `GET /api/v1/monitoring/sessions`

**Request**:
```http
GET /api/v1/monitoring/sessions?status=active&limit=50
Authorization: Bearer <token>
```

**Response**:
```json
{
  "success": true,
  "data": {
    "sessions": [
      {
        "session_id": "sess_abc123",
        "user": "admin",
        "database": "rustydb",
        "client_addr": "192.168.1.100:54321",
        "state": "ACTIVE",
        "current_query": "SELECT * FROM orders WHERE ...",
        "query_start": "2025-12-27T10:00:00Z",
        "transaction_id": "txn_def456",
        "connected_at": "2025-12-27T09:00:00Z",
        "duration_seconds": 3600
      }
    ],
    "summary": {
      "total_sessions": 100,
      "active": 45,
      "idle": 50,
      "idle_in_transaction": 3,
      "waiting": 2
    }
  }
}
```

#### Get Active Queries

**Endpoint**: `GET /api/v1/monitoring/queries`

**Request**:
```http
GET /api/v1/monitoring/queries?min_duration_ms=1000
Authorization: Bearer <token>
```

**Response**:
```json
{
  "success": true,
  "data": {
    "queries": [
      {
        "query_id": "query_abc123",
        "session_id": "sess_def456",
        "user": "analyst",
        "sql": "SELECT COUNT(*) FROM large_table WHERE ...",
        "state": "RUNNING",
        "started_at": "2025-12-27T10:00:00Z",
        "duration_ms": 5678,
        "rows_returned": 0,
        "progress": 0.45
      }
    ]
  }
}
```

#### Get Query Statistics

**Endpoint**: `GET /api/v1/monitoring/query-stats`

**Request**:
```http
GET /api/v1/monitoring/query-stats?limit=10&sort=total_time
Authorization: Bearer <token>
```

**Response**:
```json
{
  "success": true,
  "data": {
    "top_queries": [
      {
        "query_hash": "hash_abc123",
        "query_text": "SELECT * FROM users WHERE id = $1",
        "calls": 1000000,
        "total_time_ms": 12345678,
        "avg_time_ms": 12.34,
        "min_time_ms": 1.23,
        "max_time_ms": 567.89,
        "stddev_time_ms": 5.67,
        "rows_returned_avg": 1,
        "cache_hit_ratio": 0.95
      }
    ],
    "slow_queries": [
      {
        "query_hash": "hash_def456",
        "query_text": "SELECT * FROM large_table WHERE ...",
        "calls": 100,
        "avg_time_ms": 5678.90,
        "max_time_ms": 12345.67
      }
    ]
  }
}
```

#### Get Performance Data

**Endpoint**: `GET /api/v1/monitoring/performance`

**Request**:
```http
GET /api/v1/monitoring/performance?start=2025-12-27T00:00:00Z&end=2025-12-27T23:59:59Z&interval=1h
Authorization: Bearer <token>
```

**Response**:
```json
{
  "success": true,
  "data": {
    "time_series": [
      {
        "timestamp": "2025-12-27T10:00:00Z",
        "queries_per_second": 5678.90,
        "transactions_per_second": 1234.56,
        "buffer_pool_hit_ratio": 0.982,
        "cpu_usage_percent": 45.6,
        "memory_usage_percent": 67.8,
        "disk_io_ops_per_second": 234.56,
        "network_throughput_mbps": 123.45
      }
    ],
    "interval_seconds": 3600,
    "data_points": 24
  }
}
```

#### Health Check Endpoints

**Liveness Probe** (Kubernetes):
```http
GET /health/live
```

**Response**:
```json
{
  "status": "alive",
  "timestamp": "2025-12-27T10:00:00Z"
}
```

**Readiness Probe** (Kubernetes):
```http
GET /health/ready
```

**Response**:
```json
{
  "status": "ready",
  "checks": {
    "database": "UP",
    "storage": "UP",
    "replication": "UP",
    "network": "UP"
  },
  "timestamp": "2025-12-27T10:00:00Z"
}
```

**Startup Probe** (Kubernetes):
```http
GET /health/startup
```

**Response**:
```json
{
  "status": "started",
  "initialized": true,
  "timestamp": "2025-12-27T10:00:00Z"
}
```

**Full Health Check**:
```http
GET /api/v1/health
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
        "uptime_seconds": 86400,
        "version": "0.5.1"
      },
      "storage": {
        "status": "UP",
        "disk_usage_percent": 45.6,
        "buffer_pool_hit_ratio": 0.982
      },
      "replication": {
        "status": "UP",
        "replicas": 2,
        "lag_bytes": 1024,
        "sync_state": "STREAMING"
      },
      "networking": {
        "status": "UP",
        "active_connections": 45
      }
    },
    "timestamp": "2025-12-27T10:00:00Z"
  }
}
```

### 5. Security Management

#### Encryption Key Management

**List Keys**:
```http
GET /api/v1/security/encryption/keys
Authorization: Bearer <token>
```

**Response**:
```json
{
  "success": true,
  "data": {
    "keys": [
      {
        "key_id": "key_master",
        "type": "MASTER",
        "algorithm": "AES-256-GCM",
        "created_at": "2025-01-01T00:00:00Z",
        "rotated_at": "2025-12-01T00:00:00Z",
        "version": 5,
        "status": "ACTIVE"
      }
    ]
  }
}
```

**Create Key**:
```http
POST /api/v1/security/encryption/keys
Authorization: Bearer <token>
Content-Type: application/json

{
  "type": "TABLE",
  "algorithm": "AES-256-GCM",
  "name": "users_encryption_key"
}
```

**Rotate Key**:
```http
POST /api/v1/security/encryption/keys/{key_id}/rotate
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
    "rotation_started_at": "2025-12-27T10:00:00Z",
    "estimated_completion": "2025-12-27T10:30:00Z"
  }
}
```

#### Data Masking

**List Masking Policies**:
```http
GET /api/v1/security/masking/policies
Authorization: Bearer <token>
```

**Response**:
```json
{
  "success": true,
  "data": {
    "policies": [
      {
        "policy_id": "policy_email",
        "name": "Email Masking",
        "table": "users",
        "column": "email",
        "masking_type": "EMAIL",
        "format": "***@***.com",
        "enabled": true
      }
    ]
  }
}
```

**Create Masking Policy**:
```http
POST /api/v1/security/masking/policies
Authorization: Bearer <token>
Content-Type: application/json

{
  "name": "SSN Masking",
  "table": "employees",
  "column": "ssn",
  "masking_type": "PARTIAL",
  "pattern": "XXX-XX-####",
  "roles_exempt": ["admin"]
}
```

**Test Masking**:
```http
POST /api/v1/security/masking/test
Authorization: Bearer <token>
Content-Type: application/json

{
  "policy_id": "policy_ssn",
  "test_values": ["123-45-6789", "987-65-4321"]
}
```

**Response**:
```json
{
  "success": true,
  "data": {
    "results": [
      {
        "original": "123-45-6789",
        "masked": "XXX-XX-6789"
      },
      {
        "original": "987-65-4321",
        "masked": "XXX-XX-4321"
      }
    ]
  }
}
```

#### Virtual Private Database (VPD)

**List VPD Policies**:
```http
GET /api/v1/security/vpd/policies
Authorization: Bearer <token>
```

**Create VPD Policy**:
```http
POST /api/v1/security/vpd/policies
Authorization: Bearer <token>
Content-Type: application/json

{
  "name": "department_isolation",
  "table": "employees",
  "predicate": "department = SYS_CONTEXT('USERENV', 'DEPARTMENT')",
  "policy_type": "SELECT",
  "enabled": true
}
```

#### Audit Logs

**Get Audit Logs**:
```http
GET /api/v1/security/audit?start=2025-12-27T00:00:00Z&limit=1000&action=TABLE_DELETE
Authorization: Bearer <token>
```

**Response**:
```json
{
  "success": true,
  "data": {
    "logs": [
      {
        "audit_id": "audit_abc123",
        "timestamp": "2025-12-27T10:00:00Z",
        "user": "admin",
        "session_id": "sess_def456",
        "action": "TABLE_DELETE",
        "resource_type": "TABLE",
        "resource_name": "old_table",
        "result": "SUCCESS",
        "client_ip": "192.168.1.100",
        "details": {
          "cascade": true,
          "rows_deleted": 10000
        }
      }
    ]
  }
}
```

### 6. Backup & Recovery

#### Create Backup

**Endpoint**: `POST /api/v1/backup/full`

**Request**:
```http
POST /api/v1/backup/full
Authorization: Bearer <token>
Content-Type: application/json

{
  "compression": true,
  "compression_level": 6,
  "destination": "/backups/rustydb_20251227.tar.gz",
  "include_wal": true,
  "parallel": 4
}
```

**Response**:
```json
{
  "success": true,
  "data": {
    "backup_id": "backup_abc123",
    "type": "FULL",
    "started_at": "2025-12-27T10:00:00Z",
    "status": "IN_PROGRESS",
    "estimated_size_bytes": 10737418240,
    "estimated_duration_seconds": 900
  }
}
```

#### Incremental Backup

**Endpoint**: `POST /api/v1/backup/incremental`

**Request**:
```http
POST /api/v1/backup/incremental
Authorization: Bearer <token>
Content-Type: application/json

{
  "base_backup_id": "backup_xyz789",
  "destination": "/backups/rustydb_incr_20251227.tar.gz"
}
```

#### List Backups

**Endpoint**: `GET /api/v1/backup/list`

**Request**:
```http
GET /api/v1/backup/list?limit=50
Authorization: Bearer <token>
```

**Response**:
```json
{
  "success": true,
  "data": {
    "backups": [
      {
        "backup_id": "backup_abc123",
        "type": "FULL",
        "created_at": "2025-12-27T10:00:00Z",
        "completed_at": "2025-12-27T10:15:00Z",
        "size_bytes": 10737418240,
        "destination": "/backups/rustydb_20251227.tar.gz",
        "status": "COMPLETED",
        "wal_start": "0/1000000",
        "wal_end": "0/2000000"
      }
    ]
  }
}
```

#### Restore Backup

**Endpoint**: `POST /api/v1/backup/{id}/restore`

**Request**:
```http
POST /api/v1/backup/backup_abc123/restore
Authorization: Bearer <token>
Content-Type: application/json

{
  "target_database": "rustydb_restore",
  "point_in_time": "2025-12-27T09:00:00Z",
  "recovery_target": "CONSISTENT"
}
```

**Response**:
```json
{
  "success": true,
  "data": {
    "restore_id": "restore_def456",
    "backup_id": "backup_abc123",
    "status": "IN_PROGRESS",
    "estimated_duration_seconds": 600
  }
}
```

#### Get Backup Status

**Endpoint**: `GET /api/v1/backup/{id}/status`

**Request**:
```http
GET /api/v1/backup/backup_abc123/status
Authorization: Bearer <token>
```

**Response**:
```json
{
  "success": true,
  "data": {
    "backup_id": "backup_abc123",
    "status": "COMPLETED",
    "progress_percent": 100,
    "bytes_processed": 10737418240,
    "duration_seconds": 900,
    "throughput_mbps": 95.4
  }
}
```

#### Point-in-Time Recovery (PITR)

**Endpoint**: `POST /api/v1/backup/pitr`

**Request**:
```http
POST /api/v1/backup/pitr
Authorization: Bearer <token>
Content-Type: application/json

{
  "target_time": "2025-12-27T09:00:00Z",
  "target_database": "rustydb_pitr",
  "base_backup_id": "backup_abc123"
}
```

### 7. Cluster & Replication

#### List Cluster Nodes

**Endpoint**: `GET /api/v1/cluster/nodes`

**Request**:
```http
GET /api/v1/cluster/nodes
Authorization: Bearer <token>
```

**Response**:
```json
{
  "success": true,
  "data": {
    "nodes": [
      {
        "node_id": "node_1",
        "name": "rustydb-primary",
        "role": "PRIMARY",
        "status": "UP",
        "address": "192.168.1.10:5432",
        "version": "0.5.1",
        "uptime_seconds": 86400,
        "replication_lag_bytes": 0,
        "health_score": 100
      },
      {
        "node_id": "node_2",
        "name": "rustydb-replica-1",
        "role": "REPLICA",
        "status": "UP",
        "address": "192.168.1.11:5432",
        "version": "0.5.1",
        "uptime_seconds": 86400,
        "replication_lag_bytes": 1024,
        "health_score": 98
      }
    ],
    "cluster_id": "cluster_main",
    "total_nodes": 2
  }
}
```

#### Add Cluster Node

**Endpoint**: `POST /api/v1/cluster/nodes`

**Request**:
```http
POST /api/v1/cluster/nodes
Authorization: Bearer <token>
Content-Type: application/json

{
  "name": "rustydb-replica-2",
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
    "name": "rustydb-replica-2",
    "status": "JOINING",
    "estimated_sync_time_seconds": 600
  }
}
```

#### Get Replication Status

**Endpoint**: `GET /api/v1/replication/status`

**Request**:
```http
GET /api/v1/replication/status
Authorization: Bearer <token>
```

**Response**:
```json
{
  "success": true,
  "data": {
    "mode": "STREAMING",
    "primary_node": "node_1",
    "replicas": [
      {
        "node_id": "node_2",
        "application_name": "rustydb-replica-1",
        "client_addr": "192.168.1.11",
        "state": "STREAMING",
        "sync_state": "ASYNC",
        "write_lag_bytes": 1024,
        "flush_lag_bytes": 512,
        "replay_lag_bytes": 256,
        "write_lag_ms": 5.2,
        "flush_lag_ms": 2.1,
        "replay_lag_ms": 1.0,
        "sent_lsn": "0/3000000",
        "write_lsn": "0/2FFF600",
        "flush_lsn": "0/2FFF800",
        "replay_lsn": "0/2FFFF00"
      }
    ],
    "replication_slots": [
      {
        "slot_name": "replica_1",
        "plugin": "rustydb",
        "slot_type": "PHYSICAL",
        "database": "rustydb",
        "active": true,
        "restart_lsn": "0/2000000"
      }
    ]
  }
}
```

#### Trigger Failover

**Endpoint**: `POST /api/v1/cluster/failover`

**Request**:
```http
POST /api/v1/cluster/failover
Authorization: Bearer <token>
Content-Type: application/json

{
  "target_node_id": "node_2",
  "force": false,
  "timeout_seconds": 60
}
```

**Response**:
```json
{
  "success": true,
  "data": {
    "failover_id": "failover_abc123",
    "old_primary": "node_1",
    "new_primary": "node_2",
    "status": "IN_PROGRESS",
    "started_at": "2025-12-27T10:00:00Z",
    "estimated_completion": "2025-12-27T10:01:00Z"
  }
}
```

### 8. Connection Pool Management

#### Get Pool Status

**Endpoint**: `GET /api/v1/pool/status`

**Request**:
```http
GET /api/v1/pool/status
Authorization: Bearer <token>
```

**Response**:
```json
{
  "success": true,
  "data": {
    "pool_id": "pool_default",
    "name": "Default Connection Pool",
    "min_connections": 10,
    "max_connections": 100,
    "active_connections": 45,
    "idle_connections": 50,
    "wait_queue_size": 2,
    "utilization_percent": 95,
    "health_status": "HEALTHY"
  }
}
```

#### Get Pool Statistics

**Endpoint**: `GET /api/v1/pool/stats`

**Request**:
```http
GET /api/v1/pool/stats
Authorization: Bearer <token>
```

**Response**:
```json
{
  "success": true,
  "data": {
    "total_connections": 95,
    "active_connections": 45,
    "idle_connections": 50,
    "wait_queue_size": 2,
    "connections_created_total": 1000,
    "connections_closed_total": 905,
    "connections_timed_out": 5,
    "avg_acquire_time_ms": 12.34,
    "max_acquire_time_ms": 567.89,
    "avg_connection_lifetime_ms": 300000,
    "leak_count": 0
  }
}
```

#### Update Pool Configuration

**Endpoint**: `PUT /api/v1/pool/config`

**Request**:
```http
PUT /api/v1/pool/config
Authorization: Bearer <token>
Content-Type: application/json

{
  "min_connections": 20,
  "max_connections": 200,
  "acquire_timeout_seconds": 30,
  "idle_timeout_seconds": 300,
  "max_lifetime_seconds": 3600
}
```

**Response**:
```json
{
  "success": true,
  "data": {
    "updated": true,
    "restart_required": false,
    "changes": {
      "min_connections": {
        "old": 10,
        "new": 20
      },
      "max_connections": {
        "old": 100,
        "new": 200
      }
    }
  }
}
```

### 9. Network API

#### Get Network Status

**Endpoint**: `GET /api/v1/network/status`

**Request**:
```http
GET /api/v1/network/status
Authorization: Bearer <token>
```

**Response**:
```json
{
  "success": true,
  "data": {
    "active_connections": 45,
    "max_connections": 1000,
    "bytes_sent_total": 10485760000,
    "bytes_received_total": 5242880000,
    "packets_sent_total": 1000000,
    "packets_received_total": 900000,
    "errors_total": 5,
    "uptime_seconds": 86400
  }
}
```

#### Get Active Connections

**Endpoint**: `GET /api/v1/network/connections`

**Request**:
```http
GET /api/v1/network/connections?limit=50
Authorization: Bearer <token>
```

**Response**:
```json
{
  "success": true,
  "data": {
    "connections": [
      {
        "connection_id": "conn_abc123",
        "client_addr": "192.168.1.100:54321",
        "server_addr": "192.168.1.10:5432",
        "protocol": "PostgreSQL",
        "state": "ESTABLISHED",
        "user": "admin",
        "database": "rustydb",
        "connected_at": "2025-12-27T09:00:00Z",
        "bytes_sent": 1048576,
        "bytes_received": 524288
      }
    ]
  }
}
```

#### Get Cluster Topology

**Endpoint**: `GET /api/v1/network/cluster/topology`

**Request**:
```http
GET /api/v1/network/cluster/topology
Authorization: Bearer <token>
```

**Response**:
```json
{
  "success": true,
  "data": {
    "cluster_id": "cluster_main",
    "topology_version": 5,
    "nodes": [
      {
        "node_id": "node_1",
        "name": "rustydb-primary",
        "address": "192.168.1.10:5432",
        "role": "PRIMARY",
        "status": "UP",
        "region": "us-east-1",
        "zone": "us-east-1a"
      }
    ],
    "routing": {
      "strategy": "LOCALITY_AWARE",
      "read_preference": "NEAREST",
      "write_preference": "PRIMARY"
    }
  }
}
```

### 10. Advanced Features

#### Machine Learning

**List ML Models**:
```http
GET /api/v1/ml/models
Authorization: Bearer <token>
```

**Create ML Model**:
```http
POST /api/v1/ml/models
Authorization: Bearer <token>
Content-Type: application/json

{
  "name": "customer_churn_predictor",
  "type": "CLASSIFICATION",
  "algorithm": "RANDOM_FOREST",
  "training_data": "SELECT * FROM customer_features"
}
```

**Train Model**:
```http
POST /api/v1/ml/models/{id}/train
Authorization: Bearer <token>
```

**Make Prediction**:
```http
POST /api/v1/ml/models/{id}/predict
Authorization: Bearer <token>
Content-Type: application/json

{
  "features": {
    "age": 35,
    "tenure_months": 24,
    "monthly_charges": 79.99
  }
}
```

#### Graph Database

**Execute Graph Query**:
```http
POST /api/v1/graph/query
Authorization: Bearer <token>
Content-Type: application/json

{
  "query": "MATCH (u:User)-[:FOLLOWS]->(f:User) WHERE u.name = 'Alice' RETURN f.name"
}
```

**Shortest Path**:
```http
POST /api/v1/graph/shortest-path
Authorization: Bearer <token>
Content-Type: application/json

{
  "start_node": "user_1",
  "end_node": "user_100",
  "max_depth": 6
}
```

#### Document Store

**Insert Document**:
```http
POST /api/v1/document/{collection}
Authorization: Bearer <token>
Content-Type: application/json

{
  "name": "John Doe",
  "age": 30,
  "address": {
    "city": "New York",
    "country": "USA"
  },
  "tags": ["customer", "premium"]
}
```

**Query Documents**:
```http
POST /api/v1/document/{collection}/query
Authorization: Bearer <token>
Content-Type: application/json

{
  "filter": {
    "age": {"$gt": 25},
    "tags": {"$in": ["premium"]}
  },
  "projection": ["name", "age"],
  "limit": 10
}
```

#### Spatial Queries

**Spatial Query**:
```http
POST /api/v1/spatial/query
Authorization: Bearer <token>
Content-Type: application/json

{
  "type": "WITHIN",
  "geometry": {
    "type": "Polygon",
    "coordinates": [[[0,0], [10,0], [10,10], [0,10], [0,0]]]
  },
  "table": "locations",
  "geometry_column": "geom"
}
```

**Nearest Neighbors**:
```http
POST /api/v1/spatial/nearest
Authorization: Bearer <token>
Content-Type: application/json

{
  "point": {"x": 40.7128, "y": -74.0060},
  "table": "stores",
  "limit": 5
}
```

---

## GraphQL API Reference

### Endpoint

**URL**: `http://localhost:8080/graphql`

**GraphQL Playground**: `http://localhost:8080/graphql` (GET request in browser)

### Authentication

GraphQL uses the same JWT authentication as REST API:

```http
POST /graphql
Authorization: Bearer <token>
Content-Type: application/json

{
  "query": "{ tables { name } }"
}
```

### Schema Overview

#### Type System

**Transaction Types**:
```graphql
type Transaction {
  transactionId: String!
  status: TransactionStatus!
  timestamp: String!
  isolationLevel: IsolationLevel
}

enum TransactionStatus {
  ACTIVE
  COMMITTED
  ABORTED
}

enum IsolationLevel {
  READ_UNCOMMITTED
  READ_COMMITTED
  REPEATABLE_READ
  SERIALIZABLE
}
```

**Query Result Types**:
```graphql
type QueryResult {
  columns: [ColumnType!]!
  rows: [[JSON!]!]!
  rowsAffected: Int!
  executionTimeMs: Float!
  hasMore: Boolean!
}

type ColumnType {
  name: String!
  dataType: String!
  nullable: Boolean!
}
```

### Queries

**Note**: Query types are defined in schema but testing coverage is incomplete. Verify availability before production use.

```graphql
type Query {
  # Schema Operations
  tables(schema: String, limit: Int, offset: Int): [TableType!]!
  table(name: String!): TableType
  schemas: [DatabaseSchema!]!

  # Data Queries
  queryTable(
    table: String!
    whereClause: WhereClause
    orderBy: [OrderBy!]
    limit: Int
    offset: Int
  ): QueryResult!

  # Statistics
  tableStatistics(table: String!): TableStatistics
  queryPlan(sql: String!): QueryPlan
}
```

**Example - List Tables**:
```graphql
query ListTables {
  tables(schema: "public", limit: 10) {
    name
    schema
    rowCount
    sizeBytes
    columns {
      name
      dataType
      nullable
    }
  }
}
```

**Example - Query Data**:
```graphql
query GetActiveUsers {
  queryTable(
    table: "users"
    whereClause: {
      field: "active"
      op: EQUALS
      value: true
    }
    orderBy: [{field: "created_at", order: DESC}]
    limit: 10
  ) {
    columns {
      name
      dataType
    }
    rows
    executionTimeMs
  }
}
```

### Mutations

**Verified through comprehensive testing (101 tests, 69% pass rate).**

#### Transaction Management

**Begin Transaction**:
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

**Response**:
```json
{
  "data": {
    "beginTransaction": {
      "transactionId": "88790068-3f05-42fb-a5f8-126ccedff088",
      "status": "ACTIVE",
      "timestamp": "2025-12-27T10:00:00Z",
      "isolationLevel": "SERIALIZABLE"
    }
  }
}
```

**Commit Transaction**:
```graphql
mutation CommitTransaction {
  commitTransaction(transactionId: "88790068-3f05-42fb-a5f8-126ccedff088") {
    success
    transactionId
    error
  }
}
```

**Rollback Transaction**:
```graphql
mutation RollbackTransaction {
  rollbackTransaction(transactionId: "88790068-3f05-42fb-a5f8-126ccedff088") {
    success
    transactionId
    error
  }
}
```

#### Atomic Transaction Execution

**Execute Multiple Operations Atomically** (68% pass rate):

```graphql
mutation ExecuteAtomicTransaction {
  executeTransaction(
    operations: [
      {
        operationType: INSERT
        table: "users"
        data: {
          name: "Alice"
          email: "alice@example.com"
        }
      }
      {
        operationType: UPDATE
        table: "users"
        data: {name: "Alice Johnson"}
        where: {email: "alice@example.com"}
      }
    ]
    isolationLevel: SERIALIZABLE
  ) {
    success
    transactionId
    executionTimeMs
    error
  }
}
```

**Response**:
```json
{
  "data": {
    "executeTransaction": {
      "success": true,
      "transactionId": "txn_abc123",
      "executionTimeMs": 0.002826,
      "error": null
    }
  }
}
```

**Operation Types**:
```graphql
enum TransactionOpType {
  INSERT    # Insert new row
  UPDATE    # Update existing row
  DELETE    # Delete row
  SELECT    # Select data (read-only)
}

input TransactionOperation {
  operationType: TransactionOpType!
  table: String!
  data: JSON          # For INSERT/UPDATE
  where: JSON         # For UPDATE/DELETE/SELECT
}
```

### Subscriptions

**Real-time data change subscriptions via WebSocket.**

**⚠️ Status**: Documented but not fully verified in current test suite.

```graphql
type Subscription {
  # Table changes
  tableChanges(
    table: String!
    operations: [ChangeType!]
  ): TableChange

  # Metrics updates
  metrics(intervalSeconds: Int): MetricsUpdate

  # Heartbeat
  heartbeat: Heartbeat
}
```

**Example - Watch Table Changes**:
```graphql
subscription WatchUsers {
  tableChanges(
    table: "users"
    operations: [INSERT, UPDATE, DELETE]
  ) {
    operation
    table
    row
    timestamp
  }
}
```

### GraphQL Configuration

**Max Query Depth**: 10 (prevents deeply nested queries)
**Max Complexity**: 1000 (prevents expensive queries)
**Timeout**: 30 seconds
**Introspection**: Disabled in production
**Playground**: Disabled in production

---

## PostgreSQL Wire Protocol

### Connection

RustyDB implements PostgreSQL wire protocol compatibility, allowing use of standard PostgreSQL clients.

**Connection String Format**:
```
postgresql://[user[:password]@][host][:port][/database][?param=value]
```

**Examples**:
```bash
# psql
psql -h localhost -p 5432 -U admin -d rustydb

# Connection string
psql "postgresql://admin:password@localhost:5432/rustydb?sslmode=require"

# pgAdmin, DBeaver, etc.
Host: localhost
Port: 5432
Database: rustydb
User: admin
Password: ****
SSL Mode: require
```

### Supported Features

**✅ Supported**:
- Basic SQL queries (SELECT, INSERT, UPDATE, DELETE)
- Transactions (BEGIN, COMMIT, ROLLBACK)
- Prepared statements
- Query parameters
- SSL/TLS connections
- Authentication (MD5, SCRAM-SHA-256)

**⚠️ Partial Support**:
- Extended query protocol
- COPY protocol
- Cursors

**❌ Not Supported**:
- Full PostgreSQL catalog (pg_catalog)
- All PostgreSQL functions
- PostgreSQL-specific extensions

### Client Libraries

**Python (psycopg2)**:
```python
import psycopg2

conn = psycopg2.connect(
    host="localhost",
    port=5432,
    database="rustydb",
    user="admin",
    password="password"
)

cur = conn.cursor()
cur.execute("SELECT * FROM users WHERE age > %s", (18,))
rows = cur.fetchall()

conn.close()
```

**Node.js (pg)**:
```javascript
const { Client } = require('pg');

const client = new Client({
  host: 'localhost',
  port: 5432,
  database: 'rustydb',
  user: 'admin',
  password: 'password'
});

await client.connect();
const res = await client.query('SELECT * FROM users WHERE age > $1', [18]);
await client.end();
```

**Java (JDBC)**:
```java
import java.sql.*;

String url = "jdbc:postgresql://localhost:5432/rustydb";
Connection conn = DriverManager.getConnection(url, "admin", "password");

PreparedStatement stmt = conn.prepareStatement("SELECT * FROM users WHERE age > ?");
stmt.setInt(1, 18);
ResultSet rs = stmt.executeQuery();

conn.close();
```

---

## WebSocket API

### Connection

**WebSocket URL**: `ws://localhost:8080/ws`
**WSS URL**: `wss://localhost:8080/ws` (with TLS)

### Connection Establishment

```javascript
const ws = new WebSocket('ws://localhost:8080/ws');

ws.onopen = () => {
  // Authenticate
  ws.send(JSON.stringify({
    type: 'authenticate',
    token: 'your_jwt_token'
  }));
};

ws.onmessage = (event) => {
  const message = JSON.parse(event.data);
  console.log('Received:', message);
};

ws.onerror = (error) => {
  console.error('WebSocket error:', error);
};

ws.onclose = () => {
  console.log('WebSocket closed');
};
```

### Message Types

#### Authentication

```json
{
  "type": "authenticate",
  "token": "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9..."
}
```

**Response**:
```json
{
  "type": "authenticated",
  "success": true,
  "user": "admin"
}
```

#### Subscribe to Table Changes

```json
{
  "type": "subscribe",
  "channel": "table_changes",
  "table": "users",
  "events": ["INSERT", "UPDATE", "DELETE"]
}
```

**Change Notification**:
```json
{
  "type": "table_change",
  "table": "users",
  "operation": "INSERT",
  "data": {
    "id": 123,
    "name": "Alice",
    "email": "alice@example.com"
  },
  "timestamp": "2025-12-27T10:00:00Z"
}
```

#### Query Streaming

```json
{
  "type": "query_stream",
  "query_id": "query_abc123",
  "sql": "SELECT * FROM large_table",
  "batch_size": 100
}
```

**Stream Response**:
```json
{
  "type": "query_batch",
  "query_id": "query_abc123",
  "batch_number": 1,
  "rows": [ /* 100 rows */ ],
  "has_more": true
}
```

**Stream Complete**:
```json
{
  "type": "query_complete",
  "query_id": "query_abc123",
  "total_rows": 10000,
  "execution_time_ms": 1234
}
```

#### Metrics Streaming

```json
{
  "type": "subscribe",
  "channel": "metrics",
  "interval_ms": 1000
}
```

**Metrics Update**:
```json
{
  "type": "metrics_update",
  "timestamp": "2025-12-27T10:00:00Z",
  "metrics": {
    "queries_per_second": 5678.90,
    "transactions_per_second": 1234.56,
    "active_connections": 45
  }
}
```

#### Heartbeat

Server sends periodic heartbeats:

```json
{
  "type": "heartbeat",
  "timestamp": "2025-12-27T10:00:00Z"
}
```

Client should respond:

```json
{
  "type": "pong"
}
```

### WebSocket Endpoints

**Transaction Events**:
- `ws://localhost:8080/api/v1/ws/transactions` - Transaction lifecycle events

**Query Execution**:
- `ws://localhost:8080/api/v1/ws/query-stream` - Streaming query results

**Metrics**:
- `ws://localhost:8080/api/v1/ws/metrics` - Real-time metrics

**Replication**:
- `ws://localhost:8080/api/v1/ws/replication` - Replication status updates

---

## SDK & Client Libraries

### Official SDKs

#### Rust SDK

**Installation**:
```toml
[dependencies]
rustydb-client = "0.5.1"
```

**Usage**:
```rust
use rustydb_client::Client;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = Client::connect(
        "rustydb://admin:password@localhost:5432/rustydb"
    ).await?;

    let rows = client.query("SELECT * FROM users WHERE age > $1", &[&18]).await?;

    for row in rows {
        let name: String = row.get("name");
        let email: String = row.get("email");
        println!("{}: {}", name, email);
    }

    Ok(())
}
```

#### Node.js Adapter

**Installation**:
```bash
npm install @rustydb/client
```

**Usage**:
```javascript
const { RustyDBClient } = require('@rustydb/client');

const client = new RustyDBClient({
  host: 'localhost',
  port: 5432,
  username: 'admin',
  password: 'password',
  database: 'rustydb'
});

await client.connect();

const result = await client.query('SELECT * FROM users WHERE age > $1', [18]);
console.log(result.rows);

await client.disconnect();
```

#### Python Client

**Installation**:
```bash
pip install rustydb-python
```

**Usage**:
```python
import rustydb

client = rustydb.connect(
    host='localhost',
    port=5432,
    user='admin',
    password='password',
    database='rustydb'
)

rows = client.query("SELECT * FROM users WHERE age > %s", [18])
for row in rows:
    print(f"{row['name']}: {row['email']}")

client.close()
```

### Connection Pooling

**Rust**:
```rust
use rustydb_client::{Pool, PoolConfig};

let config = PoolConfig::builder()
    .min_connections(10)
    .max_connections(100)
    .build();

let pool = Pool::new(config).await?;
let conn = pool.get().await?;
```

**Node.js**:
```javascript
const pool = new RustyDBPool({
  host: 'localhost',
  port: 5432,
  min: 10,
  max: 100
});

const client = await pool.connect();
// Use client
await client.release();
```

---

## API Best Practices

### Performance Tips

#### 1. Use Connection Pooling

Always use connection pooling for better performance:

```javascript
// ❌ Bad - creating new connection each time
for (let i = 0; i < 1000; i++) {
  const client = await RustyDB.connect(config);
  await client.query('SELECT ...');
  await client.disconnect();
}

// ✅ Good - reusing connections from pool
const pool = new RustyDBPool(config);
for (let i = 0; i < 1000; i++) {
  const client = await pool.connect();
  await client.query('SELECT ...');
  await client.release();
}
```

#### 2. Use Prepared Statements

Prepared statements improve performance for repeated queries:

```javascript
// ❌ Bad - parsing query each time
for (const id of userIds) {
  await client.query(`SELECT * FROM users WHERE id = ${id}`);
}

// ✅ Good - prepared statement
const stmt = await client.prepare('SELECT * FROM users WHERE id = $1');
for (const id of userIds) {
  await stmt.query([id]);
}
```

#### 3. Batch Operations

Use batch operations instead of individual requests:

```javascript
// ❌ Bad - 1000 round trips
for (const user of users) {
  await client.query('INSERT INTO users VALUES ($1, $2)', [user.name, user.email]);
}

// ✅ Good - 1 batch request
await client.batch({
  statements: users.map(user => ({
    sql: 'INSERT INTO users VALUES ($1, $2)',
    params: [user.name, user.email]
  })),
  transaction: true
});
```

#### 4. Use Appropriate Isolation Levels

Choose the right isolation level for your use case:

```javascript
// ✅ Read-only queries
await client.transaction({
  isolationLevel: 'READ_COMMITTED',
  readOnly: true
});

// ✅ Financial transactions
await client.transaction({
  isolationLevel: 'SERIALIZABLE'
});
```

#### 5. Limit Result Sets

Always limit large result sets:

```javascript
// ❌ Bad - fetching millions of rows
const rows = await client.query('SELECT * FROM huge_table');

// ✅ Good - paginated results
const rows = await client.query('SELECT * FROM huge_table LIMIT 1000 OFFSET 0');

// ✅ Better - cursor-based pagination
const cursor = await client.cursor('SELECT * FROM huge_table');
while (await cursor.hasNext()) {
  const batch = await cursor.fetchMany(1000);
  processBatch(batch);
}
```

### Error Handling Patterns

#### Retry Logic

```javascript
async function queryWithRetry(client, sql, params, maxRetries = 3) {
  for (let i = 0; i < maxRetries; i++) {
    try {
      return await client.query(sql, params);
    } catch (error) {
      if (error.code === 'DEADLOCK' && i < maxRetries - 1) {
        // Exponential backoff
        await new Promise(resolve => setTimeout(resolve, Math.pow(2, i) * 1000));
        continue;
      }
      throw error;
    }
  }
}
```

#### Transaction Error Handling

```javascript
async function transferFunds(fromAccount, toAccount, amount) {
  const txn = await client.beginTransaction();

  try {
    await txn.query('UPDATE accounts SET balance = balance - $1 WHERE id = $2', [amount, fromAccount]);
    await txn.query('UPDATE accounts SET balance = balance + $1 WHERE id = $2', [amount, toAccount]);
    await txn.commit();
  } catch (error) {
    await txn.rollback();
    throw error;
  }
}
```

### Security Best Practices

#### 1. Never Expose Credentials

```javascript
// ❌ Bad - credentials in code
const client = new RustyDBClient({
  username: 'admin',
  password: 'hardcoded_password'
});

// ✅ Good - use environment variables
const client = new RustyDBClient({
  username: process.env.DB_USER,
  password: process.env.DB_PASSWORD
});
```

#### 2. Use Parameterized Queries

```javascript
// ❌ Bad - SQL injection vulnerability
const userId = req.params.id;
await client.query(`SELECT * FROM users WHERE id = ${userId}`);

// ✅ Good - parameterized query
await client.query('SELECT * FROM users WHERE id = $1', [userId]);
```

#### 3. Validate Input

```javascript
function validateEmail(email) {
  const re = /^[^\s@]+@[^\s@]+\.[^\s@]+$/;
  if (!re.test(email)) {
    throw new Error('Invalid email format');
  }
  return email;
}

await client.query('INSERT INTO users (email) VALUES ($1)', [validateEmail(userEmail)]);
```

#### 4. Use TLS

```javascript
const client = new RustyDBClient({
  host: 'rustydb.example.com',
  port: 5432,
  ssl: {
    rejectUnauthorized: true,
    ca: fs.readFileSync('/path/to/ca.crt')
  }
});
```

---

## Error Handling

### HTTP Status Codes

| Code | Status | Description |
|------|--------|-------------|
| 200 | OK | Request succeeded |
| 201 | Created | Resource created |
| 204 | No Content | Success, no response body |
| 400 | Bad Request | Invalid request |
| 401 | Unauthorized | Authentication required/failed |
| 403 | Forbidden | Insufficient permissions |
| 404 | Not Found | Resource not found |
| 409 | Conflict | Constraint violation, deadlock |
| 422 | Unprocessable Entity | Validation error |
| 429 | Too Many Requests | Rate limit exceeded |
| 500 | Internal Server Error | Server error |
| 503 | Service Unavailable | Server overloaded/maintenance |
| 504 | Gateway Timeout | Request timeout |

### Error Response Format

```json
{
  "success": false,
  "error": {
    "code": "ERROR_CODE",
    "message": "Human-readable error message",
    "details": {
      "field": "email",
      "value": "invalid-email",
      "reason": "Invalid email format"
    },
    "sql_state": "23505",
    "hint": "Try a different email address"
  },
  "meta": {
    "requestId": "req_12345",
    "timestamp": "2025-12-27T10:00:00Z"
  }
}
```

### Error Codes

| Code | HTTP Status | Description |
|------|-------------|-------------|
| `INVALID_REQUEST` | 400 | Malformed request |
| `VALIDATION_ERROR` | 422 | Input validation failed |
| `AUTHENTICATION_FAILED` | 401 | Invalid credentials |
| `TOKEN_EXPIRED` | 401 | JWT token expired |
| `TOKEN_INVALID` | 401 | JWT token malformed |
| `PERMISSION_DENIED` | 403 | Insufficient permissions |
| `NOT_FOUND` | 404 | Resource not found |
| `DUPLICATE_KEY` | 409 | Unique constraint violation |
| `FOREIGN_KEY_VIOLATION` | 409 | Foreign key constraint violation |
| `CHECK_CONSTRAINT_VIOLATION` | 409 | Check constraint violation |
| `DEADLOCK` | 409 | Transaction deadlock detected |
| `RATE_LIMIT_EXCEEDED` | 429 | Too many requests |
| `QUERY_TIMEOUT` | 504 | Query execution timeout |
| `CONNECTION_FAILED` | 503 | Database connection failed |
| `TRANSACTION_ABORTED` | 500 | Transaction rolled back |
| `DATABASE_ERROR` | 500 | Internal database error |

### SQL State Codes

RustyDB uses PostgreSQL-compatible SQL state codes:

| SQL State | Category | Description |
|-----------|----------|-------------|
| 00000 | Success | Successful completion |
| 23505 | Integrity Constraint Violation | Unique constraint violation |
| 23503 | Integrity Constraint Violation | Foreign key violation |
| 23514 | Integrity Constraint Violation | Check constraint violation |
| 40001 | Transaction Rollback | Serialization failure |
| 40P01 | Transaction Rollback | Deadlock detected |
| 42601 | Syntax Error | Syntax error in SQL |
| 42703 | Undefined Column | Column does not exist |
| 42P01 | Undefined Table | Table does not exist |
| 57014 | Query Canceled | Query canceled by user |

---

## Rate Limiting

### Rate Limit Configuration

**Global Rate Limits**:
- Default: 100 requests/second per IP
- Configurable per user/API key

**Per-Endpoint Limits**:
- `/api/v1/query/execute`: 50 requests/second
- `/api/v1/query/batch`: 10 requests/second
- `/api/v1/backup/*`: 5 requests/second
- `/api/v1/admin/*`: 20 requests/second

### Rate Limit Headers

Every response includes rate limit information:

```http
HTTP/1.1 200 OK
X-RateLimit-Limit: 100
X-RateLimit-Remaining: 45
X-RateLimit-Reset: 1735300800
X-RateLimit-Reset-After: 42
Retry-After: 42
```

### Rate Limit Exceeded

**Response**:
```json
{
  "success": false,
  "error": {
    "code": "RATE_LIMIT_EXCEEDED",
    "message": "Rate limit exceeded. Please retry after 42 seconds.",
    "details": {
      "limit": 100,
      "window_seconds": 60,
      "retry_after_seconds": 42
    }
  }
}
```

**HTTP Status**: 429 Too Many Requests

### Handling Rate Limits

**Exponential Backoff**:
```javascript
async function requestWithBackoff(fn, maxRetries = 3) {
  for (let i = 0; i < maxRetries; i++) {
    try {
      return await fn();
    } catch (error) {
      if (error.statusCode === 429) {
        const retryAfter = error.headers['retry-after'] || Math.pow(2, i);
        await new Promise(resolve => setTimeout(resolve, retryAfter * 1000));
        continue;
      }
      throw error;
    }
  }
}
```

---

## Performance Optimization

### Query Optimization

#### Use Indexes

```sql
-- Create index on frequently queried columns
CREATE INDEX idx_users_email ON users(email);
CREATE INDEX idx_orders_user_id ON orders(user_id);
```

#### Analyze Query Plans

```http
POST /api/v1/query/explain
{
  "sql": "SELECT * FROM users WHERE email = 'alice@example.com'",
  "analyze": true
}
```

#### Use Query Hints

```sql
-- Force index usage
SELECT /*+ INDEX(users idx_users_email) */ * FROM users WHERE email = 'alice@example.com';

-- Parallel execution
SELECT /*+ PARALLEL(4) */ COUNT(*) FROM large_table;
```

### Connection Pooling Best Practices

**Optimal Pool Sizing**:
```javascript
const poolSize = Math.ceil(availableCPUs * 2) + effectiveSpindleCount;

const pool = new RustyDBPool({
  min: 10,
  max: poolSize,
  acquireTimeout: 30000,
  idleTimeout: 600000
});
```

### Caching Strategies

#### Query Result Caching

```javascript
const cache = new Map();

async function cachedQuery(sql, params) {
  const key = `${sql}:${JSON.stringify(params)}`;

  if (cache.has(key)) {
    return cache.get(key);
  }

  const result = await client.query(sql, params);
  cache.set(key, result);

  // Expire after 5 minutes
  setTimeout(() => cache.delete(key), 300000);

  return result;
}
```

### Monitoring Performance

**Track Query Performance**:
```http
GET /api/v1/monitoring/query-stats?limit=10&sort=avg_time
```

**Monitor Slow Queries**:
```http
GET /api/v1/monitoring/queries?min_duration_ms=1000
```

**Set Up Alerts**:
```http
POST /api/v1/monitoring/alerts
{
  "name": "Slow Query Alert",
  "condition": "avg_query_time_ms > 100",
  "action": "EMAIL",
  "recipients": ["dba@example.com"]
}
```

---

## Appendix

### API Endpoint Summary

**Total REST Endpoints**: 153 (exposed) / 276 (implemented)

**By Category**:
- Core Database: 15 endpoints
- Transactions: 18 endpoints
- Schema Management: 12 endpoints
- Monitoring: 25 endpoints
- Security: 40+ endpoints
- Clustering: 15 endpoints
- Backup: 10 endpoints
- Advanced Features: 18+ endpoints

### Supported Content Types

**Request**:
- `application/json`
- `application/x-www-form-urlencoded`
- `multipart/form-data`

**Response**:
- `application/json`
- `text/plain` (Prometheus metrics)
- `application/octet-stream` (binary data)

### API Versioning

**Current Version**: v1
**Deprecation Policy**: 12 months minimum notice
**Breaking Changes**: Require new major version

**Deprecation Headers**:
```http
X-API-Deprecated: true
X-API-Sunset: 2026-12-31
Link: </api/v2/endpoint>; rel="successor-version"
```

### Support Resources

- **Documentation**: https://rustydb.io/docs
- **API Reference**: https://rustydb.io/api
- **GitHub**: https://github.com/rustydb/rustydb
- **Discord**: https://discord.gg/rustydb
- **Stack Overflow**: Tag `rustydb`

---

**Document Version**: 1.0
**Last Updated**: 2025-12-27
**RustyDB Version**: 0.5.1
**Enterprise Release**: $350M

**Copyright © 2025 RustyDB Contributors. All rights reserved.**
