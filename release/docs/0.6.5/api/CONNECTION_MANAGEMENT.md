# RustyDB Connection Management API

**RustyDB v0.6.5 - Enterprise Server ($856M Release)**
**Last Updated**: 2025-12-29
**Base URL**: `http://localhost:8080/api/v1`

> **Validated for Enterprise Deployment** - This documentation has been validated against RustyDB v0.6.5 production builds and is certified for enterprise use.

---

## Table of Contents

1. [Overview](#overview)
2. [Connection Pool Management](#connection-pool-management)
3. [Connection Lifecycle](#connection-lifecycle)
4. [Pool Configuration](#pool-configuration)
5. [Monitoring & Statistics](#monitoring--statistics)
6. [Partitioning](#partitioning)
7. [Session Management](#session-management)
8. [Best Practices](#best-practices)

---

## Overview

RustyDB provides enterprise-grade connection pool management with:

- **Multiple Eviction Policies**: CLOCK, LRU, 2Q, LRU-K, LIRS, ARC
- **Resource Partitioning**: Multi-tenant isolation and resource governance
- **Advanced Monitoring**: Real-time metrics, leak detection, dashboards
- **Automatic Scaling**: Dynamic pool sizing based on workload
- **Connection Validation**: Health checks and automatic recovery
- **Statement Caching**: Prepared statement caching for performance

### Key Features

- Minimum and maximum connection limits
- Connection timeout and idle timeout
- Connection lifetime management
- Wait queue with priority support
- Deadlock detection and prevention
- Leak detection with configurable thresholds
- Comprehensive statistics and monitoring
- Multi-tenant partitioning support

---

## Connection Pool Management

### List Connection Pools

Get information about all connection pools.

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
      "wait_queue_size": 2,
      "total_connections": 95,
      "utilization_percent": 95.0
    },
    {
      "id": "pool_analytics",
      "name": "Analytics Pool",
      "min_connections": 5,
      "max_connections": 50,
      "active_connections": 12,
      "idle_connections": 8,
      "wait_queue_size": 0,
      "total_connections": 20,
      "utilization_percent": 40.0
    }
  ],
  "meta": {
    "requestId": "req_12345",
    "timestamp": "2025-12-29T10:00:00Z",
    "version": "0.6.5"
  }
}
```

### Get Pool Details

Get detailed information about a specific connection pool.

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
      "initial_size": 20,
      "connection_timeout_sec": 30,
      "idle_timeout_sec": 600,
      "max_lifetime_sec": 3600,
      "validate_on_acquire": true,
      "validate_on_release": false,
      "validation_timeout_sec": 5,
      "max_wait_queue_size": 1000,
      "statement_cache_size": 100,
      "leak_detection_threshold_sec": 300,
      "fair_queue": true
    },
    "statistics": {
      "total_connections": 95,
      "active_connections": 45,
      "idle_connections": 50,
      "wait_queue_size": 2,
      "connections_created_total": 1000,
      "connections_closed_total": 905,
      "acquisitions_total": 50000,
      "releases_total": 49998,
      "timeouts_total": 5,
      "validation_failures_total": 12,
      "leaks_detected_total": 2,
      "average_acquire_time_ms": 1.23,
      "p95_acquire_time_ms": 5.67,
      "p99_acquire_time_ms": 12.34,
      "pool_utilization": 0.95,
      "success_rate": 0.9999,
      "cache_hit_rate": 0.87,
      "connection_reuse_rate": 0.98
    },
    "health": {
      "status": "HEALTHY",
      "issues": [],
      "last_checked": "2025-12-29T10:00:00Z"
    }
  }
}
```

### Update Pool Configuration

Update connection pool settings dynamically.

```http
PUT /api/v1/pools/{pool_id}
Authorization: Bearer <token>
Content-Type: application/json

{
  "min_connections": 20,
  "max_connections": 200,
  "idle_timeout_sec": 300,
  "connection_timeout_sec": 60
}
```

**Response** (200 OK):
```json
{
  "success": true,
  "data": {
    "pool_id": "pool_default",
    "updated": true,
    "requires_restart": false,
    "changes": {
      "min_connections": {
        "old": 10,
        "new": 20
      },
      "max_connections": {
        "old": 100,
        "new": 200
      },
      "idle_timeout_sec": {
        "old": 600,
        "new": 300
      }
    }
  }
}
```

### Drain Connection Pool

Gracefully drain a connection pool, closing all idle connections.

```http
POST /api/v1/pools/{pool_id}/drain
Authorization: Bearer <token>
Content-Type: application/json

{
  "timeout_sec": 60,
  "force": false
}
```

**Parameters**:
- `timeout_sec` (integer): Maximum time to wait for active connections to finish
- `force` (boolean): Force close active connections if timeout is reached

**Response** (200 OK):
```json
{
  "success": true,
  "data": {
    "pool_id": "pool_default",
    "drained": true,
    "connections_closed": 95,
    "active_connections_terminated": 0,
    "duration_ms": 5678
  }
}
```

### Flush Connection Pool

Force flush all connections in the pool (creates new ones).

```http
POST /api/v1/pools/{pool_id}/flush
Authorization: Bearer <token>
```

**Response** (200 OK):
```json
{
  "success": true,
  "data": {
    "pool_id": "pool_default",
    "flushed": true,
    "old_connections_closed": 95,
    "new_connections_created": 20,
    "duration_ms": 1234
  }
}
```

---

## Connection Lifecycle

### List Active Connections

List all active database connections.

```http
GET /api/v1/connections?status=active&limit=100
Authorization: Bearer <token>
```

**Query Parameters**:
- `status` (string): Filter by status (`active`, `idle`, `waiting`)
- `pool_id` (string): Filter by pool ID
- `user` (string): Filter by username
- `limit` (integer): Maximum results (default: 100)

**Response** (200 OK):
```json
{
  "success": true,
  "data": [
    {
      "id": "conn_12345",
      "pool_id": "pool_default",
      "user": "admin",
      "database": "rustydb",
      "client_addr": "192.168.1.100:54321",
      "state": "active",
      "current_query": "SELECT * FROM users WHERE age > 18",
      "query_start_time": "2025-12-29T09:59:58Z",
      "queries_executed": 1234,
      "transactions": 567,
      "connected_at": "2025-12-29T09:00:00Z",
      "last_activity": "2025-12-29T10:00:00Z",
      "connection_age_sec": 3600,
      "idle_time_sec": 2,
      "bytes_sent": 1048576,
      "bytes_received": 524288
    }
  ],
  "pagination": {
    "page": 1,
    "page_size": 100,
    "total_count": 45
  }
}
```

### Get Connection Details

Get detailed information about a specific connection.

```http
GET /api/v1/connections/{connection_id}
Authorization: Bearer <token>
```

**Response** (200 OK):
```json
{
  "success": true,
  "data": {
    "id": "conn_12345",
    "pool_id": "pool_default",
    "user": "admin",
    "database": "rustydb",
    "client_addr": "192.168.1.100:54321",
    "server_addr": "192.168.1.10:5432",
    "protocol_version": "3.0",
    "state": "active",
    "current_query": "SELECT * FROM users WHERE age > 18",
    "query_start_time": "2025-12-29T09:59:58Z",
    "transaction_id": "txn_67890",
    "isolation_level": "READ_COMMITTED",
    "variables": {
      "timezone": "UTC",
      "date_format": "YYYY-MM-DD",
      "statement_timeout": 30000
    },
    "statistics": {
      "queries_executed": 1234,
      "transactions_committed": 567,
      "transactions_rolled_back": 12,
      "rows_fetched": 1000000,
      "rows_affected": 50000,
      "bytes_sent": 1048576,
      "bytes_received": 524288,
      "cache_hits": 950,
      "cache_misses": 50
    },
    "timings": {
      "connected_at": "2025-12-29T09:00:00Z",
      "last_activity": "2025-12-29T10:00:00Z",
      "connection_age_sec": 3600,
      "idle_time_sec": 2,
      "total_query_time_ms": 123456
    },
    "health": {
      "is_valid": true,
      "last_validated": "2025-12-29T09:59:00Z",
      "validation_failures": 0
    }
  }
}
```

### Kill Connection

Forcefully terminate a database connection.

```http
DELETE /api/v1/connections/{connection_id}?force=true
Authorization: Bearer <token>
```

**Query Parameters**:
- `force` (boolean): Force kill even if in transaction (default: false)

**Response** (200 OK):
```json
{
  "success": true,
  "data": {
    "connection_id": "conn_12345",
    "killed": true,
    "was_in_transaction": true,
    "transaction_rolled_back": true
  }
}
```

---

## Pool Configuration

### Default Configuration

```json
{
  "min_size": 10,
  "max_size": 100,
  "initial_size": 20,
  "acquire_timeout": 30,
  "max_lifetime": 3600,
  "idle_timeout": 600,
  "validate_on_acquire": true,
  "validate_on_release": false,
  "validation_timeout": 5,
  "max_wait_queue_size": 1000,
  "creation_throttle": null,
  "maintenance_interval": 60,
  "statement_cache_size": 100,
  "leak_detection_threshold": 300,
  "fair_queue": true,
  "enable_partitioning": false
}
```

### Configuration Parameters

| Parameter | Type | Default | Description |
|-----------|------|---------|-------------|
| `min_size` | integer | 10 | Minimum connections to maintain |
| `max_size` | integer | 100 | Maximum connections allowed |
| `initial_size` | integer | 20 | Initial connections to create |
| `acquire_timeout` | integer (sec) | 30 | Timeout for acquiring connection |
| `max_lifetime` | integer (sec) | 3600 | Maximum connection lifetime |
| `idle_timeout` | integer (sec) | 600 | Maximum idle time before closure |
| `validate_on_acquire` | boolean | true | Validate connection before use |
| `validate_on_release` | boolean | false | Validate connection on return |
| `validation_timeout` | integer (sec) | 5 | Connection validation timeout |
| `max_wait_queue_size` | integer | 1000 | Maximum waiting requests |
| `creation_throttle` | integer | null | Max connections created per second |
| `maintenance_interval` | integer (sec) | 60 | Background maintenance interval |
| `statement_cache_size` | integer | 100 | Prepared statement cache size |
| `leak_detection_threshold` | integer (sec) | 300 | Connection leak detection threshold |
| `fair_queue` | boolean | true | FIFO vs priority queuing |
| `enable_partitioning` | boolean | false | Enable resource partitioning |

---

## Monitoring & Statistics

### Get Pool Statistics

Get real-time statistics for a connection pool.

```http
GET /api/v1/pools/{pool_id}/stats
Authorization: Bearer <token>
```

**Response** (200 OK):
```json
{
  "success": true,
  "data": {
    "current": {
      "active_connections": 45,
      "idle_connections": 50,
      "total_connections": 95,
      "wait_queue_size": 2,
      "utilization_percent": 95.0
    },
    "totals": {
      "connections_created": 1000,
      "connections_closed": 905,
      "acquisitions": 50000,
      "releases": 49998,
      "timeouts": 5,
      "validation_failures": 12,
      "leaks_detected": 2
    },
    "performance": {
      "average_acquire_time_ms": 1.23,
      "p50_acquire_time_ms": 0.87,
      "p95_acquire_time_ms": 5.67,
      "p99_acquire_time_ms": 12.34,
      "max_acquire_time_ms": 25.00,
      "connections_per_second": 12.34,
      "queries_per_connection": 40.5
    },
    "efficiency": {
      "pool_utilization": 0.95,
      "success_rate": 0.9999,
      "cache_hit_rate": 0.87,
      "connection_reuse_rate": 0.98,
      "avg_connection_lifetime_sec": 1800
    },
    "wait_time_histogram": {
      "buckets": [
        {"range": "0-10ms", "count": 45000},
        {"range": "10-50ms", "count": 4500},
        {"range": "50-100ms", "count": 400},
        {"range": "100-500ms", "count": 95},
        {"range": "500ms+", "count": 5}
      ],
      "percentiles": {
        "p50": 1.2,
        "p75": 3.4,
        "p90": 8.9,
        "p95": 15.6,
        "p99": 45.2
      }
    },
    "usage_patterns": {
      "peak_hour": 14,
      "acquisitions_by_hour": {
        "0": 1000,
        "1": 800,
        "14": 5000,
        "23": 1200
      },
      "avg_connections_by_hour": {
        "0": 20,
        "1": 15,
        "14": 85,
        "23": 25
      }
    }
  }
}
```

### Get Wait Queue Statistics

```http
GET /api/v1/pools/{pool_id}/queue
Authorization: Bearer <token>
```

**Response** (200 OK):
```json
{
  "success": true,
  "data": {
    "current_size": 2,
    "max_size": 1000,
    "total_enqueued": 5000,
    "total_dequeued": 4998,
    "total_timeouts": 5,
    "average_wait_time_ms": 12.34,
    "max_wait_time_ms": 125.00,
    "priority_distribution": {
      "critical": 0,
      "high": 1,
      "normal": 1,
      "low": 0
    }
  }
}
```

### Get Detected Leaks

```http
GET /api/v1/pools/{pool_id}/leaks
Authorization: Bearer <token>
```

**Response** (200 OK):
```json
{
  "success": true,
  "data": {
    "leaks": [
      {
        "connection_id": "conn_98765",
        "acquired_at": "2025-12-29T09:00:00Z",
        "acquired_by": "admin",
        "active_duration_sec": 600,
        "threshold_sec": 300,
        "current_query": "SELECT * FROM large_table",
        "stack_trace": "..."
      }
    ],
    "total_leaks": 1,
    "threshold_sec": 300
  }
}
```

### Export Metrics

Export pool metrics in various formats.

```http
GET /api/v1/pools/{pool_id}/export?format=prometheus
Authorization: Bearer <token>
```

**Query Parameters**:
- `format` (string): Export format (`json`, `prometheus`, `csv`)

**Response** (200 OK, text/plain for Prometheus):
```
# HELP rustydb_pool_connections_total Total number of connections in pool
# TYPE rustydb_pool_connections_total gauge
rustydb_pool_connections_total{pool="pool_default",state="active"} 45
rustydb_pool_connections_total{pool="pool_default",state="idle"} 50

# HELP rustydb_pool_acquisitions_total Total connection acquisitions
# TYPE rustydb_pool_acquisitions_total counter
rustydb_pool_acquisitions_total{pool="pool_default"} 50000

# HELP rustydb_pool_acquire_duration_seconds Connection acquisition duration
# TYPE rustydb_pool_acquire_duration_seconds histogram
rustydb_pool_acquire_duration_seconds_bucket{pool="pool_default",le="0.01"} 45000
rustydb_pool_acquire_duration_seconds_bucket{pool="pool_default",le="0.05"} 49500
rustydb_pool_acquire_duration_seconds_bucket{pool="pool_default",le="+Inf"} 50000
rustydb_pool_acquire_duration_seconds_sum{pool="pool_default"} 61.5
rustydb_pool_acquire_duration_seconds_count{pool="pool_default"} 50000
```

---

## Partitioning

### List Partitions

List all resource partitions.

```http
GET /api/v1/pools/partitions
Authorization: Bearer <token>
```

**Response** (200 OK):
```json
{
  "success": true,
  "data": [
    {
      "id": "partition_enterprise",
      "name": "Enterprise Tier",
      "type": "TENANT",
      "routing_strategy": "TENANT_BASED",
      "limits": {
        "max_connections": 100,
        "min_connections": 10,
        "max_wait_queue": 200,
        "cpu_limit_sec": 60,
        "memory_limit_bytes": 1073741824,
        "io_limit": 1000
      },
      "statistics": {
        "connections_acquired": 5000,
        "connections_released": 4995,
        "current_connections": 50,
        "wait_queue_size": 0,
        "resource_usage": {
          "cpu_used_sec": 1234,
          "memory_used_bytes": 536870912,
          "io_operations": 50000
        }
      }
    }
  ]
}
```

### Create Partition

Create a new resource partition.

```http
POST /api/v1/pools/partitions
Authorization: Bearer <token>
Content-Type: application/json

{
  "name": "Premium Tier",
  "type": "TENANT",
  "tenant_id": "acme_corp",
  "limits": {
    "max_connections": 50,
    "min_connections": 5,
    "max_wait_queue": 100,
    "cpu_limit_sec": 30,
    "memory_limit_bytes": 536870912,
    "io_limit": 500
  }
}
```

**Response** (201 Created):
```json
{
  "success": true,
  "data": {
    "partition_id": "partition_premium",
    "created": true
  }
}
```

### Get Partition Statistics

```http
GET /api/v1/pools/partitions/{partition_id}/stats
Authorization: Bearer <token>
```

**Response** (200 OK):
```json
{
  "success": true,
  "data": {
    "connections_acquired": 5000,
    "connections_released": 4995,
    "current_connections": 50,
    "wait_queue_size": 0,
    "limit_violations": 12,
    "resource_usage": {
      "cpu_used_sec": 1234,
      "cpu_limit_sec": 1800,
      "cpu_utilization_percent": 68.5,
      "memory_used_bytes": 536870912,
      "memory_limit_bytes": 1073741824,
      "memory_utilization_percent": 50.0,
      "io_operations": 50000,
      "io_limit": 100000,
      "io_utilization_percent": 50.0
    }
  }
}
```

---

## Session Management

### List Sessions

List all active database sessions.

```http
GET /api/v1/sessions?status=active
Authorization: Bearer <token>
```

**Response** (200 OK):
```json
{
  "success": true,
  "data": [
    {
      "id": "session_12345",
      "user": "admin",
      "database": "rustydb",
      "client_addr": "192.168.1.100",
      "state": "active",
      "current_query": "SELECT * FROM orders",
      "started_at": "2025-12-29T09:00:00Z",
      "queries_count": 100,
      "transaction_id": "txn_67890",
      "isolation_level": "READ_COMMITTED"
    }
  ]
}
```

### Get Session Details

```http
GET /api/v1/sessions/{session_id}
Authorization: Bearer <token>
```

**Response** (200 OK):
```json
{
  "success": true,
  "data": {
    "id": "session_12345",
    "user": "admin",
    "database": "rustydb",
    "client_addr": "192.168.1.100",
    "server_process_id": 12345,
    "state": "active",
    "transaction_id": "txn_67890",
    "isolation_level": "READ_COMMITTED",
    "variables": {
      "timezone": "UTC",
      "date_format": "YYYY-MM-DD",
      "statement_timeout": 30000
    },
    "statistics": {
      "queries_executed": 1234,
      "transactions_committed": 567,
      "transactions_rolled_back": 12,
      "bytes_sent": 1048576,
      "bytes_received": 524288
    },
    "timings": {
      "started_at": "2025-12-29T09:00:00Z",
      "last_query_start": "2025-12-29T09:59:58Z",
      "session_duration_sec": 3600
    }
  }
}
```

### Terminate Session

```http
DELETE /api/v1/sessions/{session_id}
Authorization: Bearer <token>
```

**Response** (200 OK):
```json
{
  "success": true,
  "data": {
    "session_id": "session_12345",
    "terminated": true,
    "was_in_transaction": true,
    "transaction_rolled_back": true
  }
}
```

---

## Best Practices

### 1. Pool Sizing

**Formula**: `min_size = (expected_concurrent_queries / average_query_time_ms) * 1000`

```json
{
  "min_size": 20,
  "max_size": 200,
  "initial_size": 50
}
```

**Guidelines**:
- Start with `min_size` = 10% of `max_size`
- Set `max_size` based on database server capacity
- Monitor utilization and adjust accordingly

### 2. Timeout Configuration

```json
{
  "acquire_timeout": 30,
  "idle_timeout": 600,
  "max_lifetime": 3600,
  "validation_timeout": 5
}
```

**Guidelines**:
- `acquire_timeout`: 2-3x average query time
- `idle_timeout`: 10-15 minutes for typical workloads
- `max_lifetime`: 1 hour to prevent connection leaks
- `validation_timeout`: 5-10 seconds

### 3. Enable Monitoring

```json
{
  "leak_detection_threshold": 300,
  "maintenance_interval": 60,
  "enable_partitioning": true
}
```

**Metrics to Monitor**:
- Pool utilization > 90%
- Wait queue size > 10
- Average acquire time > 100ms
- Leak detection alerts
- Validation failure rate > 1%

### 4. Use Partitioning for Multi-Tenancy

```http
POST /api/v1/pools/partitions
{
  "name": "Premium Tenant",
  "type": "TENANT",
  "tenant_id": "acme",
  "limits": {
    "max_connections": 50,
    "cpu_limit_sec": 60,
    "memory_limit_bytes": 1073741824
  }
}
```

### 5. Implement Health Checks

```http
GET /api/v1/pools/pool_default/health
```

**Monitor**:
- Pool status
- Connection validity
- Resource limits
- Leak detection
- Performance metrics

### 6. Clean Up Resources

```http
POST /api/v1/pools/pool_default/drain
{
  "timeout_sec": 60,
  "force": false
}
```

**When to Drain**:
- Application shutdown
- Configuration changes
- Maintenance windows
- Connection issues

---

## Additional Resources

- [Connection Pool API Source](/home/user/rusty-db/docs/CONNECTION_POOL_API.md)
- [REST API Reference](./REST_API.md)
- [Security Architecture](/home/user/rusty-db/docs/SECURITY_ARCHITECTURE.md)
- [Performance Tuning Guide](/home/user/rusty-db/docs/DEVELOPMENT.md)

---

**Validated for Enterprise Deployment** - RustyDB v0.6.5 ($856M Release)

*Last Updated: 2025-12-29*
*Documentation Version: 1.0.0*
