# RustyDB API Endpoint Reference

**Version**: 1.0
**Last Updated**: 2025-12-12
**Total Endpoints**: 281+ REST handlers + GraphQL API

---

## Table of Contents

1. [Authentication & Authorization](#authentication--authorization)
2. [Database Operations](#database-operations)
3. [Enterprise Features](#enterprise-features)
4. [Monitoring & Health](#monitoring--health)
5. [Advanced Data Stores](#advanced-data-stores)
6. [GraphQL API](#graphql-api)

---

## Authentication & Authorization

### Basic Authentication (4 endpoints)
**File**: `src/api/rest/handlers/auth.rs`

```
POST   /api/v1/auth/login       - Username/password login
POST   /api/v1/auth/logout      - Session termination
POST   /api/v1/auth/refresh     - Token refresh
GET    /api/v1/auth/validate    - Session validation
```

### Enterprise Authentication (7 endpoints)
**File**: `src/api/rest/handlers/enterprise_auth_handlers.rs`

```
POST   /api/v1/auth/ldap/configure     - Configure LDAP provider
POST   /api/v1/auth/ldap/login         - LDAP authentication
POST   /api/v1/auth/oauth2/configure   - Configure OAuth2 provider
POST   /api/v1/auth/oauth2/callback    - OAuth2 callback handler
POST   /api/v1/auth/mfa/enable         - Enable multi-factor auth
POST   /api/v1/auth/mfa/verify         - Verify MFA code
GET    /api/v1/auth/api-keys           - Manage API keys
```

### Privilege Management (7 endpoints)
**File**: `src/api/rest/handlers/privileges_handlers.rs`

```
GET    /api/v1/privileges              - List privileges
POST   /api/v1/privileges/grant        - Grant privilege
POST   /api/v1/privileges/revoke       - Revoke privilege
GET    /api/v1/privileges/audit        - Audit privilege usage
POST   /api/v1/privileges/role         - Assign role privileges
GET    /api/v1/privileges/user/{id}    - Get user privileges
GET    /api/v1/privileges/effective    - Get effective privileges
```

---

## Database Operations

### Core Database (10 endpoints)
**File**: `src/api/rest/handlers/db.rs`

```
GET    /api/v1/databases              - List all databases
POST   /api/v1/databases              - Create database
GET    /api/v1/databases/{id}         - Get database details
PUT    /api/v1/databases/{id}         - Update database
DELETE /api/v1/databases/{id}         - Delete database
GET    /api/v1/schema                 - Get database schema
GET    /api/v1/tables/{name}          - Get table details
POST   /api/v1/tables/{name}          - Create table
PUT    /api/v1/tables/{name}          - Update table
DELETE /api/v1/tables/{name}          - Delete table
```

### SQL Execution (12 endpoints)
**File**: `src/api/rest/handlers/sql.rs`

```
POST   /api/v1/query                  - Execute SQL query
POST   /api/v1/query/explain          - Get query plan
POST   /api/v1/query/prepare          - Prepare statement
POST   /api/v1/query/execute          - Execute prepared statement
POST   /api/v1/batch                  - Execute batch operations
GET    /api/v1/query/history          - Query history
GET    /api/v1/query/stats            - Query statistics
POST   /api/v1/query/cancel           - Cancel running query
GET    /api/v1/query/plan             - Get execution plan
POST   /api/v1/query/optimize         - Optimize query
GET    /api/v1/query/cache            - Query cache status
POST   /api/v1/query/cache/clear      - Clear query cache
```

### Transaction Management (11 endpoints)
**File**: `src/api/rest/handlers/transaction_handlers.rs`

```
POST   /api/v1/transactions                    - Begin transaction
POST   /api/v1/transactions/{id}/commit        - Commit transaction
POST   /api/v1/transactions/{id}/rollback      - Rollback transaction
GET    /api/v1/transactions/{id}/status        - Get transaction status
POST   /api/v1/transactions/{id}/savepoint     - Create savepoint
POST   /api/v1/transactions/{id}/rollback-to   - Rollback to savepoint
GET    /api/v1/transactions                    - List active transactions
DELETE /api/v1/transactions/{id}               - Abort transaction
GET    /api/v1/transactions/{id}/locks         - Get transaction locks
POST   /api/v1/transactions/{id}/isolation     - Set isolation level
GET    /api/v1/transactions/stats              - Transaction statistics
```

### Storage Management (12 endpoints)
**File**: `src/api/rest/handlers/storage_handlers.rs`

```
GET    /api/v1/storage/pages          - Page statistics
POST   /api/v1/storage/checkpoint     - Create checkpoint
GET    /api/v1/storage/buffer-pool    - Buffer pool status
POST   /api/v1/storage/flush          - Flush dirty pages
GET    /api/v1/storage/partitions     - List partitions
POST   /api/v1/storage/partition      - Create partition
DELETE /api/v1/storage/partition/{id} - Drop partition
GET    /api/v1/storage/tiered         - Tiered storage status
POST   /api/v1/storage/tier-move      - Move data between tiers
GET    /api/v1/storage/columnar       - Columnar storage stats
GET    /api/v1/storage/lsm            - LSM tree statistics
POST   /api/v1/storage/compact        - Trigger compaction
```

---

## Enterprise Features

### Backup & Recovery (8 endpoints)
**File**: `src/api/rest/handlers/backup_handlers.rs`

```
POST   /api/v1/backups/full           - Create full backup
POST   /api/v1/backups/incremental    - Create incremental backup
POST   /api/v1/backups/differential   - Create differential backup
GET    /api/v1/backups                - List all backups
GET    /api/v1/backups/{id}           - Get backup details
POST   /api/v1/backups/{id}/restore   - Restore from backup
POST   /api/v1/backups/{id}/verify    - Verify backup integrity
DELETE /api/v1/backups/{id}           - Delete backup
```

### Audit Logging (5 endpoints)
**File**: `src/api/rest/handlers/audit_handlers.rs`

```
GET    /api/v1/audit/logs             - Query audit logs
POST   /api/v1/audit/logs/search      - Advanced audit search
GET    /api/v1/audit/policies         - List audit policies
POST   /api/v1/audit/compliance       - Generate compliance report
GET    /api/v1/audit/incidents        - Get security incidents
```

### Encryption Management (6 endpoints)
**File**: `src/api/rest/handlers/encryption_handlers.rs`

```
GET    /api/v1/encryption/status      - TDE status
POST   /api/v1/encryption/enable      - Enable TDE
POST   /api/v1/encryption/rotate-key  - Rotate encryption key
GET    /api/v1/encryption/keys        - List encryption keys
POST   /api/v1/encryption/keys        - Create new key
DELETE /api/v1/encryption/keys/{id}   - Remove key
```

### Data Masking (8 endpoints)
**File**: `src/api/rest/handlers/masking_handlers.rs`

```
GET    /api/v1/masking/policies       - List masking policies
POST   /api/v1/masking/policies       - Create masking policy
GET    /api/v1/masking/policies/{id}  - Get policy details
PUT    /api/v1/masking/policies/{id}  - Update masking policy
DELETE /api/v1/masking/policies/{id}  - Delete policy
GET    /api/v1/masking/formats        - Available masking formats
POST   /api/v1/masking/test           - Test masking rule
GET    /api/v1/masking/audit          - Masking audit log
```

### Virtual Private Database (9 endpoints)
**File**: `src/api/rest/handlers/vpd_handlers.rs`

```
GET    /api/v1/vpd/policies           - List VPD policies
POST   /api/v1/vpd/policies           - Create VPD policy
GET    /api/v1/vpd/policies/{id}      - Get policy details
PUT    /api/v1/vpd/policies/{id}      - Update VPD policy
DELETE /api/v1/vpd/policies/{id}      - Delete policy
POST   /api/v1/vpd/test               - Test VPD policy
GET    /api/v1/vpd/contexts           - List VPD contexts
POST   /api/v1/vpd/contexts           - Create VPD context
GET    /api/v1/vpd/audit              - VPD audit log
```

### Cluster Management (15 endpoints)
**File**: `src/api/rest/handlers/cluster.rs`

```
GET    /api/v1/cluster/nodes          - List cluster nodes
POST   /api/v1/cluster/nodes          - Add node to cluster
GET    /api/v1/cluster/nodes/{id}     - Get node details
DELETE /api/v1/cluster/nodes/{id}     - Remove node from cluster
GET    /api/v1/cluster/topology       - Get cluster topology
POST   /api/v1/cluster/failover       - Trigger failover
GET    /api/v1/cluster/replication    - Replication status
POST   /api/v1/cluster/replication    - Configure replication
GET    /api/v1/cluster/shards         - List shards
POST   /api/v1/cluster/rebalance      - Rebalance shards
GET    /api/v1/cluster/rac/cache      - RAC Cache Fusion stats
GET    /api/v1/cluster/rac/locks      - Global lock status
POST   /api/v1/cluster/consensus      - Raft consensus operations
GET    /api/v1/cluster/health         - Cluster health
GET    /api/v1/cluster/config         - Cluster configuration
```

---

## Monitoring & Health

### Health Checks (4 endpoints)
**File**: `src/api/rest/handlers/health_handlers.rs`

```
GET    /api/v1/health                 - Basic health check
GET    /api/v1/health/live            - Liveness probe
GET    /api/v1/health/ready           - Readiness probe
GET    /api/v1/health/startup         - Startup probe
```

### Monitoring & Metrics (16 endpoints)
**File**: `src/api/rest/handlers/monitoring.rs`

```
GET    /api/v1/metrics                - Custom metrics (JSON)
GET    /api/v1/metrics/prometheus     - Prometheus format
GET    /api/v1/stats/sessions         - Session statistics
GET    /api/v1/stats/queries          - Query statistics
GET    /api/v1/stats/performance      - Performance data
GET    /api/v1/stats/buffer-pool      - Buffer pool stats
GET    /api/v1/stats/cache            - Cache statistics
GET    /api/v1/stats/locks            - Lock statistics
GET    /api/v1/logs                   - Log entries
GET    /api/v1/alerts                 - Alert list
POST   /api/v1/alerts/{id}/ack        - Acknowledge alert
GET    /api/v1/profiling/cpu          - CPU profiling
GET    /api/v1/profiling/memory       - Memory profiling
POST   /api/v1/profiling/start        - Start profiling
POST   /api/v1/profiling/stop         - Stop profiling
GET    /api/v1/tracing                - Distributed tracing
```

### Diagnostics (6 endpoints)
**File**: `src/api/rest/handlers/diagnostics_handlers.rs`

```
GET    /api/v1/diagnostics/deadlocks  - Detect deadlocks
GET    /api/v1/diagnostics/locks      - Lock analysis
GET    /api/v1/diagnostics/slow       - Slow query analysis
POST   /api/v1/diagnostics/analyze    - Run diagnostics
GET    /api/v1/diagnostics/report     - Get diagnostic report
GET    /api/v1/diagnostics/suggestions - Performance suggestions
```

### Dashboard (5 endpoints)
**File**: `src/api/rest/handlers/dashboard_handlers.rs`

```
GET    /api/v1/dashboard/overview     - Dashboard overview
GET    /api/v1/dashboard/realtime     - Real-time metrics
GET    /api/v1/dashboard/historical   - Historical data
GET    /api/v1/dashboard/widgets      - Dashboard widgets
POST   /api/v1/dashboard/custom       - Custom dashboard
```

---

## Advanced Data Stores

### Document Store (12 endpoints)
**File**: `src/api/rest/handlers/document_handlers.rs`

```
GET    /api/v1/documents              - List documents
POST   /api/v1/documents              - Insert document
GET    /api/v1/documents/{id}         - Get document
PUT    /api/v1/documents/{id}         - Update document
DELETE /api/v1/documents/{id}         - Delete document
POST   /api/v1/documents/query        - Query documents
POST   /api/v1/documents/aggregate    - Aggregation pipeline
GET    /api/v1/documents/collections  - List collections
POST   /api/v1/documents/collections  - Create collection
GET    /api/v1/documents/indexes      - List indexes
POST   /api/v1/documents/indexes      - Create index
POST   /api/v1/documents/bulk         - Bulk operations
```

### Graph Database (8 endpoints)
**File**: `src/api/rest/handlers/graph_handlers.rs`

```
GET    /api/v1/graph/nodes            - List graph nodes
POST   /api/v1/graph/nodes            - Create node
GET    /api/v1/graph/edges            - List edges
POST   /api/v1/graph/edges            - Create edge
POST   /api/v1/graph/query            - Execute graph query
POST   /api/v1/graph/traverse         - Graph traversal
POST   /api/v1/graph/shortest-path    - Find shortest path
GET    /api/v1/graph/algorithms       - Run graph algorithms
```

### Geospatial (10 endpoints)
**File**: `src/api/rest/handlers/spatial_handlers.rs`

```
POST   /api/v1/spatial/query          - Spatial query
POST   /api/v1/spatial/index          - Create spatial index
GET    /api/v1/spatial/bbox           - Bounding box query
POST   /api/v1/spatial/intersects     - Intersection query
POST   /api/v1/spatial/within         - Within query
POST   /api/v1/spatial/distance       - Distance calculation
POST   /api/v1/spatial/nearest        - Nearest neighbor
GET    /api/v1/spatial/formats        - Supported formats
POST   /api/v1/spatial/transform      - Coordinate transformation
POST   /api/v1/spatial/buffer         - Buffer operation
```

### In-Memory Analytics (10 endpoints)
**File**: `src/api/rest/handlers/inmemory_handlers.rs`

```
POST   /api/v1/inmemory/load          - Load data to memory
GET    /api/v1/inmemory/status        - In-memory status
POST   /api/v1/inmemory/query         - In-memory query
POST   /api/v1/inmemory/aggregate     - In-memory aggregation
GET    /api/v1/inmemory/stats         - Memory statistics
POST   /api/v1/inmemory/evict         - Evict from memory
POST   /api/v1/inmemory/columnar      - Columnar operations
POST   /api/v1/inmemory/vectorized    - SIMD operations
GET    /api/v1/inmemory/compression   - Compression stats
POST   /api/v1/inmemory/optimize      - Optimize memory usage
```

### Data Streaming (11 endpoints)
**File**: `src/api/rest/handlers/streams_handlers.rs`

```
POST   /api/v1/streams/create         - Create stream
GET    /api/v1/streams                - List streams
GET    /api/v1/streams/{id}           - Get stream details
DELETE /api/v1/streams/{id}           - Delete stream
POST   /api/v1/streams/{id}/publish   - Publish to stream
POST   /api/v1/streams/{id}/subscribe - Subscribe to stream
GET    /api/v1/streams/{id}/cdc       - Change Data Capture
POST   /api/v1/streams/transform      - Stream transformation
GET    /api/v1/streams/{id}/stats     - Stream statistics
POST   /api/v1/streams/{id}/replay    - Replay stream
POST   /api/v1/streams/pipeline       - Create pipeline
```

### Machine Learning (9 endpoints)
**File**: `src/api/rest/handlers/ml_handlers.rs`

```
POST   /api/v1/ml/train               - Train model
POST   /api/v1/ml/predict             - Make prediction
GET    /api/v1/ml/models              - List models
GET    /api/v1/ml/models/{id}         - Get model details
DELETE /api/v1/ml/models/{id}         - Delete model
POST   /api/v1/ml/evaluate            - Evaluate model
POST   /api/v1/ml/feature-engineering - Feature engineering
GET    /api/v1/ml/algorithms          - Available algorithms
POST   /api/v1/ml/batch-predict       - Batch prediction
```

---

## Additional Features

### Connection Pooling (12 endpoints)
**File**: `src/api/rest/handlers/pool.rs`

### Network Operations (13 endpoints)
**File**: `src/api/rest/handlers/network_handlers.rs`

### API Gateway (19 endpoints)
**File**: `src/api/rest/handlers/gateway_handlers.rs`

### Admin Operations (16 endpoints)
**File**: `src/api/rest/handlers/admin.rs`

### System Information (5 endpoints)
**File**: `src/api/rest/handlers/system.rs`

### Label Management (9 endpoints)
**File**: `src/api/rest/handlers/labels_handlers.rs`

---

## GraphQL API

**Endpoint**: `/graphql` (POST/GET)
**WebSocket**: `/graphql/ws`
**Implementation**: 8,295 lines across 11 files

### Features

1. **Query Operations**
   - Database queries
   - Table queries
   - User and role queries
   - Metrics queries
   - Health status queries

2. **Mutation Operations**
   - CRUD operations
   - Transaction management
   - User management
   - Configuration updates

3. **Subscriptions**
   - Real-time data changes
   - Query progress
   - System metrics
   - Audit log streaming
   - Replication status

4. **Security**
   - Complexity analysis (max: 1000)
   - Depth limiting (max: 10)
   - Field-level authorization
   - Rate limiting
   - Introspection control

5. **Performance**
   - DataLoader batching
   - Query caching
   - Performance monitoring extension

### Schema Types

- Database, Table, Column, Index
- User, Role, Permission
- Transaction, Query, QueryResult
- Backup, Snapshot, Restore
- Cluster, Node, Replication
- Audit, SecurityEvent, Compliance
- And 40+ more types...

---

## API Conventions

### Authentication

All endpoints (except `/health*` and `/auth/login`) require authentication:

```
Authorization: Bearer <JWT_TOKEN>
```

Or API key:

```
X-API-Key: <API_KEY>
```

### Response Format

```json
{
  "success": true,
  "data": { ... },
  "meta": {
    "timestamp": "2025-12-12T00:00:00Z",
    "request_id": "uuid"
  }
}
```

Error response:

```json
{
  "success": false,
  "error": {
    "code": "ERROR_CODE",
    "message": "Human readable error",
    "details": { ... }
  },
  "meta": {
    "timestamp": "2025-12-12T00:00:00Z",
    "request_id": "uuid"
  }
}
```

### Pagination

```
GET /api/v1/endpoint?page=1&limit=50&sort=created_at:desc
```

### Filtering

```
GET /api/v1/endpoint?filter[status]=active&filter[type]=backup
```

---

## Rate Limiting

- Default: 100 requests/minute per user
- Burst: 200 requests/minute
- GraphQL: 1000 complexity points/minute

Headers:

```
X-RateLimit-Limit: 100
X-RateLimit-Remaining: 95
X-RateLimit-Reset: 1670812800
```

---

## API Versioning

Current version: `v1`

Version in URL: `/api/v1/...`

Version in header: `Accept: application/vnd.rustydb.v1+json`

---

## Support

- **Documentation**: https://docs.rustydb.io/api
- **OpenAPI Spec**: `/api/v1/openapi.json`
- **GraphQL Playground**: `/graphql` (when enabled)
- **GraphQL Schema**: `/graphql/schema.graphql`

---

*Last Updated: 2025-12-12*
*Total Endpoints: 281+ REST handlers + GraphQL API*
*Coverage: ~95% of enterprise features*
