# RustyDB Connection Pool API Reference

**RustyDB v0.6.0 - Enterprise Server**
**Last Updated**: 2025-12-28
**Status**: Production Ready

---

## Table of Contents

1. [Introduction](#introduction)
2. [Pool Creation & Configuration](#pool-creation--configuration)
3. [Connection Management](#connection-management)
4. [Partitioning](#partitioning)
5. [Monitoring & Statistics](#monitoring--statistics)
6. [REST API Endpoints](#rest-api-endpoints)
7. [Rust API](#rust-api)
8. [Best Practices](#best-practices)

---

## Introduction

RustyDB provides enterprise-grade connection pooling for efficient resource management and multi-tenant isolation.

### Key Features

- **Connection Reuse**: Minimize connection overhead
- **Multi-Tenant Partitioning**: Isolated resource pools per tenant
- **Leak Detection**: Automatic detection of connection leaks
- **Health Monitoring**: Real-time pool health metrics
- **Adaptive Sizing**: Dynamic pool sizing based on load
- **Statement Caching**: Prepared statement caching
- **Fair Queuing**: FIFO or priority-based allocation

### Benefits

| Feature | Benefit |
|---------|---------|
| **Connection Reuse** | Reduced overhead, better performance |
| **Partitioning** | Tenant isolation, resource guarantees |
| **Monitoring** | Real-time insights, proactive management |
| **Auto-scaling** | Efficient resource utilization |
| **Leak Detection** | Prevent resource exhaustion |

---

## Pool Creation & Configuration

### Basic Pool Creation (Rust)

```rust
use rusty_db::pool::{ConnectionPool, PoolConfig};
use std::sync::Arc;

let config = PoolConfig::default();
let factory = Arc::new(MyConnectionFactory::new());
let pool = ConnectionPool::new(config, factory).await?;
```

### Custom Configuration

```rust
let config = PoolConfig::builder()
    .min_size(10)                            // Minimum connections
    .max_size(100)                           // Maximum connections
    .initial_size(20)                        // Initial pool size
    .acquire_timeout(Duration::from_secs(30)) // Connection timeout
    .max_lifetime(Duration::from_secs(3600)) // Max connection age
    .idle_timeout(Duration::from_secs(600))  // Max idle time
    .validate_on_acquire(true)               // Validate before use
    .statement_cache_size(100)               // Statement cache
    .enable_partitioning(true)               // Multi-tenant support
    .build()?;
```

### Configuration Options

| Parameter | Type | Default | Description |
|-----------|------|---------|-------------|
| `min_size` | usize | 10 | Minimum connections |
| `max_size` | usize | 100 | Maximum connections |
| `initial_size` | usize | 10 | Initial pool size |
| `acquire_timeout` | Duration | 30s | Timeout for acquiring connection |
| `max_lifetime` | Option<Duration> | None | Max connection age |
| `idle_timeout` | Option<Duration> | None | Max idle time |
| `validate_on_acquire` | bool | true | Validate before use |
| `validate_on_release` | bool | false | Validate on return |
| `max_wait_queue_size` | usize | 1000 | Max waiters |
| `statement_cache_size` | usize | 100 | Statement cache size |
| `enable_partitioning` | bool | false | Multi-tenant support |

---

## Connection Management

### Acquiring Connections (Rust)

```rust
// Acquire a connection
let conn = pool.acquire().await?;

// Use the connection
let result = conn.connection().execute_query("SELECT * FROM users").await?;

// Connection automatically returned on drop
drop(conn);
```

### Connection Guard API

```rust
impl<C> PooledConnectionGuard<C> {
    pub fn connection(&self) -> &C;
    pub fn connection_mut(&mut self) -> &mut C;
    pub fn id(&self) -> u64;
    pub fn age(&self) -> Duration;
}
```

### REST API Endpoints

#### List Connection Pools

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

#### Get Pool Statistics

```http
GET /api/v1/pools/{pool_id}/stats
Authorization: Bearer <token>
```

**Response** (200 OK):
```json
{
  "success": true,
  "data": {
    "connections_created": 100,
    "connections_closed": 5,
    "active_connections": 45,
    "idle_connections": 50,
    "total_connections": 95,
    "wait_queue_size": 2,
    "connections_per_second": 12.34,
    "avg_acquire_time_ms": 5.67,
    "avg_wait_time_ms": 12.34,
    "success_rate": 0.99,
    "pool_utilization": 0.95,
    "cache_hit_rate": 0.87
  }
}
```

#### List Active Connections

```http
GET /api/v1/connections?status=active&pool_id=pool_default
Authorization: Bearer <token>
```

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
      "query": "SELECT * FROM users",
      "connected_at": "2025-12-28T09:00:00Z",
      "last_activity": "2025-12-28T10:00:00Z",
      "age_seconds": 3600
    }
  ]
}
```

#### Kill Connection

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

## Partitioning

### Multi-Tenant Partitioning

Partitioning provides resource isolation for multi-tenant applications.

### Creating Partitions (Rust)

```rust
use rusty_db::pool::{PartitionManager, PartitionType, PartitionLimits, RoutingStrategy};

// Create partition manager
let manager = PartitionManager::new(RoutingStrategy::TenantBased);

// Define resource limits
let limits = PartitionLimits {
    max_connections: 50,
    min_connections: 5,
    max_wait_queue: 100,
    cpu_limit: Some(Duration::from_secs(60)),
    memory_limit: Some(1024 * 1024 * 1024),  // 1GB
    io_limit: Some(1000),  // 1000 IOPS
};

// Create partition
let partition = manager.create_partition(
    "tenant_acme".to_string(),
    PartitionType::Tenant("ACME Corp".to_string()),
    limits,
);
```

### Routing Strategies

```rust
// Tenant-based routing
let manager = PartitionManager::new(RoutingStrategy::TenantBased);

// User-based routing
let manager = PartitionManager::new(RoutingStrategy::UserBased);

// Application-based routing
let manager = PartitionManager::new(RoutingStrategy::ApplicationBased);

// Load-balanced routing
let manager = PartitionManager::new(RoutingStrategy::LoadBalanced);

// Custom routing
let custom_router = Arc::new(|request: &PartitionRequest| {
    Some(format!("partition_{}", request.user.as_ref()?))
});
let manager = PartitionManager::new(RoutingStrategy::Custom(custom_router));
```

### REST API for Partitions

#### Create Partition

```http
POST /api/v1/pools/{pool_id}/partitions
Authorization: Bearer <token>
Content-Type: application/json

{
  "name": "tenant_acme",
  "type": "TENANT",
  "limits": {
    "max_connections": 50,
    "min_connections": 5,
    "max_wait_queue": 100,
    "cpu_limit_seconds": 60,
    "memory_limit_bytes": 1073741824,
    "io_limit_iops": 1000
  }
}
```

**Response** (201 Created):
```json
{
  "success": true,
  "data": {
    "id": "partition_12345",
    "name": "tenant_acme",
    "type": "TENANT",
    "created_at": "2025-12-28T10:00:00Z"
  }
}
```

#### List Partitions

```http
GET /api/v1/pools/{pool_id}/partitions
Authorization: Bearer <token>
```

**Response** (200 OK):
```json
{
  "success": true,
  "data": [
    {
      "id": "partition_12345",
      "name": "tenant_acme",
      "type": "TENANT",
      "limits": {
        "max_connections": 50,
        "min_connections": 5
      },
      "usage": {
        "active_connections": 23,
        "idle_connections": 15
      }
    }
  ]
}
```

---

## Monitoring & Statistics

### Pool Statistics (Rust)

```rust
let stats = pool.statistics();

println!("Connections created: {}", stats.connections_created);
println!("Active connections: {}", stats.active_connections);
println!("Success rate: {:.2}%", stats.success_rate * 100.0);
println!("Avg acquire time: {:?}", stats.average_acquire_time);
```

### Wait Time Histogram

```rust
let histogram = stats.wait_time_histogram;
println!("p50: {:?}", histogram.percentiles.p50);
println!("p95: {:?}", histogram.percentiles.p95);
println!("p99: {:?}", histogram.percentiles.p99);
```

### Efficiency Metrics

```rust
let efficiency = stats.efficiency_metrics;
println!("Pool utilization: {:.2}%", efficiency.pool_utilization * 100.0);
println!("Cache hit rate: {:.2}%", efficiency.cache_hit_rate * 100.0);
println!("Connection reuse: {:.2}%", efficiency.connection_reuse_rate * 100.0);
```

### Leak Detection (Rust)

```rust
use rusty_db::pool::LeakDetector;

let detector = LeakDetector::new(
    Duration::from_secs(300),  // Leak threshold
    Duration::from_secs(60)    // Check interval
);

let leaks = detector.get_leaks();
for leak in leaks {
    eprintln!("LEAK: Connection {} active for {:?}",
              leak.connection_id, leak.active_duration);
}
```

### REST API for Monitoring

#### Get Leak Detection Results

```http
GET /api/v1/pools/{pool_id}/leaks
Authorization: Bearer <token>
```

**Response** (200 OK):
```json
{
  "success": true,
  "data": {
    "detected_leaks": [
      {
        "connection_id": "conn_67890",
        "age_seconds": 1800,
        "user": "admin",
        "query": "SELECT * FROM large_table",
        "allocated_at": "2025-12-28T09:30:00Z"
      }
    ],
    "total_leaks": 1
  }
}
```

#### Export Metrics

```http
GET /api/v1/pools/{pool_id}/metrics?format=prometheus
Authorization: Bearer <token>
```

**Response** (200 OK - Prometheus format):
```
# HELP rustydb_pool_connections_total Total connections
# TYPE rustydb_pool_connections_total gauge
rustydb_pool_connections_total{pool="default"} 95

# HELP rustydb_pool_active_connections Active connections
# TYPE rustydb_pool_active_connections gauge
rustydb_pool_active_connections{pool="default"} 45

# HELP rustydb_pool_wait_queue_size Wait queue size
# TYPE rustydb_pool_wait_queue_size gauge
rustydb_pool_wait_queue_size{pool="default"} 2
```

---

## Rust API

### Connection Factory Implementation

```rust
use rusty_db::pool::ConnectionFactory;
use async_trait::async_trait;

struct MyConnectionFactory;

#[async_trait]
impl ConnectionFactory<MyConnection> for MyConnectionFactory {
    async fn create(&self) -> Result<MyConnection> {
        // Create new connection
        Ok(MyConnection::new())
    }

    async fn validate(&self, connection: &MyConnection) -> Result<bool> {
        // Validate connection is healthy
        Ok(connection.is_connected())
    }

    async fn reset(&self, connection: &mut MyConnection) -> Result<()> {
        // Reset connection state
        connection.reset_state()
    }

    async fn close(&self, connection: MyConnection) -> Result<()> {
        // Close connection
        connection.disconnect()
    }
}
```

### Aging Policies

```rust
use rusty_db::pool::AgingPolicy;

// Time-based aging
let policy = AgingPolicy::TimeBased {
    max_lifetime: Duration::from_secs(3600),
};

// Usage-based aging
let policy = AgingPolicy::UsageBased {
    max_borrows: 1000,
};

// Combined aging
let policy = AgingPolicy::Combined {
    max_lifetime: Duration::from_secs(3600),
    max_borrows: 1000,
};

// Adaptive aging
let policy = AgingPolicy::Adaptive {
    base_lifetime: Duration::from_secs(3600),
    error_threshold: 0.05,  // Recycle if >5% error rate
};
```

### Connection Validator

```rust
use rusty_db::pool::ConnectionValidator;

let validator = ConnectionValidator::new(Duration::from_secs(5))
    .with_query("SELECT 1".to_string())
    .with_fast_validation(true);

let is_valid = validator.validate(&connection).await?;
let stats = validator.statistics();
println!("Validation success rate: {:.2}%", stats.success_rate * 100.0);
```

---

## Best Practices

### Configuration

1. **Set Appropriate Pool Sizes**:
   - Min: 10-20% of max
   - Max: Based on database limits and workload
   - Initial: Equal to min for predictable startup

2. **Enable Validation**:
   - `validate_on_acquire: true` for critical applications
   - `validate_on_release: false` to reduce overhead

3. **Configure Timeouts**:
   - `acquire_timeout`: 10-30 seconds
   - `max_lifetime`: 1-2 hours
   - `idle_timeout`: 5-10 minutes

4. **Enable Statement Caching**:
   - `statement_cache_size`: 100-500 for typical workloads

### Multi-Tenant Applications

1. **Use Partitioning**:
   - Isolate resources per tenant
   - Set resource limits based on tier

2. **Choose Routing Strategy**:
   - Tenant-based for SaaS applications
   - User-based for user-specific workloads
   - Custom for complex routing logic

3. **Monitor Per-Partition**:
   - Track usage per tenant
   - Detect resource violations
   - Alert on quota exceeds

### Monitoring

1. **Track Key Metrics**:
   - Active/idle connections
   - Wait queue size
   - Acquire time (p50, p95, p99)
   - Pool utilization

2. **Enable Leak Detection**:
   - Set threshold: 5-10 minutes
   - Alert on leaks
   - Log stack traces

3. **Export Metrics**:
   - Prometheus for time-series data
   - JSON for custom dashboards
   - CSV for offline analysis

### Performance

1. **Connection Reuse**:
   - Minimize connection creation
   - Keep connections warm
   - Use statement cache

2. **Fair Queuing**:
   - Enable for consistent latency
   - Disable for throughput priority

3. **Resource Limits**:
   - Set per-partition CPU limits
   - Limit memory per partition
   - Throttle I/O if needed

---

## Complete Example: Multi-Tenant Application

```rust
use rusty_db::pool::*;
use std::sync::Arc;
use std::time::Duration;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 1. Create pool with partitioning
    let config = PoolConfig::builder()
        .min_size(20)
        .max_size(200)
        .initial_size(50)
        .enable_partitioning(true)
        .build()?;

    let factory = Arc::new(MyConnectionFactory::new());
    let pool = ConnectionPool::new(config, factory).await?;

    // 2. Set up partitioning for tenants
    let partition_mgr = PartitionManager::new(RoutingStrategy::TenantBased);

    // Enterprise tier
    partition_mgr.create_partition(
        "enterprise_tier".to_string(),
        PartitionType::ResourceGroup("Enterprise".to_string()),
        PartitionLimits {
            max_connections: 100,
            min_connections: 10,
            ..Default::default()
        },
    );

    // Standard tier
    partition_mgr.create_partition(
        "standard_tier".to_string(),
        PartitionType::ResourceGroup("Standard".to_string()),
        PartitionLimits {
            max_connections: 50,
            min_connections: 5,
            ..Default::default()
        },
    );

    // 3. Set up monitoring
    let dashboard = DashboardProvider::new(
        pool.stats.clone(),
        Duration::from_secs(5),
    );

    let leak_detector = LeakDetector::new(
        Duration::from_secs(300),
        Duration::from_secs(60),
    );

    // 4. Monitor in background
    tokio::spawn(async move {
        loop {
            tokio::time::sleep(Duration::from_secs(10)).await;

            let data = dashboard.get_dashboard_data();
            println!("Pool health: {:.1}% efficient, {} active",
                     data.pool_efficiency * 100.0,
                     data.active_connections);

            let leaks = leak_detector.get_leaks();
            if !leaks.is_empty() {
                eprintln!("WARNING: {} leaks detected!", leaks.len());
            }
        }
    });

    Ok(())
}
```

---

## Error Handling

```rust
use rusty_db::pool::PoolError;

match pool.acquire().await {
    Ok(conn) => {
        // Use connection
    }
    Err(PoolError::ConnectionTimeout(duration)) => {
        eprintln!("Timeout after {:?}", duration);
    }
    Err(PoolError::PoolExhausted { active, max }) => {
        eprintln!("Pool full: {}/{} connections", active, max);
    }
    Err(PoolError::ValidationFailed(msg)) => {
        eprintln!("Validation failed: {}", msg);
    }
    Err(e) => {
        eprintln!("Pool error: {}", e);
    }
}
```

---

## Additional Resources

- **API Overview**: [API_OVERVIEW.md](./API_OVERVIEW.md)
- **REST API Reference**: [REST_API.md](./REST_API.md)
- **Multi-Tenant API**: [MULTITENANT_API.md](./MULTITENANT_API.md)

---

**Last Updated**: 2025-12-28
**Product Version**: RustyDB v0.6.0 Enterprise Server
