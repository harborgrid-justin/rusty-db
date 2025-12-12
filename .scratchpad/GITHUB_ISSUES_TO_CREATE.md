# GitHub Issues to Create - RustyDB API Coverage

**Date**: 2025-12-12
**Coordinator**: Agent 11
**Purpose**: Track all identified issues for API coverage completion

---

## Issue #1: CRITICAL - Missing CTE Module File

**Title**: `[CRITICAL] src/execution/cte.rs exported but file doesn't exist`

**Labels**: `bug`, `compilation-error`, `priority-critical`, `execution`

**Description**:
The CTE (Common Table Expressions) module is exported in `src/execution/mod.rs` but the file doesn't exist, blocking compilation.

**Error**:
```
File not found: /home/user/rusty-db/src/execution/cte.rs
Module exported in mod.rs but implementation missing
```

**Exported Types**:
```rust
pub use cte::{CteContext, CteDefinition, RecursiveCteEvaluator, CteOptimizer};
```

**Impact**:
- Blocks compilation of execution module
- CTE queries will not work
- Prevents testing of query execution

**Solution**:
Create `/home/user/rusty-db/src/execution/cte.rs` with implementation of:
- `CteContext` - CTE evaluation context
- `CteDefinition` - CTE definition structure
- `RecursiveCteEvaluator` - Recursive CTE evaluation
- `CteOptimizer` - CTE optimization

**Estimated Effort**: 4-6 hours

**Agent**: Agent 4 (Query Processing)

---

## Issue #2: CRITICAL - ML Handlers Not Imported

**Title**: `[CRITICAL] ML handlers exist but not imported in handlers/mod.rs`

**Labels**: `bug`, `api`, `priority-critical`, `ml`

**Description**:
ML REST API handlers are fully implemented in `src/api/rest/handlers/ml_handlers.rs` (507 lines) but the module is not imported in `mod.rs`, making them inaccessible.

**Affected File**: `/home/user/rusty-db/src/api/rest/handlers/mod.rs`

**Missing Import**:
```rust
pub mod ml_handlers;

pub use ml_handlers::{
    create_model, train_model, predict, list_models, get_model,
    delete_model, get_model_metrics, evaluate_model, export_model
};
```

**Impact**:
- 9 ML REST endpoints completely inaccessible
- ML features hidden from API users
- 0% API coverage for ML despite full implementation

**Endpoints Affected**:
1. POST `/api/v1/ml/models` - Create model
2. POST `/api/v1/ml/models/{id}/train` - Train model
3. POST `/api/v1/ml/models/{id}/predict` - Predict
4. GET `/api/v1/ml/models` - List models
5. GET `/api/v1/ml/models/{id}` - Get model
6. DELETE `/api/v1/ml/models/{id}` - Delete model
7. GET `/api/v1/ml/models/{id}/metrics` - Get metrics
8. POST `/api/v1/ml/models/{id}/evaluate` - Evaluate
9. GET `/api/v1/ml/models/{id}/export` - Export model

**Additional Work**:
- Fix lazy_static state management issues
- Register routes in `server.rs`
- Add to ApiState properly

**Estimated Effort**: 2-3 hours

**Agent**: Agent 9 (ML & Analytics)

---

## Issue #3: CRITICAL - InMemory Handlers Not Imported

**Title**: `[CRITICAL] InMemory handlers exist but not imported in handlers/mod.rs`

**Labels**: `bug`, `api`, `priority-critical`, `inmemory`

**Description**:
InMemory column store REST API handlers are fully implemented in `src/api/rest/handlers/inmemory_handlers.rs` (401 lines) but the module is not imported.

**Affected File**: `/home/user/rusty-db/src/api/rest/handlers/mod.rs`

**Missing Import**:
```rust
pub mod inmemory_handlers;

pub use inmemory_handlers::{
    enable_inmemory, disable_inmemory, inmemory_status, inmemory_stats,
    populate_table, evict_tables, get_table_status, compact_memory,
    update_inmemory_config, get_inmemory_config
};
```

**Impact**:
- 10 InMemory REST endpoints completely inaccessible
- In-memory column store features hidden
- 0% API coverage despite full implementation

**Endpoints Affected**:
1. POST `/api/v1/inmemory/enable` - Enable inmemory
2. POST `/api/v1/inmemory/disable` - Disable inmemory
3. GET `/api/v1/inmemory/status` - Get status
4. GET `/api/v1/inmemory/stats` - Get statistics
5. POST `/api/v1/inmemory/populate` - Populate table
6. POST `/api/v1/inmemory/evict` - Evict tables
7. GET `/api/v1/inmemory/tables/{table}/status` - Table status
8. POST `/api/v1/inmemory/compact` - Compact memory
9. PUT `/api/v1/inmemory/config` - Update config
10. GET `/api/v1/inmemory/config` - Get config

**Additional Work**:
- Fix lazy_static state management issues
- Register routes in `server.rs`

**Estimated Effort**: 2-3 hours

**Agent**: Agent 9 (ML & Analytics)

---

## Issue #4: HIGH - Storage Routes Not Registered

**Title**: `[HIGH] Storage handler routes not registered in REST API server`

**Labels**: `enhancement`, `api`, `priority-high`, `storage`

**Description**:
Storage handlers are fully implemented in `src/api/rest/handlers/storage_handlers.rs` but routes are not registered in `src/api/rest/server.rs`.

**Impact**:
- 12 storage endpoints inaccessible
- Buffer pool, tablespace, partition management unavailable
- Current coverage: 0% despite handlers being ready

**Missing Routes** (add to `src/api/rest/server.rs`):
```rust
// Storage Management API
.route("/api/v1/storage/status", get(storage_handlers::get_storage_status))
.route("/api/v1/storage/disks", get(storage_handlers::get_disks))
.route("/api/v1/storage/partitions", get(storage_handlers::get_partitions))
.route("/api/v1/storage/partitions", post(storage_handlers::create_partition))
.route("/api/v1/storage/partitions/{id}", delete(storage_handlers::delete_partition))
.route("/api/v1/storage/buffer-pool", get(storage_handlers::get_buffer_pool_stats))
.route("/api/v1/storage/buffer-pool/flush", post(storage_handlers::flush_buffer_pool))
.route("/api/v1/storage/tablespaces", get(storage_handlers::get_tablespaces))
.route("/api/v1/storage/tablespaces", post(storage_handlers::create_tablespace))
.route("/api/v1/storage/tablespaces/{id}", put(storage_handlers::update_tablespace))
.route("/api/v1/storage/tablespaces/{id}", delete(storage_handlers::delete_tablespace))
.route("/api/v1/storage/io-stats", get(storage_handlers::get_io_stats))
```

**Estimated Effort**: 1 hour

**Agent**: Agent 1 (Storage)

---

## Issue #5: HIGH - Health Probe Routes Not Registered

**Title**: `[HIGH] Kubernetes health probe handlers not registered`

**Labels**: `enhancement`, `api`, `priority-high`, `monitoring`, `k8s`

**Description**:
Health probe handlers are fully implemented in `src/api/rest/handlers/health_handlers.rs` but routes are not registered, breaking Kubernetes compatibility.

**Impact**:
- Kubernetes liveness/readiness/startup probes not working
- Cannot deploy to Kubernetes properly
- Health checks unavailable

**Missing Routes** (add to `src/api/rest/server.rs`):
```rust
use super::handlers::health_handlers;

.route("/api/v1/health/liveness", get(health_handlers::liveness_probe))
.route("/api/v1/health/readiness", get(health_handlers::readiness_probe))
.route("/api/v1/health/startup", get(health_handlers::startup_probe))
.route("/api/v1/health/full", get(health_handlers::full_health_check))
```

**Estimated Effort**: 30 minutes

**Agent**: Agent 8 (Monitoring & Admin)

---

## Issue #6: HIGH - Diagnostics Routes Not Registered

**Title**: `[HIGH] Diagnostics handler routes not registered`

**Labels**: `enhancement`, `api`, `priority-high`, `monitoring`

**Description**:
Diagnostics handlers are fully implemented in `src/api/rest/handlers/diagnostics_handlers.rs` but routes are not registered.

**Impact**:
- Incident tracking unavailable
- Diagnostic dumps unavailable
- Query profiling API inaccessible
- ASH (Active Session History) not exposed

**Missing Routes** (add to `src/api/rest/server.rs`):
```rust
use super::handlers::diagnostics_handlers;

.route("/api/v1/diagnostics/incidents", get(diagnostics_handlers::get_incidents))
.route("/api/v1/diagnostics/dump", post(diagnostics_handlers::create_dump))
.route("/api/v1/diagnostics/dump/{id}", get(diagnostics_handlers::get_dump_status))
.route("/api/v1/diagnostics/dump/{id}/download", get(diagnostics_handlers::download_dump))
.route("/api/v1/profiling/queries", get(diagnostics_handlers::get_query_profiling))
.route("/api/v1/monitoring/ash", get(diagnostics_handlers::get_active_session_history))
```

**Estimated Effort**: 30 minutes

**Agent**: Agent 8 (Monitoring & Admin)

---

## Issue #7: CRITICAL - RAC API Zero Coverage

**Title**: `[CRITICAL] RAC (Real Application Clusters) has ZERO API exposure`

**Labels**: `enhancement`, `api`, `priority-critical`, `rac`, `enterprise`

**Description**:
RAC is a flagship enterprise feature (Oracle RAC-like) but has absolutely zero API exposure despite full implementation.

**Impact**:
- RAC features completely inaccessible
- Cache Fusion unavailable via API
- Global Resource Directory not exposed
- Interconnect monitoring unavailable
- Enterprise customers cannot use RAC

**Implemented Features** (no API):
- Cache Fusion (memory-to-memory block transfers)
- Global Resource Directory (GRD)
- Cluster Interconnect
- Instance Recovery
- Parallel Query Coordination

**Required Work**:
1. Create `/home/user/rusty-db/src/api/rest/handlers/rac_handlers.rs`
2. Implement 15 core RAC endpoints:
   - GET `/api/v1/rac/cluster/status`
   - GET `/api/v1/rac/cluster/statistics`
   - GET `/api/v1/rac/cache-fusion/status`
   - GET `/api/v1/rac/cache-fusion/stats`
   - GET `/api/v1/rac/cache-fusion/transfers`
   - GET `/api/v1/rac/grd/topology`
   - GET `/api/v1/rac/grd/resources`
   - POST `/api/v1/rac/grd/remaster`
   - GET `/api/v1/rac/interconnect/status`
   - GET `/api/v1/rac/interconnect/stats`
   - GET `/api/v1/rac/recovery/status`
   - GET `/api/v1/rac/recovery/history`
   - POST `/api/v1/rac/parallel-query/execute`
   - GET `/api/v1/rac/parallel-query/status`
   - GET `/api/v1/rac/parallel-query/stats`
3. Register routes in `server.rs`
4. Add GraphQL operations

**Estimated Effort**: 16-20 hours

**Agent**: Agent 7 (Replication & Clustering)

---

## Issue #8: HIGH - Transaction Savepoints API Missing

**Title**: `[HIGH] Transaction savepoints fully implemented but no API`

**Labels**: `enhancement`, `api`, `priority-high`, `transactions`

**Description**:
Transaction savepoints are fully implemented in the transaction module but have zero API exposure.

**Impact**:
- Enterprise transaction control limited
- Cannot use savepoints via API
- Partial rollback unavailable

**Required Endpoints**:
1. GET `/api/v1/transactions/{id}/savepoints` - List savepoints
2. POST `/api/v1/transactions/{id}/savepoints` - Create savepoint
3. POST `/api/v1/transactions/{id}/savepoints/{name}` - Rollback to savepoint
4. DELETE `/api/v1/transactions/{id}/savepoints/{name}` - Release savepoint

**GraphQL Operations**:
```graphql
mutation {
  createSavepoint(transactionId: ID!, name: String!): Savepoint
  rollbackToSavepoint(transactionId: ID!, name: String!): Boolean
}
```

**Estimated Effort**: 4 hours

**Agent**: Agent 2 (Transactions)

---

## Issue #9: HIGH - Analytics Handlers Missing

**Title**: `[HIGH] Analytics module fully implemented but no REST handlers exist`

**Labels**: `enhancement`, `api`, `priority-high`, `analytics`

**Description**:
Analytics module (`src/analytics/`) is fully implemented with OLAP, data profiling, query statistics, etc. but has zero REST API handlers.

**Impact**:
- OLAP cubes inaccessible
- Data profiling unavailable
- Query statistics hidden
- Workload analysis not exposed

**Required Work**:
Create `/home/user/rusty-db/src/api/rest/handlers/analytics_handlers.rs` with:
1. OLAP Operations (4 endpoints)
   - POST `/api/v1/analytics/olap/cubes`
   - GET `/api/v1/analytics/olap/cubes`
   - POST `/api/v1/analytics/olap/cubes/{id}/query`
   - DELETE `/api/v1/analytics/olap/cubes/{id}`
2. Query Analytics (3 endpoints)
   - GET `/api/v1/analytics/query-stats`
   - GET `/api/v1/analytics/workload`
   - GET `/api/v1/analytics/recommendations`
3. Data Quality (3 endpoints)
   - POST `/api/v1/analytics/profile/{table}`
   - GET `/api/v1/analytics/quality/{table}`
   - GET `/api/v1/analytics/quality/{table}/issues`
4. Materialized Views (3 endpoints)
   - POST `/api/v1/analytics/materialized-views`
   - GET `/api/v1/analytics/materialized-views`
   - POST `/api/v1/analytics/materialized-views/{id}/refresh`
5. Time Series Analytics (2 endpoints)
   - POST `/api/v1/analytics/timeseries/analyze`
   - POST `/api/v1/analytics/timeseries/detect-anomalies`

**Estimated Effort**: 16 hours

**Agent**: Agent 9 (ML & Analytics)

---

## Issue #10: HIGH - Query Processing API Gaps

**Title**: `[HIGH] Advanced query processing features not exposed via API`

**Labels**: `enhancement`, `api`, `priority-high`, `query-processing`

**Description**:
Advanced query processing features (optimizer hints, plan baselines, adaptive execution) are fully implemented but have zero API exposure.

**Impact**:
- Optimizer hints (25+ hints) inaccessible
- Plan baselines (SPM) unavailable
- Adaptive execution not controllable
- Query tuning severely limited

**Required Work**:
1. Create `/home/user/rusty-db/src/api/rest/handlers/optimizer.rs`
2. Implement EXPLAIN functionality
3. Add optimizer hint endpoints (7 endpoints)
4. Add plan baseline endpoints (11 endpoints)
5. Add adaptive execution endpoints (6 endpoints)
6. Add parallel query configuration (3 endpoints)

**Total Endpoints**: 27+

**Estimated Effort**: 24 hours

**Agent**: Agent 4 (Query Processing)

---

## Issue #11: MEDIUM - GraphQL Monitoring Coverage

**Title**: `[MEDIUM] GraphQL monitoring queries and mutations missing`

**Labels**: `enhancement`, `graphql`, `priority-medium`, `monitoring`

**Description**:
GraphQL types are defined for monitoring, but queries and mutations are not implemented in `QueryRoot` and `MutationRoot`.

**Impact**:
- GraphQL users cannot access monitoring data
- No queries for metrics, sessions, performance
- No mutations for admin operations

**Required Work**:
1. Add monitoring queries to `src/api/graphql/queries.rs`:
   - metrics, sessionStats, queryStats, performanceData
   - activeQueries, slowQueries
   - clusterTopology, replicationStatus
   - storageStatus, bufferPoolStats, ioStats
   - activeTransactions, locks, deadlocks, mvccStatus
   - serverConfig, healthStatus, alerts
2. Add admin mutations to `src/api/graphql/mutations.rs`:
   - createUser, updateUser, deleteUser
   - createRole, updateRole, deleteRole
   - updateServerConfig, acknowledgeAlert
   - runMaintenance, killQuery, terminateSession

**Total Operations**: 30+

**Estimated Effort**: 16 hours

**Agent**: Agent 8 (Monitoring & Admin)

---

## Issue #12: MEDIUM - GraphQL Network/Pool Coverage

**Title**: `[MEDIUM] GraphQL network and pool operations missing`

**Labels**: `enhancement`, `graphql`, `priority-medium`, `network`, `pool`

**Description**:
Network and pool management is excellent in REST (95% coverage) but GraphQL has only type definitions with no queries/mutations.

**Impact**:
- GraphQL users cannot manage network/pools
- REST-only operations limit GraphQL adoption

**Required Operations**:
1. Network Queries (8)
2. Network Mutations (5)
3. Pool Queries (8)
4. Pool Mutations (5)
5. Subscriptions (3)

**Total Operations**: 29

**Estimated Effort**: 16 hours

**Agent**: Agent 6 (Network & Pool)

---

## Issue #13: MEDIUM - Security Core API Missing

**Title**: `[MEDIUM] Core security features (RBAC, threats, hardening) not exposed`

**Labels**: `enhancement`, `api`, `priority-medium`, `security`

**Description**:
Security vault has excellent coverage (91%), but core security features have <2% API exposure.

**Impact**:
- RBAC management unavailable
- Insider threat detection not exposed
- Network hardening controls missing
- Injection prevention not accessible

**Missing Handlers**:
1. RBAC: 10 endpoints
2. Insider Threat Detection: 9 endpoints
3. Network Hardening: 8 endpoints
4. Injection Prevention: 5 endpoints
5. Auto Recovery: 6 endpoints

**Total Endpoints**: 38+

**Estimated Effort**: 20 hours

**Agent**: Agent 3 (Security)

---

## Issue #14: LOW - Advanced Replication API

**Title**: `[LOW] Advanced replication features not exposed via API`

**Labels**: `enhancement`, `api`, `priority-low`, `replication`

**Description**:
Basic replication works, but advanced features (multi-master, logical, sharding, GDS, XA) have no API.

**Missing Features**:
1. Multi-Master Replication: 8 endpoints
2. Logical Replication: 10 endpoints
3. Sharding: 8 endpoints
4. Global Data Services: 6 endpoints
5. XA Distributed Transactions: 8 endpoints

**Total Endpoints**: 40+

**Estimated Effort**: 32 hours

**Agent**: Agent 7 (Replication & Clustering)

---

## Issue #15: LOW - ML Advanced Features API

**Title**: `[LOW] ML advanced features (AutoML, TimeSeries, PMML) not exposed`

**Labels**: `enhancement`, `api`, `priority-low`, `ml`

**Description**:
Basic ML API will be exposed once Issue #2 is fixed, but advanced features need additional endpoints.

**Missing Features**:
1. AutoML: 3 endpoints
2. Time Series Forecasting: 2 endpoints
3. PMML Import/Export: 2 endpoints
4. Model Versioning: 4 endpoints
5. Feature Explanations: 1 endpoint

**Total Endpoints**: 12

**Estimated Effort**: 16 hours

**Agent**: Agent 9 (ML & Analytics)

---

## Issue #16: LOW - GraphQL Subscriptions

**Title**: `[LOW] GraphQL real-time subscriptions for monitoring`

**Labels**: `enhancement`, `graphql`, `priority-low`, `subscriptions`

**Description**:
Add real-time subscriptions for monitoring metrics, alerts, and query execution.

**Required Subscriptions**:
- metricsStream - Real-time metrics
- alertsStream - Alert notifications
- activeQueriesStream - Query monitoring
- performanceStream - Performance data

**Estimated Effort**: 16 hours

**Agent**: Agent 8 (Monitoring & Admin)

---

## Priority Summary

| Priority | Issues | Est. Hours |
|----------|--------|------------|
| CRITICAL | 4 | 24-31 |
| HIGH | 7 | 89 |
| MEDIUM | 3 | 48 |
| LOW | 3 | 64 |
| **TOTAL** | **17** | **225-232** |

---

## Quick Wins (Implement First)

1. Issue #4 - Register storage routes (1 hour)
2. Issue #5 - Register health probe routes (30 min)
3. Issue #6 - Register diagnostics routes (30 min)
4. Issue #2 - Import ML handlers (2 hours)
5. Issue #3 - Import InMemory handlers (2 hours)

**Total Quick Wins**: 6 hours for immediate 15% coverage improvement

---

**Document Created By**: Agent 11 - Master Coordinator
**Date**: 2025-12-12
**Purpose**: GitHub issue tracking for API coverage completion
